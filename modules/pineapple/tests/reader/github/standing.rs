//! Who may stop Joel on GitHub, where any account may comment: a
//! stranger's stop is only logged; a collaborator's stands, but even an
//! owner's is only logged in a repository Joel can't push to unless he
//! opened what it's said on. An opening names where a stop reaches only
//! when the repository's own wrote it in one he works in; a repository
//! that can't be read still hears Joel; and one its rate limit refuses
//! is asked again rather than taken as gone. A thread with nothing worth
//! judging never asks about its repository, sparing Joel's rate limit.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use pineapple::github::stamp;
use serde_json::{Value, json};

use super::{LOGIN, answer_with, comment, commented, github_reader, only_stop, until_asked};
use crate::{Answered, answering, answering_whole, decisions, each, home, until_decided};

#[test]
fn github_a_strangers_stop_is_only_logged_and_a_collaborators_stands() {
    let (origin, _) = answering(|line, now| {
        answer_with(
            line,
            now,
            &[
                commented(
                    1,
                    "stranger",
                    "NONE",
                    &format!("@{LOGIN} pineapple override"),
                    (now, 90),
                ),
                commented(
                    2,
                    "passer-by",
                    "CONTRIBUTOR",
                    &format!("@{LOGIN} STOP your robots"),
                    (now, 85),
                ),
                comment(
                    3,
                    "colleague",
                    &format!("@{LOGIN} pineapple override"),
                    now,
                    80,
                ),
            ],
        )
    });
    let home = home();
    let mut reader = github_reader(&home, &origin, &["--live"]);
    until_decided(&home, 4);
    reader.kill().expect("the reader stops");
    reader.wait().expect("the reader is gone");

    let said = decisions(&home);
    assert_eq!(
        each(&said, "verdict"),
        ["waved:outsider", "waved:outsider", "stop", "stop-asked"],
        "a stranger's stop and ask are waved off; a collaborator's and the owner's heard"
    );
    assert_eq!(
        each(&said, "tell"),
        ["log", "log", "loud", "loud"],
        "a stranger sounds nothing"
    );
    let record: Value = serde_json::from_str(&only_stop(&home).expect("one stop's record"))
        .expect("the record is JSON");
    assert_eq!(
        record["said_by"], "colleague",
        "the one stop is the collaborator's"
    );
    assert_eq!(record["shadow"], false, "and it stands, live");
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}

/// What GitHub would answer when a stranger mentions Joel on an issue in
/// a repository of their own, which they own and he may only read.
fn in_a_strangers_own(line: &str, now: u64) -> Value {
    if line.contains(" /notifications?") {
        return json!([{"subject": {"type": "Issue",
            "url": "https://api.example.com/repos/stranger/trap/issues/1"},
            "repository": {"full_name": "stranger/trap"}}]);
    }
    if line.contains(" /repos/stranger/trap ") {
        return json!({"full_name": "stranger/trap", "permissions": {"pull": true, "push": false}});
    }
    if line.contains(" /repos/stranger/trap/issues/1/comments?") {
        let stop = format!("@{LOGIN} pineapple override");
        return json!([commented(1, "stranger", "OWNER", &stop, (now, 90))]);
    }
    if line.contains(" /repos/stranger/trap/issues/1 ") {
        return json!({"title": "Hello", "body": "",
            "html_url": "https://code.example.com/stranger/trap/issues/1",
            "user": {"login": "stranger", "type": "User"},
            "author_association": "OWNER", "created_at": stamp(now - 3600)});
    }
    answer_with(line, now, &[])
}

#[test]
fn github_an_owners_stop_in_a_repository_joel_cant_push_to_is_only_logged() {
    let (origin, _) = answering(in_a_strangers_own);
    let home = home();
    let mut reader = github_reader(&home, &origin, &["--live"]);
    until_decided(&home, 1);
    reader.kill().expect("the reader stops");
    reader.wait().expect("the reader is gone");

    let said = decisions(&home);
    assert_eq!(
        each(&said, "verdict"),
        ["waved:outsider"],
        "anyone may own a repository and mention him there"
    );
    assert_eq!(each(&said, "tell"), ["log"], "and it sounds nothing");
    assert!(only_stop(&home).is_err(), "nor is any stop kept");
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}

