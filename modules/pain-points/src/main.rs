//! Names the interactions with the launcher and pi that slowed Joel most
//! today, so what slows him down is read from his own use rather than
//! guessed (Joel, 2026-11-11: "identifying what i'm trying to do, and what
//! slows me down from getting there, particuularly the launcher").
//!
//! `pain-points read` reads the launcher's own record of each opening
//! (launcher-trials.jsonl, under the shell's state) and its journal's
//! "opening usable" lines, and what's new in pi's session logs since its
//! last read, works out what each of today's openings and messages lost
//! him (src/pain.rs and src/pi.rs say how), and writes the worst, a line
//! each, to `$XDG_STATE_HOME/pain-points/<local day>.jsonl`, rewritten
//! whole each time so the file always says the day so far. It keeps kinds
//! and timings alone: what he typed, copied or opened by name never
//! leaves the launcher's own record, and what he typed to pi is never
//! written or sent anywhere.

mod pain;
mod pi;
mod trial;
mod words;

use pain::{Painful, Usable, Usual};
use pi::Log;
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

/// How far back his usual time for each kind of pick is drawn from.
const USUAL_DAYS: i64 = 14;
const DAY_MS: i64 = 86_400_000;

/// Where a run reads from and writes to, and the time it counts the day
/// of.
struct Run {
    trials: Option<PathBuf>,
    sessions: PathBuf,
    journalctl: PathBuf,
    date: PathBuf,
    state: PathBuf,
    at: i64,
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.first().map(String::as_str) != Some("read") {
        eprintln!(
            "usage: pain-points read [--trials FILE] [--sessions DIR] [--journalctl PATH] \
             [--date PATH] [--at MILLIS]"
        );
        std::process::exit(64);
    }
    let given = |flag: &str| {
        args.windows(2)
            .find(|pair| pair[0] == flag)
            .map(|pair| pair[1].clone())
    };
    let home = PathBuf::from(env::var_os("HOME").unwrap_or_default());
    let run = Run {
        trials: given("--trials").map(PathBuf::from).or_else(newest_trials),
        sessions: given("--sessions")
            .map_or_else(|| home.join(".pi/agent/sessions"), PathBuf::from),
        journalctl: given("--journalctl").map_or_else(|| "journalctl".into(), PathBuf::from),
        date: given("--date").map_or_else(|| "date".into(), PathBuf::from),
        state: state_home().join("pain-points"),
        // A past day can be counted again, to try a change on it.
        at: given("--at")
            .and_then(|at| at.parse().ok())
            .unwrap_or_else(millis_now),
    };
    match read(&run) {
        Ok(said) => println!("{said}"),
        Err(error) => {
            eprintln!("pain-points: {error}");
            std::process::exit(1);
        }
    }
}

/// Writes the day file and says, in a line for the journal, how the day
/// stands.
fn read(run: &Run) -> Result<String, String> {
    let offset = output(Command::new(&run.date).arg("+%z"))
        .as_deref()
        .and_then(offset_of)
        .unwrap_or(0);
    let (name, start) = local_day(run.at, offset);
    let until = start + DAY_MS;
    let (launcher, openings) = match &run.trials {
        Some(trials) => launcher(trials, &run.journalctl, start, until)?,
        None => (Vec::new(), 0),
    };
    fs::create_dir_all(&run.state).map_err(|e| format!("{}: {e}", run.state.display()))?;
    let logs = follow_pi(&run.sessions, &run.state.join("pi-logs.json"), start)?;
    let turns: Vec<&[pi::Turn]> = logs.values().map(|log| log.turns.as_slice()).collect();
    let messages = turns
        .iter()
        .flat_map(|t| t.iter())
        .filter(|t| (start..until).contains(&t.enter))
        .count();
    let named_pi = pi::painful(&turns, start, until);
    let (from_launcher, from_pi) = (launcher.len(), named_pi.len());
    let named = pain::ranked(launcher.into_iter().chain(named_pi).collect());
    let lines: String = named
        .iter()
        .map(|p| serde_json::to_string(p).map(|line| line + "\n"))
        .collect::<Result<_, _>>()
        .map_err(|e| e.to_string())?;
    write_whole(&run.state.join(format!("{name}.jsonl")), &lines)?;
    Ok(summary(
        &named,
        (from_launcher, openings),
        (from_pi, messages),
    ))
}

/// The journal's line for the day: how many openings and messages lost
/// him a second or more, of how many, and the worst.
fn summary(named: &[Painful], launcher: (usize, usize), pi: (usize, usize)) -> String {
    let counted = format!(
        "{} of {} openings and {} of {} messages to pi today lost him a second or more",
        launcher.0, launcher.1, pi.0, pi.1
    );
    match named.first() {
        Some(worst) => format!(
            "pain-points: {counted}; the worst, {} in {}, lost {} s ({})",
            worst.trying, worst.place, worst.lost_seconds, worst.slowed
        ),
        None => format!("pain-points: {counted}"),
    }
}

