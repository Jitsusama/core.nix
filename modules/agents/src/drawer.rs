//! What a session's drawer says, made ready as the launcher's
//! SessionDrawer.js once made it, so its first screen holds what matters:
//! a glance of six facts at most, in VKCF3Z's order (2b03e43c), and the
//! rest folded under z a. The drawer shows only whole facts, so a long
//! one ahead of the rest left the title and "13 more" alone on every
//! session (Joel, 2026-11-07: "the agents side panel generally just shows
//! its title. i have to ctrl-w h into it and then scroll a ton of lines
//! before i see any more details"), and fourteen of them made a glance
//! no one could take in at once.
//!
//! It's a record of its own, `drawer:agents:<address>`, never part of a
//! row, since its figures move every turn: the shell watches it by key
//! only while it's shown. Its times go out as `{at, as}`, worded by the
//! shell, so a drawer left open doesn't change each minute.
//!
//! Long because its cases are over half of it: each place in the glance
//! and the fold is held to one, and every fact is made in one pass that
//! splitting would leave two files reading the same session.

use serde_json::{Value, json};

use crate::clock;
use crate::figures;
use crate::row::Words;
use crate::session::Session;

/// The longest thing a step is on that's said beside its tool: anything
/// longer is a command or a path the harness has already cut short.
const SHORT: usize = 24;
const ELLIPSIS: char = '\u{2026}';
const DOT: &str = " \u{b7} ";

/// The key a session's drawer goes by.
pub fn key(address: &str) -> String {
    format!("drawer:agents:{address}")
}

/// Its glance, six places at most, each taking the first of its facts
/// that says something, in VKCF3Z's order (2b03e43c, 2026-11-14): what
/// it needs Joel for, else what it's doing, since a waiting session's
/// step would only say so again; its last turn; what it spent, its
/// councils and fleets on that line; how full its context is; then its
/// sessions, for an orchestration, or its host where it isn't here. One
/// with neither stops at four, nothing moving up. What its quest is
/// about is the drawer's prose under them rather than a fact (VKCF3Z,
/// 2026-11-14, a2101591): it runs to 47 to 282 letters in each of Joel's
/// 202 drawers that day, which as the ruling's second fact wrapped past
/// the first screen of a 288-pixel drawer, hiding every figure after it.
const GLANCE: [&[&str]; 5] = [
    &["wants", "step"],
    &["last-turn"],
    &["spent"],
    &["context"],
    &["under", "host"],
];

/// Its fold under z a, in order: its figures that move least and its
/// wiring (VKCF3Z, 2b03e43c), then every fact its glance once held, so
/// none is lost, what its quest is about last where what it stands for
/// at the top is its prose. A step that gave way to what it needs Joel
/// for is in neither.
const FOLD: [&str; 14] = [
    "model",
    "turns",
    "tokens",
    "cache",
    "session",
    "rebuilt",
    "hears",
    "last-landing",
    "budget",
    "task",
    "own",
    "host",
    "machine",
    "works",
];

/// A fact by the id its name is said by, as the drawer gives it.
type Told = (String, Value);

/// What a drawer is made from beyond the session's own record, each as
/// its source tells of it.
pub struct Known<'a> {
    /// The titles of what a quest waits on Joel for, in the order For You
    /// says them, by the quest's id (the to-do daemon's).
    pub needs: &'a dyn Fn(&str) -> Vec<String>,
    /// Spend-watch's collection, or none before it's written.
    pub spend: Option<&'a Value>,
    /// Spend-watch's budgets, or none before they're written.
    pub budgets: Option<&'a Value>,
    /// Whether the screen is shared, so no amount is said.
    pub sharing: bool,
    /// Today's date where Joel is, as the harness dates a session's usage.
    pub today: &'a str,
}

