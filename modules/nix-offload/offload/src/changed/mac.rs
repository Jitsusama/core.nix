//! A change a Mac's checks read is compared on a Mac too, before its run's
//! pass is kept.
//!
//! A gate compared only Linux's checks, so a change that broke a Mac's
//! merged unseen: on 15 November a Mac built its whole set with no Linux
//! builder for the first time, and 53 of its 256 checks failed or didn't
//! evaluate, none of them ever having run. Once a Mac nearby builds for
//! [`SYSTEM`], a run that builds compares that system's checks as well,
//! when the change touches a file a Mac's checks read ([`reads`]): the
//! lent builder names what changed and the Mac builds it, as any run for
//! its system does. A failure there fails the run. A Mac that doesn't
//! answer can't hold up a merge for as long as it's away, so the run
//! passes, and its record keeps that the Mac's comparison was skipped, for
//! the net to run once the Mac is back. So does one that answers and then
//! takes past [`LONGEST`], or stops answering before it ends, and one
//! under `--lent-only`, which keeps a fenced session's commit off every
//! machine that builds as the person, as a Mac nearby does.

use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;

use super::wanted;
use crate::there::Load;

/// The system a Mac builds for.
pub const SYSTEM: &str = "aarch64-darwin";

/// How long a Mac's comparison may take before it's ended and kept as
/// skipped, so one that stalls never holds a gate. On 15 November a Mac
/// evaluated and built its whole set of 205 checks, nothing kept, in 8
/// minutes; a change's comparison builds a few of them, after an
/// evaluation of each side on a lent builder, which may take as long as
/// any run's (`EVALUATING_LONGEST`).
pub const LONGEST: Duration = Duration::from_mins(20);

/// [`LONGEST`], or as many seconds as `NIX_OFFLOAD_MAC_LONGEST` says, as a
/// test asks.
pub fn longest() -> Duration {
    std::env::var("NIX_OFFLOAD_MAC_LONGEST")
        .ok()
        .and_then(|seconds| seconds.parse().ok())
        .map_or(LONGEST, Duration::from_secs)
}

/// `took`, as a person reads it: in minutes when it's whole minutes.
pub fn said(took: Duration) -> String {
    let seconds = took.as_secs();
    if seconds >= 60 && seconds.is_multiple_of(60) {
        format!("{} minutes", seconds / 60)
    } else {
        format!("{seconds} seconds")
    }
}

/// The files a Mac's checks take in, by name wherever they are: what a
/// module gives a Mac or both systems, the checks themselves, and what
/// builds a package. A Mac's checks read most home modules, so a shared
/// one that fails only on a Mac was the commonest break. Exact, per check,
/// once a Mac keeps what each of its checks read, as Linux's do.
const READ_BY_NAME: [&str; 5] = [
    "darwin.nix",
    "system.nix",
    "checks.nix",
    "home.nix",
    "package.nix",
];

/// The files every check reads, at the top of the repository.
const READ_AT_THE_TOP: [&str; 2] = ["flake.nix", "flake.lock"];

/// Whether a Mac's checks read the file at `path`, from the top of the
/// repository.
pub fn reads(path: &str) -> bool {
    let name = path.rsplit('/').next().unwrap_or(path);
    let a_modules_own = path
        .strip_prefix("modules/")
        .and_then(|rest| rest.strip_suffix("/default.nix"))
        .is_some_and(|module| !module.contains('/'));
    READ_AT_THE_TOP.contains(&path) || READ_BY_NAME.contains(&name) || a_modules_own
}

/// The first of `touched`, files of the tree in `folder`, that a Mac's
/// checks read, or none. By name alone, unless a lent builder keeps a
/// Mac's checks evaluated (`kept`, asked only when a file is in a
/// package's folder and none is read by name): then a file in a package's own
/// folder, one holding a `package.nix`, counts too, since a Mac builds
/// most packages from their sources there, and the lent builder names
/// exactly which of its checks such a change reaches, at the cost of a
/// trace. Without that record it would evaluate all of a Mac's checks
/// for every change to any package's sources, so it's left out.
pub fn read_among<'a>(
    folder: &Path,
    touched: &'a [String],
    kept: impl FnOnce() -> bool,
) -> Option<&'a String> {
    let in_a_package = |path: &&String| {
        Path::new(path.as_str())
            .ancestors()
            .skip(1)
            .take_while(|above| !above.as_os_str().is_empty())
            .any(|above| folder.join(above).join("package.nix").is_file())
    };
    touched
        .iter()
        .find(|path| reads(path))
        .or_else(|| touched.iter().find(in_a_package).filter(|_| kept()))
}

