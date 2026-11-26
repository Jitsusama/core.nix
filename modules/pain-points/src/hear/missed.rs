//! Writes each pain sent to an owner into Joel's misses ledger, which his
//! orchestrator's keeper had kept by hand and left unwritten since 21
//! November: a line a pain, written once, its owner the cell it went to.
//! Once a landing a switch carried closes the pain, its line's check says
//! what landed, and cadence keeps the miss open until the change it names
//! runs here. One the keeper carries into a ledger of its own is marked
//! carried, as that ledger tracks its fix from then, and is written even
//! when it was carried first: it is still a miss, and cadence asks its
//! owner for a test that runs. One carried before it was ever sent, so
//! never sent at all, is written to the owner the first carrying line
//! naming one the routes allow names, or else to the ledgers' keeper that
//! carried it, and left unwritten only while the routes refuse both or
//! name no keeper. One a line the keeper wrote before
//! quotes is written too: said again, it is that miss come back, which
//! needs a test of its own. No other line of the ledger is changed, and a
//! pain closes only on its fix, or on its owner's word at its line of why
//! it was none, never on how long it has been quiet.
//!
//! Only pains sent, or heard and carried unsent, from `FROM_MS` on are
//! written: the 214 sent before it had no thread kept, so most would only
//! ever read as lost, and would each ask its owner for a check at once.
//! They still count as open or lost in the hour until a landing naming
//! each closes it.
//!
//! pain-points is the ledger's only writer, under its lock, and nobody
//! has written it by hand since the keeper's last line of 21 November; a
//! line changed is still written whole only once the ledger reads as it
//! did before, read again just ahead of the rename that replaces it, so
//! a write from outside the lock is lost only in the moment between that
//! read and the rename, and is otherwise left for the next run.

pub mod tested;

use super::heard::{self, Heard, Line};
use super::route::Routes;
use super::stalled;
use serde_json::{Value, json};
use std::fs::{self, File, TryLockError};
use std::io::{ErrorKind, Write};
use std::path::Path;
use std::time::{Duration, Instant};

/// The field naming the pain a line is of, by its key.
pub const PAIN: &str = "pain";

/// How long a run waits for another writer to let go of the ledger.
const LOCK_AT_MOST: Duration = Duration::from_secs(2);

/// When the first pain written was sent: 24 November 2026 at midnight,
/// Joel's time, in ms.
pub const FROM_MS: i64 = 1_795_496_400_000;

/// Writes each of `kept` owed to a cell from `from_ms` on into the ledger
/// at `ledger` once, at his time by `offset_s`, and the check of each
/// closed since, or that it is carried; one carried unsent is owed as
/// `owning` says.
/// Lines are added at its end; the ledger is written whole only when a
/// line changes, under its lock either way, so another run's writes are
/// kept, and only once the ledger reads as it did before, so a writer that
/// skips the lock loses nothing but in the moment of the rename.
///
/// # Errors
///
/// When the ledger or its lock can't be read or written, or stays locked,
/// or the ledger changed as a line of it was being written.
pub fn write(
    ledger: &Path,
    kept: &[Heard],
    owning: &Owning,
    offset_s: i64,
    from_ms: i64,
) -> Result<(), String> {
    let owed: Vec<(&Heard, &str)> = kept
        .iter()
        .filter(|h| !h.void && !stalled::is(h))
        .filter_map(|h| Some((h, owner(h, owning, from_ms)?)))
        .collect();
    if owed.is_empty() {
        return Ok(());
    }
    let (_lock, name) = held(ledger)?;
    let text = match fs::read_to_string(ledger) {
        Ok(text) => text,
        Err(e) if e.kind() == ErrorKind::NotFound => String::new(),
        Err(e) => return Err(format!("{}: {e}", ledger.display())),
    };
    let (checked, added) = written(&text, &owed, offset_s);
    let new: String = added.into_iter().map(|line| line + "\n").collect();
    if let Some(checked) = checked {
        if fs::read_to_string(ledger).unwrap_or_default() != text {
            return Err(format!(
                "{name} changed as its pains were checked, so they wait for the next run"
            ));
        }
        return crate::write_whole(ledger, &(checked + &new));
    }
    let gap = if text.is_empty() || text.ends_with('\n') {
        ""
    } else {
        "\n"
    };
    if new.is_empty() {
        return Ok(());
    }
    File::options()
        .create(true)
        .append(true)
        .open(ledger)
        .and_then(|mut file| file.write_all(format!("{gap}{new}").as_bytes()))
        .map_err(|e| format!("{}: {e}", ledger.display()))
}

/// Whom a pain carried unsent may be owed to: the ledger lines naming an
/// owner a miss may be written to, and the ledgers' keeper that carried
/// it, for one whose carrying lines name none.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Owning {
    pub lines: Vec<Line>,
    pub keeper: Option<String>,
}

/// Who may own a pain carried unsent, by `ledgers` and `routes`: each
/// line's owner named, trimmed, that `routes` doesn't refuse, as no pain
/// is sent to, and the ledgers' keeper `routes` names, unless refused.
#[must_use]
pub fn owning(ledgers: &[Line], routes: &Routes) -> Owning {
    let allowed = |owner: &str| !owner.is_empty() && !routes.refuses(owner);
    let lines = ledgers
        .iter()
        .filter_map(|line| {
            let owner = line.owner.trim();
            allowed(owner).then(|| Line {
                words: line.words.clone(),
                owner: owner.to_owned(),
            })
        })
        .collect();
    let keeper = routes.ledgers.as_deref().map(str::trim);
    Owning {
        lines,
        keeper: keeper.filter(|k| allowed(k)).map(str::to_owned),
    }
}

