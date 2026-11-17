//! Names the interactions with the launcher, pi and Chrome that slowed Joel
//! most today, so what slows him down is read from his own use rather than
//! guessed (Joel, 2026-11-11: "identifying what i'm trying to do, and what
//! slows me down from getting there, particuularly the launcher").
//!
//! `pain-points read` reads the launcher's own record of each opening
//! (launcher-trials.jsonl, under the shell's state) and its journal's
//! "opening usable" lines, and what's new in pi's session logs since its
//! last read, and each Chrome profile's visits by when and how alone,
//! works out what each of today's openings, messages and visits lost him
//! (src/pain.rs, src/pi.rs and src/chrome.rs say how), and writes the
//! worst, a line
//! each, to `$XDG_STATE_HOME/pain-points/<local day>.jsonl`, rewritten
//! whole each time so the file always says the day so far. It keeps kinds
//! and timings alone: what he typed, copied or opened by name never
//! leaves the launcher's own record, and what he typed to pi is never
//! written or sent anywhere, nor is any page Chrome visited. Its help,
//! its refusals and its exits take the terminal kit's shape (`help()`).

mod chrome;
mod hear;
mod pain;
mod pi;
mod trial;
mod words;

use pain::{Painful, Usable, Usual};
use pi::Log;
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::io::{self, BufRead};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};
use terminal_kit::{Exit, Failure, Help, flags};

