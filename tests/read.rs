// Generated from tko.org. Do not edit by hand.

use serde_json::Value;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

fn tko_bin() -> &'static str {
    env!("CARGO_BIN_EXE_tko")
}

struct Fixture {
    _temp: TempDir,
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
            ":PROPERTIES:\n:TKO_ID: sys-a\n:TKO_STATUS: open\n:TKO_DEPS: []\n:TKO_LINKS: []\n:TKO_CREATED: 2026-06-11T10:00:00Z\n:TKO_TYPE: bug\n:TKO_PRIORITY: 1\n:TKO_ASSIGNEE: rosin\n:TKO_TAGS: [repo/tko]\n:END:\n\n* Alpha\n\n** Description\n\nAlpha body.\n",
        );
        write_ticket(
            &tickets_dir,
            "sys-b",
            ":PROPERTIES:\n:TKO_ID: sys-b\n:TKO_STATUS: open\n:TKO_DEPS: [sys-a]\n:TKO_LINKS: []\n:TKO_CREATED: 2026-06-11T11:00:00Z\n:TKO_TYPE: task\n:TKO_PRIORITY: 2\n:TKO_ASSIGNEE: rosin\n:TKO_TAGS: [repo/tko, archived]\n:END:\n\n* Bravo\n",
        );
        write_ticket(
            &tickets_dir,
            "sys-c",
            ":PROPERTIES:\n:TKO_ID: sys-c\n:TKO_STATUS: closed\n:TKO_DEPS: []\n:TKO_LINKS: []\n:TKO_CREATED: 2026-06-11T12:00:00Z\n:TKO_TYPE: task\n:TKO_PRIORITY: 0\n:TKO_TAGS: []\n:END:\n\n* Charlie\n",
        );
        write_ticket(
            &tickets_dir,
            "sys-d",
            ":PROPERTIES:\n:TKO_ID: sys-d\n:TKO_STATUS: in_progress\n:TKO_DEPS: [sys-c]\n:TKO_LINKS: []\n:TKO_CREATED: 2026-06-11T13:00:00Z\n:TKO_TYPE: task\n:TKO_PRIORITY: 3\n:TKO_ASSIGNEE: rosin\n:TKO_TAGS: [tooling]\n:END:\n\n* Delta\n",
        );
        Self {
            _temp: temp,
            tickets_dir,
        }
    }

    fn run(&self, args: &[&str]) -> std::process::Output {
        Command::new(tko_bin())
            .args(args)
            .env("TICKETS_DIR", &self.tickets_dir)
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

fn write_ticket(tickets_dir: &std::path::Path, id: &str, text: &str) {
    std::fs::write(tickets_dir.join(format!("{id}.org")), text).expect("write ticket");
}

#[test]
fn show_prints_metadata_and_outline_or_full_body() {
    let fixture = Fixture::new();

    let outline = fixture.stdout(&["show", "sys-a"]);
    assert!(outline.contains("id: sys-a\n"));
    assert!(outline.contains("priority: 1\n"));
    assert!(outline.contains("tags: [repo/tko]\n\n"));
    assert!(outline.contains("* Alpha\n"));
    assert!(outline.contains("** Description\n"));
    assert!(!outline.contains("Alpha body."));

    let full = fixture.stdout(&["show", "--full", "sys-a"]);
    assert!(full.contains("Alpha body."));
}

#[test]
fn list_ready_and_blocked_use_filters_and_dependency_state() {
    let fixture = Fixture::new();

    let list = fixture.stdout(&["list", "--status", "open", "-T", "repo/tko"]);
    assert_eq!(
        list,
        "sys-a    [open] :: Alpha <- []\nsys-b    [open] :: Bravo <- [sys-a]\n"
    );

    let ready = fixture.stdout(&["ready", "--assignee", "rosin"]);
    assert_eq!(
        ready,
        "sys-a    [open] :: Alpha <- []\nsys-d    [in_progress] :: Delta <- [sys-c]\n"
    );

    let blocked = fixture.stdout(&["blocked", "-T", "repo/tko"]);
    assert_eq!(blocked, "sys-b    [open] :: Bravo <- [sys-a]\n");

    let ready_ids = fixture.stdout(&["ready", "--output", "id", "--assignee", "rosin"]);
    assert_eq!(ready_ids, "sys-a\nsys-d\n");

    let blocked_json = fixture.stdout(&["blocked", "--output", "json", "-T", "repo/tko"]);
    let row = serde_json::from_str::<Value>(blocked_json.trim()).expect("json row");
    assert_eq!(row["id"], "sys-b");
    assert_eq!(row["deps"], serde_json::json!(["sys-a"]));
}

#[test]
fn output_modes_cover_ids_summaries_and_json() {
    let fixture = Fixture::new();

    let default_out = fixture.stdout(&["query", "status", "=", "open"]);
    assert_eq!(
        default_out,
        "sys-a    [open] :: Alpha <- []\nsys-b    [open] :: Bravo <- [sys-a]\n"
    );

    let ids = fixture.stdout(&["query", "--output", "id", "status", "=", "open"]);
    assert_eq!(ids, "sys-a\nsys-b\n");

    let summary = fixture.stdout(&["query", "--output", "summary", "status", "=", "open"]);
    assert_eq!(
        summary,
        "sys-a    [open] :: Alpha <- []\nsys-b    [open] :: Bravo <- [sys-a]\n"
    );

    let output = fixture.stdout(&["query", "--output", "json", "status", "=", "open"]);
    let rows = output
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).expect("json row"))
        .collect::<Vec<_>>();

    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["id"], "sys-a");
    assert_eq!(rows[0]["priority"], 1);
    assert!(rows[0]["priority"].is_number());
}

#[test]
fn query_dsl_supports_boolean_membership_and_presence() {
    let fixture = Fixture::new();

    let repo = fixture.stdout(&[
        "query",
        "--output",
        "id",
        "status",
        "in",
        "[open,",
        "in_progress]",
        "and",
        "tags",
        "contain",
        "repo/tko",
    ]);
    assert_eq!(lines(&repo), ["sys-a", "sys-b"]);

    let complex = fixture.stdout(&[
        "query", "--output", "id", "(", "type", "=", "bug", "or", "priority", "=", "3", ")", "and", "not", "tags",
        "contain", "archived",
    ]);
    assert_eq!(lines(&complex), ["sys-a", "sys-d"]);

    let no_deps = fixture.stdout(&["query", "--output", "id", "no", "deps"]);
    assert_eq!(lines(&no_deps), ["sys-a", "sys-c"]);
}

#[test]
fn query_does_not_accept_jq_filters() {
    let fixture = Fixture::new();
    let output = fixture.run(&["query", ".status", "==", "open"]);

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.is_empty());
}

fn lines(output: &str) -> Vec<&str> {
    output.lines().collect()
}
