//! Whether a merged check runs on this machine yet, since a merge isn't a
//! fix on Joel's machine: at 21:10 on 19 November the card told him his
//! 19:05 miss, the background pi host's crash, was fixed on core #5800's
//! merge, while the last switch predated it and the host still loaded the
//! older runtime's pi. Whatever cadence can't read counts as not running,
//! so a blind hour never tells him fixed.

mod conversations;
mod program;
mod resumed;

use conversations::conversations;

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

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

/// The repositories whose running revision cadence reads: core's and
/// World's from the last switch, mycelium's as that switch's core pins it,
/// and the harness's from the version installed. A change elsewhere,
/// deployed apart, is followed no further than its merge.
const RUN_HERE: &[&str] = &["core", "world", "mycelium", "harness"];

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
/// when it started, and whether it was stopped.
type Listed = (Option<std::path::PathBuf>, Option<i64>, bool);

/// The states of each patch of a change, by name, that carry it: its
/// text, or `None` for one the change removed.
type Wanted = BTreeMap<String, Vec<Option<String>>>;

/// A pi host as cadence reads it: the patches its runtime applied, and
/// when it started, each `None` where that couldn't be read, and whether
/// it was stopped, with no process to keep an older unit.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Host {
    applied: Option<Applied>,
    started: Option<i64>,
    stopped: bool,
}

