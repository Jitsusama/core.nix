//! The guard's loop, one step at a time: read the clocks, band and
//! forecast every filesystem a source lies on, and run a pass on each one
//! that needs it, journalling what it did and keeping what it learned.

mod asked;
mod hearing;
mod nix_garbage;
mod passes;
mod runaways;
mod status;
mod steps;
mod surveying;

use crate::band::{Band, Space};
use crate::config::{Config, Source};
use crate::decide::{Decision, Fill};
use crate::disk_reads::Walked;
use crate::finished::{Answers, Predicate};
use crate::garbage::{Listed, Listing};
use crate::journal::Journal;
use crate::moments::{Power, Pressure};
use crate::pass::{End, Goal};
use crate::plan::Candidate;
use crate::replaced::Uses;
use crate::socket::Responder;
use crate::state;
use crate::state::Memory;
use crate::writers::Writers;
pub use hearing::LISTEN_TRIES;
pub use status::{
    ClearedStatus, FilesystemStatus, LastPass, Passing, Preview, STATUS_VERSION, Status, Writing,
    news,
};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

/// What the machine is doing, read by the caller each step.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Senses {
    pub power: Power,
    /// Whether the power profile is saving power.
    pub saver: bool,
    pub pressure: Pressure,
}

/// A filesystem a source lies on, as last seen.
struct Watched {
    path: PathBuf,
    band: Band,
    fill: Fill,
    space: Option<Space>,
    /// After a pass that fell short: no other until this monotonic time,
    /// unless the band gets worse than it was.
    rest: Option<Rest>,
    /// The pool this filesystem's freed blocks go back to, as an image on
    /// it that discards (`pool::backed_by`).
    backing: Option<u64>,
}

#[derive(Clone, Copy, Debug)]
struct Rest {
    until: f64,
    seconds: f64,
    band: Band,
    /// Whether the pass fell short while the system daemon couldn't be
    /// asked about the processes this account can't read, so what it kept
    /// may only have wanted that word. Such a rest ends once the daemon
    /// answers again: on 8 November the first pass after a start sometimes
    /// went unheard, and a critical disk's cache waited a minute. Never
    /// twice in a row, so a daemon whose word keeps failing costs a survey
    /// a rest, not one a step.
    unvouched: bool,
    /// Whether the pass ended with nothing left it may clear, the system
    /// daemon having answered, so a second such pass in a row on a filling
    /// disk empties it.
    exhausted: bool,
    /// Whether a second pass in a row found nothing left to clear on a
    /// filling disk, so the disk rests the hour and the status says it has
    /// cleared all it may, whatever passes run on other disks meanwhile.
    emptied: bool,
}

/// How far back writes are weighed when choosing what to freeze.
const WRITING_WINDOW: f64 = 60.0;

/// What the garbage's commands and candidates are named for.
const GARBAGE: &str = "garbage";

type DedupeOut = (f64, Arc<AtomicBool>, Receiver<Result<Option<u64>, String>>);

/// What holds the store, as cheaply read as it changes: the processes with
/// temporary roots, by their files' names, and each of Nix's roots that
/// still leads somewhere, by its link.
#[derive(Clone, Debug, Default, PartialEq)]
struct Holding {
    processes: BTreeSet<OsString>,
    roots: BTreeSet<PathBuf>,
}

impl Holding {
    fn read(temp_roots: &Path, roots: &Path) -> Holding {
        let names = |dir: &Path| -> Vec<fs::DirEntry> {
            fs::read_dir(dir).map_or_default(|entries| entries.filter_map(Result::ok).collect())
        };
        let processes = names(temp_roots)
            .into_iter()
            .map(|entry| entry.file_name())
            .collect();
        // Following each link, as Nix does: a result deleted leaves its
        // link behind, leading nowhere, until a collection tidies it.
        let roots = names(&roots.join("auto"))
            .into_iter()
            .map(|entry| entry.path())
            .filter(|link| fs::metadata(link).is_ok())
            .collect();
        Holding { processes, roots }
    }
}

