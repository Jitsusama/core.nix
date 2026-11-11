//! The feed contract: how a machine brings in what it's billed. A feed is
//! a program spend-watch starts, supervises and restarts. It writes JSON
//! lines on stdout, each a record with `"v": 1` and a `kind`; spend-watch
//! writes pace lines on its stdin, so it asks its sources often only while
//! there's something to find. contract/README.md describes every record
//! and contract/*.jsonl holds examples both sides test against.

use crate::history::History;
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const VERSION: u64 = 1;

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Record {
    Billed(Billed),
    Settled(Settled),
    Cap(Cap),
    Action(Action),
    Health(Health),
    Read(ReadCost),
    Said(Said),
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Billed {
    /// Which of the feed's sources said it: one figure per source per key,
    /// so a live meter and a warehouse never add up to twice the bill.
    pub source: String,
    /// The hour it was billed in, as the instant it starts.
    pub hour: String,
    /// The tool that spent it, or "all" when the source can't tell.
    pub tool: String,
    pub model: String,
    #[serde(default)]
    pub vendor: String,
    #[serde(default)]
    pub session: Option<String>,
    /// The kind of agent that spent it: interactive, extension, one-shot.
    #[serde(default)]
    pub agent: Option<String>,
    /// The credential it was billed to, for spotting one never seen.
    #[serde(default)]
    pub token: Option<String>,
    /// The budget it was drawn from; the feed's own when none is given.
    #[serde(default)]
    pub budget: Option<String>,
    #[serde(default)]
    pub tokens: BilledTokens,
    pub dollars: f64,
    /// The hour is still open and this is what it stands at so far: the
    /// next record for the same key replaces it.
    #[serde(default)]
    pub provisional: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BilledTokens {
    #[serde(default)]
    pub input: f64,
    #[serde(default)]
    pub cached: f64,
    #[serde(default)]
    pub cache_write: f64,
    #[serde(default)]
    pub output: f64,
}

/// A source's bill is complete before this instant, an hour boundary.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Settled {
    pub source: String,
    pub through: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Cap {
    pub name: String,
    pub label: String,
    /// The budget it limits; the feed's own when none is given.
    #[serde(default)]
    pub budget: Option<String>,
    /// Dollars, or a share of a plan's window.
    #[serde(default)]
    pub unit: crate::caps::Unit,
    /// Dollars allowed in the window; null means no limit.
    #[serde(default)]
    pub allowance: Option<f64>,
    pub window_hours: u32,
    /// What the provider counts against it in dollars, as of `at`.
    #[serde(default)]
    pub spent: Option<f64>,
    /// The share of a plan's window used, from nothing to all.
    #[serde(default)]
    pub used: Option<f64>,
    /// The fraction of the allowance it warns at.
    pub warn: f64,
    /// The allowance that applies when an override ends.
    #[serde(default)]
    pub fallback: Option<f64>,
    pub state: CapState,
    /// The last day an override holds, when the feed knows it.
    #[serde(default)]
    pub ends: Option<String>,
    /// Whether the feed could know the end: false means it's unknown, not
    /// that there's none.
    #[serde(default)]
    pub ends_known: bool,
    /// When the spend next goes back to nothing, for a cap counted over
    /// periods of `window_hours` (a calendar day) rather than a rolling
    /// window. A period's spend doesn't roll out an hour at a time; it all
    /// goes at once, then.
    #[serde(default)]
    pub resets: Option<String>,
    pub at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CapState {
    Ok,
    Notified,
    Refused,
}

impl CapState {
    pub fn name(self) -> &'static str {
        match self {
            CapState::Ok => "ok",
            CapState::Notified => "notified",
            CapState::Refused => "refused",
        }
    }
}

/// Something the person can do: a command to run, or a link to open.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Action {
    pub id: String,
    pub name: String,
    pub run: Vec<String>,
    /// What it answers: "cap", "sign-in", "premium".
    #[serde(default)]
    pub fits: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Health {
    pub source: String,
    pub state: HealthState,
    #[serde(default)]
    pub message: String,
    /// The action that signs this source in, by id.
    #[serde(default)]
    pub sign_in: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum HealthState {
    Fine,
    Stalled,
    SignedOut,
    /// An optional source not signed in for: nothing is wrong.
    Optional,
    Error,
}

impl HealthState {
    pub fn name(self) -> &'static str {
        match self {
            HealthState::Fine => "fine",
            HealthState::Stalled => "stalled",
            HealthState::SignedOut => "signed-out",
            HealthState::Optional => "optional",
            HealthState::Error => "error",
        }
    }
}

/// What one read of a source cost.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReadCost {
    pub source: String,
    #[serde(default)]
    pub bytes: f64,
    #[serde(default)]
    pub millis: f64,
    #[serde(default)]
    pub job: String,
}

/// Something the provider itself said to the person.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Said {
    pub severity: String,
    pub text: String,
}

