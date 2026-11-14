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
//! way, so a project's own tools are asked for through its dev shell,
//! `direnv exec . <cmd>`. An ask is refused unseen too while the agent
//! fence's list of what it hides can't be read, since the command must
//! be kept from the same.
//!
//! The command runs in bubblewrap, in a process namespace of its own
//! that ends with the daemon, and given the value as its one variable,
//! never on a command line. It may write only its tree, the parts of
//! the tree's git folder git writes as it commits, and ~/.cache, as a
//! fenced agent may; everything else is read-only. /tmp, /dev/shm, the
//! runtime folder and pi's sessions are emptied, and so is all the
//! fence's list names, read at every ask and every release so the
//! harness's own update reaches it: each file, folder and link's target
//! it lists, ~/.ssh but for its settings and public keys, and ~/.npmrc
//! but for its tokens, as `fence.ts` hides them. Abstract sockets and
//! the network stay in its reach, as they do a fenced agent's.

use std::ffi::OsString;
use std::os::unix::fs::MetadataExt;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

use regex_lite::Regex;
use secret_asks::ask::Ask;
use secret_asks::engine::{self, Releaser, Secret};
use serde::Deserialize;
use serde_json::json;

use crate::{Shared, ask_of, asking};

/// How long a release waits on its manager, which may be waiting on Joel
/// to let it in, as `let-in` does.
const RELEASE_PATIENCE: u64 = 120_000;

/// The most links a path is followed through, as the kernel's own limit
/// is 40, so a loop of links ends.
const MOST_LINKS: usize = 40;

/// Why an ask to run a program through a path root didn't choose is
/// refused.
const NOT_ROOTS: &str = "not a root-owned program";

/// Why an ask naming what no profile holds is refused, saying how to ask
/// for a project's own tools: through its dev shell, which direnv, a
/// profile's program, opens.
const NOT_IN_A_PROFILE: &str =
    "not in a profile; ask for direnv exec . <cmd> to use the project's own tools";

/// Why an ask is refused while the fence's list can't be read.
const UNFENCED: &str = "no fence list to hide by";

/// The fence's list's form this reads; any other is refused as unread,
/// since what it hides may have moved.
const FENCE_VERSION: u32 = 1;

/// The parts of a git folder git writes as it commits, fetches and
/// rebases, bound back writable in a read-only one, as the fence's
/// `GIT_WRITTEN` are: never its settings or hooks, which run code.
const GIT_WRITTEN: [&str; 5] = ["objects", "refs", "logs", "lfs", "rr-cache"];

/// A setting in npm's settings that is a credential, bare or for one
/// registry, as `fence-npmrc.ts` finds it.
const NPM_CREDENTIAL: &str = r"^\s*((?:[^=;#\s]*:)?_(?:authToken|auth|password))\s*=";

/// A setting whose value is an address carrying a name and password,
/// which npm sends as the registry's credential.
const NPM_USERINFO: &str = r"^\s*([^=;#\s]+)\s*=.*://[^/@\s]+@";

/// The agent fence's list of what it hides, as the harness publishes it.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct FenceList {
    version: u32,
    /// Paths from home, a folder's ending in `/`.
    home: Vec<String>,
    /// What a file in ~/.ssh may be named to be kept.
    ssh_kept: Vec<String>,
    /// npm's settings, from home.
    npmrc: Option<String>,
}

/// The fence's list, read and its patterns made.
struct Fence {
    home: Vec<String>,
    ssh_kept: Vec<Regex>,
    npmrc: Option<String>,
}

