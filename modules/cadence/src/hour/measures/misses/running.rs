//! Whether a merged check runs on this machine yet, since a merge isn't a
//! fix on Joel's machine: at 21:10 on 19 November the card told him his
//! 19:05 miss, the background pi host's crash, was fixed on core #5800's
//! merge, while the last switch predated it and the host still loaded the
//! older runtime's pi. Whatever cadence can't read counts as not running,
//! so a blind hour never tells him fixed. Long because each repository's
//! running revision is read its own way, from the switches, core's lock,
//! the harness's install and the hosts' patches, and a change's verdict
//! weighs them together, each with the reason it gives when it can't read.

mod conversations;
mod hosts;
mod program;
mod resumed;
mod services;

use conversations::conversations;
use hosts::hosts;

use std::collections::{BTreeMap, BTreeSet};
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::{OnceLock, mpsc};
use std::time::{Duration, Instant};

use serde_json::Value;

use super::{Checkouts, Named, main_of, merged_numbers};
use crate::{command, write_whole};
pub use resumed::hosts_started;

/// What a miss's line says between its change and why, after its check's
/// note, when the change merged and doesn't run here yet. The card reads
/// it to tell the miss fixed again once it runs, and the owner's note to
/// ask when it will rather than which check.
pub const UNRUN: &str = " merged, but ";

/// How each reason cadence's own blindness gives begins, so the owner isn't
/// asked about what only the hour can mend.
pub const UNSEEN: &str = "cadence can't ";

/// What a host loads can't be read from a fix to how core builds its
/// runtime, ever, since a runtime built elsewhere copies only its patches.
const LOADS_UNSEEN: &str = "cadence can't tell what a pi host loads";

/// The reasons of cadence's blindness that no fetch or listing mends, so
/// a hold on one lasts rather than passing with the hour. Each is given
/// by one path alone: a reason a passing failure also gives would close a
/// drag one hour and file it again the next.
pub const LASTING: [&str; 1] = [LOADS_UNSEEN];

/// The repositories whose running revision cadence reads: core's and
/// World's from the last switch, mycelium's as that switch's core pins it,
/// and the harness's from the version installed. A change elsewhere,
/// deployed apart, is followed no further than its merge.
pub(super) const RUN_HERE: &[&str] = &["core", "world", "mycelium", "harness"];

/// The repositories that run as core's `flake.lock` pins them, by the name
/// of core's input for each.
const PINNED_BY_CORE: &[&str] = &["mycelium"];

/// Where core keeps the patches a pi host's runtime applies, which a host
/// loads only once it restarts onto a runtime built with them, apart from
/// any switch. Whoever builds a runtime chooses which it applies, from
/// which core, so neither a switch nor a lock says.
const PATCHES: &str = "modules/pi/host-patches/";

/// Where a runtime keeps a copy of each patch it applied, by its name, as
/// core's `hosted-pi.nix` does: the one record of what a host loads.
const APPLIED: &str = "share/pi-host/patches";

/// The pi hosts' own unit, which a switch leaves a running host keeping
/// (its `X-SwitchMethod` is `keep-old`) until the host next starts.
const HOST_UNIT: &str = "modules/pi/host.nix";

/// How core builds the runtime its own checks run, applying its patches.
const HOSTED_PI: &str = "modules/pi/hosted-pi.nix";

/// The patches a runtime applied, by name, each as its copy reads.
type Applied = BTreeMap<String, String>;

/// A pi host as listed, before its copies are read: the runtime it loads,
/// when it started, and how it stands.
type Listed = (Option<std::path::PathBuf>, Option<i64>, Standing);

/// How a pi host stands: running, stopped, or failed, which systemd leaves
/// it at once it restarts past its limit, until someone resets it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum Standing {
    #[default]
    Running,
    Stopped,
    Failed,
}

/// The states of each patch of a change, by name, that carry it: its
/// text, or `None` for one the change removed.
type Wanted = BTreeMap<String, Vec<Option<String>>>;

/// A pi host as cadence reads it: the patches its runtime applied, and
/// when it started, each `None` where that couldn't be read, and how it
/// stands: a stopped one has no process to keep an older unit.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Host {
    applied: Option<Applied>,
    started: Option<i64>,
    standing: Standing,
}

