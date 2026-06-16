// Generated from tko.org. Do not edit by hand.

use crate::storage::{TicketStore, format_list_value};
use chrono::Utc;
use std::error::Error;
use std::fmt;
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

pub type Result<T> = std::result::Result<T, WriteError>;

#[derive(Debug)]
pub struct WriteError {
    message: String,
}

impl WriteError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for WriteError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.message)
    }
}

impl Error for WriteError {}

#[derive(Debug, Clone)]
pub struct CreateTicket {
    pub title: String,
    pub description: Option<String>,
    pub scope: Option<String>,
    pub design: Option<String>,
    pub acceptance: Option<String>,
    pub ticket_type: String,
    pub priority: u8,
    pub assignee: Option<String>,
    pub external_ref: Option<String>,
    pub parent: Option<String>,
    pub tags: Vec<String>,
}

pub fn create(store: &TicketStore, cwd: &Path, input: CreateTicket) -> Result<String> {
    let title = input.title.trim();
    if title.is_empty() {
        return Err(WriteError::new("ticket title is required"));
    }
    validate_type(&input.ticket_type)?;
    validate_priority(input.priority)?;

    let id = unique_id(store, cwd)?;
    let parent = input
        .parent
        .as_ref()
        .map(|parent| resolved_id(store, parent))
        .transpose()?;

    let mut text = String::new();
    text.push_str(":PROPERTIES:\n");
    push_property(&mut text, "TKO_ID", &id);
    push_property(&mut text, "TKO_STATUS", "open");
    push_property(&mut text, "TKO_DEPS", "[]");
    push_property(&mut text, "TKO_LINKS", "[]");
    push_property(
        &mut text,
        "TKO_CREATED",
        &Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
    );
    push_property(&mut text, "TKO_TYPE", &input.ticket_type);
    push_property(&mut text, "TKO_PRIORITY", &input.priority.to_string());
    if let Some(assignee) = input.assignee.filter(|value| !value.trim().is_empty()) {
        push_property(&mut text, "TKO_ASSIGNEE", assignee.trim());
    }
    if let Some(external_ref) = input.external_ref.filter(|value| !value.trim().is_empty()) {
        push_property(&mut text, "TKO_EXTERNAL_REF", external_ref.trim());
    }
    if let Some(parent) = parent {
        push_property(&mut text, "TKO_PARENT", &parent);
    }
    if !input.tags.is_empty() {
        push_property(&mut text, "TKO_TAGS", &format_list_value(&input.tags));
    }
    text.push_str(":END:\n\n");
    text.push_str(&format!("* {title}\n"));
    push_section(&mut text, "Description", input.description.as_deref());
    push_section(&mut text, "Scope", input.scope.as_deref());
    push_section(&mut text, "Design", input.design.as_deref());
    push_section(
        &mut text,
        "Acceptance Criteria",
        input.acceptance.as_deref(),
    );

    fs::write(store.tickets_dir().join(format!("{id}.org")), text)
        .map_err(|error| WriteError::new(error.to_string()))?;
    Ok(id)
}

pub fn set_status(store: &TicketStore, id: &str, status: &str) -> Result<String> {
    validate_status(status)?;
    let resolved = resolved_id(store, id)?;
    store
        .set_property(&resolved, "TKO_STATUS", status)
        .map_err(|error| WriteError::new(error.to_string()))?;
    Ok(format!("Updated {resolved} -> {status}\n"))
}

pub fn add_dependency(store: &TicketStore, id: &str, dep_id: &str) -> Result<String> {
    mutate_relation(
        store,
        id,
        dep_id,
        "TKO_DEPS",
        RelationKind::Dependency,
        Mutation::Add,
    )
}

pub fn remove_dependency(store: &TicketStore, id: &str, dep_id: &str) -> Result<String> {
    mutate_relation(
        store,
        id,
        dep_id,
        "TKO_DEPS",
        RelationKind::Dependency,
        Mutation::Remove,
    )
}

pub fn add_link(store: &TicketStore, id: &str, target_id: &str) -> Result<String> {
    mutate_link(store, id, target_id, Mutation::Add)
}

pub fn remove_link(store: &TicketStore, id: &str, target_id: &str) -> Result<String> {
    mutate_link(store, id, target_id, Mutation::Remove)
}

pub fn add_tags(store: &TicketStore, id: &str, tags: &[String]) -> Result<String> {
    mutate_tags(store, id, tags, Mutation::Add)
}

pub fn remove_tags(store: &TicketStore, id: &str, tags: &[String]) -> Result<String> {
    mutate_tags(store, id, tags, Mutation::Remove)
}