/// Which of `merged` don't run here yet, each with why. A change runs once
/// its repository's running revision carries its merge, a core change to
/// the hosts' patches once every running host also loads a runtime that
/// applied each as merged or since, one to the hosts' unit once every
/// host started after the switch that carried it, and a harness change to
/// the hosts' program once every host and conversation started after it
/// was installed. A change seen running is kept in `folder`'s `ran.json` and never read again, so a
/// closed miss isn't reopened, and told fixed twice, by an hour that can't
/// see for a moment: a host restarting at :10, or a switch from a branch.
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
    let revs = running(switches, repos);
    let mut hosts: Option<Option<Vec<Host>>> = None;
    let mut talks: Option<Option<Vec<Host>>> = None;
    let mut unrun = BTreeMap::new();
    for change in merged.iter().filter(|change| !ran.contains(*change)) {
        let Some(path) = repos.get(&change.repo) else {
            continue;
        };
        let path = path.to_string_lossy();
        let why = judge(
            &change.repo,
            revs.get(&change.repo).map(String::as_str),
            merge_of(&path, change.number).as_deref(),
            |sha, rev| carries(&path, sha, rev),
            |sha| changed(&path, sha),
            |sha| program::files(&path, sha),
            |sha, reaches| {
                let read = hosts.get_or_insert_with(read_hosts);
                let carried = |rev: &str| carries(&path, sha, rev) == Some(true);
                // Each conversation's process runs the program too.
                let listed: Option<Vec<Host>> = if reaches.program {
                    let talks = talks.get_or_insert_with(conversations).as_ref();
                    read.as_ref()
                        .zip(talks)
                        .map(|(read, talks)| read.iter().chain(talks).cloned().collect())
                } else {
                    read.clone()
                };
                let since = if reaches.program {
                    program::swapped(&crate::axes::landings::harness_mirror())
                } else if reaches.unit {
                    first_switched(switches, carried)
                } else {
                    None
                };
                // Walked only once a host keeps copies to compare with it.
                let merged = std::cell::OnceCell::new();
                hosts_lack(listed.as_deref(), &reaches, since, |applied| {
                    merged
                        .get_or_init(|| versions(&path, sha, &reaches.patches))
                        .as_ref()
                        .map(|wanted| loads(applied, wanted))
                })
            },
        );
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
    if now_ran != ran
        && let Err(error) = serde_json::to_string(&now_ran)
            .map_err(std::io::Error::from)
            .and_then(|kept| write_whole(&ran_at, &kept))
    {
        eprintln!(
            "cadence: keeping what runs in {}: {error}",
            ran_at.display()
        );
    }
    unrun
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
/// files the hosts' program loads at a commit, and `hosts` why the pi
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
        return Some("cadence can't find its merge");
    };
    match carries(sha, rev) {
        None => return Some("cadence can't find the revision running here"),
        Some(false) if repo == "harness" => return Some("the harness installed doesn't carry it"),
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
        let Some(loaded) = program(sha) else {
            return Some("cadence can't tell which harness files a pi host loads");
        };
        let reaches = Reaches {
            program: files.lines().any(|file| loaded.contains(file)),
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
    // A fix to how core builds its runtime can't be seen in a runtime built
    // elsewhere, which copies only its patches, so it isn't told fixed;
    // `changed` leaves the build out where it only listed the patches.
    if files.lines().any(|file| file == HOSTED_PI) {
        return Some("cadence can't tell what a pi host loads");
    }
    (!reaches.patches.is_empty() || reaches.unit)
        .then(|| hosts(sha, reaches))
        .flatten()
}

/// Why the pi hosts lack a change that `reaches` them, if they do: each
/// host's runtime must have applied its patches, as `loads` says of what a
/// runtime applied (`None` when it can't tell), and each must have started
/// after `since`: when a switch first carried a change to their unit, or
/// `current` last moved to the version carrying one to their program,
/// `None` when cadence found neither. For the program, `hosts` holds each
/// conversation's process too, and is `None` when they couldn't be listed.
/// Something lacking outranks what can't be read, so the owner is asked.
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
    if hosts.is_empty() {
        return Some("no pi host runs to load it");
    }
    let loaded: Vec<Option<bool>> = hosts
        .iter()
        .map(|host| host.applied.as_ref().and_then(&loads))
        .collect();
    if runtime && loaded.contains(&Some(false)) {
        return Some("the pi hosts don't load it yet");
    }
    // A stopped host keeps no unit: it starts on the one switched in.
    let running = || hosts.iter().filter(|host| !host.stopped);
    let restarts = reaches.unit || reaches.program;
    if restarts
        && let Some(at) = since
        && running().any(|host| host.started.is_some_and(|started| started < at))
    {
        return Some(if reaches.program {
            "a pi host or conversation runs the harness from before it"
        } else {
            "a pi host keeps its unit from before it"
        });
    }
    if runtime && loaded.contains(&None) {
        return Some("cadence can't tell what a pi host loads");
    }
    if reaches.program && since.is_none() {
        return Some("cadence can't tell when the harness carrying it was installed");
    }
    if restarts && (since.is_none() || running().any(|host| host.started.is_none())) {
        return Some("cadence can't tell when a pi host started");
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
fn running(switches: &str, repos: &Checkouts) -> BTreeMap<String, String> {
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
/// `current` points at.
fn installed() -> Option<String> {
    let mirror = crate::axes::landings::harness_mirror();
    let current = std::fs::read_link(mirror.parent()?.join("current")).ok()?;
    let name = current.file_name()?.to_string_lossy().into_owned();
    is_rev(&name).then_some(name)
}

/// The commit on `path`'s main that merged pull request `number`, by its
/// subject as `merged_numbers` reads it, since git's grep matches a line of
/// any commit's body too.
fn merge_of(path: &str, number: u64) -> Option<String> {
    let merged = format!("--grep=^Merge pull request #{number} from ");
    let squashed = format!("--grep=\\(#{number}\\)\\s*$");
    let found = command(
        "git",
        &[
            "-C",
            path,
            "log",
            "--first-parent",
            "--format=%H %s",
            "-E",
            &merged,
            &squashed,
            main_of(path),
        ],
    )
    .ok()?;
    subject_merging(&found, number)
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
    Some(
        hosts()?
            .into_iter()
            .map(|(runtime, started, stopped)| Host {
                applied: runtime.and_then(|runtime| applied(&runtime)),
                started,
                stopped,
            })
            .collect(),
    )
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

/// The runtime each pi host loads its pi from and when it started, each
/// `None` where it couldn't be read, and whether it was stopped. A host is
/// the main process of a unit core's host module names
/// `pi-host-REALM-KIND`; the conversations it starts are read only for its
/// program, in `conversations`. A host stopped comes back on the
/// runtime its unit names, so it's judged by that rather than left out.
/// One failed or restarting may never load that runtime, as when it
/// can't start on it, so what it loads can't be told.
fn hosts() -> Option<Vec<Listed>> {
    let units = command(
        "systemctl",
        &[
            "--user",
            "list-units",
            "--type=service",
            "--all",
            "--no-legend",
            "--plain",
            "pi-host-*",
        ],
    )
    .ok()?;
    Some(
        units
            .lines()
            .filter_map(listed_host)
            .map(|(unit, stopped)| {
                let shown = command(
                    "systemctl",
                    &[
                        "--user",
                        "show",
                        "--timestamp=unix",
                        "-p",
                        "MainPID",
                        "-p",
                        "ActiveEnterTimestamp",
                        "-p",
                        "Environment",
                        unit,
                    ],
                )
                .unwrap_or_default();
                let (pid, started) = shown_host(&shown);
                match pid {
                    Some(pid) => {
                        let runtime = std::fs::read(format!("/proc/{pid}/environ"))
                            .ok()
                            .and_then(|environ| runtime(&environ));
                        (runtime, started, false)
                    }
                    None if stopped => (unit_runtime(&shown), None, true),
                    None => (None, None, false),
                }
            })
            .collect(),
    )
}

/// The host unit a line of `systemctl list-units --all` names, and whether
/// it was stopped, unless it's one gone from the config and stopped: a host
/// still running after its unit's file went still loads its runtime, so
/// it's read.
fn listed_host(line: &str) -> Option<(&str, bool)> {
    let mut fields = line.split_whitespace();
    let unit = fields.next().filter(|unit| host_unit(unit))?;
    let (load, active) = (fields.next(), fields.next());
    let stopped = active == Some("inactive");
    (load == Some("loaded") || !stopped).then_some((unit, stopped))
}

/// A host unit's main process and when it started, from `systemctl show`.
fn shown_host(shown: &str) -> (Option<u32>, Option<i64>) {
    let field = |name: &str| {
        shown
            .lines()
            .find_map(|line| line.strip_prefix(name)?.strip_prefix('='))
    };
    let pid = field("MainPID")
        .and_then(|pid| pid.trim().parse().ok())
        .filter(|&pid| pid != 0);
    let started = field("ActiveEnterTimestamp")
        .and_then(|at| at.trim().strip_prefix('@')?.parse().ok())
        .filter(|&at| at > 0);
    (pid, started)
}

/// Whether `unit` is a pi host's, `pi-host-REALM-KIND.service`, and not one
/// of the units beside them, such as `pi-host-down@`.
fn host_unit(unit: &str) -> bool {
    unit.strip_prefix("pi-host-")
        .and_then(|rest| rest.strip_suffix(".service"))
        .is_some_and(|named| !named.contains('@') && named.contains('-'))
}

/// The runtime a pi host loads its pi from, as the store path its
/// `PI_PACKAGE_DIR` lies in: a host's node may come from a newer runtime
/// while it still loads the older one's pi, as the background host did from
/// 20:24 on 19 November.
fn runtime(environ: &[u8]) -> Option<std::path::PathBuf> {
    let dir = environ
        .split(|&b| b == 0)
        .find_map(|var| var.strip_prefix(b"PI_PACKAGE_DIR="))?;
    store_root(std::str::from_utf8(dir).ok()?)
}

/// The runtime a host's unit starts it on, from its `Environment` as
/// `systemctl show` gives it, for a host with no process to read.
fn unit_runtime(shown: &str) -> Option<std::path::PathBuf> {
    let environment = shown
        .lines()
        .find_map(|line| line.strip_prefix("Environment="))?;
    let dir = environment
        .split_whitespace()
        .find_map(|var| var.trim_matches('"').strip_prefix("PI_PACKAGE_DIR="))?;
    store_root(dir)
}

/// The store path `dir` lies in, whatever it's named: what makes it a host's
/// runtime is the copies of its patches it keeps, which `applied` reads.
fn store_root(dir: &str) -> Option<std::path::PathBuf> {
    let dir = Path::new(dir);
    // `..` would name one store path while pi loads from another.
    if dir
        .components()
        .any(|part| part == std::path::Component::ParentDir)
    {
        return None;
    }
    let name = dir
        .strip_prefix("/nix/store")
        .ok()?
        .components()
        .next()?
        .as_os_str()
        .to_str()?;
    let (hash, _) = name.split_once('-')?;
    (hash.len() == 32).then(|| Path::new("/nix/store").join(name))
}

#[cfg(test)]
mod tests;
