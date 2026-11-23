//! The marks of the jobs that go before the others waiting for the turn:
//! a job Joel waits on, started `turn --ahead`, and a night job paused for
//! one (src/pause.rs). Each is a file in `RUNTIME/ahead` named by its
//! process and holding its name and its start, so a later process given
//! the same number isn't taken for it. src/turn.rs reads them to leave a
//! free turn to the one that goes next; they are apart from it because
//! who goes next changes for other reasons than how the turn is held.

use crate::holding::shown;
use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Where each job waiting `--ahead` leaves a file named by its process.
fn aheads(runtime: &Path) -> PathBuf {
    runtime.join("ahead")
}

/// The fields of `/proc/PID/stat` after the program's name, which may
/// hold spaces and brackets of its own: the state first, then the parent.
pub fn stat(pid: &str) -> Option<Vec<String>> {
    let text = fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let (_, after) = text.rsplit_once(") ")?;
    Some(after.split(' ').map(str::to_owned).collect())
}

/// When a process started, in the kernel's ticks since boot, the 22nd
/// field of its stat, which tells it apart from a later one given the
/// same number.
fn started(pid: &str) -> Option<u64> {
    stat(pid)?.get(19)?.parse().ok()
}

/// A job that goes before the others waiting for the turn.
#[derive(Debug, PartialEq)]
pub struct Waiter {
    pub name: String,
    /// Its process, which tells it from a later job of the same name.
    pub pid: String,
    /// Whether it is a night job paused for a job Joel waits on, rather
    /// than one he waits on.
    pub paused: bool,
}

impl Waiter {
    /// Who it is and why it goes first, as a line says it.
    pub fn why(&self) -> String {
        if self.paused {
            format!("{}, paused for a job Joel waits on", self.name)
        } else {
            format!("{}, which Joel waits on", self.name)
        }
    }
}

/// The first job going ahead that waits for the turn, other than this
/// process: one Joel waits on before a night job paused for one. A file
/// whose process has gone, or that this process can't see, as from
/// another PID namespace, is passed over but left, since it may be
/// another's to remove.
pub fn next(runtime: &Path) -> Option<Waiter> {
    next_but(runtime, &HashSet::new())
}

/// The job that goes next, as `next` gives it, passing over the processes
/// in `passed_over`, so a night job that has paused for one of them still
/// pauses for the next one Joel waits on behind it.
pub fn next_but(runtime: &Path, passed_over: &HashSet<String>) -> Option<Waiter> {
    let own = std::process::id().to_string();
    let files = fs::read_dir(aheads(runtime)).ok()?;
    let mut paused = None;
    for file in files.flatten() {
        let pid = file.file_name().to_string_lossy().into_owned();
        // Only a process's own mark is named by its number; a file left
        // half written, or by hand, isn't a waiter.
        if pid == own || pid.parse::<u32>().is_err() || passed_over.contains(&pid) {
            continue;
        }
        let noted = fs::read_to_string(file.path()).unwrap_or_default();
        // Only its line break is trimmed, so an empty name shifts no field.
        let mut fields = noted.trim_end_matches('\n').split('\t');
        let (Some(name), Some(since)) = (fields.next(), fields.next()) else {
            continue;
        };
        if !started(&pid).is_some_and(|ticks| since.parse() == Ok(ticks)) {
            continue;
        }
        let waiter = Waiter {
            name: name.to_owned(),
            pid,
            paused: fields.next() == Some(PAUSED),
        };
        if !waiter.paused {
            return Some(waiter);
        }
        paused.get_or_insert(waiter);
    }
    paused
}

/// What a paused night job's mark says after its start.
const PAUSED: &str = "paused";

/// A mark that a job goes ahead, named by its process, there until it
/// has the turn or gives up.
pub struct Ahead(PathBuf);

impl Ahead {
    /// Marks this process, a job Joel waits on, as waiting for the turn.
    pub fn wait(runtime: &Path, name: &str) -> io::Result<Self> {
        Self::mark(runtime, name, "")
    }

    /// Marks this process, a night job paused for a job Joel waits on,
    /// as waiting to take the turn back.
    pub fn paused(runtime: &Path, name: &str) -> io::Result<Self> {
        Self::mark(runtime, name, &format!("\t{PAUSED}"))
    }