/// Which of `merged` don't run here yet, each with why. A change runs once
/// its repository's running revision carries its merge, a core change to
/// the hosts' patches once every running host and conversation also
/// loads a runtime that applied each as merged or since, one to the hosts'
/// unit once every host and conversation started after the switch that
/// carried it, and a harness change to the hosts' program once every host
/// and conversation started after it was installed. A change seen
/// running is kept in `folder`'s `ran.json` and never read again, so a
/// closed miss isn't reopened, and told fixed twice, by an hour that
/// can't see for a moment: a host restarting at :10, or a switch from a
/// branch. When the harness installed first carried a program change is
/// kept in its `installed.json` until an hour sees it not carried, or
/// pi-packages marks a version lacking it left since, so a later install
/// doesn't hold it open again.
#[must_use]
pub fn unrun(
    merged: &BTreeSet<Named>,
    repos: &Checkouts,
    switches: &str,
    folder: &Path,
) -> BTreeMap<Named, &'static str> {
    let ran_at = folder.join("ran.json");
    let ran: BTreeSet<Named> = std::fs::read_to_string(&ran_at)
        .ok()
        .and_then(|kept| serde_json::from_str(&kept).ok())
        .unwrap_or_default();
    let firsts_at = folder.join("installed.json");
    let firsts: BTreeMap<String, i64> = std::fs::read_to_string(&firsts_at)
        .ok()
        .and_then(|kept| serde_json::from_str(&kept).ok())
        .unwrap_or_default();
    let mut now_firsts = BTreeMap::new();
    let revs = running(switches, repos);
    let mut hosts: Option<Option<Vec<Host>>> = None;
    let mut talks: Option<Option<Vec<Host>>> = None;
    let running_services = std::cell::OnceCell::new();
    let mut unrun = BTreeMap::new();
    for change in merged.iter().filter(|change| !ran.contains(*change)) {
        let Some(path) = repos.get(&change.repo) else {
            continue;
        };
        let path = path.to_string_lossy();
        let key = format!("{} #{}", change.repo, change.number);
        let rev = revs.get(&change.repo).map(String::as_str);
        let walked = merge_judged(&change.repo, rev, &path, change.number);
        let left = walked.is_err();
        let merge = walked.ok().flatten();
        let why = judge(
            &change.repo,
            rev,
            merge.as_deref(),
            |sha, rev| carries(&path, sha, rev),
            |sha| changed(&path, sha),
            |sha| {
                let files = program::files(&path, sha);
                if files.is_none() {
                    eprintln!(
                        "cadence: can't follow the pi host's program at {sha}, so each harness \
                         change waits on every pi host and conversation"
                    );
                }
                files
            },
            |sha, reaches| {
                let read = hosts.get_or_insert_with(read_hosts).as_deref();
                let talks = talks.get_or_insert_with(read_talks).as_deref();
                let carried = |rev: &str| carries(&path, sha, rev) == Some(true);
                let since = if reaches.program {
                    let mirror = crate::axes::landings::harness_mirror();
                    let named = revs.get(&change.repo).map(String::as_str);
                    first_installed(
                        &firsts,
                        &mut now_firsts,
                        &key,
                        |at| stepped_back(program::left_since(&mirror, at), carried),
                        || program::swapped(&mirror, named),
                    )
                } else if reaches.unit {
                    first_switched(switches, carried)
                } else {
                    None
                };
                // Walked only once a host keeps copies to compare with it.
                let merged = std::cell::OnceCell::new();
                lacking(read, talks, &reaches, since, |applied| {
                    merged
                        .get_or_init(|| versions(&path, sha, &reaches.patches))
                        .as_ref()
                        .map(|wanted| loads(applied, wanted))
                })
            },
        );
        // A change its revision carries runs only once each service it
        // reaches runs the build its unit names, read once an hour.
        let why = why.or_else(|| served(&change.repo, &path, merge.as_deref(), &running_services));
        // A walk left at its time says so, since its owner can't find a
        // merge that is there, only wait for a quieter hour.
        let why = said_of_walk(why, left);
        keep_unseen(&firsts, &mut now_firsts, &key, why);
        if let Some(why) = why {
            unrun.insert(change.clone(), why);
        }
    }
    let now_ran: BTreeSet<Named> = merged
        .iter()
        .filter(|change| !unrun.contains_key(*change))
        .chain(&ran)
        .cloned()
        .collect();
    keep_changed(&ran_at, &ran, &now_ran, "what runs");
    keep_changed(
        &firsts_at,
        &firsts,
        &now_firsts,
        "when changes were installed",
    );
    unrun
}

/// Why a change of `repo`, merged as `merge` in the checkout at `path`,
/// doesn't run yet though its revision carries it, if it doesn't: a
/// service it reaches runs another build than its unit names, as
/// `services::lack` says, with what runs read into `read` once an hour.
fn served(
    repo: &str,
    path: &str,
    merge: Option<&str>,
    read: &std::cell::OnceCell<Option<Vec<services::Service>>>,
) -> Option<&'static str> {
    let files = match (repo, merge) {
        ("core", Some(sha)) => changed(path, sha),
        _ => None,
    };
    let reached = services::reached(repo, files.as_deref());
    if reached.is_empty() {
        return None;
    }
    let read = read.get_or_init(services::read);
    services::lack(&reached, read.as_deref())
}

