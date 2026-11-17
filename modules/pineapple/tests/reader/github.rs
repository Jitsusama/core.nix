//! pineapple-github against a stand-in for GitHub's API: a stop said on a
//! pull request reaches it and is kept in shadow, an ask without the
//! phrase stops nothing, a quoted one is waved off, and what doesn't name
//! the word or Joel isn't judged at all; it only asks, never posts nor
//! marks anything read, and a restart judges nothing twice. Joel's own
//! resume said there ends only shadow stops, even let go live. A round
//! GitHub's rate limit refuses is read again rather than skipped, and a
//! list's later pages are read too. A site with no notifications is read
//! by searching for what Joel opened, its keys its own, from far enough
//! back that a search's lagging index misses nothing, and never for more
//! repositories than its searches allow, and its journal names it, not
//! GitHub, when it can't be read. A site of a company's own weighs
//! no writer's standing; who may stop Joel on GitHub itself, where any
//! account may comment, is in standing.rs.

mod standing;

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

use pineapple::github::stamp;
use serde_json::{Value, json};

use super::{
    Answered, PACE, PATIENCE, a_stop, answering, answering_whole, decisions, each, home, resumed,
    until_decided,
};

/// The stand-in's Joel on GitHub.
const LOGIN: &str = "joel.example";

/// One comment on example/shop#12, said `ago` seconds before `now` by a
/// collaborator on it.
fn comment(id: u64, by: &str, body: &str, now: u64, ago: u64) -> Value {
    commented(id, by, "COLLABORATOR", body, (now, ago))
}

/// One comment on example/shop#12 by a writer GitHub names as
/// `association` to the repository, said `ago` seconds before `now`.
fn commented(id: u64, by: &str, association: &str, body: &str, when: (u64, u64)) -> Value {
    let (now, ago) = when;
    json!({"id": id, "body": body, "user": {"login": by, "type": "User"},
        "author_association": association,
        "html_url": format!("https://code.example.com/example/shop/pull/12#c{id}"),
        "created_at": stamp(now - ago), "updated_at": stamp(now - ago)})
}

