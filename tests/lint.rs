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
fn lint_fails_duplicate_bad_level_long_note_and_legacy_key() {
    let fixture = Fixture::new();
    let title = "x".repeat(73);
    fixture.write(
        "sys-bad",
        &format!(
            ":PROPERTIES:\n:TK_STATUS: open\n:END:\n\n* Bad\n\n*** Description\n\n** Description\n\n** Notes\n*** [2026-06-11 Thu 10:00Z] {title}\n"
        ),
    );

    let output = fixture.run(&["lint", "bad"]);

    assert_eq!(output.status.code(), Some(2));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("L001 duplicate semantic heading"));
    assert!(stdout.contains("L002 semantic heading must be level-2"));
    assert!(stdout.contains("L003 note title exceeds hard limit"));
    assert!(stdout.contains("L004 legacy TK_STATUS property key remains"));
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

fn display_path(path: &Path) -> String {
    path.display().to_string()
}
