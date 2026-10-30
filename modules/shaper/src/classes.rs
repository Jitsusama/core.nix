//! Which cgroups belong to which class, and the nftables sets that say so.
//!
//! nftables matches a socket's cgroup by its ID at a depth from the root
//! (`socket cgroupv2 level N @set`), and a cgroup's ID changes each time
//! its slice or unit is made again, so the sets are refilled from the
//! cgroup tree as it stands. nft takes a cgroup by its path and looks the ID
//! up itself, so the sets are written as paths.

use std::collections::BTreeMap;
use std::fmt::Write;
use std::fs;
use std::path::{Path, PathBuf};

/// The sets the table declares, each with the depth its rule matches at.
/// The depths count from /sys/fs/cgroup: build.slice is 1,
/// system.slice/nix-daemon.service 2, an account's jobs.slice 4
/// (user.slice/user-1000.slice/user@1000.service/jobs.slice), and a unit
/// in its app.slice 5.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Sets {
    /// The system's work: Nix's builds and machines.
    pub bulk1: Vec<String>,
    /// The Nix daemon itself, which downloads what builds need, where a
    /// system keeps it in system.slice; nixpkgs runs it in build.slice,
    /// which bulk1 already covers.
    pub bulk2: Vec<String>,
    /// Each account's work: jobs (timed work inside it), background, its
    /// containers in user.slice and its machines.
    pub bulk4: Vec<String>,
    /// The shaper's own pings, which must measure the connection, not its
    /// queue.
    pub voice2: Vec<String>,
    /// Each account's apps, whose UDP to anything but 443 is a call, as is
    /// anything they send Meet's media servers.
    pub app4: Vec<String>,
    /// Programs that are calls through and through: Tuple's daemon.
    pub video5: Vec<String>,
    /// The work Joel waits on at a terminal, which the slices keep inside
    /// jobs and ahead of the rest of it: it goes at an app's pace, not
    /// bulk's.
    pub awaited5: Vec<String>,
    /// The windows in an account's apps whose pi sessions run turns nobody
    /// typed, which `untyped` finds.
    pub bulk5: Vec<String>,
}

const SYSTEM_BULK: [&str; 2] = ["build.slice", "machine.slice"];
const ACCOUNT_BULK: [&str; 4] = [
    "jobs.slice",
    "background.slice",
    "user.slice",
    "machine.slice",
];
const CALLS: [&str; 1] = ["app.slice/tuple.service"];
const AWAITED: [&str; 1] = ["jobs.slice/jobs-awaited.slice"];

/// Reads the classes' cgroups from a cgroup tree mounted at `root`. Only
/// cgroups that exist are named, since nft refuses a path it can't find.
pub fn read(root: &Path) -> Sets {
    let exists = |path: &str| root.join(path).is_dir();
    let mut sets = Sets::default();
    for slice in SYSTEM_BULK {
        if exists(slice) {
            sets.bulk1.push(slice.to_string());
        }
    }
    for (path, set) in [
        ("system.slice/nix-daemon.service", &mut sets.bulk2),
        ("system.slice/shaper.service", &mut sets.voice2),
    ] {
        if exists(path) {
            set.push(path.to_string());
        }
    }
    for manager in managers(root) {
        for slice in ACCOUNT_BULK {
            let path = format!("{manager}/{slice}");
            if exists(&path) {
                sets.bulk4.push(path);
            }
        }
        let apps = format!("{manager}/app.slice");
        if exists(&apps) {
            sets.app4.push(apps);
        }
        for (cgroups, set) in [(&CALLS, &mut sets.video5), (&AWAITED, &mut sets.awaited5)] {
            for cgroup in cgroups {
                let path = format!("{manager}/{cgroup}");
                if exists(&path) {
                    set.push(path);
                }
            }
        }
    }
    sets
}

/// A pi session in an account's apps: its process, the unit it runs in
/// there, and the folder where the harness keeps a file named for each pi
/// running a turn a person typed, while it runs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Session {
    pid: u32,
    unit: String,
    typed: PathBuf,
}

/// Every pi running in a unit of an account's app.slice, read from `proc`,
/// with its typed folder under `run`, the accounts' runtime folders. About
/// 30 sessions, each sending its whole conversation at every turn, filled
/// the upload ahead of the turn Joel had typed and waited on. A pi
/// elsewhere is sorted by its slice already.
pub fn sessions(proc: &Path, run: &Path) -> Vec<Session> {
    let mut found = Vec::new();
    let Ok(entries) = fs::read_dir(proc) else {
        return found;
    };
    for entry in entries.flatten() {
        let Some(pid) = entry.file_name().to_str().and_then(|n| n.parse().ok()) else {
            continue;
        };
        let dir = entry.path();
        if fs::read_to_string(dir.join("comm")).is_ok_and(|comm| comm.trim() == "pi")
            && let Some((unit, uid)) = fs::read_to_string(dir.join("cgroup"))
                .ok()
                .as_deref()
                .and_then(app_unit)
        {
            found.push(Session {
                pid,
                unit,
                typed: run.join(uid).join("pi/typed"),
            });
        }
    }
    found.sort_by_key(|session| session.pid);
    found
}

