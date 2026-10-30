//! When a VPN carries nothing but bulk, so the packets carrying it go as
//! bulk on the connection too.
//!
//! The connection's CAKE sees a VPN's packets as one flow of best effort,
//! whatever they carry inside. On 30 October agents' uploads through a VPN
//! took about 8 Mbit/s of a 12.5 Mbit/s connection, while those going
//! straight out shared about 1, bulk's sixteenth. A VPN routes its own
//! packets past itself by a firewall mark, `not fwmark M lookup T`, as
//! `wg-quick` does for a `WireGuard` tunnel, so its packets are told apart
//! by that mark.
//! While its own CAKE has sent next to nothing outside bulk for a moment,
//! and no typed turn runs, they go as bulk, beside every other upload.
//! Anything more inside puts them back in best effort within a step.

use serde_json::Value;

/// Bytes a step a VPN's CAKE may send outside bulk and still count as
/// quiet: less than a full packet, such as a keepalive or a name looked up.
const CHATTER: u64 = 1500;

/// How long after its last busy step a VPN still counts as busy, so the
/// pauses inside one request don't put it back in bulk.
const HOLD_S: f64 = 2.0;

/// The mark a VPN's own packets carry: the `fwmark` of a rule, in
/// `ip -j rule show`, whose table routes through `dev`. `routes` gives
/// `ip -j route show table T` for a table.
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

/// Whether a VPN carries nothing but bulk, from what its CAKE has sent
/// outside bulk, counted step by step.
#[derive(Clone, Debug, Default)]
pub struct Quiet {
    /// What its CAKE had sent outside bulk when last counted, in bytes.
    seen: Option<u64>,
    /// When, in seconds, a step last sent more than chatter outside bulk.
    busy_at: Option<f64>,
}

impl Quiet {
    /// Takes what the VPN's CAKE has sent outside bulk, in bytes, at `t`
    /// seconds, and says whether the VPN is quiet. One not yet counted
    /// isn't, until it has been quiet for a while; a count that fell is a
    /// CAKE made again, which has sent nothing since.
    pub fn sample(&mut self, unbulk: u64, t: f64) -> bool {
        match self.seen {
            None => self.busy_at = Some(t),
            Some(was) if unbulk.saturating_sub(was) > CHATTER => self.busy_at = Some(t),
            Some(_) => {}
        }
        self.seen = Some(unbulk);
        self.busy_at.is_none_or(|at| t - at >= HOLD_S)
    }
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
        assert_eq!(mark_of(rules, "wg0", routes), Some("0xca6c".to_owned()));
        assert_eq!(mark_of(rules, "other0", routes), Some("0x51".to_owned()));
        assert_eq!(mark_of(rules, "wg1", routes), None);
        assert_eq!(mark_of("not json", "wg0", routes), None);
    }

    #[test]
    fn a_vpn_is_quiet_once_it_has_sent_only_chatter_outside_bulk_for_a_while() {
        let mut quiet = Quiet::default();
        assert!(!quiet.sample(10_000, 0.0), "not yet counted");
        assert!(!quiet.sample(10_100, 1.0));
        assert!(quiet.sample(10_200, 2.0), "chatter alone for the hold");
        assert!(!quiet.sample(20_000, 2.2), "a request inside");
        assert!(!quiet.sample(20_000, 4.0), "held over its pauses");
        assert!(quiet.sample(20_000, 4.5));
        assert!(quiet.sample(5, 4.7), "a CAKE made again has sent nothing");
    }
}
