//! last-seconds: says how each kept boot's record ended, and what the
//! machine was doing in its last seconds, from the rings
//! `last-seconds-keep` writes (src/lib.rs says why).
//!
//! It reads the rings as the account that asks, which the wheel group
//! may; nothing runs it as root. A boot whose newest record says the
//! recorder was stopped ended as the machine powered off, after its
//! services, or was stopped by hand; one whose newest record says the
//! machine went to sleep never woke from it; any other was cut short, by a
//! reset or a loss of power some time after its newest second, and its last
//! seconds are the ones to read. The boot running now whose newest record
//! is more than a few seconds old has a recorder that stopped writing.
//! `LAST_SECONDS_STATE` names another folder of rings, for the tests.

use last_seconds::record::Record;
use last_seconds::ring::{self, Header};
use last_seconds::table::{self, clock};
use last_seconds::{RING, STATE, STATE_VARIABLE};
use serde_json::{Value, json};
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use terminal_kit::layout::{self, Align};
use terminal_kit::{Context, Exit, Failure, Help, Row};

/// How long a boot id is, without hyphens.
const BOOT_ID: usize = 32;
/// How many seconds one boot shows in text, unless asked otherwise.
const SHOWN: usize = 20;
/// How old the boot running now's newest record may be, in milliseconds,
/// before its recorder is said to have stopped writing.
const QUIET_MS: u64 = 10_000;

/// How a boot's record ended.
#[derive(Debug, Clone, PartialEq, Eq)]
enum End {
    /// It's the boot running now, and still being recorded.
    Recording,
    /// It's the boot running now, but nothing was written for a while.
    Quiet,
    /// The recorder was stopped, as at a shutdown.
    Stopped,
    /// The machine went to sleep and the boot never woke: a reset or a
    /// cut while asleep or waking, or a hibernation never resumed, which
    /// the record can't tell apart.
    Asleep,
    /// Its newest record doesn't say it was stopped: a reset or a cut.
    Cut,
    /// Its ring holds no whole record.
    Empty,
    /// Its ring couldn't be read, and why.
    Unreadable(String),
}

impl End {
    fn name(&self) -> &'static str {
        match self {
            Self::Recording => "recording",
            Self::Quiet => "quiet",
            Self::Stopped => "stopped",
            Self::Asleep => "asleep",
            Self::Cut => "cut",
            Self::Empty => "empty",
            Self::Unreadable(_) => "unreadable",
        }
    }

    /// How it ended, in words, `newest` being when its newest second was
    /// kept: never said as the moment of a reset, which came after it.
    fn words(&self, newest: Option<&str>) -> String {
        let when = |before: &str| newest.map_or_else(String::new, |at| format!(" {before} {at}"));
        match self {
            Self::Recording => "still being recorded".to_owned(),
            Self::Quiet => format!(
                "its recorder has written nothing{}: journalctl -b -u last-seconds says why",
                when("since")
            ),
            Self::Stopped => format!(
                "its recorder was stopped{}, as at a power off or by hand",
                when("at")
            ),
            Self::Asleep => format!(
                "it went to sleep{} and never woke: it reset or lost power asleep or waking, \
                 or it hibernated and wasn't resumed",
                when("at")
            ),
            Self::Cut => format!(
                "cut short: the machine reset or lost power{}",
                when("after")
            ),
            Self::Empty => "no second was kept".to_owned(),
            Self::Unreadable(why) => format!("its ring couldn't be read: {why}"),
        }
    }
}

/// One kept boot, read.
struct Kept {
    boot: String,
    header: Option<Header>,
    records: Vec<Record>,
    end: End,
}

/// Where the rings are.
fn state() -> PathBuf {
    env::var_os(STATE_VARIABLE)
        .filter(|state| !state.is_empty())
        .map_or_else(|| PathBuf::from(STATE), PathBuf::from)
}

/// The boot running now, as the folders name it.
fn current() -> String {
    fs::read_to_string("/proc/sys/kernel/random/boot_id")
        .map(|id| id.trim().replace('-', ""))
        .unwrap_or_default()
}

