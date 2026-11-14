//! What Joel is shown: one row per stop standing, per alert he hasn't
//! acted on, and per resume someone else asked him for, as `pineapple
//! serve` hands them to the launcher's Needs You, each with the presses
//! it offers (src/acts.rs takes them back).
//!
//! - A stop standing offers Resume, and Widen to everything unless it
//!   already reaches everything. Narrow isn't offered: one press can't
//!   say narrower to what, so Joel says the safe word again with words.
//! - An ask to stop without the phrase, a joke waved off, and the phrase
//!   without his tag each offer Stop, reaching what a stop said there
//!   would, and Dismiss. Those in one thread with one verdict are one
//!   row, counted, so a busy thread is one row and not many.
//! - Someone else's "pineapple resume" offers Approve, resuming what it
//!   reaches, and Dismiss.
//!
//! A row stays until it's acted on, its stop resumed, or its alert a
//! week old; an alert acted on comes back only for a saying heard since.
//! Each row says whether it's in shadow, which the server never serves
//! and only writes down (src/serve.rs).

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde_json::{Value, json};

use crate::resume::{self, Standing};
use crate::vocabulary::Words;

/// The kind of every record the server keeps.
pub const KIND: &str = "pineapple";

/// How long an alert is offered, in milliseconds: a week, so one heard
/// while Joel was away is still there when he's back.
const OFFERED_FOR: u64 = 7 * 24 * 60 * 60 * 1000;

/// What's kept on disk that the rows are made of.
#[derive(Debug, Default)]
pub struct Kept {
    /// The stops standing.
    pub stops: Vec<Standing>,
    /// Every decision's line, as the reader wrote it.
    pub decisions: Vec<Value>,
    /// Every press Joel made, as acts.rs wrote it.
    pub acted: Vec<Value>,
}

/// What's kept under the state folder `state`: stops/, decisions.jsonl
/// and acted.jsonl; a torn line is skipped, as everywhere else.
#[must_use]
pub fn kept(state: &Path) -> Kept {
    Kept {
        stops: resume::standing(&state.join("stops")),
        decisions: lines(&state.join("decisions.jsonl")),
        acted: lines(&state.join("acted.jsonl")),
    }
}

fn lines(path: &Path) -> Vec<Value> {
    fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect()
}

/// The rows of what's kept at `now` (milliseconds since the epoch), the
/// oldest first, each titled in `words`.
#[must_use]
pub fn rows(kept: &Kept, now: u64, words: &Words) -> Vec<Value> {
    let mut rows: Vec<Value> = kept
        .stops
        .iter()
        .map(|stop| stop_row(stop, &kept.decisions, words))
        .collect();
    rows.extend(alert_rows(kept, now, words));
    rows.sort_by_key(|row| row["since"].as_u64().unwrap_or(0));
    rows
}

fn text<'a>(value: &'a Value, field: &str) -> &'a str {
    value[field].as_str().unwrap_or_default()
}

fn strings(value: &Value, field: &str) -> Vec<String> {
    value[field]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect()
}

fn act(verb: &str, key: &str) -> Value {
    json!({"key": format!("verb:{KIND}:{verb}"), "ask": {"ask": verb, "key": key}})
}

fn who(by_joel: bool, words: &Words) -> String {
    words.fill(if by_joel { "you" } else { "someone" }, &[])
}

/// A title read as a sentence: the vocabulary says a word in lower case
/// wherever it may stand, so one opening a title takes its capital here.
fn sentence(text: &str) -> String {
    let mut letters = text.chars();
    letters.next().map_or_else(String::new, |first| {
        first.to_uppercase().chain(letters).collect()
    })
}

