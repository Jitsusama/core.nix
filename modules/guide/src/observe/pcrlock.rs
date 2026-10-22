//! Whether a boot the TPM wouldn't open the disk for went through one of
//! the firmware's own programs, such as its boot menu, and differs from
//! what the disk's policy expects in that alone.
//!
//! systemd-pcrlock locks into PCR 4 every program the firmware starts
//! before the boot loader's separator, from what the last boot measured,
//! since some firmware measures its own programs there at every boot. A
//! boot through the firmware's boot menu measures the menu too, so the
//! TPM refuses it. Sealing again then would lock that boot's menu in, and
//! the next ordinary boot would be refused instead. An ordinary boot
//! needs nothing: the policy already expects it.
//!
//! The question is answered exactly: PCR 4 is replayed from the boot's
//! event log without the firmware's programs, and has to come out as a
//! value the policy holds, with every other PCR the policy covers already
//! one.

use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::Fault;

/// PCR 4, where the firmware measures the programs it starts.
const BOOT_LOADER_CODE: u32 = 4;

/// The most firmware programs whose every combination is tried, which
/// keeps the replays to a few dozen; firmware starts one or two.
const MOST_PROGRAMS: usize = 5;

/// The PCRs `/var/lib/systemd/pcrlock.json` holds the disk to, each with
/// every value the policy lets it have.
#[derive(Debug, Deserialize)]
pub struct Policy {
    #[serde(rename = "pcrBank", default)]
    bank: String,
    #[serde(rename = "pcrValues", default)]
    pcrs: Vec<Pcr>,
}

#[derive(Debug, Deserialize)]
struct Pcr {
    pcr: u32,
    values: Vec<String>,
}

impl Policy {
    pub fn parse(bytes: &[u8]) -> Result<Policy, Fault> {
        serde_json::from_slice(bytes).map_err(|why| Fault::unreadable("the TPM's policy", why))
    }

    /// Whether this policy is one a firmware program could explain a
    /// refusal of: one in the SHA-256 bank that covers PCR 4.
    pub fn may_explain(&self) -> bool {
        self.bank == "sha256" && self.covers().any(|pcr| pcr == BOOT_LOADER_CODE)
    }

    /// The PCRs the policy covers.
    pub fn covers(&self) -> impl Iterator<Item = u32> + '_ {
        self.pcrs.iter().map(|pcr| pcr.pcr)
    }

    fn allows(&self, pcr: u32, value: &[u8]) -> Option<bool> {
        let held = self.pcrs.iter().find(|held| held.pcr == pcr)?;
        Some(
            held.values
                .iter()
                .any(|allowed| unhex(allowed).as_deref() == Some(value)),
        )
    }
}

/// One measurement in the boot's event log, as `systemd-pcrlock log
/// --json=short` prints it.
#[derive(Debug, Deserialize)]
pub struct Record {
    pcr: Option<u32>,
    event: Option<String>,
    sha256: Option<String>,
}

#[derive(Deserialize)]
struct Log {
    log: Vec<Record>,
}

pub fn parse_log(text: &str) -> Result<Vec<Record>, Fault> {
    let log: Log =
        serde_json::from_str(text).map_err(|why| Fault::unreadable("the boot's event log", why))?;
    Ok(log.log)
}

/// A PCR's value as the kernel shows it under
/// `/sys/class/tpm/tpm0/pcr-sha256/`.
pub fn pcr_path(pcr: u32) -> String {
    format!("/sys/class/tpm/tpm0/pcr-sha256/{pcr}")
}

pub fn parse_pcr(text: &str) -> Option<Vec<u8>> {
    unhex(text.trim())
}

