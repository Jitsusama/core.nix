use serde_json::json;

use super::*;

const MACHINES: &str = concat!(
    r#"{"Ok":{"Machines":"#,
    r#"[{"name":"here"},{"name":"mac"},{"name":"cloud"}]}}"#
);

fn session(id: &str, machine: &str, state: &str, presence: &str) -> Value {
    json!({"id": id, "machine": machine, "state": state, "presence": {"state": presence}})
}

/// A busy session on `machine` listed with neither id nor address.
fn nameless(machine: &str) -> Value {
    json!({"machine": machine, "state": "busy", "presence": {"state": "busy"}})
}

fn answer(sessions: &[Value]) -> String {
    json!({"Ok": {"Sessions": sessions}}).to_string()
}

fn judged(read: &Measure) -> (String, Option<String>) {
    (read.now.to_string(), read.target.map(|t| t.to_string()))
}

/// The measure with the work realm alone listing `sessions`.
fn alone(sessions: &[Value]) -> Measure {
    let sessions = [("work".to_owned(), answer(sessions))];
    carried(&sessions, &[MACHINES.to_owned()], "here")
}

/// The measure with one realm answering `one` and another `other`, read
/// in both orders.
fn both_ways(one: &str, other: &str) -> [Measure; 2] {
    [(one, other), (other, one)].map(|(first, second)| {
        let sessions = [
            ("work".to_owned(), first.to_owned()),
            ("personal".to_owned(), second.to_owned()),
        ];
        carried(&sessions, &[MACHINES.to_owned()], "here")
    })
}

#[test]
fn each_machine_says_its_busy_and_listening_and_this_ones_busy_are_judged() {
    let work = answer(&[
        session("a", "here", "busy", "busy"),
        session("b", "here", "busy", "busy"),
        session("c", "here", "busy", "busy"),
        session("d", "here", "idle", "listening"),
        session("e", "here", "idle", "parked"),
        session("f", "here", "ended", "parked"),
        session("g", "mac", "busy", "busy"),
        session("h", "mac", "idle", "listening"),
    ]);
    // One session both realms list, and one ended that still listens.
    let personal = answer(&[
        session("a", "here", "busy", "busy"),
        session("i", "here", "ended", "listening"),
    ]);
    let sessions = [("work".to_owned(), work), ("personal".to_owned(), personal)];
    let read = carried(&sessions, &[MACHINES.to_owned()], "here");
    assert_eq!(
        judged(&read),
        ("3".to_owned(), Some("10".to_owned())),
        "this machine's busy sessions weren't judged against its most: {}",
        read.says
    );
    assert_eq!(
        read.behind,
        [
            "here: 3 busy, 1 listening",
            "mac: 1 busy, 1 listening",
            "cloud: 0 busy, 0 listening",
        ],
        "a machine's sessions were miscounted, or one carrying none was left out"
    );
    assert_eq!(
        read.says,
        "sessions busy on this machine, of 4 busy on 3 machines",
        "the realm's busy sessions were miscounted, or a doubt said where none was"
    );
}

#[test]
fn a_session_two_realms_list_counts_as_its_liveliest_listing() {
    let busy = answer(&[session("a", "here", "busy", "busy")]);
    let listening = answer(&[session("a", "here", "idle", "listening")]);
    let listening_away = answer(&[session("a", "mac", "idle", "listening")]);
    let ended = answer(&[session("a", "mac", "ended", "parked")]);
    let cases = [
        (&busy, &ended, "here: 1 busy, 0 listening"),
        (&busy, &listening_away, "here: 1 busy, 0 listening"),
        (&listening, &ended, "here: 0 busy, 1 listening"),
    ];
    for (livelier, other, row) in cases {
        for read in both_ways(livelier, other) {
            let idle = read
                .behind
                .iter()
                .filter(|r| r.ends_with(": 0 busy, 0 listening"))
                .count();
            assert!(
                read.behind.iter().any(|r| r == row) && idle == 2,
                "a livelier listing lost to another copy, or both counted: {:?}",
                read.behind
            );
        }
    }
}

