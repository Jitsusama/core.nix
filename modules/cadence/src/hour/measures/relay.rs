//! The model relay as the sessions meet it, from model-relay-agents's
//! lines in the user journal. On 26 November proxy.shopify.ai reset the
//! relay's streams 1,900 to 10,250 times an hour from 08:00, against about
//! 150 an hour the day before, and the relay answered pi 502 hundreds of
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

/// How long the hour waits for journalctl's reading of the relay's lines
/// before reading the measures as unread, as wifid's are.
const READ_WITHIN: std::time::Duration = std::time::Duration::from_secs(30);

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
    /// The requests carried by the connections that closed in the hour,
    /// from their "carried N asked" lines.
    asked: u64,
    reset_why: BTreeMap<String, usize>,
    direct_why: BTreeMap<String, usize>,
    answered_why: BTreeMap<String, usize>,
}

/// The four measures of the hour, from the relay's lines as journalctl
/// printed them, or each unjudged where they couldn't be read. An hour
/// with no line from the relay is judged, each count nought, since the
/// relay exits when idle and an idle relay failed nobody. The share is
/// unjudged in an hour no connection closed in, as it has nothing carried
/// to weigh its resets against.
#[must_use]
pub fn relay(journal: Option<&str>) -> Vec<Measure> {
    let Some(journal) = journal else {
        return [RESETS, DIRECT, SHARE, ANSWERED_502]
            .into_iter()
            .map(|key| unjudged(key, ": the relay's journal unread"))
            .collect();
    };
    let tally = tallied(journal);
    let share = if tally.asked == 0 {
        unjudged(
            SHARE,
            ": no connection closed this hour, so nothing carried to weigh",
        )
    } else {
        #[expect(
            clippy::cast_precision_loss,
            reason = "an hour's requests are far below where f64 loses a unit"
        )]
        let percent = count(tally.resets) * 100.0 / tally.asked as f64;
        Measure {
            key: SHARE.to_owned(),
            class: Class::Functional,
            says: format!(
                "per cent of the model relay's requests the origin reset this hour, of {} \
                 carried by the connections that closed in it",
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
            "requests proxy.shopify.ai reset on the model relay this hour",
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
        .map(|(cause, n)| format!("{n} on {cause}"))
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
/// - `connection 1.432 carried 56 asked, 11 of them cancelled and 0 cut`
///   is what a closed connection carried.
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
        if let Some(asked) = carried(line) {
            tally.asked += asked;
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
    Some(
        before
            .split_once(" on it: ")
            .map_or(before, |(_, cause)| cause),
    )
}

/// The requests a closed connection carried, from its `connection N
/// carried N asked, ...` line.
fn carried(line: &str) -> Option<u64> {
    let rest = line.strip_prefix("connection ")?;
    let (_, rest) = rest.split_once(" carried ")?;
    let (asked, _) = rest.split_once(" asked")?;
    asked.parse().ok()
}

#[cfg(test)]
mod tests;
