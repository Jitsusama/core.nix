use super::*;
use crate::hour::measures;
use crate::tracking::Tracking;
use jiff::tz::TimeZone;

const NOW: i64 = 1_795_600_000;

/// The dates at risk of the kind `which` in an empty asks list.
fn at_risk(which: Tracking) -> Measure {
    let none = |_: u32| -> measures::at_risk::Shared { Ok(Vec::new()) };
    let no_branch = |_: &str| -> measures::at_risk::Shared { Ok(Vec::new()) };
    measures::at_risk::at_risk("", NOW, which, &none, &no_branch)
}

/// A measure the hour would show under `key`, held to `target`.
fn held(key: &str, target: Option<f64>) -> Measure {
    Measure {
        key: key.to_owned(),
        class: Class::Cells,
        says: key.to_owned(),
        now: 0.0,
        target,
        owner: "OWNER".to_owned(),
        behind: Vec::new(),
    }
}

#[test]
fn the_page_gives_each_measure_a_target_a_line_of_joels_and_its_cost() {
    let faults = unsaid(&sections(PAGE));
    assert!(
        faults.is_empty(),
        "the definition of good says too little: {faults:?}"
    );
}

#[test]
fn every_measure_the_hour_shows_has_a_line_that_holds_it_to_its_target() {
    let mut shown = vec![
        measures::slips("", NOW, Tracking::His),
        measures::slips("", NOW, Tracking::Derived),
        measures::parked("", Tracking::His),
        measures::parked("", Tracking::Derived),
        measures::told_late("", NOW, Tracking::His),
        measures::told_late("", NOW, Tracking::Derived),
        measures::to_first_slice("", NOW, Tracking::His),
        measures::to_first_slice("", NOW, Tracking::Derived),
        measures::undated("", "2026-11-24", "OWNER"),
        measures::reverts("", NOW, "OWNER"),
        measures::crowded(&[], "OWNER"),
        measures::pains(None, NOW, "OWNER"),
        measures::wifi::video(None, &TimeZone::UTC),
        measures::unbilled::unbilled(None, None, NOW, "OWNER"),
        measures::unresolved::unresolved(None, None, NOW, "OWNER"),
        measures::carried(&[], &[], "here"),
        measures::mapped(None, "", "OWNER"),
        measures::cited::unread(""),
        measures::holds::holds(&[], "", NOW),
        at_risk(Tracking::His),
        at_risk(Tracking::Derived),
    ];
    shown.extend(measures::apps::apps(None, NOW));
    // Each of the relay's four judged, the share between two tallies.
    shown.extend(measures::way_out::relay::relay(Some(
        "model-relay: 10 asked, 0 refused, 0 failed\nmodel-relay: 19 asked, 0 refused, 0 failed",
    )));
    // The calls judged with one call in the hour, and a path of three samples.
    let call = measures::way_out::calls::Turn {
        at_ms: NOW * 1000 - 60_000,
        error: None,
    };
    shown.extend(measures::way_out::calls::calls(Some(&[call]), NOW));
    let sample = |ago: i64| {
        serde_json::json!({"at": NOW - ago, "origin": "forge.example", "tls_s": 0.25}).to_string()
    };
    let samples = [sample(60), sample(660), sample(1260)].join("\n");
    shown.extend(measures::way_out::path::path(
        &samples,
        &["forge.example".to_owned()],
        NOW,
    ));
    // The site judged on three samples of one watch.
    let live =
        |ago: i64| serde_json::json!({"at": NOW - ago, "kinds": {"cell": 0.4}}).to_string();
    let site_lines = [live(60), live(660), live(1260)].join("\n");
    shown.push(measures::site::site(&site_lines, NOW));
    let said = serde_json::json!({"none": "no pain", "none_at_ms": NOW * 1000});
    shown.extend(measures::said_none(&said.to_string(), NOW));
    // Those read from a machine's records, under the keys and targets the
    // hour gives them, each target read where the hour holds it; a
    // family's by one of its keys.
    let at_most = [
        ("misses", 0.0),
        (measures::UNTESTED, 0.0),
        (measures::FINDINGS, 0.0),
        ("broken", 0.0),
        (measures::locked::KEY, measures::locked::AT_MOST_MIN),
        (measures::wifi::KEY, measures::wifi::TARGET),
        (measures::apps::UNREAD, measures::apps::LOST),
        ("reports-unasked", 0.0),
        ("map-cells", 0.0),
        ("cells-idle", 0.0),
        ("cells-parked-done", 0.0),
        ("quests-without-cells", 0.0),
        ("quests-cells-ended", 0.0),
        ("quests-parked-past-start", 0.0),
        (measures::rounds::KEY, measures::rounds::TARGET),
        ("rounds-past-five-fixed-nothing:#6372", 5.0),
        ("merge-to-on", 18.0),
        ("commit-to-merge", 20.0),
        ("apps:launcher-open", 50.0),
        ("clean:core:long-files", 0.0),
    ];
    shown.extend(at_most.map(|(key, most)| held(key, Some(most))));
    for queue in measures::queues::ALL {
        let key = format!("{}{}", measures::queues::QUEUE, queue.key);
        shown.push(held(&key, Some(queue.target)));
    }
    let unjudged = ["landed", "balance", measures::spent::KEY];
    shown.extend(unjudged.map(|key| held(key, None)));
    let undefined = undefined(&shown);
    assert_eq!(
        undefined.behind,
        Vec::<String>::new(),
        "a measure the hour shows has no line, or another target"
    );
}

