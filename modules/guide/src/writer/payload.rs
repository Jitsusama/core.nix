//! What a stick carries for one machine, as payload.nix built it: the
//! files its vault gets, how much room they need, and what the person is
//! told to do with the stick once it's written.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use super::unread;
use crate::Fault;
use crate::install::{LAYOUT_SCRIPT, MOUNT_SCRIPT, SYSTEM_CACHE, TARGET, Target};
use crate::rescue::Keys;

/// What a stick carries for one machine, as payload.nix built it.
pub struct Payload {
    root: PathBuf,
    pub target: Target,
    /// The cache's files, relative to it.
    cached: Vec<PathBuf>,
    /// The machine's Secure Boot keys, opened from its rescue bundle, when
    /// the stick carries them, so a reinstall needs no firmware visit.
    secure_boot: Option<Keys>,
}

/// Where a file the vault gets is read from: a file of the payload's, or
/// memory, for keys that are never written down in the plain.
pub(super) enum Source<'a> {
    File(PathBuf),
    Memory(&'a [u8]),
}

/// Every file under a directory, links followed, relative to it, in a
/// steady order.
fn files_under(root: &Path) -> Result<Vec<PathBuf>, Fault> {
    let mut found = vec![];
    let mut pending = vec![PathBuf::new()];
    while let Some(relative) = pending.pop() {
        let here = root.join(&relative);
        for entry in std::fs::read_dir(&here).map_err(unread(&here))? {
            let entry = entry.map_err(unread(&here))?;
            let path = relative.join(entry.file_name());
            if root.join(&path).is_dir() {
                pending.push(path);
            } else {
                found.push(path);
            }
        }
    }
    found.sort();
    Ok(found)
}

impl Payload {
    pub fn open(root: &Path) -> Result<Payload, Fault> {
        let target = root.join(TARGET);
        let target: Target =
            serde_json::from_slice(&std::fs::read(&target).map_err(unread(&target))?)
                .map_err(|why| Fault::unreadable("the payload's target.json", why))?;
        Ok(Payload {
            cached: files_under(&root.join("cache"))?,
            root: root.to_path_buf(),
            target,
            secure_boot: None,
        })
    }

    /// The machine's Secure Boot keys too, opened from its rescue bundle,
    /// which has to be this machine's.
    pub fn with_secure_boot_keys(mut self, machine: &str, keys: Keys) -> Result<Payload, Fault> {
        if machine != self.target.machine {
            return Err(Fault::Blocked {
                step: "write-stick",
                why: format!(
                    "the rescue bundle is {machine}'s, and the stick is {}'s",
                    self.target.machine
                ),
            });
        }
        self.secure_boot = Some(keys);
        Ok(self)
    }

    /// Every file the vault gets, by its path there and where it's read
    /// from.
    pub(super) fn files(&self) -> Vec<(String, Source<'_>)> {
        let mut files = vec![
            (TARGET.to_string(), Source::File(self.root.join(TARGET))),
            (
                LAYOUT_SCRIPT.to_string(),
                Source::File(self.root.join("layout.sh")),
            ),
            (
                MOUNT_SCRIPT.to_string(),
                Source::File(self.root.join("mount.sh")),
            ),
        ];
        for relative in &self.cached {
            files.push((
                format!("{SYSTEM_CACHE}/{}", relative.display()),
                Source::File(self.root.join("cache").join(relative)),
            ));
        }
        if let Some(keys) = &self.secure_boot {
            for (name, bytes) in keys.files() {
                files.push((format!("sbctl/{name}"), Source::Memory(bytes)));
            }
        }
        files
    }

    /// Whether the stick carries the machine's Secure Boot keys.
    pub fn carries_keys(&self) -> bool {
        self.secure_boot.is_some()
    }

    /// What the person does with the stick once it's written, from what
    /// the machine's hardware says of its firmware: a card's words. `here`
    /// is the name of the machine writing it, when it has one: a stick
    /// written on the machine it's for is kept for when that machine
    /// needs it, so it's never told to be started from at once.
    pub fn next(&self, here: Option<&str>) -> String {
        let target = &self.target;
        let machine = &target.machine;
        let mut said = if here.is_some_and(|here| here.trim().eq_ignore_ascii_case(machine)) {
            format!(
                "Keep this stick away from {machine} until you need it. When {machine} won't start, or needs installing again, switch it off and plug the stick in."
            )
        } else {
            format!("Take the stick to {machine}, switched off, and plug it in.")
        };
        if !self.carries_keys()
            && let Some(clear) = &target.clear_secure_boot_keys
        {
            // A system already there may put its old keys back the moment
            // it starts, so it's never let start once they're cleared.
            // Writing to a String can't fail.
            _ = write!(
                said,
                " This stick carries none of its Secure Boot keys, so clear its firmware's first: {clear} As you leave the firmware's setup, hold the power button until {machine} is off, so its old system never starts and puts its keys back."
            );
        }
        match &target.start_from_stick {
            // Writing to a String can't fail.
            Some(start) => _ = write!(said, " Then start it from the stick: {start}"),
            None => said.push_str(" Then start it from the stick."),
        }
        // In the order a person meets it, since a stick that started and
        // asked nothing for minutes was taken for a fault.
        said.push_str(
            " The guide carries on there. First it copies itself into memory, which takes a few minutes and asks nothing. Then it asks you to type a word to check the keyboard, and then for the stick's passphrase, the seven words you saved on your phone.",
        );
        said
    }

    /// How much vault it needs: each file in whole clusters, FAT32's
    /// largest being 32 KiB, with what age adds to each.
    pub fn needed(&self) -> Result<u64, Fault> {
        let cluster = 32 << 10;
        let mut needed: u64 = 16 << 20;
        for (_, from) in self.files() {
            let bytes = match from {
                Source::File(path) => std::fs::metadata(&path).map_err(unread(&path))?.len(),
                Source::Memory(bytes) => bytes.len() as u64,
            };
            needed += (bytes + bytes / 1000 + 512).div_ceil(cluster) * cluster;
        }
        Ok(needed)
    }
}
