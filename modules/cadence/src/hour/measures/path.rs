//! The path to each origin the sessions reach, from the samples
//! cadence-path (path.sh) writes every ten minutes: how long TCP and TLS
//! took to set up, the route's device, or the sample's failure. On 26
//! November TLS to the model origin and to the code forge took 6 to 10 s
//! through a starved WARP lane while warp-cli said Connected and the
//! relay's own measures read nought (WCJQSG, 720d7b2f); unshaped it takes
//! about 0.25 s. Each origin's median TLS setup of the hour is held to a
//! ceiling, a failed sample counting as the probe's whole wait, and what
//! is behind it names the route, so a red says which path.

use serde_json::Value;

use super::{Class, Measure};

/// What each origin's measure is known by opens so, which files each
/// apart with the path's keeper, whatever leads the hour (owing.rs).
pub const PREFIX: &str = "path:tls:";

/// Who keeps the path the model calls take, and so moves each.
const OWNER: &str = "OXUW1Y";

/// The most an hour's median TLS setup may take, in seconds: four times
/// what the path took unshaped on 26 November, and a sixth of what it took
/// starved.
pub const TLS_AT_MOST: f64 = 1.0;

/// How long path.sh waits for an origin before the sample fails, which a
/// failed sample counts as.
pub const PROBE_WAIT: f64 = 20.0;

/// The fewest samples an hour judges an origin by: the probe takes six.
const SAMPLES_AT_LEAST: usize = 3;

/// The folder under the state home path.sh writes its days in.
pub const FOLDER: &str = "cadence/path";

/// One sample of one origin.
struct Sample {
    device: String,
    /// The TLS setup in seconds, or none for a failed sample.
    tls: Option<f64>,
    error: Option<String>,
}

/// A measure for each of `origins` over the hour to `now`, from path.sh's
/// lines, each unjudged where fewer than three samples of it were read.
#[must_use]
pub fn path(lines: &str, origins: &[String], now: i64) -> Vec<Measure> {
    let hour = now - 3600..now;
    origins
        .iter()
        .map(|origin| {
            let samples: Vec<Sample> = lines
                .lines()
                .filter_map(|line| serde_json::from_str::<Value>(line).ok())
                .filter(|v| v.get("origin").and_then(Value::as_str) == Some(origin))
                .filter(|v| {
                    v.get("at")
                        .and_then(Value::as_i64)
                        .is_some_and(|at| hour.contains(&at))
                })
                .map(|v| Sample {
                    device: v
                        .get("device")
                        .and_then(Value::as_str)
                        .unwrap_or("an unread device")
                        .to_owned(),
                    tls: v.get("tls_s").and_then(Value::as_f64),
                    error: v.get("error").and_then(Value::as_str).map(str::to_owned),
                })
                .collect();
            measured(origin, &samples)
        })
        .collect()
}

/// The measure of one origin from its hour's samples.
fn measured(origin: &str, samples: &[Sample]) -> Measure {
    let key = format!("{PREFIX}{origin}");
    if samples.len() < SAMPLES_AT_LEAST {
        return Measure {
            key,
            class: Class::Functional,
            says: format!(
                "the path to {origin}: {} of its samples this hour, too few to judge",
                samples.len()
            ),
            now: 0.0,
            target: None,
            owner: OWNER.to_owned(),
            behind: Vec::new(),
        };
    }
    let mut times: Vec<f64> = samples
        .iter()
        .map(|s| s.tls.unwrap_or(PROBE_WAIT))
        .collect();
    times.sort_by(f64::total_cmp);
    let median = times[times.len() / 2];
    let worst = times[times.len() - 1];
    let mut devices: Vec<&str> = samples.iter().map(|s| s.device.as_str()).collect();
    devices.sort_unstable();
    devices.dedup();
    let mut behind = vec![
        format!("via {}", devices.join(" and ")),
        format!("worst {worst:.2} s"),
    ];
    let failed: Vec<&Sample> = samples.iter().filter(|s| s.tls.is_none()).collect();
    if let Some(first) = failed.first() {
        let error: String = first
            .error
            .as_deref()
            .unwrap_or("an unsaid error")
            .chars()
            .take(120)
            .collect();
        behind.push(format!(
            "{} of {} failed, as {error}",
            failed.len(),
            samples.len()
        ));
    }
    Measure {
        key,
        class: Class::Functional,
        says: format!(
            "median seconds to set up TLS to {origin} this hour, of {} samples",
            samples.len()
        ),
        now: (median * 100.0).round() / 100.0,
        target: Some(TLS_AT_MOST),
        owner: OWNER.to_owned(),
        behind,
    }
}

/// The path's figures said in a line each, as behind a measure of the
/// calls that took it: `TLS to github.com via CloudflareWARP: median
/// 0.25 s`, so a red of the calls names the path too.
#[must_use]
pub fn said(paths: &[Measure]) -> Vec<String> {
    paths
        .iter()
        .filter(|m| m.target.is_some())
        .map(|m| {
            let origin = m.key.trim_start_matches(PREFIX);
            let via = m.behind.first().map_or("", String::as_str);
            format!("TLS to {origin} {via}: median {:.2} s", m.now)
        })
        .collect()
}

#[cfg(test)]
mod tests;
