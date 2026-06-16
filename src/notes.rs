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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct NoteEntry<'a> {
    heading: &'a str,
    title: &'a str,
    timestamp: Option<&'a str>,
    subtree: &'a str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Line<'a> {
    start: usize,
    end: usize,
    text: &'a str,
}

pub fn list_notes(store: &TicketStore, id: &str) -> Result<String> {
    let ticket = store
        .load(id)
        .map_err(|error| NotesError::new(error.to_string()))?;
    Ok(finish_lines(
        note_entries(&ticket.body)
            .into_iter()
            .map(|entry| entry.heading),
    ))
}

pub fn show_note(store: &TicketStore, id: &str, note_match: &str) -> Result<String> {
    let ticket = store
        .load(id)
        .map_err(|error| NotesError::new(error.to_string()))?;
    let matches = note_entries(&ticket.body)
        .into_iter()
        .filter(|entry| contains_ascii_case_insensitive(entry.heading, note_match))
        .collect::<Vec<_>>();

    match matches.len() {
        0 => Err(NotesError::new(format!("note not found: {note_match}"))),
        1 => Ok(ensure_trailing_newline(matches[0].subtree.to_string())),
        _ => {
            let candidates = finish_lines(
                matches
                    .iter()
                    .map(|entry| format!("candidate: {}", entry.heading)),
            );
            Err(NotesError::new(format!(
                "ambiguous note match: {note_match}\n{candidates}"
            )))
        }
    }
}

fn note_entries(body: &str) -> Vec<NoteEntry<'_>> {
    let lines = line_spans(body);
    let Some(notes_index) = lines.iter().position(|line| {
        heading(line.text)
            .is_some_and(|(level, title)| level == 2 && title.eq_ignore_ascii_case("Notes"))
    }) else {
        return Vec::new();
    };

    let mut entries = Vec::new();
    let mut index = notes_index + 1;
    while index < lines.len() {
        let Some((3, heading_text)) = heading(lines[index].text) else {
            index += 1;
            continue;
        };
        let start = index;
        let end = lines
            .iter()
            .enumerate()
            .skip(index + 1)
            .find_map(|(candidate, line)| match heading(line.text) {
                Some((level, _)) if level <= 3 => Some(candidate),
                _ => None,
            })
            .unwrap_or(lines.len());
        let subtree = &body[lines[start].start..lines[end - 1].end];
        let (timestamp, title) = split_timestamp_title(heading_text);
        entries.push(NoteEntry {
            heading: heading_text,
            title,
            timestamp,
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
    let bytes = trimmed.as_bytes();
    let mut stars = 0usize;
    while matches!(bytes.get(stars), Some(b'*')) {
        stars += 1;
    }
    if stars == 0 || !matches!(bytes.get(stars), Some(b' ')) {
        return None;
    }
    Some((stars, trimmed[stars + 1..].trim_end()))
}

fn line_spans(text: &str) -> Vec<Line<'_>> {
    let mut start = 0usize;
    text.split_inclusive('\n')
        .map(|line| {
            let end = start + line.len();
            let span = Line {
                start,
                end,
                text: line,
            };
            start = end;
            span
        })
        .collect()
}

fn finish_lines<I, S>(lines: I) -> String
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut output = String::new();
    for line in lines {
        output.push_str(line.as_ref());
        output.push('\n');
    }
    output
}

fn ensure_trailing_newline(mut text: String) -> String {
    if !text.ends_with('\n') {
        text.push('\n');
    }
    text
}

fn contains_ascii_case_insensitive(haystack: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return true;
    }
    haystack
        .as_bytes()
        .windows(needle.len())
        .any(|window| window.eq_ignore_ascii_case(needle.as_bytes()))
}