/// Writes `now` whole to `at` where it differs from `was`, what an hour
/// read there, saying `what` it couldn't keep.
fn keep_changed<T: serde::Serialize + PartialEq>(at: &Path, was: &T, now: &T, what: &str) {
    if now != was
        && let Err(error) = serde_json::to_string(now)
            .map_err(std::io::Error::from)
            .and_then(|kept| write_whole(at, &kept))
    {
        eprintln!("cadence: keeping {what} in {}: {error}", at.display());
    }
}

/// Why a change merged here is held open when no merge of it is on main.
const UNFOUND: &str = "cadence can't find its merge";

/// Why a change is held open when its walk for its merge was left at its
/// time, as on a loaded machine: the hour's own blindness, as `UNSEEN`
/// begins it, so the change's owner isn't asked about it.
const LEFT: &str = "cadence can't walk main to its merge in the time it has";

/// `why`, said as the walk running out of time where it was `left` and
/// found no merge.
fn said_of_walk(why: Option<&'static str>, left: bool) -> Option<&'static str> {
    why.map(|why| if left && why == UNFOUND { LEFT } else { why })
}

/// Why the harness installed doesn't hold a change, which ends the time
/// kept for its first install.
const NOT_CARRIED: &str = "the harness installed doesn't carry it";

/// When the harness installed first carried the change kept as `key`: as
/// `firsts`, last hour's, kept it, else as `swapped` reads `current` now,
/// kept in `now_firsts` for the next hour. An hour that doesn't see it
/// carried keeps nothing, so a step back counts from the step forward; a
/// step back and forward between two hours does too, once `stepped_back`
/// says `current` named a version lacking it since the time kept.
fn first_installed(
    firsts: &BTreeMap<String, i64>,
    now_firsts: &mut BTreeMap<String, i64>,
    key: &str,
    stepped_back: impl FnOnce(i64) -> bool,
    swapped: impl FnOnce() -> Option<i64>,
) -> Option<i64> {
    let since = firsts
        .get(key)
        .copied()
        .filter(|&at| !stepped_back(at))
        .or_else(swapped);
    if let Some(at) = since {
        now_firsts.insert(key.to_owned(), at);
    }
    since
}

/// Keeps the first install `firsts` holds for `key` through an hour that
/// didn't reach it, as one that couldn't read git, so a later install
/// doesn't hold a conversation started on the first; but not once the
/// change ran, judged `why` as `None`, or the hour saw it not carried.
fn keep_unseen(
    firsts: &BTreeMap<String, i64>,
    now_firsts: &mut BTreeMap<String, i64>,
    key: &str,
    why: Option<&str>,
) {
    if why.is_some_and(|why| why != NOT_CARRIED)
        && !now_firsts.contains_key(key)
        && let Some(&at) = firsts.get(key)
    {
        now_firsts.insert(key.to_owned(), at);
    }
}

/// Whether `current` named a version lacking the change since the time
/// kept, from the versions pi-packages marks it `left` since: one that
/// isn't `carried`, or whose name isn't a commit's, so nothing but a
/// commit reaches git; or marks that couldn't be read.
fn stepped_back(left: Option<Vec<String>>, carried: impl Fn(&str) -> bool) -> bool {
    left.is_none_or(|left| left.iter().any(|rev| !is_rev(rev) || !carried(rev)))
}

/// Why the pi hosts lack a change that `reaches` them, as `hosts_lack`
/// says, with each conversation's process, `talks`, read beside the
/// `hosts`, since each runs their program, unit and runtime too, and
/// `None` when they couldn't be listed. A host lacking it outranks the
/// conversations unread, so its owner is asked.
fn lacking(
    hosts: Option<&[Host]>,
    talks: Option<&[Host]>,
    reaches: &Reaches,
    since: Option<i64>,
    loads: impl Fn(&Applied) -> Option<bool>,
) -> Option<&'static str> {
    let Some(talks) = talks else {
        return hosts_lack(hosts, reaches, since, loads)
            .or(Some("cadence can't list the pi conversations"));
    };
    let all: Option<Vec<Host>> = hosts.map(|hosts| hosts.iter().chain(talks).cloned().collect());
    hosts_lack(all.as_deref(), reaches, since, loads)
}

/// What a change reaches that only a host's restart brings in: the
/// patches the hosts' runtime applies, by path, or their unit, of core's,
/// or their program, of the harness's.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Reaches {
    patches: Vec<String>,
    unit: bool,
    program: bool,
}

