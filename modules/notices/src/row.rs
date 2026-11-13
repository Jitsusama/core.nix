//! Each notice's rows, in the launcher's one form for every daemon's rows
//! (Y2YBTS's ready-rows.md), made here from the record its card reads, so
//! cards and rows come off one record and the shell sorts, words and
//! builds nothing (Y2YBTS, 2026-11-11). They're what the shell's
//! PastNotices.qml and Notice.js made: the same name, drawer and acts, in
//! the same order, since Joel learns one notice, not two.
//!
//! Each list says a notice's standing its own way, so each is a place of
//! its own, ordered newest first (Y2YBTS, 1dad3527):
//!
//! - `notices`, the lists of read and unread together (a search, a
//!   program's), where an unread one says so before its age.
//! - `notices-desktop`, For You's Desktop, the unread, its age alone,
//!   since Desktop's head says they're unread; one read while Joel looks
//!   stays in place saying "read", until he no longer does (VKCF3Z,
//!   2026-11-09).
//! - `notices-history`, For You's History, the read, its age alone.
//!
//! Long because a row, its side and its drawer are one notice's one
//! reading, which change together, and its cases, nearly half of it, show
//! each list's rows beside the code making them.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Value, json};

use crate::body::{self, Body};
use crate::notice::{Notice, Urgency};
use crate::sender::{Sender, Signs};
use crate::words::Words;

/// The places a notice's rows are in.
pub const PLACE: &str = "notices";
pub const DESKTOP: &str = "notices-desktop";
pub const HISTORY: &str = "notices-history";

/// How many read notices the `notices` and `notices-history` places each
/// serve beside every unread and live one: the newest, which
/// Notifications, a program's own row and History open on, the rest asked
/// for by `rows` (daemon.rs). Serving all of a thousand cost the shell
/// about 11 MB of its live heap in the lab, held whether anyone looked or
/// not (LR3WMN, 2026-11-12).
pub const PAGE: usize = 100;

/// The record beside the `notices` place's rows saying how many notices
/// it has rows for, served or not, as Notifications counts them.
pub const SUMMARY: &str = "summary:notices";

/// The record beside History's rows saying how many read notices it has
/// rows for, served or not, so History can count what it keeps.
pub const HISTORY_SUMMARY: &str = "summary:notices-history";

/// The minute a moment falls in, so two times said alike are known alike.
const MINUTE: i64 = 60_000;

/// What a notice's name and note are: its title, its note its body's first
/// line; or, with no title, its body's first line, with nothing in its
/// note.
fn named(summary: &str, plain: &str) -> (String, String) {
    let first = plain
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("")
        .to_owned();
    let title = summary.trim();
    if title.is_empty() {
        (first, String::new())
    } else {
        (title.to_owned(), first)
    }
}

/// All a notice said, its title and its words plain.
fn said(notice: &Notice, read: &Body) -> String {
    if read.plain.is_empty() {
        notice.summary.clone()
    } else {
        format!("{}\n{}", notice.summary, read.plain)
    }
}

/// What a search finds a notice by: the name its row has, else all it
/// said, as its row says them.
pub fn searched(notice: &Notice, read: &Body) -> (String, String) {
    (named(&notice.summary, &read.plain).0, said(notice, read))
}

/// What kind of notice it is, in its drawer's words, from the category its
/// sender gives as the spec's `class.subtype`: "message" for `im`, an
/// error subtype as "transfer failed", a bounced mail as "mail bounced"
/// (VKCF3Z, 2026-11-08), and one it doesn't name left out, never shown
/// raw.
fn kind(category: &str, words: &Words) -> String {
    let (class, subtype) = category.split_once('.').unwrap_or((category, ""));
    let word = words.state_said("notices:kind", class);
    if word.is_empty() {
        return String::new();
    }
    if category == "email.bounced" {
        return words.said(PLACE, "bounced").to_owned();
    }
    if subtype == "error" {
        return words.said(PLACE, "failed").replace("{kind}", word);
    }
    word.to_owned()
}

