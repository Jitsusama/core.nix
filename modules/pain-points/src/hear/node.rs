//! Asking the machine's mycelium within a run's time. Each command has its
//! own limit, and a run's sends end by `SENDS_END`, so a node slow to
//! answer, as one is while a switch's writes hold the disk, leaves what's
//! left for the next run rather than holding this one past its unit's
//! limit.

use std::io::{ErrorKind, Read};
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
/// in home.nix, so what a run asks and writes after its sends has its time.
pub const SENDS_END: Duration = Duration::from_secs(180);

/// How long a stopped child is given to end: one held in the kernel, as a
/// read of a disk a switch is writing holds it, is left to end alone.
const STOPPING_AT_MOST: Duration = Duration::from_secs(1);

/// How a send came out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sent {
    Went,
    /// Tried and not taken, which spends one of the pain's tries.
    Failed,
    /// Not tried this run, its tries kept: the node had stopped answering,
    /// had no one to send as, or the cell is one the routes never send to.
    Held,
    /// Not tried this run, its tries kept, as the mycelium couldn't be
    /// started: a trouble of the hour's, as nothing else then says it.
    Unstarted,
    /// Not tried this run, its tries kept, as no send could end by the
    /// run's end for sends: a trouble of the hour's, so a run always too
    /// slow to send is seen to be. Unlike a send that ran past its limit,
    /// which was tried and so failed, this one never started.
    OutOfTime,
}

/// Why the node gives no more answers this run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Silence {
    /// An ask outran its limit.
    Late,
    /// The mycelium couldn't be started, as this error says.
    Unstarted(ErrorKind),
}

/// The machine's mycelium, sending as `from`. Once one ask outruns its
/// limit, or the mycelium can't be started, the node isn't answering, so
/// the rest of the run's sends wait.
pub struct Node {
    mycelium: PathBuf,
    from: Option<String>,
    ends: Instant,
    limit: Duration,
    /// Why the node gives no more answers this run, once it doesn't.
    pub silent: Option<Silence>,
    /// How many sends were held as the node had stopped answering.
    pub held: usize,
    /// How many sends were held as it had no one to send as.
    pub fromless: usize,
}

impl Node {
    /// The node at `mycelium`, for a run whose sends end by `ends`, each
    /// ask given `limit`.
    pub fn new(mycelium: PathBuf, from: Option<String>, ends: Instant, limit: Duration) -> Self {
        Self {
            mycelium,
            from,
            ends,
            limit,
            silent: None,
            held: 0,
            fromless: 0,
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
            .unwrap_or_else(|unanswered| {
                self.silent = Some(match unanswered {
                    Unanswered::Late => Silence::Late,
                    Unanswered::Unstarted(kind) => Silence::Unstarted(kind),
                });
                None
            })
            .unwrap_or_default()
    }

    /// Sends `text` to `to`: an owner is asked, as it must say when its fix
    /// lands; the keeper is told, with an fyi, of what its list leaves to
    /// their owners, so it needn't answer.
    pub fn send(&mut self, to: &str, text: &str, put: Put) -> Sent {
        match self.silent {
            Some(Silence::Late) => {
                self.held += 1;
                return Sent::Held;
            }
            Some(Silence::Unstarted(_)) => return Sent::Unstarted,
            None => {}
        }
        if Instant::now() + self.limit > self.ends {
            return Sent::OutOfTime;
        }
        // Nothing to send as, as when routes.json gained it as the run
        // began: the next run, reading it whole, sends this one, and the
        // journal says this run held it.
        let Some(from) = self.from.as_deref() else {
            self.fromless += 1;
            return Sent::Held;
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
            Err(Unanswered::Late) => {
                self.silent = Some(Silence::Late);
                Sent::Failed
            }
            // Refused for what it was given, not for the mycelium: this
            // text won't start a send on any run, so it spends its tries
            // and is given up, as a send the node refuses is.
            Err(Unanswered::Unstarted(
                ErrorKind::InvalidInput | ErrorKind::ArgumentListTooLong,
            )) => Sent::Failed,
            // Never sent, so it spends no try; one the machine won't start
            // now won't start for the rest of the run either.
            Err(Unanswered::Unstarted(kind)) => {
                self.silent = Some(Silence::Unstarted(kind));
                Sent::Unstarted
            }
        }
    }
}

/// Whether the node's answer to a send says it took it: the mycelium
/// exits well on a send it refused, and says so only in its answer.
pub fn node_took(answer: &str) -> bool {
    answer.trim_start().starts_with(r#"{"Ok""#)
}

/// Why a command gave no answer, beside ending with none.
#[derive(Debug, PartialEq, Eq)]
enum Unanswered {
    /// Still running at its limit, and so stopped.
    Late,
    /// It couldn't be started at all, as this error says.
    Unstarted(ErrorKind),
}

/// What a command printed, when it ended within `limit`.
pub fn output_within(command: &mut Command, limit: Duration) -> Option<String> {
    answer_within(command, limit).ok().flatten()
}

/// What a command printed, if it ended well and printed anything, or why
/// it gave no answer when it ran past `limit` or couldn't be started.
fn answer_within(command: &mut Command, limit: Duration) -> Result<Option<String>, Unanswered> {
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| Unanswered::Unstarted(e.kind()))?;
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
        None => Err(Unanswered::Late),
        Some(false) => Ok(None),
        // One that ended well isn't late, though a child it left holds
        // the pipe: the node answered, so what it said is all there is.
        Some(true) => Ok(reading
            .recv_timeout(limit.saturating_sub(started.elapsed()))
            .ok()
            .flatten()
            .filter(|s| !s.is_empty())),
    }
}

/// Stops a child, and waits a moment for it to end: a kill is never
/// refused, but a child held in the kernel ends only once it is let go.
pub fn stop(child: &mut Child) {
    let _ = child.kill();
    let started = Instant::now();
    while matches!(child.try_wait(), Ok(None)) && started.elapsed() < STOPPING_AT_MOST {
        std::thread::sleep(Duration::from_millis(10));
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
                stop(child);
                return running.is_err().then_some(false);
            }
        }
    }
}
