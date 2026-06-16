// Generated from tko.org. Do not edit by hand.

use crate::storage::TicketStore;
use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

pub type Result<T> = std::result::Result<T, LintError>;

#[derive(Debug)]
pub struct LintError {
    message: String,
}

impl LintError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for LintError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.message)
    }
}

impl Error for LintError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Warning,
    Failure,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub path: PathBuf,
    pub line: usize,
    pub code: &'static str,
    pub severity: Severity,
    pub message: String,
}

impl Finding {
    pub fn format(&self) -> String {
        match self.severity {
            Severity::Warning => format!(
                "{}:{}: {} warning: {}",
                self.path.display(),
                self.line,
                self.code,
                self.message
            ),
            Severity::Failure => format!(
                "{}:{}: {} {}",
                self.path.display(),
                self.line,
                self.code,
                self.message
            ),
        }
    }
}

pub fn lint_store(store: &TicketStore) -> Result<Vec<Finding>> {
    let mut findings = Vec::new();
    for path in store
        .ticket_paths()
        .map_err(|error| LintError::new(error.to_string()))?
    {
        findings.extend(lint_path(&path)?);
    }
    Ok(findings)
}

pub fn lint_id_or_path(store: &TicketStore, id_or_path: &str) -> Result<Vec<Finding>> {
    let path = PathBuf::from(id_or_path);
    if path.exists() {
        lint_path(&path)
    } else {
        let path = store
            .resolve_id(id_or_path)
            .map_err(|error| LintError::new(error.to_string()))?;
        lint_path(&path)
    }
}

pub fn lint_path(path: &Path) -> Result<Vec<Finding>> {
    let text = fs::read_to_string(path).map_err(|error| LintError::new(error.to_string()))?;
    let mut findings = Vec::new();
    findings.extend(lint_semantic_headings(path, &text));
    findings.extend(lint_note_titles(path, &text));
    Ok(findings)
}

pub fn has_failures(findings: &[Finding]) -> bool {
    findings
        .iter()
        .any(|finding| finding.severity == Severity::Failure)
}

fn lint_semantic_headings(path: &Path, text: &str) -> Vec<Finding> {
    let mut findings = Vec::new();
    let mut seen: HashMap<String, usize> = HashMap::new();
    for (index, line) in text.lines().enumerate() {
        let Some((level, title)) = org_heading(line) else {
            continue;
        };
        let Some(canonical) = semantic_heading(title) else {
            continue;
        };
        if level != 2 {
            findings.push(Finding {
                path: path.to_path_buf(),
                line: index + 1,
                code: "L002",
                severity: Severity::Failure,
                message: format!("semantic heading must be level-2 (**): {canonical}"),
            });
        }
        if let Some(first_line) = seen.insert(canonical.to_string(), index + 1) {
            findings.push(Finding {
                path: path.to_path_buf(),
                line: index + 1,
                code: "L001",
                severity: Severity::Failure,
                message: format!(
                    "duplicate semantic heading: {canonical} (first at line {first_line})"
                ),
            });
        }
    }
    findings
}

fn lint_note_titles(path: &Path, text: &str) -> Vec<Finding> {
    let mut findings = Vec::new();
    let lines = text.lines().collect::<Vec<_>>();
    let mut index = 0usize;
    while index < lines.len() {
        let Some((2, title)) = org_heading(lines[index]) else {
            index += 1;
            continue;
        };
        if !title.trim().eq_ignore_ascii_case("Notes") {
            index += 1;
            continue;
        }
        index += 1;
        while index < lines.len() {
            match org_heading(lines[index]) {
                Some((level, _)) if level <= 2 => break,
                Some((3, note_title)) => {
                    let title = note_title_after_timestamp(note_title);
                    let length = title.chars().count();
                    if length > 72 {
                        findings.push(Finding {
                            path: path.to_path_buf(),
                            line: index + 1,
                            code: "L003",
                            severity: Severity::Failure,
                            message: format!("note title exceeds hard limit: {length} > 72"),
                        });
                    } else if length > 50 {
                        findings.push(Finding {
                            path: path.to_path_buf(),
                            line: index + 1,
                            code: "L003",
                            severity: Severity::Warning,
                            message: format!("note title exceeds target length: {length} > 50"),
                        });
                    }
                }
                _ => {}
            }
            index += 1;
        }
    }
    findings
}

fn org_heading(line: &str) -> Option<(usize, &str)> {
    let stars = line.chars().take_while(|ch| *ch == '*').count();
    if stars == 0 || !line.chars().nth(stars).is_some_and(|ch| ch == ' ') {
        return None;
    }
    Some((stars, line[stars + 1..].trim_end()))
}

fn semantic_heading(title: &str) -> Option<&'static str> {
    match title.trim().to_ascii_lowercase().as_str() {
        "description" => Some("Description"),
        "scope" => Some("Scope"),
        "design" => Some("Design"),
        "acceptance criteria" => Some("Acceptance Criteria"),
        "notes" => Some("Notes"),
        _ => None,
    }
}

fn note_title_after_timestamp(note_title: &str) -> &str {
    let trimmed = note_title.trim();
    if let Some(rest) = trimmed
        .strip_prefix('[')
        .and_then(|_| trimmed.split_once(']'))
    {
        rest.1.trim_start()
    } else {
        trimmed
    }
}
