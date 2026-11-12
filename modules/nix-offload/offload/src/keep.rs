//! The keeper of what passed (`nix-offload keep`): a service of its own
//! account, started by its socket, that alone writes the record of what
//! passed at each commit (`passed.rs`), in a folder every account reads and
//! only it writes.
//!
//! On 11 November a session meaning to remove one commit's record removed
//! them all, and every gate that had passed but not merged ran again. The
//! account could, since the records were its own files. Here it can only
//! ask: to add lines to a commit's record, which the keeper checks and
//! appends whole, never rewriting one already there, or to forget one
//! commit's record (`nix-offload passed forget COMMIT`), which is how one is
//! built again. A record untouched for 30 days ages out; nothing else
//! removes one.
//!
//! The account can reach the socket, so what one asks is bounded: a line
//! of at most 64 KiB, a record of at most 16 MiB, 20,000 records, and five
//! seconds to say it all. Idle for a minute, the
//! keeper ends, and its socket starts it again when it's next asked.

use std::fs;
use std::io::{self, Read, Write};
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;
use std::process::ExitCode;
use std::time::{Duration, Instant, SystemTime};

use crate::passed::At;

/// Where the keeper keeps the records.
pub const FOLDER: &str = "/var/lib/nix-offload-passed";

/// Where the keeper is asked.
pub const SOCKET: &str = "/run/nix-offload-keep.sock";

/// The socket's unit, there only while the machine runs the keeper, so a
/// folder left by a machine that dropped it isn't read.
pub const UNIT: &str = "/etc/systemd/system/nix-offload-keep.socket";

/// The longest line the keeper takes.
const LINE_AT_MOST: usize = 64 * 1024;

/// The largest record it lets a commit's file grow to.
const RECORD_AT_MOST: u64 = 16 * 1024 * 1024;

/// The most commits it keeps a record for, so an asker looping over
/// commits can't fill the disk a record at a time.
const RECORDS_AT_MOST: usize = 20_000;

/// How long a record stays untouched before it ages out.
const KEPT_FOR: Duration = Duration::from_hours(30 * 24);

/// How long the keeper waits, idle, before ending.
const IDLE: Duration = Duration::from_mins(1);

/// How long one asking may take, in all, to say what it asks.
const HEARD_WITHIN: Duration = Duration::from_secs(5);

/// Whether this machine runs the keeper.
/// `NIX_OFFLOAD_KEEP_UNIT` names another unit in its place, so a test run
/// on a machine with a keeper keeps its records apart: on 12 November
/// tests run in a dev shell put three made-up commits in the real
/// record, which read as zero-minute gates.
pub fn stands() -> bool {
    std::env::var_os("NIX_OFFLOAD_KEEP_UNIT")
        .map_or_else(|| Path::new(UNIT).to_owned(), std::path::PathBuf::from)
        .exists()
}

/// What the keeper is asked.
#[derive(Debug, PartialEq, Eq)]
pub enum Asked {
    /// Add these lines, each a JSON object, to a commit's record.
    Keep(At, Vec<String>),
    /// Remove a commit's record.
    Forget(At),
}

/// What `text` asks, checked: a whole commit, and lines that are each a
/// JSON object within the bound, written again in one line each.
pub fn heard(text: &str) -> Result<Asked, String> {
    let asked: serde_json::Value =
        serde_json::from_str(text).map_err(|error| format!("not JSON: {error}"))?;
    let at = |commit: &serde_json::Value| {
        commit
            .as_str()
            .and_then(|commit| At::new(commit, None))
            .ok_or_else(|| "not a whole commit".to_owned())
    };
    if let Some(commit) = asked.get("forget") {
        return Ok(Asked::Forget(at(commit)?));
    }
    let Some(commit) = asked.get("keep") else {
        return Err("asks neither keep nor forget".to_owned());
    };
    let at = at(commit)?;
    let Some(lines) = asked["lines"].as_array() else {
        return Err("keeps no lines".to_owned());
    };
    let lines = lines
        .iter()
        .map(|line| {
            let written = line.to_string();
            if !line.is_object() {
                Err("a line that isn't a JSON object".to_owned())
            } else if written.len() > LINE_AT_MOST {
                Err(format!("a line past {LINE_AT_MOST} bytes"))
            } else {
                Ok(written)
            }
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Asked::Keep(at, lines))
}

/// Do what was asked in `folder`.
pub fn answer(folder: &Path, asked: &Asked) -> Result<(), String> {
    answer_holding(folder, asked, RECORDS_AT_MOST)
}

/// Do what was asked in `folder`, taking no new commit once it holds
/// `most` records.
fn answer_holding(folder: &Path, asked: &Asked, most: usize) -> Result<(), String> {
    match asked {
        Asked::Forget(at) => match fs::remove_file(folder.join(&at.commit)) {
            Err(error) if error.kind() != io::ErrorKind::NotFound => Err(error.to_string()),
            _ => Ok(()),
        },
        Asked::Keep(at, lines) => {
            let file = folder.join(&at.commit);
            let text = lines
                .iter()
                .fold(String::new(), |text, line| text + line + "\n");
            let had = fs::metadata(&file).map_or(0, |kept| kept.len());
            let holds = || fs::read_dir(folder).map_or(0, Iterator::count);
            if !file.exists() && holds() >= most {
                return Err(format!("the keeper holds {most} records already"));
            }
            if had + text.len() as u64 > RECORD_AT_MOST {
                return Err(format!("{} would pass {RECORD_AT_MOST} bytes", at.short()));
            }
            fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&file)
                .and_then(|mut opened| opened.write_all(text.as_bytes()))
                .map_err(|error| error.to_string())
        }
    }
}

