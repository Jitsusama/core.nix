//! Installing a machine from its stick, booted on that machine: open the
//! vault, lay the disk out encrypted with a key held only in memory, enrol
//! each YubiKey and a recovery key, make the Secure Boot keys, install the
//! system offline from the vault, and take the key held in memory off the
//! disk, so the YubiKeys and the recovery key are the only ways in.
//!
//! What a step holds between steps it holds under /run/guide, which is
//! memory: the vault's key once it's open, the disk's first key. A stick
//! restarted part way forgets them, and the journey starts the disk again.

pub mod disk;
pub mod keys;
pub mod system;
pub mod vault;

use serde::{Deserialize, Serialize};

use crate::vault::store::Store;
use crate::vault::{Vault, VaultKey};
use crate::{Fault, Machine, Secret, Step};

/// Where the stick holds what it must not write down.
pub const HELD: &str = "/run/guide";
pub const VAULT_KEY: &str = "/run/guide/vault-key";
/// The disk's first key, which the machine's layout reads, as core's disko
/// module's `jitsusama.disk.installKey` names it.
pub const INSTALL_KEY: &str = "/run/guide/install-key";
pub const LAYOUT: &str = "/run/guide/layout";
pub const RECOVERY_KEY: &str = "/run/guide/recovery-key";
/// Where the new system is mounted while it's installed.
pub const ROOT: &str = "/mnt";

/// What's in the vault for an install, by its path there.
pub const TARGET: &str = "target.json";
pub const LAYOUT_SCRIPT: &str = "layout.sh";
pub const SYSTEM_CACHE: &str = "cache/system";

/// What a reinstall stick installs, as the host that wrote it said, sealed
/// in the vault.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Target {
    /// The machine's name, typed back before its disk is wiped.
    pub machine: String,
    /// Its disk, by a name that doesn't change between boots.
    pub disk: String,
    /// What core calls its partitions and its open volume: `<name>-boot`,
    /// `<name>`, `/dev/mapper/<name>`.
    pub disk_name: String,
    /// Where its layout reads the disk's first key from.
    pub install_key: String,
    /// The packages its layout runs, by name.
    pub layout_packages: Vec<String>,
    /// The system, by its store path.
    pub system: String,
    /// The account the person signs in with.
    pub account: String,
    /// The YubiKeys that open the disk, by the names the person knows them
    /// by, in the order they're enrolled.
    pub yubikeys: Vec<String>,
}

impl Target {
    /// The partition its encrypted volume is on.
    pub fn volume(&self) -> String {
        format!("/dev/disk/by-partlabel/{}", self.disk_name)
    }

    /// Its volume, once it's open.
    pub fn mapped(&self) -> String {
        format!("/dev/mapper/{}", self.disk_name)
    }
}

/// A secret a step held in memory, if it's still there.
pub fn held(machine: &mut dyn Machine, path: &str) -> Result<Option<Secret>, Fault> {
    Ok(machine
        .read(path)?
        .map(|bytes| Secret::new(String::from_utf8_lossy(&bytes).into_owned())))
}

/// The vault's key, held since it was opened.
pub fn vault_key(machine: &mut dyn Machine) -> Result<Option<VaultKey>, Fault> {
    held(machine, VAULT_KEY)?
        .map(|text| VaultKey::from_secret(&text))
        .transpose()
}

/// The open vault, with the key held since it was opened.
pub fn vault<'m>(machine: &'m mut dyn Machine) -> Result<Vault<'m, dyn Store + 'm>, Fault> {
    let key = vault_key(machine)?.ok_or_else(|| Fault::Blocked {
        step: "reading the vault",
        why: "the vault isn't open".into(),
    })?;
    Vault::open(machine.vault()?, key)
}

/// What the vault says to install.
pub fn target(machine: &mut dyn Machine) -> Result<Target, Fault> {
    let mut json = vec![];
    vault(machine)?.extract(TARGET, &mut json)?;
    serde_json::from_slice(&json)
        .map_err(|why| Fault::unreadable("what the vault says to install", why))
}

/// The steps before the vault is open, which say what else there is.
pub fn opening() -> Vec<Box<dyn Step>> {
    vec![Box::new(vault::CheckKeyboard), Box::new(vault::OpenVault)]
}

/// Every step of installing from the stick, the YubiKeys named as the
/// vault's target names them, in the order they're enrolled.
pub fn steps(yubikeys: &[String]) -> Vec<Box<dyn Step>> {
    let mut steps = opening();
    steps.push(Box::new(disk::ConfirmDisk));
    steps.push(Box::new(disk::LayOutDisk));
    for name in yubikeys {
        steps.push(Box::new(keys::EnrolYubikey::new(name)));
    }
    steps.push(Box::new(keys::MakeRecoveryKey));
    steps.push(Box::new(keys::KeepRecoveryKey));
    steps.push(Box::new(system::MakeSecureBootKeys));
    steps.push(Box::new(system::InstallSystem));
    steps.push(Box::new(system::SetPassword));
    steps.push(Box::new(disk::WipeInstallKey {
        yubikeys: yubikeys.len(),
    }));
    steps
}
