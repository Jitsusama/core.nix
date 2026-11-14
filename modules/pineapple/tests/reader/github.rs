//! pineapple-github against a stand-in for GitHub's API: a stop said on a
//! pull request reaches it and is kept in shadow, an ask without the
//! phrase stops nothing, a quoted one is waved off, and what doesn't name
//! the word or Joel isn't judged at all; it only asks, never posts nor
//! marks anything read, and a restart judges nothing twice. Joel's own
//! resume said there ends only shadow stops, even let go live.

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant};

use pineapple::github::stamp;
use serde_json::{Value, json};

use super::{
    PACE, PATIENCE, a_stop, answering, decisions, each, home, resumed, until_decided,
};

/// The stand-in's Joel on GitHub.
const LOGIN: &str = "joel.example";

/// One comment on example/shop#12, said `ago` seconds before `now`.
fn comment(id: u64, by: &str, body: &str, now: u64, ago: u64) -> Value {
    json!({"id": id, "body": body, "user": {"login": by, "type": "User"},
        "html_url": format!("https://code.example.com/example/shop/pull/12#c{id}"),
        "created_at": stamp(now - ago), "updated_at": stamp(now - ago)})
}

/// What GitHub would answer: Joel's notification for example/shop#12, an
/// hour-old opening, and on it, in the last minutes, a stop, a quoted
/// stop, a comment naming neither, and a review asking him to stop
/// without the phrase.
fn answer(line: &str, now: u64) -> Value {
    answer_with(line, now, &[
        comment(1, "colleague", &format!("@{LOGIN} pineapple override"), now, 90),
        comment(2, "colleague", &format!("> @{LOGIN} pineapple override\n\nsaw this"), now, 80),
        comment(3, "colleague", "looks good to me", now, 70),
    ])
}

/// GitHub's answer to one request, with `comments` on example/shop#12.
fn answer_with(line: &str, now: u64, comments: &[Value]) -> Value {
    if line.contains(" /user ") {
        return json!({"login": LOGIN});
    }
    if line.contains(" /notifications?") {
        return json!([{"subject": {"type": "PullRequest",
            "url": "https://api.example.com/repos/example/shop/pulls/12"},
            "repository": {"full_name": "example/shop"}}]);
    }
    if line.contains(" /repos/example/shop/issues/12/comments?") {
        return Value::Array(comments.to_vec());
    }
    if line.contains(" /repos/example/shop/issues/12 ") {
        return json!({"title": "Make checkout faster", "body": "",
            "html_url": "https://code.example.com/example/shop/pull/12",
            "user": {"login": "colleague", "type": "User"},
            "created_at": stamp(now - 3600)});
    }
    if line.contains(" /repos/example/shop/pulls/12/reviews?") {
        return json!([{"id": 9, "body": format!("@{LOGIN} can you please STOP your robots?"),
            "user": {"login": "colleague2", "type": "User"},
            "html_url": "https://code.example.com/example/shop/pull/12#r9",
            "submitted_at": stamp(now - 60)}]);
    }
    json!([])
}

/// What GitHub would answer when Joel himself says "pineapple resume" on
/// example/shop#12, a minute ago.
fn joel_resumes(line: &str, now: u64) -> Value {
    answer_with(line, now, &[comment(4, LOGIN, "pineapple resume", now, 60)])
}

/// The GitHub reader, signed in with a stand-in token, in shadow unless
/// `more` says otherwise.
fn github_reader(home: &Path, origin: &str, more: &[&str]) -> Child {
    Command::new(env!("CARGO_BIN_EXE_pineapple-github"))
        .args(["run", "--origin", origin, "--every", PACE])
        .args(more)
        .env("HOME", home)
        .env("XDG_STATE_HOME", home.join("state"))
        .env("SIGN_IN_KEYRING", keyring(home))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("the reader starts")
}

/// A keyring program printing a stand-in token, as gh's would print his.
fn keyring(home: &Path) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let path = home.join("github-sign-in");
    std::fs::write(&path, "#!/bin/sh\necho stand-in-token\n").expect("the program written");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))
        .expect("the program runs");
    path
}