/// Its drawer's eyebrow: where it's from, then its kind, never a time,
/// which a fact says (VKCF3Z, 2026-11-08). One signed as the desktop's,
/// which For You's path already names, is from the program it came
/// through, "from notify-send", and one whose category names no kind is a
/// notice (VKCF3Z, 2026-11-09).
fn eyebrow(notice: &Notice, sender: &Sender, words: &Words) -> String {
    let signed = if notice.app.is_empty() {
        ""
    } else {
        sender.name.as_str()
    };
    let who = if signed == words.name("for-you:source:desktop", "") {
        notice.app.as_str()
    } else {
        signed
    };
    let mut said = kind(&notice.category, words);
    if said.is_empty() {
        words.said(PLACE, "notice").clone_into(&mut said);
    }
    let from = (!who.is_empty()).then(|| words.said(PLACE, "from").replace("{who}", who));
    from.into_iter()
        .chain((!said.is_empty()).then_some(said))
        .collect::<Vec<_>>()
        .join(" \u{b7} ")
}

/// Its drawer's facts: when it arrived, how it left and when, or that
/// it's still open, and the urgency its sender asked where it isn't
/// normal, the urgency nearly every notice is sent at (VKCF3Z,
/// 2026-11-09). Arrived goes where Left says the same minute, "put away at
/// 10:51" then standing for both (VKCF3Z, 2026-11-08). Each time is a
/// moment, since its row's side says when it came.
fn facts(notice: &Notice, words: &Words) -> Vec<Value> {
    let how = notice.left.as_deref().unwrap_or(if notice.resting {
        "resting"
    } else if notice.live {
        "shown"
    } else {
        ""
    });
    let left = words.state_said("notices:left", how);
    let left_at = notice.left_at.filter(|_| left.contains("{at}"));
    let same = left_at.is_some_and(|at| at / MINUTE == notice.at / MINUTE);
    let mut facts = Vec::new();
    if notice.at > 0 && !same {
        facts.push(json!({
            "name": words.name("notices:arrived", ""),
            "at": notice.at, "as": "moment",
        }));
    }
    let name = words.name("notices:left", "");
    match left_at {
        Some(at) => facts.push(json!({
            "name": name, "said": left.replace("{at}", "{time}"), "at": at, "as": "moment",
        })),
        None if !left.is_empty() => facts.push(json!({
            "name": name, "said": left.replace("{at}", ""),
        })),
        None => {}
    }
    if notice.urgency != Urgency::Normal {
        let urgency = if notice.urgency == Urgency::Low {
            "low"
        } else {
            "critical"
        };
        facts.push(json!({
            "name": words.name("notices:urgency", ""),
            "said": words.state_said("notices:urgency", urgency),
        }));
    }
    facts
}

/// The ask marking it read, which every act on it sends besides, as each
/// act on its card reads it.
fn reads(notice: &Notice) -> Value {
    json!({ "ask": "mark", "notices": [notice.id], "read": true })
}

