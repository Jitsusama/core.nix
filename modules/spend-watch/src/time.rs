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
#[expect(
    clippy::cast_possible_truncation,
    reason = "the floor is already whole, and hours since the epoch are far inside i64"
)]
pub fn hour_of(at: f64) -> i64 {
    (at / HOUR).floor() as i64
}

#[expect(
    clippy::cast_precision_loss,
    reason = "hours since the epoch are far below 2^52, so f64 holds them exactly"
)]
pub fn hour_start(hour: i64) -> f64 {
    hour as f64 * HOUR
}

/// The whole minute an instant falls in.
#[expect(
    clippy::cast_possible_truncation,
    reason = "the floor is already whole, and minutes since the epoch are far inside i64"
)]
pub fn minute_of(at: f64) -> i64 {
    (at / 60.0).floor() as i64
}

/// An instant in whole seconds, cut toward zero, for a key that names it.
#[expect(
    clippy::cast_possible_truncation,
    reason = "dropping the fraction of a second is the intent, and seconds since the epoch \
              are far inside i64"
)]
pub fn whole_seconds(at: f64) -> i64 {
    at as i64
}

/// The instant a whole minute since the epoch starts.
#[expect(
    clippy::cast_precision_loss,
    reason = "minutes since the epoch are far below 2^52, so f64 holds them exactly"
)]
pub fn minute_start(minute: i64) -> f64 {
    minute as f64 * 60.0
}

/// An RFC 3339 instant, as pi and the feeds write them.
pub fn parse(text: &str) -> Option<f64> {
    let stamp: Timestamp = text.parse().ok()?;
    Some(from_milliseconds(stamp.as_millisecond()))
}

pub fn rfc3339(at: f64) -> String {
    Timestamp::from_millisecond(milliseconds(at))
        .map(|stamp| stamp.to_string())
        .unwrap_or_default()
}

pub fn now() -> f64 {
    from_milliseconds(Timestamp::now().as_millisecond())
}

/// Milliseconds since the epoch as an instant in seconds.
#[expect(
    clippy::cast_precision_loss,
    reason = "milliseconds since the epoch stay below 2^53 for 285,000 years, so f64 holds \
              them exactly"
)]
pub fn from_milliseconds(milliseconds: i64) -> f64 {
    milliseconds as f64 / 1000.0
}

/// An instant in whole milliseconds since the epoch, cut toward zero.
#[expect(
    clippy::cast_possible_truncation,
    reason = "dropping the fraction of a millisecond is the intent, and an instant past \
              i64 saturates, which jiff then refuses as out of range"
)]
fn milliseconds(at: f64) -> i64 {
    (at * 1000.0) as i64
}

/// One of jiff's weekdays, hours or months as an index, since jiff never
/// gives them below zero.
fn index(value: i8) -> usize {
    usize::from(value.unsigned_abs())
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
        Timestamp::from_millisecond(milliseconds(at))
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
        let weekday = index(zoned.weekday().to_monday_zero_offset());
        weekday * 24 + index(zoned.hour())
    }

    pub fn local_hour(&self, at: f64) -> u32 {
        u32::from(self.zoned(at).hour().unsigned_abs())
    }

    /// The instant a local day starts, which daylight saving can move.
    #[expect(
        clippy::cast_precision_loss,
        reason = "days since the epoch are far below 2^52, so f64 holds them exactly"
    )]
    pub fn day_start(&self, day: i64) -> f64 {
        let date = date_of(day);
        date.to_zoned(self.tz.clone()).map_or_else(
            |_| day as f64 * DAY,
            |zoned| from_milliseconds(zoned.timestamp().as_millisecond()),
        )
    }

    /// The instant on a local day at the wall-clock time `like` shows, so
    /// "by this hour" means the same hour across a change of clocks.
    pub fn same_time(&self, day: i64, like: f64) -> f64 {
        let zoned = self.zoned(like);
        date_of(day)
            .at(zoned.hour(), zoned.minute(), zoned.second(), 0)
            .to_zoned(self.tz.clone())
            .map_or_else(
                |_| self.day_start(day) + (like - self.day_start(self.day(like))),
                |zoned| from_milliseconds(zoned.timestamp().as_millisecond()),
            )
    }

    /// The weekday of a local day, Monday 0.
    pub fn weekday(day: i64) -> usize {
        index(date_of(day).weekday().to_monday_zero_offset())
    }

    /// The Monday of the week a local day falls in.
    pub fn week_start(day: i64) -> i64 {
        day - i64::from(date_of(day).weekday().to_monday_zero_offset())
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
                weekday_name(index(zoned.weekday().to_monday_zero_offset()))
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
            weekday_name(index(date.weekday().to_monday_zero_offset())),
            date.day(),
            month_name(index(date.month()))
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
        .map_or(0, |span| i64::from(span.get_days()))
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
    #[expect(
        clippy::float_cmp,
        reason = "both instants are whole seconds, which f64 holds and subtracts exactly"
    )]
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

    // Noon in Toronto after the clocks go back is noon before, an hour of
    // UTC apart.
    #[test]
    #[expect(
        clippy::float_cmp,
        reason = "both instants are whole seconds, which f64 holds exactly"
    )]
    fn the_same_time_holds_across_a_change_of_clocks() {
        let toronto = Zone::named("America/Toronto").unwrap();
        let after = parse("2026-11-02T17:00:00Z").unwrap();
        let before = toronto.same_time(toronto.day(after) - 7, after);
        assert_eq!(before, parse("2026-10-26T16:00:00Z").unwrap());
    }
}
