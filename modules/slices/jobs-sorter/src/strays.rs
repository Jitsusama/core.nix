//! Says once of each service systemd-run made in a slice it landed in
//! because nothing named one, with what it runs, and never moves it.
//!
//! An agent's `systemd-run --user` reaches the agent's own slice through
//! the wrapper on PATH; one run by its store path lands in app.slice,
//! sharing the floor kept for Joel's apps. Who asked for such a service
//! can't be seen: systemd-run asks the user manager over its private
//! socket, which no bus monitor hears, and the unit keeps no note of its
//! caller, whose parent is always the manager. Moving it could take a
//! unit Joel started himself, so the sorter only says, and each one is
//! fixed where it was asked for.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

/// How much of a command a line holds, so one with a long argument list
/// still reads as a line.
const LONGEST: usize = 300;

/// The services said of, so each is said of once while it lasts.
#[derive(Default)]
pub struct Told {
    units: HashSet<PathBuf>,
}

impl Told {
    /// Says, on standard error and so in the journal, what `look` finds.
    pub fn say(&mut self, under: &[PathBuf]) {
        for says in self.look(under, Path::new("/proc")) {
            eprintln!("{says}");
        }
    }

    /// A line for each of systemd-run's services under `under` not said of
    /// yet, reading its first process's command from `proc`. One with no
    /// process yet waits for a later look, and one gone is forgotten, so a
    /// name used again is said of again.
    pub fn look(&mut self, under: &[PathBuf], proc: &Path) -> Vec<String> {
        let found: Vec<PathBuf> = under.iter().flat_map(|slice| made_by_run(slice)).collect();
        self.units.retain(|unit| found.contains(unit));
        let mut says = Vec::new();
        for unit in found {
            if self.units.contains(&unit) {
                continue;
            }
            let Some(pid) = first(&unit) else {
                continue;
            };
            let command = fs::read(proc.join(pid.to_string()).join("cmdline")).unwrap_or_default();
            says.push(line(&unit, &command));
            self.units.insert(unit);
        }
        says
    }
}

/// The services in a slice named as systemd-run names one it isn't told
/// a name for: `run-u<n>.service`, or `run-p<pid>-i<n>.service`.
fn made_by_run(slice: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(slice) else {
        return Vec::new();
    };
    entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("run-") && name.ends_with(".service"))
        })
        .collect()
}

/// A cgroup's first process, if it has one.
fn first(cgroup: &Path) -> Option<u32> {
    let procs = fs::read_to_string(cgroup.join("cgroup.procs")).ok()?;
    procs.lines().next()?.trim().parse().ok()
}

/// What's said of one service, its command's arguments joined by spaces
/// and cut at `LONGEST` characters.
fn line(unit: &Path, command: &[u8]) -> String {
    let name = |path: &Path| {
        path.file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned()
    };
    let slice = unit.parent().map(name).unwrap_or_default();
    let words: Vec<String> = command
        .split(|byte| *byte == 0)
        .filter(|word| !word.is_empty())
        .map(|word| String::from_utf8_lossy(word).into_owned())
        .collect();
    let mut command = words.join(" ");
    if command.chars().count() > LONGEST {
        command = command.chars().take(LONGEST).collect::<String>() + "...";
    }
    format!(
        "Stray service {} in {slice} runs {command}; nothing named its slice, so systemd-run put \
         it there.",
        name(unit)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unit(dir: &Path, name: &str, procs: &str) -> PathBuf {
        let unit = dir.join("app.slice").join(name);
        fs::create_dir_all(&unit).unwrap();
        fs::write(unit.join("cgroup.procs"), procs).unwrap();
        unit
    }

    #[test]
    fn a_service_systemd_run_named_is_said_of_once_while_it_lasts() {
        let dir = std::env::temp_dir().join(format!("strays-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let run = unit(&dir, "run-u7.service", "42\n");
        unit(&dir, "run-p9-i10.service", "");
        unit(&dir, "launcher.service", "43\n");
        unit(&dir, "run-u8.scope", "44\n");
        fs::create_dir_all(dir.join("proc/42")).unwrap();
        fs::write(dir.join("proc/42/cmdline"), b"sleep\x001009\x00").unwrap();
        let under = [dir.join("app.slice")];
        let proc = dir.join("proc");
        let mut told = Told::default();

        assert_eq!(
            told.look(&under, &proc),
            [
                "Stray service run-u7.service in app.slice runs sleep 1009; nothing named its \
              slice, so systemd-run put it there."
            ]
        );
        assert_eq!(told.look(&under, &proc), Vec::<String>::new());

        fs::write(
            dir.join("app.slice/run-p9-i10.service/cgroup.procs"),
            "45\n",
        )
        .unwrap();
        let later = told.look(&under, &proc);
        assert_eq!(later.len(), 1);
        assert!(
            later[0].starts_with("Stray service run-p9-i10.service in app.slice runs ;"),
            "one whose process has gone is still said of, with no command: {later:?}"
        );

        fs::remove_dir_all(&run).unwrap();
        assert_eq!(told.look(&under, &proc), Vec::<String>::new());
        unit(&dir, "run-u7.service", "42\n");
        assert_eq!(told.look(&under, &proc).len(), 1);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_long_command_is_cut() {
        let command = [b"x".repeat(LONGEST + 50).as_slice(), b"\x00"].concat();
        let says = line(Path::new("/u/app.slice/run-u1.service"), &command);
        assert!(
            says.contains(&format!("runs {}...;", "x".repeat(LONGEST))),
            "the command stops at {LONGEST} characters: {says}"
        );
    }
}
