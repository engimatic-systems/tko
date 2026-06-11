// Generated from tko.org. Do not edit by hand.

use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub type Result<T> = std::result::Result<T, StorageError>;

#[derive(Debug)]
pub enum StorageError {
    Io(io::Error),
    TicketsDirNotFound(PathBuf),
    MissingFileStem(PathBuf),
    TicketNotFound(String),
    AmbiguousTicketId { query: String, matches: Vec<String> },
    InvalidList(String),
    InvalidProperty(String),
}

impl fmt::Display for StorageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StorageError::Io(error) => write!(formatter, "{error}"),
            StorageError::TicketsDirNotFound(start) => {
                write!(
                    formatter,
                    "tickets directory not found from {}",
                    start.display()
                )
            }
            StorageError::MissingFileStem(path) => {
                write!(
                    formatter,
                    "ticket path has no file stem: {}",
                    path.display()
                )
            }
            StorageError::TicketNotFound(id) => write!(formatter, "ticket not found: {id}"),
            StorageError::AmbiguousTicketId { query, matches } => {
                write!(
                    formatter,
                    "ambiguous ticket id {query}: {}",
                    matches.join(", ")
                )
            }
            StorageError::InvalidList(value) => write!(formatter, "invalid list property: {value}"),
            StorageError::InvalidProperty(message) => {
                write!(formatter, "invalid property: {message}")
            }
        }
    }
}

impl Error for StorageError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            StorageError::Io(error) => Some(error),
            _ => None,
        }
    }
}