/// Why a change of `repo` merged as `merge` doesn't run here yet, if it
/// doesn't: `rev` is its repository's running revision, `carries` says
/// whether a revision holds a commit (`None` for a revision cadence doesn't
/// have), `changed` the files a merge changed, `program` the harness's
/// files the hosts' program loads at a commit, any change reaching it
/// when cadence can't follow it, and `hosts` why the pi
/// hosts lack a change that reaches them. What cadence couldn't read counts
/// as not running.
fn judge(
    repo: &str,
    rev: Option<&str>,
    merge: Option<&str>,
    carries: impl Fn(&str, &str) -> Option<bool>,
    changed: impl Fn(&str) -> Option<String>,
    program: impl FnOnce(&str) -> Option<BTreeSet<String>>,
    hosts: impl FnOnce(&str, Reaches) -> Option<&'static str>,
) -> Option<&'static str> {
    if !RUN_HERE.contains(&repo) {
        return None;
    }
    let Some(rev) = rev else {
        return Some("cadence can't see what runs here");
    };
    let Some(sha) = merge else {
        return Some(UNFOUND);
    };
    match carries(sha, rev) {
        None => return Some("cadence can't find the revision running here"),
        Some(false) if repo == "harness" => return Some(NOT_CARRIED),
        Some(false) => return Some("the last switch doesn't carry it"),
        Some(true) => {}
    }
    if repo != "core" && repo != "harness" {
        return None;
    }
    let Some(files) = changed(sha) else {
        return Some("cadence can't read what it changed");
    };
    if repo == "harness" {
        // A program cadence can't follow may load anything, so every change
        // waits on the hosts, rather than none closing until it's followed.
        let reaches = Reaches {
            program: program(sha)
                .is_none_or(|loaded| files.lines().any(|file| loaded.contains(file))),
            ..Reaches::default()
        };
        return reaches.program.then(|| hosts(sha, reaches)).flatten();
    }
    let reaches = Reaches {
        patches: files
            .lines()
            .filter(|file| {
                file.starts_with(PATCHES)
                    && std::path::Path::new(file)
                        .extension()
                        .is_some_and(|it| it == "patch")
            })
            .map(str::to_owned)
            .collect(),
        unit: files.lines().any(|file| file == HOST_UNIT),
        program: false,
    };
    let lacking = (!reaches.patches.is_empty() || reaches.unit)
        .then(|| hosts(sha, reaches))
        .flatten();
    // A fix to how core builds its runtime can't be seen in a runtime built
    // elsewhere, which copies only its patches, so it isn't told fixed;
    // `changed` leaves the build out where it only listed the patches. What
    // the hosts lack beside it, or an hour can't read of them, speaks
    // first, as a reason a passing failure gives must never last. The
    // build alone never asks them: it never reads as run, so any host
    // failing in its four weeks, for whatever cause, would ask its owner,
    // and a blind hour would close and refile that ask.
    if files.lines().any(|file| file == HOSTED_PI) {
        return lacking.or(Some(LOADS_UNSEEN));
    }
    lacking
}

/// Why the pi hosts lack a change that `reaches` them, if they do: each
/// host's runtime must have applied its patches, as `loads` says of what a
/// runtime applied (`None` when it can't tell), and each must have started
/// after `since`: when a switch first carried a change to their unit, or
/// `current` last moved to the version carrying one to their program,
/// `None` when cadence found neither. `hosts` holds each conversation's
/// process too, as `lacking` gives them, and with none running a program
/// change lacks nothing. A host failed and left so lacks anything
/// reaching the hosts until it's reset. Something lacking outranks what
/// can't be read, so the owner is asked.
fn hosts_lack(
    hosts: Option<&[Host]>,
    reaches: &Reaches,
    since: Option<i64>,
    loads: impl Fn(&Applied) -> Option<bool>,
) -> Option<&'static str> {
    let runtime = !reaches.patches.is_empty();
    let Some(hosts) = hosts else {
        return Some("cadence can't list the pi hosts");
    };
    // With nothing running, the next to start loads the harness installed.
    if hosts.is_empty() {
        return (runtime || reaches.unit).then_some("no pi host runs to load it");
    }
    let loaded: Vec<Option<bool>> = hosts
        .iter()
        .map(|host| host.applied.as_ref().and_then(&loads))
        .collect();
    if runtime && loaded.contains(&Some(false)) {
        return Some("the pi hosts don't load it yet");
    }
    // A failed host runs nothing, and judging it as stopped would close a
    // fix whose own fault failed it, so its owner is asked.
    if hosts.iter().any(|host| host.standing == Standing::Failed) {
        return Some("a pi host failed and waits on a reset");
    }
    // A stopped host keeps no unit: it starts on the one switched in.
    let running = || {
        hosts
            .iter()
            .filter(|host| host.standing == Standing::Running)
    };
    let restarts = reaches.unit || reaches.program;
    if restarts
        && let Some(at) = since
        && running().any(|host| host.started.is_some_and(|started| started < at))
    {
        return Some(if reaches.program {
            "a pi host or conversation runs the harness from before it"
        } else {
            "a pi host or conversation keeps its unit from before it"
        });
    }
    // Git failing, a host between processes or a runtime keeping no copies
    // each mends with a later hour or restart, so this passes.
    if runtime && loaded.contains(&None) {
        return Some("cadence can't tell whether a pi host applied it");
    }
    // Only `current` moving as the hour reads it leaves no install time.
    if reaches.program && since.is_none() {
        return Some("cadence can't tell when the harness carrying it was installed");
    }
    if restarts && (since.is_none() || running().any(|host| host.started.is_none())) {
        return Some("cadence can't tell when a pi host or conversation started");
    }
    None
}