/// Every kept boot, the newest first, `now_ms` being how long the boot
/// running now has run; `only` those whose ids start with it, so asking
/// for one boot reads its ring alone, as the sleep's hook does each look.
fn kept_in(state: &Path, current: &str, now_ms: u64, only: Option<&str>) -> io::Result<Vec<Kept>> {
    let entries = match fs::read_dir(state) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error),
    };
    let mut kept = Vec::new();
    for entry in entries {
        let entry = entry?;
        let Ok(boot) = entry.file_name().into_string() else {
            continue;
        };
        if boot.len() != BOOT_ID
            || !boot.bytes().all(|byte| byte.is_ascii_hexdigit())
            || only.is_some_and(|only| !boot.starts_with(only))
        {
            continue;
        }
        let (header, records, end) = match ring::read(&entry.path().join(RING)) {
            Ok((header, records)) => {
                let end = match records.last() {
                    None => End::Empty,
                    Some(newest) if newest.stopped => End::Stopped,
                    Some(newest) if newest.asleep && boot != current => End::Asleep,
                    Some(newest)
                        if boot == current
                            && now_ms.saturating_sub(newest.since_boot_ms) > QUIET_MS =>
                    {
                        End::Quiet
                    }
                    Some(_) if boot == current => End::Recording,
                    Some(_) => End::Cut,
                };
                (Some(header), records, end)
            }
            // A folder the ageing emptied is no kept boot.
            Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
            Err(error) => (None, Vec::new(), End::Unreadable(error.to_string())),
        };
        kept.push(Kept {
            boot,
            header,
            records,
            end,
        });
    }
    kept.sort_by(|a, b| {
        let began = |boot: &Kept| boot.header.as_ref().map(|header| header.began);
        began(b).cmp(&began(a)).then_with(|| a.boot.cmp(&b.boot))
    });
    Ok(kept)
}

/// A moment as the machine's clock reads it, to the minute.
fn minute(seconds: i64) -> String {
    terminal_kit::local::at(seconds).map_or_else(
        || seconds.to_string(),
        |at| format!("{} {:02}:{:02}", at.date(), at.hour, at.minute),
    )
}

/// A moment as the machine's clock reads it, to the second, with its day.
fn moment(at_ms: i64) -> String {
    let seconds = at_ms.div_euclid(1_000);
    terminal_kit::local::at(seconds).map_or_else(
        || seconds.to_string(),
        |at| format!("{} {}", at.date(), clock(at_ms)),
    )
}

fn record_json(record: &Record) -> Value {
    let stalls = record.stalls;
    json!({
        "sequence": record.sequence,
        "at": terminal_kit::time::rfc3339(record.at_ms.div_euclid(1_000)),
        "at_ms": record.at_ms,
        "since_boot_ms": record.since_boot_ms,
        "package_mw": record.package_mw,
        "platform_mw": record.platform_mw,
        "package_mc": record.package_mc,
        "fans_rpm": record.fans,
        "load_hundredths": record.load,
        "tasks_running": record.tasks.map(|(running, _)| running),
        "tasks": record.tasks.map(|(_, all)| all),
        "stalls_us": {
            "cpu_some": stalls.cpu_some,
            "memory_some": stalls.memory_some,
            "memory_full": stalls.memory_full,
            "io_some": stalls.io_some,
            "io_full": stalls.io_full,
        },
        "available_kib": record.available_kib,
        "swapped_kib": record.swapped_kib,
        "battery": {
            "mv": record.battery_mv,
            "ma": record.battery_ma,
            "percent": record.battery_percent,
            "state": record.battery.name(),
        },
        "outside_power": record.outside_power,
        "rests_us": record.rests,
        "firmware_read": record.firmware_read,
        "stopped": record.stopped,
        "asleep": record.asleep,
    })
}

fn boot_json(boot: &Kept, current: &str) -> Value {
    let newest = boot.records.last();
    json!({
        "boot": boot.boot,
        "current": boot.boot == current,
        "began": boot.header.as_ref().map(|header| terminal_kit::time::rfc3339(header.began)),
        "end": boot.end.name(),
        // The newest second kept, never said as when the boot ended: a
        // reset or a sleep never woken from came some time after it.
        "newest": newest.map(|newest| terminal_kit::time::rfc3339(newest.at_ms.div_euclid(1_000))),
        "seconds_kept": boot.records.len(),
        "processors": boot.header.as_ref().map(|header| header.processors),
        "rests": boot.header.as_ref().map(|header| header.rests.clone()),
    })
}

fn rows(kept: &[Kept], current: &str) -> Vec<Row> {
    kept.iter()
        .map(|boot| {
            let name = boot.header.as_ref().map_or_else(
                || "began at no known time".to_owned(),
                |header| minute(header.began),
            );
            let words = boot.end.words(
                boot.records
                    .last()
                    .map(|newest| clock(newest.at_ms))
                    .as_deref(),
            );
            let side = if boot.boot == current {
                format!("{} now", &boot.boot[..8])
            } else {
                boot.boot[..8].to_owned()
            };
            let row = Row::named(name).holding(words).side(side);
            match boot.end {
                End::Cut | End::Asleep | End::Quiet => row.marked("caution", "warning"),
                End::Unreadable(_) => row.marked("failed", "alert"),
                _ => row,
            }
        })
        .collect()
}

