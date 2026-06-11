// Generated from tko.org. Do not edit by hand.

use std::process::Command;

fn tko_bin() -> &'static str {
    env!("CARGO_BIN_EXE_tko")
}

fn run(args: &[&str]) -> std::process::Output {
    Command::new(tko_bin())
        .args(args)
        .output()
        .expect("tko command should run")
}

#[test]
fn root_help_lists_command_surface() {
    let output = run(&["--help"]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("minimal org-mode ticket system"));
    assert!(stdout.contains("create"));
    assert!(stdout.contains("ready"));
    assert!(stdout.contains("blocked"));
    assert!(stdout.contains("add-note"));
    assert!(stdout.contains("notes"));
}

#[test]
fn help_command_prints_help() {
    let output = run(&["help"]);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Usage:"));
    assert!(stdout.contains("Commands:"));
}

#[test]
fn known_commands_parse_but_remain_stubbed() {
    let cases: &[(&[&str], &str)] = &[
        (&["create", "Ticket title", "--tags", "repo/tko"], "create"),
        (&["start", "sys-ywp7"], "start"),
        (&["block", "sys-ywp7"], "block"),
        (&["close", "sys-ywp7"], "close"),
        (&["reopen", "sys-ywp7"], "reopen"),
        (&["status", "sys-ywp7", "open"], "status"),
        (&["dep", "sys-rer6", "sys-ywp7"], "dep"),
        (&["undep", "sys-rer6", "sys-ywp7"], "undep"),
        (&["link", "sys-ywp7", "sys-rer6"], "link"),
        (&["unlink", "sys-ywp7", "sys-rer6"], "unlink"),
        (&["tag", "sys-ywp7", "repo/tko"], "tag"),
        (&["untag", "sys-ywp7", "repo/tko"], "untag"),
        (&["add-note", "sys-ywp7", "Title"], "add-note"),
        (&["lint", "sys-ywp7"], "lint"),
        (&["notes", "sys-ywp7"], "notes"),
    ];

    for (args, command_name) in cases {
        let output = run(args);
        assert_eq!(output.status.code(), Some(2), "args: {args:?}");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains(&format!("not implemented: {command_name}")),
            "args: {args:?}, stderr: {stderr}"
        );
    }
}

#[test]
fn unknown_flags_are_usage_errors() {
    let output = run(&["ready", "--mystery"]);

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("unexpected argument '--mystery'"));
}

#[test]
fn note_fetch_remains_explicitly_unimplemented() {
    let temp = tempfile::tempdir().expect("tempdir");
    let tickets_dir = temp.path().join(".tickets");
    std::fs::create_dir(&tickets_dir).expect("tickets dir");
    std::fs::write(tickets_dir.join("sys-ywp7.org"), "* Ticket\n").expect("write ticket");

    let output = Command::new(tko_bin())
        .args(["show", "sys-ywp7", "--note", "Spec"])
        .env("TICKETS_DIR", &tickets_dir)
        .output()
        .expect("tko command should run");

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("not implemented: show --note"));
}

#[test]
fn migration_command_reports_and_applies() {
    let temp = tempfile::tempdir().expect("tempdir");
    let tickets_dir = temp.path().join(".tickets");
    std::fs::create_dir(&tickets_dir).expect("tickets dir");
    let ticket_path = tickets_dir.join("sys-legacy.org");
    std::fs::write(
        &ticket_path,
        ":PROPERTIES:\n:TK_STATUS: open\n:END:\n\n* Legacy\n",
    )
    .expect("write ticket");

    let dry_run = Command::new(tko_bin())
        .args(["migrate-legacy-properties"])
        .env("TICKETS_DIR", &tickets_dir)
        .output()
        .expect("tko command should run");

    assert!(dry_run.status.success());
    let stdout = String::from_utf8_lossy(&dry_run.stdout);
    assert!(stdout.contains("rename TK_STATUS -> TKO_STATUS"));
    let unchanged = std::fs::read_to_string(&ticket_path).expect("read ticket");
    assert!(unchanged.contains(":TK_STATUS: open"));

    let apply = Command::new(tko_bin())
        .args(["migrate-legacy-properties", "--apply", "legacy"])
        .env("TICKETS_DIR", &tickets_dir)
        .output()
        .expect("tko command should run");

    assert!(apply.status.success());
    let migrated = std::fs::read_to_string(&ticket_path).expect("read ticket");
    assert!(migrated.contains(":TKO_STATUS: open"));
    assert!(!migrated.contains(":TK_STATUS: open"));
}
