//! Secret asks, answered by the daemon: secret-asks' engine over the
//! runtime folder's `secret-asks`, its rows served among Passwords' own
//! and its yes and no taken only from the shell, on the daemon's own
//! standard input, so no other program of the account can say yes.
//!
//! What it may release is the `releases` its managers' file lists, each
//! read from the manager whose `references` prefix it starts with, as
//! Joel's own ask, so the manager releases only what its own setting
//! lists too. An ask for anything else, or to run a program that isn't
//! in the store or in a folder only root may write, is refused unseen.
//! The command runs in bubblewrap: everything read-only, its tree its
//! working folder, the folders where other secrets and sessions live
//! hidden, in a process namespace of its own that ends with the daemon,
//! and given the value as its one variable, never on a command line.

use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use secret_asks::ask::Ask;
use secret_asks::engine::{self, Releaser, Secret};
use serde_json::json;

use crate::{Shared, ask_of, asking};

/// How long a release waits on its manager, which may be waiting on Joel
/// to let it in, as `let-in` does.
const RELEASE_PATIENCE: u64 = 120_000;

/// What the daemon releases, and how.
pub struct Passwords {
    pub shared: Shared,
    /// The references that may be released.
    pub releases: Vec<String>,
    /// Each manager's name and the prefix its references start with.
    pub references: Vec<(String, String)>,
    pub bwrap: PathBuf,
    /// Where the session's lock is asked; none says it's never locked,
    /// as where the daemon runs with no logind, as in its tests.
    pub loginctl: Option<PathBuf>,
    /// The account's runtime folder and home, hidden from the command
    /// where they hold what it mustn't read.
    pub runtime: PathBuf,
    pub home: PathBuf,
    /// The folders a bare program's name is found in, in order.
    pub profiles: Vec<PathBuf>,
}

impl Passwords {
    /// The manager that reads a reference, if it's listed and some
    /// manager reads references of its kind.
    fn manager_of(&self, item: &str) -> Option<&str> {
        if !self.releases.iter().any(|listed| listed == item) {
            return None;
        }
        self.references
            .iter()
            .find(|(_, prefix)| item.starts_with(prefix.as_str()))
            .map(|(manager, _)| manager.as_str())
    }

    /// The program an ask's command names, resolved: a bare name in the
    /// profiles, an absolute path as it is, either taken only where the
    /// file it comes to is root's to change alone.
    fn program(&self, named: &str) -> Option<PathBuf> {
        let candidates: Vec<PathBuf> = if named.contains('/') {
            vec![PathBuf::from(named)]
        } else {
            self.profiles
                .iter()
                .map(|folder| folder.join(named))
                .collect()
        };
        candidates
            .into_iter()
            .filter(|candidate| candidate.is_absolute())
            .filter_map(|candidate| std::fs::canonicalize(candidate).ok())
            .find(|program| program.is_file() && root_owned(program))
    }
}

/// Whether only root may change a file: one in the store, which nothing
/// writes once it's built, or one where it and every folder above it are
/// root's and writable by no group and nobody else.
fn root_owned(program: &Path) -> bool {
    if program.starts_with("/nix/store") {
        return true;
    }
    program.ancestors().all(|path| {
        std::fs::metadata(path).is_ok_and(|meta| meta.uid() == 0 && meta.mode() & 0o022 == 0)
    })
}

impl Releaser for Passwords {
    fn refused(&self, ask: &Ask) -> Option<String> {
        if self.manager_of(&ask.item).is_none() {
            return Some("not listed".to_owned());
        }
        let named = ask.argv.first().map_or("", String::as_str);
        if self.program(named).is_none() {
            return Some("not a root-owned program".to_owned());
        }
        None
    }

    fn locked(&self) -> bool {
        self.loginctl.as_deref().is_some_and(engine::session_locked)
    }

    /// The value, asked of its manager as Joel's own ask, which has the
    /// manager ask him through its app where it must.
    fn read_value(&self, ask: &Ask) -> Result<Secret, String> {
        let Some(manager) = self.manager_of(&ask.item) else {
            return Err("not listed".to_owned());
        };
        let asked = asking(
            "release",
            &[("reference", json!(ask.item)), ("asked", json!(true))],
        );
        let mut answer = ask_of(&self.shared, manager, asked, RELEASE_PATIENCE)
            .map_err(|(_, why)| format!("{manager} said: {why}"))?;
        match answer.remove("value") {
            Some(serde_json::Value::String(value)) => Ok(Secret::new(value)),
            _ => Err(format!("{manager} gave no value")),
        }
    }

    /// Bubblewrap running the program, with the value as the variable the
    /// ask names and nothing else of the daemon's own.
    fn command(&self, ask: &Ask, value: &Secret) -> Result<Command, String> {
        let (named, rest) = ask.argv.split_first().ok_or("it names no program")?;
        let program = self.program(named).ok_or("not a root-owned program")?;
        let path = std::env::join_paths(&self.profiles).map_err(|why| why.to_string())?;
        let mut command = Command::new(&self.bwrap);
        command
            .env_clear()
            .env("PATH", path)
            .env("HOME", &self.home)
            .env(&ask.given, value.reveal());
        if let Some(lang) = std::env::var_os("LANG") {
            command.env("LANG", lang);
        }
        command.args(["--ro-bind", "/", "/", "--dev", "/dev", "--proc", "/proc"]);
        let hidden = [
            PathBuf::from("/tmp"),
            PathBuf::from("/dev/shm"),
            self.runtime.clone(),
            self.home.join(".pi/agent/sessions"),
        ];
        for folder in hidden.iter().filter(|folder| folder.is_dir()) {
            command.arg("--tmpfs").arg(folder);
        }
        command
            .arg("--chdir")
            .arg(&ask.tree)
            .args(["--unshare-pid", "--die-with-parent", "--new-session", "--"])
            .arg(program)
            .args(rest);
        Ok(command)
    }
}
