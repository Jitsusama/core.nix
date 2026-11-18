//! What of core.nix's, the harness's and mycelium's merges the running
//! system carries, so a landing waits to be tried until it is on the
//! laptop (WCJQSG, 2026-11-13: Joel tried a line at 20:10 that no switch
//! had carried yet, and it failed, as clock-follows had at 19:35).
//!
//! update-to-main says the core it followed in its last ask's answer,
//! `followed.core`, which is what's running once that ask's `step` is
//! `done`; a failed ask rewrites it, and the folder it's in goes at a
//! boot, so the daemon keeps the last done one in a file of its own.
//! Which merges that commit holds is read from a clone of core's
//! (cadence keeps one, fetched each tick) by the subjects on its first
//! parents, `Merge pull request #N from ...`, as main merges each change.
//!
//! The harness runs as pi-packages installed it: its `current` link names
//! the commit, whose merges its mirror's history says the same way, read
//! again as the link moves. Mycelium runs as that core's flake.lock pins
//! it, but no clone of mycelium's is kept here, so its pin is told by the
//! time of the commit it pins. That's most often the merge of the
//! change it was bumped for, as on 13 November, when core pinned #152's
//! and the harness wrote #152's landing 7.5 s later; the harness writes
//! one 3 to 34 s after GitHub merges it, so a landing written within
//! 34 s of the pin, or before it, is the pin's own or older. Only a merge
//! within 31 s of the one pinned would be taken as carried when it isn't;
//! mycelium's closest that day were 52 s apart.
//! Where any of it can't be told, nothing is held back, as before.
//!
//! When each of core's merges reached the machine is told by the
//! switches the integration record lists, a line a switch with when and
//! the core it ran (`{"at", "revs": {"core"}}`), in the order they ran:
//! what a switch brought is what its first parents hold past the one
//! before, and the latest switch to bring a merge is its. Read from the
//! newest back, the first line that can't be read, or pair git can't
//! say, ends what is told, since a switch back across it may have taken
//! a merge away and brought it again. The time is shown only for a merge
//! the machine is known to carry, so a switch back leaves no date on
//! what it took away. The harness's merges carry no time yet: the
//! markers pi-packages leaves in `retired` go with a version it sweeps,
//! and one is written for a build never swapped to, so they don't say
//! each swap.
//!
//! Long because more than half of it is the tests beside it, which build
//! real histories and records for each way an arrival is told; the code
//! itself changes for one reason, what the machine carries and since when.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, OnceLock};

use serde_json::{Value, json};

use crate::{clock, file};

/// How long after its merge the harness writes a landing at the most,
/// in ms: 34 s, the most seen on 13 November.
const RECORDED_WITHIN: i64 = 34_000;

/// How many of the latest arrivals are read: a few weeks of switches,
/// past which a landing's own drawer is long gone.
const ARRIVALS_READ: usize = 200;

/// One arrival: when, the commit running before and the commit after;
/// none where the record can't say, which ends what is told before it.
type Arrival = Option<(i64, String, String)>;

/// What of each repository's merges the running system carries, by their
/// numbers, or of mycelium's by the time of its pin, each none where that
/// can't be told.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Carried {
    core: Option<HashSet<u64>>,
    harness: Option<HashSet<u64>>,
    mycelium: Option<i64>,
    /// When the arrival that brought each merge happened, in ms, by its
    /// repository's name and its number, where that's told.
    since: HashMap<&'static str, HashMap<u64, i64>>,
}

/// Where what runs is said: update-to-main's last answer, the core the
/// daemon kept, core's clone, and where pi-packages installs the harness.
#[derive(Debug, Clone, Copy)]
pub struct Said<'a> {
    pub switched: Option<&'a Path>,
    pub kept: &'a Path,
    pub clone: Option<&'a Path>,
    pub harness: Option<&'a Path>,
    /// The integration's record of switches, a line each.
    pub switches: Option<&'a Path>,
}

