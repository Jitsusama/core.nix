//! The hour's own tests, kept beside it so `hour.rs` reads as the card's
//! figures alone.

use super::*;

#[test]
fn joel_s_calls_give_their_worst_sending_wait_and_say_whose() {
    let call = |ended, p99: Option<f64>, who: &str, apps: &[&str]| crate::watch::Real {
        began: ended - 600,
        ended,
        ticks: 120,
        read: 110,
        flagged: 3,
        sending_p99: p99,
        worst_ms: p99,
        worst_who: Some(who.to_string()),
        load15_max: 121.4,
        unsure: false,
        cut: ended == 2_000,
        apps: apps.iter().map(ToString::to_string).collect(),
    };
    let calls = [
        call(1_000, Some(40.0), "chrome:renderer:Compositor", &["chrome"]),
        call(
            2_000,
            Some(412.5),
            "chrome:renderer:CodecWorker",
            &["chrome"],
        ),
        call(2_500, None, "none", &["chrome", "slack"]),
        call(2_550, None, "none", &[]),
    ];
    let figures = real(&calls, 1_500, Some(2_300), 2_600);
    let figure = &figures[0];
    assert_eq!(figure.now, Some(412.5));
    assert!(
        figure.lower_is_better && (figure.target - 300.0).abs() < 1e-9,
        "a wait is lower-better against the 300 ms/s bar"
    );
    assert_eq!(figure.owner, OWNER);
    assert!(
        figure.behind[0].starts_with("Joel's own calls, 2 of 3 read:"),
        "unread calls were counted as read: {}",
        figure.behind[0]
    );
    assert_eq!(
        figure.behind[1..],
        [
            "the worst ended 10 min ago, 110 of its 120 ticks read, 3 of them past 300 ms/s \
             or load 80, its 15-minute load up to 121, its slowest thread \
             chrome:renderer:CodecWorker",
            "it was cut short when the watch stopped",
        ],
        "the figure doesn't say which call and thread it is, or that it was cut"
    );
    let blind = &figures[1];
    assert_eq!(
        (blind.now, blind.target, blind.lower_is_better),
        (Some(1.0), 0.0, true),
        "a call the watch couldn't read in a call app filed no fault, or a microphone did"
    );
    assert_eq!(
        blind.behind[1..],
        [
            "one ended 1 min ago, in chrome and slack",
            "1 with no call app of his open, left out: the microphone in use outside a \
             call, or a call in a browser his terminal started",
        ],
        "an unread call wasn't said with when and where"
    );
    assert_eq!(
        real(&calls, 2_550, Some(2_300), 2_600)[1].now,
        Some(0.0),
        "an unread call from before the last hour filed its fault again"
    );
    let watch = &figures[2];
    assert_eq!(
        (watch.now, watch.target, watch.lower_is_better),
        (Some(5.0), 10.0, true),
        "the watch's last clear answer wasn't said in minutes against 10"
    );
    let none = real(&[], 0, None, 0);
    assert_eq!(
        (none[0].now, none[1].now),
        (None, Some(0.0)),
        "no call watched read as a wait, or as one unread"
    );
    assert_eq!(
        none[2].now,
        Some(1440.0),
        "a watch that never said read as on time, or dropped off the card"
    );
    assert_eq!(
        real(&[], 0, Some(0), 1_800_000_000)[2].now,
        Some(1440.0),
        "a watch that never could tell read as minutes since 1970"
    );
}