/// What a step is, from the record's `stepDetail`: "thinking", or a tool
/// and what it's on, a file by its name alone.
pub fn step(detail: Option<&Value>) -> String {
    let kind = detail
        .and_then(|detail| detail.get("kind"))
        .and_then(Value::as_str)
        .unwrap_or("");
    if kind != "tool" {
        return kind.to_owned();
    }
    let said = |field| {
        detail
            .and_then(|detail| detail.get(field))
            .and_then(Value::as_str)
            .unwrap_or("")
    };
    let on = said("detail");
    let whole = !on.is_empty() && !on.ends_with(ELLIPSIS);
    let file = if whole && (on.starts_with('/') || on.starts_with('~')) {
        on.rsplit('/').next().unwrap_or("")
    } else {
        ""
    };
    let what = if !file.is_empty() {
        file
    } else if whole && on.chars().count() <= SHORT {
        on
    } else {
        ""
    };
    joined(&[said("name"), what], " ")
}

/// What its quest is about, from the record's `worksOn`, which leads with
/// the quest's title, the drawer's own title already, then its summary.
pub fn about<'a>(works: &'a str, titles: &[&str]) -> &'a str {
    titles
        .iter()
        .filter(|title| !title.is_empty())
        .find_map(|title| works.strip_prefix(&format!("{title}: ")))
        .unwrap_or(works)
}

/// Parts said one after another, those saying nothing left out.
fn joined(parts: &[&str], between: &str) -> String {
    parts
        .iter()
        .filter(|part| !part.is_empty())
        .copied()
        .collect::<Vec<_>>()
        .join(between)
}

/// A field of a record as words, or none.
fn text<'a>(record: Option<&'a Value>, path: &str) -> &'a str {
    record
        .and_then(|record| record.pointer(path))
        .and_then(Value::as_str)
        .unwrap_or("")
}

/// The budget a session drew on last and how it stands, as its Budget
/// says it: "Your AI proxy token · $24,349 of $40,000". None when the
/// budgets don't name the session or the budget it names.
fn budget(budgets: Option<&Value>, session: &str) -> String {
    let Some(budgets) = budgets else {
        return String::new();
    };
    let name = text(budgets.get("sessions"), &format!("/{session}"));
    if name.is_empty() {
        return String::new();
    }
    budgets
        .get("budgets")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .find(|budget| budget.get("name").and_then(Value::as_str) == Some(name))
        .map(|budget| {
            joined(
                &[
                    text(Some(budget), "/label"),
                    text(Some(budget), "/standing"),
                ],
                DOT,
            )
        })
        .unwrap_or_default()
}

/// What it landed last, newest by when: what and its ref, each an item
/// the drawer keeps whole on its line, and when.
fn landed(presence: Option<&Value>) -> Option<(Vec<String>, Option<i64>)> {
    let landings = presence?.get("landings")?.as_array()?;
    let at = |one: &Value| {
        one.get("at")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_owned()
    };
    let last = landings
        .iter()
        .fold(None::<&Value>, |newest, one| match newest {
            Some(newest) if at(newest) >= at(one) => Some(newest),
            _ => Some(one),
        })?;
    let items = [text(Some(last), "/what"), text(Some(last), "/ref")]
        .into_iter()
        .filter(|item| !item.is_empty())
        .map(str::to_owned)
        .collect();
    Some((items, last.get("at").and_then(clock::ms_of)))
}

