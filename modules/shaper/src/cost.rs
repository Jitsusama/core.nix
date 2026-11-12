//! What a VPN costs the connection it rides past what its own CAKE counts.
//!
//! A VPN's CAKE counts each packet it sends with 80 bytes more, for the
//! header the VPN wraps it in, and holds the VPN to a share of the
//! connection that leaves under a twentieth for anything else. On 10
//! November WARP's own packets on the Wi-Fi ran about a tenth over what its
//! CAKE had sent beside uploads alone, and about a seventh over while Joel
//! downloaded through it: MASQUE wraps a packet in more than 80 bytes, and
//! WARP acknowledges what it brings in with packets of its own that never
//! pass its CAKE. WARP alone then passed the connection's rate, its queue
//! formed in the connection's CAKE, where WARP is one flow, and Joel's
//! upload inside it lost packets alike with 32 jobs uploads: it sent
//! 0.14 Mbit/s, against 3.8 without his download. So the connection counts
//! the VPNs' own packets by their marks, and the VPNs give up what those
//! carry past their CAKEs' count, so what they cost the connection is the
//! share they're held to.

use serde_json::Value;

use crate::rate::{float, whole};

/// The counter shaper.nft keeps of what the connection carries for VPNs.
pub const COUNTER: &str = "vpnout";

/// How much of the gap a second closes when what the VPN costs past its
/// count falls: it gives the rate back over about ten seconds, since a
/// download starting again would cost it at once.
const FALL: f64 = 0.9;

/// What a VPN carries past its CAKE's count, in kbit/s, from the counters
/// read about once a second.
#[derive(Clone, Debug, Default)]
pub struct Cost {
    /// What the connection had carried for the VPNs and what their CAKEs
    /// had counted, in bytes, and when, in seconds.
    seen: Option<(u64, u64, f64)>,
    /// What they carry past it, smoothed.
    extra: f64,
}

impl Cost {
    /// Takes the bytes the connection has carried for the VPNs and what
    /// their CAKEs have counted sending, at `t` seconds, and answers what
    /// the VPNs carry past that count, in kbit/s.
    pub fn sample(&mut self, carried: u64, counted: u64, t: f64) -> u64 {
        if let Some((was_carried, was_counted, at)) = self.seen
            && carried >= was_carried
            && counted >= was_counted
        {
            let dt = t - at;
            if dt <= 0.0 {
                return whole(self.extra);
            }
            let kbit = |bytes: u64| float(bytes) * 8.0 / 1_000.0 / dt;
            let now = (kbit(carried - was_carried) - kbit(counted - was_counted)).max(0.0);
            self.extra = if now >= self.extra {
                now
            } else {
                now + (self.extra - now) * FALL.powf(dt)
            };
        }
        // A counter that went back was made again: counting starts over
        // from it, and what was learned of the VPN still holds.
        self.seen = Some((carried, counted, t));
        whole(self.extra)
    }
}

/// The mark a VPN's own packets carry, by which the connection counts
/// them: the `fwmark` of a rule, in `ip -j rule show`, whose table routes
/// through `dev`. A VPN routes its own packets past itself by one, `not
/// fwmark M lookup T`, as `wg-quick` does for a `WireGuard` tunnel.
/// `routes` gives `ip -j route show table T` for a table.
pub fn mark_of(rules: &str, dev: &str, routes: impl Fn(&str) -> String) -> Option<String> {
    let Ok(Value::Array(rules)) = serde_json::from_str::<Value>(rules) else {
        return None;
    };
    rules.iter().find_map(|rule| {
        // A masked mark, `0xca6c/0xffff`, names its value before the mask.
        let mark = rule["fwmark"].as_str()?.split('/').next()?;
        let table = match &rule["table"] {
            Value::String(name) => name.clone(),
            Value::Number(number) => number.to_string(),
            _ => return None,
        };
        let Ok(Value::Array(found)) = serde_json::from_str::<Value>(&routes(&table)) else {
            return None;
        };
        found
            .iter()
            .any(|route| route["dev"] == dev)
            .then(|| mark.to_owned())
    })
}

