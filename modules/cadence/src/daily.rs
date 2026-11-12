//! `cadence day`: reads the day just ended from the machine's records,
//! keeps it as `day/<date>.json` beside the days before, holds it against
//! them, writes the note to `day/<date>.txt`, and sends it to the orchestrator.
//! Each finding goes into the ledger (findings.rs), and each owner is
//! told of its new ones.
//! A program does it, not a session, so the loop costs no model turns.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::Path;

use jiff::Timestamp;
use jiff::tz::TimeZone;

use crate::day::{self, Day};
use crate::findings::{self, Entry};
use crate::judge;
use crate::mesh::Mesh;
use crate::{DAY, command, message, write_whole};

/// Whom the day is told to: the orchestrator its note, each owner its
/// findings by the prefix of their keys, and the watcher what no owner has.
/// None of them given, as when Joel types it, the note is only printed.
#[derive(Default)]
pub struct Told {
    pub orchestrator: Option<String>,
    pub watcher: Option<String>,
    pub owners: BTreeMap<String, String>,
}

/// Reads, keeps and judges the day that ends at `now`, brings the ledger
/// up to it, and tells each of `told` its part.
pub fn run(folder: &Path, state_home: &Path, told: &Told, now: i64) -> io::Result<()> {
    let days = folder.join("day");
    fs::create_dir_all(&days)?;
    let today = read(state_home, now - DAY, now);
    let before = kept(&days, now);
    let judged = judge::judge(&today, &before);
    let kept_at = folder.join("findings.jsonl");
    let mut ledger = ledger(&kept_at);
    findings::update(&mut ledger, &today, &judged.worse, &told.owners);
    // A typed run tells nobody, so it marks nobody told or late.
    let sending = told.orchestrator.is_some();
    let late = if sending {
        findings::late(&mut ledger, now)
    } else {
        Vec::new()
    };
    let asks = if sending {
        findings::tell(&mut ledger, &today.date, told.watcher.as_deref())
    } else {
        Vec::new()
    };
    let lines: Vec<String> = ledger
        .iter()
        .map(serde_json::to_string)
        .collect::<Result<_, _>>()?;
    write_whole(&kept_at, &(lines.join("\n") + "\n"))?;
    let note = judge::note(&today, &judged, before.len(), &late);
    let mut record = serde_json::to_value(&today)?;
    record["worse"] = serde_json::to_value(&judged.worse)?;
    record["better"] = serde_json::to_value(&judged.better)?;
    write_whole(
        &days.join(format!("{}.json", today.date)),
        &serde_json::to_string_pretty(&record)?,
    )?;
    write_whole(&days.join(format!("{}.txt", today.date)), &note)?;
    print!("{note}");
    if let Some(orchestrator) = &told.orchestrator {
        let mesh = Mesh {
            root: state_home.join("pi/agentic-harness.pi"),
        };
        // Findings, so an owner reads them with its next turn rather than
        // being woken for them, which costs a cold start each.
        for (to, text) in asks {
            mesh.send(&message(&to, "finding", text))?;
        }
        mesh.send(&message(orchestrator, "fyi", note))?;
    }
    Ok(())
}

/// The ledger as kept, a line each; a line that isn't an entry is left
/// out rather than failing the day.
fn ledger(path: &Path) -> Vec<Entry> {
    fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect()
}

/// The days kept before the one ending at `now`: the newest `BEFORE` that
/// ended at least half a day and at most eight days before it, so a run
/// again the same evening isn't held against itself.
fn kept(days: &Path, now: i64) -> Vec<Day> {
    let mut kept: Vec<Day> = fs::read_dir(days)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|entry| entry.path().extension().is_some_and(|e| e == "json"))
        .filter_map(|entry| serde_json::from_slice::<Day>(&fs::read(entry.path()).ok()?).ok())
        .filter(|d| d.to <= now - DAY / 2 && d.to > now - 8 * DAY)
        .collect();
    kept.sort_by_key(|d| std::cmp::Reverse(d.to));
    kept.truncate(judge::BEFORE);
    kept
}

