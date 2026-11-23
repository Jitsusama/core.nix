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

/// A video clause with its p95 and the share of its echoes answered past
/// a second, two of them never answered.
fn clause(p95: &str, late: &str) -> String {
    format!("{p95} ms at p95, {late}% of 120 echoes answered past 1 s, 2 unanswered")
}

/// The clause of a minute whose every video echo went unanswered.
const NONE: &str = "none at p95, 0.0% of 120 echoes answered past 1 s, 120 unanswered, too few \
                    to time";

/// The clause of a minute with fewer than half its video echoes answered,
/// many of those few late.
const FEW: &str = "2400.0 ms at p95, 37.5% of 120 echoes answered past 1 s, 70 unanswered, too \
                   few to time";

/// The clause as wifid's first version wrote it, its loss unsplit.
const LOST: &str = "881.0 ms at p95, 38.3% of 120 echoes lost";

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
fn a_minute_whose_video_went_mostly_unanswered_is_set_aside_not_clean() {
    let read = |message: &str| minute(&serde_json::from_str(&entry(AT, message)).ok()?);
    // Mostly unanswered, judged only by answers late past 1%.
    let few = |late: &str| {
        let clause = format!(
            "none at p95, {late}% of 120 echoes answered past 1 s, 70 unanswered, too few to time"
        );
        read(&said_by_wifid(NEAR, Some(&clause), false))
            .as_ref()
            .is_some_and(Minute::answered)
    };
    assert!(
        !few("1.0") && few("1.1"),
        "a minute mostly unanswered wasn't judged by its late answers past 1%"
    );
}

/// A minute held 300 ms at p95 while every best-effort echo went
/// unanswered.
fn dark(n: i64) -> String {
    let held = said_by_wifid(NEAR, Some(&clause("300.0", "0.0")), false)
        .replace("5.3 ms at p50, 15.0 ms at p95", "none at p50, none at p95");
    entry(AT + 60 * n, &held)
}

#[test]
fn minutes_not_judged_are_counted_by_why_and_too_many_leave_the_hour_unjudged() {
    let on = |n: i64, video: Option<&str>| entry(AT + 60 * n, &said_by_wifid(NEAR, video, false));
    let quick = clause("9.0", "0.0");
    let unread = clause("soon", "0.0");
    let mixed = journal(&[
        dark(0),
        on(1, Some(&quick)),
        on(2, Some(&quick)),
        on(3, Some(NONE)),
        on(4, Some(&unread)),
        on(5, Some(LOST)),
    ]);
    let m = video(Some(&mixed), &TimeZone::UTC);
    assert_eq!(
        (m.says.as_str(), m.now.to_string(), m.behind.as_slice()),
        (
            "minutes the Wi-Fi held a call's video past 100 ms at p95 or over 1% of it past 1 s, \
             this hour, 3 of 6 Wi-Fi minutes judged (1 set aside, their video unanswered; 1 \
             unread; 1 from a wifid writing no video figure)",
            "1".to_owned(),
            [format!(
                "on {NEAR}, 1 of 3 minutes held, the worst 300.0 ms at p95 and 0.0% past 1 s at \
                 11:25, best effort then unanswered"
            )]
            .as_slice(),
        ),
        "a mixed hour's minutes weren't each counted by why, or its held one misnamed"
    );
    let sliver = journal(&[
        dark(0),
        on(1, Some(NONE)),
        on(2, Some(NONE)),
        on(3, Some("unmeasured")),
    ]);
    let m = video(Some(&sliver), &TimeZone::UTC);
    assert!(
        m.target.is_none()
            && m.says.ends_with(
                ": 1 of 4 Wi-Fi minutes judged (2 set aside, their video unanswered; 1 with no \
                 video echo sent)"
            ),
        "an hour judged on under half its minutes was judged: {m:?}"
    );
    let past: Vec<String> = (0..10)
        .map(|n| if n < 3 { dark(n) } else { on(n, Some(NONE)) })
        .collect();
    let m = video(Some(&journal(&past)), &TimeZone::UTC);
    assert!(
        m.past().is_some(),
        "three held minutes went unjudged for the minutes set aside beside them: {m:?}"
    );
}