/// The unit of an account's app.slice a process's cgroup file names, and
/// the account's uid, or none for any cgroup else.
fn app_unit(cgroup: &str) -> Option<(String, String)> {
    let path = cgroup.lines().find_map(|line| line.strip_prefix("0::/"))?;
    let parts: Vec<&str> = path.split('/').collect();
    let ["user.slice", account, manager, "app.slice", unit] = parts[..] else {
        return None;
    };
    let uid = account.strip_prefix("user-")?.strip_suffix(".slice")?;
    (manager == format!("user@{uid}.service") && !unit.is_empty())
        .then(|| (path.to_string(), uid.to_string()))
}

/// The units under `root` whose every pi session runs a turn nobody
/// typed. A unit where one is typed stays an app's, and one that has gone
/// is left out, since nft refuses a path it can't find. The typed files
/// are read afresh each time, so a typed turn leaves bulk at the next
/// step.
pub fn untyped(root: &Path, sessions: &[Session]) -> Vec<String> {
    let mut typed: BTreeMap<&str, bool> = BTreeMap::new();
    for session in sessions {
        let this = session.typed.join(session.pid.to_string()).exists();
        *typed.entry(&session.unit).or_default() |= this;
    }
    typed
        .into_iter()
        .filter(|&(unit, typed)| !typed && root.join(unit).is_dir())
        .map(|(unit, _)| unit.to_string())
        .collect()
}

/// Every account's user manager: user.slice/user-<uid>.slice/user@<uid>.service.
fn managers(root: &Path) -> Vec<String> {
    let mut found = Vec::new();
    let Ok(accounts) = fs::read_dir(root.join("user.slice")) else {
        return found;
    };
    for account in accounts.flatten() {
        let name = account.file_name().to_string_lossy().to_string();
        let Some(uid) = name
            .strip_prefix("user-")
            .and_then(|n| n.strip_suffix(".slice"))
        else {
            continue;
        };
        let manager = format!("user.slice/{name}/user@{uid}.service");
        if root.join(&manager).is_dir() {
            found.push(manager);
        }
    }
    found.sort();
    found
}

/// The links the shaper holds, and the addresses they reach without the
/// router, which it leaves unshaped.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Local {
    pub links: Vec<String>,
    pub v4: Vec<String>,
    pub v6: Vec<String>,
}

