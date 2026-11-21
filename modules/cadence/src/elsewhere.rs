//! Other machines' updates, read from the node that keeps each realm
//! (`mycelium machines`), so cadence says one failing elsewhere as it
//! says its own. On 15 November the work Mac's updates failed on one test
//! for hours and nothing said so, since cadence watched only this machine.
//!
//! Each machine's node also says its room: the pressure on it, what it's
//! over, and what work it takes. At 14:01 the same day Joel asked why the
//! other machines weren't used more, with this laptop over its room on
//! processor and memory under 115 sessions while the Mac had none of
//! either and 46 GB free, so the hour names a machine crowded so.

/// Another machine of the realm whose newest update failed, as its node
/// says in `mycelium machines`.
#[derive(Debug, Clone, PartialEq)]
pub struct Failing {
    pub machine: String,
    /// When its newest update ended, in seconds since the epoch.
    pub at: i64,
    /// The first error its update gave, else the reason its node says.
    pub error: String,
    /// Where a test it built panicked, when one did.
    pub panicked: Option<String>,
}

/// The machines in a `mycelium machines` answer, but `this`, whose newest
/// update failed; none when the answer doesn't parse.
#[must_use]
pub fn failing_elsewhere(answer: &str, this: &str) -> Vec<Failing> {
    machines(answer)
        .unwrap_or_default()
        .iter()
        .filter(|m| m["name"].as_str().is_some_and(|name| name != this))
        .filter_map(|m| {
            let updated = &m["updated"];
            if updated["step"].as_str() != Some("failed") {
                return None;
            }
            let at = updated["at"].as_str()?.parse::<jiff::Timestamp>().ok()?;
            let lines: Vec<&str> = updated["lines"]
                .as_array()
                .map(|lines| lines.iter().filter_map(serde_json::Value::as_str).collect())
                .unwrap_or_default();
            let error = lines
                .iter()
                .map(|line| line.trim())
                .find(|line| line.starts_with("error:"))
                .or_else(|| updated["why"].as_str())?;
            let panicked = lines
                .iter()
                .find_map(|line| line.find("panicked at ").map(|from| &line[from..]))
                .map(|p| p.trim_end_matches(':').to_owned());
            Some(Failing {
                machine: m["name"].as_str()?.to_owned(),
                at: at.as_second(),
                error: error.to_owned(),
                panicked,
            })
        })
        .collect()
}

/// A machine over its room while another of its realm has room to take
/// work, as their nodes say in `mycelium machines`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Crowded {
    pub machine: String,
    /// What it's over, as its node says: `cpu`, `memory`, `io`.
    pub over: Vec<String>,
    /// The machines of its realm over nothing that take builds or VM tests.
    pub roomy: Vec<String>,
}

