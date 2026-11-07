//! pi's session logs, read as they grow. Each log is JSON lines: a header
//! naming the session and its directory, then entries. What counts here:
//!
//! - an assistant message with usage is a turn, priced at pi's list rates;
//! - one that ended in a provider's refusal for being over a cap is a
//!   refusal, dated, which is how a cut-off shows with no feed at all;
//! - a compaction entry with usage is a turn of its own kind;
//! - a custom entry with usage is an extension's side call, named by its
//!   type;
//! - a quest-workflow entry says which quest the turns after it serve, and
//!   before one, a session working in a quest's workspace serves that quest;
//! - a session-info entry names the session, as an agent's names its agent;
//! - a budget entry says which budget the turns after it on its branch
//!   draw on, by the nearest among each turn's ancestors;
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
use std::collections::HashMap;

/// Below this many tokens a rebuild is noise: providers cache in blocks.
const REBUILD_FLOOR: f64 = 1024.0;
/// How long a provider keeps cache: five minutes, or an hour for writes
/// asked to last one.
const CACHE_LIFE: f64 = 300.0;
const CACHE_LIFE_LONG: f64 = 3600.0;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Carry {
    /// The kind of work an agent's log is, by where it was found; none for
    /// a person's own. Given afresh each read, so never kept.
    #[serde(skip)]
    pub kind: Option<String>,
    /// Where quests' workspaces are, given afresh each read.
    #[serde(skip)]
    pub workspaces: Option<String>,
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
    /// For a log pi carried a session into, when it began: what comes
    /// before is a copy of the parent's log, which holds it already.
    pub copied_until: Option<f64>,
    /// Whether the header was read with copies in mind, so a log held
    /// from before they were is checked once.
    pub copies_known: bool,
    /// The budget the branch being read draws on, by its nearest budget
    /// entry: none before any, empty for the default, else its name. Once
    /// any entry has named one it's never none again, so a branch from an
    /// entry no budget entry is above draws on the default. Which budget a
    /// branch's parent drew on is found in the log itself when one is
    /// taken, which is rare, rather than every entry's being kept here,
    /// which is written at every read.
    pub budget: Option<String>,
    /// The last entry read, by its id, to tell a branch taken from
    /// further back from the one carrying on.
    pub last_id: Option<String>,
}

/// What a log's entries say of their branches: each one's parent, and
/// what each budget entry names. It's gathered only to find where a branch
/// taken from further back came from, and never kept.
#[derive(Debug, Default)]
pub struct Ancestry {
    parents: HashMap<String, String>,
    named: HashMap<String, String>,
}

/// Just what of an entry tells its branch, so a long message is passed
/// over rather than held.
#[derive(Deserialize)]
struct Lineage {
    id: Option<String>,
    #[serde(rename = "parentId")]
    parent: Option<String>,
    #[serde(rename = "type")]
    kind: Option<String>,
    #[serde(rename = "customType")]
    custom: Option<String>,
    data: Option<Value>,
}

