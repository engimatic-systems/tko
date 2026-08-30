// Generated from tko.org. Do not edit by hand.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::thread;
use std::time::Duration;
use tempfile::TempDir;

fn tko_bin() -> &'static str {
    env!("CARGO_BIN_EXE_tko")
}

struct GitFixture {
    _temp: TempDir,
    repo: PathBuf,
    tickets: PathBuf,
}

impl GitFixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().expect("tempdir");
        let repo = temp.path().join("sys");
        fs::create_dir(&repo).expect("repo dir");
        git_ok(&repo, &["init", "--quiet", "--initial-branch=main"]);
        git_ok(&repo, &["config", "user.name", "TKO Transaction Test"]);
        git_ok(
            &repo,
            &["config", "user.email", "tko-transaction@example.invalid"],
        );
        git_ok(&repo, &["config", "commit.gpgsign", "false"]);
        fs::write(repo.join("README"), "fixture\n").expect("README");
        git_ok(&repo, &["add", "README"]);
        git_ok(&repo, &["commit", "--quiet", "-m", "initial"]);

        git_ok(&repo, &["switch", "--quiet", "--orphan", "tickets"]);
        write_ticket(&repo, "sys-a", "open", "[]");
        write_ticket(&repo, "sys-b", "open", "[]");
        write_ticket(&repo, "sys-c", "open", "[]");
        git_ok(&repo, &["add", "sys-a.org", "sys-b.org", "sys-c.org"]);
        git_ok(&repo, &["commit", "--quiet", "-m", "ticket snapshot"]);
        git_ok(&repo, &["switch", "--quiet", "main"]);

        let tickets = temp.path().join("tickets-worktree");
        let output = Command::new("git")
            .arg("-C")
            .arg(&repo)
            .args(["worktree", "add", "--quiet"])
            .arg(&tickets)
            .arg("tickets")
            .output()
            .expect("git worktree add");
        assert_git_success("git worktree add", &output);

        Self {
            _temp: temp,
            repo,
            tickets,
        }
    }

    fn command(&self, args: &[&str]) -> Command {
        let mut command = Command::new(tko_bin());
        command
            .args(args)
            .env("TICKETS_DIR", &self.tickets)
            .current_dir(&self.repo);
        command
    }

    fn run(&self, args: &[&str]) -> Output {
        self.command(args).output().expect("run tko")
    }

    fn success(&self, args: &[&str]) -> String {
        let output = self.run(args);
        assert!(
            output.status.success(),
            "stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).expect("UTF-8 stdout")
    }

    fn git(&self, args: &[&str]) -> Output {
        Command::new("git")
            .arg("-C")
            .arg(&self.tickets)
            .args(args)
            .output()
            .expect("run git")
    }

    fn git_success(&self, args: &[&str]) -> String {
        let output = self.git(args);
        assert_git_success("git fixture command", &output);
        String::from_utf8(output.stdout).expect("UTF-8 git output")
    }

    fn git_with_input(&self, args: &[&str], input: &str) -> Output {
        let mut child = Command::new("git")
            .arg("-C")
            .arg(&self.tickets)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn git");
        child
            .stdin
            .as_mut()
            .expect("git stdin")
            .write_all(input.as_bytes())
            .expect("write git stdin");
        child.wait_with_output().expect("wait for git")
    }

    fn head(&self) -> String {
        self.git_success(&["rev-parse", "HEAD"]).trim().to_string()
    }

    fn subject(&self) -> String {
        self.git_success(&["log", "-1", "--format=%s"])
            .trim()
            .to_string()
    }

    fn commit_paths(&self) -> Vec<String> {
        let mut paths = self
            .git_success(&["show", "--format=", "--name-only", "HEAD"])
            .lines()
            .filter(|line| !line.is_empty())
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>();
        paths.sort();
        paths
    }

    fn ticket(&self, id: &str) -> String {
        fs::read_to_string(self.tickets.join(format!("{id}.org"))).expect("read ticket")
    }

    fn assert_empty_index(&self) {
        let output = self.git(&["diff", "--cached", "--quiet", "--"]);
        assert!(output.status.success(), "ticket index is not empty");
    }
}

fn git_ok(root: &Path, args: &[&str]) {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .expect("run fixture git");
    assert_git_success("fixture git", &output);
}

