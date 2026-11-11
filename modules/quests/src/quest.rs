//! A quest as the node serves it (mycelium's `quest` and `ended-quest`
//! kinds, the harness's two lists): read the way the launcher's
//! `Quests.qml` read it, its own kind under `quest.kind` since the node's
//! kind stands over it, the folder a session on it starts in, and how its
//! status and the day it was last touched are said.

use serde_json::Value;

/// One quest's record, as the node holds it.
#[derive(Clone, Copy)]
pub struct Quest<'a>(pub &'a Value);

/// The order of the priorities a quest open is placed by, the most
/// pressing first; one with none, or another, after them all.
pub const PRIORITIES: [&str; 5] = ["driving", "active", "queued", "bench", "someday"];

impl<'a> Quest<'a> {
    fn said(self, field: &str) -> &'a str {
        self.0.get(field).and_then(Value::as_str).unwrap_or("")
    }

    pub fn id(self) -> &'a str {
        self.said("id")
    }

    /// What it's called: its title, or its id where it has none.
    pub fn name(self) -> &'a str {
        let title = self.said("title");
        if title.is_empty() { self.id() } else { title }
    }

    /// Its own kind, quest, subquest or sidequest.
    pub fn kind(self) -> &'a str {
        let kind = self.said("quest.kind");
        if kind.is_empty() { "quest" } else { kind }
    }

    pub fn status(self) -> &'a str {
        self.said("status")
    }

    pub fn priority(self) -> &'a str {
        self.said("priority")
    }

    pub fn parent(self) -> &'a str {
        self.said("parent")
    }

    pub fn summary(self) -> &'a str {
        self.said("summary")
    }

    /// When it was last touched, a date or a time as the harness writes
    /// either, or nothing.
    pub fn updated(self) -> &'a str {
        self.said("updated")
    }

    /// Where a session on it starts, as the harness's lists say, or
    /// `home` where they don't.
    pub fn folder(self, home: &'a str) -> &'a str {
        let start = self.said("start");
        if start.is_empty() { home } else { start }
    }

    /// Whether it's still open: neither concluded nor retired.
    pub fn open(self) -> bool {
        !matches!(self.status(), "concluded" | "retired")
    }

    /// Its priority's place among the priorities, or after them all.
    pub fn bucket(self) -> usize {
        PRIORITIES
            .iter()
            .position(|one| *one == self.priority())
            .unwrap_or(PRIORITIES.len())
    }

    /// Its rank among those of its priority, as a number, or after every
    /// ranked one where it has none (or a nought, as the shell read it).
    pub fn rank(self) -> f64 {
        let rank = match self.0.get("rank") {
            Some(Value::Number(number)) => number.as_f64().unwrap_or(0.0),
            Some(Value::String(text)) => text.trim().parse().unwrap_or(0.0),
            _ => 0.0,
        };
        if rank == 0.0 || rank.is_nan() {
            1e9
        } else {
            rank
        }
    }

    /// The theme's glyph its status wears.
    pub fn glyph(self) -> &'static str {
        match self.status() {
            "active" => "active",
            "paused" => "stopped",
            "concluded" => "done",
            "retired" => "cancelled",
            _ => "pending",
        }
    }
}

const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// A day as a clock's calendar says it, from a date or a time as the
/// harness writes either: `22 Oct`, with its year where it isn't
/// `this_year`'s, `5 Jan 2025`; nothing where it doesn't read as a day.
pub fn day(updated: &str, this_year: i64) -> String {
    let date = updated.get(..10).unwrap_or("");
    let mut parts = date.splitn(3, '-');
    let (Some(year), Some(month), Some(of)) = (parts.next(), parts.next(), parts.next()) else {
        return String::new();
    };
    let (Ok(year), Ok(month), Ok(of)) = (
        year.parse::<i64>(),
        month.parse::<usize>(),
        of.parse::<u32>(),
    ) else {
        return String::new();
    };
    let Some(month) = month.checked_sub(1).and_then(|at| MONTHS.get(at)) else {
        return String::new();
    };
    if !(1..=31).contains(&of) {
        return String::new();
    }
    if year == this_year {
        format!("{of} {month}")
    } else {
        format!("{of} {month} {year}")
    }
}

/// The year it is, by the clock, from the days since 1970 began
/// (Howard Hinnant's civil calendar), so the daemon needs no calendar of
/// its own. In UTC, so for a few hours as a year turns a day may say its
/// year a little early or late.
pub fn year_of(days: i64) -> i64 {
    let shifted = days + 719_468;
    let era = shifted.div_euclid(146_097);
    let of_era = shifted.rem_euclid(146_097);
    let year_of_era = (of_era - of_era / 1460 + of_era / 36_524 - of_era / 146_096) / 365;
    let of_year = of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_shifted = (5 * of_year + 2) / 153;
    let month = if month_shifted < 10 {
        month_shifted + 3
    } else {
        month_shifted - 9
    };
    year_of_era + era * 400 + i64::from(month <= 2)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn a_quest_reads_as_the_launcher_read_it() {
        let record = json!({"key": "quest:Q", "kind": "quest", "id": "Q",
            "quest.kind": "subquest", "status": "blocked", "priority": "bench",
            "rank": "2", "start": "/w"});
        let quest = Quest(&record);
        assert_eq!(quest.name(), "Q", "its id where it has no title");
        assert_eq!(quest.kind(), "subquest", "its own kind, not the node's");
        assert_eq!(quest.glyph(), "pending", "blocked waits");
        assert_eq!(quest.bucket(), 3);
        assert_eq!(quest.rank().total_cmp(&2.0), std::cmp::Ordering::Equal);
        assert_eq!(quest.folder("/home"), "/w");
        assert!(quest.open(), "blocked is still open");
    }

    #[test]
    fn one_unranked_or_unplaced_comes_after() {
        let record = json!({"id": "Q", "status": "retired", "priority": "later"});
        let quest = Quest(&record);
        assert!(quest.rank() > 1e8, "no rank comes last");
        assert_eq!(quest.bucket(), PRIORITIES.len());
        assert_eq!(quest.kind(), "quest");
        assert_eq!(quest.folder("/home"), "/home");
        assert!(!quest.open(), "retired has ended");
    }

    #[test]
    fn a_day_says_its_year_only_when_it_isnt_this_one() {
        assert_eq!(day("2026-10-22", 2026), "22 Oct");
        assert_eq!(day("2026-10-21T09:00:00.000Z", 2026), "21 Oct");
        assert_eq!(day("2025-01-05", 2026), "5 Jan 2025");
        assert_eq!(day("soon", 2026), "");
        assert_eq!(day("2026-13-01", 2026), "");
    }

    #[test]
    fn the_year_is_read_from_the_days_since_1970() {
        assert_eq!(year_of(0), 1970);
        assert_eq!(year_of(20_767), 2026, "2026-11-10");
        assert_eq!(year_of(20_453), 2025, "2025-12-31");
        assert_eq!(year_of(20_454), 2026, "2026-01-01");
    }
}
