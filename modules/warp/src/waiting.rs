//! What waits on the way through the VPN, as the network's shaper hears it
//! (modules/shaper), for the Work Network's Waiting. While it runs, the
//! shaper writes how each link stands to its status.json, and the link it
//! holds carries what it heard: `vpn_ms`, the round trip of a DNS question
//! asked through the VPN every 200 ms, null where one went unanswered for
//! 2 s, and `vpn_base_ms`, the least of those over the last five minutes,
//! the way at rest. Past that is time spent waiting on the way, the link's
//! own queue and the VPN's together. B8RBNW keeps the two names, and says
//! before changing either (2026-11-12). The shaper is an experiment, on and
//! off, and its file is there only while it runs, so a missing one is the
//! shaper not running, never nothing waiting.

use std::path::Path;

use serde_json::{Value, json};

/// What the shaper heard, as the link record's `waiting`: `{ running:
/// false }` with no file, else `{ running: true, through_ms, at_rest_ms }`,
/// each figure null where it wasn't heard, from the first link that heard
/// anything, the one the shaper holds.
pub fn heard(status: &Path) -> Value {
    let Ok(text) = std::fs::read_to_string(status) else {
        return json!({ "running": false });
    };
    let links: Vec<Value> = serde_json::from_str(&text).unwrap_or_default();
    let heard = links
        .iter()
        .map(|link| &link["heard"])
        .find(|heard| !heard.is_null());
    let figure = |name: &str| heard.and_then(|heard| heard[name].as_f64());
    json!({
        "running": true,
        "through_ms": figure("vpn_ms"),
        "at_rest_ms": figure("vpn_base_ms"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    const CONTRACT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/contract");

    fn written(name: &str, text: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("warp-{name}-{}.json", std::process::id()));
        std::fs::write(&path, text).expect("a file is written");
        path
    }

    #[test]
    fn no_file_is_the_shaper_not_running() {
        let none = Path::new(CONTRACT).join("no-such-status.json");
        assert_eq!(heard(&none), json!({ "running": false }));
    }

    #[test]
    fn the_link_held_says_the_vpns_round_trip_and_its_least() {
        let status = Path::new(CONTRACT).join("shaper-status.json");
        assert_eq!(
            heard(&status),
            json!({
                "running": true,
                "through_ms": 103.121_740_000_000_02,
                "at_rest_ms": 27.667_19,
            }),
            "the first link heard nothing, so the second, held, says it"
        );
    }

    #[test]
    fn a_probe_unanswered_leaves_its_round_trip_unknown() {
        let status = written(
            "lost",
            r#"[{"name": "wlan0", "heard": {"vpn_ms": null, "vpn_base_ms": 27.5}}]"#,
        );
        assert_eq!(
            heard(&status),
            json!({ "running": true, "through_ms": null, "at_rest_ms": 27.5 })
        );
    }

    #[test]
    fn a_shaper_holding_no_link_heard_nothing() {
        let status = written("idle", r#"[{"name": "wlan0", "heard": null}]"#);
        assert_eq!(
            heard(&status),
            json!({ "running": true, "through_ms": null, "at_rest_ms": null })
        );
    }
}
