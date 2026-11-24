//! The misses ledger as the listener writes it: a line a pain sent, once,
//! its check set once a landing closed it, and the keeper's own lines
//! left as they were.

use super::*;
use crate::hear::heard::Landing;

/// 01:30 UTC on 22 November 2026, 20:30 on the 21st at his offset.
const AT: i64 = 1_795_311_000_000;

const OFFSET: i64 = -5 * 3600;

fn sent(entry: &str, words: &str) -> Heard {
    serde_json::from_value(json!({
        "key": format!("s/{entry}"), "session": "s", "entry": entry, "at": AT,
        "words": words, "kinds": ["corrected"], "hits": 1, "carried": false,
        "to": "ABC123", "routed_at": AT + 60_000
    }))
    .expect("a pain")
}

fn closed(mut pain: Heard) -> Heard {
    pain.landed.push(Landing {
        commit: "255494bb414c".into(),
        said: "ABC123 landed core #5342, merged at 255494bb414c".into(),
        by_entry: false,
    });
    pain.closed_by = Some("255494bb414c".into());
    pain
}

/// A folder of the test's own, emptied first.
fn folder(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("missed-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("a folder");
    dir
}

fn read(path: &Path) -> Vec<Value> {
    fs::read_to_string(path)
        .expect("the ledger")
        .lines()
        .map(|line| serde_json::from_str(line).expect("a line"))
        .collect()
}

#[test]
fn a_moment_is_said_as_the_keeper_writes_one() {
    assert_eq!(
        stamp(AT, OFFSET),
        "2026-11-21T20:30:00-05:00",
        "his evening"
    );
    assert_eq!(stamp(AT, 0), "2026-11-22T01:30:00+00:00", "UTC");
    assert_eq!(stamp(0, 0), "1970-01-01T00:00:00+00:00", "the epoch");
    assert_eq!(
        stamp(951_782_400_000, 19_800),
        "2000-02-29T05:30:00+05:30",
        "a leap day, half an hour out"
    );
}

#[test]
fn a_pain_sent_is_written_once_beside_the_keepers_lines() {
    let ledger = folder("once").join("misses.jsonl");
    let keepers = r#"{"at":"2026-11-21T20:30:00-05:00","words":"fuck.","owner":"KP0L6A"}"#;
    fs::write(&ledger, keepers).expect("the keeper's ledger");
    let mut void = sent("v", "said to the void");
    void.void = true;
    let mut carried = sent("c", "the keeper has this one");
    carried.carried = true;
    let unsent = Heard {
        to: None,
        ..sent("u", "sent to no one yet")
    };
    let colour = sent("e1", "the bar is the wrong colour");
    let kept = [colour, void, carried, unsent];
    write(&ledger, &kept, OFFSET).expect("written");
    write(&ledger, &kept, OFFSET).expect("written again");
    let lines = read(&ledger);
    assert_eq!(lines.len(), 2, "the keeper's and the one sent: {lines:?}");
    assert_eq!(
        lines[0]["words"], "fuck.",
        "the keeper's line is kept as it was"
    );
    assert_eq!(
        lines[1],
        json!({"at": "2026-11-21T20:30:00-05:00", "words": "the bar is the wrong colour",
               "owner": "ABC123", "pain": "s/e1", "heard_by": "pain-points"}),
        "the void's, the carried and the unsent aren't written"
    );
    assert!(
        fs::read_to_string(&ledger)
            .expect("the ledger")
            .ends_with("}\n"),
        "each line ends whole"
    );
}

#[test]
fn a_pain_closed_since_gains_its_check_and_a_carried_one_says_so() {
    let ledger = folder("checked").join("misses.jsonl");
    let kept = [
        sent("e1", "the bar is the wrong colour"),
        sent("e2", "the clock is out"),
    ];
    write(&ledger, &kept, OFFSET).expect("written");
    let mut carried = kept[1].clone();
    carried.carried = true;
    let later = [closed(kept[0].clone()), carried];
    write(&ledger, &later, OFFSET).expect("written again");
    let lines = read(&ledger);
    assert_eq!(lines.len(), 2, "no line twice: {lines:?}");
    assert_eq!(
        (&lines[0]["check"], &lines[0]["closed_by"]),
        (
            &json!("ABC123 landed core #5342, merged at 255494bb414c"),
            &json!("255494bb414c")
        ),
        "its check names what landed"
    );
    assert_eq!(lines[1]["carried"], true, "the keeper carries it from now");
    let first = folder("closed-first").join("misses.jsonl");
    let first_closed = [closed(kept[0].clone())];
    write(&first, &first_closed, OFFSET).expect("written");
    assert_eq!(
        read(&first)[0]["closed_by"],
        "255494bb414c",
        "one closed before it was ever written is written with its check"
    );
}

#[test]
fn a_ledger_held_by_another_writer_is_left_and_said() {
    let place = folder("held");
    let ledger = place.join("misses.jsonl");
    let held = File::options()
        .create(true)
        .truncate(false)
        .write(true)
        .open(place.join("misses.jsonl.lock"))
        .expect("the lock");
    held.lock().expect("held");
    let one = [sent("e1", "the bar")];
    let why = write(&ledger, &one, OFFSET).expect_err("it waits");
    assert!(why.contains("stayed locked"), "{why}");
    assert!(!ledger.exists(), "nothing written past the lock");
}
