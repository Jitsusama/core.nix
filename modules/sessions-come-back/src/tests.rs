//! What sessions-come-back keeps, brings back and passes over, read from
//! node answers shaped as a Mac's node gives them.

use super::*;

const HOLDS: &str = concat!(
    r#"{"id":"4d783094","command":["pi","--session","01a1-a"],"folder":"/Users/j/src","pid":11}"#,
    "\n",
    r#"{"id":"f0520da9","command":["/etc/profiles/bin/pi","--session","/x/01a1-b.jsonl"],"folder":"/Users/j"}"#,
    "\n",
    r#"{"id":"eaad11f8","command":["sh","-c","exec \"$@\"","sprout","pi"],"folder":"/Users/j"}"#,
    "\n",
    r#"{"id":"0d0d0d0d","command":["pi"],"folder":"/Users/j"}"#,
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
fn what_cannot_be_kept_is_passed_over_and_said_and_the_rest_kept() {
    let holds = concat!(
        "not a hold\n",
        r#"{"id":"11111111","command":["pi"],"folder":""}"#,
        "\n",
        r#"{"id":"22222222","command":["pi"],"folder":"/f"}"#,
        "\n",
        r#"{"id":"33333333","command":["pi"],"folder":"/f"}"#,
        "\n",
        r#"{"id":"44444444","command":["pi"],"folder":"/f"}"#,
        "\n",
        r#"{"id":"55555555","command":["pi"],"folder":"/f"}"#,
        "\n",
        r#"{"id":"66666666","command":["pi"],"folder":"/g"}"#,
    );
    let sessions = r#"{"Ok":{"Sessions":[
        {"id":"one","machine":"mac","held":"11111111"},
        {"id":"twice","machine":"mac","held":"22222222"},
        {"id":"again","machine":"mac","held":"22222222"},
        {"id":"x; rm -rf ~","machine":"mac","held":"33333333"},
        {"id":"good","machine":"mac","held":"44444444"},
        {"id":"far","machine":"other","held":"55555555"},
        {"id":"--help","machine":"mac","held":"66666666"}
    ]}}"#;
    let found = held(holds, sessions, running).expect("the listing reads");
    assert_eq!(found.sessions, vec![session("good", "/f")]);
    assert_eq!(found.machine, Some("mac".into()));
    assert_eq!(
        found.passed.len(),
        6,
        "each passed over is said: {:?}",
        found.passed
    );
    assert!(
        found.passed[0].starts_with("a line of its holds isn't one"),
        "a line that isn't a hold is said first: {}",
        found.passed[0]
    );
    assert_eq!(found.passed[1], "hold 11111111 names no folder");
    assert_eq!(found.passed[2], "2 sessions name hold 22222222");
    assert_eq!(found.passed[3], "hold 33333333's session id isn't plain");
    assert_eq!(found.passed[4], "far names other, not mac, as its machine");
    assert_eq!(found.passed[5], "hold 66666666's session id isn't plain");
}

#[test]
fn a_node_that_answers_with_an_error_is_not_read_as_holding_nothing() {
    assert!(
        held(HOLDS, r#"{"Err":"no node"}"#, running).is_err(),
        "an Err answer is a failure, so what's kept isn't emptied"
    );
}

#[test]
fn the_first_run_keeps_what_is_held_and_brings_nothing_back() {
    let a = session("a", "/f");
    assert_eq!(
        next(None, "one", &now(std::slice::from_ref(&a))),
        Kept {
            era: "one".into(),
            machine: Some("mac".into()),
            sessions: vec![a],
            ..Kept::default()
        },
    );
}

fn holding(sessions: &[Session]) -> Kept {
    Kept {
        era: "one".into(),
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
        })
        .collect()
}