/// Where a guard keeps and reads things.
#[derive(Clone, Debug, PartialEq)]
pub struct Places {
    /// Its memory and journal.
    pub state: PathBuf,
    /// Its status file, when the config asks for one.
    pub status: Option<PathBuf>,
    /// Where processes are read from: `/proc`.
    pub processes: PathBuf,
    /// The system daemon's socket, for a user daemon to ask about the
    /// processes it can't read.
    pub system: Option<PathBuf>,
    /// Links to the systems running and booted, whose generations stay.
    pub running: Vec<PathBuf>,
}

/// What `path` holds, or a fresh one when it can't be read, since what it
/// remembers is only ever a head start.
fn loaded<T: serde::de::DeserializeOwned + Default>(path: &Path) -> T {
    state::load(path).ok().flatten().unwrap_or_default()
}

/// How `config`'s finished sources ask whether a checkout is done, leaving
/// out any whose remote isn't a pattern.
fn predicates(config: &Config) -> Vec<Predicate> {
    config
        .sources
        .values()
        .filter_map(|source| match source {
            Source::Finished {
                remote,
                command,
                timeout,
            } => Some(Predicate {
                remote: regex_lite::Regex::new(remote).ok()?,
                command: command.iter().map(OsString::from).collect(),
                timeout: timeout.0,
            }),
            _ => None,
        })
        .collect()
}

