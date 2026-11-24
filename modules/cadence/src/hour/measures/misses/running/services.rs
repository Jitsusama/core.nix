//! Whether each running service runs the program its unit names. A switch
//! writes a service's new unit, but the service's process keeps the program
//! it started with until it restarts, as a unit marked keep-old does by
//! design, and a runtime drop-in that copies a whole `ExecStart` pins it
//! through every switch after. From 21:06 on 23 November to 16:59 on 24
//! November optimus's work mycelium node ran a build no core lock named,
//! through three switches, while cadence read each mycelium merge as
//! running. So a change reaching a service runs only once that service
//! runs what its unit names.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::command;

/// Why a change doesn't run yet while a service it reaches runs a build
/// other than the one its unit names, as `UNRUN` follows a merge with.
pub(super) const STALE: &str = "a service it reaches runs a build other than its unit names";

/// Why a change is held open when systemd couldn't list what runs.
pub(super) const UNLISTED: &str = "cadence can't list the services running";

/// The programs that only run another, a unit's or a process's: the
/// program it's for is the first in the store after them.
const STARTERS: &[&str] = &["env", "bash", "sh", "dash", "python", "node", "perl"];

/// The services whose process runs another program than the one their
/// unit names, by design, each with why, so they aren't read as stale.
const RUN_ANOTHER: &[(&str, &str)] = &[(
    "sessions-kitty.service",
    "its unit waits, then starts the kitty the account's profile holds",
)];

/// A running service, by its unit's name, and whether its process runs
/// another build than its unit names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Service {
    pub(super) unit: String,
    pub(super) stale: bool,
}

/// Every running service, of the system's and the account's, whose build
/// cadence can read, as `Service`; `None` when systemd can't list them.
/// Each found running another build is said on stderr, with what it runs
/// and what its unit names, so the hour's journal names it. A service
/// naming no build in the store, as a script run by `bash -c` does, or
/// whose process the account can't read, as root's nix-daemon, is left
/// to its revision's verdict: either is so every hour, so holding on it
/// would hold every change to its folder for good. On optimus at 17:50
/// on 24 November, those were the two nix-daemon processes alone, and
/// no service read stale.
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
        let units: Vec<&str> = listed
            .lines()
            .filter_map(|line| line.split_whitespace().next())
            .filter(|unit| unit.ends_with(".service"))
            .collect();
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
        ];
        args.extend(&units);
        let shown = command("systemctl", &args).ok()?;
        for block in shown.split("\n\n") {
            let Some(unit) = field(block, "Id") else {
                continue;
            };
            if RUN_ANOTHER.iter().any(|(another, _)| *another == unit) {
                continue;
            }
            let named = field(block, "ExecStart")
                .and_then(named)
                .and_then(|path| resolved(&path));
            let pid = field(block, "MainPID")
                .and_then(|pid| pid.parse::<u32>().ok())
                .filter(|&pid| pid != 0);
            let (Some(named), Some(programs)) = (named, pid.map(running)) else {
                continue;
            };
            let runs_named = runs(&programs, &named, |root| {
                closures
                    .entry(root.to_owned())
                    .or_insert_with(|| closure(root))
                    .clone()
            });
            let Some(runs_named) = runs_named else {
                continue;
            };
            if !runs_named {
                eprintln!(
                    "cadence: {unit} runs {}, not {named}, which its unit names",
                    programs.join(" or "),
                );
            }
            services.push(Service {
                unit: unit.to_owned(),
                stale: !runs_named,
            });
        }
    }
    Some(services)
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

/// The program an `ExecStart` as `systemctl show` gives it names: its
/// `path`, or where that only starts another, as `env` does, the first
/// program in a store path's `bin` its arguments name after it, which a
/// setting such as `FOO=/nix/store/...` before it isn't.
pub(super) fn named(exec: &str) -> Option<String> {
    let part = |name: &str| {
        exec.split(" ; ")
            .find_map(|part| part.trim_start_matches(['{', ' ']).strip_prefix(name))
    };
    let path = part("path=")?.trim();
    if !starts_another(path) {
        return Some(path.to_owned());
    }
    part("argv[]=")?
        .split_whitespace()
        .skip(1)
        .find(|arg| arg.starts_with("/nix/store/") && arg.contains("/bin/") && !starts_another(arg))
        .map(str::to_owned)
}

/// Whether `path`'s program only runs another, by its name, as
/// `python3.12` is `python`'s.
fn starts_another(path: &str) -> bool {
    let name = path.rsplit('/').next().unwrap_or(path);
    let bare = name.trim_end_matches(|c: char| c.is_ascii_digit() || c == '.');
    STARTERS.contains(&bare)
}

