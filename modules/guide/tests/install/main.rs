//! The install's rules that guard a disk, held against what the tools
//! printed in the guide's VM (fixtures/qemu-canokey/): nothing is wiped
//! until the machine's name is typed back, never a removable disk, no key
//! enrolled twice, and the install's key comes off only once every
//! YubiKey and the recovery key can open the disk.
//!
//! Each area of the install has a file of its own here, beside this one, which
//! keeps what more than one of them uses.

#[path = "../common/mod.rs"]
mod common;

use std::path::PathBuf;

use guide::Tool;
use guide::install::{INSTALL_KEY, Start, TARGET, Target, VAULT_KEY};
use guide::stand_in::StandIn;
use guide::vault::store::Directory;
use guide::vault::{Manifest, Packer, VaultKey};

macro_rules! fixture {
    ($file:literal) => {
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/fixtures/qemu-canokey/",
            $file
        ))
    };
}

mod bundle;
mod disk;
mod keys;
mod memory;
mod recovery_key;
mod secure_boot;
mod shared_disk;
mod stopped;
mod vault;

fn target(disk: &str) -> Target {
    Target {
        machine: "machine".into(),
        disk: disk.into(),
        disk_name: "system".into(),
        install_key: INSTALL_KEY.into(),
        layout_packages: vec!["cryptsetup".into()],
        system: "/nix/store/abc-nixos-system-machine".into(),
        account: "alice".into(),
        yubikeys: vec!["the Nano".into(), "the 5C".into()],
        recipients: vec![],
        start_from_stick: None,
        clear_secure_boot_keys: None,
        leave_out: vec![],
        secure_boot: true,
        shared: false,
    }
}

/// The ids of the install's steps for this target, starting afresh.
fn step_ids(target: &Target) -> Vec<&'static str> {
    guide::install::steps(target, Start::Afresh)
        .iter()
        .map(|step| step.id())
        .collect()
}

/// A stand-in machine on the stick, its vault open and holding this target.
fn stick(directory: &std::path::Path, target: &Target) -> StandIn {
    stick_carrying(directory, target, &[])
}

/// The same, its vault holding these files too.
fn stick_carrying(
    directory: &std::path::Path,
    target: &Target,
    files: &[(&str, &[u8])],
) -> StandIn {
    let mut store = Directory::new(PathBuf::from(&directory));
    let key = VaultKey::generate();
    let mut packer = Packer::new(&mut store, &key);
    packer
        .add(TARGET, serde_json::to_vec(target).unwrap().as_slice())
        .unwrap();
    for (path, bytes) in files {
        packer.add(path, *bytes).unwrap();
    }
    packer
        .finish(&Manifest {
            format: 1,
            machine: target.machine.clone(),
            opened_by: vec![],
        })
        .unwrap();
    let mut stand_in = StandIn::new();
    stand_in
        .with_vault(Box::new(store))
        .file(VAULT_KEY, key.to_secret().expose().as_bytes())
        .no_file(INSTALL_KEY)
        .link("/dev/disk/by-id/virtio-root", Some("/dev/vda"))
        .link("/dev/disk/by-id/usb-stick", Some("/dev/sda"));
    stand_in
}

/// These keys plugged in: each canokey as `fido2-token` lists and
/// describes one, and "linux-id" as the XPS 14's virtual key.
fn plugged(stand_in: &mut StandIn, keys: &[(&str, &str)]) {
    let mut listed = String::new();
    for (path, kind) in keys {
        let (line, info) = match *kind {
            "linux-id" => (
                format!("{path}: vendor=0x15d9, product=0x0a37 ( linux-id)\n"),
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/fixtures/dell-xps-14-da14260/fido2-token-I.txt"
                )),
            ),
            _ => (
                fixture!("fido2-token-L.txt").replace("/dev/hidraw1", path),
                fixture!("fido2-token-I.txt"),
            ),
        };
        listed.push_str(&line);
        stand_in.prints(Tool::Fido2Token, &["-I", path], info);
    }
    stand_in.prints(Tool::Fido2Token, &["-L"], &listed);
}

fn lsblk(stand_in: &mut StandIn, json: &str) {
    stand_in.prints(Tool::Lsblk, &["--json", "--output-all", "--bytes"], json);
}

fn wiped(stand_in: &StandIn) -> bool {
    stand_in
        .calls
        .iter()
        .any(|call| call.starts_with("wipefs") || call.starts_with("sgdisk"))
}

const DUMP: [&str; 3] = [
    "luksDump",
    "--dump-json-metadata",
    "/dev/disk/by-partlabel/system",
];

const BUNDLE: &str = "/mnt/var/lib/guide/rescue-bundle.json";
const RECOVERY: &str = "fkhvbicb-urbtgcbh-gdbbcbkf-ekdjvcrg-jhcvdbbe-jtbifkkv-ncdvurrb-tutbkbcl";
const SBCTL: &str = "/mnt/var/lib/sbctl";

const VOLUME: &str = "/dev/disk/by-partlabel/system";
const TEST: [&str; 5] = [
    "open",
    "--test-passphrase",
    "--verbose",
    "--key-file=-",
    VOLUME,
];
const IMPORT: [&str; 4] = ["token", "import", "--json-file=-", VOLUME];
