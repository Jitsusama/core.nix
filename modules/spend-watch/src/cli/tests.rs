//! The command line's tests: each refusal and exit, run through the kit
//! as a caller would see them.

use super::*;
use terminal_kit::{Ending, Environment};

fn config_in(dir: &Path) -> Config {
    serde_json::from_value(json!({
        "sessions": "/none", "runs": "/none", "registry": "/none", "quests": "/none",
        "state": dir.join("state"), "zone": "UTC", "runtime": dir,
    }))
    .expect("a config this test writes reads")
}

/// The command run over a line, in a pipe, with its config in `dir`.
fn ran(dir: &Path, line: &[&str]) -> Ending {
    let config = dir.join("config.json");
    if !config.exists() {
        let written = json!({
            "sessions": "/none", "runs": "/none", "registry": dir, "quests": "/none",
            "state": dir.join("state"), "zone": "UTC", "runtime": dir,
        });
        std::fs::write(&config, written.to_string()).expect("the config is written");
    }
    let mut args = vec!["--config".to_owned(), config.display().to_string()];
    args.extend(line.iter().map(|word| (*word).to_owned()));
    let help = help();
    terminal_kit::run(
        &help,
        "0",
        &Environment::of([false; 3], &[]),
        args,
        |context| act(context, &help),
    )
}

// The daemon's last word on the budgets answers while it's within an
// idle wait and a minute, and not once it's older, from a shape this
// one doesn't write, or from the future, so a stopped daemon's file
// falls back to a pass of the command's own.
#[test]
fn the_budgets_are_answered_from_the_last_pass_while_recent() {
    let dir = tempfile::tempdir().expect("a folder");
    let config = config_in(dir.path());
    let at = crate::time::parse("2026-10-14T12:00:00Z").expect("an instant");
    assert_eq!(written_budgets(&config, at), None);
    let write = |v: u32| {
        let written = json!({"v": v, "at": "2026-10-14T12:00:00Z", "budgets": []});
        std::fs::write(dir.path().join("budgets.json"), written.to_string())
            .expect("the budgets are written");
    };
    write(crate::standing::VERSION);
    let answered = written_budgets(&config, at + WRITTEN_LASTS).expect("recent enough");
    assert_eq!(answered["budgets"], json!([]), "{answered}");
    assert_eq!(written_budgets(&config, at + WRITTEN_LASTS + 1.0), None);
    assert_eq!(written_budgets(&config, at - 1.0), None);
    write(crate::standing::VERSION + 1);
    assert_eq!(written_budgets(&config, at), None);
}

// A mistyped command, period or cut is the command line's mistake,
// and says the nearest it knows.
#[test]
fn a_wrong_word_is_a_usage_mistake_naming_the_nearest() {
    let dir = tempfile::tempdir().expect("a folder");
    let command = ran(dir.path(), &["budgts"]);
    assert_eq!(command.exit, Exit::Usage, "{}", command.stderr);
    assert!(command.stderr.contains("budgets"), "{}", command.stderr);
    let cut = ran(dir.path(), &["report", "last", "week", "by", "modle"]);
    assert_eq!(cut.exit, Exit::Usage, "{}", cut.stderr);
    assert!(cut.stderr.contains("model"), "{}", cut.stderr);
    let flag = ran(dir.path(), &["report", "--sesion", "x"]);
    assert_eq!(flag.exit, Exit::Usage, "{}", flag.stderr);
    assert!(flag.stderr.contains("--session"), "{}", flag.stderr);
}

// A command asked with the wrong words, or a flag meant for another,
// is refused before anything is read.
#[test]
fn a_command_asked_wrongly_is_refused() {
    let dir = tempfile::tempdir().expect("a folder");
    let extra = ran(dir.path(), &["refresh", "now"]);
    assert_eq!(extra.exit, Exit::Usage, "{}", extra.stderr);
    let session = ran(dir.path(), &["findings", "--session", "s1"]);
    assert_eq!(session.exit, Exit::Usage, "{}", session.stderr);
    let form = ran(dir.path(), &["budgets", "--format", "csv"]);
    assert_eq!(form.exit, Exit::Usage, "{}", form.stderr);
    let instant = ran(dir.path(), &["tokens", "replay", "noon", "later"]);
    assert_eq!(instant.exit, Exit::Usage, "{}", instant.stderr);
}

