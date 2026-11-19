//! What a command finds itself in: which of its streams reach a terminal,
//! and the variables that change how it draws. Read once as it starts,
//! so every decision after reads one snapshot, and a test hands it one
//! of its own.

use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::{self, IsTerminal};
use std::os::fd::AsFd;
use std::os::unix::fs::MetadataExt;
use std::path::Path;
use std::process::{Command, Stdio};

/// The variables the kit reads, and no others.
const READ: [&str; 8] = [
    "NO_COLOR",
    "FORCE_COLOR",
    "TERM",
    "COLORTERM",
    "COLUMNS",
    "LINES",
    "HOME",
    "XDG_CONFIG_HOME",
];

/// Where a command runs: its streams and the variables it draws by.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "each is a fact of its own about one stream, never a state of the others"
)]
pub struct Environment {
    /// Whether stdin is a terminal, so a question can be asked.
    pub stdin: bool,
    /// Whether stdout is a terminal.
    pub stdout: bool,
    /// Whether stderr is a terminal.
    pub stderr: bool,
    /// Whether stderr is the journal's stream, as a unit's is, so its
    /// failures can say they're errors.
    pub journal: bool,
    vars: HashMap<String, String>,
}

impl Environment {
    /// The one this process runs in.
    pub fn here() -> Self {
        Self {
            stdin: io::stdin().is_terminal(),
            stdout: io::stdout().is_terminal(),
            stderr: io::stderr().is_terminal(),
            journal: stderr_is_the_journal(),
            vars: READ
                .iter()
                .filter_map(|name| Some(((*name).to_owned(), env::var(name).ok()?)))
                .collect(),
        }
    }

    /// One made up, for a test: which streams are terminals, and the
    /// variables set.
    pub fn of(terminals: [bool; 3], vars: &[(&str, &str)]) -> Self {
        let [stdin, stdout, stderr] = terminals;
        Self {
            stdin,
            stdout,
            stderr,
            journal: false,
            vars: vars
                .iter()
                .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
                .collect(),
        }
    }

    /// A variable, where it's set to something: an empty one counts as
    /// unset, as no-color.org and force-color.org both read them.
    pub fn var(&self, name: &str) -> Option<&str> {
        self.vars
            .get(name)
            .map(String::as_str)
            .filter(|value| !value.is_empty())
    }
}

/// Where the kernel keeps the machine's name, then where the system
/// keeps it when the kernel's isn't there to read. macOS keeps it in
/// neither, so `hostname` answers there.
const NAMED_IN: [&str; 2] = ["/proc/sys/kernel/hostname", "/etc/hostname"];

/// The machine's name as it calls itself, trimmed: read where the kernel
/// keeps it, else asked of `hostname`, and "this machine" only when
/// neither says. Records carry it to be matched against mycelium's
/// machine names, so it is the bare hostname, cut at its first dot as
/// macOS's `hostname` can dress it with `.local` or a domain.
pub fn machine() -> String {
    machine_named(&NAMED_IN.map(Path::new), || {
        Command::new("hostname")
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .output()
            .ok()
            .map(|out| String::from_utf8_lossy(&out.stdout).into_owned())
    })
}

/// The machine's name from the first of `files` that names one, else
/// from what `asked` says.
fn machine_named(files: &[&Path], asked: impl FnOnce() -> Option<String>) -> String {
    let named = |name: String| {
        let bare = name.trim().split('.').next().unwrap_or_default();
        Some(bare.to_owned()).filter(|bare| !bare.is_empty())
    };
    files
        .iter()
        .find_map(|file| std::fs::read_to_string(file).ok().and_then(named))
        .or_else(|| asked().and_then(named))
        .unwrap_or_else(|| "this machine".to_owned())
}

