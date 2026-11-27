//! The misses ledger as the listener writes it: a line a pain sent, once,
//! its check set once a landing closed it, and the keeper's own lines
//! left as they were.

use super::*;
use crate::hear::heard::Landing;

/// 01:30 UTC on 22 November 2026, 20:30 on the 21st at his offset.
const AT: i64 = 1_795_311_000_000;

const OFFSET: i64 = -5 * 3600;

/// Nobody a pain carried unsent may be owed to.
const NOBODY: Owning = Owning {
    lines: Vec::new(),
    keeper: None,
    apart: BTreeSet::new(),
};

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
    // Carried by the keeper's own line here: said again, the miss came back.
    let mut quoted = sent("q", "fuck.");
    quoted.carried = true;
    let colour = sent("e1", "the bar is the wrong colour");
    let kept = [colour, void, carried, unsent, quoted];
    write(&ledger, &kept, &NOBODY, OFFSET, 0).expect("written");
    write(&ledger, &kept, &NOBODY, OFFSET, 0).expect("written again");
    let lines = read(&ledger);
    assert_eq!(lines.len(), 4, "the keeper's and the three sent: {lines:?}");
    assert_eq!(
        lines[0]["words"], "fuck.",
        "the keeper's line is kept as it was"
    );
    assert_eq!(
        lines[1],
        json!({"at": "2026-11-21T20:30:00-05:00", "words": "the bar is the wrong colour",
               "owner": "ABC123", "pain": "s/e1", "heard_by": "pain-points"}),
        "the void's and the unsent aren't written"
    );
    // One the keeper carried first is still a miss, which needs a test
    // that runs, though the keeper's ledger tracks its fix.
    assert_eq!(
        (&lines[2]["pain"], &lines[2]["owner"], &lines[2]["carried"]),
        (&json!("s/c"), &json!("ABC123"), &json!(true)),
        "a pain carried before it was written wasn't written as carried, to its owner"
    );
    assert_eq!(
        (&lines[3]["pain"], &lines[3]["carried"]),
        (&json!("s/q"), &json!(true)),
        "a miss come back, said as the keeper's line said it, wasn't written"
    );
    assert!(
        fs::read_to_string(&ledger)
            .expect("the ledger")
            .ends_with("}\n"),
        "each line ends whole"
    );
}

#[test]
fn a_pain_said_in_a_cell_kept_apart_gains_no_line_but_an_old_line_its_check() {
    let ledger = folder("apart").join("misses.jsonl");
    // Written before its cell was kept apart, then closed by a landing.
    let before = sent("e0", "the old errand");
    fs::write(&ledger, line_of(&before, "ABC123", OFFSET) + "\n").expect("its line");
    let fixed = closed(before);
    let errand = sent("e1", "the budget sheet is wrong");
    // Carried unsent by the keeper, whose line names it, yet said apart.
    let carried = Heard {
        to: None,
        routed_at: None,
        carried: true,
        ..sent("e3", "the errand the keeper quoted")
    };
    let work = Heard {
        to: Some("GHI789".into()),
        ..sent("e2", "the bar is the wrong colour")
    };
    let owning = Owning {
        keeper: Some("ORCH01".into()),
        apart: ["s/e0", "s/e1", "s/e3"].map(str::to_owned).into(),
        ..NOBODY
    };
    write(&ledger, &[fixed, errand, carried, work], &owning, OFFSET, 0).expect("written");
    let lines = read(&ledger);
    assert_eq!(
        lines.iter().map(|l| l["pain"].clone()).collect::<Vec<_>>(),
        [json!("s/e0"), json!("s/e2")],
        "a pain said in a cell kept apart was written, sent or carried, or another wasn't"
    );
    assert_eq!(
        lines[0]["closed_by"],
        json!("255494bb414c"),
        "a line written before its cell was kept apart didn't gain its check"
    );
}

#[test]
fn a_pain_closed_since_gains_its_check_and_a_carried_one_says_so() {
    let ledger = folder("checked").join("misses.jsonl");
    let kept = [
        sent("e1", "the bar is the wrong colour"),
        sent("e2", "the clock is out"),
    ];
    write(&ledger, &kept, &NOBODY, OFFSET, 0).expect("written");
    let mut carried = kept[1].clone();
    carried.carried = true;
    let later = [closed(kept[0].clone()), carried];
    write(&ledger, &later, &NOBODY, OFFSET, 0).expect("written again");
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
    write(&first, &first_closed, &NOBODY, OFFSET, 0).expect("written");
    assert_eq!(
        read(&first)[0]["closed_by"],
        "255494bb414c",
        "one closed before it was ever written is written with its check"
    );
}

/// A pain the keeper carried before it was sent: never sent, so never
/// routed, carried.
fn carried_unsent(entry: &str, words: &str) -> Heard {
    let mut pain = Heard {
        to: None,
        routed_at: None,
        ..sent(entry, words)
    };
    pain.carried = true;
    pain
}

