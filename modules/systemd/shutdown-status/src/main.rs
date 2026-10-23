//! Tells Plymouth, by keyword, when a shutdown is forced on or held up, so
//! the theme can say so in its own words. It starts with the shutdown and
//! survives its last stage, and reads that stage's steps from the kernel's
//! log, where systemd-shutdown writes them once the journal is gone.
//!
//! forced: the last stage began before the reboot, power-off, halt or kexec
//! service had run, so the services' minute ran out.
//! leftovers: the last stage asked what's left to stop, or killed it, and
//! heard nothing back for three seconds, so something won't go.
//!
//! Usage: shutdown-status REACHED, where REACHED is the mark those services
//! leave. PLYMOUTH names Plymouth's client, `plymouth` by default.

use std::env;
use std::ffi::{OsStr, OsString};
use std::fs::File;
use std::io::{ErrorKind, Read, Seek, SeekFrom};
use std::path::PathBuf;
use std::process::{Command, ExitCode, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::thread;
use std::time::{Duration, Instant};

/// How long the last stage may go quiet after asking what's left to stop.
const HELD_AFTER: Duration = Duration::from_secs(3);

/// How long Plymouth's client gets to take a keyword.
const SAY_WITHIN: Duration = Duration::from_secs(2);

/// The steps after which the last stage waits on what's left.
const WAITS: [&str; 2] = [
    "Sending SIGTERM to remaining",
    "Sending SIGKILL to remaining",
];

/// The kernel's answer to a reader its log has overtaken.
const EPIPE: i32 = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Word {
    Forced,
    Leftovers,
}

impl Word {
    fn as_str(self) -> &'static str {
        match self {
            Word::Forced => "forced",
            Word::Leftovers => "leftovers",
        }
    }
}

/// The last stage's step that a kernel log record carries, if it carries
/// one. The last stage is systemd-shutdown, and shutdown once in the ramfs,
/// and a record's text follows its first semicolon, one line of it.
fn step(record: &str) -> Option<&str> {
    let text = record.split_once(';').map_or(record, |(_, text)| text);
    let text = text.strip_prefix("systemd-").unwrap_or(text);
    let step = text.strip_prefix("shutdown[1]: ")?;
    step.split('\n').next()
}

/// What the last stage has shown so far.
#[derive(Debug, Default)]
struct Watch {
    begun: bool,
    waiting_since: Option<Instant>,
}

impl Watch {
    /// Takes a kernel log record heard at `now`, and says what Plymouth
    /// should be told. `reached` says whether the services that end a
    /// healthy stop left their mark, and is asked once, at the first step.
    fn heard(
        &mut self,
        record: &str,
        now: Instant,
        reached: impl FnOnce() -> bool,
    ) -> Option<Word> {
        let step = step(record)?;
        let mut word = None;
        if !self.begun {
            self.begun = true;
            if !reached() {
                word = Some(Word::Forced);
            }
        }
        self.waiting_since = WAITS
            .iter()
            .any(|wait| step.starts_with(wait))
            .then_some(now);
        word
    }

    /// When the last stage's silence would mean something won't go.
    fn deadline(&self) -> Option<Instant> {
        self.waiting_since.map(|since| since + HELD_AFTER)
    }

    /// The deadline passed with nothing heard.
    fn silence(&mut self) -> Word {
        self.waiting_since = None;
        Word::Leftovers
    }
}

/// Follows the kernel's log from its end, one record a message. The reads
/// block, so they run on their own thread and the caller can wait with a
/// timeout. The channel closes if the log can't be read.
fn follow_kernel_log() -> std::io::Result<Receiver<String>> {
    let mut log = File::open("/dev/kmsg")?;
    log.seek(SeekFrom::End(0))?;
    let (send, records) = mpsc::channel();
    thread::spawn(move || {
        let mut buffer = [0u8; 8192];
        loop {
            match log.read(&mut buffer) {
                Ok(read) => {
                    let record = String::from_utf8_lossy(&buffer[..read]).into_owned();
                    if send.send(record).is_err() {
                        return;
                    }
                }
                Err(error) if error.kind() == ErrorKind::Interrupted => {}
                // Records were overwritten before they were read.
                Err(error) if error.raw_os_error() == Some(EPIPE) => {
                    if log.seek(SeekFrom::End(0)).is_err() {
                        return;
                    }
                }
                Err(error) => {
                    eprintln!("shutdown-status: can't read the kernel's log: {error}");
                    return;
                }
            }
        }
    });
    Ok(records)
}

/// Tells Plymouth a keyword, giving up if its client won't take it in time.
fn say(plymouth: &OsStr, word: Word) {
    let Ok(mut client) = Command::new(plymouth)
        .arg("update")
        .arg(format!("--status={}", word.as_str()))
        .stdin(Stdio::null())
        .spawn()
    else {
        return;
    };
    let until = Instant::now() + SAY_WITHIN;
    while Instant::now() < until {
        match client.try_wait() {
            Ok(None) => thread::sleep(Duration::from_millis(20)),
            Ok(Some(_)) | Err(_) => return,
        }
    }
    let _ = client.kill();
    let _ = client.wait();
}

