//! The hour's figures in cadence's shape, `apps-under-load/hour.json`
//! in the state home, and a line for each run in `history.jsonl` beside
//! it. Cadence reads the first (modules/cadence/src/hour/measures/apps.rs);
//! the second is kept for a person looking back.

use crate::figures::Call;
use crate::history::Waits;
use serde::Serialize;
use serde_json::{Value, json};
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::Path;

/// Who answers for these figures, as cadence names owners.
const OWNER: &str = "5193FV";
/// Who answers for the launcher's.
const LAUNCHER: &str = "XY9H5C";
/// The one sound figure cadence always reads, higher being better.
const HEARD: &str = "call sound heard and counted";
/// Each app's wait figure: the p99 across the hour's 10 s windows of each
/// window's worst drawing-thread wait, which the name says, so no one
/// reads it as the p99 of every wakeup.
const CHROME_WAIT: &str = "Chrome worst wait per 10 s p99 ms";
const SLACK_WAIT: &str = "Slack worst wait per 10 s p99 ms";
/// Each app's figure for the time a drawing thread was blocked in the
/// kernel: the p99 across the hour's 10 s windows of the most any one was.
const CHROME_BLOCKED: &str = "Chrome most blocked per 10 s p99 ms";
const SLACK_BLOCKED: &str = "Slack most blocked per 10 s p99 ms";
/// Joel's own calls' figure, in ms of waiting per second.
const REAL: &str = "real call sending wait p99 ms per s";
/// How many of them the watch couldn't read.
const REAL_BLIND: &str = "real calls with no sending thread found";
/// How long since the watch last could tell whether a call was on.
const REAL_WATCH: &str = "real call watch minutes since it could tell";
/// Past this many minutes, the watch has stopped or can't tell.
const WATCH_LATE: f64 = 10.0;
/// The most the watch's minutes read: a day, so one that never could tell
/// reads as that rather than minutes since 1970.
const DAY_MINUTES: u64 = 24 * 60;
/// How far, in tenths, a figure may sit past a whole tenth and still be
/// read as on it: a ten-thousandth, which is 10 ns of a time. The page's
/// times carry error near 1e-8 ms, far inside it, and the page's clock
/// steps by at least 5 microseconds. A wait from lag-watch can sit 10 ns
/// past a tenth and read as on it, which no reader could tell apart.
const NOISE: f64 = 1e-4;

/// What stands in for Joel's voice, said beside the sound's figures. The
/// far end plays on this machine too, under the same load, so what it
/// made up counts its own playing as well as the near end's sending.
const SOUND: &str = "Chromium's fake microphone, a beep each second, sent as Opus; \
                     the far end plays it on this machine, into a fake output";

/// One figure, as cadence reads it.
#[derive(Debug, PartialEq, Serialize)]
#[expect(
    clippy::struct_field_names,
    reason = "cadence reads each figure's name as its measure key"
)]
pub struct Measure {
    pub measure: &'static str,
    pub now: Option<f64>,
    pub target: f64,
    pub lower_is_better: bool,
    pub owner: &'static str,
    pub behind: Vec<String>,
}

fn measure(
    measure: &'static str,
    now: Option<f64>,
    target: f64,
    lower_is_better: bool,
    behind: Vec<String>,
) -> Measure {
    // Rounded to the tenth cadence shows, toward the worse side, so a
    // figure just past its target never shows as on it. The nudge keeps a
    // whole tenth's float error from moving it a tenth.
    let tenth = |x: f64| {
        let tenths = x * 10.0;
        let whole = if lower_is_better {
            (tenths - NOISE).ceil()
        } else {
            (tenths + NOISE).floor()
        };
        (whole + 0.0) / 10.0
    };
    Measure {
        measure,
        // A figure that isn't a number reads as unread, never on target.
        now: now.filter(|x| x.is_finite()).map(tenth),
        target,
        lower_is_better,
        owner: OWNER,
        behind,
    }
}