impl Ancestry {
    /// Learns what one line says of its entry; a line that isn't an entry
    /// says nothing. Only a custom entry can be a budget entry, so of any
    /// other just the start is read, where pi writes its id and parent:
    /// most lines are messages, and a log is read whole this way.
    pub fn learn(&mut self, line: &[u8]) {
        let head = &line[..line.len().min(HEAD)];
        if !head.starts_with(br#"{"type":"custom""#)
            && let Some((id, parent)) = head_ids(head)
        {
            if let Some(parent) = parent {
                self.parents.insert(id.to_owned(), parent.to_owned());
            }
            return;
        }
        let Ok(entry) = serde_json::from_slice::<Lineage>(line) else {
            return;
        };
        let Some(id) = entry.id else {
            return;
        };
        if let Some(name) = budget_named(
            entry.kind.as_deref(),
            entry.custom.as_deref(),
            entry.data.as_ref(),
        ) {
            self.named.insert(id.clone(), name);
        }
        if let Some(parent) = entry.parent.filter(|parent| !parent.is_empty()) {
            self.parents.insert(id, parent);
        }
    }

    /// What the nearest budget entry at or above an entry names: empty for
    /// the default, none when no budget entry is above it.
    fn budget_of(&self, id: &str) -> Option<String> {
        let mut at = id;
        // A log's parents can't loop, but a damaged one is walked no
        // further than it has entries.
        for _ in 0..=self.parents.len() {
            if let Some(name) = self.named.get(at) {
                return Some(name.clone());
            }
            at = self.parents.get(at)?;
        }
        None
    }
}

/// How far into a line an entry's type, id and parent are looked for.
const HEAD: usize = 256;

/// An entry's id and parent where pi writes them, among the top-level
/// fields that open its line: `"id"` as a string, then `"parentId"` as a
/// string or null. None when they aren't there before any nested object, so the
/// whole line is read instead.
fn head_ids(head: &[u8]) -> Option<(&str, Option<&str>)> {
    let text = match std::str::from_utf8(head) {
        Ok(text) => text,
        Err(cut) => std::str::from_utf8(&head[..cut.valid_up_to()]).ok()?,
    };
    let quoted = |key: &str| -> Option<Option<&str>> {
        let at = text.find(key)?;
        if text[1..at].contains('{') {
            return None;
        }
        let rest = &text[at + key.len()..];
        if rest.starts_with("null") {
            return Some(None);
        }
        let rest = rest.strip_prefix('"')?;
        Some(Some(&rest[..rest.find('"')?]))
    };
    let id = quoted("\"id\":")??;
    let parent = quoted("\"parentId\":")?;
    Some((id, parent.filter(|parent| !parent.is_empty())))
}

/// The ancestry of a log's lines, read one at a time.
pub fn ancestry(lines: impl std::io::BufRead) -> Ancestry {
    let mut ancestry = Ancestry::default();
    for line in lines.split(b'\n').map_while(Result::ok) {
        ancestry.learn(&line);
    }
    ancestry
}

/// The version of the harness's budget entry this reads; another is
/// passed over, as a feed's newer line is.
const BUDGET_ENTRY: u64 = 1;

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
    /// The budget its log said it draws on: none where no entry said,
    /// empty for the default, else the budget's name.
    pub budget: Option<String>,
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
        kind: Option<String>,
    },
    Named {
        session: String,
        name: String,
    },
    Turn(Turn),
    Refusal {
        session: String,
        at: f64,
        model: String,
        said: String,
    },
}

/// Reads what's whole in `bytes`, the start of a log, and says how many
/// bytes it used: up to the last line break.
pub fn read(bytes: &[u8], carry: &mut Carry, refusals: &[String]) -> (Vec<Event>, usize) {
    read_after(bytes, carry, refusals, &Ancestry::default)
}

/// Reads what's whole in `bytes`, which start where the last read
/// stopped, as `read` does. `earlier` gives the ancestry of what the log
/// held before them, asked only for a branch taken from further back.
pub fn read_after(
    bytes: &[u8],
    carry: &mut Carry,
    refusals: &[String],
    earlier: &dyn Fn() -> Ancestry,
) -> (Vec<Event>, usize) {
    let used = bytes
        .iter()
        .rposition(|&byte| byte == b'\n')
        .map_or(0, |at| at + 1);
    let mut events = Vec::new();
    let mut start = 0;
    // Gathered at the first branch taken from further back, and then only
    // added to, so a read with several pays for one.
    let mut known: Option<(Ancestry, usize)> = None;
    for line in bytes[..used].split(|&byte| byte == b'\n') {
        let begins = start;
        start += line.len() + 1;
        if line.is_empty() {
            continue;
        }
        // A line that isn't JSON is skipped rather than stopping the log.
        let Ok(entry) = serde_json::from_slice::<Value>(line) else {
            continue;
        };
        if let Some(parent) = branched_from(&entry, carry) {
            let (ancestry, learned) = known.get_or_insert_with(|| (earlier(), 0));
            for before in bytes[*learned..begins].split(|&byte| byte == b'\n') {
                ancestry.learn(before);
            }
            *learned = begins;
            carry.budget = ancestry
                .budget_of(parent)
                .or_else(|| carry.budget.is_some().then(String::new));
        }
        let copied = carry
            .copied_until
            .is_some_and(|until| stamp(&entry).is_some_and(|at| at < until));
        let from = events.len();
        entry_events(&entry, carry, refusals, &mut events);
        // A copy still sets the context its log's own turns read, but
        // its turns and refusals are its parent's to count.
        if copied {
            let read = events.split_off(from);
            events.extend(
                read.into_iter()
                    .filter(|event| !matches!(event, Event::Turn(_) | Event::Refusal { .. })),
            );
        }
    }
    (events, used)
}

