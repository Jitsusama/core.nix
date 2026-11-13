//! How a local day's connection did under load, from shaper-record's day
//! files, kept as a file a reader can hold to a bar: `shaper day` writes
//! `<local date>.json` into the state folder's `shaper/day`. The status
//! file the shaper keeps is live only, and the recorder's lines are a
//! second each, so nothing else says how a day went.
//!
//! A direction is under load in a minute it carried at least half its
//! full rate, the most it carried in all but the busiest hundredth of the
//! day's minutes: the recorder counts what the link carried, the home
//! network's own traffic too, so its very busiest minutes aren't the
//! connection's. Its added delay is each second's own delay over its
//! reflectors' least, as the controller reads it (src/replay.rs): out for
//! the upload, back for the download. Whether the shaper was on comes from
//! its unit's jobs in the journal, so a day reads the same with it on and
//! off, and the two are told apart.

use crate::rate::{float, whole};
use crate::record;
use crate::replay::{self, Sample};
use serde::Serialize;
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// A direction carrying at least this share of its full rate is under load.
const LOADED: f64 = 0.5;
/// The share of the day's minutes left out above the full rate.
const BUSIEST: f64 = 0.01;
/// A full rate is never taken under this, in kbit/s, so an idle day has no
/// minutes under load rather than every one.
const FULL_LEAST_KBIT: f64 = 1_000.0;
/// The reflectors' least looks back this far, in ms, so the samples from
/// before midnight are read too.
const LOOK_BACK_MS: u64 = 600_000;
/// How far back the journal is read for the shaper's state at midnight.
const JOURNAL_DAYS: u64 = 8;
/// Where journald keeps a machine's files: on disk, and in memory until
/// the next boot.
const JOURNAL_DIRS: [&str; 2] = ["/var/log/journal", "/run/log/journal"];
/// How long before a moment, in s, a journal file last changed may still
/// hold an entry from it.
const JOURNAL_MARGIN_S: u64 = 3_600;

/// The bars a day is held to, and the date they were set.
pub const BARS_SET: &str = "2026-11-12";
/// The most the 90th percentile of the added delay under load may be, in
/// ms: Waveform's bufferbloat test gives an A under 30.
pub const ADDED_DELAY_P90_MS: f64 = 30.0;
/// The share of probes that may go unanswered.
pub const PROBE_LOSS: f64 = 0.01;

/// The added delay a direction's seconds took, in ms.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Delay {
    pub p50: f64,
    pub p90: f64,
}

/// A direction under load, over some of the day's minutes.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Loaded {
    pub loaded_minutes: usize,
    /// None when no second under load heard a reflector.
    pub added_delay_ms: Option<Delay>,
}

/// A direction under load over the day, and with the shaper on and off.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Way {
    pub full_kbit: f64,
    #[serde(flatten)]
    pub all: Loaded,
    pub shaper_on: Loaded,
    pub shaper_off: Loaded,
    /// Whether the day's added delay under load kept to the bar; None when
    /// nothing was under load.
    pub meets_bar: Option<bool>,
}

/// The bars, as the file states them.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Bars {
    pub set: &'static str,
    pub added_delay_p90_ms_at_most: f64,
    pub probe_loss_under: f64,
}

/// What a day file holds.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Day {
    pub date: String,
    pub written_at: u64,
    pub minutes_recorded: usize,
    /// None when the journal couldn't be read.
    pub shaper_on_minutes: Option<usize>,
    pub up: Way,
    pub down: Way,
    /// None when no probe went.
    pub probe_loss: Option<f64>,
    pub probe_loss_meets_bar: Option<bool>,
    pub bars: Bars,
}

/// The value at a share of the way through sorted values, nearest rank.
fn rank(sorted: &[f64], share: f64) -> Option<f64> {
    if sorted.is_empty() {
        return None;
    }
    let last = sorted.len() - 1;
    let at = whole(share * float(last as u64));
    sorted.get(usize::try_from(at).unwrap_or(last)).copied()
}

