//! Finishing a machine on itself, held against what its firmware and its
//! disk said: Secure Boot checked before anything is sealed to it, the TPM
//! sealed only once both PINs match, every privileged act through run0,
//! no PIN ever an argument, and the seal done only once its slot is tried
//! first and proven to open.

use guide::fake::Fake;
use guide::installed::{CheckSecureBoot, FACTS, SealTpm};
use guide::{Answer, Answers, Fault, Journey, Program, Secret, Standing, Step, Tool};

const GLOBAL: &str = "8be4df61-93ca-11d2-aa0d-00e098032b8c";
const DUMP: &[&str] = &[
    "luksDump",
    "--dump-json-metadata",
    "/dev/disk/by-partlabel/system",
];

macro_rules! fixture {
    ($machine:literal, $file:literal) => {
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/fixtures/",
            $machine,
            "/",
            $file
        ))
    };
}

fn efivars(fake: &mut Fake, machine: &str, names: &[(&str, &[u8])]) {
    for name in ["SecureBoot", "SetupMode", "AuditMode", "DeployedMode"] {
        let path = format!("/sys/firmware/efi/efivars/{name}-{GLOBAL}");
        match names.iter().find(|(found, _)| *found == name) {
            Some((_, bytes)) => fake.file(&path, bytes),
            None => fake.no_file(&path),
        };
    }
    let _ = machine;
}

#[test]
fn secure_boot_enforcing_with_the_machines_own_keys_is_done() {
    let mut xps = Fake::new();
    efivars(
        &mut xps,
        "xps",
        &[
            (
                "SecureBoot",
                fixture!(
                    "dell-xps-14-da14260",
                    "efivar-SecureBoot-8be4df61-93ca-11d2-aa0d-00e098032b8c"
                ),
            ),
            (
                "SetupMode",
                fixture!(
                    "dell-xps-14-da14260",
                    "efivar-SetupMode-8be4df61-93ca-11d2-aa0d-00e098032b8c"
                ),
            ),
            (
                "AuditMode",
                fixture!(
                    "dell-xps-14-da14260",
                    "efivar-AuditMode-8be4df61-93ca-11d2-aa0d-00e098032b8c"
                ),
            ),
            (
                "DeployedMode",
                fixture!(
                    "dell-xps-14-da14260",
                    "efivar-DeployedMode-8be4df61-93ca-11d2-aa0d-00e098032b8c"
                ),
            ),
        ],
    );
    assert!(matches!(
        CheckSecureBoot.observe(&mut xps).unwrap(),
        Standing::Done { .. }
    ));

    let mut surface = Fake::new();
    efivars(
        &mut surface,
        "surface",
        &[
            (
                "SecureBoot",
                fixture!(
                    "microsoft-surface-laptop-4-1958",
                    "efivar-SecureBoot-8be4df61-93ca-11d2-aa0d-00e098032b8c"
                ),
            ),
            (
                "SetupMode",
                fixture!(
                    "microsoft-surface-laptop-4-1958",
                    "efivar-SetupMode-8be4df61-93ca-11d2-aa0d-00e098032b8c"
                ),
            ),
        ],
    );
    assert!(matches!(
        CheckSecureBoot.observe(&mut surface).unwrap(),
        Standing::Done { .. }
    ));
}

#[test]
fn secure_boot_switched_off_blocks_the_seal() {
    // The Surface's own variables, Secure Boot's value turned to 0.
    let mut off = fixture!(
        "microsoft-surface-laptop-4-1958",
        "efivar-SecureBoot-8be4df61-93ca-11d2-aa0d-00e098032b8c"
    )
    .to_vec();
    *off.last_mut().unwrap() = 0;
    let mut fake = Fake::new();
    efivars(
        &mut fake,
        "surface",
        &[
            ("SecureBoot", &off),
            (
                "SetupMode",
                fixture!(
                    "microsoft-surface-laptop-4-1958",
                    "efivar-SetupMode-8be4df61-93ca-11d2-aa0d-00e098032b8c"
                ),
            ),
        ],
    );
    assert!(matches!(
        CheckSecureBoot.observe(&mut fake).unwrap(),
        Standing::Blocked { .. }
    ));
}

