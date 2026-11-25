//! The dates at risk, read from asks lists in the shapes the list's
//! keeper writes, with what main changed given rather than asked of git.

use serde_json::{Value, json};

use super::super::slipped::{at, list};
use super::*;

const NOW: i64 = 1_794_700_000;

/// An open ask by `owner`, its merge piece `what` due at `due` waiting on
/// `pr`, with `more` fields on the piece.
fn ask_waiting(what: &str, owner: &str, (due, pr): (i64, &str), more: &Value) -> Value {
    let mut piece = json!({
        "what": what, "owner": owner, "expected": at(due), "landed": null, "pr": pr,
    });
    if let (Some(piece), Some(more)) = (piece.as_object_mut(), more.as_object()) {
        piece.extend(more.clone());
    }
    json!({
        "at": at(NOW - DAY), "words": format!("the ask behind {what}"), "owner": owner,
        "expected": at(due + DAY), "landed": null, "pr": null, "by": "agent",
        "pieces": [
            {"what": "a step before", "owner": owner, "expected": at(NOW - DAY),
             "landed": at(NOW - DAY), "pr": null},
            piece,
        ],
    })
}

/// The files main changed beside core change `number`: `README.md`
/// beside 6595, and nothing beside any other.
fn beside(number: u32) -> Vec<String> {
    if number == 6595 {
        vec!["README.md".to_owned()]
    } else {
        Vec::new()
    }
}

/// Joel's dates at risk in `text`, as [`read_as`] reads them.
fn read(text: &str) -> Measure {
    read_as(text, Tracking::His)
}

/// The dates at risk of the kind `which` in `text`, with main's changes
/// as [`beside`] gives them and every branch sharing nothing.
fn read_as(text: &str, which: Tracking) -> Measure {
    let core = |number| -> Shared { Ok(beside(number)) };
    let no_branch = |_: &str| -> Shared { Ok(Vec::new()) };
    at_risk(text, NOW, which, &core, &no_branch)
}

#[test]
fn each_form_the_keeper_has_written_names_its_change() {
    let cases: [(&str, &[(Repo, u32)]); 10] = [
        ("core.nix#6595", &[(Repo::Core, 6595)]),
        ("core#4057", &[(Repo::Core, 4057)]),
        ("Jitsusama/core.nix#4220", &[(Repo::Core, 4220)]),
        (
            "https://github.com/Jitsusama/core.nix/pull/6102",
            &[(Repo::Core, 6102)],
        ),
        ("world#2536168", &[(Repo::World, 2_536_168)]),
        ("World#2507660", &[(Repo::World, 2_507_660)]),
        ("harness#1555", &[(Repo::Harness, 1555)]),
        ("agentic-harness.pi#2211", &[(Repo::Harness, 2211)]),
        (
            "mycelium#371, #384, #393",
            &[
                (Repo::Mycelium, 371),
                (Repo::Mycelium, 384),
                (Repo::Mycelium, 393),
            ],
        ),
        ("4224", &[]),
    ];
    for (pr, named) in cases {
        assert_eq!(changes(pr), named, "{pr} read as other changes");
    }
}

#[test]
fn a_date_waiting_on_a_change_main_has_changed_beside_is_at_risk_and_named() {
    let text = list(&[ask_waiting(
        "the holds count merged",
        "1JQPPC",
        (NOW + 3 * 3600, "core.nix#6595"),
        &json!({}),
    )]);
    let m = read(&text);
    assert!((m.now - 1.0).abs() < 0.5, "one date at risk, got {}", m.now);
    assert_eq!(m.owner, "1JQPPC", "the piece's owner owns it");
    assert_eq!(
        m.behind,
        ["the holds count merged, due in 3 h, main changed README.md since it parted · 1JQPPC"],
        "the line names the piece, when it's due, the files and its owner"
    );
    assert_eq!(m.key, KEY, "Joel's dates go by the bare key");
}

#[test]
fn a_change_main_left_alone_or_a_date_already_past_is_not_at_risk() {
    let text = list(&[
        ask_waiting("apart", "A", (NOW + 3600, "core.nix#6000"), &json!({})),
        ask_waiting("passed", "B", (NOW - 60, "core.nix#6595"), &json!({})),
    ]);
    let m = read(&text);
    assert!(
        m.behind.is_empty(),
        "a change main left alone, or a date that's a slip now, counted: {:?}",
        m.behind
    );
}

#[test]
fn a_landed_or_closed_piece_or_a_closed_parked_or_confirmed_ask_waits_on_nothing() {
    let piece = |what, more: Value| ask_waiting(what, "A", (NOW + 3600, "core.nix#6595"), &more);
    let ask = |what, more: Value| {
        let mut ask = piece(what, json!({}));
        if let (Some(ask), Some(more)) = (ask.as_object_mut(), more.as_object()) {
            ask.extend(more.clone());
        }
        ask
    };
    let text = list(&[
        piece("landed", json!({"landed": at(NOW - 60)})),
        piece("dropped", json!({"closed": "dropped"})),
        ask("put down", json!({"closed": "put down"})),
        ask("confirmed", json!({"confirmed": at(NOW - 60)})),
        ask("parked", json!({"parked": "b95ea57b"})),
    ]);
    let m = read(&text);
    assert!(
        m.behind.is_empty(),
        "a piece or ask that waits on no merge counted: {:?}",
        m.behind
    );
}

