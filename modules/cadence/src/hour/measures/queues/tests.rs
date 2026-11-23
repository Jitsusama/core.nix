use super::*;

/// 2026-11-21T12:00:00Z.
const NOW: i64 = 1_795_262_400;

/// One job as the priority workflow writes it when the job ends, its
/// fields as on 21 November: given its turn at `started`, or giving up
/// waiting when that is none.
fn outcome(kind: &str, asked: &str, started: Option<&str>, ended: &str, waited: i64) -> String {
    let mut o = serde_json::json!({
        "v": 1, "id": "mwqw-1", "kind": kind, "class": "bulk", "what": "git fetch",
        "session": "s", "asked": asked, "ended": ended, "waitedS": waited,
        "capped": false, "beside": {"vms": 0, "builds": 0, "cappedBuilds": 0, "cpuPressure": 0},
        "bookedGib": 6, "waitedOn": {"memory": waited, "order": 0},
    });
    if let Some(started) = started {
        o["started"] = started.into();
        o["ranS"] = 1.into();
    }
    o.to_string()
}

/// A send as nix-offload notes it when it ends, to a builder `on`.
fn send(at: i64, on: &str, waited: i64) -> String {
    format!(r#"{{"at":{at},"on":"{on}","seconds":30,"status":0,"waited":{waited}}}"#)
}

fn found<'a>(measures: &'a [Measure], key: &str) -> &'a Measure {
    measures
        .iter()
        .find(|m| m.key == format!("{QUEUE}{key}"))
        .unwrap_or_else(|| panic!("no measure for {key} in {measures:?}"))
}

fn sorted(mut waits: Vec<f64>) -> Vec<f64> {
    waits.sort_by(f64::total_cmp);
    waits
}

#[test]
fn each_turn_counts_its_own_wait_once_it_ends_within_the_day() {
    // Each waitedS differs from its asked-to-started time, so the test sees
    // which was read: the workflow's own wait, where the line has one.
    let outcomes = [
        outcome(
            "world-walk",
            "2026-11-21T11:00:00Z",
            Some("2026-11-21T11:00:05Z"),
            "2026-11-21T11:01:00Z",
            4,
        ),
        // Gave up after ten minutes without a turn.
        outcome(
            "world-walk",
            "2026-11-21T10:00:00Z",
            None,
            "2026-11-21T10:10:00Z",
            590,
        ),
        // Asked more than a day ago, it waited long and ended today.
        outcome(
            "world-walk",
            "2026-11-20T10:00:00Z",
            Some("2026-11-21T11:00:00Z"),
            "2026-11-21T11:30:00Z",
            89_000,
        ),
        // Ended a day and more ago, and a build: neither is today's turn.
        outcome(
            "world-walk",
            "2026-11-20T11:00:00Z",
            Some("2026-11-20T11:30:00Z"),
            "2026-11-20T11:40:00Z",
            1800,
        ),
        outcome(
            "build",
            "2026-11-21T11:00:00Z",
            Some("2026-11-21T11:01:00Z"),
            "2026-11-21T11:02:00Z",
            50,
        ),
        // An older line with no waitedS waited from asking to its turn.
        serde_json::json!({
            "kind": "build",
            "asked": "2026-11-21T09:00:00Z",
            "started": "2026-11-21T09:02:00Z",
            "ended": "2026-11-21T09:05:00Z",
        })
        .to_string(),
    ]
    .join("\n");
    let walks = turns(&outcomes, "world-walk", NOW);
    assert_eq!(
        sorted(walks.waits),
        vec![4.0, 590.0, 89_000.0],
        "each of the turns that ended today, one asked yesterday too, by its waitedS"
    );
    assert_eq!(walks.flow, "3 ended in a day", "its flow is what ended");
    assert_eq!(
        sorted(turns(&outcomes, "build", NOW).waits),
        vec![50.0, 120.0],
        "builds read apart, an old line by asked to started"
    );
}

