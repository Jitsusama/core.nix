//! Whether each move between access points arrived, and how long the card
//! was off the air for it.
//!
//! wpa_supplicant takes a move as soon as it's asked, so wifid said "moved
//! to, as asked" then, whether the card got there or not. On 16 Nov 2026 one
//! didn't: the kernel had forgotten the access point, the card went back to
//! scanning, and it was 18 s on no network with the journal saying it had
//! moved. So each move is followed from the ask, through the card leaving
//! the point it was on, to the card joined again, there or somewhere else.
//! A card that gives up on the way is said lost as it does, and followed
//! on, so where it lands is said too: that one rejoined another point.
//!
//! The time off the air runs from the first step away from the old point,
//! as wpa_supplicant says it, to the card joined again. wpa_supplicant says
//! a step some tens of milliseconds after the kernel takes it, and the card
//! leaves the old point only once the new one's channel is tuned, so the
//! time from the ask is said too: the link is down for no longer than that.
//!
//! A move whose end isn't seen, as when the card goes or wpa_supplicant
//! says its steps too fast to tell apart, is forgotten after a while
//! rather than closed by whatever the card does next.
//!
//! A move that hasn't reached its point within `BACK_AFTER`, or whose card
//! comes off its network on the way, is handed back, once, with the point
//! its card left, so the card is sent back there rather than left on no
//! network: the 16 Nov failure scanned for 18 s. The way back is followed
//! as a move of its own, given `BACK_FOR`, whatever its steps say.

use std::collections::HashMap;

use tokio::time::{Duration, Instant};

/// The steps wpa_supplicant says once a card has given up joining, for a
/// move on its way to them that has failed.
const GAVE_UP: [&str; 4] = ["disconnected", "inactive", "scanning", "interface_disabled"];

/// The steps that take a card off the point it was on. A rekey's
/// handshakes don't, since the card stays on its point meanwhile.
const LEAVING: [&str; 3] = ["authenticating", "associating", "associated"];

/// How long a move is followed. The move that failed took 18 s to land
/// somewhere (2026-11-16), and a good one about 100 ms.
const FOLLOWED: Duration = Duration::from_secs(30);

/// How long a move has to reach its point before its card is sent back to
/// the one it left. Good moves reached theirs 113 and 166 ms after the ask
/// (19 Nov 2026), and the slowest of 15 Nov went silent for 401 ms.
pub const BACK_AFTER: Duration = Duration::from_secs(1);

/// How long a way back has to reach the point it left before its card may
/// join whatever it hears. A card moving leaves its point without a word,
/// so a point that protects its frames, as every WPA3 one does, still
/// holds it as joined and bids it come back after 1024 ms, while it asks
/// whether the card is still there; in a test the way back took 1.25 s
/// for that (2026-11-20).
pub const BACK_FOR: Duration = Duration::from_secs(2);

/// Each card's move on its way, by the card's name.
#[derive(Default)]
pub struct Arrivals {
    moves: HashMap<String, Move>,
}

struct Move {
    /// The access point it was asked to move to.
    to: String,
    /// The access point it left, to go back to if it doesn't get there,
    /// until it's handed back. None for a way back, or a card already off
    /// its point when it was held.
    from: Option<String>,
    asked: Instant,
    /// When it was first said to have left the point it was on.
    left: Option<Instant>,
    /// Whether it's been said lost already.
    lost: bool,
    /// Whether it's a way back, followed past a join elsewhere.
    back: bool,
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
    /// It joined a point wifid can't name.
    Unnamed { to: String, asked: Duration },
    /// It gave up joining, and is at this step.
    Lost {
        to: String,
        step: String,
        asked: Duration,
    },
    /// Nothing said where it went while it was followed.
    Unseen { to: String },
}

/// A move that hasn't reached its point, to be sent back.
#[derive(Debug, PartialEq)]
pub struct Unreached {
    pub card: String,
    /// The point the card left.
    pub from: String,
    /// The point it didn't reach.
    pub to: String,
    pub why: Short,
}

/// Why a move is sent back.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Short {
    /// It hadn't reached its point within `BACK_AFTER`.
    Late,
    /// Its card came off its network on the way.
    Off,
}

