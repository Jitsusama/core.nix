//! `cadence owe`: a dated must a quest files in the loop's ledger itself,
//! such as a known exposure and the date it closes by, so the loop's axes
//! read it beside cadence's own findings. No day's read finds one, so
//! `cadence day` never closes it; its owner does, with `--done`. The owner
//! of a finding of cadence's own says what fixed it the same way, with the
//! change or why it's expected after its key, and it closes once its
//! figure is back two days, never on its figure alone. Past its
//! date the orchestrator hears of it once, as of any finding, and its
//! owner, who filed it, is told nothing. On 12 November the security
//! owner asked for a way to file its plan's musts this way.

use jiff::Timestamp;
use jiff::civil::DateTime;
use jiff::tz::TimeZone;
use terminal_kit::{Exit, Failure};

use crate::findings::{self, Entry};
use crate::{failed, folders, hour, wrongly};

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

/// What `cadence owe` was asked: to file a must, or to close one, or a
/// finding with what fixed it.
#[derive(Debug, PartialEq, Eq)]
pub enum Asked {
    Owe(Owed),
    Done { key: String, fix: Option<String> },
}

/// Reads what was asked, its due read as a time in `zone`. A must is
/// `must` unless a kind is given.
pub fn parse(mut args: impl Iterator<Item = String>, zone: &TimeZone) -> Result<Asked, String> {
    let key = args.next().ok_or("it needs a key")?;
    if key == "--done" {
        let key = args.next().ok_or("--done needs a key")?;
        let fix: Vec<String> = args.collect();
        if let Some(flag) = fix.iter().find(|word| word.starts_with("--")) {
            return Err(format!(
                "--done takes no {flag}: what follows its key is the fix"
            ));
        }
        let fix = fix.join(" ");
        let fix = (!fix.trim().is_empty()).then_some(fix);
        return Ok(Asked::Done { key, fix });
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
    // A finding of cadence's own under the key stays as it is, so a must
    // filed over it can't close it without its owner's word of the fix.
    if let Some(entry) = ledger
        .iter_mut()
        .find(|e| e.open && e.key == owed.key && e.kind.is_some())
    {
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
    // A drag that closed and is due again is the same drag come back, so
    // its line reopens, counted, rather than a second line that reads as
    // new. Unlike a finding of cadence's own, it takes today as its first
    // day, as a new line would, so an ask made once a day still reads as
    // made today; when it first opened is lost, and `reopened` says how
    // often it has come back.
    if owed.kind == "drag"
        && let Some(entry) = ledger
            .iter_mut()
            .rev()
            .find(|e| e.key == owed.key && e.kind.as_deref() == Some("drag"))
    {
        entry.open = true;
        entry.reopened += 1;
        entry.late = false;
        entry.told = Some(date.to_owned());
        date.clone_into(&mut entry.first);
        entry.axis = owed.axis;
        entry.owner = Some(owed.owner);
        entry.due = owed.due;
        entry.words = owed.says;
        entry.evidence = evidence;
        entry.fell_when = fell_when;
        date.clone_into(&mut entry.last);
        entry.last_to = now;
        entry.hub = None;
        entry.put_to.clear();
        entry.back = None;
        entry.fixed = None;
        entry.closed = None;
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
        reopened: 0,
        hub: None,
        put_to: Vec::new(),
        back: None,
        fixed: None,
        closed: None,
    });
}

/// Closes the open must filed as `key`, saying whether there was one. A
/// finding of cadence's own isn't closed this way: its owner says what
/// fixed it (`findings::fixed`), and it closes once its figure is back.
pub fn settle(ledger: &mut [Entry], key: &str) -> bool {
    ledger
        .iter_mut()
        .find(|e| e.open && e.kind.is_some() && e.key == key)
        .map(|entry| entry.open = false)
        .is_some()
}

/// Closes the must filed as `key` at `now`, or says the finding filed as
/// `key` was fixed by `fix`, saying which; a finding with no fix said is
/// refused, since its figure gone quiet shows no fix. A drag closed this
/// way keeps when, and the fix its owner named; so does a class of cause
/// the hour already closed, when its fix is named, since a root fix often
/// lands after its class dips under the bar, and its old cases still
/// count. Another drag the hour closed holds nothing, so it is unfiled. A
/// key ending in `:` closes every open must under it, as [`done_under`]
/// does, unless a must is filed as that key itself, which then closes
/// alone.
fn done(ledger: &mut [Entry], key: &str, fix: Option<&str>, now: i64) -> Result<String, Failure> {
    if let Some(entry) = ledger
        .iter_mut()
        .find(|e| e.open && e.kind.is_some() && e.key == key)
    {
        let drag = close(entry, fix, now);
        // A class closed with no fix named is held by nothing, so the next
        // hour that still finds it asks again: the owner is told so, rather
        // than reading "done" as the week's hold.
        return Ok(match fix {
            Some(fix) => format!("cadence: {key} is done, by {fix}"),
            None if drag && hour::held_when_fixed(key) => format!(
                "cadence: {key} is done, but with no fix named the next hour that still \
                 finds it asks again"
            ),
            None => format!("cadence: {key} is done"),
        });
    }
    // The exact key is tried first, so a must filed as a key ending in `:`
    // can still be closed by it, rather than only ever with all under it.
    if key.ends_with(':') {
        return done_under(ledger, key, fix, now);
    }
    let unfiled = || {
        Failure::new(
            Exit::NotFound,
            format!("no open must or finding is filed as {key}"),
        )
    };
    let settled =
        ledger.iter_mut().rev().find(|e| e.key == key).filter(|e| {
            !e.open && e.kind.as_deref() == Some("drag") && hour::held_when_fixed(&e.key)
        });
    if let (Some(entry), Some(fix)) = (settled, fix) {
        entry.closed = Some(now);
        entry.fixed = Some(fix.to_owned());
        return Ok(format!(
            "cadence: {key} was closed already, and is now held by {fix}"
        ));
    }
    let Some(fix) = fix else {
        if ledger.iter().any(|e| e.open && e.key == key) {
            return Err(wrongly(
                "owe",
                format!("{key} is a finding, so it needs what fixed it after its key"),
            ));
        }
        return Err(unfiled());
    };
    match findings::fixed(ledger, key, fix) {
        Some(true) => Ok(format!("cadence: {key} is closed, fixed by {fix}")),
        Some(false) => Ok(format!(
            "cadence: {key} is fixed by {fix}, closing once its figure is back two days"
        )),
        None => Err(unfiled()),
    }
}

/// Closes the open must `entry` at `now`, saying whether it was a drag;
/// a drag keeps when, and the fix its owner named, so one its measure
/// counts over a week isn't filed again from the cases the fix came after.
fn close(entry: &mut Entry, fix: Option<&str>, now: i64) -> bool {
    entry.open = false;
    let drag = entry.kind.as_deref() == Some("drag");
    if drag {
        entry.closed = Some(now);
        entry.fixed = fix.map(str::to_owned);
    }
    drag
}

/// Closes every open must whose key opens with `prefix`, at `now`, each as
/// its own key would close it, naming each. The hour files an ask a
/// message about each claim of his rule, so settling a backlog, or the
/// measure's revert, would otherwise take a `--done` for every one. Only
/// that family closes this way, as [`hour::closed_together`] holds it,
/// since a prefix such as `hour:` would close every owner's musts, each
/// told nothing. A finding of cadence's own under it is left open, since
/// each closes only on its own fix.
fn done_under(
    ledger: &mut [Entry],
    prefix: &str,
    fix: Option<&str>,
    now: i64,
) -> Result<String, Failure> {
    if !hour::closed_together(prefix) {
        return Err(wrongly(
            "owe",
            format!(
                "{prefix} closes nothing at once: only the asks about claims of his rule do, \
                 so close each must under it by its own key"
            ),
        ));
    }
    let mut closed = Vec::new();
    for entry in ledger
        .iter_mut()
        .filter(|e| e.open && e.kind.is_some() && e.key.starts_with(prefix))
    {
        close(entry, fix, now);
        closed.push(entry.key.clone());
    }
    if closed.is_empty() {
        return Err(Failure::new(
            Exit::NotFound,
            format!("no open must is filed under {prefix}"),
        ));
    }
    let (count, named) = (closed.len(), closed.join(", "));
    let are = if count == 1 { "is" } else { "are" };
    Ok(match fix {
        Some(fix) => format!("cadence: {count} open under {prefix} {are} done, by {fix}: {named}"),
        None => format!("cadence: {count} open under {prefix} {are} done: {named}"),
    })
}

/// Runs `cadence owe` on the ledger in cadence's folder at `now`, once
/// what was asked has been read.
pub fn run(args: impl Iterator<Item = String>, now: i64) -> Result<(), Failure> {
    let zone = TimeZone::system();
    let asked = parse(args, &zone).map_err(|why| wrongly("owe", why))?;
    let (_, _, folder) = folders().map_err(failed)?;
    let kept_at = folder.join("findings.jsonl");
    let _held = findings::lock(&kept_at).map_err(|err| {
        Failure::new(Exit::Failed, format!("locking {}", kept_at.display()))
            .because(err.to_string())
    })?;
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
        Asked::Done { key, fix } => done(&mut ledger, &key, fix.as_deref(), now)?,
    };
    findings::keep(&kept_at, &ledger).map_err(|err| {
        Failure::new(Exit::Failed, format!("keeping {}", kept_at.display()))
            .because(err.to_string())
    })?;
    println!("{said}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use super::*;
    use crate::DAY;
    use crate::findings::tests::{day, late};

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
            Ok(Asked::Done {
                key: "security:s2".to_owned(),
                fix: None,
            }),
            "--done wasn't read as closing the key"
        );
        assert_eq!(
            asked(&["--done", "crashes:tec", "core #6000", "and", "core #6001"]),
            Ok(Asked::Done {
                key: "crashes:tec".to_owned(),
                fix: Some("core #6000 and core #6001".to_owned()),
            }),
            "--done didn't read what fixed it after the key"
        );
    }

    #[test]
    fn a_must_stays_open_through_days_that_never_find_it_until_done() {
        let mut ledger = Vec::new();
        owe(&mut ledger, owed("security:s2", DUE), "d1", DAY);
        for at in 2..6 {
            findings::update(
                &mut ledger,
                &day("d", at * DAY),
                &[],
                &BTreeMap::new(),
                &BTreeSet::new(),
            );
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
        findings::update(
            &mut ledger,
            &day("d1", DAY),
            &[found],
            &BTreeMap::new(),
            &BTreeSet::new(),
        );
        assert!(
            !settle(&mut ledger, "crashes:tec") && ledger[0].open,
            "a finding of cadence's own was closed by hand"
        );
        let unsaid = done(&mut ledger, "crashes:tec", None, DAY).map_err(|f| f.exit);
        assert_eq!(
            unsaid,
            Err(Exit::Usage),
            "a finding closed with no fix said"
        );
        let waits = "cadence: crashes:tec is fixed by core #6000, closing once its figure is back \
                     two days";
        let said = done(&mut ledger, "crashes:tec", Some("core #6000"), DAY).map_err(|f| f.exit);
        assert_eq!(
            said,
            Ok(waits.to_owned()),
            "a fix said wasn't kept, waiting on its figure"
        );
        let nowhere = done(&mut ledger, "crashes:nothing", Some("core #6000"), DAY);
        assert_eq!(
            nowhere.map_err(|f| f.exit),
            Err(Exit::NotFound),
            "a key filed nowhere took a fix"
        );
        ledger[0].fixed = None;
        owe(&mut ledger, owed("crashes:tec", DUE), "d1", DAY);
        let over = done(&mut ledger, "crashes:tec", None, DAY).map_err(|f| f.exit);
        assert_eq!(
            (over, ledger[0].open, ledger[0].kind.as_deref()),
            (Ok("cadence: crashes:tec is done".to_owned()), true, None),
            "a must filed over a finding's key closed the finding with no fix said"
        );
        assert_eq!(
            asked(&["--done", "crashes:tec", "core #6000", "--axis"]),
            Err("--done takes no --axis: what follows its key is the fix".to_owned()),
            "a flag after the key was taken as the fix"
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
            late(&mut ledger, DUE).is_empty(),
            "a must at its date was late"
        );
        assert_eq!(
            late(&mut ledger, DUE + 1),
            vec!["Late: security:s2 is still open, open since d1, with S"],
            "a must past its date wasn't said late with its owner"
        );
        assert!(
            late(&mut ledger, DUE + 2).is_empty(),
            "a late must was said twice"
        );
    }

    #[test]
    fn filing_a_must_again_moves_its_date_rather_than_adding_one() {
        let mut ledger = Vec::new();
        owe(&mut ledger, owed("security:s2", DUE), "d1", DAY);
        let _ = late(&mut ledger, DUE + 1);
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
            late(&mut ledger, DUE + DAY + 1).len(),
            1,
            "a must moved to a new date wasn't said late again past it"
        );
    }

    /// A miss's ask and a queue's are drags as a measure's are, so a
    /// repeat of any of them is counted on its own line.
    #[test]
    fn a_drag_of_any_kind_filed_again_after_it_closed_reopens_its_own_line() {
        for key in [
            "hour:miss:the host crashed",
            "hour:queue:core",
            "hour:slips",
        ] {
            let drag = || Owed {
                kind: "drag".to_owned(),
                ..owed(key, DUE)
            };
            let mut ledger = Vec::new();
            owe(&mut ledger, drag(), "d1", DAY);
            ledger[0].late = true;
            ledger[0].put_to = vec!["S".to_owned()];
            let said = done(&mut ledger, key, Some("core #6000"), 2 * DAY).map_err(|f| f.exit);
            assert_eq!(
                (said, ledger[0].closed, ledger[0].fixed.as_deref()),
                (
                    Ok(format!("cadence: {key} is done, by core #6000")),
                    Some(2 * DAY),
                    Some("core #6000")
                ),
                "{key} closed by its owner didn't keep when, or the fix named"
            );
            let again = Owed {
                due: DUE + 3 * DAY,
                says: format!("{key} is back"),
                owner: "T".to_owned(),
                ..drag()
            };
            owe(&mut ledger, again, "d3", 3 * DAY);
            let entry = &ledger[0];
            assert_eq!(
                (
                    ledger.len(),
                    entry.open,
                    entry.reopened,
                    entry.first.as_str()
                ),
                (1, true, 1, "d3"),
                "{key} filed again after it closed opened a second line: {ledger:?}"
            );
            assert_eq!(
                (
                    entry.due,
                    entry.words.as_str(),
                    entry.closed,
                    entry.fixed.as_deref()
                ),
                (DUE + 3 * DAY, format!("{key} is back").as_str(), None, None),
                "{key} reopened kept its closed run's date, words or fix: {entry:?}"
            );
            assert_eq!(
                (entry.owner.as_deref(), entry.evidence.as_str()),
                (Some("T"), "filed by T with `cadence owe`"),
                "{key} reopened with a new owner still named the old: {entry:?}"
            );
            assert!(
                !entry.late && entry.put_to.is_empty(),
                "{key} reopened still read as late from its closed run: {entry:?}"
            );
        }
    }

    /// A root fix often lands after the hour closed its class, which fell
    /// under the bar; naming it then still holds the class.
    #[test]
    fn a_fix_named_after_the_hour_closed_its_drag_is_kept() {
        let key = "hour:causes-recurring:test passes undone";
        let mut ledger = Vec::new();
        owe(
            &mut ledger,
            Owed {
                kind: "drag".to_owned(),
                ..owed(key, DUE)
            },
            "d1",
            DAY,
        );
        assert!(settle(&mut ledger, key), "the hour couldn't close its drag");
        let bare = done(&mut ledger, key, None, 2 * DAY).map_err(|f| f.exit);
        assert_eq!(
            (bare, ledger[0].closed),
            (Err(Exit::NotFound), None),
            "a bare --done on a closed drag held it, though it named no fix"
        );
        let said = done(&mut ledger, key, Some("core #6000"), 2 * DAY).map_err(|f| f.exit);
        assert_eq!(
            (said, ledger[0].closed, ledger[0].fixed.as_deref()),
            (
                Ok(format!(
                    "cadence: {key} was closed already, and is now held by core #6000"
                )),
                Some(2 * DAY),
                Some("core #6000")
            ),
            "a fix named after the hour closed the drag wasn't kept"
        );
        assert!(!ledger[0].open, "naming the fix opened the drag again");
    }

    /// The hour files an ask a message about each claim of his rule, so a
    /// key ending in `:` closes every open must under it at once, and
    /// nothing outside it, while an exact key still closes itself alone.
    #[test]
    fn a_key_ending_in_a_colon_closes_every_open_must_under_it() {
        let prefix = "hour:rulings-unsourced:";
        let mut ledger = Vec::new();
        for key in [
            "hour:rulings-unsourced:T6E7X6:0000000a",
            "hour:rulings-unsourced:5UTLKM:0000000b",
            "hour:rulings-unsourcedness",
            "hour:slips",
        ] {
            let drag = Owed {
                kind: "drag".to_owned(),
                ..owed(key, DUE)
            };
            owe(&mut ledger, drag, "d1", DAY);
        }
        let mut own = ledger[0].clone();
        own.key = format!("{prefix}A2VLJN:0000000c");
        own.kind = None;
        ledger.push(own);
        let said = done(&mut ledger, prefix, None, 2 * DAY).map_err(|f| f.exit);
        assert_eq!(
            said,
            Ok(format!(
                "cadence: 2 open under {prefix} are done: {prefix}T6E7X6:0000000a, \
                 {prefix}5UTLKM:0000000b"
            )),
            "a key ending in a colon didn't close every must under it, or didn't name each"
        );
        let open: Vec<&str> = ledger
            .iter()
            .filter(|e| e.open)
            .map(|e| e.key.as_str())
            .collect();
        assert_eq!(
            open,
            [
                "hour:rulings-unsourcedness",
                "hour:slips",
                "hour:rulings-unsourced:A2VLJN:0000000c",
            ],
            "a must under the key stayed open, or one outside it, or a finding of cadence's own \
             under it, closed"
        );
        assert_eq!(
            ledger[0].closed,
            Some(2 * DAY),
            "a drag closed under its key's opening didn't keep when"
        );
        let again = done(&mut ledger, prefix, None, 2 * DAY).map_err(|f| f.exit);
        assert_eq!(
            again,
            Err(Exit::NotFound),
            "a key ending in a colon with no must open under it was said done"
        );
        let exact = done(&mut ledger, "hour:slips", None, 2 * DAY).map_err(|f| f.exit);
        assert_eq!(
            (exact, ledger[2].open, ledger[3].open),
            (Ok("cadence: hour:slips is done".to_owned()), true, false),
            "an exact key didn't close itself alone"
        );
    }

    /// A must filed as a key ending in `:` is closed by that key alone,
    /// rather than the key only ever closing every must under it, so one
    /// filed so can still be closed and none beside it is closed unasked.
    #[test]
    fn a_must_filed_as_a_key_ending_in_a_colon_closes_alone() {
        let key = "hour:rulings-unsourced:T6E7X6:";
        let mut ledger = Vec::new();
        for filed in [key, "hour:rulings-unsourced:T6E7X6:0000000a"] {
            owe(&mut ledger, owed(filed, DUE), "d1", DAY);
        }
        let said = done(&mut ledger, key, None, 2 * DAY).map_err(|f| f.exit);
        assert_eq!(
            (said, ledger[0].open, ledger[1].open),
            (Ok(format!("cadence: {key} is done")), false, true),
            "a must filed as a key ending in a colon wasn't closed by it alone, or one under it \
             closed with it"
        );
    }

    /// A key ending in `:` closes at once only the asks about claims of his
    /// rule, every sender's or one sender's, so a prefix such as `hour:` is
    /// refused rather than closing every owner's musts, and closes nothing.
    #[test]
    fn only_the_asks_about_claims_close_under_a_prefix() {
        let mut ledger = Vec::new();
        for key in [
            "hour:rulings-unsourced:T6E7X6:0000000a",
            "hour:rulings-unsourced:5UTLKM:0000000b",
            "hour:slips",
        ] {
            owe(&mut ledger, owed(key, DUE), "d1", DAY);
        }
        for prefix in [
            "hour:",
            "hour:rulings",
            "hour:rulings-unsourced:T6E7X6:0000000a:",
        ] {
            let said = done(&mut ledger, prefix, None, 2 * DAY).map_err(|f| f.exit);
            assert!(
                said.is_err() && ledger.iter().all(|e| e.open),
                "a prefix outside the asks about claims closed a must: {prefix}"
            );
        }
        let one = done(&mut ledger, "hour:rulings-unsourced:T6E7X6:", None, 2 * DAY);
        let open: Vec<bool> = ledger.iter().map(|e| e.open).collect();
        assert_eq!(
            (one.is_ok(), open),
            (true, vec![false, true, true]),
            "one sender's prefix didn't close that sender's asks alone"
        );
    }

    /// Only a class of cause is held by its fix, so a fix named for another
    /// drag the hour closed is refused, as before, rather than said held.
    #[test]
    fn a_fix_named_after_the_hour_closed_another_drag_is_unfiled() {
        let key = "hour:slips";
        let mut ledger = Vec::new();
        owe(
            &mut ledger,
            Owed {
                kind: "drag".to_owned(),
                ..owed(key, DUE)
            },
            "d1",
            DAY,
        );
        assert!(settle(&mut ledger, key), "the hour couldn't close its drag");
        let said = done(&mut ledger, key, Some("core #6000"), 2 * DAY).map_err(|f| f.exit);
        assert_eq!(
            (said, ledger[0].closed, ledger[0].fixed.as_deref()),
            (Err(Exit::NotFound), None, None),
            "a drag that no fix holds was said held once the hour closed it"
        );
    }

    /// A class of cause closed with no fix named is asked again the next
    /// hour that still finds it, and its owner is told so.
    #[test]
    fn a_class_closed_with_no_fix_says_it_holds_only_till_the_next_hour() {
        let key = "hour:causes-recurring:test passes undone";
        let mut ledger = Vec::new();
        owe(
            &mut ledger,
            Owed {
                kind: "drag".to_owned(),
                ..owed(key, DUE)
            },
            "d1",
            DAY,
        );
        let said = done(&mut ledger, key, None, 2 * DAY).map_err(|f| f.exit);
        assert_eq!(
            said,
            Ok(format!(
                "cadence: {key} is done, but with no fix named the next hour that still \
                 finds it asks again"
            )),
            "a bare close of a class read as the week's hold"
        );
    }
}