/// Waits until the stand-in has been asked `more` requests beyond `from`.
fn until_asked(asked: &Mutex<Vec<String>>, from: usize, more: usize) {
    let began = Instant::now();
    while asked.lock().expect("what was asked").len() < from + more {
        assert!(began.elapsed() < PATIENCE, "the reader asked nothing more");
        // It asks every tenth of a second, so a short wait sees each round.
        thread::sleep(Duration::from_millis(50));
    }
}

#[test]
fn github_a_stop_on_a_pull_request_reaches_it_in_shadow_and_is_judged_once() {
    let (origin, asked) = answering(answer);
    let home = home();
    let mut first = github_reader(&home, &origin, &[]);
    until_decided(&home, 3);
    first.kill().expect("the reader stops");
    first.wait().expect("the reader is gone");

    let said = decisions(&home);
    assert_eq!(
        each(&said, "verdict"),
        ["stop", "waved:quoted", "stop-asked"],
        "oldest first; a comment naming neither isn't judged"
    );
    assert_eq!(
        said[0]["reaches"],
        json!(["pr:example/shop#12"]),
        "the pull request it was said on is what stops"
    );
    assert_eq!(said[0]["shadow"], true, "it is in shadow");
    assert_eq!(
        said[0]["gate"], "held:no-replies-here",
        "nobody is answered on GitHub"
    );
    assert_eq!(said[0]["thread"], "github:example/shop#12");
    let record: Value = serde_json::from_str(&only_stop(&home).expect("the stop's record"))
        .expect("the record is JSON");
    assert_eq!(record["shadow"], true, "the record says it is in shadow");
    assert_eq!(record["said_by"], "colleague", "it names who said it");

    let methods: Vec<String> = asked
        .lock()
        .expect("what was asked")
        .iter()
        .filter_map(|line| line.split_whitespace().next().map(str::to_owned))
        .collect();
    assert!(
        methods.iter().all(|method| method == "GET"),
        "it only asks: {methods:?}"
    );

    let mut again = github_reader(&home, &origin, &[]);
    let rounds = asked.lock().expect("what was asked").len();
    until_asked(&asked, rounds, 12);
    again.kill().expect("the reader stops");
    again.wait().expect("the reader is gone");
    assert_eq!(decisions(&home).len(), 3, "a restart judges nothing twice");
    let read = Command::new(env!("CARGO_BIN_EXE_pineapple-github"))
        .arg("--json")
        .env("XDG_STATE_HOME", home.join("state"))
        .output()
        .expect("it answers");
    let read: Value = serde_json::from_slice(&read.stdout).expect("its answer is JSON");
    assert!(read["read_at"].as_u64().is_some(), "it says when GitHub was read: {read}");
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}

/// The one stop's record, whatever its file is named.
fn only_stop(home: &Path) -> std::io::Result<String> {
    let mut stops = std::fs::read_dir(home.join("state/pineapple/stops"))?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()));
    let one = stops.next().ok_or(std::io::ErrorKind::NotFound)?;
    assert!(stops.next().is_none(), "one stop is kept");
    std::fs::read_to_string(one)
}

#[test]
fn github_joels_resume_ends_only_shadow_stops_even_let_go_live() {
    let (origin, asked) = answering(joel_resumes);
    let home = home();
    a_stop(&home, "named", "pr:example/shop#12", true, 120);
    a_stop(&home, "live", "pr:example/shop#12", false, 120);
    let mut reading = github_reader(&home, &origin, &["--live"]);
    until_decided(&home, 1);
    let began = Instant::now();
    while resumed(&home, "named").is_null() {
        assert!(began.elapsed() < PATIENCE, "Joel's resume ended nothing");
        // As above, a short wait sees it land without spinning.
        thread::sleep(Duration::from_millis(50));
    }
    reading.kill().expect("the reader stops");
    reading.wait().expect("the reader is gone");

    assert_eq!(decisions(&home)[0]["verdict"], "resume", "Joel's own resume");
    assert_eq!(
        resumed(&home, "live"),
        Value::Null,
        "his agents post as him on GitHub, so a live stop waits for his press"
    );
    assert!(
        asked.lock().expect("what was asked").iter().all(|line| line.starts_with("GET ")),
        "nobody is answered for a resume"
    );
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}