#[expect(
    clippy::struct_excessive_bools,
    reason = "its four flags are independent facts about unrelated work, not one state"
)]
pub struct Guard {
    config: Config,
    processes: PathBuf,
    system: Option<PathBuf>,
    memory_path: PathBuf,
    memory: Memory,
    journal: Journal,
    status_path: Option<PathBuf>,
    predicates: Vec<Predicate>,
    answers: Answers,
    /// Where the answers are kept for the next guard.
    answers_path: PathBuf,
    watched: BTreeMap<u64, Watched>,
    last: Option<LastPass>,
    /// The pass whose failing still stands: kept through passes elsewhere
    /// or ones that free nothing, until a pass on its disk frees some or
    /// finds it no longer filling.
    failed: Option<LastPass>,
    /// The pass at work, while one is, so the status each beat writes
    /// again says so.
    passing: Option<Passing>,
    saved_at: Option<f64>,
    written: Option<(f64, Status)>,
    /// The status last written, which each beat writes again fresh.
    beaten: Arc<Mutex<Option<Status>>>,
    /// Whether what an earlier run left moved aside has been seen to.
    resumed: bool,
    /// When this step began, as its tick was read, so what a step does
    /// is timed from the tick's wall time by how long the step has run:
    /// a pass after a long one in the same step, or after the survey, is
    /// stamped when it ran, not when the step began.
    stepped: Instant,
    /// Nix's garbage as last listed, for the system daemon, and where it's
    /// kept for the next guard.
    listing: Option<Listing>,
    listing_path: PathBuf,
    /// When a listing last began, so one that failed waits.
    listing_begun: Option<f64>,
    /// A listing under way on its own thread, and when it began.
    listing_out: Option<(f64, Listening)>,
    /// When the last listing ended, and how long it took.
    listing_ended: Option<(f64, f64)>,
    /// Whether the last listing failed, which rests however full the store:
    /// it freed nothing, and the locked one held Nix's lock all along.
    listing_failed: bool,
    /// Whether a writers' hold's mark stood when the step began: no pass,
    /// listing or collection starts short of nearly full while one does.
    /// Not `held_ended`, a process that held store paths having ended.
    writers_held: bool,
    /// Since when a listing that's due has waited for the machine to have
    /// room.
    room_awaited: Option<f64>,
    /// What a reach cut short at its length judged, by the filesystem it
    /// reached on, so the next reach there takes up from where it stopped
    /// rather than judging the same again.
    reached_short: BTreeMap<u64, BTreeSet<PathBuf>>,
    /// The step whose upkeep passes are being judged, by its wall clock,
    /// and whether the upkeep before them had gone unheard, so each of
    /// them is judged against that and not against the one before it.
    upkeep_judged: Option<(f64, bool)>,
    /// What a system daemon hears from user daemons.
    hearing: Option<Hearing>,
    /// For a user daemon, the filesystems the system daemon collects
    /// garbage on, by device, as it last said.
    collected_on: BTreeSet<u64>,
    /// Links to the systems running and booted.
    running: Vec<PathBuf>,
    /// For a user daemon, whether the system daemon answered its last
    /// heartbeat.
    system_heard: bool,
    /// Whether garbage is to be listed again at once, generations having
    /// expired since the listing began, or a batch having found it stale.
    relist: bool,
    /// What this round's passes have tried, so the pass on the disk beneath
    /// an image doesn't try again what the image's own pass did: a target
    /// taken stays an empty folder, a trimmed log stays, and a command
    /// leaves nothing to look at. A kept one counts too: an install kept
    /// as written again while it was cleared has been emptied once, and a
    /// second pass would empty what its owner just wrote. The next round
    /// tries again.
    tried: BTreeSet<PathBuf>,
    /// Whether a batch from the listing held was refused as stale. Builds
    /// root again what a listing called dead, on 28 October 538 roots
    /// reaching into one batch of 2,000 two minutes after it was made, so
    /// its other batches meet the same, each refusal a whole look for roots, while the listing
    /// that would cure it waits on the lock those deletes take: none is
    /// offered until the next listing is heard.
    stale_listing: bool,
    /// The store's dead paths the system refused a delete, left out of every
    /// batch after: from 04:11 on 12 November a Mac's guard met one app
    /// under App Management in 33 passes of 87 and freed nothing beside it.
    /// Kept until the guard restarts, as a switch restarts it.
    refused: BTreeSet<String>,
    /// What held the store at the last look: the processes with temporary
    /// roots, and the results Nix's roots link to that are still there.
    holding: Option<Holding>,
    /// When owners were last looked at, in monotonic seconds.
    owners_at: Option<f64>,
    /// The producers whose watched files said something new at that look,
    /// each to run on its own rather than make all of upkeep due.
    produced: Vec<String>,
    /// When, in wall seconds, the soonest build the last whole survey found
    /// in a pinned snapshot will have gone an hour unused, and so be spent.
    snapshot_spends: Option<f64>,
    /// The pinned snapshots the last look at the owners found, so one cut
    /// since, which no survey has offered yet, times its own hour.
    snapshots_known: Option<Vec<PathBuf>>,
    /// Whether the upkeep due now was made due by builds whose owner ended
    /// or went an hour unused, which go within a minute, past a survey's
    /// spacing, since nothing an agent leaves should outlive it for long.
    spent_due: bool,
    /// When, in wall seconds, a cargo source's disk past its budget last
    /// made upkeep due, so one still past it after the pass, its builds
    /// all recent, asks again only an hour on.
    budget_looked: Option<f64>,
    /// Whether a root left since the listing began, or a process holding
    /// paths ended, either of which makes garbage the listing lacks.
    root_left: bool,
    held_ended: bool,
    /// Whether the listing's garbage is to be looked at for a pass of its
    /// own, something having let some of it go.
    collect: bool,
    /// Until when, in wall seconds, the garbage's own passes rest, and how
    /// long the rest was, after one that left all it was offered.
    collect_rest: Option<(f64, f64)>,
    /// The paths such a pass left, which bring no pass while it rests.
    left_by_collect: BTreeSet<String>,
    /// Until when, in wall seconds, Nix's deletes rest after the last,
    /// short of nearly full.
    deletes_rest: Option<f64>,
    /// A run linking the store's identical files: when it began, what
    /// stops it, and where it tells how it ended.
    dedupe_out: Option<DedupeOut>,
    /// When the last such run failed.
    dedupe_failed: Option<f64>,
    /// When each build's artifacts were last opened, shared with whatever
    /// hears the opens, and where it's kept for the next guard.
    uses: Arc<Mutex<Uses>>,
    uses_path: PathBuf,
    /// What hears the opens, once the guard listens, and when it last
    /// marked what's built.
    opens: Option<Arc<crate::opens::Watch>>,
    marked_at: Option<f64>,
    /// Whether what hears the opens still does: its thread ends on an
    /// error reading them, after which nothing is heard.
    listening: Arc<AtomicBool>,
    /// Nix's own `min-free`, when it was read, and how many reads of it in
    /// a row have failed since.
    min_free: Option<(f64, Option<u64>, u32)>,
    /// The account's home, which sentences write as `~`, for a guard run
    /// in one; the system's guard has none.
    home: Option<PathBuf>,
    /// What freezes a runaway writer, once given one, and what each
    /// process has written lately.
    freeze: Option<Box<dyn crate::freeze::Freeze>>,
    /// The disks it says the writes and wear of, once given them.
    disks: Option<crate::disks::Watch>,
    /// Whether the daemon has been told to stop, which a survey asks as it
    /// walks; without one given, nothing stops a survey.
    stop: Box<dyn Fn() -> bool>,
    writers: Writers,
    /// What the last survey couldn't read, and why it went unvouched.
    unread: (usize, Option<String>),
    /// Whether the memory read was damaged, and is still to be set aside.
    damaged: bool,
    /// The trees the last survey kept for their use without offering.
    kept: Vec<(PathBuf, crate::activity::Use)>,
    /// What the last survey offered, less what passes took since, so a
    /// holder can say how much of it the guard may clear, and how much a
    /// clear would take now; none before the first survey, which a disk
    /// well under its budget may not make for six hours.
    offered: Option<crate::holders::Offers>,
    /// What surveys sized of quest workspaces, so the next sizes again
    /// only what was written since.
    sizes: crate::survey::Sizes,
    /// Whether the holders are to be measured again on the next step,
    /// since a clear has just changed them.
    remeasure: bool,
    /// What holds the disks, as last measured and when, in wall seconds,
    /// and how much each grew in the day before.
    measured: Measured,
    /// The thread measuring them, while it does, giving how long it took
    /// and what it read from disk.
    measuring: Option<std::thread::JoinHandle<Walked>>,
    /// When the last measure began, how full each disk was then, and how
    /// long it took once it's done.
    began: Option<crate::holders::LastMeasure>,
}

