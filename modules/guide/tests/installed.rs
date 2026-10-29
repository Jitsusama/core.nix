//! Finishing a machine on itself, held against what its firmware and its
//! disk said: Secure Boot checked before anything is sealed to it, the TPM
//! sealed only once both PINs match, every privileged act through run0,
//! no PIN ever an argument, and the seal done only once its slot is tried
//! first and proven to open.

use guide::installed::{CheckSecureBoot, FACTS, PCRLOCK, SealTpm};
use guide::stand_in::StandIn;
use guide::{Answer, Answers, Ask, Fault, Journey, Machine, Program, Secret, Standing, Step, Tool};
use std::fmt::Write as _;

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

fn efivars(stand_in: &mut StandIn, machine: &str, names: &[(&str, &[u8])]) {
    for name in ["SecureBoot", "SetupMode", "AuditMode", "DeployedMode"] {
        let path = format!("/sys/firmware/efi/efivars/{name}-{GLOBAL}");
        match names.iter().find(|(found, _)| *found == name) {
            Some((_, bytes)) => stand_in.file(&path, bytes),
            None => stand_in.no_file(&path),
        };
    }
    let _ = machine;
}

#[test]
fn secure_boot_enforcing_with_the_machines_own_keys_is_done() {
    let mut xps = StandIn::new();
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

    let mut surface = StandIn::new();
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
    let mut stand_in = StandIn::new();
    efivars(
        &mut stand_in,
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
        CheckSecureBoot.observe(&mut stand_in).unwrap(),
        Standing::Blocked { .. }
    ));
}

/// `tpm2_getcap`'s arguments for the TPM's NV indices.
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
        .fold(String::new(), |mut kept, line| {
            // Writing to a String can't fail.
            _ = writeln!(kept, "{line}");
            kept
        })
}

