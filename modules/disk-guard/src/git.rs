//! What git stages in a repository and never moved into place: the
//! object folders a fetch receives its packs in, the packs and indexes a
//! fetch or a repack writes, and loose objects on their way. Git removes
//! them as it exits, so one killed on the way, as a fetch is when the
//! network goes or the service that ran it stops, leaves them for good,
//! and nothing in git looks for them again. One large repository held
//! 70 such folders, 9.8 GiB, the oldest two weeks old.
//!
//! They go as soon as no git works on the repository, however young,
//! since one still there with no git at work is one a killed git left.

use crate::activity::{Process, names};
use crate::plan::{Candidate, Class};
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

/// Where in a git directory git stages what isn't in place yet, and the
/// start of the names it gives what it stages there.
const STAGED: [(&str, &str); 3] = [
    ("objects", "tmp_objdir-"),
    ("objects", "incoming-"),
    ("objects/pack", "tmp_"),
];

/// The start of a loose object's name while git writes it, in its
/// fan-out folder.
const LOOSE: &str = "tmp_obj_";

/// Everything git staged in the repository whose git directory is
/// `git_dir` and never moved into place.
pub fn temporaries(git_dir: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    for (folder, start) in STAGED {
        found.extend(named(&git_dir.join(folder), start));
    }
    for fan_out in named(&git_dir.join("objects"), "") {
        let two_hex = fan_out
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.len() == 2 && name.bytes().all(|b| b.is_ascii_hexdigit()));
        if two_hex && fan_out.is_dir() {
            found.extend(named(&fan_out, LOOSE));
        }
    }
    found
}

/// The entries of `dir` whose names begin with `start`, never through a
/// link.
fn named(dir: &Path, start: &str) -> Vec<PathBuf> {
    let Ok(listing) = fs::read_dir(dir) else {
        return Vec::new();
    };
    listing
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_ok_and(|kind| !kind.is_symlink()))
        .filter(|entry| {
            entry
                .file_name()
                .to_str()
                .is_some_and(|name| name.starts_with(start))
        })
        .map(|entry| entry.path())
        .collect()
}

/// The folders git works on the repository from: its git directory, and
/// each worktree it keeps a `gitdir` file for, which names the worktree's
/// own `.git`.
pub fn places(git_dir: &Path) -> Vec<PathBuf> {
    let mut found = vec![git_dir.to_owned()];
    let Ok(listing) = fs::read_dir(git_dir.join("worktrees")) else {
        return found;
    };
    for entry in listing.filter_map(Result::ok) {
        let Ok(named) = fs::read_to_string(entry.path().join("gitdir")) else {
            continue;
        };
        if let Some(tree) = Path::new(named.trim()).parent() {
            found.push(tree.to_owned());
        }
    }
    found
}

/// Whether git may be working on the repository now: a git runs whose
/// folder, or a file it holds open or has mapped, lies in one of its
/// `places`, or one of the account's own gits can't be read, so it can't
/// be ruled out. A git at work keeps everything staged, since between one
/// file and the next, as between writing a pack and indexing it, it may
/// hold none of them open. Another account's git can't stage here.
pub fn busy(places: &[PathBuf], processes: &[Process], uid: u32) -> bool {
    let inside = |path: &PathBuf| places.iter().any(|place| path.starts_with(place));
    processes
        .iter()
        .filter(|process| is_git(process))
        .any(|process| {
            if !process.readable {
                return process.uid == uid;
            }
            process
                .cwd
                .iter()
                .chain(&process.files)
                .chain(&process.maps)
                .any(inside)
        })
}

/// Whether a process is git at work: git itself or one of the programs it
/// keeps in its `git-core` folder, as `git-remote-https`, known by its
/// executable, since other tools are named for git too, and never one of
/// git's daemons, which stage nothing and run for hours. A process that
/// can't be read is known only by the name it was called by.
fn is_git(process: &Process) -> bool {
    let program = match &process.exe {
        Some(exe) => {
            plain(exe.file_name()) == "git"
                || exe
                    .parent()
                    .is_some_and(|folder| plain(folder.file_name()) == "git-core")
        }
        None => names(process).any(|name| name == "git" || name.starts_with("git-")),
    };
    program && !subcommand(process).is_some_and(|word| word.ends_with("--daemon"))
}

/// A file name as a person would say it, through Nix's `.name-wrapped`.
fn plain(name: Option<&OsStr>) -> String {
    let name = name.map(OsStr::to_string_lossy).unwrap_or_default();
    let name = name.strip_prefix('.').unwrap_or(&name);
    name.strip_suffix("-wrapped").unwrap_or(name).to_owned()
}

/// The git command a process runs: its first word after git's own
/// options, two of which take the next word as their value.
fn subcommand(process: &Process) -> Option<&str> {
    let mut words = process
        .cmdline
        .iter()
        .skip(1)
        .filter_map(|word| word.to_str());
    while let Some(word) = words.next() {
        match word {
            "-c" | "-C" => {
                words.next();
            }
            option if option.starts_with('-') => {}
            command => return Some(command),
        }
    }
    None
}

/// A temporary to take whole, at once, as waste.
pub fn candidate(path: PathBuf, size: u64) -> Candidate {
    Candidate {
        path,
        class: Class::GitTemporary,
        idle: 0.0,
        size,
        occupied: false,
        spent: false,
        checkout: None,
    }
}
