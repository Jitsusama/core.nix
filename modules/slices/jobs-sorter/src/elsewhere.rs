//! Follows whether a call is on on another of the person's machines, such
//! as a laptop at the same desk, so a call there brakes builds and jobs
//! here as a call here does (`brake_for_calls` in src/main.rs).
//!
//! A meeting begun on one machine is often carried on on another, and its
//! person is the same: the machine left beside it went on loading their
//! network and fans, unbraked, for the whole call. Each machine's mycelium
//! node runs a-call-is-on as a probe and says what it answered, as
//! `reached.call`, in the machine's record, which every node of the realm
//! hears from its peers (modules/mycelium). This asks this machine's own
//! node for those records, every realm whose socket it serves, on a thread
//! of its own so the sorter's loop never waits on it.
//!
//! It reads strictly, and fails open: only a literal `true`, in a record
//! measured lately, is a call. A machine that can't be reached, a record
//! gone stale, a probe that couldn't tell and anything else unreadable is
//! no call, so nothing elsewhere can brake builds here for ever. A call
//! heard stands two minutes past its last yes, so one probe that ran past
//! its time doesn't let go mid-call.

use serde_json::Value;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// How often the node is asked: its records are measured each minute, and
/// each probe is run every thirty seconds.
const EVERY: Duration = Duration::from_secs(30);

/// How long a node has to answer before it counts as saying nothing.
const ASK_WITHIN: Duration = Duration::from_secs(10);

/// How old a record may be, in seconds, and still say a call is on: three
/// of its minutes, so one late measure doesn't drop a call, while a machine
/// gone quiet stops braking here within a few minutes.
const FRESH: i64 = 180;

/// How long a call heard elsewhere stands past its last yes.
pub const HELD: Duration = Duration::from_secs(120);

/// Whether a call was heard elsewhere lately, as the thread last heard.
pub struct Elsewhere {
    heard: Arc<Mutex<Heard>>,
}

impl Elsewhere {
    /// Starts asking `mycelium` for the records of every realm whose node's
    /// socket is in `sockets`.
    pub fn follow(sockets: &Path, mycelium: &Path) -> Elsewhere {
        let heard = Arc::new(Mutex::new(Heard::default()));
        let hearing = Arc::clone(&heard);
        let (sockets, mycelium) = (sockets.to_path_buf(), mycelium.to_path_buf());
        // The node's answer holds this machine's own record too, whose call
        // the sorter already hears from the sound server; mycelium names a
        // machine by its hostname.
        let here = fs::read_to_string("/proc/sys/kernel/hostname")
            .map(|name| name.trim().to_owned())
            .unwrap_or_default();
        let followed = thread::Builder::new()
            .name("calls-elsewhere".into())
            .spawn(move || follow(&sockets, &mycelium, &here, &hearing));
        if let Err(error) = followed {
            eprintln!("jobs-sorter: calls elsewhere can't be followed: {error}");
        }
        Elsewhere { heard }
    }

    /// Whether a call elsewhere stands `now`.
    pub fn on(&self, now: Instant) -> bool {
        self.heard
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .on(now)
    }
}

/// When a call was last heard elsewhere.
#[derive(Debug, Default)]
pub struct Heard {
    yes: Option<Instant>,
}

impl Heard {
    /// Takes what one round of asking said, `yes` when some machine said a
    /// call is on, at `at`.
    pub fn hear(&mut self, yes: bool, at: Instant) {
        if yes {
            self.yes = Some(at);
        }
    }

    /// Whether a call stands `now`: one heard in the last [`HELD`].
    pub fn on(&self, now: Instant) -> bool {
        self.yes
            .is_some_and(|yes| now.saturating_duration_since(yes) < HELD)
    }
}

/// Whether a node's answer to `mycelium machines` says a call is on on any
/// machine but `here` whose record was measured within [`FRESH`] of `now`,
/// in seconds since the epoch. Only a literal `true` under `reached.call`
/// counts. The answer is a JSON document a line, its machines' and then
/// its builders', read up to the first that isn't JSON.
pub fn a_call_in(answer: &str, now: i64, here: &str) -> bool {
    let mut machines = serde_json::Deserializer::from_str(answer)
        .into_iter::<Value>()
        .map_while(Result::ok)
        .filter_map(|document| document["Ok"]["Machines"].as_array().cloned())
        .flatten();
    machines.any(|machine| {
        let measured = machine["measured_at"].as_str().and_then(seconds_of);
        let fresh = measured.is_some_and(|at| (now - FRESH..=now + 60).contains(&at));
        let elsewhere = machine["name"].as_str() != Some(here);
        elsewhere && fresh && machine["reached"]["call"] == Value::Bool(true)
    })
}