fn assert_git_success(operation: &str, output: &Output) {
    assert!(
        output.status.success(),
        "{operation}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn write_ticket(root: &Path, id: &str, status: &str, links: &str) {
    fs::write(
        root.join(format!("{id}.org")),
        format!(
            ":PROPERTIES:\n:TKO_ID: {id}\n:TKO_STATUS: {status}\n:TKO_DEPS: []\n:TKO_LINKS: {links}\n:TKO_CREATED: 2026-08-30T00:00:00Z\n:TKO_TYPE: task\n:TKO_PRIORITY: 2\n:TKO_TAGS: []\n:END:\n\n* {id}\n"
        ),
    )
    .expect("write ticket");
}

fn install_failing_hook(fixture: &GitFixture, body: &str) {
    let hooks = fixture.tickets.join(".githooks");
    fs::create_dir(&hooks).expect("hooks dir");
    let hook = hooks.join("pre-commit");
    fs::write(&hook, body).expect("pre-commit hook");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&hook, fs::Permissions::from_mode(0o755)).expect("hook mode");
    }
    fixture.git_success(&["config", "core.hooksPath", ".githooks"]);
}

#[test]
fn start_commits_one_ticket_and_repeated_start_is_a_noop() {
    let fixture = GitFixture::new();

    assert_eq!(
        fixture.success(&["start", "sys-a"]),
        "Updated sys-a -> in_progress\n"
    );
    assert_eq!(fixture.subject(), "tko: start sys-a");
    assert_eq!(fixture.commit_paths(), ["sys-a.org"]);
    println!("representative start commit: {}", fixture.head());
    assert!(fixture.ticket("sys-a").contains(":TKO_STATUS: in_progress"));
    fixture.assert_empty_index();

    let committed = fixture.head();
    assert_eq!(
        fixture.success(&["start", "sys-a"]),
        "Updated sys-a -> in_progress\n"
    );
    assert_eq!(fixture.head(), committed);
    fixture.assert_empty_index();
}

#[test]
fn link_commits_both_tickets_and_preserves_unrelated_dirt() {
    let fixture = GitFixture::new();
    fs::write(
        fixture.tickets.join("sys-c.org"),
        format!("{}unrelated edit\n", fixture.ticket("sys-c")),
    )
    .expect("dirty unrelated ticket");

    assert_eq!(
        fixture.success(&["link", "sys-a", "sys-b"]),
        "Added link: sys-a <-> sys-b\n"
    );
    assert_eq!(fixture.subject(), "tko: link sys-a sys-b");
    assert_eq!(fixture.commit_paths(), ["sys-a.org", "sys-b.org"]);
    println!("representative link commit: {}", fixture.head());
    assert!(fixture.ticket("sys-a").contains(":TKO_LINKS: [sys-b]"));
    assert!(fixture.ticket("sys-b").contains(":TKO_LINKS: [sys-a]"));
    assert!(fixture.ticket("sys-c").ends_with("unrelated edit\n"));
    assert_eq!(
        fixture.git_success(&["status", "--short", "--", "sys-c.org"]),
        " M sys-c.org\n"
    );
    fixture.assert_empty_index();
}

#[test]
fn preedited_affected_ticket_is_refused_without_mutation() {
    let fixture = GitFixture::new();
    let edited = format!("{}manual edit\n", fixture.ticket("sys-a"));
    fs::write(fixture.tickets.join("sys-a.org"), &edited).expect("manual edit");
    let head = fixture.head();

    let output = fixture.run(&["start", "sys-a"]);

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("affected ticket has local modifications: sys-a.org")
    );
    assert_eq!(fixture.ticket("sys-a"), edited);
    assert_eq!(fixture.head(), head);
    fixture.assert_empty_index();
}

