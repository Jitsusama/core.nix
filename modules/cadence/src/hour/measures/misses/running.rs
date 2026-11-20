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
/// World's from the last switch, the harness's from the version installed.
/// A change elsewhere, deployed apart, is followed no further than its
/// merge.
const RUN_HERE: &[&str] = &["core", "world", "harness"];

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
/// its repository's running revision carries its merge, and a core change
/// to the hosts' runtime once every running host also loads a runtime
/// `pin` shows built from a core carrying it. A change seen running is kept
/// in `folder`'s `ran.json` and never read again, so a closed miss isn't
/// reopened, and told fixed twice, by an hour that can't see for a moment:
/// a host restarting at :10, or a switch from a branch.
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
    let mut revs = switched(switches);
    if let Some(installed) = installed() {
        revs.insert("harness".to_owned(), installed);
    }
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
    let mut hosts: Option<Option<BTreeSet<Option<String>>>> = None;
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
            |sha| {
                let pins = hosts.get_or_insert_with(|| pinned(pin, repos));
                hosts_lack(pins.as_ref(), pin, |rev| carries(&path, sha, rev))
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

/// Why a change of `repo` merged as `merge` doesn't run here yet, if it
/// doesn't: `rev` is its repository's running revision, `carries` says
/// whether a revision holds a commit, `changed` the files a merge changed,
/// and `hosts` why the pi hosts lack a hosts' runtime change. What cadence
/// couldn't read counts as not running.
fn judge(
    repo: &str,
    rev: Option<&str>,
    merge: Option<&str>,
    carries: impl Fn(&str, &str) -> bool,
    changed: impl Fn(&str) -> Option<String>,
    hosts: impl FnOnce(&str) -> Option<&'static str>,
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
    if !carries(sha, rev) {
        return Some(if repo == "harness" {
            "the harness installed doesn't carry it"
        } else {
            "the last switch doesn't carry it"
        });
    }
    if repo != "core" {
        return None;
    }
    match changed(sha) {
        None => Some("cadence can't read what it changed"),
        Some(files) if hosted(&files) => hosts(sha),
        Some(_) => None,
    }
}

/// Why the hosts don't load a hosted change, if they don't: the core each
/// running host's runtime was pinned with, `None` for one whose pin wasn't
/// found, must carry it. `None` for them all is hosts cadence couldn't list.
fn hosts_lack(
    pins: Option<&BTreeSet<Option<String>>>,
    pin: Option<Pin<'_>>,
    carries: impl Fn(&str) -> bool,
) -> Option<&'static str> {
    match pins {
        _ if pin.is_none() => Some("cadence can't see what the pi hosts load"),
        None => Some("cadence can't list the pi hosts"),
        Some(pins) if pins.is_empty() => Some("no pi host runs to load it"),
        Some(pins) if pins.iter().any(|rev| !rev.as_deref().is_some_and(&carries)) => {
            Some("the pi hosts don't load it yet")
        }
        Some(_) => None,
    }
}

