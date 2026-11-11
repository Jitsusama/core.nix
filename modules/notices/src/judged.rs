//! What's judged of a notice as it arrives: how far it reaches, by the
//! level its sender is kept to (`jitsusama.quickshell.notices.senders`),
//! whether Quiet holds it back, and whether it waits to be seen. The
//! levels are Levels.js's, and the senders that only answer what Joel
//! just did are those Notices.qml never counts as missed, read from
//! senders.json beside this crate.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde::Deserialize;

use crate::notice::{Notice, Urgency};

/// From the least reach to the most: kept in the history alone, counted
/// as waiting but never shown, shown as a card, or shown as a card that
/// stays until it's answered.
const LEVELS: [&str; 4] = ["history", "count", "card", "urgent"];

/// The level a notice gets: what it asks for, a card or, critical, an
/// urgent one, but never past its sender's level. A sender with no level,
/// or one not known, reaches as far as it asks.
pub fn reached(level: Option<&str>, critical: bool) -> &'static str {
    let asked = if critical { 3 } else { 2 };
    let kept = level.and_then(|level| LEVELS.iter().position(|one| *one == level));
    LEVELS[kept.map_or(asked, |kept| kept.min(asked))]
}

/// What a level does: whether the card shows, whether the notice counts
/// as waiting once it's gone unseen, and whether it stays until answered.
#[derive(Debug, PartialEq, Eq)]
pub struct Reach {
    pub shown: bool,
    pub counted: bool,
    pub urgent: bool,
}

pub fn reach(level: &str) -> Reach {
    let at = LEVELS.iter().position(|one| *one == level);
    Reach {
        shown: at.is_some_and(|at| at >= 2),
        counted: at.is_some_and(|at| at >= 1),
        urgent: level == "urgent",
    }
}

/// The senders' own words: those whose every notice answers what Joel
/// just did, as the recorder's saying where a file went, so none is missed.
#[derive(Debug, Default, Deserialize)]
pub struct Senders {
    #[serde(default)]
    pub answering: BTreeSet<String>,
}

/// What every arrival is judged by.
#[derive(Debug, Default)]
pub struct Judge {
    /// Each sender's level, by its name in lower case.
    pub levels: BTreeMap<String, String>,
    pub senders: Senders,
}

impl Judge {
    /// The levels as home.nix writes them, `{ "<sender>": "<level>" }`,
    /// and the senders as senders.json says them.
    pub fn read(levels: &str, senders: &str) -> Result<Judge, serde_json::Error> {
        Ok(Judge {
            levels: serde_json::from_str(levels)?,
            senders: serde_json::from_str(senders)?,
        })
    }

    /// Judges a notice arriving: its level, whether it stays, whether it's
    /// held back, kept from the screen by its level or by Quiet, which an
    /// urgent one passes, and whether it waits to be seen. One sent at low
    /// urgency, or saying it answers with the `x-answers` hint, never does.
    pub fn judge(&self, notice: &mut Notice, quiet: bool) {
        let name = notice.app.to_lowercase();
        let level = reached(
            self.levels.get(&name).map(String::as_str),
            notice.urgency == Urgency::Critical,
        );
        let reach = reach(level);
        level.clone_into(&mut notice.level);
        notice.urgent = reach.urgent;
        notice.held = !reach.shown || (quiet && !reach.urgent);
        notice.missed = reach.counted
            && notice.urgency != Urgency::Low
            && !self.senders.answering.contains(&name)
            && !notice.hints.contains_key("x-answers");
    }
}

