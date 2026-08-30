// Generated from tko.org. Do not edit by hand.

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

pub(crate) type Result<T> = std::result::Result<T, TransactionError>;

#[derive(Debug)]
pub(crate) struct TransactionError {
    message: String,
}

impl TransactionError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for TransactionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.message)
    }
}

impl Error for TransactionError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExactReplacement {
    pub(crate) path: PathBuf,
    pub(crate) original: String,
    pub(crate) replacement: String,
}

#[derive(Debug)]
pub(crate) struct LocalGitTransaction {
    root: PathBuf,
    affected: BTreeSet<PathBuf>,
    _lock: StoreLock,
}

impl LocalGitTransaction {
    pub(crate) fn begin(ticket_root: &Path, affected_paths: &[PathBuf]) -> Result<Option<Self>> {
        if !ticket_root.join(".git").exists() {
            return Ok(None);
        }

        let root = fs::canonicalize(ticket_root).map_err(|error| {
            TransactionError::new(format!(
                "cannot resolve ticket store {}: {error}",
                ticket_root.display()
            ))
        })?;
        let git_root = git_text(&root, &["rev-parse", "--show-toplevel"])?;
        let git_root = fs::canonicalize(git_root.trim()).map_err(|error| {
            TransactionError::new(format!("cannot resolve Git worktree root: {error}"))
        })?;
        if git_root != root {
            return Err(TransactionError::new(format!(
                "ticket store {} is not the root of its Git worktree",
                root.display()
            )));
        }

        let lock_path = PathBuf::from(
            git_text(
                &root,
                &[
                    "rev-parse",
                    "--path-format=absolute",
                    "--git-path",
                    "tko-transaction.lock",
                ],
            )?
            .trim(),
        );
        let lock = StoreLock::acquire(lock_path)?;
        let affected = affected_paths
            .iter()
            .map(|path| relative_ticket_path(&root, path))
            .collect::<Result<BTreeSet<_>>>()?;
        if affected.is_empty() {
            return Err(TransactionError::new(
                "local Git transaction requires at least one affected ticket",
            ));
        }

        let transaction = Self {
            root,
            affected,
            _lock: lock,
        };
        transaction.require_healthy_index()?;
        transaction.require_clean_affected_paths()?;
        Ok(Some(transaction))
    }

    fn require_healthy_index(&self) -> Result<()> {
        let unmerged = git_text(&self.root, &["ls-files", "--unmerged"])?;
        if !unmerged.is_empty() {
            return Err(TransactionError::new(
                "ticket Git worktree has unresolved conflicts",
            ));
        }

        match git_diff_status(&self.root, &["diff", "--cached", "--quiet", "--"])? {
            false => Ok(()),
            true => Err(TransactionError::new(
                "ticket Git worktree has staged changes; commit or unstage them first",
            )),
        }
    }

