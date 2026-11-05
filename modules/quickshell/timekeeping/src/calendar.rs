//! A day in the Chinese, Japanese, Hebrew and Islamic calendars, for the
//! launcher's Date › Calendars: each one's date in Latin letters and in
//! its own script, and its figures, both ways.
//!
//! ICU4X reckons each calendar and writes its dates from Unicode's own
//! data; the rest is tables of what ICU4X doesn't say: the Chinese year's
//! stem and branch in pinyin, its animal, the 24 solar terms, the Japanese
//! eras and rokuyō. The Islamic calendar is Umm al-Qura's, Saudi Arabia's
//! civil calendar, reckoned from its published months rather than from a
//! sighting.
//!
//! The Hebrew and Islamic days begin at sunset, so where the day is asked
//! from, it says when the sun sets there (sun.rs) and those calendars'
//! dates from then, for the shell to say whichever is now's.

use icu_calendar::cal::{ChineseTraditional, Hebrew, Hijri, Japanese};
use icu_calendar::types::{LeapStatus, RataDie};
use icu_calendar::{Date, Iso};
use icu_datetime::DateTimeFormatter;
use icu_datetime::fieldsets::{D, M, MD, Y, YMD};
use icu_locale::Locale;
use serde::{Deserialize, Serialize};
use zlink::introspect::CustomType;

use crate::sun::{self, Sunset};

/// Which calendar a date is in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, CustomType)]
#[serde(rename_all = "camelCase")]
#[zlink(rename_all = "camelCase")]
pub enum Reckoning {
    Chinese,
    Japanese,
    Hebrew,
    Islamic,
}

/// One of a calendar's figures, in Latin letters and in its own script, or
/// a day it's reckoned from, as an ISO date, for the shell to say.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, CustomType)]
#[serde(rename_all = "camelCase")]
#[zlink(rename_all = "camelCase")]
pub struct Figure {
    /// What it is: year, month, day, and the rest each calendar has.
    pub id: String,
    pub latin: String,
    pub native: String,
    pub day: Option<String>,
}

/// A day in one calendar: its date both ways, and its figures.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, CustomType)]
#[serde(rename_all = "camelCase")]
#[zlink(rename_all = "camelCase")]
pub struct Calendar {
    pub reckoning: Reckoning,
    pub latin: String,
    pub native: String,
    pub figures: Vec<Figure>,
    /// Whether its day begins at sunset rather than at midnight, as the
    /// Hebrew and the Islamic do.
    pub from_sunset: bool,
}

/// A day, as an ISO date, in every calendar, and its evening where it's
/// asked from.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, CustomType)]
#[serde(rename_all = "camelCase")]
#[zlink(rename_all = "camelCase")]
#[expect(
    clippy::struct_field_names,
    reason = "the shell reads a day's calendars by the name it says them by"
)]
pub struct Calendars {
    pub day: String,
    pub calendars: Vec<Calendar>,
    /// When the day ends for the calendars whose days begin at sunset,
    /// or none when it's asked from nowhere.
    pub evening: Option<Evening>,
    /// When the day before ends for them, or none when it's asked from
    /// nowhere: far enough north in summer, after the day's midnight.
    pub eve: Option<Eve>,
}

/// The sunset that ends a day, and the dates that begin then.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, CustomType)]
#[serde(rename_all = "camelCase")]
#[zlink(rename_all = "camelCase")]
pub struct Evening {
    /// The sunset, in milliseconds since 1970 began in UTC, or none on a
    /// day the sun doesn't set there, being up all day or down.
    pub sunset: Option<i64>,
    /// The calendars whose days begin at sunset, on the day that begins
    /// then: none on a day with no sunset, which keeps its civil date.
    pub calendars: Vec<Calendar>,
}

/// The sunset that ends the day before, and the dates it ends, which are
/// still now's until it comes where it comes after midnight.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, CustomType)]
#[serde(rename_all = "camelCase")]
#[zlink(rename_all = "camelCase")]
pub struct Eve {
    /// The sunset, in milliseconds since 1970 began in UTC, or none on a
    /// day the sun doesn't set there.
    pub sunset: Option<i64>,
    /// The calendars whose days begin at sunset, on the day before's
    /// date: none on a day with no sunset.
    pub calendars: Vec<Calendar>,
}

