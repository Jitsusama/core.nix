//! Whether ssh reaches a machine nearby, where nothing listens at its
//! address: asked of ssh in a process group of its own, and what it said
//! kept a minute for every run.

use super::answers;
use std::fs;
use std::io::{BufRead, BufReader, Read};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant};

/// Whether a machine nearby answers: something listens at its address,
/// or else ssh reaches it as `host`. A name that doesn't resolve here may
/// still be reached by ssh's own entry for it, as a Mac's `.local` name is
/// through the last address it answered at: from 07:01 on 22 November its
/// name went unanswered for hours while ssh reached it, and every build
/// for it failed as away.
pub fn nearby_answers(address: &str, host: &str) -> bool {
    answers(address) || ssh_reaches_lately(host)
}

/// How long what ssh said of a machine nearby is believed before it's
/// asked again, so a run asking of one Mac several times, and the runs of
/// this program it starts, wait on ssh once, while a lender running for
/// days still sees one come and go.
const REACHED_FOR: Duration = Duration::from_secs(60);

/// What ssh last said of each machine nearby, and when, in this run: read
/// before the runtime folder's, and the one kept where that folder can't
/// be written or there's none.
static REACHED: Memo = Mutex::new(Vec::new());

/// What ssh said of each machine nearby within a run, and when.
type Memo = Mutex<Vec<(String, Instant, Said)>>;

/// What ssh said of a machine nearby: whether it reaches it, and, where it
/// doesn't, the line said then, said again while it's believed, so a
/// refused host key isn't heard of only by the run that asked.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Said {
    reaches: bool,
    why: String,
}

impl Said {
    /// Whether it reaches, as said `age` ago, saying again why not.
    fn recalled(&self, age: Duration) -> bool {
        if !self.reaches {
            eprintln!(
                "nix-offload: {} (as ssh said {} s ago)",
                self.why,
                age.as_secs()
            );
        }
        self.reaches
    }
}

/// The folder what ssh said of machines nearby is kept in, a file each,
/// for every run of this program: the account's runtime folder's, which
/// only it may write. With none, each run keeps its own, since the temp
/// folder is every account's, and one could say a machine answers.
fn reached_folder() -> Option<PathBuf> {
    std::env::var_os("XDG_RUNTIME_DIR")
        .map(|runtime| PathBuf::from(runtime).join("nix-offload-reached"))
}

/// Whether ssh reaches `host`, as it said within [`REACHED_FOR`] or as it
/// says now, saying each time it's asked, since asking can take seconds.
fn ssh_reaches_lately(host: &str) -> bool {
    reached_lately(&REACHED, reached_folder().as_deref(), host, || {
        eprintln!(
            "nix-offload: nothing listens at {host}'s address, so asking ssh whether it reaches it"
        );
        let why = match ssh_reaches("ssh", host, REACHING) {
            Reached::Ran => {
                return Said {
                    reaches: true,
                    why: String::new(),
                };
            }
            Reached::Refused(said) => {
                format!("warning: ssh refused {host}'s host key, so it's taken as away: {said}")
            }
            Reached::Away(said) => format!("ssh doesn't reach {host} either: {said}"),
        };
        eprintln!("nix-offload: {why}");
        Said {
            reaches: false,
            why,
        }
    })
}

/// What `ask` says of `host`, or what it said within [`REACHED_FOR`]: as
/// this run's `memo` holds it, or else as `folder` does, a file for each
/// machine, for the runs this one starts. An answer asked for goes into
/// both, so a folder that can't be written still leaves this run asking
/// once.
fn reached_lately(
    memo: &Memo,
    folder: Option<&Path>,
    host: &str,
    ask: impl FnOnce() -> Said,
) -> bool {
    // Held while ssh is asked, so this run's threads ask of a machine once.
    let mut memo = memo
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    memo.retain(|(_, at, _)| at.elapsed() < REACHED_FOR);
    if let Some((_, at, said)) = memo.iter().find(|(name, _, _)| name == host) {
        return said.recalled(at.elapsed());
    }
    if let Some((said, age)) = folder.and_then(|folder| kept(folder, host)) {
        let at = Instant::now().checked_sub(age).unwrap_or_else(Instant::now);
        let reaches = said.recalled(age);
        memo.push((host.to_owned(), at, said));
        return reaches;
    }
    let said = ask();
    if let Some(folder) = folder {
        keep(folder, host, &said);
    }
    let reaches = said.reaches;
    memo.push((host.to_owned(), Instant::now(), said));
    reaches
}

