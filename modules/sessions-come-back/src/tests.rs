//! What sessions-come-back keeps, brings back and passes over, read from
//! node answers shaped as a Mac's node gives them.
//!
//! Long because every test reads the same node answers and stand-in node,
//! held once at its top, and a run is one path from a node's answer to
//! what comes back, so splitting it would copy those or split one path.

use super::*;
use std::cell::RefCell;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::thread;
use std::time::Instant;

const HOLDS: &str = concat!(
    r#"{"id":"4d783094","command":["pi","--session","01a1-a"],"folder":"/Users/j/src","pid":11}"#,
    "\n",
    r#"{"id":"f0520da9","command":["/etc/profiles/bin/pi","--session","/x/01a1-b.jsonl"],"folder":"/Users/j","pid":13}"#,
    "\n",
    r#"{"id":"eaad11f8","command":["sh","-c","exec \"$@\"","sprout","pi"],"folder":"/Users/j","pid":14}"#,
    "\n",
    r#"{"id":"0d0d0d0d","command":["pi"],"folder":"/Users/j","pid":15}"#,
    "\n",
    r#"{"id":"c1c1c1c1","command":["pi-claude","--session","01a1-f"],"folder":"/Users/j/own","pid":12}"#,
    "\n",
);

const SESSIONS: &str = r#"{"Ok":{"Sessions":[
    {"id":"01a1-a","machine":"mac","held":"4d783094","state":"idle"},
    {"id":"01a1-b","machine":"mac","held":"f0520da9"},
    {"id":"01a1-c","machine":"mac","held":"eaad11f8"},
    {"id":"01a1-d","machine":"other","held":"77777777"},
    {"id":"01a1-e","machine":"mac"},
    {"id":"01a1-f","machine":"mac","held":"c1c1c1c1"}
]}}"#;

/// When the runs in these tests happen, in seconds since 1970.
const AT: u64 = 1_795_215_241;

fn session(id: &str, folder: &str) -> Session {
    Session {
        id: id.into(),
        folder: folder.into(),
        program: "pi".into(),
    }
}

fn now(sessions: &[Session]) -> Held {
    Held {
        machine: Some("mac".into()),
        sessions: sessions.to_vec(),
        passed: Vec::new(),
        live: sessions.iter().map(|session| session.id.clone()).collect(),
    }
}

fn running(_: u32) -> bool {
    true
}

#[test]
fn the_pi_sessions_held_here_are_found_by_their_holds() {
    assert_eq!(
        held(HOLDS, SESSIONS, running),
        Ok(Held {
            machine: Some("mac".into()),
            sessions: vec![
                session("01a1-a", "/Users/j/src"),
                session("01a1-b", "/Users/j"),
                Session {
                    program: "pi-claude".into(),
                    ..session("01a1-f", "/Users/j/own")
                },
            ],
            passed: vec![],
            live: ["01a1-a", "01a1-b", "01a1-c", "01a1-d", "01a1-e", "01a1-f"]
                .map(String::from)
                .to_vec(),
        }),
        "a pi held by name or path, or as pi-claude, is found by its hold and kept as the \
         program it ran as; a sprout's wrapper and a hold the node names no session for are not"
    );
}

#[test]
fn a_hold_whose_pi_has_died_holds_nothing_unless_it_is_parked() {
    let holds = concat!(
        r#"{"id":"4d783094","command":["pi"],"folder":"/f","pid":11}"#,
        "\n",
        r#"{"id":"f0520da9","command":["pi"],"folder":"/g","pid":12,"parked":true}"#,
    );
    let sessions = r#"{"Ok":{"Sessions":[
        {"id":"a","machine":"mac","held":"4d783094"},
        {"id":"b","machine":"mac","held":"f0520da9"}
    ]}}"#;
    assert_eq!(
        held(holds, sessions, |_| false).map(|h| h.sessions),
        Ok(vec![session("b", "/g")]),
        "a dead pi's left-behind hold would bring back a session nobody ran"
    );
}

#[test]
fn a_hold_the_node_says_is_gone_holds_nothing_whatever_pid_0_reads_as() {
    let holds = concat!(
        r#"{"id":"4d783094","command":["pi"],"folder":"/f","pid":0}"#,
        "\n",
        r#"{"id":"f0520da9","command":["pi"],"folder":"/g"}"#,
        "\n",
        r#"{"id":"0a0a0a0a","command":["pi"],"folder":"/h","pid":0,"parked":true}"#,
    );
    let sessions = r#"{"Ok":{"Sessions":[
        {"id":"a","machine":"mac","held":"4d783094"},
        {"id":"b","machine":"mac","held":"f0520da9"},
        {"id":"c","machine":"mac","held":"0a0a0a0a"}
    ]}}"#;
    assert_eq!(
        held(holds, sessions, running).map(|h| h.sessions),
        Ok(vec![session("c", "/h")]),
        "a Mac's kill -0 0 succeeds, so pid 0, or none, is read as gone before it's asked, \
         and only a parked hold holds with no pi"
    );
}