fn machine() -> Fake {
    let mut fake = Fake::new();
    fake.file(FACTS, br#"{"disk_name":"system"}"#);
    fake
}

fn answers(pin: &str, again: &str) -> Answers {
    let mut answers = Answers::new();
    answers.give("tpm-pin", Answer::Secret(Secret::new(pin.into())));
    answers.give("again", Answer::Secret(Secret::new(again.into())));
    answers.give("key", Answer::Done);
    answers.give("key-pin", Answer::Secret(Secret::new("123456".into())));
    answers
}

#[test]
fn the_tpm_is_sealed_through_run0_its_slot_preferred_and_proven() {
    let mut fake = machine();
    let installed =
        std::str::from_utf8(fixture!("qemu-canokey", "luks-dump-installed.json")).unwrap();
    let sealed = std::str::from_utf8(fixture!("qemu-canokey", "luks-dump-sealed.json")).unwrap();
    // Unsealed as observed here, as the journey observes before running and
    // as the step reads it before enrolling; sealed after, and from then on.
    fake.prints(Tool::Cryptsetup, DUMP, installed)
        .prints(Tool::Cryptsetup, DUMP, installed)
        .prints(Tool::Cryptsetup, DUMP, installed)
        .prints(Tool::Cryptsetup, DUMP, sealed);
    fake.answers(
        Program::Tool(Tool::SystemdCryptenroll),
        &[
            "--unlock-fido2-device=auto",
            "--tpm2-device=auto",
            "--tpm2-with-pin=yes",
            "--tpm2-pcrlock=/var/lib/systemd/pcrlock.json",
            "/dev/disk/by-partlabel/system",
        ],
        0,
        "",
        "",
    );
    fake.prints(
        Tool::Cryptsetup,
        &[
            "config",
            "/dev/disk/by-partlabel/system",
            "--key-slot",
            "0",
            "--priority",
            "prefer",
        ],
        "",
    );
    fake.prints(
        Tool::Cryptsetup,
        &[
            "open",
            "--test-passphrase",
            "--token-only",
            "--token-type",
            "systemd-tpm2",
            "/dev/disk/by-partlabel/system",
        ],
        "",
    );
    let Standing::Needed { asks } = SealTpm.observe(&mut fake).unwrap() else {
        panic!("the TPM is still to seal")
    };
    assert_eq!(asks.len(), 4, "{asks:?}");
    Journey::attempt(&SealTpm, &mut fake, &answers("24681357", "24681357")).unwrap();

    let privileged: Vec<&String> = fake
        .calls
        .iter()
        .filter(|call| !call.starts_with("reading"))
        .collect();
    assert!(
        privileged.iter().all(|call| call.ends_with("(as root)")),
        "{privileged:#?}"
    );
    let enrolled = fake
        .calls
        .iter()
        .find(|call| call.starts_with("systemd-cryptenroll"))
        .unwrap();
    assert!(
        enrolled.contains("[cryptsetup.fido2-pin cryptenroll.fido2-pin cryptenroll.new-tpm2-pin]"),
        "{enrolled}"
    );
    assert!(
        fake.calls
            .iter()
            .all(|call| !call.contains("24681357") && !call.contains("123456")),
        "{:#?}",
        fake.calls
    );
}

#[test]
fn pins_that_differ_seal_nothing() {
    let mut fake = machine();
    let refused = SealTpm.run(&mut fake, &answers("24681357", "24681358"));
    assert!(
        matches!(&refused, Err(Fault::Blocked { why, .. }) if why.contains("weren't the same")),
        "{refused:?}"
    );
    assert!(fake.calls.is_empty(), "{:#?}", fake.calls);
}

#[test]
fn a_tpm_slot_not_tried_first_is_still_to_do() {
    let mut fake = machine();
    // Proven to open, but no longer preferred: the real header, its
    // priority taken off.
    let mut header: serde_json::Value =
        serde_json::from_slice(fixture!("qemu-canokey", "luks-dump-sealed.json")).unwrap();
    header["keyslots"]["0"]
        .as_object_mut()
        .unwrap()
        .remove("priority");
    fake.prints(Tool::Cryptsetup, DUMP, &header.to_string());
    use guide::Machine;
    fake.journal().record("seal-tpm", "opened:0").unwrap();
    assert!(matches!(
        SealTpm.observe(&mut fake).unwrap(),
        Standing::Needed { .. }
    ));
}

#[test]
fn a_tpm_slot_nothing_proved_is_still_to_do() {
    let mut fake = machine();
    // Sealed and preferred, but this guide never saw it open.
    fake.prints(
        Tool::Cryptsetup,
        DUMP,
        std::str::from_utf8(fixture!("qemu-canokey", "luks-dump-sealed.json")).unwrap(),
    );
    assert!(matches!(
        SealTpm.observe(&mut fake).unwrap(),
        Standing::Needed { .. }
    ));
}