/// What holds the disks, as a thread of its own measured it.
type Measured = Arc<Mutex<Option<crate::ledger::Measure>>>;

/// Why a path stays or would go.
#[derive(Clone, Debug, PartialEq)]
pub enum Explained {
    /// A source offers it, or what holds it, in use this way now.
    Offered {
        candidate: Candidate,
        used: crate::activity::Use,
    },
    /// A source found the tree holding it, which is kept for its use.
    Kept {
        path: PathBuf,
        used: crate::activity::Use,
    },
    /// No source finds it or anything holding it.
    Nothing,
}

/// Where a listing on its own thread sends the dead paths, or why not.
type Listening = Receiver<Result<Vec<Listed>, String>>;

/// What a system daemon hears from user daemons through its socket.
pub struct Hearing {
    /// When a user daemon last spoke for each filesystem, by device.
    pub spoken: Arc<Mutex<HashMap<u64, Instant>>>,
    /// The collects asked for: a path on the filesystem, the band to take
    /// its garbage as, and the connection to answer on.
    pub collects: Receiver<(PathBuf, Band, Responder)>,
}

/// What set a pass going, for its journal line: a goal alone can't say,
/// since Nix's garbage passes as upkeep, and a user daemon's collect as a
/// reach, and in the day to 03:00 on 21 November half the passes freed
/// nothing with no way to tell which of these each was.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Why {
    /// Free space short of the target, or forecast to be; a forecast's line
    /// shows the band it reaches for, not the one the disk is in.
    Reach,
    /// A healthy disk's routine: every six hours, and at once when an
    /// owner ends, a new boot or guard binary starts, a snapshot's build
    /// goes an hour unused, a builds disk passes its budget, or a garbage
    /// listing finds some to let go; again ten minutes after one that kept
    /// what only an unheard system daemon could vouch for, and at the next
    /// spare step after one left unfinished or stopped. The line can't say
    /// which of these.
    Upkeep,
    /// Nix's garbage, once something let some go: alone, with no survey,
    /// unless another disk was due in the same step, when the survey's
    /// offers on the store's disk come too, and the line says how long
    /// that survey took. It runs as the routine, so it puts off every
    /// disk's next six-hourly upkeep.
    Garbage,
    /// Producers whose watched files said something new.
    Producers,
    /// What Joel asked cleared.
    Clear,
    /// The system's share of the store, asked for by a user daemon.
    Collect,
}