/// The machines in a `mycelium machines` answer over their room while
/// another has room; `None` when no document in it names the machines,
/// so an answer that went unread isn't taken for an hour none was crowded.
#[must_use]
pub fn crowded(answer: &str) -> Option<Vec<Crowded>> {
    let machines = machines(answer)?;
    let words = |list: &serde_json::Value| -> Vec<String> {
        list.as_array()
            .map(|list| {
                list.iter()
                    .filter_map(serde_json::Value::as_str)
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default()
    };
    let takes = |room: &serde_json::Value| {
        ["builds", "vm_tests"]
            .iter()
            .any(|work| room["takes"][work].as_u64().is_some_and(|n| n > 0))
    };
    let roomy: Vec<String> = machines
        .iter()
        .filter(|m| m["room"]["over"].is_array() && words(&m["room"]["over"]).is_empty())
        .filter(|m| takes(&m["room"]))
        .filter_map(|m| m["name"].as_str().map(str::to_owned))
        .collect();
    if roomy.is_empty() {
        return Some(Vec::new());
    }
    Some(
        machines
            .iter()
            .filter_map(|m| {
                let over = words(&m["room"]["over"]);
                (!over.is_empty()).then(|| Crowded {
                    machine: m["name"].as_str().unwrap_or_default().to_owned(),
                    over,
                    roomy: roomy.clone(),
                })
            })
            .collect(),
    )
}

/// The machines a `mycelium machines` answer names; `None` when no document
/// in it does. The answer is a document a line, the builders' following the
/// machines' once a machine names builders (16 November), so each document
/// is read until one holds the machines.
pub(crate) fn machines(answer: &str) -> Option<Vec<serde_json::Value>> {
    serde_json::Deserializer::from_str(answer)
        .into_iter::<serde_json::Value>()
        .map_while(Result::ok)
        .find_map(|said| said["Ok"]["Machines"].as_array().cloned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn machine(name: &str, over: &str, builds: u32) -> String {
        format!(
            r#"{{"name":"{name}","room":{{"over":[{over}],
               "takes":{{"builds":{builds},"vm_tests":0}}}}}}"#
        )
    }

    #[test]
    fn a_machine_over_its_room_while_another_has_room_is_crowded() {
        let answer = format!(
            r#"{{"Ok":{{"Machines":[{},{},{},{}]}}}}"#,
            machine("laptop", r#""memory","cpu""#, 0),
            machine("mac", "", 10),
            machine("takes-nothing", "", 0),
            machine("also-over", r#""io""#, 4),
        );
        assert_eq!(
            crowded(&answer).unwrap_or_default(),
            [
                Crowded {
                    machine: "laptop".into(),
                    over: vec!["memory".into(), "cpu".into()],
                    roomy: vec!["mac".into()],
                },
                Crowded {
                    machine: "also-over".into(),
                    over: vec!["io".into()],
                    roomy: vec!["mac".into()],
                },
            ],
            "a machine over its room beside one with room wasn't named, or one that takes \
             no work was counted as room"
        );
    }

    #[test]
    fn a_machine_over_its_room_with_nowhere_else_to_go_is_not_crowded() {
        let answer = format!(
            r#"{{"Ok":{{"Machines":[{},{}]}}}}"#,
            machine("laptop", r#""cpu""#, 0),
            machine("busy", r#""memory""#, 10),
        );
        assert_eq!(
            crowded(&answer),
            Some(Vec::new()),
            "a machine was named crowded with no other machine having room"
        );
        assert_eq!(
            crowded("not json"),
            None,
            "an answer that doesn't parse was read as one naming machines"
        );
    }

    #[test]
    fn another_machines_failed_update_is_read_from_its_node() {
        let answer = r#"{"Ok":{"Machines":[
            {"name":"here","updated":{"at":"2026-11-15T04:17:03Z","step":"failed",
             "why":"this machine's own, which the follower says"}},
            {"name":"fine","updated":{"at":"2026-11-15T04:00:00Z","step":"done"}},
            {"name":"unsaid"},
            {"name":"mac","updated":{"at":"2026-11-15T04:14:23Z","step":"failed",
             "why":"building the system failed (exit status: 1)",
             "lines":["error: Cannot build '/nix/store/ys2n-mycelium-0.1.0.drv'.",
               "       Reason: builder failed with exit code 101.",
               "       > thread 'a_holder' (11811942) panicked at tests/held.rs:118:5:"]}},
            {"name":"fetching","updated":{"at":"2026-11-15T03:00:00Z","step":"failed",
             "why":"fetching the input failed"}}
        ]}}"#;
        assert_eq!(
            failing_elsewhere(answer, "here"),
            [
                Failing {
                    machine: "mac".into(),
                    at: 1_794_716_063,
                    error: "error: Cannot build '/nix/store/ys2n-mycelium-0.1.0.drv'.".into(),
                    panicked: Some("panicked at tests/held.rs:118:5".into()),
                },
                Failing {
                    machine: "fetching".into(),
                    at: 1_794_711_600,
                    error: "fetching the input failed".into(),
                    panicked: None,
                },
            ]
        );
        assert_eq!(failing_elsewhere("not json", "here"), []);
        assert_eq!(failing_elsewhere(r#"{"Err":"no node"}"#, "here"), []);
    }

    #[test]
    fn the_machines_are_read_past_the_builders_line_after_them() {
        let machines = format!(
            r#"{{"Ok":{{"Machines":[{},{}]}}}}"#,
            machine("laptop", r#""cpu""#, 0),
            machine("mac", "", 10),
        );
        let builders = r#"{"Ok":{"Builders":[{"name":"lent"}]}}"#;
        let answer = format!("{machines}\n{builders}\n");
        assert_eq!(
            crowded(&answer).map(|c| c.len()),
            Some(1),
            "the builders' line after the machines' made the whole answer unreadable"
        );
        assert_eq!(
            crowded(&format!("{builders}\n{machines}")).map(|c| c.len()),
            Some(1),
            "a builders' line before the machines' hid them"
        );
        let failed = answer.replace(
            r#""name":"mac","#,
            r#""name":"mac","updated":{"at":"2026-11-15T04:14:23Z","step":"failed","why":"x"},"#,
        );
        assert_eq!(
            failing_elsewhere(&failed, "laptop").len(),
            1,
            "the builders' line hid another machine's failed update"
        );
    }
}
