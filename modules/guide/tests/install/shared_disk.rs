//! A disk shared with macOS: Asahi's table read, one partition added for the
//! machine, and nothing else on the disk touched.

use guide::install::Target;
use guide::install::disk::ConfirmDisk;
use guide::stand_in::StandIn;
use guide::{Fault, Standing, Step, Tool};

use super::{lsblk, stick, target};
use crate::common::Scratch;

/// What sfdisk printed for a disk laid out as Asahi's installer leaves an
/// Apple silicon Mac, and once the machine's partition was added.
const ASAHI_BEFORE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/fixtures/asahi-disk-image/sfdisk-before.json"
));
const ASAHI_AFTER: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/fixtures/asahi-disk-image/sfdisk-after.json"
));
const MAC_DISK: &str = "/dev/nvme0n1";
const SFDISK: [&str; 2] = ["--json", MAC_DISK];
const MADE_ROOM: [&str; 6] = [
    "--set-alignment=256",
    "--new=6:25274624:242965539",
    "--typecode=6:8309",
    "--change-name=6:system",
    "--change-name=5:system-boot",
    MAC_DISK,
];

/// A Mac's target, its disk shared with macOS.
fn shared_target() -> Target {
    let mut target = target("/dev/disk/by-id/nvme-APPLE_SSD_AP1024Z");
    target.shared = true;
    target.secure_boot = false;
    target
}

/// The Mac's disk as lsblk shows it, only its boot partition named here.
fn mac_disk() -> guide::observe::disks::Disk {
    guide::observe::disks::Disk {
        path: MAC_DISK.into(),
        by_id: Some("nvme-APPLE_SSD_AP1024Z".into()),
        model: Some("APPLE SSD AP1024Z".into()),
        serial: None,
        bytes: 1_000_555_581_440,
        kind: guide::observe::disks::Kind::Internal,
        partitions: vec![],
    }
}

/// A machine on the Mac's stick whose partition table reads `before`, then
/// `after` once written, m1n1 naming no boot partition.
fn mac_stick(before: &str, after: &str) -> StandIn {
    let mut stand_in = StandIn::new();
    stand_in
        .prints(Tool::Sfdisk, &SFDISK, before)
        .prints(Tool::Sfdisk, &SFDISK, after)
        .no_file(guide::install::disk::ASAHI_ESP)
        .prints(Tool::Udevadm, &["settle"], "")
        .link("/dev/disk/by-partlabel/system", Some("/dev/nvme0n1p6"))
        .prints(Tool::Wipefs, &["--all", "/dev/nvme0n1p6"], "");
    stand_in
}

#[test]
fn asahis_table_is_read_with_its_numbers_and_free_space() {
    let table = guide::observe::table::parse(ASAHI_BEFORE, MAC_DISK).unwrap();
    assert_eq!(table.sector_size, 4096);
    let numbers: Vec<u32> = table.entries.iter().map(|entry| entry.number).collect();
    assert_eq!(numbers, [1, 2, 3, 4, 5]);
    assert_eq!(table.free_number(), Some(6));
    let esps: Vec<u32> = table
        .entries
        .iter()
        .filter(|entry| entry.is_esp())
        .map(|entry| entry.number)
        .collect();
    assert_eq!(esps, [5]);
    // From the first MiB after Asahi's boot partition to the recovery
    // container at the end.
    let gap = table.largest_gap().unwrap();
    assert_eq!((gap.first, gap.last), (25_274_624, 242_965_539));
}

#[test]
fn a_shared_disk_gains_one_partition_and_a_name_for_its_boot_partition() {
    let mut stand_in = mac_stick(ASAHI_BEFORE, ASAHI_AFTER);
    stand_in.prints(Tool::Sgdisk, &MADE_ROOM, "");
    guide::install::disk::make_room(&mut stand_in, &shared_target(), &mac_disk()).unwrap();
    assert_eq!(
        stand_in.calls,
        [
            "sfdisk --json /dev/nvme0n1",
            &format!("sgdisk {}", MADE_ROOM.join(" ")),
            "sfdisk --json /dev/nvme0n1",
            "udevadm settle",
            "wipefs --all /dev/nvme0n1p6",
        ]
    );
}

