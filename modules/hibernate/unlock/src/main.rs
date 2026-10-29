//! Lets the lock go after a resume from hibernation, since the disk prompt
//! has just asked for Joel's password, as he chose: one prompt, not two. A
//! wake from sleep keeps its lock. And a hibernation that rolled back, with
//! the machine never off, is never left on in silence.
//!
//! ```text
//! hibernate-unlock pre|post <action> [<disk>...]
//! hibernate-unlock rescue
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
//!   asks for a PIN, each FIDO2 token for its PIN, a recovery key is typed,
//!   and a keyslot with no token is a password. Any other token keeps the
//!   lock, and the log names it. A disk the TPM opens alone would let anyone
//!   through, and with no disk named nothing is let go.
//!
//! It asks `tpm2_readclock` for the counts, `cryptsetup` for each disk's
//! tokens and `loginctl` to let the locks go, each from `PATH`. Anything it
//! can't read keeps the lock, and it never fails the sleep.
//!
//! A hibernation can write its image and still roll back, when a device
//! refuses to power off, and the kernel then tells systemd it worked. Its
//! sign is a hibernation back with neither count moved. Ten seconds later,
//! once the sleep is over, `rescue` runs on its own: with the lid shut
//! nobody is looking, and a laptop left on in a bag runs flat and hot, so it
//! hibernates once more, and powers off if that rolls back too. With the
//! lid open, a notice tells whoever is signed in.

use serde_json::Value;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

/// Where the counts from before the image was written are kept.
const KEPT: &str = "/run/hibernate-unlock";

/// Kept while a rolled-back hibernation is tried again, so a second
/// rollback powers off rather than trying forever.
const TRIED_AGAIN: &str = "/run/hibernate-unlock-tried-again";

/// What a rescue does once a hibernation rolled back.
#[derive(Debug, PartialEq, Eq)]
enum Rescue {
    HibernateAgain,
    PowerOff,
    Say,
}

/// With the lid open someone is there to be told; with it shut, one more
/// try, then off.
fn rescue(lid_shut: bool, tried_again: bool) -> Rescue {
    match (lid_shut, tried_again) {
        (false, _) => Rescue::Say,
        (true, false) => Rescue::HibernateAgain,
        (true, true) => Rescue::PowerOff,
    }
}

/// Whether a lid's `/proc/acpi/button/lid/*/state` says it's shut.
fn says_shut(state: &str) -> bool {
    state.split_whitespace().last() == Some("closed")
}

/// The users signed in at a seat, from `loginctl list-sessions --json=short`.
fn seated(sessions: &Value) -> Vec<String> {
    let mut users: Vec<String> = sessions
        .as_array()
        .into_iter()
        .flatten()
        .filter(|session| {
            session["seat"]
                .as_str()
                .is_some_and(|seat| !seat.is_empty())
        })
        .filter_map(|session| session["user"].as_str().map(str::to_owned))
        .collect();
    users.sort();
    users.dedup();
    users
}

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

/// Why a disk could open without the prompt asking for anything, or
/// nothing when it couldn't.
fn disk_unasked(disk: &str) -> Option<String> {
    let header = said("cryptsetup", &["luksDump", "--dump-json-metadata", disk])
        .and_then(|json| serde_json::from_str::<Value>(&json).ok());
    match header {
        Some(header) => unasked(&header),
        None => Some("its header couldn't be read".to_owned()),
    }
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
/// disk prompt, and rescue a hibernation that never went off.
fn after(action: &str, disks: &[String]) {
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
        if action == "hibernate" {
            rolled_back();
        } else {
            eprintln!(
                "hibernate-unlock: woke without a start, so through no prompt; the lock stays"
            );
        }
        return;
    }
    let _ = fs::remove_file(TRIED_AGAIN);
    if disks.is_empty() {
        eprintln!("hibernate-unlock: no disk asked at the prompt; the lock stays");
        return;
    }
    if let Some((disk, why)) = disks
        .iter()
        .find_map(|disk| disk_unasked(disk).map(|why| (disk, why)))
    {
        eprintln!("hibernate-unlock: {disk}: {why}; the lock stays");
        return;
    }
    match said("loginctl", &["unlock-sessions"]) {
        Some(_) => eprintln!("hibernate-unlock: back through the disk prompt; the lock is let go"),
        None => eprintln!("hibernate-unlock: loginctl couldn't let the lock go"),
    }
}

