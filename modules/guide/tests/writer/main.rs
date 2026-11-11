//! Writing a stick, held against what lsblk printed in the
//! guide's VM (fixtures/qemu-canokey/), whose stick is `/dev/sda` on USB:
//! only ever the one stick plugged in, what's mounted from it handed over
//! to be unmounted, and a stick that sfdisk reads back whole.
//!
//! Each part of writing a stick has a file of its own here, beside this one,
//! which keeps what more than one of them uses.

#[path = "../common/mod.rs"]
mod common;

use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use guide::Tool;
use guide::stand_in::StandIn;
use guide::stick::gpt::{SECTOR, Table};
use guide::writer::WriteStick;

macro_rules! fixture {
    ($file:literal) => {
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/fixtures/qemu-canokey/",
            $file
        ))
    };
}

mod bundles;
mod phrase;
mod roots;
mod which_stick;

/// An image laid out as core's is, 8 MiB, its three partitions filled.
fn image(directory: &Path) -> PathBuf {
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
        for (unit, at) in name.encode_utf16().zip((56..).step_by(2)) {
            entry[at..at + 2].copy_from_slice(&unit.to_le_bytes());
        }
    }
    let table = Table {
        disk_guid: [0x5a; 16],
        slots,
    };
    let sectors = 16384;
    let mut bytes = vec![0u8; usize::try_from(sectors * SECTOR).unwrap()];
    for (at, rendered) in table.render(sectors) {
        let at = usize::try_from(at).unwrap();
        bytes[at..at + rendered.len()].copy_from_slice(&rendered);
    }
    let path = directory.join("image.raw");
    fs::write(&path, bytes).unwrap();
    path
}

/// A used stick of 100 MiB, standing in for the device.
fn stick_file(directory: &Path) -> PathBuf {
    let path = directory.join("stick.raw");
    let mut file = File::create(&path).unwrap();
    file.write_all(&vec![0xee; 100 << 20]).unwrap();
    path
}

fn repair(directory: &Path) -> WriteStick {
    WriteStick::new(image(directory), None, vec![])
}

fn lsblk(stand_in: &mut StandIn, json: &str) {
    stand_in.prints(Tool::Lsblk, &["--json", "--output-all", "--bytes"], json);
}

/// A payload of nothing but its target, which is all the card reads.
fn payload(directory: &Path, target: &str) -> guide::writer::Payload {
    let payload = directory.join("payload");
    fs::create_dir_all(payload.join("cache")).unwrap();
    fs::write(payload.join("target.json"), target).unwrap();
    fs::write(payload.join("layout.sh"), "").unwrap();
    fs::write(payload.join("mount.sh"), "").unwrap();
    guide::writer::Payload::open(&payload).unwrap()
}
