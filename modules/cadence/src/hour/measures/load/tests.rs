use std::fmt::Write as _;

use super::*;

/// The end of the hour judged, on a minute's start.
const NOW: i64 = 1_794_700_800;

/// A ten-second window ending at `ended`, each slice's stalls given by
/// `stalls` as (group, cpu some, memory full, io full), and the memory
/// left, when there is any.
fn window(ended: i64, stalls: &[(&str, f64, f64, f64)], left_mib: Option<u64>) -> String {
    let stalls: serde_json::Map<String, Value> = stalls
        .iter()
        .map(|&(group, cpu, memory, io)| {
            let stall = serde_json::json!({
                "cpu_some": cpu, "cpu_full": 0.0, "memory_some": memory,
                "memory_full": memory, "io_some": io, "io_full": io
            });
            (group.to_owned(), stall)
        })
        .collect();
    let mut window = serde_json::json!({"ended": ended, "seconds": 10.0, "stalls": stalls});
    if let Some(mib) = left_mib {
        window["available_mib"] = mib.into();
    }
    window.to_string()
}

/// Every slice quiet, 32 GiB left.
const QUIET: [(&str, f64, f64, f64); 4] = [
    ("app", 0.0, 0.0, 0.0),
    ("session", 0.5, 0.0, 0.0),
    ("agents", 4.0, 0.0, 0.0),
    ("machine", 10.0, 0.0, 0.0),
];

/// The hour's windows, six a minute for each of `minutes` minutes before
/// `NOW`, each given by `each` from its minute, counted from the hour's
/// first.
fn hour(
    minutes: i64,
    each: impl Fn(i64) -> (Vec<(&'static str, f64, f64, f64)>, Option<u64>),
) -> String {
    (0..minutes).fold(String::new(), |mut text, minute| {
        let (stalls, left) = each(minute);
        for tick in 1..=6 {
            let ended = NOW - 3600 + minute * 60 + tick * 10 - 1;
            writeln!(text, "{}", window(ended, &stalls, left)).expect("a string takes a line");
        }
        text
    })
}

#[test]
fn a_quiet_hour_is_judged_with_no_minute_past() {
    let text = hour(60, |_| (QUIET.to_vec(), Some(32 << 10)));
    let m = load(&text, NOW);
    assert_eq!(
        (m.now, m.target, m.behind.len()),
        (0.0, Some(0.0), 0),
        "a quiet hour wasn't judged quiet: {m:?}"
    );
}

#[test]
fn each_minute_past_any_limit_counts_once_naming_each_limit_by_its_minutes_and_worst() {
    // The agents wait 30% of minutes 0 to 11, 38% in minute 5; memory left
    // falls to 9 GiB in minutes 10 to 12, so two of those overlap.
    let text = hour(60, |minute| {
        let mut stalls = QUIET.to_vec();
        if minute < 12 {
            stalls[2].1 = if minute == 5 { 38.0 } else { 30.0 };
        }
        let left = if (10..13).contains(&minute) {
            9 << 10
        } else {
            32 << 10
        };
        (stalls, Some(left))
    });
    let m = load(&text, NOW);
    assert_eq!(
        m.now, 13.0,
        "minutes past a limit weren't counted once each: {m:?}"
    );
    assert_eq!(
        m.behind,
        [
            "the agents waiting for a CPU over 15%: 12 min, worst 38%",
            "memory left under 16 GiB: 3 min, least 9 GiB",
        ],
        "the limits past weren't each named, most minutes first"
    );
}

#[test]
fn a_minute_is_judged_by_its_windows_average_not_its_worst() {
    // One window of six at 9% averages 1.5%, under his apps' 2%; two at
    // 9% average 3%, over it.
    let mut text = String::new();
    for minute in 0..60 {
        for tick in 1..=6 {
            let ended = NOW - 3600 + minute * 60 + tick * 10 - 1;
            let mut stalls = QUIET.to_vec();
            if tick <= 1 + i64::from(minute == 0) {
                stalls[0].1 = 9.0;
            }
            writeln!(text, "{}", window(ended, &stalls, Some(32 << 10))).expect("a line");
        }
    }
    let m = load(&text, NOW);
    assert_eq!(
        m.now, 1.0,
        "a minute was judged by more than its average: {m:?}"
    );
}

#[test]
fn an_hour_too_little_watched_or_missing_a_part_is_unread_never_passing() {
    let half = hour(30, |_| (QUIET.to_vec(), Some(32 << 10)));
    let older = hour(60, |_| (QUIET[..3].to_vec(), Some(32 << 10)));
    let unsaid = hour(60, |_| (QUIET.to_vec(), None));
    for (text, why) in [
        (half.as_str(), "30 of 60 minutes watched by lag-watch"),
        (
            older.as_str(),
            "the machine waiting for a CPU read in 0 of 60 minutes",
        ),
        (unsaid.as_str(), "the memory left read in 0 of 60 minutes"),
        ("", "0 of 60 minutes watched by lag-watch"),
    ] {
        let m = load(text, NOW);
        assert_eq!(m.target, None, "an hour short of {why} was judged: {m:?}");
        assert!(
            m.says.ends_with(why),
            "an unread hour didn't say why: {}",
            m.says
        );
    }
}

#[test]
fn windows_outside_the_hour_or_not_windows_are_left_out() {
    let mut text = hour(60, |_| (QUIET.to_vec(), Some(32 << 10)));
    let mut loaded = QUIET.to_vec();
    loaded[2].1 = 90.0;
    for ended in [NOW - 3600, NOW + 10] {
        writeln!(text, "{}", window(ended, &loaded, Some(1 << 10))).expect("a line");
    }
    text.push_str("not a window\n{\"ended\": \"soon\"}\n");
    let m = load(&text, NOW);
    assert_eq!(
        (m.now, m.target),
        (0.0, Some(0.0)),
        "a window outside the hour, or a line not one, was judged: {m:?}"
    );
}