impl Why {
    fn word(self) -> &'static str {
        match self {
            Why::Reach => "reach",
            Why::Upkeep => "upkeep",
            Why::Garbage => "garbage",
            Why::Producers => "producers",
            Why::Clear => "clear",
            Why::Collect => "collect",
        }
    }
}

/// What a pass plans by and is for, what set it going, and how long the
/// survey that offered it its candidates took and what it read, when one
/// did. Every pass one survey served says so, the passes of one step
/// sharing its walk, so none reads as having needed none.
struct Aim {
    band: Band,
    goal: Goal,
    why: Why,
    surveyed: Option<Walked>,
}

/// A decision's aim, offered by the survey `surveyed`, when one did.
fn aim(decision: Decision, surveyed: Option<Walked>) -> Aim {
    let (band, goal, why) = match decision {
        Decision::Reach(band) => (band, Goal::Reach, Why::Reach),
        Decision::Collect => (Band::Healthy, Goal::Upkeep, Why::Garbage),
        // A rest is never due, so never aimed; it reads as the routine only
        // so that the match needs no arm that can't run.
        Decision::Upkeep | Decision::Rest => (Band::Healthy, Goal::Upkeep, Why::Upkeep),
    };
    Aim {
        band,
        goal,
        why,
        surveyed,
    }
}

/// The time now, in wall seconds.
fn wall_now() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0.0, |since| since.as_secs_f64())
}

/// Whether a writers' hold, when `held`, keeps passes off a disk in
/// `band`: every band short of nearly full waits for the hold to end.
/// Passes and the status both ask this, so what the status says is what
/// the passes do.
pub(crate) fn held_off(held: bool, band: Band) -> bool {
    held && band < Band::NearlyFull
}

/// How a pass ended, in the journal's words.
fn ending(end: &End) -> String {
    match end {
        End::Reached => "reached".to_owned(),
        End::Exhausted => "exhausted".to_owned(),
        End::Stalled => "stalled".to_owned(),
        End::Stopped => "stopped".to_owned(),
        End::Unfinished => "unfinished".to_owned(),
        End::Waiting => crate::pass::WAITING.to_owned(),
        End::Failed(why) => format!("failed: {why}"),
    }
}

// Pins which why and goal each decision gives its pass. Which survey a
// garbage pass beside another disk's is handed is chosen in `passes`,
// which needs two watched disks in one step, and no fixture builds that,
// so this test doesn't cover it.
#[cfg(test)]
mod tests {
    use super::nix_garbage::min_free_again;
    use super::passes::{pressed, survey_spacing};
    use super::surveying::upkept_after;

