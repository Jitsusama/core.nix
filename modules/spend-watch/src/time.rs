//! Instants are seconds since the epoch, as f64, and hours are whole hours
//! since it; days and the hour of the week are local to the machine's
//! zone, which is where the person lives their days, so a day is the one
//! they'd name, across midnight and daylight saving alike.

use jiff::civil::Date;
use jiff::tz::TimeZone;
use jiff::{Timestamp, Zoned};

pub const HOUR: f64 = 3600.0;
pub const DAY: f64 = 86_400.0;

/// The whole hour an instant falls in.
pub fn hour_of(at: f64) -> i64 {
    (at / HOUR).floor() as i64
}

pub fn hour_start(hour: i64) -> f64 {
    hour as f64 * HOUR
}

/// An RFC 3339 instant, as pi and the feeds write them.
pub fn parse(text: &str) -> Option<f64> {
    let stamp: Timestamp = text.parse().ok()?;
    Some(stamp.as_millisecond() as f64 / 1000.0)
}

pub fn rfc3339(at: f64) -> String {
    Timestamp::from_millisecond((at * 1000.0) as i64)
        .map(|stamp| stamp.to_string())
        .unwrap_or_default()
}

pub fn now() -> f64 {
    Timestamp::now().as_millisecond() as f64 / 1000.0
}

#[derive(Debug, Clone)]
pub struct Zone {
    tz: TimeZone,
}

impl Zone {
    pub fn named(name: &str) -> Result<Zone, String> {
        TimeZone::get(name)
            .map(|tz| Zone { tz })
            .map_err(|error| format!("no zone {name}: {error}"))
    }

    /// The zone the machine keeps, as /etc/localtime or TZ says.
    pub fn system() -> Zone {
        Zone {
            tz: TimeZone::system(),
        }
    }

    pub fn utc() -> Zone {
        Zone { tz: TimeZone::UTC }
    }

    fn zoned(&self, at: f64) -> Zoned {
        Timestamp::from_millisecond((at * 1000.0) as i64)
            .unwrap_or(Timestamp::UNIX_EPOCH)
            .to_zoned(self.tz.clone())
    }

    /// The local day an instant falls in, as days since 1970-01-01, so
    /// days are plain integers to count and compare.
    pub fn day(&self, at: f64) -> i64 {
        day_number(self.zoned(at).date())
    }

    /// The local hour of the week: Monday's first hour is 0.
    pub fn slot(&self, at: f64) -> usize {
        let zoned = self.zoned(at);
        let weekday = zoned.weekday().to_monday_zero_offset() as usize;
        weekday * 24 + zoned.hour() as usize
    }

    pub fn local_hour(&self, at: f64) -> u32 {
        self.zoned(at).hour() as u32
    }

    /// The instant a local day starts, which daylight saving can move.
    pub fn day_start(&self, day: i64) -> f64 {
        let date = date_of(day);
        date.to_zoned(self.tz.clone())
            .map(|zoned| zoned.timestamp().as_millisecond() as f64 / 1000.0)
            .unwrap_or(day as f64 * DAY)
    }

    /// The weekday of a local day, Monday 0.
    pub fn weekday(day: i64) -> usize {
        date_of(day).weekday().to_monday_zero_offset() as usize
    }

    /// How a local instant reads to a person: "14:00", or "Thu 14:00" when
    /// it isn't today.
    pub fn clock(&self, at: f64, now: f64) -> String {
        let zoned = self.zoned(at);
        let time = format!("{:02}:{:02}", zoned.hour(), zoned.minute());
        let days = self.day(at) - self.day(now);
        if days == 0 {
            time
        } else if days == 1 {
            format!("{time} tomorrow")
        } else if days.abs() < 7 {
            format!(
                "{} {time}",
                weekday_name(zoned.weekday().to_monday_zero_offset() as usize)
            )
        } else {
            // A weekday alone would name the wrong week.
            format!("{} {time}", Zone::day_name(self.day(at)))
        }
    }

    /// A day as a person names it: "Mon 12 Oct".
    pub fn day_name(day: i64) -> String {
        let date = date_of(day);
        format!(
            "{} {} {}",
            weekday_name(date.weekday().to_monday_zero_offset() as usize),
            date.day(),
            month_name(date.month() as usize)
        )
    }

    pub fn iso_day(day: i64) -> String {
        date_of(day).to_string()
    }
}

pub fn parse_day(text: &str) -> Option<i64> {
    text.parse::<Date>().ok().map(day_number)
}

fn day_number(date: Date) -> i64 {
    date.since(Date::constant(1970, 1, 1))
        .map(|span| span.get_days() as i64)
        .unwrap_or(0)
}

fn date_of(day: i64) -> Date {
    Date::constant(1970, 1, 1)
        .checked_add(jiff::Span::new().days(day))
        .unwrap_or(Date::constant(1970, 1, 1))
}

pub fn weekday_name(weekday: usize) -> &'static str {
    ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"][weekday % 7]
}

fn month_name(month: usize) -> &'static str {
    [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ][(month + 11) % 12]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_day_is_local() {
        let toronto = Zone::named("America/Toronto").unwrap();
        // 03:30 UTC on 14 October is still the 13th in Toronto.
        let at = parse("2026-10-14T03:30:00Z").unwrap();
        assert_eq!(Zone::iso_day(toronto.day(at)), "2026-10-13");
        assert_eq!(Zone::iso_day(Zone::utc().day(at)), "2026-10-14");
    }

    #[test]
    fn a_day_across_daylight_saving_is_its_own_length() {
        let toronto = Zone::named("America/Toronto").unwrap();
        let day = parse_day("2026-11-01").unwrap();
        let length = toronto.day_start(day + 1) - toronto.day_start(day);
        assert_eq!(length, 25.0 * HOUR);
    }

    #[test]
    fn the_week_starts_on_monday() {
        let toronto = Zone::named("America/Toronto").unwrap();
        let monday_nine = parse("2026-10-12T13:00:00Z").unwrap();
        assert_eq!(toronto.slot(monday_nine), 9);
        assert_eq!(Zone::day_name(toronto.day(monday_nine)), "Mon 12 Oct");
    }

    #[test]
    fn a_clock_names_its_day_when_it_isnt_today() {
        let toronto = Zone::named("America/Toronto").unwrap();
        let now = parse("2026-10-14T16:00:00Z").unwrap();
        assert_eq!(toronto.clock(now + 2.0 * HOUR, now), "14:00");
        assert_eq!(toronto.clock(now + DAY, now), "12:00 tomorrow");
        assert_eq!(toronto.clock(now + 2.0 * DAY, now), "Fri 12:00");
        assert_eq!(toronto.clock(now - 23.0 * DAY, now), "Mon 21 Sep 12:00");
    }
}