impl Arrivals {
    /// Takes in that the card was asked to move to that access point from
    /// the one it's on, if it's to go back there. One asked again before it
    /// arrived is followed from the newer ask.
    pub fn asked(&mut self, card: &str, from: Option<&str>, to: &str, now: Instant) {
        let to = to.to_owned();
        let asked = now;
        let moved = Move {
            to,
            from: from.map(str::to_owned),
            asked,
            left: None,
            lost: false,
            back: false,
        };
        self.moves.insert(card.to_owned(), moved);
    }

    /// Takes in that the card was sent back to the access point it left.
    /// A card sent back as it falls off is already on its way to whatever
    /// wpa_supplicant took next, and can join there first, as it joined
    /// the Roadside Inn in a test for 85 ms (2026-11-20). So a way back is
    /// followed past a join elsewhere, each said, to the point it was sent
    /// to, until it's stranded.
    pub fn back(&mut self, card: &str, to: &str, now: Instant) {
        let moved = Move {
            to: to.to_owned(),
            from: None,
            asked: now,
            left: None,
            lost: false,
            back: true,
        };
        self.moves.insert(card.to_owned(), moved);
    }

    /// Takes in that the card, already off its point and joining another
    /// on its own, is now held to that access point, so wherever it joins
    /// is to be judged by it.
    pub fn heading(&mut self, card: &str, to: &str, now: Instant) {
        let to = to.to_owned();
        if let Some(moving) = self.moves.get_mut(card) {
            moving.to = to;
            return;
        }
        let moved = Move {
            to,
            from: None,
            asked: now,
            left: Some(now),
            lost: false,
            back: false,
        };
        self.moves.insert(card.to_owned(), moved);
    }

    /// When the next move with a way back is due to take it, or the next
    /// way back to be stranded, if any.
    pub fn due(&self) -> Option<Instant> {
        self.moves
            .values()
            .filter_map(|moving| {
                if moving.back {
                    Some(moving.asked + BACK_FOR)
                } else {
                    moving.from.as_ref().map(|_| moving.asked + BACK_AFTER)
                }
            })
            .min()
    }

    /// Whether the card is on its way back to the point it left. Selected
    /// there alone, wpa_supplicant starts its join over, and says the card
    /// disconnected meanwhile, twice in a test, 117 and 18 ms in, before it
    /// got there 279 ms after it was sent (2026-11-20). So a card on its way
    /// back is given until it's stranded, rather than judged by its steps.
    pub fn going_back(&self, card: &str) -> bool {
        self.moves.get(card).is_some_and(|moving| moving.back)
    }

    /// Hands back each card whose way back hasn't reached its point
    /// `BACK_FOR` after it was sent, with that point, and forgets it, so
    /// the card can be let join whatever it hears.
    pub fn stranded(&mut self, now: Instant) -> Vec<(String, String)> {
        let stranded: Vec<String> = self
            .moves
            .iter()
            .filter(|(_, moving)| moving.back && now >= moving.asked + BACK_FOR)
            .map(|(card, _)| card.clone())
            .collect();
        let mut stranded: Vec<_> = stranded
            .into_iter()
            .filter_map(|card| {
                let moving = self.moves.remove(&card)?;
                Some((card, moving.to))
            })
            .collect();
        stranded.sort();
        stranded
    }

    /// Hands back each move not at its point `BACK_AFTER` after the ask,
    /// once, with the point its card left. It's still followed, so where
    /// the card joins until it's sent back is said too.
    pub fn unreached(&mut self, now: Instant) -> Vec<Unreached> {
        let mut unreached: Vec<_> = self
            .moves
            .iter_mut()
            .filter(|(_, moving)| now >= moving.asked + BACK_AFTER)
            .filter_map(|(card, moving)| {
                let from = moving.from.take()?;
                let card = card.clone();
                let to = moving.to.clone();
                let why = Short::Late;
                Some(Unreached {
                    card,
                    from,
                    to,
                    why,
                })
            })
            .collect();
        unreached.sort_by(|one, other| one.card.cmp(&other.card));
        unreached
    }

    /// Hands back the card's move at once, with the point it left, as the
    /// card comes off its network on the way. wpa_supplicant gives up on a
    /// point that doesn't answer within some tens of milliseconds, and then
    /// takes whatever other network it hears, as it did in a test
    /// (2026-11-20), unless the card is sent back first. None where the
    /// move has no way back, or was handed back already.
    pub fn fell_off(&mut self, card: &str) -> Option<Unreached> {
        let moving = self.moves.get_mut(card)?;
        let from = moving.from.take()?;
        Some(Unreached {
            card: card.to_owned(),
            from,
            to: moving.to.clone(),
            why: Short::Off,
        })
    }

