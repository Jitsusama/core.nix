//! Whether each running service runs the program its unit names. A switch
//! writes a service's new unit, but the service's process keeps the program
//! it started with until it restarts, as a unit marked keep-old does by
//! design, and a runtime drop-in that copies a whole `ExecStart` pins it
//! through every switch after. For most of a day, one machine's mycelium
//! node ran a build no lock named, through three switches, while cadence
//! read each mycelium merge as running. So a change reaching a service runs only once that service
//! runs what its unit, as the switch wrote it, names.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::command;

/// Why a change doesn't run yet while a service it reaches runs a build
/// other than the one its unit names, as `UNRUN` follows a merge with.
pub(super) const STALE: &str = "a service it reaches runs a build other than its unit names";

/// Why a change is held open when systemd couldn't list what runs.
pub(super) const UNLISTED: &str = "cadence can't list the services running";

/// Why a change is held open when a service it reaches runs a build cadence
/// couldn't find in the closure its unit names, since `nix-store` failed.
pub(super) const UNREAD: &str = "cadence can't read the build a service it reaches runs";

/// The programs that only run another, a unit's or a process's: the
/// program it's for is the first in the store after them.
const STARTERS: &[&str] = &["env", "bash", "sh", "dash", "python", "node", "perl"];

/// The services whose process runs another program than the one their
/// unit names, by design, each with why, so they aren't read as stale.
const RUN_ANOTHER: &[(&str, &str)] = &[
    (
        "sessions-kitty.service",
        "its unit waits, then starts the kitty the account's profile holds",
    ),
    (
        "merge-net.service",
        "its watch follows the account profile's merge-net and runs that one",
    ),
];

/// What a running service's process runs, against what its unit names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Runs {
    /// The build its unit names, or one in that build's closure.
    Named,
    /// Another build, as a pinned or not yet restarted service does.
    Another,
    /// A build cadence couldn't find in the named build's closure, since
    /// reading the closure failed.
    Unread,
}

/// A running service, by its unit's name, and what its process runs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Service {
    pub(super) unit: String,
    pub(super) runs: Runs,
}

/// Every running service, of the system's and the account's, whose build
/// cadence can read, as `Service`; `None` when systemd can't list them.
/// Each found running another build, or whose closure couldn't be read, is
/// said on stderr, so the hour's journal names it. A service naming no
/// build in the store, as a script run by `bash -c` does, or started by a
/// path outside the store whose executable is only an interpreter, is left
/// to its revision's verdict: either is so every hour, so holding on it
/// would hold every change to its folder for good.
pub(super) fn read() -> Option<Vec<Service>> {
    let mut services = Vec::new();
    let mut closures = BTreeMap::new();
    for scope in ["--system", "--user"] {
        let listed = command(
            "systemctl",
            &[
                scope,
                "list-units",
                "--type=service",
                "--state=running",
                "--no-legend",
                "--plain",
            ],
        )
        .ok()?;
        let units = listed_units(&listed);
        if units.is_empty() {
            continue;
        }
        let mut args = vec![
            scope,
            "show",
            "-p",
            "Id",
            "-p",
            "MainPID",
            "-p",
            "ExecStart",
            "-p",
            "FragmentPath",
        ];
        args.extend(&units);
        let shown = command("systemctl", &args).ok()?;
        let read_closure = |root: &str| {
            closures
                .entry(root.to_owned())
                .or_insert_with(|| closure(root))
                .clone()
        };
        services.extend(judged(
            &shown,
            |path| std::fs::read_to_string(path).ok(),
            running,
            read_closure,
        ));
    }
    Some(services)
}

/// The services `list-units --plain --no-legend` lists, by their names.
fn listed_units(listed: &str) -> Vec<&str> {
    listed
        .lines()
        .filter_map(|line| line.split_whitespace().next())
        .filter(|unit| unit.ends_with(".service"))
        .collect()
}

