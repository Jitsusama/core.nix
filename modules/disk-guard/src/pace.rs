//! Deleting at a pace the disk can take, however busy the machine is.
//!
//! Btrfs frees what was unlinked only when its transaction commits, and
//! the commit writes out all of a delete's metadata while saves on the
//! filesystem can wait behind it. On a disk with no I/O scheduler and no
//! I/O cost model, neither an I/O class nor a cgroup weight reaches that
//! write, and the metadata is charged to no cgroup, so the only way to
//! delete without holding up others is to delete less at once. A
//! pass that can wait therefore deletes in batches and rests after each,
//! long enough for the filesystem's own commit to carry the batch, and
//! longer while the disk is strained. It feels the strain with its own
//! probe: a small write beside its state, synced once a second while it
//! rests, the kind of sync an editor or git makes. It never gives up, so a
//! machine that's always busy still gets cleaned, but it stops resting
//! once the disk falls to urgent, where room matters more than pace, or
//! once the guard is asked to stop, which a pass can't keep waiting.

use crate::config::Pacing;
use std::collections::VecDeque;
use std::time::Duration;

/// How often a resting pacer probes the disk and beats.
const LOOK_EVERY: Duration = Duration::from_secs(1);
/// How often a pacer says it's alive, in seconds. The desktop calls a
/// guard silent after two minutes without a heartbeat.
const BEAT_EVERY: f64 = 10.0;
/// How often a pacer waiting on work looks whether it has ended.
const WAIT_LOOK: Duration = Duration::from_millis(100);
/// A sync this slow means the disk is holding writers up.
const SLOW: Duration = Duration::from_millis(100);
/// More than this many slow syncs within `WINDOW` strain the disk, and it
/// stays strained until `WINDOW` passes with none.
const SLOW_SYNCS: usize = 3;
const WINDOW: f64 = 120.0;

/// How the disk has felt to the guard's own syncs lately.
#[derive(Debug, Default)]
pub struct Strain {
    /// When each slow sync within the window came back, oldest first.
    slow: VecDeque<f64>,
    strained: bool,
}

impl Strain {
    /// A sync that came back at `at`, in monotonic seconds, after `took`.
    pub fn felt(&mut self, at: f64, took: Duration) {
        if took >= SLOW {
            self.slow.push_back(at);
        }
        self.forget(at);
    }

    /// Whether the disk is strained at `at`: more than three slow syncs in
    /// two minutes, and from then until two minutes pass without one.
    pub fn strained(&mut self, at: f64) -> bool {
        self.forget(at);
        if self.slow.len() > SLOW_SYNCS {
            self.strained = true;
        } else if self.slow.is_empty() {
            self.strained = false;
        }
        self.strained
    }

    fn forget(&mut self, at: f64) {
        while self.slow.front().is_some_and(|&slow| at - slow > WINDOW) {
            self.slow.pop_front();
        }
    }
}

/// How a pass waits for what it deleted to show as free.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Settling {
    /// Until the filesystem commits in its own time, asking nothing of it,
    /// for a pass that can wait.
    Patient,
    /// Asking the filesystem to commit now, for a pass that can't.
    Hurried,
}

/// The time a pacer rests by.
pub trait Clock {
    /// Monotonic seconds.
    fn now(&self) -> f64;
    fn sleep(&mut self, how_long: Duration);
}

/// A sync the pacer times to feel the disk.
pub trait Probe {
    /// How long one small synced write took, or `None` when it failed.
    fn sync(&mut self) -> Option<Duration>;
}

/// Paces one pass's deletes. Every unlink is counted, and a pass that can
/// wait rests once a batch is done; one that can't only keeps its
/// heartbeat. Before each measurement a pass that can wait settles
/// patiently instead (`Settling::Patient`), which spaces its batches by
/// the filesystem's own commits.
pub struct Pacer {
    pacing: Pacing,
    hurried: bool,
    unlinked: usize,
    strain: Strain,
    clock: Box<dyn Clock>,
    probe: Option<Box<dyn Probe>>,
    /// Says the guard is alive while a pass holds it.
    beat: Box<dyn FnMut()>,
    /// Whether the disk has fallen to urgent.
    pressed: Box<dyn FnMut() -> bool>,
    beaten: f64,
    rested: f64,
}

impl Pacer {
    /// A pacer for a pass that can wait, which rests as `pacing` says.
    pub fn paced(
        pacing: Pacing,
        clock: Box<dyn Clock>,
        probe: Option<Box<dyn Probe>>,
        beat: Box<dyn FnMut()>,
        pressed: Box<dyn FnMut() -> bool>,
    ) -> Pacer {
        let beaten = clock.now();
        Pacer {
            pacing,
            hurried: false,
            unlinked: 0,
            strain: Strain::default(),
            clock,
            probe,
            beat,
            pressed,
            beaten,
            rested: 0.0,
        }
    }