/// The files the tree in `folder` changes from `base`, committed or not,
/// or none when git can't say.
pub fn touched(folder: &Path, base: &str) -> Vec<String> {
    // A fenced session's tree may name programs for git to start, as
    // there.rs's git says, and listing what changed needs none.
    Command::new("git")
        .args([
            "-c",
            "core.hooksPath=/dev/null",
            "-c",
            "core.fsmonitor=false",
        ])
        .arg("-C")
        .arg(folder)
        .args(["diff", "--name-only", base])
        .stderr(Stdio::null())
        .output()
        .ok()
        .filter(|listed| listed.status.success())
        .map(|listed| {
            String::from_utf8_lossy(&listed.stdout)
                .lines()
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

/// Whether a run with `arguments` is one a Mac's comparison joins: one
/// that builds, for a system other than a Mac's, rather than one that
/// only names what changed or keeps a base.
pub fn joins(arguments: &[String]) -> bool {
    let asked = wanted(arguments);
    asked.build
        && !arguments.iter().any(|word| word == "--keep-base")
        && asked.system.as_deref() != Some(SYSTEM)
}

/// `arguments`, for a Mac's system rather than the one they name.
pub fn for_a_mac(arguments: &[String]) -> Vec<String> {
    let mut mac = arguments.to_vec();
    match mac.iter().position(|word| word == "--system") {
        Some(at) if at + 1 < mac.len() => SYSTEM.clone_into(&mut mac[at + 1]),
        _ => mac.extend(["--system".to_owned(), SYSTEM.to_owned()]),
    }
    mac
}

/// Whether a run `asked` for a system, from a machine building for `here`,
/// asks a Mac for room first: one that `builds` for a Mac's system from
/// elsewhere, which only a Mac can build. A Mac's checks need Linux
/// builds, which it gives its own Linux VM, so a run sent to a busy Mac
/// adds the VM's processors to its load: on 17 November the net's run for
/// a Mac, which no gate's room held, woke its 12-processor VM at a load
/// over 100 on 12 processors, and the load reached 264. One that only
/// names what changed is evaluated on a lent builder, and goes.
pub fn asks_for_room(asked: Option<&str>, here: &str, builds: bool) -> bool {
    builds && asked == Some(SYSTEM) && here != SYSTEM
}

/// Whether a run for a Mac's system goes to the Mac `host`, as `load`
/// found it: `Ok` saying the load it goes at, or `Err` saying why it
/// waits. A Mac too busy to say its load has no room either, and nor has
/// one that said nothing a load reads from: an ssh that couldn't finish
/// its handshake says nothing, and a run sent on would only find the same
/// Mac.
pub fn room(host: &str, load: Result<Option<Load>, String>) -> Result<String, String> {
    match load {
        Ok(Some(load)) if load.over() => Err(format!("{host} has no room, at {}", load.said())),
        Ok(Some(load)) => Ok(format!("{host} has room, at {}", load.said())),
        Ok(None) => Err(format!("{host} didn't say its load")),
        Err(why) => Err(why),
    }
}

/// [`room`], for a run that waits its turn behind any other: a gate's or
/// the net's comparison, which is owed rather than wanted now, so it waits
/// while another run sent to the Mac builds there, however light its load.
/// At 04:50 on 18 November the net's comparison found a Mac at 35 over
/// five, under its room, while a session's darwin build ran there, and
/// the two took it to 60. A person's own part for a Mac takes [`room`]
/// alone, since its turn there bounds it beside another.
pub fn room_alone(host: &str, load: Result<Option<Load>, String>) -> Result<String, String> {
    match load {
        Ok(Some(load)) if load.building > 0 && !load.over() => Err(format!(
            "{host} has no room, building {} run{} sent to it already, at {}",
            load.building,
            if load.building == 1 { "" } else { "s" },
            load.said()
        )),
        load => room(host, load),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(text: &str) -> Vec<String> {
        text.split_whitespace().map(str::to_owned).collect()
    }

    #[test]
    fn a_run_for_a_mac_past_three_a_processor_waits_and_one_within_goes() {
        let at = |minute: f64, five: f64| {
            Ok(Some(Load {
                minute,
                five,
                cores: 12,
                building: 0,
            }))
        };
        let busy = |why: &str| Err(why.to_owned());
        assert_eq!(
            room("skiff", at(140.0, 120.0)),
            busy(
                "skiff has no room, at load 140.0 over a minute and 120.0 over five on 12 processors"
            ),
            "a run for a Mac far past its room went"
        );
        assert_eq!(
            room("skiff", at(30.0, 36.6)),
            busy(
                "skiff has no room, at load 30.0 over a minute and 36.6 over five on 12 processors"
            ),
            "a run for a Mac just past three a processor went"
        );
        assert_eq!(
            room("skiff", at(36.0, 36.0)),
            Ok(
                "skiff has room, at load 36.0 over a minute and 36.0 over five on 12 processors"
                    .into()
            ),
            "a run for a Mac at three a processor waited"
        );
        assert_eq!(
            room("skiff", Ok(None)),
            busy("skiff didn't say its load"),
            "a Mac that said nothing a load reads from was taken to have room"
        );
        assert_eq!(
            room("skiff", Err("slow".to_owned())),
            busy("slow"),
            "a Mac too busy to say its load was taken to have room"
        );
    }

    #[test]
    fn a_run_for_a_mac_waits_while_another_builds_there_however_light_its_load() {
        let building = |building: u32| {
            Ok(Some(Load {
                minute: 20.0,
                five: 35.0,
                cores: 12,
                building,
            }))
        };
        assert_eq!(
            room_alone("skiff", building(1)),
            Err(
                "skiff has no room, building 1 run sent to it already, at load 20.0 \
                 over a minute and 35.0 over five on 12 processors"
                    .to_owned()
            ),
            "a run for a Mac went beside a build already there"
        );
        assert_eq!(
            room_alone("skiff", building(0)),
            Ok(
                "skiff has room, at load 20.0 over a minute and 35.0 over five on 12 processors"
                    .to_owned()
            ),
            "a run for a Mac building nothing waited"
        );
        assert_eq!(
            room("skiff", building(1)),
            Ok(
                "skiff has room, at load 20.0 over a minute and 35.0 over five on 12 processors"
                    .to_owned()
            ),
            "a person's own part for a Mac waited on another's build, which its turn bounds"
        );
        assert_eq!(
            room_alone(
                "skiff",
                Ok(Some(Load {
                    minute: 20.0,
                    five: 60.0,
                    cores: 12,
                    building: 1,
                }))
            ),
            Err(
                "skiff has no room, at load 20.0 over a minute and 60.0 over five on 12 processors"
                    .to_owned()
            ),
            "a Mac past its load and building said the build rather than the load"
        );
        assert_eq!(
            room_alone("skiff", Ok(None)),
            Err("skiff didn't say its load".to_owned()),
            "a Mac that said nothing was taken to have room"
        );
    }

    #[test]
    fn only_a_build_for_a_macs_system_from_elsewhere_asks_for_room() {
        let linux = "x86_64-linux";
        assert!(
            asks_for_room(Some(SYSTEM), linux, true),
            "the net's build for a Mac went unasked"
        );
        assert!(
            !asks_for_room(Some(SYSTEM), linux, false),
            "a run naming what changed for a Mac, built nowhere, waited"
        );
        assert!(
            !asks_for_room(Some(linux), linux, true),
            "a build for Linux waited on a Mac"
        );
        assert!(
            !asks_for_room(None, linux, true),
            "a build naming no system waited on a Mac"
        );
        assert!(
            !asks_for_room(Some(SYSTEM), SYSTEM, true),
            "a Mac's own build waited on itself"
        );
    }

    #[test]
    fn a_module_a_mac_imports_and_the_flake_are_taken_as_read() {
        for path in [
            "flake.nix",
            "flake.lock",
            "modules/zsh/home.nix",
            "modules/zsh/darwin.nix",
            "modules/nix/system.nix",
            "modules/zsh/checks.nix",
            "checks.nix",
            "modules/kitty/package.nix",
            "modules/pi/default.nix",
        ] {
            assert!(reads(path), "{path} isn't taken as read by a Mac's checks");
        }
    }

    #[test]
    fn linux_modules_and_prose_are_taken_as_unread() {
        for path in [
            "modules/zsh/nixos.nix",
            "README.md",
            "docs/testing.md",
            "flake.nix.orig",
        ] {
            assert!(!reads(path), "{path} is taken as unread by a Mac's checks");
        }
    }

    /// A tree holding a package's folder, with its `package.nix` and a
    /// source beside it, and a folder with no package.
    fn packaged() -> tempfile::TempDir {
        let tree = tempfile::tempdir().expect("no folder for a tree");
        let crate_ = tree.path().join("modules/tool/program");
        std::fs::create_dir_all(crate_.join("src")).expect("no package folder");
        std::fs::write(crate_.join("package.nix"), "{ }\n").expect("no package.nix");
        std::fs::create_dir_all(tree.path().join("docs")).expect("no docs folder");
        tree
    }

    #[test]
    fn a_packages_source_is_read_while_a_macs_checks_are_kept() {
        let tree = packaged();
        let touched = words("docs/testing.md modules/tool/program/src/main.rs");
        assert_eq!(
            read_among(tree.path(), &touched, || true).map(String::as_str),
            Some("modules/tool/program/src/main.rs"),
            "a package's source wasn't taken as read with a Mac's checks kept"
        );
    }

    #[test]
    fn a_packages_source_is_unread_while_no_macs_checks_are_kept() {
        let tree = packaged();
        let touched = words("modules/tool/program/src/main.rs");
        assert_eq!(
            read_among(tree.path(), &touched, || false),
            None,
            "a package's source was taken as read with no Mac's checks kept"
        );
    }

    #[test]
    fn a_file_in_no_packages_folder_is_unread_either_way() {
        let tree = packaged();
        let touched = words("docs/testing.md modules/tool/nixos.nix");
        for kept in [true, false] {
            assert_eq!(
                read_among(tree.path(), &touched, || kept),
                None,
                "a file outside any package was taken as read (kept: {kept})"
            );
        }
    }

    #[test]
    fn a_file_read_by_name_comes_first() {
        let tree = packaged();
        let touched = words("modules/tool/program/src/main.rs modules/tool/home.nix");
        assert_eq!(
            read_among(tree.path(), &touched, || true).map(String::as_str),
            Some("modules/tool/home.nix"),
            "a file read by name wasn't the one said"
        );
    }

    #[test]
    fn only_a_run_that_builds_for_another_system_is_joined() {
        for builds in [
            "--build --no-vm --system x86_64-linux",
            "--system x86_64-linux",
        ] {
            assert!(joins(&words(builds)), "a run of {builds} wasn't joined");
        }
        for apart in [
            "--name-only --system x86_64-linux",
            "--keep-base --system x86_64-linux",
            "--build --system aarch64-darwin",
        ] {
            assert!(!joins(&words(apart)), "a run of {apart} was joined");
        }
    }

    #[test]
    fn a_bound_is_said_in_minutes_when_whole() {
        assert_eq!(
            said(LONGEST),
            "20 minutes",
            "the bound wasn't said in minutes"
        );
        assert_eq!(
            said(Duration::from_secs(3)),
            "3 seconds",
            "a test's bound wasn't said"
        );
    }

    #[test]
    fn a_macs_run_names_its_system_in_place_of_the_other() {
        assert_eq!(
            for_a_mac(&words("--build --system x86_64-linux --base origin/main")),
            words("--build --system aarch64-darwin --base origin/main"),
            "the system named wasn't replaced by a Mac's"
        );
        assert_eq!(
            for_a_mac(&words("--no-vm")),
            words("--no-vm --system aarch64-darwin"),
            "a Mac's system wasn't added where none was named"
        );
    }
}
