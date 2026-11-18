//! The day's histories and followers on systemd's own journalctl, over a
//! journal systemd's journal-remote writes, rather than on stand-ins that
//! only say what journalctl is thought to do. Both are the systemd first
//! on PATH: in the build, the one package.nix gives the checks.

use std::cell::RefCell;
use std::fmt::Write as _;
use std::fs;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::Duration;

use jiff::Timestamp;
use serde_json::Value;

use super::super::*;
use crate::{asks, launcher, launches};

/// The boot before a reboot, and the boot after.
const BOOTS: [&str; 2] = [
    "11111111111111111111111111111111",
    "22222222222222222222222222222222",
];

/// How long a follower may take to say a line already written.
const FOLLOWING: Duration = Duration::from_secs(10);

/// How long a follower that has said its lines must go on saying nothing
/// and keep its output open, as one following does.
const STILL_FOLLOWING: Duration = Duration::from_millis(500);

/// The start of the day the followers are checked on, long past.
const DAY: i64 = 1_794_891_600;

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

/// systemd's journal-remote, beside the journalctl on PATH, so the
/// journal it writes is one that journalctl reads.
fn journal_remote() -> PathBuf {
    let path = std::env::var_os("PATH").unwrap_or_default();
    std::env::split_paths(&path)
        .map(|folder| folder.join("journalctl"))
        .find(|journalctl| journalctl.is_file())
        .and_then(|journalctl| fs::canonicalize(journalctl).ok())
        .and_then(|journalctl| {
            Some(
                journalctl
                    .parent()?
                    .parent()?
                    .join("lib/systemd/systemd-journal-remote"),
            )
        })
        .filter(|remote| remote.is_file())
        .expect(
            "systemd-journal-remote sits beside the journalctl on PATH, in its lib/systemd: \
             where it doesn't, run these through the health-tallies-the-days-warnings check, \
             or put a systemd that has it first on PATH",
        )
}

impl Journal {
    fn of(lines: &[Line]) -> Self {
        let folder = tempfile::tempdir().expect("a folder for the journal");
        let mut export = String::new();
        for (written, line) in lines.iter().enumerate() {
            write!(
                export,
                "__REALTIME_TIMESTAMP={}\n__MONOTONIC_TIMESTAMP={}\n_BOOT_ID={}\n",
                line.at * 1_000_000,
                written + 1,
                line.boot
            )
            .expect("a string takes what's written");
            for field in &line.fields {
                export.push_str(field);
                export.push('\n');
            }
            export.push('\n');
        }
        let exported = folder.path().join("day.export");
        fs::write(&exported, export).expect("the day is exported");
        let written = Command::new(journal_remote())
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

    /// `command` reading this journal rather than the machine's.
    fn here(&self, command: &Command) -> Command {
        here_in(self.folder.path(), command)
    }

    /// `command` reading this journal, and ending at its end rather than
    /// following on.
    fn ending(&self, command: &Command) -> Command {
        ending_in(self.folder.path(), command)
    }

    /// What `command` gives from this journal, read to its end.
    fn read(&self, command: &Command) -> Vec<Said> {
        History::of(self.ending(command))
            .expect("journalctl starts")
            .map(|line| said(&line))
            .collect()
    }

    /// The first `count` lines `command`, a follower, gives from this
    /// journal, following as health's do; each must come in `FOLLOWING`,
    /// and then nothing more, the follower still following, for
    /// `STILL_FOLLOWING`.
    fn follow(&self, command: &Command, count: usize) -> Vec<Said> {
        let mut follower = self
            .here(command)
            .stdout(Stdio::piped())
            .spawn()
            .expect("journalctl starts");
        let out = follower.stdout.take().expect("its output");
        let (send, heard) = mpsc::channel();
        let reading = thread::spawn(move || {
            for line in BufReader::new(out).lines().map_while(Result::ok) {
                if send.send(line).is_err() {
                    break;
                }
            }
        });
        let said: Vec<Option<Said>> = (0..count)
            .map(|_| heard.recv_timeout(FOLLOWING).ok().map(|line| said(&line)))
            .collect();
        // Still following once it has said them: one that read to the
        // journal's end flushed its lines as it ended, closing its output,
        // which a wait for its next line hears where its exit can still
        // be unreaped.
        let after = heard.recv_timeout(STILL_FOLLOWING);
        // The follower is ended however many it said, and its reading
        // with it, so a test that fails leaves no journalctl behind.
        let _ = follower.kill();
        let _ = follower.wait();
        let _ = reading.join();
        match after {
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                panic!("{} ended rather than following on", named(command))
            }
            Ok(line) => panic!("{} said a line it shouldn't: {line}", named(command)),
        }
        said.into_iter()
            .map(|said| {
                said.unwrap_or_else(|| panic!("{} didn't say a line it should", named(command)))
            })
            .collect()
    }
}

