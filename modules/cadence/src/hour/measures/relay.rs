//! The model relay as the sessions meet it, from model-relay-agents's
//! lines in the user journal. On 26 November the model origin reset the
//! relay's streams 1,900 to 10,250 times an hour from 08:00, against 4 to
//! 808 an hour the day before, and the relay answered pi 502 hundreds of
//! times an hour, which Joel saw in his own sessions at 16:38 before any
//! card did (WCJQSG, d892570d). Four counts of the hour say it, each held
//! to the relay keeper's ceiling (OXUW1Y, cdb89d6a): the origin's resets,
//! the requests the relay sent pi to make directly, its resets as a share
//! of the requests it carried, and the 502s it answered pi with.

use std::collections::BTreeMap;

use super::{Class, Measure, count};

/// What each of the four is known by opens so, which files each apart
/// with the relay's keeper, whatever leads the hour (owing.rs).
pub const PREFIX: &str = "relay:";

/// The origin's resets of a request, counted by the relay's lines.
pub const RESETS: &str = "relay:origin-resets";
/// The requests the relay gave back for pi to make itself.
pub const DIRECT: &str = "relay:goes-direct";
/// The resets as a share of the requests carried.
pub const SHARE: &str = "relay:reset-share";
/// The requests the relay could carry nowhere and answered 502.
pub const ANSWERED_502: &str = "relay:answered-502";

/// Who keeps the relay, and so moves each of the four.
const OWNER: &str = "OXUW1Y";

/// The ceilings OXUW1Y set on 26 November (cdb89d6a): per hour, 400 resets,
/// 150 sent direct, resets past 2% of what was carried, and 50 502s.
pub const RESETS_AT_MOST: f64 = 400.0;
pub const DIRECT_AT_MOST: f64 = 150.0;
pub const SHARE_AT_MOST: f64 = 2.0;
pub const ANSWERED_502_AT_MOST: f64 = 50.0;

/// How many of each count's causes the card shows behind it.
const CAUSES_SHOWN: usize = 3;

/// The most of a cause the card and the keeper's ask carry. A cause is
/// the error text another machine sent, so it is cut short rather than
/// carried whole into a request an agent reads.
const CAUSE_AT_MOST: usize = 120;

/// How long the hour waits for journalctl's reading of the relay's lines
/// before reading the measures as unread, as wifid's are. An hour of a
/// flood held about 15,000 lines on 26 November, read in under 3 s, so a
/// minute leaves the worst hour room to be read.
const READ_WITHIN: std::time::Duration = std::time::Duration::from_secs(60);

/// The four measures of the hour to `now`, read from the relay's journal.
#[must_use]
pub fn hour(now: i64) -> Vec<Measure> {
    relay(read(now).as_deref())
}

/// The relay's lines in the hour to `now`, each its message alone, or none
/// when journalctl fails or takes past `READ_WITHIN`, said in cadence's
/// journal.
#[must_use]
pub fn read(now: i64) -> Option<String> {
    let args = journal(now);
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    crate::watch::down::answer("journalctl", &args, READ_WITHIN)
        .inspect_err(|e| eprintln!("cadence: the model relay's journal: {e}"))
        .ok()
}

/// journalctl's arguments for the relay's lines in the hour to `now`. The
/// relay is the user's unit, and an hour can hold tens of thousands of its
/// lines, so each is read as its message alone rather than as json.
#[must_use]
pub fn journal(now: i64) -> [String; 8] {
    [
        "--user".to_owned(),
        "-u".to_owned(),
        "model-relay-agents.service".to_owned(),
        "-o".to_owned(),
        "cat".to_owned(),
        "--no-pager".to_owned(),
        format!("--since=@{}", now - 3600),
        format!("--until=@{now}"),
    ]
}

/// What an hour of the relay's lines counts.
#[derive(Default)]
struct Tally {
    resets: usize,
    direct: usize,
    answered_502: usize,
    /// The requests asked of the relay between its first and last
    /// ten-minute tally of the hour, and the resets between them.
    asked: u64,
    resets_tallied: usize,
    /// Its last tally's requests asked, and the resets counted by then.
    last_tally: Option<(u64, usize)>,
    reset_why: BTreeMap<String, usize>,
    direct_why: BTreeMap<String, usize>,
    answered_why: BTreeMap<String, usize>,
}

/// The four measures of the hour, from the relay's lines as journalctl
/// printed them, or each unjudged where they couldn't be read. An hour
/// with no line from the relay is unjudged too: the relay carried
/// nothing, so pi went to the origin itself, and on 26 November, with the
/// relay stopped, its nought read green while a third of pi's calls
/// failed; pi's own calls (calls.rs) judge that hour. The share is
/// unjudged in an hour holding fewer than two of the relay's tallies, as
/// it has nothing asked to weigh its resets against.
#[must_use]
pub fn relay(journal: Option<&str>) -> Vec<Measure> {
    let why = match journal {
        None => ": the relay's journal unread",
        Some(journal) if journal.trim().is_empty() => {
            ": no line from the relay this hour, so it carried nothing"
        }
        Some(_) => "",
    };
    let Some(journal) = journal.filter(|_| why.is_empty()) else {
        return [RESETS, DIRECT, SHARE, ANSWERED_502]
            .into_iter()
            .map(|key| unjudged(key, why))
            .collect();
    };
    let tally = tallied(journal);
    let share = if tally.asked == 0 {
        unjudged(
            SHARE,
            ": fewer than two of its tallies this hour, so nothing asked to weigh",
        )
    } else {
        #[expect(
            clippy::cast_precision_loss,
            reason = "an hour's requests are far below where f64 loses a unit"
        )]
        let percent = count(tally.resets_tallied) * 100.0 / tally.asked as f64;
        Measure {
            key: SHARE.to_owned(),
            class: Class::Functional,
            says: format!(
                "per cent of the model relay's requests the origin reset this hour, of {} \
                 asked between its tallies",
                tally.asked
            ),
            now: (percent * 10.0).round() / 10.0,
            target: Some(SHARE_AT_MOST),
            owner: OWNER.to_owned(),
            behind: causes(&tally.reset_why),
        }
    };
    vec![
        judged(
            RESETS,
            "requests the model origin reset on the model relay this hour",
            tally.resets,
            RESETS_AT_MOST,
            &tally.reset_why,
        ),
        judged(
            DIRECT,
            "requests the model relay sent pi to make directly this hour",
            tally.direct,
            DIRECT_AT_MOST,
            &tally.direct_why,
        ),
        share,
        judged(
            ANSWERED_502,
            "requests the model relay answered pi 502 this hour",
            tally.answered_502,
            ANSWERED_502_AT_MOST,
            &tally.answered_why,
        ),
    ]
}

