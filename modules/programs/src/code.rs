//! Code's rows as this daemon serves them (`code_rows.rs`): every
//! repository on the machine, read from git's own files by the repos
//! library (modules/repos) under the folders `REPOS_ROOTS` names, with no
//! process started. The first watch of them waits for the first read,
//! outside the daemon's lock, so its answer holds them; every watch after
//! is answered from the rows held at once, and the disk is read again on
//! a thread of its own, telling each watch only the rows that read
//! differently and those gone. A read takes about 60 ms warm and 230 ms
//! cold over 18 repositories and 800 trees (#4653), well past the
//! launcher's 50 ms, which is why no watch but the first waits on one.
//! Each read takes in what the repos timer's last refresh kept of each
//! repository's open changes, as the repos command does, so a row
//! counts them.
//!
//! How many tracked files each worked-in checkout has changed and not
//! committed is counted by git behind a watch, after the rows read are
//! served, never on the first watch's own read: a status of the largest
//! checkout here takes about 150 ms warm, and seconds cold. Code is
//! watched whenever the launcher is open, so each opening counts again,
//! a few checkouts at once, unless one counted in the last `FRESH`. A
//! row's count is as old as the last opening, and is brought up to date
//! just after.
//!
//! A checkout's files are listed only while a watch holds its Open a File
//! by Name row ahead (ready-rows.md's "What's read only while it's looked
//! at"), since one holds thousands: listed as Neovim's finder lists them
//! (repositories.rs), on the thread that follows the watches, outside the
//! lock, and served inside the row. Once no watch names the row, its
//! files go. A checkout's branches are listed the same way, while a watch
//! holds its Switch Branch row ahead (switch.rs). A Pull or a switch
//! lists both again, since each moves what the checkout holds. Forget Gone
//! Trees and Gather Its Store (tend.rs) read the disk again once git has
//! acted, so the row and its page no longer say they're wanted.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::time::{Duration, Instant};

use programs::{code_rows, command, pull, repositories, switch, tend, visited};
use repos::model::Record;
use repos::read::{Reading, read};
use repos::{github, upstreams};
use serde_json::Value;

use super::{Daemon, Shared, keep, keys_watched, lock};

/// The longest a count of one checkout's uncommitted files may take
/// before it's stopped, so one on a stalled disk never holds the others'
/// for long.
const COUNTING: Duration = Duration::from_secs(5);

/// How many checkouts' files are counted at once.
const AT_ONCE: usize = 4;

/// How long a count stands before a watch beginning counts again, so the
/// launcher opened and closed in a run counts each checkout once.
const FRESH: Duration = Duration::from_secs(10);

/// How long a pass waits for zoxide's database to be read, past which
/// nothing orders Code, and then for its folders and the checkouts' to be
/// resolved, past which they're counted as written: a database or folder
/// on a mount that no longer answers would otherwise hold every pass
/// after it, and the order costs only a link's worth of being wrong.
const RESOLVING: Duration = Duration::from_secs(2);

/// Hears each watch begin or end, on a thread of its own, and has `ahead`
/// list what the watches name then. daemon-serve says so with every
/// record held, so it's only passed on there, and the keys read after.
pub fn follow(shared: &Arc<Shared>) {
    let (moved, heard) = mpsc::channel();
    shared.served().on_watches(Box::new(move || {
        let _ = moved.send(());
    }));
    let shared = Arc::clone(shared);
    std::thread::spawn(move || {
        while heard.recv().is_ok() {
            let _ = heard.try_iter().count();
            ahead(&shared);
        }
    });
}