/// What it says of itself, set as help and read by the check.
fn help() -> Help {
    Help::new(
        "last-seconds",
        "says what the machine was doing in the seconds before each boot ended",
    )
    .argument(
        "BOOT",
        "a boot's id, or the start of it, as journalctl --list-boots gives it",
    )
    .option(
        "--seconds N",
        "how many of a boot's last seconds to show; 20 in text",
    )
    .example("last-seconds", "every kept boot, and how its record ended")
    .example("last-seconds 6bccd85d", "that boot's last 20 seconds")
    .example(
        "last-seconds 6bccd85d --seconds 120",
        "its last two minutes",
    )
    .example(
        "last-seconds --format json",
        "every kept boot, for a program",
    )
    .exit(
        Exit::Failed,
        "the rings' folder couldn't be read, or the answer couldn't be said",
    )
    .exit(Exit::NotFound, "no kept boot has that id")
    .exit(
        Exit::Usage,
        "it isn't a boot id, it names more than one kept boot, or the line is wrong",
    )
    .exit(Exit::Refused, "only the wheel group may read the rings")
}

fn unread(state: &Path, error: &io::Error) -> Failure {
    if error.kind() == io::ErrorKind::PermissionDenied {
        // The try line lists the account's groups, which shows whether
        // wheel is among them; it never sends anyone to root.
        return Failure::new(Exit::Refused, "it may not read the rings")
            .because(format!(
                "{} is the wheel group's to read: {error}",
                state.display()
            ))
            .try_this("id -nG");
    }
    let shown = state.to_string_lossy();
    Failure::new(Exit::Failed, "it couldn't read the rings")
        .because(error.to_string())
        .try_this(format!(
            "ls -l {}",
            terminal_kit::failure::shell_word(&shown)
        ))
}

fn said(result: io::Result<()>) -> Result<(), Failure> {
    result.map_err(|error| {
        Failure::new(Exit::Failed, "the answer couldn't be said").because(error.to_string())
    })
}

fn usage(what: impl Into<String>) -> Failure {
    Failure::new(Exit::Usage, what).try_this("last-seconds --help")
}

/// What was asked: a boot, and how many of its seconds.
struct Asked {
    boot: Option<String>,
    seconds: Option<usize>,
}

fn asked(args: &[String]) -> Result<Asked, Failure> {
    let mut asked = Asked {
        boot: None,
        seconds: None,
    };
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--seconds" => {
                let value = args
                    .next()
                    .ok_or_else(|| usage("--seconds needs a number"))?;
                asked.seconds = Some(value.parse().map_err(|_| {
                    usage(format!("--seconds takes a whole number, not {value:?}"))
                })?);
            }
            flag if flag.starts_with("--") => {
                return Err(terminal_kit::flags::unknown("option", flag, &["--seconds"]));
            }
            _ if asked.boot.is_some() => return Err(usage("it takes one boot at most")),
            boot => asked.boot = Some(boot.to_owned()),
        }
    }
    Ok(asked)
}

/// How many of a boot id's digits name a boot, at the fewest, so neither
/// journalctl's index of a boot, as -1 or 0, nor a mistyped flag is taken
/// for the start of one.
const SHORTEST_ID: usize = 4;

/// A boot id as any tool writes it, hyphens or not, in either case; none
/// where it holds anything but hex digits and hyphens, starts with a
/// hyphen, or has fewer than `SHORTEST_ID` digits.
fn plain_id(id: &str) -> Option<String> {
    let plain: String = id
        .chars()
        .filter(|char| *char != '-')
        .map(|char| char.to_ascii_lowercase())
        .collect();
    (!id.starts_with('-')
        && plain.len() >= SHORTEST_ID
        && plain.len() <= BOOT_ID
        && id
            .chars()
            .all(|char| char == '-' || char.is_ascii_hexdigit()))
    .then_some(plain)
}