/// One nft transaction that empties each set and fills it again, so nft
/// applies all of it or none.
pub fn transaction(table: &str, sets: &Sets, local: &Local) -> String {
    let mut out = String::new();
    let mut fill = |name: &str, elements: &[String], quote: bool| {
        let _ = writeln!(out, "flush set inet {table} {name}");
        if !elements.is_empty() {
            let items: Vec<String> = elements
                .iter()
                .map(|e| if quote { format!("\"{e}\"") } else { e.clone() })
                .collect();
            let _ = writeln!(
                out,
                "add element inet {table} {name} {{ {} }}",
                items.join(", ")
            );
        }
    };
    fill("bulk1", &sets.bulk1, true);
    fill("bulk2", &sets.bulk2, true);
    fill("bulk4", &sets.bulk4, true);
    fill("voice2", &sets.voice2, true);
    fill("app4", &sets.app4, true);
    fill("video5", &sets.video5, true);
    fill("awaited5", &sets.awaited5, true);
    fill("bulk5", &sets.bulk5, true);
    fill("local4", &local.v4, false);
    fill("local6", &local.v6, false);
    fill("uplinks", &local.links, true);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn tree(dirs: &[&str]) -> PathBuf {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!("shaper-classes-{}-{n}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        for dir in dirs {
            fs::create_dir_all(root.join(dir)).unwrap();
        }
        root
    }

    const ACCOUNT: &str = "user.slice/user-1000.slice/user@1000.service";

    #[test]
    fn the_systems_work_is_bulk() {
        let root = tree(&[
            "build.slice",
            "machine.slice",
            "system.slice/nix-daemon.service",
        ]);
        let sets = read(&root);
        assert_eq!(sets.bulk1, ["build.slice", "machine.slice"]);
        assert_eq!(sets.bulk2, ["system.slice/nix-daemon.service"]);
    }

    #[test]
    fn an_accounts_work_is_bulk_and_its_apps_are_not() {
        let root = tree(&[
            "user.slice/user-1000.slice/user@1000.service/jobs.slice/jobs.service",
            "user.slice/user-1000.slice/user@1000.service/background.slice",
            "user.slice/user-1000.slice/user@1000.service/app.slice/tuple.service",
            "user.slice/user-1000.slice/user@1000.service/session.slice",
        ]);
        let sets = read(&root);
        assert_eq!(
            sets.bulk4,
            [
                format!("{ACCOUNT}/jobs.slice"),
                format!("{ACCOUNT}/background.slice")
            ]
        );
        assert_eq!(sets.app4, [format!("{ACCOUNT}/app.slice")]);
        assert_eq!(sets.video5, [format!("{ACCOUNT}/app.slice/tuple.service")]);
    }

    #[test]
    fn the_work_joel_waits_on_is_named_apart_from_the_rest_of_jobs() {
        let root = tree(&[
            &format!("{ACCOUNT}/jobs.slice/jobs-awaited.slice/awaited.service"),
            &format!("{ACCOUNT}/jobs.slice/jobs.service"),
        ]);
        let sets = read(&root);
        assert_eq!(
            sets.awaited5,
            [format!("{ACCOUNT}/jobs.slice/jobs-awaited.slice")]
        );
        assert_eq!(sets.bulk4, [format!("{ACCOUNT}/jobs.slice")]);
    }

    #[test]
    fn every_account_is_read() {
        let root = tree(&[
            "user.slice/user-1000.slice/user@1000.service/jobs.slice",
            "user.slice/user-1001.slice/user@1001.service/jobs.slice",
            "user.slice/user-1002.slice/session-3.scope",
        ]);
        let sets = read(&root);
        assert_eq!(sets.bulk4.len(), 2);
    }

    /// A process in a stand-in for /proc, named `comm`, in `cgroup`.
    fn process(proc: &Path, pid: u32, comm: &str, cgroup: &str) {
        let dir = proc.join(pid.to_string());
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("comm"), format!("{comm}\n")).unwrap();
        fs::write(dir.join("cgroup"), format!("0::/{cgroup}\n")).unwrap();
    }

    #[test]
    fn a_pi_whose_turn_nobody_typed_is_bulk_and_one_a_person_typed_is_not() {
        let apps = format!("{ACCOUNT}/app.slice");
        let root = tree(&[
            &format!("{apps}/kitty-1-1.scope"),
            &format!("{apps}/kitty-2-2.scope"),
            &format!("{apps}/kitty-3-3.scope"),
            &format!("{apps}/kitty-4-4.scope"),
        ]);
        let proc = root.join("proc");
        let run = root.join("run");
        process(&proc, 10, "pi", &format!("{apps}/kitty-1-1.scope"));
        process(&proc, 11, "kitty", &format!("{apps}/kitty-1-1.scope"));
        process(&proc, 20, "pi", &format!("{apps}/kitty-2-2.scope"));
        // Two in one window: one typed keeps the window best effort.
        process(&proc, 30, "pi", &format!("{apps}/kitty-3-3.scope"));
        process(&proc, 31, "pi", &format!("{apps}/kitty-3-3.scope"));
        // A pi that's gone since it was found names no cgroup.
        process(&proc, 40, "pi", &format!("{apps}/kitty-9-9.scope"));
        // Outside the apps, where its slice already sorts it.
        process(
            &proc,
            50,
            "pi",
            &format!("{ACCOUNT}/jobs.slice/jobs.service"),
        );
        fs::create_dir_all(run.join("1000/pi/typed")).unwrap();
        fs::write(run.join("1000/pi/typed/20"), "").unwrap();
        fs::write(run.join("1000/pi/typed/31"), "").unwrap();
        let found = sessions(&proc, &run);
        assert_eq!(found.len(), 5, "{found:?}");
        assert_eq!(untyped(&root, &found), [format!("{apps}/kitty-1-1.scope")]);
        // The turn ends and its file goes: it's bulk at once.
        fs::remove_file(run.join("1000/pi/typed/20")).unwrap();
        assert_eq!(untyped(&root, &found).len(), 2);
    }

    #[test]
    fn a_cgroup_that_isnt_there_is_left_out() {
        let root = tree(&["system.slice"]);
        assert_eq!(read(&root), Sets::default());
    }

    #[test]
    fn the_transaction_empties_every_set_and_fills_what_has_members() {
        let sets = Sets {
            bulk1: vec!["build.slice".into()],
            ..Sets::default()
        };
        let local = Local {
            links: vec!["enp1".into()],
            v4: vec!["192.168.4.0/22".into()],
            v6: vec![],
        };
        let text = transaction("shaper", &sets, &local);
        assert!(text.contains(
            "flush set inet shaper bulk1\nadd element inet shaper bulk1 { \"build.slice\" }\n"
        ));
        assert!(text.contains(
            "flush set inet shaper awaited5\nflush set inet shaper bulk5\n\
             flush set inet shaper local4\n"
        ));
        assert!(text.contains("add element inet shaper local4 { 192.168.4.0/22 }\n"));
        assert!(text.contains("flush set inet shaper local6\nflush set inet shaper uplinks\n"));
        assert!(text.ends_with("add element inet shaper uplinks { \"enp1\" }\n"));
    }
}