/// What `folder` says ssh said of `host` within [`REACHED_FOR`], and how
/// long ago: a first line `reaches` or `away`, and for `away` the line
/// said then. Anything else reads as nothing kept, so it's asked again.
fn kept(folder: &Path, host: &str) -> Option<(Said, Duration)> {
    let file = folder.join(reached_name(host));
    let age = fs::metadata(&file).ok()?.modified().ok()?.elapsed().ok()?;
    if age >= REACHED_FOR {
        return None;
    }
    let text = fs::read_to_string(&file).ok()?;
    let mut lines = text.lines();
    let said = match lines.next()? {
        "reaches" => Said {
            reaches: true,
            why: String::new(),
        },
        "away" => Said {
            reaches: false,
            why: lines
                .next()
                .unwrap_or("ssh didn't reach it")
                .chars()
                .map(|c| if c.is_control() { ' ' } else { c })
                .collect(),
        },
        _ => return None,
    };
    Some((said, age))
}

/// Keeps what ssh said of `host` in `folder`, written beside it under a
/// name of this run's own and then named, so no reader finds half of it
/// and no two runs, or two machines, write through one draft. The draft
/// starts with a `.`, which no machine's own file does. Failing leaves
/// this run's memo alone to keep it.
fn keep(folder: &Path, host: &str, said: &Said) {
    let name = reached_name(host);
    let writing = folder.join(format!(".{name}.{}", std::process::id()));
    let text = if said.reaches {
        "reaches\n".to_owned()
    } else {
        format!("away\n{}\n", said.why.replace('\n', " "))
    };
    let kept = fs::create_dir_all(folder)
        .and_then(|()| fs::write(&writing, text))
        .and_then(|()| fs::rename(&writing, folder.join(&name)));
    if kept.is_err() {
        let _ = fs::remove_file(&writing);
    }
}

/// Forgets what ssh said of `host`, so the next ask is answered fresh, as
/// after a comparison or a build there failed, which may be the machine
/// going.
pub fn forget_reached(host: &str) {
    forget_reached_in(&REACHED, reached_folder().as_deref(), host);
}

/// Forgets what `memo` and `folder` hold of `host`.
fn forget_reached_in(memo: &Memo, folder: Option<&Path>, host: &str) {
    memo.lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .retain(|(name, _, _)| name != host);
    if let Some(folder) = folder {
        let _ = fs::remove_file(folder.join(reached_name(host)));
    }
}

/// The file `host` is kept under: its name, with anything but letters,
/// digits, `.`, `-` and `_` as `_`, and a leading `.` too, so no name
/// reaches outside the folder.
fn reached_name(host: &str) -> String {
    host.chars()
        .enumerate()
        .map(|(at, letter)| {
            let kept = letter.is_ascii_alphanumeric()
                || "-_".contains(letter)
                || (letter == '.' && at > 0);
            if kept { letter } else { '_' }
        })
        .collect()
}

/// How long ssh may take to reach a machine nearby. Its ssh entry may wait
/// on the name for three seconds and then on the last address for three
/// more before the handshake, about seven in all on 22 November.
const REACHING: Duration = Duration::from_secs(20);

/// What ssh is told to reach `host` and run `true` there: never asking
/// anything, as every ssh nix-offload starts ([`crate::asking::NEVER_ASKS`]),
/// giving up on a connection within 15 s and on a quiet one within 10,
/// and writing nothing to known hosts, since only reaching is asked. A
/// `host` starting with `-` is a name, never an option.
fn reaching_args(host: &str) -> Vec<String> {
    crate::asking::NEVER_ASKS
        .split_whitespace()
        .chain([
            "-o",
            "ConnectTimeout=15",
            "-o",
            "ServerAliveInterval=5",
            "-o",
            "ServerAliveCountMax=2",
            "-o",
            "UpdateHostKeys=no",
            "-o",
            "CheckHostIP=no",
            "--",
            host,
            "true",
        ])
        .map(str::to_owned)
        .collect()
}

/// What ssh said, asked whether it reaches a machine nearby: that it ran
/// there; that it refused the key the machine showed, which is for Joel
/// to look at rather than a machine away; or that it didn't reach it. The
/// last two carry ssh's last line, or why there's none.
#[derive(Debug, PartialEq, Eq)]
enum Reached {
    Ran,
    Refused(String),
    Away(String),
}

/// What ssh says when the key a machine shows isn't the one held for it.
const KEY_REFUSED: [&str; 2] = [
    "Host key verification failed",
    "REMOTE HOST IDENTIFICATION HAS CHANGED",
];

/// What ssh said on its standard error: its last line saying anything,
/// and whether any line said it refused a host key. Only those are kept,
/// so an ssh saying a great deal holds no more than a line.
#[derive(Default)]
struct Heard {
    last: Option<String>,
    refused: bool,
}

impl Heard {
    fn take(&mut self, line: &str) {
        let line = line.trim();
        if KEY_REFUSED.iter().any(|refused| line.contains(refused)) {
            self.refused = true;
        }
        if !line.is_empty() {
            self.last = Some(line.to_owned());
        }
    }
}

