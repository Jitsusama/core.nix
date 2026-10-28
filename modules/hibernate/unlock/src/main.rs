//! Lets the lock go after a resume from hibernation, since the disk prompt
//! has just asked for Joel's password, as he chose: one prompt, not two. A
//! wake from sleep keeps its lock.
//!
//! ```text
//! hibernate-unlock pre|post <action> [<disk>...]
//! ```
//!
//! systemd-sleep runs it before and after each sleep, naming the action
//! (`suspend`, `hibernate`, `hybrid-sleep` or `suspend-then-hibernate`), and
//! the disks are those the initrd opens. Before anything that writes an
//! image it keeps the TPM's reset and restart counts in `/run`, which the
//! image keeps too. After, it lets every session's lock go only when both
//! hold:
//!
//! - A count went up, so the firmware started the machine: the image was
//!   read back, through the initrd, rather than a hybrid sleep waking from
//!   its sleep. Linux tells the TPM to save its state going down, so the
//!   firmware's start counts as a restart, not a reset; a wake from sleep
//!   moves neither.
//! - Every disk can only be opened with something typed: each TPM token
//!   asks for a PIN, each FIDO2 token for its PIN, and any other way in is a
//!   password. A disk the TPM opens alone would let anyone through, and
//!   with no disk named nothing is let go.
//!
//! It asks `tpm2_readclock` for the counts, `cryptsetup` for each disk's
//! tokens and `loginctl` to let the locks go, each from `PATH`. Anything it
//! can't read keeps the lock, and it never fails the sleep.

use serde_json::Value;
use std::fs;
use std::process::{Command, ExitCode};

/// Where the counts from before the image was written are kept.
const KEPT: &str = "/run/hibernate-unlock";

/// The TPM's counts of the times it started afresh and the times it started
/// from saved state, the two a firmware start moves.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
struct Starts {
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
    fn kept(self) -> String {
        format!("{} {}\n", self.reset, self.restart)
    }

    fn unkept(text: &str) -> Option<Self> {
        let mut counts = text.split_whitespace().map(str::parse);
        let (Some(Ok(reset)), Some(Ok(restart)), None) =
            (counts.next(), counts.next(), counts.next())
        else {
            return None;
        };
        Some(Self { reset, restart })
    }

    /// Whether the firmware started the machine between `self` and `now`.
    fn started_since(self, now: Self) -> bool {
        now.reset > self.reset || now.restart > self.restart
    }
}

/// Whether a sleep writes an image the machine can start again from.
fn writes_an_image(action: &str) -> bool {
    matches!(
        action,
        "hibernate" | "hybrid-sleep" | "suspend-then-hibernate"
    )
}

/// Whether every token in a disk's LUKS2 header, as `cryptsetup luksDump
/// --dump-json-metadata` prints it, needs something typed. A keyslot with
/// no token is a password.
fn only_typed(header: &Value) -> bool {
    let Some(tokens) = header["tokens"].as_object() else {
        return false;
    };
    tokens.values().all(|token| match token["type"].as_str() {
        Some("systemd-tpm2") => token["tpm2-pin"] == true,
        Some("systemd-fido2") => token["fido2-clientPin-required"] == true,
        _ => false,
    })
}

/// What a command printed, or nothing if it couldn't run or failed.
fn said(program: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(program).args(args).output().ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).into_owned())
}

fn starts() -> Option<Starts> {
    Starts::parse(&said("tpm2_readclock", &[])?)
}

fn disk_is_typed(disk: &str) -> bool {
    said("cryptsetup", &["luksDump", "--dump-json-metadata", disk])
        .and_then(|json| serde_json::from_str(&json).ok())
        .is_some_and(|header| only_typed(&header))
}

/// Before an image is written, keep the TPM's counts. Before a sleep that
/// writes none, forget any kept from a hibernate that never happened.
fn before(action: &str) {
    let _ = fs::remove_file(KEPT);
    if !writes_an_image(action) {
        return;
    }
    match starts() {
        Some(now) => {
            if let Err(error) = fs::write(KEPT, now.kept()) {
                eprintln!("hibernate-unlock: couldn't keep the TPM's counts: {error}");
            }
        }
        None => eprintln!("hibernate-unlock: couldn't read the TPM's counts; the lock stays"),
    }
}

/// After the machine is back, let the locks go if it came back through the
/// disk prompt.
fn after(disks: &[String]) {
    let Some(then) = fs::read_to_string(KEPT)
        .ok()
        .and_then(|t| Starts::unkept(&t))
    else {
        return;
    };
    let _ = fs::remove_file(KEPT);
    let Some(now) = starts() else {
        eprintln!("hibernate-unlock: couldn't read the TPM's counts; the lock stays");
        return;
    };
    if !then.started_since(now) {
        eprintln!("hibernate-unlock: woke without a start, so through no prompt; the lock stays");
        return;
    }
    if disks.is_empty() || !disks.iter().all(|disk| disk_is_typed(disk)) {
        eprintln!("hibernate-unlock: a disk opens without a typed secret; the lock stays");
        return;
    }
    match said("loginctl", &["unlock-sessions"]) {
        Some(_) => eprintln!("hibernate-unlock: back through the disk prompt; the lock is let go"),
        None => eprintln!("hibernate-unlock: loginctl couldn't let the lock go"),
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [when, action, ..] if when == "pre" => before(action),
        [when, _, disks @ ..] if when == "post" => after(disks),
        _ => {
            eprintln!("usage: hibernate-unlock pre|post <action> [<disk>...]");
            return ExitCode::from(2);
        }
    }
    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::*;
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
        assert!(then.started_since(Starts {
            reset: 95,
            restart: 1
        }));
        assert!(then.started_since(Starts {
            reset: 96,
            restart: 0
        }));
        assert!(!then.started_since(then));
    }

    #[test]
    fn only_sleeps_that_write_an_image_are_kept() {
        for action in ["hibernate", "hybrid-sleep", "suspend-then-hibernate"] {
            assert!(writes_an_image(action), "{action}");
        }
        assert!(!writes_an_image("suspend"));
    }

    #[test]
    fn a_disk_whose_tokens_all_ask_is_typed() {
        let header = json!({"tokens": {
            "0": {"type": "systemd-tpm2", "tpm2-pin": true},
            "1": {"type": "systemd-fido2", "fido2-clientPin-required": true}
        }});
        assert!(only_typed(&header));
        assert!(
            only_typed(&json!({"tokens": {}})),
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
            let header =
                json!({"tokens": {"0": {"type": "systemd-tpm2", "tpm2-pin": true}, "1": token}});
            assert!(!only_typed(&header), "{token}");
        }
        assert!(!only_typed(&json!({})), "a header it can't read");
    }
}