    /// Forgets the card's move, as one wpa_supplicant refused never began
    /// and one whose card has gone can't end.
    pub fn forget(&mut self, card: &str) {
        self.moves.remove(card);
    }

    /// Takes in a step wpa_supplicant says the card is at, short of joined,
    /// and says if its move has just given up, or was followed too long.
    pub fn step(&mut self, card: &str, step: &str, now: Instant) -> Option<Arrival> {
        if let Some(gone) = self.overdue(card, now) {
            return gone.unseen();
        }
        let moving = self.moves.get_mut(card)?;
        let gave_up = GAVE_UP.contains(&step);
        if gave_up || LEAVING.contains(&step) {
            moving.left.get_or_insert(now);
        }
        if !gave_up || moving.lost {
            return None;
        }
        moving.lost = true;
        Some(Arrival::Lost {
            to: moving.to.clone(),
            step: step.to_owned(),
            asked: now - moving.asked,
        })
    }

    /// Takes in that the card has joined a network, at that access point
    /// where it can be named, and says how its move ended, if it was
    /// moving.
    pub fn joined(&mut self, card: &str, on: Option<&str>, now: Instant) -> Option<Arrival> {
        if let Some(gone) = self.overdue(card, now) {
            return gone.unseen();
        }
        let moving = self.moves.get(card)?;
        let asked = now - moving.asked;
        let to = moving.to.clone();
        let reached = on.is_some_and(|on| to.eq_ignore_ascii_case(on));
        let followed = moving.back && !reached;
        let done = if followed {
            None
        } else {
            self.moves.remove(card)
        };
        let Some(on) = on else {
            return Some(Arrival::Unnamed { to, asked });
        };
        if !reached {
            let on = on.to_owned();
            return Some(Arrival::Elsewhere { to, on, asked });
        }
        let off = done.and_then(|done| done.left).map(|left| now - left);
        Some(Arrival::Reached { to, asked, off })
    }

    /// Ends the card's move if it's been followed too long, handing it back.
    /// None where it's still followed.
    fn overdue(&mut self, card: &str, now: Instant) -> Option<Move> {
        let moving = self.moves.get(card)?;
        if now - moving.asked <= FOLLOWED {
            return None;
        }
        self.moves.remove(card)
    }
}

