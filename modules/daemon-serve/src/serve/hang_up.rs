//! A connection that stops reading: what can't be queued for its writer
//! and why, the hang-up on it, and the one line on standard error saying
//! so, for the daemon's journal, where before it was shut without a word.
//! One whose writer had ended went of its own accord, and is shut without
//! one.

use std::io::Write;
use std::net::Shutdown;
use std::os::fd::AsRawFd;
use std::os::unix::net::UnixStream;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{self, SyncSender, TrySendError};

use serde_json::Value;

use super::{Held, LONGEST, QUEUED, Shared};

/// How many hang-up lines may wait for standard error before more are
/// dropped, so a stalled journal neither holds the daemon up nor fills it.
pub(super) const UNSAID: usize = 64;

/// Why a line couldn't be queued for a connection's writer: its queue was
/// full, as it stopped reading, or its writer had ended, as it went.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Unqueued {
    Unread,
    Gone,
}

/// Queues `line` for the writer `told` feeds, or says why it couldn't.
pub(super) fn queue(told: &SyncSender<String>, line: &Value) -> Result<(), Unqueued> {
    told.try_send(line.to_string()).map_err(|why| match why {
        TrySendError::Full(_) => Unqueued::Unread,
        TrySendError::Disconnected(_) => Unqueued::Gone,
    })
}

/// A connection as its hang-up names it: its number, the process that
/// connected it as the kernel gave it, and whether its hang-up was said,
/// one flag for every path that may find it stopped reading.
pub(super) struct Peer {
    pub(super) connection: u64,
    /// The process, by its pid and, where it could be read, its name, read
    /// just after it connected: a pid alone may be gone, or another's, by
    /// the time the journal is read. It names another only where the one
    /// that connected went first, or handed its connection on.
    process: String,
    said: AtomicBool,
}

impl Peer {
    pub(super) fn of(connection: u64, stream: &UnixStream) -> Self {
        let pid = pid_of(stream);
        let name = pid.and_then(|pid| std::fs::read_to_string(format!("/proc/{pid}/comm")).ok());
        Peer {
            connection,
            process: process(pid, name.as_deref()),
            said: AtomicBool::new(false),
        }
    }
}

/// A process as a hang-up names it, by `pid` and `name` where each could
/// be read. A name is the process's own to set, so what in it isn't
/// printable is escaped, keeping a hang-up to the one line it's said on.
pub(super) fn process(pid: Option<u32>, name: Option<&str>) -> String {
    match (pid, name) {
        (Some(pid), Some(name)) => format!("pid {pid}, {}", name.trim_end().escape_debug()),
        (Some(pid), None) => format!("pid {pid}"),
        (None, _) => "an unknown process".to_owned(),
    }
}

/// The process at the other end of `stream`, as the kernel took it as it
/// connected (`SO_PEERCRED`), or none where it couldn't say.
pub(super) fn pid_of(stream: &UnixStream) -> Option<u32> {
    const SOL_SOCKET: i32 = 1;
    const SO_PEERCRED: i32 = 17;
    let mut said = [0u32; 3];
    let mut length = 12u32;
    // SAFETY: SO_PEERCRED fills three u32s (pid, uid, gid) into a buffer
    // of that size, which `length` says.
    let read = unsafe {
        getsockopt(
            stream.as_raw_fd(),
            SOL_SOCKET,
            SO_PEERCRED,
            said.as_mut_ptr().cast(),
            &raw mut length,
        )
    };
    (read == 0 && length == 12).then_some(said[0])
}

unsafe extern "C" {
    fn getsockopt(
        fd: i32,
        level: i32,
        name: i32,
        value: *mut std::ffi::c_void,
        length: *mut u32,
    ) -> i32;
}