/// The day an ISO date names, in every calendar, and its evening where it's
/// asked from, in degrees north and east; or none if it names no day or one
/// ICU4X can't write.
pub fn on(day: &str, here: Option<(f64, f64)>) -> Option<Calendars> {
    let mut parts = day.splitn(3, '-');
    let year = parts.next()?.parse().ok()?;
    let month = parts.next()?.parse().ok()?;
    let date = parts.next()?.parse().ok()?;
    let iso = Date::try_new_iso(year, month, date).ok()?;
    Some(Calendars {
        day: iso_day(iso),
        calendars: vec![chinese(iso)?, japanese(iso)?, hebrew(iso)?, islamic(iso)?],
        evening: match here {
            Some((latitude, longitude)) => Some(evening(iso, latitude, longitude)?),
            None => None,
        },
        eve: match here {
            Some((latitude, longitude)) => Some(eve(iso, latitude, longitude)?),
            None => None,
        },
    })
}

/// The day before's sunset at a place, and the Hebrew and Islamic dates
/// until then: the day before's.
fn eve(iso: Date<Iso>, latitude: f64, longitude: f64) -> Option<Eve> {
    let before = Date::from_rata_die(iso.to_rata_die() - 1, Iso);
    let Some(sunset) = sunset(before, latitude, longitude) else {
        return Some(Eve {
            sunset: None,
            calendars: Vec::new(),
        });
    };
    Some(Eve {
        sunset: Some(sunset),
        calendars: vec![hebrew(before)?, islamic(before)?],
    })
}

/// A day's sunset at a place, and the Hebrew and Islamic dates from then:
/// the next civil day's.
fn evening(iso: Date<Iso>, latitude: f64, longitude: f64) -> Option<Evening> {
    let Some(sunset) = sunset(iso, latitude, longitude) else {
        return Some(Evening {
            sunset: None,
            calendars: Vec::new(),
        });
    };
    let next = Date::from_rata_die(iso.to_rata_die() + 1, Iso);
    Some(Evening {
        sunset: Some(sunset),
        calendars: vec![hebrew(next)?, islamic(next)?],
    })
}

/// The day 1970 began, which a sunset's milliseconds are counted from.
const EPOCH: RataDie = RataDie::new(719_163);

/// A day's sunset at a place, in milliseconds since 1970 began in UTC, or
/// none on a day the sun doesn't set there.
fn sunset(iso: Date<Iso>, latitude: f64, longitude: f64) -> Option<i64> {
    let year = iso.year().extended_year();
    let month = u32::from(iso.month().ordinal);
    let date = u32::from(iso.day_of_month().0);
    let Sunset::At(minutes) = sun::sets(year, month, date, latitude, longitude) else {
        return None;
    };
    let days = iso.to_rata_die() - EPOCH;
    #[expect(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        reason = "days since 1970 and a day's milliseconds are far within what both hold whole"
    )]
    Some(((days as f64 * sun::DAY + minutes) * 60_000.0).round() as i64)
}

/// A date as one locale writes it, in the calendar its tag names. The tag
/// is read as it's used, since ICU4X's `locale!` takes a calendar of one
/// word only, and Umm al-Qura's is two.
macro_rules! written {
    ($tag:expr, $fields:expr, $day:expr) => {
        $tag.parse::<Locale>()
            .ok()
            .and_then(|locale| DateTimeFormatter::try_new(locale.into(), $fields).ok())
            .map(|formatter| formatter.format(&$day).to_string())
    };
}

fn figure(id: &str, latin: impl Into<String>, native: impl Into<String>) -> Figure {
    Figure {
        id: id.into(),
        latin: latin.into(),
        native: native.into(),
        day: None,
    }
}

fn reckoned_from(id: &str, day: Date<Iso>) -> Figure {
    let day = iso_day(day);
    Figure {
        id: id.into(),
        latin: day.clone(),
        native: day.clone(),
        day: Some(day),
    }
}

fn iso_day(day: Date<Iso>) -> String {
    format!(
        "{:04}-{:02}-{:02}",
        day.year().extended_year(),
        day.month().ordinal,
        day.day_of_month().0
    )
}

/// The ten heavenly stems, in pinyin and in characters.
const STEMS: [(&str, &str); 10] = [
    ("Jiǎ", "甲"),
    ("Yǐ", "乙"),
    ("Bǐng", "丙"),
    ("Dīng", "丁"),
    ("Wù", "戊"),
    ("Jǐ", "己"),
    ("Gēng", "庚"),
    ("Xīn", "辛"),
    ("Rén", "壬"),
    ("Guǐ", "癸"),
];