/// Whether `text` is a commit's name git can only read as one: hex, at
/// least seven long, so nothing from a state file reaches git as an option.
fn is_rev(text: &str) -> bool {
    (7..=40).contains(&text.len()) && text.bytes().all(|b| b.is_ascii_hexdigit())
}

/// The revision running here of each repository cadence follows: the last
/// switch's, while the system it built still runs, each that core pins, and
/// the harness installed.
pub(super) fn running(switches: &str, repos: &Checkouts) -> BTreeMap<String, String> {
    let last = switches
        .lines()
        .rev()
        .find_map(|line| serde_json::from_str::<Value>(line).ok());
    let mut revs = last
        .as_ref()
        .filter(|last| still_runs(last["generation"].as_u64(), &system_profile()))
        .map(switched)
        .unwrap_or_default();
    let core = revs.get("core").cloned();
    if let (Some(core), Some(path)) = (core, repos.get("core")) {
        let path = path.to_string_lossy();
        let lock = command("git", &["-C", &path, "show", &format!("{core}:flake.lock")]);
        for input in PINNED_BY_CORE {
            if let Some(rev) = lock.as_deref().ok().and_then(|lock| input_pin(lock, input)) {
                revs.insert((*input).to_owned(), rev);
            }
        }
    }
    if let Some(installed) = installed() {
        revs.insert("harness".to_owned(), installed);
    }
    revs
}

/// Whether the system the switch `line` of integration's `switches.jsonl`
/// built carries `merge` of `repo`: core's and World's as the switch names
/// their revisions, and each core pins as that switch's core locks it, so
/// a pin is what carries a change, never the switch alone. `None` where
/// cadence can't say: no checkout, no revision named or pinned, or one the
/// checkout lacks.
pub(in crate::hour::measures) fn holds(
    line: &Value,
    repo: &str,
    merge: &str,
    repos: &Checkouts,
) -> Option<bool> {
    let revs = switched(line);
    let rev = if PINNED_BY_CORE.contains(&repo) {
        let core = repos.get("core")?.to_string_lossy();
        let at = format!("{}:flake.lock", revs.get("core")?);
        input_pin(&command("git", &["-C", &core, "show", &at]).ok()?, repo)?
    } else {
        revs.get(repo)?.clone()
    };
    carries(&repos.get(repo)?.to_string_lossy(), merge, &rev)
}

/// The system generation running here, while the system profile is what
/// runs; `None` otherwise, as after a boot into an older generation.
pub(in crate::hour::measures) fn running_generation() -> Option<u64> {
    let (generation, same) = system_profile();
    generation.filter(|_| same)
}

/// The revisions a switch was built from, by repository, from a line of
/// integration's `switches.jsonl`.
fn switched(switch: &Value) -> BTreeMap<String, String> {
    switch["revs"]
        .as_object()
        .map(|revs| {
            revs.iter()
                .filter_map(|(repo, rev)| Some((repo.clone(), rev.as_str()?)))
                .filter(|(_, rev)| is_rev(rev))
                .map(|(repo, rev)| (repo, rev.to_owned()))
                .collect()
        })
        .unwrap_or_default()
}

/// The system profile's generation, and whether the running system is it.
fn system_profile() -> (Option<u64>, bool) {
    let profile = Path::new("/nix/var/nix/profiles/system");
    let generation = std::fs::read_link(profile).ok().and_then(|link| {
        link.to_str()?
            .strip_prefix("system-")?
            .strip_suffix("-link")?
            .parse()
            .ok()
    });
    let same = std::fs::canonicalize(profile)
        .ok()
        .zip(std::fs::canonicalize("/run/current-system").ok())
        .is_some_and(|(profile, current)| profile == current);
    (generation, same)
}

/// Whether the switch to `generation` is what runs: the system profile is
/// at it and the running system is the profile's. A switch back, or a boot
/// into an older generation, leaves the record naming what doesn't run.
fn still_runs(generation: Option<u64>, (profile, same): &(Option<u64>, bool)) -> bool {
    *same && generation.is_some() && generation == *profile
}

