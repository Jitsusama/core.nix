//! How long a program takes to show its first window, and to end once
//! its last window closes, said to the journal for health to read
//! (modules/health). niri's event stream says when each window opens and
//! closes and which process it is; the process's start is read from
//! /proc, and its end is waited for on a thread of its own, only while a
//! process outlives its last window, so nothing runs while the desktop
//! rests.
//!
//! A window of a process that started long before it, Chrome's second
//! window or a session in kitty's one instance, says nothing of how long
//! a program takes to start, so only a process's first window within
//! [`OPENING_AT_MOST`] of its start is timed.

use std::collections::BTreeMap;
use std::fs;
use std::io::Write as _;
use std::time::{Duration, Instant};

use serde_json::Value;

use crate::desktop::app_of;

/// How long after its process started a window still counts as that
/// program's opening.
pub const OPENING_AT_MOST: u64 = 60_000;

/// How long a process that outlived its last window is waited for.
pub const ENDING_AT_MOST: Duration = Duration::from_secs(30);

/// How often a process waited for is looked at at first: a finer step
/// than the 200 ms docs/design.md gives a quit.
pub const STEP: Duration = Duration::from_millis(20);

/// How long a wait looks at that step, twice a quit's budget, before each
/// look waits a tenth of the time waited so far. A program kept in the
/// background, as Chrome, Slack and Zoom are, outlives every wait, and at
/// 20 ms throughout it cost 1,500 wakes over the 30 s, where the desktop
/// at rest keeps under 5 a second; growing, it costs about 65, and an end
/// past the budget is still said to within a tenth.
pub const FINE: Duration = Duration::from_millis(400);

/// /proc's clock ticks a second, which Linux fixes at 100 for every
/// program whatever the kernel's own rate.
const TICKS: u64 = 100;

/// What a window opening or closing says.
#[derive(Debug, PartialEq, Eq)]
pub enum Timing {
    /// `app`'s first window, `ms` after its process started.
    Opened { app: String, ms: u64 },
    /// `app`'s last window closed, with its process `pid` to wait for.
    Closed { app: String, pid: u32 },
}

/// The windows niri has said, by id, with their program and process.
#[derive(Default)]
pub struct Timed {
    windows: BTreeMap<u64, (String, u32)>,
}

impl Timed {
    /// Hears one event of niri's stream. `now` is the time since boot and
    /// `started` a process's start since boot, both in milliseconds.
    pub fn heard(
        &mut self,
        event: &Value,
        now: impl Fn() -> Option<u64>,
        started: impl Fn(u32) -> Option<u64>,
    ) -> Option<Timing> {
        let (name, said) = event.as_object()?.iter().next()?;
        match name.as_str() {
            // The whole desktop as a stream begins: what's there opened
            // before it, so none of it is an opening.
            "WindowsChanged" => {
                self.windows = said["windows"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|window| Some((window["id"].as_u64()?, held(window)?)))
                    .collect();
                None
            }
            "WindowOpenedOrChanged" => {
                let window = &said["window"];
                let id = window["id"].as_u64()?;
                let (app, pid) = held(window)?;
                let first = !self.windows.contains_key(&id) && !self.holds(pid);
                self.windows.insert(id, (app.clone(), pid));
                if !first {
                    return None;
                }
                let ms = now()?.checked_sub(started(pid)?)?;
                (ms <= OPENING_AT_MOST).then_some(Timing::Opened { app, ms })
            }
            "WindowClosed" => {
                let (app, pid) = self.windows.remove(&said["id"].as_u64()?)?;
                (!self.holds(pid)).then_some(Timing::Closed { app, pid })
            }
            _ => None,
        }
    }

    /// Whether a window held is of process `pid`.
    fn holds(&self, pid: u32) -> bool {
        self.windows.values().any(|(_, held)| *held == pid)
    }
}

/// A window's program, as every list names it, and its process.
fn held(window: &Value) -> Option<(String, u32)> {
    let app = app_of(window["app_id"].as_str()?).0.to_owned();
    let pid = u32::try_from(window["pid"].as_u64()?).ok()?;
    Some((app, pid))
}

/// Says what a window opening or closing timed to the journal, each line
/// in one write so none is torn by another's: an opening at once, and a
/// closing once its process ends, waited for on a thread of its own that
/// ends with the wait.
pub fn say(timing: Timing) {
    match timing {
        Timing::Opened { app, ms } => written(&opened_line(&app, ms)),
        Timing::Closed { app, pid } => {
            std::thread::spawn(move || {
                let ended = waited(pid, gone, pause, ENDING_AT_MOST);
                written(&closed_line(&app, ended));
            });
        }
    }
}

