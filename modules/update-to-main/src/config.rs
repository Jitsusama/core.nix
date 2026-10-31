//! What the machine's module tells update-to-main, as one JSON file in the
//! store: where main is, what to build from it, whose signatures count and
//! how to tell the machine is whole again. Nothing an asker sends can change
//! any of it.

use serde::Deserialize;
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
    /// The system profile, which the machine boots and rolls back by.
    pub profile: PathBuf,
    /// The signatures a commit may carry to be built.
    pub signers: Signers,
    /// When set, main's head is what the repository's host names, asked
    /// over TLS, rather than what a signature vouches for: for a host whose
    /// merges aren't signed.
    pub hosted: Option<Hosted>,
    /// Commands that each have to succeed once the switch is done, within
    /// `health_within` seconds, or the machine goes back.
    pub health: Vec<Vec<String>>,
    pub health_within: u64,
    /// Below this charge, on battery, nothing is started.
    pub battery_floor: u8,
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
}
