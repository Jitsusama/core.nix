//! Finding the machine to recover, held against what lsblk printed in the
//! guide's VM (fixtures/qemu-canokey/): core's layout found on the
//! internal disk and never on the stick, and nothing found on a blank one.

use guide::Machine;
use guide::fake::Fake;
use guide::recover::copy::{COPIED, CopyHome};
use guide::recover::{
    AlreadyThere, BootGeneration, CHECKED, CheckSystem, FindMachine, MountSystem, OpenDisk,
    STOPPED, STOPPED_EARLY, already_there, the_machine,
};
use guide::{Answer, Answers, Ask, Fault, Journey, Program, Secret, Standing, Step, Tool};

macro_rules! fixture {
    ($file:literal) => {
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/fixtures/qemu-canokey/",
            $file
        ))
    };
}

fn lsblk(json: &str) -> Fake {
    let mut fake = Fake::new();
    fake.prints(Tool::Lsblk, &["--json", "--output-all", "--bytes"], json);
    fake
}

const RECOVERY_KEY: &str = "seven words read off the phone to open it";

/// The machine's disk, whatever's typed tried first as its recovery key,
/// which takes it or doesn't, and this one key plugged in: a canokey, or
/// the XPS 14's virtual key, which can't be the person's.
fn shut_disk(recovery_takes_it: bool, key: &str) -> Fake {
    let mut fake = lsblk(fixture!("lsblk-laid-out.json"));
    let (status, said) = if recovery_takes_it {
        (0, "")
    } else {
        (2, "No key available with this passphrase.")
    };
    fake.answers(
        Program::Tool(Tool::Cryptsetup),
        &[
            "open",
            "--key-file=-",
            "/dev/disk/by-partlabel/system",
            "system",
        ],
        status,
        "",
        said,
    );
    let (listed, info) = match key {
        "linux-id" => (
            "/dev/hidraw6: vendor=0x15d9, product=0x0a37 ( linux-id)\n",
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/fixtures/dell-xps-14-da14260/fido2-token-I.txt"
            )),
        ),
        _ => (fixture!("fido2-token-L.txt"), fixture!("fido2-token-I.txt")),
    };
    let path = listed.split(':').next().unwrap().to_string();
    fake.prints(Tool::Fido2Token, &["-L"], listed)
        .prints(Tool::Fido2Token, &["-I", &path], info);
    fake
}

fn secret(typed: &str) -> Answers {
    let mut answers = Answers::new();
    answers.give("key", Answer::Done);
    answers.give("secret", Answer::Secret(Secret::new(typed.into())));
    answers
}

fn asked_the_key(fake: &Fake) -> bool {
    fake.calls.iter().any(|call| call.contains("--token-only"))
}

#[test]
fn the_recovery_key_typed_with_a_key_plugged_in_opens_the_disk_without_the_key() {
    let mut fake = shut_disk(true, "canokey");
    OpenDisk.run(&mut fake, &secret(RECOVERY_KEY)).unwrap();
    assert!(!asked_the_key(&fake), "{:#?}", fake.calls);
}

#[test]
fn a_laptops_virtual_key_is_never_asked_for_a_pin() {
    let mut fake = shut_disk(false, "linux-id");
    let refused = OpenDisk.run(&mut fake, &secret("123456"));
    assert!(
        matches!(&refused, Err(Fault::Refused { status: 2, .. })),
        "{refused:?}"
    );
    assert!(!asked_the_key(&fake), "{:#?}", fake.calls);
}

#[test]
fn the_machine_is_the_internal_disk_with_cores_layout() {
    let mut fake = lsblk(fixture!("lsblk-laid-out.json"));
    let found = the_machine(&mut fake)
        .unwrap()
        .map_err(|standing| format!("{standing:?}"))
        .unwrap();
    assert_eq!(
        (found.disk.path.as_str(), found.name.as_str()),
        ("/dev/vda", "system")
    );
    assert_eq!(found.volume(), "/dev/disk/by-partlabel/system");
}

const BOOT_MOUNT: &[&str] = &[
    "-o",
    "ro,X-mount.mkdir",
    "/dev/disk/by-partlabel/system-boot",
    "/run/guide/boot",
];
const BOOT_COPY: &str = "/run/guide/boot/guide/install-journal";