fn sorted(mut values: Vec<f64>) -> Vec<f64> {
    values.sort_by(f64::total_cmp);
    values
}

/// What a minute's seconds carried each way, kbit/s, summed in the order
/// they came, and each way's delays. A day is folded into these as its
/// seconds go by, so it never holds more than a minute's worth of each.
#[derive(Default)]
struct Minute {
    kbit: [f64; 2],
    seconds: u64,
    delay: [Vec<f64>; 2],
}

impl Minute {
    /// What a way carried on average over the minute's seconds, or nothing
    /// carried when it had none.
    fn mean(&self, way: usize) -> f64 {
        if self.seconds == 0 {
            0.0
        } else {
            self.kbit[way] / float(self.seconds)
        }
    }
}

/// Which minutes the shaper was on in, given as minutes since 1970.
pub trait On {
    fn on(&self, minute: u64) -> bool;
}

/// A direction's load and delay, over the minutes `keep` lets through.
fn loaded(
    minutes: &BTreeMap<u64, Minute>,
    way: usize,
    full: f64,
    keep: impl Fn(u64) -> bool,
) -> Loaded {
    let mut count = 0;
    let mut delays = Vec::new();
    for minute in minutes.iter().filter(|&(&at, _)| keep(at)).map(|(_, m)| m) {
        if minute.mean(way) >= LOADED * full {
            count += 1;
            delays.extend(minute.delay[way].iter().copied());
        }
    }
    let delays = sorted(delays);
    Loaded {
        loaded_minutes: count,
        added_delay_ms: rank(&delays, 0.5)
            .zip(rank(&delays, 0.9))
            .map(|(p50, p90)| Delay { p50, p90 }),
    }
}

fn way(minutes: &BTreeMap<u64, Minute>, way: usize, on: &dyn On) -> Way {
    let means = sorted(minutes.values().map(|m| m.mean(way)).collect());
    let full = rank(&means, 1.0 - BUSIEST)
        .unwrap_or(0.0)
        .max(FULL_LEAST_KBIT);
    let all = loaded(minutes, way, full, |_| true);
    let meets_bar = all
        .added_delay_ms
        .as_ref()
        .map(|d| d.p90 <= ADDED_DELAY_P90_MS);
    Way {
        full_kbit: full.round(),
        shaper_on: loaded(minutes, way, full, |m| on.on(m)),
        shaper_off: loaded(minutes, way, full, |m| !on.on(m)),
        all,
        meets_bar,
    }
}

/// The day between two moments, in ms since 1970, from the samples read
/// up to it, with the shaper's minutes on when known. The samples are read
/// once, as they come, and folded into the day's minutes: a day's record
/// is half a million of them, which held whole took the most memory a day
/// file did.
pub fn summarise(
    samples: impl IntoIterator<Item = Sample>,
    (from, to): (u64, u64),
    on: Option<&dyn On>,
    date: &str,
    written_at: u64,
) -> Day {
    let inside = |at: u64| at >= from && at < to;
    let (mut lost, mut heard) = (0_u64, 0_u64);
    let counted = samples.into_iter().inspect(|sample| {
        if inside(sample.at()) {
            match sample {
                Sample::Lost { .. } => lost += 1,
                Sample::Stamp { .. } | Sample::Clock { .. } | Sample::Gateway { .. } => heard += 1,
                Sample::Pool { .. } => {}
            }
        }
    });
    let mut minutes: BTreeMap<u64, Minute> = BTreeMap::new();
    replay::seconds(counted, |began_ms, second| {
        let at = began_ms + whole(second.t_s * 1_000.0);
        if !inside(at) {
            return;
        }
        let minute = minutes.entry(at / 60_000).or_default();
        minute.kbit[0] += second.up_kbit;
        minute.kbit[1] += second.down_kbit;
        minute.seconds += 1;
        minute.delay[0].extend(second.up_delay_ms);
        minute.delay[1].extend(second.down_delay_ms);
    });
    let never = Never;
    let shaper: &dyn On = on.unwrap_or(&never);
    let probe_loss = (lost + heard > 0).then(|| float(lost) / float(lost + heard));
    Day {
        date: date.to_string(),
        written_at,
        minutes_recorded: minutes.len(),
        shaper_on_minutes: on.map(|on| minutes.keys().filter(|&&m| on.on(m)).count()),
        up: way(&minutes, 0, shaper),
        down: way(&minutes, 1, shaper),
        probe_loss,
        probe_loss_meets_bar: probe_loss.map(|l| l < PROBE_LOSS),
        bars: Bars {
            set: BARS_SET,
            added_delay_p90_ms_at_most: ADDED_DELAY_P90_MS,
            probe_loss_under: PROBE_LOSS,
        },
    }
}