impl Carried {
    /// What the system carries, as `said` tells it: core as update-to-main
    /// says it followed once done, else as the daemon kept it, its merges
    /// and mycelium's pin read from core's clone; the harness as installed.
    /// Each done ask's core is kept for the next start.
    pub fn read(said: Said) -> Self {
        let done = said
            .switched
            .and_then(file::read)
            .and_then(|answer| followed(&answer).map(str::to_owned));
        let was = file::read(said.kept).and_then(|kept| kept["core"].as_str().map(str::to_owned));
        if let Some(core) = &done
            && was.as_ref() != Some(core)
            && let Err(why) = keep(said.kept, core)
        {
            eprintln!("agents: can't keep {}: {why}", said.kept.display());
        }
        let core = done.or(was);
        let (core, mycelium) = match (said.clone, core) {
            (Some(clone), Some(core)) => (
                git(clone, &["log", "--first-parent", "--format=%s", &core])
                    .map(|log| merged(&log)),
                git(clone, &["show", &format!("{core}:flake.lock")])
                    .and_then(|lock| pinned(&serde_json::from_str(&lock).ok()?)),
            ),
            _ => (None, None),
        };
        let harness = said.harness.and_then(|folder| {
            let commit = installed(&fs::read_link(folder.join("current")).ok()?)?;
            let log = git(
                &folder.join("mirror.git"),
                &["log", "--first-parent", "--format=%s", &commit],
            )?;
            Some(merged(&log))
        });
        let mut since = HashMap::new();
        if let (Some(clone), Some(switches)) = (said.clone, said.switches) {
            // A stray byte costs its line, never the record.
            let lines = fs::read(switches).unwrap_or_default();
            let switches = switched(&String::from_utf8_lossy(&lines));
            since.insert("core.nix", brought(&switches, between(clone)));
        }
        Self {
            core,
            harness,
            mycelium,
            since,
        }
    }

    /// When the switch that brought landing `one` happened, in ms: one of
    /// core's the machine is known to carry, where that's told. With what
    /// runs unknown, a merge a switch back took away would keep its date.
    pub fn since(&self, one: &Value) -> Option<i64> {
        let (name, number) = numbered(one);
        let number = number?;
        let at = self.since.get(name)?.get(&number).copied()?;
        let merged = match name {
            "core.nix" => self.core.as_ref(),
            "agentic-harness.pi" => self.harness.as_ref(),
            _ => None,
        };
        merged?.contains(&number).then_some(at)
    }

    /// Whether landing `one` is on the running system: one of core's or
    /// the harness's once its merge is carried, one of mycelium's once its
    /// pin is surely past it, and any other, or any where that can't be
    /// told, as it lands.
    pub fn carries(&self, one: &Value) -> bool {
        let (name, number) = numbered(one);
        let holds = |merged: &Option<HashSet<u64>>| match (merged, number) {
            (Some(merged), Some(number)) => merged.contains(&number),
            _ => true,
        };
        match name {
            "core.nix" => holds(&self.core),
            "agentic-harness.pi" => holds(&self.harness),
            "mycelium" => match (self.mycelium, clock::ms_of(&one["at"]), number) {
                (Some(pin), Some(at), Some(_)) => at - RECORDED_WITHIN <= pin,
                _ => true,
            },
            _ => true,
        }
    }

    /// One carrying exactly `merged` of core's, for a test.
    #[cfg(test)]
    pub fn of(merged: &[u64]) -> Self {
        Self {
            core: Some(merged.iter().copied().collect()),
            ..Self::default()
        }
    }

    /// One telling only that core's merge `number` reached the machine
    /// at `at`, in ms, for a test.
    #[cfg(test)]
    pub fn arrived(number: u64, at: i64) -> Self {
        Self {
            core: Some(HashSet::from([number])),
            since: HashMap::from([("core.nix", HashMap::from([(number, at)]))]),
            ..Self::default()
        }
    }
}

/// Landing `one`'s repository, by its name alone, and its merge's number,
/// where its ref names one.
fn numbered(one: &Value) -> (&str, Option<u64>) {
    let repo = one["repo"].as_str().unwrap_or("");
    let name = repo.rsplit('/').next().unwrap_or("");
    let number = one["ref"]
        .as_str()
        .and_then(|said| said.strip_prefix(name)?.strip_prefix('#'))
        .and_then(|number| number.parse::<u64>().ok());
    (name, number)
}

