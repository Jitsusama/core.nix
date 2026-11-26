use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::json;

use super::{
    CITES_LAST, DAY, KEEPER, Measure, SAYS, Said, cited_ids, clock_times, gathered, said_in,
    uncited,
};
use crate::mesh::Sent;

const NOW: i64 = 1_795_100_000;

/// The writer's first session's log, as pi names it.
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

/// Who sent each message and what it said, by its short id.
fn sent(by: &[(&str, &str, &str)]) -> BTreeMap<String, Sent> {
    by.iter()
        .map(|(id, from, text)| {
            let message = Sent {
                from: (*from).to_owned(),
                text: (*text).to_owned(),
            };
            ((*id).to_owned(), message)
        })
        .collect()
}

/// `text` said to Joel a minute before now.
fn said(text: &str) -> Vec<Said> {
    vec![Said {
        at: NOW - 60,
        text: text.to_owned(),
    }]
}

/// The measure for what was `said`, the `note` and the `cites`, with
/// WCJQSG writing.
fn judged(said: &[Said], note: &str, cites: &str, by: &BTreeMap<String, Sent>) -> Measure {
    uncited(said, note, cites, by, "WCJQSG").expect("the citations read whole")
}

#[test]
fn a_switch_time_cited_from_the_switches_cell_counts_none() {
    let line = said("Switch 1195 goes live at 12:00, worst 13:00.");
    let cites = cited(&[(
        NOW - 300,
        "16V6F4",
        "aa11bb22",
        "live at 12:00, worst 13:00",
    )]);
    let by = sent(&[("aa11bb22", "16V6F4", "Live at 12:00, at worst 13:00.")]);
    let m = judged(&line, "", &cites, &by);
    assert!(m.behind.is_empty(), "a cited time counted: {:?}", m.behind);
    assert!(m.target.is_some(), "the measure went unjudged");
}

#[test]
fn a_switch_time_backed_by_another_cell_counts_naming_who_runs_the_switches() {
    for line in ["It rides the next switch at 12:00.", "Switching at 12:00."] {
        let cites = cited(&[(NOW - 300, "80ECO2", "aa11bb22", "12:00")]);
        let m = judged(
            &said(line),
            "",
            &cites,
            &sent(&[("aa11bb22", "80ECO2", "by 12:00")]),
        );
        let shown = line.trim_end_matches('.');
        assert_eq!(
            m.behind,
            [format!(
                "{shown}, a time with no word cited from 16V6F4 · WCJQSG"
            )],
            "a switch time not from the switches' cell went uncounted"
        );
    }
}

#[test]
fn a_live_time_counts_unless_another_cell_said_it_in_what_was_cited_before_it() {
    let line = said("core.nix#6609 goes live by 13:15.");
    let by = sent(&[
        ("aa11bb22", "80ECO2", "merged by 13:15"),
        ("cc33dd44", "WCJQSG", "13:15"),
        ("ff00ff00", "80ECO2", "no time here"),
    ]);
    let backed = cited(&[(NOW - 300, "80ECO2", "aa11bb22", "13:15")]);
    assert!(
        judged(&line, "", &backed, &by).behind.is_empty(),
        "a live time another cell gave counted"
    );
    let ago = NOW - 300;
    for (owner, msgid, at, item, why) in [
        ("WCJQSG", "cc33dd44", ago, "13:15", "the writer's word"),
        ("80ECO2", "cc33dd44", ago, "13:15", "another's message"),
        ("80ECO2", "eeee0000", ago, "13:15", "no message found"),
        ("80ECO2", "ff00ff00", ago, "13:15", "words with no time"),
        ("80ECO2", "aa11bb22", NOW, "13:15", "a later citation"),
        ("80ECO2", "aa11bb22", ago, "14:00", "another time"),
        ("", "", NOW, "", "no citation"),
    ] {
        let cites = if owner.is_empty() {
            String::new()
        } else {
            cited(&[(at, owner, msgid, item)])
        };
        let m = judged(&line, "", &cites, &by);
        assert!(
            (m.now - 1.0).abs() < f64::EPSILON,
            "a live time backed by {why} wasn't counted"
        );
        assert_eq!(m.owner, "WCJQSG", "the writer doesn't answer for {why}");
    }
}