/// Whether a header is that of a log pi carried a session into, from a
/// parent whose log is still there to count what was copied.
pub fn carried(header: &Value) -> bool {
    header.get("type").and_then(Value::as_str) == Some("session")
        && header
            .get("parentSession")
            .and_then(Value::as_str)
            .is_some_and(|parent| std::path::Path::new(parent).is_file())
}

/// The quest whose workspace `cwd` is in: the directory under `workspaces`
/// that it is or is inside, which the harness names by the quest's id.
fn workspace_quest(workspaces: &str, cwd: &str) -> Option<String> {
    let inside = cwd.strip_prefix(workspaces.trim_end_matches('/'))?;
    let id = inside.strip_prefix('/')?.split('/').next()?;
    (!id.is_empty()).then(|| id.to_owned())
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

/// The parent of an entry that's a branch taken from further back: one
/// whose parent isn't the last entry read. A log without ids has none.
fn branched_from<'a>(entry: &'a Value, carry: &Carry) -> Option<&'a str> {
    entry.get("id").and_then(Value::as_str)?;
    let parent = entry
        .get("parentId")
        .and_then(Value::as_str)
        .filter(|parent| !parent.is_empty())?;
    (carry.last_id.as_deref() != Some(parent)).then_some(parent)
}

/// Follows an entry onto its branch: a budget entry names the budget the
/// entries after it on its branch draw on. A branch taken from further
/// back has its budget found before this, in `read_after`; a log without
/// ids follows the file's order.
fn follow(entry: &Value, carry: &mut Carry) {
    let Some(id) = entry.get("id").and_then(Value::as_str) else {
        return;
    };
    if let Some(named) = budget_entry(entry) {
        carry.budget = Some(named);
    }
    carry.last_id = Some(id.to_owned());
}

/// The budget a budget entry of a version this reads names: empty for
/// the default.
fn budget_entry(entry: &Value) -> Option<String> {
    budget_named(
        entry.get("type").and_then(Value::as_str),
        entry.get("customType").and_then(Value::as_str),
        entry.get("data"),
    )
}

/// What an entry of a type, a custom type and data names, if it's a
/// budget entry of a version this reads: empty for the default.
fn budget_named(kind: Option<&str>, custom: Option<&str>, data: Option<&Value>) -> Option<String> {
    if kind != Some("custom") || custom != Some("budget") {
        return None;
    }
    let data = data?;
    if data.get("v").and_then(Value::as_u64) != Some(BUDGET_ENTRY) {
        return None;
    }
    match data.get("name")? {
        Value::Null => Some(String::new()),
        Value::String(name) => Some(name.clone()),
        _ => None,
    }
}

