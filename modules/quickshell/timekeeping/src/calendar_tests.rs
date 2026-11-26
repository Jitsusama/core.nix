//! The tests of the calendars: a day seen from somewhere and its sunset,
//! the solar terms against the observatory, and each calendar's date and
//! figures.

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
        let began = figure_of(&calendars.calendars[0], "termBegan").and_then(|it| it.day.clone());
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
    assert!(
        on("2026-02-30", None).is_none(),
        "the 30th of February was read as a day"
    );
    assert!(
        on("yesterday", Some(GREENWICH)).is_none(),
        "a word was read as a day"
    );
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