#[test]
fn a_clock_time_is_matched_whole_and_each_in_a_line_needs_citing() {
    let line = said("It goes live at 2:00, worst 3:00.");
    let by = sent(&[
        ("aa11bb22", "80ECO2", "live at 12:00"),
        ("cc33dd44", "80ECO2", "live at 2:00"),
        ("ee55ff66", "80ECO2", "at worst 3:00"),
    ]);
    let ago = NOW - 300;
    for (cites, why) in [
        (
            vec![(ago, "80ECO2", "aa11bb22", "12:00")],
            "a citation of 12:00",
        ),
        (
            vec![(ago, "80ECO2", "cc33dd44", "2:00")],
            "the first time alone",
        ),
    ] {
        let m = judged(&line, "", &cited(&cites), &by);
        assert!(
            (m.now - 1.0).abs() < f64::EPSILON,
            "a line backed by {why} wasn't counted"
        );
    }
    let both = cited(&[
        (ago, "80ECO2", "cc33dd44", "2:00"),
        (ago, "80ECO2", "ee55ff66", "3:00"),
    ]);
    assert!(
        judged(&line, "", &both, &by).behind.is_empty(),
        "a line whose every time was cited counted"
    );
}

#[test]
fn a_time_with_no_live_word_or_a_live_word_with_no_time_is_no_timed_line() {
    let line = said("The train lands by 12:45. The fix is live now. A 3:1 vote.");
    let m = judged(&line, "", "", &BTreeMap::new());
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
                **Blocked:** one thing left.\n1. Sign the form.\n2) Pick a name.\nIt waits.\n\
                - A list after.\n- Could you check the lock?\nShould I ship it or hold it?\n\
                I don't need you for anything:\n- All is well.";
    let cites = cited(&[
        (NOW - 300, "FCKPWX", "aa11bb22", "chrome flag"),
        (NOW - 300, "5HJZ8Z", "cc33dd44", "sign the form"),
        (NOW - 300, "5HJZ8Z", "eeee0000", ""),
        (NOW - 300, "5HJZ8Z", "eeee0000", "name"),
    ]);
    let by = sent(&[
        ("aa11bb22", "FCKPWX", "the Chrome flag"),
        ("cc33dd44", "OTHER1", "sign the form"),
        ("eeee0000", "5HJZ8Z", "a name"),
    ]);
    let m = judged(&said(text), "", &cites, &by);
    let asks = [
        "Say which slot suits",
        "Could you log in to the VPN",
        "Sign the form",
        "Pick a name",
        "Could you check the lock",
        "Should I ship it or hold it",
    ];
    let expected: Vec<String> = asks
        .iter()
        .map(|ask| format!("{ask}, an ask citing no owner's word · WCJQSG"))
        .collect();
    assert_eq!(
        m.behind, expected,
        "the asks backed, unbacked or no asks at all were read wrong"
    );
}

#[test]
fn a_line_both_timing_and_asking_counts_once() {
    let line = said("Can you check it goes live at 12:00?");
    let m = judged(&line, "", "", &BTreeMap::new());
    assert_eq!(
        m.behind,
        ["Can you check it goes live at 12:00, a time with no word cited from its owner · WCJQSG"],
        "one sentence was counted as two lines"
    );
}

const NOTE: &str = "## Overnight Into Wednesday\n\n\
    For your reading, a summary.\n\
    - Not an item.\n\n\
    **For you this morning (updated 10:31 am):**\n\n\
    - Pick a slot. <!-- owner: FCKPWX 6c213a39 -->\n\
    - Say yes\n  to the plan. <!-- owner: WCJQSG 11112222 -->\n\
    - Look at it. <!-- owner: FCKPWX 99999999 -->\n\
    1. Read this.\n\
    - Choose one. <!-- owner: 5HJZ8Z 33334444 -->\n  - with a note under it\n\n\
    **Earlier this morning, now settled:**\n\
    - An old item.\n";

