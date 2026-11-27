//! For You's History of looks as ready rows (Y2YBTS's ready-rows.md): each
//! look done with, opened, cleared, resolved, expired or gone quiet, as the
//! row History lists it, newest done first, so a fresh shell's first step
//! into History lists rows already made rather than working out each of
//! Joel's ~470 looks, 114 to 117 ms against the 50 ms bar (RCTRF8,
//! 2026-11-14). Each is what Now.qml's `lookRow` and `lookDrawer`, and
//! Looks.js and ForYou.js beneath them, made of a look done with: its
//! name, side, facts and act, in the same words, since Joel learns one
//! History, not two.
//!
//! Left to the shell, which alone knows them: a terminal look's Go, to its
//! session; Talk, Go To and Explain It on the cell behind a look, found by
//! the `cell` its row holds; the drawer's eyebrow, its quest's title and
//! how it stands; an answer the node took, which moves a look still
//! waiting in its file to History at once; and settling a quiet look Joel
//! opens, as `held.state` says one is.
//!
//! A look waiting past its time is History's by the clock alone, its file
//! unchanged: expired at its `expiresAt`, or gone quiet three days after it
//! came, as Looks.js's `quieted` says. Each is made as the clock stands
//! when the rows are made, and [`History::due`] says when the next look
//! will be, so the daemon wakes then.

use std::cmp::Reverse;

use ready_rows::kept::Kept;
use serde_json::{Value, json};

use crate::words::Words;

/// The place History's looks are in.
pub const PLACE: &str = "looks-history";

/// The record beside History's rows counting them.
pub const SUMMARY: &str = "summary:looks-history";

/// What a look is about and how its file says it stands, as Looks.js
/// reads them, and the one version of its file read.
const KINDS: [&str; 5] = ["pr", "page", "file", "diff", "terminal"];
const STATES: [&str; 5] = ["waiting", "opened", "cleared", "resolved", "expired"];
const VERSION: u64 = 1;

/// How long a look waits before it goes quiet, as Looks.js's
/// `QUIET_AFTER`: by then why it came is lost (Joel, 2026-11-11).
const QUIET_AFTER: i64 = 3 * 24 * 60 * 60 * 1000;

/// The kinds whose drawer says what they point at; a file's and a
/// terminal's say it in their name.
const POINTED: [&str; 3] = ["pr", "page", "diff"];

/// History's rows as last made, and when they're next to be made again.
#[derive(Debug, Default)]
pub struct History {
    kept: Kept,
    stale: bool,
    made: bool,
    due: Option<i64>,
}

impl History {
    /// Marks the rows to be made again, as the looks were read again.
    pub fn looks_changed(&mut self) {
        self.stale = true;
    }

    /// Every record History serves at `now`, in milliseconds: its rows,
    /// made again only where the looks changed or one's time has come,
    /// and its count.
    pub fn records(&mut self, looks: &[Value], words: &Words, now: i64) -> Vec<Value> {
        if !self.made || self.stale || self.due.is_some_and(|due| now >= due) {
            let (rows, due) = made(looks, words, now);
            let summary = json!({ "key": SUMMARY, "kind": "summary", "count": rows.len() });
            self.kept.take(rows, vec![summary]);
            self.due = due;
            self.stale = false;
            self.made = true;
        }
        self.kept.all().cloned().collect()
    }

    /// When, in seconds, the next look waiting goes to History by the
    /// clock, if one will.
    pub fn due(&self) -> Option<i64> {
        self.due.map(|ms| (ms + 999).div_euclid(1000))
    }
}

/// A look done with, as History lists it: its file, how it stands by
/// the clock, when it came and when it was done with.
struct Done<'a> {
    look: &'a Value,
    state: &'a str,
    since: Option<i64>,
    at: Option<i64>,
}

