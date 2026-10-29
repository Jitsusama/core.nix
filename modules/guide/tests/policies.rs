//! The TPM freed of the policies earlier installs left in it, held against
//! what an XPS 14's TPM printed: only an index shaped as systemd-pcrlock
//! defines one, never this install's own, and nothing at all while the
//! TPM doesn't hold this install's.

use guide::installed::PCRLOCK;
use guide::installed::policies::FreeOldPolicies;
use guide::stand_in::StandIn;
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
fn machine(earlier: bool) -> StandIn {
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
fn tpm(listed: &str, earlier: &str) -> StandIn {
    let mut stand_in = StandIn::new();
    stand_in.file(PCRLOCK, xps!("pcrlock-nvindex.json").as_bytes());
    stand_in.prints(Tool::Tpm2Getcap, NV, listed);
    stand_in.prints(
        Tool::Tpm2Nvreadpublic,
        &["-T", "device:/dev/tpmrm0", OWN],
        &pcrlocks(OWN),
    );
    stand_in.prints(
        Tool::Tpm2Nvreadpublic,
        &["-T", "device:/dev/tpmrm0", EARLIER],
        earlier,
    );
    stand_in
}

fn read(stand_in: &StandIn) -> Vec<&String> {
    stand_in
        .calls
        .iter()
        .filter(|call| call.starts_with("tpm2_nvreadpublic"))
        .collect()
}

fn freed(stand_in: &StandIn) -> Vec<&String> {
    stand_in
        .calls
        .iter()
        .filter(|call| call.starts_with("tpm2_nvundefine"))
        .collect()
}

#[test]
fn an_earlier_installs_policy_is_freed_and_this_ones_kept() {
    let mut stand_in = machine(true);
    assert!(matches!(
        FreeOldPolicies.observe(&mut stand_in).unwrap(),
        Standing::Needed { .. }
    ));
    stand_in.prints(
        Tool::Tpm2Nvundefine,
        &["-T", "device:/dev/tpmrm0", "-C", "o", EARLIER],
        "",
    );
    FreeOldPolicies.run(&mut stand_in, &Answers::new()).unwrap();
    let freed = freed(&stand_in);
    assert_eq!(freed.len(), 1, "{:#?}", stand_in.calls);
    assert!(freed[0].contains(EARLIER) && freed[0].ends_with("(as root)"));
    // Only the indices in systemd's range were read, never this one's.
    assert!(
        read(&stand_in).iter().all(|call| call.contains(EARLIER)),
        "{:#?}",
        stand_in.calls
    );
}

#[test]
fn a_tpm_holding_only_this_installs_policy_is_done() {
    let mut stand_in = machine(false);
    let Standing::Done { evidence } = FreeOldPolicies.observe(&mut stand_in).unwrap() else {
        panic!("nothing earlier is held")
    };
    assert!(evidence.contains("no earlier one"), "{evidence}");
    assert!(read(&stand_in).is_empty(), "{:#?}", stand_in.calls);
}

#[test]
fn an_index_of_another_shape_in_the_range_is_left() {
    // Someone else's: deletable through its own policy, which pcrlock's
    // never is.
    let theirs = pcrlocks(EARLIER).replace("0x20021008", "0x20021408");
    let listed = format!("{}- 0x1B00042\n", xps!("tpm2-handles-nv-index.txt"));
    let mut stand_in = tpm(&listed, &theirs);
    assert!(matches!(
        FreeOldPolicies.observe(&mut stand_in).unwrap(),
        Standing::Done { .. }
    ));
    FreeOldPolicies.run(&mut stand_in, &Answers::new()).unwrap();
    assert!(freed(&stand_in).is_empty(), "{:#?}", stand_in.calls);
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
    let mut stand_in = tpm(&listed, &pcrlocks(EARLIER));
    let Standing::Done { evidence } = FreeOldPolicies.observe(&mut stand_in).unwrap() else {
        panic!("a TPM without this install's policy is left alone")
    };
    assert!(evidence.contains("doesn't hold"), "{evidence}");
    FreeOldPolicies.run(&mut stand_in, &Answers::new()).unwrap();
    assert!(freed(&stand_in).is_empty(), "{:#?}", stand_in.calls);
}

#[test]
fn a_machine_without_a_policy_file_is_left_alone() {
    let mut stand_in = machine(true);
    stand_in.no_file(PCRLOCK);
    assert!(matches!(
        FreeOldPolicies.observe(&mut stand_in).unwrap(),
        Standing::Done { .. }
    ));
    assert!(read(&stand_in).is_empty(), "{:#?}", stand_in.calls);
}
