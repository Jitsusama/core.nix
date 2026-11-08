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
}

/// A length of time in its two largest units: 37 s, 19 min 55 s,
/// 15 h 31 min, 2 d 19 h, 5 y 359 d.
pub fn span(secs: u64) -> String {
    const UNITS: [(u64, &str); 5] = [
        (365 * 86_400, "y"),
        (86_400, "d"),
        (3_600, "h"),
        (60, "min"),
        (1, "s"),
    ];
    let Some(first) = UNITS.iter().position(|(size, _)| secs >= *size) else {
        return "0 s".into();
    };
    let (size, unit) = UNITS[first];
    let whole = secs / size;
    let rest = UNITS
        .get(first + 1)
        .map(|(next, unit)| ((secs % size) / next, *unit));
    match rest {
        Some((count, next)) if count > 0 => format!("{whole} {unit} {count} {next}"),
        _ => format!("{whole} {unit}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_span_is_said_in_its_two_largest_units() {
        assert_eq!(span(0), "0 s");
        assert_eq!(span(37), "37 s");
        assert_eq!(span(1_195), "19 min 55 s");
        assert_eq!(span(3_600), "1 h");
        assert_eq!(span(55_897), "15 h 31 min");
        assert_eq!(span(2 * 86_400 + 19 * 3_600 + 59), "2 d 19 h");
        assert_eq!(span(6 * 365 * 86_400), "6 y");
    }

    #[test]
    fn eeros_times_read_to_the_nanosecond_and_show_in_the_machines_zone() {
        let clock = Clock {
            now: parse("2026-11-07T22:55:20Z").unwrap(),
            zone: TimeZone::fixed(jiff::tz::offset(-5)),
        };
        let reboot = parse("2026-11-07T07:22:42.035551983Z").unwrap();
        assert_eq!(clock.minute(reboot), "2026-11-07 02:22");
        assert_eq!(clock.ago(reboot), "15 h 32 min ago");
        assert_eq!(clock.ago(clock.now), "now");
    }
}
