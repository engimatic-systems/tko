// Generated from tko.org. Do not edit by hand.

use std::path::{Path, PathBuf};
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

    fn write(&self, id: &str, text: &str) -> PathBuf {
        let path = self.tickets_dir.join(format!("{id}.org"));
        std::fs::write(&path, text).expect("write ticket");
        path
    }

    fn run(&self, args: &[&str]) -> std::process::Output {
        Command::new(tko_bin())
            .args(args)
            .env("TICKETS_DIR", &self.tickets_dir)
            .current_dir(self.temp.path())
            .output()
            .expect("tko command should run")
    }
}

#[test]
fn lint_passes_clean_ticket() {
    let fixture = Fixture::new();
    fixture.write(
        "sys-clean",
        ":PROPERTIES:\n:TKO_ID: sys-clean\n:END:\n\n* Clean\n\n** Description\n\nBody.\n\n** Notes\n*** [2026-06-11 Thu 10:00Z] Short title\n",
    );

    let output = fixture.run(&["lint", "clean"]);

    assert!(output.status.success());
    assert!(output.stdout.is_empty());
}

#[test]
fn lint_warning_for_note_title_target_exits_successfully() {
    let fixture = Fixture::new();
    let title = "x".repeat(51);
    fixture.write(
        "sys-warn",
        &format!("* Warn\n\n** Notes\n*** [2026-06-11 Thu 10:00Z] {title}\n"),
    );

    let output = fixture.run(&["lint", "sys-warn"]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("L003 warning"));
    assert!(stdout.contains("51 > 50"));
}

#[test]
fn lint_fails_duplicate_bad_level_and_long_note() {
    let fixture = Fixture::new();
    let title = "x".repeat(73);
    fixture.write(
        "sys-bad",
        &format!(
            "* Bad\n\n*** Description\n\n** Description\n\n** Notes\n*** [2026-06-11 Thu 10:00Z] {title}\n"
        ),
    );

    let output = fixture.run(&["lint", "bad"]);

    assert_eq!(output.status.code(), Some(2));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("L001 duplicate semantic heading"));
    assert!(stdout.contains("L002 semantic heading must be level-2"));
    assert!(stdout.contains("L003 note title exceeds hard limit"));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("lint failed"));
}

#[test]
fn lint_accepts_path_targets() {
    let fixture = Fixture::new();
    let path = fixture.write("sys-path", "* Path\n\n*** Scope\n");

    let output = fixture.run(&["lint", path.to_str().expect("utf8 path")]);

    assert_eq!(output.status.code(), Some(2));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(&format!("{}:3: L002", display_path(&path))));
}

#[test]
fn lint_without_target_checks_all_tickets() {
    let fixture = Fixture::new();
    fixture.write("sys-clean", "* Clean\n\n** Scope\n");
    fixture.write("sys-bad", "* Bad\n\n*** Design\n");

    let output = fixture.run(&["lint"]);

    assert_eq!(output.status.code(), Some(2));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("sys-bad.org:3: L002"));
    assert!(!stdout.contains("sys-clean.org"));
}

#[test]
fn lint_passes_typed_tickets_with_satisfied_shapes() {
    let fixture = Fixture::new();
    fixture.write(
        "sys-dec-open",
        ":PROPERTIES:\n:TKO_TYPE: decision\n:END:\n\n* Open decision\n\n** Question\n\nQ?\n\n** Resolution\n",
    );
    fixture.write(
        "sys-dec-done",
        ":PROPERTIES:\n:TKO_TYPE: decision\n:TKO_STATUS: closed\n:END:\n\n* Closed decision\n\n** Question\n\nQ?\n\n** Resolution\n\nSettled.\n",
    );
    fixture.write(
        "sys-epic",
        ":PROPERTIES:\n:TKO_TYPE: epic\n:END:\n\n* Epic\n\n** Not yet specified\n\n** Decisions\n",
    );
    fixture.write(
        "sys-inc",
        ":PROPERTIES:\n:TKO_TYPE: incident\n:END:\n\n* Incident\n\n** Impact\n\nWrites down 20 minutes.\n\n** Resolution\n",
    );

    let output = fixture.run(&["lint"]);

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stdout.is_empty());
}

