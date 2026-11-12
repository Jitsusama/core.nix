//! Readings of Joel's network, from the day files its two owners write:
//! the eero daemon's probes of the router, the internet and name lookups
//! while the laptop is at home and idle (PN2VZC), and the shaper's delay
//! added under load and its probes lost (B8RBNW). Both are his side, his
//! laptop on his network. Each is read as its owner wrote it, with its
//! owner's bars of 2026-11-12, and stays unread until its file exists.

use std::fs;
use std::path::Path;

use serde_json::Value;

use super::{Bar, Reading, shown};

/// The two day files, each as written, for the day being read.
#[derive(Debug, Default, Clone)]
pub struct Sources {
    pub eero: Option<Value>,
    pub shaper: Option<Value>,
}

/// Reads `eero/day/<date>.json` and `shaper/day/<date>.json` from the
/// state folder, `date` being the local day.
#[must_use]
pub fn gather(state_home: &Path, date: &str) -> Sources {
    let read = |owner: &str| {
        fs::read_to_string(state_home.join(format!("{owner}/day/{date}.json")))
            .ok()
            .and_then(|json| serde_json::from_str(&json).ok())
    };
    Sources {
        eero: read("eero"),
        shaper: read("shaper"),
    }
}

/// The eero's four: the internet and the router answering at the 95th
/// percentile, probes finding the internet out, and minutes out. The
/// probes are timed idle, every five minutes while awake, so they show
/// the line, not what load does to it; the shaper reads that. A day spent
/// away reads unread, not as a quiet day at home.
#[must_use]
pub fn eero(s: &Sources) -> Vec<Reading> {
    let day = s.eero.as_ref();
    let probes = day.and_then(|d| d["probes"].as_f64()).unwrap_or(0.0);
    let away = day.and_then(|d| d["away_probes"].as_f64()).unwrap_or(0.0);
    let at_home = |key: &str| day.and_then(|d| d[key].as_f64()).filter(|_| probes > 0.0);
    let looked = day.map(|_| {
        if probes > 0.0 {
            format!(
                "of {} probes from home, timed while idle, not under load",
                shown(probes, "")
            )
        } else if away > 0.0 {
            "away from home all day".to_owned()
        } else {
            "no probes yet today".to_owned()
        }
    });
    let from = "the eero's day file";
    vec![
        Reading::new(
            "the internet answering {} at the 95th percentile today",
            at_home("internet_p95_ms"),
            " ms",
            "PN2VZC",
        )
        .bar(Bar::AtMost(100.0))
        .detail(looked.clone())
        .from(from)
        .his(),
        Reading::new(
            "the router answering {} at the 95th percentile today",
            at_home("gateway_p95_ms"),
            " ms",
            "PN2VZC",
        )
        .bar(Bar::AtMost(40.0))
        .detail(
            at_home("dns_p95_ms")
                .map(|dns| format!("names looked up in {} at the 95th", shown(dns, " ms"))),
        )
        .from(from)
        .his(),
        Reading::new(
            "{} probes finding the internet out today",
            at_home("unreachable"),
            "",
            "PN2VZC",
        )
        .bar(Bar::AtMost(1.0))
        .detail(looked)
        .from(from)
        .his(),
        Reading::new(
            "time without the internet {} today",
            at_home("outage_minutes"),
            " min",
            "PN2VZC",
        )
        .bar(Bar::AtMost(5.0))
        .from(from)
        .his(),
    ]
}