pub fn add_note(store: &TicketStore, id: &str, title: &str, body: Option<&str>) -> Result<String> {
    let resolved = resolved_id(store, id)?;
    let path = store
        .resolve_id(&resolved)
        .map_err(|error| WriteError::new(error.to_string()))?;
    let document = fs::read_to_string(&path).map_err(|error| WriteError::new(error.to_string()))?;
    let title = title.trim();
    if title.is_empty() {
        return Err(WriteError::new("note title is required"));
    }
    if title.contains("\\n") {
        return Err(WriteError::new(
            "note title must not contain escaped newlines",
        ));
    }
    if title.contains('\n') {
        return Err(WriteError::new("note title must be one line"));
    }
    if title.chars().count() > 72 {
        return Err(WriteError::new("note title exceeds 72 characters"));
    }
    let body = body.map(expand_escaped_newlines).unwrap_or_default();

    let timestamp = Utc::now().format("[%Y-%m-%d %a %H:%MZ]").to_string();
    let mut note = format!("*** {timestamp} {title}\n");
    if !body.is_empty() {
        note.push_str(&body);
        if !note.ends_with('\n') {
            note.push('\n');
        }
    }

    let updated = append_note(document, &note);
    fs::write(path, updated).map_err(|error| WriteError::new(error.to_string()))?;
    Ok(format!("Note added to {resolved}\n"))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mutation {
    Add,
    Remove,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RelationKind {
    Dependency,
    Link,
}

fn mutate_relation(
    store: &TicketStore,
    id: &str,
    target_id: &str,
    property: &str,
    kind: RelationKind,
    mutation: Mutation,
) -> Result<String> {
    let resolved = resolved_id(store, id)?;
    let target = resolved_id(store, target_id)?;
    if resolved == target {
        return Err(WriteError::new("self-relation is not allowed"));
    }
    let mut ticket = store
        .load(&resolved)
        .map_err(|error| WriteError::new(error.to_string()))?;
    let values = match property {
        "TKO_DEPS" => &mut ticket.properties.deps,
        "TKO_LINKS" => &mut ticket.properties.links,
        _ => return Err(WriteError::new("unknown relation property")),
    };
    let changed = mutate_values(values, &target, mutation);
    store
        .set_property(&resolved, property, &format_list_value(values))
        .map_err(|error| WriteError::new(error.to_string()))?;
    Ok(relation_message(
        kind, mutation, changed, &resolved, &target,
    ))
}

fn mutate_link(
    store: &TicketStore,
    id: &str,
    target_id: &str,
    mutation: Mutation,
) -> Result<String> {
    let resolved = resolved_id(store, id)?;
    let target = resolved_id(store, target_id)?;
    if resolved == target {
        return Err(WriteError::new("self-link is not allowed"));
    }
    let mut left = store
        .load(&resolved)
        .map_err(|error| WriteError::new(error.to_string()))?;
    let mut right = store
        .load(&target)
        .map_err(|error| WriteError::new(error.to_string()))?;
    let left_changed = mutate_values(&mut left.properties.links, &target, mutation);
    let right_changed = mutate_values(&mut right.properties.links, &resolved, mutation);
    store
        .set_property(
            &resolved,
            "TKO_LINKS",
            &format_list_value(&left.properties.links),
        )
        .map_err(|error| WriteError::new(error.to_string()))?;
    store
        .set_property(
            &target,
            "TKO_LINKS",
            &format_list_value(&right.properties.links),
        )
        .map_err(|error| WriteError::new(error.to_string()))?;
    Ok(relation_message(
        RelationKind::Link,
        mutation,
        left_changed || right_changed,
        &resolved,
        &target,
    ))
}

fn mutate_tags(
    store: &TicketStore,
    id: &str,
    tags: &[String],
    mutation: Mutation,
) -> Result<String> {
    let resolved = resolved_id(store, id)?;
    let mut ticket = store
        .load(&resolved)
        .map_err(|error| WriteError::new(error.to_string()))?;
    let mut changed = Vec::new();
    let mut unchanged = Vec::new();
    for tag in tags {
        if mutate_values(&mut ticket.properties.tags, tag, mutation) {
            changed.push(tag.clone());
        } else {
            unchanged.push(tag.clone());
        }
    }
    store
        .set_property(
            &resolved,
            "TKO_TAGS",
            &format_list_value(&ticket.properties.tags),
        )
        .map_err(|error| WriteError::new(error.to_string()))?;

    let has_changed = !changed.is_empty();
    let tags = if has_changed { changed } else { unchanged };
    let label = tags.join(" ");
    let message = match mutation {
        Mutation::Add if has_changed => format!("Added tag(s) to {resolved}: {label}\n"),
        Mutation::Add => format!("Tag(s) already present on {resolved}: {label}\n"),
        Mutation::Remove if has_changed => {
            format!("Removed tag(s) from {resolved}: {label}\n")
        }
        Mutation::Remove => format!("Tag(s) not present on {resolved}: {label}\n"),
    };
    Ok(message)
}

fn mutate_values(values: &mut Vec<String>, value: &str, mutation: Mutation) -> bool {
    match mutation {
        Mutation::Add => {
            if values.iter().any(|item| item == value) {
                false
            } else {
                values.push(value.to_string());
                true
            }
        }
        Mutation::Remove => {
            let original_len = values.len();
            values.retain(|item| item != value);
            values.len() != original_len
        }
    }
}

fn relation_message(
    kind: RelationKind,
    mutation: Mutation,
    changed: bool,
    id: &str,
    target: &str,
) -> String {
    match (kind, mutation, changed) {
        (RelationKind::Dependency, Mutation::Add, true) => {
            format!("Added dependency: {id} -> {target}\n")
        }
        (RelationKind::Dependency, Mutation::Add, false) => {
            format!("Dependency already exists: {id} -> {target}\n")
        }
        (RelationKind::Dependency, Mutation::Remove, true) => {
            format!("Removed dependency: {id} -/-> {target}\n")
        }
        (RelationKind::Dependency, Mutation::Remove, false) => {
            format!("Dependency not present: {id} -/-> {target}\n")
        }
        (RelationKind::Link, Mutation::Add, true) => format!("Added link: {id} <-> {target}\n"),
        (RelationKind::Link, Mutation::Add, false) => {
            format!("Link already exists: {id} <-> {target}\n")
        }
        (RelationKind::Link, Mutation::Remove, true) => {
            format!("Removed link: {id} <-> {target}\n")
        }
        (RelationKind::Link, Mutation::Remove, false) => {
            format!("Link not present: {id} <-> {target}\n")
        }
    }
}

fn append_note(mut document: String, note: &str) -> String {
    let has_notes = document
        .lines()
        .any(|line| line.trim_end_matches('\r').eq_ignore_ascii_case("** Notes"));
    if !document.ends_with('\n') && !document.is_empty() {
        document.push('\n');
    }
    if !has_notes {
        if !document.is_empty() {
            document.push('\n');
        }
        document.push_str("** Notes\n");
    }
    document.push_str(note);
    document
}

fn push_property(text: &mut String, key: &str, value: &str) {
    text.push_str(&format!(":{key}: {value}\n"));
}

fn push_section(text: &mut String, heading: &str, body: Option<&str>) {
    let Some(body) = body.map(expand_escaped_newlines) else {
        return;
    };
    let body = body.trim();
    if body.is_empty() {
        return;
    }
    text.push_str(&format!("\n** {heading}\n\n{body}\n"));
}

fn expand_escaped_newlines(value: &str) -> String {
    value.replace("\\n", "\n")
}

fn resolved_id(store: &TicketStore, id: &str) -> Result<String> {
    let path = store
        .resolve_id(id)
        .map_err(|error| WriteError::new(error.to_string()))?;
    file_stem(&path)
}

fn unique_id(store: &TicketStore, cwd: &Path) -> Result<String> {
    for _ in 0..128 {
        let candidate = format!("{}-{}", id_prefix(cwd), id_suffix());
        if !store
            .tickets_dir()
            .join(format!("{candidate}.org"))
            .exists()
        {
            return Ok(candidate);
        }
    }
    Err(WriteError::new("failed to generate unique ticket id"))
}

fn id_prefix(cwd: &Path) -> String {
    let name = cwd
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("tko");
    let words = name
        .split(['-', '_'])
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>();
    let initials = words
        .iter()
        .filter_map(|word| word.chars().next())
        .collect::<String>();
    if initials.chars().count() >= 2 {
        initials
    } else {
        name.chars().take(3).collect()
    }
}

fn id_suffix() -> String {
    const ALPHABET: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";
    let mut value = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let mut suffix = String::new();
    for _ in 0..4 {
        let index = (value % ALPHABET.len() as u128) as usize;
        suffix.push(ALPHABET[index] as char);
        value /= ALPHABET.len() as u128;
    }
    suffix
}

fn validate_status(status: &str) -> Result<()> {
    if matches!(status, "open" | "in_progress" | "blocked" | "closed") {
        Ok(())
    } else {
        Err(WriteError::new(format!("invalid status: {status}")))
    }
}

fn validate_type(ticket_type: &str) -> Result<()> {
    if matches!(ticket_type, "bug" | "feature" | "task" | "epic" | "chore") {
        Ok(())
    } else {
        Err(WriteError::new(format!("invalid type: {ticket_type}")))
    }
}

fn validate_priority(priority: u8) -> Result<()> {
    if priority <= 4 {
        Ok(())
    } else {
        Err(WriteError::new(format!("invalid priority: {priority}")))
    }
}

fn file_stem(path: &Path) -> Result<String> {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .map(ToOwned::to_owned)
        .ok_or_else(|| WriteError::new(format!("ticket path has no file stem: {}", path.display())))
}
