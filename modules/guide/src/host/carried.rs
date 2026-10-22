//! The rescue bundles sticks carry from machine to machine. A host keeps
//! every bundle a stick it's about to write over carries, along with its
//! own machine's, and writes every bundle it keeps onto each stick it
//! writes, so a machine's bundle reaches every laptop and stick just by
//! their being used, with no network and nothing for the person to do. A
//! host writing a stick for a machine whose bundle it keeps signs the
//! stick with it, a YubiKey's touch opening it.
//!
//! They're ciphertext, so they can go anywhere, and a forged one does no
//! harm to a machine: an install puts keys back only when the firmware
//! enforces Secure Boot and its db already trusts them, so a stick signed
//! from one only fails to boot. What a forged one could cost is a host's
//! copy of a real one, so a bundle can't displace the host's own
//! machine's, one sealed later than a day from now isn't kept, and a host
//! keeps at most `MACHINES` machines' bundles.

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

/// How many machines' bundles a host keeps and a stick carries, which is
/// more than one person has machines and few enough that the most they
/// could all be fits easily in a stick's vault.
pub const MACHINES: usize = 32;

/// How far past the host's clock a bundle may say it was sealed, for a
/// clock set a little wrong, before it's taken for one stamped to win.
const AHEAD: u64 = 24 * 60 * 60;

/// Whether a machine's name makes a bundle's file name every vault and
/// host keeps apart from every other: lowercase, as a hostname is, so no
/// two differ only in case, which FAT doesn't tell apart.
fn named_plainly(machine: &str) -> bool {
    (1..=63).contains(&machine.len())
        && machine
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        && !machine.starts_with('-')
}

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
        if !named_plainly(bundle.machine())
            || named.is_some_and(|name| name != rescue::carried_as(bundle.machine()))
        {
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
/// as a bundle for the machine its name says, the first `MACHINES` of
/// them by name. One that won't read is left where it is, unread.
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
    found.truncate(MACHINES);
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

/// Keeps the host's `own` machine's bundle, the one its install left on
/// its disk, in place of any other for it, and each `found` on a stick
/// unless the host keeps a newer one for that machine already, it's for
/// the host's own, it says it was sealed more than a day past `now`, or
/// the host keeps `MACHINES` others already. Says what it keeps
/// afterwards.
pub fn keep(
    store: &mut dyn Store,
    own: Option<Carried>,
    mut found: Vec<Carried>,
    now: u64,
) -> Result<Vec<Carried>, Fault> {
    let held = kept(store);
    let mut sealed: BTreeMap<String, u64> = held
        .iter()
        .map(|kept| (kept.machine.clone(), kept.sealed))
        .collect();
    if let Some(own) = own {
        if !held.contains(&own) {
            replace(store, &own)?;
        }
        found.retain(|bundle| bundle.machine != own.machine);
        sealed.insert(own.machine.clone(), own.sealed);
    }
    found.retain(|bundle| bundle.sealed <= now.saturating_add(AHEAD));
    found.sort_by_key(|bundle| bundle.sealed);
    for bundle in found {
        let newer = match sealed.get(&bundle.machine) {
            Some(held) => *held < bundle.sealed,
            None => sealed.len() < MACHINES,
        };
        if newer {
            sealed.insert(bundle.machine.clone(), bundle.sealed);
            replace(store, &bundle)?;
        }
    }
    Ok(kept(store))
}

/// The bundle written whole under a name of its own, then put in place of
/// the one kept for its machine, so a write that fails leaves that one.
fn replace(store: &mut dyn Store, bundle: &Carried) -> Result<(), Fault> {
    let name = rescue::carried_as(&bundle.machine);
    let new = format!("{name}.new");
    let mut file = store.create(&new)?;
    file.write_all(&bundle.bytes)
        .and_then(|()| file.flush())
        .map_err(|why| Fault::Unkept {
            path: name.clone(),
            why: why.to_string(),
        })?;
    drop(file);
    store.replace(&new, &name)
}
