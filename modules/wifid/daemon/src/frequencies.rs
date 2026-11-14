//! The frequencies a machine keeps its cards off, network by network, read
//! from avoid.json, which `jitsusama.wifid.avoid` writes:
//!
//! ```json
//! { "Home": [{ "from": 5735, "to": 5835 }] }
//! ```
//!
//! Each range is in MHz, its ends included. A network it names is handed to
//! wpa_supplicant with a `freq_list` of every standard channel's centre but
//! those, so the card joins and roams only there; one it doesn't name keeps
//! every channel. It's a list kept rather than a list avoided because
//! that's what wpa_supplicant takes, and every channel stays but those named,
//! so a home access point radar moves to another 5 GHz channel, or its
//! 2.4 GHz radio, is still joined (14 Nov 2026: on the channel kept,
//! 5260 MHz, the gateway answered at a p95 of 15 ms against 119 on the one
//! avoided, 5745 MHz, beside the access point's backhaul).

use std::collections::HashMap;

use serde::Deserialize;

/// What avoid.json says.
#[derive(Debug, Default, PartialEq)]
pub struct Avoided(HashMap<String, Vec<Range>>);

/// Frequencies kept off, in MHz, both ends included.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Range {
    from: u32,
    to: u32,
}

impl Avoided {
    /// Reads avoid.json, refusing a range that ends before it starts.
    pub fn read(text: &str) -> Result<Avoided, String> {
        let named: HashMap<String, Vec<Range>> =
            serde_json::from_str(text).map_err(|error| error.to_string())?;
        for (network, ranges) in &named {
            if let Some(range) = ranges.iter().find(|range| range.to < range.from) {
                return Err(format!(
                    "{network}'s range from {} to {} ends before it starts",
                    range.from, range.to
                ));
            }
        }
        Ok(Avoided(named))
    }

    /// The `freq_list` the network is joined with, or none where it keeps
    /// every channel.
    pub fn list(&self, network: &str) -> Option<String> {
        let ranges = self.0.get(network).filter(|ranges| !ranges.is_empty())?;
        let avoided = |frequency: &u32| {
            ranges
                .iter()
                .any(|range| (range.from..=range.to).contains(frequency))
        };
        let kept: Vec<String> = channels()
            .filter(|frequency| !avoided(frequency))
            .map(|frequency| frequency.to_string())
            .collect();
        Some(kept.join(" "))
    }
}

/// Every standard channel's centre, in MHz, lowest first: 2.4 GHz's 1 to 13
/// and Japan's 14, 5 GHz's 36 to 144 and 149 to 177 four apart (149 sits
/// 25 MHz above 144, off the step below it), and 6 GHz's 1 to 233 four
/// apart. It's a table rather than what the card says it tunes to, so the
/// list needs nothing asked of the card; wpa_supplicant passes over a
/// frequency the card can't reach.
fn channels() -> impl Iterator<Item = u32> {
    let low = (2412..=2472).step_by(5).chain([2484]);
    let middle = (5180..=5720).step_by(20).chain((5745..=5885).step_by(20));
    let high = (5955..=7115).step_by(20);
    low.chain(middle).chain(high)
}

#[cfg(test)]
mod tests {
    use super::*;

    const HOME: &str = r#"{"Home": [{"from": 5735, "to": 5835}]}"#;

    fn kept(network: &str) -> Vec<u32> {
        let avoided = Avoided::read(HOME).expect("the ranges read");
        let list = avoided.list(network).expect("the network keeps a list");
        list.split(' ')
            .map(|frequency| frequency.parse().expect("each is a number"))
            .collect()
    }

    #[test]
    fn a_network_named_keeps_every_channel_but_those_avoided() {
        let kept = kept("Home");
        for avoided in [5745, 5765, 5785, 5805, 5825] {
            assert!(!kept.contains(&avoided), "{avoided} MHz is kept: {kept:?}");
        }
        for still in [
            2412, 2437, 2484, 5180, 5260, 5500, 5720, 5845, 5865, 5955, 7115,
        ] {
            assert!(kept.contains(&still), "{still} MHz isn't kept: {kept:?}");
        }
    }

    #[test]
    fn the_kept_channels_are_each_said_once_lowest_first() {
        let kept = kept("Home");
        assert!(
            kept.windows(2).all(|pair| pair[0] < pair[1]),
            "the list isn't rising: {kept:?}"
        );
        assert_eq!(
            kept.first(),
            Some(&2412),
            "the list starts elsewhere: {kept:?}"
        );
        assert_eq!(
            kept.last(),
            Some(&7115),
            "the list ends elsewhere: {kept:?}"
        );
    }

    #[test]
    fn a_network_not_named_keeps_every_channel_with_no_list() {
        let avoided = Avoided::read(HOME).expect("the ranges read");
        assert_eq!(
            avoided.list("Roadside Inn"),
            None,
            "a network not named is narrowed"
        );
        assert_eq!(
            Avoided::default().list("Home"),
            None,
            "nothing read narrows a network"
        );
    }

    #[test]
    fn a_range_ending_before_it_starts_or_unknown_is_refused() {
        let backwards = Avoided::read(r#"{"Home": [{"from": 5835, "to": 5735}]}"#);
        assert!(
            backwards.is_err_and(|why| why.contains("ends before it starts")),
            "a backwards range is read"
        );
        let unknown = Avoided::read(r#"{"Home": [{"from": 5735, "until": 5835}]}"#);
        assert!(
            unknown.is_err(),
            "a range with a key it doesn't know is read"
        );
    }
}