/// Whether this boot differs from what the policy expects only in the
/// firmware programs measured into PCR 4 before its separator. `current`
/// gives each PCR's value now, as the TPM holds it.
pub fn through_firmware(
    policy: &Policy,
    log: &[Record],
    current: impl Fn(u32) -> Option<Vec<u8>>,
) -> bool {
    if policy.bank != "sha256" {
        return false;
    }
    for pcr in policy.covers().filter(|&pcr| pcr != BOOT_LOADER_CODE) {
        let Some(value) = current(pcr) else {
            return false;
        };
        if policy.allows(pcr, &value) != Some(true) {
            return false;
        }
    }
    let Some(now) = current(BOOT_LOADER_CODE) else {
        return false;
    };
    // Covering PCR 4 and already allowed: whatever the TPM refused, it
    // wasn't this.
    if policy.allows(BOOT_LOADER_CODE, &now) != Some(false) {
        return false;
    }
    let Some(measured) = log
        .iter()
        .filter(|record| record.pcr == Some(BOOT_LOADER_CODE))
        .filter(|record| record.event.as_deref() != Some("no-action"))
        .map(|record| {
            let digest = unhex(record.sha256.as_deref()?)?;
            Some((record.event.as_deref(), digest))
        })
        .collect::<Option<Vec<_>>>()
    else {
        return false;
    };
    // A log that doesn't replay to what the TPM holds can't say what the
    // boot measured.
    if replay(measured.iter().map(|(_, digest)| digest.as_slice())) != now {
        return false;
    }
    let separator = measured
        .iter()
        .position(|(event, _)| *event == Some("separator"))
        .unwrap_or(measured.len());
    let programs: Vec<usize> = measured[..separator]
        .iter()
        .enumerate()
        .filter(|(_, (event, _))| *event == Some("efi-boot-services-application"))
        .map(|(at, _)| at)
        .collect();
    if programs.is_empty() || programs.len() > MOST_PROGRAMS {
        return false;
    }
    // Every non-empty set of them left out, one replay each.
    (1..1u32 << programs.len()).any(|left_out| {
        let kept = measured.iter().enumerate().filter(|(at, _)| {
            !programs
                .iter()
                .enumerate()
                .any(|(bit, program)| program == at && left_out & (1 << bit) != 0)
        });
        let value = replay(kept.map(|(_, (_, digest))| digest.as_slice()));
        policy.allows(BOOT_LOADER_CODE, &value) == Some(true)
    })
}

/// A PCR extended from zero by each digest in turn, as the TPM does.
fn replay<'a>(digests: impl Iterator<Item = &'a [u8]>) -> Vec<u8> {
    digests.fold(vec![0; 32], |pcr, digest| {
        let mut hash = Sha256::new();
        hash.update(&pcr);
        hash.update(digest);
        hash.finalize().to_vec()
    })
}