/// Asks systemd to run the rescue in ten seconds, once this sleep is over
/// and the sessions are thawed, with the same programs on its `PATH`.
fn rolled_back() {
    eprintln!("hibernate-unlock: the hibernation rolled back, so the machine is still on");
    let Ok(me) = env::current_exe() else {
        eprintln!("hibernate-unlock: couldn't find itself to rescue the machine");
        return;
    };
    let path = format!("--setenv=PATH={}", env::var("PATH").unwrap_or_default());
    let me = me.to_string_lossy();
    let args = [
        "--on-active=10s",
        "--collect",
        "--quiet",
        "--description=Rescues a hibernation that rolled back",
        &path,
        &me,
        "rescue",
    ];
    if said("systemd-run", &args).is_none() {
        eprintln!("hibernate-unlock: systemd-run couldn't start the rescue");
    }
}

fn lid_shut() -> bool {
    fs::read_dir("/proc/acpi/button/lid")
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|lid| fs::read_to_string(lid.path().join("state")).ok())
        .any(|state| says_shut(&state))
}

fn on_path(program: &str) -> Option<PathBuf> {
    let path = env::var_os("PATH")?;
    env::split_paths(&path)
        .map(|dir| dir.join(program))
        .find(|candidate| candidate.is_file())
}

/// Puts a notice in each seated user's session, through their own systemd.
fn say_to_each_session() {
    let Some(notify) = on_path("notify-send") else {
        eprintln!("hibernate-unlock: no notify-send to say the hibernation didn't finish");
        return;
    };
    let notify = notify.to_string_lossy();
    let sessions = said("loginctl", &["list-sessions", "--json=short"])
        .and_then(|json| serde_json::from_str(&json).ok())
        .unwrap_or(Value::Null);
    for user in seated(&sessions) {
        let machine = format!("--machine={user}@.host");
        let args = [
            machine.as_str(),
            "--user",
            "--collect",
            "--quiet",
            &notify,
            "--urgency=critical",
            "--app-name=Hibernation",
            "Hibernation didn't finish",
            "The laptop is still on. Hibernate it again, or shut it down.",
        ];
        if said("systemd-run", &args).is_none() {
            eprintln!("hibernate-unlock: couldn't tell {user} the hibernation didn't finish");
        }
    }
}

/// What the rescue ten seconds after a rollback does.
fn rescue_now() {
    let tried_again = Path::new(TRIED_AGAIN).exists();
    match rescue(lid_shut(), tried_again) {
        Rescue::HibernateAgain => {
            eprintln!("hibernate-unlock: the lid is shut, so it hibernates again");
            let _ = fs::write(TRIED_AGAIN, "");
            let _ = said("systemctl", &["hibernate", "--check-inhibitors=no"]);
        }
        Rescue::PowerOff => {
            eprintln!(
                "hibernate-unlock: the lid is shut and it rolled back twice, so it powers off"
            );
            let _ = fs::remove_file(TRIED_AGAIN);
            let _ = said("systemctl", &["poweroff", "--check-inhibitors=no"]);
        }
        Rescue::Say => {
            eprintln!("hibernate-unlock: the lid is open, so it says so");
            let _ = fs::remove_file(TRIED_AGAIN);
            say_to_each_session();
        }
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    match args.as_slice() {
        [when, action, ..] if when == "pre" => before(action),
        [when, action, disks @ ..] if when == "post" => after(action, disks),
        [what] if what == "rescue" => rescue_now(),
        _ => {
            eprintln!("usage: hibernate-unlock pre|post <action> [<disk>...] | rescue");
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
    fn a_rollback_with_the_lid_open_is_said() {
        assert_eq!(rescue(false, false), Rescue::Say);
        assert_eq!(rescue(false, true), Rescue::Say);
    }

    #[test]
    fn a_rollback_with_the_lid_shut_tries_once_then_powers_off() {
        assert_eq!(rescue(true, false), Rescue::HibernateAgain);
        assert_eq!(rescue(true, true), Rescue::PowerOff);
    }

    #[test]
    fn the_lid_reads_shut_only_when_it_says_closed() {
        assert!(says_shut("state:      closed\n"));
        assert!(!says_shut("state:      open\n"));
        assert!(!says_shut(""));
    }

    #[test]
    fn only_users_at_a_seat_are_told_once_each() {
        let sessions = json!([
            {"session": "1", "user": "joel", "seat": "seat0"},
            {"session": "2", "user": "joel", "seat": "seat0"},
            {"session": "3", "user": "ssh", "seat": null},
            {"session": "4", "user": "cron", "seat": ""}
        ]);
        assert_eq!(seated(&sessions), vec!["joel".to_owned()]);
        assert!(seated(&Value::Null).is_empty());
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
