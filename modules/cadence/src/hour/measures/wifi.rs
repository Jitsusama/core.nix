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
/// with 16 to 38% late. A held queue answers late, in order, so an echo
/// never answered says nothing of the air and isn't weighed.
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
/// leaves best effort quick.
#[derive(Debug, PartialEq)]
struct Minute {
    at: i64,
    bssid: String,
    best: f64,
    p95: f64,
    late: f64,
    call: bool,
}

impl Minute {
    fn held(&self) -> bool {
        self.p95 > HELD_MS || self.late > LATE_PERCENT
    }
}

/// A minute read from one journal entry: none for any other line, for a
/// minute with no video echo sent, and for one whose clause ends `, not
/// weighed`, fewer than half its video echoes answered, which is a gateway
/// dropping marked echoes rather than a hold, as wifid itself reads it.
fn minute(entry: &Value) -> Option<Minute> {
    let message = entry["MESSAGE"].as_str()?;
    let micros: i64 = entry["__REALTIME_TIMESTAMP"].as_str()?.parse().ok()?;
    let (_, rest) = message.split_once(" on ")?;
    let (bssid, rest) = rest.split_once(": gateway ")?;
    let (_, best) = rest.split_once(" ms at p50, ")?;
    let (best, _) = best.split_once(" ms at p95, ")?;
    let (_, video) = rest.split_once("; video ")?;
    let video = video.split(';').next()?;
    if video.ends_with(", not weighed") {
        return None;
    }
    let (p95, video) = video.split_once(" ms at p95, ")?;
    let (late, video) = video.split_once("% of ")?;
    video.split_once(" echoes answered past 1 s")?;
    Some(Minute {
        at: micros / 1_000_000,
        bssid: bssid.to_owned(),
        best: best.parse().ok()?,
        p95: p95.parse().ok()?,
        late: late.parse().ok()?,
        call: message.ends_with(", during a call"),
    })
}

/// The hour's held minutes, from wifid's journal as journalctl printed it,
/// or none where it couldn't be read. Behind the figure, each access point
/// its minutes held on, most first, with its worst minute, so the owner
/// reads which radio jammed and when. An hour with no minute read is
/// unjudged, saying why: the journal unread, no Wi-Fi minute, wired or
/// asleep, or no video echo answered.
#[must_use]
pub fn video(journal: Option<&str>, zone: &TimeZone) -> Measure {
    let minutes: Vec<Minute> = journal
        .map(|text| lines(text).filter_map(|entry| minute(&entry)).collect())
        .unwrap_or_default();
    let unread = match journal {
        None => Some(": wifid's journal unread"),
        Some(text) if !text.contains(": gateway ") => Some(": no Wi-Fi minute"),
        Some(_) if minutes.is_empty() => Some(": no video echo answered"),
        Some(_) => None,
    };
    if let Some(why) = unread {
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
            let worst = held.iter().max_by(|a, b| a.p95.total_cmp(&b.p95))?;
            Some((held.len(), said(bssid, held.len(), read.len(), worst, zone)))
        })
        .collect();
    behind.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
    let held: usize = behind.iter().map(|(n, _)| n).sum();
    Measure {
        key: KEY.to_owned(),
        class: Class::Apps,
        says: format!("{SAYS}, of {} read", minutes.len()),
        now: count(held),
        target: Some(TARGET),
        owner: OWNER.to_owned(),
        behind: behind.into_iter().map(|(_, line)| line).collect(),
    }
}

/// One access point's held minutes, as the card names them.
fn said(bssid: &str, held: usize, read: usize, worst: &Minute, zone: &TimeZone) -> String {
    let when = Timestamp::from_second(worst.at)
        .map(|at| at.to_zoned(zone.clone()).strftime(" at %H:%M").to_string())
        .unwrap_or_default();
    let call = if worst.call { ", during a call" } else { "" };
    let (p95, late, best) = (worst.p95, worst.late, worst.best);
    format!(
        "on {bssid}, {held} of {read} minutes held, the worst {p95:.1} ms at p95 and \
         {late:.1}% past 1 s{when}{call}, best effort then {best:.1} ms"
    )
}

#[cfg(test)]
mod tests;