/// When a switch first carried a change, as `carries` says of its core, in
/// seconds; `None` when no switch line that can be read did.
fn first_switched(switches: &str, carries: impl Fn(&str) -> bool) -> Option<i64> {
    switches
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .filter(|switch| {
            switch["revs"]["core"]
                .as_str()
                .is_some_and(|rev| is_rev(rev) && carries(rev))
        })
        .find_map(|switch| {
            let at: jiff::Timestamp = switch["at"].as_str()?.parse().ok()?;
            Some(at.as_second())
        })
}

/// The harness's commit installed, as the name of the version pi-packages'
/// `current` points at, less the `+<seconds>` a rebuild of the same commit
/// adds.
fn installed() -> Option<String> {
    let mirror = crate::axes::landings::harness_mirror();
    let current = std::fs::read_link(mirror.parent()?.join("current")).ok()?;
    let name = program::commit_of(&current.file_name()?.to_string_lossy()).to_owned();
    is_rev(&name).then_some(name)
}

/// `merge_of`, only where `judge` reads it, since a merge that isn't found
/// costs a walk to main's root.
fn merge_judged(
    repo: &str,
    rev: Option<&str>,
    path: &str,
    number: u64,
) -> Result<Option<String>, Duration> {
    if rev.is_some() && RUN_HERE.contains(&repo) {
        merge_of(path, number)
    } else {
        Ok(None)
    }
}

/// The commit on `path`'s main that merged pull request `number`, by its
/// subject as `merged_numbers` reads it, since git's grep matches a line of
/// any commit's body too. The log is read as it comes and left at the first
/// merge found, since git would otherwise read every commit's message to
/// main's root: at 05:10 on 23 November, World's #2511610 had an hour's
/// run reading 920 MB/s at 3.7 GB for over ten minutes, every hour, on a
/// machine already short of memory.
fn merge_of(path: &str, number: u64) -> Result<Option<String>, Duration> {
    let first = *WALKS_FROM.get_or_init(Instant::now);
    let left = WALKS_FOR.saturating_sub(first.elapsed());
    // Once the run's time is spent, no git is started only to be stopped.
    if left.is_zero() {
        eprintln!("cadence: no time left this run to walk {path} for #{number}'s merge");
        return Err(left);
    }
    merge_within(path, number, WALK_FOR.min(left))
}

/// When this run's first walk for a merge began, so every walk after it
/// shares `WALKS_FOR`.
static WALKS_FROM: OnceLock<Instant> = OnceLock::new();

/// How long all of a run's walks for merges may read together: one hour
/// may hold several merges not yet on main, and walks run one after
/// another under the hour's half-hour limit.
const WALKS_FOR: Duration = Duration::from_secs(300);

/// How long a walk for a merge reads before it's left: a merge not yet on
/// main is walked for to its root, which took 41 s on an idle machine and
/// many minutes on a loaded one, every hour. A merge not found in time
/// reads as one cadence ran out of time for, which keeps its miss open,
/// never closed.
const WALK_FOR: Duration = Duration::from_secs(60);

/// `merge_of`, left once `within` has passed, which it then answers as
/// its error. The log is read on a thread of its own, so a walk with
/// nothing to say for a while can still be left at its time.
fn merge_within(path: &str, number: u64, within: Duration) -> Result<Option<String>, Duration> {
    let mut log = match walk(path, number).spawn() {
        Ok(log) => log,
        Err(e) => {
            eprintln!("cadence: can't walk {path} for #{number}'s merge: {e}");
            return Ok(None);
        }
    };
    let (sent, found) = mpsc::channel();
    if let Some(out) = log.stdout.take() {
        std::thread::spawn(move || {
            // Read as bytes, since a subject that isn't UTF-8 above the
            // merge would otherwise end the read before it.
            let merge = BufReader::new(out)
                .split(b'\n')
                .map_while(Result::ok)
                .find_map(|line| subject_merging(&String::from_utf8_lossy(&line), number));
            sent.send(merge).ok();
        });
    }
    // Said, since a merge read unfound keeps its miss open on the card. A
    // reader that ended without a word read nothing, not out of time.
    let merge = match found.recv_timeout(within) {
        Ok(merge) => Ok(merge),
        Err(mpsc::RecvTimeoutError::Timeout) => {
            eprintln!("cadence: #{number}'s merge walk in {path} left at {within:?}");
            Err(within)
        }
        Err(mpsc::RecvTimeoutError::Disconnected) => {
            eprintln!("cadence: #{number}'s merge walk in {path} ended unread");
            Ok(None)
        }
    };
    // A walk that already ended has nothing to stop, and one stopped here is
    // only reaped, so neither result says anything the merge found doesn't;
    // its end closes the pipe, which ends the reading thread too.
    log.kill().ok();
    log.wait().ok();
    merge
}

