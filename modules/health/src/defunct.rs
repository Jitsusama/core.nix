//! The defunct children a parent leaves unreaped: a process that ended
//! but whose parent never asked how, so it stays in the process table.
//! One parent leaving hundreds fills the table and slows every new
//! process, as pi's `ps` children did on a Mac on 16 November, before
//! any journal says so.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::process::Command;

use serde::{Deserialize, Serialize};

/// The parent leaving the most defunct children at one look.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parent {
    /// Its defunct children.
    pub children: u64,
    /// Its process id.
    pub pid: u32,
    /// The last part of the path it was started by.
    pub command: String,
}

/// Look in `proc` for the parent leaving the most defunct children; none
/// when no process is defunct.
#[must_use]
pub fn look(proc: &Path) -> Option<Parent> {
    let mut children = BTreeMap::new();
    for process in fs::read_dir(proc).ok()?.flatten() {
        let Ok(stat) = fs::read_to_string(process.path().join("stat")) else {
            continue;
        };
        // The name, in brackets, may hold spaces and brackets of its own,
        // so the state and the parent are read after the last.
        let mut after = stat
            .rsplit_once(')')
            .map_or("", |(_, after)| after)
            .split_whitespace();
        let (Some("Z"), Some(parent)) = (after.next(), after.next()) else {
            continue;
        };
        if let Ok(parent) = parent.parse::<u32>() {
            *children.entry(parent).or_insert(0) += 1;
        }
    }
    most(&children, |pid| {
        command_of(&proc.join(pid.to_string())).unwrap_or_default()
    })
}

/// The last part of the path a process was started by, or its name when
/// it says no path.
fn command_of(process: &Path) -> Option<String> {
    let started = fs::read(process.join("cmdline")).ok()?;
    let first = started.split(|&byte| byte == 0).next().unwrap_or_default();
    let path = String::from_utf8_lossy(first);
    match path.rsplit('/').next() {
        Some(name) if !name.is_empty() => Some(name.to_owned()),
        _ => {
            let stat = fs::read_to_string(process.join("stat")).ok()?;
            let (_, named) = stat.split_once('(')?;
            let (name, _) = named.rsplit_once(')')?;
            Some(name.to_owned())
        }
    }
}

/// Ask `ps` for the parent leaving the most defunct children, where
/// there's no `/proc` to look in, as on a Mac.
#[must_use]
pub fn asked() -> Option<Parent> {
    Command::new("/bin/ps")
        .args(["-axo", "pid=,ppid=,stat=,comm="])
        .output()
        .ok()
        .filter(|said| said.status.success())
        .and_then(|said| from_ps(&String::from_utf8_lossy(&said.stdout)))
}

/// The parent leaving the most defunct children in what `ps -o
/// pid=,ppid=,stat=,comm=` said: a line a process, the path it was
/// started by last, which may hold spaces.
fn from_ps(said: &str) -> Option<Parent> {
    let mut commands = BTreeMap::new();
    let mut children = BTreeMap::new();
    for line in said.lines() {
        let mut words = line.split_whitespace();
        let (Some(pid), Some(parent), Some(state)) = (words.next(), words.next(), words.next())
        else {
            continue;
        };
        let (Ok(pid), Ok(parent)) = (pid.parse::<u32>(), parent.parse::<u32>()) else {
            continue;
        };
        let started_by = words.collect::<Vec<_>>().join(" ");
        let name = started_by.rsplit('/').next().unwrap_or_default();
        commands.insert(pid, name.to_owned());
        if state.starts_with('Z') {
            *children.entry(parent).or_insert(0) += 1;
        }
    }
    most(&children, |pid| {
        commands.get(&pid).cloned().unwrap_or_default()
    })
}

/// The parent of the most defunct children, among `children` counted by
/// each parent's id, named by `named`; the lower id when two leave
/// as many.
fn most(children: &BTreeMap<u32, u64>, named: impl Fn(u32) -> String) -> Option<Parent> {
    let (&pid, &children) = children
        .iter()
        .max_by_key(|&(&pid, &children)| (children, std::cmp::Reverse(pid)))?;
    Some(Parent {
        children,
        pid,
        command: named(pid),
    })
}