/// What can be done to it, in the order its card lists them (Notice.js's
/// acts): first going to what it's about, which Return does, by opening
/// it through its sender while it's open and offers that, else going to
/// its program where one is known, else into its drawer. Then what its
/// sender offers while it's open, the links in what it said, whether it's
/// read, and forgetting it while it's open, since what changes a notice
/// comes before what ends it; then copying what it said, last, never
/// first (Joel, 2026-10-28: "having the separate copy option at the top is
/// very confusing"). Each act reads it as it's done.
fn acts(notice: &Notice, sender: &Sender, read: &Body, said: &str, words: &Words) -> Vec<Value> {
    let invoke = |action: &str| {
        let id = notice.id;
        json!({ "ask": "invoke", "notice": id, "action": action, "read": true })
    };
    let mut acts = Vec::new();
    let opens = notice.live && notice.actions.iter().any(|action| action.id == "default");
    if opens {
        acts.push(json!({ "key": "verb:notice:open", "ask": invoke("default"), "token": true }));
    } else if let Some(entry) = &sender.entry {
        acts.push(json!({
            "key": "verb:notice:goTo", "go": entry, "side": sender.name, "also": reads(notice),
        }));
    } else {
        acts.push(json!({ "key": "verb:details", "step": "drawer" }));
    }
    if notice.live {
        for offer in notice
            .actions
            .iter()
            .filter(|action| action.id != "default")
        {
            let glyph = if offer.text.to_lowercase().contains("reply") {
                "reply"
            } else {
                "offer"
            };
            acts.push(json!({
                "key": format!("verb:notice:offer:{}", offer.id), "name": offer.text,
                "glyph": glyph, "ask": invoke(&offer.id), "token": true,
            }));
        }
    }
    // Each link keyed by the words it's said in, which the shell tells
    // apart by where it goes (Y2YBTS, 19edbd31).
    for link in &read.links {
        acts.push(json!({
            "key": "verb:notice:link", "open": link.href,
            "side": body::host(&link.href), "also": reads(notice),
        }));
    }
    // One act, its words saying what it will do, Mark It Read or Mark It
    // Unread, as the vocabulary words its key's states, since the : line
    // names one command for it (VKCF3Z, 2026-11-08). Unread and still
    // open, it's read at its sender where the sender offers Read, so
    // :delete and d tell the program too, with no token, since nothing
    // need come forward.
    let state = if notice.read { "read" } else { "" };
    let reading = (notice.live && !notice.read)
        .then(|| {
            notice
                .actions
                .iter()
                .find(|action| says_read(&action.id) || says_read(&action.text))
        })
        .flatten();
    let ask = reading.map_or_else(
        || json!({ "ask": "mark", "notices": [notice.id], "read": !notice.read }),
        |action| invoke(&action.id),
    );
    acts.push(json!({
        "key": "verb:notice:read", "name": words.name("verb:notice:read", state),
        "glyph": words.glyph("verb:notice:read", state), "ask": ask,
    }));
    if notice.live {
        acts.push(json!({
            "key": "verb:notice:forget", "ask": { "ask": "dismiss", "notices": [notice.id] },
        }));
    }
    acts.push(json!({ "key": "verb:notice:copy", "copy": said, "also": reads(notice) }));
    acts
}

/// Whether an action's identifier or words are a sender's own Read, as
/// "read", "mark-read" or "Mark as read", whatever the case.
fn says_read(word: &str) -> bool {
    let word = word.to_lowercase();
    let Some(before) = word.strip_suffix("read") else {
        return false;
    };
    if before.is_empty() {
        return true;
    }
    let Some(after_mark) = before.strip_prefix("mark") else {
        return false;
    };
    let after_mark = after_mark
        .strip_prefix(['-', '_', ' '])
        .unwrap_or(after_mark);
    after_mark.is_empty()
        || after_mark
            .strip_prefix("as")
            .is_some_and(|after_as| matches!(after_as, "" | "-" | "_" | " "))
}

/// How a notice says it stands, its `x-said` key worded, or "" for none
/// or a key there's no word for: "sent" or "answered" of a talk, "taken"
/// once what it filed is taken back, or "told" with what its sender said
/// instead in `x-told`, so a card asked to take it back is never silent
/// (VKCF3Z, 2026-11-12 [244ed4fd]).
fn stands(notice: &Notice, words: &Words) -> String {
    match notice.hint("x-said") {
        key @ ("sent" | "answered" | "taken") => words.said(PLACE, key).to_owned(),
        "told" => words
            .said(PLACE, "told")
            .replace("{told}", notice.hint("x-told")),
        _ => String::new(),
    }
}

/// A notice's row in `place`, not yet given its `order`, its side saying
/// its standing where `standing` holds words, then its age.
fn row(notice: &Notice, sender: &Sender, words: &Words, place: &str, standing: &str) -> Value {
    let read = body::read(&notice.body);
    let (name, note) = named(&notice.summary, &read.plain);
    let said = said(notice, &read);
    let marked = body::marked(&read);
    // One standing for those it folded says how many came in all, as a
    // conversation's card is one, not each of its messages.
    let folds = (notice.folds > 0).then(|| {
        let all = (notice.folds + 1).to_string();
        json!({ "said": words.said(PLACE, "folds").replace("{count}", &all) })
    });
    // One saying how it stands says so before its moment, "sent · 12:40".
    let stood = stands(notice, words);
    let when = if stood.is_empty() {
        json!({ "at": notice.at, "as": "came" })
    } else {
        json!({ "at": notice.at, "as": "moment" })
    };
    let side: Vec<Value> = (!standing.is_empty())
        .then(|| json!({ "said": standing }))
        .into_iter()
        .chain((!stood.is_empty()).then(|| json!({ "said": stood })))
        .chain(folds)
        .chain(std::iter::once(when))
        .collect();
    // Its words styled as its card draws them where they carry markup or
    // a link, and plain otherwise, so a long log keeps its own lines; its
    // sender a name from the world, set as it's written (VKCF3Z,
    // 2026-11-07).
    let drawer = json!({
        "eyebrow": eyebrow(notice, sender, words), "eyebrowNames": true,
        "title": notice.summary,
        "body": if marked { &read.styled } else { &read.plain }, "styled": marked,
        "prose": true, "picture": notice.picture, "facts": facts(notice, words),
    });
    json!({
        "key": format!("row:{place}:{}", notice.id), "thing": notice.key(), "place": place,
        "kind": "notice", "name": name, "note": note, "glyph": sender.glyph,
        "hue": if notice.urgency == Urgency::Low { "muted" } else { sender.hue.as_str() },
        "side": side, "drawer": drawer, "acts": acts(notice, sender, &read, &said, words),
        "held": {
            "id": notice.id, "app": notice.app, "entry": sender.entry, "read": notice.read,
            "at": notice.at, "live": notice.live, "said": said,
        },
    })
}