/// Each service `shown`, the blocks of `systemctl show`, holds, with what
/// it runs: its unit's program read from its unit file, by `fragment`, so
/// a drop-in that copies a whole `ExecStart` can't hide what the switch
/// wrote; what its process runs, by `process`; and the closure of its
/// unit's build, by `closure`, where neither matches directly.
pub(super) fn judged(
    shown: &str,
    fragment: impl Fn(&str) -> Option<String>,
    process: impl Fn(u32) -> Vec<String>,
    mut closure: impl FnMut(&str) -> Option<BTreeSet<String>>,
) -> Vec<Service> {
    let mut services = Vec::new();
    for block in shown.split("\n\n") {
        let Some(unit) = field(block, "Id") else {
            continue;
        };
        if RUN_ANOTHER.iter().any(|(another, _)| *another == unit) {
            continue;
        }
        let named = field(block, "FragmentPath")
            .and_then(&fragment)
            .and_then(|text| fragment_named(&text))
            .or_else(|| field(block, "ExecStart").and_then(named))
            .and_then(|path| resolved(&path));
        let pid = field(block, "MainPID")
            .and_then(|pid| pid.parse::<u32>().ok())
            .filter(|&pid| pid != 0);
        let (Some(named), Some(programs)) = (named, pid.map(&process)) else {
            continue;
        };
        let Some(runs) = runs(&programs, &named, &mut closure) else {
            continue;
        };
        match runs {
            Runs::Named => {}
            Runs::Another => eprintln!(
                "cadence: {unit} runs {}, not {named}, which its unit names",
                programs.join(" or "),
            ),
            Runs::Unread => eprintln!(
                "cadence: {unit} runs {}, and the closure of {named}, which its unit names, \
                 couldn't be read",
                programs.join(" or "),
            ),
        }
        services.push(Service {
            unit: unit.to_owned(),
            runs,
        });
    }
    services
}

/// The value of `name` in a block of `systemctl show`, the first where a
/// unit has several, as `ExecStart` does for each of its commands.
fn field<'a>(block: &'a str, name: &str) -> Option<&'a str> {
    block
        .lines()
        .find_map(|line| line.strip_prefix(name)?.strip_prefix('='))
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

/// The program a unit file's first `ExecStart` names, as `program` reads
/// its words, past the prefixes systemd allows before the path, such as
/// `-` and `@`, and any quotes around each word.
pub(super) fn fragment_named(text: &str) -> Option<String> {
    let exec = text
        .lines()
        .find_map(|line| line.trim().strip_prefix("ExecStart="))
        .map(|exec| exec.trim_start_matches(['@', '-', ':', '+', '!']))
        .filter(|exec| !exec.trim().is_empty())?;
    let mut words = exec
        .split_whitespace()
        .map(|word| word.trim_matches(['"', '\'']));
    let path = words.next()?;
    program(path, words)
}

/// The program an `ExecStart` as `systemctl show` gives it names, as
/// `program` reads its path and arguments.
pub(super) fn named(exec: &str) -> Option<String> {
    let part = |name: &str| {
        exec.split(" ; ")
            .find_map(|part| part.trim_start_matches(['{', ' ']).strip_prefix(name))
    };
    let path = part("path=")?.trim();
    let args = part("argv[]=").unwrap_or_default();
    program(path, args.split_whitespace().skip(1))
}

/// The program a command of `path` and `args` runs: `path`, or where that
/// only starts another, as `env` does, the first program in a store path's
/// `bin` its arguments name, which a setting such as `FOO=/nix/store/...`
/// isn't.
fn program<'a>(path: &str, mut args: impl Iterator<Item = &'a str>) -> Option<String> {
    if !starts_another(path) {
        return Some(path.to_owned());
    }
    args.find(|arg| arg.starts_with("/nix/store/") && arg.contains("/bin/") && !starts_another(arg))
        .map(str::to_owned)
}

/// Whether `path`'s program only runs another, by its name, as
/// `python3.12` is `python`'s.
fn starts_another(path: &str) -> bool {
    let name = path.rsplit('/').next().unwrap_or(path);
    let name = name.strip_suffix(" (deleted)").unwrap_or(name);
    let bare = name.trim_end_matches(|c: char| c.is_ascii_digit() || c == '.');
    STARTERS.contains(&bare)
}

/// A unit's program's store path, through any link to it as the unit is
/// now, as `/run/current-system/sw/bin/x` leads to one; `None` for a
/// program outside the store, whose build cadence can't tell, or a bare
/// name.
fn resolved(path: &str) -> Option<String> {
    store_root(path).or_else(|| {
        if !path.starts_with('/') {
            return None;
        }
        let real = std::fs::canonicalize(path).ok()?;
        store_root(real.to_str()?)
    })
}

/// The store path `path` lies in, as `/nix/store/<hash>-<name>`.
pub(super) fn store_root(path: &str) -> Option<String> {
    let rest = path.strip_prefix("/nix/store/")?;
    let name = rest.split('/').next()?;
    let (hash, _) = name.split_once('-')?;
    (hash.len() == 32).then(|| format!("/nix/store/{name}"))
}

