//! The stick's system copied into memory and checked before it's run from
//! there, so the stick can come out.

use guide::install::keys::EnrolYubikey;
use guide::install::memory::{self, RunFromMemory};
use guide::stand_in::StandIn;
use guide::{Answers, Ask, Fault, Journey, Standing, Step, Tool};

use super::{stick, target};
use crate::common::Scratch;

const STORE_TABLE: &str = "0 16 verity 1 8:3 8:2 4096 4096 2 1 sha256 1f2e3d4c5b6a -";

/// The stick's store as the kernel shows it: two data blocks on sda3, its
/// hashes on sda2, with this much memory free.
fn store_on_the_stick(stand_in: &mut StandIn, available_kilobytes: u64) {
    stand_in
        .prints(Tool::Dmsetup, &["table", "usr"], STORE_TABLE)
        .file(
            "/proc/meminfo",
            format!("MemTotal: 16000000 kB\nMemAvailable: {available_kilobytes} kB\n").as_bytes(),
        )
        .file("/sys/dev/block/8:2/size", b"16\n")
        .file(
            "/sys/dev/block/8:3/uevent",
            b"MAJOR=8\nMINOR=3\nDEVNAME=sda3\nDEVTYPE=partition\n",
        )
        .file(
            "/sys/dev/block/8:2/uevent",
            b"MAJOR=8\nMINOR=2\nDEVNAME=sda2\nDEVTYPE=partition\n",
        )
        .file("/proc/mounts", b"tmpfs / tmpfs rw 0 0\n")
        .file("/dev/sda3", &[3u8; 12288])
        .file("/dev/sda2", &[2u8; 8192]);
}

const VERIFY: &[&str] = &[
    "verify",
    "/run/stick/store",
    "/run/stick/store-verity",
    "1f2e3d4c5b6a",
    "--no-superblock",
    "--format=1",
    "--data-block-size=4096",
    "--hash-block-size=4096",
    "--data-blocks=2",
    "--hash-offset=4096",
    "--hash=sha256",
    "--salt=-",
];

fn position(stand_in: &StandIn, call: &str) -> usize {
    stand_in
        .calls
        .iter()
        .position(|made| made.starts_with(call))
        .unwrap_or_else(|| panic!("{call} wasn't run: {:#?}", stand_in.calls))
}

#[test]
fn the_sticks_system_is_copied_into_memory_checked_then_run_from_there() {
    let mut stand_in = StandIn::new();
    store_on_the_stick(&mut stand_in, 8_000_000);
    stand_in
        .prints(
            Tool::Mount,
            &[
                "-t",
                "tmpfs",
                "-o",
                "size=1024k,mode=0700,X-mount.mkdir=0700",
                "stick",
                "/run/stick",
            ],
            "",
        )
        .prints(Tool::Veritysetup, VERIFY, "")
        .prints(Tool::Losetup, &["--associated", "/run/stick/store"], "")
        .prints(
            Tool::Losetup,
            &["--associated", "/run/stick/store-verity"],
            "",
        )
        .prints(
            Tool::Losetup,
            &["--find", "--show", "--read-only", "/run/stick/store"],
            "/dev/loop0\n",
        )
        .prints(
            Tool::Losetup,
            &["--find", "--show", "--read-only", "/run/stick/store-verity"],
            "/dev/loop1\n",
        )
        .file("/sys/class/block/loop0/dev", b"7:0\n")
        .file("/sys/class/block/loop1/dev", b"7:1\n");
    let swapped = STORE_TABLE.replace("8:3 8:2", "7:0 7:1");
    stand_in
        .prints(
            Tool::Dmsetup,
            &[
                "reload",
                "--readonly",
                "--noudevsync",
                "usr",
                "--table",
                &swapped,
            ],
            "",
        )
        .prints(Tool::Dmsetup, &["resume", "--noudevsync", "usr"], "");
    assert_eq!(
        RunFromMemory.observe(&mut stand_in).unwrap(),
        Standing::Needed { asks: vec![] },
        "it asks nothing, and runs as soon as the stick starts"
    );
    // Once the table is swapped, the kernel shows the loop devices.
    stand_in
        .prints(Tool::Dmsetup, &["table", "usr"], STORE_TABLE)
        .prints(Tool::Dmsetup, &["table", "usr"], &swapped);
    Journey::attempt(&RunFromMemory, &mut stand_in, &Answers::new()).unwrap();

    assert_eq!(stand_in.holds("/run/stick/store"), Some(&[3u8; 8192][..]));
    assert_eq!(
        stand_in.holds("/run/stick/store-verity"),
        Some(&[2u8; 8192][..])
    );
    assert!(position(&stand_in, "veritysetup verify") < position(&stand_in, "dmsetup reload"));
    assert!(position(&stand_in, "dmsetup reload") < position(&stand_in, "dmsetup resume"));
    assert!(
        memory::port_words(&mut stand_in).contains("take the stick out first"),
        "the key steps say the stick can come out"
    );
}