/// An issue, `repo`#1, as GitHub would answer for it: opened by `opened`
/// (its writer, their standing and its words), its repository as
/// `repo_answer` says, or gone when that's none, and `said` on it.
struct Issue {
    repo: &'static str,
    opened: (&'static str, &'static str, &'static str),
    repo_answer: Option<Value>,
    said: fn(u64) -> Vec<Value>,
}

impl Issue {
    fn answer(&self, line: &str, now: u64) -> Answered {
        let repo = self.repo;
        let (by, association, body) = self.opened;
        if line.contains(" /notifications?") {
            return Answered::ok(json!([{"subject": {"type": "Issue",
                "url": format!("https://api.example.com/repos/{repo}/issues/1")},
                "repository": {"full_name": repo}}]));
        }
        if line.contains(&format!(" /repos/{repo} ")) {
            return self.repo_answer.clone().map_or_else(
                || Answered {
                    status: 404,
                    headers: Vec::new(),
                    body: json!({"message": "Not Found"}),
                },
                Answered::ok,
            );
        }
        if line.contains(&format!(" /repos/{repo}/issues/1/comments?")) {
            return Answered::ok(Value::Array((self.said)(now)));
        }
        if line.contains(&format!(" /repos/{repo}/issues/1 ")) {
            return Answered::ok(json!({"title": "Broken", "body": body,
                "html_url": format!("https://code.example.com/{repo}/issues/1"),
                "user": {"login": by, "type": "User"},
                "author_association": association, "created_at": stamp(now - 3600)}));
        }
        Answered::ok(answer_with(line, now, &[]))
    }
}

/// What the GitHub reader, live, judges on `issue`, once it has judged
/// `count`.
fn judged_on(issue: Issue, count: usize) -> (Vec<Value>, PathBuf) {
    let (origin, _) = answering_whole(move |line, now| issue.answer(line, now));
    let home = home();
    let mut reader = github_reader(&home, &origin, &["--live"]);
    until_decided(&home, count);
    reader.kill().expect("the reader stops");
    reader.wait().expect("the reader is gone");
    (decisions(&home), home)
}

#[test]
fn github_a_maintainers_stop_where_joel_opened_an_issue_he_cant_push_to_stands() {
    let (said, home) = judged_on(
        Issue {
            repo: "upstream/project",
            opened: (LOGIN, "CONTRIBUTOR", "it crashes"),
            repo_answer: Some(json!({"permissions": {"pull": true, "push": false}})),
            said: |now| {
                let stop = format!("@{LOGIN} pineapple override");
                vec![commented(1, "maintainer", "MEMBER", &stop, (now, 90))]
            },
        },
        1,
    );
    assert_eq!(each(&said, "verdict"), ["stop"], "he chose to work there");
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}

#[test]
fn github_an_outsiders_opening_cant_steer_a_collaborators_stop() {
    let (said, home) = judged_on(
        Issue {
            repo: "example/shop",
            opened: (
                "stranger",
                "NONE",
                "see https://code.example.com/example/other/pull/3",
            ),
            repo_answer: Some(json!({"permissions": {"pull": true, "push": true}})),
            said: |now| {
                let stop = format!("@{LOGIN} pineapple override");
                vec![comment(1, "colleague", &stop, now, 90)]
            },
        },
        1,
    );
    assert_eq!(each(&said, "verdict"), ["stop"]);
    assert_eq!(
        said[0]["reaches"],
        json!(["everything"]),
        "not the pull request the stranger named"
    );
    assert_eq!(said[0]["fell_back"], true, "and Joel is told it fell back");
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}

#[test]
fn github_a_repository_that_cant_be_read_still_hears_joel_alone() {
    let (said, home) = judged_on(
        Issue {
            repo: "example/shop",
            opened: ("colleague", "MEMBER", ""),
            repo_answer: None,
            said: |now| {
                let stop = format!("@{LOGIN} pineapple override");
                vec![
                    comment(1, LOGIN, "pineapple override", now, 90),
                    comment(2, "colleague", &stop, now, 80),
                ]
            },
        },
        2,
    );
    assert_eq!(
        each(&said, "verdict"),
        ["stop", "waved:outsider"],
        "his own stop still stands; no one else's counts unasked"
    );
    assert_eq!(said[0]["by_joel"], true);
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}

#[test]
fn github_an_opening_by_the_repositorys_own_names_where_a_stop_reaches() {
    let (said, home) = judged_on(
        Issue {
            repo: "example/shop",
            opened: (
                "colleague",
                "MEMBER",
                "see https://code.example.com/example/other/pull/3",
            ),
            repo_answer: Some(json!({"permissions": {"pull": true, "push": true}})),
            said: |now| {
                let stop = format!("@{LOGIN} pineapple override");
                vec![comment(1, "colleague2", &stop, now, 90)]
            },
        },
        1,
    );
    assert_eq!(each(&said, "verdict"), ["stop"]);
    assert_eq!(
        said[0]["reaches"],
        json!(["pr:example/other#3"]),
        "the pull request a colleague's opening names"
    );
    assert_eq!(said[0]["fell_back"], false, "so nothing fell back");
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}

#[test]
fn github_an_owners_opening_where_joel_cant_push_cant_steer_his_own_stop() {
    let (said, home) = judged_on(
        Issue {
            repo: "stranger/trap",
            opened: (
                "stranger",
                "OWNER",
                "see https://code.example.com/example/other/pull/3",
            ),
            repo_answer: Some(json!({"permissions": {"pull": true, "push": false}})),
            said: |now| vec![comment(1, LOGIN, "pineapple override", now, 90)],
        },
        1,
    );
    assert_eq!(each(&said, "verdict"), ["stop"], "his own stop stands");
    assert_eq!(
        said[0]["reaches"],
        json!(["everything"]),
        "not the pull request the owner of a repository he can't push to named"
    );
    assert_eq!(said[0]["fell_back"], true, "and Joel is told it fell back");
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}

#[test]
fn github_a_thread_with_nothing_worth_judging_never_asks_about_its_repository() {
    let issue = Issue {
        repo: "example/shop",
        opened: ("colleague", "MEMBER", ""),
        repo_answer: Some(json!({"permissions": {"pull": true, "push": true}})),
        said: |now| vec![comment(1, "colleague", "lunch at noon?", now, 90)],
    };
    let (origin, asked) = answering_whole(move |line, now| issue.answer(line, now));
    let home = home();
    let mut reader = github_reader(&home, &origin, &["--live"]);
    // Two rounds, each asking at least for what changed, the issue and its comments.
    until_asked(&asked, 0, 8);
    reader.kill().expect("the reader stops");
    reader.wait().expect("the reader is gone");

    let asked = asked.lock().expect("what was asked");
    assert!(
        asked
            .iter()
            .any(|line| line.contains(" /repos/example/shop/issues/1/comments?")),
        "the thread was read: {asked:?}"
    );
    assert!(
        !asked
            .iter()
            .any(|line| line.contains(" /repos/example/shop ")),
        "but its repository was never asked about: {asked:?}"
    );
    drop(asked);
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}

#[test]
fn github_a_repository_its_rate_limit_refuses_is_asked_again_not_taken_as_gone() {
    let issue = Issue {
        repo: "example/shop",
        opened: ("colleague", "MEMBER", ""),
        repo_answer: Some(json!({"permissions": {"pull": true, "push": true}})),
        said: |now| {
            let stop = format!("@{LOGIN} pineapple override");
            vec![comment(1, "colleague2", &stop, now, 90)]
        },
    };
    let refused = Arc::new(AtomicBool::new(false));
    let once = Arc::clone(&refused);
    let (origin, _) = answering_whole(move |line, now| {
        let repository = line.contains(" /repos/example/shop ");
        if repository && !once.swap(true, Ordering::SeqCst) {
            // A secondary limit may say so only in its words.
            return Answered {
                status: 403,
                headers: Vec::new(),
                body: json!({"message": "You have exceeded a secondary rate limit."}),
            };
        }
        issue.answer(line, now)
    });
    let home = home();
    let mut reader = github_reader(&home, &origin, &["--live"]);
    until_decided(&home, 1);
    reader.kill().expect("the reader stops");
    reader.wait().expect("the reader is gone");

    let said = decisions(&home);
    assert!(refused.load(Ordering::SeqCst), "the limit refused it once");
    assert_eq!(
        each(&said, "verdict"),
        ["stop"],
        "a collaborator's stop, read once the limit let it be, not set aside"
    );
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}
