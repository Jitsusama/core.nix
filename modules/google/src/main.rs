//! google holds the Google account for the desktop, so the launcher can
//! show Joel's next meeting and join it, and the mail that wants him.
//!
//! It asks Google's own API as the account the harness signed in (pi's
//! `/google-auth`), reading the harness's file and never writing it
//! (account.rs), and serves what it learns (state.rs) in the daemon
//! interface, a record for each thing it holds, on the socket systemd
//! hands it or else one it binds (daemon.rs): the shell holds it there,
//! and call transcripts ask it for the meeting on.
//!
//! Google only pushes changes to a public web address, which a laptop
//! hasn't got, so this program asks: once a minute while answers come,
//! less often while Google can't be reached, up to every ten minutes. Its
//! clock is the wall's, looked at every ten seconds, so the first look
//! after the laptop wakes comes at once rather than a minute later. With
//! nobody signed in, the file is looked at, and Google never asked.
//!
//! The shell asks it to answer an invitation, to mark mail read or
//! archive it, to fetch a file mail carries, to make a Meet meeting for
//! its link, which Joel's sign-in must grant, and to list a Drive folder
//! or download a file in one, and to follow the Meet calls open in
//! windows of their own, asking Meet who's in each every half minute while
//! they're open, as acts on the same socket (ask.rs), which
//! it does at once, telling every watch what changed. A file to open, or
//! to hand a program through the picker, is kept in google-files, in the
//! session's runtime folder; one to save goes where Joel
//! picked, else to Downloads (`$XDG_DOWNLOAD_DIR`, or `~/Downloads`)
//! (files.rs). One ask is done at a time, so a large download holds the
//! next until it's written.
//!
//! What Joel saves into Drive through the picker is staged in the state
//! home's google/drive-saves, which this program watches, sending each
//! file up once it's written, and telling every shell how it went
//! (saves.rs).
//!
//! It says in the journal when Google can't be reached, and never a
//! token, a meeting or anything else the account holds.
//!
//! usage: google CREDENTIALS DAEMON, CREDENTIALS the harness's file, the
//! account asked as being the file's default, as pi's own is, and DAEMON
//! the daemon interface's socket.

use std::env;
use std::path::Path;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::mpsc::{self, RecvTimeoutError};
use std::thread;
use std::time::{Duration, SystemTime};

use jiff::{Timestamp, Zoned};

use google::api::{Google, Origins};
use google::ask::{self, Ask};
use google::daemon::Daemon;
use google::files::Places;
use google::mail::Tidy;
use google::saves;
use google::state::{Standing, State};
use google::watch::Watch;

/// How often Google is asked while it answers.
const EVERY: Duration = Duration::from_secs(60);
/// The longest wait between asks while it doesn't.
const LONGEST: Duration = Duration::from_secs(600);
/// How often the wall's clock is looked at.
const TICK: Duration = Duration::from_secs(10);
/// How often Meet is asked who's in a call followed, since people come
/// and go by the minute; a call starting is a window opening, which the
/// shell's follow says at once.
const CALLS_EVERY: Duration = Duration::from_secs(30);

