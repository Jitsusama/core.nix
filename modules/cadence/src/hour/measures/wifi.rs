//! The Wi-Fi as a call over it meets it, from wifid's minute lines in the
//! system journal. Each minute wifid times its own echoes to the gateway,
//! one a second marked as video (TOS 0xa0) as a call's video is, so every
//! minute on Wi-Fi has a reading, a call on or not (PN2VZC, 23 November).
//! On 22 November the eero held that class for up to 5 s while best effort
//! answered in 6 ms, which the call test can't see, since its call never
//! leaves the machine. Counting the minutes that held it puts the next jam
//! on the card, in the drag of his apps, before he hears it on a call.

use std::collections::BTreeMap;

use jiff::Timestamp;
use jiff::tz::TimeZone;
use serde_json::Value;

use super::{Class, Measure, count, lines};

/// What the measure is known by from one hour to the next.
pub const KEY: &str = "wifi-video-held";

/// Who keeps wifid and the eero, and so moves a held video class.
const OWNER: &str = "PN2VZC";

/// A minute holds a call's video past this at its 95th percentile, late
/// answers included, or with over `LATE_PERCENT` of its echoes answered
/// past a second: PN2VZC's bars of 23 November, well between a clean
/// point's 14 to 48 ms with none late and a holding one's 2.4 to 3.9 s
/// with 16 to 38% late. Only a held queue answers late, in order, so late
/// answers count on every line, while an echo never answered says nothing
/// of the air and a p95 of too few answers isn't weighed.
const HELD_MS: f64 = 100.0;
const LATE_PERCENT: f64 = 1.0;

/// How many held minutes an hour may have: two, since each line covers
/// the two minutes before it, so one bad moment shows in two running.
const TARGET: f64 = 2.0;

const SAYS: &str = "minutes the Wi-Fi held a call's video past 100 ms at p95 or over 1% of it \
                    past 1 s, this hour";

/// journalctl's arguments for wifid's lines in the hour to `now`, every
/// one of them, since `--grep` matching nothing exits as if it failed.
#[must_use]
pub fn journal(now: i64) -> [String; 7] {
    [
        "-u".to_owned(),
        "wifid.service".to_owned(),
        "-o".to_owned(),
        "json".to_owned(),
        "--no-pager".to_owned(),
        format!("--since=@{}", now - 3600),
        format!("--until=@{now}"),
    ]
}

/// One minute's reading of the video class, as wifid writes it after its
/// best-effort figures: `wlp0s20f3 on <bssid>: gateway ...; video 2400.0
/// ms at p95, 37.5% of 120 echoes answered past 1 s, 2 unanswered; ...`,
/// ending `, during a call` while one is on. Best effort's p95 is
/// kept beside it, since wifid times its own echoes, so a minute with the
/// machine starved of CPU reads slow in both, where one held in the air
/// leaves best effort quick; it is none where every best-effort echo went
/// unanswered. The video p95 is none on a clause ending `, too few to
/// time`, fewer than half its echoes answered.
#[derive(Debug, PartialEq)]
struct Minute {
    at: i64,
    bssid: String,
    best: Option<f64>,
    p95: Option<f64>,
    late: f64,
    call: bool,
}

impl Minute {
    fn held(&self) -> bool {
        self.late > LATE_PERCENT || self.p95.is_some_and(|p95| p95 > HELD_MS)
    }

    /// Whether enough of its video was answered to judge it: timed, or
    /// held by the answers that came late. One whose video went mostly
    /// unanswered and never late is set aside, neither held nor clean, as
    /// a gateway dropping marked echoes would read held every minute.
    fn answered(&self) -> bool {
        self.p95.is_some() || self.held()
    }

    /// How far it held, for choosing an access point's worst minute: its
    /// late share first, as every line has one, then its timed p95.
    fn worse(&self, other: &Self) -> std::cmp::Ordering {
        let p95 = |m: &Self| m.p95.unwrap_or(0.0);
        self.late
            .total_cmp(&other.late)
            .then(p95(self).total_cmp(&p95(other)))
    }
}

/// A time as wifid writes it, `2400.0 ms` or `none` when nothing was
/// answered to time, or none for anything else.
fn time(said: &str) -> Option<Option<f64>> {
    if said == "none" {
        return Some(None);
    }
    Some(Some(said.strip_suffix(" ms")?.parse().ok()?))
}

/// A minute read from one journal entry: none for any other line, and for
/// a minute with no video echo sent, whose clause reads `video
/// unmeasured`, or whose line from an older wifid leaves it out.
fn minute(entry: &Value) -> Option<Minute> {
    let message = entry["MESSAGE"].as_str()?;
    let micros: i64 = entry["__REALTIME_TIMESTAMP"].as_str()?.parse().ok()?;
    let (_, rest) = message.split_once(" on ")?;
    let (bssid, rest) = rest.split_once(": gateway ")?;
    let (_, best) = rest.split_once(" at p50, ")?;
    let (best, _) = best.split_once(" at p95, ")?;
    let (_, video) = rest.split_once("; video ")?;
    let video = video.split(';').next()?;
    let (video, timed) = match video.strip_suffix(", too few to time") {
        Some(few) => (few, false),
        None => (video, true),
    };
    let (p95, video) = video.split_once(" at p95, ")?;
    let (late, video) = video.split_once("% of ")?;
    video.split_once(" echoes answered past 1 s")?;
    let p95 = if timed { Some(time(p95)??) } else { None };
    Some(Minute {
        at: micros / 1_000_000,
        bssid: bssid.to_owned(),
        best: time(best)?,
        p95,
        late: late.parse().ok()?,
        call: message.ends_with(", during a call"),
    })
}

