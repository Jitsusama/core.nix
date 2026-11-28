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
//!
//! That cost is never read from one second alone. WARP sends what its
//! CAKE passed it in bursts, so in one second its own packets may run well
//! over the count and in the next well under. Read a second at a time, and
//! kept at its highest while it fell back slowly, those swings alone held
//! the cost near their peaks: on 28 November WARP's CAKE stood at 5,444
//! kbit/s, its 9,310 less 3,866 of cost, while the whole upload was about
//! 5 Mbit/s, and the model relay's calls inside it were dropped. So a cost
//! is taken up at once only once two seconds in a row both show it, as a
//! download does from its second second, and otherwise as it stands over
//! the last `WINDOW_S`. A burst within one read costs only its share of the
//! window; one spanning two reads costs the smaller of them for one step.

use std::collections::VecDeque;

use serde_json::Value;

use crate::rate::{float, whole};

/// The counter shaper.nft keeps of what the connection carries for VPNs.
pub const COUNTER: &str = "vpnout";

/// The seconds over which what the VPN costs past its count is read: long
/// enough that a burst WARP sends and the lull after it fall in the same
/// read, short enough that a download's cost is given up within a few
/// seconds of its start.
const WINDOW_S: f64 = 6.0;

/// What a VPN carries past its CAKE's count, in kbit/s, from the counters
/// read about once a second.
#[derive(Clone, Debug, Default)]
pub struct Cost {
    /// What the connection had carried for the VPNs and what their CAKEs
    /// had counted, in bytes, and when, in seconds: the reads of the last
    /// `WINDOW_S`, with the one just before it.
    seen: VecDeque<(u64, u64, f64)>,
    /// What they carried past it between the last two reads, in kbit/s.
    last: Option<f64>,
    /// What they carry past it, as last read.
    extra: f64,
}

impl Cost {
    /// Takes the bytes the connection has carried for the VPNs and what
    /// their CAKEs have counted sending, at `t` seconds, and answers what
    /// the VPNs carry past that count, in kbit/s.
    pub fn sample(&mut self, carried: u64, counted: u64, t: f64) -> u64 {
        match self.seen.back() {
            Some(&(_, _, at)) if t <= at => return whole(self.extra),
            // A counter that went back was made again: counting starts
            // over from it, and what was learned of the VPN still holds.
            Some(&(was_carried, was_counted, _))
                if carried < was_carried || counted < was_counted =>
            {
                self.seen.clear();
                self.last = None;
            }
            _ => {}
        }
        let past = |&(was_carried, was_counted, at): &(u64, u64, f64)| {
            let kbit = |bytes: u64| float(bytes) * 8.0 / 1_000.0 / (t - at);
            (kbit(carried - was_carried) - kbit(counted - was_counted)).max(0.0)
        };
        let previous = self.seen.back().map(past);
        self.seen.push_back((carried, counted, t));
        let gone = |&(_, _, at): &(u64, u64, f64)| at <= t - WINDOW_S;
        while self.seen.get(1).is_some_and(gone) {
            self.seen.pop_front();
        }
        let Some(now) = previous else {
            self.last = None;
            return whole(self.extra);
        };
        // Two seconds in a row, or the whole window, but never one second.
        let held = self.last.map(|last| last.min(now));
        let windowed = self
            .seen
            .front()
            .filter(|read| read.2 <= t - WINDOW_S)
            .map(past);
        self.last = Some(now);
        let Some(read) = held.into_iter().chain(windowed).reduce(f64::max) else {
            return whole(self.extra);
        };
        self.extra = read;
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
        assert_eq!(
            cost.sample(bytes(11_500), bytes(10_000), 1.0),
            0,
            "one second alone is never taken up"
        );
        assert_eq!(cost.sample(bytes(23_000), bytes(20_000), 2.0), 1500);
        assert_eq!(cost.sample(bytes(34_500), bytes(30_000), 3.0), 1500);
    }

    #[test]
    fn a_cost_two_seconds_show_is_given_up_at_once_and_goes_as_the_window_shows() {
        let mut cost = Cost::default();
        cost.sample(0, 0, 0.0);
        cost.sample(bytes(12_000), bytes(10_000), 1.0);
        assert_eq!(cost.sample(bytes(24_000), bytes(20_000), 2.0), 2000);
        // The download ends: the connection carries what was counted.
        assert_eq!(
            cost.sample(bytes(34_000), bytes(30_000), 3.0),
            0,
            "one second without it, and no full window, keeps none of it"
        );
        let mut costs = Vec::new();
        for second in 4..30 {
            let n = u64::try_from(second).expect("a small count");
            costs.push(cost.sample(
                bytes(4000 + 10_000 * n),
                bytes(10_000 * n),
                f64::from(second),
            ));
        }
        assert!(
            costs.iter().all(|&kbit| kbit <= 700),
            "the window keeps no more than its share of the download: {costs:?}"
        );
        assert_eq!(
            costs.last(),
            Some(&0),
            "given back within half a minute: {costs:?}"
        );
        // A download starting again costs 3 Mbit/s past the count.
        let mut costs = Vec::new();
        for second in 30..32 {
            let n = u64::try_from(second - 29).expect("a small count");
            costs.push(cost.sample(
                bytes(4000 + 290_000 + 13_000 * n),
                bytes(290_000 + 10_000 * n),
                f64::from(second),
            ));
        }
        assert_eq!(
            costs.last(),
            Some(&3000),
            "a download starting again costs it whole from its second second: {costs:?}"
        );
    }