#[test]
fn staged_changes_and_unresolved_conflicts_are_refused() {
    let staged = GitFixture::new();
    fs::write(
        staged.tickets.join("sys-c.org"),
        format!("{}staged\n", staged.ticket("sys-c")),
    )
    .expect("staged edit");
    staged.git_success(&["add", "sys-c.org"]);
    let original_a = staged.ticket("sys-a");
    let head = staged.head();
    let output = staged.run(&["start", "sys-a"]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("has staged changes"));
    assert_eq!(staged.ticket("sys-a"), original_a);
    assert_eq!(staged.head(), head);

    let conflicted = GitFixture::new();
    let base = conflicted.git_with_input(&["hash-object", "-w", "--stdin"], "base\n");
    let ours = conflicted.git_with_input(&["hash-object", "-w", "--stdin"], "ours\n");
    let theirs = conflicted.git_with_input(&["hash-object", "-w", "--stdin"], "theirs\n");
    assert_git_success("hash base", &base);
    assert_git_success("hash ours", &ours);
    assert_git_success("hash theirs", &theirs);
    conflicted.git_success(&["update-index", "--force-remove", "--", "sys-a.org"]);
    let stages = format!(
        "100644 {} 1\tsys-a.org\n100644 {} 2\tsys-a.org\n100644 {} 3\tsys-a.org\n",
        String::from_utf8_lossy(&base.stdout).trim(),
        String::from_utf8_lossy(&ours.stdout).trim(),
        String::from_utf8_lossy(&theirs.stdout).trim(),
    );
    let update = conflicted.git_with_input(&["update-index", "--index-info"], &stages);
    assert_git_success("install conflict stages", &update);
    let original_b = conflicted.ticket("sys-b");
    let head = conflicted.head();
    let output = conflicted.run(&["start", "sys-b"]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("unresolved conflicts"));
    assert_eq!(conflicted.ticket("sys-b"), original_b);
    assert_eq!(conflicted.head(), head);
}

#[test]
fn commit_failure_restores_affected_files_and_empty_index() {
    let fixture = GitFixture::new();
    install_failing_hook(&fixture, "#!/bin/sh\nexit 1\n");
    let original = fixture.ticket("sys-a");
    let head = fixture.head();

    let output = fixture.run(&["start", "sys-a"]);

    assert!(!output.status.success());
    println!(
        "failure injection: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("restored affected files and empty index")
    );
    assert_eq!(fixture.ticket("sys-a"), original);
    assert_eq!(fixture.head(), head);
    assert_eq!(
        fixture.git_success(&["status", "--short", "--", "sys-a.org"]),
        ""
    );
    fixture.assert_empty_index();
}

#[test]
fn concurrent_mutations_cannot_interleave_transaction_state() {
    let fixture = GitFixture::new();
    install_failing_hook(
        &fixture,
        "#!/bin/sh\nset -eu\n: >\"$TKO_HOOK_READY\"\nwhile [ ! -e \"$TKO_HOOK_RELEASE\" ]; do sleep 0.01; done\n",
    );
    let ready = fixture.repo.join("hook-ready");
    let release = fixture.repo.join("hook-release");
    let mut first = fixture.command(&["start", "sys-a"]);
    first
        .env("TKO_HOOK_READY", &ready)
        .env("TKO_HOOK_RELEASE", &release)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let first = first.spawn().expect("spawn first tko");

    for _ in 0..500 {
        if ready.exists() {
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }
    assert!(ready.exists(), "first mutation did not reach commit hook");

    let second = fixture.run(&["link", "sys-b", "sys-c"]);
    assert!(!second.status.success());
    assert!(
        String::from_utf8_lossy(&second.stderr).contains("cannot acquire ticket transaction lock")
    );
    fs::write(&release, "release\n").expect("release first mutation");
    let first = first.wait_with_output().expect("wait for first tko");
    assert!(
        first.status.success(),
        "first stderr: {}",
        String::from_utf8_lossy(&first.stderr)
    );
    assert!(fixture.ticket("sys-b").contains(":TKO_LINKS: []"));
    assert!(fixture.ticket("sys-c").contains(":TKO_LINKS: []"));
    fixture.assert_empty_index();
}

#[test]
fn plain_non_git_stores_retain_start_and_link_behavior() {
    let temp = tempfile::tempdir().expect("tempdir");
    let tickets = temp.path().join(".tickets");
    fs::create_dir(&tickets).expect("tickets dir");
    write_ticket(&tickets, "sys-a", "open", "[]");
    write_ticket(&tickets, "sys-b", "open", "[]");

    let run = |args: &[&str]| {
        Command::new(tko_bin())
            .args(args)
            .env("TICKETS_DIR", &tickets)
            .current_dir(temp.path())
            .output()
            .expect("run plain tko")
    };
    let start = run(&["start", "sys-a"]);
    assert!(start.status.success());
    assert_eq!(start.stdout, b"Updated sys-a -> in_progress\n");
    let link = run(&["link", "sys-a", "sys-b"]);
    assert!(link.status.success());
    assert_eq!(link.stdout, b"Added link: sys-a <-> sys-b\n");
    assert!(
        fs::read_to_string(tickets.join("sys-a.org"))
            .expect("read sys-a")
            .contains(":TKO_LINKS: [sys-b]")
    );
    assert!(!tickets.join(".git").exists());
}