/// A history's folder and two of its commits, which name what lies between.
type Span = (PathBuf, String, String);

/// The subjects on the first parents from one commit to another in the
/// history at `folder`, a line each, where git says them. Kept once
/// said, since two commits' history never changes: a re-read asks git
/// only of an arrival it hasn't read before.
fn between(folder: &Path) -> impl Fn(&str, &str) -> Option<String> + '_ {
    static SAID: OnceLock<Mutex<HashMap<Span, String>>> = OnceLock::new();
    move |was, now| {
        let key = (folder.to_path_buf(), was.to_owned(), now.to_owned());
        let said = SAID.get_or_init(Mutex::default);
        if let Some(subjects) = said.lock().ok()?.get(&key) {
            return Some(subjects.clone());
        }
        let range = format!("{was}..{now}");
        let subjects = git(folder, &["log", "--first-parent", "--format=%s", &range])?;
        said.lock().ok()?.insert(key, subjects.clone());
        Some(subjects)
    }
}

/// When each merge last arrived, in ms, by its number: what each of
/// `arrivals`, in the order they ran, brought past the commit before it,
/// as `log` says the first parents between them, of the latest
/// `ARRIVALS_READ`. The latest arrival to bring one is its, since a
/// switch back that dropped it and a switch on that brought it again
/// mean it has run only since the second. Read from the newest back, so
/// an arrival the record or `log` can't say ends it: past that, a date
/// may be from before a switch back. Their own times aren't trusted to
/// order them, so one clock set wrong can't outrank every later switch.
fn brought(arrivals: &[Arrival], log: impl Fn(&str, &str) -> Option<String>) -> HashMap<u64, i64> {
    let latest = arrivals.len().saturating_sub(ARRIVALS_READ);
    let mut since = HashMap::new();
    for arrival in arrivals[latest..].iter().rev() {
        let Some((at, was, now)) = arrival else {
            break;
        };
        if was == now {
            continue;
        }
        let Some(subjects) = log(was, now) else {
            break;
        };
        for number in merged(&subjects) {
            since.entry(number).or_insert(*at);
        }
    }
    since
}

/// Each switch in the integration's record `lines`, when, the core
/// before it and the core it ran, from lines next to each other that
/// both say when and a whole core, in the order the record keeps, which
/// is the order its switches ran. A line that doesn't is a break, since
/// what ran across it isn't said, and a switch past it may have gone
/// through a switch back the pair can't see.
fn switched(lines: &str) -> Vec<Arrival> {
    let said: Vec<Option<(i64, String)>> = lines
        .lines()
        .map(|line| {
            let one = serde_json::from_str::<Value>(line).ok()?;
            let core = one["revs"]["core"].as_str().filter(|core| whole(core))?;
            Some((clock::ms_of(&one["at"])?, core.to_owned()))
        })
        .collect();
    said.windows(2)
        .map(|pair| match pair {
            [Some((_, was)), Some((at, now))] => Some((*at, was.clone(), now.clone())),
            _ => None,
        })
        .collect()
}

/// What git says in the repository at `folder`, run with `args`, where it
/// says it without failing.
fn git(folder: &Path, args: &[&str]) -> Option<String> {
    let ran = Command::new("git")
        .arg("-C")
        .arg(folder)
        .args(args)
        .output()
        .ok()
        .filter(|ran| ran.status.success())?;
    Some(String::from_utf8_lossy(&ran.stdout).into_owned())
}

/// The commit pi-packages installed, from where its `current` link
/// points, `versions/<commit>`.
fn installed(points: &Path) -> Option<String> {
    let commit = points.file_name()?.to_str()?;
    whole(commit).then(|| commit.to_owned())
}

/// When the commit core's `lock` pins mycelium at was made, in ms.
fn pinned(lock: &Value) -> Option<i64> {
    lock["nodes"]["mycelium"]["locked"]["lastModified"]
        .as_i64()
        .map(|seconds| seconds * 1000)
}

/// Whether `commit` names one whole, as forty hexadecimal digits.
fn whole(commit: &str) -> bool {
    commit.len() == 40 && commit.chars().all(|c| c.is_ascii_hexdigit())
}