fn main() -> ExitCode {
    let Some(reached) = env::args_os().nth(1).map(PathBuf::from) else {
        eprintln!("usage: shutdown-status REACHED");
        return ExitCode::from(2);
    };
    let plymouth = env::var_os("PLYMOUTH").unwrap_or_else(|| OsString::from("plymouth"));
    let records = match follow_kernel_log() {
        Ok(records) => records,
        Err(error) => {
            eprintln!("shutdown-status: can't open the kernel's log: {error}");
            return ExitCode::FAILURE;
        }
    };
    let mut watch = Watch::default();
    loop {
        let heard = match watch.deadline() {
            None => records.recv().map_err(|_| RecvTimeoutError::Disconnected),
            Some(deadline) => {
                records.recv_timeout(deadline.saturating_duration_since(Instant::now()))
            }
        };
        match heard {
            Ok(record) => {
                if let Some(word) = watch.heard(&record, Instant::now(), || reached.exists()) {
                    say(&plymouth, word);
                }
            }
            Err(RecvTimeoutError::Timeout) => say(&plymouth, watch.silence()),
            Err(RecvTimeoutError::Disconnected) => return ExitCode::FAILURE,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    const SYNCING: &str =
        "5,812,9031733,-;systemd-shutdown[1]: Syncing filesystems and block devices.";
    const SIGTERM: &str =
        "5,813,9032001,-;systemd-shutdown[1]: Sending SIGTERM to remaining processes...";
    const SIGKILL: &str =
        "5,820,9122001,-;systemd-shutdown[1]: Sending SIGKILL to remaining processes...";
    const UNMOUNT: &str = "5,821,9122302,-;systemd-shutdown[1]: Unmounting file systems.";
    const OTHER: &str = "6,822,9122400,-;systemd[1]: Stopped target Basic System.";

    #[test]
    fn reads_the_last_stage_from_systemd_shutdown_and_the_ramfs() {
        assert_eq!(
            step(SYNCING),
            Some("Syncing filesystems and block devices.")
        );
        assert_eq!(
            step("5,900,9200000,-;shutdown[1]: Unmounting /oldroot."),
            Some("Unmounting /oldroot.")
        );
    }

    #[test]
    fn ignores_every_other_writer() {
        assert_eq!(step(OTHER), None);
        assert_eq!(step("5,1,2,-;systemd-shutdown[2]: Syncing."), None);
        assert_eq!(step("5,1,2,-;kernel: shutdown[1]: Syncing."), None);
        assert_eq!(step("5,1,2,-;systemd-journald[1]: Syncing."), None);
    }

    #[test]
    fn reads_only_the_first_line_of_a_record() {
        let record = "5,1,2,-;systemd-shutdown[1]: Sending SIGTERM to remaining processes...\n \
                      SUBSYSTEM=block\n";
        assert_eq!(
            step(record),
            Some("Sending SIGTERM to remaining processes...")
        );
    }

    #[test]
    fn reads_a_record_without_a_prefix() {
        assert_eq!(step("shutdown[1]: Syncing."), Some("Syncing."));
    }

    #[test]
    fn says_forced_when_the_last_stage_begins_unmarked() {
        let mut watch = Watch::default();
        assert_eq!(
            watch.heard(SYNCING, Instant::now(), || false),
            Some(Word::Forced)
        );
    }

    #[test]
    fn says_nothing_when_the_last_stage_begins_marked() {
        let mut watch = Watch::default();
        assert_eq!(watch.heard(SYNCING, Instant::now(), || true), None);
    }

    #[test]
    fn asks_for_the_mark_once_and_says_forced_once() {
        let asked = Cell::new(0);
        let mut watch = Watch::default();
        let now = Instant::now();
        let unmarked = || {
            asked.set(asked.get() + 1);
            false
        };
        assert_eq!(watch.heard(SYNCING, now, unmarked), Some(Word::Forced));
        assert_eq!(watch.heard(UNMOUNT, now, unmarked), None);
        assert_eq!(asked.get(), 1);
    }

    #[test]
    fn other_writers_do_not_begin_the_last_stage() {
        let mut watch = Watch::default();
        let now = Instant::now();
        assert_eq!(watch.heard(OTHER, now, || false), None);
        assert!(!watch.begun);
        assert_eq!(watch.heard(SYNCING, now, || false), Some(Word::Forced));
    }

    #[test]
    fn waits_three_seconds_after_asking_whats_left_to_stop() {
        let mut watch = Watch::default();
        let now = Instant::now();
        watch.heard(SIGTERM, now, || true);
        assert_eq!(watch.deadline(), Some(now + HELD_AFTER));
        let later = now + Duration::from_secs(1);
        watch.heard(SIGKILL, later, || true);
        assert_eq!(watch.deadline(), Some(later + HELD_AFTER));
    }

    #[test]
    fn any_step_after_a_wait_ends_it() {
        let mut watch = Watch::default();
        let now = Instant::now();
        watch.heard(SIGTERM, now, || true);
        watch.heard(UNMOUNT, now, || true);
        assert_eq!(watch.deadline(), None);
    }

    #[test]
    fn other_writers_do_not_end_a_wait() {
        let mut watch = Watch::default();
        let now = Instant::now();
        watch.heard(SIGTERM, now, || true);
        watch.heard(OTHER, now, || true);
        assert_eq!(watch.deadline(), Some(now + HELD_AFTER));
    }

    #[test]
    fn silence_after_a_wait_says_leftovers_once() {
        let mut watch = Watch::default();
        watch.heard(SIGTERM, Instant::now(), || true);
        assert_eq!(watch.silence(), Word::Leftovers);
        assert_eq!(watch.deadline(), None);
    }

    #[test]
    fn says_its_keywords_as_the_theme_reads_them() {
        assert_eq!(Word::Forced.as_str(), "forced");
        assert_eq!(Word::Leftovers.as_str(), "leftovers");
    }
}
