//! Who may stop Joel on GitHub, where any account may comment: a
//! stranger's stop is only logged; a collaborator's stands, but even an
//! owner's is only logged in a repository Joel can't push to unless he
//! opened what it's said on. An opening names where a stop reaches only
//! when the repository's own wrote it in one he works in; a repository
//! that can't be read still hears Joel, and is asked about and said to
//! the journal once a round; and one its rate limit refuses
//! is asked again rather than taken as gone. A thread with nothing the
//! round judges never asks about its repository, and a round asks about
//! each repository once, sparing Joel's rate limit.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use pineapple::github::stamp;
use serde_json::{Value, json};

use super::{
    LOGIN, answer_with, comment, commented, github_reader, journal, only_stop, until_asked,
};
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
    let home = decided_live(&origin, 4);

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
    let home = decided_live(&origin, 1);

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
    judged_opened_as(issue, |_, _| {}, count)
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

/// An issue in example/shop, a repository Joel works in, opened by a
/// stranger naming example/other#3, and a stop on it from a colleague
/// GitHub names as the repository's collaborator, which is why the stop
/// stands in both cases that read it.
fn steered_by_a_stranger() -> Issue {
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
    }
}

#[test]
fn github_an_outsiders_opening_cant_steer_a_collaborators_stop() {
    let (said, home) = judged_on(steered_by_a_stranger(), 1);
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

/// The home of a GitHub reader that ran live against `origin` until it had
/// judged `count`, then was stopped.
fn decided_live(origin: &str, count: usize) -> PathBuf {
    let home = home();
    let mut reader = github_reader(&home, origin, &["--live"]);
    until_decided(&home, count);
    reader.kill().expect("the reader stops");
    reader.wait().expect("the reader is gone");
    home
}

/// What the GitHub reader, live, judges on `issue` once it has judged
/// `count`, its opening answered as `opening` makes it from `issue`'s.
fn judged_opened_as(
    issue: Issue,
    opening: fn(&mut Value, u64),
    count: usize,
) -> (Vec<Value>, PathBuf) {
    let line_of_it = format!(" /repos/{}/issues/1 ", issue.repo);
    let (origin, _) = answering_whole(move |line, now| {
        let mut answered = issue.answer(line, now);
        if line.contains(&line_of_it) {
            opening(&mut answered.body, now);
        }
        answered
    });
    let home = decided_live(&origin, count);
    (decisions(&home), home)
}

#[test]
fn github_an_opening_that_cant_be_read_is_vouched_for_by_no_one_said_after_it() {
    let (said, home) = judged_opened_as(
        steered_by_a_stranger(),
        // Without its time, the opening makes no saying of its own.
        |opening, _| {
            opening
                .as_object_mut()
                .expect("an opening is an object")
                .remove("created_at");
        },
        1,
    );
    assert_eq!(each(&said, "verdict"), ["stop"], "the collaborator's stop");
    assert_eq!(
        said[0]["reaches"],
        json!(["everything"]),
        "not the pull request the stranger's opening named"
    );
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}

#[test]
fn github_a_members_stop_in_an_opening_alone_is_heard() {
    let (said, home) = judged_opened_as(
        Issue {
            repo: "example/shop",
            opened: ("colleague", "MEMBER", ""),
            repo_answer: Some(json!({"permissions": {"pull": true, "push": true}})),
            said: |now| vec![comment(1, "colleague2", "lunch at noon?", now, 80)],
        },
        |opening, now| {
            opening["body"] = json!(format!("@{LOGIN} pineapple override"));
            opening["created_at"] = json!(stamp(now - 90));
        },
        1,
    );
    assert_eq!(
        each(&said, "verdict"),
        ["stop"],
        "a member's stop in the opening is heard, though nothing after it says a word"
    );
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}

/// What the GitHub reader asks of the site answering as `answered` in
/// two rounds, each asking at least for what changed, the thread and its
/// comments.
fn asked_of(answered: impl Fn(&str, u64) -> Answered + Send + 'static) -> Vec<String> {
    let (origin, asked) = answering_whole(answered);
    let home = home();
    let mut reader = github_reader(&home, &origin, &["--live"]);
    until_asked(&asked, 0, 8);
    reader.kill().expect("the reader stops");
    reader.wait().expect("the reader is gone");
    std::fs::remove_dir_all(&home).expect("the case's home goes");
    let asked: &Mutex<Vec<String>> = &asked;
    asked.lock().expect("what was asked").clone()
}

/// Whether its repository was ever asked about, failing the case unless
/// its thread was read at all.
fn read_and_asked_about_repository(asked: &[String]) -> bool {
    assert!(
        asked
            .iter()
            .any(|line| line.contains(" /repos/example/shop/issues/1/comments?")),
        "the thread was read: {asked:?}"
    );
    asked
        .iter()
        .any(|line| line.contains(" /repos/example/shop "))
}

#[test]
fn github_a_thread_with_nothing_worth_judging_never_asks_about_its_repository() {
    let issue = Issue {
        repo: "example/shop",
        opened: ("colleague", "MEMBER", ""),
        repo_answer: Some(json!({"permissions": {"pull": true, "push": true}})),
        said: |now| vec![comment(1, "colleague", "lunch at noon?", now, 90)],
    };
    let asked = asked_of(move |line, now| issue.answer(line, now));
    assert!(
        !read_and_asked_about_repository(&asked),
        "its repository was never asked about: {asked:?}"
    );
}

#[test]
fn github_a_stop_too_old_to_judge_never_asks_about_its_repository() {
    let issue = Issue {
        repo: "example/shop",
        // An hour old, as every opening here is: long before what a round reads.
        opened: ("colleague", "MEMBER", "@joel.example pineapple override"),
        repo_answer: Some(json!({"permissions": {"pull": true, "push": true}})),
        said: |now| vec![comment(1, "colleague", "lunch at noon?", now, 90)],
    };
    assert_eq!(LOGIN, "joel.example", "the opening tags Joel by his login");
    let asked = asked_of(move |line, now| issue.answer(line, now));
    assert!(
        !read_and_asked_about_repository(&asked),
        "a stop no round judges spends nothing: {asked:?}"
    );
}

#[test]
fn github_an_ask_to_stop_said_only_in_a_review_is_heard() {
    let (origin, asked) = answering(|line, now| {
        answer_with(
            line,
            now,
            &[comment(1, "colleague", "lunch at noon?", now, 90)],
        )
    });
    let home = decided_live(&origin, 1);

    let said = decisions(&home);
    assert_eq!(
        each(&said, "verdict"),
        ["stop-asked"],
        "the owner's review asking him to stop is heard, not waved off as an outsider's"
    );
    let asked: &Mutex<Vec<String>> = &asked;
    let asked = asked.lock().expect("what was asked").clone();
    assert!(
        asked
            .iter()
            .any(|line| line.contains(" /repos/example/shop ")),
        "its repository was asked about: {asked:?}"
    );
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}

/// GitHub's answer when two issues in example/shop each carry a
/// collaborator's stop.
fn two_stops_in_one_repository(line: &str, now: u64) -> Answered {
    if line.contains(" /notifications?") {
        let issue = |number: u64| {
            json!({"subject": {"type": "Issue",
                "url": format!("https://api.example.com/repos/example/shop/issues/{number}")},
                "repository": {"full_name": "example/shop"}})
        };
        return Answered::ok(json!([issue(1), issue(2)]));
    }
    for number in [1, 2] {
        let thread = format!(" /repos/example/shop/issues/{number}");
        if line.contains(&format!("{thread}/comments?")) {
            let stop = format!("@{LOGIN} pineapple override");
            return Answered::ok(json!([comment(number, "colleague", &stop, now, 90)]));
        }
        if line.contains(&format!("{thread} ")) {
            return Answered::ok(json!({"title": "Broken", "body": "",
                "html_url": format!("https://code.example.com/example/shop/issues/{number}"),
                "user": {"login": "colleague", "type": "User"},
                "author_association": "MEMBER", "created_at": stamp(now - 3600)}));
        }
    }
    Answered::ok(answer_with(line, now, &[]))
}

/// GitHub's answer for two collaborators' stops in example/shop, on two
/// issues, with the repository's own details refused.
fn two_stops_in_an_unreadable_repository(line: &str, now: u64) -> Answered {
    if line.contains(" /repos/example/shop ") {
        return Answered {
            status: 404,
            headers: Vec::new(),
            body: json!({"message": "Not Found"}),
        };
    }
    two_stops_in_one_repository(line, now)
}

#[test]
fn github_a_repository_that_cant_be_read_is_asked_and_told_of_once_in_a_round() {
    let (origin, asked) = answering_whole(two_stops_in_an_unreadable_repository);
    let home = decided_live(&origin, 2);

    assert_eq!(
        each(&decisions(&home), "verdict"),
        ["waved:outsider", "waved:outsider"],
        "collaborators' stops on what Joel didn't open are an outsider's where the repository \
         can't be read"
    );
    let asked: &Mutex<Vec<String>> = &asked;
    let asked = asked.lock().expect("what was asked").clone();
    let times = asked
        .iter()
        .filter(|line| line.contains(" /repos/example/shop "))
        .count();
    assert_eq!(times, 1, "its repository was asked about once: {asked:?}");
    let said = journal(&home);
    let warning = "<4>GitHub answered 404 for /repos/example/shop: Not Found, \
                   so a stop there by anyone but Joel counts this round \
                   only on what he opened";
    let told = said.lines().filter(|line| *line == warning).count();
    assert_eq!(
        told, 1,
        "its journal warns of it once, with GitHub's reason: {said}"
    );
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}

/// GitHub's answer when a collaborator stops Joel in example/shop, where
/// he works, and its owner in stranger/trap, where he doesn't, both in
/// one round.
fn stops_in_two_repositories(line: &str, now: u64) -> Answered {
    if line.contains(" /notifications?") {
        return Answered::ok(json!([
            {"subject": {"type": "Issue",
                "url": "https://api.example.com/repos/example/shop/issues/1"},
                "repository": {"full_name": "example/shop"}},
            {"subject": {"type": "Issue",
                "url": "https://api.example.com/repos/stranger/trap/issues/1"},
                "repository": {"full_name": "stranger/trap"}},
        ]));
    }
    if line.contains(" /repos/stranger/") {
        return Answered::ok(in_a_strangers_own(line, now));
    }
    two_stops_in_one_repository(line, now)
}

#[test]
fn github_what_a_round_learns_of_one_repository_never_vouches_for_another() {
    let (origin, _) = answering_whole(stops_in_two_repositories);
    let home = decided_live(&origin, 2);

    let said = decisions(&home);
    let mut by_repository: Vec<(String, String)> = said
        .iter()
        .map(|one| {
            let key = one["key"].as_str().unwrap_or_default();
            let repository = ["example/shop", "stranger/trap"]
                .into_iter()
                .find(|name| key.contains(&format!("{name}#")) || key.contains(&format!("{name}:")))
                .unwrap_or(key);
            let verdict = one["verdict"].as_str().unwrap_or_default();
            (repository.to_owned(), verdict.to_owned())
        })
        .collect();
    by_repository.sort();
    assert_eq!(
        by_repository,
        [
            ("example/shop".to_owned(), "stop".to_owned()),
            ("stranger/trap".to_owned(), "waved:outsider".to_owned()),
        ],
        "the collaborator's stop where Joel works stands; \
         the owner's where he can't push is only logged"
    );
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}

#[test]
fn github_a_repository_joel_is_let_into_is_heard_the_round_after() {
    let let_in = Arc::new(AtomicBool::new(false));
    let once = Arc::clone(&let_in);
    let (origin, _) = answering_whole(move |line, now| {
        if line.contains(" /notifications?") {
            return Answered::ok(json!([{"subject": {"type": "Issue",
                "url": "https://api.example.com/repos/example/shop/issues/1"},
                "repository": {"full_name": "example/shop"}}]));
        }
        if line.contains(" /repos/example/shop ") {
            // The first round finds him only reading; by the next, he may push.
            let push = once.swap(true, Ordering::SeqCst);
            return Answered::ok(json!({"permissions": {"pull": true, "push": push}}));
        }
        if line.contains(" /repos/example/shop/issues/1/comments?") {
            let stop = format!("@{LOGIN} pineapple override");
            let mut said = vec![comment(1, "colleague", &stop, now, 90)];
            if once.load(Ordering::SeqCst) {
                said.push(comment(2, "colleague", &stop, now, 5));
            }
            return Answered::ok(Value::Array(said));
        }
        two_stops_in_one_repository(line, now)
    });
    let home = decided_live(&origin, 2);

    assert!(
        let_in.load(Ordering::SeqCst),
        "its repository was asked about"
    );
    assert_eq!(
        each(&decisions(&home), "verdict"),
        ["waved:outsider", "stop"],
        "a round asks afresh, so what he may do there now is what counts"
    );
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}

#[test]
fn github_a_round_asks_about_a_repository_once_however_many_threads_it_holds() {
    let (origin, asked) = answering_whole(two_stops_in_one_repository);
    let home = decided_live(&origin, 2);

    assert_eq!(
        each(&decisions(&home), "verdict"),
        ["stop", "stop"],
        "both collaborators' stops"
    );
    let asked: &Mutex<Vec<String>> = &asked;
    let asked = asked.lock().expect("what was asked").clone();
    let times = asked
        .iter()
        .filter(|line| line.contains(" /repos/example/shop "))
        .count();
    assert_eq!(times, 1, "its repository was asked about once: {asked:?}");
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
    let home = decided_live(&origin, 1);

    let said = decisions(&home);
    assert!(refused.load(Ordering::SeqCst), "the limit refused it once");
    assert_eq!(
        each(&said, "verdict"),
        ["stop"],
        "a collaborator's stop, read once the limit let it be, not set aside"
    );
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}