/// Where Code looks and how it words its rows, set as the daemon starts.
pub struct Code {
    roots: Vec<PathBuf>,
    home: Option<PathBuf>,
    machine: String,
    /// zoxide's database, whose visits order the repositories by when
    /// Joel was last in each (visited.rs).
    visited: Option<PathBuf>,
    words: code_rows::Words,
    /// What Pull's answer says.
    pulled: pull::Words,
    /// What Switch to It's answer says.
    switched: switch::Words,
    /// What Forget Gone Trees' and Gather Its Store's answers say.
    tended: tend::Words,
    /// Whether the disk has been read once, so a watch knows whether rows
    /// are held to answer with.
    read_once: AtomicBool,
    /// Whether a read is under way on a thread of its own, so watches
    /// beginning together read the disk once.
    reading: AtomicBool,
    /// Whether the disk is to be read again, asked while a read was under
    /// way, which may have read it before what moved it.
    wanted: AtomicBool,
    /// How many reads of zoxide's database, and apart from them how many
    /// resolutions of its folders, ran past `RESOLVING` and haven't ended,
    /// so no pass starts another of the same while one is held. Counted
    /// apart, so a folder on a stuck mount leaves the visits ordering Code
    /// as written, and a stuck database leaves nothing to resolve anyway.
    late_reads: Arc<AtomicUsize>,
    late: Arc<AtomicUsize>,
    /// The number the next pass making Code's rows is given, under
    /// `found`'s lock as it holds or reads the repositories it orders, and
    /// the latest whose rows were taken, kept under the daemon's lock: a
    /// pass numbered before the one last taken lets its rows go, so no
    /// order read before another is taken after it, and no lock is held
    /// while the disk is read.
    passes: AtomicU64,
    taken: AtomicU64,
    /// The checkouts a Pull, a switch or a tending is under way in, so a
    /// second waits for none and none races another.
    busy: Mutex<BTreeSet<String>>,
    /// Held for the whole of a listing, so two at once, as the watches
    /// moving and a switch's give, never keep each other's older list.
    listing: Mutex<()>,
    /// How many tracked files each checkout counted has changed and not
    /// committed, by its path, kept until it's counted again.
    uncommitted: Mutex<BTreeMap<String, u32>>,
    /// When they were last counted.
    counted_at: Mutex<Option<Instant>>,
    /// The rows the last read made: each repository's and those inside it.
    made: Mutex<Vec<Value>>,
    /// The repositories the last read found, so a watch beginning can make
    /// their rows again with zoxide's visits as they stand, walking no root.
    found: Mutex<Option<Arc<Record>>>,
    /// The files listed of each Open a File by Name row ahead, and the
    /// branches of each Switch Branch row ahead, by its key.
    listed: Mutex<BTreeMap<String, Vec<Value>>>,
}

impl Code {
    /// Code's settings from the environment: the folders `REPOS_ROOTS`
    /// names, colon-separated as PATH is, else ~/src, as repos itself reads
    /// them, and zoxide's database where `PROGRAMS_VISITED` names it. With
    /// none named, as on an account without zoxide, no visit orders Code,
    /// so a database zoxide left behind can't hold the order still.
    pub fn new(home: &str, vocabulary: &serde_json::Value) -> Code {
        let home = (!home.is_empty()).then(|| PathBuf::from(home));
        let mut roots: Vec<PathBuf> = std::env::var("REPOS_ROOTS")
            .unwrap_or_default()
            .split(':')
            .filter(|root| !root.is_empty())
            .map(PathBuf::from)
            .collect();
        if roots.is_empty() {
            roots.extend(home.as_ref().map(|home| home.join("src")));
        }
        let visited = std::env::var_os("PROGRAMS_VISITED")
            .filter(|path| !path.is_empty())
            .map(PathBuf::from);
        Code {
            roots,
            home,
            machine: terminal_kit::environment::machine(),
            visited,
            words: code_rows::Words::read(vocabulary),
            pulled: pull::Words::read(vocabulary),
            switched: switch::Words::read(vocabulary),
            tended: tend::Words::read(vocabulary),
            read_once: AtomicBool::new(false),
            reading: AtomicBool::new(false),
            wanted: AtomicBool::new(false),
            late_reads: Arc::new(AtomicUsize::new(0)),
            late: Arc::new(AtomicUsize::new(0)),
            passes: AtomicU64::new(1),
            taken: AtomicU64::new(0),
            busy: Mutex::new(BTreeSet::new()),
            listing: Mutex::new(()),
            uncommitted: Mutex::new(BTreeMap::new()),
            counted_at: Mutex::new(None),
            made: Mutex::new(Vec::new()),
            found: Mutex::new(None),
            listed: Mutex::new(BTreeMap::new()),
        }
    }