impl From<io::Error> for StorageError {
    fn from(error: io::Error) -> Self {
        StorageError::Io(error)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ticket {
    pub path: PathBuf,
    pub id: String,
    pub title: String,
    pub properties: TicketProperties,
    pub body: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TicketProperties {
    pub status: String,
    pub deps: Vec<String>,
    pub links: Vec<String>,
    pub created: Option<String>,
    pub ticket_type: String,
    pub priority: u8,
    pub assignee: Option<String>,
    pub external_ref: Option<String>,
    pub parent: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct TicketStore {
    tickets_dir: PathBuf,
}

impl TicketStore {
    pub fn new(tickets_dir: impl Into<PathBuf>) -> Self {
        Self {
            tickets_dir: tickets_dir.into(),
        }
    }

    pub fn discover_from(
        start: &Path,
        tickets_dir_env: Option<&Path>,
        create_if_missing: bool,
    ) -> Result<Self> {
        let tickets_dir = discover_tickets_dir(start, tickets_dir_env, create_if_missing)?;
        if create_if_missing {
            fs::create_dir_all(&tickets_dir)?;
        }
        Ok(Self::new(tickets_dir))
    }

    pub fn tickets_dir(&self) -> &Path {
        &self.tickets_dir
    }

    pub fn ticket_paths(&self) -> Result<Vec<PathBuf>> {
        let mut paths = Vec::new();
        for entry in fs::read_dir(&self.tickets_dir)? {
            let path = entry?.path();
            if path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with('.'))
            {
                continue;
            }
            if path.is_file() && path.extension().is_some_and(|extension| extension == "org") {
                paths.push(path);
            }
        }
        paths.sort();
        Ok(paths)
    }

    pub fn resolve_id(&self, id: &str) -> Result<PathBuf> {
        let exact = self.tickets_dir.join(format!("{id}.org"));
        if exact.exists() {
            return Ok(exact);
        }

        let mut matches = Vec::new();
        for path in self.ticket_paths()? {
            let stem = file_stem(&path)?;
            if stem.contains(id) {
                matches.push(stem);
            }
        }

        match matches.len() {
            0 => Err(StorageError::TicketNotFound(id.to_string())),
            1 => Ok(self.tickets_dir.join(format!("{}.org", matches[0]))),
            _ => Err(StorageError::AmbiguousTicketId {
                query: id.to_string(),
                matches,
            }),
        }
    }

    pub fn load(&self, id: &str) -> Result<Ticket> {
        let path = self.resolve_id(id)?;
        load_ticket(&path)
    }

    pub fn set_property(&self, id: &str, key: &str, value: &str) -> Result<()> {
        let path = self.resolve_id(id)?;
        set_property(&path, key, value)
    }
}

pub fn discover_tickets_dir(
    start: &Path,
    tickets_dir_env: Option<&Path>,
    create_if_missing: bool,
) -> Result<PathBuf> {
    if let Some(path) = tickets_dir_env {
        return Ok(path.to_path_buf());
    }

    for ancestor in start.ancestors() {
        let candidate = ancestor.join(".tickets");
        if candidate.is_dir() {
            return Ok(candidate);
        }
    }

    if create_if_missing {
        return Ok(start.join(".tickets"));
    }

    Err(StorageError::TicketsDirNotFound(start.to_path_buf()))
}

pub fn load_ticket(path: &Path) -> Result<Ticket> {
    let text = fs::read_to_string(path)?;
    parse_ticket(path, &text)
}

pub fn parse_ticket(path: &Path, text: &str) -> Result<Ticket> {
    let stem = file_stem(path)?;
    let document = OrgDocument::parse(text);
    let properties = document.tko_properties();
    let body = document.body().to_string();
    let id = optional_property(&properties, "TKO_ID").unwrap_or_else(|| stem.clone());

    Ok(Ticket {
        path: path.to_path_buf(),
        id,
        title: first_title(&body),
        properties: TicketProperties {
            status: property_or_default(&properties, "TKO_STATUS", "open"),
            deps: list_property(&properties, "TKO_DEPS")?,
            links: list_property(&properties, "TKO_LINKS")?,
            created: optional_property(&properties, "TKO_CREATED"),
            ticket_type: property_or_default(&properties, "TKO_TYPE", "task"),
            priority: priority_property(&properties)?,
            assignee: optional_property(&properties, "TKO_ASSIGNEE"),
            external_ref: optional_property(&properties, "TKO_EXTERNAL_REF"),
            parent: optional_property(&properties, "TKO_PARENT"),
            tags: list_property(&properties, "TKO_TAGS")?,
        },
        body,
    })
}

pub fn parse_list_value(value: &str) -> Result<Vec<String>> {
    let trimmed = value.trim();
    let Some(inner) = trimmed
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
    else {
        return Err(StorageError::InvalidList(value.to_string()));
    };

    if inner.trim().is_empty() {
        return Ok(Vec::new());
    }

    Ok(inner
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(ToOwned::to_owned)
        .collect())
}

pub fn format_list_value(items: &[String]) -> String {
    if items.is_empty() {
        "[]".to_string()
    } else {
        format!("[{}]", items.join(", "))
    }
}

pub fn set_property(path: &Path, key: &str, value: &str) -> Result<()> {
    if !key.starts_with("TKO_") {
        return Err(StorageError::InvalidProperty(format!(
            "normal writes require TKO_* key, got {key}"
        )));
    }

    let text = fs::read_to_string(path)?;
    let mut lines = split_lines(&text);
    let document = OrgDocument::parse(&text);

    if let Some(drawer) = document.drawer {
        if let Some(entry) = drawer.entries.iter().find(|entry| entry.key == key) {
            let ending = line_ending(&lines[entry.line_index]);
            lines[entry.line_index] = format!(":{key}: {value}{ending}");
        } else {
            lines.insert(drawer.end_line, format!(":{key}: {value}\n"));
        }
        fs::write(path, lines.concat())?;
    } else {
        let mut updated = format!(":PROPERTIES:\n:{key}: {value}\n:END:\n\n");
        updated.push_str(&text);
        fs::write(path, updated)?;
    }

    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationReport {
    pub path: PathBuf,
    pub actions: Vec<MigrationAction>,
    pub conflicts: Vec<MigrationConflict>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MigrationAction {
    Rename {
        legacy_key: String,
        canonical_key: String,
        value: String,
    },
    RemoveLegacy {
        legacy_key: String,
        canonical_key: String,
        value: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationConflict {
    pub legacy_key: String,
    pub canonical_key: String,
    pub legacy_value: String,
    pub canonical_value: String,
}

pub fn migrate_legacy_properties(path: &Path, apply: bool) -> Result<MigrationReport> {
    let text = fs::read_to_string(path)?;
    let document = OrgDocument::parse(&text);
    let mut report = MigrationReport {
        path: path.to_path_buf(),
        actions: Vec::new(),
        conflicts: Vec::new(),
    };

    let Some(drawer) = document.drawer else {
        return Ok(report);
    };

    let mut canonical_values = HashMap::new();
    for entry in &drawer.entries {
        if entry.key.starts_with("TKO_") {
            canonical_values.insert(entry.key.as_str(), entry.value.as_str());
        }
    }

    let mut lines = split_lines(&text);
    let mut remove_lines = Vec::new();

    for entry in &drawer.entries {
        let Some(canonical_key) = canonical_key_for_legacy(&entry.key) else {
            continue;
        };

        match canonical_values.get(canonical_key) {
            None => {
                report.actions.push(MigrationAction::Rename {
                    legacy_key: entry.key.clone(),
                    canonical_key: canonical_key.to_string(),
                    value: entry.value.clone(),
                });
                if apply {
                    let ending = line_ending(&lines[entry.line_index]);
                    lines[entry.line_index] = format!(":{canonical_key}: {}{ending}", entry.value);
                }
            }
            Some(canonical_value) if *canonical_value == entry.value => {
                report.actions.push(MigrationAction::RemoveLegacy {
                    legacy_key: entry.key.clone(),
                    canonical_key: canonical_key.to_string(),
                    value: entry.value.clone(),
                });
                if apply {
                    remove_lines.push(entry.line_index);
                }
            }
            Some(canonical_value) => {
                report.conflicts.push(MigrationConflict {
                    legacy_key: entry.key.clone(),
                    canonical_key: canonical_key.to_string(),
                    legacy_value: entry.value.clone(),
                    canonical_value: (*canonical_value).to_string(),
                });
            }
        }
    }

    if apply && (!report.actions.is_empty() || !remove_lines.is_empty()) {
        remove_lines.sort_unstable();
        remove_lines.dedup();
        for line_index in remove_lines.into_iter().rev() {
            lines.remove(line_index);
        }
        fs::write(path, lines.concat())?;
    }

    Ok(report)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LintFinding {
    pub path: PathBuf,
    pub line: usize,
    pub code: &'static str,
    pub message: String,
}

pub fn lint_legacy_property_keys(path: &Path) -> Result<Vec<LintFinding>> {
    let text = fs::read_to_string(path)?;
    let document = OrgDocument::parse(&text);
    let Some(drawer) = document.drawer else {
        return Ok(Vec::new());
    };

    Ok(drawer
        .entries
        .iter()
        .filter(|entry| canonical_key_for_legacy(&entry.key).is_some())
        .map(|entry| LintFinding {
            path: path.to_path_buf(),
            line: entry.line_index + 1,
            code: "L004",
            message: format!("legacy {} property key remains after migration", entry.key),
        })
        .collect())
}

#[derive(Debug, Clone)]
struct OrgDocument<'a> {
    text: &'a str,
    drawer: Option<PropertyDrawer>,
}

impl<'a> OrgDocument<'a> {
    fn parse(text: &'a str) -> Self {
        let lines = split_lines(text);
        let drawer = parse_property_drawer(&lines);
        Self { text, drawer }
    }

    fn body(&self) -> &str {
        if let Some(drawer) = &self.drawer {
            let mut offset = 0;
            for (line_index, line) in split_lines(self.text).iter().enumerate() {
                offset += line.len();
                if line_index == drawer.end_line {
                    break;
                }
            }
            self.text[offset..].trim_start_matches(['\r', '\n'])
        } else {
            self.text
        }
    }

    fn tko_properties(&self) -> HashMap<String, String> {
        let mut properties = HashMap::new();
        if let Some(drawer) = &self.drawer {
            for entry in &drawer.entries {
                if entry.key.starts_with("TKO_") {
                    properties.insert(entry.key.clone(), entry.value.clone());
                }
            }
        }
        properties
    }
}

#[derive(Debug, Clone)]
struct PropertyDrawer {
    end_line: usize,
    entries: Vec<PropertyEntry>,
}

#[derive(Debug, Clone)]
struct PropertyEntry {
    key: String,
    value: String,
    line_index: usize,
}

fn parse_property_drawer(lines: &[String]) -> Option<PropertyDrawer> {
    if lines
        .first()
        .map(|line| trim_line_ending(line) != ":PROPERTIES:")
        .unwrap_or(true)
    {
        return None;
    }

    let end_line = lines
        .iter()
        .position(|line| trim_line_ending(line) == ":END:")?;
    let entries = lines[1..end_line]
        .iter()
        .enumerate()
        .filter_map(|(offset, line)| {
            parse_property_line(line).map(|(key, value)| PropertyEntry {
                key,
                value,
                line_index: offset + 1,
            })
        })
        .collect();

    Some(PropertyDrawer { end_line, entries })
}

fn parse_property_line(line: &str) -> Option<(String, String)> {
    let line = trim_line_ending(line);
    let rest = line.strip_prefix(':')?;
    let (key, value) = rest.split_once(':')?;
    let value = value.strip_prefix(' ').unwrap_or(value);
    Some((key.to_string(), value.to_string()))
}

fn property_or_default(properties: &HashMap<String, String>, key: &str, default: &str) -> String {
    optional_property(properties, key).unwrap_or_else(|| default.to_string())
}

fn optional_property(properties: &HashMap<String, String>, key: &str) -> Option<String> {
    properties
        .get(key)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn list_property(properties: &HashMap<String, String>, key: &str) -> Result<Vec<String>> {
    match optional_property(properties, key) {
        Some(value) => parse_list_value(&value),
        None => Ok(Vec::new()),
    }
}

fn priority_property(properties: &HashMap<String, String>) -> Result<u8> {
    match optional_property(properties, "TKO_PRIORITY") {
        Some(value) => value
            .parse()
            .map_err(|_| StorageError::InvalidProperty(format!("TKO_PRIORITY={value}"))),
        None => Ok(2),
    }
}

fn first_title(body: &str) -> String {
    body.lines()
        .find_map(|line| line.strip_prefix("* "))
        .map(str::trim)
        .filter(|title| !title.is_empty())
        .unwrap_or("Untitled")
        .to_string()
}

fn file_stem(path: &Path) -> Result<String> {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .map(ToOwned::to_owned)
        .ok_or_else(|| StorageError::MissingFileStem(path.to_path_buf()))
}

fn split_lines(text: &str) -> Vec<String> {
    if text.is_empty() {
        Vec::new()
    } else {
        text.split_inclusive('\n').map(ToOwned::to_owned).collect()
    }
}

fn trim_line_ending(line: &str) -> &str {
    line.trim_end_matches(['\r', '\n'])
}

fn line_ending(line: &str) -> &'static str {
    if line.ends_with("\r\n") {
        "\r\n"
    } else if line.ends_with('\n') {
        "\n"
    } else {
        ""
    }
}

fn canonical_key_for_legacy(key: &str) -> Option<&'static str> {
    match key {
        "TK_ID" => Some("TKO_ID"),
        "TK_STATUS" => Some("TKO_STATUS"),
        "TK_DEPS" => Some("TKO_DEPS"),
        "TK_LINKS" => Some("TKO_LINKS"),
        "TK_CREATED" => Some("TKO_CREATED"),
        "TK_TYPE" => Some("TKO_TYPE"),
        "TK_PRIORITY" => Some("TKO_PRIORITY"),
        "TK_ASSIGNEE" => Some("TKO_ASSIGNEE"),
        "TK_EXTERNAL_REF" => Some("TKO_EXTERNAL_REF"),
        "TK_PARENT" => Some("TKO_PARENT"),
        "TK_TAGS" => Some("TKO_TAGS"),
        _ => None,
    }
}