/// Brings each session log written today up to date in the reading state
/// kept at `state`, and gives every log's turns. A log is named in the
/// state by its file's name alone, which says when it began, not where.
fn follow_pi(sessions: &Path, state: &Path, since: i64) -> Result<BTreeMap<String, Log>, String> {
    let mut logs: BTreeMap<String, Log> = fs::read(state)
        .ok()
        .and_then(|kept| serde_json::from_slice(&kept).ok())
        .unwrap_or_default();
    let mut present = Vec::new();
    for folder in fs::read_dir(sessions)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
    {
        for entry in fs::read_dir(folder.path())
            .into_iter()
            .flatten()
            .filter_map(Result::ok)
        {
            let path = entry.path();
            let Some(name) = path
                .file_name()
                .and_then(|n| n.to_str())
                .map(str::to_string)
            else {
                continue;
            };
            if path.extension().is_none_or(|e| e != "jsonl") {
                continue;
            }
            present.push(name.clone());
            let written = entry.metadata().and_then(|m| m.modified()).map_or(0, |t| {
                t.duration_since(UNIX_EPOCH)
                    .map_or(0, |d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
            });
            if written < since {
                continue;
            }
            let log = logs.entry(name).or_default();
            if let Ok(mut file) = fs::File::open(&path) {
                // A log that can't be read now is read on the next run.
                let _ = pi::follow(&mut file, log);
            }
        }
    }
    logs.retain(|name, _| present.contains(name));
    for log in logs.values_mut() {
        log.prune(since);
    }
    let kept = serde_json::to_string(&logs).map_err(|e| e.to_string())?;
    write_whole(state, &kept)?;
    Ok(logs)
}

/// Writes a file whole, so a reader never sees half of it.
fn write_whole(file: &Path, text: &str) -> Result<(), String> {
    let name = file.file_name().and_then(|n| n.to_str()).unwrap_or("file");
    let partial = file.with_file_name(format!(".{name}.part"));
    fs::write(&partial, text).map_err(|e| format!("{}: {e}", partial.display()))?;
    fs::rename(&partial, file).map_err(|e| format!("{}: {e}", file.display()))
}

/// The launcher's named openings for the day in [start, until), and how
/// many openings it had.
fn launcher(
    trials: &Path,
    journalctl: &Path,
    start: i64,
    until: i64,
) -> Result<(Vec<Painful>, usize), String> {
    let text = fs::read_to_string(trials).map_err(|e| format!("{}: {e}", trials.display()))?;
    let every = trial::read(&text);
    // His usual for a kind of pick counts today's too, so one he began
    // making today is held to his own pace once he has made it a few
    // times, rather than to the spec.
    let fortnight: Vec<_> = every
        .iter()
        .filter(|t| (start - USUAL_DAYS * DAY_MS..until).contains(&t.at))
        .cloned()
        .collect();
    let today: Vec<_> = every
        .iter()
        .filter(|t| (start..until).contains(&t.at))
        .cloned()
        .collect();
    let usable = Usable::new(
        output(
            Command::new(journalctl)
                .args(["--user", "-u", "quickshell.service", "-o", "json"])
                .arg(format!("--since=@{}", start / 1000))
                .arg(format!("--until=@{}", until / 1000))
                .args(["-g", "launcher: opening usable"]),
        )
        .unwrap_or_default()
        .lines()
        .filter_map(journal_entry)
        .filter_map(|(at, message)| pain::usable_of(&message).map(|ms| (at, ms)))
        .collect(),
    );
    Ok((
        pain::painful(&today, &usable, &Usual::of(&fortnight)),
        today.len(),
    ))
}

/// What a command printed, where it ran and said anything.
fn output(command: &mut Command) -> Option<String> {
    let out = command
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    String::from_utf8(out.stdout).ok().filter(|s| !s.is_empty())
}

fn state_home() -> PathBuf {
    env::var_os("XDG_STATE_HOME").map_or_else(
        || PathBuf::from(env::var_os("HOME").unwrap_or_default()).join(".local/state"),
        PathBuf::from,
    )
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
fn offset_of(printed: &str) -> Option<i64> {
    let printed = printed.trim();
    let sign = match printed.chars().next()? {
        '+' => 1,
        '-' => -1,
        _ => return None,
    };
    let digits = printed
        .get(1..5)
        .filter(|d| d.len() == 4 && printed.len() == 5)?;
    let hours: i64 = digits[..2].parse().ok()?;
    let minutes: i64 = digits[2..].parse().ok()?;
    Some(sign * (hours * 3600 + minutes * 60))
}

/// The local day holding `at` (milliseconds since 1970) at `offset`
/// seconds from UTC: its name, YYYY-MM-DD, and when it began, in
/// milliseconds since 1970.
fn local_day(at: i64, offset: i64) -> (String, i64) {
    let local = at + offset * 1000;
    let days = local.div_euclid(DAY_MS);
    // Days since 1970 as a civil date, by Howard Hinnant's algorithm.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    (
        format!("{y:04}-{m:02}-{d:02}"),
        days * DAY_MS - offset * 1000,
    )
}

/// A journal entry printed by `journalctl -o json`: when it was logged, in
/// milliseconds since 1970, and its message.
fn journal_entry(line: &str) -> Option<(i64, String)> {
    let entry: serde_json::Value = serde_json::from_str(line).ok()?;
    let micros: i64 = entry.get("__REALTIME_TIMESTAMP")?.as_str()?.parse().ok()?;
    // The shell colours its lines, and journald keeps a message holding
    // escapes as an array of its bytes rather than a string.
    let message = match entry.get("MESSAGE")? {
        serde_json::Value::String(text) => text.clone(),
        serde_json::Value::Array(bytes) => {
            let bytes: Option<Vec<u8>> = bytes
                .iter()
                .map(|b| b.as_u64().and_then(|b| u8::try_from(b).ok()))
                .collect();
            String::from_utf8_lossy(&bytes?).into_owned()
        }
        _ => return None,
    };
    Some((micros / 1000, message))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nothing_typed_or_answered_reaches_the_day_file_the_state_or_the_journal() {
        let root = env::temp_dir().join(format!("pain-points-{}", std::process::id()));
        let folder = root.join("sessions/--home-someone-src-a-secret-project--");
        fs::create_dir_all(&folder).expect("a scratch folder");
        // 2026-11-11 16:00 UTC; the day is counted in UTC, as `date` here
        // says nothing.
        let at: i64 = 1_794_412_800_000;
        let line = |value: serde_json::Value| value.to_string() + "\n";
        let typed = |enter: i64, delivered: &str| {
            line(serde_json::json!({
                "type": "message", "id": "a", "parentId": null, "timestamp": delivered,
                "message": {"role": "user", "timestamp": enter, "content": [
                    {"type": "text", "text": "use the token sk-FAKE-7QEY1S-0001 now"}]}
            }))
        };
        let log = typed(at, "2026-11-11T16:00:20.000Z")
            + &line(serde_json::json!({
                "type": "message", "id": "b", "parentId": "a",
                "timestamp": "2026-11-11T16:00:23.000Z",
                "message": {"role": "assistant", "timestamp": at + 21_000, "durationMs": 2000,
                    "stopReason": "toolUse", "content": [
                        {"type": "text", "text": "sk-FAKE-7QEY1S-0002"},
                        {"type": "toolCall", "arguments": {"command": "sk-FAKE-7QEY1S-0003"}}]}
            }))
            + &line(serde_json::json!({
                "type": "message", "id": "c", "parentId": "b",
                "timestamp": "2026-11-11T16:00:24.000Z",
                "message": {"role": "toolResult", "timestamp": at + 24_000,
                    "content": [{"type": "text", "text": "sk-FAKE-7QEY1S-0004"}]}
            }))
            + &typed(at + 60_000, "2026-11-11T16:01:00.000Z");
        fs::write(folder.join("2026-11-11T16-00-00-000Z_0001.jsonl"), log).expect("a log");
        let run = Run {
            trials: None,
            sessions: root.join("sessions"),
            journalctl: root.join("no-journalctl"),
            date: root.join("no-date"),
            state: root.join("state"),
            at,
        };
        let said = read(&run).expect("the run reads");
        let day = fs::read_to_string(root.join("state/2026-11-11.jsonl")).expect("a day file");
        let state = fs::read_to_string(root.join("state/pi-logs.json")).expect("a state");
        fs::remove_dir_all(&root).expect("the scratch folder goes");
        assert!(
            said.contains("of 2 messages") && day.contains("\"place\":\"pi\""),
            "both messages were counted and the wait named: {said} / {day}"
        );
        for kept in [&said, &day, &state] {
            assert!(
                !kept.contains("FAKE"),
                "nothing typed or answered is kept: {kept}"
            );
            assert!(
                !kept.contains("secret"),
                "no session's folder is kept: {kept}"
            );
        }
    }

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
        assert_eq!(
            local_day(at, -18_000),
            ("2026-11-11".to_string(), 1_794_373_200_000)
        );
        assert_eq!(
            local_day(at, 0),
            ("2026-11-12".to_string(), 1_794_441_600_000)
        );
    }

    #[test]
    fn a_journal_entry_gives_its_time_and_message() {
        let line = r#"{"__REALTIME_TIMESTAMP":"1794455125123456","MESSAGE":"  INFO qml: launcher: opening usable 195 ms from the chord, its longest stall 48 ms","_PID":"1"}"#;
        let (at, message) = journal_entry(line).expect("the entry reads");
        assert_eq!(at, 1_794_455_125_123);
        assert_eq!(pain::usable_of(&message), Some(195.0));
        let coloured = r#"{"__REALTIME_TIMESTAMP":"1000","MESSAGE":[27,91,51,50,109,104,105]}"#;
        assert_eq!(
            journal_entry(coloured),
            Some((1, "\u{1b}[32mhi".to_string()))
        );
        assert_eq!(journal_entry(r#"{"MESSAGE":"no time"}"#), None);
        assert_eq!(
            journal_entry(r#"{"__REALTIME_TIMESTAMP":"1","MESSAGE":[300]}"#),
            None
        );
    }
}
