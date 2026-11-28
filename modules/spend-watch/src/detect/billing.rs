//! Spend billed to another key than its budget's: a budget whose logs priced
//! more than its key was billed, the warehouse feeds whose settling holds
//! that back, and the budgets sessions asked for and couldn't draw on.

use super::finding;
use crate::config::Config;
use crate::findings::{Finding, Severity};
use crate::history::{History, sql as fail};
use crate::money::{dollars, percent, whole};
use crate::picture::Picture;
use crate::time::DAY;
use std::collections::{BTreeMap, HashMap, HashSet};

/// How many of the warehouse's last settled hours a budget's logged spend
/// is held against what it billed. A day, so a quiet hour or two of a
/// team's work never reads as its key going unbilled.
const UNBILLED_HOURS: i64 = 24;

/// The least share of its logged spend a budget must be billed. A key's
/// bill runs near its list price, and the default's at 1.1 to 1.3 times
/// it on 17 and 18 November, so under half is spend gone to another key.
const BILLED_AT_LEAST: f64 = 0.5;

/// How far behind the hour a warehouse's settled bill may fall before the
/// day it settled is too old to judge by: past it, the feed has stopped
/// settling, and that is what's told. The picture keeps the logs' spend
/// by hour back this far and a day more.
const UNBILLED_STALE_HOURS: i64 = crate::picture::LISTED_HOURS - UNBILLED_HOURS;

/// A budget whose logs priced more than its key was billed: its sessions'
/// requests went on another key, the default's most likely, so its own
/// allowance never saw them. From 23:00 on 17 November to 19:00 on the
/// 18th, two team budgets' sessions logged $170.55 and $375.60, and their
/// keys were billed nothing and $2.67, because the pi host wrote its own
/// key over each conversation's.
///
/// It's held over the hours every warehouse feed the config runs has
/// settled, since a settled hour's bill no longer grows and a feed that
/// lags holds the window back rather than reading as a key billing
/// nothing. A feed settled more than two days behind has stopped, one
/// settled past now is wrong, and one billing with no settled hour holds
/// the window back; each is told in place of any budget, the wrong row
/// first, since a feed that holds back hides nothing. Both sides are pi's
/// alone: the bill's `pi` tool, and the logs' list prices as the
/// picture's series attributed them, a run's agents to the budget of the session that
/// started them. Only budgets a feed billed pi for before the window are
/// judged, since one no feed bills is estimated from the logs and can't
/// disagree with them, and a feed new within the day hasn't billed all of
/// it.
pub fn unbilled(
    picture: &Picture,
    history: &History,
    config: &Config,
) -> Result<Vec<Finding>, String> {
    let runs: HashSet<&str> = config.feeds.iter().map(|feed| feed.name.as_str()).collect();
    let settled = warehouse_settled(history, &runs)?;
    // A feed's settled hour only moves on, so one it gave past now stands
    // for good, and no day under it can be judged: the hours before it
    // weren't all billed, whatever it says. Any feed's can be, however far
    // behind another is, and however another holds back. It's told in
    // place of any budget, as a warehouse that stopped settling is.
    let now = crate::time::hour_of(picture.now);
    // Every held feed is told beside a wrong row, so another feed's row
    // can't clear a held note and start its count again.
    let held = held_findings(picture, history, &settled.held)?;
    let wrong = settled.most.filter(|(most, _)| *most > now).or_else(|| {
        settled
            .least
            .clone()
            .filter(|(least, _)| *least < now - UNBILLED_STALE_HOURS)
    });
    if let Some((through, feed)) = wrong {
        let mut told = vec![unsettled_finding(picture, &feed, through)];
        told.extend(held);
        return Ok(told);
    }
    if !held.is_empty() {
        return Ok(held);
    }
    let Some((through, _)) = settled.least else {
        return Ok(Vec::new());
    };
    let first = through - UNBILLED_HOURS;
    let budgets = &picture.budgets;
    let mut judged = HashSet::new();
    let mut billed: HashMap<String, f64> = HashMap::new();
    let mut statement = history
        .db
        .prepare(
            "SELECT feed, budget, SUM(CASE WHEN hour >= ?1 AND hour < ?2 THEN dollars ELSE 0 END),
             MIN(hour) FROM billed WHERE source = 'warehouse' AND tool = 'pi'
             GROUP BY feed, budget",
        )
        .map_err(fail)?;
    let rows = statement
        .query_map([first, through], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, f64>(2)?,
                row.get::<_, i64>(3)?,
            ))
        })
        .map_err(fail)?;
    for row in rows {
        let (feed, budget, dollars, since) = row.map_err(fail)?;
        // A feed the config no longer runs bills nothing more, so what it
        // billed once doesn't make a budget's day one to judge.
        if !runs.contains(feed.as_str()) {
            continue;
        }
        let budget = budgets.of_record(&feed, &budget);
        *billed.entry(budget.clone()).or_default() += dollars;
        if since < first {
            judged.insert(budget);
        }
    }
    let mut logged: HashMap<&str, f64> = HashMap::new();
    for (hour, budget, list) in &picture.listed {
        if (first..through).contains(hour) {
            *logged.entry(budget.as_str()).or_default() += list;
        }
    }
    let through = picture
        .zone
        .clock(crate::time::hour_start(through), picture.now);
    let mut out: Vec<Finding> = logged
        .into_iter()
        .filter(|(budget, _)| judged.contains(*budget) && !budgets.is_plan(budget))
        .filter_map(|(budget, logged)| {
            let paid = billed.get(budget).copied().unwrap_or_default();
            (logged - paid >= config.detectors.floor && paid < logged * BILLED_AT_LEAST)
                .then(|| unbilled_finding(budgets, budget, logged, paid, &through))
        })
        .collect();
    out.sort_by(|one, other| other.dollars.total_cmp(&one.dollars));
    Ok(out)
}

