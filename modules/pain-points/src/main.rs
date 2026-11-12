//! Names the interactions with the launcher that cost Joel most today,
//! so what slows him down is read from his own use rather than guessed
//! (Joel, 2026-11-11: "identifying what i'm trying to do, and what slows
//! me down from getting there, particuularly the launcher").
//!
//! `pain-points read` reads the launcher's own record of each opening
//! (launcher-trials.jsonl, under the shell's state) and its journal's
//! "opening usable" lines, works out what each of today's openings lost
//! him (src/pain.rs says how), and writes the worst, a line each, to
//! `$XDG_STATE_HOME/pain-points/<local day>.jsonl`, rewritten whole each
//! time so the file always says the day so far. It keeps kinds and
//! timings alone: what he typed, copied or opened by name never leaves
//! the launcher's own record.

mod pain;
mod trial;

use pain::{Usable, Usual};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

/// How far back his usual time for each kind of pick is drawn from.
const USUAL_DAYS: i64 = 14;
const DAY_MS: i64 = 86_400_000;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.first().map(String::as_str) != Some("read") {
        eprintln!(
            "usage: pain-points read [--trials FILE] [--journalctl PATH] [--date PATH]"
        );
        std::process::exit(64);
    }
    let given = |flag: &str| {
        args.windows(2)
            .find(|pair| pair[0] == flag)
            .map(|pair| PathBuf::from(&pair[1]))
    };
    let journalctl = given("--journalctl").unwrap_or_else(|| "journalctl".into());
    let date = given("--date").unwrap_or_else(|| "date".into());
    let Some(trials) = given("--trials").or_else(newest_trials) else {
        eprintln!("pain-points: the launcher has kept no trials yet");
        return;
    };
    if let Err(error) = run(&trials, &journalctl, &date) {
        eprintln!("pain-points: {error}");
        std::process::exit(1);
    }
}

fn run(trials: &Path, journalctl: &Path, date: &Path) -> Result<(), String> {
    let text = fs::read_to_string(trials).map_err(|e| format!("{}: {e}", trials.display()))?;
    let every = trial::read(&text);
    let offset = output(Command::new(date).arg("+%z"))
        .as_deref()
        .and_then(offset_of)
        .unwrap_or(0);
    let (name, start) = local_day(millis_now(), offset);
    let before: Vec<_> = every
        .iter()
        .filter(|t| t.at < start && t.at >= start - USUAL_DAYS * DAY_MS)
        .cloned()
        .collect();
    let today: Vec<_> = every.iter().filter(|t| t.at >= start).cloned().collect();
    let usable = Usable::new(
        output(
            Command::new(journalctl)
                .args(["--user", "-u", "quickshell.service", "-o", "json", "--since", "today"])
                .args(["-g", "launcher: opening usable"]),
        )
        .unwrap_or_default()
        .lines()
        .filter_map(journal_entry)
        .filter_map(|(at, message)| pain::usable_of(&message).map(|ms| (at, ms)))
        .collect(),
    );
    let named = pain::painful(&today, &usable, &Usual::of(&before));
    let lines: String = named
        .iter()
        .map(|p| serde_json::to_string(p).map(|line| line + "\n"))
        .collect::<Result<_, _>>()
        .map_err(|e| e.to_string())?;
    let dir = state_home().join("pain-points");
    fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let file = dir.join(format!("{name}.jsonl"));
    let partial = dir.join(format!(".{name}.jsonl.part"));
    fs::write(&partial, lines).map_err(|e| format!("{}: {e}", partial.display()))?;
    fs::rename(&partial, &file).map_err(|e| format!("{}: {e}", file.display()))?;
    match named.first() {
        Some(worst) => println!(
            "pain-points: {} of {} openings today cost him a second or more; the worst, {}, lost {} s ({})",
            named.len(),
            today.len(),
            worst.trying,
            worst.lost_seconds,
            worst.slowed
        ),
        None => println!("pain-points: none of {} openings today cost him a second", today.len()),
    }
    Ok(())
}

/// What a command printed, where it ran and said anything.
fn output(command: &mut Command) -> Option<String> {
    let out = command.stdin(Stdio::null()).stderr(Stdio::null()).output().ok()?;
    String::from_utf8(out.stdout).ok().filter(|s| !s.is_empty())
}

fn state_home() -> PathBuf {
    env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env::var_os("HOME").unwrap_or_default()).join(".local/state"))
}

/// The launcher's record from the shell that wrote it last: the shell
/// keeps its state in a folder named for its config, so the newest is the
/// one running.
fn newest_trials() -> Option<PathBuf> {
    fs::read_dir(state_home().join("quickshell/by-shell"))
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.path().join("launcher-trials.jsonl"))
        .filter_map(|path| Some((fs::metadata(&path).ok()?.modified().ok()?, path)))
        .max()
        .map(|(_, path)| path)
}

fn millis_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
}

/// The offset from UTC, in seconds, from what `date +%z` prints ("-0500").
fn offset_of(_printed: &str) -> Option<i64> {
    None
}

/// The local day holding `at` (milliseconds since 1970) at `offset`
/// seconds from UTC: its name, YYYY-MM-DD, and when it began, in
/// milliseconds since 1970.
fn local_day(_at: i64, _offset: i64) -> (String, i64) {
    (String::new(), 0)
}

/// A journal entry printed by `journalctl -o json`: when it was logged, in
/// milliseconds since 1970, and its message.
fn journal_entry(_line: &str) -> Option<(i64, String)> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_offset_reads_from_date_s_own_form() {
        assert_eq!(offset_of("-0500\n"), Some(-18_000));
        assert_eq!(offset_of("+0530"), Some(19_800));
        assert_eq!(offset_of("EST"), None);
    }

    #[test]
    fn a_day_is_named_and_begun_in_local_time() {
        // 2026-11-12 03:30 UTC is still the 11th at 22:30 in Toronto.
        let at = 1_794_454_200_000;
        assert_eq!(local_day(at, -18_000), ("2026-11-11".to_string(), 1_794_373_200_000));
        assert_eq!(local_day(at, 0), ("2026-11-12".to_string(), 1_794_441_600_000));
    }

    #[test]
    fn a_journal_entry_gives_its_time_and_message() {
        let line = r#"{"__REALTIME_TIMESTAMP":"1794455125123456","MESSAGE":"  INFO qml: launcher: opening usable 195 ms from the chord, its longest stall 48 ms","_PID":"1"}"#;
        let (at, message) = journal_entry(line).expect("the entry reads");
        assert_eq!(at, 1_794_455_125_123);
        assert_eq!(pain::usable_of(&message), Some(195.0));
        assert_eq!(journal_entry(r#"{"MESSAGE":[1,2,3]}"#), None);
    }
}