fn main() -> ExitCode {
    let mut args = env::args_os().skip(1);
    let (Some(credentials), Some(at)) = (
        args.next().map(PathBuf::from),
        args.next().map(PathBuf::from),
    ) else {
        eprintln!("<3>usage: google CREDENTIALS DAEMON");
        return ExitCode::from(2);
    };
    let (asks, asked) = mpsc::channel();
    let daemon = match Daemon::listen(&at, asks.clone(), Timestamp::now().as_millisecond()) {
        Ok(listening) => listening,
        Err(error) => {
            eprintln!("<3>couldn't serve at {}: {error}", at.display());
            return ExitCode::FAILURE;
        }
    };
    let tell = |state: &State| daemon.say(state, Timestamp::now().as_millisecond());
    let staged = saves::root(env::var_os("XDG_STATE_HOME"), env::var_os("HOME"));
    saves::prune(&staged, SystemTime::now());
    // Without its watch, what Joel saves stays staged, to go up as the
    // program next starts.
    if let Err(error) = saves::watch(staged.clone(), asks) {
        eprintln!("<4>couldn't watch Drive saves: {error}");
    }
    let places = Places {
        kept: env::var_os("XDG_RUNTIME_DIR")
            .map_or_else(env::temp_dir, PathBuf::from)
            .join("google-files"),
        saves: env::var_os("XDG_DOWNLOAD_DIR").map_or_else(
            || PathBuf::from(env::var_os("HOME").unwrap_or_default()).join("Downloads"),
            PathBuf::from,
        ),
    };
    let mut watch = Watch::new(Google::new(Origins::default()), credentials, None);
    let mut wait = EVERY;
    let mut failures: u32 = 0;
    let mut last: Option<SystemTime> = None;
    let mut called: Option<SystemTime> = None;
    loop {
        let due = last.is_none_or(|last| {
            SystemTime::now()
                .duration_since(last)
                .map_or(true, |since| since >= wait)
        });
        if due {
            last = Some(SystemTime::now());
            let state = watch.look(&Zoned::now());
            // The first miss, as often as not the network still coming
            // up after a wake, is tried again a minute on; each after it
            // waits twice as long as the last.
            failures = match state.account {
                Standing::Unreachable => failures.saturating_add(1),
                _ => 0,
            };
            wait = (EVERY * 2u32.saturating_pow(failures.saturating_sub(1).min(4))).min(LONGEST);
            tell(state);
        }
        // Asked less often while Meet can't be, as the calendar is.
        let calls_wait = CALLS_EVERY * 2u32.saturating_pow(watch.calls_failed().min(4));
        let calls_due = called.is_none_or(|called| {
            SystemTime::now()
                .duration_since(called)
                .map_or(true, |since| since >= calls_wait)
        });
        if watch.following(&Zoned::now()) && calls_due {
            called = Some(SystemTime::now());
            tell(watch.calls(&Zoned::now()));
        }
        // A shell's ask is done the moment it comes; otherwise the clock
        // is looked at again a tick on.
        match asked.recv_timeout(TICK) {
            Ok(line) => {
                let now = Zoned::now();
                let state = match ask::read(&line) {
                    Some(Ask::Answer { id, with }) => watch.answer(&id, with, &now),
                    Some(Ask::Cancel { id }) => watch.cancel(&id, &now),
                    Some(Ask::Move { id, start, end }) => watch.reschedule(&id, start, end, &now),
                    Some(Ask::Read { id }) => watch.tidy(&id, Tidy::Read, &now),
                    Some(Ask::Unread { id }) => watch.unread(&id, &now),
                    Some(Ask::Archive { id }) => watch.tidy(&id, Tidy::Archive, &now),
                    // Said as it starts, so the shell can say so while
                    // Gmail answers, then again once it's done.
                    Some(Ask::Fetch { id, file, how, to }) => {
                        tell(watch.fetching(&id, file, how, to, &now));
                        watch.fetch(&places, &now)
                    }
                    Some(Ask::Make {}) => {
                        tell(watch.making(&now));
                        watch.make(&now)
                    }
                    Some(Ask::List { folder }) => watch.list(&folder, &now),
                    Some(Ask::Follow { codes }) => {
                        called = Some(SystemTime::now());
                        watch.follow(codes, &now)
                    }
                    Some(Ask::Download { file, how, to }) => {
                        tell(watch.downloading(&file, how, to, &now));
                        watch.download(&places, &now)
                    }
                    Some(Ask::Upload { path }) => {
                        let Some(file) = saves::staged(&staged, Path::new(&path)) else {
                            continue;
                        };
                        tell(watch.uploading(&file, &now));
                        watch.upload(&now)
                    }
                    None => continue,
                };
                tell(state);
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => thread::sleep(TICK),
        }
    }
}