/// History's rows at `now`, newest done first, without their `order`,
/// and when the next look waiting will join them, in milliseconds.
fn made(looks: &[Value], words: &Words, now: i64) -> (Vec<Value>, Option<i64>) {
    let mut due: Option<i64> = None;
    let mut done = Vec::new();
    for look in looks.iter().filter(|look| whole(look)) {
        let since = when(&look["at"]);
        let (state, at) = standing(look, since, now);
        if state == "waiting" {
            let expires = when(&look["expiresAt"]);
            let quiet = since.map(|since| since + QUIET_AFTER + 1);
            due = [due, expires, quiet].into_iter().flatten().min();
        } else {
            done.push(Done {
                look,
                state,
                since,
                at,
            });
        }
    }
    // Newest done first, as Order.js's `stream` puts them, those of no
    // time last; a sort that keeps their files' order between equals.
    done.sort_by_key(|one| Reverse(one.at));
    (done.iter().map(|one| row(one, words)).collect(), due)
}

/// Whether a look's file is one read whole, as Looks.js's `whole` says:
/// version 1, with what it's about and how it stands.
fn whole(look: &Value) -> bool {
    let named = |field: &str| look[field].as_str().is_some_and(|text| !text.is_empty());
    let one_of = |field: &str, among: &[&str]| {
        look[field]
            .as_str()
            .is_some_and(|text| among.contains(&text))
    };
    look["v"].as_u64() == Some(VERSION)
        && named("id")
        && named("title")
        && named("target")
        && one_of("kind", &KINDS)
        && one_of("state", &STATES)
}

/// A time as milliseconds, from a number or an ISO time, as ForYou.js's
/// `when` reads it, or none.
fn when(at: &Value) -> Option<i64> {
    match at {
        Value::Number(number) => number.as_i64(),
        Value::String(text) => text
            .parse::<jiff::Timestamp>()
            .ok()
            .map(jiff::Timestamp::as_millisecond),
        _ => None,
    }
}

/// How a look stands at `now`, and when it was done with, as Looks.js's
/// `quieted` says: as its file says, but for one waiting past its expiry,
/// expired then, or waiting three days, gone quiet then.
fn standing(look: &Value, since: Option<i64>, now: i64) -> (&str, Option<i64>) {
    let state = look["state"].as_str().unwrap_or_default();
    let resolved = when(&look["resolvedAt"]);
    if state != "waiting" {
        return (state, resolved.or(since));
    }
    if let Some(expires) = when(&look["expiresAt"])
        && now >= expires
    {
        return ("expired", Some(expires));
    }
    match since {
        Some(since) if now - since > QUIET_AFTER => ("quiet", Some(since + QUIET_AFTER)),
        _ => (state, resolved.or(since)),
    }
}

/// The theme's name for a look's mark, by its kind, as ForYou.js's
/// `glyphOf`.
fn glyph(kind: &str) -> &'static str {
    match kind {
        "pr" => "branch",
        "page" => "web",
        "diff" => "lines",
        "terminal" => "prompt",
        _ => "path",
    }
}

/// What a look points at, short enough for one line of its drawer, as
/// ForYou.js's `pointsAt` says it: a change as "core.nix#4368 review", a
/// page as its site and last step, a file as its name; none for nothing.
fn points_at(kind: &str, target: &str, words: &Words) -> String {
    let named = change(target).unwrap_or_else(|| shortly(target));
    if named.is_empty() {
        return named;
    }
    match kind {
        "pr" => words.said("review", &[("at", &named)]),
        "diff" => words.said("diff", &[("at", &named)]),
        _ => named,
    }
}

/// What follows an address's scheme, `<scheme>://`, its scheme in
/// lower-case letters, or none where it has none.
fn after_scheme(at: &str) -> Option<&str> {
    let (scheme, rest) = at.split_once("://")?;
    (!scheme.is_empty() && scheme.bytes().all(|byte| byte.is_ascii_lowercase())).then_some(rest)
}

/// A change's repository and number, "core.nix#4368", from its address,
/// as ForYou.js's `CHANGE` finds them, or none where it isn't one.
fn change(at: &str) -> Option<String> {
    let mut steps = after_scheme(at)?.split('/');
    let mut step = || steps.next().filter(|step| !step.is_empty());
    let (_host, _owner, repository) = (step()?, step()?, step()?);
    (step()? == "pull").then_some(())?;
    let number: String = step()?.chars().take_while(char::is_ascii_digit).collect();
    (!number.is_empty()).then(|| format!("{repository}#{number}"))
}