#[test]
fn a_measure_with_no_section_or_another_target_is_counted() {
    let page = "# Good\n\n## Held\n\n- Measures: `held`, `family:*`\n- Target: 2 minutes.\n\
                - Joel, 1 Nov: \"good\"\n- Over costs him: much.\n\n## Shown\n\n\
                - Measures: `shown`\n- Target: none.\n- Joel, 1 Nov: \"good\"\n\
                - Over costs him: nothing.\n\n## Read\n\n- Measures: `read`\n\
                - Target: set where it's read: the source's own.\n\
                - Joel, 1 Nov: \"good\"\n- Over costs him: much.\n";
    let cases = [
        (held("held", Some(2.0)), None),
        (held("held", None), None),
        (held("family:one", Some(2.0)), None),
        (held("shown", None), None),
        (held("read", Some(7.0)), None),
        (
            held("unnamed", None),
            Some("unnamed: no section says what good is"),
        ),
        (
            held("held", Some(3.0)),
            Some("held: held to 3 here, 2 on the page"),
        ),
        (
            held("family:two", Some(1.0)),
            Some("family:two: held to 1 here, 2 on the page"),
        ),
        (
            held("shown", Some(0.0)),
            Some("shown: held to 0 here, unjudged"),
        ),
    ];
    for (measure, fault) in cases {
        let read = judged(page, std::slice::from_ref(&measure));
        assert_eq!(
            read.behind,
            fault.map(str::to_owned).into_iter().collect::<Vec<_>>(),
            "{} held to {:?} was judged wrongly",
            measure.key,
            measure.target
        );
    }
}

#[test]
fn a_section_saying_less_than_all_four_or_a_key_twice_is_counted() {
    let page = "# Good\n\n## Bare\n\n- Measures: `bare`\n\n## Unquoted\n\n\
                - Measures: `bare`, `other`\n- Target: 0.\n- Joel, 1 Nov: good\n\
                - Over costs him: much.\n\n## Undated\n\n- Measures: `third`\n- Target: 0.\n\
                - Joel, sometime: \"good\"\n- Over costs him:\n  much, once wrapped.\n\n\
                ## Worded\n\n- Measures: `family:*`\n- Target: six.\n\
                - Joel, 1 Nov: \"good\"\n- Over costs him: much.\n\n## Keyless\n\n\
                - Target: 0.\n- Joel, 1 Nov: \"good\"\n- Over costs him: much.\n\n\
                ## Overlap\n\n- Measures: `family:one`\n- Target: 0.\n\
                - Joel, 1 Nov: \"good\"\n- Over costs him: much.\n";
    assert_eq!(
        judged(page, &[]).behind,
        [
            "Bare: no target, no dated line of Joel's, no cost to him",
            "Unquoted: no dated line of Joel's",
            "bare: named twice",
            "Undated: no dated line of Joel's",
            "Worded: no target",
            "Keyless: no measure",
            "family:one: named twice",
        ],
        "a section's faults were misread"
    );
}

#[test]
fn an_items_wrapped_lines_are_read_as_one() {
    let page = "# Good\n\n## Wrapped\n\n- Measures: `one`,\n  `two`\n- Target: 0.\n\
                - Joel, 1 Nov: \"a line\n\twrapped\"\n- Over costs him: much.\n";
    let read = sections(page);
    assert_eq!(
        (&read[0].keys, read[0].joel.as_deref()),
        (
            &vec!["one".to_owned(), "two".to_owned()],
            Some("Joel, 1 Nov: \"a line wrapped\"")
        ),
        "a wrapped item wasn't read whole: {read:?}"
    );
    assert!(
        judged(page, &[]).behind.is_empty(),
        "a page wrapped by spaces and a tab was faulted"
    );
}

#[test]
fn a_line_or_item_no_field_reads_or_a_key_holding_a_space_is_counted() {
    let page = "# Good\n\n## Loose\n\n- Measures: `one`\n- Target: 0.\n  - Target: 9.\n\
                left unindented, with more words than six\n  indented after it\n\
                - Joel, 1 Nov: \"good\"\n- Joel, 2 Nov: \"again\"\n\
                - Over costs him: much,\n\n  after a blank.\n- Over costs him: again.\n\
                - Over costs him: thrice.\n- Owner: me.\n\n## Broken\n\n\
                - Measures: `two\n  three`\n- Target: 0.\n- Joel, 1 Nov: \"good\"\n\
                - Over costs him: much.\n";
    assert_eq!(
        sections(page)[0].cost.as_deref(),
        Some("much,"),
        "a line after a blank was joined to the item above it"
    );
    // Each line here joined to the item above it would leave this list.
    assert_eq!(
        judged(page, &[]).behind,
        [
            "Loose: under no item: - Target: 9.",
            "Loose: under no item: left unindented, with more words than",
            "Loose: under no item: indented after it",
            "Loose: under no item: after a blank.",
            "Loose: Joel said again: Joel, 2 Nov: \"again\"",
            "Loose: Over costs him said again: Over costs him: again.",
            "Loose: Over costs him said again: Over costs him: thrice.",
            "Loose: no such field: Owner: me.",
            "Broken: two three holds a space, as a key broken across lines does",
        ],
        "a line or item no field reads, or a key holding a space, went uncounted"
    );
}