    /// A pacer for a pass that can't wait, which never rests.
    pub fn hurried(clock: Box<dyn Clock>, beat: Box<dyn FnMut()>) -> Pacer {
        let mut pacer = Pacer::paced(Pacing::default(), clock, None, beat, Box::new(|| true));
        pacer.hurried = true;
        pacer
    }

    /// One entry was just unlinked.
    pub fn unlinked(&mut self) {
        self.unlinked += 1;
        if !self.hurried && self.unlinked >= self.pacing.files {
            self.rest();
        } else {
            self.beat_when_due();
        }
    }

    /// Rests after a batch: for the pacing's rest, then for as long as
    /// the disk is strained, unless the disk falls to urgent or a stop
    /// comes meanwhile, after which the pacer never rests again. A stop
    /// waits, blocked, for the guard to take it once the pass ends, and
    /// systemd kills a guard that keeps it waiting past its stop timeout:
    /// a rest on a disk a repack strained kept one resting until it did.
    pub fn rest(&mut self) {
        if self.hurried {
            return;
        }
        self.unlinked = 0;
        let began = self.clock.now();
        let rest = self.pacing.rest.0.as_secs_f64();
        loop {
            let at = self.clock.now();
            if at - began >= rest && !self.strain.strained(at) {
                break;
            }
            if (self.pressed)() || crate::commands::stop_waiting() {
                self.hurried = true;
                break;
            }
            self.clock.sleep(LOOK_EVERY);
            if let Some(took) = self.probe.as_mut().and_then(|probe| probe.sync()) {
                self.strain.felt(self.clock.now(), took);
            }
            self.beat_when_due();
        }
        self.rested += self.clock.now() - began;
    }

    /// Runs `work` on a thread of its own and waits for it, beating while
    /// it waits. A step that blocks, such as a commit waited for, a
    /// command or an answer from the system daemon, otherwise left the
    /// guard silent: a garbage pass of many batches, each waiting out a
    /// Btrfs commit, went minutes without a heartbeat, and the user guard
    /// asking for it went as long.
    ///
    /// A stop waiting ends the wait at the next look, and the work is left
    /// to end with the guard: a stop waited on an answer from the system
    /// daemon for a minute, and the session's shell, which stops and
    /// starts with the guard, was gone for that minute. A command the work
    /// runs is ended by the stop itself.
    ///
    /// # Errors
    ///
    /// `Stopped` when a stop came before the work ended.
    ///
    /// # Panics
    ///
    /// With `work`'s own panic, when it panics.
    pub fn waiting<T: Send + 'static>(
        &mut self,
        work: impl FnOnce() -> T + Send + 'static,
    ) -> Result<T, Stopped> {
        let worker = std::thread::spawn(work);
        while !worker.is_finished() {
            if crate::commands::stop_waiting() {
                return Err(Stopped);
            }
            self.clock.sleep(WAIT_LOOK);
            self.beat_when_due();
        }
        Ok(worker
            .join()
            .unwrap_or_else(|panic| std::panic::resume_unwind(panic)))
    }

    /// The seconds this pacer has rested.
    pub fn rested(&self) -> f64 {
        self.rested
    }

    fn beat_when_due(&mut self) {
        let now = self.clock.now();
        if now - self.beaten >= BEAT_EVERY {
            (self.beat)();
            self.beaten = now;
        }
    }
}

/// Work a pacer stopped waiting for, since the guard is stopping.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stopped;

/// The machine's own monotonic clock.
pub struct Monotonic(std::time::Instant);

impl Monotonic {
    pub fn new() -> Monotonic {
        Monotonic(std::time::Instant::now())
    }
}

impl Default for Monotonic {
    fn default() -> Monotonic {
        Monotonic::new()
    }
}

impl Clock for Monotonic {
    fn now(&self) -> f64 {
        self.0.elapsed().as_secs_f64()
    }

    fn sleep(&mut self, how_long: Duration) {
        std::thread::sleep(how_long);
    }
}

/// A probe file in a directory on the filesystem being reclaimed, rewritten
/// in place and synced, so it never grows and syncs only its own data.
pub struct SyncedWrite(std::fs::File);

/// What each probe writes: one page.
const PAGE: [u8; 4096] = [0; 4096];

impl SyncedWrite {
    /// Opens or makes the probe file `name` in `dir`.
    pub fn open(dir: &std::path::Path, name: &str) -> std::io::Result<SyncedWrite> {
        let file = std::fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(dir.join(name))?;
        Ok(SyncedWrite(file))
    }
}

impl Probe for SyncedWrite {
    fn sync(&mut self) -> Option<Duration> {
        use std::os::unix::fs::FileExt;
        let began = std::time::Instant::now();
        self.0.write_all_at(&PAGE, 0).ok()?;
        self.0.sync_data().ok()?;
        Some(began.elapsed())
    }
}
