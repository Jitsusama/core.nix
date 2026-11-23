//! How sure this machine is of its own reach, and what the journal says
//! as that changes.

use super::super::reach::heard;
use super::*;

#[test]
fn this_machine_reads_its_reach_by_every_network_probed_but_never_the_call() {
    let machines = |reached: &str| {
        format!(
            r#"{{"Ok":{{"Machines":[{{"name":"here","reached":{reached}}},
            {{"name":"yonder","reached":{{"home":true}}}}]}}}}"#
        )
    };
    for (reached, reads) in [
        (r#"{"call":false,"home":true,"office":true}"#, Some(true)),
        (r#"{"call":true,"home":true}"#, Some(true)),
        (r#"{"call":false,"home":true,"office":false}"#, Some(false)),
        (r#"{"call":true,"home":false}"#, Some(false)),
        (r#"{"call":false}"#, None),
        (r#"{"call":true}"#, None),
        ("{}", None),
        ("null", None),
    ] {
        assert_eq!(
            reach(&machines(reached), "here"),
            reads,
            "a machine reaching {reached} misread"
        );
    }
    assert_eq!(
        reach(&machines(r#"{"home":false}"#), "far"),
        None,
        "another machine's record was read as this one's"
    );
    assert_eq!(reach("", "here"), None, "no answer read as a reach");
}

#[test]
fn a_network_one_realm_reads_missed_outranks_another_reading_all_reached() {
    for (was, realm, folded) in [
        (Some(true), Some(false), Some(false)),
        (Some(false), Some(true), Some(false)),
        (Some(false), None, Some(false)),
        (None, Some(false), Some(false)),
        (None, Some(true), Some(true)),
        (Some(true), None, Some(true)),
        (None, None, None),
    ] {
        assert_eq!(
            reaches(was, realm),
            folded,
            "{was:?} then {realm:?} folded wrongly"
        );
    }
}

#[test]
fn a_tick_no_node_answered_reads_nothing_of_this_machines_reach() {
    for (answered, here, reads) in [
        (false, Some(true), Reach::Unheard),
        (false, None, Reach::Unheard),
        (true, Some(true), Reach::Sure),
        (true, Some(false), Reach::Misses),
        (true, None, Reach::Unsaid),
    ] {
        assert_eq!(
            Reach::of(answered, here),
            reads,
            "answered {answered}, saying {here:?}, read wrongly"
        );
    }
}

#[test]
fn each_change_in_how_sure_it_is_of_its_reach_is_said_once() {
    let again = Some("as reached again");
    let misses = Some("doesn't read every network");
    let unsaid = Some("says nothing of the networks");
    let mut was = Reach::default();
    let ticks = [
        (Reach::Sure, None),
        (Reach::Misses, misses),
        (Reach::Misses, None),
        // No node answered: its silence is said on its own, and what was
        // read last stands, so being back is still said.
        (Reach::Unheard, None),
        (Reach::Sure, again),
        (Reach::Unheard, None),
        (Reach::Unsaid, unsaid),
        // A move between ways of being unsure is said too.
        (Reach::Misses, misses),
        (Reach::Sure, again),
    ];
    for (tick, (now, words)) in ticks.into_iter().enumerate() {
        let line = heard("here", &mut was, now);
        match words {
            Some(words) => assert!(
                line.as_ref().is_some_and(|line| line.contains(words)),
                "tick {tick}, reading {now:?}, said {line:?}, not {words:?}"
            ),
            None => assert_eq!(line, None, "tick {tick}, reading {now:?}, said it again"),
        }
    }
}

#[test]
fn a_record_kept_before_the_reach_was_its_own_keeps_its_machines() {
    let old = r#"{"machines": {"yonder": {"ready_at": 7, "listed_at": 7}}, "unsure": true}"#;
    let told: Told = serde_json::from_str(old).expect("a record an earlier tick kept");
    assert_eq!(
        (told.machines.len(), told.reach),
        (1, Reach::Sure),
        "an earlier tick's record lost its machines"
    );
}