fn stop_row(stop: &Standing, decisions: &[Value], words: &Words) -> Value {
    let record = &stop.record;
    let key = format!("{KIND}:{}", stop.id);
    let reaches = strings(record, "reaches");
    let everything = reaches.iter().any(|reach| reach == "everything");
    let by_joel = record["by_joel"] == true;
    let fell_back = record["fell_back"] == true;
    let who = who(by_joel, words);
    let title = sentence(&if everything && fell_back {
        words.fill("stoppedAll", &[("who", &who)])
    } else {
        words.fill(
            "stopped",
            &[("reach", &said_reaches(&reaches, words)), ("who", &who)],
        )
    });
    let heard_at = record["heard_at"].as_u64().unwrap_or(0);
    // Said again in its thread since, so Joel's row counts the repeats
    // nobody was answered for.
    let thread = decisions
        .iter()
        .find(|line| line["key"] == stop.id.as_str())
        .map(|line| text(line, "thread"));
    let repeats = decisions
        .iter()
        .filter(|line| {
            thread.is_some_and(|thread| !thread.is_empty() && line["thread"] == thread)
                && line["verdict"] == "stop"
                && line["key"] != stop.id.as_str()
                && line["heard_at"].as_u64().unwrap_or(0) >= heard_at
        })
        .count();
    let mut acts = vec![act("resume", &key)];
    if !everything {
        acts.push(act("widen", &key));
    }
    json!({
        "key": key,
        "kind": KIND,
        "what": "stop",
        "title": title,
        "since": heard_at,
        "stop": stop.id,
        "saying": stop.id,
        "link": text(record, "where"),
        "words": text(record, "words"),
        "said_by": text(record, "said_by"),
        "by_joel": by_joel,
        "reaches": reaches,
        "fell_back": fell_back,
        "count": repeats + 1,
        "shadow": record["shadow"] != false,
        "acts": acts,
    })
}

/// What a verdict Joel is alerted to is named in a row, and its title's
/// word; none for a verdict that isn't one.
fn alerted(verdict: &str) -> Option<(&'static str, &'static str)> {
    match verdict {
        "stop-asked" => Some(("asked", "asked")),
        "waved:joke" => Some(("joke", "joke")),
        "waved:no-tag" => Some(("no-tag", "noTag")),
        "resume-asked" => Some(("resume", "resumeAsked")),
        _ => None,
    }
}

fn alert_rows(kept: &Kept, now: u64, words: &Words) -> Vec<Value> {
    let last_act = |key: &str| {
        kept.acted
            .iter()
            .filter(|line| line["key"] == key)
            .filter_map(|line| line["at"].as_u64())
            .max()
    };
    let mut groups: BTreeMap<String, Vec<&Value>> = BTreeMap::new();
    for line in &kept.decisions {
        let Some((what, _)) = alerted(text(line, "verdict")) else {
            continue;
        };
        let heard_at = line["heard_at"].as_u64().unwrap_or(0);
        if line["by_joel"] == true || heard_at.saturating_add(OFFERED_FOR) < now {
            continue;
        }
        let thread = match text(line, "thread") {
            "" => text(line, "key"),
            thread => thread,
        };
        groups
            .entry(format!("{KIND}:{what}:{thread}"))
            .or_default()
            .push(line);
    }
    groups
        .into_iter()
        .filter_map(|(key, mut said)| {
            if let Some(acted) = last_act(&key) {
                said.retain(|line| line["heard_at"].as_u64().unwrap_or(0) > acted);
            }
            said.sort_by_key(|line| line["heard_at"].as_u64().unwrap_or(0));
            let latest = *said.last()?;
            Some(alert_row(&key, latest, said.len(), words))
        })
        .collect()
}

fn alert_row(key: &str, latest: &Value, count: usize, words: &Words) -> Value {
    let (what, word) = alerted(text(latest, "verdict")).unwrap_or_default();
    let resuming = what == "resume";
    // A resume asked for names what it resumes; anything else names what
    // a stop said there would have reached, for Joel's Stop.
    let (reaches, fell_back) = if resuming {
        (strings(latest, "reaches"), latest["fell_back"] == true)
    } else {
        (
            strings(latest, "would_reach"),
            latest["would_fall_back"] == true,
        )
    };
    let title = sentence(&words.fill(
        word,
        &[
            ("who", &who(false, words)),
            ("reach", &said_reaches(&reaches, words)),
        ],
    ));
    let acts = if resuming {
        vec![act("approve", key), act("dismiss", key)]
    } else {
        vec![act("stop", key), act("dismiss", key)]
    };
    json!({
        "key": key,
        "kind": KIND,
        "what": what,
        "title": title,
        "since": latest["heard_at"].as_u64().unwrap_or(0),
        "saying": text(latest, "key"),
        "link": text(latest, "link"),
        "words": text(latest, "text"),
        "said_by": text(latest, "author"),
        "said_at": text(latest, "said_at"),
        "by_joel": false,
        "reaches": reaches,
        "fell_back": fell_back,
        "count": count,
        "shadow": latest["shadow"] != false,
        "acts": acts,
    })
}