/// What spend-watch writes on a feed's stdin.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Pace {
    Active,
    Idle,
    Asleep,
}

pub fn pace_line(pace: Pace) -> String {
    serde_json::json!({ "v": VERSION, "pace": pace }).to_string()
}

/// What the watch keeps of a feed's own, said as it starts: for each
/// source, when the newest hour of its bill starts, and for each cap, when
/// its newest reading was taken, since one a feed reads more often mustn't
/// hide a gap in another's. A feed that keeps how far it has read starts
/// again no later than this, so a history built afresh, or one set aside,
/// is told the bill again rather than only what comes next.
pub fn kept_line(history: &History, feed: &str) -> Result<String, String> {
    let fail = |error: rusqlite::Error| error.to_string();
    // Each name's newest instant, as `instant` makes it from the column.
    let newest = |sql: &str, instant: fn(f64) -> f64| {
        let mut statement = history.db.prepare(sql).map_err(fail)?;
        let rows = statement
            .query_map([feed], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?))
            })
            .map_err(fail)?;
        let mut kept = serde_json::Map::new();
        for row in rows {
            let (name, at) = row.map_err(fail)?;
            kept.insert(name, crate::time::rfc3339(instant(at)).into());
        }
        Ok::<_, String>(kept)
    };
    let billed = newest(
        "SELECT source, MAX(hour) FROM billed WHERE feed = ?1 GROUP BY source",
        // The column holds whole hours, so this is the hour's start.
        |hour| hour * crate::time::HOUR,
    )?;
    let caps = newest(
        "SELECT name, MAX(at) FROM caps WHERE feed = ?1 GROUP BY name",
        |at| at,
    )?;
    Ok(serde_json::json!({ "v": VERSION, "kept": { "billed": billed, "caps": caps } }).to_string())
}

pub fn refresh_line() -> String {
    serde_json::json!({ "v": VERSION, "refresh": true }).to_string()
}

/// One line as the contract reads it, or why it doesn't.
pub fn parse(line: &str) -> Result<Record, String> {
    let value: Value = serde_json::from_str(line).map_err(|error| format!("not JSON: {error}"))?;
    match value.get("v").and_then(Value::as_u64) {
        Some(VERSION) => {}
        Some(other) => return Err(format!("version {other}, not {VERSION}")),
        None => return Err("no version".into()),
    }
    let mut value = value;
    if let Some(object) = value.as_object_mut() {
        object.remove("v");
    }
    let record: Record =
        serde_json::from_value(value).map_err(|error| format!("not a record: {error}"))?;
    check(&record)?;
    Ok(record)
}

fn instant(text: &str, what: &str) -> Result<f64, String> {
    crate::time::parse(text).ok_or_else(|| format!("{what} {text} isn't an instant"))
}

fn check(record: &Record) -> Result<(), String> {
    match record {
        Record::Billed(billed) => {
            let at = instant(&billed.hour, "hour")?;
            if at % crate::time::HOUR != 0.0 {
                return Err(format!("hour {} doesn't start an hour", billed.hour));
            }
            if !billed.dollars.is_finite() || billed.dollars < 0.0 {
                return Err("dollars are a figure of none or more".into());
            }
        }
        Record::Settled(settled) => {
            if instant(&settled.through, "through")? % crate::time::HOUR != 0.0 {
                return Err(format!(
                    "through {} isn't an hour boundary",
                    settled.through
                ));
            }
        }
        Record::Cap(cap) => {
            let at = instant(&cap.at, "at")?;
            if let Some(resets) = &cap.resets
                && instant(resets, "resets")? <= at
            {
                return Err(format!("resets {resets} isn't after the cap was read"));
            }
            if let Some(ends) = &cap.ends
                && crate::time::parse_day(ends).is_none()
            {
                return Err(format!("ends {ends} isn't a day"));
            }
            if cap.window_hours == 0 || !(0.0..=1.0).contains(&cap.warn) {
                return Err("a cap's window is hours and its warning a fraction".into());
            }
            check_unit(cap)?;
            check_label(&cap.label)?;
            // Each figure is a share of it; no allowance is null.
            if cap.allowance.is_some_and(|allowance| allowance <= 0.0)
                || cap.fallback.is_some_and(|fallback| fallback <= 0.0)
            {
                return Err("a cap's allowance and fallback are more than nothing".into());
            }
        }
        Record::Action(action) => {
            if action.run.is_empty() {
                return Err(format!("action {} runs nothing", action.id));
            }
        }
        Record::Health(_) | Record::Read(_) | Record::Said(_) => {}
    }
    Ok(())
}