/// An address as its site and last step, as ForYou.js's `shortly`, or a
/// path's last step where it isn't an address.
fn shortly(at: &str) -> String {
    let Some(rest) = after_scheme(at) else {
        return last_step(at).to_owned();
    };
    let host_ends = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let (host, path) = rest.split_at(host_ends);
    if host.is_empty() {
        return last_step(at).to_owned();
    }
    let path = &path[..path.find(['?', '#']).unwrap_or(path.len())];
    match last_step(path) {
        "" => host.to_owned(),
        step => format!("{host}/{step}"),
    }
}

/// A path's last step, or none.
fn last_step(path: &str) -> &str {
    path.split('/')
        .rfind(|step| !step.is_empty())
        .unwrap_or_default()
}

/// How and when a look was done with, as History's side says it, "opened
/// 14:02", as ForYou.js's `historySide`: its own how, or its state's word,
/// the time left for the shell's clock.
fn side(done: &Done, words: &Words) -> Value {
    let how = match done.look["how"].as_str() {
        Some(how) if !how.is_empty() => how.to_owned(),
        _ => words.phrase(done.state),
    };
    let said = words.said("history", &[("how", &how)]);
    match done.at {
        Some(at) => json!({ "said": said, "at": at, "as": "moment" }),
        None => json!({ "said": said.replace("{time}", "").trim() }),
    }
}