/// Whether Joel holds notices now, as Quiet.qml writes it to the runtime
/// folder's `quiet`: "<since> <until>", until 0 for until he ends it, or
/// nothing while he doesn't. Quiet held while sharing the screen is the
/// shell's to say, since it's the shell that sees the share.
pub fn kept_quiet(file: &Path, now: i64) -> bool {
    let Ok(text) = std::fs::read_to_string(file) else {
        return false;
    };
    let mut said = text.split_whitespace();
    let (Some(_since), Some(until)) = (said.next(), said.next()) else {
        return false;
    };
    // The shell writes whole milliseconds, Date.now()'s; one it can't read
    // is taken as held until ended, as holding too long loses nothing.
    let until = until.parse::<i64>().unwrap_or(0);
    until == 0 || until > now
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    /// A notice's hints holding only `name`, as one saying it answers.
    fn hinted(name: &str) -> serde_json::Map<String, Value> {
        let mut hints = serde_json::Map::new();
        hints.insert(name.into(), Value::String(String::new()));
        hints
    }

    fn judge() -> Judge {
        Judge::read(
            r#"{"chatty": "history", "counter": "count", "loud": "card"}"#,
            r#"{"answering": ["recorder"]}"#,
        )
        .expect("they read")
    }

    fn from(app: &str, urgency: Urgency) -> Notice {
        Notice {
            app: app.into(),
            urgency,
            ..Notice::default()
        }
    }

    #[test]
    fn a_notice_reaches_as_far_as_it_asks_but_never_past_its_senders_level() {
        assert_eq!(reached(None, false), "card");
        assert_eq!(reached(None, true), "urgent");
        assert_eq!(reached(Some("count"), true), "count");
        assert_eq!(reached(Some("urgent"), false), "card");
        assert_eq!(reached(Some("nonsense"), true), "urgent");
    }

    #[test]
    fn each_level_shows_counts_and_stays_as_levels_js_says() {
        let at = |level| reach(level);
        assert_eq!(
            at("history"),
            Reach {
                shown: false,
                counted: false,
                urgent: false
            }
        );
        assert_eq!(
            at("count"),
            Reach {
                shown: false,
                counted: true,
                urgent: false
            }
        );
        assert_eq!(
            at("card"),
            Reach {
                shown: true,
                counted: true,
                urgent: false
            }
        );
        assert_eq!(
            at("urgent"),
            Reach {
                shown: true,
                counted: true,
                urgent: true
            }
        );
    }

    #[test]
    fn a_card_is_shown_and_missed_until_seen() {
        let mut notice = from("mail", Urgency::Normal);
        judge().judge(&mut notice, false);
        assert_eq!(notice.level, "card");
        // Held, missed and urgent.
        assert_eq!(
            (notice.held, notice.missed, notice.urgent),
            (false, true, false)
        );
    }

    #[test]
    fn quiet_holds_back_all_but_an_urgent_one() {
        let mut normal = from("mail", Urgency::Normal);
        let mut critical = from("mail", Urgency::Critical);
        judge().judge(&mut normal, true);
        judge().judge(&mut critical, true);
        // Held and missed, then held and urgent.
        assert_eq!((normal.held, normal.missed), (true, true));
        assert_eq!((critical.held, critical.urgent), (false, true));
    }

    #[test]
    fn a_sender_kept_to_its_level_is_held_and_counted_as_it_says() {
        let mut chatty = from("Chatty", Urgency::Critical);
        let mut counter = from("counter", Urgency::Normal);
        judge().judge(&mut chatty, false);
        judge().judge(&mut counter, false);
        // Held, missed and urgent, then held and missed.
        assert_eq!(
            (chatty.held, chatty.missed, chatty.urgent),
            (true, false, false)
        );
        assert_eq!((counter.held, counter.missed), (true, true));
    }

    #[test]
    fn one_low_answering_or_hinted_as_answering_is_never_missed() {
        let mut low = from("mail", Urgency::Low);
        let mut recorder = from("Recorder", Urgency::Normal);
        let mut hinted = Notice {
            hints: hinted("x-answers"),
            ..from("google drive", Urgency::Normal)
        };
        for notice in [&mut low, &mut recorder, &mut hinted] {
            judge().judge(notice, false);
            assert!(!notice.missed, "{} was missed", notice.app);
        }
    }

    #[test]
    fn quiet_is_kept_while_its_file_says_so_and_its_time_isnt_up() {
        let folder = std::env::temp_dir().join(format!("notices-quiet-{}", std::process::id()));
        std::fs::create_dir_all(&folder).expect("a folder");
        let file = folder.join("quiet");
        assert!(!kept_quiet(&file, 5_000), "no file, no quiet");
        std::fs::write(&file, "").expect("written");
        assert!(!kept_quiet(&file, 5_000), "an empty file, no quiet");
        std::fs::write(&file, "1000 0\n").expect("written");
        assert!(kept_quiet(&file, 5_000), "until it's ended");
        std::fs::write(&file, "1000 9000\n").expect("written");
        assert!(kept_quiet(&file, 5_000), "before its time is up");
        assert!(!kept_quiet(&file, 9_000), "once its time is up");
        let _ = std::fs::remove_dir_all(&folder);
    }
}