#[test]
fn a_figure_is_rounded_toward_its_worse_side() {
    let now =
        |x: f64, lower_is_better: bool| measure("x", Some(x), 30.0, lower_is_better, vec![]).now;
    assert_eq!(
        now(30.04, true),
        Some(30.1),
        "30.04 ms showed on a 30 ms target"
    );
    assert_eq!(
        now(29.96, false),
        Some(29.9),
        "29.96 fps showed on a 30 fps target"
    );
    assert_eq!(now(1.1, true), Some(1.1), "a whole tenth moved up");
    assert_eq!(now(28.8, false), Some(28.8), "a whole tenth moved down");
    // Float error leaves 0.1 * 3 just over 0.3 and 1 - 0.9 just under
    // 0.1; the nudge keeps each on its tenth.
    assert_eq!(now(0.1 * 3.0, true), Some(0.3), "float error moved up");
    assert_eq!(now(1.0 - 0.9, false), Some(0.1), "float error moved down");
    // The page's click times come with error near 1e-8 ms: 50.3 ms
    // arrived as this at 05:37 on 23 November and showed as 50.4.
    assert_eq!(
        now(50.300_000_011_920_93, true),
        Some(50.3),
        "a click time's error moved it up"
    );
    assert_eq!(now(50.301, true), Some(50.4), "a real thousandth was lost");
    // The same error under a figure where higher is better.
    assert_eq!(
        now(29.999_999_988_079_07, false),
        Some(30.0),
        "a rate's error moved it down"
    );
    assert_eq!(now(29.999, false), Some(29.9), "a real thousandth was lost");
    assert_eq!(now(f64::NAN, true), None, "NaN read as a figure");
    let none = now(0.0, true).unwrap_or(f64::NAN);
    assert!(
        none == 0.0 && none.is_sign_positive(),
        "no wait is 0, not -0: {none}"
    );
}

fn call() -> Call {
    Call {
        camera: Some(30.0),
        sent: Some(29.96),
        got: Some(30.0),
        freezes: Some(0.0),
        frozen_s: Some(0.0),
        clicks_p50: Some(3.0),
        clicks_p99: Some(9.84),
        heard: true,
        concealed: Some(0.42),
        jitter_p99: Some(24.0),
        lost: Some(3.0),
        sound_why: None,
        samples: 140,
    }
}

#[test]
fn every_figure_says_which_way_is_better_and_who_owns_it() {
    let chrome = Waits {
        worst_p99: Some(19.66),
        windows: 360,
        lowered: 12,
        unwatched: 0,
        blocked_p99: Some(120.04),
        blocked_windows: 300,
    };
    let all = measures(
        &call(),
        "a fake camera",
        &chrome,
        &Waits::default(),
        &[20.0, 31.0],
        Some("journalctl ended with code 1: no"),
    );
    assert_eq!(all.len(), 14);
    let named = |name: &str| all.iter().find(|m| m.measure == name).unwrap();
    for (name, now, target) in [
        (HEARD, 1.0, 1.0),
        // 0.42% made up shows as 0.5, rounded toward the worse side.
        ("call sound concealed %", 0.5, 1.0),
        ("call sound jitter p99 ms", 24.0, 30.0),
        ("call sound packets lost", 3.0, 0.0),
    ] {
        assert_eq!(
            (named(name).now, named(name).target),
            (Some(now), target),
            "{name} reads the wrong figure or target"
        );
        assert_eq!(
            named(name).behind[1],
            SOUND,
            "{name} doesn't say what it heard"
        );
    }
    // 29.96 fps sent is short of 30, so it shows as 29.9, never on target.
    assert_eq!(named("call sent fps p10").now, Some(29.9));
    // A figure read the wrong way passes cadence at any value.
    for m in &all {
        let higher = m.measure.contains(" fps ") || m.measure == HEARD;
        assert_eq!(
            m.lower_is_better,
            !higher,
            "{} reads as {} is better",
            m.measure,
            if m.lower_is_better { "lower" } else { "higher" }
        );
    }
    assert_eq!(named(CHROME_WAIT).now, Some(19.7));
    let behind = &named(CHROME_WAIT).behind;
    assert!(
        behind[1].contains("12 windows"),
        "Chrome's wait doesn't say how many windows it read: {behind:?}"
    );
    assert_eq!(named(SLACK_WAIT).now, None);
    let blocked = named(CHROME_BLOCKED);
    assert_eq!(
        (blocked.now, blocked.target, blocked.lower_is_better),
        (Some(120.1), 50.0, true),
        "Chrome's time blocked reads the wrong figure or target"
    );
    assert!(
        blocked.behind[0].contains("300 windows"),
        "Chrome's time blocked doesn't say how many windows kept it: {:?}",
        blocked.behind
    );
    let unkept = named(SLACK_BLOCKED);
    assert_eq!(
        unkept.now, None,
        "Slack's time blocked, never kept, was read"
    );
    assert!(
        unkept.behind[0].contains("kernel.sched_schedstats"),
        "a time blocked never kept doesn't say why: {:?}",
        unkept.behind
    );
    assert_eq!(named("launcher open ms p99").owner, "XY9H5C");
    assert_eq!(named("launcher open ms p99").now, Some(31.0));
    assert_eq!(
        named("launcher open ms p99").behind[1],
        "the shell's journal wasn't read whole: journalctl ended with code 1: no",
        "an unread journal is said beside the figure cadence reads"
    );
    assert_eq!(named("call freezes per 10 min").behind[1], "a fake camera");
}