fn entry_events(entry: &Value, carry: &mut Carry, refusals: &[String], events: &mut Vec<Event>) {
    let kind = entry.get("type").and_then(Value::as_str).unwrap_or("");
    let at = stamp(entry);
    follow(entry, carry);
    match kind {
        "session" => session_entry(entry, at, carry, events),
        "session_info" => {
            let name = entry.get("name").and_then(Value::as_str).unwrap_or("");
            if let Some(session) = carry.session.clone().filter(|_| !name.is_empty()) {
                events.push(Event::Named {
                    session,
                    name: name.to_owned(),
                });
            }
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
        "custom" => custom_entry(entry, at, carry, events),
        "message" => {
            if let (Some(message), Some(at)) = (entry.get("message"), at) {
                message_entry(message, at, carry, refusals, events);
            }
        }
        _ => {}
    }
}

/// A log's header: the session it is, where it works, and whether what
/// comes before it was copied from its parent's log.
fn session_entry(entry: &Value, at: Option<f64>, carry: &mut Carry, events: &mut Vec<Event>) {
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
    carry.copies_known = true;
    if carried(entry) {
        carry.copied_until = at;
    }
    if carry.quest.is_none() {
        carry.quest = carry
            .workspaces
            .as_deref()
            .and_then(|workspaces| workspace_quest(workspaces, &cwd));
    }
    events.push(Event::Session {
        id,
        cwd,
        at: at.unwrap_or(0.0),
        kind: carry.kind.clone(),
    });
}

/// An extension's own entry: a quest taken up, or a side call's turn.
fn custom_entry(entry: &Value, at: Option<f64>, carry: &mut Carry, events: &mut Vec<Event>) {
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

/// A message with an instant: an assistant's turn, and a refusal when it
/// ended in an error a refusal's pattern matches.
fn message_entry(
    message: &Value,
    at: f64,
    carry: &mut Carry,
    refusals: &[String],
    events: &mut Vec<Event>,
) {
    if message.get("role").and_then(Value::as_str) != Some("assistant") {
        return;
    }
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

#[expect(
    clippy::too_many_arguments,
    reason = "a turn is what the log's entry gives apart: its instant, kind, tokens, cost, \
              model and provider, onto what the log has carried so far"
)]
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
        kind,
        tokens,
        cost,
        quest: carry.quest.clone(),
        budget: carry.budget.clone(),
        gap: None,
        rebuilt: 0.0,
        rebuilt_cost: 0.0,
        cause: None,
    };
    if turn.kind == Kind::Agent {
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

    // pi carries a session into a new log by copying its entries, with
    // their times, ahead of the new header; the parent's log still holds
    // them, so only what comes after the header is the new log's own.
    fn carried(parent: &str) -> String {
        serde_json::json!({
            "type": "session", "id": "s2", "timestamp": "2026-10-14T10:30:00Z",
            "cwd": "/home/someone/elsewhere", "parentSession": parent,
        })
        .to_string()
    }

    #[test]
    fn a_carried_log_counts_only_its_own_turns() {
        let parent = concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml");
        let bytes = lines(&[
            &carried(parent),
            &turn("2026-10-14T10:00:00Z", 0, 50_000, 0),
            &turn("2026-10-14T10:31:00Z", 50_000, 2_000, 0),
        ]);
        let mut carry = Carry::default();
        let (events, _) = read(&bytes, &mut carry, &refusals());
        let turns = turns(&events);
        assert_eq!(turns.len(), 1);
        assert_eq!(turns[0].session, "s2");
        // The copy is still the context the first turn of its own read.
        assert_eq!(turns[0].gap, Some(31.0 * 60.0));
        assert_eq!(turns[0].cause, None);
    }

    // A copy whose parent's log is gone is the only record of its turns.
    #[test]
    fn a_copy_without_its_parent_keeps_every_turn() {
        let bytes = lines(&[
            &carried("/nowhere/2026_gone.jsonl"),
            &turn("2026-10-14T10:00:00Z", 0, 50_000, 0),
            &turn("2026-10-14T10:31:00Z", 50_000, 2_000, 0),
        ]);
        let mut carry = Carry::default();
        let (events, _) = read(&bytes, &mut carry, &refusals());
        assert_eq!(turns(&events).len(), 2);
    }

    #[test]
    #[expect(
        clippy::float_cmp,
        reason = "token counts are whole numbers, which f64 holds and subtracts exactly"
    )]
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

    // An agent working in a quest's workspace serves that quest, until a
    // quest entry says otherwise; its session-info names it.
    #[test]
    fn an_agent_in_a_workspace_serves_its_quest() {
        let header = concat!(
            r#"{"type":"session","id":"a1","timestamp":"2026-10-14T10:00:00Z","#,
            r#""cwd":"/w/QEST-2/hub/agents/acp@team/team"}"#
        );
        let named = r#"{"type":"session_info","name":"acp@team"}"#;
        let quest = concat!(
            r#"{"type":"custom","customType":"quest-workflow","#,
            r#""timestamp":"2026-10-14T10:00:30Z","data":{"questId":"QEST-1"}}"#
        );
        let bytes = lines(&[
            header,
            named,
            &turn("2026-10-14T10:00:10Z", 0, 1, 0),
            quest,
            &turn("2026-10-14T10:01:00Z", 0, 1, 0),
        ]);
        let mut carry = Carry {
            kind: Some("swarm".into()),
            workspaces: Some("/w/".into()),
            ..Carry::default()
        };
        let (events, _) = read(&bytes, &mut carry, &refusals());
        assert_eq!(
            events[0],
            Event::Session {
                id: "a1".into(),
                cwd: "/w/QEST-2/hub/agents/acp@team/team".into(),
                at: crate::time::parse("2026-10-14T10:00:00Z").unwrap(),
                kind: Some("swarm".into()),
            }
        );
        assert_eq!(
            events[1],
            Event::Named {
                session: "a1".into(),
                name: "acp@team".into(),
            }
        );
        let turns = turns(&events);
        assert_eq!(turns[0].quest.as_deref(), Some("QEST-2"));
        assert_eq!(turns[1].quest.as_deref(), Some("QEST-1"));
    }

    /// A turn in a log whose entries have ids, with its parent's.
    fn tree_turn(id: &str, parent: &str, at: &str) -> String {
        let mut entry: Value = serde_json::from_str(&turn(at, 0, 1, 0)).unwrap();
        entry["id"] = id.into();
        entry["parentId"] = parent.into();
        entry.to_string()
    }

    fn budget(id: &str, parent: &str, name: Option<&str>) -> String {
        serde_json::json!({
            "type": "custom", "customType": "budget", "id": id, "parentId": parent,
            "timestamp": "2026-11-07T02:20:33Z",
            "data": {"v": 1, "name": name, "label": "Delta team token"},
        })
        .to_string()
    }

    fn budgets(events: &[Event]) -> Vec<Option<&str>> {
        turns(events)
            .iter()
            .map(|turn| turn.budget.as_deref())
            .collect()
    }

    // Before any entry a turn's log says nothing of its budget; after one,
    // it draws on the budget named, or the default for none.
    #[test]
    fn a_budget_entry_names_the_turns_after_it() {
        let bytes = lines(&[
            HEADER,
            &tree_turn("a", "", "2026-11-07T02:00:00Z"),
            &budget("b", "a", Some("delta")),
            &tree_turn("c", "b", "2026-11-07T02:21:00Z"),
            &budget("d", "c", None),
            &tree_turn("e", "d", "2026-11-07T02:22:00Z"),
        ]);
        let (events, _) = read(&bytes, &mut Carry::default(), &refusals());
        assert_eq!(budgets(&events), [None, Some("delta"), Some("")]);
    }

    // A branch taken from further back draws on what its parent did, not
    // on the entry last written, even when the branch is read later, from
    // the log itself: the carry kept between reads holds no entry's id
    // but the last.
    #[test]
    fn a_branch_draws_on_its_own_ancestors() {
        let first = lines(&[
            HEADER,
            &budget("a", "", None),
            &tree_turn("b", "a", "2026-11-07T02:00:00Z"),
            &budget("c", "b", Some("delta")),
            &tree_turn("d", "c", "2026-11-07T02:21:00Z"),
            &budget("e", "d", Some("aquifer")),
            &tree_turn("f", "e", "2026-11-07T02:22:00Z"),
        ]);
        let mut carry = Carry::default();
        let (events, _) = read(&first, &mut carry, &refusals());
        assert_eq!(budgets(&events), [Some(""), Some("delta"), Some("aquifer")]);
        let kept = serde_json::to_string(&carry).unwrap();
        assert!(!kept.contains("\"c\"") && !kept.contains("\"d\""), "{kept}");
        let mut carry: Carry = serde_json::from_str(&kept).unwrap();
        let later = lines(&[
            &tree_turn("g", "d", "2026-11-07T02:30:00Z"),
            &tree_turn("h", "g", "2026-11-07T02:31:00Z"),
            &tree_turn("i", "b", "2026-11-07T02:32:00Z"),
        ]);
        let earlier = || ancestry(first.as_slice());
        let (events, _) = read_after(&later, &mut carry, &refusals(), &earlier);
        assert_eq!(budgets(&events), [Some("delta"), Some("delta"), Some("")]);
    }

    // A message's id and parent are read from the start of its line alone,
    // and a line laid out otherwise, or a budget entry, is read whole.
    #[test]
    fn an_ancestry_reads_a_line_s_start_where_it_can() {
        let long = "x".repeat(4000);
        let message = format!(
            r#"{{"type":"message","id":"b","parentId":"a","message":{{"text":"{long}"}}}}"#
        );
        assert_eq!(head_ids(message.as_bytes()), Some(("b", Some("a"))));
        let first = r#"{"type":"model_change","id":"a","parentId":null}"#;
        assert_eq!(head_ids(first.as_bytes()), Some(("a", None)));
        let nested = r#"{"type":"x","message":{"id":"n"},"id":"c","parentId":"b"}"#.to_owned();
        assert_eq!(head_ids(nested.as_bytes()), None);
        let mut ancestry = Ancestry::default();
        for line in [
            first.to_owned(),
            message,
            budget("d", "b", Some("delta")),
            nested,
        ] {
            ancestry.learn(line.as_bytes());
        }
        assert_eq!(ancestry.budget_of("d").as_deref(), Some("delta"));
        assert_eq!(ancestry.budget_of("c"), None);
        assert_eq!(ancestry.parents.get("c").map(String::as_str), Some("b"));
    }

    // A branch from an entry no budget entry is above draws on the default
    // once any budget has been named, and on nothing said before then.
    #[test]
    fn a_branch_from_above_every_budget_entry_draws_on_the_default() {
        let bytes = lines(&[
            HEADER,
            &tree_turn("a", "", "2026-11-07T02:00:00Z"),
            &budget("b", "a", Some("delta")),
            &tree_turn("c", "b", "2026-11-07T02:21:00Z"),
            &tree_turn("d", "a", "2026-11-07T02:22:00Z"),
        ]);
        let (events, _) = read(&bytes, &mut Carry::default(), &refusals());
        assert_eq!(budgets(&events), [None, Some("delta"), Some("")]);
    }

    // A log without ids is followed in the file's order, and an entry of a
    // version this doesn't read is passed over.
    #[test]
    fn a_budget_entry_of_another_version_is_passed_over() {
        let newer = budget("x", "", Some("delta")).replace("\"v\":1", "\"v\":2");
        let bytes = lines(&[
            HEADER,
            &newer,
            &turn("2026-11-07T02:00:00Z", 0, 1, 0),
            &budget("y", "", Some("aquifer")),
            &turn("2026-11-07T02:01:00Z", 0, 1, 0),
        ]);
        let (events, _) = read(&bytes, &mut Carry::default(), &refusals());
        assert_eq!(budgets(&events), [None, Some("aquifer")]);
    }

    #[test]
    fn a_workspace_s_quest_is_the_directory_under_it() {
        assert_eq!(
            workspace_quest("/w", "/w/QEST-2/a/b").as_deref(),
            Some("QEST-2")
        );
        assert_eq!(
            workspace_quest("/w/", "/w/QEST-2").as_deref(),
            Some("QEST-2")
        );
        assert_eq!(workspace_quest("/w", "/work/QEST-2"), None);
        assert_eq!(workspace_quest("/w", "/w"), None);
        assert_eq!(workspace_quest("/w", "/elsewhere"), None);
    }

    // A carry kept before it had more to say still reads, so a log under
    // way isn't read again from its start.
    #[test]
    fn an_older_carry_still_reads() {
        let carry: Carry = serde_json::from_str(r#"{"session":"s1","quest":"QEST-1"}"#).unwrap();
        assert_eq!(carry.session.as_deref(), Some("s1"));
        assert_eq!(carry.quest.as_deref(), Some("QEST-1"));
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
