//! What the machine's module tells update-to-main, as one JSON file in the
//! store: where main is, what to build from it, whose signatures count and
//! how to tell the machine is whole again. Nothing an asker sends can change
//! any of it.

use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// The kind of system the machine runs, which decides how it's switched.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Kind {
    /// NixOS, switched by the system's own switch-to-configuration.
    Nixos,
    /// nix-darwin, switched by the system's own activate.
    Darwin,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// The repository's URL, fetched as git fetches it.
    pub repository: String,
    /// The branch the machine follows, main.
    pub branch: String,
    /// The flake attribute of the machine's system, such as
    /// `nixosConfigurations.NAME.config.system.build.toplevel`.
    pub output: String,
    pub kind: Kind,
    /// Root's folder for the repository's copy and what it last switched to.
    pub state: PathBuf,
    /// The socket an ask comes in on.
    pub socket: PathBuf,
    /// The group that may write the socket, and only it.
    pub group: String,
    /// Where the last update's ending is kept, as its line, for the group
    /// to read.
    pub ended: PathBuf,
    /// Where the running update's latest line is kept, for the group to
    /// read, and taken away once it ends: an ask over the socket shows in
    /// no process list, so this is how another program sees one running.
    pub running: PathBuf,
    /// The system profile, which the machine boots and rolls back by.
    pub profile: PathBuf,
    /// The system the machine runs now, which a switch moves only once its
    /// activation is through, after the profile: a switch that dies between
    /// the two leaves the profile naming a system nothing activated.
    /// `/run/current-system` unless given, so a binary handed a config
    /// written before it, as a switch's restart may do, still starts. No
    /// system's module writes it, since a binary from before it refuses a
    /// config naming it, and a Mac's daemon that a switch reloads runs the
    /// old binary until activation is through.
    #[serde(default = "current_system")]
    pub current: PathBuf,
    /// The signatures a commit may carry to be built.
    pub signers: Signers,
    /// When set, main's head is what the repository's host names, asked
    /// over TLS, rather than what a signature vouches for: for a host whose
    /// merges aren't signed.
    pub hosted: Option<Hosted>,
    /// The flake's inputs that follow their own repository's main, by
    /// their names in the flake, rather than staying where its lock pins
    /// them until the lock moves.
    pub follows: BTreeMap<String, Follow>,
    /// Whether the build leaves everything but what nixpkgs marks light to
    /// builders, failing rather than building it here unless `here` says.
    pub elsewhere: bool,
    /// The builders the pieces built elsewhere go to, as Nix's `builders`
    /// setting takes them; none for the Nix daemon's own.
    pub builders: Option<String>,
    /// Pieces built elsewhere only on builders of their own, never the
    /// daemon's, with every piece built from one of them.
    pub kept: Kept,
    /// When set, pieces built elsewhere that their builders didn't build
    /// are built here, small and only while the machine is quiet, rather
    /// than the update failing.
    pub here: Option<Here>,
    /// Commands that each have to succeed once the switch is done, within
    /// `health_within` seconds, or the machine goes back.
    pub health: Vec<Vec<String>>,
    pub health_within: u64,
    /// The command that has to pass, given the new system's path after its
    /// own arguments, before a system whose kernel, initrd or kernel
    /// parameters differ from the booted one's becomes what the machine
    /// boots. Until it does, the new system runs and the boot default
    /// stays as it was. None to make it the boot default at once.
    #[serde(default)]
    pub boot_check: Option<Vec<String>>,
    /// Below this charge, on battery, nothing is started.
    pub battery_floor: u8,
    /// What the machine may be busy with, such as a call, when an update
    /// mustn't start or switch.
    pub refuse_while: Vec<Busy>,
    /// The command the follower runs, why given after its own arguments,
    /// to tell its person the machine can't update (`crate::tell`); none
    /// to only log it.
    #[serde(default)]
    pub follower_tells: Option<Vec<String>>,
    /// Daemons a switch mustn't restart unless an asker has granted it,
    /// such as the one every build on the machine goes through.
    #[serde(default)]
    pub holds: Option<Holds>,
    /// Where Linux says how the machine is powered.
    pub power_supplies: PathBuf,
    /// Where Linux says how long runnable tasks waited for a CPU, which
    /// says whether `here` may build; a Mac has none, so nothing builds
    /// there.
    pub cpu_pressure: PathBuf,
    /// When set, a system built here of more pieces than a batch holds is
    /// built a batch at a time, each started only once the machine has the
    /// memory for it.
    #[serde(default)]
    pub batches: Option<Batches>,
    /// Where Linux says how much memory is available and how much swap is
    /// free; a Mac has none, so a batch there never finds room.
    #[serde(default = "meminfo")]
    pub memory: PathBuf,
    /// How many seconds a build may say nothing, its pieces' logs
    /// included, before it's stopped as one waiting on a builder that
    /// doesn't answer: four minutes unless given.
    #[serde(default = "silent_for")]
    pub silent_for: u64,
    /// The programs it runs, by their paths in the store.
    pub tools: Tools,
    /// How a command is run in an account's login session, where a Mac's
    /// keychain answers; none on Linux.
    pub session: Option<Session>,
    /// The path the running system gives update-to-main at, which it starts
    /// itself again from once it has switched, so the newest one serves;
    /// none to serve on as it is.
    pub program: Option<PathBuf>,
    /// The system NixOS booted, whose kernel says whether a switch leaves
    /// a new one for the next boot.
    pub booted: PathBuf,
}

