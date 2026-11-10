//! Where each fact the to-dos wait on is read: the boot's start from
//! Linux's `/proc/stat`, a switch from the system profile's link and
//! update-to-main's follower's journal, a sign-in from the file its writer
//! touches, the devices from sysfs, and core's history from a bare clone
//! of the daemon's own.

use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, UNIX_EPOCH};

use serde::Deserialize;

use crate::bounded;
use crate::judge::{History, Switch};

/// Core's repository, which the daemon keeps a bare clone of to say
/// whether a switch reached a commit.
pub const CORE: &str = "https://github.com/Jitsusama/core.nix";

/// When this boot began, from `/proc/stat`'s `btime`.
pub fn boot(stat: &str) -> Option<i64> {
    stat.lines()
        .find_map(|line| line.strip_prefix("btime "))
        .and_then(|at| at.trim().parse().ok())
}

/// When `path` itself last changed, a link's own time rather than what it
/// points at, in seconds since 1970.
pub fn changed(path: &Path) -> Option<i64> {
    let modified = fs::symlink_metadata(path).ok()?.modified().ok()?;
    i64::try_from(modified.duration_since(UNIX_EPOCH).ok()?.as_secs()).ok()
}

/// When each service was last signed in to: each file in `folder` is
/// named for one, touched by whatever signs it in.
pub fn signins(folder: &Path) -> HashMap<String, i64> {
    let Ok(entries) = fs::read_dir(folder) else {
        return HashMap::new();
    };
    entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let name = entry.file_name().to_str()?.to_owned();
            Some((name, changed(&entry.path())?))
        })
        .collect()
}

/// The names of the devices present: each USB device's product, each
/// Thunderbolt device's name, from `sys`.
pub fn devices(sys: &Path) -> Vec<String> {
    let mut found = Vec::new();
    for (bus, name) in [
        ("bus/usb/devices", "product"),
        ("bus/thunderbolt/devices", "device_name"),
    ] {
        let Ok(entries) = fs::read_dir(sys.join(bus)) else {
            continue;
        };
        for entry in entries.filter_map(Result::ok) {
            if let Ok(said) = fs::read_to_string(entry.path().join(name)) {
                let said = said.trim();
                if !said.is_empty() && !found.iter().any(|f| f == said) {
                    found.push(said.to_owned());
                }
            }
        }
    }
    found.sort();
    found
}

/// A line update-to-main writes as it goes, saying the step it reached.
#[derive(Deserialize)]
struct Said {
    step: Option<String>,
    followed: Option<Followed>,
}

#[derive(Deserialize)]
struct Followed {
    core: Option<String>,
}

/// What one line of the follower's journal (`journalctl -o short-unix`)
/// says: a switch, with the core it built, or the machine found already
/// running that core. An ask names the core main stood at, as `asking at
/// ... core@<sha>`, and the step that ends it names the core it built,
/// which wins, as cadence reads it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Look {
    Asked(String),
    Switched(Switch),
    Unchanged(Option<String>),
}

pub fn look(line: &str) -> Option<Look> {
    let (stamp, rest) = line.split_once(' ')?;
    let at = stamp.split('.').next()?.parse::<i64>().ok()?;
    if rest.contains("asking at") {
        let (_, sha) = rest.split_once(" core@")?;
        return Some(Look::Asked(sha.trim().to_owned()));
    }
    let said: Said = serde_json::from_str(&rest[rest.find('{')?..]).ok()?;
    let core = said.followed.and_then(|f| f.core);
    match said.step.as_deref() {
        Some("done") => Some(Look::Switched(Switch { at, core })),
        Some("unchanged") => Some(Look::Unchanged(core)),
        _ => None,
    }
}

/// The follower's switches and the core the machine runs, kept as its
/// journal is read line by line.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Follower {
    pub switches: Vec<Switch>,
    pub core: Option<String>,
    asked: Option<String>,
}

impl Follower {
    /// Takes in one journal line, answering whether anything changed.
    pub fn read(&mut self, line: &str) -> bool {
        match look(line) {
            Some(Look::Asked(core)) => {
                self.asked = Some(core);
                false
            }
            Some(Look::Switched(mut switch)) => {
                if switch.core.is_none() {
                    switch.core = self.asked.take();
                }
                self.asked = None;
                self.core.clone_from(&switch.core);
                self.switches.push(switch);
                true
            }
            Some(Look::Unchanged(core)) => {
                let core = core.or_else(|| self.asked.take());
                self.asked = None;
                let moved = core.is_some() && core != self.core;
                if moved {
                    self.core = core;
                }
                moved
            }
            None => false,
        }
    }
}

/// How long git may take to read the clone, and to clone or fetch it.
const READING: Duration = Duration::from_secs(10);
const FETCHING: Duration = Duration::from_secs(120);

/// Core's history, from a bare clone of its own.
#[derive(Clone)]
pub struct Core {
    pub folder: PathBuf,
    /// The git it's read with.
    pub git: PathBuf,
}

/// Runs git for at most `bound`, saying whether it succeeded.
fn succeeds(command: Command, bound: Duration) -> io::Result<bool> {
    let ended = bounded::run(command, bound, bounded::GRACE)?;
    Ok(ended.status.is_some_and(|s| s.success()))
}