    /// The repositories as the disk holds them now.
    fn record(&self) -> Record {
        let mut record = read(&Reading {
            roots: &self.roots,
            home: self.home.as_deref(),
            machine: &self.machine,
            now: programs::now_ms() / 1000,
        });
        if let Some(kept) = github::seen_file(self.home.as_deref()) {
            github::merge_kept(&mut record, &kept);
        }
        if let Some(kept) = upstreams::counted_file(self.home.as_deref()) {
            upstreams::merge_kept(&mut record, &kept);
        }
        record
    }

    /// Holds `record` as the repositories last found, answering the number
    /// of the pass that orders them. Numbered under the lock that holds
    /// it, as `latest` is, so a pass on an older record is never numbered
    /// after one on a newer.
    fn hold(&self, record: &Arc<Record>) -> u64 {
        let mut found = lock(&self.found);
        *found = Some(Arc::clone(record));
        self.passes.fetch_add(1, Ordering::SeqCst)
    }

    /// The repositories last found, if any, with the number of the pass
    /// that orders them, taken under the same lock as `hold` takes its.
    fn latest(&self) -> Option<(Arc<Record>, u64)> {
        let found = lock(&self.found);
        let record = Arc::clone(found.as_ref()?);
        Some((record, self.passes.fetch_add(1, Ordering::SeqCst)))
    }

    /// When Joel was last in each repository of `record`, by its key, from
    /// zoxide's visits as they stand, read once a pass. It reads the disk,
    /// so it runs before the daemon's lock is taken: the database `within`
    /// `RESOLVING`, past which nothing orders Code and the log says why,
    /// and then its folders and the checkouts' resolved to their real
    /// paths, past which both are counted as written.
    fn last_visits(&self, record: &Arc<Record>) -> BTreeMap<String, u64> {
        let Some(database) = self.visited.clone() else {
            return BTreeMap::new();
        };
        // Read on a thread of its own too, so a database on a mount that
        // stopped answering holds that thread, never the watch.
        let reading = database.clone();
        let read = visited::within(RESOLVING, &self.late_reads, move || {
            visited::visits_in(&reading)
        });
        let Some(written) = read else {
            visited::late(&database, RESOLVING);
            return BTreeMap::new();
        };
        // None read leaves nothing to resolve.
        if written.is_empty() {
            return BTreeMap::new();
        }
        let resolving = Arc::clone(record);
        let visits = written.clone();
        let resolved = visited::within(RESOLVING, &self.late, move || {
            let places = code_rows::places(&resolving);
            let visits = visited::resolved(visits, |folder| code_rows::placed(&places, folder));
            code_rows::last_visits(&places, &visits)
        });
        resolved
            .unwrap_or_else(|| code_rows::last_visits(&code_rows::written_places(record), &written))
    }

    /// Code's rows from `record`, with the uncommitted files last counted
    /// and when Joel was `last` in each, and the rows inside them.
    fn made(&self, record: &Record, last: &BTreeMap<String, u64>) -> Vec<Value> {
        let uncommitted = lock(&self.uncommitted).clone();
        let home = self.home.as_deref();
        let mut rows = code_rows::rows(record, home, &self.words, &uncommitted, last);
        rows.extend(code_rows::inside(record, &self.words));
        rows
    }

    /// Counts the uncommitted files of each repository's worked-in
    /// checkout, unless they were counted in the last `FRESH`, answering
    /// whether any count moved. git runs outside every lock.
    fn count(&self, record: &Record) -> bool {
        {
            let mut at = lock(&self.counted_at);
            if at.is_some_and(|at| at.elapsed() < FRESH) {
                return false;
            }
            *at = Some(Instant::now());
        }
        let paths: Vec<&str> = record
            .repositories
            .iter()
            .filter_map(code_rows::worked_in)
            .filter(|checkout| !checkout.bare)
            .map(|checkout| checkout.path.as_str())
            .collect();
        let counted = counted(&git(), &paths);
        let mut held = lock(&self.uncommitted);
        let now = kept(&held, counted);
        let moved = now != *held;
        *held = now;
        moved
    }
}