#[test]
fn each_item_put_to_joel_names_a_message_its_owner_sent() {
    let by = sent(&[
        ("6c213a39", "FCKPWX", ""),
        ("11112222", "WCJQSG", ""),
        ("99999999", "5HJZ8Z", ""),
        ("33334444", "5HJZ8Z", ""),
    ]);
    let m = judged(&[], NOTE, "", &by);
    assert_eq!(
        m.behind,
        [
            "for you this morning: Say yes to the plan, naming no owner's message · WCJQSG",
            "for you this morning: Look at it, naming no owner's message · WCJQSG",
            "for you this morning: Read this, naming no owner's message · WCJQSG",
        ],
        "the items backed, unbacked, settled or under another heading were read wrong"
    );
}

#[test]
fn an_indented_list_is_read_item_by_item_with_its_points_carrying_each_on() {
    let note = "**For you:**\n  - Pick one.\n    - the first\n    - the second\n  - Read it.\n";
    let m = judged(&[], note, "", &BTreeMap::new());
    assert_eq!(
        m.behind,
        [
            "for you this morning: Pick one. - the first - the second, naming no owner's message \
             · WCJQSG",
            "for you this morning: Read it, naming no owner's message · WCJQSG",
        ],
        "an indented list was read as none, or its points as items of their own"
    );
}

#[test]
fn words_leading_into_the_list_are_passed_over_and_words_with_no_list_are_unread() {
    let note = "**For you:**\nTwo calls today:\n- Pick one.\n- Read it.\n**Later:**\n- Not one.\n";
    let m = judged(&[], note, "", &BTreeMap::new());
    assert_eq!(
        m.behind,
        [
            "for you this morning: Pick one, naming no owner's message · WCJQSG",
            "for you this morning: Read it, naming no owner's message · WCJQSG",
        ],
        "words leading into the list hid it, or the next part was read into it"
    );
    let said = uncited(
        &[],
        "**For you:**\nNothing urgent, see below.\n",
        "",
        &BTreeMap::new(),
        "WCJQSG",
    );
    assert_eq!(
        said.expect_err("words with no list were read as nothing put to him"),
        "the morning note's \"For you\" holds words but no list",
        "words with no list weren't said unread, with why"
    );
    let empty = judged(
        &[],
        "**For you:**\n\n## Overnight\n- Not one.\n",
        "",
        &BTreeMap::new(),
    );
    assert!(
        empty.behind.is_empty(),
        "an empty part read the next part's list as his: {empty:?}"
    );
}

#[test]
fn only_the_messages_cited_are_asked_of_the_mesh() {
    let cites = cited(&[(NOW, "80ECO2", "aa11bb22", "12:00")]);
    assert_eq!(
        cited_ids(&cites, NOTE)
            .expect("the citations read whole")
            .into_iter()
            .collect::<Vec<_>>(),
        ["11112222", "33334444", "6c213a39", "99999999", "aa11bb22"],
        "a message cited was missed, or one not cited asked for"
    );
}

/// One assistant entry of a session log, saying `text` at `at`.
fn entry(at: i64, text: &str) -> String {
    json!({"type": "message", "timestamp": iso(at), "message": {"role": "assistant",
        "content": [{"type": "text", "text": text}]}})
    .to_string()
}

/// The mesh folder of the machine at `root`.
fn mesh_of(root: &Path) -> PathBuf {
    root.join("state/pi/agentic-harness.pi/mesh-workflow")
}

