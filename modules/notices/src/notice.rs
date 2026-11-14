//! One notice as the daemon holds it: what its sender said over the bus,
//! what was judged of it as it arrived, and what became of it since. The
//! card on screen, the held list and the launcher's rows all read this one
//! record (Y2YBTS, 2026-11-11: "cards and rows come off one record"), so
//! its fields keep the names the shell's history gave them.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

/// The shell's own programs whose notices For You lists as what they ask,
/// a session's question or a want, so its History never shows them
/// (launcher/Lately.js's LISTED).
const LISTED: [&str; 2] = ["waiting", "pi"];

/// How long one done with is still served as a record, in milliseconds:
/// a day, past any launcher's opening, so one read while Joel looks still
/// holds still in For You (launcher/Lately.js's holdingStill).
const RECENT: i64 = 86_400_000;

/// How urgent its sender said it is, in the specification's three levels.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Urgency {
    Low,
    #[default]
    Normal,
    Critical,
}

impl Urgency {
    /// The level the `urgency` hint's byte names, normal for any other.
    pub fn from_byte(byte: u8) -> Urgency {
        match byte {
            0 => Urgency::Low,
            2 => Urgency::Critical,
            _ => Urgency::Normal,
        }
    }
}

/// One of the actions its sender offers, by the key the sender hears it
/// by and the words it's offered in.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Action {
    pub id: String,
    pub text: String,
}

/// A notice, as held and as kept on disk.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "each is a state the shell reads by its name, as its history kept them"
)]
pub struct Notice {
    /// The id its sender knows it by, never used twice, even across a
    /// restart, so a key names one notice for good.
    pub id: u32,
    /// When it arrived, in milliseconds since 1970.
    pub at: i64,
    pub app: String,
    /// The icon its sender named for itself, kept only so a picture that's
    /// the same icon is known for one.
    pub icon: String,
    /// The desktop entry its sender named, `desktop-entry`, if any.
    pub entry: String,
    pub summary: String,
    /// Its words as sent, with whatever markup its sender gave them.
    pub body: String,
    /// Its actions, in the order offered, without `inline-reply`.
    pub actions: Vec<Action>,
    /// Where it takes a reply in place, what the reply's line says before
    /// anything's typed; none where it takes none.
    pub reply: Option<String>,
    pub urgency: Urgency,
    /// What kind of notice it is, in the specification's words, as
    /// `im.received`.
    pub category: String,
    /// Every hint it came with that's a word, a number or a yes or no, so
    /// the shell's own (`x-waiting`, `x-arrival`, `x-answers`) reach it,
    /// and a sender's: `x-from`, the name it signs by, keeping its mark,
    /// and `x-said`, a key for how it stands ("sent", "answered"), which
    /// the daemon words, never words of its own.
    pub hints: Map<String, Value>,
    /// How long its sender asked it be shown, in milliseconds: -1 for as
    /// long as the server likes, 0 for until it's answered.
    pub expire: i32,
    /// Whether its sender asked that it pass without being kept.
    pub transient: bool,
    /// Whether its sender asked that it stay open once acted on.
    pub resident: bool,
    /// Its own picture as a `file://` address, or "" where it has none.
    pub picture: String,

    /// The level it reached (Levels.js's): history, count, card or urgent.
    pub level: String,
    /// Whether it stays until answered.
    pub urgent: bool,
    /// Whether it was never shown, kept back by its sender's level or by
    /// Quiet.
    pub held: bool,
    /// Whether it waits to be seen.
    pub missed: bool,
    /// Whether the shell sounds it as it's shown: one to Joel himself from
    /// a chat program the machine says who he is in (judged.rs).
    pub chimes: bool,