/// Stands in when the journal can't say: every minute counts as off.
struct Never;

impl On for Never {
    fn on(&self, _: u64) -> bool {
        false
    }
}

/// The spells the shaper ran, in ms since 1970, from its unit's jobs.
#[derive(Debug, Default, PartialEq)]
pub struct Spells(pub Vec<(u64, u64)>);

impl On for Spells {
    fn on(&self, minute: u64) -> bool {
        let (start, end) = (minute * 60_000, (minute + 1) * 60_000);
        self.0.iter().any(|&(a, b)| a < end && b > start)
    }
}

/// Reads the spells from journalctl's JSON lines for the unit: a start job
/// done begins one, and a stop job, a unit that failed or a new boot ends
/// it. One still running at `now` runs to it.
pub fn spells(lines: &str, now: u64) -> Spells {
    let mut all = Vec::new();
    let mut began: Option<u64> = None;
    let mut boot: Option<String> = None;
    for line in lines.lines() {
        let Ok(entry) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        let field = |key: &str| entry.get(key).and_then(serde_json::Value::as_str);
        let Some(at) = field("__REALTIME_TIMESTAMP").and_then(|t| t.parse::<u64>().ok()) else {
            continue;
        };
        let at = at / 1_000;
        let this_boot = field("_BOOT_ID").map(String::from);
        if this_boot != boot {
            if let Some(from) = began.take() {
                all.push((from, at));
            }
            boot = this_boot;
        }
        let starts = field("JOB_TYPE") == Some("start") && field("JOB_RESULT") == Some("done");
        let ends = field("JOB_TYPE") == Some("stop") || field("UNIT_RESULT").is_some();
        if starts && began.is_none() {
            began = Some(at);
        } else if ends && let Some(from) = began.take() {
            all.push((from, at));
        }
    }
    if let Some(from) = began {
        all.push((from, now));
    }
    Spells(all)
}

/// The system's journal files that can hold an entry from `since`, in s
/// since 1970, on, among those in `dirs`. The shaper is a system unit, so
/// the users' files hold none of its lines. A file's last change is at or
/// after its newest entry, but for one written into a page journald
/// already had waiting to sync, which it does every few minutes; so a
/// file last changed over an hour before `since` holds nothing after it.
fn journal_files(dirs: &[PathBuf], since: u64) -> Vec<PathBuf> {
    let after = UNIX_EPOCH + Duration::from_secs(since.saturating_sub(JOURNAL_MARGIN_S));
    let mut files: Vec<PathBuf> = dirs
        .iter()
        .filter_map(|dir| fs::read_dir(dir).ok())
        .flatten()
        .filter_map(Result::ok)
        .filter(|entry| {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            name.starts_with("system")
                && (name.ends_with(".journal") || name.ends_with(".journal~"))
        })
        .filter(|entry| {
            entry
                .metadata()
                .and_then(|m| m.modified())
                .is_ok_and(|changed| changed >= after)
        })
        .map(|entry| entry.path())
        .collect();
    files.sort();
    files
}