/// Writes one line to the journal in a single write.
fn written(line: &str) {
    let _ = std::io::stderr().write_all(format!("{line}\n").as_bytes());
}

/// The journal's line for an opening.
pub fn opened_line(app: &str, ms: u64) -> String {
    format!("opened {app} {ms} ms after its process started")
}

/// The journal's line for a last window closed: how long its process
/// went on, or that it was still running when the wait ended.
pub fn closed_line(app: &str, ended: Option<Duration>) -> String {
    match ended {
        Some(after) => format!(
            "closed {app}; its process ended {} ms later",
            after.as_millis()
        ),
        None => format!(
            "closed {app}; its process was still running {} s later",
            ENDING_AT_MOST.as_secs()
        ),
    }
}

/// How long a wait pauses before its next look, once it has waited
/// `waited`: [`STEP`] until [`FINE`], then a tenth of the time so far.
#[must_use]
pub fn pause(waited: Duration) -> Duration {
    if waited < FINE { STEP } else { waited / 10 }
}

/// How long `pid` took to end, looked at after each `pause` of the time
/// waited so far, up to `most`, or none when it hadn't.
pub fn waited(
    pid: u32,
    gone: impl Fn(u32) -> bool,
    pause: impl Fn(Duration) -> Duration,
    most: Duration,
) -> Option<Duration> {
    let began = Instant::now();
    loop {
        if gone(pid) {
            return Some(began.elapsed());
        }
        if began.elapsed() >= most {
            return None;
        }
        std::thread::sleep(pause(began.elapsed()));
    }
}

/// Whether `pid` has ended: gone from /proc, or a zombie its parent
/// hasn't reaped.
pub fn gone(pid: u32) -> bool {
    fs::read_to_string(format!("/proc/{pid}/stat"))
        .ok()
        .and_then(|stat| fields(&stat).next().map(|state| matches!(state, "Z" | "X")))
        .unwrap_or(true)
}

/// When `pid` started, in milliseconds since boot.
pub fn started(pid: u32) -> Option<u64> {
    started_of(&fs::read_to_string(format!("/proc/{pid}/stat")).ok()?)
}

/// The time since boot, in milliseconds, as /proc/uptime says it.
pub fn now() -> Option<u64> {
    uptime_of(&fs::read_to_string("/proc/uptime").ok()?)
}

/// A process's start in milliseconds since boot, from its stat line:
/// its 22nd field, in ticks.
fn started_of(stat: &str) -> Option<u64> {
    // The state is the third field, so the start is the 20th after it.
    let ticks: u64 = fields(stat).nth(19)?.parse().ok()?;
    ticks.checked_mul(1000 / TICKS)
}

/// The fields of a stat line after its command's name, which can hold
/// spaces and parentheses of its own.
fn fields(stat: &str) -> impl Iterator<Item = &str> {
    stat.rsplit_once(") ")
        .map_or("", |(_, rest)| rest)
        .split_whitespace()
}

