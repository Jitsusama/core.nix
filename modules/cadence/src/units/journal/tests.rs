use std::cell::RefCell;

use super::*;

const NOW: i64 = 1_795_810_200;

/// A line of journalctl's JSON: `cursor`, written `at` seconds, by `unit`,
/// saying `message`, with fields none of the readers keep.
fn entry(cursor: &str, at: i64, unit: &str, message: &str) -> String {
    serde_json::json!({
        "__CURSOR": cursor,
        "__REALTIME_TIMESTAMP": (at * 1_000_000 + 250).to_string(),
        "_SYSTEMD_USER_UNIT": unit,
        "_HOSTNAME": "host",
        "SYSLOG_IDENTIFIER": "said",
        "_PID": "7",
        "MESSAGE": message,
        "_CMDLINE": "a field no reader reads",
    })
    .to_string()
        + "\n"
}

fn follows(fields: &Map<String, Value>) -> bool {
    is(fields, "_SYSTEMD_USER_UNIT", "follows.service")
}

fn wants() -> [Want; 1] {
    [Want {
        name: "follows",
        journal: Journal::User,
        keep: 3600,
        fill: &["-u", "follows"],
        takes: follows,
    }]
}

/// A stand-in journalctl answering `said` and writing down each ask.
struct Stand {
    asked: RefCell<Vec<Vec<String>>>,
    said: String,
}

impl Stand {
    fn new(said: &str) -> Self {
        Self {
            asked: RefCell::new(Vec::new()),
            said: said.to_owned(),
        }
    }

    fn run(&self) -> impl Fn(&[String]) -> Result<String, String> + '_ {
        |args: &[String]| {
            self.asked.borrow_mut().push(args.to_vec());
            Ok(self.said.clone())
        }
    }
}

fn matched(args: &[String]) -> bool {
    args.iter()
        .any(|a| a == "-u" || (a.contains('=') && !a.starts_with("--")))
}

#[test]
fn a_first_read_fills_by_its_match_and_later_ones_read_forward_unmatched() {
    let folder = tempfile::tempdir().expect("a folder for the kept lines");
    let first = Stand::new(&entry(
        "a",
        NOW - 600,
        "follows.service",
        "asking at core@ab",
    ));
    let read = forward(folder.path(), &wants(), NOW, first.run());
    assert_eq!(
        read["follows"].len(),
        1,
        "the fill's line wasn't kept: {read:?}"
    );
    let asked = first.asked.borrow();
    assert_eq!(
        asked.len(),
        1,
        "a first read asked more than its fill: {asked:?}"
    );
    assert!(
        asked[0].iter().any(|a| a == "-u") && asked[0].iter().any(|a| a == "--since=@1795806600"),
        "the fill didn't read its own match over the hour it keeps: {asked:?}"
    );

    let later = Stand::new(
        &(entry("a", NOW - 600, "follows.service", "asking at core@ab")
            + &entry("b", NOW + 200, "other.service", "not the follower's")
            + &entry("c", NOW + 250, "follows.service", "{\"step\":\"done\"}")),
    );
    let read = forward(folder.path(), &wants(), NOW + 300, later.run());
    let asked = later.asked.borrow();
    assert_eq!(
        asked.len(),
        1,
        "a tick read the user journal more than once: {asked:?}"
    );
    assert!(
        !matched(&asked[0]),
        "a tick after the first read the journal by a match, which walks every file: {asked:?}"
    );
    assert!(
        asked[0]
            .iter()
            .any(|a| a == &format!("--since=@{}", NOW - OVERLAP)),
        "the tick didn't read on from where the last one stopped: {asked:?}"
    );
    let cursors: Vec<&str> = read["follows"].iter().map(|l| l.cursor.as_str()).collect();
    assert_eq!(
        cursors,
        ["a", "c"],
        "a line read twice was kept twice, or another unit's was kept"
    );
}

#[test]
fn a_line_older_than_its_reader_looks_back_is_dropped() {
    let folder = tempfile::tempdir().expect("a folder for the kept lines");
    let first = Stand::new(&entry("a", NOW - 3000, "follows.service", "old"));
    forward(folder.path(), &wants(), NOW, first.run());
    let later = Stand::new(&entry("b", NOW + 900, "follows.service", "new"));
    let read = forward(folder.path(), &wants(), NOW + 1000, later.run());
    let cursors: Vec<&str> = read["follows"].iter().map(|l| l.cursor.as_str()).collect();
    assert_eq!(
        cursors,
        ["b"],
        "a line past the hour its reader keeps was kept"
    );
}