/// The walk down `path`'s main for the merge of `number`. Each record is
/// flushed as it's found, since an inherited `GIT_FLUSH=0` would hold the
/// merge in git's buffer until the walk reached main's root.
fn walk(path: &str, number: u64) -> Command {
    let mut git = Command::new("git");
    git.args([
        "-C",
        path,
        "log",
        "--first-parent",
        "--format=%H %s",
        "-E",
        &format!("--grep=^Merge pull request #{number} from "),
        &format!("--grep=\\(#{number}\\)\\s*$"),
        main_of(path),
    ])
    .env("GIT_FLUSH", "1")
    .stdin(Stdio::null())
    .stdout(Stdio::piped())
    .stderr(Stdio::null());
    git
}

/// The newest of `found`, each `SHA SUBJECT`, whose subject says it merged
/// `number`.
fn subject_merging(found: &str, number: u64) -> Option<String> {
    found.lines().find_map(|line| {
        let (sha, subject) = line.split_once(' ')?;
        merged_numbers(subject)
            .contains(&number)
            .then(|| sha.to_owned())
    })
}

/// Whether `rev` carries `sha`, or `None` when `path` doesn't have `rev`,
/// which a fetch, not the owner, mends.
fn carries(path: &str, sha: &str, rev: &str) -> Option<bool> {
    let have = format!("{rev}^{{commit}}");
    command("git", &["-C", path, "cat-file", "-e", &have]).ok()?;
    Some(
        command(
            "git",
            &["-C", path, "merge-base", "--is-ancestor", sha, rev],
        )
        .is_ok(),
    )
}

/// The files the merge `sha` changed on its main, one a line, if git could
/// say, leaving out core's runtime build where all it changed was the list
/// of the patches the merge changed.
fn changed(path: &str, sha: &str) -> Option<String> {
    let parent = format!("{sha}^1");
    // A rename would hide the name a runtime still applies the patch by,
    // and a name git quotes or splits would miss the patches' folder and
    // read as reaching no host, so names come whole, apart by NUL.
    let names = command(
        "git",
        &[
            "-C",
            path,
            "diff",
            "-z",
            "--no-renames",
            "--name-only",
            &parent,
            sha,
        ],
    )
    .ok()?;
    let mut files: Vec<&str> = names.split('\0').filter(|name| !name.is_empty()).collect();
    if files.iter().any(|name| name.contains('\n')) {
        return None;
    }
    if files.contains(&HOSTED_PI) {
        let diff = command(
            "git",
            &["-C", path, "diff", "-U0", &parent, sha, "--", HOSTED_PI],
        )
        .ok()?;
        if only_lists(&diff, &files) {
            files.retain(|name| *name != HOSTED_PI);
        }
    }
    Some(files.join("\n"))
}

/// Whether each line `diff` of core's runtime build adds or removes is the
/// line listing one of the patches in `files`, so it changed no step of the
/// build a runtime built elsewhere would lack.
fn only_lists(diff: &str, files: &[&str]) -> bool {
    diff.lines()
        .filter(|line| !line.starts_with("+++") && !line.starts_with("---"))
        .filter_map(|line| line.strip_prefix('+').or_else(|| line.strip_prefix('-')))
        .all(|listed| {
            files.iter().any(|file| {
                file.strip_prefix(PATCHES)
                    .is_some_and(|name| listed.trim() == format!("./host-patches/{name}"))
            })
        })
}

/// Each pi host, with the patches its runtime applied where it keeps them,
/// or `None` when the hosts couldn't be listed.
fn read_hosts() -> Option<Vec<Host>> {
    hosts().map(as_hosts)
}

/// Each running conversation, read as `read_hosts` reads a host.
fn read_talks() -> Option<Vec<Host>> {
    conversations().map(as_hosts)
}

/// Each of `listed` with the patches its runtime applied.
fn as_hosts(listed: Vec<Listed>) -> Vec<Host> {
    listed
        .into_iter()
        .map(|(runtime, started, standing)| Host {
            applied: runtime.and_then(|runtime| applied(&runtime)),
            started,
            standing,
        })
        .collect()
}

/// The patches the runtime at `root` applied, from the copies it keeps;
/// `None` for a runtime that keeps none, built before it kept them.
fn applied(root: &Path) -> Option<Applied> {
    let mut applied = Applied::new();
    for entry in std::fs::read_dir(root.join(APPLIED)).ok()? {
        let path = entry.ok()?.path();
        let name = path.file_name()?.to_str()?.to_owned();
        applied.insert(name, std::fs::read_to_string(&path).ok()?);
    }
    Some(applied)
}

