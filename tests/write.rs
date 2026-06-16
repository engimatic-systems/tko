// Generated from tko.org. Do not edit by hand.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
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
        write_ticket(
            &tickets_dir,
            "sys-a",
            ":PROPERTIES:\n:TKO_ID: sys-a\n:TKO_STATUS: open\n:TKO_DEPS: []\n:TKO_LINKS: []\n:TKO_CREATED: 2026-06-11T10:00:00Z\n:TKO_TYPE: task\n:TKO_PRIORITY: 2\n:TKO_TAGS: []\n:END:\n\n* Alpha\n",
        );
        write_ticket(
            &tickets_dir,
            "sys-b",
            ":PROPERTIES:\n:TKO_ID: sys-b\n:TKO_STATUS: open\n:TKO_DEPS: []\n:TKO_LINKS: []\n:TKO_CREATED: 2026-06-11T11:00:00Z\n:TKO_TYPE: task\n:TKO_PRIORITY: 2\n:TKO_TAGS: []\n:END:\n\n* Bravo\n",
        );
        Self { temp, tickets_dir }
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

    fn read(&self, id: &str) -> String {
        std::fs::read_to_string(self.tickets_dir.join(format!("{id}.org"))).expect("read ticket")
    }
}

fn write_ticket(tickets_dir: &Path, id: &str, text: &str) {
    std::fs::write(tickets_dir.join(format!("{id}.org")), text).expect("write ticket");
}

#[test]
fn create_writes_tko_properties_sections_and_parent_resolution() {
    let fixture = Fixture::new();

    let id = fixture.stdout(&[
        "create",
        "Ignored",
        "Created ticket",
        "--description",
        "Line one\\nLine two",
        "--scope",
        "Small",
        "--type",
        "feature",
        "--priority",
        "1",
        "--assignee",
        "rosin",
        "--external-ref",
        "gh-123",
        "--parent",
        "sys-a",
        "--tags",
        "repo/tko, tooling,,",
    ]);
    let id = id.trim();
    let text = fixture.read(id);

    assert!(text.contains(":TKO_ID: "));
    assert!(text.contains(":TKO_STATUS: open"));
    assert!(text.contains(":TKO_DEPS: []"));
    assert!(text.contains(":TKO_LINKS: []"));
    assert!(text.contains(":TKO_TYPE: feature"));
    assert!(text.contains(":TKO_PRIORITY: 1"));
    assert!(text.contains(":TKO_ASSIGNEE: rosin"));
    assert!(text.contains(":TKO_EXTERNAL_REF: gh-123"));
    assert!(text.contains(":TKO_PARENT: sys-a"));
    assert!(text.contains(":TKO_TAGS: [repo/tko, tooling]"));
    assert!(text.contains("* Created ticket\n"));
    assert!(text.contains("** Description\n\nLine one\nLine two\n"));
    assert!(text.contains("** Scope\n\nSmall\n"));
    assert!(!text.contains(":TK_"));
}

#[test]
fn status_aliases_update_tko_status() {
    let fixture = Fixture::new();

    assert_eq!(
        fixture.stdout(&["start", "sys-a"]),
        "Updated sys-a -> in_progress\n"
    );
    assert!(fixture.read("sys-a").contains(":TKO_STATUS: in_progress"));
    assert_eq!(
        fixture.stdout(&["block", "sys-a"]),
        "Updated sys-a -> blocked\n"
    );
    assert!(fixture.read("sys-a").contains(":TKO_STATUS: blocked"));
    assert_eq!(
        fixture.stdout(&["close", "sys-a"]),
        "Updated sys-a -> closed\n"
    );
    assert!(fixture.read("sys-a").contains(":TKO_STATUS: closed"));
    assert_eq!(
        fixture.stdout(&["reopen", "sys-a"]),
        "Updated sys-a -> open\n"
    );
    assert!(fixture.read("sys-a").contains(":TKO_STATUS: open"));
}

