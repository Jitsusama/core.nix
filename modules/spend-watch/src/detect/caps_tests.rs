//! The tests of what the caps find: a window named with its allowance, an
//! override's end unknown, guessed, passed or close, the reach said by its
//! day, and a cap's fall told only while it stays down.

use super::*;
use crate::detect::tests::{NOW, config};
use crate::history::History;

// The premium day as a feed reads it: its allowance at a moment.
fn premium(history: &History, at: f64, allowance: f64) {
    let line = serde_json::json!({
        "v": 1, "kind": "cap", "name": "premium", "label": "Premium Day",
        "allowance": allowance, "windowHours": 24, "spent": 0, "warn": 0.8,
        "state": "ok", "at": crate::time::rfc3339(at),
    });
    let record = crate::feed::parse(&line.to_string()).unwrap();
    crate::feed::store(history, "proxy", &record, at).unwrap();
}

fn cap_titles(history: &History, now: f64) -> Vec<String> {
    let picture = Picture::read(history, &config(), now).unwrap();
    caps(&picture)
        .into_iter()
        .map(|found| found.title)
        .collect()
}

// A window is named in a sentence as its feed names it, with the
// glossary's word for what it is after it.
#[test]
fn a_window_s_name_is_said_with_its_allowance() {
    let history = History::memory();
    let line = serde_json::json!({
        "v": 1, "kind": "cap", "name": "five-hours", "label": "Five Hours",
        "unit": "share", "used": 0.85, "windowHours": 5, "warn": 0.8, "state": "ok",
        "resets": crate::time::rfc3339(NOW + 2.0 * HOUR),
        "at": crate::time::rfc3339(NOW - 60.0),
    });
    let record = crate::feed::parse(&line.to_string()).unwrap();
    crate::feed::store(&history, "proxy", &record, NOW - 60.0).unwrap();
    let picture = Picture::read(&history, &config(), NOW).unwrap();
    let found = caps(&picture);
    assert_eq!(
        found
            .iter()
            .map(|found| (found.title.as_str(), found.detail.split(',').next()))
            .collect::<Vec<_>>(),
        [(
            "near the Five Hours allowance",
            Some("85% of the Five Hours allowance used")
        )]
    );
}

/// The weekly cap's findings when raised to $40,000 over a $4,000
/// fallback, with `spent` spent and the override ending `ends`, known
/// when given and unknown when not.
fn raised(spent: f64, ends: Option<&str>) -> Vec<Finding> {
    unending(&raised_picture(spent, ends, ends.is_some()))
}

fn unending(picture: &Picture) -> Vec<Finding> {
    caps(picture)
        .into_iter()
        .filter(|found| found.key.ends_with(":unending"))
        .collect()
}

/// The picture of that cap, its end given as `ends`, known or not.
fn raised_picture(spent: f64, ends: Option<&str>, known: bool) -> Picture {
    let history = History::memory();
    let mut line = serde_json::json!({
        "v": 1, "kind": "cap", "name": "weekly", "label": "Week",
        "allowance": 40_000, "windowHours": 168, "spent": spent, "warn": 0.9,
        "fallback": 4000, "state": "ok", "at": crate::time::rfc3339(NOW - 60.0),
    });
    if let Some(ends) = ends {
        line["ends"] = ends.into();
    }
    // Left out when false, as a feed that can't know the end may.
    if known {
        line["endsKnown"] = true.into();
    }
    let record = crate::feed::parse(&line.to_string()).unwrap();
    crate::feed::store(&history, "proxy", &record, NOW - 60.0).unwrap();
    Picture::read(&history, &config(), NOW).unwrap()
}

// A raise whose end nobody knows, over a fallback already spent, is
// warned: the cap refuses the moment it ends, and no day can be warned
// ahead of. Under the fallback, or with its end known, it isn't.
#[test]
fn a_raise_with_no_known_end_over_what_follows_is_warned() {
    let found = raised(33_671.0, None);
    assert_eq!(found.len(), 1, "{found:?}");
    let found = &found[0];
    assert_eq!(
        (found.title.as_str(), found.severity, found.cap),
        (
            "the Week allowance override's end is unknown",
            Severity::Warning,
            true
        )
    );
    // Without a clock there's no reach to say, so the detail ends there.
    assert_eq!(
        found.detail,
        "Nobody knows when the $40,000 override ends, and the $33,671 spent is \
         already over the $4,000 that follows it, so the cap refuses the moment \
         it ends."
    );
    assert_eq!(found.dollars, 29_671.0);
    assert_eq!(found.badge, "end unknown");
    assert!(raised(3_999.0, None).is_empty(), "under the fallback");
    assert_eq!(raised(4_000.0, None).len(), 1, "at the fallback");
    assert!(
        raised(33_671.0, Some("2026-10-20")).is_empty(),
        "its end is known"
    );
}

