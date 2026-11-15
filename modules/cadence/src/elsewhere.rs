//! Other machines' updates, read from the node that keeps each realm
//! (`mycelium machines`), so cadence says one failing elsewhere as it
//! says its own. On 15 November the work Mac's updates failed on one test
//! for hours and nothing said so, since cadence watched only this machine.

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
    let Ok(said) = serde_json::from_str::<serde_json::Value>(answer) else {
        return Vec::new();
    };
    let machines = said["Ok"]["Machines"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    machines
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