/// How far the warehouse feeds the config runs have settled pi's bill.
struct Settled {
    /// The hour settled through, exclusive, of the feed settled least
    /// far, and its name; none when no feed has said.
    least: Option<(i64, String)>,
    /// The same of the feed settled furthest.
    most: Option<(i64, String)>,
    /// The feeds, by name, that bill pi from the warehouse but haven't
    /// said how far they've settled, so no day can be judged on their
    /// bills until they do.
    held: Vec<String>,
}

/// How far every warehouse feed the config runs has settled. A feed
/// removed or renamed in the config keeps its row, and doesn't hold the
/// window back.
fn warehouse_settled(history: &History, runs: &HashSet<&str>) -> Result<Settled, String> {
    let mut statement = history
        .db
        .prepare("SELECT feed, through FROM settled WHERE source = 'warehouse'")
        .map_err(fail)?;
    let rows = statement
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })
        .map_err(fail)?;
    let mut least: Option<(i64, String)> = None;
    let mut most: Option<(i64, String)> = None;
    let mut settling = HashSet::new();
    for row in rows {
        let (feed, settled) = row.map_err(fail)?;
        if runs.contains(feed.as_str()) {
            // Ties go by name, so the feed told is the same each pass.
            let here = (settled, feed.clone());
            if least.as_ref().is_none_or(|was| here < *was) {
                least = Some(here.clone());
            }
            if most.as_ref().is_none_or(|was| here > *was) {
                most = Some(here);
            }
            settling.insert(feed);
        }
    }
    // A feed the config runs that bills pi from the warehouse but hasn't
    // said how far it has settled may not have billed any hour in full,
    // so no day can be judged on its bills until it does.
    let mut billing = history
        .db
        .prepare(
            "SELECT DISTINCT feed FROM billed WHERE source = 'warehouse' AND tool = 'pi'
             ORDER BY feed",
        )
        .map_err(fail)?;
    let feeds = billing
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(fail)?;
    let mut held = Vec::new();
    for feed in feeds {
        let feed = feed.map_err(fail)?;
        if runs.contains(feed.as_str()) && !settling.contains(&feed) {
            held.push(feed);
        }
    }
    Ok(Settled { least, most, held })
}