    fn mark(runtime: &Path, name: &str, after: &str) -> io::Result<Self> {
        let pid = std::process::id().to_string();
        let since = started(&pid).ok_or_else(|| io::Error::other("can't read its own start"))?;
        fs::create_dir_all(aheads(runtime))?;
        // Written whole beside it, then named, so a reader never finds
        // half of it, the name as a note writes it so it shifts no field.
        let writing = aheads(runtime).join(format!(".{pid}"));
        fs::write(&writing, format!("{}\t{since}{after}\n", shown(name)))?;
        let file = aheads(runtime).join(&pid);
        fs::rename(&writing, &file).inspect_err(|_| {
            let _ = fs::remove_file(&writing);
        })?;
        Ok(Ahead(file))
    }
}

impl Drop for Ahead {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_job_joel_waits_on_goes_before_a_paused_night_job_and_one_passed_over() {
        let runtime = std::env::temp_dir().join(format!("night-shift-next-{}", std::process::id()));
        fs::create_dir_all(aheads(&runtime)).unwrap();
        let mut marked = Vec::new();
        for (name, after) in [("repack", "\tpaused"), ("switch", ""), ("eval", "")] {
            let child = std::process::Command::new("sleep")
                .arg("30")
                .spawn()
                .unwrap();
            let pid = child.id().to_string();
            let since = started(&pid).unwrap();
            fs::write(
                aheads(&runtime).join(&pid),
                format!("{name}\t{since}{after}\n"),
            )
            .unwrap();
            marked.push((name, pid, child));
        }
        let pid_of = |wanted: &str| {
            marked
                .iter()
                .find(|(name, _, _)| *name == wanted)
                .map(|(_, pid, _)| pid.clone())
                .unwrap()
        };
        let first = next(&runtime).unwrap();
        assert!(!first.paused, "{} went first", first.name);
        let passed: HashSet<String> = [pid_of(&first.name)].into();
        let second = next_but(&runtime, &passed).unwrap();
        assert!(!second.paused, "{} went second", second.name);
        assert_ne!(second.name, first.name);
        let passed: HashSet<String> = [pid_of("switch"), pid_of("eval")].into();
        assert_eq!(next_but(&runtime, &passed).unwrap().name, "repack");
        for (_, _, mut child) in marked {
            let _ = child.kill();
            let _ = child.wait();
        }
        let _ = fs::remove_dir_all(&runtime);
    }

    // A name holding a tab, or none at all, still marks a job that goes
    // ahead: a job Joel waits on is never passed over for its name.
    #[test]
    fn a_job_joel_waits_on_goes_ahead_whatever_its_name_holds() {
        for (number, name, read) in [(0, "switch\tx", "switch x"), (1, "", "")] {
            let runtime = std::env::temp_dir().join(format!(
                "night-shift-named-ahead-{}-{number}",
                std::process::id()
            ));
            let marked = Ahead::wait(&runtime, name).unwrap();
            // The mark is this process's, which next passes over, so it's
            // read as another process's would be, under a number it isn't.
            let mut child = std::process::Command::new("sleep")
                .arg("30")
                .spawn()
                .unwrap();
            let pid = child.id().to_string();
            let text = fs::read_to_string(&marked.0).unwrap();
            let since = started(&pid).unwrap();
            let (written, _) = text.rsplit_once('\t').unwrap();
            fs::write(aheads(&runtime).join(&pid), format!("{written}\t{since}\n")).unwrap();
            let next = next(&runtime);
            let _ = child.kill();
            let _ = child.wait();
            drop(marked);
            let _ = fs::remove_dir_all(&runtime);
            assert_eq!(
                next.map(|waiter| waiter.name),
                Some(read.to_owned()),
                "a job named {name:?} was passed over"
            );
        }
    }

    #[test]
    fn a_process_reads_its_own_start_as_the_kernel_counts_it() {
        // The kernel counts 100 ticks a second, and this test's process
        // started within the hour before uptime was read.
        let ticks = started(&std::process::id().to_string()).unwrap();
        let uptime: f64 = fs::read_to_string("/proc/uptime")
            .unwrap()
            .split(' ')
            .next()
            .unwrap()
            .parse()
            .unwrap();
        let age = uptime - f64::from(u32::try_from(ticks / 100).unwrap());
        assert!(
            (0.0..3600.0).contains(&age),
            "it read {ticks} ticks, {age} s ago"
        );
    }
}