#[test]
fn a_copy_that_doesnt_match_the_sticks_hash_is_never_run_from() {
    let mut stand_in = StandIn::new();
    store_on_the_stick(&mut stand_in, 8_000_000);
    stand_in
        .prints(
            Tool::Mount,
            &[
                "-t",
                "tmpfs",
                "-o",
                "size=1024k,mode=0700,X-mount.mkdir=0700",
                "stick",
                "/run/stick",
            ],
            "",
        )
        .refuses(
            Tool::Veritysetup,
            VERIFY,
            1,
            "Verification failed at position 4096.",
        );
    let refused = RunFromMemory.run(&mut stand_in, &Answers::new());
    assert!(
        matches!(&refused, Err(Fault::Refused { said, .. }) if said.contains("Verification failed")),
        "{refused:?}"
    );
    assert!(
        !stand_in
            .calls
            .iter()
            .any(|call| call.starts_with("losetup") || call.starts_with("dmsetup reload")),
        "{:#?}",
        stand_in.calls
    );
    assert_eq!(memory::port_words(&mut stand_in), "");
}

#[test]
fn a_machine_without_the_memory_keeps_the_stick_in_and_says_so() {
    let mut stand_in = StandIn::new();
    // 16 KiB of copies and a gibibyte for the install, and not quite that.
    store_on_the_stick(&mut stand_in, (1 << 20) + 15);
    RunFromMemory.run(&mut stand_in, &Answers::new()).unwrap();
    assert!(
        !stand_in.calls.iter().any(|call| call.starts_with("mount")),
        "{:#?}",
        stand_in.calls
    );
    let Standing::Done { evidence } = RunFromMemory.observe(&mut stand_in).unwrap() else {
        panic!("nothing more to do")
    };
    assert!(evidence.contains("stays in"), "{evidence}");
    assert!(memory::port_words(&mut stand_in).contains("Keep the stick in"));
}

#[test]
fn a_system_not_running_from_a_sticks_store_has_nothing_to_copy() {
    let mut stand_in = StandIn::new();
    stand_in.refuses(
        Tool::Dmsetup,
        &["table", "usr"],
        1,
        "device-mapper: table ioctl on usr failed: No such device or address",
    );
    assert!(matches!(
        RunFromMemory.observe(&mut stand_in).unwrap(),
        Standing::Done { .. }
    ));
}

#[test]
fn a_key_step_says_the_stick_can_come_out_once_its_system_is_in_memory() {
    let target = target("/dev/disk/by-id/virtio-root");
    let scratch = Scratch::new("install-key-port");
    let mut stand_in = stick(&scratch, &target);
    stand_in.prints(
        Tool::Cryptsetup,
        &[
            "luksDump",
            "--dump-json-metadata",
            "/dev/disk/by-partlabel/system",
        ],
        fixture!("luks-dump-before-wipe.json"),
    );
    guide::Machine::journal(&mut stand_in)
        .record("run-from-memory", "in memory")
        .unwrap();
    let Standing::Needed { asks } = EnrolYubikey::new("the Nano")
        .observe(&mut stand_in)
        .unwrap()
    else {
        panic!("the Nano isn't enrolled yet")
    };
    let Ask::Act { what, .. } = &asks[0] else {
        panic!("{asks:?}")
    };
    assert_eq!(
        what,
        "Plug in the Nano, and only the Nano. If no port is free, take the stick out first: the guide asks for it back when it needs it. Don't touch it yet: once you've typed its PIN, it blinks twice, and each blink wants one touch."
    );
}
