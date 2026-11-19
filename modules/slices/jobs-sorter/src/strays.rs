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
    /// process yet, or whose process is still systemd setting it up, waits
    /// for a later look, and one gone is forgotten, so a name used again is
    /// said of again.
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
            let process = proc.join(pid.to_string());
            let command = fs::read(process.join("cmdline")).unwrap_or_default();
            if starting(&process, &command) {
                continue;
            }
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

/// Whether a service's process is still systemd setting it up, so a look
/// would name systemd rather than the service's own command. The manager
/// forks the process into the service's cgroup as a copy of itself, which
/// runs `systemd-executor`, which renames itself to the command's name in
/// brackets, `(sleep)`, before it hands over. A look in that last step once
/// said a stray ran `(sleep)`, so the program is read from the process's
/// `exe`, which no rename changes, and only systemd's two programs wait:
/// any other program named in brackets is said of. Where `exe` can't be
/// read, the command's first word is the guide.
fn starting(process: &Path, command: &[u8]) -> bool {
    let systemds = |name: &str| name == "systemd" || name == "systemd-executor";
    if let Ok(exe) = fs::read_link(process.join("exe")) {
        let exe = exe.to_string_lossy();
        let exe = exe.strip_suffix(" (deleted)").unwrap_or(&exe);
        return Path::new(exe)
            .file_name()
            .is_some_and(|name| systemds(&name.to_string_lossy()));
    }
    let program = command.split(|byte| *byte == 0).next().unwrap_or_default();
    Path::new(&*String::from_utf8_lossy(program))
        .file_name()
        .is_some_and(|name| systemds(&name.to_string_lossy()))
}

/// A cgroup's first process, if it has one.
fn first(cgroup: &Path) -> Option<u32> {
    let procs = fs::read_to_string(cgroup.join("cgroup.procs")).ok()?;
    procs.lines().next()?.trim().parse().ok()
}

/// What's said of one service, its command's arguments joined by spaces
/// and cut at `LONGEST` characters. A control character in an argument is
/// written out, `\n` for a newline, so a command can't start a journal line
/// of its own.
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
        .map(|word| {
            String::from_utf8_lossy(word)
                .chars()
                .map(|c| {
                    if c.is_control() {
                        c.escape_default().to_string()
                    } else {
                        c.to_string()
                    }
                })
                .collect()
        })
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
    fn a_service_still_starting_is_said_of_once_it_runs_its_command() {
        let dir = std::env::temp_dir().join(format!("strays-starting-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        unit(&dir, "run-u3.service", "46\n");
        fs::create_dir_all(dir.join("proc/46")).unwrap();
        fs::write(
            dir.join("proc/46/cmdline"),
            b"systemd-executor\x00--deserialize\x0075\x00",
        )
        .unwrap();
        let under = [dir.join("app.slice")];
        let proc = dir.join("proc");
        let mut told = Told::default();

        assert_eq!(
            told.look(&under, &proc),
            Vec::<String>::new(),
            "systemd's executor hasn't started the command yet, so it waits for a later look"
        );
        let exe = dir.join("proc/46/exe");
        let runs = |program: &str, command: &[u8]| {
            let _ = fs::remove_file(&exe);
            std::os::unix::fs::symlink(program, &exe).unwrap();
            fs::write(dir.join("proc/46/cmdline"), command).unwrap();
        };
        runs(
            "/nix/store/a-systemd/lib/systemd/systemd",
            b"/nix/store/a-systemd/lib/systemd/systemd\x00--user\x00",
        );
        assert_eq!(
            told.look(&under, &proc),
            Vec::<String>::new(),
            "the manager's own copy, forked into the service before it runs the executor, waits"
        );
        runs(
            "/nix/store/a-systemd/lib/systemd/systemd-executor",
            b"(sleep)\x00",
        );
        assert_eq!(
            told.look(&under, &proc),
            Vec::<String>::new(),
            "the executor renamed after the command in brackets as it sets up still waits"
        );
        runs(
            "/nix/store/a-systemd/lib/systemd/systemd-executor (deleted)",
            b"(sleep)\x00",
        );
        assert_eq!(
            told.look(&under, &proc),
            Vec::<String>::new(),
            "so does an executor whose file a later switch removed"
        );
        runs("/nix/store/a-coreutils/bin/sleep", b"sleep\x001008\x00");
        assert_eq!(
            told.look(&under, &proc),
            [
                "Stray service run-u3.service in app.slice runs sleep 1008; nothing named its \
              slice, so systemd-run put it there."
            ],
            "once the command runs, it's said of by its own words"
        );
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_command_that_names_itself_in_brackets_is_still_said_of() {
        let dir = std::env::temp_dir().join(format!("strays-brackets-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        unit(&dir, "run-u4.service", "47\n");
        fs::create_dir_all(dir.join("proc/47")).unwrap();
        std::os::unix::fs::symlink("/nix/store/a-coreutils/bin/sleep", dir.join("proc/47/exe"))
            .unwrap();
        fs::write(dir.join("proc/47/cmdline"), b"(hidden)\x001007\x00").unwrap();
        let mut told = Told::default();

        assert_eq!(
            told.look(&[dir.join("app.slice")], &dir.join("proc")),
            [
                "Stray service run-u4.service in app.slice runs (hidden) 1007; nothing named its \
              slice, so systemd-run put it there."
            ],
            "a name in brackets alone doesn't make a program systemd's executor"
        );
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_commands_control_characters_are_written_out_rather_than_breaking_the_line() {
        let says = line(
            Path::new("/u/app.slice/run-u2.service"),
            b"sh\x00-c\x00x\nStray service run-u9.service\x00",
        );
        assert!(
            says.contains("runs sh -c x\\nStray service run-u9.service;"),
            "a newline in an argument can't start a line of its own: {says}"
        );
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
