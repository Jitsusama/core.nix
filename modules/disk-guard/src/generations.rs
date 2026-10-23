//! Nix profiles' generations: which may go, and deleting them, so that
//! the closures only they keep become garbage. The running and booted
//! systems, the newest few and any the boot loader can still boot stay.

use regex_lite::Regex;
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use std::time::{Duration, UNIX_EPOCH};

/// A generation's link, as Nix names it beside its profile.
static LINK: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(?P<name>.+)-(?P<number>\d+)-link$").unwrap());
/// A boot entry for a generation of the system profile: lanzaboote's
/// `nixos-generation-<n>[-specialisation-<s>]-<hash>[+tries].efi` in
/// `EFI/Linux`, or systemd-boot's `nixos-generation-<n>[...].conf` in
/// `loader/entries`.
static ENTRY: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^nixos-generation-(?P<number>\d+)[-+.]").unwrap());
/// Where each boot loader keeps its entries on the boot partition.
const ENTRY_DIRS: [&str; 2] = ["EFI/Linux", "loader/entries"];
/// How long deleting a profile's generations may take.
const EXPIRE_TIMEOUT: Duration = Duration::from_mins(5);

/// One generation of a profile.
#[derive(Clone, Debug, PartialEq)]
pub struct Generation {
    pub number: u32,
    /// What it links to.
    pub target: PathBuf,
    /// When its link was made, in wall seconds.
    pub made: f64,
}

/// A profile with numbered generations.
#[derive(Clone, Debug, PartialEq)]
pub struct Profile {
    /// The profile's own link, which `nix-env -p` names.
    pub path: PathBuf,
    pub name: String,
    /// The generation the profile points at.
    pub current: Option<u32>,
    /// Oldest first.
    pub generations: Vec<Generation>,
}

/// What the boot loader can boot of a profile.
#[derive(Clone, Debug, PartialEq)]
pub enum Boot {
    /// Nothing boots this profile.
    NotBooted,
    /// The generations with an entry on the boot partition.
    Entries(BTreeSet<u32>),
    /// The boot partition couldn't be read, so any generation may boot.
    Unreadable,
}

/// The profiles with numbered generations in `dir` and in each directory
/// of `dir/per-user`. A profile linking straight into the store has none.
pub fn profiles(dir: &Path) -> io::Result<Vec<Profile>> {
    let mut dirs = vec![dir.to_owned()];
    if let Ok(users) = fs::read_dir(dir.join("per-user")) {
        for user in users.flatten() {
            if user.file_type().is_ok_and(|kind| kind.is_dir()) {
                dirs.push(user.path());
            }
        }
    }
    let mut found = Vec::new();
    for dir in dirs {
        let mut named: BTreeMap<String, Vec<Generation>> = BTreeMap::new();
        for entry in fs::read_dir(&dir)?.flatten() {
            let file = entry.file_name();
            let Some(parts) = file.to_str().and_then(|file| LINK.captures(file)) else {
                continue;
            };
            let (Ok(number), Ok(target), Ok(meta)) = (
                parts["number"].parse(),
                fs::read_link(entry.path()),
                fs::symlink_metadata(entry.path()),
            ) else {
                continue;
            };
            let made = meta
                .modified()
                .ok()
                .and_then(|at| at.duration_since(UNIX_EPOCH).ok());
            named
                .entry(parts["name"].to_owned())
                .or_default()
                .push(Generation {
                    number,
                    target,
                    made: made.map_or(0.0, |at| at.as_secs_f64()),
                });
        }
        for (name, mut generations) in named {
            generations.sort_by_key(|generation| generation.number);
            let path = dir.join(&name);
            let current = fs::read_link(&path).ok().and_then(|link| {
                let file = link.file_name()?.to_str()?.to_owned();
                let parts = LINK.captures(&file)?;
                (&parts["name"] == name.as_str()).then(|| parts["number"].parse().ok())?
            });
            found.push(Profile {
                path,
                name,
                current,
                generations,
            });
        }
    }
    Ok(found)
}

/// The system generations with an entry on the boot partition at `boot`.
/// A partition that can't be read is an error, and then any may boot.
pub fn entries(boot: &Path) -> io::Result<BTreeSet<u32>> {
    fs::read_dir(boot)?;
    let mut numbers = BTreeSet::new();
    for sub in ENTRY_DIRS {
        let listed = match fs::read_dir(boot.join(sub)) {
            Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
            listed => listed?,
        };
        for entry in listed {
            let file = entry?.file_name();
            if let Some(parts) = file.to_str().and_then(|file| ENTRY.captures(file))
                && let Ok(number) = parts["number"].parse()
            {
                numbers.insert(number);
            }
        }
    }
    Ok(numbers)
}

/// The profile's generations that may go: those routine may take, and
/// those under `young` seconds old that only pressure takes. The current
/// generation stays, as do any linking to a system `in_use`, the newest
/// `keep` and any with a boot entry; with the boot partition unreadable,
/// all of them.
pub fn expired(
    profile: &Profile,
    in_use: &[PathBuf],
    boot: &Boot,
    keep: usize,
    young: f64,
    now: f64,
) -> (Vec<u32>, Vec<u32>) {
    let (mut routine, mut pressed) = (Vec::new(), Vec::new());
    if *boot == Boot::Unreadable {
        return (routine, pressed);
    }
    let newest: BTreeSet<u32> = profile
        .generations
        .iter()
        .rev()
        .take(keep)
        .map(|generation| generation.number)
        .collect();
    for generation in &profile.generations {
        let booted = matches!(boot, Boot::Entries(numbers) if numbers.contains(&generation.number));
        let stays = profile.current == Some(generation.number)
            || in_use.contains(&generation.target)
            || newest.contains(&generation.number)
            || booted;
        if stays {
            continue;
        }
        if now - generation.made < young {
            pressed.push(generation.number);
        } else {
            routine.push(generation.number);
        }
    }
    (routine, pressed)
}

/// Deletes the profile's generations `numbers` through `nix-env` beside
/// `nix`, which holds the profile's lock while it does.
pub fn expire(nix: &Path, profile: &Path, numbers: &[u32], idle: bool) -> Result<(), String> {
    let nix_env = nix.with_file_name("nix-env");
    let numbers: Vec<String> = numbers.iter().map(u32::to_string).collect();
    let mut argv: Vec<&OsStr> = vec![
        nix_env.as_os_str(),
        "-p".as_ref(),
        profile.as_os_str(),
        "--delete-generations".as_ref(),
    ];
    argv.extend(numbers.iter().map(OsStr::new));
    crate::garbage::ran(&argv, EXPIRE_TIMEOUT, idle).map(drop)
}