/// `path`'s store path, through any link to it, as
/// `/run/current-system/sw/bin/x` leads to one; `None` for a program
/// outside the store, whose build cadence can't tell, or a bare name.
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
    programs(&args, exe.as_deref(), resolved)
}

/// The builds a process may run, by `resolve`, as `resolved` does: its
/// first two arguments', since an interpreter's second names its script,
/// and its executable's, which the account can't read of root's. Not a
/// later argument's, which is as often a data file in the store, as a
/// service's `words.json` is.
pub(super) fn programs(
    args: &[String],
    exe: Option<&str>,
    resolve: impl Fn(&str) -> Option<String>,
) -> Vec<String> {
    let found: BTreeSet<String> = args
        .iter()
        .take(2)
        .map(String::as_str)
        .chain(exe)
        .filter_map(resolve)
        .collect();
    found.into_iter().collect()
}

/// Whether a process running one of `programs`, the builds it may run,
/// runs the build of `named`, its unit's: one of them is it, or in its
/// closure, as a wrapper's target is, by `closure`; `None` when none
/// could be read or the closure can't be.
pub(super) fn runs(
    programs: &[String],
    named: &str,
    closure: impl FnOnce(&str) -> Option<BTreeSet<String>>,
) -> Option<bool> {
    let named = store_root(named)?;
    if programs.is_empty() {
        return None;
    }
    if programs.contains(&named) {
        return Some(true);
    }
    closure(&named).map(|closure| programs.iter().any(|program| closure.contains(program)))
}

