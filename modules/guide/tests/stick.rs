//! Writing a whole stick into an image file, judged by the tools a person
//! would reach for: sfdisk reads its partition table, fsck.fat its vault
//! and its EFI system partition, sbverify its boot program, and the vault
//! opens with its key. They come from the package's check inputs or the
//! dev shell's, with systemd's EFI programs in `GUIDE_TEST_EFI`.

mod common;

use common::Scratch;
use std::ffi::OsStr;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use guide::sign::Signer;
use guide::stick::gpt::{EFI_SYSTEM, SECTOR, Table};
use guide::stick::{self, Image};
use guide::vault::store::Store;
use guide::vault::{self, Manifest, Packer, Vault, VaultKey};
use guide::{Fault, Secret};

fn try_run(program: &str, args: &[&OsStr]) -> Output {
    Command::new(program)
        .args(args)
        .env("MTOOLS_SKIP_CHECK", "1")
        .output()
        .unwrap()
}

fn run(program: &str, args: &[&OsStr]) -> Output {
    let output = try_run(program, args);
    assert!(
        output.status.success(),
        "{program}: {}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

/// systemd-boot, the program the made-up image boots.
fn boot_program() -> PathBuf {
    Path::new(
        &std::env::var("GUIDE_TEST_EFI").expect("GUIDE_TEST_EFI names systemd's EFI programs"),
    )
    .join("systemd-bootx64.efi")
}

/// A 1 MiB EFI system partition, made as repart makes the stick's, holding
/// this program where firmware boots a removable disk from.
fn make_esp(directory: &Path, program: Option<&Path>) -> Vec<u8> {
    let esp = directory.join("esp.fat");
    let _ = fs::remove_file(&esp);
    run(
        "mkfs.fat",
        &["-C".as_ref(), esp.as_os_str(), "1024".as_ref()],
    );
    if let Some(program) = program {
        run(
            "mmd",
            &[
                "-i".as_ref(),
                esp.as_os_str(),
                "::EFI".as_ref(),
                "::EFI/BOOT".as_ref(),
            ],
        );
        run(
            "mcopy",
            &[
                "-i".as_ref(),
                esp.as_os_str(),
                program.as_os_str(),
                "::EFI/BOOT/BOOTX64.EFI".as_ref(),
            ],
        );
    }
    fs::read(esp).unwrap()
}

/// An image laid out as core's is: an ESP holding this boot program, then
/// verity hashes and a store each filled with bytes of its own, in an
/// 8 MiB file with its tables.
fn make_image(directory: &Path, program: Option<&Path>) -> PathBuf {
    let parts: [(&str, u8, u64, u64); 3] = [
        ("esp", 0x11, 2048, 4095),
        ("store-verity", 0x22, 4096, 4999),
        ("store", 0x33, 5000, 14000),
    ];
    let mut slots = vec![[0u8; 128]; 128];
    for (slot, (name, fill, first, last)) in parts.iter().enumerate() {
        let entry = &mut slots[slot];
        let kind = if slot == 0 { EFI_SYSTEM } else { [*fill; 16] };
        entry[..16].copy_from_slice(&kind);
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
    for (_, fill, first, last) in &parts[1..] {
        let range = (first * SECTOR) as usize..((last + 1) * SECTOR) as usize;
        for (n, byte) in bytes[range].iter_mut().enumerate() {
            *byte = fill ^ (n as u8);
        }
    }
    let esp = make_esp(directory, program);
    bytes[(2048 * SECTOR) as usize..(4096 * SECTOR) as usize].copy_from_slice(&esp);
    for (at, rendered) in table.render(sectors) {
        bytes[at as usize..at as usize + rendered.len()].copy_from_slice(&rendered);
    }
    let path = directory.join("image.raw");
    fs::write(&path, bytes).unwrap();
    path
}

fn image(directory: &Path) -> Image {
    Image::open(File::open(make_image(directory, Some(&boot_program()))).unwrap()).unwrap()
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

/// One partition of the stick, copied out into its own file.
fn copy_out(disk: &mut File, partition: &stick::gpt::Partition, to: &Path) {
    let mut region = vec![0u8; partition.bytes() as usize];
    disk.seek(SeekFrom::Start(partition.offset())).unwrap();
    disk.read_exact(&mut region).unwrap();
    fs::write(to, region).unwrap();
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

/// A db key and certificate as sbctl keeps them, in their own directory.
fn db_key(directory: &Path) -> PathBuf {
    let keys = directory.join("keys");
    fs::create_dir_all(&keys).unwrap();
    run(
        "openssl",
        &[
            "req".as_ref(),
            "-new".as_ref(),
            "-x509".as_ref(),
            "-newkey".as_ref(),
            "rsa:2048".as_ref(),
            "-nodes".as_ref(),
            "-days".as_ref(),
            "3650".as_ref(),
            "-subj".as_ref(),
            "/CN=Database Key".as_ref(),
            "-keyout".as_ref(),
            keys.join("db.key").as_os_str(),
            "-out".as_ref(),
            keys.join("db.pem").as_os_str(),
        ],
    );
    keys
}

#[test]
fn a_written_stick_is_whole_to_sfdisk_fsck_and_its_key() {
    let directory = Scratch::new("whole");
    let mut image = image(&directory);
    // A stick's size is whole sectors but rarely whole mebibytes.
    let path = stick_file(&directory, (100 << 20) + 7 * SECTOR);
    let mut disk = open(&path);
    let key = VaultKey::generate();
    let nano = age::x25519::Identity::generate();
    let written = stick::write(
        &mut image,
        &mut disk,
        stick::new_guid().unwrap(),
        None,
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

    let verified = run("sfdisk", &["--verify".as_ref(), path.as_os_str()]);
    assert!(
        String::from_utf8_lossy(&verified.stdout).contains("No errors detected"),
        "{}",
        String::from_utf8_lossy(&verified.stdout)
    );
    let listed = run("sfdisk", &["--json".as_ref(), path.as_os_str()]);
    let listed: serde_json::Value = serde_json::from_slice(&listed.stdout).unwrap();
    let names: Vec<&str> = listed["partitiontable"]["partitions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|partition| partition["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["esp", "store-verity", "store", "vault"]);

    let vault_copy = directory.join("vault.fat");
    copy_out(&mut disk, &written.vault, &vault_copy);
    run("fsck.fat", &["-n".as_ref(), vault_copy.as_os_str()]);
}

#[test]
fn the_images_partitions_are_copied_byte_for_byte() {
    let directory = Scratch::new("copied");
    let image_path = make_image(&directory, Some(&boot_program()));
    let mut image = Image::open(File::open(&image_path).unwrap()).unwrap();
    let path = stick_file(&directory, 100 << 20);
    let mut disk = open(&path);
    let key = VaultKey::generate();
    let nano = age::x25519::Identity::generate();
    stick::write(
        &mut image,
        &mut disk,
        stick::new_guid().unwrap(),
        None,
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
}

/// The boot program an EFI system partition holds, copied out.
fn boot_program_in(directory: &Path, esp: &Path) -> PathBuf {
    let program = directory.join("BOOTX64.EFI");
    let _ = fs::remove_file(&program);
    run(
        "mcopy",
        &[
            "-i".as_ref(),
            esp.as_os_str(),
            "::EFI/BOOT/BOOTX64.EFI".as_ref(),
            program.as_os_str(),
        ],
    );
    program
}

#[test]
fn a_sticks_own_boot_program_is_signed_again_in_place_by_the_machines_key() {
    let directory = Scratch::new("signed-in-place");
    let esp = directory.join("esp-in-place.fat");
    fs::write(&esp, make_esp(&directory, Some(&boot_program()))).unwrap();
    let first = db_key(&directory.join("first"));
    let second = db_key(&directory.join("second"));

    // Unsigned as the image made it, then signed by one machine's key and
    // again by another's, as a stick is once an install has the keys.
    for keys in [&first, &second] {
        stick::sign_in_place(&mut open(&esp), &Signer::open(keys).unwrap()).unwrap();
    }
    run("fsck.fat", &["-n".as_ref(), esp.as_os_str()]);
    let program = boot_program_in(&directory, &esp);
    let verify = |keys: &Path| {
        try_run(
            "sbverify",
            &[
                "--cert".as_ref(),
                keys.join("db.pem").as_os_str(),
                program.as_os_str(),
            ],
        )
        .status
        .success()
    };
    assert!(verify(&second), "the last key's signature doesn't verify");
    assert!(!verify(&first), "the first key's signature is still there");

    // The same key again, as a reinstall from the machine's own stick
    // finds it, writes nothing, a second on so a new signature would
    // differ.
    let before = fs::read(&esp).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(1100));
    stick::sign_in_place(&mut open(&esp), &Signer::open(&second).unwrap()).unwrap();
    assert!(
        fs::read(&esp).unwrap() == before,
        "the partition was written"
    );
}

#[test]
fn only_the_boot_program_firmware_boots_is_signed_in_place() {
    let directory = Scratch::new("signed-in-place-only");
    let esp = directory.join("esp-in-place.fat");
    fs::write(&esp, make_esp(&directory, Some(&boot_program()))).unwrap();
    // Another program, as anyone who could write the stick's partition
    // might leave beside the one firmware boots.
    run(
        "mcopy",
        &[
            "-i".as_ref(),
            esp.as_os_str(),
            boot_program().as_os_str(),
            "::EFI/BOOT/OTHER.EFI".as_ref(),
        ],
    );
    let keys = db_key(&directory.join("keys"));
    stick::sign_in_place(&mut open(&esp), &Signer::open(&keys).unwrap()).unwrap();
    let other = directory.join("OTHER.EFI");
    run(
        "mcopy",
        &[
            "-i".as_ref(),
            esp.as_os_str(),
            "::EFI/BOOT/OTHER.EFI".as_ref(),
            other.as_os_str(),
        ],
    );
    let verify = |program: &Path| {
        try_run(
            "sbverify",
            &[
                "--cert".as_ref(),
                keys.join("db.pem").as_os_str(),
                program.as_os_str(),
            ],
        )
        .status
        .success()
    };
    assert!(verify(&boot_program_in(&directory, &esp)));
    assert!(!verify(&other), "the other program was signed");
}

#[test]
fn a_signed_sticks_boot_program_verifies_with_the_machines_db_key() {
    let directory = Scratch::new("signed");
    let keys = db_key(&directory);
    let signer = Signer::open(&keys).unwrap();
    let mut image = image(&directory);
    let path = stick_file(&directory, 100 << 20);
    let mut disk = open(&path);
    let key = VaultKey::generate();
    let nano = age::x25519::Identity::generate();
    let written = stick::write(
        &mut image,
        &mut disk,
        stick::new_guid().unwrap(),
        Some(&signer),
        fill(&key, &nano),
    )
    .unwrap();
    stick::verify(&written, &mut disk, |_| Ok(())).unwrap();

    let esp = image
        .partitions()
        .into_iter()
        .find(|partition| partition.name == "esp")
        .unwrap();
    let esp_copy = directory.join("esp-written.fat");
    copy_out(&mut disk, &esp, &esp_copy);
    run("fsck.fat", &["-n".as_ref(), esp_copy.as_os_str()]);
    let program = directory.join("BOOTX64.EFI");
    run(
        "mcopy",
        &[
            "-i".as_ref(),
            esp_copy.as_os_str(),
            "::EFI/BOOT/BOOTX64.EFI".as_ref(),
            program.as_os_str(),
        ],
    );
    run(
        "sbverify",
        &[
            "--cert".as_ref(),
            keys.join("db.pem").as_os_str(),
            program.as_os_str(),
        ],
    );
}

#[test]
fn an_image_with_nothing_to_sign_is_refused_with_nothing_written() {
    let directory = Scratch::new("unsignable");
    let keys = db_key(&directory);
    let signer = Signer::open(&keys).unwrap();
    let mut image = Image::open(File::open(make_image(&directory, None)).unwrap()).unwrap();
    let path = stick_file(&directory, 100 << 20);
    let mut disk = open(&path);
    let result = stick::write(
        &mut image,
        &mut disk,
        stick::new_guid().unwrap(),
        Some(&signer),
        |_| Ok(()),
    );
    assert!(
        matches!(result, Err(Fault::Unreadable { .. })),
        "{:?}",
        result.err()
    );
    assert!(fs::read(&path).unwrap().iter().all(|byte| *byte == 0xee));
}

#[test]
fn a_byte_that_didnt_reach_the_stick_is_caught() {
    let directory = Scratch::new("caught");
    let mut image = image(&directory);
    let path = stick_file(&directory, 100 << 20);
    let mut disk = open(&path);
    let key = VaultKey::generate();
    let nano = age::x25519::Identity::generate();
    let written = stick::write(
        &mut image,
        &mut disk,
        stick::new_guid().unwrap(),
        None,
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
}

#[test]
fn a_write_that_stops_part_way_leaves_no_partition_table() {
    let directory = Scratch::new("stops");
    let mut image = image(&directory);
    let path = stick_file(&directory, 100 << 20);
    let mut disk = open(&path);
    // A stick that held a table before, as one rewritten does.
    let mut earlier = self::image(&directory);
    let key = VaultKey::generate();
    let nano = age::x25519::Identity::generate();
    stick::write(
        &mut earlier,
        &mut disk,
        stick::new_guid().unwrap(),
        None,
        fill(&key, &nano),
    )
    .unwrap();

    let stopped = stick::write(
        &mut image,
        &mut disk,
        stick::new_guid().unwrap(),
        None,
        |_| {
            Err(Fault::Unkept {
                path: "the vault".into(),
                why: "the stick was pulled".into(),
            })
        },
    );
    assert!(stopped.is_err());
    let listed = try_run("sfdisk", &["--json".as_ref(), path.as_os_str()]);
    assert!(
        !listed.status.success(),
        "{}",
        String::from_utf8_lossy(&listed.stdout)
    );
    let stick = fs::read(&path).unwrap();
    assert!(Table::read(&stick).is_err());
    let backup = &stick[stick.len() - SECTOR as usize..];
    assert_ne!(&backup[..8], b"EFI PART");
}

#[test]
fn a_stick_too_small_is_refused_with_nothing_written() {
    let directory = Scratch::new("small");
    let mut image = image(&directory);
    let path = stick_file(&directory, 32 << 20);
    let mut disk = open(&path);
    let result = stick::write(
        &mut image,
        &mut disk,
        stick::new_guid().unwrap(),
        None,
        |_| Ok(()),
    );
    assert!(
        matches!(result, Err(Fault::Blocked { .. })),
        "{:?}",
        result.err()
    );
    assert!(fs::read(&path).unwrap().iter().all(|byte| *byte == 0xee));
}

#[test]
fn the_passphrase_opens_a_vault_on_the_stick_when_no_yubikey_will() {
    let directory = Scratch::new("passphrase");
    let mut image = image(&directory);
    let path = stick_file(&directory, 100 << 20);
    let mut disk = open(&path);
    let key = VaultKey::generate();
    let passphrase = Secret::new("a long recovery passphrase".into());
    let sealed = key.seal_with_passphrase(&passphrase, Some(10)).unwrap();
    let written = stick::write(
        &mut image,
        &mut disk,
        stick::new_guid().unwrap(),
        None,
        |store| {
            store
                .create(vault::KEY_PASSPHRASE)?
                .write_all(&sealed)
                .unwrap();
            let mut packer = Packer::new(store, &key);
            packer.add("secure-boot/db.key", &b"the machine's db key"[..])?;
            packer.finish(&manifest())
        },
    )
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
}