/// What a wait's figure leaves out, said beside it.
fn left_out(app: &str, waits: &Waits) -> Vec<String> {
    let mut said = vec![format!(
        "Joel's {app}: each 10 s window's worst drawing-thread wait, from lag-watch, {} windows",
        waits.windows
    )];
    if waits.lowered > 0 {
        said.push(format!("hidden tabs left out in {} windows", waits.lowered));
    }
    if waits.unwatched > 0 {
        said.push(format!(
            "some of it unwatched in {} windows",
            waits.unwatched
        ));
    }
    said
}

/// What an app's time blocked stands for, or why it's unread.
fn kept_blocked(app: &str, waits: &Waits) -> Vec<String> {
    if waits.blocked_windows == 0 {
        return vec![format!(
            "Joel's {app}: unread, since the kernel keeps no time blocked without \
             kernel.sched_schedstats"
        )];
    }
    vec![format!(
        "Joel's {app}: each 10 s window's most any drawing thread was blocked in the kernel, \
         on a lock, a page read back or reclaim, from lag-watch, {} windows",
        waits.blocked_windows
    )]
}

/// The call's sound as four figures, each saying what stood in for Joel's
/// voice and, when sound went unheard or a figure is missing, why.
fn sound(call: &Call) -> Vec<Measure> {
    let call_sound = |first: &str| {
        let mut said = vec![first.to_string(), SOUND.to_string()];
        let heard = if call.heard { "heard, but" } else { "unheard" };
        said.extend(call.sound_why.iter().map(|why| format!("{heard}: {why}")));
        said
    };
    vec![
        // A figure cadence reads whatever the sound did, since it drops a
        // figure with no reading: a call that heard none, or heard some
        // but couldn't count it all, files a fault carrying why, rather
        // than its other sound figures dropping off unread.
        measure(
            HEARD,
            Some(if call.heard && call.sound_why.is_none() {
                1.0
            } else {
                0.0
            }),
            1.0,
            false,
            call_sound(
                "1 when the far end played the near end's sound and every figure of it was \
                 counted, 0 when not",
            ),
        ),
        // Past 1% made up, a voice starts to sound clipped; past 30 ms of
        // jitter, a call's buffer has to grow or drop sound to keep up.
        measure(
            "call sound concealed %",
            call.concealed,
            1.0,
            true,
            call_sound("the share the far end made up for late or lost sound"),
        ),
        measure(
            "call sound jitter p99 ms",
            call.jitter_p99,
            30.0,
            true,
            call_sound("how unevenly the sound's packets reached the far end"),
        ),
        measure(
            "call sound packets lost",
            call.lost,
            0.0,
            true,
            call_sound("a count, lost between two programs on one machine, so only under load"),
        ),
    ]
}

/// The hour's figures: the call's, the apps' waits and the launcher's,
/// whose note says when the shell's journal wasn't read whole, and why.
pub fn measures(
    call: &Call,
    stand_in: &str,
    chrome: &Waits,
    slack: &Waits,
    opens: &[f64],
    unread: Option<&str>,
) -> Vec<Measure> {
    let call_too = |first: &str| vec![first.to_string(), stand_in.to_string()];
    let mut behind = vec![format!(
        "{} openings in the last hour, chord or ask to on screen",
        opens.len()
    )];
    if let Some(why) = unread {
        behind.push(format!("the shell's journal wasn't read whole: {why}"));
    }
    let mut launcher = measure(
        "launcher open ms p99",
        crate::figures::percentile(opens, 99),
        50.0,
        true,
        behind,
    );
    launcher.owner = LAUNCHER;
    let mut all = vec![
        measure(
            "call camera fps p10",
            call.camera,
            30.0,
            false,
            call_too("the camera's source"),
        ),
        measure(
            "call sent fps p10",
            call.sent,
            30.0,
            false,
            call_too("VP9 encoded in software"),
        ),
        measure(
            "call far end got fps p10",
            call.got,
            30.0,
            false,
            call_too("decoded"),
        ),
        measure(
            "call freezes per 10 min",
            call.freezes,
            0.0,
            true,
            call_too(&call.frozen_s.map_or_else(
                || "how long it froze is unread".to_string(),
                |s| format!("{s} s frozen in the run"),
            )),
        ),
    ];
    all.extend(sound(call));
    all.extend([
        measure(
            "call click to next frame p99 ms",
            call.clicks_p99,
            50.0,
            true,
            vec![
                "a click every 5 s on the test call's own Chromium tab, not Joel's Chrome"
                    .to_string(),
            ],
        ),
        measure(
            CHROME_WAIT,
            chrome.worst_p99,
            2.0,
            true,
            left_out("Chrome", chrome),
        ),
        measure(
            SLACK_WAIT,
            slack.worst_p99,
            2.0,
            true,
            left_out("Slack", slack),
        ),
        // Past 50 ms blocked in ten seconds, the click's own target, a
        // window could have missed several frames running.
        measure(
            CHROME_BLOCKED,
            chrome.blocked_p99,
            50.0,
            true,
            kept_blocked("Chrome", chrome),
        ),
        measure(
            SLACK_BLOCKED,
            slack.blocked_p99,
            50.0,
            true,
            kept_blocked("Slack", slack),
        ),
        launcher,
    ]);
    all
}

