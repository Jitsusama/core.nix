//! Whether each move between access points arrived, and how long the card
//! was off the air for it.
//!
//! wpa_supplicant takes a move as soon as it's asked, so wifid said "moved
//! to, as asked" then, whether the card got there or not. On 16 Nov 2026 one
//! didn't: the kernel had forgotten the access point, the card went back to
//! scanning, and it was 18 s on no network with the journal saying it had
//! moved. So each move is followed from the ask, through the card leaving
//! the point it was on, to the card joined again, there or somewhere else,
//! or back to looking for a network.
//!
//! The time off the air runs from the first step away from the old point,
//! as wpa_supplicant says it, to the card joined again. wpa_supplicant says
//! a step some tens of milliseconds after the kernel takes it, and the card
//! leaves the old point only once the new one's channel is tuned, so the
//! time from the ask is said too: the link is down for no longer than that.

use std::collections::HashMap;

use tokio::time::{Duration, Instant};

/// The steps wpa_supplicant says once a card has given up joining, for a
/// move on its way to them that has failed.
const GAVE_UP: [&str; 4] = ["disconnected", "inactive", "scanning", "interface_disabled"];

/// Each card's move on its way, by the card's name.
#[derive(Default)]
pub struct Arrivals {
    moves: HashMap<String, Move>,
}

struct Move {
    /// The access point it was asked to move to.
    to: String,
    asked: Instant,
    /// When it was first said to have left the point it was on.
    left: Option<Instant>,
}

/// How a move ended.
#[derive(Debug, PartialEq)]
pub enum Arrival {
    /// The card joined the point it was asked to, so long after the ask,
    /// and so long after it left the one it was on.
    Reached {
        to: String,
        asked: Duration,
        off: Option<Duration>,
    },
    /// It joined another point instead.
    Elsewhere {
        to: String,
        on: String,
        asked: Duration,
    },
    /// It gave up joining, and is at this step.
    Lost {
        to: String,
        step: String,
        asked: Duration,
    },
}

impl Arrivals {
    /// Takes in that the card was asked to move to that access point. One
    /// asked again before it arrived is followed from the newer ask.
    pub fn asked(&mut self, card: &str, to: &str, now: Instant) {
        let to = to.to_owned();
        let asked = now;
        self.moves.insert(
            card.to_owned(),
            Move {
                to,
                asked,
                left: None,
            },
        );
    }

    /// Forgets the card's move, as one wpa_supplicant refused never began.
    pub fn forget(&mut self, card: &str) {
        self.moves.remove(card);
    }

    /// Takes in a step wpa_supplicant says the card is at, short of joined,
    /// and says how its move ended if it has given up.
    pub fn step(&mut self, card: &str, step: &str, now: Instant) -> Option<Arrival> {
        let on_its_way = self.moves.get_mut(card)?;
        on_its_way.left.get_or_insert(now);
        if !GAVE_UP.contains(&step) {
            return None;
        }
        let gone = self.moves.remove(card)?;
        Some(Arrival::Lost {
            to: gone.to,
            step: step.to_owned(),
            asked: now - gone.asked,
        })
    }

    /// Takes in that the card has joined that access point, and says how
    /// its move ended, if it was moving.
    pub fn joined(&mut self, card: &str, on: &str, now: Instant) -> Option<Arrival> {
        let done = self.moves.remove(card)?;
        let asked = now - done.asked;
        if !done.to.eq_ignore_ascii_case(on) {
            let on = on.to_owned();
            return Some(Arrival::Elsewhere {
                to: done.to,
                on,
                asked,
            });
        }
        let off = done.left.map(|left| now - left);
        Some(Arrival::Reached {
            to: done.to,
            asked,
            off,
        })
    }
}