    fn require_clean_affected_paths(&self) -> Result<()> {
        for path in &self.affected {
            let tracked = git_with_path(&self.root, &["ls-files", "--error-unmatch", "--"], path)?;
            if !tracked.status.success() {
                return Err(TransactionError::new(format!(
                    "affected ticket is not tracked by Git: {}",
                    path.display()
                )));
            }

            let dirty = git_with_path(&self.root, &["diff", "--quiet", "--"], path)?;
            match dirty.status.code() {
                Some(0) => {}
                Some(1) => {
                    return Err(TransactionError::new(format!(
                        "affected ticket has local modifications: {}",
                        path.display()
                    )));
                }
                _ => return Err(git_failure("inspect affected ticket", &dirty)),
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
struct StoreLock {
    path: PathBuf,
}

impl StoreLock {
    fn acquire(path: PathBuf) -> Result<Self> {
        fs::create_dir(&path).map_err(|error| {
            TransactionError::new(format!(
                "cannot acquire ticket transaction lock {}: {error}",
                path.display()
            ))
        })?;
        Ok(Self { path })
    }
}

impl Drop for StoreLock {
    fn drop(&mut self) {
        let _ = fs::remove_dir(&self.path);
    }
}

impl LocalGitTransaction {
    pub(crate) fn record(&self, replacements: &[ExactReplacement], subject: &str) -> Result<bool> {
        if replacements.is_empty() {
            return Ok(false);
        }

        let mut relative_paths = BTreeSet::new();
        for replacement in replacements {
            let relative = relative_ticket_path(&self.root, &replacement.path)?;
            if !self.affected.contains(&relative) {
                return Err(TransactionError::new(format!(
                    "replacement is outside the resolved affected set: {}",
                    relative.display()
                )));
            }
            if !relative_paths.insert(relative) {
                return Err(TransactionError::new("duplicate replacement path"));
            }
        }

        for replacement in replacements {
            let current = fs::read_to_string(&replacement.path).map_err(|error| {
                TransactionError::new(format!(
                    "cannot verify original {}: {error}",
                    replacement.path.display()
                ))
            })?;
            if current != replacement.original {
                return Err(TransactionError::new(format!(
                    "affected ticket changed while planning: {}",
                    replacement.path.display()
                )));
            }
        }

        for replacement in replacements {
            if let Err(error) = fs::write(&replacement.path, &replacement.replacement) {
                return Err(self.rollback_after(
                    replacements,
                    format!("cannot persist {}: {error}", replacement.path.display()),
                ));
            }
        }

        let mut add = Command::new("git");
        add.arg("-C").arg(&self.root).arg("add").arg("--");
        for path in &relative_paths {
            add.arg(path);
        }
        let add = add.output().map_err(|error| {
            self.rollback_after(replacements, format!("cannot invoke git add: {error}"))
        })?;
        if !add.status.success() {
            return Err(self.rollback_after(replacements, git_failure_message("git add", &add)));
        }

        let staged_output =
            match git_output(&self.root, &["diff", "--cached", "--name-only", "-z", "--"]) {
                Ok(output) => output,
                Err(error) => return Err(self.rollback_after(replacements, error.to_string())),
            };
        if !staged_output.status.success() {
            return Err(self.rollback_after(
                replacements,
                git_failure_message("inspect staged paths", &staged_output),
            ));
        }
        let staged = String::from_utf8(staged_output.stdout).map_err(|error| {
            self.rollback_after(
                replacements,
                format!("staged path output is not UTF-8: {error}"),
            )
        })?;
        let staged = staged
            .split('\0')
            .filter(|path| !path.is_empty())
            .map(PathBuf::from)
            .collect::<BTreeSet<_>>();
        if staged != relative_paths {
            return Err(self.rollback_after(
                replacements,
                "Git staged a path set different from the exact replacements".to_string(),
            ));
        }

        let commit = Command::new("git")
            .arg("-C")
            .arg(&self.root)
            .args(["commit", "--quiet", "-m", subject, "--"])
            .output()
            .map_err(|error| {
                self.rollback_after(replacements, format!("cannot invoke git commit: {error}"))
            })?;
        if !commit.status.success() {
            return Err(
                self.rollback_after(replacements, git_failure_message("git commit", &commit))
            );
        }

        Ok(true)
    }

    fn rollback_after(
        &self,
        replacements: &[ExactReplacement],
        failure: String,
    ) -> TransactionError {
        let mut rollback_failures = Vec::new();
        for replacement in replacements {
            if let Err(error) = fs::write(&replacement.path, &replacement.original) {
                rollback_failures.push(format!("restore {}: {error}", replacement.path.display()));
            }
        }

        let reset = git_output(&self.root, &["reset", "--quiet", "--"]);
        match reset {
            Ok(output) if output.status.success() => {}
            Ok(output) => rollback_failures.push(git_failure_message("git reset", &output)),
            Err(error) => rollback_failures.push(error.to_string()),
        }

        if rollback_failures.is_empty() {
            TransactionError::new(format!(
                "{failure}; restored affected files and empty index"
            ))
        } else {
            TransactionError::new(format!(
                "{failure}; rollback also failed: {}",
                rollback_failures.join("; ")
            ))
        }
    }
}

fn relative_ticket_path(root: &Path, path: &Path) -> Result<PathBuf> {
    let path = fs::canonicalize(path).map_err(|error| {
        TransactionError::new(format!(
            "cannot resolve affected path {}: {error}",
            path.display()
        ))
    })?;
    path.strip_prefix(root).map(Path::to_path_buf).map_err(|_| {
        TransactionError::new(format!(
            "affected path is outside ticket worktree: {}",
            path.display()
        ))
    })
}

fn git_text(root: &Path, args: &[&str]) -> Result<String> {
    let output = git_output(root, args)?;
    if !output.status.success() {
        return Err(git_failure("run Git command", &output));
    }
    String::from_utf8(output.stdout)
        .map_err(|error| TransactionError::new(format!("Git output is not UTF-8: {error}")))
}

fn git_output(root: &Path, args: &[&str]) -> Result<Output> {
    Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .map_err(|error| TransactionError::new(format!("cannot invoke git: {error}")))
}

fn git_with_path(root: &Path, args: &[&str], path: &Path) -> Result<Output> {
    Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .arg(path)
        .output()
        .map_err(|error| TransactionError::new(format!("cannot invoke git: {error}")))
}

fn git_diff_status(root: &Path, args: &[&str]) -> Result<bool> {
    let output = git_output(root, args)?;
    match output.status.code() {
        Some(0) => Ok(false),
        Some(1) => Ok(true),
        _ => Err(git_failure("inspect Git diff", &output)),
    }
}

fn git_failure(operation: &str, output: &Output) -> TransactionError {
    TransactionError::new(git_failure_message(operation, output))
}

fn git_failure_message(operation: &str, output: &Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr);
    let detail = stderr.trim();
    if detail.is_empty() {
        format!("{operation} failed with {}", output.status)
    } else {
        format!("{operation} failed: {detail}")
    }
}
