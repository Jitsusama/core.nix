//! The day's histories and followers on the journalctl the machine ships,
//! over a journal systemd's own journal-remote writes, rather than on
//! stand-ins that only say what journalctl is thought to do.

use std::fs;
use std::sync::mpsc;

use serde_json::Value;

use super::super::*;
use crate::{asks, launcher, launches};

/// The boot before a reboot, and the boot after.
const BOOTS: [&str; 2] = [
    "11111111111111111111111111111111",
    "22222222222222222222222222222222",
];

/// A line of the day: its boot, when it says it was written, in seconds
/// since 1970, and its fields.
struct Line {
    boot: &'static str,
    at: i64,
    fields: Vec<String>,
}

/// A journal journal-remote wrote from the day's lines, in the order
/// journald would have written them.
struct Journal {
    folder: tempfile::TempDir,
}

impl Journal {
    fn of(lines: &[Line]) -> Self {
        let remote = std::env::var("STAND_IN_JOURNAL_REMOTE")
            .expect("STAND_IN_JOURNAL_REMOTE names systemd-journal-remote, as package.nix does");
        let folder = tempfile::tempdir().expect("a folder for the journal");
        let mut export = String::new();
        for (written, line) in lines.iter().enumerate() {
            export.push_str(&format!(
                "__REALTIME_TIMESTAMP={}\n__MONOTONIC_TIMESTAMP={}\n_BOOT_ID={}\n",
                line.at * 1_000_000,
                written + 1,
                line.boot
            ));
            for field in &line.fields {
                export.push_str(field);
                export.push('\n');
            }
            export.push('\n');
        }
        let exported = folder.path().join("day.export");
        fs::write(&exported, export).expect("the day is exported");
        let written = Command::new(remote)
            .arg(format!(
                "--output={}",
                folder.path().join("day.journal").display()
            ))
            .arg("--split-mode=none")
            .arg(&exported)
            .output()
            .expect("journal-remote starts");
        assert!(
            written.status.success(),
            "journal-remote refused the day: {}",
            String::from_utf8_lossy(&written.stderr)
        );
        fs::remove_file(&exported).expect("the export is removed");
        Self { folder }
    }

    /// `command` reading this journal rather than the machine's, and
    /// ending at its end rather than following on.
    fn reading(&self, command: &Command) -> Command {
        let mut reading = Command::new(command.get_program());
        reading
            .args(command.get_args().filter(|arg| *arg != "--follow"))
            .arg(format!("--directory={}", self.folder.path().display()));
        reading
    }

    /// What `command` gives from this journal: each line's message and
    /// cursor.
    fn read(&self, command: &Command) -> Vec<(String, String)> {
        History::of(self.reading(command))
            .expect("journalctl starts")
            .map(|line| {
                let line: Value = serde_json::from_str(&line).expect("journalctl writes JSON");
                let field = |name: &str| line[name].as_str().unwrap_or_default().to_owned();
                (field("MESSAGE"), field("__CURSOR"))
            })
            .collect()
    }
}

/// A line each of `journal::followed()` asks for, in the order it gives
/// them, said by `message`: only that follower's asking matches it.
fn asked_by_each(message: impl Fn(usize) -> String) -> Vec<Vec<String>> {
    let line = |follower: usize, fields: &[&str]| {
        let mut line: Vec<String> = fields.iter().map(|field| (*field).to_owned()).collect();
        line.push(format!("MESSAGE={}", message(follower)));
        line
    };
    vec![
        line(0, &["PRIORITY=4", "SYSLOG_IDENTIFIER=d"]),
        line(
            1,
            &[
                "PRIORITY=6",
                &format!("MESSAGE_ID={}", journal::STARTUP_FINISHED),
            ],
        ),
        line(2, &["PRIORITY=5", "CODE_FUNC=manager_log_caller"]),
        line(
            3,
            &[
                "PRIORITY=6",
                &format!("MESSAGE_ID={}", journal::UNIT_STARTED),
            ],
        ),
        line(
            4,
            &[
                "PRIORITY=6",
                &format!("SYSLOG_IDENTIFIER={}", launcher::IDENTIFIER),
            ],
        ),
        line(
            5,
            &[
                "PRIORITY=6",
                &format!("SYSLOG_IDENTIFIER={}", launches::IDENTIFIER),
            ],
        ),
        line(6, &["PRIORITY=6", "SYSLOG_IDENTIFIER=d"]),
        line(
            7,
            &[
                "PRIORITY=6",
                &format!("SYSLOG_IDENTIFIER={}", asks::IDENTIFIER),
            ],
        ),
        line(8, &["PRIORITY=6", "_TRANSPORT=kernel"]),
    ]
}

