//! Lets a burst of changes settle. Joining a network says a dozen things in
//! a few seconds, as the card comes up, gets its address and has its DNS
//! servers set, so the network is looked at once it goes quiet, or a few
//! seconds after the burst began if it never does.

use std::cmp::min;

use tokio::time::{Duration, Instant};

/// How long the network stays quiet before it's looked at.
pub const QUIET: Duration = Duration::from_secs(1);

/// The longest a burst is waited out.
pub const MOST: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, Copy, Default)]
pub struct Settling {
    heard: Option<(Instant, Instant)>,
}

impl Settling {
    pub fn heard(&mut self, at: Instant) {
        let began = self.heard.map_or(at, |(began, _)| began);
        self.heard = Some((began, at));
    }

    /// When to look, once anything was heard.
    pub fn due(&self) -> Option<Instant> {
        let (began, last) = self.heard?;
        Some(min(last + QUIET, began + MOST))
    }

    /// Looked: the next change begins another burst.
    pub fn looked(&mut self) {
        self.heard = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nothing_heard_is_nothing_to_look_at() {
        assert_eq!(Settling::default().due(), None);
    }

    #[test]
    fn one_change_is_looked_at_once_the_network_goes_quiet() {
        let now = Instant::now();
        let mut settling = Settling::default();

        settling.heard(now);

        assert_eq!(settling.due(), Some(now + QUIET));
    }

    #[test]
    fn each_change_in_a_burst_puts_the_look_off() {
        let now = Instant::now();
        let mut settling = Settling::default();

        settling.heard(now);
        settling.heard(now + Duration::from_millis(400));

        assert_eq!(
            settling.due(),
            Some(now + Duration::from_millis(400) + QUIET)
        );
    }

    #[test]
    fn a_burst_that_never_goes_quiet_is_looked_at_anyway() {
        let now = Instant::now();
        let mut settling = Settling::default();

        for tenth in 0..100 {
            settling.heard(now + Duration::from_millis(tenth * 100));
        }

        assert_eq!(settling.due(), Some(now + MOST));
    }

    #[test]
    fn a_change_after_a_look_begins_another_burst() {
        let now = Instant::now();
        let mut settling = Settling::default();
        settling.heard(now);
        settling.looked();

        let later = now + Duration::from_secs(60);
        settling.heard(later);

        assert_eq!(settling.due(), Some(later + QUIET));
    }
}
