//! The rescue bundles sticks carry from machine to machine. A host keeps
//! every bundle a stick it's about to write over carries, along with its
//! own machine's, and writes every bundle it keeps onto each stick it
//! writes, so a machine's bundle reaches every laptop and stick just by
//! their being used, with no network and nothing for the person to do. A
//! host writing a stick for a machine whose bundle it keeps signs the
//! stick with it, a YubiKey's touch opening it.
//!
//! They're ciphertext, so they can go anywhere, and a forged one does no
//! harm: an install puts keys back only when the firmware already trusts
//! them, so a stick signed from one only fails to boot.

use std::collections::BTreeMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};

use crate::Fault;
use crate::rescue::{self, Bundle};
use crate::stick::blocks::Partition;
use crate::stick::gpt::Table;
use crate::vault::store::{Fat, Store};

/// A bundle is a few KB; anything far bigger isn't one.
const MOST: u64 = 1 << 20;

/// How much of a disk holds its partition table: its first MiB, where
/// every partition the guide's images lay out starts after.
const TABLE: u64 = 1 << 20;

/// One machine's bundle, as its file holds it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Carried {
    pub machine: String,
    pub bytes: Vec<u8>,
    sealed: u64,
}

impl Carried {
    /// The bundle in `bytes`, when it's one, and for the machine `named`
    /// says, when it's read from a file named for one.
    pub fn read(bytes: Vec<u8>, named: Option<&str>) -> Option<Carried> {
        let bundle = Bundle::read(&bytes).ok()?;
        if named.is_some_and(|name| name != rescue::carried_as(bundle.machine())) {
            return None;
        }
        Some(Carried {
            machine: bundle.machine().to_string(),
            sealed: bundle.sealed(),
            bytes,
        })
    }
}

/// The bundle a stick for `machine` is signed from: the one given, else
/// the one the host keeps for that machine, else none, and a repair stick,
/// for no machine, from only the one given.
pub fn bundle_for(
    given: Option<Bundle>,
    machine: Option<&str>,
    kept: &[Carried],
) -> Result<Option<Bundle>, Fault> {
    if given.is_some() {
        return Ok(given);
    }
    machine
        .and_then(|machine| kept.iter().find(|bundle| bundle.machine == machine))
        .map(|bundle| Bundle::read(&bundle.bytes))
        .transpose()
}

/// Every bundle a store keeps beside whatever else it holds, each read
/// as a bundle for the machine its name says. One that won't read is left
/// where it is, unread.
pub fn kept(store: &mut dyn Store) -> Vec<Carried> {
    let Ok(names) = store.names() else {
        return vec![];
    };
    let mut found: Vec<Carried> = names
        .iter()
        .filter(|name| name.ends_with(&rescue::carried_as("")))
        .filter_map(|name| {
            let mut bytes = vec![];
            store
                .open(name)
                .ok()?
                .take(MOST + 1)
                .read_to_end(&mut bytes)
                .ok()?;
            if bytes.len() as u64 > MOST {
                return None;
            }
            Carried::read(bytes, Some(name))
        })
        .collect();
    found.sort_by(|one, other| one.machine.cmp(&other.machine));
    found
}

/// Every bundle the stick on `disk` carries, read from its vault's
/// partition straight off the disk, such as an image of one. A disk the
/// guide never wrote carries none.
pub fn on_stick(disk: &mut File) -> Vec<Carried> {
    let mut read = || -> Result<Vec<Carried>, Fault> {
        let unread = |why: std::io::Error| Fault::unreadable("the stick", why);
        let bytes = disk.seek(SeekFrom::End(0)).map_err(unread)?;
        let mut start = vec![0u8; TABLE.min(bytes) as usize];
        disk.seek(SeekFrom::Start(0))
            .and_then(|_| disk.read_exact(&mut start))
            .map_err(unread)?;
        let Some(vault) = Table::read(&start)?
            .partitions()
            .into_iter()
            .find(|partition| partition.name == "vault")
        else {
            return Ok(vec![]);
        };
        let mut region = Partition::new(disk, vault.offset(), vault.bytes()).map_err(unread)?;
        let mut fat = Fat::open(&mut region)?;
        Ok(kept(&mut fat))
    };
    read().unwrap_or_default()
}

/// Keeps each bundle in `store` unless it keeps a newer one for that
/// machine already, and says what it keeps afterwards.
pub fn keep(store: &mut dyn Store, mut found: Vec<Carried>) -> Result<Vec<Carried>, Fault> {
    let mut sealed: BTreeMap<String, u64> = kept(store)
        .into_iter()
        .map(|kept| (kept.machine, kept.sealed))
        .collect();
    found.sort_by_key(|bundle| bundle.sealed);
    for bundle in found {
        if sealed
            .get(&bundle.machine)
            .is_none_or(|held| *held < bundle.sealed)
        {
            sealed.insert(bundle.machine.clone(), bundle.sealed);
            store
                .create(&rescue::carried_as(&bundle.machine))?
                .write_all(&bundle.bytes)
                .map_err(|why| Fault::Unkept {
                    path: rescue::carried_as(&bundle.machine),
                    why: why.to_string(),
                })?;
        }
    }
    Ok(kept(store))
}