/// The day from `from` to `to`, read from each source; a source that
/// can't be read leaves its figures out rather than failing the day.
fn read(state_home: &Path, from: i64, to: i64) -> Day {
    let since = format!("--since=@{from}");
    let until = format!("--until=@{to}");
    let mut day = Day {
        date: local_date(to),
        from,
        to,
        ..Day::default()
    };

    // coredumpctl says it found none with a failure, which is no crash.
    let dumps = command(
        "coredumpctl",
        &["--json=short", "list", &since, &until, "--no-pager"],
    );
    day.named.insert(
        "crashes".to_owned(),
        day::crashes(&dumps.unwrap_or_default()),
    );
    // systemd's message for a unit that failed, the account's units too.
    let failed = command(
        "journalctl",
        &[
            "-o",
            "json",
            "--no-pager",
            &since,
            &until,
            "MESSAGE_ID=d9b373ed55a64feb8242e02dbe79a49c",
        ],
    );
    day.named.insert(
        "failed".to_owned(),
        day::failed_units(&failed.unwrap_or_default()),
    );

    let kernel = command(
        "journalctl",
        &[
            "-k",
            "-o",
            "short-unix",
            "--no-pager",
            &since,
            &until,
            "--grep=oom-kill:",
        ],
    );
    let (machine, cgroup) = day::oom_kills(&kernel.unwrap_or_default());
    day.figures
        .insert("oom.machine".to_owned(), f64::from(machine));
    day.figures
        .insert("oom.cgroup".to_owned(), f64::from(cgroup));

    // lag-watch names each file by the UTC date its records end on.
    let mut lines = String::new();
    let mut dates = vec![utc_date(from), utc_date(to)];
    dates.dedup();
    for date in dates {
        let path = state_home.join(format!("lag-watch/{date}.jsonl"));
        lines.push_str(&fs::read_to_string(path).unwrap_or_default());
    }
    for (key, value) in day::pressure(&lines, from, to) {
        if day::MEASURES.iter().any(|m| m.key == key) {
            day.figures.insert(key, value);
        }
    }

    let updates = command(
        "journalctl",
        &[
            "-u",
            "update-to-main.service",
            "-o",
            "short-unix",
            "--no-pager",
            &since,
            &until,
        ],
    );
    #[expect(clippy::cast_precision_loss, reason = "a time in seconds")]
    let read = day::switches(&updates.unwrap_or_default(), to as f64);
    if let Some(median) = day::median(&read.lengths) {
        day.figures
            .insert("switch.minutes.median".to_owned(), median);
    }
    // One still building counts as long as it has run so far.
    let longest = read.lengths.iter().copied().chain(read.under_way);
    if let Some(longest) = longest.reduce(f64::max) {
        day.figures
            .insert("switch.minutes.longest".to_owned(), longest);
    }
    day.figures
        .insert("switch.pieces.here".to_owned(), f64::from(read.here));
    day.figures
        .insert("switch.pieces.heavy".to_owned(), f64::from(read.heavy));
    day
}

fn local_date(at: i64) -> String {
    Timestamp::from_second(at)
        .map(|t| {
            t.to_zoned(TimeZone::system())
                .strftime("%Y-%m-%d")
                .to_string()
        })
        .unwrap_or_default()
}

fn utc_date(at: i64) -> String {
    Timestamp::from_second(at)
        .map(|t| t.strftime("%Y-%m-%d").to_string())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_day_kept_today_isnt_held_against_itself() {
        let folder = tempfile::tempdir().expect("a scratch folder");
        let now = 10 * DAY;
        for (name, to) in [
            ("a", now - DAY),
            ("b", now - 2 * DAY),
            ("c", now - 60),
            ("d", now - 9 * DAY),
        ] {
            let day = Day {
                date: name.to_owned(),
                to,
                ..Day::default()
            };
            let text = serde_json::to_string(&day).expect("a day says itself");
            fs::write(folder.path().join(format!("{name}.json")), text).expect("written");
        }
        fs::write(folder.path().join("a.txt"), "a note").expect("written");
        let dates: Vec<String> = kept(folder.path(), now)
            .into_iter()
            .map(|d| d.date)
            .collect();
        assert_eq!(
            dates,
            vec!["a", "b"],
            "a day kept this evening, or one past eight days, was held against today"
        );
    }
}