// A feed that could know the end and gives none says the raise has
// none, so nothing follows it to warn of.
#[test]
fn a_raise_known_to_have_no_end_is_not_warned() {
    let found = unending(&raised_picture(33_671.0, None, true));
    assert!(found.is_empty(), "{found:?}");
}

// A known end already past is said to have passed where the margin
// shows it first, not to be unknown.
#[test]
fn a_passed_end_is_said_to_have_passed() {
    let found = unending(&raised_picture(33_671.0, Some("2026-10-10"), true));
    assert_eq!(
        found
            .iter()
            .map(|found| (found.title.as_str(), found.badge.as_str()))
            .collect::<Vec<_>>(),
        [("the Week allowance override's end has passed", "end passed")]
    );
}

// A guessed end is no end known, and nor is a known one already past
// while the override stands: each is warned, saying which it is.
#[test]
fn a_guessed_or_passed_end_is_said_so() {
    let detail = |ends: &str, known: bool| {
        let found = unending(&raised_picture(33_671.0, Some(ends), known));
        assert_eq!(found.len(), 1, "{ends} {known}: {found:?}");
        found[0].detail.clone()
    };
    assert!(
        detail("2026-10-20", false).starts_with(
            "Nobody knows when the $40,000 override ends (its end, Tue 20 Oct, is a guess),"
        ),
        "{}",
        detail("2026-10-20", false)
    );
    assert!(
        detail("2026-10-10", true).starts_with(
            "The $40,000 override's end, Sat 10 Oct, has passed, but the feed still reads \
             it and may not have run since, and the $33,671 spent is already over the \
             $4,000 that follows it, so the cap may already refuse."
        ),
        "{}",
        detail("2026-10-10", true)
    );
    assert!(
        detail("2026-10-10", false).starts_with(
            "Nobody knows when the $40,000 override ends (its end, Sat 10 Oct, a guess, \
             has passed),"
        ),
        "{}",
        detail("2026-10-10", false)
    );
}

// With a clock, the warning and the near one say by when the allowance
// is reached at this pace, as a day and a time that don't move, and
// the hour that pace was judged in, since a notice keeps its first
// telling while the pace moves on.
#[test]
fn the_reach_is_said_by_its_day() {
    let mut picture = raised_picture(37_000.0, None, false);
    picture.caps[0].clock = Some(crate::caps::Clock {
        reaches_at: Some(NOW + 36.5 * HOUR),
        ..crate::caps::Clock::default()
    });
    let found = caps(&picture);
    let detail = |what: &str| {
        found
            .iter()
            .find(|found| found.key.ends_with(what))
            .map(|found| found.detail.clone())
            .unwrap_or_default()
    };
    assert!(
        detail(":unending")
            .ends_with(" At Wed 14 Oct 14:00's pace it reaches $40,000 by Fri 16 Oct 03:00."),
        "{found:?}"
    );
    assert!(
        detail(":near").ends_with(" At Wed 14 Oct 14:00's pace it's reached by Fri 16 Oct 03:00."),
        "{found:?}"
    );
}

/// What the weekly cap says, keys and details, with $37,000 spent, its
/// end `ends`, known or not, and a clock reaching it by `reaches` and
/// over what follows the end on every path.
fn ending_in(ends: &str, known: bool, reaches: f64) -> Vec<(String, String)> {
    spent_ending_in(37_000.0, ends, known, reaches)
}

/// The same, with `spent` spent.
fn spent_ending_in(spent: f64, ends: &str, known: bool, reaches: f64) -> Vec<(String, String)> {
    let mut picture = raised_picture(spent, Some(ends), known);
    picture.caps[0].clock = Some(crate::caps::Clock {
        reaches_at: Some(reaches),
        over_after_end: Some(1.0),
        ..crate::caps::Clock::default()
    });
    caps(&picture)
        .into_iter()
        .map(|found| {
            (
                found.key.rsplit(':').next().unwrap_or_default().into(),
                found.detail,
            )
        })
        .collect()
}