/// The laid-out disk, its boot partition mounting read-only and holding
/// this copy of an install's journal, or none.
fn laid_out_holding(copy: Option<&str>) -> Fake {
    let mut fake = lsblk(fixture!("lsblk-laid-out.json"));
    fake.prints(Tool::Mount, BOOT_MOUNT, "")
        .prints(Tool::Umount, &["/run/guide/boot"], "");
    match copy {
        Some(text) => fake.file(BOOT_COPY, text.as_bytes()),
        None => fake.no_file(BOOT_COPY),
    };
    fake
}

fn line(step: &str, fact: &str) -> String {
    format!("{{\"step\":\"{step}\",\"fact\":\"{fact}\",\"at\":1}}\n")
}

fn machine_question(there: Option<AlreadyThere>) -> String {
    match there {
        Some(AlreadyThere::Machine { question }) => question,
        other => panic!("asked about a machine, not {other:?}"),
    }
}

#[test]
fn a_machines_stick_asks_before_installing_over_a_machine() {
    let mut fake = laid_out_holding(None);
    let question = machine_question(already_there(&mut fake, "laptop").unwrap());
    assert!(question.contains("serial root"), "{question}");
    assert!(
        question.contains("install laptop on it again"),
        "{question}"
    );
    // The boot partition was let go again, having changed nothing.
    assert!(
        fake.calls
            .iter()
            .any(|call| call == "umount /run/guide/boot"),
        "{:#?}",
        fake.calls
    );
}

#[test]
fn an_install_of_this_machine_stopped_part_way_is_offered_to_carry_on() {
    let copy = [
        line("confirm-disk", "/dev/vda"),
        line("lay-out-disk", "laptop"),
        line("enrol-nano", "opened:abc"),
    ]
    .concat();
    let mut fake = laid_out_holding(Some(&copy));
    let Some(AlreadyThere::Stopped {
        question,
        disk,
        facts,
    }) = already_there(&mut fake, "laptop").unwrap()
    else {
        panic!("a stopped install")
    };
    assert!(
        question.contains("The install of laptop onto") && question.contains("serial root"),
        "{question}"
    );
    assert_eq!(disk, "/dev/vda");
    assert!(facts.contains(&("enrol-nano".into(), "opened:abc".into())));
    assert_eq!(STOPPED, &["Carry it on", "Start again, wiping the disk"]);
}

#[test]
fn an_install_stopped_before_any_key_was_enrolled_is_offered_only_to_start_again() {
    let copy = [
        line("confirm-disk", "/dev/vda"),
        line("lay-out-disk", "laptop"),
    ]
    .concat();
    let mut fake = laid_out_holding(Some(&copy));
    let Some(AlreadyThere::StoppedEarly { question }) = already_there(&mut fake, "laptop").unwrap()
    else {
        panic!("a stopped install nothing can open")
    };
    assert!(
        question.contains("The install of laptop onto")
            && question.contains("serial root")
            && question.contains("before a YubiKey was enrolled"),
        "{question}"
    );
    assert_eq!(STOPPED_EARLY, &["Start again, wiping the disk"]);
}

#[test]
fn an_install_stopped_for_another_machine_is_a_machine_to_install_over() {
    let copy = line("lay-out-disk", "desktop");
    let mut fake = laid_out_holding(Some(&copy));
    machine_question(already_there(&mut fake, "laptop").unwrap());
}

#[test]
fn a_boot_partition_that_wont_mount_is_a_machine_to_install_over() {
    let mut fake = lsblk(fixture!("lsblk-laid-out.json"));
    fake.refuses(Tool::Mount, BOOT_MOUNT, 32, "wrong fs type");
    machine_question(already_there(&mut fake, "laptop").unwrap());
}

#[test]
fn a_blank_disk_is_installed_on_without_asking() {
    let mut fake = lsblk(fixture!("lsblk-blank.json"));
    assert_eq!(already_there(&mut fake, "laptop").unwrap(), None);
}

#[test]
fn an_install_confirmed_this_boot_is_not_asked_about_again() {
    let mut fake = lsblk(fixture!("lsblk-laid-out.json"));
    fake.journal().record("confirm-disk", "/dev/vda").unwrap();
    assert_eq!(already_there(&mut fake, "laptop").unwrap(), None);
}

#[test]
fn a_blank_disk_holds_no_machine() {
    let mut fake = lsblk(fixture!("lsblk-blank.json"));
    assert!(matches!(
        FindMachine.observe(&mut fake).unwrap(),
        Standing::Blocked { .. }
    ));
}

#[test]
fn the_machine_is_shown_before_anything_is_asked() {
    let mut fake = lsblk(fixture!("lsblk-laid-out.json"));
    let Standing::Needed { asks } = FindMachine.observe(&mut fake).unwrap() else {
        panic!("the machine is still to be shown")
    };
    assert!(
        format!("{asks:?}").contains("serial root holds the machine"),
        "{asks:?}"
    );
}

