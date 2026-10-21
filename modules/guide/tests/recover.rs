//! Finding the machine to recover, held against what lsblk printed in the
//! guide's VM (fixtures/qemu-canokey/): core's layout found on the
//! internal disk and never on the stick, and nothing found on a blank one.

use guide::Machine;
use guide::fake::Fake;
use guide::recover::{BootGeneration, FindMachine, already_there, the_machine};
use guide::{Answer, Answers, Ask, Journey, Standing, Step, Tool};

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

#[test]
fn a_machines_stick_asks_before_installing_over_a_machine() {
    let mut fake = lsblk(fixture!("lsblk-laid-out.json"));
    let question = already_there(&mut fake, "laptop")
        .unwrap()
        .expect("a question");
    assert!(question.contains("serial root"), "{question}");
    assert!(
        question.contains("install laptop on it again"),
        "{question}"
    );
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
