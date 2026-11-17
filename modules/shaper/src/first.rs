//! The work a session has put first: named by its quest's id or its unit's
//! name in the account's runtime folder, under shaper/first, and sent as
//! what Joel uses is, ahead of offload and agents.
//!
//! On 16 November a production deploy, run from a quest's jobs, got 32 KB/s
//! of upload beside offload's copies, IAP tunnels and some sixty agent
//! sessions, every one of them bulk in the agents' lane. Freeing it took a
//! hold on the whole machine and offload frozen for ten minutes. A name in
//! the folder lifts just that work, and lapses on its own.

use std::fs;
use std::path::Path;
use std::time::{Duration, SystemTime};

use crate::classes;

/// How many depths a cgroup put first can be at: 5, a unit in jobs, to 8,
/// a unit in a quest's cell.
pub const DEPTHS: usize = 4;

/// The shallowest of them: a unit directly in an account's jobs.
pub const DEPTH: usize = 5;

/// How long a name put first holds unless it's put first again: one that
/// was forgotten mustn't hold the upload for good.
pub const FOR: Duration = Duration::from_hours(2);

/// The most names an account's shaper/first is read for at each step, so a
/// folder filled by mistake can't slow the shaper's loop.
pub const MOST: usize = 16;

/// What's put first, by depth, as each of the `first` sets holds it.
pub type Firsts = [Vec<String>; DEPTHS];

/// What was put first at the last step, so a change is said once.
#[derive(Default)]
pub struct Heed {
    held: Firsts,
}

impl Heed {
    /// What's put first now, saying so in the journal when it changes, so
    /// a name that matched nothing, or one let go, shows there.
    pub fn heed(&mut self, root: &Path, run: &Path) -> Firsts {
        let now = find(root, run, SystemTime::now());
        if now != self.held {
            self.held.clone_from(&now);
            say(&now);
        }
        now
    }
}

fn say(first: &Firsts) {
    let names: Vec<&str> = first
        .iter()
        .flatten()
        .filter_map(|path| path.rsplit('/').next())
        .collect();
    if names.is_empty() {
        eprintln!("no work is put first now");
    } else {
        eprintln!(
            "putting first, ahead of offload and agents: {}",
            names.join(", ")
        );
    }
}

/// The work in each account's jobs it has put first, by naming it in its
/// runtime folder's shaper/first under `run`: a quest's id, as
/// QEST-20261116-2LNVLS, names its cell's slice wherever it sits in jobs,
/// and any other name a unit of that name, never a slice, which would lift
/// a whole tier of the agents' work. A name not touched within `FOR` of
/// `now`, either way, is let go, so a date set ahead can't hold it. The
/// folder is the account's, so a link in it isn't followed and only `MOST`
/// names are read.
pub fn find(root: &Path, run: &Path, now: SystemTime) -> Firsts {
    let mut found = Firsts::default();
    for manager in classes::managers(root) {
        let Some(uid) = manager
            .rsplit('@')
            .next()
            .and_then(|unit| unit.strip_suffix(".service"))
        else {
            continue;
        };
        let shaper = run.join(uid).join("shaper");
        let folder = shaper.join("first");
        let plain = |path: &Path| fs::symlink_metadata(path).is_ok_and(|m| m.is_dir());
        if !plain(&shaper) || !plain(&folder) {
            continue;
        }
        let Ok(names) = fs::read_dir(&folder) else {
            continue;
        };
        let held = |touched: SystemTime| {
            let age = now
                .duration_since(touched)
                .unwrap_or_else(|ahead| ahead.duration());
            age <= FOR
        };
        let wanted: Vec<String> = names
            .flatten()
            .take(MOST)
            .filter(|name| name.metadata().and_then(|m| m.modified()).is_ok_and(held))
            .filter_map(|name| name.file_name().into_string().ok())
            .collect();
        if !wanted.is_empty() {
            seek(
                root,
                &format!("{manager}/jobs.slice"),
                DEPTH,
                &wanted,
                &mut found,
            );
        }
    }
    for set in &mut found {
        set.sort();
    }
    found
}

/// Finds what `wanted` names among the cgroups in `path`, which sit at
/// `depth`, and in theirs, down to the deepest `find` holds.
fn seek(root: &Path, path: &str, depth: usize, wanted: &[String], found: &mut Firsts) {
    let Ok(entries) = fs::read_dir(root.join(path)) else {
        return;
    };
    for entry in entries.flatten() {
        let (Ok(kind), Ok(name)) = (entry.file_type(), entry.file_name().into_string()) else {
            continue;
        };
        if !kind.is_dir() {
            continue;
        }
        let here = format!("{path}/{name}");
        if wanted.iter().any(|want| names(want, &name)) {
            found[depth - DEPTH].push(here);
        } else if depth + 1 < DEPTH + DEPTHS {
            seek(root, &here, depth + 1, wanted, found);
        }
    }
}