/// What one notice was made into: its card's sender and words, and its
/// row in each place it's in, as it read when made.
struct Made {
    notice: Notice,
    looked: bool,
    card: Value,
    rows: [Option<Value>; 3],
}

/// A notice's rows in each place: in the mixed place always, Desktop's
/// while unread or while it's `looked` at there, and History's once read.
/// One passing unkept is in none, since the history never holds it.
fn placed(notice: &Notice, sender: &Sender, words: &Words, looked: bool) -> [Option<Value>; 3] {
    if notice.transient {
        return [None, None, None];
    }
    let (unread, read) = (words.said(PLACE, "unread"), words.said(PLACE, "read"));
    let standing = if notice.read { "" } else { unread };
    let desktop = match (notice.read, looked) {
        (false, _) => Some(row(notice, sender, words, DESKTOP, "")),
        (true, true) => Some(row(notice, sender, words, DESKTOP, read)),
        (true, false) => None,
    };
    [
        Some(row(notice, sender, words, PLACE, standing)),
        desktop,
        notice.read.then(|| row(notice, sender, words, HISTORY, "")),
    ]
}

/// Each notice's rows and its card's sender and words, each made again
/// only once its notice reads differently, so a notice arriving reads
/// one body rather than every one the history keeps.
#[derive(Default)]
pub struct Rows {
    made: BTreeMap<u32, Made>,
}

impl Rows {
    /// Lets everything made go, as when the desktop entries change, which
    /// may sign any notice anew.
    pub fn forget(&mut self) {
        self.made.clear();
    }

    /// Every notice's rows, by place, newest first as `notices` are: the
    /// mixed place's, then Desktop's, then History's. `looked` are the
    /// notices Desktop shows while Joel looks at it, kept there once read.
    pub fn make<'a>(
        &mut self,
        notices: impl Iterator<Item = &'a Notice>,
        signs: &Signs,
        looked: &BTreeSet<u32>,
    ) -> [Vec<Value>; 3] {
        let mut places: [Vec<Value>; 3] = Default::default();
        let mut made = BTreeMap::new();
        for notice in notices {
            let shown = looked.contains(&notice.id);
            let one = match self.made.remove(&notice.id) {
                Some(was) if was.notice == *notice && was.looked == shown => was,
                _ => {
                    let sender = signs.sender(&notice.app, &notice.entry, notice.hint("x-from"));
                    Made {
                        notice: notice.clone(),
                        looked: shown,
                        card: json!({
                            "sender": {
                                "glyph": sender.glyph, "hue": sender.hue, "name": sender.name,
                            },
                            "body": body::read(&notice.body),
                            "said": stands(notice, signs.words),
                        }),
                        rows: placed(notice, &sender, signs.words, shown),
                    }
                }
            };
            for (place, row) in places.iter_mut().zip(&one.rows) {
                place.extend(row.iter().cloned());
            }
            made.insert(notice.id, one);
        }
        self.made = made;
        places
    }

    /// Each notice's row in the mixed place, as last made, by notice.
    pub fn mixed(&self) -> impl Iterator<Item = &Value> {
        self.made.values().filter_map(|made| made.rows[0].as_ref())
    }

    /// Each read notice's row in History, as last made, by notice.
    pub fn history(&self) -> impl Iterator<Item = &Value> {
        self.made.values().filter_map(|made| made.rows[2].as_ref())
    }

    /// What a notice's card reads beside the notice: who sent it, signed
    /// as its row is, and what it said, plain and styled, and its links.
    pub fn card(&self, id: u32) -> Option<&Value> {
        self.made.get(&id).map(|made| &made.card)
    }
}