/// The time since boot in milliseconds, from /proc/uptime's first field,
/// seconds to two places.
fn uptime_of(uptime: &str) -> Option<u64> {
    let (secs, hundredths) = uptime.split_whitespace().next()?.split_once('.')?;
    let secs: u64 = secs.parse().ok()?;
    let hundredths: u64 = hundredths.parse().ok()?;
    secs.checked_mul(1000)?.checked_add(hundredths * 10)
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::*;

    fn event(text: &str) -> Value {
        serde_json::from_str(text).unwrap()
    }

    fn opened(id: u64, app: &str, pid: u32) -> Value {
        event(&format!(
            r#"{{"WindowOpenedOrChanged":{{"window":{{"id":{id},"app_id":"{app}","pid":{pid}}}}}}}"#
        ))
    }

    fn closed(id: u64) -> Value {
        event(&format!(r#"{{"WindowClosed":{{"id":{id}}}}}"#))
    }

    /// Hears `event` at 100 s since boot, every process started at
    /// 99.75 s but process 7, started at 2 s.
    fn hear(timed: &mut Timed, event: &Value) -> Option<Timing> {
        timed.heard(
            event,
            || Some(100_000),
            |pid| Some(if pid == 7 { 2_000 } else { 99_750 }),
        )
    }

    #[test]
    fn a_programs_first_window_is_timed_from_its_process_start_and_its_next_says_nothing() {
        let mut timed = Timed::default();
        let first = hear(&mut timed, &opened(1, "foot", 40));
        assert_eq!(
            first,
            Some(Timing::Opened {
                app: "foot".to_owned(),
                ms: 250
            }),
            "a program's first window wasn't timed from its process's start"
        );
        assert_eq!(
            hear(&mut timed, &opened(1, "foot", 40)),
            None,
            "a window changing was taken for another opening"
        );
        assert_eq!(
            hear(&mut timed, &opened(2, "foot", 40)),
            None,
            "a second window of a running process was taken for its program starting"
        );
    }

    #[test]
    fn windows_there_as_the_stream_begins_and_old_processes_say_nothing() {
        let mut timed = Timed::default();
        let listed =
            event(r#"{"WindowsChanged":{"windows":[{"id":3,"app_id":"pi-session.2","pid":50}]}}"#);
        assert_eq!(
            hear(&mut timed, &listed),
            None,
            "the listed desktop was said"
        );
        assert_eq!(
            hear(&mut timed, &opened(3, "pi-session.2", 50)),
            None,
            "a window there as the stream began was taken for an opening"
        );
        assert_eq!(
            hear(&mut timed, &opened(4, "google-chrome", 7)),
            None,
            "a window of a process started 98 s before it was taken for its opening"
        );
    }

    #[test]
    fn a_process_is_waited_for_only_once_its_last_window_closes() {
        let mut timed = Timed::default();
        hear(&mut timed, &opened(1, "pi-session.4", 60));
        hear(&mut timed, &opened(2, "pi-session.5", 60));
        assert_eq!(
            hear(&mut timed, &closed(1)),
            None,
            "a process was waited for while it still had a window"
        );
        assert_eq!(
            hear(&mut timed, &closed(2)),
            Some(Timing::Closed {
                app: "kitty".to_owned(),
                pid: 60
            }),
            "a process's last window closing didn't wait for it, named as every list names it"
        );
        assert_eq!(
            hear(&mut timed, &closed(2)),
            None,
            "a window closed twice was said twice"
        );
    }

    #[test]
    fn a_process_outliving_the_wait_is_looked_at_seldom_once_past_the_budget() {
        let mut waited = Duration::ZERO;
        let mut looks = 0;
        while waited < ENDING_AT_MOST {
            let next = pause(waited);
            if waited < FINE {
                assert_eq!(next, STEP, "a wait within the budget looked coarsely");
            } else {
                assert!(
                    next <= waited / 10 + STEP,
                    "a wait of {waited:?} paused {next:?}, past a tenth of it"
                );
            }
            waited += next;
            looks += 1;
        }
        assert!(
            looks < 100,
            "a process outliving the wait was looked at {looks} times in {ENDING_AT_MOST:?}"
        );
    }

    #[test]
    fn a_wait_says_how_long_a_process_went_on_or_that_it_still_runs() {
        let looks = Cell::new(0);
        let ended = waited(
            9,
            |_| {
                looks.set(looks.get() + 1);
                looks.get() > 2
            },
            |_| Duration::from_millis(1),
            Duration::from_secs(5),
        );
        assert!(ended.is_some(), "a process that ended was said to run on");
        assert_eq!(
            looks.get(),
            3,
            "it wasn't looked at each step until it ended"
        );
        let running = waited(
            9,
            |_| false,
            |_| Duration::from_millis(1),
            Duration::from_millis(5),
        );
        assert_eq!(
            running, None,
            "a process still running was said to have ended"
        );
        assert_eq!(
            closed_line("zoom", None),
            "closed zoom; its process was still running 30 s later",
            "a process outliving the wait was said wrongly"
        );
        assert_eq!(
            closed_line("foot", Some(Duration::from_millis(12))),
            "closed foot; its process ended 12 ms later",
            "a process's end was said wrongly"
        );
    }

    #[test]
    fn a_start_is_read_past_a_name_with_spaces_and_the_uptime_to_its_hundredths() {
        let stat = "40 (Web Content (x)) S 1 40 40 0 -1 4194560 100 0 0 0 1 2 0 0 20 0 \
                    1 0 123456 0 0";
        assert_eq!(
            started_of(stat),
            Some(1_234_560),
            "the 22nd field wasn't read as ticks of a hundredth"
        );
        assert_eq!(
            uptime_of("8123.45 99.10\n"),
            Some(8_123_450),
            "the uptime was misread"
        );
        assert_eq!(
            opened_line("foot", 250),
            "opened foot 250 ms after its process started",
            "an opening was said wrongly"
        );
    }
}