/// The most defunct children one parent left in the day, and who.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Defunct {
    /// Its defunct children.
    pub most: u64,
    /// The parent's process id.
    pub pid: u32,
    /// The last part of the path it was started by.
    pub command: String,
    /// When, in seconds since the epoch.
    pub at: i64,
}

impl Defunct {
    /// Keep what a look at `at`, in microseconds since the epoch, saw,
    /// when it's more than the day's most so far.
    pub fn saw(&mut self, at: i64, parent: Option<&Parent>) {
        let Some(parent) = parent.filter(|parent| parent.children > self.most) else {
            return;
        };
        *self = Self {
            most: parent.children,
            pid: parent.pid,
            command: parent.command.clone(),
            at: at / 1_000_000,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A process in a stand-in `/proc`: its id, its parent's, its state
    /// and the command it was started by.
    fn process(proc: &Path, pid: u32, parent: u32, state: char, started_by: &str) {
        let path = proc.join(pid.to_string());
        fs::create_dir_all(&path).expect("a process");
        fs::write(path.join("cmdline"), started_by).expect("its command");
        fs::write(
            path.join("stat"),
            format!("{pid} (a (name)) {state} {parent} 1 1 0 -1 0\n"),
        )
        .expect("stat");
    }

    #[test]
    fn the_parent_of_most_defunct_children_is_found_in_proc() {
        let proc = tempfile::tempdir().expect("a stand-in /proc");
        process(
            proc.path(),
            10,
            1,
            'S',
            "/nix/store/x-pi/bin/pi\0--resume\0",
        );
        process(proc.path(), 11, 1, 'S', "bash\0");
        for child in 20..23 {
            process(proc.path(), child, 10, 'Z', "");
        }
        process(proc.path(), 23, 11, 'Z', "");
        process(proc.path(), 24, 10, 'S', "ps\0");
        fs::write(proc.path().join("uptime"), "1.0 1.0\n").expect("not a process");
        assert_eq!(
            look(proc.path()),
            Some(Parent {
                children: 3,
                pid: 10,
                command: "pi".to_owned()
            }),
            "pi, by its path, with its three defunct children; the running one not counted"
        );
    }

    #[test]
    fn no_defunct_child_names_no_parent() {
        let proc = tempfile::tempdir().expect("a stand-in /proc");
        process(proc.path(), 10, 1, 'S', "pi\0");
        assert_eq!(look(proc.path()), None);
    }

    #[test]
    fn the_parent_of_most_defunct_children_is_found_in_what_ps_says() {
        let said = concat!(
            "    1     0 Ss   /sbin/launchd\n",
            "  700     1 S    /Users/someone/My Tools/pi\n",
            "  701   700 Z    (ps)\n",
            "  702   700 Z+   (ps)\n",
            "  703     1 S    /bin/zsh\n",
            "  704   703 Z    (sh)\n",
            "  705   700 R    /bin/ps\n",
            "a line ps never says\n",
        );
        assert_eq!(
            from_ps(said),
            Some(Parent {
                children: 2,
                pid: 700,
                command: "pi".to_owned()
            }),
            "pi, by the last part of a path with a space, with its two defunct children"
        );
        assert_eq!(from_ps("  1 0 Ss /sbin/launchd\n"), None, "none defunct");
    }

    #[test]
    fn two_parents_leaving_as_many_name_the_lower_id() {
        let children = BTreeMap::from([(9, 2), (4, 2), (6, 1)]);
        let named = most(&children, |pid| format!("p{pid}"));
        assert_eq!(
            named,
            Some(Parent {
                children: 2,
                pid: 4,
                command: "p4".to_owned()
            })
        );
    }

    #[test]
    fn the_days_most_is_kept_with_its_parent_and_when() {
        let parent = |children, pid: u32| Parent {
            children,
            pid,
            command: format!("p{pid}"),
        };
        let mut defunct = Defunct::default();
        defunct.saw(1_000_000, Some(&parent(5, 7)));
        defunct.saw(2_000_000, None);
        defunct.saw(3_000_000, Some(&parent(5, 8)));
        assert_eq!(
            defunct,
            Defunct {
                most: 5,
                pid: 7,
                command: "p7".to_owned(),
                at: 1
            },
            "the first to leave the most, in seconds; as many later isn't more"
        );
        defunct.saw(4_000_000, Some(&parent(469, 8)));
        assert_eq!((defunct.most, defunct.pid, defunct.at), (469, 8, 4));
    }
}