fn judged(key: &str, says: &str, n: usize, most: f64, why: &BTreeMap<String, usize>) -> Measure {
    Measure {
        key: key.to_owned(),
        class: Class::Functional,
        says: says.to_owned(),
        now: count(n),
        target: Some(most),
        owner: OWNER.to_owned(),
        behind: causes(why),
    }
}

/// The measure `key` unjudged, saying `why`.
fn unjudged(key: &str, why: &str) -> Measure {
    Measure {
        key: key.to_owned(),
        class: Class::Functional,
        says: format!("the model relay's {}{why}", key.trim_start_matches(PREFIX)),
        now: 0.0,
        target: None,
        owner: OWNER.to_owned(),
        behind: Vec::new(),
    }
}

/// The commonest causes, most first, so the owner reads what failed.
fn causes(why: &BTreeMap<String, usize>) -> Vec<String> {
    let mut most: Vec<(&String, &usize)> = why.iter().collect();
    most.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));
    most.into_iter()
        .take(CAUSES_SHOWN)
        .map(|(cause, n)| {
            let cause: String = cause.chars().take(CAUSE_AT_MOST).collect();
            format!("{n} on {cause}")
        })
        .collect()
}

/// The hour's counts, from each of the relay's lines as it writes them:
///
/// - `the origin reset a request on connection 0.407, kept, ..., after 60
///   asked and 6 cancelled on it: <error>; pi goes directly` is a reset;
/// - any line ending `pi goes directly` or saying `so pi goes directly:`
///   is a request given back to pi;
/// - `a request couldn't be carried, so pi is answered 502: <error>` is a
///   502;
/// - `172694 asked, 0 refused, 745 failed, ...` is the relay's tally of
///   what it was asked since it started, written each ten minutes and as
///   it stops. The share weighs the resets between the hour's first and
///   last tally against the requests asked between them, as a share of
///   what closed connections carried swings on which closed when.
fn tallied(journal: &str) -> Tally {
    let mut tally = Tally::default();
    for line in journal.lines() {
        // journalctl's cat prints the message alone, which the relay opens
        // with its name.
        let line = line.strip_prefix("model-relay: ").unwrap_or(line);
        if line.starts_with("the origin reset a request") {
            tally.resets += 1;
            let cause = line
                .split_once(" on it: ")
                .map_or("an unread cause", |(_, rest)| {
                    rest.split(';').next().unwrap_or(rest)
                });
            *tally.reset_why.entry(cause.to_owned()).or_default() += 1;
        }
        if let Some(cause) = direct(line) {
            tally.direct += 1;
            *tally.direct_why.entry(cause.to_owned()).or_default() += 1;
        }
        if let Some((_, cause)) = line.split_once("so pi is answered 502: ") {
            tally.answered_502 += 1;
            *tally.answered_why.entry(cause.to_owned()).or_default() += 1;
        }
        if let Some(asked) = tallied_asked(line) {
            if let Some((before, resets_then)) = tally.last_tally {
                // A tally below the last is a relay started again, which
                // counts from nought.
                tally.asked += if asked >= before {
                    asked - before
                } else {
                    asked
                };
                tally.resets_tallied += tally.resets - resets_then;
            }
            tally.last_tally = Some((asked, tally.resets));
        }
    }
    tally
}

/// Why a request went back to pi, where the line says it did: the error
/// after `so pi goes directly: `, or a reset's own error before its `; pi
/// goes directly`.
fn direct(line: &str) -> Option<&str> {
    if let Some((_, cause)) = line.split_once("so pi goes directly: ") {
        return Some(cause);
    }
    let before = line.strip_suffix("; pi goes directly")?;
    // A reset's error follows "on it: "; a request no connection would
    // take says why after its connection, which is left out so the same
    // cause groups whichever connection it was.
    Some(before.split_once(" on it: ").map_or_else(
        || before.rsplit_once(", ").map_or(before, |(_, cause)| cause),
        |(_, cause)| cause,
    ))
}

/// The requests asked of the relay since it started, from its tally line
/// `N asked, N refused, ...`.
fn tallied_asked(line: &str) -> Option<u64> {
    let (asked, rest) = line.split_once(" asked, ")?;
    rest.starts_with(|c: char| c.is_ascii_digit())
        .then(|| asked.parse().ok())
        .flatten()
}

#[cfg(test)]
mod tests;