fn unhex(text: &str) -> Option<Vec<u8>> {
    if !text.len().is_multiple_of(2) {
        return None;
    }
    (0..text.len())
        .step_by(2)
        .map(|at| u8::from_str_radix(text.get(at..at + 2)?, 16).ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // A rehearsal's reinstalled disk in QEMU on 2026-10-22: the policy
    // systemd-pcrlock made, and PCR 4 of the boot OVMF sent through its
    // boot menu app first, which the TPM refused.
    const POLICY: &str = r#"{"pcrBank":"sha256","pcrValues":[
        {"pcr":0,"values":["a4597c7162476a8c53aa825e9c7eec9becefad902c82eca12086adab07872e34"]},
        {"pcr":4,"values":["0052db0f175d97fb9405c7035b7e6bc2aacb52aa2a46754c7dd7f7f5cb921e29"]},
        {"pcr":7,"values":["4aa21defa27253a09af5c44180d0b85bddb0df96910402eee518972eacbaf91d"]}]}"#;
    const MENU: &str = "8f914fac9df41e4a5f26a85423a74474590380dfa5d8f2c291b56569d71e95e9";
    const ACTION: &str = "3d6772b4f84ed47595d72a2c4c5ffd15f5bb72c7507fe26f2aaee2c69d5633ba";
    const SEPARATOR: &str = "df3f619804a92fdb4057192dc43dd748ea778adc52bc498ce80524c014b81119";
    const LOADER: &str = "d3a1401a0fcfa1dc5219ecee69bf3b8bc3f1aa875f786b7ce84a34089738fff4";
    const UKI: &str = "9e141118321e67a68f9fcb081ea283eed2a25a2d413659c06ab75a0f1f3e4daa";
    const PCR0: &str = "A4597C7162476A8C53AA825E9C7EEC9BECEFAD902C82ECA12086ADAB07872E34";
    const PCR7: &str = "4AA21DEFA27253A09AF5C44180D0B85BDDB0DF96910402EEE518972EACBAF91D";

    fn log(records: &[(&str, &str)]) -> Vec<Record> {
        records
            .iter()
            .map(|(event, sha256)| Record {
                pcr: Some(4),
                event: Some(event.to_string()),
                sha256: Some(sha256.to_string()),
            })
            .collect()
    }

    fn through_menu() -> Vec<Record> {
        log(&[
            ("efi-boot-services-application", MENU),
            ("efi-action", ACTION),
            ("separator", SEPARATOR),
            ("efi-boot-services-application", LOADER),
            ("efi-boot-services-application", UKI),
        ])
    }

    fn replayed(records: &[Record]) -> Vec<u8> {
        replay(
            records
                .iter()
                .map(|record| unhex(record.sha256.as_deref().unwrap()).unwrap())
                .collect::<Vec<_>>()
                .iter()
                .map(Vec::as_slice),
        )
    }

    fn holding(pcr0: &str, pcr4: Vec<u8>) -> impl Fn(u32) -> Option<Vec<u8>> {
        let pcr0 = parse_pcr(pcr0);
        move |pcr| match pcr {
            0 => pcr0.clone(),
            4 => Some(pcr4.clone()),
            7 => parse_pcr(PCR7),
            _ => None,
        }
    }

    #[test]
    fn a_boot_through_the_firmwares_menu_is_told_from_the_rest() {
        let policy = Policy::parse(POLICY.as_bytes()).unwrap();
        let log = through_menu();
        // As the TPM held it then.
        assert_eq!(
            replayed(&log),
            parse_pcr("52219BA496217DB3160C4570246D1A104E8A8D5FC5C5D576D91B10C81AA35CCE").unwrap()
        );
        assert!(through_firmware(
            &policy,
            &log,
            holding(PCR0, replayed(&log))
        ));
    }

    #[test]
    fn an_ordinary_boot_is_no_detour() {
        let policy = Policy::parse(POLICY.as_bytes()).unwrap();
        let log = through_menu().into_iter().skip(1).collect::<Vec<_>>();
        assert!(!through_firmware(
            &policy,
            &log,
            holding(PCR0, replayed(&log))
        ));
    }

    #[test]
    fn new_firmware_is_no_detour() {
        let policy = Policy::parse(POLICY.as_bytes()).unwrap();
        let log = through_menu();
        let other = "B4597C7162476A8C53AA825E9C7EEC9BECEFAD902C82ECA12086ADAB07872E34";
        assert!(!through_firmware(
            &policy,
            &log,
            holding(other, replayed(&log))
        ));
    }

    #[test]
    fn a_program_the_boot_loader_started_is_no_detour() {
        let policy = Policy::parse(POLICY.as_bytes()).unwrap();
        let mut log = through_menu().into_iter().skip(1).collect::<Vec<_>>();
        log.extend(super::tests::log(&[(
            "efi-boot-services-application",
            MENU,
        )]));
        assert!(!through_firmware(
            &policy,
            &log,
            holding(PCR0, replayed(&log))
        ));
    }

    #[test]
    fn a_log_the_tpm_disagrees_with_says_nothing() {
        let policy = Policy::parse(POLICY.as_bytes()).unwrap();
        let log = through_menu();
        // Neither what the log replays to nor what the policy allows.
        let other = replayed(&log[2..]);
        assert!(!through_firmware(&policy, &log, holding(PCR0, other)));
    }
}