const LIST: &[&str] = &["--esp-path=/mnt/boot", "list", "--json=short"];
const DEFAULT: &str =
    "/sys/firmware/efi/efivars/LoaderEntryDefault-4a67b082-0a4c-41cf-b6c7-440b29bb8c4f";

/// The Surface's entries, as bootctl listed them, its default moved to
/// another, as set-default leaves them.
fn defaulting(id: &str) -> String {
    let surface = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/fixtures/microsoft-surface-laptop-4-1958/bootctl-list.json"
    ));
    let mut entries: serde_json::Value = serde_json::from_str(surface).unwrap();
    for entry in entries.as_array_mut().unwrap() {
        entry["isDefault"] = (entry["id"] == id).into();
    }
    entries.to_string()
}

#[test]
fn a_generation_chosen_boots_next_and_nothing_is_removed() {
    let surface = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/fixtures/microsoft-surface-laptop-4-1958/bootctl-list.json"
    ));
    let chosen = "nixos-generation-14-6nxexgjjuzafv5ivu5pnki6sdotgw7kicz2l2pt3ebpfh2crqqjq.efi";
    let mut fake = Fake::new();
    // As observed here, then before running, then as run reads it, then after.
    fake.prints(Tool::Bootctl, LIST, surface)
        .prints(Tool::Bootctl, LIST, surface)
        .prints(Tool::Bootctl, LIST, surface)
        .prints(Tool::Bootctl, LIST, &defaulting(chosen))
        .no_file(DEFAULT);

    let Standing::Needed { asks } = BootGeneration.observe(&mut fake).unwrap() else {
        panic!("a generation is still to choose")
    };
    let [Ask::Choose { choices, .. }] = asks.as_slice() else {
        panic!("{asks:?}")
    };
    // Newest first, the one it boots now said so, the firmware's own entry
    // left out.
    assert!(
        choices[0].contains("Generation 15") && choices[0].ends_with(", which it boots now"),
        "{choices:?}"
    );
    assert!(choices[1].contains("Generation 14"), "{choices:?}");
    assert_eq!(choices.len(), 18, "{choices:?}");

    let mut answers = Answers::new();
    answers.give("generation", Answer::Chosen(1));
    Journey::attempt(&BootGeneration, &mut fake, &answers).unwrap();
    // As bootctl set-default writes it: the attributes, then the id in
    // UTF-16 with its terminator.
    let mut expected = vec![0x07, 0, 0, 0];
    expected.extend(chosen.encode_utf16().chain([0]).flat_map(u16::to_le_bytes));
    assert_eq!(fake.holds(DEFAULT), Some(expected.as_slice()));
    assert!(
        !fake
            .calls
            .iter()
            .any(|call| call.contains("unlink") || call.contains("remove")),
        "{:#?}",
        fake.calls
    );
}

const CHECK: &[&str] = &["check", "--readonly", "/dev/mapper/system"];

/// The opened machine, nothing of it mounted yet, as a stick that's just
/// opened it finds it (the fixture was read after an install, still
/// mounted), its filesystem checked with the status and words `btrfs
/// check` gives, and every mount it's given taken.
fn checked(status: i32, said: &str) -> Fake {
    let unmounted = ["boot", "var/log", "swap", "nix", "home", ""].iter().fold(
        fixture!("lsblk-laid-out.json").to_string(),
        |json, at| {
            let at = format!("\"/mnt/{at}\"").replace("/\"", "\"");
            json.replace(&at, "null")
        },
    );
    assert!(!unmounted.contains("/mnt"), "{unmounted}");
    let mut fake = lsblk(&unmounted);
    fake.no_file(CHECKED)
        .answers(Program::Tool(Tool::Btrfs), CHECK, status, "", said);
    for (subvolume, at) in [
        ("@", ""),
        ("@nix", "/nix"),
        ("@home", "/home"),
        ("@log", "/var/log"),
    ] {
        for mode in ["compress=zstd:1,noatime,discard=async", "ro,rescue=all"] {
            let options = format!("subvol={subvolume},{mode},X-mount.mkdir");
            fake.prints(
                Tool::Mount,
                &["-o", &options, "/dev/mapper/system", &format!("/mnt{at}")],
                "",
            );
        }
    }
    fake.prints(
        Tool::Mount,
        &[
            "-o",
            "umask=0077,X-mount.mkdir",
            "/dev/disk/by-partlabel/system-boot",
            "/mnt/boot",
        ],
        "",
    );
    fake
}

