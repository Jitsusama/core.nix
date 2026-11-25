//! Asking the machine's mycelium within a run's time. Each command has its
//! own limit, and a run's sends end by `SENDS_END`, so a node slow to
//! answer, as one is while a switch restarts it, leaves what's left for the
//! next run rather than holding this one past its unit's limit.

use std::io::Read;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use super::Put;

/// The longest a send may take: the node answers in well under a second,
/// and a run every five minutes must end long before the next begins.
pub const SEND_AT_MOST: Duration = Duration::from_secs(20);

/// How far into a run its sends may go: a send starts only while it could
/// end by then, a minute short of the four minutes pain-hear's unit allows
/// in home.nix, so what a run writes after its sends has its time.
const SENDS_END: Duration = Duration::from_secs(180);

/// How a send came out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sent {
    Went,
    /// Tried and not taken, which spends one of the pain's tries.
    Failed,
    /// Not tried this run, its tries kept: the node had stopped answering,
    /// or no send could end by `SENDS_END`.
    Held,
}

/// The machine's mycelium, sending as `from`. Once one send outruns its
/// limit the node isn't answering, so the rest of the run's are held.
pub struct Node {
    mycelium: PathBuf,
    from: Option<String>,
    begun: Instant,
    limit: Duration,
    silent: bool,
    /// How many sends the run held for the next.
    pub held: usize,
}

impl Node {
    /// The node at `mycelium`, for a run begun at `begun`, each send given
    /// `limit`.
    pub fn new(mycelium: PathBuf, from: Option<String>, begun: Instant, limit: Duration) -> Self {
        Self {
            mycelium,
            from,
            begun,
            limit,
            silent: false,
            held: 0,
        }
    }

    /// The sessions the node knows of, as it prints them, or none when it
    /// printed nothing within its limit; one that ran past it is stalled,
    /// as a node serving while a switch holds the disk has been, so the
    /// run's sends wait for the next run.
    pub fn sessions(&mut self) -> String {
        let mut asking = Command::new(&self.mycelium);
        asking.arg("sessions");
        answer_within(&mut asking, self.limit)
            .unwrap_or_else(|Late| {
                self.silent = true;
                None
            })
            .unwrap_or_default()
    }

    /// Sends `text` to `to`: an owner is asked, as it must say when its fix
    /// lands; the keeper is told, with an fyi, of what its list leaves to
    /// their owners, so it needn't answer.
    pub fn send(&mut self, to: &str, text: &str, put: Put) -> Sent {
        if self.silent || self.begun.elapsed() + self.limit > SENDS_END {
            self.held += 1;
            return Sent::Held;
        }
        let Some(from) = self.from.as_deref() else {
            return Sent::Failed;
        };
        let intent = match put {
            Put::Asked => "question",
            Put::Told => "fyi",
        };
        let mut sending = Command::new(&self.mycelium);
        sending
            .args(["send", "--from", from, "--intent", intent, "--text", text])
            .arg(format!("peer.{}", to.to_lowercase()));
        match answer_within(&mut sending, self.limit) {
            Ok(answer) if answer.as_deref().is_some_and(node_took) => Sent::Went,
            Ok(_) => Sent::Failed,
            // It was tried, so it spends a try as any failed send does.
            Err(Late) => {
                self.silent = true;
                Sent::Failed
            }
        }
    }
}

/// Whether the node's answer to a send says it took it: the mycelium
/// exits well on a send it refused, and says so only in its answer.
pub fn node_took(answer: &str) -> bool {
    answer.trim_start().starts_with(r#"{"Ok""#)
}

/// A command still running at its limit, and so stopped.
#[derive(Debug, PartialEq, Eq)]
pub struct Late;

/// What a command printed, when it ended within `limit`.
pub fn output_within(command: &mut Command, limit: Duration) -> Option<String> {
    answer_within(command, limit).ok().flatten()
}

/// What a command printed, if it ended well and printed anything, or
/// `Late` when it ran past `limit`.
fn answer_within(command: &mut Command, limit: Duration) -> Result<Option<String>, Late> {
    let Ok(mut child) = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
    else {
        return Ok(None);
    };
    let Some(mut out) = child.stdout.take() else {
        return Ok(None);
    };
    // Read beside the wait, so a command printing more than a pipe holds
    // isn't stopped waiting on its reader; and read with the same limit,
    // as a child it left behind may hold the pipe open long after.
    let started = Instant::now();
    let (read, reading) = mpsc::channel();
    std::thread::spawn(move || {
        let mut text = String::new();
        let _ = read.send(out.read_to_string(&mut text).ok().map(|_| text));
    });
    match ended_within(&mut child, limit) {
        None => Err(Late),
        Some(false) => Ok(None),
        Some(true) => reading
            .recv_timeout(limit.saturating_sub(started.elapsed()))
            .map_err(|_| Late)
            .map(|text| text.filter(|s| !s.is_empty())),
    }
}

/// Whether a child ended well within `limit`, or `None` when it was still
/// running then, and so killed.
fn ended_within(child: &mut Child, limit: Duration) -> Option<bool> {
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Some(status.success()),
            Ok(None) if started.elapsed() < limit => std::thread::sleep(Duration::from_millis(50)),
            running => {
                let _ = child.kill();
                let _ = child.wait();
                return running.is_err().then_some(false);
            }
        }
    }
}