/// The twelve earthly branches, and the animal each stands for.
const BRANCHES: [(&str, &str, &str, &str); 12] = [
    ("zǐ", "子", "Rat", "鼠"),
    ("chǒu", "丑", "Ox", "牛"),
    ("yín", "寅", "Tiger", "虎"),
    ("mǎo", "卯", "Rabbit", "兔"),
    ("chén", "辰", "Dragon", "龙"),
    ("sì", "巳", "Snake", "蛇"),
    ("wǔ", "午", "Horse", "马"),
    ("wèi", "未", "Goat", "羊"),
    ("shēn", "申", "Monkey", "猴"),
    ("yǒu", "酉", "Rooster", "鸡"),
    ("xū", "戌", "Dog", "狗"),
    ("hài", "亥", "Pig", "猪"),
];

/// The Chinese months by their number, as English names them; ICU4X's
/// English for a leap month is a code, "Sixth Monthbis".
const MONTHS: [&str; 12] = [
    "First", "Second", "Third", "Fourth", "Fifth", "Sixth", "Seventh", "Eighth", "Ninth", "Tenth",
    "Eleventh", "Twelfth",
];

/// The 24 solar terms, each 15° of the sun's path, from the spring
/// equinox at 0°.
const TERMS: [(&str, &str); 24] = [
    ("Chūnfēn", "春分"),
    ("Qīngmíng", "清明"),
    ("Gǔyǔ", "谷雨"),
    ("Lìxià", "立夏"),
    ("Xiǎomǎn", "小满"),
    ("Mángzhòng", "芒种"),
    ("Xiàzhì", "夏至"),
    ("Xiǎoshǔ", "小暑"),
    ("Dàshǔ", "大暑"),
    ("Lìqiū", "立秋"),
    ("Chǔshǔ", "处暑"),
    ("Báilù", "白露"),
    ("Qiūfēn", "秋分"),
    ("Hánlù", "寒露"),
    ("Shuāngjiàng", "霜降"),
    ("Lìdōng", "立冬"),
    ("Xiǎoxuě", "小雪"),
    ("Dàxuě", "大雪"),
    ("Dōngzhì", "冬至"),
    ("Xiǎohán", "小寒"),
    ("Dàhán", "大寒"),
    ("Lìchūn", "立春"),
    ("Yǔshuǐ", "雨水"),
    ("Jīngzhé", "惊蛰"),
];

/// The six days of the rokuyō, by the lunar month and day's sum, modulo
/// six.
const ROKUYO: [(&str, &str); 6] = [
    ("Taian", "大安"),
    ("Shakkō", "赤口"),
    ("Senshō", "先勝"),
    ("Tomobiki", "友引"),
    ("Senbu", "先負"),
    ("Butsumetsu", "仏滅"),
];

/// A Japanese era: ICU4X's code for it, its name both ways, and the day
/// it began, as year, month and day.
struct Era {
    code: &'static str,
    latin: &'static str,
    native: &'static str,
    began: (i32, u8, u8),
}

/// The eras since Meiji.
const ERAS: [Era; 5] = [
    Era {
        code: "meiji",
        latin: "Meiji",
        native: "明治",
        began: (1868, 10, 23),
    },
    Era {
        code: "taisho",
        latin: "Taishō",
        native: "大正",
        began: (1912, 7, 30),
    },
    Era {
        code: "showa",
        latin: "Shōwa",
        native: "昭和",
        began: (1926, 12, 25),
    },
    Era {
        code: "heisei",
        latin: "Heisei",
        native: "平成",
        began: (1989, 1, 8),
    },
    Era {
        code: "reiwa",
        latin: "Reiwa",
        native: "令和",
        began: (2019, 5, 1),
    },
];

