//! What the sessions met on their way out in the hour: the calls pi made
//! to a model and how they ended, the path to each origin path.sh
//! samples, the model relay's own counts, and how wifid's minutes held a
//! call's video. The path is said beside the calls, so a red of the calls
//! names the route as well as the failure.

use std::fs;
use std::path::Path;

use jiff::tz::TimeZone;
use serde_json::Value;

use super::measures::{Measure, agent_folder, calls, path, relay, wifi};
use super::today;
use crate::DAY;

/// The sessions' logs, in pi's own folder.
const SESSIONS: &str = "sessions";

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
    let paths = path::path(&lines, &sampled(&lines), now);
    for m in &mut calls {
        m.behind.extend(path::said(&paths));
    }
    calls.extend(paths);
    calls.extend(relay::hour(now));
    let wifid = wifi::read(now);
    calls.push(wifi::video(wifid.as_deref(), &TimeZone::system()));
    calls
}

/// Each origin path.sh sampled in the day read, so an origin it stopped
/// sampling reads unjudged for a day rather than gone.
fn sampled(lines: &str) -> Vec<String> {
    let mut origins: Vec<String> = lines
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .filter_map(|v| v.get("origin").and_then(Value::as_str).map(str::to_owned))
        .collect();
    origins.sort_unstable();
    origins.dedup();
    origins
}
