// Generated from tko.org. Do not edit by hand.

use crate::storage::TicketStore;
use std::error::Error;
use std::fmt;

pub type Result<T> = std::result::Result<T, NotesError>;

#[derive(Debug)]
pub struct NotesError {
    message: String,
}

impl NotesError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for NotesError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.message)
    }
}

impl Error for NotesError {}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NoteEntry {
    heading: String,
    title: String,
    timestamp: Option<String>,
    subtree: String,
}

pub fn list_notes(store: &TicketStore, id: &str) -> Result<String> {
    let ticket = store
        .load(id)
        .map_err(|error| NotesError::new(error.to_string()))?;
    let lines = note_entries(&ticket.body)
        .into_iter()
        .map(|entry| entry.heading)
        .collect::<Vec<_>>();
    Ok(finish_lines(lines))
}

pub fn show_note(store: &TicketStore, id: &str, note_match: &str) -> Result<String> {
    let ticket = store
        .load(id)
        .map_err(|error| NotesError::new(error.to_string()))?;
    let needle = note_match.to_ascii_lowercase();
    let matches = note_entries(&ticket.body)
        .into_iter()
        .filter(|entry| {
            entry.title.to_ascii_lowercase().contains(&needle)
                || entry
                    .timestamp
                    .as_ref()
                    .is_some_and(|timestamp| timestamp.to_ascii_lowercase().contains(&needle))
        })
        .collect::<Vec<_>>();

    match matches.len() {
        0 => Err(NotesError::new(format!("note not found: {note_match}"))),
        1 => Ok(ensure_trailing_newline(matches[0].subtree.clone())),
        _ => {
            let candidates = finish_lines(
                matches
                    .iter()
                    .map(|entry| format!("candidate: {}", entry.heading))
                    .collect(),
            );
            Err(NotesError::new(format!(
                "ambiguous note match: {note_match}\n{candidates}"
            )))
        }
    }
}

fn note_entries(body: &str) -> Vec<NoteEntry> {
    let lines = split_lines(body);
    let Some(notes_index) = lines.iter().position(|line| {
        heading(line)
            .is_some_and(|(level, title)| level == 2 && title.eq_ignore_ascii_case("Notes"))
    }) else {
        return Vec::new();
    };

    let notes_end = lines
        .iter()
        .enumerate()
        .skip(notes_index + 1)
        .find_map(|(index, line)| match heading(line) {
            Some((level, _)) if level <= 2 => Some(index),
            _ => None,
        })
        .unwrap_or(lines.len());

    let mut entries = Vec::new();
    let mut index = notes_index + 1;
    while index < notes_end {
        let Some((3, heading_text)) = heading(&lines[index]) else {
            index += 1;
            continue;
        };
        let start = index;
        let end = lines
            .iter()
            .enumerate()
            .take(notes_end)
            .skip(index + 1)
            .find_map(|(candidate, line)| match heading(line) {
                Some((level, _)) if level <= 3 => Some(candidate),
                _ => None,
            })
            .unwrap_or(notes_end);
        let subtree = lines[start..end].concat();
        let (timestamp, title) = split_timestamp_title(heading_text);
        entries.push(NoteEntry {
            heading: heading_text.to_string(),
            title: title.to_string(),
            timestamp: timestamp.map(ToOwned::to_owned),
            subtree,
        });
        index = end;
    }
    entries
}

fn split_timestamp_title(heading_text: &str) -> (Option<&str>, &str) {
    let trimmed = heading_text.trim();
    if let Some((timestamp, rest)) = trimmed
        .strip_prefix('[')
        .and_then(|_| trimmed.split_once(']'))
    {
        (Some(timestamp.trim_start_matches('[')), rest.trim_start())
    } else {
        (None, trimmed)
    }
}

fn heading(line: &str) -> Option<(usize, &str)> {
    let trimmed = line.trim_end_matches(['\r', '\n']);
    let stars = trimmed.chars().take_while(|ch| *ch == '*').count();
    if stars == 0 || !trimmed.chars().nth(stars).is_some_and(|ch| ch == ' ') {
        return None;
    }
    Some((stars, trimmed[stars + 1..].trim_end()))
}

fn split_lines(text: &str) -> Vec<String> {
    if text.is_empty() {
        Vec::new()
    } else {
        text.split_inclusive('\n').map(ToOwned::to_owned).collect()
    }
}

fn finish_lines(lines: Vec<String>) -> String {
    if lines.is_empty() {
        String::new()
    } else {
        format!("{}\n", lines.join("\n"))
    }
}

fn ensure_trailing_newline(mut text: String) -> String {
    if !text.ends_with('\n') {
        text.push('\n');
    }
    text
}