impl Arrival {
    /// The journal's line for it, with its priority.
    pub fn said(&self, card: &str) -> String {
        match self {
            Arrival::Reached { to, asked, off } => {
                let off = off.map_or_else(String::new, |off| {
                    format!(", off the air {} ms of them", off.as_millis())
                });
                format!(
                    "<6>{card} reached {to} {} ms after it was asked{off}",
                    asked.as_millis()
                )
            }
            Arrival::Elsewhere { to, on, asked } => format!(
                "<4>{card} joined {on} rather than {to}, {} ms after it was asked",
                asked.as_millis()
            ),
            Arrival::Lost { to, step, asked } => format!(
                "<4>{card} didn't reach {to}: it was {step} {} ms after it was asked",
                asked.as_millis()
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DESK: &str = "6c:ae:f6:c1:84:26";
    const HALL: &str = "6c:ae:f6:c1:84:28";

    fn ms(millis: u64) -> Duration {
        Duration::from_millis(millis)
    }

    #[test]
    fn a_move_that_arrives_says_how_long_from_the_ask_and_off_the_air() {
        let mut arrivals = Arrivals::default();
        let now = Instant::now();
        arrivals.asked("wlan0", HALL, now);

        let left = arrivals.step("wlan0", "authenticating", now + ms(72));
        let joining = arrivals.step("wlan0", "4way_handshake", now + ms(99));
        let joined = arrivals.joined("wlan0", "6C:AE:F6:C1:84:28", now + ms(113));

        assert_eq!(left, None, "leaving isn't arriving");
        assert_eq!(joining, None, "nor is a step on the way");
        assert_eq!(
            joined,
            Some(Arrival::Reached {
                to: HALL.into(),
                asked: ms(113),
                off: Some(ms(41)),
            }),
            "timed from the ask, and from the first step away"
        );
        let said = joined.map(|arrival| arrival.said("wlan0"));
        assert_eq!(
            said.as_deref(),
            Some(
                "<6>wlan0 reached 6c:ae:f6:c1:84:28 113 ms after it was asked, \
                 off the air 41 ms of them"
            )
        );
    }

    #[test]
    fn a_move_that_gives_up_says_so_once() {
        let mut arrivals = Arrivals::default();
        let now = Instant::now();
        arrivals.asked("wlan0", HALL, now);

        arrivals.step("wlan0", "authenticating", now + ms(20));
        let lost = arrivals.step("wlan0", "scanning", now + ms(201));
        let later = arrivals.step("wlan0", "scanning", now + ms(900));
        let rejoined = arrivals.joined("wlan0", DESK, now + ms(18_000));

        assert_eq!(
            lost,
            Some(Arrival::Lost {
                to: HALL.into(),
                step: "scanning".into(),
                asked: ms(201),
            })
        );
        assert_eq!(later, None, "said once");
        assert_eq!(rejoined, None, "a join after it's said lost is no move's");
        let said = lost.map(|arrival| arrival.said("wlan0"));
        assert_eq!(
            said.as_deref(),
            Some(
                "<4>wlan0 didn't reach 6c:ae:f6:c1:84:28: it was scanning 201 ms after it was asked"
            )
        );
    }

    #[test]
    fn a_move_that_lands_elsewhere_names_both() {
        let mut arrivals = Arrivals::default();
        let now = Instant::now();
        arrivals.asked("wlan0", HALL, now);

        let joined = arrivals.joined("wlan0", DESK, now + ms(300));

        let said = joined.map(|arrival| arrival.said("wlan0"));
        assert_eq!(
            said.as_deref(),
            Some(
                "<4>wlan0 joined 6c:ae:f6:c1:84:26 rather than 6c:ae:f6:c1:84:28, \
                 300 ms after it was asked"
            )
        );
    }

    #[test]
    fn a_card_not_moving_says_nothing() {
        let mut arrivals = Arrivals::default();
        let now = Instant::now();

        let step = arrivals.step("wlan0", "scanning", now);
        let joined = arrivals.joined("wlan0", DESK, now);

        assert_eq!(step, None);
        assert_eq!(joined, None);
    }

    #[test]
    fn a_refused_move_is_forgotten_and_a_newer_ask_followed() {
        let mut arrivals = Arrivals::default();
        let now = Instant::now();
        arrivals.asked("wlan0", HALL, now);
        arrivals.forget("wlan0");
        let forgotten = arrivals.joined("wlan0", HALL, now + ms(50));

        arrivals.asked("wlan0", HALL, now);
        arrivals.asked("wlan0", DESK, now + ms(500));
        let newer = arrivals.joined("wlan0", DESK, now + ms(600));

        assert_eq!(forgotten, None, "a refused move never began");
        assert_eq!(
            newer,
            Some(Arrival::Reached {
                to: DESK.into(),
                asked: ms(100),
                off: None,
            }),
            "timed from the newer ask, with no step away said"
        );
    }

    #[test]
    fn a_card_moving_doesnt_answer_for_another() {
        let mut arrivals = Arrivals::default();
        let now = Instant::now();
        arrivals.asked("wlan0", HALL, now);

        let other = arrivals.step("wlan1", "scanning", now + ms(10));
        let ours = arrivals.joined("wlan0", HALL, now + ms(90));

        assert_eq!(other, None);
        assert!(matches!(ours, Some(Arrival::Reached { .. })));
    }
}
