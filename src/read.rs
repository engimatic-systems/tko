// Generated from tko.org. Do not edit by hand.

use crate::query::Predicate;
use crate::storage::{Ticket, TicketStore, format_list_value};
use serde_json::json;
use std::collections::HashMap;
use std::error::Error;
use std::fmt;

pub type Result<T> = std::result::Result<T, ReadError>;

#[derive(Debug)]
pub struct ReadError {
    message: String,
}

impl ReadError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for ReadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.message)
    }
}

impl Error for ReadError {}

#[derive(Debug, Clone, Default)]
pub struct Filters {
    pub status: Option<String>,
    pub assignee: Option<String>,
    pub tag: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputMode {
    Id,
    Summary,
    Json,
}

pub fn show(store: &TicketStore, id: &str, full: bool) -> Result<String> {
    let ticket = store
        .load(id)
        .map_err(|error| ReadError::new(error.to_string()))?;
    let mut output = metadata_header(&ticket);
    output.push('\n');
    if full {
        output.push_str(&ticket.body);
        if !output.ends_with('\n') {
            output.push('\n');
        }
    } else {
        for heading in ticket.body.lines().filter(|line| line.starts_with('*')) {
            output.push_str(heading);
            output.push('\n');
        }
    }
    Ok(output)
}

pub fn list(store: &TicketStore, filters: &Filters, output: OutputMode) -> Result<String> {
    validate_filters(filters)?;
    let mut lines = Vec::new();
    for ticket in load_all(store)? {
        if !matches_filters(&ticket, filters) {
            continue;
        }
        lines.push(render_ticket(&ticket, output));
    }
    Ok(finish_lines(lines))
}

pub fn ready(store: &TicketStore, filters: &Filters, output: OutputMode) -> Result<String> {
    let tickets = load_all(store)?;
    let by_id = ticket_map(&tickets);
    let mut ready = active_tickets(tickets, filters)?
        .into_iter()
        .filter(|ticket| unresolved_deps(ticket, &by_id).is_empty())
        .collect::<Vec<_>>();
    ready.sort_by_key(|ticket| ticket.id.clone());
    Ok(finish_lines(
        ready
            .into_iter()
            .map(|ticket| render_ticket(&ticket, output))
            .collect(),
    ))
}

pub fn blocked(store: &TicketStore, filters: &Filters, output: OutputMode) -> Result<String> {
    let tickets = load_all(store)?;
    let by_id = ticket_map(&tickets);
    let mut blocked = active_tickets(tickets, filters)?
        .into_iter()
        .filter_map(|ticket| {
            let unresolved = unresolved_deps(&ticket, &by_id);
            if unresolved.is_empty() {
                None
            } else {
                Some((ticket, unresolved))
            }
        })
        .collect::<Vec<_>>();
    blocked.sort_by_key(|(ticket, _)| ticket.id.clone());
    Ok(finish_lines(
        blocked
            .into_iter()
            .map(|(ticket, _)| render_ticket(&ticket, output))
            .collect(),
    ))
}

pub fn query(store: &TicketStore, predicate: Option<&str>, output: OutputMode) -> Result<String> {
    let predicate = predicate
        .filter(|predicate| !predicate.trim().is_empty())
        .map(Predicate::parse)
        .transpose()
        .map_err(|error| ReadError::new(error.to_string()))?;
    let mut lines = Vec::new();
    for ticket in load_all(store)? {
        if predicate
            .as_ref()
            .map(|predicate| predicate.matches(&ticket))
            .transpose()
            .map_err(|error| ReadError::new(error.to_string()))?
            .unwrap_or(true)
        {
            lines.push(render_ticket(&ticket, output));
        }
    }
    Ok(finish_lines(lines))
}

fn render_ticket(ticket: &Ticket, output: OutputMode) -> String {
    match output {
        OutputMode::Id => ticket.id.clone(),
        OutputMode::Summary => summary_line(ticket),
        OutputMode::Json => query_json(ticket).to_string(),
    }
}

fn summary_line(ticket: &Ticket) -> String {
    format!(
        "{:<8} [{}] - {} <- {}",
        ticket.id,
        ticket.properties.status,
        ticket.title,
        format_list_value(&ticket.properties.deps)
    )
}

fn metadata_header(ticket: &Ticket) -> String {
    let mut lines = vec![
        format!("id: {}", ticket.id),
        format!("status: {}", ticket.properties.status),
        format!("deps: {}", format_list_value(&ticket.properties.deps)),
        format!("links: {}", format_list_value(&ticket.properties.links)),
        format!(
            "created: {}",
            ticket.properties.created.clone().unwrap_or_default()
        ),
        format!("type: {}", ticket.properties.ticket_type),
        format!("priority: {}", ticket.properties.priority),
    ];
    if let Some(assignee) = &ticket.properties.assignee {
        lines.push(format!("assignee: {assignee}"));
    }
    if let Some(external_ref) = &ticket.properties.external_ref {
        lines.push(format!("external-ref: {external_ref}"));
    }
    if let Some(parent) = &ticket.properties.parent {
        lines.push(format!("parent: {parent}"));
    }
    lines.push(format!(
        "tags: {}",
        format_list_value(&ticket.properties.tags)
    ));
    finish_lines(lines)
}

fn query_json(ticket: &Ticket) -> serde_json::Value {
    let mut value = json!({
        "id": ticket.id,
        "status": ticket.properties.status,
        "deps": ticket.properties.deps,
        "links": ticket.properties.links,
        "created": ticket.properties.created.clone().unwrap_or_default(),
        "type": ticket.properties.ticket_type,
        "priority": ticket.properties.priority,
        "tags": ticket.properties.tags,
    });
    let object = value.as_object_mut().expect("query object");
    if let Some(assignee) = &ticket.properties.assignee {
        object.insert("assignee".to_string(), json!(assignee));
    }
    if let Some(external_ref) = &ticket.properties.external_ref {
        object.insert("external-ref".to_string(), json!(external_ref));
    }
    if let Some(parent) = &ticket.properties.parent {
        object.insert("parent".to_string(), json!(parent));
    }
    value
}

fn load_all(store: &TicketStore) -> Result<Vec<Ticket>> {
    store
        .ticket_paths()
        .map_err(|error| ReadError::new(error.to_string()))?
        .iter()
        .map(|path| {
            crate::storage::load_ticket(path).map_err(|error| ReadError::new(error.to_string()))
        })
        .collect()
}

fn ticket_map(tickets: &[Ticket]) -> HashMap<String, Ticket> {
    tickets
        .iter()
        .map(|ticket| (ticket.id.clone(), ticket.clone()))
        .collect()
}

fn active_tickets(tickets: Vec<Ticket>, filters: &Filters) -> Result<Vec<Ticket>> {
    validate_filters(filters)?;
    Ok(tickets
        .into_iter()
        .filter(|ticket| matches!(ticket.properties.status.as_str(), "open" | "in_progress"))
        .filter(|ticket| matches_filters(ticket, filters))
        .collect())
}

fn unresolved_deps(ticket: &Ticket, by_id: &HashMap<String, Ticket>) -> Vec<String> {
    ticket
        .properties
        .deps
        .iter()
        .filter(|dep| {
            by_id
                .get(*dep)
                .map(|ticket| ticket.properties.status.as_str() != "closed")
                .unwrap_or(true)
        })
        .cloned()
        .collect()
}

fn matches_filters(ticket: &Ticket, filters: &Filters) -> bool {
    if filters
        .status
        .as_ref()
        .is_some_and(|status| ticket.properties.status != *status)
    {
        return false;
    }
    if filters
        .assignee
        .as_ref()
        .is_some_and(|assignee| ticket.properties.assignee.as_deref() != Some(assignee.as_str()))
    {
        return false;
    }
    if filters
        .tag
        .as_ref()
        .is_some_and(|tag| !ticket.properties.tags.contains(tag))
    {
        return false;
    }
    true
}

fn validate_filters(filters: &Filters) -> Result<()> {
    if let Some(status) = &filters.status {
        if !matches!(
            status.as_str(),
            "open" | "in_progress" | "blocked" | "closed"
        ) {
            return Err(ReadError::new(format!("invalid status filter: {status}")));
        }
    }
    Ok(())
}

fn finish_lines(lines: Vec<String>) -> String {
    if lines.is_empty() {
        String::new()
    } else {
        format!("{}\n", lines.join("\n"))
    }
}
