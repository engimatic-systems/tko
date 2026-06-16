// Generated from tko.org. Do not edit by hand.

use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

fn tko_bin() -> &'static str {
    env!("CARGO_BIN_EXE_tko")
}

struct Fixture {
    temp: TempDir,
    tickets_dir: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().expect("tempdir");
        let tickets_dir = temp.path().join(".tickets");
        std::fs::create_dir(&tickets_dir).expect("tickets dir");
        Self { temp, tickets_dir }
    }

    fn write(&self, id: &str, text: &str) {
        std::fs::write(self.tickets_dir.join(format!("{id}.org")), text).expect("write ticket");
    }

    fn run(&self, args: &[&str]) -> std::process::Output {
        Command::new(tko_bin())
            .args(args)
            .env("TICKETS_DIR", &self.tickets_dir)
            .current_dir(self.temp.path())
            .output()
            .expect("tko command should run")
    }

    fn stdout(&self, args: &[&str]) -> String {
        let output = self.run(args);
        assert!(
            output.status.success(),
            "stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).expect("utf8 stdout")
    }
}

#[test]
fn notes_handles_missing_section_successfully() {
    let fixture = Fixture::new();
    fixture.write("sys-none", "* No notes\n\n** Description\nBody.\n");

    let output = fixture.run(&["notes", "sys-none"]);

    assert!(output.status.success());
    assert!(output.stdout.is_empty());
}

#[test]
fn notes_lists_entries_in_document_order() {
    let fixture = Fixture::new();
    fixture.write(
        "sys-notes",
        "* Ticket\n\n** Notes\n*** [2026-06-11 Thu 10:00Z] First title\nFirst body.\n*** [2026-06-11 Thu 11:00Z]\nTimestamp-only body.\n*** Untimestamped title\nLoose body.\n",
    );

    let output = fixture.stdout(&["notes", "notes"]);

    assert_eq!(
        output,
        "[2026-06-11 Thu 10:00Z] First title\n[2026-06-11 Thu 11:00Z]\nUntimestamped title\n"
    );
}

#[test]
fn show_note_matches_title_case_insensitively_and_prints_one_subtree() {
    let fixture = Fixture::new();
    fixture.write(
        "sys-notes",
        "* Ticket\n\n** Notes\n*** [2026-06-11 Thu 10:00Z] First title\nFirst body.\n**** Child\nChild body.\n*** [2026-06-11 Thu 11:00Z] Second title\nSecond body.\n",
    );

    let output = fixture.stdout(&["show", "sys-notes", "--note", "FIRST"]);

    assert_eq!(
        output,
        "*** [2026-06-11 Thu 10:00Z] First title\nFirst body.\n**** Child\nChild body.\n"
    );
    assert!(!output.contains("Second body"));
}

#[test]
fn show_note_can_match_timestamp_text() {
    let fixture = Fixture::new();
    fixture.write(
        "sys-notes",
        "* Ticket\n\n** Notes\n*** [2026-06-11 Thu 10:00Z] First title\nFirst body.\n*** [2026-06-12 Fri 11:00Z] Second title\nSecond body.\n",
    );

    let output = fixture.stdout(&["show", "sys-notes", "--note", "2026-06-12"]);

    assert_eq!(
        output,
        "*** [2026-06-12 Fri 11:00Z] Second title\nSecond body.\n"
    );
}

#[test]
fn ambiguous_note_match_lists_candidates_without_bodies() {
    let fixture = Fixture::new();
    fixture.write(
        "sys-notes",
        "* Ticket\n\n** Notes\n*** [2026-06-11 Thu 10:00Z] Alpha deploy\nsecret body one\n*** [2026-06-11 Thu 11:00Z] Alpha rollback\nsecret body two\n",
    );

    let output = fixture.run(&["show", "sys-notes", "--note", "alpha"]);

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("ambiguous note match: alpha"));
    assert!(stderr.contains("candidate: [2026-06-11 Thu 10:00Z] Alpha deploy"));
    assert!(stderr.contains("candidate: [2026-06-11 Thu 11:00Z] Alpha rollback"));
    assert!(!stderr.contains("secret body"));
}

#[test]
fn add_note_refuses_overlong_title() {
    let fixture = Fixture::new();
    fixture.write("sys-notes", "* Ticket\n");
    let title = "x".repeat(73);

    let output = fixture.run(&["add-note", "sys-notes", "--title", &title]);

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("note title exceeds 72 characters"));
}
