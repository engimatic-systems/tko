// Generated from tko.org. Do not edit by hand.

use std::fs;
use std::path::Path;
use tempfile::tempdir;
use tko::storage::{
    MigrationAction, StorageError, TicketStore, discover_tickets_dir, format_list_value,
    lint_legacy_property_keys, load_ticket, migrate_legacy_properties, parse_list_value,
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
fn missing_optional_properties_default_and_legacy_keys_are_ignored() {
    let temp = tempdir().expect("tempdir");
    let tickets = temp.path().join(".tickets");
    fs::create_dir(&tickets).expect("tickets dir");
    let path = tickets.join("sys-legacy.org");
    write(
        &path,
        ":PROPERTIES:\n:TK_STATUS: closed\n:TK_PRIORITY: 0\n:END:\n\n* Legacy ticket\n",
    );

    let ticket = load_ticket(&path).expect("load ticket");

    assert_eq!(ticket.id, "sys-legacy");
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

#[test]
fn migration_dry_run_reports_without_writing_and_apply_renames() {
    let temp = tempdir().expect("tempdir");
    let path = temp.path().join("sys-legacy.org");
    write(
        &path,
        ":PROPERTIES:\n:TK_STATUS: open\n:TK_TAGS: [repo/tko]\n:END:\n\n* Legacy\n",
    );

    let dry_run = migrate_legacy_properties(&path, false).expect("dry run");
    assert_eq!(dry_run.actions.len(), 2);
    assert_eq!(
        dry_run.actions[0],
        MigrationAction::Rename {
            legacy_key: "TK_STATUS".to_string(),
            canonical_key: "TKO_STATUS".to_string(),
            value: "open".to_string(),
        }
    );
    let unchanged = fs::read_to_string(&path).expect("read unchanged");
    assert!(unchanged.contains(":TK_STATUS: open"));

    let apply = migrate_legacy_properties(&path, true).expect("apply");
    assert_eq!(apply.actions.len(), 2);
    let migrated = fs::read_to_string(&path).expect("read migrated");
    assert!(migrated.contains(":TKO_STATUS: open"));
    assert!(migrated.contains(":TKO_TAGS: [repo/tko]"));
    assert!(!migrated.contains(":TK_STATUS: open"));
}

#[test]
fn migration_removes_matching_legacy_keys_and_reports_conflicts() {
    let temp = tempdir().expect("tempdir");
    let matching = temp.path().join("sys-matching.org");
    write(
        &matching,
        ":PROPERTIES:\n:TKO_STATUS: open\n:TK_STATUS: open\n:END:\n\n* Matching\n",
    );
    let report = migrate_legacy_properties(&matching, true).expect("matching migration");
    assert_eq!(report.actions.len(), 1);
    let matching_text = fs::read_to_string(&matching).expect("read matching");
    assert!(matching_text.contains(":TKO_STATUS: open"));
    assert!(!matching_text.contains(":TK_STATUS: open\n"));

    let conflict = temp.path().join("sys-conflict.org");
    write(
        &conflict,
        ":PROPERTIES:\n:TKO_STATUS: open\n:TK_STATUS: closed\n:END:\n\n* Conflict\n",
    );
    let report = migrate_legacy_properties(&conflict, true).expect("conflict migration");
    assert!(report.actions.is_empty());
    assert_eq!(report.conflicts.len(), 1);
    let conflict_text = fs::read_to_string(&conflict).expect("read conflict");
    assert!(conflict_text.contains(":TKO_STATUS: open"));
    assert!(conflict_text.contains(":TK_STATUS: closed"));
}

#[test]
fn l004_reports_legacy_keys_in_active_property_drawer() {
    let temp = tempdir().expect("tempdir");
    let path = temp.path().join("sys-legacy.org");
    write(&path, ":PROPERTIES:\n:TK_STATUS: open\n:END:\n\n* Legacy\n");

    let findings = lint_legacy_property_keys(&path).expect("lint");

    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].code, "L004");
    assert_eq!(findings[0].line, 2);
    assert!(findings[0].message.contains("TK_STATUS"));
}
