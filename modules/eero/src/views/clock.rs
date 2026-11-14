//! When things happened, as Joel reads it: eero's UTC times in the
//! machine's own zone, and how long ago, from one moment given to every
//! view so a page is a pure function of the record and the time.

use jiff::Timestamp;
use jiff::tz::TimeZone;

#[derive(Debug, Clone)]
pub struct Clock {
    pub now: Timestamp,
    pub zone: TimeZone,
}

/// One of eero's times, RFC 3339 with up to nine figures of a second.
pub fn parse(stamp: &str) -> Option<Timestamp> {
    stamp.parse().ok()
}

impl Clock {
    /// The machine's time and zone, as the system names them.
    pub fn system() -> Self {
        Self {
            now: Timestamp::now(),
            zone: TimeZone::system(),
        }
    }

    /// Whole seconds from a time to now, none for a time still to come.
    pub fn age(&self, at: Timestamp) -> u64 {
        u64::try_from(self.now.as_second() - at.as_second()).unwrap_or(0)
    }

    /// How long ago a time was, or "now" for one under a second old.
    pub fn ago(&self, at: Timestamp) -> String {
        match self.age(at) {
            0 => "now".into(),
            secs => format!("{} ago", span(secs)),
        }
    }

    fn local(&self, at: Timestamp, format: &str) -> String {
        at.to_zoned(self.zone.clone()).strftime(format).to_string()
    }

    /// The day and minute: 2026-11-05 05:52.
    pub fn minute(&self, at: Timestamp) -> String {
        self.local(at, "%Y-%m-%d %H:%M")
    }

    /// The day: 2026-11-05.
    pub fn day(&self, at: Timestamp) -> String {
        self.local(at, "%Y-%m-%d")
    }

    /// The hour: 17:00.
    pub fn hour(&self, at: Timestamp) -> String {
        self.local(at, "%H:%M")
    }

    /// The time to the second, for events that come seconds apart: 17:42:35.
    pub fn second(&self, at: Timestamp) -> String {
        self.local(at, "%H:%M:%S")
    }
}

/// A length of time in its two largest units, as every command of ours
/// sets one (docs/commands.md, Figures and Words): 37 s, 19 m 55 s,
/// 15 h 31 m, 2 d 19 h, 5 y 359 d. Under a year it reads as the terminal
/// kit's own span does; eero alone keeps years, which a node's time
/// since it last heard from the cloud can run to.
pub fn span(secs: u64) -> String {
    spoken(secs, "m")
}

/// A length of time as the desktop writes one in a notice
/// (docs/design.md), its minutes as "min": 19 min 55 s.
pub fn span_on_desktop(secs: u64) -> String {
    spoken(secs, "min")
}

/// A length of time in its two largest units, minutes called `minute`.
fn spoken(secs: u64, minute: &str) -> String {
    let units: [(u64, &str); 5] = [
        (365 * 86_400, "y"),
        (86_400, "d"),
        (3_600, "h"),
        (60, minute),
        (1, "s"),
    ];
    let Some(first) = units.iter().position(|(size, _)| secs >= *size) else {
        return "0 s".into();
    };
    let (size, unit) = units[first];
    let whole = secs / size;
    let rest = units
        .get(first + 1)
        .map(|(next, unit)| ((secs % size) / next, *unit));
    match rest {
        Some((count, next)) if count > 0 => format!("{whole} {unit} {count} {next}"),
        _ => format!("{whole} {unit}"),
    }
}

/// Seconds until an age, as `span` says it, reads differently: a second
/// while it's said to the second, then the rest of its smaller unit.
pub fn changes_in(secs: u64) -> u64 {
    let smaller = match secs {
        0..3_600 => 1,
        3_600..86_400 => 60,
        86_400..31_536_000 => 3_600,
        _ => 86_400,
    };
    smaller - secs % smaller
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_age_changes_when_its_smaller_unit_next_turns() {
        assert_eq!(changes_in(0), 1);
        assert_eq!(changes_in(1_195), 1);
        assert_eq!(changes_in(3_600), 60);
        assert_eq!(changes_in(55_897), 23);
        assert_eq!(changes_in(2 * 86_400 + 59), 3_541);
        assert_eq!(changes_in(6 * 365 * 86_400 + 1), 86_399);
        for secs in [59, 3_599, 55_897, 86_399, 200_000] {
            let next = secs + changes_in(secs);
            assert_ne!(span(secs), span(next), "{secs} and {next}");
            assert_eq!(span(secs), span(next - 1), "{secs} changes before {next}");
        }
    }

    #[test]
    fn a_span_is_said_in_its_two_largest_units() {
        assert_eq!(span(0), "0 s");
        assert_eq!(span(37), "37 s");
        assert_eq!(span(1_195), "19 m 55 s");
        assert_eq!(span(3_600), "1 h");
        assert_eq!(span(55_897), "15 h 31 m");
        assert_eq!(span(2 * 86_400 + 19 * 3_600 + 59), "2 d 19 h");
        assert_eq!(span(6 * 365 * 86_400), "6 y");
    }

    #[test]
    fn under_a_year_a_span_reads_as_every_other_command_says_it() {
        for secs in [
            0,
            1,
            37,
            59,
            60,
            1_195,
            3_600,
            55_897,
            86_400,
            200_000,
            364 * 86_400,
        ] {
            assert_eq!(
                span(secs),
                terminal_kit::time::span(secs),
                "eero and the terminal kit say {secs} s apart"
            );
        }
    }

    #[test]
    fn a_notice_says_its_minutes_as_the_desktop_does() {
        assert_eq!(span_on_desktop(1_195), "19 min 55 s");
        assert_eq!(span_on_desktop(55_897), "15 h 31 min");
        assert_eq!(span_on_desktop(37), "37 s");
    }

    #[test]
    fn eeros_times_read_to_the_nanosecond_and_show_in_the_machines_zone() {
        let clock = Clock {
            now: parse("2026-11-07T22:55:20Z").unwrap(),
            zone: TimeZone::fixed(jiff::tz::offset(-5)),
        };
        let reboot = parse("2026-11-07T07:22:42.035551983Z").unwrap();
        assert_eq!(clock.minute(reboot), "2026-11-07 02:22");
        assert_eq!(clock.ago(reboot), "15 h 32 m ago");
        assert_eq!(clock.ago(clock.now), "now");
    }
}
