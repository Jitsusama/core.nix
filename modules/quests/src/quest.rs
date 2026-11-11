//! A quest as the node serves it (mycelium's `quest` and `ended-quest`
//! kinds, the harness's two lists): read the way the launcher's
//! `Quests.qml` read it, its own kind under `quest.kind` since the node's
//! kind stands over it, the folder a session on it starts in, and how its
//! status is said, and the moment it was last touched, which the shell
//! words as a day by its own calendar.

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

const MS_A_DAY: i64 = 86_400_000;

/// The moment a quest was last touched, in milliseconds since 1970, from
/// a date or a time as the harness writes either: a time by its own zone,
/// `2026-10-21T09:00:00.000Z`, and a date alone at its noon in UTC,
/// which is that day by any calendar within eleven hours of it. Nothing
/// where it doesn't read as one. Only the moment travels, never its day
/// in words, since what day it is depends on the clock and the zone the
/// shell reads it by (KP0L6A, 2026-11-10).
pub fn moment(updated: &str) -> Option<i64> {
    let mut date = updated.get(..10)?.splitn(3, '-');
    let year: i64 = date.next()?.parse().ok()?;
    let month: i64 = date.next()?.parse().ok()?;
    let of: i64 = date.next()?.parse().ok()?;
    if !(1..=12).contains(&month) || !(1..=31).contains(&of) {
        return None;
    }
    let day = days_since_1970(year, month, of) * MS_A_DAY;
    let Some(time) = updated.get(10..).filter(|rest| !rest.is_empty()) else {
        return Some(day + MS_A_DAY / 2);
    };
    let time = time.strip_prefix('T')?;
    let zone_at = time.find(['Z', '+', '-']).unwrap_or(time.len());
    let (clock, zone) = time.split_at(zone_at);
    Some(day + ms_of_clock(clock)? - ms_of_zone(zone)?)
}

/// The days from 1970's first to a day of the civil calendar (Howard
/// Hinnant's), so the daemon needs no calendar of its own.
fn days_since_1970(year: i64, month: i64, of: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let of_era = year - era * 400;
    let of_year = (153 * ((month + 9) % 12) + 2) / 5 + of - 1;
    let days_of_era = of_era * 365 + of_era / 4 - of_era / 100 + of_year;
    era * 146_097 + days_of_era - 719_468
}

/// A clock's `09:00`, `09:00:30` or `09:00:30.250`, in milliseconds.
fn ms_of_clock(clock: &str) -> Option<i64> {
    let mut parts = clock.splitn(3, ':');
    let hours: i64 = parts.next()?.parse().ok()?;
    let minutes: i64 = parts.next()?.parse().ok()?;
    let seconds: f64 = parts.next().map_or(Some(0.0), |s| s.parse().ok())?;
    #[expect(
        clippy::cast_possible_truncation,
        reason = "a minute's seconds, in milliseconds, fit"
    )]
    let ms = (seconds * 1000.0).round() as i64;
    Some((hours * 60 + minutes) * 60_000 + ms)
}

/// A zone's `Z`, `+02:00` or `-05:00`, or none (taken as UTC), as how
/// far ahead of UTC it is, in milliseconds.
fn ms_of_zone(zone: &str) -> Option<i64> {
    if zone.is_empty() || zone == "Z" {
        return Some(0);
    }
    let ahead = if zone.starts_with('-') { -1 } else { 1 };
    let mut parts = zone.get(1..)?.splitn(2, ':');
    let hours: i64 = parts.next()?.parse().ok()?;
    let minutes: i64 = parts.next().map_or(Some(0), |m| m.parse().ok())?;
    Some(ahead * (hours * 60 + minutes) * 60_000)
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
    fn a_time_is_its_moment_by_its_own_zone() {
        assert_eq!(moment("2026-10-21T09:00:00.000Z"), Some(1_792_573_200_000));
        assert_eq!(
            moment("2026-10-21T04:00:00-05:00"),
            moment("2026-10-21T09:00:00Z"),
            "five hours behind UTC"
        );
        assert_eq!(moment("1970-01-01T00:00:00.250Z"), Some(250));
    }

    #[test]
    fn a_date_alone_is_its_noon_and_anything_else_none() {
        assert_eq!(moment("1970-01-01"), Some(43_200_000));
        assert_eq!(moment("2026-11-10"), Some(20_767 * MS_A_DAY + 43_200_000));
        assert_eq!(
            moment("2024-03-01"),
            Some(19_783 * MS_A_DAY + 43_200_000),
            "after a leap day"
        );
        assert_eq!(moment("soon"), None);
        assert_eq!(moment("2026-13-01"), None);
        assert_eq!(moment("2026-10-21 09:00"), None, "a time is after a T");
    }
}