/// `command` reading the journal in `folder` rather than the machine's.
fn here_in(folder: &Path, command: &Command) -> Command {
    let mut here = Command::new(command.get_program());
    here.args(command.get_args())
        .arg(format!("--directory={}", folder.display()));
    here
}

/// `command` reading the journal in `folder`, and ending at its end
/// rather than following on.
fn ending_in(folder: &Path, command: &Command) -> Command {
    let here = here_in(folder, command);
    let mut ending = Command::new(here.get_program());
    ending.args(here.get_args().filter(|arg| *arg != "--follow"));
    ending
}

/// A line's message and cursor, as journalctl wrote it.
struct Said {
    message: String,
    cursor: String,
}

fn said(line: &str) -> Said {
    let line: Value = serde_json::from_str(line).expect("journalctl writes JSON");
    let field = |name: &str| line[name].as_str().unwrap_or_default().to_owned();
    Said {
        message: field("MESSAGE"),
        cursor: field("__CURSOR"),
    }
}

fn messages(said: &[Said]) -> Vec<&str> {
    said.iter().map(|said| said.message.as_str()).collect()
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

/// What each follower's line says `when`, matching the `--grep` the
/// launcher's, the windows daemon's and the ready followers ask.
fn said_in(follower: usize, when: &str) -> String {
    let tag = format!("{follower} {when}");
    match follower {
        4 => format!("launcher: home kept as the shell started {tag}"),
        5 => format!("opened a window 5 ms after its process started {tag}"),
        6 => format!("d ready in 5 ms {tag}"),
        _ => format!("line {tag}"),
    }
}

#[test]
fn every_follower_reads_every_boot_of_the_day_and_follows_on_across_a_reboot() {
    // Neither boot is the build's own, which journalctl follows alone
    // unless asked for every boot; and the first began the day before.
    let followed = journal::followed();
    let mut lines = Vec::new();
    for (boot, when, from) in [
        (0, "yesterday", DAY - 600),
        (0, "in boot 0", DAY + 100),
        (1, "in boot 1", DAY + 200),
    ] {
        let asked = asked_by_each(|follower| said_in(follower, when));
        assert_eq!(asked.len(), followed.len(), "a follower has no line here");
        for (follower, fields) in asked.into_iter().enumerate() {
            lines.push(Line {
                boot: BOOTS[boot],
                at: from + i64::try_from(follower).unwrap_or_default(),
                fields,
            });
        }
    }
    let journal = Journal::of(&lines);
    // Each follower is checked on its own thread, so their waits on
    // following on are waited at once.
    thread::scope(|scope| {
        for (follower, asked) in followed.iter().enumerate() {
            let journal = &journal;
            scope.spawn(move || follows_on(journal, follower, asked));
        }
    });
}

/// Follower `follower`, asking `asked`, reads its line of each boot of
/// the day in `journal` and follows on, from its history's cursor and
/// from the day's start.
fn follows_on(journal: &Journal, follower: usize, asked: &[String]) {
    let today = [
        said_in(follower, "in boot 0"),
        said_in(follower, "in boot 1"),
    ];
    let read = journal.read(&journal::reading(asked, DAY));
    assert_eq!(
        messages(&read),
        today,
        "{asked:?} didn't read its own line of every boot of the day, and none before"
    );
    // Each follower is read to the journal's end, for every line it
    // gives, then followed, for following on across the reboot.
    let on = journal::following(asked, Some(&read[0].cursor), 0);
    assert_eq!(
        messages(&journal.read(&on)),
        [said_in(follower, "in boot 1")],
        "{asked:?} didn't go on into the next boot from its history's cursor, and only there"
    );
    assert_eq!(
        messages(&journal.follow(&on, 1)),
        [said_in(follower, "in boot 1")],
        "{asked:?} didn't follow on into the next boot from its history's cursor"
    );
    let from_the_start = journal::following(asked, None, DAY);
    assert_eq!(
        messages(&journal.read(&from_the_start)),
        today,
        "{asked:?} with no history didn't go on from the day's start, and only through it"
    );
    assert_eq!(
        messages(&journal.follow(&from_the_start, 2)),
        today,
        "{asked:?} with no history didn't follow every boot from the day's start"
    );
}

#[test]
fn an_empty_journal_has_no_last_line_and_nothing_is_said_of_it() {
    // journalctl writes nothing here and ends whole (systemd 262, 18
    // November), which is a journal with no lines, not a fault.
    let folder = tempfile::tempdir().expect("a folder with no journal");
    let newest = ending_in(folder.path(), &journal::newest());
    let name = named(&newest);
    let said = asking(newest)
        .expect("journalctl starts")
        .wait_with_output();
    let notices = RefCell::new(Vec::new());
    let last = last_of(&name, said, |words| {
        notices.borrow_mut().push(words.to_owned());
    });
    assert_eq!(
        (last, notices.into_inner()),
        (None, Vec::<String>::new()),
        "an empty journal had a last line, or was said as a fault"
    );
}

#[test]
fn a_replay_hears_a_real_day_in_written_order_through_a_clock_stepped_back() {
    // The second boot's clock came up an hour fast and was stepped back,
    // so a line it wrote later is stamped earlier than one before it,
    // and that one is stamped past now: a history ending at now, or at
    // any time before the fast clock's stamps, would end before the lines
    // written after it, and one ending later wouldn't end at the lines
    // written as the replay runs (`journal::reading`).
    let now = Timestamp::now().as_second();
    let warning = |message: &str| vec!["PRIORITY=4".to_owned(), format!("MESSAGE={message}")];
    let reload = |message: &str| {
        vec![
            "PRIORITY=5".to_owned(),
            "CODE_FUNC=manager_log_caller".to_owned(),
            format!("MESSAGE={message}"),
        ]
    };
    let line = |boot: usize, at: i64, fields: Vec<String>| Line {
        boot: BOOTS[boot],
        at,
        fields,
    };
    let journal = Journal::of(&[
        line(0, now - 3600, warning("w1")),
        line(0, now - 3590, reload("r1")),
        line(1, now + 3600, warning("w2")),
        line(1, now - 1800, reload("r2")),
        line(1, now - 1790, warning("w3")),
        // Written last, and asked for by no history, so the journal's
        // last line is no history's last.
        line(
            1,
            now - 1780,
            vec!["PRIORITY=7".to_owned(), "MESSAGE=d1".to_owned()],
        ),
    ]);
    let last = last_written(journal.ending(&journal::newest())).expect("journalctl starts");
    let Some((counter, 6)) = last else {
        panic!("the journal's last line wasn't the one written last: {last:?}");
    };
    let followed = journal::followed();
    let since = now - 86_400;
    let line_of = |asked: &[String], message: &str| {
        History::of(journal.ending(&journal::reading(asked, since)))
            .expect("journalctl starts")
            .find(|line| said(line).message == message)
            .unwrap_or_else(|| panic!("{message} is in the journal"))
    };
    assert_eq!(
        replay::numbering(&line_of(&followed[0], "w3")),
        Some((counter, 5)),
        "the journal's lines weren't numbered as the histories number them"
    );
    // As if w3 was written once the replay began, the journal's last line
    // is r2 as health asks for it: the warnings' history ends at w2, and
    // its follower hears w3. The real newest, checked above, can't stand
    // in here, since no line can be written while the replay runs.
    let as_newest = super::writing(&[line_of(&followed[2], "r2").trim_end().to_owned()]);
    let folder = journal.folder.path().to_owned();
    let reading = {
        let folder = folder.clone();
        move |asked: &[String]| ending_in(&folder, &journal::reading(asked, since))
    };
    let after = Arc::new(Mutex::new(Vec::new()));
    let following = {
        let after = Arc::clone(&after);
        move |asked: &[String], from: Option<&str>| {
            after
                .lock()
                .expect("the cursors are kept")
                .push(from.map(str::to_owned));
            ending_in(&folder, &journal::following(asked, from, since))
        }
    };
    let (send, heard) = mpsc::channel();
    let count = replayed_by(as_newest, reading, following, &send);
    drop(send);
    let mut heard_said = Vec::new();
    let mut ended = 0;
    while ended < count {
        match heard
            .recv_timeout(FOLLOWING)
            .expect("a follower neither said a line nor ended")
        {
            Some(line) => heard_said.push(said(&line).message),
            None => ended += 1,
        }
    }
    assert!(
        matches!(
            heard.recv_timeout(FOLLOWING),
            Err(mpsc::RecvTimeoutError::Disconnected)
        ),
        "the replay said more than it counted, once every count had ended"
    );
    assert_eq!(
        heard_said,
        ["w1", "r1", "w2", "r2", "w3"],
        "the day wasn't heard in the order it was written, once, through a clock stepped back"
    );
    let cursor_of = |asked: &[String], message: &str| Some(said(&line_of(asked, message)).cursor);
    let mut ends = vec![None; followed.len()];
    ends[0] = cursor_of(&followed[0], "w2");
    ends[2] = cursor_of(&followed[2], "r2");
    assert_eq!(
        *after.lock().expect("the cursors are kept"),
        ends,
        "a history didn't end at the journal's last line as the replay began, its follower \
         following on from there"
    );
}