fn machine() -> StandIn {
    let mut stand_in = StandIn::new();
    stand_in.file(FACTS, br#"{"disk_name":"system"}"#);
    stand_in.file(
        PCRLOCK,
        fixture!("dell-xps-14-da14260", "pcrlock-nvindex.json"),
    );
    stand_in
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
fn holding_after(indices: &str, made: &str) -> StandIn {
    let mut stand_in = machine();
    stand_in.prints(Tool::Tpm2Getcap, NV, indices);
    stand_in.prints(Tool::Systemctl, MADE, &format!("{made}\n"));
    stand_in
}

fn holding(indices: &str) -> StandIn {
    holding_after(indices, "success")
}

/// The YubiKey's PIN, then the TPM's own, typed twice.
fn answers(key_pin: &str, pin: &str, again: &str) -> Answers {
    let mut answers = Answers::new();
    answers.give("key", Answer::Done);
    answers.give("key-pin", Answer::Secret(Secret::new(key_pin.into())));
    answers.give("tpm-pin", Answer::Secret(Secret::new(pin.into())));
    answers.give("tpm-pin-again", Answer::Secret(Secret::new(again.into())));
    answers
}

#[test]
fn the_tpm_is_sealed_through_run0_its_slot_preferred_and_proven() {
    let mut stand_in = holding(held());
    let installed =
        std::str::from_utf8(fixture!("qemu-canokey", "luks-dump-installed.json")).unwrap();
    let sealed = std::str::from_utf8(fixture!("qemu-canokey", "luks-dump-sealed.json")).unwrap();
    // Unsealed as observed here, as the journey observes before running and
    // as the step reads it before enrolling; sealed after, and from then on.
    stand_in
        .prints(Tool::Cryptsetup, DUMP, installed)
        .prints(Tool::Cryptsetup, DUMP, installed)
        .prints(Tool::Cryptsetup, DUMP, installed)
        .prints(Tool::Cryptsetup, DUMP, sealed);
    stand_in.answers(
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
    stand_in.prints(
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
    stand_in.prints(
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
    let Standing::Needed { asks } = SealTpm.observe(&mut stand_in).unwrap() else {
        panic!("the TPM is still to seal")
    };
    let ids: Vec<&str> = asks.iter().map(Ask::id).collect();
    assert_eq!(ids, ["key", "key-pin", "tpm-pin", "tpm-pin-again"]);
    Journey::attempt(
        &SealTpm,
        &mut stand_in,
        &answers("123456", "an account password", "an account password"),
    )
    .unwrap();

    let privileged: Vec<&String> = stand_in
        .calls
        .iter()
        // Reading how a unit ended is anyone's to do.
        .filter(|call| !call.starts_with("reading") && !call.starts_with("systemctl show"))
        .collect();
    assert!(
        privileged.iter().all(|call| call.ends_with("(as root)")),
        "{privileged:#?}"
    );
    let enrolled = stand_in
        .calls
        .iter()
        .find(|call| call.starts_with("systemd-cryptenroll"))
        .unwrap();
    assert!(
        enrolled.contains("[cryptsetup.fido2-pin cryptenroll.fido2-pin cryptenroll.new-tpm2-pin]"),
        "{enrolled}"
    );
    // The YubiKey's PIN opens the disk this once; the TPM is sealed to a
    // PIN of its own, and the proof opens it with that one.
    let handed: Vec<(&str, &str)> = stand_in
        .secrets
        .iter()
        .map(|(place, secret)| (place.as_str(), secret.as_str()))
        .collect();
    assert_eq!(
        handed,
        [
            ("cryptsetup.fido2-pin", "123456"),
            ("cryptenroll.fido2-pin", "123456"),
            ("cryptenroll.new-tpm2-pin", "an account password"),
            ("stdin", "an account password\n"),
        ]
    );
    assert!(
        stand_in
            .calls
            .iter()
            .all(|call| !call.contains("123456") && !call.contains("account password")),
        "{:#?}",
        stand_in.calls
    );
}

#[test]
fn two_different_tpm_pins_seal_nothing() {
    let mut stand_in = machine();
    let refused = SealTpm.run(
        &mut stand_in,
        &answers("123456", "an account password", "an account pasword"),
    );
    assert!(
        matches!(&refused, Err(Fault::Blocked { why, .. }) if why == "the TPM's new PIN wasn't the same both times, so nothing changed"),
        "{refused:?}"
    );
    assert!(stand_in.calls.is_empty(), "{:#?}", stand_in.calls);
}

#[test]
fn no_pin_seals_nothing() {
    let mut stand_in = machine();
    let mut answers = Answers::new();
    answers.give("key", Answer::Done);
    let refused = SealTpm.run(&mut stand_in, &answers);
    assert!(
        matches!(&refused, Err(Fault::Unanswered { ask: "key-pin", .. })),
        "{refused:?}"
    );
    assert!(stand_in.calls.is_empty(), "{:#?}", stand_in.calls);
}

#[test]
fn a_tpm_slot_not_tried_first_is_still_to_do() {
    let mut stand_in = machine();
    // Proven to open, but no longer preferred: the real header, its
    // priority taken off.
    let mut header: serde_json::Value =
        serde_json::from_slice(fixture!("qemu-canokey", "luks-dump-sealed.json")).unwrap();
    header["keyslots"]["0"]
        .as_object_mut()
        .unwrap()
        .remove("priority");
    stand_in.prints(Tool::Cryptsetup, DUMP, &header.to_string());
    stand_in.journal().record("seal-tpm", "opened:0").unwrap();
    assert!(matches!(
        SealTpm.observe(&mut stand_in).unwrap(),
        Standing::Needed { .. }
    ));
}

#[test]
fn a_tpm_slot_nothing_proved_is_still_to_do() {
    let mut stand_in = machine();
    // Sealed and preferred, but this guide never saw it open.
    stand_in.prints(
        Tool::Cryptsetup,
        DUMP,
        std::str::from_utf8(fixture!("qemu-canokey", "luks-dump-sealed.json")).unwrap(),
    );
    assert!(matches!(
        SealTpm.observe(&mut stand_in).unwrap(),
        Standing::Needed { .. }
    ));
}

#[test]
fn a_seal_proven_once_holds_while_the_tpm_keeps_its_policy() {
    let mut stand_in = holding(held());
    stand_in.prints(
        Tool::Cryptsetup,
        DUMP,
        std::str::from_utf8(fixture!("qemu-canokey", "luks-dump-sealed.json")).unwrap(),
    );
    stand_in.journal().record("seal-tpm", "opened:0").unwrap();
    assert!(matches!(
        SealTpm.observe(&mut stand_in).unwrap(),
        Standing::Done { .. }
    ));
}

#[test]
fn a_cleared_tpm_undoes_a_seal_proven_before() {
    // lanzaboote's unit still ends well after a clear, as vm-guide-installs
    // shows: its prediction is unchanged, so it never writes the index it
    // would find gone. Only the TPM's own list tells.
    let mut stand_in = holding_after(&cleared(), "success");
    stand_in.prints(
        Tool::Cryptsetup,
        DUMP,
        std::str::from_utf8(fixture!("qemu-canokey", "luks-dump-sealed.json")).unwrap(),
    );
    stand_in.journal().record("seal-tpm", "opened:0").unwrap();
    assert!(matches!(
        SealTpm.observe(&mut stand_in).unwrap(),
        Standing::Needed { .. }
    ));
}

#[test]
fn a_firmware_update_undoes_a_seal_proven_before() {
    // The TPM still holds the policy, which no longer follows what this
    // boot's firmware measured, so lanzaboote's unit couldn't remake it.
    let mut stand_in = holding_after(held(), "exit-code");
    stand_in.prints(
        Tool::Cryptsetup,
        DUMP,
        std::str::from_utf8(fixture!("qemu-canokey", "luks-dump-sealed.json")).unwrap(),
    );
    stand_in.journal().record("seal-tpm", "opened:0").unwrap();
    assert!(matches!(
        SealTpm.observe(&mut stand_in).unwrap(),
        Standing::Needed { .. }
    ));
}

#[test]
fn a_cleared_tpm_gets_its_policy_made_again_before_the_disk_is_sealed_to_it() {
    // Cleared when the step looks, then held once lanzaboote's unit made
    // the policy again, from then on.
    let mut stand_in = machine();
    stand_in.prints(Tool::Tpm2Getcap, NV, &cleared());
    stand_in.prints(Tool::Tpm2Getcap, NV, held());
    // A TPM without the index needs no more asked of it, so the unit is
    // first asked once it made the policy again.
    stand_in.prints(Tool::Systemctl, MADE, "success\n");
    let sealed = std::str::from_utf8(fixture!("qemu-canokey", "luks-dump-sealed.json")).unwrap();
    stand_in.prints(Tool::Cryptsetup, DUMP, sealed);
    // As pcrlock answers when the index it would remove is gone.
    stand_in.answers(
        Program::Tool(Tool::Pcrlock),
        &["remove-policy"],
        1,
        "",
        "Failed to remove NV index, assuming data out of date, removing policy file.",
    );
    stand_in.prints(
        Tool::Systemctl,
        &["restart", "systemd-pcrlock-make-policy.service"],
        "",
    );
    stand_in.answers(
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
    stand_in.prints(
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
    stand_in.prints(
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
    Journey::attempt(
        &SealTpm,
        &mut stand_in,
        &answers("123456", "an account password", "an account password"),
    )
    .unwrap();

    let at = |start: &str| {
        stand_in
            .calls
            .iter()
            .position(|call| call.starts_with(start))
            .unwrap_or_else(|| panic!("no {start} in {:#?}", stand_in.calls))
    };
    assert!(
        at("systemd-pcrlock remove-policy")
            < at("systemctl restart systemd-pcrlock-make-policy.service")
    );
    assert!(
        at("systemctl restart") < at("systemd-cryptenroll"),
        "{:#?}",
        stand_in.calls
    );
    let renewing: Vec<&String> = stand_in
        .calls
        .iter()
        .filter(|call| call.starts_with("systemd-pcrlock") || call.starts_with("systemctl restart"))
        .collect();
    assert!(
        renewing.iter().all(|call| call.ends_with("(as root)")),
        "{renewing:#?}"
    );
}

/// The rehearsal's reinstalled disk in QEMU: its seal proven, the TPM
/// holding its policy, and lanzaboote's unit unable to remake it, as when
/// the TPM refused the boot. Its NV indices are those that TPM held, the
/// first install's left among them.
const REHEARSED: &str =
    "- 0x194A666\n- 0x194FA70\n- 0x1D10200\n- 0x1D10201\n- 0x1D10202\n- 0x1D10203\n";

/// That disk on a boot that measured this into PCR 4 and logged this.
fn rehearsed(indices: &str, log: &[u8], pcr4: &[u8]) -> StandIn {
    let mut stand_in = StandIn::new();
    stand_in.file(FACTS, br#"{"disk_name":"system"}"#);
    stand_in.file(PCRLOCK, fixture!("qemu-canokey", "pcrlock-policy.json"));
    stand_in.prints(Tool::Tpm2Getcap, NV, indices);
    stand_in.prints(Tool::Systemctl, MADE, "exit-code\n");
    stand_in.answers(
        Program::Tool(Tool::Pcrlock),
        &["log", "--json=short"],
        0,
        std::str::from_utf8(log).unwrap(),
        "",
    );
    stand_in.file(
        "/sys/class/tpm/tpm0/pcr-sha256/0",
        fixture!("qemu-canokey", "pcr-sha256-0"),
    );
    stand_in.file("/sys/class/tpm/tpm0/pcr-sha256/4", pcr4);
    stand_in.file(
        "/sys/class/tpm/tpm0/pcr-sha256/7",
        fixture!("qemu-canokey", "pcr-sha256-7"),
    );
    stand_in.prints(
        Tool::Cryptsetup,
        DUMP,
        std::str::from_utf8(fixture!("qemu-canokey", "luks-dump-sealed.json")).unwrap(),
    );
    stand_in.journal().record("seal-tpm", "opened:0").unwrap();
    stand_in
}

/// Booted through OVMF's boot menu, which measured its own program into
/// PCR 4 before the separator.
fn through_the_menu_holding(indices: &str) -> StandIn {
    rehearsed(
        indices,
        fixture!("qemu-canokey", "pcrlock-log-through-the-menu.json"),
        fixture!("qemu-canokey", "pcr-sha256-4-through-the-menu"),
    )
}

fn through_the_menu() -> StandIn {
    through_the_menu_holding(REHEARSED)
}

#[test]
fn a_boot_through_the_firmwares_menu_keeps_the_seal_for_the_next_ordinary_one() {
    // Sealing again here would lock the menu in, and the next ordinary
    // boot, which the TPM opened after this one in the rehearsal, would be
    // the one refused.
    let mut stand_in = through_the_menu();
    let standing = SealTpm.observe(&mut stand_in).unwrap();
    assert!(
        matches!(&standing, Standing::Done { evidence } if evidence.contains("firmware's own menu")),
        "{standing:?}"
    );
    let log = stand_in
        .calls
        .iter()
        .find(|call| call.starts_with("systemd-pcrlock log"))
        .unwrap();
    assert!(log.ends_with("(as root)"), "{log}");
}

#[test]
fn a_boot_straight_from_the_disk_the_policy_failed_on_still_needs_the_seal() {
    // PCR 4 as the policy foresaw it, so whatever made the policy fail,
    // no firmware program explains it.
    let mut stand_in = rehearsed(
        REHEARSED,
        fixture!("qemu-canokey", "pcrlock-log.json"),
        fixture!("qemu-canokey", "pcr-sha256-4"),
    );
    assert!(matches!(
        SealTpm.observe(&mut stand_in).unwrap(),
        Standing::Needed { .. }
    ));
}

#[test]
fn new_firmware_on_a_boot_through_the_menu_still_needs_the_seal() {
    let mut stand_in = through_the_menu();
    stand_in.file(
        "/sys/class/tpm/tpm0/pcr-sha256/0",
        b"B4597C7162476A8C53AA825E9C7EEC9BECEFAD902C82ECA12086ADAB07872E34\n",
    );
    assert!(matches!(
        SealTpm.observe(&mut stand_in).unwrap(),
        Standing::Needed { .. }
    ));
}

#[test]
fn a_cleared_tpm_on_a_boot_through_the_menu_still_needs_the_seal() {
    // Its owner's indices gone, the platform's left.
    let mut stand_in = through_the_menu_holding("- 0x1D10200\n");
    assert!(matches!(
        SealTpm.observe(&mut stand_in).unwrap(),
        Standing::Needed { .. }
    ));
}
