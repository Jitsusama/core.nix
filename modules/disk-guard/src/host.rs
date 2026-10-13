//! The guard's hold on this machine: what its processes are doing, read
//! from `/proc`, its filesystems, and reclaiming what a survey offered.

use crate::activity::{self, Process, Use};
use crate::anchor::Dir;
use crate::band::{Band, Space};
use crate::commands::{self, Job, Output, Ran};
use crate::pass::{Disk, Outcome, Reclaimer};
use crate::plan::Candidate;
use crate::reclaim::{self, Looker};
use crate::survey::{Action, Offer};
use crate::vouch::{self, Answer};
use crate::{garbage, logs, socket, workspaces};
use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

/// How long a scan of every process is trusted for. One takes about
/// 117 ms on a desktop of 285 processes, so judging thousands of entries
/// one scan each would take minutes.
pub const FRESH: Duration = Duration::from_secs(1);

/// Every process as last scanned, read again once something has been
/// moved aside or the scan is older than its freshness: the judgement
/// after a move has to see a process that arrived just before it.
pub struct Scan {
    proc: PathBuf,
    locks: PathBuf,
    me: u32,
    fresh: Duration,
    seen: Option<(Instant, Vec<Process>)>,
    voucher: Option<Voucher>,
}

/// The system daemon, asked about the processes this account can't read.
struct Voucher {
    socket: PathBuf,
    roots: Vec<PathBuf>,
    /// The last answer, or the failure to get one, and when.
    heard: Option<(Instant, Result<Answer, String>)>,
}

/// How long to wait for the system daemon, which scans every process to
/// answer.
const ASK_TIMEOUT: Duration = Duration::from_secs(2);

/// How long to go without asking a system daemon that didn't answer, so
/// one that hangs costs a pass two seconds now and then, not every scan.
const ASK_AGAIN: Duration = Duration::from_secs(30);

impl Scan {
    pub fn new(proc: &Path, locks: &Path, me: u32, fresh: Duration) -> Self {
        Scan {
            proc: proc.to_owned(),
            locks: locks.to_owned(),
            me,
            fresh,
            seen: None,
            voucher: None,
        }
    }

    /// Asks the system daemon at `socket` about the processes this account
    /// can't read, for what they touch beneath `roots`. Its answer is kept
    /// as long as a scan is, even across a move: a process started since
    /// has a pid and start it didn't vouch for, and stays unknown.
    pub fn vouched_by(mut self, socket: &Path, roots: &[PathBuf]) -> Self {
        self.voucher = Some(Voucher {
            socket: socket.to_owned(),
            roots: roots.to_vec(),
            heard: None,
        });
        self
    }

    /// Why the system daemon last couldn't be asked, if it couldn't.
    pub fn unheard(&self) -> Option<&str> {
        match &self.voucher.as_ref()?.heard {
            Some((_, Err(why))) => Some(why),
            _ => None,
        }
    }

    fn vouched(&mut self, local: Vec<Process>) -> Vec<Process> {
        let fresh = self.fresh;
        let Some(voucher) = &mut self.voucher else {
            return local;
        };
        if local.iter().all(|process| process.readable) {
            return local;
        }
        let current = match &voucher.heard {
            Some((at, Ok(_))) => at.elapsed() < fresh,
            Some((at, Err(_))) => at.elapsed() < ASK_AGAIN,
            None => false,
        };
        if !current {
            let request = socket::Request::Processes {
                roots: voucher.roots.clone(),
            };
            let heard = match socket::ask(&voucher.socket, &request, ASK_TIMEOUT) {
                Ok(socket::Reply::Answer(answer)) => Ok(answer),
                Ok(other) => Err(format!("the system daemon answered {other:?}")),
                Err(error) => Err(error.to_string()),
            };
            voucher.heard = Some((Instant::now(), heard));
        }
        match &voucher.heard {
            Some((_, Ok(answer))) => vouch::merge(local, answer),
            _ => local,
        }
    }

    pub fn processes(&mut self) -> io::Result<&[Process]> {
        let stale = match &self.seen {
            Some((taken, _)) => taken.elapsed() >= self.fresh,
            None => true,
        };
        if stale {
            let taken = Instant::now();
            let local = crate::linux::processes(&self.proc)?;
            self.seen = Some((taken, self.vouched(local)));
        }
        Ok(self.seen.as_ref().map_or(&[], |(_, processes)| processes))
    }
}

impl Looker for Scan {
    fn judge(&mut self, target: &Path, checkout: Option<&Path>) -> io::Result<Use> {
        let me = self.me;
        Ok(activity::judge(self.processes()?, me, target, checkout))
    }

    fn moved(&mut self) {
        self.seen = None;
    }

    fn waiters(&mut self, inodes: &[u64]) -> io::Result<Vec<i32>> {
        crate::linux::lock_waiters(&self.locks, self.me, inodes)
    }
}

/// One filesystem, measured and settled through a path on it.
pub struct Filesystem {
    path: PathBuf,
    started: Instant,
}

impl Filesystem {
    pub fn new(path: &Path) -> Self {
        Filesystem {
            path: path.to_owned(),
            started: Instant::now(),
        }
    }
}

impl Disk for Filesystem {
    fn space(&mut self) -> io::Result<Space> {
        crate::linux::space(&self.path)
    }

    fn settle(&mut self) -> io::Result<()> {
        crate::linux::settle(&self.path)
    }

    fn now(&self) -> f64 {
        self.started.elapsed().as_secs_f64()
    }
}

/// Reclaims what a survey offered, each as its action says, anchored to
/// the root it was found beneath. Anything no offer names is refused.
pub struct Reclaim<'a> {
    actions: HashMap<PathBuf, Action>,
    scan: &'a mut Scan,
    notes: Vec<String>,
    gentle: bool,
    band: Band,
}