/// The bytes a named counter has counted, from `nft -j list counter`.
pub fn counter_bytes(json: &str) -> Option<u64> {
    let Ok(Value::Object(all)) = serde_json::from_str::<Value>(json) else {
        return None;
    };
    all.get("nftables")?
        .as_array()?
        .iter()
        .find_map(|item| item["counter"]["bytes"].as_u64())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_vpns_mark_is_its_rules_whose_table_routes_through_it() {
        let rules = r#"[{"priority":0,"src":"all","table":"local"},
            {"priority":32764,"not":null,"src":"all","fwmark":"0x51","table":"77"},
            {"priority":32765,"not":null,"src":"all","fwmark":"0xca6c/0xffff","table":"51820"},
            {"priority":32766,"src":"all","table":"main"}]"#;
        let routes = |table: &str| match table {
            "77" => r#"[{"dst":"default","dev":"other0"}]"#.to_owned(),
            "51820" => r#"[{"dst":"default","dev":"wg0"}]"#.to_owned(),
            _ => String::new(),
        };
        assert_eq!(
            mark_of(rules, "wg0", routes),
            Some("0xca6c".to_owned()),
            "a masked mark names its value"
        );
        assert_eq!(
            mark_of(rules, "other0", routes),
            Some("0x51".to_owned()),
            "a table named by number"
        );
        assert_eq!(
            mark_of(rules, "wg1", routes),
            None,
            "no rule routes through wg1"
        );
        assert_eq!(
            mark_of("not json", "wg0", routes),
            None,
            "rules unread are no mark"
        );
    }

    /// A second's bytes at a rate in kbit/s.
    fn bytes(kbit: u64) -> u64 {
        kbit * 1000 / 8
    }

    #[test]
    fn what_the_connection_carries_past_the_count_is_given_up() {
        let mut cost = Cost::default();
        assert_eq!(cost.sample(0, 0, 0.0), 0, "nothing measured yet");
        // The CAKE counted 10 Mbit/s and the connection carried 11.5.
        assert_eq!(cost.sample(bytes(11_500), bytes(10_000), 1.0), 1500);
        assert_eq!(cost.sample(bytes(23_000), bytes(20_000), 2.0), 1500);
    }

    #[test]
    fn a_cost_that_grows_is_given_up_at_once_and_one_that_falls_slowly() {
        let mut cost = Cost::default();
        cost.sample(0, 0, 0.0);
        assert_eq!(cost.sample(bytes(12_000), bytes(10_000), 1.0), 2000);
        // The download ends: the connection carries what was counted.
        let after = cost.sample(bytes(22_000), bytes(20_000), 2.0);
        assert_eq!(after, 1800, "a tenth of the gap closes in a second");
        let mut last = after;
        for second in 3..30 {
            let n = u64::try_from(second).expect("a small count");
            last = cost.sample(
                bytes(2000 + 10_000 * n),
                bytes(10_000 * n),
                f64::from(second),
            );
        }
        assert!(last < 200, "given back within half a minute: {last}");
        assert_eq!(
            cost.sample(bytes(2000 + 300_000 + 13_000), bytes(310_000), 30.0),
            3000,
            "a download starting again costs it at once"
        );
    }

    #[test]
    fn a_cake_that_counts_more_than_was_carried_takes_nothing_back() {
        let mut cost = Cost::default();
        cost.sample(0, 0, 0.0);
        assert_eq!(cost.sample(bytes(9000), bytes(10_000), 1.0), 0);
    }

    #[test]
    fn counters_made_again_start_the_count_over_and_keep_the_cost() {
        let mut cost = Cost::default();
        cost.sample(0, 0, 0.0);
        assert_eq!(cost.sample(bytes(12_000), bytes(10_000), 1.0), 2000);
        // nixos-rebuild loads the table afresh, its counter at nothing.
        assert_eq!(cost.sample(10, bytes(20_000), 2.0), 2000);
        assert_eq!(cost.sample(10 + bytes(12_000), bytes(30_000), 3.0), 2000);
        // A VPN brought back up has a new CAKE.
        assert_eq!(cost.sample(bytes(30_000), 5, 4.0), 2000);
    }

    #[test]
    fn a_counters_bytes_are_read_from_what_nft_prints() {
        let json = r#"{"nftables": [{"metainfo": {"version": "1.1.5"}},
            {"counter": {"family": "inet", "name": "vpnout", "table": "shaper",
            "handle": 4, "packets": 12, "bytes": 15000}}]}"#;
        assert_eq!(counter_bytes(json), Some(15_000));
        assert_eq!(counter_bytes(r#"{"nftables": []}"#), None);
        assert_eq!(counter_bytes("Error: No such file or directory"), None);
    }

    #[test]
    fn a_read_too_soon_after_the_last_waits_for_the_next() {
        let mut cost = Cost::default();
        cost.sample(0, 0, 0.0);
        assert_eq!(cost.sample(bytes(100), bytes(10), 0.0), 0);
        assert_eq!(cost.sample(bytes(12_000), bytes(10_000), 1.0), 2000);
    }
}