    // A burst that spans two reads is taken up for the second of them, and
    // at the next read only its share of the window is kept.
    #[test]
    fn a_burst_two_reads_show_costs_one_step() {
        let mut cost = Cost::default();
        cost.sample(0, 0, 0.0);
        let (mut carried, mut counted) = (0, 0);
        let mut costs = Vec::new();
        let sends = [
            4000, 7000, 7000, 4000, 4000, 4000, 4000, 4000, 4000, 4000, 4000,
        ];
        for (second, kbit) in (1..12).zip(sends) {
            carried += bytes(kbit);
            counted += bytes(4000);
            costs.push(cost.sample(carried, counted, f64::from(second)));
        }
        assert_eq!(costs[2], 3000, "two reads of it are taken up: {costs:?}");
        assert!(
            costs[3..].iter().all(|&kbit| kbit <= 1000),
            "the burst was kept past its step: {costs:?}"
        );
    }

    #[test]
    fn a_cost_two_seconds_show_after_a_full_window_is_given_up_at_once() {
        let mut cost = Cost::default();
        cost.sample(0, 0, 0.0);
        for second in 1..10_u32 {
            let n = u64::from(second);
            cost.sample(bytes(10_000 * n), bytes(10_000 * n), f64::from(second));
        }
        // A download costing 2 Mbit/s past the count starts at second 10.
        let first = cost.sample(bytes(102_000), bytes(100_000), 10.0);
        assert!(
            first <= 400,
            "one second of it kept {first}, past its share"
        );
        let second = cost.sample(bytes(114_000), bytes(110_000), 11.0);
        assert_eq!(second, 2000, "the second second of it is taken up whole");
    }

    /// What a VPN sending `sends` in turn, a second each, against 4 Mbit/s
    /// its CAKE counts, costs at each read for 40 seconds.
    fn bursts(sends: &[u64]) -> Vec<u64> {
        let mut cost = Cost::default();
        cost.sample(0, 0, 0.0);
        let (mut carried, mut counted) = (0, 0);
        sends
            .iter()
            .cycle()
            .zip(1..40)
            .map(|(&kbit, second)| {
                carried += bytes(kbit);
                counted += bytes(4000);
                cost.sample(carried, counted, f64::from(second))
            })
            .collect()
    }

    // On 28 November WARP's CAKE stood at 5,444 kbit/s, 3,866 under its
    // 9,310, with the whole upload about 5 Mbit/s: WARP sends in bursts, and
    // its cost read one second at a time and kept at its peak stood near
    // the bursts' peaks.
    #[test]
    fn a_vpn_s_bursts_cost_about_what_they_carry_past_the_count_on_the_whole() {
        // 2 and 7 in turn, 4.5 on the whole: half a megabit past the count.
        let costs = bursts(&[2000, 7000]);
        assert!(
            costs.iter().all(|&kbit| kbit <= 600),
            "bursts carrying 500 kbit/s past the count on the whole cost {costs:?}"
        );
        // Bursts whose turn the window doesn't divide: 4.5 on the whole
        // again, read within twice what they carry past the count.
        let costs = bursts(&[7000, 2000, 3000, 6000, 4500]);
        assert!(
            costs.iter().all(|&kbit| kbit <= 1000),
            "bursts carrying 500 kbit/s past the count on the whole cost {costs:?}"
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
        cost.sample(bytes(12_000), bytes(10_000), 1.0);
        assert_eq!(cost.sample(bytes(24_000), bytes(20_000), 2.0), 2000);
        // nixos-rebuild loads the table afresh, its counter at nothing.
        assert_eq!(cost.sample(10, bytes(30_000), 3.0), 2000);
        assert_eq!(cost.sample(10 + bytes(12_000), bytes(40_000), 4.0), 2000);
        assert_eq!(cost.sample(10 + bytes(24_000), bytes(50_000), 5.0), 2000);
        // A VPN brought back up has a new CAKE.
        assert_eq!(cost.sample(bytes(40_000), 5, 6.0), 2000);
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
        cost.sample(bytes(12_000), bytes(10_000), 1.0);
        assert_eq!(cost.sample(bytes(24_000), bytes(20_000), 2.0), 2000);
    }
}