fn chinese(iso: Date<Iso>) -> Option<Calendar> {
    let day = iso.to_calendar(ChineseTraditional::new());
    let cycle = usize::from(day.year().cyclic()?.year.checked_sub(1)?);
    let (stem, stem_native) = STEMS[cycle % 10];
    let (branch, branch_native, animal, animal_native) = BRANCHES[cycle % 12];
    let month = day.month();
    let leap = if month.leap_status() == LeapStatus::Leap {
        "Leap "
    } else {
        ""
    };
    let month_latin = format!(
        "{leap}{} Month",
        MONTHS.get(usize::from(month.number()) - 1)?
    );
    let days = day.days_in_month();
    let date = day.day_of_month().0;
    let (term, began) = term_on(iso.to_rata_die());
    let (term_latin, term_native) = TERMS[term];
    Some(Calendar {
        reckoning: Reckoning::Chinese,
        latin: format!("{date} {month_latin}"),
        native: format!(
            "{stem_native}{branch_native}年{}",
            written!("zh-u-ca-chinese", MD::long(), iso)?
        ),
        figures: vec![
            figure(
                "year",
                format!("{stem}{branch}"),
                format!("{stem_native}{branch_native}"),
            ),
            figure("animal", animal, animal_native),
            figure(
                "month",
                month_latin,
                written!("zh-u-ca-chinese", M::long(), iso)?,
            ),
            figure(
                "day",
                date.to_string(),
                written!("zh-u-ca-chinese", D::long(), iso)?,
            ),
            // A month of 30 days is a long one, 大月, and of 29 a short one.
            figure(
                "monthDays",
                days.to_string(),
                if days == 30 { "大月" } else { "小月" },
            ),
            figure("term", term_latin, term_native),
            reckoned_from("termBegan", Date::from_rata_die(began, Iso)),
        ],
        from_sunset: false,
    })
}

/// The solar term a day is in, and the day it began, in China's time,
/// where each term begins on the day the sun crosses its line.
fn term_on(day: RataDie) -> (usize, RataDie) {
    let term = term_at_end_of(day);
    let mut began = day;
    while term_at_end_of(began - 1) == term {
        began -= 1;
    }
    (term, began)
}

/// The solar term the sun is in at the end of a day in China, UTC+8.
fn term_at_end_of(day: RataDie) -> usize {
    #[expect(
        clippy::cast_precision_loss,
        reason = "a day's number is far within what an f64 holds whole"
    )]
    let julian_day = day.to_i64_date() as f64 + 1_721_425.5 - 8.0 / 24.0;
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a longitude from 0 up to 360, divided by 15, is a term from 0 to 23"
    )]
    let term = (sun_longitude(julian_day) / 15.0).floor() as usize;
    term % 24
}

/// The sun's apparent longitude in degrees at a Julian day, by Meeus's
/// low-precision formula (Astronomical Algorithms, chapter 25), within
/// about 0.01°, about fifteen minutes of the sun's path: a term that
/// begins that near midnight may be put a day out, though none from 2025
/// to 2030 is (solar-terms.txt, the Hong Kong Observatory's days).
fn sun_longitude(julian_day: f64) -> f64 {
    let t = (julian_day - 2_451_545.0) / 36_525.0;
    let mean = 280.466_46 + 36_000.769_83 * t + 0.000_303_2 * t * t;
    let anomaly = (357.529_11 + 35_999.050_29 * t - 0.000_153_7 * t * t).to_radians();
    let centre = (1.914_602 - 0.004_817 * t - 0.000_014 * t * t) * anomaly.sin()
        + (0.019_993 - 0.000_101 * t) * (2.0 * anomaly).sin()
        + 0.000_289 * (3.0 * anomaly).sin();
    let node = (125.04 - 1_934.136 * t).to_radians();
    (mean + centre - 0.005_69 - 0.004_78 * node.sin()).rem_euclid(360.0)
}

fn japanese(iso: Date<Iso>) -> Option<Calendar> {
    let day = iso.to_calendar(Japanese::new());
    let era = day.year().era()?;
    let Era {
        latin: name,
        native: name_native,
        began: (year, month, date),
        ..
    } = ERAS.iter().find(|each| era.era == each.code)?;
    // The rokuyō follows the old lunisolar calendar, reckoned here by
    // China's, from which Japan's differs only on the rare day a new moon
    // falls between midnight there and an hour later in Japan.
    let lunar = iso.to_calendar(ChineseTraditional::new());
    let rokuyo = (usize::from(lunar.month().number()) + usize::from(lunar.day_of_month().0)) % 6;
    let (rokuyo_latin, rokuyo_native) = ROKUYO[rokuyo];
    // An era's first year is its 元年, its "first year", rather than 1年.
    let year_native = if era.year == 1 {
        "元年".to_owned()
    } else {
        format!("{}年", era.year)
    };
    Some(Calendar {
        reckoning: Reckoning::Japanese,
        latin: format!("{name} {}", era.year),
        native: written!("ja-u-ca-japanese", YMD::long(), iso)?,
        figures: vec![
            figure("era", *name, *name_native),
            figure("eraYear", era.year.to_string(), year_native),
            reckoned_from("eraBegan", Date::try_new_iso(*year, *month, *date).ok()?),
            figure("rokuyo", rokuyo_latin, rokuyo_native),
        ],
        from_sunset: false,
    })
}

