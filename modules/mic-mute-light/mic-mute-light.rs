//! Lights a laptop's mic-mute light while the default microphone is muted.
//!
//! ```text
//! mic-mute-light
//! ```
//!
//! The kernel's own trigger for the light follows only a sound card's
//! capture switch, and a laptop's built-in microphones reached through its
//! DSP have none: PipeWire mutes them itself, so the trigger never fires.
//! This follows PipeWire instead. It reads `pw-dump --monitor`, which prints
//! the whole graph as a JSON array at the start and then an array for each
//! change, and after an array that touches a mute or the default source it
//! asks `wpctl` whether the default source is muted. It sets the light
//! through logind with `brightnessctl`, as the person at the seat may, and
//! only when that changes. Setting it can fail for a moment while the
//! person's session starts at a boot, so a light it couldn't set is asked
//! after again at the next array of any kind, rather than left wrong until
//! the mute next changes. It sleeps on
//! the pipe between changes, so it
//! costs nothing while nothing changes, and it ends when PipeWire goes, for
//! systemd to start it again.
//!
//! It needs only the standard library.

use std::io::{BufRead, BufReader};
use std::process::{Command, ExitCode, Stdio};

/// The light, by the name every laptop driver that has one gives it.
const LIGHT: &str = "platform::micmute";

/// Whether a line ends one of `pw-dump`'s arrays: its closing bracket,
/// alone at the start of a line, as only the outermost one is.
fn ends_an_array(line: &str) -> bool {
    line == "]"
}

/// Whether a line in an array could change what the light should say: a
/// node's mute, or which source is the default.
fn touches_the_mute(line: &str) -> bool {
    line.contains("\"mute\"") || line.contains("default.audio.source")
}

/// Whether an array's end calls for asking after the mute: it touched
/// the mute, or the light has yet to be set.
fn asks_again(touched: bool, lit: Option<bool>) -> bool {
    touched || lit.is_none()
}

/// Whether `wpctl get-volume` said the source is muted.
fn muted(said: &str) -> bool {
    said.contains("[MUTED]")
}

/// The brightness the light is set to, for a source muted or not.
fn brightness(muted: bool) -> &'static str {
    if muted { "1" } else { "0" }
}

/// Whether the default source is muted now. With no default source, or no
/// answer, it isn't: a light lit for a microphone that isn't there would
/// say something false.
fn source_muted() -> bool {
    Command::new("wpctl")
        .args(["get-volume", "@DEFAULT_AUDIO_SOURCE@"])
        .stderr(Stdio::null())
        .output()
        .is_ok_and(|out| out.status.success() && muted(&String::from_utf8_lossy(&out.stdout)))
}

/// Sets the light, saying so on stderr when it can't.
fn set_light(muted: bool) -> bool {
    let set = Command::new("brightnessctl")
        .args([
            "--quiet",
            &format!("--device={LIGHT}"),
            "set",
            brightness(muted),
        ])
        .status();
    match set {
        Ok(status) if status.success() => true,
        Ok(status) => {
            eprintln!("mic-mute-light: brightnessctl failed setting {LIGHT}: {status}");
            false
        }
        Err(err) => {
            eprintln!("mic-mute-light: can't run brightnessctl: {err}");
            false
        }
    }
}

fn main() -> ExitCode {
    if std::env::args().len() > 1 {
        eprintln!("usage: mic-mute-light");
        return ExitCode::from(2);
    }
    let mut dump = match Command::new("pw-dump")
        .args(["--monitor", "--no-colors"])
        .stdout(Stdio::piped())
        .spawn()
    {
        Ok(dump) => dump,
        Err(err) => {
            eprintln!("mic-mute-light: can't run pw-dump: {err}");
            return ExitCode::FAILURE;
        }
    };
    let Some(out) = dump.stdout.take() else {
        return ExitCode::FAILURE;
    };
    let mut lit: Option<bool> = None;
    let mut touched = false;
    for line in BufReader::new(out).lines() {
        let Ok(line) = line else { break };
        touched |= touches_the_mute(&line);
        if ends_an_array(&line) && asks_again(touched, lit) {
            touched = false;
            let now = source_muted();
            if lit != Some(now) {
                lit = set_light(now).then_some(now);
            }
        }
    }
    // PipeWire went, so what the light says can no longer be known.
    set_light(false);
    let _ = dump.wait();
    eprintln!("mic-mute-light: pw-dump ended");
    ExitCode::FAILURE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_outermost_closing_bracket_ends_an_array() {
        assert!(ends_an_array("]"));
        assert!(!ends_an_array("  ]"));
        assert!(!ends_an_array("    \"permissions\": [ \"r\" ],"));
    }

    #[test]
    fn a_mute_or_the_default_source_touches_the_mute() {
        assert!(touches_the_mute("            \"mute\": true,"));
        assert!(touches_the_mute(
            "        \"key\": \"default.audio.source\","
        ));
        assert!(!touches_the_mute("            \"volume\": 1.0,"));
        assert!(!touches_the_mute(
            "        \"key\": \"default.audio.sink\","
        ));
    }

    #[test]
    fn a_light_not_yet_set_is_asked_after_at_any_array() {
        assert!(asks_again(false, None));
        assert!(asks_again(true, Some(false)));
        assert!(!asks_again(false, Some(true)));
    }

    #[test]
    fn wpctl_says_muted_only_with_its_mark() {
        assert!(muted("Volume: 1.00 [MUTED]\n"));
        assert!(!muted("Volume: 1.00\n"));
        assert!(!muted(""));
    }

    #[test]
    fn a_muted_source_lights_it_and_an_unmuted_one_puts_it_out() {
        assert_eq!(brightness(true), "1");
        assert_eq!(brightness(false), "0");
    }
}