/// What each follower's line says in `boot`, matching the `--grep` the
/// launcher's, the windows daemon's and the ready followers ask.
fn said(follower: usize, boot: usize) -> String {
    let tag = format!("{follower} in boot {boot}");
    match follower {
        4 => format!("launcher: home kept as the shell started {tag}"),
        5 => format!("opened a window 5 ms after its process started {tag}"),
        6 => format!("d ready in 5 ms {tag}"),
        _ => format!("line {tag}"),
    }
}

#[test]
fn every_follower_reads_every_boot_and_follows_on_across_a_reboot() {
    let followed = journal::followed();
    let mut lines = Vec::new();
    for (boot, id) in BOOTS.iter().enumerate() {
        let asked = asked_by_each(|follower| said(follower, boot));
        assert_eq!(asked.len(), followed.len(), "a follower has no line here");
        for fields in asked {
            lines.push(Line {
                boot: id,
                at: 1_794_891_700 + i64::try_from(lines.len()).unwrap_or_default(),
                fields,
            });
        }
    }
    let journal = Journal::of(&lines);
    for (follower, asked) in followed.iter().enumerate() {
        let read = journal.read(&journal::reading(asked, 1_794_891_600, 1_794_895_200));
        let messages: Vec<&str> = read.iter().map(|(message, _)| message.as_str()).collect();
        assert_eq!(
            messages,
            [said(follower, 0), said(follower, 1)],
            "{asked:?} didn't read its own line of every boot"
        );
        let followed_on = journal.read(&journal::following(asked, Some(&read[0].1), 0));
        let messages: Vec<&str> = followed_on
            .iter()
            .map(|(message, _)| message.as_str())
            .collect();
        assert_eq!(
            messages,
            [said(follower, 1)],
            "{asked:?} didn't follow on into the next boot from its history's cursor"
        );
    }
}

#[test]
fn a_replay_hears_a_real_day_in_written_order_through_a_clock_stepped_back() {
    // Health starts at 300, after the second boot's clock came up 200
    // seconds fast and was stepped back.
    let warning = |message: &str| vec!["PRIORITY=4".to_owned(), format!("MESSAGE={message}")];
    let reload = |message: &str| {
        vec![
            "PRIORITY=5".to_owned(),
            "CODE_FUNC=manager_log_caller".to_owned(),
            format!("MESSAGE={message}"),
        ]
    };
    let journal = Journal::of(&[
        Line {
            boot: BOOTS[0],
            at: 100,
            fields: warning("w1"),
        },
        Line {
            boot: BOOTS[0],
            at: 110,
            fields: reload("r1"),
        },
        Line {
            boot: BOOTS[1],
            at: 500,
            fields: warning("w2"),
        },
        Line {
            boot: BOOTS[1],
            at: 200,
            fields: reload("r2"),
        },
        Line {
            boot: BOOTS[1],
            at: 210,
            fields: warning("w3"),
        },
    ]);
    let followed = journal::followed();
    let asked = [followed[0].clone(), followed[2].clone()];
    let histories = asked
        .iter()
        .map(|asked| {
            let history = History::of(journal.reading(&journal::reading(asked, 0, 300)))
                .expect("journalctl starts");
            (history, asked.clone())
        })
        .collect();
    let (send, heard) = mpsc::channel();
    replay(
        histories,
        |asked: &Vec<String>, after| journal.reading(&journal::following(asked, after, 0)),
        &send,
    )
    .expect("every follower starts");
    drop(send);
    let messages: Vec<String> = heard
        .iter()
        .flatten()
        .map(|line| {
            let line: Value = serde_json::from_str(&line).expect("journalctl writes JSON");
            line["MESSAGE"].as_str().unwrap_or_default().to_owned()
        })
        .collect();
    assert_eq!(
        messages,
        ["w1", "r1", "w2", "r2", "w3"],
        "the day wasn't heard in the order it was written, once, \
         through a clock stepped back"
    );
}
