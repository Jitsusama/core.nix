//! Finding the machine to recover, held against what lsblk printed in the
//! guide's VM (fixtures/qemu-canokey/): core's layout found on the
//! internal disk and never on the stick, and nothing found on a blank one.

use guide::fake::Fake;
use guide::recover::{FindMachine, the_machine};
use guide::{Standing, Step, Tool};

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