fn hebrew(iso: Date<Iso>) -> Option<Calendar> {
    let day = iso.to_calendar(Hebrew);
    let hebrew = "he-u-ca-hebrew";
    let english = "en-GB-u-ca-hebrew";
    Some(Calendar {
        reckoning: Reckoning::Hebrew,
        latin: written!(english, YMD::long(), iso)?,
        native: written!(hebrew, YMD::long(), iso)?,
        figures: vec![
            figure(
                "year",
                day.year().extended_year().to_string(),
                written!(hebrew, Y::long(), iso)?,
            ),
            figure(
                "month",
                written!(english, M::long(), iso)?,
                written!(hebrew, M::long(), iso)?,
            ),
            figure(
                "day",
                day.day_of_month().0.to_string(),
                written!(hebrew, D::long(), iso)?,
            ),
            figure(
                "yearDays",
                day.days_in_year().to_string(),
                day.days_in_year().to_string(),
            ),
            figure(
                "months",
                day.months_in_year().to_string(),
                day.months_in_year().to_string(),
            ),
        ],
        from_sunset: true,
    })
}

fn islamic(iso: Date<Iso>) -> Option<Calendar> {
    let day = iso.to_calendar(Hijri::new_umm_al_qura());
    let arabic = "ar-SA-u-ca-islamic-umalqura";
    let english = "en-GB-u-ca-islamic-umalqura";
    // The era's letter, for the Hijra, is kept beside its year by a space
    // that doesn't break, so a date that wraps never leaves it alone on a
    // line.
    let kept = |written: String| written.replace(" \u{647}\u{640}", "\u{a0}\u{647}\u{640}");
    Some(Calendar {
        reckoning: Reckoning::Islamic,
        latin: written!(english, YMD::long(), iso)?,
        native: kept(written!(arabic, YMD::long(), iso)?),
        figures: vec![
            figure(
                "year",
                written!(english, Y::long(), iso)?,
                kept(written!(arabic, Y::long(), iso)?),
            ),
            figure(
                "month",
                written!(english, M::long(), iso)?,
                written!(arabic, M::long(), iso)?,
            ),
            figure(
                "day",
                day.day_of_month().0.to_string(),
                written!(arabic, D::long(), iso)?,
            ),
            // Counts stay in the digits the shell says them in, with their
            // words, as a bare Arabic-Indic count would read as no length.
            figure(
                "monthDays",
                day.days_in_month().to_string(),
                day.days_in_month().to_string(),
            ),
            figure(
                "yearDays",
                day.days_in_year().to_string(),
                day.days_in_year().to_string(),
            ),
            // Which reckoning of the months this is, the tables Saudi Arabia
            // publishes, since a sighting of the new moon may begin a month a
            // day either side. It's named as the shell names it, in Latin
            // letters, on the plate rather than in the place's head.
            figure(
                "reckoning",
                "Umm al-Qura".to_owned(),
                "Umm al-Qura".to_owned(),
            ),
        ],
        from_sunset: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn calendar(day: &str, reckoning: Reckoning) -> Calendar {
        on(day, None)
            .and_then(|days| {
                days.calendars
                    .into_iter()
                    .find(|each| each.reckoning == reckoning)
            })
            .unwrap_or_else(|| panic!("{day} has no {reckoning:?} date"))
    }

    fn figure_of<'a>(calendar: &'a Calendar, id: &str) -> Option<&'a Figure> {
        calendar.figures.iter().find(|figure| figure.id == id)
    }

    fn said(calendar: &Calendar, id: &str) -> (String, String) {
        figure_of(calendar, id).map_or_else(
            || panic!("no {id} in {calendar:?}"),
            |figure| (figure.latin.clone(), figure.native.clone()),
        )
    }

    /// The Royal Observatory's place, in degrees north and east, as NOAA's
    /// tables give it.
    const GREENWICH: (f64, f64) = (51.4769, -0.0005);

    /// The shell's checks stand in for the reader with a day as it's seen
    /// from Greenwich, so its plates say when that day ends.
    #[test]
    fn the_shells_stand_in_is_what_the_reader_says() {
        let stood_in: Calendars = serde_json::from_str(include_str!("../calendars-reading.json"))
            .unwrap_or_else(|error| panic!("{error}"));
        assert_eq!(on(&stood_in.day, Some(GREENWICH)), Some(stood_in));
    }

    #[test]
    fn a_day_seen_from_somewhere_ends_at_its_sunset_there() {
        let day = on("2026-11-05", Some(GREENWICH)).unwrap_or_else(|| panic!("no day"));
        let evening = day.evening.unwrap_or_else(|| panic!("no evening"));
        // 16:26 in Greenwich, as NOAA's 2026 table has it, to the minute.
        let sunset = evening.sunset.unwrap_or_else(|| panic!("no sunset"));
        assert!((sunset - 1_793_895_960_000).abs() <= 60_000, "{sunset}");
        // From then it's the next day in the calendars whose days begin at
        // sunset, and in no others.
        assert_eq!(
            evening.calendars,
            vec![
                calendar("2026-11-06", Reckoning::Hebrew),
                calendar("2026-11-06", Reckoning::Islamic)
            ]
        );
        let from_sunset: Vec<_> = day
            .calendars
            .iter()
            .filter(|each| each.from_sunset)
            .map(|each| each.reckoning)
            .collect();
        assert_eq!(from_sunset, [Reckoning::Hebrew, Reckoning::Islamic]);
    }

    #[test]
    fn a_day_with_no_sunset_keeps_its_civil_date() {
        // Tromsø's midnight sun, which NOAA's table leaves without a sunset.
        let day = on("2026-06-21", Some((69.6492, 18.9553))).unwrap_or_else(|| panic!("no day"));
        assert_eq!(
            day.evening,
            Some(Evening {
                sunset: None,
                calendars: Vec::new()
            })
        );
    }

    #[test]
    fn a_sunset_after_midnight_ends_the_day_before_on_the_day() {
        // Fairbanks' 21 June sunset is at 00:48 on the 22nd in Alaska's
        // summer time, as NOAA's 2026 table has it: 08:48 UTC, after the
        // 22nd's midnight there at 08:00 UTC. Until then the Hebrew and
        // Islamic dates are still the 21st's.
        let day = on("2026-06-22", Some((64.8378, -147.7164))).unwrap_or_else(|| panic!("no day"));
        let eve = day.eve.unwrap_or_else(|| panic!("no eve"));
        let sunset = eve.sunset.unwrap_or_else(|| panic!("no sunset"));
        assert!((sunset - 1_782_118_080_000).abs() <= 60_000, "{sunset}");
        assert!(
            sunset > 1_782_115_200_000,
            "{sunset} is before the 22nd's midnight there"
        );
        assert_eq!(
            eve.calendars,
            vec![
                calendar("2026-06-21", Reckoning::Hebrew),
                calendar("2026-06-21", Reckoning::Islamic)
            ]
        );
    }

    #[test]
    fn a_day_asked_from_nowhere_has_no_evening() {
        let day = on("2026-11-05", None).unwrap_or_else(|| panic!("no day"));
        assert_eq!(day.evening, None);
        assert_eq!(day.eve, None);
    }

    /// The Hong Kong Observatory's English for each term, in TERMS' order.
    const OBSERVED: [&str; 24] = [
        "Vernal Equinox",
        "Bright & Clear",
        "Corn Rain",
        "Summer Commences",
        "Corn Forms",
        "Corn on Ear",
        "Summer Solstice",
        "Moderate Heat",
        "Great Heat",
        "Autumn Commences",
        "End of Heat",
        "White Dew",
        "Autumnal Equinox",
        "Cold Dew",
        "Frost",
        "Winter Commences",
        "Light Snow",
        "Heavy Snow",
        "Winter Solstice",
        "Moderate Cold",
        "Severe Cold",
        "Spring Commences",
        "Spring Showers",
        "Insects Waken",
    ];

    #[test]
    fn every_term_begins_on_the_day_the_observatory_says() {
        let mut wrong = Vec::new();
        let mut read = 0;
        for line in include_str!("solar-terms.txt")
            .lines()
            .filter(|line| !line.starts_with('#'))
        {
            let (day, name) = line.split_once(' ').unwrap_or_else(|| panic!("{line}"));
            let term = OBSERVED.iter().position(|each| *each == name);
            let calendars = on(day, None).unwrap_or_else(|| panic!("{day}"));
            let began =
                figure_of(&calendars.calendars[0], "termBegan").and_then(|it| it.day.clone());
            let said = figure_of(&calendars.calendars[0], "term").map(|it| it.native.clone());
            if term.map(|at| TERMS[at].1.to_owned()) != said || began.as_deref() != Some(day) {
                wrong.push(format!("{line}: {said:?} from {began:?}"));
            }
            read += 1;
        }
        assert_eq!(read, 144, "six years of 24 terms");
        assert!(wrong.is_empty(), "{}", wrong.join("\n"));
    }

    #[test]
    fn a_day_that_isnt_one_has_no_calendars() {
        assert!(on("2026-02-30", None).is_none());
        assert!(on("yesterday", Some(GREENWICH)).is_none());
    }

    #[test]
    fn the_chinese_new_year_of_the_horse() {
        let new_year = calendar("2026-02-17", Reckoning::Chinese);
        assert_eq!(new_year.latin, "1 First Month");
        assert_eq!(new_year.native, "丙午年正月初一日");
        assert_eq!(said(&new_year, "year"), ("Bǐngwǔ".into(), "丙午".into()));
        assert_eq!(said(&new_year, "animal"), ("Horse".into(), "马".into()));
    }

    #[test]
    fn a_chinese_leap_month_is_named_so() {
        let leap = calendar("2025-08-01", Reckoning::Chinese);
        assert_eq!(
            said(&leap, "month"),
            ("Leap Sixth Month".into(), "闰六月".into())
        );
    }

    #[test]
    fn the_solar_term_and_the_day_it_began() {
        // Shuangjiang began on 23 October 2025, and Lidong on 7 November.
        let frost = calendar("2025-11-05", Reckoning::Chinese);
        assert_eq!(said(&frost, "term"), ("Shuāngjiàng".into(), "霜降".into()));
        assert_eq!(said(&frost, "termBegan").0, "2025-10-23");
        let winter = calendar("2025-11-07", Reckoning::Chinese);
        assert_eq!(said(&winter, "term").1, "立冬");
        assert_eq!(said(&winter, "termBegan").0, "2025-11-07");
    }

    #[test]
    fn the_japanese_era_and_rokuyo() {
        let day = calendar("2026-11-05", Reckoning::Japanese);
        assert_eq!(day.latin, "Reiwa 8");
        assert_eq!(day.native, "令和8年11月5日");
        assert_eq!(said(&day, "eraBegan").0, "2019-05-01");
        // The lunar new year's day is always senshō.
        let new_year = calendar("2026-02-17", Reckoning::Japanese);
        assert_eq!(said(&new_year, "rokuyo"), ("Senshō".into(), "先勝".into()));
        let first = calendar("2019-05-01", Reckoning::Japanese);
        assert_eq!(said(&first, "eraYear").1, "元年");
    }

    #[test]
    fn a_hebrew_leap_year_has_thirteen_months() {
        let day = calendar("2026-11-05", Reckoning::Hebrew);
        assert_eq!(day.latin, "25 Heshvan 5787");
        assert_eq!(day.native, "כ״ה בחשוון ה׳תשפ״ז");
        assert_eq!(said(&day, "months").0, "13");
        assert_eq!(said(&day, "yearDays").0, "385");
    }

    #[test]
    fn ramadan_by_umm_al_qura() {
        let day = calendar("2023-04-10", Reckoning::Islamic);
        assert_eq!(day.latin, "19 Ramadan 1444 AH");
        assert_eq!(
            day.native,
            "\u{661}\u{669} \u{631}\u{645}\u{636}\u{627}\u{646} \u{661}\u{664}\u{664}\u{664}\u{a0}\u{647}\u{640}"
        );
        assert_eq!(said(&day, "month"), ("Ramadan".into(), "رمضان".into()));
        assert_eq!(said(&day, "day").1, "١٩");
        // The era's letter is kept beside its year, so a wrapped date never
        // leaves it alone on a line.
        assert!(day.native.ends_with("١٤٤٤\u{a0}هـ"), "{}", day.native);
        assert!(
            said(&day, "year").1.contains('\u{a0}'),
            "{:?}",
            said(&day, "year")
        );
    }
}