/// Reads the shaper's spells from the journal. journalctl opening every
/// file costs it about half of each in the page cache, charged to the
/// unit: a gigabyte over 58 files on 13 November, when the spells' week
/// was in 11 of them. So it's handed those that can hold the week, or left
/// to find them itself when none are found, as where it can't read them.
fn journal(from: u64, to: u64, now: u64) -> Option<Spells> {
    let since = (from / 1_000).saturating_sub(JOURNAL_DAYS * 86_400);
    let dirs: Vec<PathBuf> = fs::read_to_string("/etc/machine-id")
        .map(|id| {
            JOURNAL_DIRS
                .iter()
                .map(|dir| Path::new(dir).join(id.trim()))
                .collect()
        })
        .unwrap_or_default();
    let files = journal_files(&dirs, since).into_iter().map(|file| {
        let mut arg = OsString::from("--file=");
        arg.push(file);
        arg
    });
    let out = Command::new("journalctl")
        .args(files)
        .args(["-u", "shaper.service", "-o", "json", "--no-pager", "-q"])
        .args([
            "--output-fields=JOB_TYPE,JOB_RESULT,UNIT_RESULT".to_string(),
            format!("--since=@{since}"),
            format!("--until=@{}", to / 1_000),
        ])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    out.status
        .success()
        .then(|| spells(&String::from_utf8_lossy(&out.stdout), now))
}

/// The samples the record holds from a little before `from` up to `to`,
/// read as they're asked for.
fn samples(record: &Path, from: u64, to: u64) -> impl Iterator<Item = Sample> {
    let at = |ms: u64| UNIX_EPOCH + Duration::from_millis(ms);
    let first = from.saturating_sub(LOOK_BACK_MS);
    let mut files = vec![record::day_file(record, at(first))];
    for day in (first / 86_400_000 + 1)..=to.saturating_sub(1) / 86_400_000 {
        files.push(record::day_file(record, at(day * 86_400_000)));
    }
    files.dedup();
    files
        .into_iter()
        .filter_map(|path| File::open(path).ok())
        .flat_map(|file| BufReader::new(file).lines().map_while(Result::ok))
        .filter_map(|line| replay::parse(&line))
        .filter(move |s| s.at() + LOOK_BACK_MS >= from && s.at() < to)
}

/// A local date's first moment and the next day's, in ms since 1970.
fn bounds(date: &str) -> Option<(u64, u64)> {
    let mut parts = date.splitn(3, '-').map(|p| p.parse::<i32>().ok());
    let (y, m, d) = (parts.next()??, parts.next()??, parts.next()??);
    let midnight = |day: i32| {
        // SAFETY: tm is plain data, zeroed is a valid value, and mktime
        // only reads and normalises the one it is handed.
        let mut tm: libc::tm = unsafe { std::mem::zeroed() };
        tm.tm_year = y - 1900;
        tm.tm_mon = m - 1;
        tm.tm_mday = day;
        tm.tm_isdst = -1;
        // SAFETY: as above.
        let at = unsafe { libc::mktime(&raw mut tm) };
        u64::try_from(at).ok().map(|s| s * 1_000)
    };
    Some((midnight(d)?, midnight(d + 1)?))
}

/// The local date a moment falls on, in ms since 1970.
fn local_date(ms: u64) -> String {
    let at = libc::time_t::try_from(ms / 1_000).unwrap_or(0);
    // SAFETY: tm is plain data, and localtime_r writes only into it.
    let mut tm: libc::tm = unsafe { std::mem::zeroed() };
    // SAFETY: both pointers are to live values of their types.
    unsafe { libc::localtime_r(&raw const at, &raw mut tm) };
    format!(
        "{:04}-{:02}-{:02}",
        tm.tm_year + 1900,
        tm.tm_mon + 1,
        tm.tm_mday
    )
}

fn now_ms() -> u64 {
    let since = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    u64::try_from(since.as_millis()).unwrap_or(0)
}