/// The builds process `pid` may run, as `programs` reads them.
fn running(pid: u32) -> Vec<String> {
    let Ok(cmdline) = std::fs::read(format!("/proc/{pid}/cmdline")) else {
        return Vec::new();
    };
    let args: Vec<String> = cmdline
        .split(|&b| b == 0)
        .filter(|arg| !arg.is_empty())
        .map(|arg| String::from_utf8_lossy(arg).into_owned())
        .collect();
    let exe = std::fs::read_link(format!("/proc/{pid}/exe"))
        .ok()
        .and_then(|exe| exe.to_str().map(str::to_owned));
    programs(&args, exe.as_deref())
}

/// The builds a process may run: of its first two arguments, those given
/// as a path in the store, since an interpreter's second names its script,
/// and its executable's, which the kernel resolved as it started and the
/// account can't read of root's. Never an argument's link resolved now,
/// which names today's build, not the one the process started with; never
/// an interpreter's, which a new script's closure holds as well as an old
/// one's; and never a later argument's, which is as often a data file in
/// the store, as a service's `words.json` is.
pub(super) fn programs(args: &[String], exe: Option<&str>) -> Vec<String> {
    let found: BTreeSet<String> = args
        .iter()
        .take(2)
        .map(String::as_str)
        .chain(exe)
        .filter(|path| !starts_another(path))
        .filter_map(store_root)
        .collect();
    found.into_iter().collect()
}

/// What a process running one of `programs`, the builds it may run, runs
/// against `named`, its unit's build: it, or one in its closure, as a
/// wrapper's target is, by `closure`; `None` when its builds can't be
/// read at all, or `named` lies outside the store.
pub(super) fn runs(
    programs: &[String],
    named: &str,
    closure: impl FnOnce(&str) -> Option<BTreeSet<String>>,
) -> Option<Runs> {
    let named = store_root(named)?;
    if programs.is_empty() {
        return None;
    }
    if programs.contains(&named) {
        return Some(Runs::Named);
    }
    Some(match closure(&named) {
        Some(closure) if programs.iter().any(|program| closure.contains(program)) => Runs::Named,
        Some(_) => Runs::Another,
        None => Runs::Unread,
    })
}

/// Every store path `root` refers to, itself among them; `None`, said on
/// stderr, when `nix-store` can't list them.
fn closure(root: &str) -> Option<BTreeSet<String>> {
    if !Path::new(root).exists() {
        eprintln!("cadence: {root}, which a running service's unit names, isn't in the store");
        return None;
    }
    match command("nix-store", &["--query", "--requisites", root]) {
        Ok(listed) => Some(listed.lines().map(str::to_owned).collect()),
        Err(error) => {
            eprintln!("cadence: can't read the closure of {root}: {error}");
            None
        }
    }
}

/// The services a change reaches, as the names their units begin with: a
/// mycelium change, mycelium's; a core change, the folder under
/// `modules/` of each file `files` lists, as `model-relay` holds
/// `model-relay-agents.service`.
pub(super) fn reached(repo: &str, files: Option<&str>) -> BTreeSet<String> {
    match repo {
        "mycelium" => BTreeSet::from(["mycelium".to_owned()]),
        "core" => files
            .unwrap_or_default()
            .lines()
            .filter_map(|file| file.strip_prefix("modules/")?.split_once('/'))
            .map(|(folder, _)| folder.to_owned())
            .collect(),
        _ => BTreeSet::new(),
    }
}

/// Why a change reaching the services named `reached` doesn't run yet, if
/// it doesn't: one of them runs another build than its unit names, one's
/// build couldn't be read, or `services`, `None`, couldn't be listed.
pub(super) fn lack(
    reached: &BTreeSet<String>,
    services: Option<&[Service]>,
) -> Option<&'static str> {
    if reached.is_empty() {
        return None;
    }
    let Some(services) = services else {
        return Some(UNLISTED);
    };
    let reached_runs = |runs: Runs| {
        services.iter().any(|service| {
            service.runs == runs && reached.iter().any(|name| reaches(name, &service.unit))
        })
    };
    if reached_runs(Runs::Another) {
        Some(STALE)
    } else if reached_runs(Runs::Unread) {
        Some(UNREAD)
    } else {
        None
    }
}

/// Whether the service `unit` is one `name` reaches: its name, without
/// `.service` or an instance, is `name` or begins with `name-`.
fn reaches(name: &str, unit: &str) -> bool {
    let stem = unit.strip_suffix(".service").unwrap_or(unit);
    let stem = stem.split_once('@').map_or(stem, |(stem, _)| stem);
    stem == name
        || stem
            .strip_prefix(name)
            .is_some_and(|rest| rest.starts_with('-'))
}

#[cfg(test)]
mod tests;
