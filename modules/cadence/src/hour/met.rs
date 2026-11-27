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
    let paths = path::path(&lines, &origins(named.as_deref()), now);
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
/// a line naming any other origin picks no ask's words.
fn origins(named: Option<&str>) -> Vec<String> {
    named
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|origin| !origin.is_empty())
        .map(str::to_owned)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_origins_are_those_named_a_comma_apart_and_none_without_a_list() {
        assert_eq!(
            origins(Some("model.example, forge.example,")),
            ["model.example", "forge.example"],
            "the named origins weren't read"
        );
        assert!(
            origins(None).is_empty(),
            "an hour with no list measured a path"
        );
    }
}