/// Pieces of the system that may go to some builders and not others, such
/// as code that mustn't leave its owner's machines.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Kept {
    /// Patterns over a derivation's name or its `pname`, where `*` stands
    /// for any run of characters.
    pub names: Vec<String>,
    /// The builders they go to, as Nix's `builders` setting takes them;
    /// none to build none of them, so an update needing one fails.
    pub builders: Option<String>,
}

/// How small pieces are built here when their builders don't.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Here {
    /// How many pieces build at once.
    pub max_jobs: u32,
    /// How many cores each piece may use.
    pub cores: u32,
    /// The share of the last minute, in percent, that some runnable task
    /// waited for a CPU, at or above which they wait.
    pub busy_at: f64,
}

/// How a system built here is built a batch at a time.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Batches {
    /// How many pieces one build is given at most.
    pub pieces: usize,
    /// How much memory available and swap free together, in MiB, Linux has
    /// to say there is before a batch starts.
    pub room_mib: u64,
    /// How many seconds an update waits for that room before it fails,
    /// leaving the machine as it is.
    pub within: u64,
}

fn meminfo() -> PathBuf {
    PathBuf::from("/proc/meminfo")
}

fn current_system() -> PathBuf {
    PathBuf::from("/run/current-system")
}

/// Daemons whose restart waits for a hold: whoever asks puts the machine's
/// other work on hold first, then grants one switch by making `given`.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Holds {
    /// Their names: a systemd unit on NixOS, a launchd label on a Mac.
    pub units: Vec<String>,
    /// Paths in a system, such as `kernel`, whose changing waits for a
    /// grant as a held daemon's restart does.
    #[serde(default)]
    pub paths: Vec<String>,
    /// Units of an account's own systemd, by the account, that a switch
    /// mustn't restart through home-manager unless granted, such as one a
    /// person's windows run under: on 9 November a switch restarted the
    /// unit holding every session's window, and they all closed.
    #[serde(default)]
    pub account_units: BTreeMap<String, Vec<String>>,
    /// The file an asker makes to grant the next switch their restarts,
    /// which that switch takes away.
    pub given: PathBuf,
}

/// Something the machine may be busy with, which a command tells.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Busy {
    /// Succeeds while the machine is busy with it and exits 1 while it
    /// isn't; anything else counts as busy, since it couldn't tell.
    pub command: Vec<String>,
    /// What it's busy with, as the refusal says it, such as `a call is on`.
    pub why: String,
    /// Whether it refuses nothing, and only a switch the follower asked
    /// for waits on it once built: someone at the machine is no reason to
    /// refuse an ask they made themselves, nor to build nothing ahead.
    #[serde(default)]
    pub follower_waits: bool,
    /// How many seconds at most a follower's switch waits on it, after
    /// which it no longer holds the switch, though what else it waits on
    /// still does; with none, it waits the whole hour a switch waits.
    #[serde(default)]
    pub follower_waits_at_most: Option<u64>,
    /// Whether a switch a person asked for waits on it too, as a
    /// follower's does: a person's ask is no reason to restart what
    /// someone is typing into, though it is to switch once they pause.
    #[serde(default)]
    pub every_ask_waits: bool,
    /// Whether it refuses only an ask the follower alone made, before
    /// anything is fetched, and never the switch of what that ask built: a
    /// load it reads is the build's own by then, as a five-minute average
    /// stays high for minutes. A person's ask builds anyway.
    #[serde(default)]
    pub refuses_the_ask_alone: bool,
    /// How many seconds at most it refuses asks in a row, after which one
    /// goes ahead and the count starts again: a command that can never tell
    /// would otherwise refuse the very update that mends it. Read only with
    /// `refuses_the_ask_alone`, since any other would refuse the switch of
    /// the ask it let through.
    #[serde(default)]
    pub refuses_at_most: Option<u64>,
    /// What the last line it says on its error stream while busy is.
    #[serde(default)]
    pub says: Says,
}