/// The seconds since the epoch of an RFC 3339 time in UTC, as mycelium
/// writes it, `2026-11-17T14:16:00.36Z`; any other shape is none.
fn seconds_of(stamp: &str) -> Option<i64> {
    let stamp = stamp.strip_suffix('Z')?;
    let (date, time) = stamp.split_once('T')?;
    let time = time.split_once('.').map_or(time, |(whole, _)| whole);
    let figures = |text: &str| -> Option<Vec<i64>> {
        text.split(['-', ':'])
            .map(|figure| figure.parse().ok())
            .collect()
    };
    let (date, time) = (figures(date)?, figures(time)?);
    let ([year, month, day], [hour, minute, second]) = (date.as_slice(), time.as_slice()) else {
        return None;
    };
    let in_range = (1..=12).contains(month)
        && (1..=31).contains(day)
        && (0..24).contains(hour)
        && (0..60).contains(minute)
        && (0..=60).contains(second);
    in_range.then(|| {
        days_from_civil(*year, *month, *day) * 86_400 + hour * 3_600 + minute * 60 + second
    })
}

/// Days since 1970-01-01 of a date in the proleptic Gregorian calendar
/// (Howard Hinnant's `days_from_civil`).
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let of_era = year - era * 400;
    let of_year = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day - 1;
    let of_cycle = of_era * 365 + of_era / 4 - of_era / 100 + of_year;
    era * 146_097 + of_cycle - 719_468
}

/// Asks each realm's node in `sockets`, every [`EVERY`], into `heard`.
fn follow(sockets: &Path, mycelium: &Path, here: &str, heard: &Mutex<Heard>) {
    loop {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |since| i64::try_from(since.as_secs()).unwrap_or(0));
        let yes = realms(sockets).iter().any(|(realm, socket)| {
            ask(mycelium, realm, socket).is_some_and(|answer| a_call_in(&answer, now, here))
        });
        heard
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .hear(yes, Instant::now());
        thread::sleep(EVERY);
    }
}

/// Each realm whose node's socket, `<realm>.sock`, is in `sockets`.
fn realms(sockets: &Path) -> Vec<(String, PathBuf)> {
    let Ok(entries) = fs::read_dir(sockets) else {
        return Vec::new();
    };
    entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            let realm = path
                .extension()
                .is_some_and(|extension| extension == "sock")
                .then(|| path.file_stem()?.to_str().map(str::to_owned))??;
            Some((realm, path))
        })
        .collect()
}