/// The shaper's three: the delay each way added under load at the 90th
/// percentile, a minute counting as loaded when it carried at least half
/// that way's full rate, and the share of its probes lost.
#[must_use]
pub fn shaper(s: &Sources) -> Vec<Reading> {
    let day = s.shaper.as_ref();
    let recorded = day.and_then(|d| d["minutes_recorded"].as_f64());
    let way = |name: &'static str, says: &'static str| {
        let way = day.map(|d| &d[name]);
        let loaded = way.and_then(|w| w["loaded_minutes"].as_f64());
        Reading::new(
            says,
            way.and_then(|w| w["added_delay_ms"]["p90"].as_f64()),
            " ms",
            "B8RBNW",
        )
        .bar(Bar::AtMost(30.0))
        .detail(loaded.zip(recorded).map(|(loaded, recorded)| {
            format!(
                "{} of {} minutes under load",
                shown(loaded, ""),
                shown(recorded, "")
            )
        }))
        .from("the shaper's day file")
        .his()
    };
    vec![
        way(
            "up",
            "the delay uploading under load added at the 90th percentile today {}",
        ),
        way(
            "down",
            "the delay downloading under load added at the 90th percentile today {}",
        ),
        Reading::new(
            "the shaper's probes lost today {}",
            day.and_then(|d| d["probe_loss"].as_f64())
                .map(|share| share * 100.0),
            "%",
            "B8RBNW",
        )
        .bar(Bar::AtMost(1.0))
        .from("the shaper's day file")
        .his(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn said(read: &[Reading]) -> Vec<String> {
        read.iter().map(Reading::words).collect()
    }

    #[test]
    fn the_eeros_day_is_read_as_his_network_at_home() {
        let s = Sources {
            eero: serde_json::from_str(
                r#"{"date": "2026-11-12", "probes": 212, "away_probes": 0,
                "gateway_p95_ms": 35.7, "internet_p95_ms": 101.7, "dns_p95_ms": 143.6,
                "unreachable": 0, "outage_minutes": 0.0}"#,
            )
            .ok(),
            shaper: None,
        };
        let read = eero(&s);
        assert_eq!(
            said(&read),
            [
                "the internet answering 102 ms at the 95th percentile today, at most 100 ms",
                "the router answering 36 ms at the 95th percentile today, at most 40 ms",
                "0 probes finding the internet out today, at most 1",
                "time without the internet 0 min today, at most 5 min",
            ]
        );
        assert_eq!(
            read.iter().map(Reading::out).collect::<Vec<_>>(),
            [true, false, false, false]
        );
        assert!(read.iter().all(|r| r.his), "his laptop on his network");
        assert_eq!(
            read[0].detail.as_deref(),
            Some("of 212 probes from home, timed while idle, not under load")
        );
        assert_eq!(
            read[1].detail.as_deref(),
            Some("names looked up in 144 ms at the 95th")
        );
    }

    #[test]
    fn a_day_away_from_home_reads_unread_not_quiet() {
        let s = Sources {
            eero: serde_json::from_str(
                r#"{"probes": 0, "away_probes": 40, "gateway_p95_ms": null,
                "internet_p95_ms": null, "unreachable": 0, "outage_minutes": 0.0}"#,
            )
            .ok(),
            shaper: None,
        };
        let read = eero(&s);
        assert!(
            read.iter().all(|r| r.figure().is_none()),
            "no look from home is no reading, not a good one"
        );
        assert_eq!(read[0].detail.as_deref(), Some("away from home all day"));
        assert!(
            eero(&Sources::default())
                .iter()
                .all(|r| r.figure().is_none()),
            "no file is no reading"
        );
    }

    #[test]
    fn the_shapers_day_is_read_each_way_under_load() {
        let s = Sources {
            eero: None,
            shaper: serde_json::from_str(
                r#"{"date": "2026-11-12", "minutes_recorded": 972,
                "up": {"full_kbit": 11600, "loaded_minutes": 911,
                       "added_delay_ms": {"p50": 6, "p90": 15}},
                "down": {"full_kbit": 300000, "loaded_minutes": 40,
                         "added_delay_ms": {"p50": 9, "p90": 34}},
                "probe_loss": 0.0003}"#,
            )
            .ok(),
        };
        let read = shaper(&s);
        assert_eq!(
            said(&read),
            [
                "the delay uploading under load added at the 90th percentile today 15 ms, \
                 at most 30 ms",
                "the delay downloading under load added at the 90th percentile today 34 ms, \
                 at most 30 ms",
                "the shaper's probes lost today 0.0%, at most 1%",
            ]
        );
        assert_eq!(
            read.iter().map(Reading::out).collect::<Vec<_>>(),
            [false, true, false]
        );
        assert_eq!(
            read[0].detail.as_deref(),
            Some("911 of 972 minutes under load")
        );
        let unloaded = Sources {
            eero: None,
            shaper: serde_json::from_str(
                r#"{"minutes_recorded": 10, "up": {"loaded_minutes": 0,
                "added_delay_ms": null}, "down": {}, "probe_loss": null}"#,
            )
            .ok(),
        };
        assert!(
            shaper(&unloaded).iter().all(|r| r.figure().is_none()),
            "a day with no load has no delay under load to read"
        );
    }
}
