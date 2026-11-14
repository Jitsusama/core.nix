//! Secret asks, answered by the daemon: secret-asks' engine over the
//! runtime folder's `secret-asks`, its rows served among Passwords' own
//! and its yes and no taken only from the shell, on the daemon's own
//! standard input, so no other program of the account can say yes.
//!
//! What it may release is the `releases` its managers' file lists, each
//! read from the manager whose `references` prefix it starts with, as
//! Joel's own ask, so the manager releases only what its own setting
//! lists too. An ask for anything else, or to run a program root didn't
//! choose, every link and folder on the way to it root's or the store's,
//! is refused unseen; its PATH holds only folders root chose the same
//! way. The command runs in bubblewrap: everything read-only, its tree
//! its working folder, in a process namespace of its own that ends with
//! the daemon, and given the value as its one variable, never on a
//! command line. Only /tmp, /dev/shm, the runtime folder and pi's
//! sessions are hidden from it. The rest of home, ~/.ssh and
//! ~/.config/gh among it, abstract sockets and the network stay in its
//! reach until the change that lists the first reference hides the
//! fence's own list too.

use std::ffi::OsString;
use std::os::unix::fs::MetadataExt;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

use secret_asks::ask::Ask;
use secret_asks::engine::{self, Releaser, Secret};
use serde_json::json;

use crate::{Shared, ask_of, asking};

/// How long a release waits on its manager, which may be waiting on Joel
/// to let it in, as `let-in` does.
const RELEASE_PATIENCE: u64 = 120_000;

/// The most links a path is followed through, as the kernel's own limit
/// is 40, so a loop of links ends.
const MOST_LINKS: usize = 40;

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
    /// The folders a bare program's name is found in, in order, and the
    /// command's PATH, each used only where root alone chose where it
    /// comes to.
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
    /// profiles, an absolute path as it is, either taken only where root
    /// alone chose every step to the file it comes to.
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
            .iter()
            .filter_map(|candidate| chosen_by_root(candidate))
            .find(|program| program.is_file())
    }

    /// The command's PATH: the profiles root alone chose, each where it
    /// comes to, so none can be repointed while the command runs.
    fn path(&self) -> Vec<PathBuf> {
        self.profiles
            .iter()
            .filter_map(|folder| chosen_by_root(folder))
            .filter(|folder| folder.is_dir())
            .collect()
    }
}

/// Where a path comes to, followed one entry at a time, when only root
/// chose each step: every folder and link on the way is root's, and no
/// folder is writable by a group or anybody else unless it's sticky, as
/// /nix/store is. The store's own folders, and everything in a store
/// path, count too, since nothing changes them once they're built; in a
/// build's sandbox they show as nobody's. A step through a folder or a
/// link the account owns, as `~/.nix-profile` is, comes to nothing: an
/// agent could repoint it at a program it built, in the store too.
fn chosen_by_root(path: &Path) -> Option<PathBuf> {
    if !path.is_absolute() {
        return None;
    }
    let store = Path::new("/nix/store");
    // The steps still to take, the next one last.
    let mut steps: Vec<OsString> = Vec::new();
    let push = |steps: &mut Vec<OsString>, path: &Path| {
        for part in path.components().rev() {
            match part {
                Component::Normal(name) => steps.push(name.to_owned()),
                Component::ParentDir => steps.push(OsString::from("..")),
                Component::RootDir | Component::CurDir | Component::Prefix(_) => {}
            }
        }
    };
    push(&mut steps, path);
    let mut at = PathBuf::from("/");
    let mut links = 0;
    while let Some(step) = steps.pop() {
        if step == ".." {
            at.pop();
            continue;
        }
        let next = at.join(&step);
        let meta = std::fs::symlink_metadata(&next).ok()?;
        let fixed = next.starts_with(store) || store.starts_with(&next);
        if meta.file_type().is_symlink() {
            links += 1;
            if links > MOST_LINKS || (!fixed && meta.uid() != 0) {
                return None;
            }
            let target = std::fs::read_link(&next).ok()?;
            if target.is_absolute() {
                at = PathBuf::from("/");
            }
            push(&mut steps, &target);
            continue;
        }
        let sticky = meta.is_dir() && meta.mode() & 0o1000 != 0;
        let shared = meta.mode() & 0o022 != 0 && !sticky;
        if !fixed && (meta.uid() != 0 || shared) {
            return None;
        }
        at = next;
    }
    Some(at)
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
        let path = std::env::join_paths(self.path()).map_err(|why| why.to_string())?;
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A folder of the store's that holds bash, as the tests' PATH has.
    fn stored_folder() -> PathBuf {
        std::env::var_os("PATH")
            .into_iter()
            .flat_map(|path| std::env::split_paths(&path).collect::<Vec<_>>())
            .filter_map(|folder| std::fs::canonicalize(folder.join("bash")).ok())
            .find(|bash| bash.starts_with("/nix/store"))
            .and_then(|bash| bash.parent().map(Path::to_path_buf))
            .expect("no bash in the store on PATH")
    }

    #[test]
    fn a_folder_in_the_store_comes_to_itself_and_up_and_back_again() {
        let folder = stored_folder();
        assert_eq!(chosen_by_root(&folder), Some(folder.clone()));
        let name = folder.file_name().unwrap();
        assert_eq!(chosen_by_root(&folder.join("..").join(name)), Some(folder));
    }

    #[test]
    fn a_link_the_account_owns_comes_to_nothing_though_it_points_at_the_store() {
        let mine = std::env::temp_dir().join(format!("chosen-by-root-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&mine);
        std::fs::create_dir_all(&mine).unwrap();
        let link = mine.join("bin");
        std::os::unix::fs::symlink(stored_folder(), &link).unwrap();
        assert_eq!(chosen_by_root(&link), None);
        assert_eq!(chosen_by_root(&link.join("bash")), None);
        std::fs::remove_dir_all(&mine).unwrap();
    }

    #[test]
    fn a_relative_path_or_one_not_there_comes_to_nothing() {
        assert_eq!(chosen_by_root(Path::new("bin/bash")), None);
        assert_eq!(
            chosen_by_root(&stored_folder().join("no-such-program")),
            None
        );
    }
}