/// How long ssh's standard error may be quiet, once ssh has ended, before
/// what it said is taken as all of it: a proxy it started in a session of
/// its own may hold the pipe open long after.
const QUIET: Duration = Duration::from_millis(250);

/// The most of one line of ssh's that's read, so a line with no end
/// can't fill memory.
const LINE_MOST: u64 = 4096;

/// What `ssh` says of reaching `host` and running `true` there, within
/// `within` ([`REACHING`]). It runs in a process group of its own, which
/// is ended however ssh does, and while ssh is still unreaped, so the
/// group is never another's. A proxy in ssh's own group never outlives it;
/// one in a session of its own may, and is only kept from holding the
/// answer past [`QUIET`].
/// Its standard error is read a line at a time on a thread of its own,
/// so nothing holding it open can hold this past `within`, or past
/// [`QUIET`] once ssh has ended.
fn ssh_reaches(ssh: &str, host: &str, within: Duration) -> Reached {
    let spawned = Command::new(ssh)
        .args(reaching_args(host))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .process_group(0)
        .spawn();
    let mut reaching = match spawned {
        Ok(reaching) => reaching,
        Err(error) => return Reached::Away(format!("couldn't run {ssh}: {error}")),
    };
    let lines = reaching.stderr.take().map(lines_of);
    let deadline = Instant::now() + within;
    let mut heard = Heard::default();
    let ended = loop {
        for line in lines.iter().flat_map(std::sync::mpsc::Receiver::try_iter) {
            heard.take(&line);
        }
        match has_ended(reaching.id()) {
            Ok(true) => break Ok(()),
            Ok(false) if Instant::now() < deadline => thread::sleep(Duration::from_millis(50)),
            Ok(false) => break Err(format!("ssh didn't answer within {} s", within.as_secs())),
            Err(error) => break Err(format!("couldn't wait on ssh: {error}")),
        }
    };
    if let Ok(group) = libc::pid_t::try_from(reaching.id()) {
        // SAFETY: kill only signals the group ssh leads, which is still
        // its own, since ssh isn't reaped until after.
        unsafe {
            libc::kill(-group, libc::SIGKILL);
        }
    }
    let status = reaching.wait();
    if let Err(why) = ended {
        return Reached::Away(why);
    }
    let status = match status {
        Ok(status) if status.success() => return Reached::Ran,
        Ok(status) => status,
        Err(error) => return Reached::Away(format!("couldn't wait on ssh: {error}")),
    };
    if let Some(lines) = &lines {
        let until = deadline.max(Instant::now() + QUIET);
        while let Some(left) = until.checked_duration_since(Instant::now()) {
            match lines.recv_timeout(left.min(QUIET)) {
                Ok(line) => heard.take(&line),
                Err(_) => break,
            }
        }
    }
    let last = heard
        .last
        .unwrap_or_else(|| format!("ssh ended {status} and said nothing"));
    if heard.refused {
        Reached::Refused(last)
    } else {
        Reached::Away(last)
    }
}

/// The lines `stderr` says, as they're said, read on a thread of its own
/// until it closes, each at most [`LINE_MOST`] bytes and read as UTF-8
/// whatever it holds.
fn lines_of(stderr: std::process::ChildStderr) -> std::sync::mpsc::Receiver<String> {
    let (told, heard) = std::sync::mpsc::channel();
    thread::spawn(move || {
        let mut reader = BufReader::new(stderr);
        let mut line = Vec::new();
        while (&mut reader)
            .take(LINE_MOST)
            .read_until(b'\n', &mut line)
            .is_ok_and(|read| read > 0)
        {
            if told
                .send(String::from_utf8_lossy(&line).into_owned())
                .is_err()
            {
                return;
            }
            line.clear();
        }
    });
    heard
}

/// Whether child `pid` has ended, left unreaped so its process group
/// can still be ended as its own.
fn has_ended(pid: u32) -> std::io::Result<bool> {
    loop {
        // SAFETY: a zeroed siginfo_t is valid, and waitid only writes it.
        let mut info: libc::siginfo_t = unsafe { std::mem::zeroed() };
        // SAFETY: waitid with WNOWAIT reads the child's state, reaping nothing.
        let got = unsafe {
            libc::waitid(
                libc::P_PID,
                pid,
                &raw mut info,
                libc::WEXITED | libc::WNOHANG | libc::WNOWAIT,
            )
        };
        if got == 0 {
            // SAFETY: waitid filled info for a child that changed, and left
            // it zeroed when none did, so its pid is set either way. POSIX
            // names a zero pid, not a zero signal, as the sign of no change.
            return Ok(unsafe { info.si_pid() } != 0);
        }
        let error = std::io::Error::last_os_error();
        if error.kind() != std::io::ErrorKind::Interrupted {
            return Err(error);
        }
    }
}

#[cfg(test)]
#[path = "reaching_tests.rs"]
mod tests;