// A guessed end a day off is the unknown-end warning's alone, with its
// reach past the guess, since the guess doesn't end it; the ending
// warning would say as fact what nobody knows.
#[test]
fn a_guessed_end_close_by_is_not_said_to_be_ending() {
    let said = ending_in("2026-10-15", false, NOW + 36.5 * HOUR);
    let keys: Vec<&str> = said.iter().map(|(key, _)| key.as_str()).collect();
    assert!(
        keys.contains(&"unending") && !keys.contains(&"ending"),
        "{said:?}"
    );
    let unending = &said.iter().find(|(key, _)| key == "unending").unwrap().1;
    assert!(unending.ends_with(" by Fri 16 Oct 03:00."), "{unending}");
}

// Under the fallback there's no unknown-end warning, so a guessed end
// close by is still warned of as it nears, said to be a guess.
#[test]
fn a_guessed_end_close_by_under_what_follows_is_said_to_be_a_guess() {
    let said = spent_ending_in(3_000.0, "2026-10-15", false, NOW + 36.5 * HOUR);
    let keys: Vec<&str> = said.iter().map(|(key, _)| key.as_str()).collect();
    assert!(!keys.contains(&"unending"), "{said:?}");
    let ending = &said.iter().find(|(key, _)| key == "ending").unwrap().1;
    assert!(
        ending.starts_with(
            "The $40,000 override ends Thu 15 Oct, by the feed's guess. At Wed 14 Oct \
             14:00's pace, a "
        ),
        "{ending}"
    );
}

// A known end already past doesn't hide a reach after it, since the
// feed still reads the override.
#[test]
fn a_passed_end_keeps_its_reach() {
    let said = ending_in("2026-10-10", true, NOW + 36.5 * HOUR);
    let unending = &said.iter().find(|(key, _)| key == "unending").unwrap().1;
    assert!(unending.ends_with(" by Fri 16 Oct 03:00."), "{unending}");
}

// A known end a day off is said to be ending, and a reach past it isn't
// said, since by then the allowance is the fallback's.
#[test]
fn a_known_end_close_by_hides_a_reach_past_it() {
    let said = ending_in("2026-10-15", true, NOW + 36.5 * HOUR);
    let keys: Vec<&str> = said.iter().map(|(key, _)| key.as_str()).collect();
    assert!(
        keys.contains(&"ending") && !keys.contains(&"unending"),
        "{said:?}"
    );
    let near = &said.iter().find(|(key, _)| key == "near").unwrap().1;
    assert!(!near.contains("pace it's reached"), "{near}");
    let before = ending_in("2026-10-15", true, NOW + 30.5 * HOUR);
    let near = &before.iter().find(|(key, _)| key == "near").unwrap().1;
    assert!(near.ends_with(" by Thu 15 Oct 21:00."), "{near}");
}

// A reach on the hour a known end comes is still the raise's to say,
// since the allowance stands until that moment, and the hour after
// isn't. An end on 15 October is the start of the 16th, 33.5 hours on.
#[test]
fn a_reach_at_a_known_end_is_said() {
    let at = ending_in("2026-10-15", true, NOW + 33.5 * HOUR);
    let near = &at.iter().find(|(key, _)| key == "near").unwrap().1;
    assert!(near.ends_with(" by Fri 16 Oct 00:00."), "{near}");
    let after = ending_in("2026-10-15", true, NOW + 34.5 * HOUR);
    let near = &after.iter().find(|(key, _)| key == "near").unwrap().1;
    assert!(!near.contains("pace it's reached"), "{near}");
}

// A fall is told once it has stood, and only while the cap stays down:
// once it's back, the fall is past and its return is the news.
#[test]
fn a_fall_is_told_while_the_cap_stays_down() {
    let history = History::memory();
    let fell = NOW - 3.0 * HOUR;
    premium(&history, fell - 60.0, 1000.0);
    premium(&history, fell, 300.0);
    assert_eq!(
        cap_titles(&history, fell + 60.0),
        [] as [std::string::String; 0]
    );
    assert_eq!(cap_titles(&history, fell + 900.0), ["the cap fell to $300"]);
    premium(&history, NOW - HOUR, 1000.0);
    assert_eq!(cap_titles(&history, NOW), ["the cap rose to $1,000"]);
}

// The meter's premium day said $300 for one reading and $1,000 again
// thirty seconds later; that's nothing to tell.
#[test]
fn a_cap_that_flickers_is_not_told() {
    let history = History::memory();
    let at = NOW - HOUR;
    premium(&history, at - 240.0, 1000.0);
    premium(&history, at, 300.0);
    premium(&history, at + 30.0, 1000.0);
    assert_eq!(
        cap_titles(&history, at + 15.0),
        [] as [std::string::String; 0]
    );
    assert_eq!(cap_titles(&history, NOW), [] as [std::string::String; 0]);
}