impl Move {
    /// A move followed too long, said unseen where it wasn't said lost
    /// already.
    fn unseen(self) -> Option<Arrival> {
        (!self.lost).then_some(Arrival::Unseen { to: self.to })
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
            Arrival::Unnamed { to, asked } => format!(
                "<4>{card} joined an access point wifid can't name, {} ms after it was asked \
                 to move to {to}",
                asked.as_millis()
            ),
            Arrival::Lost { to, step, asked } => format!(
                "<4>{card} didn't reach {to}: it was {step} {} ms after it was asked",
                asked.as_millis()
            ),
            Arrival::Unseen { to } => format!(
                "<4>{card}'s move to {to} wasn't seen to end within {} s",
                FOLLOWED.as_secs()
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DESK: &str = "6c:ae:f6:c1:84:26";
    const HALL: &str = "6c:ae:f6:c1:84:28";
    const FAR: &str = "6c:ae:f6:ba:85:87";

    fn ms(millis: u64) -> Duration {
        Duration::from_millis(millis)
    }

    /// Plays these steps, at so many ms after the ask, ending in a join at
    /// that point, and gives every line said.
    fn played(steps: &[(u64, &str)], joined: (u64, Option<&str>)) -> Vec<String> {
        let mut arrivals = Arrivals::default();
        let now = Instant::now();
        arrivals.asked("wlan0", Some(DESK), FAR, now);
        let mut said: Vec<_> = steps
            .iter()
            .filter_map(|&(at, step)| arrivals.step("wlan0", step, now + ms(at)))
            .collect();
        let (at, on) = joined;
        said.extend(arrivals.joined("wlan0", on, now + ms(at)));
        said.iter().map(|arrival| arrival.said("wlan0")).collect()
    }

    #[test]
    fn a_move_that_arrives_says_how_long_from_the_ask_and_off_the_air() {
        // Each step of the move at 17:29:51 on 19 Nov 2026, as wifid said it.
        let steps = [
            (72, "authenticating"),
            (84, "associating"),
            (94, "associated"),
        ];
        let steps = [steps.as_slice(), &[(99, "4way_handshake")]].concat();

        let said = played(&steps, (113, Some("6C:AE:F6:BA:85:87")));

        assert_eq!(
            said,
            [
                "<6>wlan0 reached 6c:ae:f6:ba:85:87 113 ms after it was asked, \
                 off the air 41 ms of them"
            ],
            "timed from the ask, and from the first step away"
        );
    }

    #[test]
    fn a_move_with_steps_said_together_still_arrives() {
        // The move at 18:03:14 on 19 Nov 2026, associated said with the next.
        let steps = [
            (126, "authenticating"),
            (129, "associating"),
            (151, "4way_handshake"),
        ];

        let said = played(&steps, (166, Some(FAR)));

        assert_eq!(
            said,
            [
                "<6>wlan0 reached 6c:ae:f6:ba:85:87 166 ms after it was asked, \
                 off the air 40 ms of them"
            ]
        );
    }

    #[test]
    fn a_move_that_gives_up_is_said_lost_once_then_where_it_landed() {
        // The move that failed at 05:10:41 on 16 Nov 2026: it went back to
        // scanning, then rejoined the point it came from 18 s on.
        let steps = [
            (21, "authenticating"),
            (201, "scanning"),
            (18_000, "scanning"),
        ];
        let steps = [steps.as_slice(), &[(18_370, "authenticating")]].concat();

        let said = played(&steps, (18_373, Some(DESK)));

        assert_eq!(
            said,
            [
                "<4>wlan0 didn't reach 6c:ae:f6:ba:85:87: it was scanning 201 ms after it was \
                 asked",
                "<4>wlan0 joined 6c:ae:f6:c1:84:26 rather than 6c:ae:f6:ba:85:87, \
                 18373 ms after it was asked",
            ],
            "lost as it scans, once, and where it ended up"
        );
    }

    #[test]
    fn a_move_that_lands_somewhere_unnamed_says_so() {
        let said = played(&[(30, "authenticating")], (90, None));

        assert_eq!(
            said,
            [
                "<4>wlan0 joined an access point wifid can't name, 90 ms after it was asked \
                 to move to 6c:ae:f6:ba:85:87"
            ],
            "closed, not left open for a later step to close"
        );
    }

    #[test]
    fn a_move_followed_too_long_is_said_unseen_not_pinned_on_what_comes_next() {
        let said = played(
            &[(40, "authenticating"), (31_000, "disconnected")],
            (32_000, Some(DESK)),
        );

        assert_eq!(
            said,
            ["<4>wlan0's move to 6c:ae:f6:ba:85:87 wasn't seen to end within 30 s"]
        );
    }

    #[test]
    fn a_lost_move_followed_too_long_ends_quietly() {
        let said = played(&[(201, "scanning")], (40_000, Some(DESK)));

        assert_eq!(
            said,
            ["<4>wlan0 didn't reach 6c:ae:f6:ba:85:87: it was scanning 201 ms after it was asked"],
            "its loss was said already"
        );
    }

    #[test]
    fn a_rekey_isnt_leaving() {
        let said = played(
            &[
                (10, "4way_handshake"),
                (20, "group_handshake"),
                (60, "authenticating"),
            ],
            (100, Some(FAR)),
        );

        assert_eq!(
            said,
            [
                "<6>wlan0 reached 6c:ae:f6:ba:85:87 100 ms after it was asked, \
                 off the air 40 ms of them"
            ],
            "off the air from authenticating, not the rekey before it"
        );
    }

    #[test]
    fn a_card_not_moving_says_nothing() {
        let mut arrivals = Arrivals::default();
        let now = Instant::now();

        let step = arrivals.step("wlan0", "scanning", now);
        let joined = arrivals.joined("wlan0", Some(DESK), now);

        assert_eq!(step, None, "no move, so no loss");
        assert_eq!(joined, None, "no move, so no arrival");
    }

    #[test]
    fn a_refused_move_is_forgotten_and_a_newer_ask_followed() {
        let mut arrivals = Arrivals::default();
        let now = Instant::now();
        arrivals.asked("wlan0", Some(DESK), HALL, now);
        arrivals.forget("wlan0");
        let forgotten = arrivals.joined("wlan0", Some(HALL), now + ms(50));

        arrivals.asked("wlan0", Some(FAR), HALL, now);
        arrivals.asked("wlan0", Some(FAR), DESK, now + ms(500));
        let newer = arrivals.joined("wlan0", Some(DESK), now + ms(600));

        assert_eq!(forgotten, None, "a refused move never began");
        assert_eq!(
            newer,
            Some(Arrival::Reached {
                to: DESK.into(),
                asked: ms(100),
                off: None
            }),
            "timed from the newer ask, with no step away said"
        );
    }

    #[test]
    fn a_card_held_elsewhere_on_its_way_is_judged_by_where_its_now_held() {
        let mut arrivals = Arrivals::default();
        let now = Instant::now();
        arrivals.asked("wlan0", Some(FAR), HALL, now);
        arrivals.step("wlan0", "authenticating", now + ms(20));
        arrivals.heading("wlan0", DESK, now + ms(50));
        let retargeted = arrivals.joined("wlan0", Some(DESK), now + ms(120));

        arrivals.heading("wlan0", HALL, now + ms(200));
        let on_its_own = arrivals.joined("wlan0", Some(HALL), now + ms(260));

        assert_eq!(
            retargeted,
            Some(Arrival::Reached {
                to: DESK.into(),
                asked: ms(120),
                off: Some(ms(100))
            }),
            "the first ask's times, the newer hold's point"
        );
        assert_eq!(
            on_its_own,
            Some(Arrival::Reached {
                to: HALL.into(),
                asked: ms(60),
                off: Some(ms(60))
            }),
            "a card already on its way is off the air from the hold"
        );
    }

    #[test]
    fn a_card_moving_doesnt_answer_for_another() {
        let mut arrivals = Arrivals::default();
        let now = Instant::now();
        arrivals.asked("wlan0", Some(DESK), HALL, now);

        let other = arrivals.step("wlan1", "scanning", now + ms(10));
        let ours = arrivals.joined("wlan0", Some(HALL), now + ms(90));

        assert_eq!(other, None, "another card's step isn't this move's");
        assert!(
            matches!(ours, Some(Arrival::Reached { .. })),
            "this card's move still arrives, but ended {ours:?}"
        );
    }

    #[test]
    fn a_move_not_there_in_a_second_is_handed_back_once_with_the_point_it_left() {
        let mut arrivals = Arrivals::default();
        let now = Instant::now();
        arrivals.asked("wlan0", Some(DESK), FAR, now);
        let lost = arrivals.step("wlan0", "scanning", now + ms(201));

        let due = arrivals.due();
        let early = arrivals.unreached(now + ms(999));
        let unreached = arrivals.unreached(now + BACK_AFTER);
        let again = arrivals.unreached(now + ms(2000));

        assert!(lost.is_some(), "said lost as it scans, as before");
        assert_eq!(due, Some(now + BACK_AFTER), "due a second after the ask");
        assert_eq!(early, [], "not before its second is up");
        assert_eq!(
            unreached,
            [Unreached {
                card: "wlan0".into(),
                from: DESK.into(),
                to: FAR.into(),
                why: Short::Late
            }],
            "handed back with the point it left"
        );
        assert_eq!(again, [], "handed back once");
        assert_eq!(arrivals.due(), None, "nothing more due");
    }

    #[test]
    fn a_move_whose_card_falls_off_its_network_is_handed_back_at_once_and_once() {
        let mut arrivals = Arrivals::default();
        let now = Instant::now();
        arrivals.asked("wlan0", Some(DESK), FAR, now);
        arrivals.step("wlan0", "authenticating", now + ms(20));
        arrivals.step("wlan0", "disconnected", now + ms(188));

        let off = arrivals.fell_off("wlan0");
        let again = arrivals.fell_off("wlan0");
        let late = arrivals.unreached(now + BACK_AFTER);

        assert_eq!(
            off,
            Some(Unreached {
                card: "wlan0".into(),
                from: DESK.into(),
                to: FAR.into(),
                why: Short::Off
            }),
            "handed back as it falls off, with the point it left"
        );
        assert_eq!(again, None, "handed back once");
        assert_eq!(late, [], "not again when its second is up");
        assert_eq!(arrivals.due(), None, "nothing more due");
    }

    #[test]
    fn a_card_falling_off_with_no_way_back_hands_nothing_back() {
        let mut arrivals = Arrivals::default();
        let now = Instant::now();
        arrivals.heading("wlan0", FAR, now);

        let held = arrivals.fell_off("wlan0");
        let unfollowed = arrivals.fell_off("wlan1");

        assert_eq!(held, None, "a card held on its way left no point known");
        assert_eq!(unfollowed, None, "a card not moving has no move");
    }

    #[test]
    fn a_move_there_in_time_is_never_handed_back() {
        let mut arrivals = Arrivals::default();
        let now = Instant::now();
        arrivals.asked("wlan0", Some(DESK), FAR, now);
        arrivals.joined("wlan0", Some(FAR), now + ms(113));

        let unreached = arrivals.unreached(now + BACK_AFTER);

        assert_eq!(unreached, [], "it got there");
        assert_eq!(arrivals.due(), None, "nothing due once it's there");
    }

    #[test]
    fn a_way_back_is_followed_to_where_it_ends_with_no_way_back_of_its_own() {
        let mut arrivals = Arrivals::default();
        let now = Instant::now();
        arrivals.asked("wlan0", Some(DESK), FAR, now);
        arrivals.unreached(now + BACK_AFTER);
        arrivals.back("wlan0", DESK, now + BACK_AFTER);

        let off = arrivals.fell_off("wlan0");
        let late = arrivals.unreached(now + ms(1999));
        let back = arrivals.joined("wlan0", Some(DESK), now + ms(1150));

        assert_eq!(off, None, "a way back has none of its own as it falls off");
        assert_eq!(late, [], "nor as it's late");
        assert_eq!(
            back,
            Some(Arrival::Reached {
                to: DESK.into(),
                asked: ms(150),
                off: None
            }),
            "timed from its own ask"
        );
        assert_eq!(
            arrivals.joined("wlan0", Some(DESK), now + ms(2000)),
            None,
            "ended"
        );
    }

    #[test]
    fn a_way_back_overtaken_by_a_join_elsewhere_is_followed_on_to_its_point() {
        let mut arrivals = Arrivals::default();
        let now = Instant::now();
        arrivals.asked("wlan0", Some(DESK), FAR, now);
        arrivals.step("wlan0", "disconnected", now + ms(295));
        arrivals.fell_off("wlan0");
        arrivals.back("wlan0", DESK, now + ms(306));

        let inn = arrivals.joined("wlan0", Some(HALL), now + ms(344));
        let back = arrivals.joined("wlan0", Some(DESK), now + ms(689));

        assert_eq!(
            inn,
            Some(Arrival::Elsewhere {
                to: DESK.into(),
                on: HALL.into(),
                asked: ms(38)
            }),
            "the join it was already on its way to is said"
        );
        assert_eq!(
            back,
            Some(Arrival::Reached {
                to: DESK.into(),
                asked: ms(383),
                off: None
            }),
            "and then where it was sent back to"
        );
    }

    #[test]
    fn a_way_back_not_there_in_two_seconds_is_stranded_once_whatever_its_steps_said() {
        let mut arrivals = Arrivals::default();
        let now = Instant::now();
        arrivals.asked("wlan0", Some(DESK), FAR, now);
        arrivals.fell_off("wlan0");
        arrivals.back("wlan0", DESK, now + ms(189));
        arrivals.step("wlan0", "disconnected", now + ms(306));

        let due = arrivals.due();
        let going = arrivals.going_back("wlan0");
        // As long as the point bade it wait in a test, and the join after.
        let early = arrivals.stranded(now + ms(189) + ms(1250));
        let stranded = arrivals.stranded(now + ms(189) + BACK_FOR);
        let again = arrivals.stranded(now + ms(5000));

        assert_eq!(
            due,
            Some(now + ms(189) + BACK_FOR),
            "due two seconds after it's sent"
        );
        assert!(going, "still going back, though it said it disconnected");
        assert_eq!(early, [], "not while a point bids it wait its second");
        assert_eq!(
            stranded,
            [("wlan0".to_owned(), DESK.to_owned())],
            "stranded short of the point it was sent to"
        );
        assert_eq!(again, [], "once");
        assert!(!arrivals.going_back("wlan0"), "and forgotten");
    }

    #[test]
    fn a_card_held_on_its_way_has_no_way_back() {
        let mut arrivals = Arrivals::default();
        let now = Instant::now();

        arrivals.heading("wlan0", FAR, now);

        assert_eq!(arrivals.due(), None, "the point it left isn't known");
    }
}
