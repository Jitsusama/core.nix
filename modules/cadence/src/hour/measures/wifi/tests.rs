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
        read(&said_by_wifid(
            NEAR,
            Some("881.0 ms at p95, 38.3% of 120 echoes lost"),
            true
        )),
        Some(Minute {
            at: AT,
            bssid: NEAR.to_owned(),
            p95: 881.0,
            lost: 38.3,
            call: true,
        }),
        "a minute's video clause wasn't read as wifid writes it"
    );
    for (message, why) in [
        (
            said_by_wifid(NEAR, Some("none at p95, 100.0% of 120 echoes lost"), false),
            "a gateway dropping every marked echo read as a hold",
        ),
        (
            said_by_wifid(NEAR, None, false),
            "a minute with no video echo sent read as one",
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
    let near = |n: i64, video: &str| entry(AT + 60 * n, &said_by_wifid(NEAR, Some(video), false));
    let text = journal(&[
        near(0, "881.0 ms at p95, 38.3% of 120 echoes lost"),
        near(1, "12.0 ms at p95, 0.0% of 120 echoes lost"),
        // Held by its delay alone.
        near(2, "150.0 ms at p95, 0.5% of 120 echoes lost"),
        // Held by its loss alone, while a call was on.
        entry(
            AT + 180,
            &said_by_wifid(FAR, Some("20.0 ms at p95, 2.5% of 120 echoes lost"), true),
        ),
        entry(
            AT + 240,
            &said_by_wifid(FAR, Some("10.0 ms at p95, 0.0% of 120 echoes lost"), false),
        ),
        // Neither read nor held, as wifid doesn't weigh it either.
        entry(
            AT + 300,
            &said_by_wifid(FAR, Some("none at p95, 100.0% of 120 echoes lost"), false),
        ),
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
        "minutes the Wi-Fi held a call's video past 100 ms at p95 or lost over 1% of it, this \
         hour, of 5 read"
    );
    assert!(
        (m.now - 3.0).abs() < f64::EPSILON && m.target == Some(TARGET) && m.past().is_some(),
        "three held minutes in the hour didn't read as past its target: {m:?}"
    );
    assert_eq!(
        m.behind,
        [
            format!(
                "on {NEAR}, 2 of 3 minutes held, the worst 881.0 ms at p95 and 38.3% lost at \
                 11:25"
            ),
            format!(
                "on {FAR}, 1 of 2 minutes held, the worst 20.0 ms at p95 and 2.5% lost at 11:28, \
                 during a call"
            ),
        ]
    );
}

#[test]
fn one_held_minute_is_within_and_none_read_is_unjudged_saying_why() {
    let one = journal(&[
        entry(
            AT,
            &said_by_wifid(NEAR, Some("300.0 ms at p95, 0.0% of 120 echoes lost"), false),
        ),
        entry(
            AT + 60,
            &said_by_wifid(NEAR, Some("9.0 ms at p95, 0.0% of 120 echoes lost"), false),
        ),
    ]);
    let m = video(Some(&one), &TimeZone::UTC);
    assert!(
        (m.now - 1.0).abs() < f64::EPSILON && m.past().is_none(),
        "a lone held minute was filed with wifid's owner: {m:?}"
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
