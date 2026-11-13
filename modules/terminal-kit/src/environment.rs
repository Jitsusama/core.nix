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

/// The variables the kit reads, and no others.
const READ: [&str; 7] = [
    "NO_COLOR",
    "FORCE_COLOR",
    "TERM",
    "COLORTERM",
    "COLUMNS",
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
    use super::*;

    #[test]
    fn an_empty_variable_is_unset() {
        let environment = Environment::of([false; 3], &[("NO_COLOR", ""), ("TERM", "dumb")]);
        assert_eq!(environment.var("NO_COLOR"), None);
        assert_eq!(environment.var("TERM"), Some("dumb"));
        assert_eq!(environment.var("COLUMNS"), None);
    }
}