/// Each of `patches` by name, with every state of it a runtime may hold and
/// carry the change: as the merge `sha` left it, `None` where it removed
/// it, and as each merge on main's own line has left it since, up to the
/// first that removes it or brings back a state it had without the change,
/// as a revert does: what follows is built on the patch without it. A state
/// without the change is one from before `sha`, on any branch, or one on a
/// branch cut before it and merged since, which a merge may bring to main.
/// `None` when git can't say.
fn versions(path: &str, sha: &str, patches: &[String]) -> Option<Wanted> {
    let log = |how: &[&str], patch: &str| {
        let mut args = vec!["-C", path, "log", "--format=%H"];
        args.extend_from_slice(how);
        args.extend(["--", patch]);
        command("git", &args).ok()
    };
    let range = format!("{sha}..{}", main_of(path));
    let parent = format!("{sha}^1");
    // The merge and the commits it brought, which a revert names.
    let fix = command(
        "git",
        &["-C", path, "rev-list", &format!("{parent}..{sha}")],
    )
    .ok()?;
    let fix: Vec<&str> = fix.lines().collect();
    let mut versions = Wanted::new();
    for patch in patches {
        let read = |commit: &str| state(path, commit.trim(), patch).ok();
        // The log fails for a merge cadence doesn't have, and a state git
        // can't read fails the whole, so a patch missing from a commit is
        // one it lacks, not one unfetched.
        let since = log(&["--first-parent", &range], patch)?;
        let earlier = log(&["--full-history", &parent], patch)?;
        let apart = log(&["--full-history", &range], patch)?;
        // Asked of each commit, since `--ancestry-path` under a path leaves
        // out a merge that descends from `sha` and keeps its text. One git
        // can't place counts as apart, which only stops the walk sooner.
        let descends = |commit: &&str| {
            command(
                "git",
                &["-C", path, "merge-base", "--is-ancestor", sha, commit],
            )
            .is_ok()
        };
        let mut before: Vec<Option<String>> = earlier
            .lines()
            .chain(apart.lines().filter(|commit| !descends(commit)))
            .map(read)
            .collect::<Option<_>>()?;
        before.push(read(&parent)?);
        let mut states = vec![read(sha)?];
        for commit in since.lines().rev() {
            let later = read(commit)?;
            if later.is_none() || before.contains(&later) || reverts(path, commit, &fix)? {
                break;
            }
            states.push(later);
        }
        let name = Path::new(patch).file_name()?.to_str()?.to_owned();
        versions.insert(name, states);
    }
    Some(versions)
}

/// Whether the commit `commit` on main, or one it merged, says it reverts
/// one of `fix`, as `git revert` writes: a revert made after the patch
/// changed again leaves a text found nowhere before, without the fix.
/// `None` when git can't say.
fn reverts(path: &str, commit: &str, fix: &[&str]) -> Option<bool> {
    let commit = commit.trim();
    let said = command(
        "git",
        &[
            "-C",
            path,
            "log",
            "--format=%B",
            &format!("{commit}^1..{commit}"),
        ],
    )
    .ok()?;
    Some(said.lines().any(|line| {
        let Some(named) = line.trim().strip_prefix("This reverts commit ") else {
            return false;
        };
        let named: String = named.chars().take_while(char::is_ascii_hexdigit).collect();
        named.len() >= 7 && fix.iter().any(|fixed| fixed.starts_with(&named))
    }))
}

/// The patch at `file` as `commit` holds it, `None` where it holds none, or
/// an error where git can't read the commit: a failure taken for absence
/// would read a patch the merge added as one it removed, and tell a runtime
/// lacking it fixed.
fn state(path: &str, commit: &str, file: &str) -> std::io::Result<Option<String>> {
    let listed = command(
        "git",
        &["-C", path, "ls-tree", "--name-only", commit, "--", file],
    )?;
    if listed.trim().is_empty() {
        return Ok(None);
    }
    command("git", &["-C", path, "show", &format!("{commit}:{file}")]).map(Some)
}

/// Whether a runtime that `applied` these loads a change, holding each of
/// its patches in one of the states `wanted` names for it, a patch it
/// removed by applying none.
fn loads(applied: &Applied, wanted: &Wanted) -> bool {
    wanted
        .iter()
        .all(|(name, states)| states.contains(&applied.get(name).cloned()))
}

/// The revision a `flake.lock` pins its root's `input` at.
fn input_pin(lock: &str, input: &str) -> Option<String> {
    let lock: Value = serde_json::from_str(lock).ok()?;
    let node = lock["nodes"]["root"]["inputs"][input].as_str()?;
    lock["nodes"][node]["locked"]["rev"]
        .as_str()
        .filter(|rev| is_rev(rev))
        .map(str::to_owned)
}

#[cfg(test)]
mod tests;