/// The notices Desktop shows, by id, so those read while it's looked at
/// stay there.
pub fn shown(desktop: &[Value]) -> BTreeSet<u32> {
    desktop
        .iter()
        .filter_map(|row| row["held"]["id"].as_u64())
        .filter_map(|id| u32::try_from(id).ok())
        .collect()
}

/// What each place serves of its rows as made, and the records beside
/// them: the mixed place and History a page each (see [`paged`]) with the
/// count it has rows for, and Desktop every row, since it holds only the
/// unread and those read while Joel looks at it.
pub fn served(made: [Vec<Value>; 3]) -> [(Vec<Value>, Vec<Value>); 3] {
    let [mixed, desktop, history] = made;
    let page = |key: &str, rows: Vec<Value>| {
        let count = json!({ "key": key, "kind": "summary", "summary": { "count": rows.len() } });
        (paged(rows), vec![count])
    };
    [
        page(SUMMARY, mixed),
        (desktop, Vec::new()),
        (history, Vec::new()),
    ]
}

/// The rows of a paged place it serves, of every one made, in their
/// order: each unread or live one, and the newest [`PAGE`] read.
fn paged(rows: Vec<Value>) -> Vec<Value> {
    let mut read = 0;
    rows.into_iter()
        .filter(|row| {
            let held = &row["held"];
            if held["read"] != true || held["live"] == true {
                return true;
            }
            read += 1;
            read <= PAGE
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use windows::entries::Entries;

    use super::*;
    use crate::sender::{Marks, Senders};

    fn notice(id: u32, fields: &Value) -> Notice {
        let mut record = json!({ "id": id, "at": 1_000 * i64::from(id), "app": "Slack",
            "summary": format!("Message {id}"), "body": "hi\nthere" });
        if let (Some(record), Some(fields)) = (record.as_object_mut(), fields.as_object()) {
            record.extend(fields.clone());
        }
        serde_json::from_value(record).expect("a notice")
    }

    fn making(notices: &[Notice], looked: &[u32]) -> [Vec<Value>; 3] {
        making_into(&mut Rows::default(), notices, looked)
    }

    fn making_into(rows: &mut Rows, notices: &[Notice], looked: &[u32]) -> [Vec<Value>; 3] {
        let senders: Senders =
            serde_json::from_str(include_str!("../senders.json")).expect("senders.json reads");
        let entries =
            Entries::from_texts([("slack", "[Desktop Entry]\nType=Application\nName=Slack\n")]);
        let marks = Marks::of(&[("slack", "f198")]);
        let words = crate::words::tests::core();
        let signs = Signs {
            senders: &senders,
            marks: &marks,
            entries: &entries,
            words: &words,
        };
        rows.make(notices.iter(), &signs, &looked.iter().copied().collect())
    }

    #[test]
    fn one_standing_for_its_conversation_says_how_many_came() {
        let folded = notice(4, &json!({ "read": true, "folds": 2 }));
        let [mixed, _, history] = making(&[folded], &[]);
        let side = json!([{ "said": "3 in all" }, { "at": 4_000, "as": "came" }]);
        assert_eq!(mixed[0]["side"], side, "the two it took over and itself");
        assert_eq!(history[0]["side"], side, "and so in History");
    }

    #[test]
    fn a_card_reads_its_sender_and_words_off_what_its_rows_were_made_from() {
        let mut rows = Rows::default();
        let sent = notice(10, &json!({ "body": "<b>hi</b> there" }));
        making_into(&mut rows, std::slice::from_ref(&sent), &[]);
        let card = rows.card(10).expect("its card");
        assert_eq!(
            card["sender"],
            json!({ "glyph": { "code": "f198" }, "hue": "person",
            "name": "Slack" })
        );
        assert_eq!(card["body"]["plain"], "hi there");
        let read = Notice { read: true, ..sent };
        let [mixed, _, history] = making_into(&mut rows, &[read], &[]);
        assert_eq!(
            mixed[0]["side"],
            json!([{ "at": 10_000, "as": "came" }]),
            "made anew once read"
        );
        assert_eq!(keys(&history), ["row:notices-history:10"]);
        making_into(&mut rows, &[], &[]);
        assert!(
            rows.card(10).is_none(),
            "gone from the history, its card goes"
        );
    }

    #[test]
    fn a_sender_signing_by_its_own_name_and_saying_how_it_stands_reads_so() {
        // The void's notice of Joel's line to it (VKCF3Z, 2026-11-12):
        // signed by its name with the agents' mark, and saying it was sent,
        // then answered, before its time; read, so its side leads with no
        // "unread".
        let sent = notice(
            10,
            &json!({ "app": "pi", "summary": "is the beacon lit?", "read": true,
            "body": "", "hints": { "x-from": "The Void", "x-said": "sent" } }),
        );
        let mut rows = Rows::default();
        let [mixed, _, _] = making_into(&mut rows, std::slice::from_ref(&sent), &[]);
        let card = rows.card(10).expect("its card");
        assert_eq!(
            card["sender"],
            json!({ "glyph": "agent", "hue": "program", "name": "The Void" }),
            "its own name, the agents' mark and hue"
        );
        assert_eq!(card["said"], "sent", "the card says it was sent");
        assert_eq!(
            mixed[0]["side"],
            json!([{ "said": "sent" }, { "at": 10_000, "as": "moment" }]),
            "then its moment"
        );
        let answered = Notice {
            hints: json!({ "x-from": "The Void", "x-said": "answered" })
                .as_object()
                .cloned()
                .unwrap_or_default(),
            ..sent.clone()
        };
        making_into(&mut rows, &[answered], &[]);
        assert_eq!(rows.card(10).expect("its card")["said"], "answered");
        // Taken back, it says so; anything else the void answers to that
        // is said as it said it (VKCF3Z, 2026-11-12 [244ed4fd]).
        let hinted = |hints: Value| Notice {
            hints: hints.as_object().cloned().unwrap_or_default(),
            ..sent.clone()
        };
        making_into(&mut rows, &[hinted(json!({ "x-said": "taken" }))], &[]);
        assert_eq!(
            rows.card(10).expect("its card")["said"],
            "taken back",
            "what was filed, taken back"
        );
        let told = hinted(json!({ "x-said": "told", "x-told": "nothing filed then" }));
        let [mixed, _, _] = making_into(&mut rows, &[told], &[]);
        assert_eq!(
            mixed[0]["side"],
            json!([{ "said": "nothing filed then" }, { "at": 10_000, "as": "moment" }]),
            "what the void said instead, before its moment"
        );
        let unknown = Notice {
            hints: json!({ "x-said": "thinking" })
                .as_object()
                .cloned()
                .unwrap_or_default(),
            ..sent
        };
        let [mixed, _, _] = making_into(&mut rows, &[unknown], &[]);
        assert_eq!(
            rows.card(10).expect("its card")["said"],
            "",
            "a key it hasn't a word for"
        );
        assert_eq!(mixed[0]["side"], json!([{ "at": 10_000, "as": "came" }]));
        assert_eq!(rows.card(10).expect("its card")["sender"]["name"], "Agents");
    }

    fn keys(rows: &[Value]) -> Vec<&str> {
        rows.iter().filter_map(|row| row["key"].as_str()).collect()
    }

    #[test]
    fn the_place_serves_every_unread_and_live_notice_and_only_the_newest_read() {
        // Newest first, as the history holds them: a live one read, then
        // PAGE and two more read, then an unread one older than them all.
        let mut notices = vec![notice(200, &json!({ "read": true, "live": true }))];
        let read = u32::try_from(PAGE).expect("a page fits") + 2;
        notices.extend((0..read).map(|at| notice(199 - at, &json!({ "read": true }))));
        notices.push(notice(1, &json!({})));
        let [mixed, _, _] = making(&notices, &[]);
        let served = paged(mixed);
        let served = keys(&served);
        assert_eq!(
            served.len(),
            PAGE + 2,
            "the page and the two kept past it: {served:?}"
        );
        assert_eq!(
            served[..2],
            ["row:notices:200", "row:notices:199"],
            "live first"
        );
        assert_eq!(
            served.last(),
            Some(&"row:notices:1"),
            "the unread, however old"
        );
        assert!(
            !served.contains(&"row:notices:98"),
            "the oldest read past the page isn't served: {served:?}"
        );
    }

    #[test]
    fn history_serves_its_newest_page_saying_how_many_it_has_rows_for() {
        // An unread one, then PAGE and two more read, newest first.
        let mut notices = vec![notice(200, &json!({}))];
        let read = u32::try_from(PAGE).expect("a page fits") + 2;
        notices.extend((0..read).map(|at| notice(199 - at, &json!({ "read": true }))));
        let [mixed, desktop, history] = served(making(&notices, &[]));
        let rows = keys(&history.0);
        assert_eq!(rows.len(), PAGE, "a page of the read: {rows:?}");
        let newest = rows.first();
        assert_eq!(newest, Some(&"row:notices-history:199"), "newest first");
        let count = json!([{ "key": HISTORY_SUMMARY, "kind": "summary",
            "summary": { "count": PAGE + 2 } }]);
        assert_eq!(json!(history.1), count, "and how many it has rows for");
        let mixed_count = &mixed.1[0]["summary"]["count"];
        assert_eq!(mixed_count, PAGE + 3, "the mixed place counts every one");
        let desktop = keys(&desktop.0);
        assert_eq!(desktop, ["row:notices-desktop:200"], "its unread whole");
    }

    fn act_keys(row: &Value) -> Vec<&str> {
        row["acts"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|act| act["key"].as_str())
            .collect()
    }

    #[test]
    fn each_list_is_a_place_worded_for_it() {
        let notices = [
            notice(3, &json!({})),
            notice(2, &json!({ "read": true })),
            notice(1, &json!({ "read": true })),
        ];
        let [mixed, desktop, history] = making(&notices, &[2]);
        assert_eq!(
            keys(&mixed),
            ["row:notices:3", "row:notices:2", "row:notices:1"]
        );
        assert_eq!(
            mixed[0]["side"],
            json!([{ "said": "unread" }, { "at": 3000, "as": "came" }])
        );
        assert_eq!(mixed[1]["side"], json!([{ "at": 2000, "as": "came" }]));
        assert_eq!(
            keys(&desktop),
            ["row:notices-desktop:3", "row:notices-desktop:2"],
            "one read while Desktop's looked at stays there"
        );
        assert_eq!(desktop[0]["side"], json!([{ "at": 3000, "as": "came" }]));
        assert_eq!(desktop[1]["side"][0], json!({ "said": "read" }));
        assert_eq!(
            keys(&history),
            ["row:notices-history:2", "row:notices-history:1"]
        );
        assert_eq!(shown(&desktop), BTreeSet::from([3, 2]));
        let passing = [notice(4, &json!({ "transient": true, "live": true }))];
        assert!(
            making(&passing, &[]).iter().all(Vec::is_empty),
            "one passing is in none"
        );
    }

    #[test]
    fn a_senders_read_is_known_by_its_identifier_or_words() {
        for word in [
            "read",
            "Read",
            "mark-read",
            "mark_as_read",
            "Mark as read",
            "markasread",
        ] {
            assert!(says_read(word), "{word} is Read");
        }
        for word in [
            "readme",
            "Reply",
            "mark as unread",
            "marks read",
            "mark as  read",
        ] {
            assert!(!says_read(word), "{word} isn't");
        }
    }

    #[test]
    fn its_kind_is_said_from_its_category_or_left_out() {
        let words = crate::words::tests::core();
        let said = |category| kind(category, &words);
        assert_eq!(said("im"), "message", "a kind by its class");
        assert_eq!(said("im.received"), "message", "and with its subtype");
        assert_eq!(said("transfer.error"), "transfer failed", "an error fails");
        assert_eq!(said("email.bounced"), "mail bounced", "a bounced mail");
        assert_eq!(said("x-gnome.music"), "", "one never named, never raw");
        assert_eq!(said(""), "", "and none given");
    }

    #[test]
    fn a_row_is_named_and_marked_as_its_card() {
        let [mixed, ..] = making(&[notice(5, &json!({ "urgency": "low" }))], &[]);
        let row = &mixed[0];
        assert_eq!(
            (row["name"].as_str(), row["note"].as_str()),
            (Some("Message 5"), Some("hi"))
        );
        assert_eq!(row["thing"], "notice:5");
        assert_eq!(row["glyph"], json!({ "code": "f198" }));
        assert_eq!(row["hue"], "muted", "low urgency is muted");
        assert_eq!(row["held"]["said"], "Message 5\nhi\nthere");
        assert_eq!(row["held"]["entry"], "slack");
        assert_eq!(row["drawer"]["eyebrow"], "from Slack \u{b7} notice");
        assert_eq!(row["drawer"]["body"], "hi\nthere");
        assert_eq!(row["drawer"]["styled"], false);
        let facts: Vec<&str> = row["drawer"]["facts"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|fact| fact["name"].as_str())
            .collect();
        assert_eq!(facts, ["Arrived", "Urgency"]);
        let [untitled, ..] = making(&[notice(6, &json!({ "summary": "" }))], &[]);
        assert_eq!(
            (untitled[0]["name"].as_str(), untitled[0]["note"].as_str()),
            (Some("hi"), Some(""))
        );
    }

    #[test]
    fn its_acts_are_its_cards_in_order() {
        let live = notice(
            7,
            &json!({ "live": true, "body": "see <a href=\"https://x.example/a\">a</a>",
                "category": "im.received",
                "actions": [{ "id": "default", "text": "Open" }, { "id": "r", "text": "Reply" }] }),
        );
        let [mixed, ..] = making(&[live], &[]);
        assert_eq!(
            act_keys(&mixed[0]),
            [
                "verb:notice:open",
                "verb:notice:offer:r",
                "verb:notice:link",
                "verb:notice:read",
                "verb:notice:forget",
                "verb:notice:copy"
            ]
        );
        let acts = &mixed[0]["acts"];
        assert_eq!(acts[0]["token"], true);
        assert_eq!(acts[0]["ask"]["read"], true, "opening it reads it");
        assert_eq!(
            (acts[1]["name"].as_str(), acts[1]["glyph"].as_str()),
            (Some("Reply"), Some("reply"))
        );
        assert_eq!(acts[2]["side"], "x.example");
        assert_eq!(
            acts[2]["also"],
            json!({ "ask": "mark", "notices": [7], "read": true })
        );
        assert_eq!(mixed[0]["drawer"]["eyebrow"], "from Slack \u{b7} message");
        assert_eq!(mixed[0]["drawer"]["styled"], true);
        let gone = notice(
            8,
            &json!({ "read": true, "left": "dismissed", "leftAt": 8_000 }),
        );
        let [mixed, ..] = making(&[gone], &[]);
        assert_eq!(
            act_keys(&mixed[0]),
            ["verb:notice:goTo", "verb:notice:read", "verb:notice:copy"],
            "gone, it goes to its program"
        );
        assert_eq!(
            (
                &mixed[0]["acts"][1]["name"],
                &mixed[0]["acts"][1]["ask"]["read"]
            ),
            (&json!("Mark It Unread"), &json!(false)),
            "read, it's marked unread, said so"
        );
        assert_eq!(mixed[0]["acts"][0]["go"], "slack");
        assert_eq!(
            mixed[0]["drawer"]["facts"],
            json!([{ "name": "Left", "said": "put away at {time}", "at": 8000, "as": "moment" }]),
            "left the minute it came, Left says both"
        );
        let offering = notice(
            11,
            &json!({ "live": true, "actions": [{ "id": "read", "text": "Mark as read" }] }),
        );
        let [mixed, ..] = making(&[offering], &[]);
        let read = &mixed[0]["acts"][2];
        assert_eq!(
            (&read["key"], &read["ask"]),
            (
                &json!("verb:notice:read"),
                &json!({ "ask": "invoke", "notice": 11, "action": "read", "read": true })
            ),
            "one whose sender offers Read is read there"
        );
        assert!(read.get("token").is_none(), "with nothing to bring forward");
        let nowhere = notice(9, &json!({ "app": "some-tool" }));
        let [mixed, ..] = making(&[nowhere], &[]);
        assert_eq!(
            act_keys(&mixed[0])[0],
            "verb:details",
            "with nowhere to go, its drawer"
        );
    }
}