/// Its address, where it works and since when it's run are the eyebrow
/// over its title, "8IDU4E · core.nix · since 21:52" (VKCF3Z,
/// 2026-11-06), given way by whole items from its end, as the held foot
/// gives its position way whole: when it started goes first, then where
/// it works, and its address never (VKCF3Z, 2026-11-07: "since 0…" tells
/// nothing). How it stands is its row's, on the same line, so none is
/// said again. Then its facts, each as a name and a value, its glance's
/// (`GLANCE`) and the rest folded (`FOLD`). Nothing it hasn't said is
/// guessed, and no amount is said while the screen is shared.
pub fn drawer(session: Session, orchestrates: bool, words: &Words, known: &Known) -> Value {
    let presence = session.presence();
    let (title, folder) = session.named();
    let usage = presence.and_then(|presence| presence.get("usage"));
    let mut told = first(session, orchestrates, words, known);
    for figure in figures::figures(usage, known.today, &words.agents, &|id| id.to_owned()) {
        let mut said = json!({"label": words.names.word(&figure.label), "value": figure.value});
        if let Some(stroke) = figure.stroke {
            said["figure"] = json!({"shape": "stroke", "values": [stroke.value],
                "warn": stroke.warn});
        }
        told.push((figure.label, said));
    }
    told.extend(last(session, &title, words, known));
    told.extend(hears(session, words));
    let facts: Vec<Value> = GLANCE
        .iter()
        .filter_map(|place| place.iter().find_map(|id| taken(&mut told, id)))
        .collect();
    // Its prose: what it stands for at the top, else what its quest is
    // about, drawn whole, since z a opens the fold rather than reading on
    // (VKCF3Z, e3bb38e5); what runs past the card is its more line's.
    let stands_for = session.stands_for();
    let prose = if stands_for.is_empty() {
        taken(&mut told, "works")
    } else {
        None
    };
    let body = prose
        .as_ref()
        .map_or(stands_for, |about| text(Some(about), "/value"));
    let nerd: Vec<Value> = FOLD
        .iter()
        .filter_map(|id| taken(&mut told, id))
        .map(folded)
        .collect();
    let address = session.address();
    let mut drawn = json!({
        "key": key(address),
        "kind": "drawer",
        "drawer": {
            "title": title,
            "eyebrow": eyebrow(session, &folder),
            "facts": facts,
            "body": body
        }
    });
    if prose.is_some() {
        drawn["drawer"]["prose"] = json!(true);
        drawn["drawer"]["whole"] = json!(true);
    }
    if !nerd.is_empty() {
        drawn["drawer"]["nerd"] = Value::Array(nerd);
    }
    drawn
}

/// The fact `id` names, taken from those told, or none where none said it.
fn taken(told: &mut Vec<Told>, id: &str) -> Option<Value> {
    let at = told.iter().position(|(one, _)| one == id)?;
    Some(told.remove(at).1)
}

/// A fact as the fold says it: its name, its value and when, since a
/// folded bit draws nothing beside its value and runs as long as it is.
fn folded(mut fact: Value) -> Value {
    if let Some(fact) = fact.as_object_mut() {
        fact.retain(|field, _| matches!(field.as_str(), "label" | "value" | "at"));
    }
    fact
}

