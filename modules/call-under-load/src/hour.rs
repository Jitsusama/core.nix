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

/// What an app's time blocked stands for, or why it's unread. Each count
/// is said against the windows the app was open in, so an hour with it
/// closed never reads as the kernel keeping no time, and an hour kept in
/// only part shows how much of it the figure stands for.
fn kept_blocked(app: &str, waits: &Waits) -> Vec<String> {
    if waits.windows == 0 {
        return vec![format!(
            "Joel's {app}: unread, since it was not open in the hour"
        )];
    }
    if waits.blocked_windows == 0 {
        return vec![format!(
            "Joel's {app}: unread, kept in none of its {} windows, since the kernel keeps no \
             time blocked without kernel.sched_schedstats",
            waits.windows
        )];
    }
    vec![format!(
        "Joel's {app}: each 10 s window's most any drawing thread was blocked in the kernel, \
         on a lock, a page read back or reclaim, from lag-watch, kept in {} of its {} windows",
        waits.blocked_windows, waits.windows
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
#[path = "hour_tests.rs"]
mod tests;