fn mounts(fake: &Fake) -> Vec<&String> {
    fake.calls
        .iter()
        .filter(|call| call.contains("subvol="))
        .collect()
}

#[test]
fn a_filesystem_is_checked_before_anything_mounts_it() {
    let mut fake = checked(0, "");
    let standing = CheckSystem.observe(&mut fake);
    assert!(
        matches!(standing, Ok(Standing::Needed { .. })),
        "{standing:?}"
    );
    Journey::attempt(&CheckSystem, &mut fake, &Answers::new()).unwrap();
    assert_eq!(fake.holds(CHECKED), Some(b"clean".as_slice()));
    let Standing::Done { evidence } = CheckSystem.observe(&mut fake).unwrap() else {
        panic!("not done")
    };
    assert_eq!(evidence, "its filesystem has no errors");

    MountSystem.run(&mut fake, &Answers::new()).unwrap();
    let mounts = mounts(&fake);
    assert_eq!(mounts.len(), 4, "{mounts:#?}");
    assert!(
        mounts.iter().all(|call| call.contains("compress=zstd:1")),
        "{mounts:#?}"
    );
}

#[test]
fn a_filesystem_with_errors_is_mounted_only_to_read() {
    let said = "ERROR: errors found in fs roots";
    let mut fake = checked(1, said);
    Journey::attempt(&CheckSystem, &mut fake, &Answers::new()).unwrap();
    assert_eq!(fake.holds(CHECKED), Some(said.as_bytes()));
    let Standing::Done { evidence } = CheckSystem.observe(&mut fake).unwrap() else {
        panic!("not done")
    };
    assert!(
        evidence.contains("mounted only to read") && evidence.contains(CHECKED),
        "{evidence}"
    );

    MountSystem.run(&mut fake, &Answers::new()).unwrap();
    let mounts = mounts(&fake);
    assert_eq!(mounts.len(), 4, "{mounts:#?}");
    assert!(
        mounts
            .iter()
            .all(|call| call.contains(",ro,rescue=all,") && !call.contains("compress")),
        "{mounts:#?}"
    );
}

/// A USB drive as lsblk lists it, its one partition holding `filesystem`.
fn usb_drive(name: &str, filesystem: Option<&str>) -> serde_json::Value {
    serde_json::json!({
        "name": name, "path": format!("/dev/{name}"), "type": "disk",
        "model": "Flash Drive", "serial": "0123", "size": 64_000_000_000u64,
        "tran": "usb", "rm": true, "mountpoints": [null],
        "children": [{
            "name": format!("{name}1"), "path": format!("/dev/{name}1"), "type": "part",
            "size": 63_000_000_000u64, "tran": null, "rm": true, "partn": 1,
            "fstype": filesystem, "mountpoints": [null]
        }]
    })
}

const HOME_FOLDERS: &str = "machine home folders";

/// The VM's opened and mounted machine, its repair stick plugged in,
/// beside these drives, the machine naming itself `machine`.
fn with_drives(drives: &[serde_json::Value]) -> Fake {
    let mut listing: serde_json::Value =
        serde_json::from_str(fixture!("lsblk-laid-out.json")).unwrap();
    listing["blockdevices"]
        .as_array_mut()
        .unwrap()
        .extend(drives.iter().cloned());
    let mut fake = lsblk(&listing.to_string());
    fake.no_file(COPIED).file("/mnt/etc/hostname", b"machine\n");
    fake
}

const DU: &[&str] = &["--summarize", "--bytes", "--exclude=.cache", "/mnt/home/"];
const DF: &[&str] = &["--output=avail", "--block-size=1", "/run/guide/copy"];
const RSYNC: &[&str] = &[
    "--recursive",
    "--times",
    "--modify-window=2",
    "--exclude=.cache/",
    "/mnt/home/",
    "/run/guide/copy/machine home folders/",
];

/// A drive with `free` bytes left, chosen for home folders of `needed`
/// bytes, which rsync copies finishing with `status`.
fn copying(needed: u64, free: u64, status: i32, said: &str) -> Fake {
    let mut fake = with_drives(&[usb_drive("sdb", Some("exfat"))]);
    fake.prints(
        Tool::Mount,
        &["-o", "X-mount.mkdir", "/dev/sdb1", "/run/guide/copy"],
        "",
    )
    .answers(
        Program::Tool(Tool::Du),
        DU,
        1,
        &format!("{needed}\t/mnt/home/\n"),
        "du: cannot read directory '/mnt/home/alice/broken': Input/output error",
    )
    .prints(Tool::Df, DF, &format!("Avail\n{free}\n"))
    .answers(Program::Tool(Tool::Rsync), RSYNC, status, "", said)
    .prints(Tool::Umount, &["/run/guide/copy"], "");
    fake
}

