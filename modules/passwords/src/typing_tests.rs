//! How typing reads niri and kitty, and says a type to the journal.

use super::*;
use serde_json::json;

#[test]
fn the_launcher_holding_the_keys_is_read_from_its_layer() {
    let holding = json!({"Ok": {"Layers": [
        {"namespace": "launcher", "keyboard_interactivity": "Exclusive"}]}});
    let shown = json!({"Ok": {"Layers": [
        {"namespace": "launcher", "keyboard_interactivity": "None"},
        {"namespace": "margin", "keyboard_interactivity": "Exclusive"}]}});
    assert_eq!(let_go(&holding.to_string()), Some(false));
    assert_eq!(let_go(&shown.to_string()), Some(true));
    assert_eq!(let_go("{\"Ok\": {\"Windows\": []}}"), None);
}

#[test]
fn the_focused_window_is_read_with_its_process_and_app() {
    let one = json!({"Ok": {"FocusedWindow": {"id": 7, "pid": 42, "app_id": "firefox"}}});
    assert_eq!(
        focused(&one.to_string()),
        Some(Some(Window {
            id: 7,
            pid: Some(42),
            app_id: "firefox".to_owned()
        }))
    );
    assert_eq!(focused("{\"Ok\": {\"FocusedWindow\": null}}"), Some(None));
    assert_eq!(focused("not niri"), None);
}

#[test]
fn kitty_is_asked_where_it_was_told_to_listen_else_by_its_pid() {
    let words = |line: &str| line.split(' ').map(str::to_owned).collect::<Vec<_>>();
    assert_eq!(
        listens_on(
            &words("kitty --listen-on unix:/run/user/1000/k.sock --class x"),
            9
        ),
        "unix:/run/user/1000/k.sock"
    );
    assert_eq!(
        listens_on(&words("kitty --listen-on=unix:@agentic-1"), 9),
        "unix:@agentic-1"
    );
    assert_eq!(listens_on(&words("kitty"), 9), "unix:@kitty-9");
}

#[test]
fn only_the_focused_windows_foreground_is_read() {
    let listed = json!([
        {"is_focused": false, "tabs": [{"is_focused": true, "windows": [
            {"is_focused": true, "foreground_processes": [{"pid": 1, "cmdline": ["pi"]}]}]}]},
        {"is_focused": true, "tabs": [
            {"is_active": false, "windows": [
                {"is_active": true, "foreground_processes": [{"pid": 2, "cmdline": ["pi"]}]}]},
            {"is_active": true, "windows": [
                {"is_active": false, "foreground_processes": [{"pid": 3, "cmdline": ["pi"]}]},
                {"is_active": true, "foreground_processes": [{"pid": 4, "cmdline": ["-zsh"]}]}]}]}
    ]);
    assert_eq!(
        foreground(&listed.to_string()),
        Some(vec![(Some(4), vec!["-zsh".to_owned()])])
    );
    assert_eq!(foreground("[]"), None);
}

#[test]
fn a_session_is_pi_or_mycelium_by_name_or_by_any_word() {
    let words = |line: &str| line.split(' ').map(str::to_owned).collect::<Vec<_>>();
    assert!(a_sessions("pi", &[]), "a process named pi wasn't a session");
    assert!(
        a_sessions("", &words("/nix/store/x-mycelium/bin/mycelium attach 1")),
        "mycelium as its command's first word wasn't a session"
    );
    assert!(
        a_sessions("ssh", &words("ssh -t far-machine mycelium attach 1")),
        "mycelium as a later word of ssh's command wasn't a session"
    );
    assert!(
        !a_sessions("zsh", &words("-zsh")),
        "a login zsh was taken for a session"
    );
    assert!(
        !a_sessions("nvim", &words("nvim pipeline.md")),
        "nvim on a file was taken for a session"
    );
}

#[test]
fn a_journal_line_names_the_entry_and_window_on_one_line_each() {
    let line = journal_line("Git\nHub", "fire\nfox", "didn't type", "no\nwhy");
    assert_eq!(
        line.lines().count(),
        6,
        "a newline in a value started a field: {line}"
    );
    assert!(line.contains("PASSWORDS_TYPED_TITLE=Git Hub\n"), "{line}");
    assert!(line.contains("PASSWORDS_TYPED_APP_ID=fire fox\n"), "{line}");
    assert!(
        line.contains("MESSAGE=didn't type \"Git Hub\" into fire fox: no why\n"),
        "{line}"
    );
}

#[test]
fn pi_is_found_at_a_process_or_above_it() {
    // This test's own process is no pi, though it may run beneath one, so
    // a process that has pi nowhere above it is the first's.
    assert_eq!(session_above(1), None);
    assert!(
        !kitty(std::process::id()),
        "this test's own process was taken for a kitty"
    );
}