impl Fence {
    /// The list at `path`, or none where it isn't there, isn't the form
    /// this reads or a pattern won't make.
    fn read(path: &Path) -> Option<Fence> {
        let text = std::fs::read_to_string(path).ok()?;
        let list: FenceList = serde_json::from_str(&text).ok()?;
        if list.version != FENCE_VERSION {
            return None;
        }
        let ssh_kept = list
            .ssh_kept
            .iter()
            .map(|pattern| Regex::new(pattern).ok())
            .collect::<Option<Vec<_>>>()?;
        Some(Fence {
            home: list.home,
            ssh_kept,
            npmrc: list.npmrc,
        })
    }
}

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
    /// The agent fence's list of what it hides; none hides by nothing,
    /// so every ask is refused.
    pub hidden: Option<PathBuf>,
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

    /// The program an ask's command names, resolved: a bare name the
    /// first profile holding it, where root alone chose every step to
    /// the file it comes to. An absolute path counts only where it comes
    /// to what some profile's entry of its name comes to, since a store
    /// path alone is no choice of root's: an agent can build a program,
    /// or add one to the store, itself. What it isn't, it says why.
    fn program(&self, named: &str) -> Result<PathBuf, &'static str> {
        let held = |name: &std::ffi::OsStr| {
            self.profiles
                .iter()
                .filter_map(|folder| chosen_by_root(&folder.join(name)))
                .filter(|program| program.is_file())
                .collect::<Vec<_>>()
        };
        if !named.contains('/') {
            return held(named.as_ref())
                .into_iter()
                .next()
                .ok_or(NOT_IN_A_PROFILE);
        }
        let program = chosen_by_root(Path::new(named)).ok_or(NOT_ROOTS)?;
        let name = Path::new(named).file_name().ok_or(NOT_ROOTS)?;
        if held(name).contains(&program) {
            Ok(program)
        } else {
            Err(NOT_IN_A_PROFILE)
        }
    }

    /// The fence's list as it stands now.
    fn fence(&self) -> Option<Fence> {
        Fence::read(self.hidden.as_deref()?)
    }

    /// bubblewrap's arguments for what the command may write: its tree,
    /// the parts of the tree's git folder git writes, read-only else, and
    /// ~/.cache.
    fn writable(&self, tree: &Path) -> Vec<OsString> {
        let mut args = Vec::new();
        let mut bind = |how: &str, path: &Path| {
            args.extend([how.into(), path.into(), path.into()]);
        };
        bind("--bind", tree);
        let git = tree.join(".git");
        if is_folder(&git) {
            bind("--ro-bind", &git);
            for written in GIT_WRITTEN {
                let written = git.join(written);
                if is_folder(&written) {
                    bind("--bind", &written);
                }
            }
        }
        let cache = self.home.join(".cache");
        if is_folder(&cache) {
            bind("--bind", &cache);
        }
        args
    }

    /// bubblewrap's arguments emptying all the fence's list names that is
    /// there, as `fence.ts` does: a file bound over with /dev/null, a
    /// folder with an empty one made read-only, and a link's target
    /// rather than the link, since bubblewrap won't mount on a link and
    /// the link would still lead to it.
    fn hiding(&self, fence: &Fence) -> Vec<OsString> {
        let mut args: Vec<OsString> = Vec::new();
        for listed in &fence.home {
            let (named, folder) = match listed.strip_suffix('/') {
                Some(named) => (named, true),
                None => (listed.as_str(), false),
            };
            let path = self.home.join(named);
            let Some(over) = real(&path) else { continue };
            if folder && is_folder(&over) {
                args.extend(["--tmpfs".into(), over.clone().into()]);
                args.extend(["--remount-ro".into(), over.into()]);
            } else if !folder && is_file(&over) {
                args.extend(["--ro-bind".into(), "/dev/null".into(), over.into()]);
            }
        }
        args.extend(self.ssh(fence));
        args.extend(self.npmrc(fence));
        args
    }

    /// bubblewrap's arguments for ~/.ssh: emptied, with only the files
    /// the fence keeps bound back read-only, so no private key or TPM
    /// key's handle can be read.
    fn ssh(&self, fence: &Fence) -> Vec<OsString> {
        let ssh = self.home.join(".ssh");
        if !is_folder(&ssh) {
            return Vec::new();
        }
        let mut entries: Vec<String> = std::fs::read_dir(&ssh)
            .map(|entries| {
                entries
                    .filter_map(Result::ok)
                    .map(|entry| entry.file_name().to_string_lossy().into_owned())
                    .collect()
            })
            .unwrap_or_default();
        entries.sort();
        let mut args: Vec<OsString> = vec!["--tmpfs".into(), ssh.clone().into()];
        for entry in entries {
            let path = ssh.join(&entry);
            if fence.ssh_kept.iter().any(|kept| kept.is_match(&entry)) && is_file(&path) {
                args.extend(["--ro-bind".into(), path.clone().into(), path.into()]);
            }
        }
        args.extend(["--remount-ro".into(), ssh.into()]);
        args
    }

    /// bubblewrap's arguments for npm's settings: a copy without their
    /// credentials bound over them, or /dev/null where they hold one and
    /// no copy can be written, so a token is never left readable for want
    /// of a copy.
    fn npmrc(&self, fence: &Fence) -> Vec<OsString> {
        let Some(named) = &fence.npmrc else {
            return Vec::new();
        };
        let Some(over) = real(&self.home.join(named)) else {
            return Vec::new();
        };
        if !is_file(&over) {
            return Vec::new();
        }
        let Ok(text) = std::fs::read_to_string(&over) else {
            return vec!["--ro-bind".into(), "/dev/null".into(), over.into()];
        };
        let Some(kept) = without_npm_credentials(&text) else {
            return Vec::new();
        };
        let copy = self.copy_of_npmrc(&kept);
        let source: OsString = copy.map_or_else(|| "/dev/null".into(), Into::into);
        vec!["--ro-bind".into(), source, over.into()]
    }

    /// Writes npm's settings without their credentials where only the
    /// account reads them, in the daemon's own folder in the runtime
    /// folder, whole or not at all.
    fn copy_of_npmrc(&self, kept: &str) -> Option<PathBuf> {
        use std::io::Write;
        use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
        let folder = self.runtime.join("passwords");
        std::fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(&folder)
            .ok()?;
        let copy = folder.join("npmrc");
        let writing = folder.join(format!("npmrc.{}", std::process::id()));
        let _ = std::fs::remove_file(&writing);
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&writing)
            .ok()?;
        let written = file
            .write_all(kept.as_bytes())
            .and_then(|()| file.sync_all());
        if written.is_err() || std::fs::rename(&writing, &copy).is_err() {
            let _ = std::fs::remove_file(&writing);
            return None;
        }
        Some(copy)
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

