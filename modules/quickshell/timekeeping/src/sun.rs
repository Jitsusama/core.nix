//! When the sun sets on a day, where it's asked from, for the calendars
//! whose day begins then: the Hebrew and the Islamic. It's NOAA's own
//! reckoning, the one behind its solar calculator, from Meeus's
//! Astronomical Algorithms: the sun's place to a hundredth of a degree,
//! and the sun set once its centre is 0.833 degrees below the horizon,
//! for the half degree the air bends its light up and its own radius.

use std::f64::consts::PI;

/// When the sun sets on a day, or why it doesn't.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Sunset {
    /// This many minutes after the day's midnight in UTC, which may be
    /// past 1440 where the sun sets after midnight in Greenwich.
    At(f64),
    /// The sun is up the whole day: a summer's day far enough north or
    /// south.
    Never,
    /// The sun stays down the whole day: a winter's day as far.
    Unrisen,
}

/// How far below the horizon the sun's centre is once it has set, in
/// degrees: 34 minutes of arc for the air's bending and 16 for the sun's
/// radius.
const SET: f64 = 90.833;

/// Minutes in a day, and the minutes the earth takes to turn a degree.
pub const DAY: f64 = 1440.0;
const DEGREE: f64 = 4.0;

/// When the sun sets on a civil day at a place, in degrees north and east.
/// It's reckoned from noon there, then again where it sets, since the
/// sun's place moves over the afternoon; three times sets it to well
/// under a minute.
pub fn sets(year: i32, month: u32, day: u32, latitude: f64, longitude: f64) -> Sunset {
    let midnight = julian_midnight(year, month, day);
    let noon = DAY / 2.0 - DEGREE * longitude;
    let mut minutes = noon;
    for _ in 0..3 {
        let sun = Sun::at(midnight + minutes / DAY);
        let hour = radians(SET).cos() / (radians(latitude).cos() * radians(sun.declination).cos())
            - radians(latitude).tan() * radians(sun.declination).tan();
        if hour < -1.0 {
            return Sunset::Never;
        }
        if hour > 1.0 {
            return Sunset::Unrisen;
        }
        minutes = noon - sun.equation_of_time + DEGREE * degrees(hour.acos());
    }
    Sunset::At(minutes)
}

/// The Julian day at a civil day's midnight in UTC, by Meeus's chapter 7.
fn julian_midnight(year: i32, month: u32, day: u32) -> f64 {
    let (year, month) = if month <= 2 {
        (year - 1, month + 12)
    } else {
        (year, month)
    };
    let century = (f64::from(year) / 100.0).floor();
    let gregorian = 2.0 - century + (century / 4.0).floor();
    (365.25 * (f64::from(year) + 4716.0)).floor()
        + (30.6001 * f64::from(month + 1)).floor()
        + f64::from(day)
        + gregorian
        - 1524.5
}

/// The sun as seen from the earth at an instant.
struct Sun {
    /// Degrees north of the celestial equator.
    declination: f64,
    /// Minutes the sun runs ahead of the mean sun.
    equation_of_time: f64,
}

