//! Finding the machine to recover, held against what lsblk printed in the
//! guide's VM (fixtures/qemu-canokey/): core's layout found on the
//! internal disk and never on the stick, and nothing found on a blank one.

use guide::Machine;
use guide::fake::Fake;
use guide::recover::{
    AlreadyThere, BootGeneration, FindMachine, OpenDisk, STOPPED, STOPPED_EARLY, already_there,
    the_machine,
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