/// What git says each checkout at `paths` holds uncommitted, `AT_ONCE`
/// at a time, none for one it couldn't say.
fn counted<'a>(git: &str, paths: &[&'a str]) -> Vec<(&'a str, Option<u32>)> {
    let each = paths.len().div_ceil(AT_ONCE).max(1);
    std::thread::scope(|scope| {
        let runs: Vec<_> = paths
            .chunks(each)
            .map(|chunk| {
                scope.spawn(move || {
                    chunk
                        .iter()
                        .map(|path| {
                            let count = programs::git::uncommitted(git, Path::new(path), COUNTING);
                            (*path, count)
                        })
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        runs.into_iter()
            .flat_map(|run| run.join().unwrap_or_default())
            .collect()
    })
}

/// The counts to hold after counting: each checkout counted at what git
/// said, one git couldn't read at its last count, and one no longer
/// counted gone.
fn kept(held: &BTreeMap<String, u32>, counted: Vec<(&str, Option<u32>)>) -> BTreeMap<String, u32> {
    counted
        .into_iter()
        .filter_map(|(path, count)| {
            let count = count.or_else(|| held.get(path).copied())?;
            Some((path.to_owned(), count))
        })
        .collect()
}

/// As a watch of Code's rows begins: the first ever reads the disk on
/// this thread, outside the lock, so its answer holds them; any later one
/// is answered from the repositories last found, their rows made again
/// with zoxide's visits as they stand, so the order Joel's last `cd` gave
/// is in the answer, and the roots walked again behind it.
pub fn begin(shared: &Arc<Shared>) {
    let code = &shared.code;
    if code.read_once.load(Ordering::Acquire) {
        if let Some((record, pass)) = code.latest() {
            ordered(shared, &record, pass);
        }
        again(shared);
        return;
    }
    let record = Arc::new(code.record());
    let pass = code.hold(&record);
    ordered(shared, &record, pass);
    // Counted behind its answer, which never waits on git.
    again(shared);
}

/// Reads the disk again on a thread of its own. Asked while a read is
/// under way, which may have read the disk before what moved it, that
/// read reads it once more as it ends, so no ask is lost. Each flag is
/// set and then the other read, on both sides, so their order is one
/// order everywhere (`SeqCst`), or each could miss the other's.
fn again(shared: &Arc<Shared>) {
    let code = &shared.code;
    code.wanted.store(true, Ordering::SeqCst);
    if code.reading.swap(true, Ordering::SeqCst) {
        return;
    }
    let shared = Arc::clone(shared);
    std::thread::spawn(move || {
        let code = &shared.code;
        loop {
            while code.wanted.swap(false, Ordering::SeqCst) {
                // Served before counting, so nothing read waits on git.
                let record = Arc::new(code.record());
                let pass = code.hold(&record);
                ordered(&shared, &record, pass);
                // Counting takes a while, so its rows are ordered by the
                // visits as they stand once it ends, not as they stood.
                if code.count(&record)
                    && let Some((record, pass)) = code.latest()
                {
                    ordered(&shared, &record, pass);
                }
            }
            code.reading.store(false, Ordering::SeqCst);
            // An ask between the last read and letting go is read here.
            if !code.wanted.load(Ordering::SeqCst) || code.reading.swap(true, Ordering::SeqCst) {
                return;
            }
        }
    });
}

/// Takes `record`'s rows ordered by zoxide's visits as they stand, made
/// with every lock let go, unless a pass numbered after `pass` has taken
/// its rows already, so no order read before another is taken after it.
fn ordered(shared: &Shared, record: &Arc<Record>, pass: u64) {
    let code = &shared.code;
    let rows = code.made(record, &code.last_visits(record));
    let mut daemon = lock(&shared.daemon);
    if newest(&code.taken, pass) {
        take(shared, &mut daemon, rows);
    }
}

/// Whether `pass` began after every pass `taken` says was taken, marking
/// it taken if so. Called with the daemon's lock held, so two passes
/// never both read `taken` before either marks it.
fn newest(taken: &AtomicU64, pass: u64) -> bool {
    if taken.load(Ordering::SeqCst) > pass {
        return false;
    }
    taken.store(pass, Ordering::SeqCst);
    true
}

/// Keeps the rows read, telling each watch only what moved.
fn take(shared: &Shared, daemon: &mut Daemon, rows: Vec<Value>) {
    *lock(&shared.code.made) = rows;
    served(shared, daemon);
    shared.code.read_once.store(true, Ordering::Release);
}

/// Serves the rows the last read made and the files and branches listed
/// inside them, telling each watch only what moved.
fn served(shared: &Shared, daemon: &mut Daemon) {
    let mut rows = lock(&shared.code.made).clone();
    for (parent, listed) in lock(&shared.code.listed).iter() {
        let words = &shared.code.words;
        rows.extend(if parent.ends_with(code_rows::BRANCHES) {
            code_rows::branches(parent, listed, words)
        } else {
            code_rows::files(parent, listed, words)
        });
    }
    let moved = daemon.code.take(rows, Vec::new());
    keep(daemon, moved.changed, &moved.gone);
}

/// As the watches move: lists the files of each Open a File by Name row,
/// and the branches of each Switch Branch row, a watch now names, on this
/// thread and outside the lock, and lets go of those of each no watch
/// names any more, then serves them. Those still named keep what was
/// listed as their row came ahead.
pub fn ahead(shared: &Shared) {
    list(shared, &[]);
}

/// As `ahead`, but those of the rows `stale` names are listed again
/// rather than kept, and served in place of what was listed, never gone
/// between; one git can't list now keeps what it listed before. The keys
/// watched are read once the listing is this one's alone, so the last to
/// list lists what's watched last.
fn list(shared: &Shared, stale: &[String]) {
    let _alone = lock(&shared.code.listing);
    // Read first, so the daemon's lock is let go before `served` takes it.
    let watched = keys_watched(&lock(&shared.daemon));
    // Only a repository's own rows list anything, never a branch's row
    // inside one, whatever its branch is called.
    let wanted: Vec<&String> = {
        let made = lock(&shared.code.made);
        watched
            .iter()
            .filter(|key| {
                (key.ends_with(code_rows::FILES) || key.ends_with(code_rows::BRANCHES))
                    && made
                        .iter()
                        .any(|row| row["key"] == key.as_str() && row["depth"] == 1)
            })
            .collect()
    };
    let before = lock(&shared.code.listed).clone();
    let mut was = before.clone();
    if stale.is_empty()
        && wanted.len() == was.len()
        && wanted.iter().all(|key| was.contains_key(*key))
    {
        return;
    }
    was.retain(|key, _| !stale.contains(key));
    let checkouts: Vec<(String, Option<String>)> = {
        let made = lock(&shared.code.made);
        wanted
            .iter()
            .map(|key| {
                let row = made.iter().find(|row| row["key"] == key.as_str());
                let path = row.and_then(|row| row["held"]["path"].as_str());
                ((*key).clone(), path.map(str::to_owned))
            })
            .collect()
    };
    let listed: BTreeMap<String, Vec<Value>> = checkouts
        .into_iter()
        .filter_map(|(key, path)| {
            let listed = match was.get(&key) {
                Some(listed) => listed.clone(),
                // None kept when git couldn't read them and none was
                // before, so the next move of the watches asks again.
                None if key.ends_with(code_rows::BRANCHES) => {
                    branches(shared, &path?).or_else(|| before.get(&key).cloned())?
                }
                None => repositories::files(&path?),
            };
            Some((key, listed))
        })
        .collect();
    *lock(&shared.code.listed) = listed;
    served(shared, &mut lock(&shared.daemon));
}

/// New Terminal Here on a repository's row: niri starts kitty in its
/// checkout, rather than this daemon, so the terminal is no child of a
/// service a switch restarts, and niri, which spawned it, gives its window
/// the keys. Answered at once and asked on a thread of its own, as a
/// kitty tab's acts are (tabs.rs), so no other ask waits on niri.
pub fn terminal(shared: &Shared, id: &Value, folder: &str) -> Value {
    kitty(id, folder, &[], "terminal", found(shared, folder))
}

/// Whether `folder` is a checkout or tree the last read found, so zoxide
/// hears only of a folder Code offered Joel, not any path an ask names.
fn found(shared: &Shared, folder: &str) -> bool {
    lock(&shared.code.found).as_deref().is_some_and(|record| {
        record
            .repositories
            .iter()
            .flat_map(|repository| &repository.checkouts)
            .any(|checkout| {
                checkout.path == folder || checkout.trees.iter().any(|tree| tree.path == folder)
            })
    })
}

/// What Edit Here runs in its kitty: Neovim on the checkout, with its
/// `.envrc` loaded first as a shell in it would load it, so the editor
/// finds the language servers its development shell gives. A blocked
/// `.envrc`, or none, or no direnv at all, still opens Neovim, as a
/// terminal's prompt still comes; a checkout gone since it was read opens
/// nothing, rather than Neovim on whatever folder kitty fell back to.
const EDIT: &str = r#"cd -- "$0" && { eval "$(direnv export bash)"; exec nvim .; }"#;

/// Edit Here on a repository's row: the kitty New Terminal Here starts,
/// running Neovim in it, so kitty goes when Neovim ends, as an editor's
/// window does.
pub fn edit(shared: &Shared, id: &Value, folder: &str) -> Value {
    let program = ["bash", "-c", EDIT, folder];
    kitty(id, folder, &program, "editor", found(shared, folder))
}

/// Has niri start kitty in `folder`, running `program` there if any, and
/// answers that `what` was asked for. Once niri has taken the spawn,
/// zoxide hears of the folder where `PROGRAMS_ZOXIDE` names it, since Joel
/// asked for it and a shell started in a folder tells zoxide nothing until
/// its first `cd`, so Code's order and kitty's Recent move by this act of
/// his too. niri answers before kitty is up, so a kitty that then fails
/// still counts as his visit: he asked to be there. Only a folder `found`
/// says Code offered is told.
fn kitty(id: &Value, folder: &str, program: &[&str], what: &'static str, found: bool) -> Value {
    let niri = std::env::var("PROGRAMS_NIRI").unwrap_or_else(|_| "niri".to_owned());
    let argv: Vec<String> = [
        niri.as_str(),
        "msg",
        "action",
        "spawn",
        "--",
        "kitty",
        "--directory",
        folder,
    ]
    .iter()
    .chain(program)
    .map(|&word| word.to_owned())
    .collect();
    let zoxide = std::env::var("PROGRAMS_ZOXIDE")
        .ok()
        .filter(|zoxide| found && !zoxide.is_empty());
    let folder = folder.to_owned();
    std::thread::spawn(move || {
        if let Err(why) = command::run(&argv) {
            eprintln!("no {what} was started: {why}");
            return;
        }
        if let Some(zoxide) = zoxide {
            let add = [zoxide, "add".to_owned(), "--".to_owned(), folder];
            if let Err(why) = command::run(&add) {
                eprintln!("zoxide didn't hear of the {what}'s folder: {why}");
            }
        }
    });
    serde_json::json!({"id": id, "asked": what})
}

/// Pull on a repository's row, answered on a thread of its own (main.rs
/// says it's slow), since a fetch waits on the network. A Pull of a
/// checkout while a Pull or a switch of it is under way says so rather
/// than racing it. A
/// fetch moves the branch followed and a pull the head, so the disk is
/// read again behind any that reached git, and the row says how far
/// apart they are now.
pub fn pull(shared: &Arc<Shared>, id: &Value, folder: &str) -> Value {
    let pulled = {
        let Some(_held) = Holding::take(&shared.code, folder) else {
            return shared.code.pulled.answer(id, Err(pull::Refused::Busy));
        };
        pull::pull(&git(), Path::new(folder), pull::FETCH)
    };
    if !matches!(pulled, Err(pull::Refused::Tree)) {
        moved(shared, folder);
    }
    shared.code.pulled.answer(id, pulled)
}

/// Forget Gone Trees or Gather Its Store on a repository's row, answered
/// on a thread of its own as Pull is (main.rs says it's slow), since a
/// store of millions takes minutes to gather, and never beside a Pull, a
/// switch or another tending of the checkout. The disk is read again
/// behind any that reached git, so the row and its page say what's left.
pub fn tend(shared: &Arc<Shared>, id: &Value, act: tend::Act, folder: &str) -> Value {
    let tended = {
        let Some(_held) = Holding::take(&shared.code, folder) else {
            return shared.code.tended.answer(id, act, Err(tend::Refused::Busy));
        };
        let folder = Path::new(folder);
        match act {
            tend::Act::Forget => tend::forget(&git(), folder),
            tend::Act::Gather => tend::gather(&git(), folder, tend::GATHER),
        }
    };
    if !matches!(tended, Err(tend::Refused::Tree)) {
        again(shared);
    }
    shared.code.tended.answer(id, act, tended)
}

/// Switch to It on a branch's row, answered on a thread of its own as
/// Pull is, and never beside a Pull or another switch of the checkout.
/// Any that reached the checkout is followed as a Pull is (`moved`), so
/// the branch rows say which it's on now, and one refused for a branch
/// gone lists them without it.
pub fn switch(shared: &Arc<Shared>, id: &Value, folder: &str, branch: &str) -> Value {
    let switched = {
        let Some(_held) = Holding::take(&shared.code, folder) else {
            return shared
                .code
                .switched
                .answer(id, branch, Err(switch::Refused::Busy));
        };
        let home = shared.code.home.as_deref();
        switch::switch(&git(), Path::new(folder), branch, home)
    };
    if !matches!(switched, Err(switch::Refused::Tree)) {
        moved(shared, folder);
    }
    shared.code.switched.answer(id, branch, switched)
}

/// A checkout a Pull or a switch is under way in, let go however it
/// ends, a panic in git's handling included, so it's never left busy.
struct Holding<'a> {
    code: &'a Code,
    folder: &'a str,
}

impl<'a> Holding<'a> {
    /// The checkout at `folder` held, or none when it's held already.
    fn take(code: &'a Code, folder: &'a str) -> Option<Holding<'a>> {
        // The busy set let go before a hold is made, and one made only
        // when it's taken: a hold made and dropped unused would lock the
        // set again, and let the checkout go from under the hold before.
        let taken = lock(&code.busy).insert(folder.to_owned());
        taken.then(|| Holding { code, folder })
    }
}

impl Drop for Holding<'_> {
    fn drop(&mut self) {
        lock(&self.code.busy).remove(self.folder);
    }
}

/// After git moved the checkout at `folder`: the disk read again, and its
/// files and branches listed again where a watch holds them ahead, since
/// a pull or a switch changes both. Listed on a thread of its own, so the
/// act answers without waiting on a listing.
fn moved(shared: &Arc<Shared>, folder: &str) {
    again(shared);
    let shared = Arc::clone(shared);
    let folder = folder.to_owned();
    std::thread::spawn(move || {
        let inside: Vec<String> = lock(&shared.code.made)
            .iter()
            .filter(|row| row["held"]["path"] == folder.as_str() && row["depth"] == 1)
            .filter_map(|row| row["key"].as_str().map(str::to_owned))
            .collect();
        list(&shared, &inside);
    });
}

/// The git Code's acts run, which a test names.
fn git() -> String {
    std::env::var("PROGRAMS_GIT").unwrap_or_else(|_| "git".to_owned())
}

/// The branches of the checkout at `path`, as Switch Branch lists them
/// (`code_rows::branches`), or none when git can't list them.
fn branches(shared: &Shared, path: &str) -> Option<Vec<Value>> {
    let home = shared.code.home.as_deref();
    let listed = switch::branches(&git(), Path::new(path), home)?;
    let listed = listed
        .into_iter()
        .map(|branch| {
            serde_json::json!({
                "name": branch.name,
                "at_s": branch.at_s,
                "here": branch.here,
                "elsewhere": branch.elsewhere,
                "path": path,
            })
        })
        .collect();
    Some(listed)
}

#[cfg(test)]
mod tests;