/// Its wiring, folded under z a rather than in the glance (VKCF3Z,
/// 2026-11-13): the groups it hears beside its own address, as the node
/// says them.
fn hears(session: Session, words: &Words) -> Option<Told> {
    let hears: Vec<&str> = session
        .presence()
        .and_then(|presence| presence.get("groups"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect();
    if hears.is_empty() {
        return None;
    }
    let said = json!({"label": words.names.word("hears"), "value": hears.join(", ")});
    Some(("hears".to_owned(), said))
}

/// What it's doing, its step alone; what it needs Joel for, its want then
/// its quest's to-dos; what it landed last, with when; what it's spent
/// today (an orchestrator's whole orchestration, then its own and its
/// sessions'), its own carrying its hours as Graph's spark; and the
/// budget it drew on last.
fn first(session: Session, orchestrates: bool, words: &Words, known: &Known) -> Vec<Told> {
    let presence = session.presence();
    let said = |path| text(presence, path);
    let id = said("/session");
    let quest = if said("/quest").is_empty() {
        said("/questInfo/id")
    } else {
        said("/quest")
    };
    let needs = if quest.is_empty() {
        Vec::new()
    } else {
        (known.needs)(quest)
    };
    let collection = known.spend.filter(|_| !known.sharing);
    let owned = session.quest();
    let family = collection
        .and_then(|collection| collection.get("families"))
        .and_then(|families| families.get(owned))
        .filter(|_| orchestrates && !owned.is_empty());
    let spent = family.map_or_else(
        || text(collection, &format!("/sessions/{id}")).to_owned(),
        |family| {
            words
                .agents
                .fill("allOf", &[("spent", text(Some(family), "/all"))])
        },
    );
    // What went on its councils and fleets is a part of what it spent,
    // so it shares that line rather than taking its own (VKCF3Z,
    // 2026-11-13).
    let spent = if spent.is_empty() {
        spent
    } else {
        joined(&[&spent, &helpers(collection, id)], DOT)
    };
    let (landed_items, landed_at) = landed(presence).unwrap_or_default();
    let landed_said = landed_items.join(DOT);
    let budget = if known.sharing {
        String::new()
    } else {
        budget(known.budgets, id)
    };
    let hours_on = if text(family, "/own").is_empty() {
        "spent"
    } else {
        "own"
    };
    let hours = hours(known.spend, id);
    [
        ("step", step(presence.and_then(|p| p.get("stepDetail")))),
        (
            "wants",
            joined(&[said("/wants/said"), &needs.join(DOT)], DOT),
        ),
        ("last-landing", landed_said),
        ("spent", spent),
        ("own", text(family, "/own").to_owned()),
        ("under", text(family, "/under").to_owned()),
        ("budget", budget),
    ]
    .into_iter()
    .filter(|(fact, value)| !value.is_empty() || (*fact == "last-landing" && landed_at.is_some()))
    .map(|(fact, value)| {
        let mut said = json!({"label": words.names.word(fact), "value": value});
        if fact == hours_on
            && let Some(hours) = &hours
        {
            said["figure"] = hours.clone();
        }
        if fact == "last-landing"
            && let Some(at) = landed_at
        {
            said["at"] = json!({"at": at, "as": "clock"});
        }
        (fact.to_owned(), said)
    })
    .collect()
}

/// What of session `id`'s spend today went on its councils and fleets,
/// each named by what it went on as spend-watch says it ("councils
/// $4.00"), dotted.
fn helpers(collection: Option<&Value>, id: &str) -> String {
    let said: Vec<String> = collection
        .and_then(|collection| collection.get("helpers"))
        .and_then(|helpers| helpers.get(id))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|helper| {
            let on = helper.get("on")?.as_str()?;
            let spent = helper.get("spent")?.as_str()?;
            Some(format!("{on} {spent}"))
        })
        .collect();
    said.join(DOT)
}

/// Its hours today as Graph's spark, as spend-watch draws them. They
/// carry no dollars, and its busiest hour nears no limit, so the spark
/// never wears the warning (VKCF3Z, 2026-11-07).
fn hours(spend: Option<&Value>, id: &str) -> Option<Value> {
    let graph = spend?.get("sessionsToday")?.get(id)?.get("graph")?;
    graph.is_object().then(|| {
        let mut graph = graph.clone();
        graph["warn"] = json!(false);
        graph
    })
}

/// Where it belongs: what rebuilding its context cost; its session by
/// pi's id; its quest, how that stands said only where it isn't active;
/// its machine, its host where that isn't here; and what its quest is
/// about, as words, since it runs long.
fn last(session: Session, title: &str, words: &Words, known: &Known) -> Vec<Told> {
    let presence = session.presence();
    let said = |path| text(presence, path);
    let stands = said("/questInfo/status");
    let stands = if stands == "active" { "" } else { stands };
    let task = if said("/questInfo/id").is_empty() {
        String::new()
    } else {
        joined(&[said("/questInfo/id"), stands], DOT)
    };
    let rebuilt = if known.sharing {
        ""
    } else {
        text(
            known
                .spend
                .and_then(|spend| spend.get("sessionsToday"))
                .and_then(|today| today.get(said("/session"))),
            "/rebuilt",
        )
    };
    let machine = if session.here() { "machine" } else { "host" };
    let mut facts: Vec<Told> = [
        ("rebuilt", rebuilt),
        ("session", said("/session")),
        ("task", task.as_str()),
        (machine, session.machine()),
    ]
    .into_iter()
    .filter(|(_, value)| !value.is_empty())
    .map(|(fact, value)| {
        // Where it isn't here its machine takes the glance's sixth place,
        // still named Machine: Host is pi's host, said only on a session
        // given back (VKCF3Z, 2026-11-14, b6631488).
        let named = if fact == "host" { "machine" } else { fact };
        let said = json!({"label": words.names.word(named), "value": value});
        (fact.to_owned(), said)
    })
    .collect();
    let summary = about(said("/worksOn"), &[title, said("/questInfo/title")]);
    if !summary.is_empty() {
        let said = json!({"label": words.names.word("works"), "value": summary, "words": true});
        facts.push(("works".to_owned(), said));
    }
    facts
}

/// Its address, where it works (its repository as its record names it,
/// else its folder) and since when it's run, each kept as long as it
/// matters.
fn eyebrow(session: Session, folder: &str) -> Vec<Value> {
    let presence = session.presence();
    let repo = text(presence, "/repo");
    let place = if repo.is_empty() { folder } else { repo };
    let mut eyebrow = vec![
        json!({"said": session.address(), "keep": 3}),
        json!({"said": place, "keep": 2}),
    ];
    if let Some(at) = presence
        .and_then(|presence| presence.pointer("/owner/startedAt"))
        .and_then(clock::ms_of)
    {
        eyebrow.push(json!({"at": {"at": at, "as": "since"}, "keep": 1}));
    }
    eyebrow
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn words() -> Words {
        Words::from_vocabulary(crate::said::LAUNCHERS).expect("the launcher's words read")
    }

    fn no_needs(_: &str) -> Vec<String> {
        Vec::new()
    }

    fn known<'a>(spend: Option<&'a Value>, budgets: Option<&'a Value>) -> Known<'a> {
        Known {
            needs: &no_needs,
            spend,
            budgets,
            sharing: false,
            today: "2026-11-10",
        }
    }

    fn record(presence: &Value) -> Value {
        json!({"key": "session:s-1", "kind": "session", "machine": "skiff",
            "presence": presence})
    }

    /// Each fact of its glance as `label=value`, a figure's shape after a
    /// colon, a time's form after an at, words marked so.
    fn facts(drawn: &Value) -> String {
        listed(&drawn["drawer"]["facts"])
    }

    /// Each bit folded under z a, as `facts` says a fact.
    fn folded(drawn: &Value) -> String {
        listed(&drawn["drawer"]["nerd"])
    }

    fn listed(list: &Value) -> String {
        list.as_array()
            .into_iter()
            .flatten()
            .map(|fact| {
                let mut said = format!(
                    "{}={}",
                    fact["label"].as_str().unwrap_or(""),
                    fact["value"].as_str().unwrap_or("")
                );
                if let Some(shape) = fact["figure"]["shape"].as_str() {
                    said = format!("{said}:{shape}");
                }
                if let Some(form) = fact["at"]["as"].as_str() {
                    said = format!("{said}@{form}");
                }
                if fact["words"] == json!(true) {
                    said = format!("{said}:words");
                }
                said
            })
            .collect::<Vec<_>>()
            .join("; ")
    }

    /// A session with everything to say, as one working all day says it.
    fn busy() -> Value {
        json!({
            "addresses": ["8IDU4E"], "session": "s-1", "quest": "QEST-1",
            "worksOn": "A Quest: what it's for, at length",
            "stepDetail": {"kind": "tool", "name": "edit", "detail": "/src/core.nix/Drawer.qml"},
            "questInfo": {"id": "QEST-1", "title": "A Quest", "status": "active"},
            "groups": ["#desk"],
            "landings": [
                {"what": "older", "ref": "core.nix#0", "at": "2026-11-10T20:00:00Z"},
                {"what": "Serve rows", "ref": "core.nix#1", "at": "2026-11-10T20:44:00Z"}],
            "usage": {
                "day": "2026-11-10",
                "gen_ai.request.model": "claude-opus-5-5",
                "last": {"gen_ai.usage.output_tokens": 4100},
                "today": {"gen_ai.usage.input_tokens": 786,
                    "gen_ai.usage.output_tokens": 272_470,
                    "gen_ai.usage.cache_read.input_tokens": 124_620_036,
                    "gen_ai.usage.cache_creation.input_tokens": 1_194_960},
                "context": {"used": 84_000, "limit": 200_000},
                "turns": {"today": 359,
                    "last": {"duration_ms": 38000, "first_token_ms": 900}}
            }
        })
    }

    /// Its glance holds six facts at most, in VKCF3Z's order (2b03e43c):
    /// what it's doing, what it works on, its last turn, what it spent
    /// with its councils on that line, how full its context is, then its
    /// host where that isn't here. A session here stops at five, nothing
    /// moving up. The rest is folded under z a, its wiring first, then
    /// every other fact it had, so none is lost.
    #[test]
    fn its_glance_holds_six_facts_in_order_and_folds_the_rest() {
        let spend = json!({"sessions": {"s-1": "$4.20"},
            "helpers": {"s-1": [{"on": "councils", "spent": "$4.00"}]},
            "sessionsToday": {"s-1": {"graph": {"shape": "spark", "values": [0, 1]},
                "rebuilt": "$1.20"}}});
        let budgets = json!({"sessions": {"s-1": "proxy"}, "budgets": [
            {"name": "proxy", "label": "Your AI proxy token", "standing": "$24,349 of $40,000"}]});
        let glance = "Doing=edit Drawer.qml; \
                      Last Turn=38 s \u{b7} 4.1k out \u{b7} 0.9 s first token; \
                      Spent Today=$4.20 \u{b7} councils $4.00:spark; Context=84k of 200k:stroke";
        let fold = "Model=claude-opus-5-5; Turns Today=359; Tokens=126M in \u{b7} 272k out; \
                    Cache=99.0% read; Session=s-1; Context Rebuilt=$1.20; Hears=#desk; \
                    Last Landing=Serve rows \u{b7} core.nix#1@clock; \
                    Budget=Your AI proxy token \u{b7} $24,349 of $40,000; Quest=QEST-1";
        for (here, sixth, last) in [
            (false, "; Machine=skiff", ""),
            (true, "", "; Machine=skiff"),
        ] {
            let mut record = record(&busy());
            record["here"] = json!(here);
            let drawn = drawer(
                Session(&record),
                false,
                &words(),
                &known(Some(&spend), Some(&budgets)),
            );
            assert_eq!(facts(&drawn), format!("{glance}{sixth}"), "here: {here}");
            assert_eq!(
                drawn["drawer"]["body"], "what it's for, at length",
                "its prose"
            );
            assert_eq!(drawn["drawer"]["prose"], json!(true), "set as prose");
            assert_eq!(drawn["drawer"]["whole"], json!(true), "never cut");
            assert_eq!(folded(&drawn), format!("{fold}{last}"), "here: {here}");
            assert_eq!(
                drawn["drawer"]["facts"][2]["figure"]["warn"],
                json!(false),
                "the spark never warns"
            );
            assert_eq!(drawn["key"], "drawer:agents:8IDU4E");
            assert_eq!(drawn["kind"], "drawer");
            assert_eq!(drawn["drawer"]["title"], "A Quest");
        }
    }

    /// What it needs Joel for takes Doing's place rather than a seventh,
    /// since a waiting session's step would only say so again (VKCF3Z,
    /// 2026-11-14).
    #[test]
    fn what_it_needs_joel_for_takes_doings_place() {
        let mut presence = busy();
        presence["wants"] = json!({"said": "which way?"});
        let needs = |quest: &str| {
            assert_eq!(quest, "QEST-1", "asked for its own quest's needs");
            vec!["Sign In".to_owned()]
        };
        let known = Known {
            needs: &needs,
            ..known(None, None)
        };
        let drawn = drawer(Session(&record(&presence)), false, &words(), &known);
        let facts = facts(&drawn);
        assert!(
            facts.starts_with("Needs You For=which way? \u{b7} Sign In; Last Turn="),
            "{facts}"
        );
        assert!(
            !facts.contains("Doing") && !folded(&drawn).contains("Doing"),
            "it said its step as well: {facts}"
        );
    }

    /// Its last landing is folded whole, its title, its change and when,
    /// as the shell words it.
    #[test]
    fn its_last_landing_is_folded_with_when() {
        let presence = json!({"addresses": ["8IDU4E"], "session": "s-1", "landings": [
            {"what": "Serve rows", "ref": "core.nix#1", "at": "2026-11-10T20:44:00Z"}]});
        let drawn = drawer(
            Session(&record(&presence)),
            false,
            &words(),
            &known(None, None),
        );
        assert_eq!(facts(&drawn), "", "a landing in the glance");
        assert_eq!(
            folded(&drawn),
            "Session=s-1; Last Landing=Serve rows \u{b7} core.nix#1@clock; Machine=skiff"
        );
        assert_eq!(
            drawn["drawer"]["nerd"][1],
            json!({"label": "Last Landing", "value": "Serve rows \u{b7} core.nix#1",
                "at": {"at": 1_794_343_440_000_i64, "as": "clock"}}),
            "a folded bit is its name, its value and when, nothing drawn beside it"
        );
    }

    #[test]
    fn an_orchestrator_says_its_whole_orchestration_and_its_hours_on_its_own() {
        let presence = json!({"addresses": ["O"], "session": "s-o", "owns": ["QEST-O"]});
        let spend = json!({"sessions": {"s-o": "$3.10"},
            "families": {"QEST-O": {"all": "$14.20", "own": "$3.10", "under": "$11.10"}},
            "sessionsToday": {"s-o": {"graph": {"shape": "spark", "values": [1]}}}});
        let drawn = drawer(
            Session(&record(&presence)),
            true,
            &words(),
            &known(Some(&spend), None),
        );
        assert_eq!(
            facts(&drawn),
            "Spent Today=$14.20 in all; Its Sessions=$11.10"
        );
        assert_eq!(folded(&drawn), "Session=s-o; Its Own=$3.10; Machine=skiff");
        // Elsewhere, its sessions keep the sixth place, and its host is
        // folded where its machine was.
        let mut away = record(&presence);
        away["here"] = json!(false);
        let drawn = drawer(Session(&away), true, &words(), &known(Some(&spend), None));
        assert_eq!(
            facts(&drawn),
            "Spent Today=$14.20 in all; Its Sessions=$11.10"
        );
        assert_eq!(folded(&drawn), "Session=s-o; Its Own=$3.10; Machine=skiff");
    }

    #[test]
    fn it_says_what_its_councils_spent_and_which_groups_it_hears() {
        let presence = json!({"addresses": ["A"], "session": "s-a",
            "groups": ["#desk", "#quest/reef"]});
        let spend = json!({"sessions": {"s-a": "$12.50"},
            "helpers": {"s-a": [{"on": "councils", "spent": "$4.00"},
                {"on": "fleets", "spent": "$1.10"}]}});
        let drawn = drawer(
            Session(&record(&presence)),
            false,
            &words(),
            &known(Some(&spend), None),
        );
        assert_eq!(
            facts(&drawn),
            "Spent Today=$12.50 \u{b7} councils $4.00 \u{b7} fleets $1.10",
            "what its helpers spent is a part of its spend"
        );
        assert_eq!(
            folded(&drawn),
            "Session=s-a; Hears=#desk, #quest/reef; Machine=skiff",
            "what it hears is its wiring, folded"
        );
    }

    #[test]
    fn sharing_the_screen_says_no_amount() {
        let presence = json!({"addresses": ["O"], "session": "s-o", "owns": ["QEST-O"]});
        let spend = json!({"sessions": {"s-o": "$3.10"},
            "helpers": {"s-o": [{"on": "councils", "spent": "$4.00"}]},
            "families": {"QEST-O": {"all": "$14.20"}},
            "sessionsToday": {"s-o": {"rebuilt": "$1.20"}}});
        let budgets = json!({"sessions": {"s-o": "proxy"}, "budgets": [
            {"name": "proxy", "label": "Proxy", "standing": "$1 of $2"}]});
        let known = Known {
            sharing: true,
            ..known(Some(&spend), Some(&budgets))
        };
        let drawn = drawer(Session(&record(&presence)), true, &words(), &known);
        assert_eq!(facts(&drawn), "");
        assert_eq!(folded(&drawn), "Session=s-o; Machine=skiff");
    }

    #[test]
    fn a_quest_that_isnt_active_says_how_it_stands() {
        let presence = json!({"addresses": ["A"],
            "questInfo": {"id": "QEST-1", "title": "A Quest", "status": "paused"}});
        let drawn = drawer(
            Session(&record(&presence)),
            false,
            &words(),
            &known(None, None),
        );
        assert_eq!(folded(&drawn), "Quest=QEST-1 \u{b7} paused; Machine=skiff");
    }

    #[test]
    fn one_standing_at_the_top_says_what_its_for_in_its_own_sentence() {
        let for_it = "Tell it a thing once; it passes it to whoever owns it.";
        let presence = json!({"addresses": ["V"],
            "standsAtTop": {"name": "The Void", "what": for_it}});
        let drawn = drawer(
            Session(&record(&presence)),
            false,
            &words(),
            &known(None, None),
        );
        assert_eq!(
            drawn["drawer"]["body"], for_it,
            "its sentence, as its presence says it"
        );
        for presence in [
            json!({"addresses": ["V"], "standsAtTop": {"name": "The Void"}}),
            json!({"addresses": ["A"], "standsAtTop": {"what": for_it}}),
        ] {
            let drawn = drawer(
                Session(&record(&presence)),
                false,
                &words(),
                &known(None, None),
            );
            assert_eq!(
                drawn["drawer"]["body"], "",
                "none unsaid, nor of one not at the top"
            );
        }
    }

    #[test]
    fn its_eyebrow_says_its_address_where_it_works_and_since_when() {
        let presence = json!({"addresses": ["8IDU4E"], "repo": "core.nix",
            "cwd": "/src/elsewhere", "owner": {"startedAt": 1_794_352_932_300_i64}});
        let drawn = drawer(
            Session(&record(&presence)),
            false,
            &words(),
            &known(None, None),
        );
        assert_eq!(
            drawn["drawer"]["eyebrow"],
            json!([{"said": "8IDU4E", "keep": 3}, {"said": "core.nix", "keep": 2},
                {"at": {"at": 1_794_352_932_300_i64, "as": "since"}, "keep": 1}])
        );
        let bare = json!({"addresses": ["8IDU4E"], "cwd": "/src/reef"});
        let drawn = drawer(Session(&record(&bare)), false, &words(), &known(None, None));
        assert_eq!(
            drawn["drawer"]["eyebrow"],
            json!([{"said": "8IDU4E", "keep": 3}, {"said": "reef", "keep": 2}]),
            "with no repository its folder, and no time it hasn't said"
        );
    }

    #[test]
    fn a_step_says_its_tool_and_what_its_on_never_what_was_cut_short() {
        let steps: Vec<String> = [
            json!({"kind": "thinking"}),
            json!({"kind": "tool", "name": "bash", "detail": "cd ~/a; f\u{2026}"}),
            json!({"kind": "tool", "name": "quest", "detail": "journal"}),
            json!({"kind": "tool", "name": "read", "detail": "~/src/core.nix/flake.nix"}),
            json!({"kind": "tool", "name": "bash", "detail": "a command far past twenty-four"}),
        ]
        .iter()
        .map(|detail| step(Some(detail)))
        .collect();
        assert_eq!(
            steps,
            [
                "thinking",
                "bash",
                "quest journal",
                "read flake.nix",
                "bash"
            ]
        );
        assert_eq!(step(None), "", "no step says nothing");
    }

    #[test]
    fn what_it_works_on_leaves_out_the_title_already_said() {
        assert_eq!(
            about("A Quest: its summary", &["", "A Quest"]),
            "its summary"
        );
        assert_eq!(about("its summary", &["A Quest"]), "its summary");
    }

    #[test]
    fn todays_figures_are_said_only_on_their_day() {
        let presence = json!({"addresses": ["A"], "usage": {
            "day": "2026-11-09", "turns": {"today": 4}}});
        let drawn = drawer(
            Session(&record(&presence)),
            false,
            &words(),
            &known(None, None),
        );
        assert_eq!(
            folded(&drawn),
            "Machine=skiff",
            "yesterday's turns are not today's"
        );
    }
}