    pub read: bool,
    /// How it was read where that wasn't here, as `readInSlack`.
    pub read_how: Option<String>,
    pub read_at: Option<i64>,
    /// Whether the shell marked it read once what it said stopped standing.
    pub withdrawn: bool,
    /// Whether it's still open, so its actions can still be done.
    pub live: bool,
    /// Whether it's open but off the screen.
    pub resting: bool,
    /// How it left, once it has: expired, dismissed, withdrawn, or folded
    /// into a newer one from the same conversation.
    pub left: Option<String>,
    pub left_at: Option<i64>,
    /// When it was last said again with something new, so the screen
    /// brings it back to the front.
    pub raised_at: Option<i64>,
    /// How many earlier ones from the same conversation it took the place
    /// of, so its card says how many came, not only the newest.
    pub folds: u32,
}

impl Notice {
    /// Its key among the daemon's records.
    pub fn key(&self) -> String {
        format!("notice:{}", self.id)
    }

    /// The hint a word names, as a word, or "".
    pub fn hint(&self, name: &str) -> &str {
        self.hints.get(name).and_then(Value::as_str).unwrap_or("")
    }

    /// Whether it waits until it's read to leave the history: every one of
    /// the desktop's unread, but not Slack's, since Slack keeps its own and
    /// says what's read there (launcher/Lately.js's awaits).
    pub fn awaits(&self) -> bool {
        !self.read && self.source() != "slack"
    }