#[test]
fn an_ended_session_naming_a_hold_leaves_the_live_one_on_it_kept() {
    let holds = r#"{"id":"4d783094","command":["pi"],"folder":"/f","pid":11}"#;
    let sessions = r#"{"Ok":{"Sessions":[
        {"id":"old","machine":"mac","held":"4d783094","state":"ended"},
        {"id":"new","machine":"mac","held":"4d783094","state":"idle"}
    ]}}"#;
    assert_eq!(
        held(holds, sessions, running).map(|h| (h.sessions, h.passed)),
        Ok((vec![session("new", "/f")], vec![])),
        "a pi that moved to a new session in its hold is kept as the new one"
    );
}

#[test]
fn what_cannot_be_kept_is_passed_over_and_said_and_the_rest_kept() {
    let holds = concat!(
        "not a hold\n",
        r#"{"id":"11111111","command":["pi"],"folder":"","pid":1}"#,
        "\n",
        r#"{"id":"99999999","command":["pi"],"folder":"src","pid":9}"#,
        "\n",
        r#"{"id":"22222222","command":["pi"],"folder":"/f","pid":2}"#,
        "\n",
        r#"{"id":"33333333","command":["pi"],"folder":"/f","pid":3}"#,
        "\n",
        r#"{"id":"55555555","command":["pi"],"folder":"/f","pid":5}"#,
        "\n",
        r#"{"id":"44444444","command":["pi"],"folder":"/f","pid":4}"#,
        "\n",
        r#"{"id":"66666666","command":["pi"],"folder":"/g","pid":6}"#,
        "\n",
        r#"{"id":"77777777","command":["pi"],"folder":"/h","pid":7}"#,
        "\n",
        r#"{"id":"88888888","command":["pi"],"folder":"/i","pid":8}"#,
    );
    let sessions = r#"{"Ok":{"Sessions":[
        {"id":"one","machine":"mac","held":"11111111"},
        {"id":"near","machine":"mac","held":"99999999"},
        {"id":"twice","machine":"mac","held":"22222222"},
        {"id":"again","machine":"mac","held":"22222222"},
        {"id":"x; rm -rf ~","machine":"mac","held":"33333333"},
        {"id":"far","machine":"other","held":"55555555"},
        {"id":"good","machine":"mac","held":"44444444"},
        {"id":"--help","machine":"mac","held":"66666666"},
        {"id":"also","machine":"mac","held":"77777777"},
        {"id":"odd","machine":"mac,os=x","held":"88888888"}
    ]}}"#;
    let found = held(holds, sessions, running).expect("the listing reads");
    assert_eq!(
        found.sessions,
        vec![session("good", "/f"), session("also", "/h")]
    );
    assert_eq!(
        found.machine,
        Some("mac".into()),
        "the machine is the one most name, though another's session came first"
    );
    assert_eq!(
        found.passed,
        vec![
            found.passed[0].clone(),
            "hold 11111111 names no folder".to_string(),
            r#"hold 99999999 names "src", a folder that isn't absolute"#.into(),
            "2 sessions name hold 22222222".into(),
            "hold 33333333's session id isn't plain".into(),
            "hold 66666666's session id isn't plain".into(),
            "odd names no plain machine".into(),
            "far names other, not mac, as its machine".into(),
        ],
        "each passed over is said"
    );
    assert!(
        found.passed[0].starts_with("a line of its holds isn't one"),
        "a line that isn't a hold is said first: {}",
        found.passed[0]
    );
}

#[test]
fn no_machine_is_taken_when_none_is_named_by_most() {
    let holds = concat!(
        r#"{"id":"11111111","command":["pi"],"folder":"/f","pid":1}"#,
        "\n",
        r#"{"id":"22222222","command":["pi"],"folder":"/g","pid":2}"#,
    );
    let sessions = r#"{"Ok":{"Sessions":[
        {"id":"a","machine":"other","held":"11111111"},
        {"id":"b","machine":"mac","held":"22222222"}
    ]}}"#;
    let found = held(holds, sessions, running).expect("the listing reads");
    assert_eq!(
        (found.machine, found.sessions.len(), found.passed.len()),
        (None, 0, 2),
        "a tie says nothing of where this is, so neither is kept: {:?}",
        found.passed
    );
}

