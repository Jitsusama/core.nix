use super::*;

/// Two made-up access points, so no real network is named here.
const NEAR: &str = "02:00:00:00:00:0a";
const FAR: &str = "02:00:00:00:00:0b";

/// The first minute, 11:25 in UTC.
const AT: i64 = 1_795_433_100;

/// One minute's line as wifid writes it, with its video clause when
/// given, which follows the best-effort figures.
fn said_by_wifid(bssid: &str, video: Option<&str>, call: bool) -> String {
    let video = video.map(|v| format!("; video {v}")).unwrap_or_default();
    let call = if call { ", during a call" } else { "" };
    format!(
        "wlp0s20f3 on {bssid}: gateway 5.3 ms at p50, 15.0 ms at p95, 0.0% of 121 echoes \
         lost{video}; 0.00 in 100 frames sent again, 0.00 lost; signal -45 dBm; moving 11 of \
         1200 Mbit/s{call}"
    )
}

/// A video clause with its p95 and the share of its answers past a second,
/// two of its echoes never answered.
fn clause(p95: &str, late: &str) -> String {
    format!("{p95} ms at p95, {late}% of 118 answers past 1 s, 2 of 120 echoes unanswered")
}

/// The clause of a minute whose every video echo went unanswered.
const NONE: &str = "none at p95, 120 of 120 echoes unanswered";

/// One journal entry as journalctl's json prints it.
fn entry(at: i64, message: &str) -> String {
    serde_json::json!({
        "__REALTIME_TIMESTAMP": (at * 1_000_000).to_string(),
        "MESSAGE": message,
    })
    .to_string()
}

fn journal(entries: &[String]) -> String {
    entries.join("\n")
}

#[test]
fn a_minute_is_read_from_its_video_clause_and_none_is_no_hold() {
    let read = |message: &str| minute(&serde_json::from_str(&entry(AT, message)).ok()?);
    assert_eq!(
        read(&said_by_wifid(NEAR, Some(&clause("2400.0", "38.3")), true)),
        Some(Minute {
            at: AT,
            bssid: NEAR.to_owned(),
            best: 15.0,
            p95: 2400.0,
            late: 38.3,
            call: true,
        }),
        "a minute's video clause wasn't read as wifid writes it"
    );
    for (message, why) in [
        (
            said_by_wifid(NEAR, Some(NONE), false),
            "a gateway dropping every marked echo read as a hold",
        ),
        (
            said_by_wifid(NEAR, None, false),
            "a minute with no video echo sent read as one",
        ),
        (
            said_by_wifid(NEAR, Some("881.0 ms at p95, 38.3% of 120 echoes lost"), false),
            "echoes lost, which may never have been answered, read as answers late",
        ),
        (
            "wlp0s20f3 is authenticating".to_owned(),
            "a line that isn't a minute read as one",
        ),
    ] {
        assert_eq!(read(&message), None, "{why}");
    }
}

#[test]
fn held_minutes_are_counted_by_access_point_with_each_ones_worst() {
    let on = |bssid: &str, n: i64, video: &str, call: bool| {
        entry(AT + 60 * n, &said_by_wifid(bssid, Some(video), call))
    };
    let text = journal(&[
        on(NEAR, 0, &clause("2400.0", "38.3"), false),
        on(NEAR, 1, &clause("12.0", "0.0"), false),
        // Held by its delay alone.
        on(NEAR, 2, &clause("150.0", "0.5"), false),
        // Held by its late answers alone, while a call was on.
        on(FAR, 3, &clause("20.0", "2.5"), true),
        on(FAR, 4, &clause("10.0", "0.0"), false),
        // Neither read nor held, as wifid doesn't weigh it either.
        on(FAR, 5, NONE, false),
        entry(AT + 310, "wlp0s20f3 is authenticating"),
    ]);
    let m = video(Some(&text), &TimeZone::UTC);
    assert_eq!(
        (m.key.as_str(), m.class, m.owner.as_str()),
        (KEY, Class::Apps, "PN2VZC"),
        "the held minutes weren't one of his apps, moved by wifid's owner"
    );
    assert_eq!(
        m.says,
        "minutes the Wi-Fi held a call's video past 100 ms at p95 or over 1% of it past 1 s, \
         this hour, of 5 read"
    );
    assert!(
        (m.now - 3.0).abs() < f64::EPSILON && m.target == Some(TARGET) && m.past().is_some(),
        "three held minutes in the hour didn't read as past its target: {m:?}"
    );
    assert_eq!(
        m.behind,
        [
            format!(
                "on {NEAR}, 2 of 3 minutes held, the worst 2400.0 ms at p95 and 38.3% past 1 s \
                 at 11:25, best effort then 15.0 ms"
            ),
            format!(
                "on {FAR}, 1 of 2 minutes held, the worst 20.0 ms at p95 and 2.5% past 1 s at \
                 11:28, during a call, best effort then 15.0 ms"
            ),
        ]
    );
}

#[test]
fn one_moments_two_held_minutes_are_within_and_none_read_is_unjudged_saying_why() {
    let (held, quick) = (clause("300.0", "0.0"), clause("9.0", "0.0"));
    let moment = journal(&[
        entry(AT, &said_by_wifid(NEAR, Some(&held), false)),
        entry(AT + 60, &said_by_wifid(NEAR, Some(&held), false)),
        entry(AT + 120, &said_by_wifid(NEAR, Some(&quick), false)),
    ]);
    let m = video(Some(&moment), &TimeZone::UTC);
    assert!(
        (m.now - 2.0).abs() < f64::EPSILON && m.past().is_none(),
        "one bad moment, in the two lines covering it, was filed with wifid's owner: {m:?}"
    );
    let before = journal(&[entry(AT, &said_by_wifid(NEAR, None, false))]);
    for (text, why) in [
        (None, ": wifid's journal unread"),
        (Some(""), ": no Wi-Fi minute"),
        (Some("wlp0s20f3 is done looking"), ": no Wi-Fi minute"),
        (Some(before.as_str()), ": no video echo answered"),
    ] {
        let m = video(text, &TimeZone::UTC);
        assert!(
            m.target.is_none() && m.says.ends_with(why),
            "an hour with no minute read didn't say {why:?} unjudged: {m:?}"
        );
    }
}

#[test]
fn the_journal_is_read_for_the_hour_without_a_grep() {
    let args = super::journal(AT);
    assert!(
        args.contains(&"wifid.service".to_owned())
            && args.contains(&format!("--since=@{}", AT - 3600))
            && args.contains(&format!("--until=@{AT}"))
            && !args.iter().any(|a| a.starts_with("--grep")),
        "journalctl isn't asked for wifid's whole hour: {args:?}"
    );
}
