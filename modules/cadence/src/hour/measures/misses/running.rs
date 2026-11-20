//! Whether a merged check runs on this machine yet, since a merge isn't a
//! fix on Joel's machine: at 21:10 on 19 November the card told him his
//! 19:05 miss, the background pi host's crash, was fixed on core #5800's
//! merge, while the last switch predated it and the host still loaded the
//! older runtime's pi.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde_json::Value;

use super::{Checkouts, Named, main_of};
use crate::command;

/// What a miss's line says between its change and why, after its check's
/// note, when the change merged and doesn't run here yet. The card reads
/// it to tell the miss fixed again once it runs, and the owner's note to
/// ask when it will rather than which check.
pub const UNRUN: &str = " merged, but ";

/// The files of core a change carries into the pi hosts' runtime, which a
/// host loads only once it restarts onto a runtime built with them, apart
/// from any switch.
const HOSTED: &[&str] = &["modules/pi/host-patches/", "modules/pi/hosted-pi.nix"];

/// How far back the commit pinning a host's runtime is looked for. A
/// runtime pinned before then is taken to carry no change merged since.
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

/// Which of `merged` don't run here yet, each with why. A change runs once
/// its repository's running revision carries its merge: core's and any
/// other the last switch in `switches` names, by that switch's revision,
/// and the harness's by the version installed. A core change to the hosts'
/// runtime runs only once every running host also loads a runtime `pin`
/// shows built from a core carrying it. A change in a repository with no
/// running revision cadence knows, such as one deployed apart, is followed
/// no further than its merge.
#[must_use]
pub fn unrun(
    merged: &BTreeSet<Named>,
    repos: &Checkouts,
    switches: &str,
    pin: Option<Pin<'_>>,
) -> BTreeMap<Named, &'static str> {
    let mut revs = switched(switches);
    if let Some(installed) = installed() {
        revs.insert("harness".to_owned(), installed);
    }
    // A pin in a checkout cadence wasn't given can't be read.
    let pin = pin.filter(|pin| repos.contains_key(pin.checkout));
    let mut hosts: Option<BTreeSet<Option<String>>> = None;
    let mut unrun = BTreeMap::new();
    for change in merged {
        let (Some(rev), Some(path)) = (revs.get(&change.repo), repos.get(&change.repo)) else {
            continue;
        };
        let path = path.to_string_lossy();
        let why = match merge_of(&path, change.number) {
            None => Some("cadence can't find its merge"),
            Some(sha) if !carries(&path, &sha, rev) => Some(if change.repo == "harness" {
                "the harness installed doesn't carry it"
            } else {
                "the last switch doesn't carry it"
            }),
            Some(sha) if change.repo == "core" && hosted(&changed(&path, &sha)) => {
                let pins = hosts.get_or_insert_with(|| pinned(pin, repos));
                hosts_lack(pins, pin, |rev| carries(&path, &sha, rev))
            }
            Some(_) => None,
        };
        if let Some(why) = why {
            unrun.insert(change.clone(), why);
        }
    }
    unrun
}

/// Why the hosts don't load a hosted change, if they don't: the core each
/// running host's runtime was pinned with, `None` for one whose pin wasn't
/// found, must carry it.
fn hosts_lack(
    pins: &BTreeSet<Option<String>>,
    pin: Option<Pin<'_>>,
    carries: impl Fn(&str) -> bool,
) -> Option<&'static str> {
    if pin.is_none() {
        Some("cadence can't see what the pi hosts load")
    } else if pins.is_empty() {
        Some("no pi host runs to load it")
    } else if pins.iter().any(|rev| !rev.as_deref().is_some_and(&carries)) {
        Some("the pi hosts don't load it yet")
    } else {
        None
    }
}

/// The revisions the last switch was built from, by repository, from
/// integration's `switches.jsonl`. A torn last line leaves the one before
/// it, which carries no more.
fn switched(text: &str) -> BTreeMap<String, String> {
    text.lines()
        .rev()
        .find_map(|line| serde_json::from_str::<Value>(line).ok())
        .and_then(|switch| {
            switch["revs"].as_object().map(|revs| {
                revs.iter()
                    .filter_map(|(repo, rev)| Some((repo.clone(), rev.as_str()?.to_owned())))
                    .collect()
            })
        })
        .unwrap_or_default()
}