/// The core update-to-main followed, from its last answer `said`, where
/// that ask is done; a failed or stopped one says nothing of what runs.
pub fn followed(said: &Value) -> Option<&str> {
    let core = said["followed"]["core"].as_str()?;
    (said["step"] == "done" && whole(core)).then_some(core)
}

/// The changes merged on first parents whose subjects are `log`, a line
/// each, by their numbers.
pub fn merged(log: &str) -> HashSet<u64> {
    log.lines()
        .filter_map(|line| line.strip_prefix("Merge pull request #"))
        .filter_map(|rest| rest.split(' ').next()?.parse().ok())
        .collect()
}

/// Keeps `core` at `kept`, written whole and renamed into place.
fn keep(kept: &Path, core: &str) -> std::io::Result<()> {
    if let Some(folder) = kept.parent() {
        fs::create_dir_all(folder)?;
    }
    let writing = kept.with_extension("json.new");
    fs::write(&writing, json!({ "core": core }).to_string())?;
    fs::rename(writing, kept)
}

#[cfg(test)]
mod tests {
    use super::*;

    const CORE: &str = "7aab2c55ccf240baa727bd4ac09b82b1079ed695";

    #[test]
    fn only_a_done_ask_says_which_core_runs() {
        let done = json!({"step": "done", "followed": {"core": CORE}});
        assert_eq!(followed(&done), Some(CORE), "a done ask's core runs");
        let failed = json!({"step": "failed", "followed": {"core": CORE}});
        assert_eq!(followed(&failed), None, "a failed ask's core may not run");
        let short = json!({"step": "done", "followed": {"core": "7aab2c55"}});
        assert_eq!(followed(&short), None, "only a whole commit names one");
    }

    #[test]
    fn a_merge_is_read_from_its_subject_on_main() {
        let log = "Merge pull request #4199 from Jitsusama/jitsusama/host-down\n\
                   fix(pi-host): stay quiet\n\
                   Merge pull request #4178 from Jitsusama/eero-narrow\n";
        assert_eq!(
            merged(log),
            HashSet::from([4199, 4178]),
            "each merge by its number, the commits beneath none"
        );
    }

    #[test]
    fn a_landing_of_cores_is_carried_only_once_its_merge_runs() {
        let carried = Carried::of(&[4177]);
        let landing = |repo: &str, at: &str| json!({"repo": repo, "ref": at});
        for (one, carries, why) in [
            (
                landing("Jitsusama/core.nix", "core.nix#4177"),
                true,
                "its merge runs",
            ),
            (
                landing("Jitsusama/core.nix", "core.nix#4178"),
                false,
                "no switch brought it",
            ),
            (
                landing("core.nix", "core.nix#4178"),
                false,
                "core named alone is core",
            ),
            (
                landing("Jitsusama/mycelium", "mycelium#154"),
                true,
                "mycelium's pin isn't told here",
            ),
            (
                landing("Jitsusama/agentic-harness.pi", "agentic-harness.pi#1521"),
                true,
                "the harness's install isn't told here",
            ),
            (
                landing("", "PLAN-20261113-KRINS0#3"),
                true,
                "a plan's item isn't a merge",
            ),
        ] {
            assert_eq!(carried.carries(&one), carries, "{why}");
        }
        assert!(
            Carried::default().carries(&landing("Jitsusama/core.nix", "core.nix#4178")),
            "with nothing known, nothing waits"
        );
    }

    #[test]
    fn a_landing_of_the_harnesss_is_carried_once_its_install_holds_its_merge() {
        let carried = Carried {
            harness: Some(HashSet::from([1521])),
            ..Carried::default()
        };
        let landing = |number: u64| {
            json!({"repo": "Jitsusama/agentic-harness.pi",
                   "ref": format!("agentic-harness.pi#{number}")})
        };
        assert!(carried.carries(&landing(1521)), "its merge is installed");
        assert!(
            !carried.carries(&landing(1523)),
            "pi-packages hasn't moved to it"
        );
        assert!(
            carried.carries(&json!({"repo": "Jitsusama/core.nix", "ref": "core.nix#4178"})),
            "core's isn't told by the harness's install"
        );
    }