/// Whether stderr is the stream systemd names in `JOURNAL_STREAM`, by its
/// device and inode, as systemd.exec(5) says to check: a child given the
/// variable but another stderr, as a pipe's, isn't writing to the journal.
fn stderr_is_the_journal() -> bool {
    let Some((device, inode)) = env::var("JOURNAL_STREAM").ok().and_then(|stream| {
        let (device, inode) = stream.split_once(':')?;
        Some((device.to_owned(), inode.to_owned()))
    }) else {
        return false;
    };
    io::stderr()
        .as_fd()
        .try_clone_to_owned()
        .map(File::from)
        .and_then(|stream| stream.metadata())
        .is_ok_and(|facts| facts.dev().to_string() == device && facts.ino().to_string() == inode)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn an_empty_variable_is_unset() {
        let environment = Environment::of([false; 3], &[("NO_COLOR", ""), ("TERM", "dumb")]);
        assert_eq!(environment.var("NO_COLOR"), None);
        assert_eq!(environment.var("TERM"), Some("dumb"));
        assert_eq!(environment.var("COLUMNS"), None);
    }

    #[test]
    fn the_room_a_terminal_was_asked_for_is_read() {
        for name in ["COLUMNS", "LINES"] {
            assert!(
                READ.contains(&name),
                "{name} isn't read, so a width or height asked for is never seen"
            );
        }
    }

    /// A folder of name files, removed when the test that made it ends,
    /// whether it passed or not.
    struct Named(PathBuf);

    impl Named {
        /// The folder for the test `case`: each (file, what it holds), made
        /// afresh under this process's own name.
        fn of(case: &str, files: &[(&str, &str)]) -> Self {
            let folder = env::temp_dir().join(format!("kit-machine-{}-{case}", std::process::id()));
            let _ = std::fs::remove_dir_all(&folder);
            std::fs::create_dir_all(&folder).unwrap();
            for (file, name) in files {
                std::fs::write(folder.join(file), name).unwrap();
            }
            Self(folder)
        }

        fn join(&self, file: &str) -> PathBuf {
            self.0.join(file)
        }
    }

    impl Drop for Named {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn the_machine_is_the_first_file_that_names_it_trimmed() {
        let folder = Named::of("first", &[("kernel", "  kernels\n"), ("system", "other\n")]);
        let files = [folder.join("kernel"), folder.join("system")];
        let name = machine_named(&files.each_ref().map(PathBuf::as_path), || {
            panic!("hostname was asked though a file named the machine")
        });
        assert_eq!(
            name, "kernels",
            "the kernel's file, trimmed, wasn't the name"
        );
    }

    #[test]
    fn a_missing_or_blank_file_is_passed_over() {
        let folder = Named::of("passed", &[("kernel", " \n"), ("system", "systems\n")]);
        let files = [
            folder.join("gone"),
            folder.join("kernel"),
            folder.join("system"),
        ];
        let name = machine_named(&files.each_ref().map(PathBuf::as_path), || None);
        assert_eq!(
            name, "systems",
            "a missing or blank file was taken as the name"
        );
    }

    #[test]
    fn hostname_is_asked_when_no_file_names_it() {
        let folder = Named::of("asked", &[]);
        let files = [folder.join("gone")];
        let name = machine_named(&files.each_ref().map(PathBuf::as_path), || {
            Some("asked\n".into())
        });
        assert_eq!(
            name, "asked",
            "what hostname said, trimmed, wasn't the name"
        );
    }

    #[test]
    fn a_dressed_name_is_cut_at_its_first_dot() {
        let name = machine_named(&[], || Some("studio.local\n".into()));
        assert_eq!(name, "studio", "a dressed name wasn't cut to the bare one");
    }

    #[test]
    fn a_dressed_name_in_a_file_is_cut_too() {
        let folder = Named::of("dressed", &[("kernel", "studio.example.org\n")]);
        let files = [folder.join("kernel")];
        let name = machine_named(&files.each_ref().map(PathBuf::as_path), || None);
        assert_eq!(
            name, "studio",
            "a file's dressed name wasn't cut to the bare one"
        );
    }

    #[test]
    fn this_machine_has_a_bare_name() {
        let name = machine();
        assert!(
            !name.is_empty() && !name.contains('.') && name.trim() == name,
            "the machine's own name read {name:?}, not a bare one"
        );
    }

    #[test]
    fn this_machine_is_said_only_when_nothing_names_it() {
        let name = machine_named(&[], || Some("  ".into()));
        assert_eq!(name, "this machine", "a blank answer was taken as the name");
    }
}