fn chosen(at: usize) -> Answers {
    let mut answers = Answers::new();
    answers.give("drive", Answer::Chosen(at));
    answers
}

#[test]
fn a_drive_is_chosen_from_those_plugged_in_never_the_stick() {
    let mut fake = with_drives(&[
        usb_drive("sdb", Some("exfat")),
        usb_drive("sdc", None),
        usb_drive("sdd", Some("crypto_LUKS")),
    ]);
    let Ok(Standing::Needed { asks }) = CopyHome.observe(&mut fake) else {
        panic!("not asked")
    };
    let [
        Ask::Choose {
            question, choices, ..
        },
    ] = asks.as_slice()
    else {
        panic!("{asks:?}")
    };
    // The stick's partitions are FAT too, and never offered.
    assert_eq!(
        choices,
        &["Flash Drive, 64 GB, serial 0123, its exfat partition"]
    );
    assert!(question.contains(HOME_FOLDERS), "{question}");
}

#[test]
fn with_no_drive_plugged_in_it_says_to_plug_one_in() {
    let mut fake = with_drives(&[]);
    let standing = CopyHome.observe(&mut fake);
    assert!(
        matches!(&standing, Ok(Standing::Blocked { remedy, .. }) if remedy.contains("plug in a USB drive")),
        "{standing:?}"
    );
}

#[test]
fn the_home_folders_are_copied_leaving_caches_and_the_drive_unmounted() {
    let mut fake = copying(3_000_000_000, 50_000_000_000, 0, "");
    CopyHome.run(&mut fake, &chosen(0)).unwrap();
    let order: Vec<&str> = fake
        .calls
        .iter()
        .filter_map(|call| call.split_whitespace().next())
        .filter(|program| program.ends_with("mount") || ["du", "df", "rsync"].contains(program))
        .collect();
    assert_eq!(
        order,
        ["mount", "du", "df", "rsync", "umount"],
        "{:#?}",
        fake.calls
    );
    let Ok(Standing::Done { evidence }) = CopyHome.observe(&mut fake) else {
        panic!("not done")
    };
    assert!(
        evidence.contains(HOME_FOLDERS) && evidence.contains("can come out"),
        "{evidence}"
    );
    assert!(!evidence.contains("couldn't be read"), "{evidence}");
}

#[test]
fn a_drive_too_small_copies_nothing() {
    let mut fake = copying(5_000_000_000, 1_000_000_000, 0, "");
    let refused = CopyHome.run(&mut fake, &chosen(0));
    assert!(
        matches!(&refused, Err(Fault::Blocked { why, .. }) if why.contains("need 5 GB") && why.contains("1 GB free")),
        "{refused:?}"
    );
    assert!(
        !fake.calls.iter().any(|call| call.starts_with("rsync")),
        "{:#?}",
        fake.calls
    );
    assert!(fake.calls.iter().any(|call| call.starts_with("umount")));
    assert_eq!(fake.holds(COPIED), None);
}

#[test]
fn what_rsync_could_not_read_is_kept_beside_the_copy() {
    let said =
        "rsync: [sender] read errors mapping \"/mnt/home/alice/notes\": Input/output error (5)";
    let mut fake = copying(3_000_000_000, 50_000_000_000, 23, said);
    CopyHome.run(&mut fake, &chosen(0)).unwrap();
    assert_eq!(
        fake.holds("/run/guide/copy/machine home folders, not copied.txt"),
        Some(said.as_bytes())
    );
    let evidence = String::from_utf8_lossy(fake.holds(COPIED).unwrap()).into_owned();
    assert!(evidence.contains("couldn't be read"), "{evidence}");
}

#[test]
fn a_copy_rsync_gives_up_on_is_no_copy() {
    let mut fake = copying(
        3_000_000_000,
        50_000_000_000,
        11,
        "rsync: write failed: No space left on device (28)",
    );
    let refused = CopyHome.run(&mut fake, &chosen(0));
    assert!(refused.is_err(), "{refused:?}");
    assert!(fake.calls.iter().any(|call| call.starts_with("umount")));
    assert_eq!(fake.holds(COPIED), None);
}