    #[test]
    fn the_harness_install_is_the_commit_its_link_names() {
        let commit = "5e6e94320e16624795aa25b62713ce13557335b8";
        assert_eq!(
            installed(Path::new(&format!("versions/{commit}"))).as_deref(),
            Some(commit),
            "the version's folder is named by its commit"
        );
        assert_eq!(
            installed(Path::new("versions/next")),
            None,
            "only a commit names one"
        );
    }

    #[test]
    fn a_landing_of_myceliums_is_carried_once_cores_pin_is_its_merge_or_after() {
        // The pin, 2026-11-13T20:50:01Z, and landings written about it.
        let pin = 1_794_603_001_000;
        let carried = Carried {
            mycelium: Some(pin),
            ..Carried::default()
        };
        let landing =
            |at: &str| json!({"repo": "Jitsusama/mycelium", "ref": "mycelium#155", "at": at});
        for (at, carries, why) in [
            (
                "2026-11-13T20:40:00.000Z",
                true,
                "written well before the pin",
            ),
            (
                "2026-11-13T20:50:08.500Z",
                true,
                "written 7.5 s after, so it's the pin's own merge",
            ),
            (
                "2026-11-13T20:50:36.000Z",
                false,
                "written 35 s after, so merged after it",
            ),
            ("2026-11-13T22:26:00.000Z", false, "merged after the pin"),
        ] {
            assert_eq!(carried.carries(&landing(at)), carries, "{why}");
        }
        assert_eq!(
            pinned(&json!({"nodes": {"mycelium": {"locked": {"lastModified": 1_794_603_001}}}})),
            Some(pin),
            "the pin's time is its commit's, in ms"
        );
    }

    #[test]
    fn a_merge_arrived_with_the_switch_that_brought_it_past_the_one_before() {
        let (a, b, c) = ("a".repeat(40), "b".repeat(40), "c".repeat(40));
        // In the order they ran, one clock set back (150), and a switch to
        // the core already running, as one for the machine repository's
        // own change is.
        let arrivals = vec![
            Some((200, a.clone(), b.clone())),
            Some((250, b.clone(), b.clone())),
            Some((300, b.clone(), c.clone())),
            Some((400, c.clone(), b.clone())),
            Some((150, b.clone(), c.clone())),
        ];
        let log = |was: &str, now: &str| match (&was[..1], &now[..1]) {
            ("a", "b") => Some("Merge pull request #10 from x/y\nfix(z): w\n".to_owned()),
            ("b", "c") => Some(
                "Merge pull request #12 from x/y\nMerge pull request #11 from x/y\n".to_owned(),
            ),
            ("c", "b") => Some(String::new()),
            _ => panic!("asked only between the switches next to each other: {was}..{now}"),
        };
        assert_eq!(
            brought(&arrivals, log),
            HashMap::from([(10, 200), (11, 150), (12, 150)]),
            "each merge by the latest switch to run that brought it, after a switch back \
             took it away, whatever its clock said"
        );
    }

    #[test]
    fn nothing_is_told_from_before_what_the_record_or_git_cant_say() {
        let (a, b, c) = ("a".repeat(40), "b".repeat(40), "c".repeat(40));
        let log = |was: &str, now: &str| match (&was[..1], &now[..1]) {
            ("a", "b") => Some("Merge pull request #10 from x/y\n".to_owned()),
            ("b", "c") => Some("Merge pull request #11 from x/y\n".to_owned()),
            _ => None,
        };
        let unread = vec![
            Some((200, a.clone(), b.clone())),
            None,
            Some((300, b.clone(), c.clone())),
        ];
        assert_eq!(
            brought(&unread, log),
            HashMap::from([(11, 300)]),
            "a switch back may hide behind a line that can't be read"
        );
        let unsaid = vec![
            Some((200, a.clone(), b.clone())),
            Some((250, b.clone(), a.clone())),
            Some((300, b.clone(), c)),
        ];
        assert_eq!(
            brought(&unsaid, log),
            HashMap::from([(11, 300)]),
            "nor is a merge dated across a pair git can't say"
        );
    }