#[test]
fn only_a_vm_runs_send_counts_against_the_vm_runs_target() {
    let sends = [
        send(NOW - 60, "vm-12", 300),
        send(NOW - 70, "nearby", 0),
        send(NOW - 80, "nearby", 0),
        send(NOW - DAY - 1, "vm-11", 900),
        "not a line".to_owned(),
    ]
    .join("\n");
    let read = super::sends(&sends, NOW);
    assert_eq!(
        read.waits,
        vec![300.0],
        "the quick sends hid the VM run's wait"
    );
}

#[test]
fn a_riders_wait_counts_from_boarding_and_one_still_aboard_counts_so_far() {
    let landed = [
        // Three riders landed in one run: one boarded 35 minutes before,
        // and one whose file had gone, so merge-net kept no time for it.
        serde_json::json!({
            "at": NOW - 3600,
            "heads": ["a", "b", "h"],
            "boarded": [NOW - 3600 - 2100, NOW - 3600 - 600, null],
            "landed": "c",
            "seconds": 420,
        })
        .to_string(),
        // Before landings said when their riders boarded: flow, no wait.
        format!(
            r#"{{"at":{},"heads":["d"],"landed":"e","seconds":127}}"#,
            NOW - 7200
        ),
        format!(
            r#"{{"at":{},"heads":["f"],"boarded":[{}],"landed":"g","seconds":9}}"#,
            NOW - DAY - 1,
            NOW - DAY - 100
        ),
        // A rider the train refused, 15 minutes after it boarded, as
        // merge-net's keep_fell writes one.
        format!(
            r#"{{"at":{},"fell":["x"],"boarded":[{}]}}"#,
            NOW - 1800,
            NOW - 2700
        ),
        // A rider that stepped off, its pull request a draft, 100 s on.
        format!(
            r#"{{"at":{},"off":["y"],"boarded":[{}]}}"#,
            NOW - 1900,
            NOW - 2000
        ),
    ]
    .join("\n");
    let read = trains(&landed, &[NOW - 3000], NOW);
    assert_eq!(
        read.waits,
        vec![100.0, 600.0, 900.0, 2100.0, 3000.0],
        "a run's length was read as its riders' wait, or one aboard or left didn't count"
    );
    assert_eq!(
        read.flow, "4 landed in a day, 2 left unlanded, 1 aboard",
        "riders, not trains"
    );
    assert_eq!(read.through, 7, "every rider went through");
}

#[test]
fn each_queue_says_its_wait_or_that_nothing_passed_or_that_it_went_unread() {
    let outcomes = [
        ("2026-11-21T11:00:00Z", "2026-11-21T11:00:01Z", 1),
        ("2026-11-21T11:05:00Z", "2026-11-21T11:05:01Z", 1),
        ("2026-11-21T11:10:00Z", "2026-11-21T11:13:00Z", 180),
    ]
    .map(|(asked, started, waited)| {
        outcome(
            "world-walk",
            asked,
            Some(started),
            "2026-11-21T11:20:00Z",
            waited,
        )
    })
    .join("\n");
    let records = Records {
        outcomes: Some(&outcomes),
        sends: None,
        landed: Some(""),
        aboard: Some(&[]),
    };
    let measures = queues(&records, NOW);
    assert_eq!(measures.len(), 4, "every queue has its line: {measures:?}");
    let walks = found(&measures, "world-walks");
    assert_eq!(
        walks.now, 180.0,
        "three waits are judged by their p90, which a stuck one shows"
    );
    assert_eq!(walks.target, Some(60.0), "a minute at most");
    assert_eq!(walks.owner, "80ECO2", "the workflow's keeper");
    assert_eq!(walks.class, Class::Queues, "on the queues section");
    assert_eq!(
        walks.says,
        "p90 seconds waiting for a World git turn (median 1, worst 180; 3 ended in a day, \
         few waits)",
        "names the wait, its spread and its flow, and that its waits were few"
    );
    let builds = found(&measures, "builds");
    assert!(
        builds.target.is_none() && builds.says.ends_with(QUIET),
        "a quiet queue wasn't said as quiet: {builds:?}"
    );
    let sent = found(&measures, "sends");
    assert!(
        sent.target.is_none() && sent.says.ends_with(UNREAD),
        "an unread record wasn't said as unread: {sent:?}"
    );
    assert!(
        found(&measures, "trains").says.ends_with(QUIET),
        "a train with none landed and none aboard is quiet"
    );
    assert!(
        queues(
            &Records {
                aboard: None,
                ..records
            },
            NOW
        )
        .iter()
        .any(|m| m.key == "queue:trains" && m.says.ends_with(UNREAD)),
        "riders that couldn't be listed weren't said as unread"
    );
}

