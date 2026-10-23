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
    /// Dollars allowed in the window; null means no limit.
    pub allowance: Option<f64>,
    pub window_hours: u32,
    /// What the provider counts against it, as of `at`.
    pub spent: f64,
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
                 (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
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
    let before: Option<(Option<f64>, String)> = db
        .query_row(
            "SELECT allowance, state FROM caps WHERE feed = ?1 AND name = ?2 AND at < ?3
             ORDER BY at DESC LIMIT 1",
            params![feed, cap.name, at],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(fail)?;
    db.execute(
        "INSERT OR REPLACE INTO caps VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)",
        params![
            feed,
            cap.name,
            at,
            cap.label,
            cap.allowance,
            cap.window_hours,
            cap.spent,
            cap.warn,
            cap.fallback,
            cap.state.name(),
            cap.ends,
            cap.ends_known,
            cap.resets.as_deref().and_then(crate::time::parse),
        ],
    )
    .map_err(fail)?;
    let Some((allowance, state)) = before else {
        return Ok(());
    };
    let event = |kind: &str| {
        db.execute(
            "INSERT OR REPLACE INTO cap_events VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                feed,
                cap.name,
                at,
                kind,
                allowance,
                cap.allowance,
                cap.spent
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
mod tests {
    use super::*;

    const FIXTURES: [(&str, &str); 3] = [
        ("everything", include_str!("../contract/everything.jsonl")),
        ("meter-only", include_str!("../contract/meter-only.jsonl")),
        ("signed-out", include_str!("../contract/signed-out.jsonl")),
    ];

    #[test]
    fn every_fixture_keeps_the_contract() {
        for (name, text) in FIXTURES {
            for (number, line) in text
                .lines()
                .enumerate()
                .filter(|(_, line)| !line.is_empty())
            {
                parse(line).unwrap_or_else(|error| panic!("{name}:{}: {error}", number + 1));
            }
        }
    }

    #[test]
    fn a_line_without_a_version_is_refused() {
        assert!(parse(r#"{"kind":"said","severity":"info","text":"x"}"#).is_err());
        assert!(parse(r#"{"v":2,"kind":"said","severity":"info","text":"x"}"#).is_err());
    }

    #[test]
    fn an_hour_must_start_an_hour() {
        let line = concat!(
            r#"{"v":1,"kind":"billed","source":"meter","hour":"2026-10-14T05:30:00Z","#,
            r#""tool":"all","model":"m","dollars":1}"#
        );
        assert!(parse(line).unwrap_err().contains("doesn't start an hour"));
    }

    #[test]
    fn a_cap_of_nothing_is_refused() {
        let line = serde_json::json!({
            "v": 1, "kind": "cap", "name": "weekly", "label": "weekly cap",
            "allowance": 0, "windowHours": 168, "spent": 1, "warn": 0.9,
            "state": "refused", "at": "2026-10-14T05:00:00Z",
        });
        assert!(
            parse(&line.to_string())
                .unwrap_err()
                .contains("more than nothing")
        );
    }

    // A day's cap that resets at the next midnight is kept with its reset;
    // one claiming to reset before it was read can't be judged.
    #[test]
    fn a_period_s_reset_is_kept_after_it_was_read() {
        let day = |resets: &str| {
            serde_json::json!({
                "v": 1, "kind": "cap", "name": "premium", "label": "premium day",
                "allowance": 1000, "windowHours": 24, "spent": 421.43, "warn": 0.5,
                "state": "ok", "resets": resets, "at": "2026-10-14T05:40:00Z",
            })
            .to_string()
        };
        assert!(
            parse(&day("2026-10-14T05:00:00Z"))
                .unwrap_err()
                .contains("isn't after")
        );
        let history = History::memory();
        store(
            &history,
            "proxy",
            &parse(&day("2026-10-15T04:00:00Z")).unwrap(),
            0.0,
        )
        .unwrap();
        let kept = crate::caps::latest(&history).unwrap();
        assert_eq!(kept[0].resets, crate::time::parse("2026-10-15T04:00:00Z"));
    }

    #[test]
    fn an_unknown_field_is_refused_rather_than_ignored() {
        let line = r#"{"v":1,"kind":"said","severity":"info","text":"x","colour":"red"}"#;
        assert!(parse(line).is_err());
    }

    fn cap(allowance: Option<f64>, at: &str, state: &str) -> Record {
        let line = serde_json::json!({
            "v": 1, "kind": "cap", "name": "weekly", "label": "weekly cap",
            "allowance": allowance, "windowHours": 168, "spent": 4995, "warn": 0.9,
            "fallback": 4000, "state": state, "at": at,
        });
        parse(&line.to_string()).unwrap()
    }

    fn events(history: &History) -> Vec<String> {
        let mut statement = history
            .db
            .prepare("SELECT kind FROM cap_events ORDER BY at, rowid")
            .unwrap();
        statement
            .query_map([], |row| row.get(0))
            .unwrap()
            .map(Result::unwrap)
            .collect()
    }

    #[test]
    fn a_lapse_a_refusal_and_a_raise_are_events() {
        let history = History::memory();
        store(
            &history,
            "proxy",
            &cap(Some(8000.0), "2026-09-08T23:04:00Z", "ok"),
            0.0,
        )
        .unwrap();
        store(
            &history,
            "proxy",
            &cap(Some(4000.0), "2026-09-09T00:04:00Z", "refused"),
            0.0,
        )
        .unwrap();
        store(
            &history,
            "proxy",
            &cap(Some(8000.0), "2026-09-09T00:28:00Z", "ok"),
            0.0,
        )
        .unwrap();
        store(
            &history,
            "proxy",
            &cap(None, "2026-09-10T00:28:00Z", "ok"),
            0.0,
        )
        .unwrap();
        store(
            &history,
            "proxy",
            &cap(Some(10000.0), "2026-09-11T00:28:00Z", "ok"),
            0.0,
        )
        .unwrap();
        assert_eq!(
            events(&history),
            ["lapsed", "refused", "raised", "raised", "lowered"]
        );
    }

    #[test]
    #[expect(
        clippy::float_cmp,
        reason = "the sum of the one row left is that row's figure, untouched"
    )]
    fn a_provisional_hour_is_replaced_as_it_grows() {
        let history = History::memory();
        let billed = |dollars: f64| {
            let line = serde_json::json!({
                "v": 1, "kind": "billed", "source": "meter", "hour": "2026-10-14T05:00:00Z",
                "tool": "all", "model": "m", "dollars": dollars, "provisional": true,
            });
            parse(&line.to_string()).unwrap()
        };
        store(&history, "proxy", &billed(29.19), 1.0).unwrap();
        store(&history, "proxy", &billed(29.83), 2.0).unwrap();
        let total: f64 = history
            .db
            .query_row("SELECT SUM(dollars) FROM billed", [], |row| row.get(0))
            .unwrap();
        assert_eq!(total, 29.83);
    }

    // A history built afresh keeps nothing of a feed's, and says so, so
    // the feed tells its bill again from the start; one that keeps some
    // says how far, source by source.
    #[test]
    fn a_feed_is_told_what_the_watch_keeps_of_its_own() {
        let history = History::memory();
        let said = |feed: &str| -> Value {
            serde_json::from_str(&kept_line(&history, feed).unwrap()).unwrap()
        };
        assert_eq!(
            said("proxy"),
            serde_json::json!({ "v": 1, "kept": { "billed": {}, "caps": {} } })
        );
        for (source, hour) in [
            ("warehouse", "2026-10-14T12:00:00Z"),
            ("warehouse", "2026-10-14T13:00:00Z"),
            ("meter", "2026-10-14T16:00:00Z"),
        ] {
            let line = serde_json::json!({
                "v": 1, "kind": "billed", "source": source, "hour": hour,
                "tool": "all", "model": "m", "dollars": 1.0,
            });
            store(&history, "proxy", &parse(&line.to_string()).unwrap(), 1.0).unwrap();
        }
        // Each cap by its own newest, since one a feed reads more often
        // mustn't hide a gap in another's.
        for (name, at) in [
            ("weekly", "2026-10-14T15:04:12Z"),
            ("weekly", "2026-10-14T16:04:29Z"),
            ("premium", "2026-10-14T16:50:00Z"),
        ] {
            let cap = serde_json::json!({
                "v": 1, "kind": "cap", "name": name, "label": name,
                "allowance": 15000, "windowHours": 168, "spent": 4262, "warn": 0.9,
                "state": "ok", "at": at,
            });
            store(&history, "proxy", &parse(&cap.to_string()).unwrap(), 1.0).unwrap();
        }
        assert_eq!(
            said("proxy"),
            serde_json::json!({ "v": 1, "kept": {
                "billed": {
                    "meter": "2026-10-14T16:00:00Z",
                    "warehouse": "2026-10-14T13:00:00Z",
                },
                "caps": {
                    "premium": "2026-10-14T16:50:00Z",
                    "weekly": "2026-10-14T16:04:29Z",
                },
            } })
        );
        assert_eq!(said("other")["kept"]["billed"], serde_json::json!({}));
    }
}
