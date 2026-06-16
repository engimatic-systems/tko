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
        (&["ready", "-T", "repo/tko"], "ready"),
        (&["blocked", "--assignee", "rosin"], "blocked"),
        (&["list", "--status", "open"], "list"),
        (&["ls", "--tag", "repo/tko"], "list"),
        (&["show", "--full", "sys-ywp7"], "show"),
        (&["show", "sys-ywp7", "--note", "Spec"], "show"),
        (&["add-note", "sys-ywp7", "Title"], "add-note"),
        (&["query", "status", "=", "open"], "query"),
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