/// The harness's commit installed, as the name of the version pi-packages'
/// `current` points at.
fn installed() -> Option<String> {
    let mirror = crate::axes::landings::harness_mirror();
    let current = std::fs::read_link(mirror.parent()?.join("current")).ok()?;
    Some(current.file_name()?.to_string_lossy().into_owned())
}

/// The commit on `path`'s main that merged pull request `number`, as GitHub
/// or a squash names it.
fn merge_of(path: &str, number: u64) -> Option<String> {
    let merged = format!("--grep=^Merge pull request #{number} from ");
    let squashed = format!("--grep=\\(#{number}\\)$");
    let sha = command(
        "git",
        &[
            "-C",
            path,
            "log",
            "--first-parent",
            "-1",
            "--format=%H",
            "-E",
            &merged,
            &squashed,
            main_of(path),
        ],
    )
    .ok()?;
    let sha = sha.trim();
    (!sha.is_empty()).then(|| sha.to_owned())
}

/// Whether `rev` carries `sha`. An unknown revision carries nothing.
fn carries(path: &str, sha: &str, rev: &str) -> bool {
    command(
        "git",
        &["-C", path, "merge-base", "--is-ancestor", sha, rev],
    )
    .is_ok()
}

/// The files the merge `sha` changed on its main.
fn changed(path: &str, sha: &str) -> String {
    command(
        "git",
        &["-C", path, "diff", "--name-only", &format!("{sha}^1"), sha],
    )
    .unwrap_or_default()
}

/// Whether a change touching `files` reaches the pi hosts' runtime.
fn hosted(files: &str) -> bool {
    files
        .lines()
        .any(|file| HOSTED.iter().any(|hosted| file.starts_with(hosted)))
}

/// The core each running pi host's runtime was pinned with, `None` for a
/// runtime `pin` names in no commit it reads.
fn pinned(pin: Option<Pin<'_>>, repos: &Checkouts) -> BTreeSet<Option<String>> {
    let (Some(pin), Some(checkout)) = (pin, pin.and_then(|pin| repos.get(pin.checkout))) else {
        return BTreeSet::new();
    };
    let checkout = checkout.to_string_lossy();
    let main = main_of(&checkout);
    let flake = format!("{}/flake.nix", pin.folder);
    hosts()
        .iter()
        .map(|runtime| {
            let commit = command(
                "git",
                &[
                    "-C",
                    &checkout,
                    "log",
                    "-1",
                    "--format=%H",
                    PINNED_WITHIN,
                    "-S",
                    runtime,
                    main,
                    "--",
                    &flake,
                ],
            )
            .ok()?;
            let lock = format!("{}:{}/flake.lock", commit.trim(), pin.folder);
            core_pin(&command("git", &["-C", &checkout, "show", &lock]).ok()?)
        })
        .collect()
}

/// The core revision a `flake.lock` pins, by the root's input named core.
fn core_pin(lock: &str) -> Option<String> {
    let lock: Value = serde_json::from_str(lock).ok()?;
    let node = lock["nodes"]["root"]["inputs"]["core"].as_str()?;
    lock["nodes"][node]["locked"]["rev"]
        .as_str()
        .map(str::to_owned)
}

/// Every runtime a running pi host loads its pi from.
fn hosts() -> BTreeSet<String> {
    let Ok(processes) = std::fs::read_dir("/proc") else {
        return BTreeSet::new();
    };
    processes
        .flatten()
        .map(|process| process.path())
        .filter(|path| std::fs::read(path.join("cmdline")).is_ok_and(|cmdline| host(&cmdline)))
        .filter_map(|path| runtime(&std::fs::read(path.join("environ")).ok()?))
        .collect()
}

