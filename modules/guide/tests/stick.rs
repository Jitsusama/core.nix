//! Writing a whole stick into an image file, judged by the tools a person
//! would reach for: sfdisk reads its partition table, fsck.fat its vault,
//! and the vault opens with its key. Both run from the package's check
//! inputs, or the dev shell's.

use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

use guide::stick::gpt::{SECTOR, Table};
use guide::stick::{self, Image};
use guide::vault::store::Store;
use guide::vault::{self, Manifest, Packer, Vault, VaultKey};
use guide::{Fault, Secret};

fn scratch(name: &str) -> PathBuf {
    let directory = std::env::temp_dir().join(format!("guide-stick-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&directory);
    fs::create_dir_all(&directory).unwrap();
    directory
}

/// An image laid out as core's is: an ESP, verity hashes and a store, each
/// filled with bytes of its own, in an 8 MiB file with its tables.
fn make_image(directory: &Path) -> PathBuf {
    let parts: [(&str, u8, u64, u64); 3] = [
        ("esp", 0x11, 2048, 4095),
        ("store-verity", 0x22, 4096, 4999),
        ("store", 0x33, 5000, 14000),
    ];
    let mut slots = vec![[0u8; 128]; 128];
    for (slot, (name, fill, first, last)) in parts.iter().enumerate() {
        let entry = &mut slots[slot];
        entry[..16].copy_from_slice(&[*fill; 16]);
        entry[16..32].copy_from_slice(&[fill + 1; 16]);
        entry[32..40].copy_from_slice(&first.to_le_bytes());
        entry[40..48].copy_from_slice(&last.to_le_bytes());
        entry[55] = 0x10;
        for (unit, at) in name.encode_utf16().zip((56..).step_by(2)) {
            entry[at..at + 2].copy_from_slice(&unit.to_le_bytes());
        }
    }
    let table = Table {
        disk_guid: [0x5a; 16],
        slots,
    };
    let sectors = 16384;
    let mut bytes = vec![0u8; (sectors * SECTOR) as usize];
    for (_, fill, first, last) in parts {
        let range = (first * SECTOR) as usize..((last + 1) * SECTOR) as usize;
        for (n, byte) in bytes[range].iter_mut().enumerate() {
            *byte = fill ^ (n as u8);
        }
    }
    for (at, rendered) in table.render(sectors) {
        bytes[at as usize..at as usize + rendered.len()].copy_from_slice(&rendered);
    }
    let path = directory.join("image.raw");
    fs::write(&path, bytes).unwrap();
    path
}

/// A stick of this many bytes, every byte set, as a used stick would be.
fn stick_file(directory: &Path, bytes: u64) -> PathBuf {
    let path = directory.join("stick.raw");
    let mut file = File::create(&path).unwrap();
    let chunk = vec![0xeeu8; 1 << 20];
    let mut left = bytes;
    while left > 0 {
        let count = left.min(chunk.len() as u64) as usize;
        file.write_all(&chunk[..count]).unwrap();
        left -= count as u64;
    }
    path
}

fn open(path: &Path) -> File {
    OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .unwrap()
}

fn manifest() -> Manifest {
    Manifest {
        format: 1,
        machine: "a laptop".into(),
        opened_by: vec!["the Nano".into()],
    }
}

fn fill(
    key: &VaultKey,
    nano: &age::x25519::Identity,
) -> impl FnOnce(&mut dyn Store) -> Result<(), Fault> {
    let sealed = key
        .seal(&[&nano.to_public() as &dyn age::Recipient])
        .unwrap();
    let packed = VaultKey::open(&sealed, nano).unwrap();
    move |store| {
        store.create(vault::KEY)?.write_all(&sealed).unwrap();
        let mut packer = Packer::new(store, &packed);
        packer.add("cache/nix-cache-info", &b"StoreDir: /nix/store\n"[..])?;
        let big: Vec<u8> = (0..3_000_000u32).map(|n| (n % 251) as u8).collect();
        packer.add("cache/nar/a.nar.zst", &big[..])?;
        packer.finish(&manifest())
    }
}

fn tool(variable: &str, name: &str) -> Command {
    Command::new(std::env::var(variable).unwrap_or_else(|_| name.into()))
}

#[test]
fn a_written_stick_is_whole_to_sfdisk_fsck_and_its_key() {
    let directory = scratch("whole");
    let mut image = Image::open(File::open(make_image(&directory)).unwrap()).unwrap();
    // A stick's size is whole sectors but rarely whole mebibytes.
    let path = stick_file(&directory, (100 << 20) + 7 * SECTOR);
    let mut disk = open(&path);
    let key = VaultKey::generate();
    let nano = age::x25519::Identity::generate();
    let written = stick::write(
        &mut image,
        &mut disk,
        stick::new_guid().unwrap(),
        fill(&key, &nano),
    )
    .unwrap();

    stick::verify(&written, &mut disk, |store| {
        assert_eq!(vault::manifest(store)?, manifest());
        let mut sealed = vec![];
        store.open(vault::KEY)?.read_to_end(&mut sealed).unwrap();
        let mut vault = Vault::open(store, VaultKey::open(&sealed, &nano)?)?;
        let paths: Vec<String> = vault.files().iter().map(|file| file.path.clone()).collect();
        for path in paths {
            vault.extract(&path, std::io::sink())?;
        }
        Ok(())
    })
    .unwrap();

    let verified = tool("SFDISK", "sfdisk")
        .arg("--verify")
        .arg(&path)
        .output()
        .unwrap();
    assert!(
        verified.status.success()
            && String::from_utf8_lossy(&verified.stdout).contains("No errors detected"),
        "{}{}",
        String::from_utf8_lossy(&verified.stdout),
        String::from_utf8_lossy(&verified.stderr)
    );
    let listed = tool("SFDISK", "sfdisk")
        .arg("--json")
        .arg(&path)
        .output()
        .unwrap();
    let listed: serde_json::Value = serde_json::from_slice(&listed.stdout).unwrap();
    let names: Vec<&str> = listed["partitiontable"]["partitions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|partition| partition["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["esp", "store-verity", "store", "vault"]);

    let vault_copy = directory.join("vault.fat");
    let mut region = vec![0u8; written.vault.bytes() as usize];
    disk.seek(SeekFrom::Start(written.vault.offset())).unwrap();
    disk.read_exact(&mut region).unwrap();
    fs::write(&vault_copy, region).unwrap();
    let checked = tool("FSCK_FAT", "fsck.fat")
        .arg("-n")
        .arg(&vault_copy)
        .output()
        .unwrap();
    assert!(
        checked.status.success(),
        "{}{}",
        String::from_utf8_lossy(&checked.stdout),
        String::from_utf8_lossy(&checked.stderr)
    );
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn the_images_partitions_are_copied_byte_for_byte() {
    let directory = scratch("copied");
    let image_path = make_image(&directory);
    let mut image = Image::open(File::open(&image_path).unwrap()).unwrap();
    let path = stick_file(&directory, 100 << 20);
    let mut disk = open(&path);
    let key = VaultKey::generate();
    let nano = age::x25519::Identity::generate();
    stick::write(
        &mut image,
        &mut disk,
        stick::new_guid().unwrap(),
        fill(&key, &nano),
    )
    .unwrap();
    let source = fs::read(&image_path).unwrap();
    let stick = fs::read(&path).unwrap();
    for partition in image.partitions() {
        let range = partition.offset() as usize..(partition.offset() + partition.bytes()) as usize;
        assert!(source[range.clone()] == stick[range], "{}", partition.name);
    }
    // The UKI finds its store by these, so they're the image's own.
    let table = Table::read(&stick).unwrap();
    let original = Table::read(&source).unwrap();
    assert_eq!(table.disk_guid, original.disk_guid);
    assert_eq!(table.slots[..3], original.slots[..3]);
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn a_byte_that_didnt_reach_the_stick_is_caught() {
    let directory = scratch("caught");
    let mut image = Image::open(File::open(make_image(&directory)).unwrap()).unwrap();
    let path = stick_file(&directory, 100 << 20);
    let mut disk = open(&path);
    let key = VaultKey::generate();
    let nano = age::x25519::Identity::generate();
    let written = stick::write(
        &mut image,
        &mut disk,
        stick::new_guid().unwrap(),
        fill(&key, &nano),
    )
    .unwrap();
    let store = image
        .partitions()
        .into_iter()
        .find(|partition| partition.name == "store")
        .unwrap();
    let mut other = open(&path);
    other.seek(SeekFrom::Start(store.offset() + 12345)).unwrap();
    other.write_all(&[0]).unwrap();
    other.sync_all().unwrap();

    let result = stick::verify(&written, &mut disk, |_| Ok(()));
    assert!(
        matches!(&result, Err(Fault::Unproven { standing, .. }) if standing.contains("the store partition")),
        "{result:?}"
    );
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn a_write_that_stops_part_way_leaves_no_partition_table() {
    let directory = scratch("stops");
    let mut image = Image::open(File::open(make_image(&directory)).unwrap()).unwrap();
    let path = stick_file(&directory, 100 << 20);
    let mut disk = open(&path);
    // A stick that held a table before, as one rewritten does.
    let mut earlier = Image::open(File::open(make_image(&directory)).unwrap()).unwrap();
    let key = VaultKey::generate();
    let nano = age::x25519::Identity::generate();
    stick::write(
        &mut earlier,
        &mut disk,
        stick::new_guid().unwrap(),
        fill(&key, &nano),
    )
    .unwrap();

    let stopped = stick::write(&mut image, &mut disk, stick::new_guid().unwrap(), |_| {
        Err(Fault::Unkept {
            path: "the vault".into(),
            why: "the stick was pulled".into(),
        })
    });
    assert!(stopped.is_err());
    let listed = tool("SFDISK", "sfdisk")
        .arg("--json")
        .arg(&path)
        .output()
        .unwrap();
    assert!(
        !listed.status.success(),
        "{}",
        String::from_utf8_lossy(&listed.stdout)
    );
    let stick = fs::read(&path).unwrap();
    assert!(Table::read(&stick).is_err());
    let backup = &stick[stick.len() - SECTOR as usize..];
    assert_ne!(&backup[..8], b"EFI PART");
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn a_stick_too_small_is_refused_with_nothing_written() {
    let directory = scratch("small");
    let mut image = Image::open(File::open(make_image(&directory)).unwrap()).unwrap();
    let path = stick_file(&directory, 32 << 20);
    let mut disk = open(&path);
    let result = stick::write(
        &mut image,
        &mut disk,
        stick::new_guid().unwrap(),
        |_| Ok(()),
    );
    assert!(
        matches!(result, Err(Fault::Blocked { .. })),
        "{:?}",
        result.err()
    );
    assert!(fs::read(&path).unwrap().iter().all(|byte| *byte == 0xee));
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn the_passphrase_opens_a_vault_on_the_stick_when_no_yubikey_will() {
    let directory = scratch("passphrase");
    let mut image = Image::open(File::open(make_image(&directory)).unwrap()).unwrap();
    let path = stick_file(&directory, 100 << 20);
    let mut disk = open(&path);
    let key = VaultKey::generate();
    let passphrase = Secret::new("a long recovery passphrase".into());
    let sealed = key.seal_with_passphrase(&passphrase, Some(10)).unwrap();
    let written = stick::write(&mut image, &mut disk, stick::new_guid().unwrap(), |store| {
        store
            .create(vault::KEY_PASSPHRASE)?
            .write_all(&sealed)
            .unwrap();
        let mut packer = Packer::new(store, &key);
        packer.add("secure-boot/db.key", &b"the machine's db key"[..])?;
        packer.finish(&manifest())
    })
    .unwrap();
    stick::verify(&written, &mut disk, |store| {
        let mut sealed = vec![];
        store
            .open(vault::KEY_PASSPHRASE)?
            .read_to_end(&mut sealed)
            .unwrap();
        let mut vault = Vault::open(store, VaultKey::open_with_passphrase(&sealed, &passphrase)?)?;
        let mut key = vec![];
        vault.extract("secure-boot/db.key", &mut key)?;
        assert_eq!(key, b"the machine's db key");
        Ok(())
    })
    .unwrap();
    fs::remove_dir_all(directory).unwrap();
}