/// How long the system daemon may take over the garbage a user daemon asks
/// it to collect: a pass of many batches, each measured.
const COLLECT_TIMEOUT: Duration = Duration::from_secs(30 * 60);

impl<'a> Reclaim<'a> {
    pub fn new(offers: &[Offer], scan: &'a mut Scan) -> Self {
        let actions = offers
            .iter()
            .map(|offer| (offer.candidate.path.clone(), offer.action.clone()))
            .collect();
        Reclaim {
            actions,
            scan,
            notes: Vec::new(),
            gentle: false,
            band: Band::Healthy,
        }
    }

    /// For a pass in `band`, which what it asks of the system daemon says.
    pub fn during(mut self, band: Band) -> Self {
        self.band = band;
        self
    }

    /// For a pass that can wait: the commands it runs do so at idle
    /// priority, so a collection never slows the work it makes room for.
    pub fn gently(mut self) -> Self {
        self.gentle = true;
        self
    }

    /// What went wrong after a reclaim that freed its space anyway.
    pub fn notes(&self) -> &[String] {
        &self.notes
    }
}

impl Reclaimer for Reclaim<'_> {
    fn reclaim(&mut self, candidate: &Candidate) -> Outcome {
        let Some(action) = self.actions.get(&candidate.path) else {
            return Outcome::Failed("no source offered it".to_owned());
        };
        let root_path = action.root();
        let root = match Dir::open(root_path) {
            Ok(root) => root,
            Err(error) => {
                return Outcome::Failed(format!("opening {}: {error}", root_path.display()));
            }
        };
        match action {
            Action::Target { .. } => {
                let checkout = candidate.checkout.clone();
                reclaim::target(&root, root_path, candidate, checkout.as_deref(), self.scan)
            }
            Action::Variant { .. } => {
                reclaim::variant(&root, root_path, candidate, SystemTime::now())
            }
            Action::Build { target, .. } => reclaim::build(&root, root_path, target, candidate),
            Action::Entry { .. } => reclaim::entry(&root, root_path, candidate, self.scan),
            Action::Trash { .. } => reclaim::trashed(&root, root_path, candidate),
            Action::Run { argv, timeout, .. } => run(argv, *timeout, self.gentle),
            // Nix judges liveness itself, under its own lock.
            // What it freed is measured, like any command's.
            Action::Ask { socket, .. } => {
                let collect = socket::Request::Collect {
                    path: root_path.to_owned(),
                    band: self.band,
                };
                match socket::ask(socket, &collect, COLLECT_TIMEOUT) {
                    Ok(socket::Reply::Collected(_)) => Outcome::Reclaimed,
                    Ok(other) => Outcome::Failed(format!("the system daemon answered {other:?}")),
                    Err(error) => Outcome::Failed(format!("asking the system daemon: {error}")),
                }
            }
            Action::Expire {
                nix,
                profile,
                numbers,
                ..
            } => match crate::generations::expire(nix, profile, numbers, self.gentle) {
                Ok(()) => Outcome::Reclaimed,
                Err(why) => Outcome::Failed(why),
            },
            Action::Collect { nix, paths, .. } => match garbage::delete(nix, paths, self.gentle) {
                Ok(()) => Outcome::Reclaimed,
                Err(why) => Outcome::Failed(why),
            },
            Action::Trim { .. } => {
                let Ok(relative) = candidate.path.strip_prefix(root_path) else {
                    return Outcome::Failed(format!("outside its root {}", root_path.display()));
                };
                // Its writer carries on appending to what's kept, so its use
                // judges nothing.
                match logs::trim(&root, relative, logs::KEEP) {
                    Ok(_) => Outcome::Reclaimed,
                    Err(error) => Outcome::Failed(format!("trimming it: {error}")),
                }
            }
            Action::Workspace { .. } => {
                // Read before it goes: its .git files name the repositories.
                let linked = workspaces::linked(&candidate.path);
                let outcome = reclaim::entry(&root, root_path, candidate, self.scan);
                if outcome == Outcome::Reclaimed {
                    for common in linked {
                        if let Err(error) = workspaces::prune(&common) {
                            self.notes
                                .push(format!("pruning {}: {error}", common.display()));
                        }
                    }
                }
                outcome
            }
        }
    }
}

/// How much of a command's output is kept, of which its last line of
/// errors is told.
const OUTPUT_LIMIT: usize = 64 << 10;

/// Runs a source's command; anything but a clean exit is a failure, told
/// with the last thing it said on its errors.
fn run(argv: &[String], timeout: Duration, idle: bool) -> Outcome {
    let job = Job {
        argv: argv.iter().map(Into::into).collect(),
        cwd: None,
        env: Vec::new(),
        timeout,
        limit: OUTPUT_LIMIT,
        idle,
    };
    match commands::run(&job) {
        Ok(Ran::Finished(Output {
            status: Some(0), ..
        })) => Outcome::Reclaimed,
        Ok(Ran::Finished(output)) => {
            let said = String::from_utf8_lossy(&output.stderr);
            let said = said.lines().rev().find(|line| !line.trim().is_empty());
            let how = match output.status {
                Some(code) => format!("it exited {code}"),
                None => "a signal ended it".to_owned(),
            };
            Outcome::Failed(match said {
                Some(said) => format!("{how}: {}", said.trim()),
                None => how,
            })
        }
        Ok(Ran::TimedOut | Ran::Stopped) => {
            Outcome::Failed(format!("it ran past its {timeout:?} and was killed"))
        }
        Err(error) => Outcome::Failed(format!("starting it: {error}")),
    }
}