/// A machine at `root` whose mesh has the writer's presence and the
/// message `aa11bb22` from 16V6F4 sent at `sent_at`, and whose pi logs,
/// under `logs`, hold the writer saying a switch time a minute ago and
/// another over a day ago.
fn machine(root: &Path, sent_at: i64) {
    let mesh = mesh_of(root);
    fs::create_dir_all(mesh.join("messages")).expect("the messages folder is made");
    fs::create_dir_all(mesh.join("presence")).expect("the presence folder is made");
    let presence = json!({"session": "s1", "addresses": ["WCJQSG"], "state": "busy", "at": "1"});
    fs::write(mesh.join("presence/s1.json"), presence.to_string()).expect("presence is written");
    let id = format!("{}-16V6F4-1-aa11bb22", sent_at * 1000);
    let message = json!({"v": 1, "id": id, "at": iso(sent_at), "from": "16V6F4",
        "to": ["@WCJQSG"], "intent": "fyi", "text": "live at 12:00", "pid": 1});
    let written = mesh.join(format!("messages/{id}.json"));
    fs::write(written, message.to_string()).expect("the message is written");
    let logs = root.join("logs/--work--");
    fs::create_dir_all(&logs).expect("the logs folder is made");
    let log = [
        entry(NOW - DAY - 60, "Switch 1194 is live at 10:06."),
        json!({"type": "message", "timestamp": iso(NOW - 90),
            "message": {"role": "user", "content": "hi"}})
        .to_string(),
        entry(NOW - 60, "Switch 1195 goes live at 12:00."),
    ];
    fs::write(logs.join(LOG), log.join("\n")).expect("the log is written");
}

/// Writes `text` as `name` in the mission folder of the machine at `root`.
fn write_mission(root: &Path, name: &str, text: &str) {
    let mission = root.join("state/mission");
    fs::create_dir_all(&mission).expect("the mission folder is made");
    fs::write(mission.join(name), text).expect("the mission file is written");
}

/// Cites 16V6F4's message `aa11bb22` for 12:00 on the machine at `root`.
fn cite_the_switch(root: &Path) {
    let cites = cited(&[(NOW - 300, "16V6F4", "aa11bb22", "12:00")]);
    write_mission(root, "cited.jsonl", &cites);
}

fn read_at(root: &Path) -> Result<Measure, String> {
    gathered(
        &root.join("state"),
        &root.join("logs"),
        NOW,
        "2026-11-25",
        "WCJQSG",
    )
}

fn read_machine(root: &Path) -> Measure {
    read_at(root).unwrap_or_else(|why| panic!("a whole machine read unread: {why}"))
}

#[test]
fn the_hour_reads_the_writer_s_last_day_and_the_cell_it_cites() {
    let root = tempfile::tempdir().expect("a scratch folder");
    machine(root.path(), NOW - 600);
    assert_eq!(
        read_machine(root.path()).behind,
        ["Switch 1195 goes live at 12:00, a time with no word cited from 16V6F4 · WCJQSG"],
        "with no citations written, only the last day's line counted"
    );
    cite_the_switch(root.path());
    assert!(
        read_machine(root.path()).behind.is_empty(),
        "a line cited from the switches' cell counted"
    );
}

#[test]
fn the_morning_note_is_read_from_the_mission_folder() {
    let root = tempfile::tempdir().expect("a scratch folder");
    machine(root.path(), NOW - 600);
    cite_the_switch(root.path());
    let note = "**For you:**\n- Try it. <!-- owner: 16V6F4 aa11bb22 -->\n- And this.\n";
    write_mission(root.path(), "morning-2026-11-25.md", note);
    assert_eq!(
        read_machine(root.path()).behind,
        ["for you this morning: And this, naming no owner's message · WCJQSG"],
        "today's note wasn't read, or its cited item counted"
    );
}

#[test]
fn every_session_the_writer_went_by_is_read() {
    let root = tempfile::tempdir().expect("a scratch folder");
    machine(root.path(), NOW - 600);
    let presence = json!({"session": "s2", "addresses": ["WCJQSG"], "state": "busy", "at": "2"});
    let mesh = mesh_of(root.path());
    fs::write(mesh.join("presence/s2.json"), presence.to_string()).expect("presence is written");
    let logs = root.path().join("logs/--elsewhere--");
    fs::create_dir_all(&logs).expect("the logs folder is made");
    let log = entry(NOW - 30, "It goes live at 14:00.");
    fs::write(logs.join("2026-11-25T11-00-00-000Z_s2.jsonl"), log).expect("the log is written");
    assert!(
        (read_machine(root.path()).now - 2.0).abs() < f64::EPSILON,
        "a line from the session before the writer's restart was left out"
    );
}

