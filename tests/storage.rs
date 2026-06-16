// Generated from tko.org. Do not edit by hand.

use std::fs;
use std::path::Path;
use tempfile::tempdir;
use tko::storage::{
    StorageError, TicketStore, discover_tickets_dir, format_list_value, load_ticket,
    parse_list_value,
};

fn write(path: &Path, text: &str) {
    fs::write(path, text).expect("write fixture");
}

#[test]
fn discovers_tickets_dir_by_walking_upward() {
    let temp = tempdir().expect("tempdir");
    let nested = temp.path().join("a/b/c");
    fs::create_dir_all(&nested).expect("nested dirs");
    let tickets = temp.path().join(".tickets");
    fs::create_dir(&tickets).expect("tickets dir");

    let discovered = discover_tickets_dir(&nested, None, false).expect("discover tickets");
    assert_eq!(discovered, tickets);

    let explicit = temp.path().join("explicit");
    let discovered = discover_tickets_dir(&nested, Some(&explicit), false).expect("env wins");
    assert_eq!(discovered, explicit);
}

#[test]
fn loads_canonical_ticket_properties_and_body() {
    let temp = tempdir().expect("tempdir");
    let tickets = temp.path().join(".tickets");
    fs::create_dir(&tickets).expect("tickets dir");
    write(
        &tickets.join("sys-ywp7.org"),
        ":PROPERTIES:\n:TKO_ID: sys-ywp7\n:TKO_STATUS: in_progress\n:TKO_DEPS: [sys-a, sys-b]\n:TKO_LINKS: [sys-c]\n:TKO_CREATED: 2026-06-11T18:20:12Z\n:TKO_TYPE: task\n:TKO_PRIORITY: 1\n:TKO_ASSIGNEE: rosin\n:TKO_EXTERNAL_REF: gh-123\n:TKO_PARENT: sys-root\n:TKO_TAGS: [repo/tko, tooling]\n:END:\n\n* Define spec\n\n** Description\n\nBody.\n",
    );

    let store = TicketStore::new(&tickets);
    let ticket = store.load("ywp7").expect("load ticket");

    assert_eq!(ticket.id, "sys-ywp7");
    assert_eq!(ticket.title, "Define spec");
    assert_eq!(ticket.properties.status, "in_progress");
    assert_eq!(ticket.properties.deps, ["sys-a", "sys-b"]);
    assert_eq!(ticket.properties.links, ["sys-c"]);
    assert_eq!(ticket.properties.priority, 1);
    assert_eq!(ticket.properties.assignee.as_deref(), Some("rosin"));
    assert_eq!(ticket.properties.external_ref.as_deref(), Some("gh-123"));
    assert_eq!(ticket.properties.parent.as_deref(), Some("sys-root"));
    assert_eq!(ticket.properties.tags, ["repo/tko", "tooling"]);
    assert!(ticket.body.starts_with("* Define spec"));
}

#[test]
fn missing_canonical_properties_use_defaults() {
    let temp = tempdir().expect("tempdir");
    let tickets = temp.path().join(".tickets");
    fs::create_dir(&tickets).expect("tickets dir");
    let path = tickets.join("sys-defaults.org");
    write(&path, ":PROPERTIES:\n:END:\n\n* Defaulted ticket\n");

    let ticket = load_ticket(&path).expect("load ticket");

    assert_eq!(ticket.id, "sys-defaults");
    assert_eq!(ticket.properties.status, "open");
    assert_eq!(ticket.properties.priority, 2);
    assert!(ticket.properties.deps.is_empty());
    assert!(ticket.properties.tags.is_empty());
}

#[test]
fn parses_and_formats_list_properties() {
    assert_eq!(parse_list_value("[]").expect("empty"), Vec::<String>::new());
    assert_eq!(
        parse_list_value("[one, two, repo/tko]").expect("items"),
        ["one", "two", "repo/tko"]
    );
    assert_eq!(format_list_value(&[]), "[]");
    assert_eq!(
        format_list_value(&["one".to_string(), "two".to_string()]),
        "[one, two]"
    );
    assert!(parse_list_value("one, two").is_err());
}

#[test]
fn resolves_ticket_ids_by_filename_stem() {
    let temp = tempdir().expect("tempdir");
    let tickets = temp.path().join(".tickets");
    fs::create_dir(&tickets).expect("tickets dir");
    write(&tickets.join("sys-ywp7.org"), "* One\n");
    write(&tickets.join("sys-rer6.org"), "* Two\n");

    let store = TicketStore::new(&tickets);
    assert_eq!(
        store.resolve_id("ywp").expect("partial"),
        tickets.join("sys-ywp7.org")
    );

    let ambiguous = store.resolve_id("sys").expect_err("ambiguous");
    assert!(matches!(ambiguous, StorageError::AmbiguousTicketId { .. }));

    let missing = store.resolve_id("none").expect_err("missing");
    assert!(matches!(missing, StorageError::TicketNotFound(_)));
}

#[test]
fn updates_existing_properties_and_inserts_missing_drawer() {
    let temp = tempdir().expect("tempdir");
    let tickets = temp.path().join(".tickets");
    fs::create_dir(&tickets).expect("tickets dir");
    let existing = tickets.join("sys-existing.org");
    write(
        &existing,
        ":PROPERTIES:\n:TKO_STATUS: open\n:END:\n\n* Existing\n",
    );
    let missing = tickets.join("sys-missing.org");
    write(&missing, "* Missing drawer\n");

    let store = TicketStore::new(&tickets);
    store
        .set_property("existing", "TKO_STATUS", "closed")
        .expect("update property");
    store
        .set_property("missing", "TKO_STATUS", "open")
        .expect("insert drawer");

    let existing_text = fs::read_to_string(existing).expect("read existing");
    assert!(existing_text.contains(":TKO_STATUS: closed"));
    let missing_text = fs::read_to_string(missing).expect("read missing");
    assert!(missing_text.starts_with(":PROPERTIES:\n:TKO_STATUS: open\n:END:\n\n* Missing drawer"));
}
