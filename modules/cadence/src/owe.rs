//! `cadence owe`: a dated must a quest files in the loop's ledger itself,
//! such as a known exposure and the date it closes by, so the loop's axes
//! read it beside cadence's own findings. No day's read finds one, so
//! `cadence day` never closes it; its owner does, with `--done`. Past its
//! date the orchestrator hears of it once, as of any finding, and its
//! owner, who filed it, is told nothing. On 12 November the security
//! owner asked for a way to file its plan's musts this way.

use std::path::Path;
use std::process::ExitCode;

use jiff::Timestamp;
use jiff::civil::DateTime;
use jiff::tz::TimeZone;

use crate::findings::{self, Entry};

pub const USAGE: &str = "cadence owe KEY --axis AXIS --owner ADDRESS --due YYYY-MM-DDTHH:MM \
                         --says WORDS [--kind KIND]\n       cadence owe --done KEY";

/// A must as its owner files it, due at a time in seconds since the epoch.
#[derive(Debug, PartialEq, Eq)]
pub struct Owed {
    pub key: String,
    pub axis: String,
    pub owner: String,
    pub due: i64,
    pub says: String,
    pub kind: String,
}

/// What `cadence owe` was asked: to file a must, or to close one.
#[derive(Debug, PartialEq, Eq)]
pub enum Asked {
    Owe(Owed),
    Done(String),
}

/// Reads what was asked, its due read as a time in `zone`. A must is
/// `must` unless a kind is given.
pub fn parse(mut args: impl Iterator<Item = String>, zone: &TimeZone) -> Result<Asked, String> {
    let key = args.next().ok_or("it needs a key")?;
    if key == "--done" {
        return args
            .next()
            .map(Asked::Done)
            .ok_or_else(|| "--done needs a key".to_owned());
    }
    let (mut axis, mut owner, mut due, mut says) = (None, None, None, None);
    let mut kind = "must".to_owned();
    while let Some(flag) = args.next() {
        let value = args.next().ok_or_else(|| format!("{flag} has no value"))?;
        match flag.as_str() {
            "--axis" => axis = Some(value),
            "--owner" => owner = Some(value),
            "--due" => due = Some(due_at(&value, zone)?),
            "--says" => says = Some(value),
            "--kind" => kind = value,
            _ => return Err(format!("{flag} isn't a flag it takes")),
        }
    }
    Ok(Asked::Owe(Owed {
        key,
        axis: axis.ok_or("it needs --axis")?,
        owner: owner.ok_or("it needs --owner")?,
        due: due.ok_or("it needs --due")?,
        says: says.ok_or("it needs --says")?,
        kind,
    }))
}

/// `when`, as `2026-11-14T18:00` in `zone`, in seconds since the epoch.
fn due_at(when: &str, zone: &TimeZone) -> Result<i64, String> {
    DateTime::strptime("%Y-%m-%dT%H:%M", when)
        .and_then(|at| at.to_zoned(zone.clone()))
        .map(|at| at.timestamp().as_second())
        .map_err(|_| format!("--due {when} isn't a time such as 2026-11-14T18:00"))
}

/// Files `owed` in `ledger` on `date`, at `now`: a must already open under
/// its key takes the new date and words, so filing again moves it, and is
/// said late again should its new date pass.
pub fn owe(ledger: &mut Vec<Entry>, owed: Owed, date: &str, now: i64) {
    let fell_when = format!("its owner's `cadence owe --done {}`", owed.key);
    let evidence = format!("filed by {} with `cadence owe`", owed.owner);
    if let Some(entry) = ledger.iter_mut().find(|e| e.open && e.key == owed.key) {
        entry.late = entry.late && entry.due == owed.due;
        entry.axis = owed.axis;
        entry.owner = Some(owed.owner);
        entry.due = owed.due;
        entry.words = owed.says;
        entry.kind = Some(owed.kind);
        entry.evidence = evidence;
        entry.fell_when = fell_when;
        date.clone_into(&mut entry.last);
        entry.last_to = now;
        return;
    }
    ledger.push(Entry {
        key: owed.key,
        owner: Some(owed.owner),
        first: date.to_owned(),
        last: date.to_owned(),
        last_to: now,
        due: owed.due,
        words: owed.says,
        axis: owed.axis,
        evidence,
        fell_when,
        open: true,
        // Its owner filed it, so it needn't be told of it.
        told: Some(date.to_owned()),
        late: false,
        kind: Some(owed.kind),
    });
}

/// Closes the open must filed as `key`, saying whether there was one. A
/// finding of cadence's own isn't closed by hand: it closes once its
/// figure is back.
pub fn settle(ledger: &mut [Entry], key: &str) -> bool {
    ledger
        .iter_mut()
        .find(|e| e.open && e.kind.is_some() && e.key == key)
        .map(|entry| entry.open = false)
        .is_some()
}