#[test]
fn a_read_that_fails_moves_nothing_so_the_next_reads_from_the_same_place() {
    let folder = tempfile::tempdir().expect("a folder for the kept lines");
    let first = Stand::new(&entry("a", NOW - 60, "follows.service", "kept"));
    forward(folder.path(), &wants(), NOW, first.run());
    let read = forward(folder.path(), &wants(), NOW + 300, |_: &[String]| {
        Err("journalctl failed".to_owned())
    });
    assert_eq!(
        read["follows"].len(),
        1,
        "a failed read dropped what was kept"
    );
    let again = Stand::new("");
    forward(folder.path(), &wants(), NOW + 600, again.run());
    let asked = again.asked.borrow();
    assert!(
        asked[0]
            .iter()
            .any(|a| a == &format!("--since=@{}", NOW - OVERLAP)),
        "after a failed read, the next didn't start from the last that worked: {asked:?}"
    );
}

#[test]
fn a_reader_not_read_for_longer_than_it_looks_back_is_filled_again() {
    let folder = tempfile::tempdir().expect("a folder for the kept lines");
    forward(folder.path(), &wants(), NOW, Stand::new("").run());
    let later = Stand::new("");
    forward(folder.path(), &wants(), NOW + 4000, later.run());
    let asked = later.asked.borrow();
    assert!(
        asked.len() == 1 && matched(&asked[0]),
        "a reader idle past its hour read everything since, not its own fill: {asked:?}"
    );
}

#[test]
fn a_kept_line_reads_as_journalctl_writes_it() {
    let folder = tempfile::tempdir().expect("a folder for the kept lines");
    let said = entry("a", NOW, "follows.service", "asking at core@ab");
    let read = forward(folder.path(), &wants(), NOW, Stand::new(&said).run());
    assert_eq!(
        short_unix(&read["follows"], None),
        "1795810200.000250 host said[7]: asking at core@ab\n",
        "a kept line isn't written as short-unix writes it"
    );
    let fields: Map<String, Value> =
        serde_json::from_str(json(&read["follows"]).trim()).expect("one JSON object");
    assert!(
        fields.get("_CMDLINE").is_none() && is(&fields, "MESSAGE", "asking at core@ab"),
        "kept fields no reader reads, or lost the message: {fields:?}"
    );
}

#[test]
fn each_reader_fills_by_the_lines_it_takes() {
    let restarts = super::super::RESTARTS;
    assert_eq!(
        restarts.fill,
        [
            format!("MESSAGE_ID={}", super::super::RESTART_SCHEDULED),
            format!("MESSAGE_ID={}", super::super::PROCESS_EXITED),
        ],
        "the restarts' fill matches other lines than the reader takes"
    );
    let kill = |transport: &str, message: &str| -> Map<String, Value> {
        serde_json::from_value(serde_json::json!({"_TRANSPORT": transport, "MESSAGE": message}))
            .expect("fields")
    };
    assert!(
        (KERNEL.takes)(&kill("kernel", "oom-kill:constraint=CONSTRAINT_NONE")),
        "the kernel's reader left out a kill"
    );
    assert!(
        !(KERNEL.takes)(&kill("stdout", "oom-kill: said by a program")),
        "the kernel's reader took a program's line naming a kill"
    );
}

#[test]
fn a_message_journalctl_writes_as_bytes_reads_as_text_and_other_boots_can_be_left_out() {
    let fields: Map<String, Value> = serde_json::from_value(serde_json::json!({
        "MESSAGE": [111, 111, 109, 45, 107, 105, 108, 108, 58, 255],
        "_BOOT_ID": "b1",
    }))
    .expect("fields");
    let line = Line {
        cursor: "a".to_owned(),
        micros: 5_000_001,
        fields,
    };
    let written = short_unix(std::slice::from_ref(&line), None);
    assert!(
        written.contains("oom-kill:\u{fffd}"),
        "a message given as bytes wasn't read as text: {written}"
    );
    assert_eq!(
        short_unix(&[line], Some("b2")),
        "",
        "a line of another boot was kept where only this boot's are asked for"
    );
}