/// What GitHub would answer: Joel's notification for example/shop#12, an
/// hour-old opening, and on it, in the last minutes, a stop, a quoted
/// stop, a comment naming neither, and a review asking him to stop
/// without the phrase.
fn answer(line: &str, now: u64) -> Value {
    answer_with(
        line,
        now,
        &[
            comment(
                1,
                "colleague",
                &format!("@{LOGIN} pineapple override"),
                now,
                90,
            ),
            comment(
                2,
                "colleague",
                &format!("> @{LOGIN} pineapple override\n\nsaw this"),
                now,
                80,
            ),
            comment(3, "colleague", "looks good to me", now, 70),
        ],
    )
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
    if line.contains(" /repos/example/shop ") {
        return json!({"full_name": "example/shop", "permissions": {"pull": true, "push": true}});
    }
    if line.contains(" /repos/example/shop/issues/12/comments?") {
        return Value::Array(comments.to_vec());
    }
    if line.contains(" /repos/example/shop/issues/12 ") {
        return json!({"title": "Make checkout faster", "body": "",
            "html_url": "https://code.example.com/example/shop/pull/12",
            "user": {"login": "colleague", "type": "User"},
            "author_association": "MEMBER", "created_at": stamp(now - 3600)});
    }
    if line.contains(" /repos/example/shop/pulls/12/reviews?") {
        return json!([{"id": 9, "body": format!("@{LOGIN} can you please STOP your robots?"),
            "user": {"login": "colleague2", "type": "User"}, "author_association": "OWNER",
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
/// `more` says otherwise, its journal kept in `home` for [`journal`].
fn github_reader(home: &Path, origin: &str, more: &[&str]) -> Child {
    // Added to, so a reader started again in the same home keeps what the
    // one before it said.
    let journal = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(home.join(JOURNAL))
        .expect("its journal is opened");
    Command::new(env!("CARGO_BIN_EXE_pineapple-github"))
        .args(["run", "--origin", origin, "--every", PACE])
        .args(more)
        .env("HOME", home)
        .env("XDG_STATE_HOME", home.join("state"))
        .env("SIGN_IN_KEYRING", keyring(home))
        .stdout(Stdio::null())
        .stderr(journal)
        .spawn()
        .expect("the reader starts")
}

/// Where in its home a reader's journal is kept.
const JOURNAL: &str = "journal";

/// What the reader started in `home` has said to its journal so far.
fn journal(home: &Path) -> String {
    std::fs::read_to_string(home.join(JOURNAL)).expect("its journal is read")
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
    assert!(
        read["read_at"].as_u64().is_some(),
        "it says when GitHub was read: {read}"
    );
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}

#[test]
fn github_a_round_its_rate_limit_refuses_is_read_again_not_skipped() {
    let refused = Arc::new(AtomicBool::new(false));
    let once = Arc::clone(&refused);
    let (origin, _) = answering_whole(move |line, now| {
        let comments = line.contains(" /repos/example/shop/issues/12/comments?");
        if comments && !once.swap(true, Ordering::SeqCst) {
            return Answered {
                status: 403,
                headers: vec![("x-ratelimit-remaining", "0".to_owned())],
                body: json!({"message": "API rate limit exceeded"}),
            };
        }
        Answered::ok(answer(line, now))
    });
    let home = home();
    let mut reading = github_reader(&home, &origin, &[]);
    until_decided(&home, 3);
    reading.kill().expect("the reader stops");
    reading.wait().expect("the reader is gone");

    assert!(refused.load(Ordering::SeqCst), "the limit was met");
    assert_eq!(
        each(&decisions(&home), "verdict"),
        ["stop", "waved:quoted", "stop-asked"],
        "what was said where the limit refused is judged the round after"
    );
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}

#[test]
fn github_a_second_page_of_notifications_is_read_too() {
    let at: Arc<OnceLock<String>> = Arc::new(OnceLock::new());
    let own = Arc::clone(&at);
    let (origin, asked) = answering_whole(move |line, now| {
        if line.contains(" /notifications?page=2 ") {
            return Answered::ok(answer(" /notifications? ", now));
        }
        if line.contains(" /notifications?") {
            // Known before the reader starts, so before it asks.
            let origin = own.get().expect("the stand-in's origin");
            let next = format!("<{origin}/notifications?page=2>; rel=\"next\"");
            return Answered {
                status: 200,
                headers: vec![("Link", next)],
                body: json!([{"subject": {"type": "Issue",
                    "url": "https://api.example.com/repos/example/other/issues/5"},
                    "repository": {"full_name": "example/other"}}]),
            };
        }
        Answered::ok(answer(line, now))
    });
    at.set(origin.clone()).expect("the origin is known once");
    let home = home();
    let mut reading = github_reader(&home, &origin, &[]);
    until_decided(&home, 3);
    reading.kill().expect("the reader stops");
    reading.wait().expect("the reader is gone");

    assert_eq!(
        each(&decisions(&home), "verdict"),
        ["stop", "waved:quoted", "stop-asked"],
        "the pull request on the second page is read"
    );
    let asked = asked.lock().expect("what was asked");
    assert!(
        asked
            .iter()
            .any(|line| line.contains("/repos/example/other/issues/5 ")),
        "the first page's issue is read too: {asked:?}"
    );
    drop(asked);
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}

/// What a site with no notifications would answer, where logins are
/// addresses: Joel's one pull request, example/shop#12, and on it a
/// colleague's stop, a check's, and Joel's own resume, each by a writer
/// GitHub would call a stranger, which such a site doesn't weigh.
fn by_address(line: &str, now: u64) -> Value {
    if line.contains(" /search/issues?") {
        return json!({"items": [{"number": 12, "pull_request": {},
            "repository_url": "https://code.example.com/api/v3/repos/example/shop"}]});
    }
    if line.contains(" /user ") || line.contains(" /notifications?") {
        return json!({"message": "not here"});
    }
    answer_with(
        line,
        now,
        &[
            commented(
                1,
                "colleague@example.com",
                "NONE",
                &format!("@{LOGIN} pineapple override"),
                (now, 90),
            ),
            commented(
                2,
                "checks@example.iam.gserviceaccount.com",
                "NONE",
                "pineapple override",
                (now, 80),
            ),
            commented(3, "joel@example.com", "NONE", "pineapple resume", (now, 70)),
        ],
    )
}

#[test]
fn github_a_site_with_no_notifications_is_read_by_what_joel_opened() {
    let (origin, asked) = answering(by_address);
    let home = home();
    let site = [
        "--site",
        "code",
        "--authored-by",
        "joel@example.com",
        "--login",
        LOGIN,
        "--repo",
        "example/shop",
    ];
    let mut reading = github_reader(&home, &origin, &site);
    until_decided(&home, 4);
    reading.kill().expect("the reader stops");
    reading.wait().expect("the reader is gone");

    let said = decisions(&home);
    assert_eq!(
        each(&said, "verdict"),
        ["stop", "waved:bot", "resume", "stop-asked"],
        "a check's account is a bot's, and Joel is known by his address"
    );
    assert_eq!(
        said[0]["thread"], "code:example/shop#12",
        "its site names it"
    );
    assert_eq!(said[2]["by_joel"], true, "his own resume");
    let asked = asked.lock().expect("what was asked");
    assert!(
        asked
            .iter()
            .all(|line| !line.contains(" /user ") && !line.contains("/notifications")),
        "it asks neither who Joel is nor what notified him: {asked:?}"
    );
    assert!(
        asked
            .iter()
            .any(|line| line.contains("repo%3Aexample%2Fshop%20author%3Ajoel%40example.com")),
        "it searches the repository named, by his address: {asked:?}"
    );
    drop(asked);
    let read = Command::new(env!("CARGO_BIN_EXE_pineapple-github"))
        .arg("--json")
        .env("XDG_STATE_HOME", home.join("state"))
        .output()
        .expect("it answers");
    let read: Value = serde_json::from_slice(&read.stdout).expect("its answer is JSON");
    assert!(
        read["sites"]["code"].as_u64().is_some(),
        "it says when the site was read: {read}"
    );
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}

#[test]
fn github_a_site_is_searched_from_ten_minutes_before_its_last_round() {
    let (origin, asked) = answering(by_address);
    let home = home();
    let last = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("after 1970")
        .as_secs()
        - 3600;
    let state = home.join("state/pineapple");
    std::fs::create_dir_all(&state).expect("the state folder");
    let judged = json!({"since": last, "keys": []});
    std::fs::write(state.join("judged-code.json"), judged.to_string()).expect("the mark");
    let site = [
        "--site",
        "code",
        "--authored-by",
        "joel@example.com",
        "--login",
        LOGIN,
        "--repo",
        "example/shop",
    ];
    let mut reading = github_reader(&home, &origin, &site);
    until_asked(&asked, 0, 1);
    reading.kill().expect("the reader stops");
    reading.wait().expect("the reader is gone");

    let from = format!("updated%3A%3E%3D{}", stamp(last - 600).replace(':', "%3A"));
    let first = asked.lock().expect("what was asked")[0].clone();
    assert!(
        first.contains(&from),
        "a search's index lags, so it asks from ten minutes before: {first}"
    );
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}

/// What the reader of the site "code" at `origin` says to its journal
/// once it has said `failed`, or gives up waiting for it.
fn journal_of_code_once(origin: &str, failed: &str) -> String {
    let site = [
        "--site",
        "code",
        "--authored-by",
        "joel@example.com",
        "--login",
        LOGIN,
        "--repo",
        "example/shop",
    ];
    journal_once(origin, &site, failed)
}

/// That `said` names the site "code" as the one that failed, in `failed`,
/// and never GitHub.
fn named_as_itself(said: &str, failed: &str) {
    assert!(
        said.contains(failed),
        "the site that failed is named: {said}"
    );
    assert!(
        !said.contains("GitHub"),
        "GitHub isn't named on this path: {said}"
    );
}

/// What a reader at `origin`, with `more` words, says to its journal once
/// it has said `failed`, or gives up waiting for it.
fn journal_once(origin: &str, more: &[&str], failed: &str) -> String {
    let home = home();
    let mut reading = github_reader(&home, origin, more);
    let began = Instant::now();
    while !journal(&home).contains(failed) && began.elapsed() < PATIENCE {
        thread::sleep(Duration::from_millis(50));
    }
    reading.kill().expect("the reader stops");
    reading.wait().expect("the reader is gone");
    let said = journal(&home);
    std::fs::remove_dir_all(&home).expect("the case's home goes");
    said
}

#[test]
fn github_a_site_that_answers_with_an_error_is_named_as_itself() {
    let (origin, _) = answering_whole(|_, _| Answered {
        status: 500,
        headers: Vec::new(),
        body: json!({"message": "Server Error"}),
    });
    let failed = "<4>couldn't ask Code what changed: Code answered 500 for /search/issues";
    let said = journal_of_code_once(&origin, failed);
    named_as_itself(&said, failed);
}

#[test]
fn github_a_site_that_cant_be_reached_is_named_as_itself() {
    // No case's stand-in can take a port below 1024, so nothing answers here.
    let failed = "<4>couldn't ask Code what changed: couldn't reach Code:";
    let said = journal_of_code_once("http://127.0.0.1:1", failed);
    named_as_itself(&said, failed);
}

#[test]
fn github_the_default_site_that_answers_with_an_error_is_named_github() {
    let (origin, _) = answering_whole(|_, _| Answered {
        status: 500,
        headers: Vec::new(),
        body: json!({"message": "Server Error"}),
    });
    // With no site named, the reader reads GitHub, and says so when it fails.
    let failed = "<4>couldn't ask GitHub who Joel is: GitHub answered 500 for /user";
    let said = journal_once(&origin, &[], failed);
    assert!(
        said.contains(failed),
        "GitHub is named as the site that failed: {said}"
    );
}

/// A stand-in whose every answer names a next page at `next(origin)`.
fn paging(next: fn(&str) -> String) -> String {
    let at: Arc<OnceLock<String>> = Arc::new(OnceLock::new());
    let own = Arc::clone(&at);
    let (origin, _) = answering_whole(move |_, _| {
        // Known before the reader starts, so before it asks.
        let origin = own.get().expect("the stand-in's origin");
        Answered {
            status: 200,
            headers: vec![("Link", format!("<{}>; rel=\"next\"", next(origin)))],
            body: json!({"items": []}),
        }
    });
    at.set(origin.clone()).expect("the origin is known once");
    origin
}

#[test]
fn github_a_site_naming_a_next_page_elsewhere_is_named_as_itself() {
    let origin = paging(|_| "http://evil.example/search/issues?page=2".to_owned());
    let failed = "<4>couldn't ask Code what changed: Code named a next page elsewhere: \
        http://evil.example/search/issues?page=2";
    let said = journal_of_code_once(&origin, failed);
    named_as_itself(&said, failed);
}

#[test]
fn github_a_site_whose_search_runs_on_is_named_as_itself() {
    // Every page names another after it, so the list never ends.
    let origin = paging(|origin| format!("{origin}/search/issues?page=2"));
    let failed = "<4>Code's /search/issues runs past 30 pages, and the rest isn't read";
    let said = journal_of_code_once(&origin, failed);
    assert!(
        said.contains(failed),
        "the site that ran on is named: {said}"
    );
    assert!(
        !said.contains("GitHub"),
        "GitHub isn't named on this path: {said}"
    );
}

#[test]
fn github_more_repositories_than_a_sites_searches_allow_are_refused() {
    let (origin, _) = answering(by_address);
    let home = home();
    let mut site = vec![
        "--site".to_owned(),
        "code".to_owned(),
        "--authored-by".to_owned(),
        "joel@example.com".to_owned(),
        "--login".to_owned(),
        LOGIN.to_owned(),
    ];
    for n in 0..21 {
        site.extend(["--repo".to_owned(), format!("example/repo{n}")]);
    }
    let site: Vec<&str> = site.iter().map(String::as_str).collect();
    // A reader that took them would run on, so it is waited on only so long.
    let mut reading = github_reader(&home, &origin, &site);
    let began = Instant::now();
    let status = loop {
        if let Some(status) = reading.try_wait().expect("the reader is asked") {
            break Some(status);
        }
        if began.elapsed() > PATIENCE {
            reading.kill().expect("the reader stops");
            reading.wait().expect("the reader is gone");
            break None;
        }
        thread::sleep(Duration::from_millis(50));
    };
    std::fs::remove_dir_all(&home).expect("the case's home goes");
    assert_eq!(
        status.and_then(|status| status.code()),
        Some(64),
        "more repositories than its searches allow are a usage error"
    );
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

    assert_eq!(
        decisions(&home)[0]["verdict"],
        "resume",
        "Joel's own resume"
    );
    assert_eq!(
        resumed(&home, "live"),
        Value::Null,
        "his agents post as him on GitHub, so a live stop waits for his press"
    );
    assert!(
        asked
            .lock()
            .expect("what was asked")
            .iter()
            .all(|line| line.starts_with("GET ")),
        "nobody is answered for a resume"
    );
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}
