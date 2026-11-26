//! What decides whether the lock goes after a resume: the TPM's start
//! counts, which say the firmware started the machine and so the image came
//! back through the disk prompt, and each disk's tokens, which say the
//! prompt could only open it with something typed.

use crate::said;
use serde_json::Value;

/// The TPM's counts of the times it started afresh and the times it started
/// from saved state, the two a firmware start moves.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub(crate) struct Starts {
    reset: u64,
    restart: u64,
}

impl Starts {
    /// Reads `tpm2_readclock`'s `reset_count:` and `restart_count:` lines.
    fn parse(said: &str) -> Option<Self> {
        let count = |name: &str| {
            said.lines()
                .filter_map(|line| line.trim().strip_prefix(name))
                .find_map(|rest| rest.strip_prefix(':')?.trim().parse().ok())
        };
        Some(Self {
            reset: count("reset_count")?,
            restart: count("restart_count")?,
        })
    }

    /// As kept in `/run`: the two counts on one line.
    pub(crate) fn kept(self) -> String {
        format!("{} {}\n", self.reset, self.restart)
    }

    pub(crate) fn unkept(text: &str) -> Option<Self> {
        let mut counts = text.split_whitespace().map(str::parse);
        let (Some(Ok(reset)), Some(Ok(restart)), None) =
            (counts.next(), counts.next(), counts.next())
        else {
            return None;
        };
        Some(Self { reset, restart })
    }

    /// Whether the firmware started the machine between `self` and `now`.
    pub(crate) fn started_since(self, now: Self) -> bool {
        now.reset > self.reset || now.restart > self.restart
    }
}

pub(crate) fn starts() -> Option<Starts> {
    Starts::parse(&said("tpm2_readclock", &[])?)
}

/// The first token in a disk's LUKS2 header, as `cryptsetup luksDump
/// --dump-json-metadata` prints it, that opens the disk with nothing typed,
/// or nothing when every one asks. A keyslot with no token is a password,
/// and a recovery key is typed like one.
fn unasked(header: &Value) -> Option<String> {
    let Some(tokens) = header["tokens"].as_object() else {
        return Some("its header lists no tokens".to_owned());
    };
    tokens.iter().find_map(|(slot, token)| {
        let kind = token["type"].as_str().unwrap_or("no type");
        let asks = match kind {
            "systemd-tpm2" => token["tpm2-pin"] == true,
            "systemd-fido2" => token["fido2-clientPin-required"] == true,
            "systemd-recovery" => true,
            _ => false,
        };
        (!asks).then(|| format!("token {slot} ({kind}) opens it unasked"))
    })
}

/// Why a disk could open without the prompt asking for anything, or
/// nothing when it couldn't.
pub(crate) fn disk_unasked(disk: &str) -> Option<String> {
    let header = said("cryptsetup", &["luksDump", "--dump-json-metadata", disk])
        .and_then(|json| serde_json::from_str::<Value>(&json).ok());
    match header {
        Some(header) => unasked(&header),
        None => Some("its header couldn't be read".to_owned()),
    }
}

#[cfg(test)]
mod tests {
    use super::{Starts, unasked};
    use serde_json::json;

    const READCLOCK: &str = "time: 3354065\nclock_info:\n  clock: 1776610977\n  \
                             reset_count: 95\n  restart_count: 0\n  safe: yes\n";

    #[test]
    fn the_counts_are_read_from_tpm2_readclock() {
        assert_eq!(
            Starts::parse(READCLOCK),
            Some(Starts {
                reset: 95,
                restart: 0
            })
        );
    }

    #[test]
    fn a_reading_without_both_counts_is_no_reading() {
        assert_eq!(Starts::parse("clock_info:\n  reset_count: 95\n"), None);
    }

    #[test]
    fn the_counts_come_back_as_they_were_kept() {
        let starts = Starts {
            reset: 95,
            restart: 3,
        };
        assert_eq!(Starts::unkept(&starts.kept()), Some(starts));
        assert_eq!(Starts::unkept("95"), None);
        assert_eq!(Starts::unkept("95 3 1"), None);
    }

    #[test]
    fn a_firmware_start_moves_one_count_and_a_wake_neither() {
        let then = Starts {
            reset: 95,
            restart: 0,
        };
        let started = |reset, restart| then.started_since(Starts { reset, restart });
        assert_eq!(
            (started(95, 1), started(96, 0), started(95, 0)),
            (true, true, false),
            "a moved restart or reset count wasn't taken for a firmware start, or \
             unmoved counts were"
        );
    }

    #[test]
    fn a_disk_whose_tokens_all_ask_is_typed() {
        // The three the guide enrols, the recovery key's as its installed
        // header in modules/guide/fixtures/qemu-canokey has it.
        let header = json!({"tokens": {
            "0": {"type": "systemd-tpm2", "tpm2-pin": true},
            "1": {"type": "systemd-fido2", "fido2-clientPin-required": true},
            "2": {"type": "systemd-recovery", "keyslots": ["3"]}
        }});
        assert_eq!(unasked(&header), None);
        assert_eq!(
            unasked(&json!({"tokens": {}})),
            None,
            "a password alone is typed"
        );
    }

    #[test]
    fn a_disk_any_token_opens_unasked_is_not() {
        for token in [
            json!({"type": "systemd-tpm2", "tpm2-pin": false}),
            json!({"type": "systemd-tpm2"}),
            json!({"type": "systemd-fido2", "fido2-clientPin-required": false}),
            json!({"type": "systemd-pkcs11"}),
        ] {
            let kind = token["type"].as_str().unwrap_or_default().to_owned();
            let header =
                json!({"tokens": {"0": {"type": "systemd-tpm2", "tpm2-pin": true}, "1": token}});
            assert_eq!(
                unasked(&header),
                Some(format!("token 1 ({kind}) opens it unasked")),
            );
        }
        assert_eq!(
            unasked(&json!({})),
            Some("its header lists no tokens".to_owned())
        );
    }
}
