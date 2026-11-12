//! Where the net stands against main: the head its round tests, or between
//! rounds the head it last saw, and when that lags long enough to be said.

use std::fs;
use std::path::Path;

use crate::watch::Breach;

/// The net's round this many hours behind main's head.
pub const NET_BEHIND_H: f64 = 3.0;

/// The net's round: the head it tests, when that head merged, and how many
/// merges main has past it.
#[derive(Debug, Clone, PartialEq)]
pub struct Round {
    pub head: String,
    pub merged: i64,
    pub behind: usize,
    /// Whether a round tests `head` now, or the net, between rounds, last
    /// saw it: on 10 November the net tested nothing past 19:56's head for
    /// hours while a search ran, and the card said it was caught up.
    pub testing: bool,
}

/// The net's lag at `now`, once its head is past `NET_BEHIND_H` old, and
/// again each time its age doubles, so a long search is known once said.
#[must_use]
pub fn lag(round: &Round, now: i64) -> Option<Breach> {
    #[expect(clippy::cast_precision_loss, reason = "seconds within a lifetime")]
    let hours = (now - round.merged) as f64 / 3600.0;
    if hours <= NET_BEHIND_H {
        return None;
    }
    let mut tier = NET_BEHIND_H;
    while tier * 2.0 <= hours {
        tier *= 2.0;
    }
    let short: String = round.head.chars().take(9).collect();
    let what = if round.testing {
        format!("the net's round at {short}")
    } else {
        format!("the net, between rounds, last saw {short}, which")
    };
    Some(Breach {
        key: format!("net:{short}:{tier}"),
        text: format!(
            "{what} is {hours:.1} h old, {} merges behind main",
            round.behind
        ),
    })
}

/// The net aborting itself for hanging: when, from its finding's `at`, and
/// how long it had said nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hung {
    pub at: i64,
    pub quiet_minutes: i64,
}

/// The newest hang the net kept in its `findings` folder since `since`. Its
/// findings are named for when they were kept and what they're about, so
/// only the net's own since then are read, never the thousands of others.
#[must_use]
pub fn hung(findings: &Path, since: i64) -> Option<Hung> {
    fs::read_dir(findings)
        .ok()?
        .filter_map(Result::ok)
        .filter(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            name.ends_with("-net.json")
                && name
                    .split('-')
                    .next()
                    .and_then(|at| at.parse::<i64>().ok())
                    .is_some_and(|at| at >= since)
        })
        .filter_map(|entry| fs::read_to_string(entry.path()).ok())
        .filter_map(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
        .filter(|finding| finding["kind"] == "stalled")
        .filter_map(|finding| {
            Some(Hung {
                at: finding["at"].as_i64()?,
                quiet_minutes: finding["quiet_minutes"].as_i64().unwrap_or_default(),
            })
        })
        .max_by_key(|hung| hung.at)
}

/// What the watcher is told of a hang, once: it is the net's own fault, and
/// the stacks name the lock.
#[must_use]
pub fn hang(hung: &Hung) -> Breach {
    Breach {
        key: format!("hung:{}", hung.at),
        text: format!(
            "the net hung, saying nothing for {} min with every thread on a lock, and aborted \
             to start again; its threads' stacks are in `coredumpctl info merge-net`",
            hung.quiet_minutes
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_nets_newest_hang_since_is_read_from_its_findings_alone() {
        let findings =
            std::env::temp_dir().join(format!("cadence-net-hung-{}", std::process::id()));
        fs::create_dir_all(&findings).expect("the folder wasn't made");
        let keep = |name: &str, kind: &str, at: i64| {
            let finding = serde_json::json!({"kind": kind, "at": at, "quiet_minutes": 21});
            fs::write(findings.join(name), finding.to_string()).expect("a finding wasn't kept");
        };
        keep("90-aaaa-net.json", "stalled", 90);
        keep("200-aaaa-net.json", "stalled", 200);
        keep("300-aaaa-vm-a.json", "stalled", 300);
        keep("400-aaaa-net.json", "reverted", 400);
        let read = hung(&findings, 100);
        let old = hung(&findings, 250);
        fs::remove_dir_all(&findings).expect("the folder wasn't cleared");
        assert_eq!(
            read,
            Some(Hung {
                at: 200,
                quiet_minutes: 21
            }),
            "the net's newest hang since 100 wasn't read alone"
        );
        assert_eq!(old, None, "a hang before since was read");
        assert_eq!(
            hang(&read.unwrap()).key,
            "hung:200",
            "a hang isn't said once"
        );
    }

    #[test]
    fn the_nets_lag_is_said_at_three_hours_then_at_each_doubling() {
        let round = |hours: i64, testing| Round {
            head: "cb5b9b12dce0".into(),
            merged: 100_000 - hours * 3600,
            behind: 5,
            testing,
        };
        assert_eq!(
            lag(&round(2, true), 100_000),
            None,
            "a lag under three hours was said"
        );
        assert_eq!(
            lag(&round(4, true), 100_000).unwrap().key,
            "net:cb5b9b12d:3"
        );
        let said = lag(&round(7, false), 100_000).unwrap();
        assert_eq!(said.key, "net:cb5b9b12d:6");
        assert!(
            said.text
                .starts_with("the net, between rounds, last saw cb5b9b12d"),
            "a net testing nothing was said to be in a round: {}",
            said.text
        );
    }
}
