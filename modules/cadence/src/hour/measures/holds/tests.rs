//! The holds measure: which holds still live, and which an ask dates.

use std::fs;

use serde_json::Value;

use super::super::slipped::at;
use super::*;

const NOW: i64 = 1_795_600_000;

fn id(short: &str, ago: i64, from: &str) -> String {
    format!("{}-{from}-1-{short}", (NOW - ago) * 1000)
}

fn sent(short: &str, ago: i64, from: &str, intent: &str) -> Message {
    Message {
        v: 1,
        id: id(short, ago, from),
        at: at(NOW - ago),
        from: from.to_owned(),
        to: vec!["#machine".to_owned()],
        intent: intent.to_owned(),
        text: format!("{from}: hold {short}\nuntil its fix is live"),
        pid: 1,
        re: None,
        supersedes: None,
        ends_hold: false,
    }
}

fn hold(short: &str, ago: i64) -> Message {
    sent(short, ago, "HOLDER", "hold")
}

/// The short ids a measure names, in order.
fn named(m: &Measure) -> Vec<&str> {
    m.behind.iter().map(|line| &line[1..9]).collect()
}

fn ask(hold: &str, expected: Option<i64>) -> Value {
    serde_json::json!({
        "at": at(NOW - 3000), "words": "end it", "owner": "HOLDER",
        "hold": hold, "expected": expected.map(at),
    })
}

fn list(asks: &[Value]) -> String {
    asks.iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("\n")
}

/// On 24 November a hold on tree cuts held for hours with no ask dating
/// the fix that would end it, while another was dated.
#[test]
fn a_live_hold_no_ask_dates_is_past_its_target_and_named() {
    let dating = serde_json::json!({
        "at": at(NOW - 600), "words": "end the cargo hold", "owner": "ZZ9ZZ9",
        "hold": "0000000b", "expected": at(NOW + 1200),
    });
    let all = [
        sent("0000000c", 600, "16V6F4", "hold"),
        sent("0000000a", 3000, "AE2502BA", "hold"),
        sent("0000000b", 900, "ZZ9ZZ9", "hold"),
    ];
    let m = holds(&all, &list(&[dating]), NOW);
    assert!((m.now - 2.0).abs() < 0.5, "two undated, got {}", m.now);
    assert!(
        m.past().is_some(),
        "an undated hold isn't past its target of 0"
    );
    assert_eq!(
        m.behind,
        [
            "[0000000a] AE2502BA: hold 0000000a until its fix is live · AE2502BA",
            "[0000000c] 16V6F4: hold 0000000c until its fix is live · 16V6F4"
        ],
        "the undated holds weren't named oldest first by id, words and holder"
    );
    assert_eq!(m.owner, "AE2502BA", "the oldest undated hold's holder");
}

#[test]
fn a_hold_ends_only_as_the_harness_ends_one() {
    let mut ending = sent("e0000001", 60, "WCJQSG", "fyi");
    ending.re = Some(id("0000000a", 900, "HOLDER"));
    ending.ends_hold = true;
    let mut answered = sent("e0000002", 60, "16V6F4", "fyi");
    answered.re = Some(id("0000000b", 900, "HOLDER"));
    let mut own = sent("e0000003", 60, "HOLDER", "fyi");
    own.re = Some(id("0000000f", 900, "HOLDER"));
    let mut replacing = hold("0000000d", 300);
    replacing.supersedes = Some(id("0000000c", 1800, "HOLDER"));
    let mut stranger = sent("0000000j", 100, "OTHER", "hold");
    stranger.supersedes = Some(id("0000000g", 1200, "HOLDER"));
    let mut unstamped = hold("0000000i", 600);
    unstamped.at = String::new();
    let all = [
        hold("0000000a", 900),
        hold("0000000b", 900),
        hold("0000000c", 1800),
        hold("0000000e", LASTS + 60),
        hold("0000000f", 900),
        hold("0000000g", 1200),
        hold("0000000h", LASTS - 60),
        unstamped,
        replacing,
        stranger,
        ending,
        answered,
        own,
    ];
    assert_eq!(
        named(&holds(&all, "", NOW)),
        [
            "0000000h", "0000000g", "0000000b", "0000000i", "0000000d", "0000000j"
        ],
        "a hold ended by another's plain answer or replacement, or one live counted wrong"
    );
}

#[test]
fn only_an_open_asks_latest_line_with_a_date_to_come_dates_its_hold() {
    let soon = Some(NOW + 600);
    let both = [hold("0000000a", 600), hold("0000000b", 60)];
    let moved = list(&[ask("0000000a", soon), ask("0000000b", soon)]);
    assert_eq!(
        named(&holds(&both, &moved, NOW)),
        ["0000000a"],
        "the hold the ask moved off still read as dated, or the one it moved to didn't"
    );
    let one = [hold("0000000a", 600)];
    let mut landed = ask("0000000a", soon);
    landed["landed"] = at(NOW - 60).into();
    let mut closed = ask("0000000a", soon);
    closed["closed"] = "put down".into();
    for (asks, why) in [
        (
            list(&[ask("0000000a", soon), ask("0000000a", None)]),
            "its date dropped",
        ),
        (list(&[ask("0000000a", Some(NOW - 60))]), "its date passed"),
        (list(&[landed]), "landed"),
        (list(&[closed]), "closed"),
    ] {
        let m = holds(&one, &asks, NOW);
        assert!(
            (m.now - 1.0).abs() < 0.5,
            "an ask {why} still dated its hold"
        );
    }
    assert_eq!(
        holds(&[], "", NOW).owner,
        OWNER,
        "with no undated hold, velocity answers for the measure"
    );
}

#[test]
fn an_end_and_a_replacement_are_read_by_their_names_on_the_wire() {
    let text = r#"{"v":1,"id":"1-A-1-aa","at":"","from":"A","to":[],"intent":"fyi",
        "text":"over","pid":1,"re":"0-H-1-bb","endsHold":true,"supersedes":"0-H-1-cc"}"#;
    let read: Message = serde_json::from_str(text).expect("the harness's message reads");
    assert!(read.ends_hold, "endsHold wasn't read");
    assert_eq!(
        read.supersedes.as_deref(),
        Some("0-H-1-cc"),
        "supersedes wasn't read"
    );
    let written = serde_json::to_string(&sent("0000000a", 0, "A", "fyi")).expect("it writes");
    assert!(
        !written.contains("endsHold") && !written.contains("supersedes"),
        "a message that ends and replaces nothing said so: {written}"
    );
}

#[test]
fn the_hour_reads_holds_from_the_mesh_and_says_when_there_is_none() {
    let home = tempfile::tempdir().expect("a folder");
    let lost = read(home.path(), "", NOW);
    assert!(
        lost.past().is_some() && lost.owner == KEEPER,
        "a mesh it couldn't read read as no holds undated"
    );
    let folder = home
        .path()
        .join("pi/agentic-harness.pi/mesh-workflow/messages");
    fs::create_dir_all(&folder).expect("the messages folder");
    let on = sent("0000000a", 60, "AE2502BA", "hold");
    let file = folder.join(format!("{}.json", on.id));
    fs::write(file, serde_json::to_string(&on).expect("it writes")).expect("it lands");
    assert_eq!(
        named(&read(home.path(), "", NOW)),
        ["0000000a"],
        "the hold on disk wasn't read"
    );
    let dating = list(&[ask("0000000a", Some(NOW + 600))]);
    let m = read(home.path(), &dating, NOW);
    assert!(m.now < 0.5, "a dated hold on disk counted: {:?}", m.behind);
}
