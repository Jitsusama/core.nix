//! Finishing a machine on itself, held against what its firmware and its
//! disk said: Secure Boot checked before anything is sealed to it, the TPM
//! sealed only once both PINs match, every privileged act through run0,
//! no PIN ever an argument, and the seal done only once its slot is tried
//! first and proven to open.

use guide::fake::Fake;
use guide::installed::{CheckSecureBoot, FACTS, PCRLOCK, SealTpm};
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

/// tpm2_getcap's arguments for the TPM's NV indices.
const NV: &[&str] = &["-T", "device:/dev/tpmrm0", "handles-nv-index"];

/// The XPS 14's NV indices, its pcrlock policy's among them.
fn held() -> &'static str {
    std::str::from_utf8(fixture!("dell-xps-14-da14260", "tpm2-handles-nv-index.txt")).unwrap()
}

/// The same once the TPM was cleared: its owner's indices gone, the
/// policy's with them, and the platform's left.
fn cleared() -> String {
    held()
        .lines()
        .filter(|line| !line.starts_with("- 0x1ACCCE4"))
        .map(|line| format!("{line}\n"))
        .collect()
}

fn machine() -> Fake {
    let mut fake = Fake::new();
    fake.file(FACTS, br#"{"disk_name":"system"}"#);
    fake.file(
        PCRLOCK,
        fixture!("dell-xps-14-da14260", "pcrlock-nvindex.json"),
    );
    fake
}

/// systemctl's arguments for how lanzaboote's policy unit last ended.
const MADE: &[&str] = &[
    "show",
    "--property=Result",
    "--value",
    "systemd-pcrlock-make-policy.service",
];

/// The machine, its TPM holding these NV indices, and lanzaboote's policy
/// unit having ended this way this boot: `success`, as it does on the XPS
/// 14, or `exit-code`, as when the policy can't follow the boot.
fn holding_after(indices: &str, made: &str) -> Fake {
    let mut fake = machine();
    fake.prints(Tool::Tpm2Getcap, NV, indices);
    fake.prints(Tool::Systemctl, MADE, &format!("{made}\n"));
    fake
}

fn holding(indices: &str) -> Fake {
    holding_after(indices, "success")
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
    let mut fake = holding(held());
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
        // Reading how a unit ended is anyone's to do.
        .filter(|call| !call.starts_with("reading") && !call.starts_with("systemctl show"))
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

#[test]
fn a_seal_proven_once_holds_while_the_tpm_keeps_its_policy() {
    let mut fake = holding(held());
    fake.prints(
        Tool::Cryptsetup,
        DUMP,
        std::str::from_utf8(fixture!("qemu-canokey", "luks-dump-sealed.json")).unwrap(),
    );
    use guide::Machine;
    fake.journal().record("seal-tpm", "opened:0").unwrap();
    assert!(matches!(
        SealTpm.observe(&mut fake).unwrap(),
        Standing::Done { .. }
    ));
}

#[test]
fn a_cleared_tpm_undoes_a_seal_proven_before() {
    // lanzaboote's unit still ends well after a clear, as vm-guide-installs
    // shows: its prediction is unchanged, so it never writes the index it
    // would find gone. Only the TPM's own list tells.
    let mut fake = holding_after(&cleared(), "success");
    fake.prints(
        Tool::Cryptsetup,
        DUMP,
        std::str::from_utf8(fixture!("qemu-canokey", "luks-dump-sealed.json")).unwrap(),
    );
    use guide::Machine;
    fake.journal().record("seal-tpm", "opened:0").unwrap();
    assert!(matches!(
        SealTpm.observe(&mut fake).unwrap(),
        Standing::Needed { .. }
    ));
}

#[test]
fn a_firmware_update_undoes_a_seal_proven_before() {
    // The TPM still holds the policy, which no longer follows what this
    // boot's firmware measured, so lanzaboote's unit couldn't remake it.
    let mut fake = holding_after(held(), "exit-code");
    fake.prints(
        Tool::Cryptsetup,
        DUMP,
        std::str::from_utf8(fixture!("qemu-canokey", "luks-dump-sealed.json")).unwrap(),
    );
    use guide::Machine;
    fake.journal().record("seal-tpm", "opened:0").unwrap();
    assert!(matches!(
        SealTpm.observe(&mut fake).unwrap(),
        Standing::Needed { .. }
    ));
}

#[test]
fn a_cleared_tpm_gets_its_policy_made_again_before_the_disk_is_sealed_to_it() {
    // Cleared when the step looks, then held once lanzaboote's unit made
    // the policy again, from then on.
    let mut fake = machine();
    fake.prints(Tool::Tpm2Getcap, NV, &cleared());
    fake.prints(Tool::Tpm2Getcap, NV, held());
    // A TPM without the index needs no more asked of it, so the unit is
    // first asked once it made the policy again.
    fake.prints(Tool::Systemctl, MADE, "success\n");
    let sealed = std::str::from_utf8(fixture!("qemu-canokey", "luks-dump-sealed.json")).unwrap();
    fake.prints(Tool::Cryptsetup, DUMP, sealed);
    // As pcrlock answers when the index it would remove is gone.
    fake.answers(
        Program::Tool(Tool::Pcrlock),
        &["remove-policy"],
        1,
        "",
        "Failed to remove NV index, assuming data out of date, removing policy file.",
    );
    fake.prints(
        Tool::Systemctl,
        &["restart", "systemd-pcrlock-make-policy.service"],
        "",
    );
    fake.answers(
        Program::Tool(Tool::SystemdCryptenroll),
        &[
            "--unlock-fido2-device=auto",
            "--tpm2-device=auto",
            "--tpm2-with-pin=yes",
            "--tpm2-pcrlock=/var/lib/systemd/pcrlock.json",
            "--wipe-slot=tpm2",
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
    Journey::attempt(&SealTpm, &mut fake, &answers("24681357", "24681357")).unwrap();

    let at = |start: &str| {
        fake.calls
            .iter()
            .position(|call| call.starts_with(start))
            .unwrap_or_else(|| panic!("no {start} in {:#?}", fake.calls))
    };
    assert!(
        at("systemd-pcrlock remove-policy")
            < at("systemctl restart systemd-pcrlock-make-policy.service")
    );
    assert!(
        at("systemctl restart") < at("systemd-cryptenroll"),
        "{:#?}",
        fake.calls
    );
    let renewing: Vec<&String> = fake
        .calls
        .iter()
        .filter(|call| call.starts_with("systemd-pcrlock") || call.starts_with("systemctl restart"))
        .collect();
    assert!(
        renewing.iter().all(|call| call.ends_with("(as root)")),
        "{renewing:#?}"
    );
}

/// A bundle as an install seals one, its keys stand-ins.
fn bundle(db: &str) -> Vec<u8> {
    use guide::rescue::{Bundle, KEYS, Keys, RECOVERY_KEY};
    let files = KEYS
        .iter()
        .map(|name| {
            let text = if *name == "keys/db/db.pem" { db } else { name };
            (
                name.to_string(),
                zeroize::Zeroizing::new(text.as_bytes().to_vec()),
            )
        })
        .collect();
    let target = serde_json::from_str(
        r#"{"machine":"laptop","disk":"/dev/x","disk_name":"system","install_key":"/run/guide/install-key","layout_packages":[],"system":"/nix/store/abc","account":"alice","yubikeys":[],"recipients":[],"start_from_stick":null,"clear_secure_boot_keys":null}"#,
    )
    .unwrap();
    let recovery = Secret::new("a recovery key of the disk".into());
    Bundle::seal(
        &target,
        &Keys::new(files).unwrap(),
        &recovery,
        RECOVERY_KEY,
        Some(10),
    )
    .unwrap()
}

#[test]
fn the_installs_rescue_bundle_is_copied_home_and_kept_once_for_each_bundle() {
    use guide::installed::rescue::{CopyRescueBundle, KeepRescueBundle};
    let home = std::path::PathBuf::from("/home/alice");
    let copy = "/home/alice/laptop-rescue-bundle.json";
    let made = bundle("the db certificate");
    let mut fake = Fake::new();
    fake.file(guide::rescue::KEPT, &made).no_file(copy);
    let (copying, keeping) = (
        CopyRescueBundle { home: home.clone() },
        KeepRescueBundle { home: home.clone() },
    );
    let journey = Journey::new(vec![
        Box::new(CopyRescueBundle { home: home.clone() }),
        Box::new(KeepRescueBundle { home: home.clone() }),
    ])
    .unwrap();
    assert_eq!(journey.steps().count(), 2);

    assert!(matches!(
        copying.observe(&mut fake).unwrap(),
        Standing::Needed { .. }
    ));
    copying.run(&mut fake, &Answers::new()).unwrap();
    assert_eq!(fake.holds(copy), Some(&made[..]));
    assert!(matches!(
        copying.observe(&mut fake).unwrap(),
        Standing::Done { .. }
    ));
    // Read as the person: nothing went through run0.
    assert!(
        !fake.calls.iter().any(|call| call.ends_with("(as root)")),
        "{:#?}",
        fake.calls
    );

    let standing = keeping.observe(&mut fake).unwrap();
    assert!(
        matches!(&standing, Standing::Needed { asks } if matches!(asks.as_slice(), [guide::Ask::Act { what, .. }] if what.contains(copy) && what.contains("1Password"))),
        "{standing:?}"
    );
    let mut answers = Answers::new();
    answers.give("kept", Answer::Done);
    keeping.run(&mut fake, &answers).unwrap();
    assert!(matches!(
        keeping.observe(&mut fake).unwrap(),
        Standing::Done { .. }
    ));

    // A reinstall's new bundle is copied and kept again.
    fake.file(guide::rescue::KEPT, &bundle("another db certificate"));
    assert!(matches!(
        copying.observe(&mut fake).unwrap(),
        Standing::Needed { .. }
    ));
    assert!(matches!(
        keeping.observe(&mut fake).unwrap(),
        Standing::Needed { .. }
    ));
}

#[test]
fn a_machine_with_no_rescue_bundle_has_nothing_to_hand_on() {
    use guide::installed::rescue::{CopyRescueBundle, KeepRescueBundle};
    let home = std::path::PathBuf::from("/home/alice");
    let mut fake = Fake::new();
    fake.no_file(guide::rescue::KEPT);
    for step in [
        &CopyRescueBundle { home: home.clone() } as &dyn Step,
        &KeepRescueBundle { home: home.clone() },
    ] {
        let standing = step.observe(&mut fake).unwrap();
        assert!(
            matches!(&standing, Standing::Done { evidence } if evidence.contains("seal-keys")),
            "{standing:?}"
        );
    }
}