/// Whether a process's command line is a pi host's, started by its
/// runtime's own node. Only the host itself is read, since what it starts
/// keeps its environment after the host moves on.
fn host(cmdline: &[u8]) -> bool {
    cmdline
        .split(|&b| b == 0)
        .next()
        .is_some_and(|program| program.ends_with(b"/libexec/pi-host/node"))
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
mod tests {
    use super::*;

    const OLD: &str = "xi5w06adwa04v6ifnajk1l37fp9hn79c";

    #[test]
    fn the_last_whole_switch_names_what_runs() {
        let switches = concat!(
            "{\"generation\":1,\"revs\":{\"core\":\"aaa\",\"world\":\"www\"}}\n",
            "{\"generation\":2,\"revs\":{\"core\":\"bbb\",\"world\":\"www\"}}\n",
            "{\"generation\":3,\"revs\":{\"core\":\"cc",
        );
        assert_eq!(
            switched(switches),
            BTreeMap::from([
                ("core".to_owned(), "bbb".to_owned()),
                ("world".to_owned(), "www".to_owned()),
            ]),
        );
        assert!(switched("").is_empty(), "no switch named a revision");
    }

    #[test]
    fn only_the_hosts_runtime_files_need_the_hosts() {
        assert!(
            hosted("check-descriptions.nix\nmodules/pi/host-patches/x.patch\n"),
            "a host patch didn't need the hosts"
        );
        assert!(
            hosted("modules/pi/hosted-pi.nix"),
            "the runtime's build didn't"
        );
        assert!(
            !hosted("modules/pi/home.nix\nmodules/cadence/src/main.rs"),
            "a switch alone didn't carry the rest"
        );
    }

    #[test]
    fn a_host_is_read_by_the_runtime_its_pi_loads_from() {
        let environ = format!(
            "HOME=/x\0PI_PACKAGE_DIR=/nix/store/{OLD}-pi-host-runtime-1.5.0/lib/pi\0"
        );
        assert_eq!(runtime(environ.as_bytes()), Some(OLD.to_owned()));
        let other = format!("PI_PACKAGE_DIR=/nix/store/{OLD}-pi-1.5.0/lib\0");
        assert_eq!(runtime(other.as_bytes()), None, "a pi not a host's runtime");
        assert_eq!(
            runtime(b"PI_PACKAGE_DIR=/home/x/pi\0"),
            None,
            "a pi outside the store"
        );
        assert_eq!(runtime(b"HOME=/home/x\0"), None, "no pi named");
        let node = format!("/nix/store/{OLD}-pi-host-runtime-1.5.0/libexec/pi-host/node\0--x\0");
        assert!(host(node.as_bytes()), "the host's own node wasn't a host");
        assert!(!host(b"bash\0-c\0pi-host\0"), "what a host started was one");
    }

    #[test]
    fn a_lock_pins_core_by_the_roots_input() {
        let lock = r#"{"nodes": {
            "root": {"inputs": {"core": "core_2", "nixpkgs": "nixpkgs"}},
            "core": {"locked": {"rev": "old"}},
            "core_2": {"locked": {"rev": "abc"}}}}"#;
        assert_eq!(core_pin(lock), Some("abc".to_owned()));
        assert_eq!(
            core_pin(r#"{"nodes": {"root": {"inputs": {}}}}"#),
            None,
            "no core input"
        );
        assert_eq!(core_pin("not json"), None, "a torn lock");
    }

    #[test]
    fn a_pin_names_a_checkout_and_a_folder() {
        assert_eq!(
            Pin::parse("there=a/b/"),
            Some(Pin {
                checkout: "there",
                folder: "a/b",
            })
        );
        assert_eq!(Pin::parse("there="), None, "no folder");
        assert_eq!(Pin::parse("=a"), None, "no checkout");
        assert_eq!(Pin::parse("there"), None, "no folder at all");
    }

    #[test]
    fn every_host_must_load_a_runtime_pinned_with_the_change() {
        let pin = Pin::parse("there=a");
        let has = |rev: &str| rev == "new";
        let pins = |revs: &[Option<&str>]| -> BTreeSet<Option<String>> {
            revs.iter().map(|rev| rev.map(str::to_owned)).collect()
        };
        assert_eq!(hosts_lack(&pins(&[Some("new")]), pin, has), None);
        assert_eq!(
            hosts_lack(&pins(&[Some("new"), Some("old")]), pin, has),
            Some("the pi hosts don't load it yet"),
            "one host on the old runtime left it unrun"
        );
        assert_eq!(
            hosts_lack(&pins(&[Some("new"), None]), pin, has),
            Some("the pi hosts don't load it yet"),
            "a runtime pinned nowhere cadence reads counted as carrying it"
        );
        assert_eq!(
            hosts_lack(&pins(&[]), pin, has),
            Some("no pi host runs to load it")
        );
        assert_eq!(
            hosts_lack(&pins(&[Some("new")]), None, has),
            Some("cadence can't see what the pi hosts load"),
            "with no pin named, a host's runtime counted as followed"
        );
    }
}