/// Remove the records in `folder` untouched since `KEPT_FOR` before `now`.
pub fn age_out(folder: &Path, now: SystemTime) {
    for entry in fs::read_dir(folder).into_iter().flatten().flatten() {
        let touched = entry.metadata().and_then(|kept| kept.modified());
        let aged = touched.is_ok_and(|touched| {
            now.duration_since(touched)
                .is_ok_and(|since| since > KEPT_FOR)
        });
        if aged && At::new(&entry.file_name().to_string_lossy(), None).is_some() {
            let _ = fs::remove_file(entry.path());
        }
    }
}

/// Answer each asking at `listener` in turn, keeping in `folder`, until
/// none comes for `idle`.
pub fn serve(listener: &UnixListener, folder: &Path, idle: Duration) {
    let mut waiting = libc::pollfd {
        fd: listener.as_raw_fd(),
        events: libc::POLLIN,
        revents: 0,
    };
    let within = i32::try_from(idle.as_millis()).unwrap_or(i32::MAX);
    loop {
        // SAFETY: one pollfd, alive for the call, for a listener we hold.
        let ready = unsafe { libc::poll(&raw mut waiting, 1, within) };
        if ready == 0 {
            return;
        }
        if ready < 0 {
            if io::Error::last_os_error().kind() == io::ErrorKind::Interrupted {
                continue;
            }
            return;
        }
        if let Ok((asking, _)) = listener.accept() {
            let _ = asked_of(asking, folder);
        }
    }
}

/// Hear one asking out, do it, and say how it went.
fn asked_of(mut asking: UnixStream, folder: &Path) -> io::Result<()> {
    let bound = usize::try_from(RECORD_AT_MOST).unwrap_or(usize::MAX) + 1024;
    let done = said_within(&mut asking, HEARD_WITHIN, bound)
        .and_then(|text| heard(&text))
        .and_then(|asked| answer(folder, &asked));
    let said = match done {
        Ok(()) => "ok\n".to_owned(),
        Err(why) => format!("refused: {why}\n"),
    };
    asking.write_all(said.as_bytes())
}

/// All `asking` says, within `within` of its start in all and at most
/// `bound` bytes, so one slow asker holds the keeper no longer than that.
fn said_within(asking: &mut UnixStream, within: Duration, bound: usize) -> Result<String, String> {
    let deadline = Instant::now() + within;
    let (mut said, mut read) = (Vec::new(), [0; 16 * 1024]);
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return Err(format!("said nothing whole within {within:?}"));
        }
        asking
            .set_read_timeout(Some(left))
            .map_err(|error| error.to_string())?;
        match asking.read(&mut read) {
            Ok(0) => break,
            Ok(count) => said.extend_from_slice(&read[..count]),
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {}
            Err(error) if error.kind() == io::ErrorKind::TimedOut => {}
            Err(error) => return Err(error.to_string()),
        }
        if said.len() > bound {
            return Err("asks too much at once".to_owned());
        }
    }
    String::from_utf8(said).map_err(|_| "not text".to_owned())
}

/// Ask the keeper at `socket` for `asked`, a JSON object as `heard` reads.
pub fn ask(socket: &Path, asked: &serde_json::Value) -> Result<(), String> {
    let mut keeper = UnixStream::connect(socket)
        .map_err(|error| format!("couldn't reach {}: {error}", socket.display()))?;
    keeper
        .write_all(asked.to_string().as_bytes())
        .and_then(|()| keeper.shutdown(std::net::Shutdown::Write))
        .map_err(|error| error.to_string())?;
    let mut said = String::new();
    keeper
        .read_to_string(&mut said)
        .map_err(|error| error.to_string())?;
    match said.trim_end() {
        "ok" => Ok(()),
        "" => Err("the keeper said nothing".to_owned()),
        refused => Err(refused.to_owned()),
    }
}

/// `nix-offload keep --folder DIR`, as its socket starts it: age out what's
/// old, then answer until idle.
pub fn keeping(folder: Option<String>) -> ExitCode {
    let Some(folder) = folder else {
        eprintln!("nix-offload: keep needs --folder");
        return ExitCode::from(64);
    };
    let ours = std::env::var("LISTEN_PID").ok() == Some(std::process::id().to_string());
    if !ours || std::env::var("LISTEN_FDS").ok().as_deref() != Some("1") {
        eprintln!("nix-offload: keep is started by its socket, which gave it none");
        return ExitCode::from(64);
    }
    // SAFETY: systemd hands the socket's one listener over as descriptor
    // 3, ours alone, as LISTEN_PID and LISTEN_FDS say.
    let listener = unsafe { UnixListener::from_raw_fd(3) };
    let folder = Path::new(&folder);
    age_out(folder, SystemTime::now());
    serve(&listener, folder, IDLE);
    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests;