/// What writes hang-up lines on a thread of its own, started with the
/// daemon rather than at its first hang-up, which may come under its lock.
/// A line is dropped when UNSAID already wait, and counted, and the count
/// said after the next line written, so saying never waits on the journal,
/// a write that fails ends nothing, and a gap still shows, though among
/// the lines that waited rather than after them.
pub(super) struct Saying {
    lines: SyncSender<String>,
    unsaid: Arc<AtomicUsize>,
}

impl Saying {
    /// Writes to `out` for the daemon `name`s; with no thread to be had,
    /// that's said once on standard error and every line counted unsaid,
    /// and nothing else changes.
    pub(super) fn to(name: &'static str, mut out: impl Write + Send + 'static) -> Self {
        let (lines, heard) = mpsc::sync_channel::<String>(UNSAID);
        let unsaid = Arc::new(AtomicUsize::new(0));
        let missed = Arc::clone(&unsaid);
        let started = std::thread::Builder::new()
            .name("hang-ups".to_owned())
            .spawn(move || {
                for line in heard {
                    let _ = writeln!(out, "{line}");
                    let missed = missed.swap(0, Ordering::Relaxed);
                    if missed > 0 {
                        let _ = writeln!(
                            out,
                            "{name}: {missed} more hang-ups went unsaid while the journal was \
                             behind"
                        );
                    }
                }
            });
        if started.is_err() {
            let _ = writeln!(
                std::io::stderr(),
                "{name}: hang-ups go unsaid, as no thread could be had to say them"
            );
        }
        Saying { lines, unsaid }
    }

    pub(super) fn say(&self, line: String) {
        if self.lines.try_send(line).is_err() {
            self.unsaid.fetch_add(1, Ordering::Relaxed);
        }
    }
}

impl Held {
    /// Hangs up on these connections, which stopped reading, and drops
    /// their watches, answering whether any went.
    fn hang_up(&mut self, stuck: &[u64]) -> bool {
        for watch in &self.watches {
            if stuck.contains(&watch.peer.connection) {
                let _ = watch.stream.shutdown(Shutdown::Both);
            }
        }
        self.drop_watches(|watch| stuck.contains(&watch.peer.connection))
    }
}

impl Shared {
    /// Writes each hang-up's line to `out` rather than standard error.
    #[cfg(test)]
    pub(super) fn hear_hang_ups(&self, out: impl Write + Send + 'static) {
        let _ = self.said.set(Saying::to(self.domain.name, out));
    }

    /// Says that `peer` was hung up on, which couldn't be told a line as
    /// `how` for `why`, unless its writer had ended or it was said
    /// already, so a journal shows which daemon shut which process and why.
    pub(super) fn say_once(&self, peer: &Peer, why: Unqueued, how: &str) {
        if why == Unqueued::Unread {
            self.say(peer, &format!("which left {QUEUED} lines unread as {how}"));
        }
    }

    /// Says that `peer` was hung up on for an ask that ran past what a
    /// line may hold, ended or not, unless its hang-up was said already.
    pub(super) fn say_overlong(&self, peer: &Peer) {
        let most = LONGEST >> 20;
        self.say(
            peer,
            &format!("whose ask ran past the {most} MiB a line may hold"),
        );
    }

    /// Says that `peer` was hung up on, `why`, once a connection.
    fn say(&self, peer: &Peer, why: &str) {
        if peer.said.swap(true, Ordering::Relaxed) {
            return;
        }
        let line = format!(
            "{}: hung up on connection {} ({}), {why}",
            self.domain.name, peer.connection, peer.process
        );
        self.said.get().unwrap_or(&self.saying).say(line);
    }

    /// Hangs up on the watches' connections in `stuck`, saying each that
    /// stopped reading, as `how` it was told.
    pub(super) fn hang_up(&self, held: &mut Held, stuck: &[(Arc<Peer>, Unqueued)], how: &str) {
        for (peer, why) in stuck {
            self.say_once(peer, *why, how);
        }
        let connections: Vec<u64> = stuck.iter().map(|(peer, _)| peer.connection).collect();
        self.heeded(held.hang_up(&connections));
    }
}