/// What a look's drawer leads with, to act on hours on (Joel, 2026-11-11),
/// as For You's drawer says it (LookForm.js's `wordsOf`): in its second
/// form what happened then why it matters, each ending as a sentence ends;
/// else its why, then its ask. Each is trimmed, so words only of spaces
/// leave no blank. A second-form look carries a why and an ask its harness
/// made from it too, which For You leaves unsaid, so History does as well.
fn words_of(look: &Value) -> String {
    let text = |field: &str| trimmed(look[field].as_str().unwrap_or_default());
    if text("happened").is_empty() {
        [text("why"), text("ask")]
            .into_iter()
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join("\n\n")
    } else {
        [text("happened"), text("matters")]
            .into_iter()
            .filter(|part| !part.is_empty())
            .map(ended)
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// The byte-order mark, which JavaScript's `trim` takes as a space.
const BYTE_ORDER_MARK: char = '\u{feff}';

/// `part` trimmed as For You trims it, with `trim` in Qt's JavaScript
/// engine: of what `QChar::isSpace` takes, Unicode's spaces and line ends
/// with the next-line control among them, and the byte-order mark, which
/// Rust's own `trim` keeps, so a look's words read the same in both.
fn trimmed(part: &str) -> &str {
    part.trim_matches(|c: char| c.is_whitespace() || c == BYTE_ORDER_MARK)
}

/// `part` ending as a sentence ends, as LookForm.js's `ENDED` reads one: a
/// stop, a bang, a question mark or an ellipsis, past any closing quotes
/// or brackets; else with a stop added.
fn ended(part: &str) -> String {
    let unclosed = part.trim_end_matches(['"', '\'', '\u{2019}', '\u{201d}', ')', ']']);
    if unclosed.ends_with(['.', '!', '?', '\u{2026}']) {
        part.to_owned()
    } else {
        format!("{part}.")
    }
}

/// A look's History row, not yet given its `order`.
fn row(done: &Done, words: &Words) -> Value {
    let look = done.look;
    let text = |field: &str| look[field].as_str().unwrap_or_default();
    let (id, kind, target) = (text("id"), text("kind"), text("target"));
    let side = side(done, words);
    let pointed = if POINTED.contains(&kind) { target } else { "" };
    let points = points_at(kind, pointed, words);
    let mut resolved = side.clone();
    resolved["name"] = json!(words.fact("resolved"));
    let facts: Vec<Value> = done
        .since
        .map(|since| json!({ "name": words.fact("since"), "at": since, "as": "moment" }))
        .into_iter()
        .chain(std::iter::once(resolved))
        .chain(
            (!points.is_empty()).then(|| json!({ "name": words.fact("target"), "said": points })),
        )
        .collect();
    // Its whole address and its id in the nerdy bits, where y copies them
    // (Y2YBTS, 2026-11-12).
    let nerd: Vec<Value> = [("target", pointed), ("look", id)]
        .into_iter()
        .filter(|(_, value)| !value.is_empty())
        .map(|(fact, value)| json!({ "label": words.fact(fact), "value": value }))
        .collect();
    let body = words_of(look);
    // A terminal's Go is to its session, which only the shell knows.
    let acts = if kind == "terminal" {
        json!([])
    } else {
        json!([{ "key": "verb:on:look:open", "open": target }])
    };
    // Its cell, so the shell offers Talk and Go To on the session behind
    // it from History as from what waits (Now.qml's cellActs).
    let held = json!({
        "id": id, "kind": kind, "target": target, "title": text("title"),
        "session": look["session"], "address": look["address"], "quest": look["quest"],
        "cell": look["cell"],
        "state": done.state, "how": look["how"], "at": look["at"],
        "resolvedAt": look["resolvedAt"], "since": done.since, "doneAt": done.at,
    });
    json!({
        "key": format!("row:{PLACE}:{id}"), "thing": format!("look:{id}"), "place": PLACE,
        "kind": "look", "name": text("title"), "note": trimmed(text("why")), "glyph": glyph(kind),
        "hue": "muted", "side": [side],
        "drawer": {
            "title": text("title"), "facts": facts, "nerd": nerd, "body": body, "prose": true,
        },
        "acts": acts, "held": held, "forYou": { "kind": "look", "source": "looks" },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// For You's words as the launcher has them, those a look needs.
    fn words() -> Words {
        let vocabulary =
            serde_json::from_str(include_str!("../tests/words.json")).expect("the words parse");
        Words::read(&vocabulary).expect("every word a look needs is there")
    }

    const HOUR: i64 = 60 * 60 * 1000;
    const NOW: i64 = 1_794_600_000_000;

    #[test]
    fn a_look_done_with_is_historys_row_as_the_shell_made_it() {
        let look = json!({"v": 1, "id": "b1", "kind": "pr", "title": "Fix the drawer",
            "state": "opened", "target": "https://github.com/Jitsusama/core.nix/pull/4368",
            "why": "It flickers.", "ask": "Merge it?", "session": "s1", "address": "AB12CD",
            "quest": "QEST-1", "cell": "CE11XX", "at": NOW - 3 * HOUR,
            "resolvedAt": NOW - 2 * HOUR});
        let (rows, due) = made(&[look], &words(), NOW);
        assert_eq!(due, None, "nothing waits");
        assert_eq!(
            rows,
            [json!({
                "key": "row:looks-history:b1", "thing": "look:b1", "place": "looks-history",
                "kind": "look", "name": "Fix the drawer", "note": "It flickers.",
                "glyph": "branch", "hue": "muted",
                "side": [{"said": "opened {time}", "at": NOW - 2 * HOUR, "as": "moment"}],
                "drawer": {
                    "title": "Fix the drawer",
                    "facts": [
                        {"name": "Waiting Since", "at": NOW - 3 * HOUR, "as": "moment"},
                        {"name": "Resolved", "said": "opened {time}", "at": NOW - 2 * HOUR,
                            "as": "moment"},
                        {"name": "Target", "said": "core.nix#4368 review"},
                    ],
                    "nerd": [
                        {"label": "Target",
                            "value": "https://github.com/Jitsusama/core.nix/pull/4368"},
                        {"label": "Look", "value": "b1"},
                    ],
                    "body": "It flickers.\n\nMerge it?", "prose": true,
                },
                "acts": [{"key": "verb:on:look:open",
                    "open": "https://github.com/Jitsusama/core.nix/pull/4368"}],
                "held": {
                    "id": "b1", "kind": "pr", "title": "Fix the drawer",
                    "target": "https://github.com/Jitsusama/core.nix/pull/4368",
                    "session": "s1", "address": "AB12CD", "quest": "QEST-1",
                    "cell": "CE11XX", "state": "opened", "how": null, "at": NOW - 3 * HOUR,
                    "resolvedAt": NOW - 2 * HOUR, "since": NOW - 3 * HOUR,
                    "doneAt": NOW - 2 * HOUR,
                },
                "forYou": {"kind": "look", "source": "looks"},
            })],
            "every field the shell's row and drawer made"
        );
    }

    #[test]
    fn a_second_form_look_s_drawer_leads_with_what_happened_as_for_you_s_does() {
        let drawn = |happened: &str, matters: &str| {
            let look = json!({"v": 1, "id": "b2", "kind": "pr", "title": "Fix the build",
                "state": "opened", "target": "https://github.com/Jitsusama/core.nix/pull/4369",
                "happened": happened, "matters": matters,
                "why": "Made by its harness.", "ask": "Also made.",
                "at": NOW - 3 * HOUR, "resolvedAt": NOW - 2 * HOUR});
            let (rows, _) = made(&[look], &words(), NOW);
            rows[0]["drawer"]["body"].clone()
        };
        let cases = [
            (
                ("The build broke", " Nothing merges until it's fixed. "),
                "The build broke. Nothing merges until it's fixed.",
            ),
            ((" It flickers! ", "   "), "It flickers!"),
            (
                ("He said \"go.\"", "(It's late.)"),
                "He said \"go.\" (It's late.)",
            ),
            (("Waiting\u{2026}", "Soon"), "Waiting\u{2026} Soon."),
            (("   ", "It matters."), "Made by its harness.\n\nAlso made."),
        ];
        for ((happened, matters), body) in cases {
            assert_eq!(
                drawn(happened, matters),
                json!(body),
                "a look that happened {happened:?} and matters {matters:?} wasn't said as For You's"
            );
        }
    }

    #[test]
    fn a_look_s_words_are_trimmed_as_for_you_s_drawer_trims_them() {
        let drawn = |why: &str, ask: &str| {
            let look = json!({"v": 1, "id": "b1", "kind": "pr", "title": "Fix the drawer",
                "state": "opened", "target": "https://github.com/Jitsusama/core.nix/pull/4368",
                "why": why, "ask": ask, "at": NOW - 3 * HOUR, "resolvedAt": NOW - 2 * HOUR});
            let (rows, _) = made(&[look], &words(), NOW);
            (rows[0]["drawer"]["body"].clone(), rows[0]["note"].clone())
        };
        let cases = [
            (
                ("It flickers.", " Merge it? "),
                ("It flickers.\n\nMerge it?", "It flickers."),
            ),
            (
                (" It flickers. ", "Merge it?"),
                ("It flickers.\n\nMerge it?", "It flickers."),
            ),
            (("It flickers.", "   "), ("It flickers.", "It flickers.")),
            (("\u{feff}", "Merge it?"), ("Merge it?", "")),
            (("It flickers.", "\u{85}"), ("It flickers.", "It flickers.")),
            (("   ", "Merge it?"), ("Merge it?", "")),
            ((" ", " "), ("", "")),
        ];
        for ((why, ask), (body, note)) in cases {
            assert_eq!(
                drawn(why, ask),
                (json!(body), json!(note)),
                "a why of {why:?} and an ask of {ask:?} weren't trimmed as For You's"
            );
        }
    }

    #[test]
    fn a_look_waiting_past_its_time_is_historys_by_the_clock_and_says_when_next() {
        let waiting = |id: &str, at: i64, expires: Option<i64>| {
            let mut look = json!({"v": 1, "id": id, "kind": "page", "title": id,
                "state": "waiting", "target": "https://example.com/a", "at": at});
            if let Some(expires) = expires {
                look["expiresAt"] = json!(expires);
            }
            look
        };
        let looks = [
            waiting("quiet", NOW - 96 * HOUR, None),
            waiting("expired", NOW - HOUR, Some(NOW - 1)),
            waiting("expiring", NOW - HOUR, Some(NOW + HOUR)),
            waiting("quieting", NOW - 71 * HOUR, None),
        ];
        let (rows, due) = made(&looks, &words(), NOW);
        let said: Vec<(&Value, &Value)> = rows
            .iter()
            .map(|row| (&row["key"], &row["side"][0]["said"]))
            .collect();
        assert_eq!(
            said,
            [
                (
                    &json!("row:looks-history:expired"),
                    &json!("expired {time}")
                ),
                (
                    &json!("row:looks-history:quiet"),
                    &json!("went quiet {time}")
                ),
            ],
            "expired at its expiry, quiet three days after it came, newest first"
        );
        assert_eq!(due, Some(NOW + HOUR), "woken as the next one's time comes");
        let (_, quieting) = made(&looks[3..], &words(), NOW);
        assert_eq!(
            quieting,
            Some(NOW + HOUR + 1),
            "woken just past three days, as quiet is past them"
        );
    }

    #[test]
    fn a_terminal_look_gets_no_open_and_only_whole_looks_are_listed() {
        let looks = [
            json!({"v": 1, "id": "t1", "kind": "terminal", "title": "Answer",
                "state": "cleared", "target": "pi:A1", "how": "cleared from For You",
                "at": NOW - HOUR}),
            json!({"v": 2, "id": "v2", "kind": "page", "title": "Later", "state": "opened",
                "target": "https://example.com/c", "at": NOW}),
            json!({"v": 1, "id": "n1", "kind": "page", "title": "", "state": "opened",
                "target": "https://example.com/c", "at": NOW}),
            json!({"v": 1, "id": "k1", "kind": "slide", "title": "Odd", "state": "opened",
                "target": "https://example.com/c", "at": NOW}),
        ];
        let (rows, _) = made(&looks, &words(), NOW);
        assert_eq!(rows.len(), 1, "only the whole look: {rows:?}");
        assert_eq!(
            (&rows[0]["acts"], &rows[0]["glyph"], &rows[0]["side"][0]),
            (
                &json!([]),
                &json!("prompt"),
                &json!({"said": "cleared from For You {time}", "at": NOW - HOUR,
                    "as": "moment"})
            ),
            "its Go is the shell's; it says its own how, done when it came"
        );
        assert_eq!(
            rows[0]["drawer"]["nerd"],
            json!([{"label": "Look", "value": "t1"}]),
            "a terminal points at nothing a drawer says"
        );
    }

    #[test]
    fn what_a_look_points_at_is_said_short() {
        let words = words();
        let cases = [
            (
                "pr",
                "https://github.com/Shop/world/pull/2396409/files",
                "world#2396409 review",
            ),
            ("diff", "https://github.com/a/b/pull/7", "b#7 diff"),
            (
                "page",
                "https://example.com/docs/guide/?q=1#top",
                "example.com/guide",
            ),
            ("page", "https://example.com", "example.com"),
            ("page", "notes/today.md", "today.md"),
            ("page", "", ""),
        ];
        for (kind, target, said) in cases {
            assert_eq!(points_at(kind, target, &words), said, "{kind} {target}");
        }
    }

    #[test]
    fn rows_are_made_again_only_as_the_looks_change_or_ones_time_comes() {
        let words = words();
        let look = json!({"v": 1, "id": "e1", "kind": "page", "title": "Soon",
            "state": "waiting", "target": "https://example.com/b", "at": NOW,
            "expiresAt": NOW + HOUR});
        let mut history = History::default();
        let count = |records: &[Value]| {
            records
                .iter()
                .find(|record| record["key"] == SUMMARY)
                .map(|record| record["count"].clone())
        };
        let first = history.records(std::slice::from_ref(&look), &words, NOW);
        assert_eq!(count(&first), Some(json!(0)), "nothing done with yet");
        assert_eq!(history.due(), Some((NOW + HOUR) / 1000), "due in seconds");
        let later = history.records(&[], &words, NOW + 1);
        assert_eq!(count(&later), Some(json!(0)), "unchanged looks aren't read");
        let expired = history.records(std::slice::from_ref(&look), &words, NOW + HOUR);
        assert_eq!(
            count(&expired),
            Some(json!(1)),
            "made again as its time came"
        );
        history.looks_changed();
        let gone = history.records(&[], &words, NOW + HOUR);
        assert_eq!(
            count(&gone),
            Some(json!(0)),
            "made again as the looks changed"
        );
    }
}