/// Writes the hour for cadence, whole or not at all, with the run beside
/// its figures, and adds the run's line to the history beside it. Cadence
/// reads only the figures; the run says what lies behind them, the
/// launcher's openings during the test and just before it among them.
pub fn write(folder: &Path, at: u64, measures: &[Measure], run: &Value) -> io::Result<()> {
    fs::create_dir_all(folder)?;
    let hour = json!({"at": at, "measures": measures, "run": run});
    // Named for this run, so a run by hand beside the hour's can't take it.
    let part = folder.join(format!("hour.json.{}.part", std::process::id()));
    fs::write(&part, serde_json::to_vec_pretty(&hour)?)?;
    fs::rename(&part, folder.join("hour.json"))?;
    let figures: serde_json::Map<String, Value> = measures
        .iter()
        .map(|m| (m.measure.to_string(), json!(m.now)))
        .collect();
    let line = json!({"at": at, "run": run, "measures": figures});
    append(&folder.join("history.jsonl"), &line)
}

/// Adds one line to the history, as a single write, so the hour's run
/// and the call watch can't interleave their lines.
pub fn append(history: &Path, line: &Value) -> io::Result<()> {
    if let Some(folder) = history.parent() {
        fs::create_dir_all(folder)?;
    }
    let mut file = OpenOptions::new().create(true).append(true).open(history)?;
    file.write_all(format!("{line}\n").as_bytes())
}

/// Joel's own calls' figures, as of `at`: the worst of their sending
/// threads' p99 wait for a CPU, from the calls `heard` names, with the
/// load and the thread behind it said, unread when no call was ever
/// watched; how many calls since `since`, the last hour written, the watch
/// couldn't read in a call app it found; and how long since the watch,
/// which said `clear`, last could tell whether a call was on.
pub fn real(calls: &[crate::watch::Real], since: u64, clear: Option<u64>, at: u64) -> Vec<Measure> {
    let p99 = |call: &&crate::watch::Real| call.sending_p99.unwrap_or(0.0);
    let worst = calls
        .iter()
        .filter(|call| call.sending_p99.is_some())
        .max_by(|a, b| p99(a).total_cmp(&p99(b)));
    let read = calls
        .iter()
        .filter(|call| call.sending_p99.is_some())
        .count();
    let heard = calls.iter().filter(|call| !appless(call)).count();
    let mut behind = vec![format!(
        "Joel's own calls, {read} of {heard} read: the p99 of each 5 s tick's slowest sending \
         thread's wait for a CPU per second, in his call apps, which in a call under 8 \
         minutes is its worst tick",
    )];
    if let Some(call) = worst {
        behind.push(format!(
            "the worst ended {} min ago, {} of its {} ticks read, {} of them past {} ms/s or \
             load {}, its 15-minute load up to {:.0}, its slowest thread {}",
            at.saturating_sub(call.ended) / 60,
            call.read,
            call.ticks,
            call.flagged,
            crate::watch::LATE_MS,
            crate::watch::HEAVY,
            call.load15_max,
            call.worst_who.as_deref().unwrap_or("unread"),
        ));
        if call.unsure {
            behind.push("it was closed when a-call-is-on couldn't tell for 3 min".to_string());
        }
        if call.cut {
            behind.push("it was cut short when the watch stopped".to_string());
        }
    }
    vec![
        measure(
            REAL,
            worst.and_then(|call| call.sending_p99),
            crate::watch::LATE_MS,
            true,
            behind,
        ),
        blind(calls, since, at),
        watched(clear, at),
    ]
}