#[test]
fn a_long_log_is_read_from_where_the_day_starts() {
    let root = tempfile::tempdir().expect("a scratch folder");
    let log = root.path().join(LOG);
    let mut lines: Vec<String> = (0..4320)
        .map(|n| entry(NOW - 3 * DAY + n * 60, "Nothing to say here at all."))
        .collect();
    lines.insert(10, "{\"assistant\": torn".to_owned());
    fs::write(&log, lines.join("\n")).expect("the log is written");
    let said = said_in(&log, NOW - DAY, NOW).expect("the day after the torn line reads whole");
    assert_eq!(said.len(), 1439, "the day's entries weren't each read once");
    assert!(
        said.iter().all(|s| s.at > NOW - DAY),
        "an entry from before the day was read"
    );
}

#[test]
fn a_torn_line_amid_the_day_leaves_the_log_unread_but_one_still_being_written_doesn_t() {
    let root = tempfile::tempdir().expect("a scratch folder");
    let log = root.path().join(LOG);
    let amid = [
        entry(NOW - 90, "First."),
        "{\"assistant\": torn".to_owned(),
        entry(NOW - 30, "Last."),
    ];
    fs::write(&log, amid.join("\n")).expect("the log is written");
    assert!(
        said_in(&log, NOW - DAY, NOW).is_err(),
        "a torn line amid the day read as whole"
    );
    let untimed = [
        entry(NOW - 90, "First."),
        "{\"assistant\": torn".to_owned(),
        "not a line of the log's at all".to_owned(),
    ];
    fs::write(&log, untimed.join("\n")).expect("the log is written");
    assert!(
        said_in(&log, NOW - DAY, NOW).is_err(),
        "a torn line amid the day, with only lines of no time after it, read as whole"
    );
    let last = [entry(NOW - 90, "First."), "{\"assistant\": to".to_owned()];
    fs::write(&log, last.join("\n")).expect("the log is written");
    assert_eq!(
        said_in(&log, NOW - DAY, NOW).ok().map(|said| said.len()),
        Some(1),
        "a last line still being written left the log unread"
    );
}

#[test]
fn a_torn_line_from_before_the_day_is_passed_over_once_a_later_line_dates_it() {
    let root = tempfile::tempdir().expect("a scratch folder");
    let log = root.path().join(LOG);
    let resumed = [
        entry(NOW - DAY - 120, "Before the crash."),
        "{\"assistant\": torn".to_owned(),
        "not a line of the log's at all".to_owned(),
        entry(NOW - DAY - 60, "Resumed."),
        entry(NOW - 30, "Today."),
    ];
    fs::write(&log, resumed.join("\n")).expect("the log is written");
    let said = said_in(&log, NOW - DAY, NOW).map(|said| said.len());
    assert_eq!(
        said.ok(),
        Some(1),
        "a torn line written before the day left the log unread for good"
    );
}

#[test]
fn a_message_sent_before_citations_reach_backs_nothing() {
    let root = tempfile::tempdir().expect("a scratch folder");
    machine(root.path(), NOW - CITES_LAST - 60);
    cite_the_switch(root.path());
    assert!(
        (read_machine(root.path()).now - 1.0).abs() < f64::EPSILON,
        "a message too old to look for read as backing the line"
    );
}

#[test]
fn a_message_whose_name_and_body_name_different_senders_backs_nothing() {
    let root = tempfile::tempdir().expect("a scratch folder");
    machine(root.path(), NOW - 600);
    let id = format!("{}-16V6F4-1-aa11bb22", (NOW - 600) * 1000);
    let forged = json!({"v": 1, "id": id, "at": iso(NOW - 600), "from": "WCJQSG",
        "to": ["@WCJQSG"], "intent": "fyi", "text": "live at 12:00", "pid": 1});
    let written = mesh_of(root.path()).join(format!("messages/{id}.json"));
    fs::write(written, forged.to_string()).expect("the message is written");
    cite_the_switch(root.path());
    assert!(
        (read_machine(root.path()).now - 1.0).abs() < f64::EPSILON,
        "a message named for 16V6F4 but sent by the writer backed its line"
    );
}