/// Every store path `root` refers to, itself among them.
fn closure(root: &str) -> Option<BTreeSet<String>> {
    if !Path::new(root).exists() {
        return None;
    }
    let listed = command("nix-store", &["--query", "--requisites", root]).ok()?;
    Some(listed.lines().map(str::to_owned).collect())
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
/// it doesn't: one of them runs another build than its unit names, or
/// `services`, `None`, couldn't be listed.
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
    services
        .iter()
        .any(|service| service.stale && reached.iter().any(|name| reaches(name, &service.unit)))
        .then_some(STALE)
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
mod tests {
    use super::*;

    const NODE: &str = "/nix/store/p1m6i9bhgngwvizf0nsxn5gmb7qmcg1i-mycelium-0.1.0";
    const PINNED: &str = "/nix/store/20lhvy6h000000000000000000000000-mycelium-0.1.0";
    const COREUTILS: &str = "/nix/store/a50ml7za6h61m4mv4cdbvkl33c6sx5yn-coreutils-9.12";

    fn service(unit: &str, stale: bool) -> Service {
        Service {
            unit: unit.to_owned(),
            stale,
        }
    }

    fn owned(paths: &[&str]) -> Vec<String> {
        paths.iter().map(|path| (*path).to_owned()).collect()
    }

    #[test]
    fn a_unit_names_its_path_or_what_a_starter_runs() {
        let direct = format!(
            "{{ path={NODE}/bin/mycelium ; argv[]={NODE}/bin/mycelium --realm work node ; \
             ignore_errors=no ; start_time=[n/a] ; pid=0 ; code=(null) ; status=0/0 }}"
        );
        assert_eq!(
            named(&direct),
            Some(format!("{NODE}/bin/mycelium")),
            "a unit's own path wasn't read as what it names"
        );
        let started = format!(
            "{{ path={COREUTILS}/bin/env ; argv[]={COREUTILS}/bin/env -u INVOCATION_ID \
             {NODE}/bin/mycelium node ; ignore_errors=no }}"
        );
        assert_eq!(
            named(&started),
            Some(format!("{NODE}/bin/mycelium")),
            "env, which only starts the program, was read as what the unit names"
        );
        let set = format!(
            "{{ path={COREUTILS}/bin/env ; argv[]={COREUTILS}/bin/env \
             WORDS=/nix/store/ym0pgbab76dr2zxqnfnlcr97fzhh3ybs-words.json \
             {NODE}/bin/mycelium ; ignore_errors=no }}"
        );
        assert_eq!(
            named(&set),
            Some(format!("{NODE}/bin/mycelium")),
            "a setting naming a file in the store was read as the program env starts"
        );
        assert_eq!(
            named("{ argv[]=x ; ignore_errors=no }"),
            None,
            "a unit naming no path was read as naming one"
        );
    }

    #[test]
    fn a_process_may_run_its_first_two_arguments_or_its_executable() {
        let resolve = store_root;
        let python = "/nix/store/00000000000000000000000000000000-python3-3.12";
        let words = "/nix/store/ym0pgbab76dr2zxqnfnlcr97fzhh3ybs-words.json";
        assert_eq!(
            programs(
                &owned(&[&format!("{PINNED}/bin/mycelium"), "node", "--words", words]),
                None,
                resolve,
            ),
            owned(&[PINNED]),
            "the program a process was started as wasn't read, or a data file was"
        );
        assert_eq!(
            programs(
                &owned(&[
                    &format!("{python}/bin/python3.12"),
                    &format!("{NODE}/bin/script")
                ]),
                None,
                resolve,
            ),
            owned(&[python, NODE]),
            "an interpreter's script wasn't read beside it"
        );
        assert_eq!(
            programs(
                &owned(&["kitty"]),
                Some(&format!("{NODE}/bin/.kitty-wrapped")),
                resolve
            ),
            owned(&[NODE]),
            "a process started by a bare name wasn't read by its executable"
        );
        assert!(
            programs(&owned(&["nix-daemon", "--daemon"]), None, resolve).is_empty(),
            "root's process, read neither way, was given a build"
        );
    }

    #[test]
    fn a_service_runs_its_unit_s_build_or_one_in_its_closure() {
        let never = |_: &str| -> Option<BTreeSet<String>> { panic!("the closure was read") };
        assert_eq!(
            runs(&owned(&[NODE]), &format!("{NODE}/bin/mycelium"), never),
            Some(true),
            "a service running its unit's own build was read as stale"
        );
        let wrapped = "/nix/store/11111111111111111111111111111111-mycelium-unwrapped";
        let held = |_: &str| Some(BTreeSet::from([NODE.to_owned(), wrapped.to_owned()]));
        assert_eq!(
            runs(&owned(&[wrapped]), NODE, held),
            Some(true),
            "a wrapper's target, in the closure its unit names, was read as stale"
        );
        assert_eq!(
            runs(&owned(&[PINNED]), NODE, held),
            Some(false),
            "the node a drop-in pinned to 23 November's build read as running its unit's"
        );
        assert_eq!(
            runs(&owned(&[PINNED]), NODE, |_| None),
            None,
            "a closure that couldn't be read gave an answer"
        );
        assert_eq!(
            runs(&[], NODE, never),
            None,
            "a process whose build couldn't be read was judged"
        );
    }

    #[test]
    fn a_change_reaches_the_services_its_folders_name() {
        assert_eq!(
            reached("mycelium", None),
            BTreeSet::from(["mycelium".to_owned()]),
            "a mycelium change didn't reach mycelium's services"
        );
        assert_eq!(
            reached(
                "core",
                Some("modules/model-relay/src/a.rs\nflake.lock\nmodules/x/y.nix")
            ),
            BTreeSet::from(["model-relay".to_owned(), "x".to_owned()]),
            "a core change's folders under modules/ weren't what it reaches"
        );
        assert!(
            reached("world", Some("modules/x/y.nix")).is_empty(),
            "a change to a repository whose services cadence doesn't map reached some"
        );
        assert!(reaches("model-relay", "model-relay-agents.service"));
        assert!(reaches("mycelium", "mycelium.service"));
        assert!(reaches("pi", "pi-conversation-a@1.service"));
        assert!(
            !reaches("model", "modelling.service"),
            "a unit whose name only begins with the folder's letters was reached"
        );
    }

    #[test]
    fn a_change_waits_on_a_stale_service_it_reaches() {
        let mycelium = reached("mycelium", None);
        let pinned = [
            service("mycelium.service", true),
            service("x.service", false),
        ];
        assert_eq!(
            lack(&mycelium, Some(&pinned)),
            Some(STALE),
            "a mycelium merge read as running while its node ran a pinned older build"
        );
        let fine = [
            service("mycelium.service", false),
            service("x.service", true),
        ];
        assert_eq!(
            lack(&mycelium, Some(&fine)),
            None,
            "another folder's stale service held a change that doesn't reach it"
        );
        assert_eq!(
            lack(&mycelium, None),
            Some(UNLISTED),
            "a blind hour let a change reaching a service read as running"
        );
        assert_eq!(
            lack(&BTreeSet::new(), None),
            None,
            "a change reaching no service waited on the services"
        );
        assert!(
            UNLISTED.starts_with(super::super::UNSEEN) && !STALE.starts_with(super::super::UNSEEN),
            "cadence's own blindness wasn't said as it says it elsewhere"
        );
    }

    #[test]
    fn a_service_running_another_program_by_design_says_why() {
        assert!(
            RUN_ANOTHER
                .iter()
                .all(|(unit, why)| unit.ends_with(".service") && !why.is_empty()),
            "a service left out of the read doesn't say why"
        );
    }
}
