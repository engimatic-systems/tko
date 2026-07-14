// Generated from tko.org. Do not edit by hand.

use crate::storage::{
    TicketStore, drawer_property, locate_section, org_heading, semantic_headings, type_shape,
};
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

impl fmt::Display for Finding {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.severity {
            Severity::Warning => write!(
                formatter,
                "{}:{}: {} warning: {}",
                self.path.display(),
                self.line,
                self.code,
                self.message
            ),
            Severity::Failure => write!(
                formatter,
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
    let paths = store
        .ticket_paths()
        .map_err(|error| LintError::new(error.to_string()))?;
    for path in paths {
        let ticket_findings = lint_path(&path)?;
        findings.extend(ticket_findings);
    }
    Ok(findings)
}

pub fn resolve_id_or_path(store: &TicketStore, id_or_path: &str) -> Result<PathBuf> {
    let path = Path::new(id_or_path);
    if path.exists() {
        Ok(path.to_path_buf())
    } else {
        store
            .resolve_id(id_or_path)
            .map_err(|error| LintError::new(error.to_string()))
    }
}

pub fn lint_path(path: &Path) -> Result<Vec<Finding>> {
    let text = fs::read_to_string(path).map_err(|error| LintError::new(error.to_string()))?;
    let mut findings = Vec::new();
    findings.extend(lint_semantic_headings(path, &text));
    findings.extend(lint_note_titles(path, &text));
    findings.extend(lint_type_shape(path, &text));
    Ok(findings)
}

pub fn has_failures(findings: &[Finding]) -> bool {
    findings
        .iter()
        .any(|finding| finding.severity == Severity::Failure)
}

fn lint_semantic_headings(path: &Path, text: &str) -> Vec<Finding> {
    let mut findings = Vec::new();
    let mut seen: HashMap<&'static str, usize> = HashMap::new();
    for (index, level, canonical) in semantic_heading_lines(text) {
        if level != 2 {
            findings.push(Finding {
                path: path.to_path_buf(),
                line: index + 1,
                code: "L002",
                severity: Severity::Failure,
                message: format!("semantic heading must be level-2 (**): {canonical}"),
            });
        }
        if let Some(first_line) = seen.insert(canonical, index + 1) {
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
    let mut in_notes = false;
    for (index, line) in text.lines().enumerate() {
        match org_heading(line) {
            Some((2, title)) => {
                in_notes = title.trim().eq_ignore_ascii_case("Notes");
            }
            Some((level, _)) if level < 2 => {
                in_notes = false;
            }
            Some((3, note_title)) if in_notes => {
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
    }
    findings
}

fn lint_type_shape(path: &Path, text: &str) -> Vec<Finding> {
    let ticket_type = drawer_property(text, "TKO_TYPE").unwrap_or_else(|| "task".to_string());
    let Some(shape) = type_shape(&ticket_type) else {
        return Vec::new();
    };

    let mut findings = Vec::new();
    let mut required = shape.required_at_create.to_vec();
    if drawer_property(text, "TKO_STATUS").as_deref() == Some("closed") {
        required.extend(shape.required_at_close);
    }
    for heading in required {
        match locate_section(text, heading) {
            Some((_, true)) => {}
            Some((line, false)) => findings.push(Finding {
                path: path.to_path_buf(),
                line: line + 1,
                code: "L005",
                severity: Severity::Failure,
                message: format!("required section is empty: {heading} (type {})", shape.name),
            }),
            None => findings.push(Finding {
                path: path.to_path_buf(),
                line: 1,
                code: "L005",
                severity: Severity::Failure,
                message: format!("required section missing: {heading} (type {})", shape.name),
            }),
        }
    }

    for (index, _, canonical) in semantic_heading_lines(text) {
        if !shape.allowed.contains(&canonical) {
            findings.push(Finding {
                path: path.to_path_buf(),
                line: index + 1,
                code: "L006",
                severity: Severity::Warning,
                message: format!(
                    "semantic heading does not apply to type {}: {canonical}",
                    shape.name
                ),
            });
        }
    }
    findings
}

fn semantic_heading_lines(text: &str) -> Vec<(usize, usize, &'static str)> {
    let mut lines = Vec::new();
    let mut in_notes = false;
    for (index, line) in text.lines().enumerate() {
        let Some((level, title)) = org_heading(line) else {
            continue;
        };
        if level <= 2 {
            in_notes = level == 2 && title.trim().eq_ignore_ascii_case("Notes");
        } else if in_notes {
            continue;
        }
        if let Some(canonical) = semantic_heading(title) {
            lines.push((index, level, canonical));
        }
    }
    lines
}

fn semantic_heading(title: &str) -> Option<&'static str> {
    let title = title.trim();
    semantic_headings()
        .iter()
        .find(|heading| title.eq_ignore_ascii_case(heading))
        .copied()
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