#[test]
fn a_journal_read_whole_adds_nothing_to_the_launcher_s_note() {
    let all = measures(
        &call(),
        "a fake camera",
        &Waits::default(),
        &Waits::default(),
        &[20.0, 31.0],
        None,
    );
    let launcher = all
        .iter()
        .find(|m| m.measure == "launcher open ms p99")
        .unwrap();
    assert_eq!(launcher.now, Some(31.0));
    assert_eq!(
        launcher.behind,
        ["2 openings in the last hour, chord or ask to on screen"]
    );
}

#[test]
fn sound_that_went_unheard_says_why_beside_each_of_its_figures() {
    let unheard = Call {
        heard: false,
        concealed: None,
        jitter_p99: None,
        lost: None,
        sound_why: Some("the far end's playing was refused: NotAllowedError".to_string()),
        ..call()
    };
    let all = measures(
        &unheard,
        "a fake camera",
        &Waits::default(),
        &Waits::default(),
        &[],
        None,
    );
    let sound: Vec<&Measure> = all
        .iter()
        .filter(|m| m.measure.starts_with("call sound "))
        .collect();
    assert_eq!(sound.len(), 4, "the sound's figures aren't all named");
    for m in sound {
        assert_eq!(
            m.behind.last().map(String::as_str),
            Some("unheard: the far end's playing was refused: NotAllowedError"),
            "{} doesn't say why it's missing",
            m.measure
        );
    }
    let heard = all.iter().find(|m| m.measure == HEARD);
    assert_eq!(
        heard.map(|m| m.now),
        Some(Some(0.0)),
        "a call that heard nothing reads 0, which cadence files, not a null it drops"
    );
}

#[test]
fn sound_heard_but_not_all_counted_files_why_on_a_figure_cadence_reads() {
    let why = "the far end's counts started again partway, so what it made up and lost \
               can't be told";
    let restarted = Call {
        heard: true,
        concealed: None,
        lost: None,
        sound_why: Some(why.to_string()),
        ..call()
    };
    let all = measures(
        &restarted,
        "a fake camera",
        &Waits::default(),
        &Waits::default(),
        &[],
        None,
    );
    let heard = all.iter().find(|m| m.measure == HEARD);
    assert_eq!(
        heard.map(|m| m.now),
        Some(Some(0.0)),
        "sound partly counted reads 0, past its target, so cadence files it"
    );
    assert_eq!(
        heard.and_then(|m| m.behind.last()).map(String::as_str),
        Some(format!("heard, but: {why}").as_str()),
        "the filed figure carries why"
    );
}

#[test]
fn the_hour_is_written_in_cadence_s_shape_and_the_run_kept_beside_it() {
    let root = std::env::temp_dir().join(format!("call-under-load-hour-{}", std::process::id()));
    let all = measures(
        &call(),
        "a fake camera",
        &Waits::default(),
        &Waits::default(),
        &[],
        None,
    );
    write(&root, 1_795_200_000, &all, &json!({"took_s": 190})).unwrap();
    write(&root, 1_795_203_600, &all, &json!({"took_s": 191})).unwrap();
    let hour: Value = serde_json::from_slice(&fs::read(root.join("hour.json")).unwrap()).unwrap();
    let history = fs::read_to_string(root.join("history.jsonl")).unwrap();
    let parts = fs::read_dir(&root).unwrap().count();
    fs::remove_dir_all(&root).unwrap();
    assert_eq!(hour["at"], 1_795_203_600);
    assert_eq!(
        hour["run"],
        json!({"took_s": 191}),
        "the newest run beside the hour"
    );
    let first = &hour["measures"][0];
    for key in [
        "measure",
        "now",
        "target",
        "lower_is_better",
        "owner",
        "behind",
    ] {
        assert!(first.get(key).is_some(), "{key} missing from {first}");
    }
    assert_eq!(history.lines().count(), 2);
    assert_eq!(parts, 2, "only hour.json and history.jsonl are left");
}
