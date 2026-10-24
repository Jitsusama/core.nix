//! The TPM freed of the policies earlier installs left in it, held against
//! what an XPS 14's TPM printed: only an index shaped as systemd-pcrlock
//! defines one, never this install's own, and nothing at all while the
//! TPM doesn't hold this install's.

use guide::fake::Fake;
use guide::installed::PCRLOCK;
use guide::installed::policies::FreeOldPolicies;
use guide::{Answers, Standing, Step, Tool};

macro_rules! xps {
    ($file:literal) => {
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/fixtures/dell-xps-14-da14260/",
            $file
        ))
    };
}

const NV: &[&str] = &["-T", "device:/dev/tpmrm0", "handles-nv-index"];
/// This install's index, as its policy file names it.
const OWN: &str = "0x1accce4";
/// An earlier install's, left when its disk was wiped.
const EARLIER: &str = "0x1b00042";

/// The XPS 14's TPM, with an earlier install's policy index beside its
/// own.
fn machine(earlier: bool) -> Fake {
    let mut listed = xps!("tpm2-handles-nv-index.txt").to_string();
    if earlier {
        listed.push_str("- 0x1B00042\n");
    }
    tpm(&listed, &pcrlocks(EARLIER))
}

/// An index as systemd-pcrlock defines one, read as the XPS 14's own.
fn pcrlocks(index: &str) -> String {
    xps!("tpm2-nvreadpublic-0x1accce4.txt").replace(OWN, index)
}

/// A TPM listing these indices, the earlier one reading as given.
fn tpm(listed: &str, earlier: &str) -> Fake {
    let mut fake = Fake::new();
    fake.file(PCRLOCK, xps!("pcrlock-nvindex.json").as_bytes());
    fake.prints(Tool::Tpm2Getcap, NV, listed);
    fake.prints(
        Tool::Tpm2Nvreadpublic,
        &["-T", "device:/dev/tpmrm0", OWN],
        &pcrlocks(OWN),
    );
    fake.prints(
        Tool::Tpm2Nvreadpublic,
        &["-T", "device:/dev/tpmrm0", EARLIER],
        earlier,
    );
    fake
}

fn read(fake: &Fake) -> Vec<&String> {
    fake.calls
        .iter()
        .filter(|call| call.starts_with("tpm2_nvreadpublic"))
        .collect()
}

fn freed(fake: &Fake) -> Vec<&String> {
    fake.calls
        .iter()
        .filter(|call| call.starts_with("tpm2_nvundefine"))
        .collect()
}

#[test]
fn an_earlier_installs_policy_is_freed_and_this_ones_kept() {
    let mut fake = machine(true);
    assert!(matches!(
        FreeOldPolicies.observe(&mut fake).unwrap(),
        Standing::Needed { .. }
    ));
    fake.prints(
        Tool::Tpm2Nvundefine,
        &["-T", "device:/dev/tpmrm0", "-C", "o", EARLIER],
        "",
    );
    FreeOldPolicies.run(&mut fake, &Answers::new()).unwrap();
    let freed = freed(&fake);
    assert_eq!(freed.len(), 1, "{:#?}", fake.calls);
    assert!(freed[0].contains(EARLIER) && freed[0].ends_with("(as root)"));
    // Only the indices in systemd's range were read, never this one's.
    assert!(
        read(&fake).iter().all(|call| call.contains(EARLIER)),
        "{:#?}",
        fake.calls
    );
}

#[test]
fn a_tpm_holding_only_this_installs_policy_is_done() {
    let mut fake = machine(false);
    let Standing::Done { evidence } = FreeOldPolicies.observe(&mut fake).unwrap() else {
        panic!("nothing earlier is held")
    };
    assert!(evidence.contains("no earlier one"), "{evidence}");
    assert!(read(&fake).is_empty(), "{:#?}", fake.calls);
}

#[test]
fn an_index_of_another_shape_in_the_range_is_left() {
    // Someone else's: deletable through its own policy, which pcrlock's
    // never is.
    let theirs = pcrlocks(EARLIER).replace("0x20021008", "0x20021408");
    let listed = format!("{}- 0x1B00042\n", xps!("tpm2-handles-nv-index.txt"));
    let mut fake = tpm(&listed, &theirs);
    assert!(matches!(
        FreeOldPolicies.observe(&mut fake).unwrap(),
        Standing::Done { .. }
    ));
    FreeOldPolicies.run(&mut fake, &Answers::new()).unwrap();
    assert!(freed(&fake).is_empty(), "{:#?}", fake.calls);
}

#[test]
fn nothing_is_freed_while_the_tpm_doesnt_hold_this_installs_policy() {
    // Cleared since: this install's index is gone, so which is whose
    // can't be told.
    let listed: String = xps!("tpm2-handles-nv-index.txt")
        .lines()
        .filter(|line| !line.starts_with("- 0x1ACCCE4"))
        .map(|line| format!("{line}\n"))
        .chain(["- 0x1B00042\n".to_string()])
        .collect();
    let mut fake = tpm(&listed, &pcrlocks(EARLIER));
    let Standing::Done { evidence } = FreeOldPolicies.observe(&mut fake).unwrap() else {
        panic!("a TPM without this install's policy is left alone")
    };
    assert!(evidence.contains("doesn't hold"), "{evidence}");
    FreeOldPolicies.run(&mut fake, &Answers::new()).unwrap();
    assert!(freed(&fake).is_empty(), "{:#?}", fake.calls);
}

#[test]
fn a_machine_without_a_policy_file_is_left_alone() {
    let mut fake = machine(true);
    fake.no_file(PCRLOCK);
    assert!(matches!(
        FreeOldPolicies.observe(&mut fake).unwrap(),
        Standing::Done { .. }
    ));
    assert!(read(&fake).is_empty(), "{:#?}", fake.calls);
}