/// What stops reach, said as the launcher's words say each, and joined.
#[must_use]
pub fn said_reaches(reaches: &[String], words: &Words) -> String {
    let said: Vec<String> = reaches
        .iter()
        .map(|reach| said_reach(reach, words))
        .collect();
    said.join(", ")
}

/// One reach, as `pr:owner/name#12` or `quest:ID`, in the launcher's
/// words; one of a kind it doesn't know is said as it's written.
#[must_use]
pub fn said_reach(reach: &str, words: &Words) -> String {
    let (kind, rest) = reach.split_once(':').unwrap_or((reach, ""));
    let said = match kind {
        "everything" => Some(words.fill("reach:everything", &[])),
        "pr" => rest
            .rsplit_once('#')
            .map(|(repo, number)| words.fill("reach:pr", &[("repo", repo), ("number", number)])),
        "branch" => rest.split_once('@').map(|(repo, branch)| {
            words.fill("reach:branch", &[("repo", repo), ("branch", branch)])
        }),
        "repo" => Some(words.fill("reach:repo", &[("repo", rest)])),
        "quest" => Some(words.fill("reach:quest", &[("quest", rest)])),
        "realm" => Some(words.fill("reach:realm", &[("realm", rest)])),
        "named" => Some(words.fill("reach:named", &[("name", rest)])),
        _ => None,
    };
    said.filter(|said| !said.is_empty())
        .unwrap_or_else(|| reach.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words() -> Words {
        Words::of(&[
            ("you", "you"),
            ("someone", "someone"),
            ("stopped", "{who} stopped agents on {reach}"),
            ("stoppedAll", "{who} stopped all agents"),
            ("asked", "{who} asked for a stop"),
            ("joke", "{who} joked, on {reach}"),
            ("noTag", "{who} didn't tag you"),
            ("resumeAsked", "{who} asked to resume {reach}"),
            ("reach:everything", "everything"),
            ("reach:pr", "pull request {repo}#{number}"),
            ("reach:repo", "the {repo} repository"),
            ("reach:branch", "branch {branch} of {repo}"),
            ("reach:quest", "quest {quest}"),
            ("reach:realm", "the {realm} realm"),
            ("reach:named", "{name}"),
        ])
    }

    fn standing(id: &str, reaches: &[&str], fell_back: bool) -> Standing {
        Standing {
            id: id.to_owned(),
            file: id.into(),
            record: json!({"id": id, "shadow": false, "reaches": reaches,
                "fell_back": fell_back, "heard_at": 10, "said_by": "U0OTHER001",
                "where": "https://example.slack.com/archives/C1/p1", "words": "pineapple override",
                "by_joel": false, "resumed": null}),
        }
    }

    fn line(key: &str, thread: &str, verdict: &str, heard_at: u64) -> Value {
        json!({"key": key, "thread": thread, "verdict": verdict, "heard_at": heard_at,
            "by_joel": false, "shadow": false, "text": "pineapple override jk",
            "link": "https://example.slack.com/archives/C1/p2", "author": "U0OTHER001",
            "reaches": [], "would_reach": ["pr:example/shop#12"], "would_fall_back": false})
    }

    fn verbs(row: &Value) -> Vec<&str> {
        row["acts"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|one| one["ask"]["ask"].as_str())
            .collect()
    }

    #[test]
    fn a_stop_offers_resume_and_widen_and_counts_its_repeats() {
        let kept = Kept {
            stops: vec![standing("slack:C1:1.0", &["pr:example/shop#12"], false)],
            decisions: vec![
                line("slack:C1:1.0", "slack:C1:0.5", "stop", 10),
                line("slack:C1:2.0", "slack:C1:0.5", "stop", 20),
            ],
            acted: Vec::new(),
        };
        let rows = rows(&kept, 30, &words());
        assert_eq!(rows.len(), 1, "one row: the repeat isn't a stop of its own");
        let row = &rows[0];
        assert_eq!(row["key"], "pineapple:slack:C1:1.0", "keyed by its stop");
        assert_eq!(
            row["title"], "Someone stopped agents on pull request example/shop#12",
            "titled in the launcher's words, with no ids"
        );
        assert_eq!(row["count"], 2, "said twice in its thread");
        assert_eq!(verbs(row), ["resume", "widen"], "resume and widen");
        assert_eq!(row["shadow"], false, "a live stop is served");
    }

    #[test]
    fn a_stop_reaching_everything_isnt_widened() {
        let kept = Kept {
            stops: vec![standing("s", &["everything"], true)],
            ..Kept::default()
        };
        let row = &rows(&kept, 30, &words())[0];
        assert_eq!(
            row["title"], "Someone stopped all agents",
            "said as all, since nothing narrower was named"
        );
        assert_eq!(verbs(row), ["resume"], "nothing to widen to");
    }

    #[test]
    fn alerts_in_one_thread_are_one_row_until_acted_on() {
        let mut kept = Kept {
            decisions: vec![
                line("slack:C1:2.0", "slack:C1:0.5", "waved:joke", 20),
                line("slack:C1:3.0", "slack:C1:0.5", "waved:joke", 30),
                line("slack:C1:4.0", "slack:C2:0.5", "nothing", 30),
            ],
            ..Kept::default()
        };
        let rows_now = rows(&kept, 40, &words());
        assert_eq!(rows_now.len(), 1, "two jokes in one thread are one row");
        let row = &rows_now[0];
        assert_eq!(
            row["key"], "pineapple:joke:slack:C1:0.5",
            "keyed by its thread"
        );
        assert_eq!(row["count"], 2, "counting both");
        assert_eq!(row["saying"], "slack:C1:3.0", "naming the latest");
        assert_eq!(
            row["title"], "Someone joked, on pull request example/shop#12",
            "saying what a stop would have reached"
        );
        assert_eq!(verbs(row), ["stop", "dismiss"], "stop and dismiss");
        kept.acted
            .push(json!({"at": 35, "act": "dismiss", "key": row["key"]}));
        assert!(rows(&kept, 40, &words()).is_empty(), "gone once dismissed");
        kept.decisions
            .push(line("slack:C1:5.0", "slack:C1:0.5", "waved:joke", 50));
        let again = rows(&kept, 60, &words());
        assert_eq!(again[0]["count"], 1, "back for a joke heard since, alone");
    }

    #[test]
    fn an_alert_a_week_old_or_said_by_joel_is_left_out() {
        let mut joel = line("a", "t", "stop-asked", 1_000);
        joel["by_joel"] = json!(true);
        let kept = Kept {
            decisions: vec![line("b", "t2", "stop-asked", 1_000), joel],
            ..Kept::default()
        };
        assert_eq!(
            rows(&kept, 2_000, &words()).len(),
            1,
            "a fresh one is offered"
        );
        assert!(
            rows(&kept, 1_000 + OFFERED_FOR + 1, &words()).is_empty(),
            "not past a week"
        );
    }

    #[test]
    fn someone_elses_resume_offers_approve() {
        let mut asked = line("r", "t", "resume-asked", 5);
        asked["reaches"] = json!(["quest:QEST-A"]);
        let kept = Kept {
            decisions: vec![asked],
            ..Kept::default()
        };
        let row = &rows(&kept, 10, &words())[0];
        assert_eq!(
            row["title"], "Someone asked to resume quest QEST-A",
            "what it resumes"
        );
        assert_eq!(verbs(row), ["approve", "dismiss"], "approve and dismiss");
    }

    #[test]
    fn each_reach_is_said_in_words_and_an_unknown_one_as_written() {
        let said = |reach: &str| said_reach(reach, &words());
        assert_eq!(
            said("branch:example/shop@main"),
            "branch main of example/shop",
            "a branch"
        );
        assert_eq!(
            said("repo:example/shop"),
            "the example/shop repository",
            "a repo"
        );
        assert_eq!(said("realm:work"), "the work realm", "a realm");
        assert_eq!(said("odd:thing"), "odd:thing", "an unknown kind as written");
        assert_eq!(
            said_reach("everything", &Words::default()),
            "everything",
            "a word the vocabulary lacks is said as written"
        );
    }
}