#[test]
fn what_can_t_be_read_leaves_the_measure_unjudged_saying_why() {
    let root = tempfile::tempdir().expect("a scratch folder");
    let unread = |root: &Path| read_at(root).err().unwrap_or_default();
    assert_eq!(
        unread(root.path()),
        "no mesh messages folder here",
        "a machine with no mesh wasn't said unread"
    );
    machine(root.path(), NOW - 600);
    let mission = root.path().join("state/mission");
    let good = cited(&[(NOW - 300, "16V6F4", "aa11bb22", "12:00")]);
    let bad = "{\"ts\": \"not a time\"}";
    for (text, line) in [
        (format!("{bad}\n{good}\n"), 1),
        (format!("{good}\n\n{bad}\n"), 3),
    ] {
        write_mission(root.path(), "cited.jsonl", &text);
        let m =
            read_at(root.path()).unwrap_or_else(|why| panic!("a bad citation read unread: {why}"));
        assert!(
            m.says.ends_with(&format!(
                "(1 line of cited.jsonl can't be read, backing nothing; the first, line {line})"
            )),
            "a citation that can't be read went unsaid, or not by its line: {}",
            m.says
        );
        assert!(
            m.now.abs() < f64::EPSILON,
            "a citation beside one that can't be read no longer backed its line: {:?}",
            m.behind
        );
    }
    write_mission(
        root.path(),
        "morning-2026-11-25.md",
        "## Today\n- An item.\n",
    );
    assert_eq!(
        unread(root.path()),
        "the morning note has no line opening \"For you\"",
        "a morning note in another shape was read as putting nothing to Joel"
    );
    fs::remove_file(mission.join("morning-2026-11-25.md")).expect("the note is removed");
    fs::remove_file(mission.join("cited.jsonl")).expect("the citations are removed");
    fs::create_dir(mission.join("cited.jsonl")).expect("a folder stands in for the citations");
    assert!(
        unread(root.path()).contains("cited.jsonl can't be read: "),
        "citations there but unreadable weren't said unread"
    );
    fs::remove_dir(mission.join("cited.jsonl")).expect("the folder is removed");
    cite_the_switch(root.path());
    let id = format!("{}-16V6F4-1-aa11bb22", (NOW - 600) * 1000);
    let message = mesh_of(root.path()).join(format!("messages/{id}.json"));
    fs::write(&message, "{\"v\": 1, torn").expect("the message is torn");
    assert_eq!(
        unread(root.path()),
        "the message aa11bb22 cited can't be read",
        "a cited message that can't be read wasn't said unread"
    );
    fs::remove_file(message).expect("the message is removed");
    let log = root.path().join("logs/--work--").join(LOG);
    let timeless = json!({"type": "message", "message": {"role": "assistant", "content": []}});
    let lines = [timeless.to_string(), entry(NOW - 60, "Done.")];
    fs::write(&log, lines.join("\n")).expect("the log is written");
    assert!(
        unread(root.path()).ends_with("can't be read: an assistant entry has no time to read"),
        "an assistant entry with no time wasn't said unread"
    );
    fs::remove_file(log).expect("the log is removed");
    assert_eq!(
        unread(root.path()),
        "no log of WCJQSG's session s1",
        "a writer with no log wasn't said unread"
    );
    fs::remove_file(mesh_of(root.path()).join("presence/s1.json")).expect("presence is removed");
    assert_eq!(
        unread(root.path()),
        "no session here goes by WCJQSG",
        "a writer with no session wasn't said unread"
    );
    let measure = super::read(&root.path().join("state"), NOW, "2026-11-25", None);
    assert_eq!(measure.target, None, "an unread hour was judged");
    assert_eq!(measure.owner, KEEPER, "another answers for an unread hour");
    assert!(
        measure.says.starts_with(SAYS) && measure.says.contains("(unread: "),
        "an unread hour didn't say why: {}",
        measure.says
    );
}
