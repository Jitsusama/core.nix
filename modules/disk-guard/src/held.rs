//! What running programs hold in Nix's store, from what each has in hand:
//! its program, its folder, its open and mapped files, and the text of its
//! command line and environment. Linux and macOS read the hands each their
//! own way (`crate::sys::hands`); what they hold is worked out here, alike.

use std::collections::{BTreeSet, HashSet};
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::{Component, Path, PathBuf};

/// What one process has in hand.
#[derive(Debug, Default)]
pub struct Hand {
    /// Its program, its folder, and each file it has open or mapped.
    pub files: Vec<PathBuf>,
    /// Its command line and environment, as the bytes they're kept in.
    pub texts: Vec<Vec<u8>>,
}

/// A file as every hard link to it reads: its device and inode.
pub type File = (u64, u64);

/// What running programs hold in the store.
#[derive(Debug, Default)]
pub struct Held {
    /// The store paths they hold by name.
    pub paths: HashSet<String>,
    /// The files they hold known only by Nix's hard link to them under
    /// `.links`, whose store path only a walk of each can find. A Mac's
    /// kernel names a file by whichever of its links it last looked up,
    /// so a program run from a store path Nix has linked can name only
    /// `.links/<hash>`: on 12 November 29 files that ruby, git and the
    /// terminal had mapped were left only there, their paths deleted.
    pub links: BTreeSet<File>,
}

/// Where Nix keeps one hard link to each file it has deduplicated.
const LINKS: &str = ".links";

/// What `hands` hold under `store`. A store path the text of a command
/// line or an environment names is held too, which Nix doesn't count: a
/// program handed a path copies it a file at a time, and between two
/// files it has none open. On 30 October a stick writer's payload, built
/// ten minutes before, was collected under it that way, mid-write.
pub fn held(hands: impl IntoIterator<Item = Hand>, store: &Path) -> Held {
    let mut held = Held::default();
    for hand in hands {
        for file in &hand.files {
            held.note(file, store);
        }
        for text in &hand.texts {
            held.paths.extend(named_in(text, store));
        }
    }
    held
}

impl Held {
    /// Holds the store path `file` lies in, or the file itself when it
    /// lies in `.links`. One gone since holds nothing.
    fn note(&mut self, file: &Path, store: &Path) {
        let Ok(within) = file.strip_prefix(store) else {
            return;
        };
        let Some(Component::Normal(name)) = within.components().next() else {
            return;
        };
        if name == LINKS {
            if let Ok(meta) = file.symlink_metadata() {
                self.links.insert((meta.dev(), meta.ino()));
            }
        } else {
            self.paths
                .insert(store.join(name).to_string_lossy().into_owned());
        }
    }
}

/// Whether `path` holds a file among `links`, walked without following a
/// link. Only a file with more than one link can be one of them, and
/// nothing is walked when there are none.
pub fn holds_linked(path: &Path, links: &BTreeSet<File>) -> bool {
    if links.is_empty() {
        return false;
    }
    let mut walk = vec![path.to_path_buf()];
    while let Some(at) = walk.pop() {
        let Ok(meta) = at.symlink_metadata() else {
            continue;
        };
        if meta.is_dir() {
            if let Ok(entries) = fs::read_dir(&at) {
                walk.extend(entries.filter_map(Result::ok).map(|entry| entry.path()));
            }
        } else if meta.nlink() > 1 && links.contains(&(meta.dev(), meta.ino())) {
            return true;
        }
    }
    false
}

/// The store paths `text` names, each as `store`, a slash and a name in
/// the characters Nix allows one, wherever it starts: after an `=`, a `:`
/// or nothing.
fn named_in(text: &[u8], store: &Path) -> Vec<String> {
    let prefix = format!("{}/", store.display());
    let prefix = prefix.as_bytes();
    let allowed = |byte: &u8| byte.is_ascii_alphanumeric() || b"+-._?=".contains(byte);
    let mut named = Vec::new();
    let mut rest = text;
    while let Some(at) = rest
        .windows(prefix.len())
        .position(|window| window == prefix)
    {
        let after = &rest[at + prefix.len()..];
        let length = after.iter().take_while(|byte| allowed(byte)).count();
        if length > 0 {
            let name = String::from_utf8_lossy(&after[..length]);
            named.push(store.join(name.as_ref()).to_string_lossy().into_owned());
        }
        rest = &after[length..];
    }
    named
}