/// A verb, and the value given each of its flags.
type Asked = (String, BTreeMap<&'static str, String>);

/// How far back his usual time for each kind of pick is drawn from.
const USUAL_DAYS: i64 = 14;
const DAY_MS: i64 = 86_400_000;

/// Where a run reads from and writes to, and the time it counts the day
/// of.
struct Run {
    trials: Option<PathBuf>,
    sessions: PathBuf,
    chrome: PathBuf,
    journalctl: PathBuf,
    date: PathBuf,
    state: PathBuf,
    at: i64,
}

/// The flags `hear` takes, each with a value.
const HEAR_FLAGS: [&str; 6] = [
    "--sessions",
    "--ledgers",
    "--void",
    "--mycelium",
    "--date",
    "--at",
];

/// The flags `read` takes, each with a value.
const FLAGS: [&str; 6] = [
    "--trials",
    "--sessions",
    "--journalctl",
    "--chrome",
    "--date",
    "--at",
];

/// What `pain-points --help` says, and `read`'s own.
fn help() -> Help {
    Help::new("pain-points", "names what slowed Joel most today")
        .example("pain-points read", "writes today's file and says the day")
        .subcommand(
            Help::new("read", "reads today's use and writes its worst")
                .example(
                    "pain-points read --journalctl PATH --date PATH",
                    "as its timer runs it",
                )
                .option("--trials FILE", "the launcher's openings, or its newest")
                .option("--sessions DIR", "pi's session logs, or its own")
                .option("--journalctl PATH", "the journalctl on PATH, or this")
                .option("--chrome DIR", "Chrome's profiles, or its own")
                .option("--date PATH", "the date on PATH, or this")
                .option("--at MILLIS", "the day to count, or today")
                .exit(Exit::Failed, "today's use couldn't be read or written"),
        )
        .subcommand(
            Help::new("hear", "hears what hurt him and sends it to its owner")
                .example(
                    "pain-points hear --mycelium PATH --date PATH",
                    "as its timer runs it",
                )
                .option("--sessions DIR", "pi's session logs, or its own")
                .option(
                    "--ledgers DIR",
                    "the asks and misses ledgers, or the mission's",
                )
                .option("--void DIR", "the void's folder, never sent to, or its own")
                .option("--mycelium PATH", "the mycelium on PATH, or this")
                .option("--date PATH", "the date on PATH, or this")
                .option("--at MILLIS", "the time to hear up to, or now")
                .exit(Exit::Failed, "its state couldn't be written"),
        )
}

/// The flags a subcommand was given, each by its name. The kit has walked
/// them, so the first word is the subcommand, or there are none for a
/// bare run.
///
/// # Errors
///
/// Asked wrongly, 64, for a flag it doesn't know, a flag with no value, a
/// word that isn't a flag's, or a time that isn't a number.
fn asked(args: &[String]) -> Result<Option<Asked>, Failure> {
    let Some((verb, rest)) = args.split_first() else {
        return Ok(None);
    };
    let known: &[&'static str] = if verb == "hear" { &HEAR_FLAGS } else { &FLAGS };
    let try_this = format!("pain-points {verb} --help");
    let wrongly = |what: String| Failure::new(Exit::Usage, what).try_this(&try_this);
    let mut given = BTreeMap::new();
    let mut rest = rest.iter();
    while let Some(arg) = rest.next() {
        let flag = match known.iter().find(|flag| **flag == arg) {
            Some(flag) => *flag,
            None if arg.starts_with('-') => {
                return Err(flags::unknown("option", arg, known).try_this(&try_this));
            }
            None => {
                return Err(wrongly(format!(
                    "pain-points {verb} takes no word like {arg}"
                )));
            }
        };
        let value = rest
            .next()
            .ok_or_else(|| wrongly(format!("{flag} needs a value")))?;
        given.insert(flag, value.clone());
    }
    if let Some(at) = given.get("--at").filter(|at| at.parse::<i64>().is_err()) {
        return Err(wrongly(format!("--at takes Unix milliseconds, not {at}")));
    }
    Ok(Some((verb.clone(), given)))
}

fn main() -> ExitCode {
    let help = help();
    terminal_kit::main(&help, env!("CARGO_PKG_VERSION"), |context| {
        let Some((verb, mut given)) = asked(&context.args)? else {
            return context
                .say(&help.brief(&context.out, context.room))
                .map_err(|why| {
                    Failure::new(Exit::Failed, "it couldn't say its help").because(why.to_string())
                });
        };
        let home = PathBuf::from(env::var_os("HOME").unwrap_or_default());
        if verb == "hear" {
            let said = hear::listen(&home, &mut given).map_err(|error| {
                Failure::new(Exit::Failed, "pain-points couldn't keep what it heard").because(error)
            })?;
            println!("{said}");
            return Ok(());
        }
        let mut take = |flag: &str| given.remove(flag);
        let run = Run {
            trials: take("--trials").map(PathBuf::from).or_else(newest_trials),
            sessions: take("--sessions")
                .map_or_else(|| home.join(".pi/agent/sessions"), PathBuf::from),
            chrome: take("--chrome")
                .map_or_else(|| home.join(".config/google-chrome"), PathBuf::from),
            journalctl: take("--journalctl").map_or_else(|| "journalctl".into(), PathBuf::from),
            date: take("--date").map_or_else(|| "date".into(), PathBuf::from),
            state: state_home().join("pain-points"),
            // A past day can be counted again, to try a change on it.
            at: take("--at")
                .and_then(|at| at.parse().ok())
                .unwrap_or_else(millis_now),
        };
        // The day's line goes to the journal as it always has.
        let said = read(&run).map_err(|error| {
            Failure::new(Exit::Failed, "pain-points couldn't read today's use").because(error)
        })?;
        println!("{said}");
        Ok(())
    })
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
    let logs = follow_pi(
        &run.sessions,
        &run.state.join("pi-logs.json"),
        start,
        &reader(),
    )?;
    let turns: Vec<&[pi::Turn]> = logs.values().map(|log| log.turns.as_slice()).collect();
    let messages = turns
        .iter()
        .flat_map(|t| t.iter())
        .filter(|t| (start..until).contains(&t.enter))
        .count();
    let named_pi = pi::painful(&turns, start, until);
    let profiles = chrome_profiles(&run.chrome, start);
    let named_chrome = chrome::painful(&profiles, start, until);
    let (from_launcher, from_pi, from_chrome) =
        (launcher.len(), named_pi.len(), named_chrome.len());
    let named = pain::ranked(
        launcher
            .into_iter()
            .chain(named_pi)
            .chain(named_chrome)
            .collect(),
    );
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
        from_chrome,
    ))
}