/// The hour's held minutes, from wifid's journal as journalctl printed it,
/// or none where it couldn't be read. Behind the figure, each access point
/// its minutes held on, most first, with its worst minute, so the owner
/// reads which radio jammed and when. Minutes whose video went mostly
/// unanswered, and wifid's minutes that couldn't be read, are counted
/// beside the figure. An hour with no minute judged is unjudged, saying
/// why: the journal unread, no wifid line at all, as journalctl prints
/// when it may not read the journal, no Wi-Fi minute, wired or asleep,
/// minutes unread, no video echo sent, a wifid writing no video figure,
/// or every minute's video unanswered.
#[must_use]
pub fn video(journal: Option<&str>, zone: &TimeZone) -> Measure {
    let mut tally = journal.map(tallied);
    let (minutes, aside): (Vec<Minute>, Vec<Minute>) = tally
        .as_mut()
        .map(|t| std::mem::take(&mut t.minutes))
        .unwrap_or_default()
        .into_iter()
        .partition(Minute::answered);
    let unjudged = match &tally {
        None => Some(": wifid's journal unread".to_owned()),
        Some(r) if r.lines == 0 => {
            Some(": no wifid line, so the journal may be unreadable here or wifid stopped".into())
        }
        Some(r) if r.wifi == 0 => Some(": no Wi-Fi minute".to_owned()),
        Some(_) if !minutes.is_empty() => None,
        Some(r) if r.unread > 0 => Some(format!(": {} of wifid's minutes unread", r.unread)),
        Some(_) if !aside.is_empty() => Some(": every minute's video went unanswered".into()),
        Some(r) if r.unmeasured > 0 => Some(": no video echo sent".to_owned()),
        Some(_) => Some(": wifid writes no video figure".to_owned()),
    };
    if let Some(why) = unjudged {
        return Measure {
            key: KEY.to_owned(),
            class: Class::Apps,
            says: format!("{SAYS}{why}"),
            now: 0.0,
            target: None,
            owner: OWNER.to_owned(),
            behind: Vec::new(),
        };
    }
    let mut points: BTreeMap<&str, Vec<&Minute>> = BTreeMap::new();
    for m in &minutes {
        points.entry(&m.bssid).or_default().push(m);
    }
    let mut behind: Vec<(usize, String)> = points
        .into_iter()
        .filter_map(|(bssid, read)| {
            let held: Vec<&&Minute> = read.iter().filter(|m| m.held()).collect();
            let worst = held.iter().max_by(|a, b| a.worse(b))?;
            Some((held.len(), said(bssid, held.len(), read.len(), worst, zone)))
        })
        .collect();
    behind.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
    let held: usize = behind.iter().map(|(n, _)| n).sum();
    let unanswered = match aside.len() {
        0 => String::new(),
        n => format!(", {n} more with its video unanswered"),
    };
    let unread = match tally.map_or(0, |t| t.unread) {
        0 => String::new(),
        n => format!(", {n} unread"),
    };
    Measure {
        key: KEY.to_owned(),
        class: Class::Apps,
        says: format!("{SAYS}, of {} read{unanswered}{unread}", minutes.len()),
        now: count(held),
        target: Some(TARGET),
        owner: OWNER.to_owned(),
        behind: behind.into_iter().map(|(_, line)| line).collect(),
    }
}

/// What an hour of wifid's journal holds: its lines, those that are a
/// Wi-Fi minute, and of those, each one read, or why it wasn't.
#[derive(Default)]
struct Tally {
    lines: usize,
    wifi: usize,
    unmeasured: usize,
    unread: usize,
    minutes: Vec<Minute>,
}

/// The tally of `journal`, as journalctl's json printed it. A minute with
/// no video clause, as an older wifid writes, is neither unmeasured nor
/// unread.
fn tallied(journal: &str) -> Tally {
    let mut tally = Tally::default();
    for entry in lines(journal) {
        tally.lines += 1;
        let Some(message) = entry["MESSAGE"].as_str() else {
            continue;
        };
        if !message.contains(": gateway ") {
            continue;
        }
        tally.wifi += 1;
        if let Some(read) = minute(&entry) {
            tally.minutes.push(read);
        } else if message.contains("; video unmeasured") {
            tally.unmeasured += 1;
        } else if message.contains("; video ") {
            tally.unread += 1;
        }
    }
    tally
}

/// One access point's held minutes, as the card names them.
fn said(bssid: &str, held: usize, read: usize, worst: &Minute, zone: &TimeZone) -> String {
    let when = Timestamp::from_second(worst.at)
        .map(|at| at.to_zoned(zone.clone()).strftime(" at %H:%M").to_string())
        .unwrap_or_default();
    let call = if worst.call { ", during a call" } else { "" };
    let timed = worst
        .p95
        .map(|p95| format!("{p95:.1} ms at p95 and "))
        .unwrap_or_default();
    let best = worst
        .best
        .map_or("unanswered".to_owned(), |best| format!("{best:.1} ms"));
    let late = worst.late;
    format!(
        "on {bssid}, {held} of {read} minutes held, the worst {timed}{late:.1}% past 1 s\
         {when}{call}, best effort then {best}"
    )
}

#[cfg(test)]
mod tests;