#[test]
fn an_ask_with_no_pieces_waits_on_its_own_change() {
    let ask = json!({
        "at": at(NOW - DAY), "words": "a lone fix", "owner": "A",
        "expected": at(NOW + 2 * DAY + 60), "landed": null, "pr": "core#6595",
    });
    let m = read(&list(&[ask]));
    assert_eq!(
        m.behind,
        ["a lone fix, due in 2 days, main changed README.md since it parted · A"],
        "an ask's own pr went unread"
    );
}

#[test]
fn a_world_piece_is_asked_by_its_branch_and_one_with_none_is_unread() {
    let world = |branch: &str| -> Shared {
        Ok(if branch == "gates" {
            ["README.md", "a", "b", "c"].map(str::to_owned).to_vec()
        } else {
            Vec::new()
        })
    };
    let text = list(&[
        ask_waiting(
            "the gate",
            "A",
            (NOW + 600, "world#2536168"),
            &json!({"branch": "gates"}),
        ),
        ask_waiting("no branch", "B", (NOW + 600, "world#2536169"), &json!({})),
    ]);
    let m = at_risk(&text, NOW, Tracking::His, &|n| Ok(beside(n)), &world);
    assert_eq!(
        m.behind,
        ["the gate, due in 10 min, main changed README.md, a, b and 1 more since it parted · A"],
        "the branch's files, or how many past the first three, read wrong"
    );
    assert!(
        m.says.ends_with("(1 unread)"),
        "a piece with no branch passed for none at risk: {}",
        m.says
    );
}

#[test]
fn a_change_that_cannot_be_read_is_said_unread_not_safe() {
    let unreadable = |_: u32| -> Shared { Err("the fetch failed".to_owned()) };
    let text = list(&[ask_waiting(
        "a fix",
        "A",
        (NOW + 600, "core.nix#6595"),
        &json!({}),
    )]);
    let m = at_risk(&text, NOW, Tracking::His, &unreadable, &|_| Ok(Vec::new()));
    assert!(m.behind.is_empty(), "an unread change counted at risk");
    assert!(
        m.says.ends_with("(1 unread)"),
        "an unread change passed for none at risk: {}",
        m.says
    );
}

#[test]
fn the_soonest_date_owns_it_and_leads_the_lines() {
    let text = list(&[
        ask_waiting("later", "B", (NOW + 7200, "core.nix#6595"), &json!({})),
        ask_waiting("sooner", "A", (NOW + 120, "core.nix#6595"), &json!({})),
    ]);
    let m = read(&text);
    assert_eq!(m.owner, "A", "the soonest date's owner owns the measure");
    assert_eq!(
        m.behind,
        [
            "sooner, due in 2 min, main changed README.md since it parted · A",
            "later, due in 2 h, main changed README.md since it parted · B",
        ],
        "the lines aren't in the order the dates come"
    );
}

/// The zone gate's line for a branch, `at`, of `kind`, sharing `overlap`.
fn gate_line(branch: &str, at_seconds: i64, kind: &str, overlap: &[&str]) -> Value {
    json!({
        "at": at(at_seconds), "commit": "c", "base": "b", "main": "m", "ours": overlap,
        "overlap": overlap, "branch": branch, "change": null, "kind": kind,
    })
}

#[test]
fn a_branch_shares_what_its_latest_gate_line_says() {
    let refusals = list(&[
        gate_line("gates", NOW - 3000, "drift", &[]),
        gate_line("gates", NOW - 600, "refusal", &["README.md"]),
        gate_line("other", NOW - 60, "drift", &["b"]),
    ]);
    assert_eq!(
        drifted(Some(&refusals), "gates", NOW),
        Ok(vec!["README.md".to_owned()]),
        "the branch's latest line didn't decide"
    );
}

#[test]
fn no_record_no_line_or_a_stale_one_is_unread_not_none_shared() {
    let refusals = list(&[gate_line("old", NOW - 3 * 3600, "drift", &[])]);
    for (case, read) in [
        ("no record", drifted(None, "gates", NOW)),
        ("no line", drifted(Some(&refusals), "gates", NOW)),
        ("a stale line", drifted(Some(&refusals), "old", NOW)),
    ] {
        assert!(read.is_err(), "{case} read as {read:?}");
    }
}

#[test]
fn an_ask_is_read_from_its_latest_line_and_the_list_s_own_apart() {
    let waiting = ask_waiting("a fix", "A", (NOW + 600, "core.nix#6595"), &json!({}));
    let mut landed = waiting.clone();
    landed["pieces"][1]["landed"] = json!(at(NOW - 60));
    let mut derived = ask_waiting("a mend", "B", (NOW + 600, "core.nix#6595"), &json!({}));
    derived["tracking"] = json!("derived");
    let text = list(&[waiting, landed, derived]);
    let his = read(&text);
    let ours = read_as(&text, Tracking::Derived);
    assert!(
        his.behind.is_empty(),
        "a piece landed on its latest line, or a derived ask, counted as Joel's: {:?}",
        his.behind
    );
    assert_eq!(ours.behind.len(), 1, "the derived ask went uncounted");
    assert_eq!(ours.key, "dates-at-risk-derived", "the list's own key");
}