#[test]
fn a_minute_is_read_from_its_video_clause_and_none_is_no_hold() {
    let read = |message: &str| minute(&serde_json::from_str(&entry(AT, message)).ok()?);
    assert_eq!(
        read(&said_by_wifid(NEAR, Some(&clause("2400.0", "37.5")), true)),
        Some(Minute {
            at: AT,
            bssid: NEAR.to_owned(),
            best: Some(15.0),
            p95: Some(2400.0),
            late: 37.5,
            call: true,
        }),
        "a minute's video clause wasn't read as wifid writes it"
    );
    let few = read(&said_by_wifid(NEAR, Some(FEW), false));
    assert!(
        few.as_ref()
            .is_some_and(|m| m.p95.is_none() && m.late > 37.0 && m.held()),
        "a minute too few to time wasn't held by its late answers, its p95 set aside: {few:?}"
    );
    let none = read(&said_by_wifid(NEAR, Some(NONE), false));
    assert!(
        none.as_ref().is_some_and(|m| m.p95.is_none() && !m.held()),
        "a gateway dropping every marked echo read as a hold: {none:?}"
    );
    for (message, why) in [
        (
            said_by_wifid(NEAR, None, false),
            "a minute from a wifid that left out its video clause read as one",
        ),
        (
            said_by_wifid(NEAR, Some("unmeasured"), false),
            "a minute whose video went unmeasured read as one",
        ),
        (
            said_by_wifid(NEAR, Some(LOST), false),
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
        on(NEAR, 0, &clause("2400.0", "37.5"), false),
        on(NEAR, 1, &clause("12.0", "0.0"), false),
        // Held by its delay alone.
        on(NEAR, 2, &clause("150.0", "0.5"), false),
        // Held by its late answers alone, while a call was on.
        on(FAR, 3, &clause("20.0", "2.5"), true),
        on(FAR, 4, &clause("10.0", "0.0"), false),
        // Held by its late answers, though too few were answered to time.
        on(FAR, 5, FEW, false),
        // Set aside, neither held nor clean, as no echo answered late.
        on(FAR, 6, NONE, false),
        entry(AT + 370, "wlp0s20f3 is authenticating"),
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
         this hour, 6 of 7 Wi-Fi minutes judged (1 set aside, their video unanswered)"
    );
    assert!(
        (m.now - 4.0).abs() < f64::EPSILON && m.target == Some(TARGET) && m.past().is_some(),
        "four held minutes in the hour didn't read as past its target: {m:?}"
    );
    assert_eq!(
        m.behind,
        [
            format!(
                "on {NEAR}, 2 of 3 minutes held, the worst 2400.0 ms at p95 and 37.5% past 1 s \
                 at 11:25, best effort then 15.0 ms"
            ),
            format!(
                "on {FAR}, 2 of 3 minutes held, the worst 37.5% past 1 s at 11:30, best effort \
                 then 15.0 ms"
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
    let only = |message: &str| journal(&[entry(AT, message)]);
    let unmeasured = only(&said_by_wifid(NEAR, Some("unmeasured"), false));
    let older = only(&said_by_wifid(NEAR, None, false));
    let first = only(&said_by_wifid(NEAR, Some(LOST), false));
    let other = only("wlp0s20f3 is done looking");
    for (text, why) in [
        (None, ": wifid's journal unread"),
        (
            Some(""),
            ": no wifid line, as when wired or asleep, or the journal unread here",
        ),
        (Some(other.as_str()), ": no Wi-Fi minute"),
        (
            Some(unmeasured.as_str()),
            ": 0 of 1 Wi-Fi minutes judged (1 with no video echo sent)",
        ),
        (
            Some(older.as_str()),
            ": 0 of 1 Wi-Fi minutes judged (1 from a wifid writing no video figure)",
        ),
        (
            Some(first.as_str()),
            ": 0 of 1 Wi-Fi minutes judged (1 from a wifid writing no video figure)",
        ),
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
        args.windows(2).any(|pair| pair == ["-u", "wifid.service"])
            && args.windows(2).any(|pair| pair == ["-o", "json"])
            && args.contains(&format!("--since=@{}", AT - 3600))
            && args.contains(&format!("--until=@{AT}"))
            && !args.iter().any(|a| a.starts_with("--grep")),
        "journalctl isn't asked for wifid's whole hour: {args:?}"
    );
}