/// Words a window's name never says: what the glossary's allowance
/// replaces, with cap and quota, which say the same thing, and the
/// glossary's own words for the kinds, since a sentence names a window
/// with its kind after it ("near the Week allowance").
const UNSAID: [&str; 6] = ["cap", "limit", "quota", "allowance", "budget", "window"];

/// Whether a word is one of those, plural or possessive alike, which a
/// budget's own words never say either (VKCF3Z, 2026-11-07).
pub fn unsaid(word: &str) -> bool {
    let bare = word
        .trim_end_matches("'s")
        .trim_end_matches('s')
        .to_lowercase();
    UNSAID.contains(&bare.as_str())
}

/// A window's name, as the contract's README has it: in Title Case, each
/// word and each part of a hyphenated one opening with a capital, never
/// with a figure, and in the glossary's words (VKCF3Z, 2026-11-07).
fn check_label(label: &str) -> Result<(), String> {
    let words: Vec<&str> = label.split_whitespace().collect();
    if words.is_empty() {
        return Err("a window is named".into());
    }
    let capitalized = |part: &str| part.chars().next().is_some_and(char::is_uppercase);
    if !words
        .iter()
        .flat_map(|word| word.split('-'))
        .all(capitalized)
    {
        return Err(format!("window {label:?} isn't named in Title Case"));
    }
    if let Some(said) = words.iter().find(|word| unsaid(word)) {
        return Err(format!(
            "window {label:?} says {said:?}: a window is named for its length"
        ));
    }
    Ok(())
}

/// A cap in dollars says what's spent; a share says what's used of the
/// window and when it resets, and never an amount, since a share is never
/// turned into dollars.
fn check_unit(cap: &Cap) -> Result<(), String> {
    match cap.unit {
        crate::caps::Unit::Dollars => {
            if cap.spent.is_none() || cap.used.is_some() {
                return Err("a cap in dollars says what's spent, not what's used".into());
            }
        }
        crate::caps::Unit::Share => {
            let used = cap
                .used
                .ok_or("a share cap says what's used of its window")?;
            if !(0.0..=1.0).contains(&used) {
                return Err("a share cap's use is a fraction from nothing to all".into());
            }
            if cap.resets.is_none() {
                return Err("a share cap says when its window resets".into());
            }
            if cap.allowance.is_some() || cap.spent.is_some() || cap.fallback.is_some() {
                return Err("a share cap has no allowance and spends no dollars".into());
            }
        }
    }
    Ok(())
}

