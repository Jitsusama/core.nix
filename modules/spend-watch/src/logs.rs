//! pi's session logs, read as they grow. Each log is JSON lines: a header
//! naming the session and its directory, then entries. What counts here:
//!
//! - an assistant message with usage is a turn, priced at pi's list rates;
//! - one that ended in a provider's refusal for being over a cap is a
//!   refusal, dated, which is how a cut-off shows with no feed at all;
//! - a compaction entry with usage is a turn of its own kind;
//! - a custom entry with usage is an extension's side call, named by its
//!   type;
//! - a quest-workflow entry says which quest the turns after it serve;
//! - a model change matters to why cache was written.
//!
//! A log is read from where the last read stopped, and only whole lines: a
//! line still being written waits for its end. What a read must remember
//! to carry on (the session, its quest, its last turn) is `Carry`, kept
//! beside the offset so a restart picks up exactly where it was.
//!
//! Each agent turn that wrote cache is split into growth (context new to
//! the session) and rebuilding (context the session had and lost), and the
//! rebuilding is given its cause: a pause longer than the cache lives, a
//! compaction, a fresh context, a model change, or none found.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Below this many tokens a rebuild is noise: providers cache in blocks.
const REBUILD_FLOOR: f64 = 1024.0;
/// How long a provider keeps cache: five minutes, or an hour for writes
/// asked to last one.
const CACHE_LIFE: f64 = 300.0;
const CACHE_LIFE_LONG: f64 = 3600.0;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Carry {
    pub session: Option<String>,
    pub cwd: Option<String>,
    pub quest: Option<String>,
    pub model: Option<String>,
    /// The model the last agent turn ran on, to tell a change of model.
    pub last_model: Option<String>,
    pub last_at: Option<f64>,
    /// The last agent turn's whole prompt: uncached, read and written.
    pub last_context: Option<f64>,
    pub last_long: bool,
    pub compacted: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Kind {
    Agent,
    Compaction,
    Side(String),
}

impl Kind {
    pub fn name(&self) -> String {
        match self {
            Kind::Agent => "agent".into(),
            Kind::Compaction => "compaction".into(),
            Kind::Side(name) => format!("side:{name}"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Tokens {
    pub input: f64,
    pub output: f64,
    pub cache_read: f64,
    pub cache_write: f64,
    pub cache_write_long: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Cost {
    pub input: f64,
    pub output: f64,
    pub cache_read: f64,
    pub cache_write: f64,
    pub total: f64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Cause {
    Gap,
    Compaction,
    Fresh,
    Model,
    Unknown,
}

impl Cause {
    pub fn name(&self) -> &'static str {
        match self {
            Cause::Gap => "gap",
            Cause::Compaction => "compaction",
            Cause::Fresh => "fresh",
            Cause::Model => "model",
            Cause::Unknown => "none found",
        }
    }

    pub fn words(name: &str) -> &'static str {
        match name {
            "gap" => "rebuilt after pauses longer than the cache lives",
            "compaction" => "rebuilt after compactions",
            "fresh" => "written fresh for new sessions and subagents",
            "model" => "rebuilt after changing model",
            _ => "rebuilt for no reason found",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Turn {
    pub session: String,
    pub at: f64,
    pub model: String,
    pub provider: String,
    pub kind: Kind,
    pub tokens: Tokens,
    pub cost: Cost,
    pub quest: Option<String>,
    /// Seconds since the session's last agent turn.
    pub gap: Option<f64>,
    pub rebuilt: f64,
    pub rebuilt_cost: f64,
    pub cause: Option<Cause>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    Session {
        id: String,
        cwd: String,
        at: f64,
    },
    Turn(Turn),
    Refusal {
        session: String,
        at: f64,
        model: String,
        said: String,
    },
}

/// Reads what's whole in `bytes`, which start where the last read
/// stopped, and says how many bytes it used: up to the last line break.
pub fn read(bytes: &[u8], carry: &mut Carry, refusals: &[String]) -> (Vec<Event>, usize) {
    let used = bytes
        .iter()
        .rposition(|&byte| byte == b'\n')
        .map_or(0, |at| at + 1);
    let mut events = Vec::new();
    for line in bytes[..used].split(|&byte| byte == b'\n') {
        if line.is_empty() {
            continue;
        }
        // A line that isn't JSON is skipped rather than stopping the log.
        let Ok(entry) = serde_json::from_slice::<Value>(line) else {
            continue;
        };
        entry_events(&entry, carry, refusals, &mut events);
    }
    (events, used)
}

fn number(value: &Value, key: &str) -> f64 {
    value.get(key).and_then(Value::as_f64).unwrap_or(0.0)
}

fn usage(value: &Value) -> Option<(Tokens, Cost)> {
    let usage = value.get("usage").filter(|usage| usage.is_object())?;
    let cost = usage.get("cost").cloned().unwrap_or(Value::Null);
    let tokens = Tokens {
        input: number(usage, "input"),
        output: number(usage, "output"),
        cache_read: number(usage, "cacheRead"),
        cache_write: number(usage, "cacheWrite"),
        cache_write_long: number(usage, "cacheWrite1h"),
    };
    let cost = Cost {
        input: number(&cost, "input"),
        output: number(&cost, "output"),
        cache_read: number(&cost, "cacheRead"),
        cache_write: number(&cost, "cacheWrite"),
        total: number(&cost, "total"),
    };
    Some((tokens, cost))
}

fn stamp(entry: &Value) -> Option<f64> {
    entry
        .get("timestamp")
        .and_then(Value::as_str)
        .and_then(crate::time::parse)
}

fn entry_events(entry: &Value, carry: &mut Carry, refusals: &[String], events: &mut Vec<Event>) {
    let kind = entry.get("type").and_then(Value::as_str).unwrap_or("");
    let at = stamp(entry);
    match kind {
        "session" => {
            let id = entry
                .get("id")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_owned();
            let cwd = entry
                .get("cwd")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_owned();
            carry.session = Some(id.clone());
            carry.cwd = Some(cwd.clone());
            events.push(Event::Session {
                id,
                cwd,
                at: at.unwrap_or(0.0),
            });
        }
        "model_change" => {
            if let Some(model) = entry.get("modelId").and_then(Value::as_str) {
                carry.model = Some(model.to_owned());
            }
        }
        "compaction" => {
            carry.compacted = true;
            if let (Some((tokens, cost)), Some(at)) = (usage(entry), at) {
                push_turn(
                    events,
                    carry,
                    at,
                    Kind::Compaction,
                    tokens,
                    cost,
                    None,
                    None,
                );
            }
        }
        "custom" => {
            let custom = entry
                .get("customType")
                .and_then(Value::as_str)
                .unwrap_or("");
            let data = entry.get("data").cloned().unwrap_or(Value::Null);
            if custom == "quest-workflow" && data.get("questId").is_some() {
                carry.quest = data
                    .get("questId")
                    .and_then(Value::as_str)
                    .map(str::to_owned);
            }
            if let (Some((tokens, cost)), Some(at)) = (usage(&data), at) {
                let model = data.get("model").and_then(Value::as_str).map(str::to_owned);
                if tokens.input + tokens.output + tokens.cache_read + tokens.cache_write > 0.0 {
                    push_turn(
                        events,
                        carry,
                        at,
                        Kind::Side(custom.into()),
                        tokens,
                        cost,
                        model,
                        None,
                    );
                }
            }
        }
        "message" => {
            let Some(message) = entry.get("message") else {
                return;
            };
            if message.get("role").and_then(Value::as_str) != Some("assistant") {
                return;
            }
            let Some(at) = at else { return };
            let model = message
                .get("model")
                .and_then(Value::as_str)
                .map(str::to_owned);
            let provider = message
                .get("provider")
                .and_then(Value::as_str)
                .map(str::to_owned);
            if message.get("stopReason").and_then(Value::as_str) == Some("error") {
                let said = message
                    .get("errorMessage")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                let lower = said.to_lowercase();
                if refusals
                    .iter()
                    .any(|pattern| lower.contains(&pattern.to_lowercase()))
                {
                    events.push(Event::Refusal {
                        session: carry.session.clone().unwrap_or_default(),
                        at,
                        model: model.clone().or(carry.model.clone()).unwrap_or_default(),
                        said: said.chars().take(300).collect(),
                    });
                }
            }
            if let Some((tokens, cost)) = usage(message)
                && tokens.input + tokens.output + tokens.cache_read + tokens.cache_write > 0.0
            {
                push_turn(
                    events,
                    carry,
                    at,
                    Kind::Agent,
                    tokens,
                    cost,
                    model,
                    provider,
                );
            }
        }
        _ => {}
    }
}

#[allow(clippy::too_many_arguments)]
fn push_turn(
    events: &mut Vec<Event>,
    carry: &mut Carry,
    at: f64,
    kind: Kind,
    tokens: Tokens,
    cost: Cost,
    model: Option<String>,
    provider: Option<String>,
) {
    let model = model
        .or(carry.model.clone())
        .unwrap_or_else(|| "unknown".into());
    let mut turn = Turn {
        session: carry.session.clone().unwrap_or_default(),
        at,
        model: model.clone(),
        provider: provider.unwrap_or_default(),
        kind: kind.clone(),
        tokens,
        cost,
        quest: carry.quest.clone(),
        gap: None,
        rebuilt: 0.0,
        rebuilt_cost: 0.0,
        cause: None,
    };
    if kind == Kind::Agent {
        turn.gap = carry.last_at.map(|last| (at - last).max(0.0));
        if tokens.cache_write > 0.0 {
            let (rebuilt, cause) = rebuilding(carry, &turn);
            turn.rebuilt = rebuilt;
            turn.rebuilt_cost = cost.cache_write * rebuilt / tokens.cache_write;
            turn.cause = cause;
        }
        carry.last_at = Some(at);
        carry.last_context = Some(tokens.input + tokens.cache_read + tokens.cache_write);
        carry.last_long = tokens.cache_write_long > 0.0;
        carry.last_model = Some(model);
        carry.compacted = false;
    }
    events.push(Event::Turn(turn));
}

/// How much of a turn's cache write rebuilt context the session already
/// had, and why it had to.
fn rebuilding(carry: &Carry, turn: &Turn) -> (f64, Option<Cause>) {
    let written = turn.tokens.cache_write;
    let Some(before) = carry.last_context else {
        // Nothing before it in this session: all of it is new context.
        return (written, Some(Cause::Fresh));
    };
    let lost = (before - turn.tokens.cache_read).max(0.0);
    let rebuilt = written.min(lost);
    if rebuilt < REBUILD_FLOOR {
        return (0.0, None);
    }
    let life = if carry.last_long {
        CACHE_LIFE_LONG
    } else {
        CACHE_LIFE
    };
    let cause = if carry
        .last_model
        .as_deref()
        .is_some_and(|model| model != turn.model)
    {
        Cause::Model
    } else if carry.compacted {
        Cause::Compaction
    } else if turn.gap.is_some_and(|gap| gap > life) {
        Cause::Gap
    } else {
        Cause::Unknown
    };
    (rebuilt, Some(cause))
}

/// The repository a working directory belongs to: a checkout under
/// `src/<host>/<owner>/<repo>`, a zone of a tree cut under
/// `<name>/trees/<tree>/src/<zone>`, or the directory's own name.
pub fn repo_of(cwd: &str) -> String {
    if let Some(at) = cwd.find("/trees/") {
        let before = &cwd[..at];
        let name = before.rsplit('/').next().unwrap_or("");
        if let Some((_, zone)) = cwd[at + "/trees/".len()..].split_once("/src/")
            && !zone.is_empty()
        {
            return format!("{name}/{zone}");
        }
    }
    if let Some(at) = cwd.find("/src/") {
        let parts: Vec<&str> = cwd[at + 5..]
            .split('/')
            .filter(|part| !part.is_empty())
            .collect();
        if parts.len() >= 3 {
            return parts[..3].join("/");
        }
    }
    cwd.trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or("")
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn refusals() -> Vec<String> {
        vec!["over the allowed cap".into()]
    }

    fn lines(entries: &[&str]) -> Vec<u8> {
        let mut bytes = Vec::new();
        for entry in entries {
            bytes.extend_from_slice(entry.as_bytes());
            bytes.push(b'\n');
        }
        bytes
    }

    fn turn(at: &str, read: u32, write: u32, long: u32) -> String {
        let cost = serde_json::json!({
            "input": 0.01, "output": 0.1, "cacheRead": 0.2, "cacheWrite": 1.0, "total": 1.31,
        });
        let usage = serde_json::json!({
            "input": 10, "output": 100, "cacheRead": read, "cacheWrite": write,
            "cacheWrite1h": long, "cost": cost,
        });
        let message = serde_json::json!({
            "role": "assistant", "provider": "anthropic", "model": "opus", "usage": usage,
        });
        serde_json::json!({ "type": "message", "timestamp": at, "message": message }).to_string()
    }

    const HEADER: &str = concat!(
        r#"{"type":"session","id":"s1","timestamp":"2026-10-14T10:00:00Z","#,
        r#""cwd":"/home/someone/src/github.com/me/thing"}"#
    );

    fn turns(events: &[Event]) -> Vec<&Turn> {
        events
            .iter()
            .filter_map(|event| match event {
                Event::Turn(turn) => Some(turn),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn a_torn_line_waits_for_its_end() {
        let mut bytes = lines(&[HEADER]);
        bytes.extend_from_slice(br#"{"type":"message","timest"#);
        let mut carry = Carry::default();
        let (events, used) = read(&bytes, &mut carry, &refusals());
        assert_eq!(events.len(), 1);
        assert_eq!(used, HEADER.len() + 1);
    }

    #[test]
    fn the_first_write_is_fresh_and_a_pause_rebuilds() {
        let bytes = lines(&[
            HEADER,
            &turn("2026-10-14T10:00:00Z", 0, 50_000, 0),
            &turn("2026-10-14T10:01:00Z", 50_000, 2_000, 0),
            // Ten minutes later, past the five-minute cache: all rebuilt.
            &turn("2026-10-14T10:11:00Z", 0, 52_000, 0),
        ]);
        let mut carry = Carry::default();
        let (events, _) = read(&bytes, &mut carry, &refusals());
        let turns = turns(&events);
        assert_eq!(turns[0].cause, Some(Cause::Fresh));
        assert_eq!(turns[1].cause, None, "growth alone isn't a rebuild");
        assert_eq!(turns[2].cause, Some(Cause::Gap));
        assert_eq!(turns[2].rebuilt, 52_000.0);
        assert!((turns[2].rebuilt_cost - 1.0).abs() < 1e-9);
    }

    #[test]
    fn an_hour_long_cache_survives_a_ten_minute_pause() {
        let bytes = lines(&[
            HEADER,
            &turn("2026-10-14T10:00:00Z", 0, 50_000, 50_000),
            &turn("2026-10-14T10:10:00Z", 0, 50_000, 50_000),
        ]);
        let mut carry = Carry::default();
        let (events, _) = read(&bytes, &mut carry, &refusals());
        assert_eq!(turns(&events)[1].cause, Some(Cause::Unknown));
    }

    #[test]
    fn a_compaction_is_its_own_turn_and_cause() {
        let compaction = concat!(
            r#"{"type":"compaction","timestamp":"2026-10-14T10:01:30Z","tokensBefore":300000,"#,
            r#""usage":{"input":900,"output":11000,"cacheRead":300000,"cacheWrite":0,"#,
            r#""cost":{"total":0.3}}}"#
        );
        let bytes = lines(&[
            HEADER,
            &turn("2026-10-14T10:00:00Z", 0, 50_000, 0),
            compaction,
            &turn("2026-10-14T10:02:00Z", 0, 20_000, 0),
        ]);
        let mut carry = Carry::default();
        let (events, _) = read(&bytes, &mut carry, &refusals());
        let turns = turns(&events);
        assert_eq!(turns[1].kind, Kind::Compaction);
        assert_eq!(turns[2].cause, Some(Cause::Compaction));
    }

    #[test]
    fn a_quest_names_the_turns_after_it() {
        let quest = concat!(
            r#"{"type":"custom","customType":"quest-workflow","#,
            r#""timestamp":"2026-10-14T10:00:30Z","data":{"questId":"QEST-1"}}"#
        );
        let bytes = lines(&[
            HEADER,
            &turn("2026-10-14T10:00:00Z", 0, 1, 0),
            quest,
            &turn("2026-10-14T10:01:00Z", 0, 1, 0),
        ]);
        let mut carry = Carry::default();
        let (events, _) = read(&bytes, &mut carry, &refusals());
        let turns = turns(&events);
        assert_eq!(turns[0].quest, None);
        assert_eq!(turns[1].quest.as_deref(), Some("QEST-1"));
    }

    #[test]
    fn an_extensions_call_is_named() {
        let side = concat!(
            r#"{"type":"custom","customType":"compaction-selection-tags","#,
            r#""timestamp":"2026-10-14T10:00:30Z","data":{"model":"small","#,
            r#""usage":{"input":1000,"output":100,"cacheRead":0,"cacheWrite":0,"#,
            r#""cost":{"total":0.02}}}}"#
        );
        let mut carry = Carry::default();
        let (events, _) = read(&lines(&[HEADER, side]), &mut carry, &refusals());
        let turns = turns(&events);
        assert_eq!(turns[0].kind.name(), "side:compaction-selection-tags");
        assert_eq!(turns[0].model, "small");
    }

    #[test]
    fn a_refusal_is_dated() {
        let refused = concat!(
            r#"{"type":"message","timestamp":"2026-08-01T12:21:12Z","#,
            r#""message":{"role":"assistant","model":"opus","stopReason":"error","#,
            r#""errorMessage":"401 Personal token usage is over the allowed cap.","#,
            r#""usage":{"input":0,"output":0,"cacheRead":0,"cacheWrite":0,"cost":{"total":0}}}}"#
        );
        let mut carry = Carry::default();
        let (events, _) = read(&lines(&[HEADER, refused]), &mut carry, &refusals());
        assert!(matches!(&events[1], Event::Refusal { model, .. } if model == "opus"));
        assert!(turns(&events).is_empty(), "a refusal costs nothing");
    }

    #[test]
    fn carrying_on_reads_the_same_as_reading_once() {
        let bytes = lines(&[
            HEADER,
            &turn("2026-10-14T10:00:00Z", 0, 50_000, 0),
            &turn("2026-10-14T10:11:00Z", 0, 52_000, 0),
        ]);
        let mut once = Carry::default();
        let (whole, _) = read(&bytes, &mut once, &refusals());
        let split = HEADER.len() + 1 + turn("2026-10-14T10:00:00Z", 0, 50_000, 0).len() + 1;
        let mut carry = Carry::default();
        let (mut first, _) = read(&bytes[..split], &mut carry, &refusals());
        let saved: Carry = serde_json::from_str(&serde_json::to_string(&carry).unwrap()).unwrap();
        let mut carry = saved;
        let (rest, _) = read(&bytes[split..], &mut carry, &refusals());
        first.extend(rest);
        assert_eq!(first, whole);
        assert_eq!(carry, once);
    }

    #[test]
    fn a_repository_is_named_by_where_it_lives() {
        assert_eq!(
            repo_of("/home/a/src/github.com/me/thing/sub"),
            "github.com/me/thing"
        );
        assert_eq!(
            repo_of("/home/a/big/trees/root/src/areas/tools/x"),
            "big/areas/tools/x"
        );
        assert_eq!(repo_of("/home/a/scratch"), "scratch");
    }
}
