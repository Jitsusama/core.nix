use super::*;

/// 2026-11-21T12:00:00Z.
const NOW: i64 = 1_795_262_400;

fn outcome(
    kind: &str,
    asked: &str,
    started: Option<&str>,
    ended: Option<&str>,
    waited: i64,
) -> String {
    let mut o = serde_json::json!({"kind": kind, "asked": asked, "waitedS": waited});
    if let Some(started) = started {
        o["started"] = started.into();
    }
    if let Some(ended) = ended {
        o["ended"] = ended.into();
    }
    o.to_string()
}

fn found<'a>(measures: &'a [Measure], key: &str) -> &'a Measure {
    measures
        .iter()
        .find(|m| m.key == format!("{QUEUE}{key}"))
        .unwrap_or_else(|| panic!("no measure for {key} in {measures:?}"))
}

#[test]
fn a_turn_never_given_counts_its_wait_to_its_end_or_to_now() {
    let outcomes = [
        outcome(
            "world-walk",
            "2026-11-21T11:00:00Z",
            Some("2026-11-21T11:00:05Z"),
            None,
            5,
        ),
        // Gave up after ten minutes without a turn.
        outcome(
            "world-walk",
            "2026-11-21T10:00:00Z",
            None,
            Some("2026-11-21T10:10:00Z"),
            0,
        ),
        // Still waiting, half an hour on.
        outcome("world-walk", "2026-11-21T11:30:00Z", None, None, 0),
        // A day and more ago, and a build: neither is a World turn today.
        outcome(
            "world-walk",
            "2026-11-20T11:00:00Z",
            Some("2026-11-20T12:00:00Z"),
            None,
            3600,
        ),
        outcome(
            "build",
            "2026-11-21T11:00:00Z",
            Some("2026-11-21T11:01:00Z"),
            None,
            60,
        ),
    ]
    .join("\n");
    let mut waits = turns(&outcomes, "world-walk", NOW);
    waits.sort_by(f64::total_cmp);
    assert_eq!(
        waits,
        vec![5.0, 600.0, 1800.0],
        "each of today's turns, stuck ones too"
    );
    assert_eq!(
        turns(&outcomes, "build", NOW),
        vec![60.0],
        "builds read apart"
    );
}

#[test]
fn each_queue_reads_its_own_record_for_the_day_against_its_target() {
    let outcomes = [
        outcome(
            "world-walk",
            "2026-11-21T11:00:00Z",
            Some("2026-11-21T11:00:01Z"),
            None,
            1,
        ),
        outcome(
            "world-walk",
            "2026-11-21T11:05:00Z",
            Some("2026-11-21T11:05:01Z"),
            None,
            1,
        ),
        outcome(
            "world-walk",
            "2026-11-21T11:10:00Z",
            Some("2026-11-21T11:13:00Z"),
            None,
            180,
        ),
    ]
    .join("\n");
    let sends = [
        format!(r#"{{"at":{},"waited":0}}"#, NOW - 60),
        format!(r#"{{"at":{},"waited":30}}"#, NOW - 120),
        format!(r#"{{"at":{},"waited":900}}"#, NOW - DAY - 1),
        "not a line".to_owned(),
    ]
    .join("\n");
    let trains = format!(r#"{{"at":{},"seconds":416}}"#, NOW - 3600);
    let measures = queues(&outcomes, &sends, &trains, NOW);
    assert_eq!(
        measures.len(),
        3,
        "no builds today, so no builds line: {measures:?}"
    );
    let walks = found(&measures, "world-walks");
    assert_eq!(walks.now, 180.0, "the p90 of three is the longest");
    assert_eq!(walks.target, Some(60.0), "a minute at most");
    assert_eq!(walks.owner, "80ECO2", "the workflow's owner");
    assert_eq!(walks.class, Class::Queues, "on the queues section");
    assert_eq!(
        walks.says, "p90 seconds waiting for a World git turn (median 1, worst 180; 3 in a day)",
        "names the wait, its spread and its flow"
    );
    let sent = found(&measures, "sends");
    assert_eq!(sent.now, 30.0, "yesterday's 900 s send is left out");
    assert!(sent.past().is_none(), "within two minutes");
    assert_eq!(
        found(&measures, "trains").now,
        416.0,
        "the train's own seconds"
    );
}

#[test]
fn the_worst_queue_is_the_one_furthest_past_its_target() {
    let queue = |key: &str, now: f64, target: f64| Measure {
        key: format!("{QUEUE}{key}"),
        class: Class::Queues,
        says: String::new(),
        now,
        target: Some(target),
        owner: "80ECO2".to_owned(),
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
