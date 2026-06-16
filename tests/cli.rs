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
fn unknown_flags_are_usage_errors() {
    let output = run(&["ready", "--mystery"]);

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("unexpected argument '--mystery'"));
}

#[test]
fn help_documents_note_and_lint_surfaces() {
    let show = run(&["show", "--help"]);
    assert!(show.status.success());
    let show_help = String::from_utf8_lossy(&show.stdout);
    assert!(show_help.contains("--note"));
    assert!(show_help.contains("Print exactly one matching note subtree"));

    let lint = run(&["lint", "--help"]);
    assert!(lint.status.success());
    let lint_help = String::from_utf8_lossy(&lint.stdout);
    assert!(lint_help.contains("L003"));
    assert!(lint_help.contains("note-title"));
}