    #[test]
    fn a_survey_shorter_than_five_seconds_spaces_nothing() {
        let spaced: Vec<f64> = [4.9, 5.0, 600.0, 1800.0]
            .into_iter()
            .map(survey_spacing)
            .collect();
        assert_eq!(
            spaced,
            [0.0, 60.0, 7200.0, 10800.0],
            "a survey was spaced other than not at all under five seconds, then twelve \
             times its walk up to three hours"
        );
    }
    use super::{Band, Decision, Goal, Walked, Why, aim};

    #[test]
    fn any_upkeep_of_a_step_that_kept_unvouched_brings_the_retry_once() {
        // Two disks' upkeep passes in one step, as a user guard watching a
        // home pool and a build image makes, the later one keeping what
        // only the unheard system daemon could vouch for.
        let (wall, retry) = (100_000.0, 79_000.0);
        let judged = |kept: [bool; 2], before: bool| {
            kept.iter()
                .fold(None, |upkept: Option<(f64, bool)>, &kept| {
                    upkept_after(upkept.map(|(at, _)| at), (wall, retry), before, kept).or(upkept)
                })
        };
        assert_eq!(
            judged([false, true], false),
            Some((retry, true)),
            "a later disk's unvouched keep was lost to an earlier disk that kept nothing"
        );
        assert_eq!(
            judged([true, false], false),
            Some((retry, true)),
            "a later disk that kept nothing took back the retry an earlier one set"
        );
        assert_eq!(
            judged([false, true], true),
            Some((wall, false)),
            "a retry that kept unvouched again was retried"
        );
    }

    #[test]
    fn a_disk_worse_than_filling_or_forecast_to_be_goes_past_a_survey_s_spacing() {
        let cases = [
            (Decision::Reach(Band::NearlyFull), Band::Filling),
            (Decision::Reach(Band::CriticallyFull), Band::NearlyFull),
            (Decision::Reach(Band::NearlyFull), Band::NearlyFull),
            (Decision::Reach(Band::Filling), Band::Filling),
            (Decision::Reach(Band::Filling), Band::Healthy),
            (Decision::Upkeep, Band::Healthy),
        ];
        let went: Vec<bool> = cases
            .iter()
            .map(|&(decision, band)| pressed(decision, band))
            .collect();
        assert_eq!(
            went,
            [true, true, true, false, false, false],
            "a pass went past a survey's spacing other than for a disk worse than filling \
             or forecast to be"
        );
    }

    #[test]
    fn a_failing_min_free_read_waits_twice_as_long_each_time_up_to_an_hour() {
        let waits: Vec<f64> = (1..=7).map(min_free_again).collect();
        assert_eq!(
            waits,
            [300.0, 600.0, 1200.0, 2400.0, 3600.0, 3600.0, 3600.0],
            "failures in a row waited other than 5, 10, 20 and 40 minutes, then hourly"
        );
    }

    fn walked(seconds: f64) -> Walked {
        Walked {
            seconds,
            read: Some(4096),
        }
    }

    #[test]
    fn each_decision_aims_its_pass_with_its_why_and_the_survey_it_was_given() {
        let garbage = aim(Decision::Collect, None);
        assert_eq!(
            (garbage.goal, garbage.why, garbage.surveyed),
            (Goal::Upkeep, Why::Garbage, None)
        );
        let beside = aim(Decision::Collect, Some(walked(1.5)));
        assert_eq!(
            (beside.why, beside.surveyed),
            (Why::Garbage, Some(walked(1.5)))
        );
        let reach = aim(Decision::Reach(Band::Filling), Some(walked(2.0)));
        assert_eq!(
            (reach.goal, reach.why, reach.surveyed),
            (Goal::Reach, Why::Reach, Some(walked(2.0)))
        );
        assert_eq!(reach.band, Band::Filling);
        let upkeep = aim(Decision::Upkeep, Some(walked(3.0)));
        assert_eq!(
            (upkeep.why, upkeep.surveyed),
            (Why::Upkeep, Some(walked(3.0)))
        );
    }
}