    #[test]
    fn only_the_latest_arrivals_are_read() {
        let arrivals: Vec<Arrival> = (0..=ARRIVALS_READ)
            .map(|at| {
                Some((
                    i64::try_from(at).unwrap_or(0),
                    format!("{at}"),
                    "x".to_owned(),
                ))
            })
            .collect();
        let asked = std::cell::RefCell::new(Vec::new());
        let _ = brought(&arrivals, |was, _| {
            asked.borrow_mut().push(was.to_owned());
            Some(String::new())
        });
        assert_eq!(asked.borrow().len(), ARRIVALS_READ, "a bounded record");
        assert!(
            !asked.borrow().contains(&"0".to_owned()),
            "the oldest goes unread"
        );
    }

    #[test]
    fn a_merge_whose_history_cant_be_read_arrived_at_no_time_told() {
        let arrivals = vec![Some((200, "a".repeat(40), "b".repeat(40)))];
        assert!(
            brought(&arrivals, |_, _| None).is_empty(),
            "git said nothing"
        );
    }

    #[test]
    fn a_switch_is_read_from_lines_next_to_each_other_that_say_when_and_the_core_whole() {
        let (a, b, c, d) = (
            "a".repeat(40),
            "b".repeat(40),
            "c".repeat(40),
            "d".repeat(40),
        );
        let line = |at: &str, core: &str| json!({"at": at, "revs": {"core": core}}).to_string();
        let lines = [
            line("2026-11-18T06:05:43-05:00", &a),
            line("2026-11-18T07:00:00-05:00", &b),
            line("2026-11-18T07:18:33-05:00", "a3d5300"),
            "not json".to_owned(),
            json!({"revs": {"core": CORE}}).to_string(),
            line("2026-11-18T08:00:00-05:00", &c),
            line("2026-11-18T09:00:00-05:00", &d),
        ]
        .join("\n");
        assert_eq!(
            switched(&lines),
            vec![
                Some((1_795_003_200_000, a.clone(), b.clone())),
                None,
                None,
                None,
                None,
                Some((1_795_010_400_000, c.clone(), d.clone())),
            ],
            "each pair of whole lines, and a break for each across a line that isn't"
        );
    }

    #[test]
    fn a_landing_says_since_when_the_machine_has_had_its_merge() {
        let carried = Carried {
            core: Some(HashSet::from([4177, 4178])),
            since: HashMap::from([("core.nix", HashMap::from([(4177, 200)]))]),
            ..Carried::default()
        };
        let landing = |repo: &str, at: &str| json!({"repo": repo, "ref": at});
        for (one, since, why) in [
            (
                landing("Jitsusama/core.nix", "core.nix#4177"),
                Some(200),
                "core's by its switch",
            ),
            (
                landing("Jitsusama/core.nix", "core.nix#4178"),
                None,
                "no switch told brought it",
            ),
            (
                landing("Jitsusama/core.nix", "agentic-harness.pi#1802"),
                None,
                "a ref of another repository's",
            ),
            (
                landing("", "PLAN-20261113-KRINS0#3"),
                None,
                "a plan's item isn't a merge",
            ),
        ] {
            assert_eq!(carried.since(&one), since, "{why}");
        }
        let rolled_back = Carried {
            core: Some(HashSet::from([4176])),
            ..carried.clone()
        };
        assert_eq!(
            rolled_back.since(&landing("Jitsusama/core.nix", "core.nix#4177")),
            None,
            "a merge a switch back took away isn't on the machine since any time"
        );
        let unknown = Carried {
            core: None,
            ..carried
        };
        assert_eq!(
            unknown.since(&landing("Jitsusama/core.nix", "core.nix#4177")),
            None,
            "with what runs unknown, a date may be from before a switch back"
        );
    }

    /// Runs git in `folder` as a test's own, with no settings of the
    /// person's, and answers what it printed.
    fn git_here(folder: &Path, args: &[&str]) -> String {
        let ran = Command::new("git")
            .arg("-C")
            .arg(folder)
            .args(args)
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_AUTHOR_NAME", "t")
            .env("GIT_AUTHOR_EMAIL", "t@t")
            .env("GIT_COMMITTER_NAME", "t")
            .env("GIT_COMMITTER_EMAIL", "t@t")
            .output()
            .expect("git runs");
        assert!(ran.status.success(), "git {args:?}: {ran:?}");
        String::from_utf8_lossy(&ran.stdout).trim().to_owned()
    }