#[test]
fn a_session_that_ends_while_the_machine_runs_does_not_come_back() {
    let mut kept = Some(holding(&[session("a", "/f"), session("b", "/g")]));
    for _ in 0..GONE {
        let after = next(kept, "one", &now(&[session("a", "/f")]));
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
    let rebooted = next(kept, "two", &Held::default());
    assert_eq!(
        rebooted.waiting,
        waits(&[session("a", "/f")]),
        "b went unseen for {GONE} runs before the reboot, so it ended by itself"
    );
}

#[test]
fn a_session_the_shutdown_ended_runs_before_it_still_comes_back() {
    let mut kept = Some(holding(&[session("a", "/f"), session("b", "/g")]));
    for _ in 1..GONE {
        kept = Some(next(kept, "one", &now(&[session("a", "/f")])));
    }
    let rebooted = next(kept, "two", &Held::default());
    assert_eq!(
        rebooted.waiting,
        waits(&[session("a", "/f"), session("b", "/g")]),
        "runs midway through a shutdown that missed b {} times don't keep b from coming back",
        GONE - 1
    );
}

#[test]
fn a_session_unseen_for_a_run_and_then_held_again_is_held() {
    let missed = next(Some(holding(&[session("a", "/f")])), "one", &now(&[]));
    let again = next(Some(missed), "one", &now(&[session("a", "/f")]));
    assert_eq!(
        (again.sessions, again.unseen),
        (vec![session("a", "/f")], vec![])
    );
}

#[test]
fn a_new_boot_or_login_sets_every_session_held_before_it_waiting() {
    let kept = Some(holding(&[session("a", "/f"), session("b", "/g")]));
    let after = next(kept, "two", &Held::default());
    assert_eq!(
        after,
        Kept {
            era: "two".into(),
            machine: Some("mac".into()),
            sessions: vec![],
            unseen: vec![],
            waiting: waits(&[session("a", "/f"), session("b", "/g")]),
        },
        "the machine's name is kept from before, when nothing is held yet"
    );
}

#[test]
fn a_session_still_held_across_a_new_era_is_not_started_twice() {
    let kept = Some(holding(&[session("a", "/f"), session("b", "/g")]));
    let after = next(kept, "two", &now(&[session("a", "/f")]));
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
                session: session("a", "/f"),
                tries: 2,
            },
            Waiting {
                session: session("b", "/g"),
                tries: 0,
            },
        ],
        ..holding(&[])
    });
    let after = next(kept, "one", &now(&[session("a", "/f")]));
    assert_eq!(after.waiting, waits(&[session("b", "/g")]));
}

#[test]
fn a_second_reboot_before_they_are_back_keeps_each_waiting_once() {
    let kept = Some(Kept {
        waiting: vec![Waiting {
            session: session("a", "/f"),
            tries: 3,
        }],
        ..holding(&[session("a", "/f"), session("c", "/h")])
    });
    let after = next(kept, "two", &Held::default());
    assert_eq!(
        after.waiting,
        vec![
            Waiting {
                session: session("a", "/f"),
                tries: 3
            },
            Waiting {
                session: session("c", "/h"),
                tries: 0
            },
        ],
        "a, both waiting and held before the reboot, waits once, keeping its tries"
    );
}

fn waiting(ids: &[&str]) -> Kept {
    Kept {
        waiting: ids
            .iter()
            .map(|id| Waiting {
                session: session(id, &format!("/{id}")),
                tries: 0,
            })
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
        |_| true,
        |machine, s| {
            started.push(format!("{machine} {} {}", s.folder, s.id));
            Ok(())
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
        |folder| folder != "/a",
        |_, s| {
            started.push(s.id.clone());
            Ok(())
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
    for run in 1..TRIES {
        bring_back(
            &mut kept,
            |_| true,
            |_, _| Err("no link yet".into()),
            |_| {},
        );
        assert_eq!(kept.waiting.len(), 1, "still waiting after run {run}");
    }
    bring_back(
        &mut kept,
        |_| true,
        |_, _| Err("no link yet".into()),
        |line| said.push(line),
    );
    assert_eq!(kept.waiting, vec![], "left after {TRIES} runs");
    assert_eq!(
        said,
        vec!["a wasn't started again (no link yet); leaving it"]
    );
}

#[test]
fn nothing_is_started_before_the_machine_is_known() {
    let mut kept = waiting(&["a"]);
    kept.machine = None;
    let mut started = 0;
    bring_back(
        &mut kept,
        |_| true,
        |_, _| {
            started += 1;
            Ok(())
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

#[test]
fn the_console_login_is_read_from_who() {
    let who = "another   ttys001  Nov 19 09:00 \nsomeone      console      Nov 19 12:42 \n";
    assert_eq!(console_login(who, "someone"), Some("Nov 19 12:42".into()));
    assert_eq!(
        console_login(who, "another"),
        None,
        "a terminal isn't the console"
    );
}

#[test]
fn a_command_that_hangs_is_stopped_at_its_limit() {
    let begun = Instant::now();
    let said = within(Command::new("sleep").arg("30"), Duration::from_secs(1));
    assert_eq!(said, Err("no answer within 1 s".into()));
    assert!(
        begun.elapsed() < Duration::from_secs(5),
        "it didn't wait out the sleep"
    );
}

#[test]
fn a_command_that_answers_in_time_is_read_and_one_that_fails_says_why() {
    assert_eq!(
        within(Command::new("echo").arg("held"), Duration::from_secs(10)),
        Ok("held\n".into())
    );
    assert_eq!(
        within(
            Command::new("sh").args(["-c", "echo refused >&2; exit 3"]),
            Duration::from_secs(10)
        ),
        Err("refused".into())
    );
}

#[test]
fn a_stamp_is_the_utc_time() {
    assert_eq!(stamp(0), "1970-01-01T00:00:00Z");
    assert_eq!(stamp(1_795_215_241), "2026-11-20T22:54:01Z");
    assert_eq!(stamp(951_782_400), "2000-02-29T00:00:00Z");
}