/// Keeps a record in the history, as of `now`. Caps compare with the last
/// one the feed gave, and a change is an event.
pub fn store(history: &History, feed: &str, record: &Record, now: f64) -> Result<(), String> {
    let db = &history.db;
    let fail = |error: rusqlite::Error| error.to_string();
    match record {
        Record::Billed(billed) => {
            let hour = crate::time::hour_of(instant(&billed.hour, "hour")?);
            db.execute(
                "INSERT OR REPLACE INTO billed VALUES
                 (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
                params![
                    feed,
                    billed.source,
                    hour,
                    billed.tool,
                    billed.model,
                    billed.vendor,
                    billed.session.clone().unwrap_or_default(),
                    billed.agent.clone().unwrap_or_default(),
                    billed.token.clone().unwrap_or_default(),
                    billed.tokens.input,
                    billed.tokens.cached,
                    billed.tokens.cache_write,
                    billed.tokens.output,
                    billed.dollars,
                    billed.provisional,
                    now,
                    billed.budget.clone().unwrap_or_default(),
                ],
            )
            .map_err(fail)?;
        }
        Record::Settled(settled) => {
            let through = crate::time::hour_of(instant(&settled.through, "through")?);
            db.execute(
                "INSERT INTO settled VALUES (?1, ?2, ?3) ON CONFLICT (feed, source)
                 DO UPDATE SET through = MAX(through, excluded.through)",
                params![feed, settled.source, through],
            )
            .map_err(fail)?;
        }
        Record::Cap(cap) => store_cap(history, feed, cap)?,
        Record::Action(action) => {
            db.execute(
                "INSERT OR REPLACE INTO actions VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    feed,
                    action.id,
                    action.name,
                    serde_json::to_string(&action.run).unwrap_or_default(),
                    action.fits.join(","),
                ],
            )
            .map_err(fail)?;
        }
        Record::Health(health) => {
            // Since when it's stood so: kept while the state holds, so a
            // source down for a moment can be told from one that's down.
            db.execute(
                "INSERT INTO health VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)
                 ON CONFLICT (feed, source) DO UPDATE SET
                     since = CASE WHEN health.state = excluded.state
                         THEN coalesce(health.since, health.at) ELSE excluded.at END,
                     state = excluded.state, message = excluded.message,
                     sign_in = excluded.sign_in, at = excluded.at",
                params![
                    feed,
                    health.source,
                    health.state.name(),
                    health.message,
                    health.sign_in,
                    now
                ],
            )
            .map_err(fail)?;
        }
        Record::Read(read) => {
            db.execute(
                "INSERT INTO reads VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![feed, read.source, now, read.bytes, read.millis, read.job],
            )
            .map_err(fail)?;
        }
        Record::Said(said) => {
            db.execute(
                "INSERT OR REPLACE INTO said VALUES (?1, ?2, ?3, ?4)",
                params![feed, said.severity, said.text, now],
            )
            .map_err(fail)?;
        }
    }
    Ok(())
}

fn store_cap(history: &History, feed: &str, cap: &Cap) -> Result<(), String> {
    let db = &history.db;
    let fail = |error: rusqlite::Error| error.to_string();
    let at = instant(&cap.at, "at")?;
    let budget = cap.budget.clone().unwrap_or_default();
    let before: Option<(Option<f64>, String)> = db
        .query_row(
            "SELECT allowance, state FROM caps WHERE feed = ?1 AND budget = ?2 AND name = ?3
             AND at < ?4 ORDER BY at DESC LIMIT 1",
            params![feed, budget, cap.name, at],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(fail)?;
    db.execute(
        "INSERT OR REPLACE INTO caps VALUES
         (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
        params![
            feed,
            cap.name,
            at,
            cap.label,
            cap.allowance,
            cap.window_hours,
            cap.spent.unwrap_or(0.0),
            cap.warn,
            cap.fallback,
            cap.state.name(),
            cap.ends,
            cap.ends_known,
            cap.resets.as_deref().and_then(crate::time::parse),
            budget,
            cap.unit.name(),
            cap.used,
        ],
    )
    .map_err(fail)?;
    let Some((allowance, state)) = before else {
        return Ok(());
    };
    let event = |kind: &str| {
        db.execute(
            "INSERT OR REPLACE INTO cap_events VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                feed,
                cap.name,
                at,
                kind,
                allowance,
                cap.allowance,
                cap.spent.unwrap_or(0.0),
                budget
            ],
        )
        .map(|_| ())
        .map_err(fail)
    };
    // No limit is more than any limit.
    let more = |one: Option<f64>, other: Option<f64>| match (one, other) {
        (None, Some(_)) => true,
        (Some(one), Some(other)) => one > other,
        _ => false,
    };
    if more(cap.allowance, allowance) {
        event("raised")?;
    } else if more(allowance, cap.allowance) {
        // Down to the fallback is an override that lapsed.
        let lapsed = cap.fallback.is_some() && cap.allowance == cap.fallback;
        event(if lapsed { "lapsed" } else { "lowered" })?;
    }
    if cap.state.name() != state && cap.state != CapState::Ok {
        event(cap.state.name())?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "feed_tests.rs"]
mod tests;
