//! Where the net stands against main: the head its round tests, or between
//! rounds the head it last saw, and when that lags long enough to be said.

use crate::watch::Breach;

/// The net's round this many hours behind main's head.
pub const NET_BEHIND_H: f64 = 3.0;

/// The net's round: the head it tests, when that head merged, and how many
/// merges main has past it.
#[derive(Debug, Clone, PartialEq)]
pub struct Round {
    pub head: String,
    pub merged: i64,
    pub behind: usize,
    /// Whether a round tests `head` now, or the net, between rounds, last
    /// saw it: on 10 November the net tested nothing past 19:56's head for
    /// hours while a search ran, and the card said it was caught up.
    pub testing: bool,
}

/// The net's lag at `now`, once its head is past `NET_BEHIND_H` old, and
/// again each time its age doubles, so a long search is known once said.
#[must_use]
pub fn lag(round: &Round, now: i64) -> Option<Breach> {
    #[expect(clippy::cast_precision_loss, reason = "seconds within a lifetime")]
    let hours = (now - round.merged) as f64 / 3600.0;
    if hours <= NET_BEHIND_H {
        return None;
    }
    let mut tier = NET_BEHIND_H;
    while tier * 2.0 <= hours {
        tier *= 2.0;
    }
    let short: String = round.head.chars().take(9).collect();
    let what = if round.testing {
        format!("the net's round at {short}")
    } else {
        format!("the net, between rounds, last saw {short}, which")
    };
    Some(Breach {
        key: format!("net:{short}:{tier}"),
        text: format!(
            "{what} is {hours:.1} h old, {} merges behind main",
            round.behind
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_nets_lag_is_said_at_three_hours_then_at_each_doubling() {
        let round = |hours: i64, testing| Round {
            head: "cb5b9b12dce0".into(),
            merged: 100_000 - hours * 3600,
            behind: 5,
            testing,
        };
        assert_eq!(
            lag(&round(2, true), 100_000),
            None,
            "a lag under three hours was said"
        );
        assert_eq!(
            lag(&round(4, true), 100_000).unwrap().key,
            "net:cb5b9b12d:3"
        );
        let said = lag(&round(7, false), 100_000).unwrap();
        assert_eq!(said.key, "net:cb5b9b12d:6");
        assert!(
            said.text
                .starts_with("the net, between rounds, last saw cb5b9b12d"),
            "a net testing nothing was said to be in a round: {}",
            said.text
        );
    }
}
