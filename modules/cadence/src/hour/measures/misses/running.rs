//! Whether a merged check runs on this machine yet, since a merge isn't a
//! fix on Joel's machine: at 21:10 on 19 November the card told him his
//! 19:05 miss, the background pi host's crash, was fixed on core #5800's
//! merge, while the last switch predated it and the host still loaded the
//! older runtime's pi. Whatever cadence can't read counts as not running,
//! so a blind hour never tells him fixed.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde_json::Value;

use super::{Checkouts, Named, main_of, merged_numbers};
use crate::{command, write_whole};

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

/// The files of core a change carries into the pi hosts' runtime, which a
/// host loads only once it restarts onto a runtime built with them, apart
/// from any switch.
const HOSTED: &[&str] = &["modules/pi/host-patches/", "modules/pi/hosted-pi.nix"];

/// The pi hosts' own unit, which a switch leaves a running host keeping
/// (its `X-SwitchMethod` is `keep-old`) until the host next starts.
const HOST_UNIT: &str = "modules/pi/host.nix";

/// How far back the commit pinning a host's runtime is looked for. A
/// runtime first pinned before then can't be placed.
const PINNED_WITHIN: &str = "--since=30.days";

/// Where the pi hosts' runtime is pinned: the checkout cadence reads, by the
/// name `--checkout` gives it, and the folder in it whose `flake.nix` names
/// the runtime by store path and whose `flake.lock` pins core, as
/// `NAME=FOLDER`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pin<'a> {
    checkout: &'a str,
    folder: &'a str,
}

impl<'a> Pin<'a> {
    #[must_use]
    pub fn parse(named: &'a str) -> Option<Self> {
        let (checkout, folder) = named.split_once('=')?;
        let folder = folder.trim_matches('/');
        (!checkout.is_empty() && !folder.is_empty()).then_some(Pin { checkout, folder })
    }
}

/// A running pi host as cadence reads it: the core its runtime was pinned
/// with, and when it started, each `None` where that couldn't be read.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Host {
    core: Option<String>,
    started: Option<i64>,
}