#[test]
fn a_node_that_answers_with_an_error_is_not_read_as_holding_nothing() {
    assert!(
        held(HOLDS, r#"{"Err":"no node"}"#, running).is_err(),
        "an Err answer is a failure, so what's kept isn't emptied"
    );
    assert!(
        held(HOLDS, r#"{"Ok":{"Refused":"not now"}}"#, running).is_err(),
        "an answer that isn't a listing is a failure, not an empty one"
    );
}

#[test]
fn the_grace_is_three_runs_and_the_wait_a_quarter_of_an_hour() {
    assert_eq!(
        (GONE, TRIES, QUIET, STALE),
        (3, 15, 300, 345_600),
        "the runs a shutdown may take, the runs a session waits, how long an unanswered \
         start waits, and how old a record may be, as the description says them"
    );
}

#[test]
fn the_first_run_keeps_what_is_held_and_brings_nothing_back() {
    let a = session("a", "/f");
    assert_eq!(
        next(None, "one", &now(std::slice::from_ref(&a)), AT),
        Kept {
            era: "one".into(),
            looked: AT,
            machine: Some("mac".into()),
            sessions: vec![a],
            ..Kept::default()
        },
    );
}

fn holding(sessions: &[Session]) -> Kept {
    Kept {
        era: "one".into(),
        looked: AT,
        machine: Some("mac".into()),
        sessions: sessions.to_vec(),
        ..Kept::default()
    }
}

fn waits(sessions: &[Session]) -> Vec<Waiting> {
    sessions
        .iter()
        .map(|session| Waiting {
            session: session.clone(),
            tries: 0,
            unanswered: None,
        })
        .collect()
}

#[test]
fn a_session_that_ends_while_the_machine_runs_does_not_come_back() {
    let mut kept = Some(holding(&[session("a", "/f"), session("b", "/g")]));
    for _ in 0..3 {
        let after = next(kept, "one", &now(&[session("a", "/f")]), AT);
        assert_eq!(
            after.waiting,
            vec![],
            "only a new boot or login sets a session waiting"
        );
        assert_eq!(
            after.sessions,
            vec![session("a", "/f")],
            "what's kept is what's held"
        );
        kept = Some(after);
    }
    let rebooted = next(kept, "two", &Held::default(), AT);
    assert_eq!(
        rebooted.waiting,
        waits(&[session("a", "/f")]),
        "b went unseen for three runs before the reboot, so it ended by itself"
    );
}

#[test]
fn a_session_the_shutdown_ended_runs_before_it_still_comes_back() {
    let mut kept = Some(holding(&[session("a", "/f"), session("b", "/g")]));
    for _ in 0..2 {
        kept = Some(next(kept, "one", &now(&[session("a", "/f")]), AT));
    }
    let rebooted = next(kept, "two", &Held::default(), AT);
    assert_eq!(
        rebooted.waiting,
        waits(&[session("a", "/f"), session("b", "/g")]),
        "two runs midway through a shutdown that missed b don't keep b from coming back"
    );
}

#[test]
fn a_session_unseen_for_a_run_and_then_held_again_is_held() {
    let missed = next(Some(holding(&[session("a", "/f")])), "one", &now(&[]), AT);
    let again = next(Some(missed), "one", &now(&[session("a", "/f")]), AT);
    assert_eq!(
        (again.sessions, again.unseen),
        (vec![session("a", "/f")], vec![])
    );
}

#[test]
fn a_new_boot_or_login_sets_every_session_held_before_it_waiting() {
    let kept = Some(holding(&[session("a", "/f"), session("b", "/g")]));
    let after = next(kept, "two", &Held::default(), AT + 120);
    assert_eq!(
        after,
        Kept {
            era: "two".into(),
            looked: AT + 120,
            machine: Some("mac".into()),
            sessions: vec![],
            unseen: vec![],
            waiting: waits(&[session("a", "/f"), session("b", "/g")]),
            passed: vec![],
        },
        "the machine's name is kept from before, when nothing is held yet"
    );
}

#[test]
fn a_record_kept_long_before_the_boot_brings_nothing_back() {
    let kept = Kept {
        waiting: waits(&[session("c", "/h")]),
        ..holding(&[session("a", "/f")])
    };
    let weekend = next(Some(kept.clone()), "two", &Held::default(), AT + 4 * 86_400);
    assert_eq!(
        weekend.waiting,
        waits(&[session("c", "/h"), session("a", "/f")]),
        "a long weekend off still brings them back"
    );
    let later = next(Some(kept), "two", &Held::default(), AT + 4 * 86_400 + 1);
    assert_eq!(
        later.waiting,
        vec![],
        "a record left from before a rollback or an outage starts nothing from long ago"
    );
}

#[test]
fn a_session_still_held_across_a_new_era_is_not_started_twice() {
    let kept = Some(holding(&[session("a", "/f"), session("b", "/g")]));
    let after = next(kept, "two", &now(&[session("a", "/f")]), AT);
    assert_eq!(
        after.waiting,
        waits(&[session("b", "/g")]),
        "a session the node still holds, as across a log-out its host outlived, isn't started"
    );
}

#[test]
fn a_session_held_again_no_longer_waits() {
    let kept = Some(Kept {
        waiting: vec![
            Waiting {
                tries: 2,
                ..waits(&[session("a", "/f")])[0].clone()
            },
            waits(&[session("b", "/g")])[0].clone(),
        ],
        ..holding(&[])
    });
    let after = next(kept.clone(), "one", &now(&[session("a", "/f")]), AT);
    assert_eq!(after.waiting, waits(&[session("b", "/g")]));
    let holds = r#"{"id":"11111111","command":["pi"],"folder":"src","pid":1}"#;
    let sessions = r#"{"Ok":{"Sessions":[{"id":"a","machine":"mac","held":"11111111"}]}}"#;
    let passed_over = held(holds, sessions, running).expect("the listing reads");
    let after = next(kept.clone(), "one", &passed_over, AT);
    assert_eq!(
        after.waiting,
        waits(&[session("b", "/g")]),
        "one held again in a hold that's passed over would be started a second time"
    );
    let none_listed = r#"{"Ok":{"Sessions":[]}}"#;
    let ways = [
        (
            "run by hand, in no hold",
            "",
            r#"{"Ok":{"Sessions":[{"id":"a","machine":"mac"}]}}"#,
        ),
        (
            "run on another machine",
            "",
            r#"{"Ok":{"Sessions":[{"id":"a","machine":"other","held":"22222222"}]}}"#,
        ),
        (
            "held before the node links it",
            r#"{"id":"33333333","command":["pi","--session","a"],"folder":"/f","pid":1}"#,
            none_listed,
        ),
    ];
    for (way, holds, sessions) in ways {
        let now = held(holds, sessions, running).expect("the listing reads");
        let after = next(kept.clone(), "one", &now, AT);
        assert_eq!(
            after.waiting,
            waits(&[session("b", "/g")]),
            "one that runs again, {way}, would be started a second time"
        );
    }
    let ended = r#"{"Ok":{"Sessions":[{"id":"a","machine":"mac","state":"ended"}]}}"#;
    let none_running = held("", ended, running).expect("it reads");
    let after = next(kept, "one", &none_running, AT);
    assert_eq!(
        after.waiting.len(),
        2,
        "one whose pi ended, as every pi from before a reboot has, still waits"
    );
}

#[test]
fn a_second_reboot_before_they_are_back_keeps_each_waiting_once() {
    let tried = Waiting {
        tries: 3,
        ..waits(&[session("a", "/f")])[0].clone()
    };
    let kept = Some(Kept {
        waiting: vec![tried],
        ..holding(&[session("a", "/f"), session("c", "/h")])
    });
    let after = next(kept, "two", &Held::default(), AT);
    assert_eq!(
        after.waiting,
        waits(&[session("a", "/f"), session("c", "/h")]),
        "a, both waiting and held before the reboot, waits once, with its tries given back"
    );
}

fn waiting(ids: &[&str]) -> Kept {
    Kept {
        waiting: ids
            .iter()
            .map(|id| waits(&[session(id, &format!("/{id}"))])[0].clone())
            .collect(),
        ..holding(&[])
    }
}

#[test]
fn each_waiting_session_is_started_on_this_machine_in_its_folder() {
    let mut kept = waiting(&["a", "b"]);
    let mut started = Vec::new();
    bring_back(
        &mut kept,
        AT,
        |_| true,
        |machine, s| {
            started.push(format!("{machine} {} {}", s.folder, s.id));
            Start::Started
        },
        |_| {},
    );
    assert_eq!(started, vec!["mac /a a", "mac /b b"]);
    assert_eq!(kept.waiting, vec![], "a started session waits no more");
}

#[test]
fn a_session_whose_folder_is_not_there_waits_for_it_and_is_said() {
    let mut kept = waiting(&["a", "b"]);
    let mut said = Vec::new();
    let mut started = Vec::new();
    bring_back(
        &mut kept,
        AT,
        |folder| folder != "/a",
        |_, s| {
            started.push(s.id.clone());
            Start::Started
        },
        |line| said.push(line),
    );
    assert_eq!(
        started,
        vec!["b"],
        "pi would ask to fork a session started elsewhere"
    );
    assert_eq!(
        said[0],
        "a wasn't started again (its folder, /a, isn't there); trying again next run"
    );
    assert_eq!(kept.waiting.len(), 1, "a volume not yet mounted may yet be");
}

#[test]
fn one_the_node_refuses_is_tried_again_until_it_has_waited_too_long() {
    let mut kept = waiting(&["a"]);
    let mut said = Vec::new();
    for run in 1..15 {
        bring_back(
            &mut kept,
            AT,
            |_| true,
            |_, _| Start::Refused("no link yet".into()),
            |_| {},
        );
        assert_eq!(kept.waiting.len(), 1, "still waiting after run {run}");
    }
    bring_back(
        &mut kept,
        AT,
        |_| true,
        |_, _| Start::Refused("no link yet".into()),
        |line| said.push(line),
    );
    assert_eq!(kept.waiting, vec![], "left after fifteen runs");
    assert_eq!(
        said,
        vec!["a wasn't started again (no link yet); leaving it"]
    );
}

#[test]
fn a_start_the_node_gave_no_answer_to_is_not_asked_again_at_once() {
    let mut kept = waiting(&["a"]);
    let asked = RefCell::new(0);
    let ask = |_: &str, _: &Session| {
        *asked.borrow_mut() += 1;
        Start::Unanswered("no answer within 30 s".into())
    };
    bring_back(&mut kept, AT, |_| true, ask, |_| {});
    bring_back(&mut kept, AT + 60, |_| true, ask, |_| {});
    bring_back(&mut kept, AT + 299, |_| true, ask, |_| {});
    assert_eq!(
        *asked.borrow(),
        1,
        "it may have started, and a second pi on the session would write beside it"
    );
    bring_back(&mut kept, AT + 300, |_| true, ask, |_| {});
    assert_eq!(
        (*asked.borrow(), kept.waiting.len()),
        (2, 1),
        "five minutes on, it's still not held, so it's asked again"
    );
}

#[test]
fn an_unanswered_start_waits_from_when_it_was_asked_and_says_so_once() {
    let mut kept = waiting(&["a", "b"]);
    let mut said = Vec::new();
    bring_back(
        &mut kept,
        AT,
        |_| true,
        |_, session| {
            if session.id == "a" {
                thread::sleep(Duration::from_millis(1100));
            }
            Start::Unanswered("no answer within 30 s".into())
        },
        |line| said.push(line),
    );
    assert_eq!(
        said,
        ["a", "b"].map(|id| format!(
            "{id} wasn't started again (no answer within 30 s, so it may yet start); \
             asking again once 5 min have passed"
        )),
        "an unanswered start says when it's asked again, and only that"
    );
    assert!(
        kept.waiting[1].unanswered.is_some_and(|asked| asked > AT),
        "one asked late in a slow run waited from the run's start, not from its own ask: {:?}",
        kept.waiting[1].unanswered
    );
    kept.waiting[0].tries = TRIES - 1;
    kept.waiting.truncate(1);
    said.clear();
    bring_back(
        &mut kept,
        AT + QUIET + 2,
        |_| true,
        |_, _| Start::Unanswered("no answer within 30 s".into()),
        |line| said.push(line),
    );
    assert_eq!(
        said,
        ["a wasn't started again (no answer within 30 s, so it may yet start); leaving it"],
        "its last try says it's left, not that it's asked again"
    );
}

#[test]
fn what_the_record_says_is_checked_again_before_it_is_started() {
    let mut kept = Kept {
        waiting: waits(&[
            session("--help", "/f"),
            Session {
                program: "/tmp/anything".into(),
                ..session("b", "/g")
            },
            session("c", "/h"),
            session("d.jsonl", "/i"),
            session("e", "src"),
        ]),
        ..holding(&[])
    };
    let mut started = Vec::new();
    let mut said = Vec::new();
    bring_back(
        &mut kept,
        AT,
        |_| true,
        |_, s| {
            started.push(s.id.clone());
            Start::Started
        },
        |line| said.push(line),
    );
    assert_eq!(
        started,
        vec!["c"],
        "a record written by another hand starts no other program"
    );
    assert_eq!(
        said.len(),
        5,
        "each passed over is said, the id pi would read as a file and the folder no path \
         names among them: {said:?}"
    );
    assert!(said[0].starts_with("passed over: "), "{}", said[0]);
}

#[test]
fn nothing_is_started_before_the_machine_is_known() {
    let mut kept = waiting(&["a"]);
    kept.machine = None;
    let mut started = 0;
    bring_back(
        &mut kept,
        AT,
        |_| true,
        |_, _| {
            started += 1;
            Start::Started
        },
        |_| {},
    );
    assert_eq!(
        (started, kept.waiting.len()),
        (0, 1),
        "it waits to know where to start"
    );
}

#[test]
fn a_session_is_started_as_its_program_by_its_id_in_its_folder_here() {
    let own = Session {
        program: "pi-claude".into(),
        ..session("01a1-f", "/Users/j/a folder")
    };
    assert_eq!(
        start_words("mac", &own),
        [
            "start",
            "--need",
            "machine=mac",
            "--folder",
            "/Users/j/a folder",
            "--",
            "pi-claude",
            "--session",
            "01a1-f"
        ]
    );
}

#[test]
fn only_an_answer_that_says_it_started_counts() {
    assert!(
        started(r#"{"Ok":{"Started":{"machine":"mac","held":"29d72825"}}}"#),
        "the node's own answer to a start is a start"
    );
    assert!(
        !started(r#"{"Err":"not \"Started\" yet"}"#),
        "the word alone isn't a start"
    );
    assert!(
        !started(r#"{"Ok":{"Refused":"Started"}}"#),
        "a refusal naming the word isn't a start"
    );
    assert!(!started("Started"), "a bare word isn't an answer");
}

#[test]
fn a_kept_record_reads_back_as_it_was_written() {
    let kept = Kept {
        unseen: vec![Unseen {
            session: session("b", "/g"),
            runs: 1,
        }],
        passed: vec!["hold 1 names no folder".into()],
        ..waiting(&["a"])
    };
    let text = serde_json::to_string_pretty(&kept).expect("a record writes");
    assert_eq!(recall(Some(&text)), Ok(Some(kept)));
    assert_eq!(recall(None), Ok(None), "no record is a first run");
}

#[test]
fn a_record_that_cannot_be_read_is_said_rather_than_taken_as_a_first_run() {
    assert!(
        recall(Some("{\"era\": ")).is_err(),
        "a record cut short is said"
    );
    assert!(
        recall(Some(r#"{"boot":1000,"sessions":[]}"#)).is_err(),
        "a record of an older shape is said"
    );
}

#[test]
fn a_session_kept_before_programs_were_named_comes_back_as_pi() {
    let kept: Session =
        serde_json::from_str(r#"{"id":"a","folder":"/f"}"#).expect("a session reads");
    assert_eq!(kept.program, "pi");
}

const WHO: &str = "another   ttys001  Nov 19 09:00 \nsomeone      console      Nov 19 12:42 \n";

#[test]
fn the_console_login_is_read_from_who() {
    assert_eq!(console_login(WHO, "someone"), Some("Nov 19 12:42".into()));
    assert_eq!(
        console_login(WHO, "another"),
        None,
        "a terminal isn't the console"
    );
}

#[test]
fn a_macs_era_is_its_boot_and_its_console_login() {
    let uuid = "40DCF87A-79B7-469D-80E0-C06F4FAACF06\n";
    let era = era_of(uuid, WHO, "someone").expect("an era reads");
    assert_eq!(era, "40DCF87A-79B7-469D-80E0-C06F4FAACF06 Nov 19 12:42");
    let relogged = WHO.replace("12:42", "20:39");
    assert_ne!(
        era_of(uuid, &relogged, "someone"),
        Ok(era),
        "a login restarted in the same boot is a new era"
    );
    assert!(
        era_of(uuid, WHO, "nobody").is_err(),
        "no console line is said, not read as a new login that would bring back what just ended"
    );
    assert!(era_of(" \n", WHO, "someone").is_err(), "no boot is said");
}

#[test]
fn a_macs_boot_read_that_stalls_names_its_program() {
    let look = |command: &mut Command| match command.get_program().to_str() {
        Some("/usr/sbin/sysctl") => Ok("40DCF87A-79B7-469D-80E0-C06F4FAACF06\n".to_owned()),
        Some("/usr/bin/who") => Err(Unheard::Late(LOOK_WITHIN)),
        _ => Ok("someone\n".to_owned()),
    };
    let said = mac_era(&look)
        .expect_err("a stalled read fails")
        .to_string();
    assert!(
        said.contains("/usr/bin/who: no answer within 20 s"),
        "said {said:?}, not which read stalled"
    );
    let all = |command: &mut Command| match command.get_program().to_str() {
        Some("/usr/sbin/sysctl") => Ok("40DCF87A-79B7-469D-80E0-C06F4FAACF06\n".to_owned()),
        Some("/usr/bin/who") => Ok(WHO.to_owned()),
        _ => Ok("someone\n".to_owned()),
    };
    assert_eq!(
        mac_era(&all).ok(),
        Some("40DCF87A-79B7-469D-80E0-C06F4FAACF06 Nov 19 12:42".to_owned()),
        "three answers read as the boot and login"
    );
}

#[test]
fn a_command_that_hangs_is_stopped_at_its_limit() {
    let _turn = one_at_a_time();
    let begun = Instant::now();
    let said = within(Command::new("sleep").arg("30"), Duration::from_secs(1));
    assert_eq!(said, Err(Unheard::Late(Duration::from_secs(1))));
    assert!(
        begun.elapsed() < Duration::from_secs(5),
        "it didn't wait out the sleep"
    );
}

#[test]
fn a_command_whose_child_keeps_its_output_open_is_stopped_at_its_limit() {
    let _turn = one_at_a_time();
    let begun = Instant::now();
    let folder = folder("held-open");
    fs::create_dir_all(&folder).expect("a folder is made");
    let pid = folder.join("pid");
    let said = within(
        Command::new("sh").args([
            "-c",
            &format!("sleep 30 & echo $! > {}; echo held", pid.display()),
        ]),
        Duration::from_secs(1),
    );
    assert_eq!(said, Err(Unheard::Late(Duration::from_secs(1))));
    assert!(
        begun.elapsed() < Duration::from_secs(5),
        "it didn't wait for the sleep to close the pipe"
    );
    let sleep = fs::read_to_string(&pid).expect("the sleep's pid was written");
    let state = || fs::read_to_string(format!("/proc/{}/stat", sleep.trim())).ok();
    let stopped = (0..40).any(|_| {
        let dead = state().is_none_or(|stat| stat.contains(") Z "));
        if !dead {
            thread::sleep(Duration::from_millis(50));
        }
        dead
    });
    assert!(
        stopped,
        "the sleep it left holding the pipe was stopped with it"
    );
}

#[test]
fn a_command_that_answers_in_time_is_read_and_one_that_fails_says_why() {
    let _turn = one_at_a_time();
    assert_eq!(
        within(Command::new("echo").arg("held"), Duration::from_secs(10)),
        Ok("held\n".into())
    );
    assert_eq!(
        within(
            Command::new("sh").args(["-c", "echo refused >&2; echo '{\"Err\":1}'; exit 3"]),
            Duration::from_secs(10)
        ),
        Err(Unheard::Failed("refused; {\"Err\":1}".into())),
        "what it printed on either stream is said"
    );
}

#[test]
fn a_stamp_is_the_utc_time() {
    assert_eq!(stamp(0), "1970-01-01T00:00:00Z");
    assert_eq!(stamp(1_795_215_241), "2026-11-20T22:54:01Z");
    assert_eq!(stamp(951_782_400), "2000-02-29T00:00:00Z");
}

#[test]
fn a_line_the_node_wrote_stays_one_line_of_the_log() {
    assert_eq!(
        one_line("far names x\n2026-11-20T22:54:01Z work: a started again"),
        "far names x\\n2026-11-20T22:54:01Z work: a started again"
    );
}

/// A folder of its own for one test's records.
fn folder(name: &str) -> PathBuf {
    let folder = env::temp_dir().join(format!("scb-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&folder);
    folder
}

/// Held by a test while it starts a process or runs a realm. A process
/// forked in another test shares a realm's lock until it execs, and a run
/// then would read the realm as kept by another and do nothing; in use,
/// one run is one process.
fn one_at_a_time() -> MutexGuard<'static, ()> {
    static TURN: Mutex<()> = Mutex::new(());
    TURN.lock().unwrap_or_else(PoisonError::into_inner)
}

/// A realm's run against a node that answers `holds` and `sessions` as
/// given, and starts whatever it's asked to, noting each start and line.
struct Node {
    holds: String,
    sessions: Result<String, Unheard>,
    answer: Result<String, Unheard>,
    starts: RefCell<Vec<String>>,
    said: RefCell<Vec<String>>,
}

impl Node {
    fn new(holds: &str, sessions: &str) -> Self {
        Self {
            holds: holds.into(),
            sessions: Ok(sessions.into()),
            answer: Ok(r#"{"Ok":{"Started":{}}}"#.into()),
            starts: RefCell::default(),
            said: RefCell::default(),
        }
    }

    fn run(
        &self,
        era: &str,
        folder: &Path,
        at: u64,
        on_start: &dyn Fn(&[String]),
    ) -> Result<(), Failure> {
        let ask = |words: &[String], _: Duration| match words[0].as_str() {
            "holds" => Ok(self.holds.clone()),
            "sessions" => self.sessions.clone(),
            _ => {
                on_start(words);
                self.starts
                    .borrow_mut()
                    .push(words.last().cloned().unwrap_or_default());
                self.answer.clone()
            }
        };
        let say = |line: &str| self.said.borrow_mut().push(line.into());
        let _turn = one_at_a_time();
        run_realm(
            "work",
            era,
            folder,
            at,
            &World {
                ask: &ask,
                runs: &running,
                there: &|_| true,
                say: &say,
            },
        )
    }
}

const ONE_HOLD: &str = r#"{"id":"4d783094","command":["pi"],"folder":"/f","pid":11}"#;
const ONE_SESSION: &str = r#"{"Ok":{"Sessions":[{"id":"a","machine":"mac","held":"4d783094"}]}}"#;
const NONE_HELD: &str = r#"{"Ok":{"Sessions":[]}}"#;

#[test]
fn a_realm_keeps_what_is_waiting_before_it_starts_any() {
    let folder = folder("keeps-first");
    Node::new(ONE_HOLD, ONE_SESSION)
        .run("one", &folder, AT, &|_| {})
        .expect("the first run keeps");
    let file = folder.join("work.json");
    let rebooted = Node::new("", NONE_HELD);
    let seen = AtomicU32::new(0);
    rebooted
        .run("two", &folder, AT + 60, &|_| {
            let kept = recall(fs::read_to_string(&file).ok().as_deref());
            let waiting = kept.ok().flatten().map_or(0, |k| k.waiting.len());
            seen.store(u32::try_from(waiting).unwrap_or(0), Ordering::SeqCst);
        })
        .expect("the run after the reboot keeps");
    assert_eq!(*rebooted.starts.borrow(), vec!["a"], "a comes back");
    assert_eq!(
        seen.load(Ordering::SeqCst),
        1,
        "the record already held a waiting when it was started, so a run that died after \
         would not start it twice"
    );
    let after = recall(fs::read_to_string(&file).ok().as_deref()).expect("the record reads");
    assert_eq!(
        after.map(|k| k.waiting),
        Some(vec![]),
        "a started waits no more"
    );
}

#[test]
fn a_realm_whose_record_cannot_be_read_sets_it_aside_and_says_so() {
    let folder = folder("unread");
    fs::create_dir_all(&folder).expect("a folder is made");
    fs::write(folder.join("work.json"), [0xff, 0xfe, b'{']).expect("a torn record is written");
    let node = Node::new(ONE_HOLD, ONE_SESSION);
    node.run("one", &folder, AT, &|_| {})
        .expect("the run keeps afresh");
    assert_eq!(
        fs::read(folder.join("work.json.unread")).ok(),
        Some(vec![0xff, 0xfe, b'{']),
        "a record a crash left in bytes that aren't text is kept aside, not written over"
    );
    assert!(
        node.said.borrow()[0].starts_with("its record couldn't be read"),
        "{:?}",
        node.said.borrow()
    );
}

#[test]
fn a_realm_whose_node_fails_leaves_its_record_as_it_was() {
    let folder = folder("node-fails");
    Node::new(ONE_HOLD, ONE_SESSION)
        .run("one", &folder, AT, &|_| {})
        .expect("the first run keeps");
    let file = folder.join("work.json");
    let was = fs::read(&file).expect("a record is kept");
    let mut node = Node::new(ONE_HOLD, ONE_SESSION);
    node.sessions = Err(Unheard::Late(LOOK_WITHIN));
    assert!(
        node.run("one", &folder, AT + 60, &|_| {}).is_err(),
        "a node that gave no listing fails the run"
    );
    assert_eq!(
        fs::read(&file).ok(),
        Some(was),
        "a node away empties nothing kept"
    );
}

#[test]
fn a_realm_says_what_it_passes_over_once() {
    let folder = folder("said-once");
    let holds = r#"{"id":"11111111","command":["pi"],"folder":"","pid":1}"#;
    let sessions = r#"{"Ok":{"Sessions":[{"id":"a","machine":"mac","held":"11111111"}]}}"#;
    let first = Node::new(holds, sessions);
    first.run("one", &folder, AT, &|_| {}).expect("a run keeps");
    let second = Node::new(holds, sessions);
    second
        .run("one", &folder, AT + 60, &|_| {})
        .expect("a run keeps");
    assert_eq!(
        (first.said.borrow().len(), second.said.borrow().len()),
        (1, 0),
        "a hold passed over each minute is said when it's new, not 1,440 times a day"
    );
}

#[test]
fn a_start_the_node_refused_is_tried_next_run_and_one_unheard_waits() {
    for (answer, unanswered, case) in [
        (Ok(r#"{"Err":"busy"}"#.to_string()), None, "a refusal"),
        (
            Err(Unheard::Late(START_WITHIN)),
            Some(true),
            "no answer in time",
        ),
        (
            Err(Unheard::Failed("broken pipe".into())),
            Some(true),
            "a client that failed, after the node may have taken it",
        ),
    ] {
        let folder = folder("answers");
        Node::new(ONE_HOLD, ONE_SESSION)
            .run("one", &folder, AT, &|_| {})
            .expect("the first run keeps");
        let mut rebooted = Node::new("", NONE_HELD);
        rebooted.answer = answer;
        rebooted
            .run("two", &folder, AT + 60, &|_| {})
            .expect("the run after the reboot keeps");
        let kept = recall(fs::read_to_string(folder.join("work.json")).ok().as_deref())
            .ok()
            .flatten()
            .expect("the record reads");
        assert_eq!(
            kept.waiting
                .iter()
                .map(|w| {
                    let asked = w.unanswered.map(|at| (AT + 60..=AT + 62).contains(&at));
                    (w.session.id.as_str(), w.tries, asked)
                })
                .collect::<Vec<_>>(),
            vec![("a", 1, unanswered)],
            "{case} keeps it waiting, stamped when it was asked, as a slow disk allows"
        );
        let back = Node::new(ONE_HOLD, ONE_SESSION);
        back.run("two", &folder, AT + 120, &|_| {})
            .expect("the run that finds it held keeps");
        assert!(
            back.said
                .borrow()
                .iter()
                .any(|line| line.contains("a runs again, so it waits no more")),
            "after {case}, one found running again is said to be back: {:?}",
            back.said.borrow()
        );
    }
}

#[test]
fn a_realm_that_is_off_brings_nothing_back_even_once_it_is_on_again() {
    let folder = folder("off");
    Node::new(ONE_HOLD, ONE_SESSION)
        .run("one", &folder, AT, &|_| {})
        .expect("the first run keeps");
    fs::write(folder.join("work.off"), "").expect("the realm is turned off");
    let off = Node::new("", NONE_HELD);
    off.run("two", &folder, AT + 60, &|_| {})
        .expect("a run while it's off ends well");
    assert!(
        off.starts.borrow().is_empty() && !folder.join("work.json").exists(),
        "while it's off nothing starts and nothing is kept"
    );
    fs::remove_file(folder.join("work.off")).expect("the realm is turned on");
    let on = Node::new("", NONE_HELD);
    on.run("three", &folder, AT + 120, &|_| {})
        .expect("a run once it's on keeps");
    assert!(
        on.starts.borrow().is_empty(),
        "what was held before it was off doesn't come back"
    );
}

#[test]
fn a_new_boot_gives_what_still_waits_its_tries_again() {
    let before = Kept {
        waiting: vec![Waiting {
            session: session("a", "/f"),
            tries: TRIES - 1,
            unanswered: Some(AT),
        }],
        ..holding(&[])
    };
    let kept = next(Some(before), "two", &now(&[]), AT + 60);
    assert_eq!(
        kept.waiting
            .iter()
            .map(|w| (w.tries, w.unanswered))
            .collect::<Vec<_>>(),
        vec![(0, None)],
        "a try and a wait from the boot before mean nothing after this one"
    );
}

#[test]
fn a_realm_another_run_is_keeping_is_left_to_it() {
    let folder = folder("locked");
    fs::create_dir_all(&folder).expect("a folder is made");
    let lock = fs::File::create(folder.join("work.lock")).expect("a lock is made");
    lock.lock().expect("the lock is taken");
    let node = Node::new(ONE_HOLD, ONE_SESSION);
    node.run("one", &folder, AT, &|_| {})
        .expect("the run leaves it");
    assert!(
        !folder.join("work.json").exists(),
        "a run beside another, as one by hand, neither keeps nor starts"
    );
}