/// Writes one date's file, whole, so a reader never sees half of one.
fn write(record: &Path, out: &Path, date: &str, now: u64) -> Result<(), String> {
    let (from, to) = bounds(date).ok_or_else(|| format!("{date} isn't a date"))?;
    let spells = journal(from, to, now);
    let day = summarise(
        samples(record, from, to.min(now)),
        (from, to),
        spells.as_ref().map(|s| s as &dyn On),
        date,
        now,
    );
    fs::create_dir_all(out).map_err(|e| format!("{}: {e}", out.display()))?;
    let path = out.join(format!("{date}.json"));
    let part = out.join(format!(".{date}.json.part"));
    let text = serde_json::to_string_pretty(&day).expect("a day serialises") + "\n";
    fs::write(&part, text).map_err(|e| format!("{}: {e}", part.display()))?;
    fs::rename(&part, &path).map_err(|e| format!("{}: {e}", path.display()))
}

const USAGE: &str = "shaper day [--record DIR] [--out DIR] [YYYY-MM-DD]";

/// Writes the date asked for, or today's and, until it has been written
/// after it ended, yesterday's.
pub fn command(args: &[String]) -> ExitCode {
    let mut record = PathBuf::from("/var/lib/shaper-record");
    let state = std::env::var_os("XDG_STATE_HOME").map_or_else(
        || PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".local/state"),
        PathBuf::from,
    );
    let mut out = state.join("shaper/day");
    let mut dates = Vec::new();
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        match (arg.as_str(), args.clone().next()) {
            ("--record", Some(dir)) => record = PathBuf::from(dir),
            ("--out", Some(dir)) => out = PathBuf::from(dir),
            (date, _) if !date.starts_with("--") => {
                dates.push(date.to_string());
                continue;
            }
            _ => {
                eprintln!("usage: {USAGE}");
                return ExitCode::from(2);
            }
        }
        args.next();
    }
    let now = now_ms();
    if dates.is_empty() {
        let yesterday = local_date(now - 86_400_000);
        let written = fs::read_to_string(out.join(format!("{yesterday}.json")))
            .ok()
            .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
            .and_then(|v| v.get("written_at").and_then(serde_json::Value::as_u64));
        if bounds(&yesterday).is_some_and(|(_, end)| written.is_none_or(|w| w < end)) {
            dates.push(yesterday);
        }
        dates.push(local_date(now));
    }
    let mut code = ExitCode::SUCCESS;
    for date in &dates {
        if let Err(e) = write(&record, &out, date, now) {
            eprintln!("{e}");
            code = ExitCode::FAILURE;
        }
    }
    code
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    const REFLECTOR: Ipv4Addr = Ipv4Addr::new(9, 9, 9, 9);

    /// A recording, a sample each second from `seconds`' first, sending
    /// `up` kbit/s with `out` ms of delay out each second.
    fn recording(seconds: std::ops::Range<u64>, each: impl Fn(u64) -> (u64, f64)) -> Vec<Sample> {
        let mut tx = 0;
        let mut all = vec![Sample::Pool {
            at: seconds.start * 1_000,
            network: Some("home".into()),
            link: None,
        }];
        for s in seconds {
            let (kbit, out) = each(s);
            tx += kbit * 1_000 / 8;
            all.push(Sample::Stamp {
                at: s * 1_000 + 500,
                reflector: REFLECTOR,
                out: 10.0 + out,
                back: 10.0,
                rx: 0,
                tx,
            });
        }
        all
    }

    fn day(samples: Vec<Sample>, to: u64, on: Option<&dyn On>) -> Day {
        summarise(samples, (0, to), on, "1970-01-01", to)
    }

    #[test]
    fn a_loaded_minute_gives_its_added_delay_and_an_idle_one_none() {
        // Ten idle minutes, then ten sending 10 Mbit/s with 40 ms out.
        let samples = recording(
            0..1_200,
            |s| if s < 600 { (0, 0.0) } else { (10_000, 40.0) },
        );
        let got = day(samples, 1_200_000, None);
        assert_eq!(got.minutes_recorded, 20, "the day's minutes: {got:?}");
        assert!(
            (9..=10).contains(&got.up.all.loaded_minutes),
            "the upload's loaded minutes: {:?}",
            got.up
        );
        let delay = got
            .up
            .all
            .added_delay_ms
            .clone()
            .expect("a delay under load");
        assert!(
            (39.0..=41.0).contains(&delay.p90),
            "the added delay under load: {delay:?}"
        );
        assert_eq!(
            got.up.meets_bar,
            Some(false),
            "40 ms is over the bar: {:?}",
            got.up
        );
        assert_eq!(
            got.down.all.loaded_minutes, 0,
            "nothing came down: {:?}",
            got.down
        );
        assert_eq!(
            got.down.meets_bar, None,
            "nothing to hold down to: {:?}",
            got.down
        );
    }

    #[test]
    fn minutes_are_split_by_whether_the_shaper_was_on() {
        // Loaded throughout, the path's least heard in the first minute;
        // then 60 ms out with the shaper off, and 5 with it on for the
        // second ten minutes.
        let samples = recording(0..1_200, |s| {
            (
                10_000,
                if s < 60 {
                    0.0
                } else if s < 600 {
                    60.0
                } else {
                    5.0
                },
            )
        });
        let spells = Spells(vec![(600_000, 1_200_000)]);
        let got = day(samples, 1_200_000, Some(&spells));
        assert_eq!(got.shaper_on_minutes, Some(10), "minutes on: {got:?}");
        let on = got.up.shaper_on.added_delay_ms.clone().expect("a delay on");
        let off = got
            .up
            .shaper_off
            .added_delay_ms
            .clone()
            .expect("a delay off");
        assert!(on.p90 < 10.0, "with the shaper on: {on:?}");
        assert!(off.p50 > 50.0, "with the shaper off: {off:?}");
    }

    #[test]
    fn probes_lost_are_a_share_of_those_sent() {
        let mut samples = recording(0..99, |_| (0, 0.0));
        samples.push(Sample::Lost { at: 99_500 });
        let got = day(samples, 100_000, None);
        assert!(
            got.probe_loss.is_some_and(|l| (l - 0.01).abs() < 1e-9),
            "one lost of a hundred: {got:?}"
        );
        assert_eq!(
            got.probe_loss_meets_bar,
            Some(false),
            "1% isn't under 1%: {got:?}"
        );
    }

    #[test]
    fn only_the_system_s_journal_files_changed_since_are_read() {
        let dir = std::env::temp_dir().join(format!("shaper-day-journal-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("a folder for the files");
        let since = 1_800_000_000;
        let at = |s: u64| UNIX_EPOCH + Duration::from_secs(s);
        let files = [
            ("system.journal", since + 600),
            ("system@a-1-2.journal", since - 600),
            ("system@b-3-4.journal", since - 7_200),
            ("system@c-5.journal~", since + 60),
            ("user-1000.journal", since + 600),
            ("system.notes", since + 600),
        ];
        for (name, changed) in files {
            let file = File::create(dir.join(name)).expect("a journal file");
            file.set_modified(at(changed)).expect("its time set");
        }
        let got = journal_files(&[dir.clone(), dir.join("absent")], since);
        fs::remove_dir_all(&dir).expect("the folder gone");
        let names: Vec<String> = got
            .iter()
            .filter_map(|p| p.file_name())
            .map(|n| n.to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            names,
            [
                "system.journal",
                "system@a-1-2.journal",
                "system@c-5.journal~"
            ],
            "the system's files changed within the hour before since or after"
        );
    }

    #[test]
    fn a_spell_runs_from_a_start_to_a_stop_or_a_new_boot() {
        let line = |us: u64, boot: &str, fields: &str| {
            format!(r#"{{"__REALTIME_TIMESTAMP":"{us}","_BOOT_ID":"{boot}"{fields}}}"#)
        };
        let started = r#","JOB_TYPE":"start","JOB_RESULT":"done""#;
        let lines = [
            line(1_000_000, "a", started),
            line(2_000_000, "a", r#","JOB_TYPE":"stop""#),
            line(3_000_000, "a", started),
            line(4_000_000, "b", ""),
            line(5_000_000, "b", started),
        ]
        .join("\n");
        assert_eq!(
            spells(&lines, 6_000),
            Spells(vec![(1_000, 2_000), (3_000, 4_000), (5_000, 6_000)]),
            "the spells from the journal's lines"
        );
    }
}