// A config that isn't there is missing, one that won't read is a
// config fault, and the file check-config is asked of is its input.
#[test]
fn each_config_fault_has_its_own_exit() {
    let dir = tempfile::tempdir().expect("a folder");
    let help = help();
    let missing = terminal_kit::run(
        &help,
        "0",
        &Environment::of([false; 3], &[("HOME", "/nonexistent")]),
        vec!["findings".to_owned()],
        |context| act(context, &help),
    );
    assert_eq!(missing.exit, Exit::NotFound, "{}", missing.stderr);
    let bad = dir.path().join("bad.json");
    std::fs::write(&bad, "{").expect("the bad config is written");
    let path = bad.display().to_string();
    let unread = ran(dir.path(), &["--config", &path, "findings"]);
    assert_eq!(unread.exit, Exit::Config, "{}", unread.stderr);
    let checked = ran(dir.path(), &["check-config", &path]);
    assert_eq!(checked.exit, Exit::BadInput, "{}", checked.stderr);
    let absent = ran(dir.path(), &["check-config", "/nonexistent/config.json"]);
    assert_eq!(absent.exit, Exit::NotFound, "{}", absent.stderr);
}

// A feed's line that breaks the contract is bad input, said with its
// line; a file that isn't there is missing.
#[test]
fn a_feed_breaking_the_contract_is_bad_input() {
    let dir = tempfile::tempdir().expect("a folder");
    let feed = dir.path().join("feed.jsonl");
    std::fs::write(&feed, "\nnot json\n").expect("the feed is written");
    let path = feed.display().to_string();
    let broken = ran(dir.path(), &["contract", "check", &path]);
    assert_eq!(broken.exit, Exit::BadInput, "{}", broken.stderr);
    assert!(broken.stderr.contains(":2"), "{}", broken.stderr);
    let absent = ran(dir.path(), &["contract", "check", "/nonexistent/feed"]);
    assert_eq!(absent.exit, Exit::NotFound, "{}", absent.stderr);
}

// Going to a session no longer open is its being missing, so the
// shell can tell an ended session from a terminal that failed.
#[test]
fn going_to_a_session_not_open_is_missing() {
    let dir = tempfile::tempdir().expect("a folder");
    let gone = ran(dir.path(), &["sessions", "01a1", "go"]);
    assert_eq!(gone.exit, Exit::NotFound, "{}", gone.stderr);
}

// The words notices wrote before each resource took its own command
// still answer, exactly as the new ones do, since a notice outlives a
// switch.
#[test]
fn the_old_words_answer_as_the_new_ones() {
    let dir = tempfile::tempdir().expect("a folder");
    for (old, new) in [
        (vec!["go", "01a1"], vec!["sessions", "01a1", "go"]),
        (
            vec!["findings", "expect", "burn:s1"],
            vec!["findings", "burn:s1", "expect"],
        ),
    ] {
        let (was, is) = (ran(dir.path(), &old), ran(dir.path(), &new));
        assert_eq!(
            (was.exit, &was.stdout, &was.stderr),
            (is.exit, &is.stdout, &is.stderr),
            "{old:?} should answer as {new:?} does"
        );
    }
}

// A session or a finding that isn't there is missing, and says where to
// look, rather than an empty answer.
#[test]
fn one_session_or_finding_not_there_is_missing() {
    let dir = tempfile::tempdir().expect("a folder");
    let session = ran(dir.path(), &["sessions", "01a1"]);
    assert_eq!(session.exit, Exit::NotFound, "{}", session.stderr);
    assert!(
        session.stderr.contains("spend-watch sessions"),
        "a missing session should say where to look: {}",
        session.stderr
    );
    let finding = ran(dir.path(), &["findings", "burn:s1"]);
    assert_eq!(finding.exit, Exit::NotFound, "{}", finding.stderr);
    assert!(
        finding.stderr.contains("spend-watch findings"),
        "a missing finding should say where to look: {}",
        finding.stderr
    );
}

// Bare `sessions` is the week cut by session, the report's own.
#[test]
fn the_sessions_are_the_week_by_session() {
    let dir = tempfile::tempdir().expect("a folder");
    let sessions = ran(dir.path(), &["sessions", "--json"]);
    let report = ran(dir.path(), &["report", "by", "session", "--json"]);
    assert_eq!(sessions.exit, Exit::Done, "{}", sessions.stderr);
    assert_eq!(
        sessions.stdout, report.stdout,
        "sessions should be the report by session"
    );
}

// Asking a daemon that isn't running is its being unavailable, so a
// caller can tell it from a failure of the ask itself.
#[test]
fn a_daemon_not_running_is_unavailable() {
    let dir = tempfile::tempdir().expect("a folder");
    let refreshed = ran(dir.path(), &["refresh", "--json"]);
    assert_eq!(refreshed.exit, Exit::Unavailable, "{}", refreshed.stderr);
    let said: Value = serde_json::from_str(&refreshed.stderr).expect("the error is JSON");
    assert_eq!(said["exit"], 69, "{said}");
}

// A second spend-watch can't import while one holds the history, and
// says to try again rather than that it failed.
#[test]
fn an_import_beside_a_running_daemon_is_to_try_again() {
    let dir = tempfile::tempdir().expect("a folder");
    let _held = crate::daemon::lock(&dir.path().join("state")).expect("the lock is free");
    let imported = ran(dir.path(), &["import"]);
    assert_eq!(imported.exit, Exit::TryAgain, "{}", imported.stderr);
}