fn named<'a>(kept: &'a [Kept], id: &str) -> Result<&'a Kept, Failure> {
    let Some(plain) = plain_id(id) else {
        return Err(usage(format!("{id:?} isn't a boot id")).because(format!(
            "a boot id is hex digits, with hyphens or without, and at least its first \
                 {SHORTEST_ID}"
        )));
    };
    let matching: Vec<&Kept> = kept
        .iter()
        .filter(|boot| boot.boot.starts_with(&plain))
        .collect();
    match matching.as_slice() {
        [boot] => Ok(boot),
        [] => Err(
            Failure::new(Exit::NotFound, format!("no kept boot has the id {id}"))
                .because("a boot's seconds are kept 90 days")
                .try_this("last-seconds"),
        ),
        many => Err(Failure::new(
            Exit::Usage,
            format!("{id} starts the ids of {} kept boots", many.len()),
        )
        .try_this(format!("last-seconds {}", many[0].boot))),
    }
}

fn list(context: &Context, kept: &[Kept], current: &str) -> Result<(), Failure> {
    if context.format().is_json() {
        let boots: Vec<Value> = kept.iter().map(|boot| boot_json(boot, current)).collect();
        return said(context.answer(&json!({"schema": "last-seconds.boots/1", "boots": boots})));
    }
    if kept.is_empty() {
        return said(context.say("No boot's seconds are kept yet.\n"));
    }
    said(context.say(&layout::joined(vec![layout::list(
        &rows(kept, current),
        &context.out,
        context.room,
    )])))
}

fn one(
    context: &Context,
    boot: &Kept,
    current: &str,
    seconds: Option<usize>,
) -> Result<(), Failure> {
    let header = boot.header.as_ref();
    if context.format().is_json() {
        let shown = seconds.unwrap_or(boot.records.len());
        let records = &boot.records[boot.records.len().saturating_sub(shown)..];
        let mut answer = boot_json(boot, current);
        if let Some(fields) = answer.as_object_mut() {
            fields.insert("schema".to_owned(), json!("last-seconds.boot/1"));
            fields.insert(
                "seconds".to_owned(),
                Value::Array(records.iter().map(record_json).collect()),
            );
        }
        return said(context.answer(&answer));
    }
    let newest = boot.records.last();
    let facts = [
        ("Boot", Some(boot.boot.clone())),
        ("Began", header.map(|header| minute(header.began))),
        ("Its record", Some(boot.end.words(None))),
        ("Last second", newest.map(|newest| moment(newest.at_ms))),
        ("Seconds kept", Some(boot.records.len().to_string())),
    ];
    let facts: Vec<(&str, Option<&str>)> = facts
        .iter()
        .map(|(label, value)| (*label, value.as_deref()))
        .collect();
    let mut blocks = vec![layout::facts(
        &facts,
        "not kept",
        &context.out,
        context.room,
    )];
    let shown = seconds.unwrap_or(SHOWN);
    let records = &boot.records[boot.records.len().saturating_sub(shown)..];
    if !records.is_empty() {
        let columns = table::columns(header);
        let columns: Vec<(&str, Align)> = columns
            .iter()
            .map(|(label, align)| (label.as_str(), *align))
            .collect();
        let rows: Vec<Vec<String>> = records
            .iter()
            .map(|record| table::cells(record, header))
            .collect();
        blocks.push(layout::table_in(
            &columns,
            &rows,
            &context.out,
            context.room,
            &table::GIVES_WAY,
        ));
    }
    said(context.say(&layout::joined(blocks)))
}