    #[test]
    fn the_machine_says_since_when_from_its_switches_in_real_history() {
        let folder = std::env::temp_dir().join(format!("agents-since-{}", std::process::id()));
        let _ = fs::remove_dir_all(&folder);
        let repo = folder.join("core");
        fs::create_dir_all(&repo).expect("a repository's folder");
        git_here(&repo, &["init", "-q"]);
        let commit = |subject: &str| {
            git_here(&repo, &["commit", "-q", "--allow-empty", "-m", subject]);
            git_here(&repo, &["rev-parse", "HEAD"])
        };
        let first = commit("init");
        let ten = commit("Merge pull request #10 from x/ten");
        let eleven = commit("Merge pull request #11 from x/eleven");

        let switches = folder.join("switches.jsonl");
        let line = |at: i64, core: &str| json!({"at": at, "revs": {"core": core}}).to_string();
        fs::write(
            &switches,
            format!("{}\n{}\n", line(1_000, &first), line(2_000, &eleven)),
        )
        .expect("the switches");
        // The harness's install is the same history, current naming `ten`,
        // so its merge is carried but given no time.
        let harness = folder.join("harness");
        fs::create_dir_all(&harness).expect("its folder");
        std::os::unix::fs::symlink(&repo, harness.join("mirror.git")).expect("its mirror");
        std::os::unix::fs::symlink(format!("versions/{ten}"), harness.join("current"))
            .expect("its current");
        let answer = folder.join("last.json");
        let read = |core: &str| {
            fs::write(
                &answer,
                json!({"step": "done", "followed": {"core": core}}).to_string(),
            )
            .expect("an answer");
            Carried::read(Said {
                switched: Some(&answer),
                kept: &folder.join("kept.json"),
                clone: Some(&repo),
                harness: Some(&harness),
                switches: Some(&switches),
            })
        };
        let landing = |repo: &str, number: u64| {
            let name = repo.rsplit('/').next().unwrap_or("");
            json!({"repo": repo, "ref": format!("{name}#{number}")})
        };

        let running = read(&eleven);
        let rolled_back = read(&first);
        fs::remove_dir_all(&folder).expect("the test's folder gone");

        for (one, since, why) in [
            (
                landing("Jitsusama/core.nix", 10),
                Some(2_000),
                "core's by its switch",
            ),
            (
                landing("Jitsusama/core.nix", 11),
                Some(2_000),
                "both came in one",
            ),
            (
                landing("Jitsusama/agentic-harness.pi", 10),
                None,
                "the harness's swaps aren't told",
            ),
        ] {
            assert_eq!(running.since(&one), since, "{why}");
        }
        assert!(
            running.carries(&landing("Jitsusama/agentic-harness.pi", 10)),
            "though its install carries it"
        );
        assert_eq!(
            rolled_back.since(&landing("Jitsusama/core.nix", 11)),
            None,
            "once core runs the commit before, its merge isn't on the machine"
        );
    }

    #[test]
    fn a_done_asks_core_is_kept_for_when_the_answer_is_gone() {
        let folder = std::env::temp_dir().join(format!("agents-carried-{}", std::process::id()));
        let switched = folder.join("last.json");
        let kept = folder.join("agents").join("carried.json");
        fs::create_dir_all(&folder).expect("a folder for the test");
        fs::write(
            &switched,
            json!({"step": "done", "followed": {"core": CORE}}).to_string(),
        )
        .expect("an answer to read");
        let clone = folder.join("core.git");
        let _ = Carried::read(Said {
            switched: Some(&switched),
            kept: &kept,
            clone: Some(&clone),
            harness: None,
            switches: None,
        });
        fs::remove_file(&switched).expect("the answer gone, as at a boot");
        let said = file::read(&kept).expect("the core kept");
        assert_eq!(said["core"], CORE, "the done ask's core is kept");
        fs::remove_dir_all(&folder).expect("the test's folder gone");
    }
}
