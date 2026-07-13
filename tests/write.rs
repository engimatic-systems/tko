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

    let id = fixture.stdout(&["create", "No assignee"]);
    let text = fixture.read(id.trim());
    assert!(!text.contains(":TKO_ASSIGNEE:"));

    let output = fixture.run(&["create", ""]);
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("ticket title is required"));
}

#[test]
fn create_scaffolds_typed_sections() {
    let fixture = Fixture::new();

    let id = fixture.stdout(&[
        "create",
        "Pick a store",
        "--type",
        "decision",
        "--question",
        "Postgres or SQLite?",
    ]);
    let text = fixture.read(id.trim());
    assert!(text.contains(":TKO_TYPE: decision"));
    assert!(text.ends_with("* Pick a store\n\n** Question\n\nPostgres or SQLite?\n\n** Resolution\n"));

    let id = fixture.stdout(&[
        "create",
        "Survey runtimes",
        "--type",
        "research",
        "--question",
        "Which runtimes fit?",
    ]);
    let text = fixture.read(id.trim());
    assert!(text.contains(":TKO_TYPE: research"));
    assert!(text.ends_with("* Survey runtimes\n\n** Question\n\nWhich runtimes fit?\n\n** Findings\n"));

    let id = fixture.stdout(&[
        "create",
        "API outage",
        "--type",
        "incident",
        "--impact",
        "Writes failed for 20 minutes",
    ]);
    let text = fixture.read(id.trim());
    assert!(text.contains(":TKO_TYPE: incident"));
    assert!(text.ends_with("* API outage\n\n** Impact\n\nWrites failed for 20 minutes\n\n** Resolution\n"));

    let id = fixture.stdout(&["create", "Big effort", "--type", "epic"]);
    let text = fixture.read(id.trim());
    assert!(text.ends_with(
        "* Big effort\n\n** Description\n\n** Scope\n\n** Design\n\n** Acceptance Criteria\n\n** Not yet specified\n\n** Decisions\n"
    ));

    let id = fixture.stdout(&["create", "Plain task", "--description", "Body"]);
    let text = fixture.read(id.trim());
    assert!(text.ends_with(
        "* Plain task\n\n** Description\n\nBody\n\n** Scope\n\n** Design\n\n** Acceptance Criteria\n"
    ));
}

#[test]
fn create_refuses_missing_required_and_foreign_sections() {
    let fixture = Fixture::new();

    let output = fixture.run(&["create", "No question", "--type", "decision"]);
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("decision ticket requires --question"));

    let output = fixture.run(&["create", "No impact", "--type", "incident"]);
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("incident ticket requires --impact"));

    let output = fixture.run(&[
        "create",
        "Foreign flag",
        "--type",
        "decision",
        "--question",
        "Q?",
        "--design",
        "sketch",
    ]);
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("--design does not apply to type decision"));

    let output = fixture.run(&["create", "Foreign flag", "--question", "Q?"]);
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("--question does not apply to type task"));
}

#[test]
fn create_normalizes_section_inputs_before_validation() {
    let fixture = Fixture::new();

    let output = fixture.run(&["create", "Blank question", "--type", "decision", "--question", "\\n"]);
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("decision ticket requires --question"));

    let id = fixture.stdout(&[
        "create",
        "Multiline question",
        "--type",
        "decision",
        "--question",
        "line1\\nline2",
    ]);
    let text = fixture.read(id.trim());
    assert!(text.ends_with("** Question\n\nline1\nline2\n\n** Resolution\n"));
}

#[test]
fn init_creates_ticket_storage_explicitly() {
    let temp = tempfile::tempdir().expect("tempdir");
    let output = Command::new(tko_bin())
        .args(["init"])
        .current_dir(temp.path())
        .output()
        .expect("run init");

    assert!(output.status.success());
    assert!(temp.path().join(".tickets").is_dir());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Initialized"));
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
fn close_gate_requires_resolution_content_for_decisions() {
    let fixture = Fixture::new();
    let id = fixture.stdout(&[
        "create",
        "Pick a store",
        "--type",
        "decision",
        "--question",
        "Postgres or SQLite?",
    ]);
    let id = id.trim().to_string();

    let output = fixture.run(&["close", &id]);
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("decision ticket requires non-empty Resolution before close (or pass --reason)"));

    let output = fixture.run(&["status", &id, "closed"]);
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("decision ticket requires non-empty Resolution before close (or pass --reason)"));
    assert!(fixture.read(&id).contains(":TKO_STATUS: open"));

    assert_eq!(
        fixture.stdout(&["close", &id, "--reason", "Postgres"]),
        format!("Updated {id} -> closed\n")
    );
    let text = fixture.read(&id);
    assert!(text.contains(":TKO_STATUS: closed"));
    assert!(text.ends_with("** Resolution\n\nPostgres\n"));
}

#[test]
fn close_reason_appends_to_existing_resolution() {
    let fixture = Fixture::new();
    write_ticket(
        &fixture.tickets_dir,
        "sys-dec",
        ":PROPERTIES:\n:TKO_ID: sys-dec\n:TKO_STATUS: open\n:TKO_TYPE: decision\n:END:\n\n* Decided\n\n** Question\n\nQ?\n\n** Resolution\n\nFirst call\n\n** Notes\n",
    );

    assert_eq!(
        fixture.stdout(&["close", "sys-dec", "--reason", "Second call"]),
        "Updated sys-dec -> closed\n"
    );
    let text = fixture.read("sys-dec");
    assert!(text.contains("** Resolution\n\nFirst call\n\nSecond call\n\n** Notes\n"));
}

#[test]
fn close_accepts_prefilled_resolution_and_tasks_stay_ungated() {
    let fixture = Fixture::new();
    write_ticket(
        &fixture.tickets_dir,
        "sys-dec",
        ":PROPERTIES:\n:TKO_ID: sys-dec\n:TKO_STATUS: open\n:TKO_TYPE: decision\n:END:\n\n* Decided\n\n** Question\n\nQ?\n\n** Resolution\n\nAlready settled\n",
    );

    assert_eq!(
        fixture.stdout(&["close", "sys-dec"]),
        "Updated sys-dec -> closed\n"
    );

    assert_eq!(fixture.stdout(&["close", "sys-a"]), "Updated sys-a -> closed\n");

    let output = fixture.run(&["close", "sys-b", "--reason", "done"]);
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("--reason does not apply to type task"));
    assert!(fixture.read("sys-b").contains(":TKO_STATUS: open"));
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
        fixture.stdout(&[
            "add-note",
            "sys-a",
            "--title",
            "Escaped body",
            "--body",
            "Line one\\nLine two",
        ]),
        "Note added to sys-a\n"
    );
    let text = fixture.read("sys-a");
    assert!(text.contains("] Escaped body\nLine one\nLine two\n"));

    assert_eq!(
        fixture.stdout(&["add-note", "sys-b", "--title", "Title only"]),
        "Note added to sys-b\n"
    );
    let text = fixture.read("sys-b");
    assert!(text.contains("] Title only\n"));

    let output = fixture.run(&["add-note", "sys-a"]);
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("--title"));

    let output = fixture.run(&["add-note", "sys-a", "--title", "Bad\\nTitle"]);
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("escaped newlines"));
}
