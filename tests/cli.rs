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
    assert!(stdout.contains("init"));
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
    let cases: &[(&[&str], &str)] = &[(&["notes", "sys-ywp7"], "notes")];

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