/// Whether the watch found none of his call apps open during a call it
/// couldn't read: the microphone in use outside a call, or a call in a
/// browser his terminal started, which isn't watched.
fn appless(call: &crate::watch::Real) -> bool {
    call.apps.is_empty() && call.sending_p99.is_none()
}

/// How many calls that ended after `since` the watch couldn't read in a
/// call app it found, each said with when it ended and in which apps, so
/// one such call files a fault for its own hour and not for the day its
/// figure stands. One with no call app open is said, and not counted.
fn blind(calls: &[crate::watch::Real], since: u64, at: u64) -> Measure {
    let fresh: Vec<&crate::watch::Real> = calls
        .iter()
        .filter(|call| call.ended > since && call.sending_p99.is_none())
        .collect();
    let (none_open, unread): (Vec<_>, Vec<_>) = fresh.into_iter().partition(|c| appless(c));
    let mut behind = vec![
        "Joel's calls since the last hour with a call app open but no sending thread read, \
         so their waits are unread; a call in Tuple or Zoom, whose threads aren't named as \
         Chromium's, reads so, as does the microphone in use outside a call while one is open"
            .to_string(),
    ];
    behind.extend(unread.iter().map(|call| {
        format!(
            "one ended {} min ago, in {}",
            at.saturating_sub(call.ended) / 60,
            call.apps.join(" and ")
        )
    }));
    if !none_open.is_empty() {
        behind.push(format!(
            "{} with no call app of his open, left out: the microphone in use outside a call, \
             or a call in a browser his terminal started",
            none_open.len()
        ));
    }
    #[expect(
        clippy::cast_precision_loss,
        reason = "a count of calls is far below f64's exact range"
    )]
    let count = unread.len() as f64;
    measure(REAL_BLIND, Some(count), 0.0, true, behind)
}

/// How many minutes since the watch last could tell whether a call was on,
/// at most a day, so a watch that stopped, never started or hung files a
/// fault rather than reading as a day with no calls. One that never said
/// reads as a day. The hour itself runs only when a-call-is-on can tell,
/// so this shows a watch that can't only once the hour can.
fn watched(clear: Option<u64>, at: u64) -> Measure {
    let cap = DAY_MINUTES;
    let minutes = clear.map_or(cap, |clear| (at.saturating_sub(clear) / 60).min(cap));
    let behind = if clear.is_none() {
        "the watch has never said when it could tell: it hasn't run".to_string()
    } else {
        format!(
            "the watch asks every 30 s; past {WATCH_LATE:.0} min it has stopped, hung or can't \
             tell, and a day or more reads as {cap}"
        )
    };
    #[expect(
        clippy::cast_precision_loss,
        reason = "a day's minutes are far below f64's exact range"
    )]
    let minutes = minutes as f64;
    measure(REAL_WATCH, Some(minutes), WATCH_LATE, true, vec![behind])
}

#[cfg(test)]
mod tests {
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
        let now = |x: f64, lower_is_better: bool| {
            measure("x", Some(x), 30.0, lower_is_better, vec![]).now
        };
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
        let root =
            std::env::temp_dir().join(format!("call-under-load-hour-{}", std::process::id()));
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
        let hour: Value =
            serde_json::from_slice(&fs::read(root.join("hour.json")).unwrap()).unwrap();
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
}
