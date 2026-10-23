//! Which cgroups belong to which class, and the nftables sets that say so.
//!
//! nftables matches a socket's cgroup by its ID at a depth from the root
//! (`socket cgroupv2 level N @set`), and a cgroup's ID changes each time
//! its slice or unit is made again, so the sets are refilled from the
//! cgroup tree as it stands. nft takes a cgroup by its path and looks the ID
//! up itself, so the sets are written as paths.

use std::fs;
use std::path::Path;

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
    /// The shaper's own pings, which must measure the line, not its queue.
    pub voice2: Vec<String>,
    /// Each account's apps, whose UDP to anything but 443 is a call.
    pub app4: Vec<String>,
    /// Programs that are calls through and through: Tuple's daemon.
    pub video5: Vec<String>,
}

const SYSTEM_BULK: [&str; 2] = ["build.slice", "machine.slice"];
const ACCOUNT_BULK: [&str; 4] = [
    "jobs.slice",
    "background.slice",
    "user.slice",
    "machine.slice",
];
const CALLS: [&str; 1] = ["app.slice/tuple.service"];

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
        for unit in CALLS {
            let path = format!("{manager}/{unit}");
            if exists(&path) {
                sets.video5.push(path);
            }
        }
    }
    sets
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
        out.push_str(&format!("flush set inet {table} {name}\n"));
        if !elements.is_empty() {
            let items: Vec<String> = elements
                .iter()
                .map(|e| if quote { format!("\"{e}\"") } else { e.clone() })
                .collect();
            out.push_str(&format!(
                "add element inet {table} {name} {{ {} }}\n",
                items.join(", ")
            ));
        }
    };
    fill("bulk1", &sets.bulk1, true);
    fill("bulk2", &sets.bulk2, true);
    fill("bulk4", &sets.bulk4, true);
    fill("voice2", &sets.voice2, true);
    fill("app4", &sets.app4, true);
    fill("video5", &sets.video5, true);
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
    fn every_account_is_read() {
        let root = tree(&[
            "user.slice/user-1000.slice/user@1000.service/jobs.slice",
            "user.slice/user-1001.slice/user@1001.service/jobs.slice",
            "user.slice/user-1002.slice/session-3.scope",
        ]);
        let sets = read(&root);
        assert_eq!(sets.bulk4.len(), 2);
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
        assert!(text.contains("flush set inet shaper video5\nflush set inet shaper local4\n"));
        assert!(text.contains("add element inet shaper local4 { 192.168.4.0/22 }\n"));
        assert!(text.contains("flush set inet shaper local6\nflush set inet shaper uplinks\n"));
        assert!(text.ends_with("add element inet shaper uplinks { \"enp1\" }\n"));
    }
}