impl Core {
    fn git(&self, args: &[&str], bound: Duration) -> io::Result<bool> {
        let mut command = Command::new(&self.git);
        command.arg("--git-dir").arg(&self.folder).args(args);
        succeeds(command, bound)
    }

    fn has(&self, commit: &str) -> bool {
        self.git(
            &["cat-file", "-e", &format!("{commit}^{{commit}}")],
            READING,
        )
        .unwrap_or(false)
    }

    /// Fetches main, cloning first where there's no clone yet, only when
    /// it lacks one of `commits`, so nothing is fetched while nothing
    /// waits on a commit.
    pub fn fetch_for(&self, commits: &[&str]) -> io::Result<()> {
        if !self.folder.exists() {
            let mut clone = Command::new(&self.git);
            clone
                .args(["clone", "--bare", "--quiet", CORE])
                .arg(&self.folder);
            if !succeeds(clone, FETCHING)? {
                // A clone stopped part way would be taken for a whole one.
                let _ = fs::remove_dir_all(&self.folder);
                return Err(io::Error::other("git clone failed"));
            }
        }
        if commits.iter().all(|c| self.has(c)) {
            return Ok(());
        }
        let fetch = ["fetch", "--quiet", "origin", "+refs/heads/*:refs/heads/*"];
        if self.git(&fetch, FETCHING)? {
            Ok(())
        } else {
            Err(io::Error::other("git fetch failed"))
        }
    }
}

impl History for Core {
    fn reaches(&self, core: &str, commit: &str) -> Option<bool> {
        if !self.has(core) || !self.has(commit) {
            return None;
        }
        self.git(&["merge-base", "--is-ancestor", commit, core], READING)
            .ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_boot_is_read_from_btime() {
        assert_eq!(
            boot("cpu  1 2 3\nbtime 1794200000\nprocesses 9\n"),
            Some(1_794_200_000)
        );
        assert_eq!(boot("cpu 1\n"), None);
    }

    #[test]
    fn a_switch_names_the_core_it_built_over_the_one_asked() {
        let mut follower = Follower::default();
        assert!(
            !follower.read("1794200000.5 update-to-main: asking at @abc core@aaa"),
            "asking is no switch"
        );
        assert!(
            follower
                .read(r#"1794200100.1 update-to-main: {"step":"done","followed":{"core":"bbb"}}"#),
            "a switch done is one"
        );
        assert_eq!(follower.core.as_deref(), Some("bbb"));
        assert_eq!(
            follower.switches,
            [Switch {
                at: 1_794_200_100,
                core: Some("bbb".into())
            }]
        );
    }

    #[test]
    fn a_switch_naming_no_core_keeps_the_one_asked() {
        let mut follower = Follower::default();
        follower.read("1 x: asking at core@aaa");
        follower.read(r#"2 x: {"step":"done"}"#);
        assert_eq!(follower.core.as_deref(), Some("aaa"));
    }

    #[test]
    fn an_unchanged_look_says_the_core_running_without_a_switch() {
        let mut follower = Follower::default();
        assert!(
            follower.read(r#"5 x: {"step":"unchanged","followed":{"core":"ccc"}}"#),
            "a switch to another core is one"
        );
        assert!(
            !follower.read(r#"6 x: {"step":"unchanged","followed":{"core":"ccc"}}"#),
            "the same core again is none"
        );
        assert_eq!(follower.switches, Vec::<Switch>::new());
        assert_eq!(follower.core.as_deref(), Some("ccc"));
    }

    #[test]
    fn other_lines_say_nothing() {
        let mut follower = Follower::default();
        assert!(
            !follower.read(r#"7 x: {"step":"building"}"#),
            "building is no switch"
        );
        assert!(!follower.read("not a line"), "a line it can't read is none");
        assert!(
            !follower.read(r#"8 x: {"step":"failed","why":"no room"}"#),
            "a failed switch is none"
        );
        assert_eq!(follower, Follower::default());
    }

    #[test]
    fn devices_are_each_named_once_from_sysfs() {
        let sys = tempfile::tempdir().unwrap();
        for (bus, device, file, name) in [
            ("bus/usb/devices", "1-1", "product", "USB Keyboard\n"),
            ("bus/usb/devices", "1-2", "product", "USB Keyboard\n"),
            (
                "bus/thunderbolt/devices",
                "0-1",
                "device_name",
                "Thunderbolt Dock\n",
            ),
        ] {
            let folder = sys.path().join(bus).join(device);
            fs::create_dir_all(&folder).unwrap();
            fs::write(folder.join(file), name).unwrap();
        }
        fs::create_dir_all(sys.path().join("bus/usb/devices/usb1")).unwrap();
        assert_eq!(devices(sys.path()), ["Thunderbolt Dock", "USB Keyboard"]);
    }

    #[test]
    fn each_sign_in_is_its_files_time() {
        let folder = tempfile::tempdir().unwrap();
        fs::write(folder.path().join("google"), "").unwrap();
        let found = signins(folder.path());
        assert!(
            found.get("google").is_some_and(|at| *at > 1_700_000_000),
            "a sign-in says when it was touched"
        );
        assert!(
            signins(&folder.path().join("none")).is_empty(),
            "no folder holds no sign-ins"
        );
    }
}