/// Runs `cadence owe` on the ledger in `folder` at `now`.
pub fn run(args: impl Iterator<Item = String>, folder: &Path, now: i64) -> ExitCode {
    let zone = TimeZone::system();
    let asked = match parse(args, &zone) {
        Ok(asked) => asked,
        Err(why) => {
            eprintln!("cadence: {why}\nusage: {USAGE}");
            return ExitCode::from(2);
        }
    };
    let kept_at = folder.join("findings.jsonl");
    let mut ledger = findings::read(&kept_at);
    let said = match asked {
        Asked::Owe(owed) => {
            let key = owed.key.clone();
            let date = Timestamp::from_second(now)
                .map(|at| at.to_zoned(zone).date().to_string())
                .unwrap_or_default();
            owe(&mut ledger, owed, &date, now);
            format!("cadence: {key} is owed, open until `cadence owe --done {key}`")
        }
        Asked::Done(key) => {
            if !settle(&mut ledger, &key) {
                eprintln!(
                    "cadence: no open must is filed as {key}; a finding of cadence's own \
                     closes once its figure is back"
                );
                return ExitCode::FAILURE;
            }
            format!("cadence: {key} is done")
        }
    };
    if let Err(err) = findings::keep(&kept_at, &ledger) {
        eprintln!("cadence: keeping {}: {err}", kept_at.display());
        return ExitCode::FAILURE;
    }
    println!("{said}");
    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::DAY;
    use crate::findings::tests::day;

    /// 2026-11-14 18:00 UTC.
    const DUE: i64 = 1_794_679_200;

    fn asked(words: &[&str]) -> Result<Asked, String> {
        parse(words.iter().map(|w| (*w).to_owned()), &TimeZone::UTC)
    }

    fn owed(key: &str, due: i64) -> Owed {
        Owed {
            key: key.to_owned(),
            axis: "security".to_owned(),
            owner: "S".to_owned(),
            due,
            says: format!("{key} is still open"),
            kind: "hole".to_owned(),
        }
    }

    #[test]
    fn a_must_is_read_with_its_due_in_the_zone_given() {
        let words = [
            "security:s2",
            "--axis",
            "security",
            "--owner",
            "S",
            "--due",
            "2026-11-14T18:00",
            "--says",
            "s2 open",
        ];
        assert_eq!(
            asked(&words),
            Ok(Asked::Owe(Owed {
                key: "security:s2".to_owned(),
                axis: "security".to_owned(),
                owner: "S".to_owned(),
                due: DUE,
                says: "s2 open".to_owned(),
                kind: "must".to_owned(),
            })),
            "a must wasn't read as filed, or its due not in the zone given"
        );
        assert_eq!(
            asked(&["security:s2", "--done"]),
            Err("--done has no value".to_owned()),
            "a flag with no value was taken"
        );
        assert_eq!(
            asked(&["security:s2", "--axis", "security"]),
            Err("it needs --owner".to_owned()),
            "a must with no owner was filed"
        );
        assert_eq!(
            asked(&["--done", "security:s2"]),
            Ok(Asked::Done("security:s2".to_owned())),
            "--done wasn't read as closing the key"
        );
    }

    #[test]
    fn a_must_stays_open_through_days_that_never_find_it_until_done() {
        let mut ledger = Vec::new();
        owe(&mut ledger, owed("security:s2", DUE), "d1", DAY);
        for at in 2..6 {
            findings::update(&mut ledger, &day("d", at * DAY), &[], &BTreeMap::new());
        }
        assert!(
            ledger[0].open,
            "days that didn't find a must closed it: {ledger:?}"
        );
        assert!(
            settle(&mut ledger, "security:s2"),
            "its owner couldn't close a must"
        );
        assert!(!ledger[0].open, "a must its owner closed stayed open");
    }

    #[test]
    fn a_finding_of_cadences_own_isnt_closed_by_hand() {
        let mut ledger = Vec::new();
        let found = crate::judge::Finding {
            key: "crashes:tec".to_owned(),
            bar: None,
            before: None,
            now: 1.0,
            words: "tec crashed".to_owned(),
        };
        findings::update(&mut ledger, &day("d1", DAY), &[found], &BTreeMap::new());
        assert!(
            !settle(&mut ledger, "crashes:tec") && ledger[0].open,
            "a finding of cadence's own was closed by hand"
        );
    }

    #[test]
    fn a_must_isnt_told_to_the_owner_who_filed_it_and_is_late_once_past_its_date() {
        let mut ledger = Vec::new();
        owe(&mut ledger, owed("security:s2", DUE), "d1", DAY);
        assert!(
            findings::tell(&mut ledger, "d1", Some("W")).is_empty(),
            "a must was told to the owner who filed it"
        );
        assert!(
            findings::late(&mut ledger, DUE).is_empty(),
            "a must at its date was late"
        );
        assert_eq!(
            findings::late(&mut ledger, DUE + 1),
            vec!["Late: security:s2 is still open, open since d1, with S"],
            "a must past its date wasn't said late with its owner"
        );
        assert!(
            findings::late(&mut ledger, DUE + 2).is_empty(),
            "a late must was said twice"
        );
    }

    #[test]
    fn filing_a_must_again_moves_its_date_rather_than_adding_one() {
        let mut ledger = Vec::new();
        owe(&mut ledger, owed("security:s2", DUE), "d1", DAY);
        let _ = findings::late(&mut ledger, DUE + 1);
        owe(&mut ledger, owed("security:s2", DUE + DAY), "d2", DUE + 1);
        assert_eq!(
            ledger.len(),
            1,
            "a must filed again was added twice: {ledger:?}"
        );
        assert_eq!(
            (ledger[0].due, ledger[0].first.as_str()),
            (DUE + DAY, "d1"),
            "a must filed again didn't take its new date and keep its first day"
        );
        assert_eq!(
            findings::late(&mut ledger, DUE + DAY + 1).len(),
            1,
            "a must moved to a new date wasn't said late again past it"
        );
    }
}
