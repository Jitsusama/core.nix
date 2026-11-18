//! Each distant host's quiet round trip, from the pings the shaper sends
//! it over a link (src/probe.rs), for the status and the launcher's
//! Traffic view, which say how far away the connection's servers are while
//! nothing queues. Nothing sets a rate from them: the one-way controller
//! reads each direction's own delay from the recorder (src/live.rs).

use std::collections::HashMap;

/// Each host's least round trip seen lately, in ms. It follows a faster
/// round trip at once and a slower one only over minutes, so a queue that
/// lasts a while isn't taken for the path's own delay.
#[derive(Clone, Debug, Default)]
pub struct Trips {
    hosts: HashMap<String, f64>,
}

impl Trips {
    /// Hears one ping's round trip from `host`, or none for a ping that
    /// never came back, which says nothing of the path.
    pub fn heard(&mut self, host: &str, rtt_ms: Option<f64>) {
        let Some(rtt) = rtt_ms else {
            return;
        };
        let least = self.hosts.get(host).map_or(rtt, |&least| quiet(least, rtt));
        self.hosts.insert(host.to_owned(), least);
    }

    /// Each host heard and its quiet round trip, by the host's name.
    pub fn quiet(&self) -> Vec<(String, f64)> {
        let mut all: Vec<(String, f64)> = self
            .hosts
            .iter()
            .map(|(host, &ms)| (host.clone(), ms))
            .collect();
        all.sort_by(|a, b| a.0.cmp(&b.0));
        all
    }
}

/// The quiet round trip after hearing `rtt`: most of the way to a faster
/// one at once, and a thousandth of the way to a slower one.
fn quiet(least: f64, rtt: f64) -> f64 {
    if rtt < least {
        least * 0.1 + rtt * 0.9
    } else {
        least * 0.999 + rtt * 0.001
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quiet_of(trips: &Trips, host: &str) -> f64 {
        trips
            .quiet()
            .into_iter()
            .find_map(|(h, ms)| (h == host).then_some(ms))
            .expect("the host was heard")
    }

    #[test]
    fn a_round_trip_follows_a_faster_path_at_once_and_a_slower_one_slowly() {
        let mut trips = Trips::default();
        trips.heard("1.1.1.1", Some(40.0));
        trips.heard("1.1.1.1", Some(20.0));
        assert!(quiet_of(&trips, "1.1.1.1") < 23.0, "{:?}", trips.quiet());
        for _ in 0..100 {
            trips.heard("1.1.1.1", Some(200.0));
        }
        assert!(quiet_of(&trips, "1.1.1.1") < 40.0, "{:?}", trips.quiet());
    }

    #[test]
    fn a_lost_ping_says_nothing_of_the_path() {
        let mut trips = Trips::default();
        trips.heard("9.9.9.9", None);
        assert_eq!(trips.quiet(), [], "a host never heard has no round trip");
        trips.heard("9.9.9.9", Some(17.0));
        trips.heard("9.9.9.9", None);
        assert!(
            (quiet_of(&trips, "9.9.9.9") - 17.0).abs() < f64::EPSILON,
            "{:?}",
            trips.quiet()
        );
    }

    #[test]
    fn hosts_are_said_by_name() {
        let mut trips = Trips::default();
        for host in ["9.9.9.9", "1.1.1.1", "8.8.8.8"] {
            trips.heard(host, Some(20.0));
        }
        let names: Vec<String> = trips.quiet().into_iter().map(|(h, _)| h).collect();
        assert_eq!(names, ["1.1.1.1", "8.8.8.8", "9.9.9.9"]);
    }
}