/// Each Chrome profile's visits from `since`, in the order of the
/// profiles' folders, which is how each is numbered. A profile whose
/// history can't be read whole now is passed over until the next run.
fn chrome_profiles(chrome: &Path, since: i64) -> Vec<Vec<chrome::Visit>> {
    let mut folders: Vec<PathBuf> = fs::read_dir(chrome)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .map(|entry| entry.path().join("History"))
        .filter(|history| history.is_file())
        .collect();
    folders.sort();
    folders
        .iter()
        .map(|history| chrome::visits(history, since).unwrap_or_default())
        .collect()
}

/// The journal's line for the day: how many openings and messages lost
/// him a second or more, of how many, how many goes in Chrome did, and
/// the worst.
fn summary(
    named: &[Painful],
    launcher: (usize, usize),
    pi: (usize, usize),
    chrome: usize,
) -> String {
    let counted = format!(
        "{} of {} openings, {} of {} messages to pi and {chrome} {} in Chrome today \
         lost him a second or more",
        launcher.0,
        launcher.1,
        pi.0,
        pi.1,
        if chrome == 1 { "go" } else { "goes" }
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
fn follow_pi(
    sessions: &Path,
    state: &Path,
    since: i64,
    reader: &str,
) -> Result<BTreeMap<String, Log>, String> {
    // What a turn holds is the reading of the build that read it, so a
    // state another build kept, or one from before builds were named, is
    // read again from each log's start rather than gone on from.
    let mut logs: BTreeMap<String, Log> = fs::read(state)
        .ok()
        .and_then(|kept| serde_json::from_slice::<Kept>(&kept).ok())
        .filter(|kept| kept.reader == reader)
        .map(|kept| kept.logs)
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
            let written = entry.metadata().and_then(|m| m.modified()).map_or(0, |t| {
                t.duration_since(UNIX_EPOCH)
                    .map_or(0, |d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
            });
            // Not `present` either, so a lab is never kept in the state.
            if written >= since && begun_in_a_lab(&path) {
                continue;
            }
            present.push(name.clone());
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
    let kept = Kept {
        reader: reader.to_string(),
        logs,
    };
    write_whole(
        state,
        &serde_json::to_string(&kept).map_err(|e| e.to_string())?,
    )?;
    Ok(kept.logs)
}

/// Whether a session log began in a quest's scratch `tmp`, which is the
/// `$TMPDIR` of the agents on that quest. An agent testing pi drives it
/// there exactly as Joel types into it, keys and all, so only where it began
/// tells them apart. The quest's other scratch folders stay his, since a
/// session he talks to, such as an orchestrator, may run in one.
fn begun_in_a_lab(path: &Path) -> bool {
    #[derive(serde::Deserialize)]
    struct Header {
        cwd: PathBuf,
    }
    let mut first = String::new();
    let read = fs::File::open(path).and_then(|file| io::BufReader::new(file).read_line(&mut first));
    let Some(header) = read
        .ok()
        .and_then(|_| serde_json::from_str::<Header>(&first).ok())
    else {
        return false;
    };
    a_lab(&header.cwd)
}

/// Whether a session's folder is a quest's scratch `tmp`.
fn a_lab(cwd: &Path) -> bool {
    let parts: Vec<_> = cwd.components().collect();
    parts
        .windows(3)
        .any(|w| w[0].as_os_str() == "quest-workspace" && w[2].as_os_str() == "tmp")
}

/// The reading state, with the build that read it.
#[derive(serde::Serialize, serde::Deserialize)]
struct Kept {
    reader: String,
    logs: BTreeMap<String, Log>,
}

/// This build of the program, as the path it runs from, which in the Nix
/// store changes with each build.
fn reader() -> String {
    env::current_exe().map_or_else(|_| String::new(), |p| p.display().to_string())
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
        let profile = root.join("chrome/Default");
        fs::create_dir_all(&profile).expect("a profile");
        chrome::tests::history(
            &profile.join("History"),
            &[
                chrome::Visit {
                    at,
                    transition: 0x3000_0008,
                },
                chrome::Visit {
                    at: at + 30_000,
                    transition: 0x3000_0008,
                },
            ],
        );
        let run = Run {
            trials: None,
            sessions: root.join("sessions"),
            chrome: root.join("chrome"),
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
        assert!(
            said.contains("1 go in Chrome") && day.contains("\"place\":\"chrome\""),
            "the reloads were named: {said} / {day}"
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
    fn a_state_another_build_kept_is_read_anew() {
        let root = env::temp_dir().join(format!("pain-points-reader-{}", std::process::id()));
        let folder = root.join("sessions/--home-someone--");
        fs::create_dir_all(&folder).expect("a scratch folder");
        let at: i64 = 1_794_412_800_000;
        let log = serde_json::json!({
            "type": "message", "id": "a", "parentId": null,
            "timestamp": "2026-11-11T16:00:00.000Z",
            "message": {"role": "user", "timestamp": at, "content": "go"}
        })
        .to_string()
            + "\n";
        let name = "2026-11-11T16-00-00-000Z_0001.jsonl";
        fs::write(folder.join(name), &log).expect("a log");
        let state = root.join("pi-logs.json");
        let read_through =
            serde_json::json!({name: {"end": log.len(), "open": false, "turns": []}});
        let turns = |kept: &serde_json::Value, reader: &str| {
            fs::write(&state, kept.to_string()).expect("a state");
            let logs = follow_pi(&root.join("sessions"), &state, 0, reader).expect("it reads");
            logs.get(name).map_or(0, |log| log.turns.len())
        };
        let kept = serde_json::json!({"reader": "/nix/store/a-pain-points", "logs": read_through});
        assert_eq!(
            turns(&kept, "/nix/store/a-pain-points"),
            0,
            "the build that kept it goes on from where it read to"
        );
        assert_eq!(
            turns(&kept, "/nix/store/b-pain-points"),
            1,
            "another build may count a turn otherwise, so it reads the log again"
        );
        assert_eq!(
            turns(&read_through, "/nix/store/b-pain-points"),
            1,
            "a state that names no build is read again"
        );
        fs::remove_dir_all(&root).expect("the scratch folder goes");
    }

    #[test]
    fn a_session_begun_in_a_quests_scratch_tmp_is_an_agents_lab_not_his() {
        let root = env::temp_dir().join(format!("pain-points-lab-{}", std::process::id()));
        let at: i64 = 1_794_412_800_000;
        let scratch = "/home/someone/.cache/pi/agentic-harness.pi/quest-workspace/QEST-1-A";
        let log = |cwd: Option<&str>| {
            [
                serde_json::json!({"type": "session", "version": 3, "id": "s",
                    "timestamp": "2026-11-11T16:00:00.000Z", "cwd": cwd}),
                serde_json::json!({"type": "message", "id": "a", "parentId": null,
                    "timestamp": "2026-11-11T16:00:00.000Z",
                    "message": {"role": "user", "timestamp": at, "content": "go"}}),
            ]
            .map(|line| line.to_string() + "\n")
            .concat()
        };
        for (name, cwd) in [
            ("lab.jsonl", Some(format!("{scratch}/tmp"))),
            ("lab-below.jsonl", Some(format!("{scratch}/tmp/rd"))),
            (
                "orchestrator.jsonl",
                Some(format!("{scratch}/agentic-harness.pi")),
            ),
            ("system-tmp.jsonl", Some("/tmp".to_owned())),
            ("home-tmp.jsonl", Some("/home/someone/src/tmp".to_owned())),
            ("no-cwd.jsonl", None),
        ] {
            let folder = root.join("sessions").join("sessions");
            fs::create_dir_all(&folder).expect("a scratch folder");
            fs::write(folder.join(name), log(cwd.as_deref())).expect("a log");
        }
        let logs =
            follow_pi(&root.join("sessions"), &root.join("pi-logs.json"), 0, "r").expect("reads");
        fs::remove_dir_all(&root).expect("the scratch folder goes");
        let names: Vec<&str> = logs.keys().map(String::as_str).collect();
        assert_eq!(
            names,
            [
                "home-tmp.jsonl",
                "no-cwd.jsonl",
                "orchestrator.jsonl",
                "system-tmp.jsonl"
            ],
            "an agent's lab drives pi as he would; any other tmp, or no header, is his"
        );
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
