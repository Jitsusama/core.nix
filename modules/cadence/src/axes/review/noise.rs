//! The week's noise, by sender: of the notices, the looks on Joel's list,
//! the landings and the mesh's messages, how many each sender sent, how
//! many nobody acted on, and how many said again what it had said. Each
//! kind is read from its own record, by a function of its own, so it's
//! tested without the machine.

use std::collections::{BTreeMap, BTreeSet};

use jiff::Timestamp;
use serde::Deserialize;

use crate::axes::messages::Note;
use crate::mesh;

/// What one sender sent of one kind over the week.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Tally {
    pub said: u32,
    /// Nobody acted on: a notice that cleared itself unread, a look a
    /// session cleared or took back before he opened it, a question
    /// nobody answered.
    pub unacted: u32,
    /// Said again: the same notice, look, landing or message as before.
    pub again: u32,
}

/// Each kind and sender's tally, the kind first: "notices", "looks",
/// "landings" or "messages".
pub type Tallies = BTreeMap<(&'static str, String), Tally>;

/// What a kind's figures are called on the page.
#[must_use]
pub fn unacted_words(kind: &str) -> &'static str {
    match kind {
        "notices" => "cleared themselves unread",
        "looks" => "cleared or taken back unopened",
        "messages" => "questions nobody answered",
        _ => "",
    }
}

#[derive(Deserialize)]
struct Notices {
    #[serde(default)]
    notices: Vec<Notice>,
}

#[derive(Deserialize)]
struct Notice {
    #[serde(default)]
    app: String,
    #[serde(default)]
    summary: String,
    /// What it said under its summary, which for a chat is only where it
    /// came from, `[shop] in #channel`, so a repeat is both said again.
    #[serde(default)]
    body: String,
    /// In milliseconds.
    #[serde(default)]
    at: i64,
    #[serde(default)]
    left: Option<String>,
    #[serde(default)]
    read: bool,
}

/// The notices' record since `since`, in seconds, by the app that raised
/// each.
pub fn notices(text: &str, since: i64, tallies: &mut Tallies) {
    let Ok(all) = serde_json::from_str::<Notices>(text) else {
        return;
    };
    let mut seen = BTreeSet::new();
    for notice in all.notices.into_iter().filter(|n| n.at >= since * 1000) {
        let tally = tallies.entry(("notices", notice.app.clone())).or_default();
        tally.said += 1;
        if notice.left.as_deref() == Some("expired") && !notice.read {
            tally.unacted += 1;
        }
        if !seen.insert((notice.app, notice.summary, notice.body)) {
            tally.again += 1;
        }
    }
}

#[derive(Deserialize)]
struct Look {
    #[serde(default)]
    address: String,
    #[serde(default)]
    at: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    how: Option<String>,
}

/// Each look's file, by the session that put it up.
pub fn looks(texts: &[String], since: i64, tallies: &mut Tallies) {
    let mut all: Vec<(i64, Look)> = texts
        .iter()
        .filter_map(|text| serde_json::from_str::<Look>(text).ok())
        .filter_map(|look| Some((seconds(&look.at)?, look)))
        .filter(|(at, _)| *at >= since)
        .collect();
    all.sort_by_key(|(at, _)| *at);
    let mut seen = BTreeSet::new();
    for (_, look) in all {
        let tally = tallies.entry(("looks", look.address.clone())).or_default();
        tally.said += 1;
        // Merged, closed or its quest ended: the work answered it, which
        // says nothing either way of whether he needed it.
        let how = look.how.as_deref().unwrap_or_default();
        if how == "taken back" || (how.starts_with("cleared by ") && how != "cleared by you") {
            tally.unacted += 1;
        }
        if !seen.insert((look.address, look.title)) {
            tally.again += 1;
        }
    }
}

#[derive(Deserialize)]
struct Landings {
    #[serde(default)]
    landings: Vec<Landing>,
}

#[derive(Deserialize)]
struct Landing {
    #[serde(default)]
    address: String,
    #[serde(default)]
    at: String,
    #[serde(default, rename = "ref")]
    reference: String,
}

/// The landings' record, by the session that landed each. A landing has
/// no sign of being acted on, so only its repeats count.
pub fn landings(text: &str, since: i64, tallies: &mut Tallies) {
    let Ok(all) = serde_json::from_str::<Landings>(text) else {
        return;
    };
    let mut seen = BTreeSet::new();
    for landing in all
        .landings
        .into_iter()
        .filter(|l| seconds(&l.at).is_some_and(|at| at >= since))
    {
        let tally = tallies.entry(("landings", landing.address)).or_default();
        tally.said += 1;
        if !landing.reference.is_empty() && !seen.insert(landing.reference) {
            tally.again += 1;
        }
    }
}

/// The week's messages, by sender. A question counts as unanswered when
/// no message answers it and it was asked before `until`, in seconds, so
/// one asked a moment ago isn't held against its sender.
pub fn messages(notes: &[Note], until: i64, tallies: &mut Tallies) {
    let answered: BTreeSet<&str> = notes.iter().filter_map(|n| n.re.as_deref()).collect();
    let mut sorted: Vec<&Note> = notes.iter().collect();
    sorted.sort_by_key(|n| mesh::stamped_at(&n.id));
    let mut seen = BTreeSet::new();
    for note in sorted {
        let tally = tallies.entry(("messages", note.from.clone())).or_default();
        tally.said += 1;
        let asked_before = mesh::stamped_at(&note.id).is_some_and(|ms| ms < until * 1000);
        if note.intent == "question" && asked_before && !answered.contains(note.id.as_str()) {
            tally.unacted += 1;
        }
        if !seen.insert((note.from.as_str(), note.said)) {
            tally.again += 1;
        }
    }
}