#[test]
fn ten_waits_are_judged_by_their_p90_and_waits_unsaid_keep_the_ask() {
    let records = Records {
        outcomes: None,
        sends: None,
        landed: None,
        aboard: None,
    };
    // Ten builds, nine quick and one of ten minutes: a p90 from ten.
    let builds = (0..10)
        .map(|n| {
            let asked = format!("2026-11-21T11:{n:02}:00Z");
            let waited = if n == 9 { 600 } else { 1 };
            outcome(
                "build",
                &asked,
                Some(&asked),
                "2026-11-21T11:30:00Z",
                waited,
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let ten = queues(
        &Records {
            outcomes: Some(&builds),
            ..records
        },
        NOW,
    );
    let builds = found(&ten, "builds");
    assert!(
        (builds.now - 1.0).abs() < f64::EPSILON && !builds.says.contains("few waits"),
        "ten waits' p90 wasn't the ninth, or they were said to be few: {builds:?}"
    );
    // VM runs that ended without saying how long they waited went through.
    let unsaid = format!(r#"{{"at":{},"on":"vm-12","seconds":30}}"#, NOW - 60);
    let sent = queues(
        &Records {
            sends: Some(&unsaid),
            ..records
        },
        NOW,
    );
    let sent = found(&sent, "sends");
    assert!(
        sent.target.is_none() && !answered(&sent.key, std::slice::from_ref(sent)),
        "a queue that let work through without its waits read as quiet: {sent:?}"
    );
}

#[test]
fn a_queues_ask_is_answered_back_in_or_quiet_and_held_while_unread() {
    let queue = |says: &str, now: f64, target: Option<f64>| Measure {
        key: "queue:sends".to_owned(),
        class: Class::Queues,
        says: says.to_owned(),
        now,
        target,
        owner: KEEPER.to_owned(),
        behind: Vec::new(),
    };
    let key = "queue:sends";
    assert!(
        !answered(key, &[queue("x", 300.0, Some(120.0))]),
        "still past"
    );
    assert!(answered(key, &[queue("x", 30.0, Some(120.0))]), "back in");
    assert!(
        !answered(key, &[queue(&format!("x{QUIET}"), 0.0, None)]),
        "a quiet queue answered the ask, though a wedged one reads the same"
    );
    assert!(
        !answered(key, &[queue(&format!("x{UNREAD}"), 0.0, None)]),
        "an unread record answered the ask"
    );
    assert!(answered(key, &[]), "a queue gone from the hour is answered");
}

#[test]
fn the_worst_queue_is_the_one_furthest_past_its_target() {
    let queue = |key: &str, now: f64, target: f64| Measure {
        key: format!("{QUEUE}{key}"),
        class: Class::Queues,
        says: String::new(),
        now,
        target: Some(target),
        owner: KEEPER.to_owned(),
        behind: Vec::new(),
    };
    let mut measures = vec![
        // Twice its target, against three times.
        queue("trains", 2400.0, 1200.0),
        queue("world-walks", 180.0, 60.0),
        queue("sends", 30.0, 120.0),
    ];
    assert_eq!(
        worst(&measures).map(|m| m.key.as_str()),
        Some("queue:world-walks"),
        "furthest past as a share of its target"
    );
    // A measure off the queues is never the worst queue, however far past.
    measures.push(Measure {
        key: "merge-to-on".to_owned(),
        class: Class::Ship,
        ..queue("x", 900.0, 18.0)
    });
    assert_eq!(
        worst(&measures).map(|m| m.key.as_str()),
        Some("queue:world-walks"),
        "only queues are asked"
    );
    assert!(
        worst(&[queue("sends", 30.0, 120.0)]).is_none(),
        "every queue in asks nothing"
    );
}