/// What a busy command's last line on its error stream is, while it says
/// busy. What one that can't tell says is the reason either way.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Says {
    /// Something beside, left out of a refusal: a call check's caution
    /// that a peer went unheard is no reason a call is on.
    #[default]
    Beside,
    /// Why it's busy, as a Mac's readings are, so a refusal ends with it.
    Why,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Signers {
    /// An allowed-signers file of SSH keys, as git reads one, or none.
    pub ssh: Option<PathBuf>,
    /// PGP public keys, each a file, such as GitHub's for its merges.
    pub openpgp: Vec<PathBuf>,
}

/// How main is known on a host that doesn't sign it. Root asks the host
/// itself, and builds from objects git checks against the commit the host
/// named; only a token, objects, the flake's locked inputs and the commit
/// that last changed `directory` come from `user`, and none of them can
/// change what's built.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Hosted {
    /// The account whose logins reach the host and the flake's private
    /// inputs, and whose checkout holds the repository's objects.
    pub user: String,
    /// That account's checkout of the repository, whose objects root reads
    /// and none of whose configuration.
    pub checkout: PathBuf,
    /// The folder in the repository the flake is in, empty for its top.
    pub directory: String,
    /// A command run as `user` in `checkout` that prints a token the host
    /// takes as a bearer.
    pub token: Vec<String>,
    /// A command run as `user` in `checkout` that prints the newest commit
    /// on main that changed `directory`, or none. When main's head isn't
    /// in the checkout but that commit's folder is the one last built, the
    /// folder hasn't moved, and nothing is fetched or built. Its word only
    /// ever spares a fetch: what's built is still a commit the host named.
    #[serde(default)]
    pub changed: Option<Vec<String>>,
    /// The host's REST API for the repository, as
    /// `https://HOST/api/v3/repos/OWNER/REPO`, which root asks with the
    /// token for the folder's trees and blobs at main's head, so main
    /// needn't be fetched into `checkout` (src/read.rs). None, or an API
    /// that fails, and main is fetched.
    #[serde(default)]
    pub api: Option<String>,
    /// A command run as `user` in `checkout` that says whether main may be
    /// fetched into `checkout` now: a fetch that moves a large
    /// repository's main takes minutes and gigabytes, which a machine may
    /// only spare at night. When it fails, a head that isn't there yet is
    /// left for a later ask, and the machine takes `fetched`'s commit or
    /// stays where it is, its followed inputs moved on. None, and main is
    /// fetched whenever it's needed.
    #[serde(default)]
    pub may_fetch: Option<Vec<String>>,
    /// A command run as `user` in `checkout` that prints a commit of main
    /// the checkout already holds, such as a ref another fetch keeps. It's
    /// taken while main mayn't be fetched, only when the host says main
    /// holds it and it descends from what the machine last switched to, so
    /// its word can at most hold the machine behind main's head.
    #[serde(default)]
    pub fetched: Option<Vec<String>>,
    /// The folders `user`'s commands find their programs in.
    pub path: Vec<PathBuf>,
    /// The flake's inputs only `user` can fetch, fetched as them by their
    /// locks before a build.
    pub inputs: Vec<String>,
}

/// An input's own repository, whose main it follows once a key in
/// `signers` signed it and it descends from what the lock and the machine
/// last had.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Follow {
    pub repository: String,
    pub branch: String,
    pub signers: Signers,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Tools {
    pub git: PathBuf,
    pub gpg: PathBuf,
    /// What checks an SSH signature for git.
    pub ssh_keygen: PathBuf,
    pub nix: PathBuf,
    pub nix_env: PathBuf,
    /// What unpacks the folder a hosted main's flake is in.
    pub tar: PathBuf,
    /// What reads a hosted main's folder from its host's API.
    pub curl: PathBuf,
    /// macOS's pmset, which says how a Mac is powered; none on Linux.
    pub pmset: Option<PathBuf>,
    /// What reads a unit's journal, for the last lines of one a switch
    /// failed on; none on a Mac.
    #[serde(default)]
    pub journalctl: Option<PathBuf>,
}