#[test]
fn as_lively_a_listing_here_wins_then_one_naming_a_machine() {
    let here = answer(&[session("a", "here", "busy", "busy")]);
    let away = answer(&[session("a", "mac", "busy", "busy")]);
    let nowhere = answer(&[session("a", "", "busy", "busy")]);
    let cases = [
        (&away, &here, "here: 1 busy, 0 listening"),
        (&nowhere, &here, "here: 1 busy, 0 listening"),
        (&nowhere, &away, "mac: 1 busy, 0 listening"),
    ];
    for (one, other, row) in cases {
        for read in both_ways(one, other) {
            assert!(
                read.behind.iter().any(|r| r == row) && read.target.is_some(),
                "a copy as lively decided where a session counts by realm order: {:?}",
                read.behind
            );
        }
    }
}

#[test]
fn this_machine_past_its_most_is_judged_though_a_realm_is_silent() {
    for (here, target) in [(10, None), (11, Some("10".to_owned()))] {
        // Busy elsewhere too, so only this machine's count can pass it.
        let work: Vec<Value> = (0..15)
            .map(|n| {
                let machine = if n < here { "here" } else { "mac" };
                session(&format!("s{n}"), machine, "busy", "busy")
            })
            .collect();
        let sessions = [
            ("work".to_owned(), answer(&work)),
            ("personal".to_owned(), String::new()),
        ];
        let read = carried(&sessions, &[MACHINES.to_owned()], "here");
        assert!(
            judged(&read) == (here.to_string(), target)
                && read
                    .says
                    .contains("the personal realm's node didn't answer"),
            "a count at or past the most was misjudged beside a silent realm: {}",
            read.says
        );
    }
}

#[test]
fn a_count_that_may_fall_short_is_left_unjudged_and_says_why() {
    let work = answer(&[session("a", "here", "busy", "busy")]);
    let sessions = [
        ("work".to_owned(), work.clone()),
        ("personal".to_owned(), String::new()),
    ];
    let read = carried(&sessions, &[], "here");
    assert!(
        read.target.is_none()
            && read.behind == ["here: 1 busy, 0 listening"]
            && read
                .says
                .contains("the personal realm's node didn't answer"),
        "a realm unread was judged without its sessions, or the others miscounted: {}",
        read.says
    );
    let both = carried(
        &[
            ("work".to_owned(), String::new()),
            ("personal".to_owned(), String::new()),
        ],
        &[],
        "here",
    );
    assert!(
        both.says
            .contains("the work and personal realms' nodes didn't answer"),
        "two silent realms were misworded: {}",
        both.says
    );
    let none = carried(&[], &[], "here");
    assert!(
        none.target.is_none() && none.behind == ["here: 0 busy, 0 listening"],
        "an hour asking no realm was judged: {}",
        none.says
    );
}

#[test]
fn a_machine_no_node_names_is_unjudged_while_every_busy_session_could_be_here() {
    let there = [MACHINES.replace("\"here\"", "\"there\"")];
    let unnamed = |busy: usize| {
        let away: Vec<Value> = (0..busy)
            .map(|n| session(&format!("s{n}"), "mac", "busy", "busy"))
            .collect();
        carried(&[("work".to_owned(), answer(&away))], &there, "here")
    };
    let (few, many) = (unnamed(1), unnamed(11));
    assert!(
        few.target.is_some()
            && many.target.is_none()
            && many.says.contains("no node names this machine, here"),
        "a machine no node names was misjudged: {} / {}",
        few.says,
        many.says
    );
    // Named by the nodes, or by a session alone, it is named.
    let by_node = alone(&[session("a", "mac", "busy", "busy")]);
    let by_session = carried(&[("work".to_owned(), answer(&[]))], &there, "here");
    let listed_here = answer(&[session("a", "here", "busy", "busy")]);
    let by_listing = carried(&[("work".to_owned(), listed_here)], &there, "here");
    for read in [by_node, by_listing] {
        assert!(
            !read.says.contains("names this machine"),
            "a machine its node or a session names was said unnamed: {}",
            read.says
        );
    }
    assert!(
        by_session.says.contains("no node names this machine, here"),
        "a machine nothing names went unsaid: {}",
        by_session.says
    );
    let away = answer(&[session("a", "mac", "busy", "busy")]);
    let dim = carried(&[("work".to_owned(), away)], &[String::new()], "here");
    assert!(
        dim.says
            .contains("no machines read, nor a session naming this machine, here"),
        "every machines answer unread was blamed on the node's names: {}",
        dim.says
    );
}

