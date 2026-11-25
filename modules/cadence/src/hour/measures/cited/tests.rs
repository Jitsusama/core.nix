use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde_json::json;

use super::{CITES_LAST, KEEPER, SAYS, Said, cited_ids, clock_times, gathered, uncited};

const NOW: i64 = 1_795_100_000;

/// The writer's session log, as pi names it.
const LOG: &str = "2026-11-25T10-00-00-000Z_s1.jsonl";

/// `at` seconds as the citations and the logs write a time.
fn iso(at: i64) -> String {
    jiff::Timestamp::from_second(at)
        .expect("a time in range")
        .to_string()
}

/// A citations file of one line for each of `cites`: written at, owner,
/// message and words.
fn cited(cites: &[(i64, &str, &str, &str)]) -> String {
    cites
        .iter()
        .map(|(at, owner, msgid, item)| {
            json!({"ts": iso(*at), "owner": owner, "msgid": msgid, "item": item}).to_string()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Who sent each message, by its short id.
fn sent(by: &[(&str, &str)]) -> BTreeMap<String, String> {
    by.iter()
        .map(|(id, from)| ((*id).to_owned(), (*from).to_owned()))
        .collect()
}

/// `text` said to Joel a minute before now.
fn said(text: &str) -> Vec<Said> {
    vec![Said {
        at: NOW - 60,
        text: text.to_owned(),
    }]
}

#[test]
fn a_switch_time_cited_from_the_switches_cell_counts_none() {
    let line = said("Switch 1195 goes live at 12:00, worst 13:00.");
    let cites = cited(&[(NOW - 300, "16V6F4", "aa11bb22", "live at 12:00")]);
    let m = uncited(&line, "", &cites, &sent(&[("aa11bb22", "16V6F4")]));
    assert!(m.behind.is_empty(), "a cited time counted: {:?}", m.behind);
    assert!(m.target.is_some(), "the measure went unjudged");
}

#[test]
fn a_switch_time_backed_by_another_cell_counts_naming_who_runs_the_switches() {
    let line = said("It rides the next switch at 12:00.");
    let cites = cited(&[(NOW - 300, "80ECO2", "aa11bb22", "12:00")]);
    let m = uncited(&line, "", &cites, &sent(&[("aa11bb22", "80ECO2")]));
    assert_eq!(
        m.behind,
        ["It rides the next switch at 12:00, a time with no word cited from 16V6F4 · WCJQSG"],
        "a switch time not from the switches' cell went uncounted"
    );
}

#[test]
fn a_live_time_counts_unless_its_owner_sent_what_was_cited_before_it() {
    let line = said("core.nix#6609 goes live by 13:15.");
    let by = sent(&[("aa11bb22", "80ECO2"), ("cc33dd44", "WCJQSG")]);
    let backed = cited(&[(NOW - 300, "80ECO2", "aa11bb22", "13:15")]);
    assert!(
        uncited(&line, "", &backed, &by).behind.is_empty(),
        "a live time its owner gave counted"
    );
    let ago = NOW - 300;
    for (owner, msgid, at, item, why) in [
        ("WCJQSG", "cc33dd44", ago, "13:15", "the writer's word"),
        ("80ECO2", "cc33dd44", ago, "13:15", "another's message"),
        ("80ECO2", "eeee0000", ago, "13:15", "no message found"),
        ("80ECO2", "aa11bb22", NOW, "13:15", "a later citation"),
        ("80ECO2", "aa11bb22", ago, "14:00", "another time"),
        ("", "", NOW, "", "no citation"),
    ] {
        let cites = if owner.is_empty() {
            String::new()
        } else {
            cited(&[(at, owner, msgid, item)])
        };
        let m = uncited(&line, "", &cites, &by);
        assert!(
            (m.now - 1.0).abs() < f64::EPSILON,
            "a live time backed by {why} wasn't counted"
        );
        assert_eq!(m.owner, "WCJQSG", "the writer doesn't answer for {why}");
    }
}

#[test]
fn a_time_with_no_live_word_or_a_live_word_with_no_time_is_no_timed_line() {
    let line = said("The train lands by 12:45. The fix is live now. A 3:1 vote.");
    let m = uncited(&line, "", "", &BTreeMap::new());
    assert!(m.behind.is_empty(), "a line counted: {:?}", m.behind);
}

#[test]
fn a_clock_time_is_its_hour_and_minute_alone() {
    assert_eq!(
        clock_times("at 10:06:44, then 9:05, not 123:45, 12:345 or 1:5"),
        ["10:06", "9:05"],
        "a clock time read wrong"
    );
}

#[test]
fn each_ask_of_joel_counts_unless_a_citation_s_words_are_all_in_it() {
    let text = "These need you:\n- Approve the Chrome flag.\n- Say which slot suits.\n\n\
                Other words here. Could you log in to the VPN?\n\
                Blocked: one thing left.\n- Sign the form.\nIt waits.\n- A list after.";
    let cites = cited(&[
        (NOW - 300, "FCKPWX", "aa11bb22", "chrome flag"),
        (NOW - 300, "5HJZ8Z", "cc33dd44", "sign the form"),
        (NOW - 300, "5HJZ8Z", "eeee0000", ""),
    ]);
    let by = sent(&[
        ("aa11bb22", "FCKPWX"),
        ("cc33dd44", "OTHER1"),
        ("eeee0000", "5HJZ8Z"),
    ]);
    let m = uncited(&said(text), "", &cites, &by);
    assert_eq!(
        m.behind,
        [
            "Say which slot suits, an ask citing no owner's word · WCJQSG",
            "Could you log in to the VPN, an ask citing no owner's word · WCJQSG",
            "Sign the form, an ask citing no owner's word · WCJQSG",
        ],
        "the asks backed, unbacked or no asks at all were read wrong"
    );
}

const NOTE: &str = "## Overnight Into Wednesday\n\n\
    **For you this morning (updated 10:31 am):**\n\n\
    - Pick a slot. <!-- owner: FCKPWX 6c213a39 -->\n\
    - Say yes\n  to the plan. <!-- owner: WCJQSG 11112222 -->\n\
    - Look at it. <!-- owner: FCKPWX 99999999 -->\n\
    - Read this.\n\n\
    **Earlier this morning, now settled:**\n\
    - An old item.\n";

#[test]
fn each_item_put_to_joel_names_a_message_its_owner_sent() {
    let by = sent(&[
        ("6c213a39", "FCKPWX"),
        ("11112222", "WCJQSG"),
        ("99999999", "5HJZ8Z"),
    ]);
    let m = uncited(&[], NOTE, "", &by);
    assert_eq!(
        m.behind,
        [
            "for you this morning: Say yes to the plan, naming no owner's message · WCJQSG",
            "for you this morning: Look at it, naming no owner's message · WCJQSG",
            "for you this morning: Read this, naming no owner's message · WCJQSG",
        ],
        "the items backed, unbacked or settled were read wrong"
    );
}

#[test]
fn only_the_messages_cited_are_asked_of_the_mesh() {
    let cites = cited(&[(NOW, "80ECO2", "aa11bb22", "12:00")]);
    assert_eq!(
        cited_ids(&cites, NOTE).into_iter().collect::<Vec<_>>(),
        ["11112222", "6c213a39", "99999999", "aa11bb22"],
        "a message cited was missed, or one not cited asked for"
    );
}

/// A machine at `root` whose mesh has the writer's presence and the
/// message `aa11bb22` from 16V6F4 sent at `sent_at`, and whose pi logs,
/// under `logs`, hold the writer saying a switch time a minute ago and
/// another two hours ago.
fn machine(root: &Path, sent_at: i64) {
    let mesh = root.join("state/pi/agentic-harness.pi/mesh-workflow");
    fs::create_dir_all(mesh.join("messages")).expect("the messages folder is made");
    fs::create_dir_all(mesh.join("presence")).expect("the presence folder is made");
    let presence = json!({"session": "s1", "addresses": ["WCJQSG"], "state": "busy", "at": "x"});
    fs::write(mesh.join("presence/s1.json"), presence.to_string()).expect("presence is written");
    let id = format!("{}-16V6F4-1-aa11bb22", sent_at * 1000);
    let message = json!({"v": 1, "id": id, "at": iso(sent_at), "from": "16V6F4",
        "to": ["@WCJQSG"], "intent": "fyi", "text": "live at 12:00", "pid": 1});
    let written = mesh.join(format!("messages/{id}.json"));
    fs::write(written, message.to_string()).expect("the message is written");
    let logs = root.join("logs/--work--");
    fs::create_dir_all(&logs).expect("the logs folder is made");
    let entry = |at: i64, text: &str| {
        json!({"type": "message", "timestamp": iso(at), "message": {"role": "assistant",
            "content": [{"type": "text", "text": text}]}})
        .to_string()
    };
    let log = [
        entry(NOW - 7200, "Switch 1194 is live at 10:06."),
        json!({"type": "message", "message": {"role": "user", "content": "hi"}}).to_string(),
        entry(NOW - 60, "Switch 1195 goes live at 12:00."),
    ];
    fs::write(logs.join(LOG), log.join("\n")).expect("the log is written");
}

fn read_machine(root: &Path) -> super::Measure {
    let measure = gathered(&root.join("state"), &root.join("logs"), NOW, "2026-11-25");
    measure.unwrap_or_else(|why| panic!("a whole machine read unread: {why}"))
}

#[test]
fn the_hour_reads_the_writer_s_last_hour_and_the_cell_it_cites() {
    let root = tempfile::tempdir().expect("a scratch folder");
    machine(root.path(), NOW - 600);
    assert_eq!(
        read_machine(root.path()).behind,
        ["Switch 1195 goes live at 12:00, a time with no word cited from 16V6F4 · WCJQSG"],
        "with no citations written, only the last hour's line counted"
    );
    let cites = cited(&[(NOW - 300, "16V6F4", "aa11bb22", "12:00")]);
    let mission = root.path().join("state/mission");
    fs::create_dir_all(&mission).expect("the mission folder is made");
    fs::write(mission.join("cited.jsonl"), cites).expect("the citations are written");
    assert!(
        read_machine(root.path()).behind.is_empty(),
        "a line cited from the switches' cell counted"
    );
}

#[test]
fn a_message_sent_before_citations_reach_backs_nothing() {
    let root = tempfile::tempdir().expect("a scratch folder");
    machine(root.path(), NOW - CITES_LAST - 60);
    let mission = root.path().join("state/mission");
    fs::create_dir_all(&mission).expect("the mission folder is made");
    let cites = cited(&[(NOW - 300, "16V6F4", "aa11bb22", "12:00")]);
    fs::write(mission.join("cited.jsonl"), cites).expect("the citations are written");
    assert!(
        (read_machine(root.path()).now - 1.0).abs() < f64::EPSILON,
        "a message too old to look for read as backing the line"
    );
}

#[test]
fn what_can_t_be_read_leaves_the_measure_unjudged_saying_why() {
    let root = tempfile::tempdir().expect("a scratch folder");
    let read = |root: &Path| gathered(&root.join("state"), &root.join("logs"), NOW, "2026-11-25");
    assert_eq!(
        read(root.path()).err().as_deref(),
        Some("no mesh messages folder here"),
        "a machine with no mesh wasn't said unread"
    );
    machine(root.path(), NOW - 600);
    let log = root.path().join("logs/--work--").join(LOG);
    fs::remove_file(log).expect("the log is removed");
    assert_eq!(
        read(root.path()).err().as_deref(),
        Some("no log of WCJQSG's session s1"),
        "a writer with no log wasn't said unread"
    );
    let presence = root
        .path()
        .join("state/pi/agentic-harness.pi/mesh-workflow/presence/s1.json");
    fs::remove_file(presence).expect("the presence is removed");
    assert_eq!(
        read(root.path()).err().as_deref(),
        Some("no session here goes by WCJQSG"),
        "a writer with no session wasn't said unread"
    );
    let measure = super::read(&root.path().join("state"), NOW, "2026-11-25");
    assert_eq!(measure.target, None, "an unread hour was judged");
    assert_eq!(measure.owner, KEEPER, "another answers for an unread hour");
    assert!(
        measure.says.starts_with(SAYS) && measure.says.contains("(unread: "),
        "an unread hour didn't say why: {}",
        measure.says
    );
}