#[test]
fn a_write_that_changed_another_partition_stops_before_anything_is_formatted() {
    let moved = ASAHI_AFTER.replacen("\"name\": \"Container\"", "\"name\": \"Contained\"", 1);
    let mut stand_in = mac_stick(ASAHI_BEFORE, &moved);
    stand_in.prints(Tool::Sgdisk, &MADE_ROOM, "");
    let made = guide::install::disk::make_room(&mut stand_in, &shared_target(), &mac_disk());
    assert!(
        matches!(&made, Err(Fault::Blocked { why, .. }) if why.contains("partition 2 isn't as it was")),
        "{made:?}"
    );
    assert!(
        !stand_in.calls.iter().any(|call| call.starts_with("wipefs")),
        "{:#?}",
        stand_in.calls
    );
}

#[test]
fn a_reinstall_on_a_shared_disk_wipes_only_its_own_partition() {
    let mut stand_in = mac_stick(ASAHI_AFTER, ASAHI_AFTER);
    guide::install::disk::make_room(&mut stand_in, &shared_target(), &mac_disk()).unwrap();
    assert_eq!(
        stand_in.calls,
        ["sfdisk --json /dev/nvme0n1", "wipefs --all /dev/nvme0n1p6"]
    );
}

#[test]
fn a_shared_disk_without_room_for_the_machine_is_left_alone() {
    // macOS kept nearly all of it.
    let full = ASAHI_BEFORE.replacen("\"size\": 24414062", "\"size\": 242000000", 1);
    let mut stand_in = mac_stick(&full, &full);
    let made = guide::install::disk::make_room(&mut stand_in, &shared_target(), &mac_disk());
    assert!(
        matches!(&made, Err(Fault::Blocked { why, .. }) if why.contains("largest free space is 3 GB")),
        "{made:?}"
    );
    assert_eq!(stand_in.calls, ["sfdisk --json /dev/nvme0n1"]);
}

#[test]
fn the_boot_partition_m1n1_names_is_the_one_named_for_the_machine() {
    // Partition 2 is macOS's container, which no system boots from.
    let mut stand_in = mac_stick(ASAHI_BEFORE, ASAHI_AFTER);
    stand_in.file(
        guide::install::disk::ASAHI_ESP,
        b"2e2b5c2a-6e43-4a0b-9f16-2c8b3d6a9e02\0",
    );
    let made = guide::install::disk::make_room(&mut stand_in, &shared_target(), &mac_disk());
    assert!(
        matches!(&made, Err(Fault::Blocked { why, .. }) if why.contains("isn't an EFI system partition")),
        "{made:?}"
    );

    let mut stand_in = mac_stick(ASAHI_BEFORE, ASAHI_AFTER);
    stand_in
        .file(
            guide::install::disk::ASAHI_ESP,
            b"5e5b5c2a-6e43-4a0b-9f16-2c8b3d6a9e05\0",
        )
        .prints(Tool::Sgdisk, &MADE_ROOM, "");
    guide::install::disk::make_room(&mut stand_in, &shared_target(), &mac_disk()).unwrap();
}

#[test]
fn a_shared_disk_is_confirmed_as_its_partition_rather_than_the_whole_disk() {
    let mut target = target("/dev/disk/by-id/virtio-root");
    target.shared = true;
    let scratch = Scratch::new("install-confirm-shared");
    let mut stand_in = stick(&scratch, &target);
    lsblk(&mut stand_in, fixture!("lsblk-blank.json"));
    let Standing::Needed { asks } = ConfirmDisk.observe(&mut stand_in).unwrap() else {
        panic!("the disk is still to confirm")
    };
    let said = format!("{asks:?}");
    assert!(
        said.contains("leaves its other partitions as they are"),
        "{said}"
    );
}