#[test]
fn sessions_naming_no_machine_are_a_row_among_the_busiest_and_judged_while_few() {
    let read = alone(&[
        session("a", "here", "busy", "busy"),
        session("b", "", "busy", "busy"),
        json!({"id": "c", "state": "busy", "presence": {"state": "busy"}}),
        session("d", "here", "idle", "listening"),
    ]);
    assert_eq!(
        read.behind,
        [
            "no machine named: 2 busy, 0 listening",
            "here: 1 busy, 1 listening",
            "cloud: 0 busy, 0 listening",
            "mac: 0 busy, 0 listening",
        ],
        "sessions naming no machine dropped out, or their row fell below idle machines"
    );
    assert_eq!(
        (judged(&read), read.says.as_str()),
        (
            ("1".to_owned(), Some("10".to_owned())),
            "sessions busy on this machine, of 1 busy on 3 machines (2 busy naming no machine)",
        ),
        "busy sessions naming no machine, too few to pass the most, left it unjudged"
    );
    let crowded: Vec<Value> = (0..11)
        .map(|n| {
            let machine = if n < 9 { "here" } else { "" };
            session(&format!("s{n}"), machine, "busy", "busy")
        })
        .collect();
    let read = alone(&crowded);
    assert!(
        read.target.is_none() && read.says.ends_with("(2 busy naming no machine)"),
        "busy sessions naming no machine that could pass the most were judged: {}",
        read.says
    );
    let lone = alone(&[
        session("a", "here", "busy", "busy"),
        session("b", "", "idle", "listening"),
    ]);
    let row = lone.behind.get(1).map(String::as_str);
    assert_eq!(
        (lone.target.is_some(), row),
        (true, Some("no machine named: 0 busy, 1 listening")),
        "a session listening on no machine went unsaid, or left the count unjudged"
    );
}

#[test]
fn a_session_with_no_id_counts_once_per_listing_and_never_folds_into_another() {
    let blank = alone(&[
        session("", "here", "busy", "busy"),
        session("", "here", "busy", "busy"),
    ]);
    let alike = alone(&[
        session("work #1", "here", "busy", "busy"),
        nameless("here"),
    ]);
    for read in [blank, alike] {
        assert_eq!(
            read.now.to_string(),
            "2",
            "two sessions folded into one: {:?}",
            read.behind
        );
    }
    let addressed = json!({
        "address": "ABC123", "machine": "here", "state": "busy", "presence": {"state": "busy"}
    });
    let listed = answer(std::slice::from_ref(&addressed));
    for read in both_ways(&listed, &listed) {
        assert_eq!(
            read.now.to_string(),
            "1",
            "a session two realms list by address alone counted twice"
        );
    }
}

#[test]
fn nameless_sessions_pass_the_most_alone_only_where_one_realm_answered() {
    let eleven: Vec<Value> = (0..11).map(|_| nameless("here")).collect();
    let six: Vec<Value> = (0..6).map(|_| nameless("here")).collect();
    let one = carried(
        &[
            ("work".to_owned(), answer(&eleven)),
            ("personal".to_owned(), String::new()),
        ],
        &[MACHINES.to_owned()],
        "here",
    );
    let two = carried(
        &[
            ("work".to_owned(), answer(&six)),
            ("personal".to_owned(), answer(&six)),
            ("third".to_owned(), String::new()),
        ],
        &[MACHINES.to_owned()],
        "here",
    );
    assert!(
        one.target.is_some() && two.target.is_none(),
        "nameless sessions two realms may both list read as surely past: {} / {}",
        one.says,
        two.says
    );
}