/// Where a path really is, its links followed, or none where it isn't
/// there.
fn real(path: &Path) -> Option<PathBuf> {
    std::fs::canonicalize(path).ok()
}

/// Whether a path is a folder itself, not a link to one.
fn is_folder(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_ok_and(|meta| meta.is_dir())
}

/// Whether a path is a file itself, not a link to one.
fn is_file(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_ok_and(|meta| meta.is_file())
}

/// npm's settings with each credential's line replaced by a comment
/// naming its setting, never its value, as `fence-npmrc.ts` makes them;
/// or none where they hold no credential and need no copy.
fn without_npm_credentials(text: &str) -> Option<String> {
    let credential = Regex::new(NPM_CREDENTIAL).expect("npm's credential pattern makes");
    let userinfo = Regex::new(NPM_USERINFO).expect("npm's address pattern makes");
    let setting = |line: &str| {
        credential
            .captures(line)
            .or_else(|| userinfo.captures(line))
            .and_then(|found| found.get(1))
            .map(|found| found.as_str().to_owned())
    };
    let lines: Vec<&str> = text.split('\n').collect();
    if !lines.iter().any(|line| setting(line).is_some()) {
        return None;
    }
    let kept: Vec<String> = lines
        .iter()
        .map(|line| {
            setting(line).map_or_else(
                || (*line).to_owned(),
                |setting| format!("; {setting} is held back by the agent fence"),
            )
        })
        .collect();
    Some(kept.join("\n"))
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
        if let Err(why) = self.program(named) {
            return Some(why.to_owned());
        }
        if self.fence().is_none() {
            return Some(UNFENCED.to_owned());
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
        let program = self.program(named)?;
        let fence = self.fence().ok_or(UNFENCED)?;
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
        // What it may write comes after what's emptied, so a tree in /tmp
        // is still its own, and what's hidden comes last, so no tree or
        // cache binds it back.
        command.args(self.writable(Path::new(&ask.tree)));
        command.args(self.hiding(&fence));
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
