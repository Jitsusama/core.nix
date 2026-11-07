//! Which of Claude Code's config folders holds the plan's sign-in: the one
//! the feed is given, else pi-claude's, else Claude Code's own.

use std::fs;
use std::path::PathBuf;

use claude_plan_feed::config_folder;

#[test]
fn the_folder_it_is_given_wins() {
    let home = tempfile::tempdir().expect("a home");
    fs::create_dir(home.path().join(".claude-personal")).expect("pi-claude's folder");

    let folder = config_folder(
        Some(PathBuf::from("/given")),
        Some(PathBuf::from("/named")),
        home.path(),
    );

    assert_eq!(folder, PathBuf::from("/given"));
}

#[test]
fn the_folder_pi_claude_is_told_of_comes_next() {
    let home = tempfile::tempdir().expect("a home");

    let folder = config_folder(None, Some(PathBuf::from("/named")), home.path());

    assert_eq!(folder, PathBuf::from("/named"));
}

#[test]
fn pi_claude_s_own_folder_is_read_where_there_is_one() {
    let home = tempfile::tempdir().expect("a home");
    fs::create_dir(home.path().join(".claude-personal")).expect("pi-claude's folder");

    let folder = config_folder(None, None, home.path());

    assert_eq!(folder, home.path().join(".claude-personal"));
}

#[test]
fn claude_code_s_own_folder_is_read_where_it_is_the_only_login() {
    let home = tempfile::tempdir().expect("a home");

    let folder = config_folder(None, Some(PathBuf::new()), home.path());

    assert_eq!(folder, home.path().join(".claude"));
}