/// How a command is run in an account's login session (src/session.rs).
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Session {
    /// macOS's launchctl, which starts a job in the session.
    pub launchctl: PathBuf,
    /// The folder each job's own folder is made in: root's, and passable by
    /// any account but readable and writable by none, so the account can
    /// reach the folder it's given. A root daemon's temporary folder on a
    /// Mac is root's alone.
    pub jobs: PathBuf,
}

fn silent_for() -> u64 {
    240
}

impl Config {
    /// Reads the config, refusing a field it doesn't know.
    pub fn read(path: &Path) -> Result<Config, String> {
        let text = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))
    }

    /// The program a config names, read from a config `read` refuses: a
    /// switch can write a newer update-to-main's config, with fields an
    /// older one doesn't know, while the older one still follows, and
    /// this is how it finds the newer one to start in its place.
    pub fn program_in(path: &Path) -> Option<PathBuf> {
        let text = fs::read_to_string(path).ok()?;
        let config: serde_json::Value = serde_json::from_str(&text).ok()?;
        config.get("program")?.as_str().map(PathBuf::from)
    }

    /// The bare repository root keeps its copy of main in.
    pub fn repository_copy(&self) -> PathBuf {
        self.state.join("repository.git")
    }

    /// The file holding the commit it last switched to.
    pub fn last_file(&self) -> PathBuf {
        self.state.join("last")
    }

    /// The file made when an update first builds a new main, and taken away
    /// once the machine runs it; its age is how long the machine has been
    /// behind.
    pub fn behind_file(&self) -> PathBuf {
        self.state.join("behind")
    }

    /// The file holding why the machine boots the system it booted before,
    /// from the switch whose boot check failed until one passes.
    pub fn boots_file(&self) -> PathBuf {
        self.state.join("boots")
    }

    /// The link that keeps the system being built from being collected.
    pub fn building_link(&self) -> PathBuf {
        self.state.join("building")
    }

    /// The folder of links that keep the derivations the system needs
    /// from being collected while it builds.
    pub fn evaluated_folder(&self) -> PathBuf {
        self.state.join("evaluated")
    }

    /// The folder a hosted main's files are unpacked in, one commit at a
    /// time.
    pub fn source(&self) -> PathBuf {
        self.state.join("source")
    }

    /// The folder keeping what each read of a hosted main's folder from its
    /// host found, by commit.
    pub fn reads(&self) -> PathBuf {
        self.state.join("read")
    }

    /// The keyring holding only the PGP keys that count.
    pub fn keyring(&self) -> PathBuf {
        self.state.join("gnupg")
    }

    /// The folder root keeps each followed input's copy and keyring in.
    pub fn follows_folder(&self) -> PathBuf {
        self.state.join("follows")
    }

    /// The file holding the commit of each input it last followed.
    pub fn followed_file(&self) -> PathBuf {
        self.state.join("followed.json")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn written(text: &str) -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().expect("a folder for the config");
        let path = dir.path().join("config.json");
        fs::write(&path, text).expect("the config written");
        (dir, path)
    }

    #[test]
    fn a_config_with_a_field_this_one_does_not_know_still_names_its_program() {
        let (_dir, path) =
            written(r#"{"program": "/run/current-system/sw/bin/update-to-main", "newer": 1}"#);
        assert!(
            Config::read(&path).is_err(),
            "read refuses this config, as an older update-to-main refuses a newer's"
        );
        assert_eq!(
            Config::program_in(&path),
            Some(PathBuf::from("/run/current-system/sw/bin/update-to-main")),
            "the program is read from it all the same"
        );
    }

    #[test]
    fn a_config_that_names_no_program_or_is_not_json_names_none() {
        let (_dir, path) = written(r#"{"newer": 1}"#);
        assert_eq!(Config::program_in(&path), None, "no program field");
        let (_dir, path) = written("half a config {");
        assert_eq!(Config::program_in(&path), None, "not JSON");
        let (_dir, path) = written(r#"{"program": null}"#);
        assert_eq!(Config::program_in(&path), None, "a program of none");
    }

    #[test]
    fn a_config_that_is_not_there_names_no_program() {
        let dir = tempfile::tempdir().expect("a folder");
        assert_eq!(Config::program_in(&dir.path().join("gone.json")), None);
    }
}