#[test]
fn deps_preserve_order_and_avoid_duplicates() {
    let fixture = Fixture::new();

    assert_eq!(
        fixture.stdout(&["dep", "sys-a", "sys-b"]),
        "Added dependency: sys-a -> sys-b\n"
    );
    assert_eq!(
        fixture.stdout(&["dep", "sys-a", "sys-b"]),
        "Dependency already exists: sys-a -> sys-b\n"
    );
    assert!(fixture.read("sys-a").contains(":TKO_DEPS: [sys-b]"));
    assert_eq!(
        fixture.stdout(&["undep", "sys-a", "sys-b"]),
        "Removed dependency: sys-a -/-> sys-b\n"
    );
    assert_eq!(
        fixture.stdout(&["undep", "sys-a", "sys-b"]),
        "Dependency not present: sys-a -/-> sys-b\n"
    );
    assert!(fixture.read("sys-a").contains(":TKO_DEPS: []"));
}

#[test]
fn links_are_symmetric_and_deduplicated() {
    let fixture = Fixture::new();

    assert_eq!(
        fixture.stdout(&["link", "sys-a", "sys-b"]),
        "Added link: sys-a <-> sys-b\n"
    );
    assert_eq!(
        fixture.stdout(&["link", "sys-a", "sys-b"]),
        "Link already exists: sys-a <-> sys-b\n"
    );
    assert!(fixture.read("sys-a").contains(":TKO_LINKS: [sys-b]"));
    assert!(fixture.read("sys-b").contains(":TKO_LINKS: [sys-a]"));
    assert_eq!(
        fixture.stdout(&["unlink", "sys-a", "sys-b"]),
        "Removed link: sys-a <-> sys-b\n"
    );
    assert!(fixture.read("sys-a").contains(":TKO_LINKS: []"));
    assert!(fixture.read("sys-b").contains(":TKO_LINKS: []"));
}

#[test]
fn tags_preserve_order_and_avoid_duplicates() {
    let fixture = Fixture::new();

    assert_eq!(
        fixture.stdout(&["tag", "sys-a", "repo/tko", "tooling"]),
        "Added tag(s) to sys-a: repo/tko tooling\n"
    );
    assert_eq!(
        fixture.stdout(&["tag", "sys-a", "repo/tko"]),
        "Tag(s) already present on sys-a: repo/tko\n"
    );
    assert!(
        fixture
            .read("sys-a")
            .contains(":TKO_TAGS: [repo/tko, tooling]")
    );
    assert_eq!(
        fixture.stdout(&["untag", "sys-a", "repo/tko"]),
        "Removed tag(s) from sys-a: repo/tko\n"
    );
    assert!(fixture.read("sys-a").contains(":TKO_TAGS: [tooling]"));
}

#[test]
fn add_note_creates_level_two_notes_and_level_three_entries() {
    let fixture = Fixture::new();

    assert_eq!(
        fixture.stdout(&[
            "add-note",
            "sys-a",
            "--title",
            "Title line",
            "--body",
            "Body line",
        ]),
        "Note added to sys-a\n"
    );
    let text = fixture.read("sys-a");
    assert!(text.contains("** Notes\n*** ["));
    assert!(text.contains("] Title line\nBody line\n"));

    assert_eq!(
        fixture.stdout(&["add-note", "sys-b", "--title", "Title only"]),
        "Note added to sys-b\n"
    );
    let text = fixture.read("sys-b");
    assert!(text.contains("] Title only\n"));

    let mut child = Command::new(tko_bin())
        .args(["add-note", "sys-a", "--title", "Piped title"])
        .env("TICKETS_DIR", &fixture.tickets_dir)
        .current_dir(fixture.temp.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn tko");
    {
        use std::io::Write;
        let stdin = child.stdin.as_mut().expect("stdin");
        stdin.write_all(b"Piped body\n").expect("write stdin");
    }
    let output = child.wait_with_output().expect("wait");
    assert!(output.status.success());
    let text = fixture.read("sys-a");
    assert!(text.contains("] Piped title\nPiped body\n"));

    let output = fixture.run(&["add-note", "sys-a"]);
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("--title"));
}