    /// Which of For You's sources it's from: Slack's, by its program's name
    /// or its desktop entry, else the desktop's (launcher/Lately.js's
    /// sourceOf).
    pub fn source(&self) -> &'static str {
        let slack = |name: &str| name.eq_ignore_ascii_case("slack");
        if slack(&self.app) || slack(&self.entry) {
            "slack"
        } else {
            "desktop"
        }
    }

    /// Whether For You's History keeps it: read, kept, and not the shell's
    /// own, which For You lists as what they ask (launcher/Lately.js's
    /// ours).
    pub fn in_history(&self) -> bool {
        self.read && !self.transient && !LISTED.contains(&self.app.as_str())
    }

    /// Whether it's served as a record at `now`: while it's unread, open
    /// or missed, and for a day once it's done with. The shell reads only
    /// those (Notices.qml), and History the daemon's own rows of the rest
    /// (row.rs's notices-history), so the shell holds about two hundred
    /// rather than every one kept (BXNMUC, 2026-11-13: 1.26 MB of 1,000).
    pub fn recent(&self, now: i64) -> bool {
        !self.read || self.live || self.missed || now - self.done_at() < RECENT
    }

    /// When it was done with, for History: when it was read, else when it
    /// left, else when it came, as Now's History said it (launcher/
    /// Lately.js's done).
    pub fn done_at(&self) -> i64 {
        self.read_at.or(self.left_at).unwrap_or(self.at)
    }

    /// How it was done with, for History, as launcher/Lately.js's readHow
    /// says it: withdrawn, by its sender or the shell; read where it came
    /// from, as `readInSlack`; read here, where it's known when; or read
    /// before the history kept when, `earlier`.
    pub fn done_how(&self) -> &str {
        if self.withdrawn || self.left.as_deref() == Some("withdrawn") {
            return "withdrawn";
        }
        match self.read_how.as_deref() {
            Some(how) if !how.is_empty() => how,
            _ if self.read_at.is_some() => "read",
            _ => "earlier",
        }
    }

    /// The record served for it: `{ key, kind: "notice", notice }`, its
    /// fields with the two the shell's history read beside them, `low` and
    /// the to-do it `waits` on.
    pub fn record(&self) -> Value {
        let mut notice = serde_json::to_value(self).unwrap_or_else(|_| json!({}));
        notice["low"] = json!(self.urgency == Urgency::Low);
        notice["waits"] = json!(self.hint("x-waiting"));
        json!({ "key": self.key(), "kind": "notice", "notice": notice })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_record_is_keyed_by_its_id_and_says_its_low_and_its_wait() {
        let mut hints = Map::new();
        hints.insert("x-waiting".into(), json!("todo:7"));
        let notice = Notice {
            id: 12,
            urgency: Urgency::Low,
            hints,
            ..Notice::default()
        };
        let record = notice.record();
        assert_eq!(record["key"], "notice:12");
        assert_eq!(record["kind"], "notice");
        assert_eq!(record["notice"]["low"], true);
        assert_eq!(record["notice"]["waits"], "todo:7");
        assert_eq!(record["notice"]["urgency"], "low");
        assert_eq!(record["notice"]["readAt"], Value::Null);
    }

    #[test]
    fn slacks_unread_dont_wait_but_the_desktops_do() {
        let desktop = Notice::default();
        let slack = Notice {
            app: "Slack".into(),
            ..Notice::default()
        };
        let read = Notice {
            read: true,
            ..Notice::default()
        };
        // The desktop's unread, Slack's unread and a read one.
        assert_eq!(
            (desktop.awaits(), slack.awaits(), read.awaits()),
            (true, false, false)
        );
    }

    #[test]
    fn its_source_and_whether_history_keeps_it() {
        let read = Notice {
            read: true,
            app: "mail".into(),
            ..Notice::default()
        };
        let slack = Notice {
            entry: "Slack".into(),
            ..read.clone()
        };
        let asked = Notice {
            app: "waiting".into(),
            ..read.clone()
        };
        let unread = Notice {
            read: false,
            ..read.clone()
        };
        let sources = (read.source(), slack.source());
        assert_eq!(sources, ("desktop", "slack"), "Slack's by its entry too");
        let all = [&read, &slack, &asked, &unread];
        let kept = all.map(Notice::in_history);
        let wanted = [true, true, false, false];
        assert_eq!(kept, wanted, "the read, not the shell's own");
    }

    #[test]
    fn it_is_served_while_it_stands_and_for_a_day_once_done_with() {
        let now = 10 * RECENT;
        let old = now - RECENT - 1;
        let done = Notice {
            read: true,
            read_at: Some(old),
            at: old,
            ..Notice::default()
        };
        let unread = Notice {
            read: false,
            ..done.clone()
        };
        let open = Notice {
            live: true,
            ..done.clone()
        };
        let missed = Notice {
            missed: true,
            ..done.clone()
        };
        let lately = Notice {
            read_at: Some(now - RECENT + 1),
            ..done.clone()
        };
        let all = [&done, &unread, &open, &missed, &lately];
        let served = all.map(|notice| notice.recent(now));
        let wanted = [false, true, true, true, true];
        assert_eq!(served, wanted, "all but the one done with a day ago");
    }

    #[test]
    fn its_history_says_how_and_when_it_was_done_with() {
        let here = Notice {
            read: true,
            read_at: Some(5),
            left_at: Some(3),
            at: 1,
            ..Notice::default()
        };
        let done = (here.done_how(), here.done_at());
        assert_eq!(done, ("read", 5), "read here, when it was read");
        let slack = Notice {
            read_how: Some("readInSlack".into()),
            ..here.clone()
        };
        assert_eq!(slack.done_how(), "readInSlack");
        let gone = Notice {
            left: Some("withdrawn".into()),
            read_at: None,
            ..here.clone()
        };
        let done = (gone.done_how(), gone.done_at());
        assert_eq!(done, ("withdrawn", 3), "withdrawn, when it left");
        let before = Notice {
            read_at: None,
            left_at: None,
            ..here
        };
        let done = (before.done_how(), before.done_at());
        assert_eq!(done, ("earlier", 1), "read untimed, when it came");
    }

    #[test]
    fn a_record_read_back_keeps_every_field() {
        let notice = Notice {
            id: 3,
            at: 1_700_000_000_000,
            app: "mail".into(),
            actions: vec![Action {
                id: "default".into(),
                text: "Open".into(),
            }],
            reply: Some("Reply".into()),
            left: Some("expired".into()),
            ..Notice::default()
        };
        let text = serde_json::to_string(&notice).expect("it's written");
        let back: Notice = serde_json::from_str(&text).expect("it's read");
        assert_eq!(back, notice);
    }
}