/// What's told of each `held` feed: its note, and beside it a warning once
/// the note has stood longer than a settled hour may lag, counted in
/// whole hours. That's how long the note has stood, not how old the
/// feed's bills are, so a feed whose settled row was just deleted, or one
/// sending old bills for the first time, is given its while.
fn held_findings(
    picture: &Picture,
    history: &History,
    held: &[String],
) -> Result<Vec<Finding>, String> {
    let mut told = Vec::new();
    for feed in held {
        let note = held_finding(feed);
        let now = crate::time::hour_of(picture.now);
        let stood = crate::findings::started(history, &note.key)?
            .is_some_and(|started| now - crate::time::hour_of(started) > UNBILLED_STALE_HOURS);
        told.push(note);
        if stood {
            told.push(not_settling_finding(feed));
        }
    }
    Ok(told)
}

/// What's told while a feed that bills pi from the warehouse hasn't said
/// how far it has settled: a note, which never takes a notice, since it
/// may only lag. It's keyed apart from a wrong row's warning, so marking
/// one expected leaves the other to be told. cadence reads it as the day
/// unjudged rather than as no budget unbilled.
fn held_finding(feed: &str) -> Finding {
    let mut f = finding(
        format!("{UNBILLED_HELD}:{feed}"),
        "unbilled",
        Severity::Note,
        format!("the {feed} feed hasn't said how far pi's bill is settled"),
        format!(
            "The {feed} feed bills pi from the warehouse but hasn't said how far its \
             bill is settled, so no budget's bill is held against its logs until it does."
        ),
    );
    f.badge = "not settled".into();
    f.note = format!("{feed} not settled");
    f
}

/// What's told beside a feed's held note once it has stood for more than
/// a settled hour may lag: the feed has stopped short of settling rather
/// than not got there yet, and the note alone would hold the detector off
/// unseen for good.
fn not_settling_finding(feed: &str) -> Finding {
    let mut f = finding(
        format!("{UNBILLED_NOT_SETTLING}:{feed}"),
        "unbilled",
        Severity::Warning,
        format!(
            "the {feed} feed hasn't said how far pi's bill is settled in \
             {UNBILLED_STALE_HOURS} hours"
        ),
        format!(
            "For more than {UNBILLED_STALE_HOURS} hours the {feed} feed has billed pi from \
             the warehouse without saying how far its bill is settled, so no budget's bill \
             has been held against its logs since. This goes an hour after it says."
        ),
    );
    f.badge = "not settling".into();
    f.note = format!("{feed} not settling");
    f.surprise = 2.0;
    f
}

/// What's told when `feed` last settled `through` (exclusive), too long
/// ago to judge any budget's bill by, or past now: none is judged until
/// it settles again, or until a row past now is deleted or its hour comes.
fn unsettled_finding(picture: &Picture, feed: &str, through: i64) -> Finding {
    let since = picture
        .zone
        .clock(crate::time::hour_start(through), picture.now);
    let (title, detail) = if through > crate::time::hour_of(picture.now) {
        (
            format!("the {feed} feed's bill is settled past now"),
            // The settled hour never moves back, so the feed can't put it
            // right by settling again; the row in spend-watch's history
            // has to be. Left alone it is trusted once its hour comes,
            // settled or not, which is why the detail says so.
            format!(
                "The {feed} feed says it has settled pi's bill to {since}, which \
                 hasn't come yet, so no budget's bill is held against its logs before \
                 then, and once it comes the hour is trusted whether or not it settled. \
                 Settling never moves the row back: delete the feed's row from the \
                 settled table of spend-watch's history, and this goes at the next pass. \
                 If it comes back, the feed itself is sending an hour still to come."
            ),
        )
    } else {
        (
            format!("the {feed} feed's bill stopped settling"),
            format!(
                "The {feed} feed has settled pi's bill only to {since}, more than \
                 {UNBILLED_STALE_HOURS} hours ago, so no budget's bill is held against \
                 its logs until it settles again."
            ),
        )
    };
    let mut f = finding(
        format!("{UNBILLED_UNSETTLED}:{feed}"),
        "unbilled",
        Severity::Warning,
        title,
        detail,
    );
    f.badge = format!("to {since}");
    f.note = format!("settled to {since}");
    f.surprise = 2.0;
    f
}

/// The key of the finding that says the warehouse stopped settling: not
/// `unbilled:` and a budget's name, since it isn't one, and cadence reads
/// it as the count unjudged.
pub const UNBILLED_UNSETTLED: &str = "unbilled-unsettled";

