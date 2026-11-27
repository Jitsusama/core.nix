//! What the sessions met on their way out in the hour: the calls pi made
//! to a model and how they ended, the path to each origin the deployer
//! names for path.sh to sample, the model relay's own counts, and how wifid's minutes held a
//! call's video. The path is said beside the calls, so a red of the calls
//! names the route as well as the failure.

use std::fs;
use std::path::Path;

use jiff::tz::TimeZone;

use super::measures::cited::agent_folder;
use super::measures::way_out::{calls, path, relay};
use super::measures::{Measure, wifi};
use super::today;
use crate::DAY;

/// The sessions' logs, in pi's own folder.
const SESSIONS: &str = "sessions";

/// The origins the deployer named, as home.nix gives them to the hour,
/// a comma apart: the same list path.sh samples.
const ORIGINS: &str = "CADENCE_ORIGINS";

/// The hour's measures of the calls, the paths, the relay and the wifi.
pub fn met(state_home: &Path, now: i64) -> Vec<Measure> {
    let sessions = agent_folder().map(|agent| agent.join(SESSIONS));
    let mut calls = sessions.map_or_else(|| calls::calls(None, now), |s| calls::hour(&s, now));
    let folder = state_home.join(path::FOLDER);
    let day = |at| fs::read_to_string(folder.join(format!("{}.jsonl", today(at))));
    let lines = [day(now - DAY), day(now)]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join("\n");
    let named = std::env::var(ORIGINS).ok();
    let paths = path::path(&lines, &origins(named.as_deref(), &lines), now);
    for m in &mut calls {
        m.behind.extend(path::said(&paths));
    }
    calls.extend(paths);
    calls.extend(relay::hour(now));
    let wifid = wifi::read(now);
    calls.push(wifi::video(wifid.as_deref(), &TimeZone::system()));
    calls
}

/// Each origin the deployer named, rather than each the samples hold, so a
/// sampler that stopped leaves its paths unjudged and their asks open, and
/// a line naming any other origin picks no ask's words. The unit always
/// names its list, empty where none is sampled; an hour run by hand, with
/// no list at all, measures each origin its samples hold instead, so it
/// doesn't settle every path ask as no longer named.
fn origins(named: Option<&str>, lines: &str) -> Vec<String> {
    let mut origins: Vec<String> = match named {
        Some(named) => named
            .split(',')
            .map(str::trim)
            .filter(|origin| !origin.is_empty())
            .map(str::to_owned)
            .collect(),
        None => lines
            .lines()
            .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
            .filter_map(|v| v.get("origin").and_then(|o| o.as_str()).map(str::to_owned))
            .collect(),
    };
    // An origin named twice is still one path, measured and said once.
    origins.sort();
    origins.dedup();
    origins
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_origins_are_those_named_a_comma_apart_or_those_sampled_without_a_list() {
        let sampled = ["model.example", "tunnel.example", "model.example"]
            .map(|origin| serde_json::json!({"origin": origin, "at": 1}).to_string())
            .join("\n");
        assert_eq!(
            origins(Some("model.example, forge.example,model.example"), &sampled),
            ["forge.example", "model.example"],
            "the named origins weren't read once each, whatever was sampled"
        );
        assert!(
            origins(Some(""), &sampled).is_empty(),
            "an empty list measured a path"
        );
        assert_eq!(
            origins(None, &sampled),
            ["model.example", "tunnel.example"],
            "an hour run with no list didn't measure each sampled origin once"
        );
    }
}
