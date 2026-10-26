//! google holds the Google account for the desktop, so the launcher can
//! show Joel's next meeting and join it, and the mail that wants him.
//!
//! It asks Google's own API as the account the harness signed in (pi's
//! `/google-auth`), reading the harness's file and never writing it
//! (account.rs), and hands what it learns to the shell over a socket
//! (socket.rs): the whole state as one JSON line when a shell connects,
//! and again whenever it changes (state.rs).
//!
//! Google only pushes changes to a public web address, which a laptop
//! hasn't got, so this program asks: once a minute while answers come,
//! less often while Google can't be reached, up to every ten minutes. Its
//! clock is the wall's, looked at every ten seconds, so the first look
//! after the laptop wakes comes at once rather than a minute later. With
//! nobody signed in, the file is looked at, and Google never asked.
//!
//! The shell asks it to answer an invitation, and to mark mail read or
//! archive it, on the same socket (ask.rs), which it does at once, telling
//! every shell what changed.
//!
//! It says in the journal when Google can't be reached, and never a
//! token, a meeting or anything else the account holds.
//!
//! usage: google SOCKET CREDENTIALS, the harness's file; the account asked
//! as is the file's default, as pi's own is.

use std::env;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::mpsc::RecvTimeoutError;
use std::thread;
use std::time::{Duration, SystemTime};

use jiff::Zoned;

use google::api::{Google, Origins};
use google::ask::{self, Ask};
use google::mail::Tidy;
use google::socket::Socket;
use google::state::{Standing, State};
use google::watch::Watch;

/// How often Google is asked while it answers.
const EVERY: Duration = Duration::from_secs(60);
/// The longest wait between asks while it doesn't.
const LONGEST: Duration = Duration::from_secs(600);
/// How often the wall's clock is looked at.
const TICK: Duration = Duration::from_secs(10);

fn main() -> ExitCode {
    let mut args = env::args_os().skip(1);
    let (Some(path), Some(credentials)) = (
        args.next().map(PathBuf::from),
        args.next().map(PathBuf::from),
    ) else {
        eprintln!("<3>usage: google SOCKET CREDENTIALS");
        return ExitCode::from(2);
    };
    let (socket, asked) = match Socket::listen(&path, State::default().line()) {
        Ok(listening) => listening,
        Err(error) => {
            eprintln!("<3>couldn't listen at {}: {error}", path.display());
            return ExitCode::FAILURE;
        }
    };
    let mut watch = Watch::new(Google::new(Origins::default()), credentials, None);
    let mut wait = EVERY;
    let mut failures: u32 = 0;
    let mut last: Option<SystemTime> = None;
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
            socket.say(state.line());
        }
        // A shell's ask is done the moment it comes; otherwise the clock
        // is looked at again a tick on.
        match asked.recv_timeout(TICK) {
            Ok(line) => {
                let now = Zoned::now();
                let state = match ask::read(&line) {
                    Some(Ask::Answer { id, with }) => watch.answer(&id, with, &now),
                    Some(Ask::Read { id }) => watch.tidy(&id, Tidy::Read, &now),
                    Some(Ask::Archive { id }) => watch.tidy(&id, Tidy::Archive, &now),
                    None => continue,
                };
                socket.say(state.line());
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => thread::sleep(TICK),
        }
    }
}
