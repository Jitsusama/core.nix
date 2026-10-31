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
    /// The system profile, which the machine boots and rolls back by.
    pub profile: PathBuf,
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
    /// the Nix daemon's builders, failing rather than building it here.
    pub elsewhere: bool,
    /// Pieces built elsewhere only on builders of their own, never the
    /// daemon's, with every piece built from one of them.
    pub kept: Kept,
    /// Commands that each have to succeed once the switch is done, within
    /// `health_within` seconds, or the machine goes back.
    pub health: Vec<Vec<String>>,
    pub health_within: u64,
    /// Below this charge, on battery, nothing is started.
    pub battery_floor: u8,
    /// What the machine may be busy with, such as a call, when an update
    /// mustn't start or switch.
    pub refuse_while: Vec<Busy>,
    /// Where Linux says how the machine is powered.
    pub power_supplies: PathBuf,
    /// The programs it runs, by their paths in the store.
    pub tools: Tools,
    /// The path the running system gives update-to-main at, which it starts
    /// itself again from once it has switched, so the newest one serves;
    /// none to serve on as it is.
    pub program: Option<PathBuf>,
    /// The system NixOS booted, whose kernel decides whether a new system
    /// can be switched to live.
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
    /// When set, kept pieces their builders didn't build are built here,
    /// small and only while the machine is quiet, rather than failing.
    pub here: Option<Here>,
}

/// How small kept pieces are built here when their own builders don't.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Here {
    /// How many pieces build at once.
    pub max_jobs: u32,
    /// How many cores each piece may use.
    pub cores: u32,
    /// The minute's load average at or above which they wait.
    pub below_load: f64,
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
/// named; only a token, objects and the flake's locked inputs come from
/// `user`, and none of them can change what's built.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
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
    /// macOS's pmset, which says how a Mac is powered; none on Linux.
    pub pmset: Option<PathBuf>,
}

impl Config {
    /// Reads the config, refusing a field it doesn't know.
    pub fn read(path: &Path) -> Result<Config, String> {
        let text = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))
    }

    /// The bare repository root keeps its copy of main in.
    pub fn repository_copy(&self) -> PathBuf {
        self.state.join("repository.git")
    }

    /// The file holding the commit it last switched to.
    pub fn last_file(&self) -> PathBuf {
        self.state.join("last")
    }

    /// The link that keeps the system being built from being collected.
    pub fn building_link(&self) -> PathBuf {
        self.state.join("building")
    }

    /// The folder a hosted main's files are unpacked in, one commit at a
    /// time.
    pub fn source(&self) -> PathBuf {
        self.state.join("source")
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