/// Whether `text` is a commit's name git can only read as one: hex, at
/// least seven long, so nothing from a state file reaches git as an option.
fn is_rev(text: &str) -> bool {
    (7..=40).contains(&text.len()) && text.bytes().all(|b| b.is_ascii_hexdigit())
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
                    .filter_map(|(repo, rev)| Some((repo.clone(), rev.as_str()?)))
                    .filter(|(_, rev)| is_rev(rev))
                    .map(|(repo, rev)| (repo, rev.to_owned()))
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

/// Whether `rev` carries `sha`. An unknown revision carries nothing.
fn carries(path: &str, sha: &str, rev: &str) -> bool {
    command(
        "git",
        &["-C", path, "merge-base", "--is-ancestor", sha, rev],
    )
    .is_ok()
}

/// The files the merge `sha` changed on its main, if git could say.
fn changed(path: &str, sha: &str) -> Option<String> {
    command(
        "git",
        &["-C", path, "diff", "--name-only", &format!("{sha}^1"), sha],
    )
    .ok()
}

/// Whether a change touching `files` reaches the pi hosts' runtime.
fn hosted(files: &str) -> bool {
    files
        .lines()
        .any(|file| HOSTED.iter().any(|hosted| file.starts_with(hosted)))
}

/// The core each running pi host's runtime was pinned with, `None` for a
/// runtime `pin` names in no commit it reads, or a host whose runtime
/// couldn't be read; no set at all when the hosts couldn't be listed.
fn pinned(pin: Option<Pin<'_>>, repos: &Checkouts) -> Option<BTreeSet<Option<String>>> {
    let (Some(pin), Some(checkout)) = (pin, pin.and_then(|pin| repos.get(pin.checkout))) else {
        return Some(BTreeSet::new());
    };
    let checkout = checkout.to_string_lossy();
    let main = main_of(&checkout);
    let flake = format!("{}/flake.nix", pin.folder);
    let show = |commit: &str, file: &str| {
        command(
            "git",
            &["-C", &checkout, "show", &format!("{commit}:{file}")],
        )
        .ok()
    };
    Some(
        hosts()?
            .iter()
            .map(|runtime| {
                let runtime = runtime.as_deref()?;
                let touched = command(
                    "git",
                    &[
                        "-C",
                        &checkout,
                        "log",
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
                let commit = first_naming(&touched, |commit| {
                    show(commit, &flake).is_some_and(|text| text.contains(runtime))
                })?;
                core_pin(&show(commit, &format!("{}/flake.lock", pin.folder))?)
            })
            .collect(),
    )
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

/// The core revision a `flake.lock` pins, by the root's input named core.
fn core_pin(lock: &str) -> Option<String> {
    let lock: Value = serde_json::from_str(lock).ok()?;
    let node = lock["nodes"]["root"]["inputs"]["core"].as_str()?;
    lock["nodes"][node]["locked"]["rev"]
        .as_str()
        .filter(|rev| is_rev(rev))
        .map(str::to_owned)
}

/// The runtime each running pi host loads its pi from, `None` for a host
/// whose runtime couldn't be read. A host is the main process of a unit
/// core's host module names `pi-host-REALM-KIND`, so what it starts, which
/// keeps its environment after the host moves on, isn't read.
fn hosts() -> Option<BTreeSet<Option<String>>> {
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
                let pid = command(
                    "systemctl",
                    &["--user", "show", "-p", "MainPID", "--value", unit],
                )
                .ok()?;
                let pid = pid.trim();
                if pid == "0" || !pid.bytes().all(|b| b.is_ascii_digit()) {
                    return None;
                }
                runtime(&std::fs::read(format!("/proc/{pid}/environ")).ok()?)
            })
            .collect(),
    )
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
mod tests {
    use super::*;

    const OLD: &str = "xi5w06adwa04v6ifnajk1l37fp9hn79c";

    #[test]
    fn the_last_whole_switch_names_what_runs() {
        let switches = concat!(
            "{\"generation\":1,\"revs\":{\"core\":\"aaaaaaa\",\"world\":\"0000000\"}}\n",
            "{\"generation\":2,\"revs\":{\"core\":\"bbbbbbb\",\"world\":\"0000000\"}}\n",
            "{\"generation\":3,\"revs\":{\"core\":\"cc",
        );
        assert_eq!(
            switched(switches),
            BTreeMap::from([
                ("core".to_owned(), "bbbbbbb".to_owned()),
                ("world".to_owned(), "0000000".to_owned()),
            ]),
        );
        assert!(switched("").is_empty(), "no switch named a revision");
        assert_eq!(
            switched("{\"revs\":{\"core\":\"--output=/x\",\"world\":\"abc\"}}"),
            BTreeMap::new(),
            "what isn't a commit's name reached git"
        );
    }

    #[test]
    fn what_cadence_cant_read_counts_as_not_running() {
        let all = |_: &str, _: &str| true;
        let none = |_: &str, _: &str| false;
        let files = |_: &str| Some("modules/cadence/src/main.rs".to_owned());
        let hosted = |_: &str| Some("modules/pi/host-patches/x.patch".to_owned());
        let lack = |_: &str| Some("the pi hosts don't load it yet");
        let never = |_: &str| -> Option<&'static str> { panic!("a host was read") };
        let at = Some("a1b2c3d");
        assert_eq!(
            judge("core", at, Some("f00d"), none, files, never),
            Some("the last switch doesn't carry it"),
            "the 21:10 case: merged, but not switched in"
        );
        assert_eq!(
            judge("core", at, Some("f00d"), all, hosted, lack),
            Some("the pi hosts don't load it yet"),
            "switched in, but the hosts still load the older pi"
        );
        assert_eq!(judge("core", at, Some("f00d"), all, files, never), None);
        assert_eq!(
            judge("core", None, Some("f00d"), all, files, never),
            Some("cadence can't see what runs here"),
            "no switch read, yet the change counted as running"
        );
        assert_eq!(
            judge("core", at, None, all, files, never),
            Some("cadence can't find its merge")
        );
        assert_eq!(
            judge("core", at, Some("f00d"), all, |_| None, never),
            Some("cadence can't read what it changed"),
            "an unread diff skipped the hosts"
        );
        assert_eq!(
            judge("harness", at, Some("f00d"), none, files, never),
            Some("the harness installed doesn't carry it")
        );
        assert_eq!(
            judge("world", at, Some("f00d"), all, |_| None, never),
            None,
            "World's changes don't reach the hosts' runtime"
        );
        assert_eq!(
            judge("mycelium", None, None, none, files, never),
            None,
            "a repository deployed apart is followed to its merge"
        );
    }

    #[test]
    fn a_merge_is_found_by_its_subject_not_a_later_body() {
        let found = concat!(
            "1111111 fix(pi): quote the crash (#5800)\n",
            "2222222 Merge pull request #5800 from me/fix\n",
        );
        assert_eq!(subject_merging(found, 5800), Some("1111111".to_owned()));
        let body = "3333333 Merge pull request #5801 from me/next\n";
        assert_eq!(
            subject_merging(body, 5800),
            None,
            "a later merge whose body named the change was taken as its merge"
        );
    }

    #[test]
    fn a_runtime_is_pinned_where_it_was_first_named() {
        let touched = "dropped\nrenamed\nadded\n";
        let names = |commit: &str| commit != "dropped";
        assert_eq!(
            first_naming(touched, names),
            Some("added"),
            "the commit dropping the runtime, or a later one, was read as its pin"
        );
        assert_eq!(
            first_naming("", |_| true),
            None,
            "a runtime pinned nowhere was read against the working lock"
        );
        assert_eq!(
            first_naming("dropped\n", |_| false),
            None,
            "a commit that only dropped the runtime counted as its pin"
        );
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
        let environ =
            format!("HOME=/x\0PI_PACKAGE_DIR=/nix/store/{OLD}-pi-host-runtime-1.5.0/lib/pi\0");
        assert_eq!(runtime(environ.as_bytes()), Some(OLD.to_owned()));
        let other = format!("PI_PACKAGE_DIR=/nix/store/{OLD}-pi-1.5.0/lib\0");
        assert_eq!(runtime(other.as_bytes()), None, "a pi not a host's runtime");
        assert_eq!(
            runtime(b"PI_PACKAGE_DIR=/home/x/pi\0"),
            None,
            "a pi outside the store"
        );
        assert_eq!(runtime(b"HOME=/home/x\0"), None, "no pi named");
        assert!(
            host_unit("pi-host-work-background.service"),
            "a host's unit wasn't a host"
        );
        assert!(
            !host_unit("pi-host-down@pi-host-work-attended.service"),
            "the unit saying a host is down was a host"
        );
        assert!(!host_unit("pi-hostess.service"), "another unit was a host");
    }

    #[test]
    fn a_lock_pins_core_by_the_roots_input() {
        let lock = r#"{"nodes": {
            "root": {"inputs": {"core": "core_2", "nixpkgs": "nixpkgs"}},
            "core": {"locked": {"rev": "0123456"}},
            "core_2": {"locked": {"rev": "abcdef0"}}}}"#;
        assert_eq!(core_pin(lock), Some("abcdef0".to_owned()));
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
        assert_eq!(hosts_lack(Some(&pins(&[Some("new")])), pin, has), None);
        assert_eq!(
            hosts_lack(Some(&pins(&[Some("new"), Some("old")])), pin, has),
            Some("the pi hosts don't load it yet"),
            "one host on the old runtime left it unrun"
        );
        assert_eq!(
            hosts_lack(Some(&pins(&[Some("new"), None])), pin, has),
            Some("the pi hosts don't load it yet"),
            "a runtime pinned nowhere cadence reads counted as carrying it"
        );
        assert_eq!(
            hosts_lack(Some(&pins(&[])), pin, has),
            Some("no pi host runs to load it")
        );
        assert_eq!(
            hosts_lack(None, pin, has),
            Some("cadence can't list the pi hosts"),
            "hosts cadence couldn't list counted as none"
        );
        assert_eq!(
            hosts_lack(Some(&pins(&[Some("new")])), None, has),
            Some("cadence can't see what the pi hosts load"),
            "with no pin named, a host's runtime counted as followed"
        );
    }

    #[test]
    fn a_change_seen_running_is_kept_and_not_read_again() {
        let folder = std::env::temp_dir().join(format!("cadence-ran-{}", std::process::id()));
        std::fs::create_dir_all(&folder).expect("a scratch folder");
        let mycelium = Named {
            repo: "mycelium".into(),
            number: 7,
        };
        let repos = Checkouts::from([("mycelium".to_owned(), folder.clone())]);
        let merged = BTreeSet::from([mycelium.clone()]);
        assert!(
            unrun(&merged, &repos, "", None, &folder).is_empty(),
            "a change deployed apart was held"
        );
        let kept: BTreeSet<Named> =
            serde_json::from_str(&std::fs::read_to_string(folder.join("ran.json")).expect("kept"))
                .expect("a set");
        assert_eq!(kept, merged, "the change seen running wasn't kept");
        let core = Named {
            repo: "core".into(),
            number: 1,
        };
        let repos = Checkouts::from([("core".to_owned(), folder.clone())]);
        let held = unrun(&BTreeSet::from([core.clone()]), &repos, "", None, &folder);
        assert_eq!(
            held.get(&core),
            Some(&"cadence can't see what runs here"),
            "a core change with no switch read counted as running"
        );
        std::fs::remove_dir_all(&folder).expect("the scratch folder removed");
    }
}