/// What `mycelium machines` says through `socket`, for `realm`, or none
/// when it can't run, fails or doesn't answer within [`ASK_WITHIN`].
fn ask(mycelium: &Path, realm: &str, socket: &Path) -> Option<String> {
    let mut asked = Command::new(mycelium)
        .arg("--realm")
        .arg(realm)
        .arg("machines")
        .arg("--socket")
        .arg(socket)
        .env_remove("MYCELIUM_REALM")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let mut out = asked.stdout.take()?;
    let reading = thread::spawn(move || {
        let mut answer = String::new();
        out.read_to_string(&mut answer).ok().map(|_| answer)
    });
    let deadline = Instant::now() + ASK_WITHIN;
    let status = loop {
        match asked.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(100)),
            _ => {
                let _ = asked.kill();
                let _ = asked.wait();
                break None;
            }
        }
    };
    let answer = reading.join().ok().flatten()?;
    status
        .filter(std::process::ExitStatus::success)
        .map(|_| answer)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// This machine's name, as the tests give it.
    const HERE: &str = "cistern";

    /// A node's answer as `mycelium machines` writes it, a document a line:
    /// its machines, this one quiet and the laptop beside it measured at
    /// `measured_at` with its probes saying `reached`, then its builders.
    fn answer(measured_at: &str, reached: &str) -> String {
        let here = format!(r#"{{"name":"{HERE}","measured_at":"{measured_at}","reached":{{}}}}"#);
        let machine =
            format!(r#"{{"name":"skiff","measured_at":"{measured_at}","reached":{reached}}}"#);
        let builders = r#"{"Ok":{"Builders":[]}}"#;
        format!("{{\"Ok\":{{\"Machines\":[{here},{machine}]}}}}\n{builders}\n")
    }

    /// 2026-11-17T14:16:00Z, in seconds since the epoch.
    const AT_0916: i64 = 1_794_924_960;

    #[test]
    fn mycelium_times_are_read_as_seconds_since_the_epoch() {
        assert_eq!(seconds_of("1970-01-01T00:00:00Z"), Some(0), "the epoch");
        assert_eq!(
            seconds_of("2026-11-17T14:16:00.365918927Z"),
            Some(AT_0916),
            "a node's measure, with its fraction"
        );
        assert_eq!(
            seconds_of("2024-02-29T23:59:59Z"),
            Some(1_709_251_199),
            "a leap day"
        );
        for unread in [
            "2026-11-17T14:16:00",
            "2026-11-17T14:16:00+01:00",
            "2026-13-17T14:16:00Z",
            "2026-11-17T24:16:00Z",
            "2026-11-17 14:16:00Z",
            "",
        ] {
            assert_eq!(seconds_of(unread), None, "{unread:?} was read as a time");
        }
    }

    #[test]
    fn only_a_literal_yes_in_a_fresh_record_is_a_call() {
        let now = AT_0916 + 30;
        assert!(
            a_call_in(
                &answer("2026-11-17T14:16:00Z", r#"{"call":true}"#),
                now,
                HERE
            ),
            "a fresh record saying a call is on wasn't heard as one"
        );
        let own = format!(
            r#"{{"Ok":{{"Machines":[{{"name":"{HERE}","measured_at":"2026-11-17T14:16:00Z","reached":{{"call":true}}}}]}}}}"#
        );
        assert!(
            !a_call_in(&own, now, HERE),
            "this machine's own call, which the sound server already says, was heard as one elsewhere"
        );
        for (said, why) in [
            (answer("2026-11-17T14:16:00Z", r#"{"call":false}"#), "a no"),
            (
                answer("2026-11-17T14:16:00Z", r#"{"call":"true"}"#),
                "a string",
            ),
            (answer("2026-11-17T14:16:00Z", r#"{"call":1}"#), "a number"),
            (
                answer("2026-11-17T14:16:00Z", r#"{"home":true}"#),
                "no call probe",
            ),
            (answer("2026-11-17T14:16:00Z", "{}"), "no probes"),
            (
                answer("2026-11-17T14:12:00Z", r#"{"call":true}"#),
                "a record four minutes old",
            ),
            (
                answer("2026-11-17T14:18:00Z", r#"{"call":true}"#),
                "a record from the future",
            ),
            (
                answer("not a time", r#"{"call":true}"#),
                "a record with no time",
            ),
            (r#"{"Err":"no such realm"}"#.to_owned(), "a refusal"),
            ("not json".to_owned(), "an answer that isn't JSON"),
        ] {
            assert!(
                !a_call_in(&said, now, HERE),
                "{why} was heard as a call: {said}"
            );
        }
    }

    #[test]
    fn a_call_heard_elsewhere_holds_this_mornings_two_calls_and_lets_go_after_each() {
        // On 17 November a meeting moved from this machine to the laptop
        // beside it at 09:16 and ran to 10:05, and another ran there from
        // 10:22 to 11:12, while this machine's own microphone was off from
        // 09:16:54: each minute, the node's answer as it would have read,
        // the laptop's record measured that minute and read 30 s on, one
        // round of asking a minute missing its yes, as a probe past its
        // time would.
        let start = Instant::now();
        let minute = |of_day: u64| start + Duration::from_secs((of_day - 9 * 60) * 60);
        let on_call = |of_day: u64| (556..605).contains(&of_day) || (622..672).contains(&of_day);
        let mut heard = Heard::default();
        for of_day in 9 * 60..11 * 60 + 40 {
            let missed = of_day == 580 || of_day == 640;
            let (hour, of_hour) = (of_day / 60 + 5, of_day % 60);
            let measured = format!("2026-11-17T{hour:02}:{of_hour:02}:00Z");
            let said = if on_call(of_day) && !missed {
                r#"{"call":true}"#
            } else {
                r#"{"call":false}"#
            };
            let read_at = AT_0916 + (i64::try_from(of_day).unwrap_or(0) - 556) * 60 + 30;
            let yes = a_call_in(&answer(&measured, said), read_at, HERE);
            heard.hear(yes, minute(of_day));
            let braked = heard.on(minute(of_day));
            if on_call(of_day) {
                assert!(
                    braked,
                    "the laptop's call wasn't braked for at {}:{:02}",
                    of_day / 60,
                    of_day % 60
                );
            }
            let after =
                |end: u64| (end + 2..622).contains(&of_day) || of_day >= end + 2 && end == 672;
            if after(605) || after(672) || of_day < 556 {
                assert!(
                    !braked,
                    "builds stayed braked at {}:{:02}, with no call on",
                    of_day / 60,
                    of_day % 60
                );
            }
        }
    }
}