/// Whom `pain` is owed to as a miss: the cell it was sent to from
/// `from_ms` on, or, for one heard from then that the keeper carried
/// before it was sent, the owner of the first line of `owning` carrying
/// it, or else its keeper.
fn owner<'a>(pain: &'a Heard, owning: &'a Owning, from_ms: i64) -> Option<&'a str> {
    match (&pain.to, pain.routed_at) {
        (Some(to), Some(at)) if at >= from_ms => Some(to.as_str()),
        (None, None) if pain.carried && pain.at >= from_ms => {
            let line = heard::carrier(&pain.words, &owning.lines);
            line.map(|line| line.owner.as_str())
                .or(owning.keeper.as_deref())
        }
        _ => None,
    }
}

/// The ledger at `ledger`'s lock, held, and the ledger's name: the lock
/// is let go as it drops.
fn held(ledger: &Path) -> Result<(File, String), String> {
    let name = ledger.file_name().map_or_else(
        || "misses.jsonl".to_owned(),
        |n| n.to_string_lossy().into_owned(),
    );
    let lock_at = ledger.with_file_name(format!("{name}.lock"));
    let lock = File::options()
        .create(true)
        .truncate(false)
        .write(true)
        .open(&lock_at)
        .map_err(|e| format!("{}: {e}", lock_at.display()))?;
    locked(&lock, &name)?;
    Ok((lock, name))
}

/// Holds `lock` within `LOCK_AT_MOST`, or says why not.
fn locked(lock: &File, name: &str) -> Result<(), String> {
    let started = Instant::now();
    loop {
        match lock.try_lock() {
            Ok(()) => return Ok(()),
            Err(TryLockError::WouldBlock) if started.elapsed() < LOCK_AT_MOST => {
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(TryLockError::WouldBlock) => {
                return Err(format!(
                    "{name} stayed locked past {} s, so its pains wait for the next run",
                    LOCK_AT_MOST.as_secs()
                ));
            }
            Err(TryLockError::Error(e)) => return Err(format!("{name}.lock: {e}")),
        }
    }
}

/// What the ledger `text` becomes for `owed`: the whole of it, when a line
/// of a pain closed or carried since says so, and the lines of pains it
/// holds none of, to add at its end.
fn written(text: &str, owed: &[(&Heard, &str)], offset_s: i64) -> (Option<String>, Vec<String>) {
    let mut held = Vec::new();
    let mut changed = false;
    let lines: Vec<String> = text
        .lines()
        .map(|line| {
            let Ok(mut read) = serde_json::from_str::<Value>(line) else {
                return line.to_owned();
            };
            let Some(key) = read.get(PAIN).and_then(Value::as_str).map(str::to_owned) else {
                return line.to_owned();
            };
            let pain = owed.iter().find(|(h, _)| h.key == key);
            held.push(key);
            let Some(&(pain, _)) = pain else {
                return line.to_owned();
            };
            if pain.carried && read.get("carried").is_none() {
                read["carried"] = json!(true);
            } else if let Some(fix) = pain.fixed_by()
                && read.get("check").is_none()
            {
                read["check"] = json!(fix.said);
                read["closed_by"] = json!(fix.commit);
            } else {
                return line.to_owned();
            }
            changed = true;
            read.to_string()
        })
        .collect();
    let added = owed
        .iter()
        .filter(|(h, _)| !held.contains(&h.key))
        .map(|(h, owner)| line_of(h, owner, offset_s))
        .collect();
    let whole = changed.then(|| lines.into_iter().map(|line| line + "\n").collect());
    (whole, added)
}

/// A pain's line, as the keeper writes a miss: when he said it, his words
/// and its `owner`, with the pain it is and what heard it.
fn line_of(pain: &Heard, owner: &str, offset_s: i64) -> String {
    let mut line = json!({
        "at": stamp(pain.at, offset_s),
        "words": pain.words,
        "owner": owner,
        PAIN: pain.key,
        "heard_by": "pain-points",
    });
    if let Some(fix) = pain.fixed_by() {
        line["check"] = json!(fix.said);
        line["closed_by"] = json!(fix.commit);
    }
    if pain.carried {
        line["carried"] = json!(true);
    }
    line.to_string()
}

/// A moment, `at_ms`, as RFC 3339 at `offset_s` from UTC, as the keeper
/// writes one: "2026-11-21T20:30:00-05:00".
fn stamp(at_ms: i64, offset_s: i64) -> String {
    let local = at_ms.div_euclid(1000) + offset_s;
    let (year, month, day) = civil(local.div_euclid(86_400));
    let time = local.rem_euclid(86_400);
    let sign = if offset_s < 0 { '-' } else { '+' };
    let offset = offset_s.abs();
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}{sign}{:02}:{:02}",
        time / 3600,
        time % 3600 / 60,
        time % 60,
        offset / 3600,
        offset % 3600 / 60
    )
}

/// The date `days` after 1 January 1970, by Howard Hinnant's
/// `civil_from_days`.
fn civil(days: i64) -> (i64, i64, i64) {
    let shifted = days + 719_468;
    let era = shifted.div_euclid(146_097);
    let of_era = shifted.rem_euclid(146_097);
    let years = (of_era - of_era / 1460 + of_era / 36_524 - of_era / 146_096) / 365;
    let of_year = of_era - (365 * years + years / 4 - years / 100);
    let from_march = (5 * of_year + 2) / 153;
    let day = of_year - (153 * from_march + 2) / 5 + 1;
    let month = if from_march < 10 {
        from_march + 3
    } else {
        from_march - 9
    };
    (years + era * 400 + i64::from(month <= 2), month, day)
}

#[cfg(test)]
mod tests;