impl Sun {
    /// The sun at a Julian day, by Meeus's chapters 25 and 28.
    fn at(julian: f64) -> Self {
        let centuries = (julian - 2_451_545.0) / 36_525.0;
        let mean_longitude =
            (280.466_46 + centuries * (36_000.769_83 + centuries * 0.000_303_2)).rem_euclid(360.0);
        let anomaly = 357.529_11 + centuries * (35_999.050_29 - 0.000_153_7 * centuries);
        let eccentricity = 0.016_708_634 - centuries * (0.000_042_037 + 0.000_000_126_7 * centuries);
        let centre = radians(anomaly).sin()
            * (1.914_602 - centuries * (0.004_817 + 0.000_014 * centuries))
            + radians(2.0 * anomaly).sin() * (0.019_993 - 0.000_101 * centuries)
            + radians(3.0 * anomaly).sin() * 0.000_289;
        let node = 125.04 - 1934.136 * centuries;
        let apparent = mean_longitude + centre - 0.005_69 - 0.004_78 * radians(node).sin();
        let seconds = 21.448 - centuries * (46.815 + centuries * (0.000_59 - centuries * 0.001_813));
        let obliquity = 23.0 + (26.0 + seconds / 60.0) / 60.0 + 0.002_56 * radians(node).cos();
        let declination = degrees((radians(obliquity).sin() * radians(apparent).sin()).asin());
        let tilt = radians(obliquity / 2.0).tan().powi(2);
        let equation_of_time = DEGREE
            * degrees(
                tilt * radians(2.0 * mean_longitude).sin()
                    - 2.0 * eccentricity * radians(anomaly).sin()
                    + 4.0
                        * eccentricity
                        * tilt
                        * radians(anomaly).sin()
                        * radians(2.0 * mean_longitude).cos()
                    - 0.5 * tilt * tilt * radians(4.0 * mean_longitude).sin()
                    - 1.25 * eccentricity * eccentricity * radians(2.0 * anomaly).sin(),
            );
        Self {
            declination,
            equation_of_time,
        }
    }
}

fn radians(degrees: f64) -> f64 {
    degrees * PI / 180.0
}

fn degrees(radians: f64) -> f64 {
    radians * 180.0 / PI
}

#[cfg(test)]
mod tests {
    use super::*;

    // Sunsets from NOAA's own 2026 tables (gml.noaa.gov/grad/solcalc,
    // table.php), given there in local time to the minute and here in
    // UTC: each place's offset, summer's included, is taken off.
    const GREENWICH: (f64, f64) = (51.4769, -0.0005);
    const FAIRBANKS: (f64, f64) = (64.8378, -147.7164);
    const TROMSO: (f64, f64) = (69.6492, 18.9553);

    fn minutes(hours: u32, minutes: u32) -> f64 {
        f64::from(hours * 60 + minutes)
    }

    fn sets_within_a_minute_of(
        (year, month, day): (i32, u32, u32),
        (latitude, longitude): (f64, f64),
        table: f64,
    ) {
        let Sunset::At(reckoned) = sets(year, month, day, latitude, longitude) else {
            panic!("no sunset on {year}-{month}-{day} at {latitude}, {longitude}");
        };
        assert!(
            (reckoned - table).abs() <= 1.0,
            "{year}-{month}-{day} at {latitude}, {longitude}: {reckoned:.1} against {table}"
        );
    }

    #[test]
    fn the_sun_sets_when_the_tables_say_in_summer_and_winter() {
        // 21:21 in British Summer Time, and 15:53 in Greenwich's own.
        sets_within_a_minute_of((2026, 6, 21), GREENWICH, minutes(20, 21));
        sets_within_a_minute_of((2026, 12, 21), GREENWICH, minutes(15, 53));
    }

    #[test]
    fn far_north_in_winter_it_sets_when_the_tables_say() {
        // 14:40 in Alaska's standard time, nine hours behind: 23:40 UTC.
        sets_within_a_minute_of((2026, 12, 21), FAIRBANKS, minutes(23, 40));
        // 00:48 in Alaska's summer time on the 22nd, eight hours behind:
        // 08:48 UTC on the 22nd, a day's minutes past the 21st's midnight.
        sets_within_a_minute_of((2026, 6, 21), FAIRBANKS, DAY + minutes(8, 48));
    }

    #[test]
    fn where_the_sun_neither_sets_nor_rises_it_says_which() {
        // The tables leave both days without a sunset.
        assert_eq!(sets(2026, 6, 21, TROMSO.0, TROMSO.1), Sunset::Never);
        assert_eq!(sets(2026, 12, 21, TROMSO.0, TROMSO.1), Sunset::Unrisen);
    }
}