/// The key of the note that says a feed billing pi hasn't said how far it
/// has settled, which cadence also reads as the count unjudged. Each of
/// the two is told behind the feed's name, so a mark on one feed's leaves
/// another's to be told.
pub const UNBILLED_HELD: &str = "unbilled-held";

/// The key of the warning told beside a held note that has stood too long,
/// apart from the note's so that marking the note expected, as a feed that
/// only lags may be, still leaves the warning to be told. cadence needn't
/// read it, since the note stands beside it.
pub const UNBILLED_NOT_SETTLING: &str = "unbilled-not-settling";

/// Each budget a session asked for in the last day and the harness
/// couldn't draw on, as the harness wrote it in the session's log: its
/// turns went on the default, so its team's allowance never saw them.
/// It's told as the sessions start rather than a day later, when the
/// warehouse bills the default for them.
pub fn unresolved(picture: &Picture, history: &History) -> Result<Vec<Finding>, String> {
    let mut statement = history
        .db
        .prepare("SELECT session, at, budget, missing FROM unresolved WHERE at >= ?1 ORDER BY at")
        .map_err(fail)?;
    let rows = statement
        .query_map([picture.now - DAY], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, f64>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })
        .map_err(fail)?;
    // Each budget's sessions, and the last it was missed: when, and why.
    let mut missed: BTreeMap<String, (HashSet<String>, f64, String)> = BTreeMap::new();
    for row in rows {
        let (session, at, budget, missing) = row.map_err(fail)?;
        let (sessions, last, why) = missed.entry(budget).or_default();
        sessions.insert(session);
        *last = at;
        *why = missing;
    }
    Ok(missed
        .into_iter()
        .map(|(budget, (sessions, last, why))| {
            unresolved_finding(picture, &budget, sessions.len(), last, &why)
        })
        .collect())
}

/// The key each budget's unresolved finding is told under, behind its
/// budget's name.
pub const UNRESOLVED: &str = "unresolved";

fn unresolved_finding(
    picture: &Picture,
    budget: &str,
    sessions: usize,
    last: f64,
    why: &str,
) -> Finding {
    let named = picture
        .budgets
        .declared(budget)
        .map_or(budget, |declared| declared.label.as_str());
    let named = if named.is_empty() { budget } else { named };
    let some = if sessions == 1 {
        "1 session".to_owned()
    } else {
        format!("{sessions} sessions")
    };
    let at = picture.zone.clock(last, picture.now);
    // A note rather than a warning: the person can't draw a budget for
    // another's session, cadence carries the count to whoever can, and a
    // warning would take one of the few notices a week a burn needs.
    let mut f = finding(
        format!("{UNRESOLVED}:{budget}"),
        UNRESOLVED,
        Severity::Note,
        format!("{some} couldn't draw on {named}"),
        format!(
            "In the last day, {some} asked for {named} and couldn't draw on it, \
             the last at {at}; the harness said: {why}. Their work goes on the \
             budget they had before, or the default's key, so {named}'s allowance \
             never sees it."
        ),
    );
    f.badge = format!("last {at}");
    f.note = "drew on another key".into();
    f.surprise = 2.0;
    f
}

/// What's told of `budget`, whose sessions logged `logged` dollars and
/// whose key was billed `paid` in the window ending at `through`.
fn unbilled_finding(
    budgets: &crate::budgets::Budgets,
    budget: &str,
    logged: f64,
    paid: f64,
    through: &str,
) -> Finding {
    let named = budgets
        .declared(budget)
        .map_or(budget, |declared| declared.label.as_str());
    let named = if named.is_empty() { budget } else { named };
    let mut f = finding(
        format!("unbilled:{budget}"),
        "unbilled",
        Severity::Warning,
        format!("{named} billed {} of {} logged", whole(paid), whole(logged)),
        format!(
            "In the {UNBILLED_HOURS} hours to {through}, pi's logs priced {named}'s \
             sessions at {} and its key was billed {}. The rest went on another key, \
             so {named}'s allowance never saw it: check which key its sessions' \
             requests carry.",
            dollars(logged),
            dollars(paid),
        ),
    );
    f.badge = format!("{} billed", percent(paid / logged));
    f.note = format!("logged {}", whole(logged));
    f.dollars = logged - paid;
    f.surprise = 2.0;
    f
}

#[cfg(test)]
#[path = "billing_tests.rs"]
mod tests;