#[test]
fn lint_l005_flags_missing_or_empty_create_sections() {
    let fixture = Fixture::new();
    fixture.write(
        "sys-missing",
        ":PROPERTIES:\n:TKO_TYPE: decision\n:END:\n\n* No question\n\n** Resolution\n",
    );
    fixture.write(
        "sys-empty",
        ":PROPERTIES:\n:TKO_TYPE: decision\n:END:\n\n* Empty question\n\n** Question\n\n** Resolution\n",
    );

    let output = fixture.run(&["lint", "sys-missing"]);
    assert_eq!(output.status.code(), Some(2));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("sys-missing.org:1: L005 required section missing: Question (type decision)"));

    let output = fixture.run(&["lint", "sys-empty"]);
    assert_eq!(output.status.code(), Some(2));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("sys-empty.org:7: L005 required section is empty: Question (type decision)"));
}

#[test]
fn lint_l005_checks_close_sections_only_when_closed() {
    let fixture = Fixture::new();
    let body = "* Decision\n\n** Question\n\nQ?\n\n** Resolution\n";
    fixture.write(
        "sys-open",
        &format!(":PROPERTIES:\n:TKO_TYPE: decision\n:TKO_STATUS: open\n:END:\n\n{body}"),
    );
    fixture.write(
        "sys-closed",
        &format!(":PROPERTIES:\n:TKO_TYPE: decision\n:TKO_STATUS: closed\n:END:\n\n{body}"),
    );
    fixture.write(
        "sys-res",
        ":PROPERTIES:\n:TKO_TYPE: research\n:TKO_STATUS: closed\n:END:\n\n* Research\n\n** Question\n\nQ?\n\n** Findings\n",
    );

    let output = fixture.run(&["lint", "sys-open"]);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stdout.is_empty());

    let output = fixture.run(&["lint", "sys-closed"]);
    assert_eq!(output.status.code(), Some(2));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("L005 required section is empty: Resolution (type decision)"));

    let output = fixture.run(&["lint", "sys-res"]);
    assert_eq!(output.status.code(), Some(2));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("L005 required section is empty: Findings (type research)"));
}

#[test]
fn lint_l006_warns_on_foreign_semantic_headings() {
    let fixture = Fixture::new();
    fixture.write(
        "sys-dec",
        ":PROPERTIES:\n:TKO_TYPE: decision\n:END:\n\n* Decision\n\n** Question\n\nQ?\n\n** Resolution\n\n** Acceptance Criteria\n\nDone when merged.\n",
    );
    fixture.write("sys-task", "* Task\n\n** Not yet specified\n");

    let output = fixture.run(&["lint", "sys-dec"]);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(
        "sys-dec.org:13: L006 warning: semantic heading does not apply to type decision: Acceptance Criteria"
    ));

    let output = fixture.run(&["lint", "sys-task"]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(
        "sys-task.org:3: L006 warning: semantic heading does not apply to type task: Not yet specified"
    ));
}

#[test]
fn lint_skips_typed_rules_for_unknown_types() {
    let fixture = Fixture::new();
    fixture.write(
        "sys-odd",
        ":PROPERTIES:\n:TKO_TYPE: banana\n:END:\n\n* Odd\n\n*** Design\n",
    );

    let output = fixture.run(&["lint", "sys-odd"]);

    assert_eq!(output.status.code(), Some(2));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("L002"));
    assert!(!stdout.contains("L005"));
    assert!(!stdout.contains("L006"));
}

#[test]
fn lint_ignores_semantic_words_inside_note_bodies() {
    let fixture = Fixture::new();
    fixture.write(
        "sys-notes",
        "* Task\n\n** Description\n\nBody.\n\n** Notes\n*** [2026-06-11 Thu 10:00Z] Weighing options\nProse.\n**** Question\nIs the exemption real?\n**** Resolution\nYes.\n",
    );
    fixture.write("sys-out", "* Task\n\n*** Scope\n");
    fixture.write(
        "sys-dup",
        "* Task\n\n** Notes\n*** [2026-06-11 Thu 10:00Z] First\n\n** Notes\n",
    );

    let output = fixture.run(&["lint", "sys-notes"]);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stdout.is_empty());

    let output = fixture.run(&["lint", "sys-out"]);
    assert_eq!(output.status.code(), Some(2));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("L002 semantic heading must be level-2 (**): Scope"));

    let output = fixture.run(&["lint", "sys-dup"]);
    assert_eq!(output.status.code(), Some(2));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("L001 duplicate semantic heading: Notes"));
}

fn display_path(path: &Path) -> String {
    path.display().to_string()
}