/// Whether the name put first, `want`, names the cgroup `dir`: a unit's
/// own name, or a quest's id naming its cell's slice, which systemd names
/// for the quest in lower case without its dashes.
fn names(want: &str, dir: &str) -> bool {
    let unit = || {
        dir == want
            && Path::new(dir)
                .extension()
                .is_some_and(|kind| kind == "scope" || kind == "service")
    };
    let cell = || {
        let quest: String = want
            .chars()
            .filter(|c| *c != '-')
            .collect::<String>()
            .to_ascii_lowercase();
        want.to_ascii_lowercase().starts_with("qest-") && dir.ends_with(&format!("-{quest}.slice"))
    };
    unit() || cell()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    const ACCOUNT: &str = "user.slice/user-1000.slice/user@1000.service";

    /// A folder for one test, gone when it ends.
    struct Tree(PathBuf);

    impl std::ops::Deref for Tree {
        type Target = Path;
        fn deref(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for Tree {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn tree(dirs: &[&str]) -> Tree {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!("shaper-first-{}-{n}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        for dir in dirs {
            fs::create_dir_all(root.join(dir)).unwrap();
        }
        Tree(root)
    }

    /// Names `names` put first in the account's runtime folder under `run`.
    fn put_first(run: &Path, names: &[&str]) {
        let folder = run.join("1000/shaper/first");
        fs::create_dir_all(&folder).unwrap();
        for name in names {
            fs::write(folder.join(name), "").unwrap();
        }
    }

    #[test]
    fn work_put_first_is_found_at_its_depth() {
        let cell = format!(
            "{ACCOUNT}/jobs.slice/jobs-work.slice/jobs-work-delta.slice/\
             jobs-work-delta-qest202611162lnvls.slice"
        );
        let other = format!(
            "{ACCOUNT}/jobs.slice/jobs-work.slice/jobs-work-delta.slice/\
             jobs-work-delta-qest20261115zejah1.slice"
        );
        let deploy = format!("{ACCOUNT}/jobs.slice/delta-deploy-1801.scope");
        let root = tree(&[
            &format!("{cell}/work-1-2.scope"),
            &other,
            &deploy,
            &format!("{ACCOUNT}/jobs.slice/delta-deploy-2045.scope"),
        ]);
        let run = tree(&[]);
        put_first(&run, &["QEST-20261116-2LNVLS", "delta-deploy-1801.scope"]);
        assert_eq!(
            find(&root, &run, SystemTime::now()),
            [vec![deploy], vec![], vec![cell], vec![]],
            "the quest's cell at 7 and the named unit at 5, nothing else"
        );
    }

    #[test]
    fn work_put_first_is_held_two_hours_either_side_of_now() {
        let units = [
            "an-hour-ago",
            "three-hours-ago",
            "in-an-hour",
            "in-three-hours",
        ];
        let scopes: Vec<String> = units
            .iter()
            .map(|u| format!("{ACCOUNT}/jobs.slice/{u}.scope"))
            .collect();
        let root = tree(&scopes.iter().map(String::as_str).collect::<Vec<_>>());
        let run = tree(&[]);
        let now = SystemTime::now();
        let hour = Duration::from_hours(1);
        let at = [now - hour, now - 3 * hour, now + hour, now + 3 * hour];
        for (unit, at) in units.iter().zip(at) {
            let name = format!("{unit}.scope");
            put_first(&run, &[&name]);
            let file = fs::File::options()
                .write(true)
                .open(run.join("1000/shaper/first").join(&name))
                .unwrap();
            file.set_modified(at).unwrap();
        }
        assert_eq!(
            find(&root, &run, now)[0],
            vec![scopes[0].clone(), scopes[2].clone()],
            "only a name touched within two hours of now, either way, holds"
        );
    }

    #[test]
    fn a_slice_put_first_lifts_nothing() {
        let root = tree(&[&format!(
            "{ACCOUNT}/jobs.slice/jobs-work.slice/jobs-work-delta.slice/work-1.scope"
        )]);
        let run = tree(&[]);
        put_first(&run, &["jobs-work.slice", "jobs-work-delta.slice"]);
        assert_eq!(
            find(&root, &run, SystemTime::now()),
            Firsts::default(),
            "a name lifted a whole tier of the agents' work"
        );
    }

    #[test]
    fn a_linked_first_folder_is_not_read() {
        let deploy = format!("{ACCOUNT}/jobs.slice/delta-deploy-1801.scope");
        let root = tree(&[&deploy]);
        let run = tree(&[]);
        let elsewhere = tree(&[]);
        put_first(&elsewhere, &["delta-deploy-1801.scope"]);
        fs::create_dir_all(run.join("1000/shaper")).unwrap();
        std::os::unix::fs::symlink(
            elsewhere.join("1000/shaper/first"),
            run.join("1000/shaper/first"),
        )
        .unwrap();
        assert_eq!(
            find(&root, &run, SystemTime::now()),
            Firsts::default(),
            "the root shaper followed a link the account made"
        );
    }
}