fn lines(of: &[(&str, &str)]) -> Vec<Line> {
    of.iter()
        .map(|(words, owner)| Line {
            words: (*words).into(),
            owner: (*owner).into(),
        })
        .collect()
}

#[test]
fn only_a_named_owner_the_routes_allow_owns_a_miss() {
    let routes = |keeper: &str| Routes {
        from: None,
        fallback: None,
        ledgers: Some(keeper.into()),
        never: vec!["VOID01".into()],
        told: Vec::new(),
        apart: Vec::new(),
    };
    // Blank, spaces alone, refused in another case, and padded.
    let ledgers = lines(&[("a", ""), ("b", "   "), ("c", "void01"), ("d", " Y2YBTS ")]);
    assert_eq!(
        owning(&ledgers, &routes(" ORCH01")),
        Owning {
            lines: lines(&[("d", "Y2YBTS")]),
            keeper: Some("ORCH01".into()),
            apart: BTreeSet::new(),
        },
        "a blank or refused owner was kept, or a named one or the keeper wasn't trimmed"
    );
    assert_eq!(
        owning(&[], &routes("Void01")).keeper,
        None,
        "a keeper the routes refuse was kept"
    );
}

#[test]
fn a_pain_carried_before_it_was_sent_is_written_to_the_owner_its_carrying_line_names() {
    let ledger = folder("carried-unsent").join("misses.jsonl");
    // As `owning` leaves them: the clock's carrying line named no owner,
    // so only a later line naming one is left to own it.
    let mut owning = Owning {
        lines: lines(&[
            ("the launcher is slow to open", "Y2YBTS"),
            ("the clock is out", "KP0L6A"),
        ]),
        keeper: None,
        apart: BTreeSet::new(),
    };
    let mut early = carried_unsent("e0", "the launcher is slow to open again");
    early.at = AT - 1;
    let uncarried = Heard {
        to: None,
        routed_at: None,
        ..sent("e3", "the launcher is slow to open")
    };
    let kept = [
        carried_unsent("e1", "the launcher is slow to open"),
        carried_unsent("e2", "the clock is out"),
        carried_unsent("e4", "the bar is the wrong colour"),
        early,
        uncarried,
    ];
    write(&ledger, &kept, &owning, OFFSET, AT).expect("written");
    write(&ledger, &kept, &owning, OFFSET, AT).expect("written again");
    let owned: Vec<(Value, Value, Value)> = read(&ledger)
        .into_iter()
        .map(|line| {
            (
                line["pain"].clone(),
                line["owner"].clone(),
                line["carried"].clone(),
            )
        })
        .collect();
    assert_eq!(
        owned,
        [
            (json!("s/e1"), json!("Y2YBTS"), json!(true)),
            (json!("s/e2"), json!("KP0L6A"), json!(true))
        ],
        "one carried unsent wasn't written once to its line's owner, or one no owning \
         line carries, one heard before the ledger's start or one never carried was"
    );
    owning.keeper = Some("ORCH01".into());
    write(&ledger, &kept, &owning, OFFSET, AT).expect("written with a keeper");
    let kept_by = read(&ledger)
        .into_iter()
        .find(|line| line["pain"] == "s/e4");
    assert_eq!(
        kept_by.map(|line| line["owner"].clone()),
        Some(json!("ORCH01")),
        "one no owning line carries wasn't written to the keeper that carried it"
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
    let why = write(&ledger, &one, &NOBODY, OFFSET, 0).expect_err("it waits");
    assert!(why.contains("stayed locked"), "{why}");
    assert!(!ledger.exists(), "nothing written past the lock");
}

#[test]
fn a_ledger_written_whole_ends_each_line_once_and_holds_no_pain_sent_before_it() {
    let ledger = folder("whole").join("misses.jsonl");
    let keepers = r#"{"at":"2026-11-21T20:30:00-05:00","words":"fuck.","owner":"KP0L6A"}"#;
    let pain = sent("e1", "the bar is the wrong colour");
    let mut early = sent("e0", "said before the ledger's start");
    early.routed_at = Some(AT);
    let theirs = format!("{}\n{keepers}", line_of(&pain, "ABC123", OFFSET));
    fs::write(&ledger, theirs).expect("a ledger ending mid-line");
    let kept = [closed(pain), early];
    write(&ledger, &kept, &NOBODY, OFFSET, AT + 30_000).expect("written");
    let text = fs::read_to_string(&ledger).expect("the ledger");
    assert!(
        !text.contains("\n\n") && text.ends_with("}\n"),
        "a blank line, or one left open: {text}"
    );
    let lines = read(&ledger);
    assert_eq!(lines.len(), 2, "the closed one and the keeper's: {lines:?}");
    assert_eq!(
        lines[0]["closed_by"], "255494bb414c",
        "the one sent since its start is checked"
    );
}