/// Which of `merged` don't run here yet, each with why. A change runs once
/// its repository's running revision carries its merge, a core change to
/// the hosts' runtime once every running host also loads a runtime `pin`
/// shows built from a core carrying it, and one to the hosts' unit once
/// every host started after the switch that carried it. A change seen
/// running is kept in `folder`'s `ran.json` and never read again, so a
/// closed miss isn't reopened, and told fixed twice, by an hour that can't
/// see for a moment: a host restarting at :10, or a switch from a branch.
#[must_use]
pub fn unrun(
    merged: &BTreeSet<Named>,
    repos: &Checkouts,
    switches: &str,
    pin: Option<Pin<'_>>,
    folder: &Path,
) -> BTreeMap<Named, &'static str> {
    let ran_at = folder.join("ran.json");
    let ran: BTreeSet<Named> = std::fs::read_to_string(&ran_at)
        .ok()
        .and_then(|kept| serde_json::from_str(&kept).ok())
        .unwrap_or_default();
    let revs = running(switches, repos);
    let pin = pin.filter(|pin| {
        let known = repos.contains_key(pin.checkout);
        if !known {
            eprintln!(
                "cadence: --host-runtime names {}, which no --checkout gives",
                pin.checkout
            );
        }
        known
    });
    let mut hosts: Option<Option<Vec<Host>>> = None;
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
            |sha, reaches| {
                let read = hosts.get_or_insert_with(|| read_hosts(pin, repos));
                let switched_at = reaches.unit.then(|| {
                    first_switched(switches, |rev| carries(&path, sha, rev) == Some(true))
                });
                hosts_lack(read.as_deref(), pin, reaches, switched_at, |rev| {
                    carries(&path, sha, rev) == Some(true)
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

/// What a core change reaches that only a host's restart brings in: the
/// hosts' runtime, or their unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Reaches {
    runtime: bool,
    unit: bool,
}

/// Why a change of `repo` merged as `merge` doesn't run here yet, if it
/// doesn't: `rev` is its repository's running revision, `carries` says
/// whether a revision holds a commit (`None` for a revision cadence doesn't
/// have), `changed` the files a merge changed, and `hosts` why the pi hosts
/// lack a change that reaches them. What cadence couldn't read counts as
/// not running.
fn judge(
    repo: &str,
    rev: Option<&str>,
    merge: Option<&str>,
    carries: impl Fn(&str, &str) -> Option<bool>,
    changed: impl Fn(&str) -> Option<String>,
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
    if repo != "core" {
        return None;
    }
    let files = match changed(sha) {
        None => return Some("cadence can't read what it changed"),
        Some(files) => files,
    };
    let reaches = Reaches {
        runtime: files
            .lines()
            .any(|file| HOSTED.iter().any(|hosted| file.starts_with(hosted))),
        unit: files.lines().any(|file| file == HOST_UNIT),
    };
    (reaches.runtime || reaches.unit)
        .then(|| hosts(sha, reaches))
        .flatten()
}

/// Why the pi hosts lack a change that `reaches` them, if they do: each
/// host's runtime must be pinned with a core that `carries` it, and each
/// must have started after `switched_at`, when a switch first carried a
/// change to their unit. `hosts` is `None` when they couldn't be listed.
/// Something lacking outranks what can't be read, so the owner is asked.
fn hosts_lack(
    hosts: Option<&[Host]>,
    pin: Option<Pin<'_>>,
    reaches: Reaches,
    switched_at: Option<Option<i64>>,
    carries: impl Fn(&str) -> bool,
) -> Option<&'static str> {
    if reaches.runtime && pin.is_none() {
        return Some("cadence can't see what the pi hosts load");
    }
    let hosts = hosts?;
    if hosts.is_empty() {
        return Some("no pi host runs to load it");
    }
    if reaches.runtime
        && hosts
            .iter()
            .any(|host| host.core.as_deref().is_some_and(|core| !carries(core)))
    {
        return Some("the pi hosts don't load it yet");
    }
    if let Some(Some(at)) = switched_at
        && hosts
            .iter()
            .any(|host| host.started.is_some_and(|started| started < at))
    {
        return Some("a pi host keeps its unit from before it");
    }
    if reaches.runtime && hosts.iter().any(|host| host.core.is_none()) {
        return Some("cadence can't tell what a pi host loads");
    }
    if reaches.unit
        && (switched_at.flatten().is_none() || hosts.iter().any(|host| host.started.is_none()))
    {
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

/// The files the merge `sha` changed on its main, if git could say.
fn changed(path: &str, sha: &str) -> Option<String> {
    command(
        "git",
        &["-C", path, "diff", "--name-only", &format!("{sha}^1"), sha],
    )
    .ok()
}

/// Each running pi host, with the core its runtime was pinned with where
/// `pin` shows it, or `None` when the hosts couldn't be listed.
fn read_hosts(pin: Option<Pin<'_>>, repos: &Checkouts) -> Option<Vec<Host>> {
    let running = hosts()?;
    let checkout = pin
        .and_then(|pin| repos.get(pin.checkout))
        .map(|path| path.to_string_lossy().into_owned());
    let core_of = |runtime: &str| {
        let (pin, checkout) = (pin?, checkout.as_deref()?);
        pinned_core(checkout, pin.folder, runtime)
    };
    Some(
        running
            .into_iter()
            .map(|(runtime, started)| Host {
                core: runtime.as_deref().and_then(core_of),
                started,
            })
            .collect(),
    )
}

/// The core the commit first naming `runtime` in `folder`'s `flake.nix`
/// pins, the core the runtime was built with; `None` when it was first
/// named before the window looked at, or nowhere.
fn pinned_core(checkout: &str, folder: &str, runtime: &str) -> Option<String> {
    let flake = format!("{folder}/flake.nix");
    let show = |commit: &str, file: &str| {
        command(
            "git",
            &["-C", checkout, "show", &format!("{commit}:{file}")],
        )
        .ok()
    };
    let touched = command(
        "git",
        &[
            "-C",
            checkout,
            "log",
            "--format=%H",
            PINNED_WITHIN,
            "-S",
            runtime,
            main_of(checkout),
            "--",
            &flake,
        ],
    )
    .ok()?;
    let names = |commit: &str| show(commit, &flake).is_some_and(|text| text.contains(runtime));
    let commit = first_naming(&touched, names)?;
    // A runtime its parent names already was pinned before the window, by
    // a lock this commit may have moved on from.
    if names(&format!("{commit}^")) {
        return None;
    }
    input_pin(&show(commit, &format!("{folder}/flake.lock"))?, "core")
}

/// The oldest of `touched`, commits newest first, that `names` the runtime:
/// git's `-S` gives the commit that dropped a runtime as well as the one
/// that pinned it, and the first pin is what the runtime was built with.
fn first_naming(touched: &str, names: impl Fn(&str) -> bool) -> Option<&str> {
    touched
        .lines()
        .map(str::trim)
        .filter(|commit| !commit.is_empty())
        .rev()
        .find(|commit| names(commit))
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

/// The runtime each running pi host loads its pi from and when it started,
/// each `None` where it couldn't be read. A host is the main process of a
/// unit core's host module names `pi-host-REALM-KIND`, so what it starts,
/// which keeps its environment after the host moves on, isn't read.
fn hosts() -> Option<Vec<(Option<String>, Option<i64>)>> {
    let units = command(
        "systemctl",
        &[
            "--user",
            "list-units",
            "--type=service",
            "--state=active",
            "--no-legend",
            "--plain",
            "pi-host-*",
        ],
    )
    .ok()?;
    Some(
        units
            .lines()
            .filter_map(|line| line.split_whitespace().next())
            .filter(|unit| host_unit(unit))
            .map(|unit| {
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
                        unit,
                    ],
                )
                .unwrap_or_default();
                let (pid, started) = shown_host(&shown);
                let runtime = pid
                    .and_then(|pid| std::fs::read(format!("/proc/{pid}/environ")).ok())
                    .and_then(|environ| runtime(&environ));
                (runtime, started)
            })
            .collect(),
    )
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

/// The runtime a pi host loads its pi from, as the hash of the store path
/// its `PI_PACKAGE_DIR` names: a host's node may come from a newer runtime
/// while it still loads the older one's pi, as the background host did from
/// 20:24 on 19 November.
fn runtime(environ: &[u8]) -> Option<String> {
    let dir = environ
        .split(|&b| b == 0)
        .find_map(|var| var.strip_prefix(b"PI_PACKAGE_DIR="))?;
    let dir = Path::new(std::str::from_utf8(dir).ok()?);
    let name = dir
        .strip_prefix("/nix/store")
        .ok()?
        .components()
        .next()?
        .as_os_str()
        .to_str()?;
    let (hash, rest) = name.split_once('-')?;
    (hash.len() == 32 && rest.starts_with("pi-host-runtime-")).then(|| hash.to_owned())
}

#[cfg(test)]
mod tests;