/// An ISO moment in seconds, or none.
fn seconds(at: &str) -> Option<i64> {
    at.parse::<Timestamp>().ok().map(Timestamp::as_second)
}

#[cfg(test)]
mod tests {
    use super::*;

    // 00:00 UTC on Thu 12 November 2026.
    const SINCE: i64 = 1_794_441_600;

    fn tally(tallies: &Tallies, kind: &'static str, sender: &str) -> Tally {
        tallies
            .get(&(kind, sender.to_owned()))
            .cloned()
            .unwrap_or_default()
    }

    #[test]
    fn a_notice_that_cleared_itself_unread_or_came_again_is_noise_of_its_app() {
        let ms = SINCE * 1000;
        let text = format!(
            r#"{{"next":5,"notices":[
            {{"app":"Slack","summary":"old","at":{old},"left":"expired"}},
            {{"app":"Slack","summary":"a","body":"x","at":{a},"left":"expired","read":false}},
            {{"app":"Slack","summary":"a","body":"x","at":{b},"left":"dismissed","read":true}},
            {{"app":"Slack","summary":"b","body":"y","at":{c},"left":"expired","read":true}},
            {{"app":"Slack","summary":"b","body":"z","at":{d},"left":"dismissed","read":true}}
            ]}}"#,
            old = ms - 1,
            a = ms + 1,
            b = ms + 2,
            c = ms + 3,
            d = ms + 4,
        );
        let mut tallies = Tallies::new();
        notices(&text, SINCE, &mut tallies);
        assert_eq!(
            tally(&tallies, "notices", "Slack"),
            Tally {
                said: 4,
                unacted: 1,
                again: 1
            },
            "one expired unread and one repeat among the four of the week; a channel's \
             summary said again over another message isn't a repeat"
        );
    }

    #[test]
    fn a_look_a_session_cleared_or_took_back_is_noise_of_the_session_that_put_it_up() {
        let look = |title: &str, how: &str| {
            format!(
                r#"{{"address":"AAAAAA","at":"2026-11-12T10:00:00Z","title":"{title}",
                "how":"{how}"}}"#
            )
        };
        let texts = vec![
            look("a", "cleared by 2RCBOJ: nothing left to look at or do"),
            look("b", "taken back"),
            look("c", "cleared by you"),
            look("c", "merged"),
            r#"{"address":"AAAAAA","at":"2026-11-01T10:00:00Z","title":"old","how":"taken back"}"#
                .to_owned(),
        ];
        let mut tallies = Tallies::new();
        looks(&texts, SINCE, &mut tallies);
        assert_eq!(
            tally(&tallies, "looks", "AAAAAA"),
            Tally {
                said: 4,
                unacted: 2,
                again: 1
            },
            "two ended unopened by a session, and one put up again"
        );
    }

    #[test]
    fn a_change_landed_twice_is_said_again() {
        let text = r#"{"v":1,"landings":[
            {"address":"AAAAAA","at":"2026-11-12T10:00:00Z","ref":"core#1"},
            {"address":"AAAAAA","at":"2026-11-12T11:00:00Z","ref":"core#1"},
            {"address":"AAAAAA","at":"2026-11-12T12:00:00Z","ref":"core#2"},
            {"address":"AAAAAA","at":"2026-11-01T12:00:00Z","ref":"core#2"}
            ]}"#;
        let mut tallies = Tallies::new();
        landings(text, SINCE, &mut tallies);
        assert_eq!(
            tally(&tallies, "landings", "AAAAAA"),
            Tally {
                said: 3,
                unacted: 0,
                again: 1
            },
            "core#1 landed twice this week; core#2's older landing is outside it"
        );
    }

    #[test]
    fn a_message_sent_again_or_a_question_left_unanswered_is_noise_of_its_sender() {
        let ms = SINCE * 1000;
        let note = |at: i64, tag: &str, intent: &str, said: u64, re: Option<String>| Note {
            id: format!("{at}-AAAAAA-1-{tag}"),
            intent: intent.to_owned(),
            re,
            from: "AAAAAA".to_owned(),
            said,
        };
        let answered = format!("{}-AAAAAA-1-b", ms + 2);
        let notes = vec![
            note(ms + 1, "a", "question", 7, None),
            note(ms + 2, "b", "question", 8, None),
            note(ms + 3, "c", "fyi", 7, None),
            Note {
                re: Some(answered),
                from: "BBBBBB".to_owned(),
                ..note(ms + 4, "d", "fyi", 9, None)
            },
            note(ms + 7_200_000, "e", "question", 10, None),
        ];
        let mut tallies = Tallies::new();
        messages(&notes, SINCE + 3600, &mut tallies);
        assert_eq!(
            tally(&tallies, "messages", "AAAAAA"),
            Tally {
                said: 4,
                unacted: 1,
                again: 1
            },
            "a's question went unanswered and was said again; e's is too new to judge"
        );
        assert_eq!(
            tally(&tallies, "messages", "BBBBBB").said,
            1,
            "the answer counts to its own sender"
        );
    }
}
