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

/// How full the upload is, as the shaper hears it, for the Work Network
/// to say whether a full line is why WARP is degraded: `{ full_pct,
/// shares }`, the share of the limit the shaper holds the upload to that
/// it carries, to the nearest 5% so a figure wavering doesn't tell every
/// watch, and each class's share of what it carries, to the nearest 5%,
/// where the shaper serves its `up_tins_kbit` (B8RBNW, 2026-11-15): bulk is
/// agents' traffic, best effort Joel's own turns and browser, video his
/// calls and apps. Null with no shaper or nothing heard.
pub fn upload(status: &Path) -> Value {
    let Ok(text) = std::fs::read_to_string(status) else {
        return Value::Null;
    };
    let links: Vec<Value> = serde_json::from_str(&text).unwrap_or_default();
    let Some(heard) = links
        .iter()
        .map(|link| &link["heard"])
        .find(|heard| !heard.is_null())
    else {
        return Value::Null;
    };
    let (Some(carried), Some(limit)) = (heard["up_kbit"].as_f64(), heard["up_limit"].as_f64())
    else {
        return Value::Null;
    };
    let tins = &heard["up_tins_kbit"];
    let names = ["bulk", "best", "video", "voice"];
    let each: Option<Vec<f64>> = names.iter().map(|name| tins[name].as_f64()).collect();
    let shares = each
        .filter(|each| each.iter().sum::<f64>() > 0.0)
        .map(|each| {
            let all: f64 = each.iter().sum();
            names
                .iter()
                .zip(each)
                .map(|(name, kbit)| ((*name).to_owned(), json!(fives(kbit / all))))
                .collect::<serde_json::Map<_, _>>()
        });
    json!({
        "full_pct": (limit > 0.0).then(|| fives(carried / limit)),
        "shares": shares,
    })
}

/// A share as a percentage to the nearest 5.
fn fives(share: f64) -> u64 {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a share is floored at 0 and a few hundred at most"
    )]
    let pct = ((share * 20.0).round().max(0.0) as u64) * 5;
    pct
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_upload_says_how_full_it_is_and_whose_where_the_shaper_says() {
        let status = Path::new(CONTRACT).join("shaper-status.json");
        assert_eq!(
            upload(&status),
            json!({ "full_pct": 90, "shares": null }),
            "6690 of 7416 kbit/s is 90% full, and the contract has no classes"
        );
        let status = written(
            "tins",
            r#"[{"heard": {"up_kbit": 4000, "up_limit": 4069,
               "up_tins_kbit": {"bulk": 3000, "best": 900, "video": 80, "voice": 0}}}]"#,
        );
        assert_eq!(
            upload(&status),
            json!({ "full_pct": 100, "shares": {"bulk": 75, "best": 25, "video": 0, "voice": 0} }),
            "each class's share went unsaid"
        );
    }

    #[test]
    fn no_shaper_or_nothing_heard_says_nothing_of_the_upload() {
        let none = Path::new(CONTRACT).join("no-such-status.json");
        assert_eq!(upload(&none), Value::Null, "no file said something");
        let idle = written("idle-up", r#"[{"name": "wlan0", "heard": null}]"#);
        assert_eq!(
            upload(&idle),
            Value::Null,
            "a link that heard nothing said something"
        );
    }
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
