//! Which cards have been looking for networks too long, and when to nudge
//! them.
//!
//! wpa_supplicant says a card is scanning from the moment it asks for a look
//! until the look is done, and a look over every channel takes about six
//! seconds. A card still scanning after far longer, with a saved network in
//! range, is told to join what it can, as the launcher does. A card that
//! keeps failing is nudged ever less often, until it joins something.
//!
//! A card that sat off every network at boot, after an access point didn't
//! answer, wasn't scanning: wpa_supplicant had nothing pending at all. The
//! wpa_supplicant module patches that where it starts.
//!
//! A card left on purpose, or whose radio is off, isn't scanning, so it's
//! never nudged.

use std::collections::HashMap;

use tokio::time::{Duration, Instant};

/// How long a card may scan before it's nudged.
pub const STUCK: Duration = Duration::from_secs(20);

/// The longest a card that keeps failing waits between nudges.
const LONGEST: Duration = Duration::from_secs(320);

/// Each card's look, by its name.
#[derive(Default)]
pub struct Stalls {
    cards: HashMap<String, Looking>,
}

struct Looking {
    /// When the card began scanning, if it's scanning now.
    since: Option<Instant>,
    /// How long it may scan before it's next nudged.
    wait: Duration,
}

impl Stalls {
    /// Takes in how far wpa_supplicant says the card has got.
    pub fn step(&mut self, card: &str, step: &str, now: Instant) {
        if step == "completed" {
            self.cards.remove(card);
            return;
        }
        let looking = self.cards.entry(card.to_string()).or_insert(Looking {
            since: None,
            wait: STUCK,
        });
        match (step, looking.since) {
            ("scanning", None) => looking.since = Some(now),
            ("scanning", Some(_)) => {}
            _ => looking.since = None,
        }
    }

    /// When the next card is due a nudge, if any is scanning.
    pub fn due(&self) -> Option<Instant> {
        self.cards
            .values()
            .filter_map(|looking| Some(looking.since? + looking.wait))
            .min()
    }

    /// The cards due a nudge now, each counted nudged: it waits twice as
    /// long before the next, should it still be scanning then.
    pub fn nudge(&mut self, now: Instant) -> Vec<(String, Duration)> {
        let mut due = Vec::new();
        for (card, looking) in &mut self.cards {
            let Some(since) = looking.since else {
                continue;
            };
            if since + looking.wait > now {
                continue;
            }
            due.push((card.clone(), now - since));
            looking.since = Some(now);
            looking.wait = (looking.wait * 2).min(LONGEST);
        }
        due.sort();
        due
    }

    /// Forgets the card, as it goes.
    pub fn gone(&mut self, card: &str) {
        self.cards.remove(card);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECOND: Duration = Duration::from_secs(1);

    #[test]
    fn a_card_scanning_past_the_limit_is_due() {
        let start = Instant::now();
        let mut stalls = Stalls::default();
        stalls.step("wlan0", "scanning", start);

        assert_eq!(stalls.due(), Some(start + STUCK));
        assert!(stalls.nudge(start + STUCK - SECOND).is_empty());
        assert_eq!(
            stalls.nudge(start + STUCK),
            vec![("wlan0".to_string(), STUCK)]
        );
    }

    #[test]
    fn a_look_that_ends_owes_nothing() {
        let start = Instant::now();
        let mut stalls = Stalls::default();
        stalls.step("wlan0", "scanning", start);
        stalls.step("wlan0", "disconnected", start + 5 * SECOND);

        assert_eq!(stalls.due(), None);
        assert!(stalls.nudge(start + STUCK).is_empty());
    }

    #[test]
    fn a_card_that_isn_t_scanning_is_never_due() {
        let start = Instant::now();
        let mut stalls = Stalls::default();
        for step in ["disconnected", "inactive", "interface_disabled"] {
            stalls.step("wlan0", step, start);
        }

        assert_eq!(stalls.due(), None);
    }

    #[test]
    fn scanning_said_again_keeps_the_first_start() {
        let start = Instant::now();
        let mut stalls = Stalls::default();
        stalls.step("wlan0", "scanning", start);
        stalls.step("wlan0", "scanning", start + 10 * SECOND);

        assert_eq!(stalls.due(), Some(start + STUCK));
    }

    #[test]
    fn a_card_still_failing_waits_longer_each_time() {
        let start = Instant::now();
        let mut stalls = Stalls::default();
        let mut now = start;
        let mut waits = Vec::new();
        stalls.step("wlan0", "scanning", now);
        for _ in 0..6 {
            let due = stalls.due().unwrap();
            waits.push((due - now).as_secs());
            now = due;
            stalls.nudge(now);
            // The nudge's join fails, and the card looks again.
            stalls.step("wlan0", "authenticating", now);
            stalls.step("wlan0", "scanning", now);
        }

        assert_eq!(waits, vec![20, 40, 80, 160, 320, 320]);
    }

    #[test]
    fn a_card_still_scanning_after_a_nudge_waits_longer_from_it() {
        let start = Instant::now();
        let mut stalls = Stalls::default();
        stalls.step("wlan0", "scanning", start);
        stalls.nudge(start + STUCK);

        assert_eq!(stalls.due(), Some(start + STUCK + 2 * STUCK));
    }

    #[test]
    fn joining_a_network_starts_the_waits_again() {
        let start = Instant::now();
        let mut stalls = Stalls::default();
        stalls.step("wlan0", "scanning", start);
        stalls.nudge(start + STUCK);
        stalls.step("wlan0", "completed", start + STUCK);
        stalls.step("wlan0", "scanning", start + 100 * SECOND);

        assert_eq!(stalls.due(), Some(start + 100 * SECOND + STUCK));
    }

    #[test]
    fn each_card_is_followed_on_its_own() {
        let start = Instant::now();
        let mut stalls = Stalls::default();
        stalls.step("wlan0", "scanning", start);
        stalls.step("wlan1", "scanning", start + 5 * SECOND);
        stalls.step("wlan0", "completed", start + 6 * SECOND);

        assert_eq!(stalls.due(), Some(start + 5 * SECOND + STUCK));
        stalls.gone("wlan1");
        assert_eq!(stalls.due(), None);
    }
}