fn main() -> ExitCode {
    terminal_kit::main(&help(), env!("CARGO_PKG_VERSION"), |context| {
        let asked = asked(&context.args)?;
        let state = state();
        let current = current();
        // An id that isn't one reads every boot, and named says why.
        let only = asked.boot.as_deref().and_then(plain_id);
        let kept = kept_in(
            &state,
            &current,
            last_seconds::sample::since_boot_ms(),
            only.as_deref(),
        )
        .map_err(|error| unread(&state, &error))?;
        match &asked.boot {
            None => list(context, &kept, &current),
            Some(id) => one(context, named(&kept, id)?, &current, asked.seconds),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use last_seconds::ring::Ring;
    use last_seconds::scratch::Folder;

    fn ring_in(state: &Path, boot: &str, records: &[Record]) {
        ring_begun(state, boot, 1_795_241_514, records);
    }

    fn ring_begun(state: &Path, boot: &str, began: i64, records: &[Record]) {
        let folder = state.join(boot);
        fs::create_dir_all(&folder).expect("a boot's folder");
        let header = Header {
            slots: 8,
            began,
            processors: 2,
            rests: vec!["C1".to_owned(), "C10".to_owned()],
        };
        let mut ring = Ring::open(&folder.join(RING), &header).expect("a ring");
        for record in records {
            ring.put(&mut record.clone()).expect("a record");
        }
    }

    fn state(name: &str) -> Folder {
        Folder::make(&format!("reader-{name}")).expect("a state folder")
    }

    #[test]
    fn a_ring_whose_newest_record_isnt_a_stop_was_cut() {
        let state = state("ends");
        let second = Record {
            since_boot_ms: 60_000,
            ..Record::default()
        };
        let stop = Record {
            stopped: true,
            ..Record::default()
        };
        // Begun in another order than their ids sort in, so the listing
        // shows it goes by when each began.
        ring_begun(
            &state,
            "53cb792037684896bc1e3870381c2398",
            100,
            &[second.clone(), stop],
        );
        ring_begun(
            &state,
            "949a397ceafb4b789dd6b3d0e50c92e9",
            200,
            std::slice::from_ref(&second),
        );
        ring_begun(&state, "6bccd85d80714583afd805c687f0cdb6", 300, &[second]);
        // A folder the ageing emptied is no kept boot.
        fs::create_dir_all(state.join("83f85022e7b24638b538d6dcafafea46"))
            .expect("an emptied folder");
        let ends = |now_ms: u64| -> Vec<(String, &'static str)> {
            kept_in(&state, "6bccd85d80714583afd805c687f0cdb6", now_ms, None)
                .expect("the boots")
                .iter()
                .map(|boot| (boot.boot[..4].to_owned(), boot.end.name()))
                .collect()
        };
        let named = |list: &[(&str, &'static str)]| -> Vec<(String, &'static str)> {
            list.iter()
                .map(|(id, end)| ((*id).to_owned(), *end))
                .collect()
        };
        assert_eq!(
            ends(61_000),
            named(&[("6bcc", "recording"), ("949a", "cut"), ("53cb", "stopped")]),
            "the boots weren't said newest first, or their ends were wrong"
        );
        assert_eq!(
            ends(80_000)[0],
            ("6bcc".to_owned(), "quiet"),
            "a recorder that stopped writing was said to be recording"
        );
    }

    #[test]
    fn a_boot_that_went_to_sleep_and_never_woke_says_so_not_when_it_reset() {
        let state = state("asleep");
        let asleep = Record {
            at_ms: 1_795_241_514_000,
            asleep: true,
            ..Record::default()
        };
        ring_in(&state, "949a397ceafb4b789dd6b3d0e50c92e9", &[asleep]);
        let kept = kept_in(&state, "6bccd85d80714583afd805c687f0cdb6", 0, None).expect("the boots");
        assert_eq!(kept[0].end, End::Asleep);
        let said = kept[0].end.words(Some("22:10:00"));
        assert!(
            said.contains("went to sleep at 22:10:00 and never woke"),
            "a boot that never woke wasn't said asleep: {said}"
        );
        assert!(
            End::Cut.words(Some("22:10:00")).contains("after 22:10:00"),
            "a cut was said at the second last kept, not after it"
        );
    }

    #[test]
    fn a_ring_that_cant_be_read_is_said_on_its_own_row() {
        let state = state("unreadable");
        let folder = state.join("53cb792037684896bc1e3870381c2398");
        fs::create_dir_all(&folder).expect("a boot's folder");
        fs::write(folder.join(RING), b"not a ring").expect("a file");
        ring_in(
            &state,
            "949a397ceafb4b789dd6b3d0e50c92e9",
            &[Record::default()],
        );
        let kept = kept_in(&state, "", 0, None).expect("the boots");
        let one = kept_in(&state, "", 0, Some("949a")).expect("the boot");
        assert_eq!(kept.len(), 2, "a bad ring took the listing with it");
        assert!(
            one.len() == 1 && one[0].end != End::Empty,
            "asking for one boot read others, or not it"
        );
        assert!(
            kept.iter()
                .any(|boot| matches!(boot.end, End::Unreadable(_))),
            "a ring that couldn't be read wasn't said unreadable"
        );
    }

    #[test]
    fn a_boot_is_found_by_any_way_its_id_is_written() {
        assert_eq!(plain_id("6BCCD85D-8071").as_deref(), Some("6bccd85d8071"));
        // journalctl's index of the boot before, and of this one, and a
        // short flag, each of which strips to the start of a hex id.
        for not_an_id in ["", "-", "6bcc/..", "zz", "-1", "0", "-6bcc", "6bc"] {
            assert_eq!(
                plain_id(not_an_id),
                None,
                "{not_an_id:?} was taken as an id"
            );
        }
    }
}
