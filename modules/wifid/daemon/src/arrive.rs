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
//! A move whose card comes off its network on the way is handed back at
//! once, with the point its card left, so the card is sent back there
//! rather than left on no network: the 16 Nov failure scanned for 18 s.
//! Every step that gives up comes after one that held a network, so this
//! is how a failed move is handed back. One still joining is handed back
//! once `BACK_FOR` is up, and one whose card hasn't left its point within
//! `BACK_AFTER` is handed back as unstarted, counted from the end of any
//! look its card was in as it was asked, since wpa_supplicant runs a move
//! after a look rather than cutting it short. Each is handed back once.
//! The way back is followed as a move of its own, given `BACK_FOR`,
//! whatever its steps say.
//!
//! Long because its cases, over half of it, stay beside the private
//! steps they prove, as every module in wifid keeps them.

use std::collections::HashMap;

use tokio::time::{Duration, Instant};

/// The steps wpa_supplicant says once a card has given up joining, for a
/// move on its way to them that has failed.
const GAVE_UP: [&str; 4] = ["disconnected", "inactive", "scanning", "interface_disabled"];

/// The steps that take a card off the point it was on. A rekey's
/// handshakes don't, since the card stays on its point meanwhile.
const LEAVING: [&str; 3] = ["authenticating", "associating", "associated"];

/// Whether a card at this step has left the point it was on, joining
/// another or having given up.
pub fn has_left(step: &str) -> bool {
    LEAVING.contains(&step) || GAVE_UP.contains(&step)
}

/// Whether which point a card is on decides if it has left the one it
/// was on: only in a handshake or done, steps a rekey on its own point
/// passes through too.
pub fn needs_point(step: &str) -> bool {
    !has_left(step) && crate::supplicant::holds_a_network(step)
}

/// Whether a card whose move wifid heard no step of has left `from`, at
/// this step and `on` the point it's on: one leaving or given up has, and
/// one in a handshake or done (`needs_point`) has unless it's on `from`.
/// Where its point can't be named (`None`), one in a handshake is taken as
/// having left, as the move it was asked is likelier than a rekey in the
/// same moment, and carried on it costs at worst a way back to where it
/// is, where taken as never begun it has its networks on mid-join. One
/// done isn't: wifid has just found it isn't done on the point it was
/// sent to, so it's likeliest still on `from`, and joined, it has no join
/// for its networks to come on in the middle of.
pub fn has_left_for(step: &str, on: Option<&str>, from: &str) -> bool {
    has_left(step)
        || (needs_point(step)
            && on.map_or(step != "completed", |on| !on.eq_ignore_ascii_case(from)))
}

/// How long a move is followed. The move that failed took 18 s to land
/// somewhere (2026-11-16), and a good one about 100 ms.
const FOLLOWED: Duration = Duration::from_secs(30);

/// How long a move's card has to leave the point it's on, from the ask or
/// the end of a look it waited on, before the move is taken as unstarted
/// and the card kept where it is. Good moves had reached their points 113
/// and 166 ms after the ask (19 Nov 2026), so had left them sooner, and
/// the slowest of 15 Nov went silent for 401 ms.
pub const BACK_AFTER: Duration = Duration::from_secs(1);

/// How long a way back has to reach the point it left before its card may
/// join whatever it hears. A card moving leaves its point without a word,
/// so a point that protects its frames, as every WPA3 one does, still
/// holds it as joined and bids it come back after 1024 ms, while it asks
/// whether the card is still there; in a test the way back took 1.25 s
/// for that (2026-11-20).
pub const BACK_FOR: Duration = Duration::from_secs(2);

/// How long a move asked while its card looks waits for the look to end,
/// should its end go unheard. A look over every channel takes 5 to 6 s,
/// and a move waits at most 2 s for one before it's asked (acts.rs).
const LOOK_ENDS: Duration = Duration::from_secs(8);

/// Each card's move on its way, by the card's name.
#[derive(Default)]
pub struct Arrivals {
    moves: HashMap<String, Move>,
}

/// A card's move as it stood, to put back with `Arrivals::resume` where an
/// ask meant to replace it fails.
pub struct Saved(Move);

impl Saved {
    /// The access point the move was going to.
    pub fn to(&self) -> &str {
        &self.0.to
    }

    /// Whether the move has a point to go back to, and so kept the card's
    /// other networks off.
    pub fn goes_back(&self) -> bool {
        self.0.from.is_some()
    }
}

#[derive(Clone)]
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
    /// Whether its card was looking as it was asked, so the move waits
    /// on the look, and when that look ended, if it has.
    looking: bool,
    looked: Option<Instant>,
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
    /// It was still joining its point so long after its time began,
    /// `BACK_FOR`: from the ask, or from the look it waited on.
    Late { after: Duration },
    /// Its card came off its network on the way.
    Off,
    /// Its card hadn't left the point it was on `BACK_AFTER` after the ask,
    /// so is still there, and has nowhere to go back to.
    Unstarted,
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
            looking: false,
            looked: None,
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
            looking: false,
            looked: None,
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
            looking: false,
            looked: None,
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
                    moving
                        .from
                        .as_ref()
                        .map(|_| moving.started() + moving.given())
                }
            })
            .min()
    }

    /// Takes in that the card was looking as its move was asked. A move
    /// waits behind a look wpa_supplicant is running, so its card leaves
    /// its point only once the look ends.
    pub fn looking(&mut self, card: &str) {
        if let Some(moving) = self.moves.get_mut(card) {
            moving.looking = true;
        }
    }

    /// Takes in that the card's look ended, so a move that waited on it
    /// has its time from now to leave its point. A look ending once the
    /// card has left isn't the one it waited on, whose end went unheard,
    /// so the move keeps its time from leaving.
    pub fn looked(&mut self, card: &str, now: Instant) {
        if let Some(moving) = self.moves.get_mut(card)
            && moving.looking
            && moving.looked.is_none()
            && moving.left.is_none()
        {
            moving.looked = Some(now);
        }
    }

    /// Takes back a move handed back unstarted, its card found looking, so
    /// it waits on that look as one asked during a look does, its way back
    /// kept: wpa_supplicant runs the move once the look ends. Only a move
    /// that hadn't waited on a look already does, so looks without end
    /// can't keep it followed. Says whether it waits.
    pub fn waits_on_look(&mut self, card: &str, from: &str) -> bool {
        let Some(moving) = self.moves.get_mut(card) else {
            return false;
        };
        if moving.looking || moving.left.is_some() || moving.back {
            return false;
        }
        moving.from = Some(from.to_owned());
        moving.looking = true;
        true
    }

    /// Takes back a move handed back unstarted whose card wpa_supplicant
    /// says has left its point after all, the step saying so not yet read,
    /// as left now, its way back kept: so it's given what a move under way
    /// is, rather than ended with the card's networks on as it joins.
    /// Only a move that hadn't left, and not a way back. Says whether it's
    /// under way.
    pub fn under_way(&mut self, card: &str, from: &str, now: Instant) -> bool {
        let Some(moving) = self.moves.get_mut(card) else {
            return false;
        };
        if moving.left.is_some() || moving.back {
            return false;
        }
        moving.from = Some(from.to_owned());
        moving.left = Some(now);
        true
    }

    /// Whether the card is on its way back to the point it left. Selected
    /// there alone, wpa_supplicant starts its join over, and says the card
    /// disconnected meanwhile, twice in a test, 117 and 18 ms in, before it
    /// got there 279 ms after it was sent (2026-11-20). So a card on its way
    /// back is given until it's stranded, rather than judged by its steps.
    pub fn going_back(&self, card: &str) -> bool {
        self.moves.get(card).is_some_and(|moving| moving.back)
    }

    /// Whether a move of the card's, or its way back, is still followed.
    pub fn moving(&self, card: &str) -> bool {
        self.moves.contains_key(card)
    }

    /// The card's move as it stands, if one is followed, to put back with
    /// `resume`.
    pub fn saved(&self, card: &str) -> Option<Saved> {
        self.moves.get(card).cloned().map(Saved)
    }

    /// Puts back the card's move as `saved` gave it, as an ask meant to
    /// replace it failed, so wpa_supplicant carries on with the first,
    /// and its timer still sends the card back from where it began.
    pub fn resume(&mut self, card: &str, saved: Saved) {
        self.moves.insert(card.to_owned(), saved.0);
    }

    /// Hands back each card whose way back hasn't reached its point
    /// `BACK_FOR` after it was sent, with that point and how long it has
    /// been going back, and forgets it, so the card can be let join
    /// whatever it hears.
    pub fn stranded(&mut self, now: Instant) -> Vec<(String, String, Duration)> {
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
                let going = now.saturating_duration_since(moving.asked);
                Some((card, moving.to, going))
            })
            .collect();
        stranded.sort();
        stranded
    }

    /// Hands back each move not at its point in its time after the ask,
    /// once, with the point its card left. It's still followed, so where
    /// the card joins until it's sent back is said too. One whose card
    /// hasn't left its point yet is handed back as unstarted, so it's
    /// kept there rather than sent back to where it is.
    pub fn unreached(&mut self, now: Instant) -> Vec<Unreached> {
        let mut unreached: Vec<_> = self
            .moves
            .iter_mut()
            .filter(|(_, moving)| now >= moving.started() + moving.given())
            .filter_map(|(card, moving)| {
                let from = moving.from.take()?;
                let card = card.clone();
                let to = moving.to.clone();
                let why = if moving.left.is_none() {
                    Short::Unstarted
                } else {
                    // From when its time began, so the line says the bound
                    // it was held to rather than the look before it.
                    Short::Late {
                        after: now - moving.started(),
                    }
                };
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

    /// Forgets every card's move, as the wpa_supplicant making them has
    /// gone.
    pub fn clear(&mut self) {
        self.moves.clear();
    }

    /// Takes in a step wpa_supplicant says the card is at, short of joined,
    /// and says if its move has just given up, or was followed too long.
    pub fn step(&mut self, card: &str, step: &str, now: Instant) -> Option<Arrival> {
        if let Some(gone) = self.overdue(card, now) {
            return gone.unseen();
        }
        let moving = self.moves.get_mut(card)?;
        let gave_up = GAVE_UP.contains(&step);
        if has_left(step) {
            moving.left.get_or_insert(now);
        }
        // A way back is judged by where it gets in its time alone, as it
        // says it disconnected on its way as it goes well.
        if !gave_up || moving.lost || moving.back {
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
        // A way back joined where wifid can't name is ended, as one at
        // its point is: its network is held there, and followed it would
        // be stranded and let go of a join that likely got there.
        let followed = moving.back && on.is_some() && !reached;
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
    /// How long after the ask the move has to reach its point. One that
    /// has left its point is under way, and can be bidden wait 1024 ms by
    /// a point that protects its frames and still holds the card, as a move
    /// back to a point it left lately would be. So it's given what a way
    /// back is, rather than undone as it nears its point. One that gave up
    /// was handed back as its card fell off, so has nothing left to give.
    /// One waiting on a look is given till the look ends, and `BACK_AFTER`
    /// from then.
    fn given(&self) -> Duration {
        if self.left.is_some() {
            BACK_FOR
        } else if self.looking && self.looked.is_none() {
            LOOK_ENDS
        } else {
            BACK_AFTER
        }
    }

    /// When its time began: as the look it waited on ended, or as its
    /// card left where that end went unheard, since wpa_supplicant moves
    /// only once a look is done; else as it was asked. A move timed from
    /// the ask once it left was sent back as it began, the look having
    /// spent its time.
    fn started(&self) -> Instant {
        match (self.looked, self.looking, self.left) {
            (Some(looked), _, _) => looked,
            (None, true, Some(left)) => left,
            _ => self.asked,
        }
    }

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
    fn a_move_aimed_elsewhere_on_its_way_keeps_its_way_back() {
        let mut arrivals = Arrivals::default();
        let now = Instant::now();
        arrivals.asked("wlan0", Some(FAR), HALL, now);
        arrivals.step("wlan0", "authenticating", now + ms(20));
        let before = arrivals.due();

        arrivals.heading("wlan0", DESK, now + ms(50));
        let due = arrivals.due();
        arrivals.step("wlan0", "disconnected", now + ms(90));
        let off = arrivals.fell_off("wlan0");

        assert!(before.is_some(), "a move with a way back is due to take it");
        assert_eq!(due, before, "aimed elsewhere, still due when it was");
        assert_eq!(
            off,
            Some(Unreached {
                card: "wlan0".into(),
                from: FAR.into(),
                to: DESK.into(),
                why: Short::Off
            }),
            "falling off, sent back to the point it left, missing the newer one"
        );
    }

    #[test]
    fn a_move_put_back_after_a_failed_ask_keeps_its_way_and_time() {
        let mut arrivals = Arrivals::default();
        let now = Instant::now();
        // Another card's move, asked late enough to fall due after this
        // one's way back could, so `due` stays this one's.
        arrivals.asked("wlan1", Some(HALL), DESK, now + BACK_FOR + ms(500));
        arrivals.asked("wlan0", Some(DESK), HALL, now);
        arrivals.step("wlan0", "authenticating", now + ms(20));
        let due = arrivals.due();
        let saved = arrivals.saved("wlan0");

        // A second ask, made mid-move from no point, then refused.
        arrivals.asked("wlan0", None, FAR, now + ms(300));
        let saved = saved.expect("a move followed is saved");
        let aimed = saved.to().to_owned();
        let goes_back = saved.goes_back();
        let unbacked = arrivals.saved("wlan0").map(|second| second.goes_back());
        arrivals.resume("wlan0", saved);
        let resumed = arrivals.due();
        let off = arrivals.fell_off("wlan0");

        assert_eq!(aimed, HALL, "saved going where the first ask sent it");
        // Which has the networks the second ask kept off stay off for it,
        // and those of one with nowhere to go back to come on.
        assert!(goes_back, "saved with its way back");
        assert_eq!(unbacked, Some(false), "the second, from no point, has none");
        assert_eq!(
            arrivals.saved("wlan1").map(|saved| saved.0.to),
            Some(DESK.to_owned()),
            "another card's move left as it was"
        );
        assert!(due.is_some(), "a move with a way back is due to take it");
        assert_eq!(resumed, due, "due when the first move was");
        assert_eq!(
            off,
            Some(Unreached {
                card: "wlan0".into(),
                from: DESK.into(),
                to: HALL.into(),
                why: Short::Off
            }),
            "put back, sent back to where the first move began"
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
    fn a_move_under_way_is_given_as_long_as_a_way_back_before_it_is_handed_back() {
        let mut arrivals = Arrivals::default();
        let now = Instant::now();
        arrivals.asked("wlan0", Some(DESK), FAR, now);
        arrivals.step("wlan0", "authenticating", now + ms(20));
        // Bidden wait 1024 ms by a point that still holds the card.
        arrivals.step("wlan0", "associating", now + ms(30));

        let due = arrivals.due();
        let at_a_second = arrivals.unreached(now + BACK_AFTER);
        let unreached = arrivals.unreached(now + BACK_FOR);

        assert_eq!(due, Some(now + BACK_FOR), "due as a way back would be");
        assert_eq!(at_a_second, [], "not undone as it nears its point");
        assert_eq!(
            unreached,
            [Unreached {
                card: "wlan0".into(),
                from: DESK.into(),
                to: FAR.into(),
                why: Short::Late { after: BACK_FOR }
            }],
            "handed back once its time is up"
        );
    }

    #[test]
    fn a_move_that_lands_past_its_second_still_arrives_with_nothing_more_due() {
        let mut arrivals = Arrivals::default();
        let now = Instant::now();
        arrivals.asked("wlan0", Some(DESK), FAR, now);
        arrivals.step("wlan0", "associating", now + ms(20));
        let early = arrivals.unreached(now + BACK_AFTER);

        let reached = arrivals.joined("wlan0", Some(FAR), now + ms(1250));

        assert_eq!(early, [], "still under way at its second");
        assert_eq!(
            reached,
            Some(Arrival::Reached {
                to: FAR.into(),
                asked: ms(1250),
                off: Some(ms(1230))
            }),
            "said reached, late as it was"
        );
        assert_eq!(arrivals.due(), None, "nothing due once it's there");
        assert_eq!(arrivals.unreached(now + BACK_FOR), [], "never handed back");
    }

    #[test]
    fn a_move_handed_back_late_and_left_to_a_join_is_said_reached_as_it_lands() {
        let mut arrivals = Arrivals::default();
        let now = Instant::now();
        arrivals.asked("wlan0", Some(DESK), FAR, now);
        arrivals.step("wlan0", "associating", now + ms(20));
        let late = arrivals.unreached(now + BACK_FOR);

        // In a join Joel asked for, the card isn't sent back, so the move
        // is still followed.
        let reached = arrivals.joined("wlan0", Some(FAR), now + ms(2100));

        assert_eq!(
            late,
            [Unreached {
                card: "wlan0".into(),
                from: DESK.into(),
                to: FAR.into(),
                why: Short::Late { after: BACK_FOR }
            }],
            "handed back late, still joining"
        );
        assert!(
            matches!(reached, Some(Arrival::Reached { .. })),
            "followed on after it was handed back, but ended {reached:?}"
        );
        assert_eq!(arrivals.due(), None, "nothing more due");
    }

    #[test]
    fn a_move_whose_card_never_left_is_handed_back_unstarted() {
        let mut arrivals = Arrivals::default();
        let now = Instant::now();
        arrivals.asked("wlan0", Some(DESK), FAR, now);

        let due = arrivals.due();
        let early = arrivals.unreached(now + ms(999));
        let unreached = arrivals.unreached(now + BACK_AFTER);

        assert_eq!(due, Some(now + BACK_AFTER), "due a second after the ask");
        assert_eq!(early, [], "not before its second is up");
        assert_eq!(
            unreached,
            [Unreached {
                card: "wlan0".into(),
                from: DESK.into(),
                to: FAR.into(),
                why: Short::Unstarted
            }],
            "still on the point it would go back to"
        );
        assert_eq!(arrivals.unreached(now + BACK_FOR), [], "handed back once");
        assert_eq!(arrivals.due(), None, "nothing more due");
    }

    #[test]
    fn a_move_found_behind_a_look_as_its_second_ends_waits_on_it_once() {
        let mut arrivals = Arrivals::default();
        let now = Instant::now();
        arrivals.asked("wlan0", Some(DESK), FAR, now);
        let unstarted = arrivals.unreached(now + BACK_AFTER);

        let waits = arrivals.waits_on_look("wlan0", DESK);
        let due = arrivals.due();
        let ended = now + ms(3000);
        arrivals.looked("wlan0", ended);
        let again = arrivals.unreached(ended + BACK_AFTER);
        let twice = arrivals.waits_on_look("wlan0", DESK);

        assert_eq!(unstarted.len(), 1, "handed back unstarted at its second");
        assert!(waits, "found looking, it waits on the look");
        assert_eq!(
            due,
            Some(now + LOOK_ENDS),
            "due should the look's end go unheard"
        );
        let whys: Vec<Short> = again.iter().map(|unreached| unreached.why).collect();
        assert_eq!(
            whys,
            [Short::Unstarted],
            "handed back a second after the look, its way back kept"
        );
        assert!(!twice, "but waits on a look only once");
    }

    #[test]
    fn a_move_asked_as_its_card_looks_has_its_second_from_the_look_s_end() {
        let mut arrivals = Arrivals::default();
        let now = Instant::now();
        arrivals.asked("wlan0", Some(DESK), FAR, now);
        arrivals.looking("wlan0");

        let waiting = arrivals.unreached(now + BACK_AFTER);
        let due = arrivals.due();
        let ended = now + ms(3500);
        arrivals.looked("wlan0", ended);
        let after = arrivals.due();
        let early = arrivals.unreached(ended + ms(999));
        let unstarted = arrivals.unreached(ended + BACK_AFTER);

        assert_eq!(waiting, [], "not while its look runs");
        assert_eq!(
            due,
            Some(now + LOOK_ENDS),
            "due should the look's end go unheard"
        );
        assert_eq!(
            after,
            Some(ended + BACK_AFTER),
            "a second from the look's end"
        );
        assert_eq!(early, [], "not before that second is up");
        let whys: Vec<Short> = unstarted.iter().map(|unreached| unreached.why).collect();
        assert_eq!(whys, [Short::Unstarted], "then still on its point");

        arrivals.asked("wlan0", Some(DESK), FAR, now);
        arrivals.looking("wlan0");
        let unheard = arrivals.unreached(now + LOOK_ENDS);
        assert_eq!(
            unheard.len(),
            1,
            "handed back once a look's end goes unheard"
        );
    }

    #[test]
    fn a_move_that_waited_on_a_look_is_timed_from_it_as_it_leaves() {
        let mut arrivals = Arrivals::default();
        let now = Instant::now();
        arrivals.asked("wlan0", Some(DESK), FAR, now);
        arrivals.looking("wlan0");
        let ended = now + ms(3500);
        arrivals.looked("wlan0", ended);
        arrivals.step("wlan0", "authenticating", ended + ms(100));

        let leaving = arrivals.unreached(ended + ms(100));
        let due = arrivals.due();

        assert_eq!(leaving, [], "not sent back as it starts leaving");
        assert_eq!(
            due,
            Some(ended + BACK_FOR),
            "a way back's time from the look"
        );
        let late = arrivals.unreached(ended + BACK_FOR);
        assert_eq!(
            late.first().map(|unreached| unreached.why),
            Some(Short::Late { after: BACK_FOR }),
            "said late by its time from the look, not the ask"
        );

        arrivals.asked("wlan0", Some(DESK), FAR, now);
        arrivals.looking("wlan0");
        let left = now + ms(3000);
        arrivals.step("wlan0", "authenticating", left);
        let unheard = arrivals.due();
        assert_eq!(
            unheard,
            Some(left + BACK_FOR),
            "from leaving, the look's end unheard"
        );
        arrivals.looked("wlan0", left + ms(1500));
        assert_eq!(
            arrivals.due(),
            Some(left + BACK_FOR),
            "still from leaving, as a look ending after it isn't the one it waited on"
        );
    }

    #[test]
    fn a_look_ending_beside_a_move_that_didn_t_wait_on_it_moves_nothing() {
        let mut arrivals = Arrivals::default();
        let now = Instant::now();
        arrivals.asked("wlan0", Some(DESK), FAR, now);

        arrivals.looked("wlan0", now + ms(500));

        assert_eq!(
            arrivals.due(),
            Some(now + BACK_AFTER),
            "still a second from the ask"
        );
    }

    #[test]
    fn a_move_asked_with_no_point_left_is_never_handed_back() {
        let mut arrivals = Arrivals::default();
        let now = Instant::now();
        arrivals.asked("wlan0", None, FAR, now);

        let due = arrivals.due();
        let unreached = arrivals.unreached(now + BACK_FOR);
        let off = arrivals.fell_off("wlan0");
        let reached = arrivals.joined("wlan0", Some(FAR), now + ms(2500));

        assert_eq!(due, None, "nowhere to go back to");
        assert_eq!(unreached, [], "so nothing handed back late");
        assert_eq!(off, None, "nor as it falls off");
        assert!(
            matches!(reached, Some(Arrival::Reached { .. })),
            "still followed to its point, but ended {reached:?}"
        );
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
        // A move that left is due at BACK_FOR, so one whose way back
        // survived would be handed back late then.
        let late = arrivals.unreached(now + BACK_FOR);

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
        arrivals.step("wlan0", "associating", now + ms(20));
        arrivals.unreached(now + BACK_FOR);
        arrivals.back("wlan0", DESK, now + BACK_FOR);

        let off = arrivals.fell_off("wlan0");
        // A second after it was sent, as late as an unstarted move would be.
        let late = arrivals.unreached(now + BACK_FOR + BACK_AFTER);
        let back = arrivals.joined("wlan0", Some(DESK), now + BACK_FOR + ms(1150));

        assert_eq!(off, None, "a way back has none of its own as it falls off");
        assert_eq!(late, [], "nor as it's late");
        assert_eq!(
            back,
            Some(Arrival::Reached {
                to: DESK.into(),
                asked: ms(1150),
                off: None
            }),
            "timed from its own ask"
        );
        assert_eq!(
            arrivals.joined("wlan0", Some(DESK), now + BACK_FOR + ms(1200)),
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
    fn a_way_back_saying_it_disconnected_on_its_way_is_said_lost_by_nothing() {
        let mut arrivals = Arrivals::default();
        let now = Instant::now();
        arrivals.back("wlan0", DESK, now);

        let disconnected = arrivals.step("wlan0", "disconnected", now + ms(117));
        let back = arrivals.joined("wlan0", Some(DESK), now + ms(279));

        assert_eq!(disconnected, None, "its steps on the way say nothing");
        assert_eq!(
            back,
            Some(Arrival::Reached {
                to: DESK.into(),
                asked: ms(279),
                off: Some(ms(162))
            }),
            "and where it gets is said, with how long it was off"
        );
    }

    #[test]
    fn a_move_found_under_way_as_it_was_taken_as_unstarted_goes_on() {
        let mut arrivals = Arrivals::default();
        let now = Instant::now();
        arrivals.asked("wlan0", Some(DESK), FAR, now);
        let unstarted = arrivals.unreached(now + BACK_AFTER);

        let on = arrivals.under_way("wlan0", DESK, now + BACK_AFTER);
        let again = arrivals.under_way("wlan0", DESK, now + BACK_AFTER);

        assert_eq!(
            unstarted.first().map(|unreached| unreached.why),
            Some(Short::Unstarted),
            "taken as unstarted, its step unread"
        );
        assert!(on, "under way, as wpa_supplicant says it left");
        assert!(!again, "once, as it's left now");
        assert_eq!(
            arrivals.due(),
            Some(now + BACK_FOR),
            "given what a move under way is"
        );
        let late = arrivals.unreached(now + BACK_FOR);
        assert_eq!(
            late.first()
                .map(|unreached| (unreached.why, unreached.from.as_str())),
            Some((Short::Late { after: BACK_FOR }, DESK)),
            "and sent back once that's up, its way back kept"
        );

        arrivals.back("wlan0", DESK, now + BACK_FOR);
        assert!(
            !arrivals.under_way("wlan0", DESK, now + BACK_FOR),
            "never a way back"
        );
    }

    #[test]
    fn a_card_unheard_from_has_left_its_point_unless_on_it_or_done_on_one_unnamed() {
        for (step, on, left) in [
            ("authenticating", None, true),
            ("disconnected", None, true),
            ("4way_handshake", Some(FAR), true),
            ("group_handshake", Some(FAR), true),
            ("completed", Some(FAR), true),
            ("4way_handshake", Some(DESK), false),
            ("completed", Some(DESK), false),
            ("completed", Some("6C:AE:F6:C1:84:26"), false),
            ("4way_handshake", None, true),
            ("group_handshake", None, true),
            ("completed", None, false),
            ("scanning", None, true),
            ("", None, false),
            ("", Some(FAR), false),
        ] {
            assert_eq!(
                has_left_for(step, on, DESK),
                left,
                "{step} on {on:?}: a handshake or a join counts unless on its own point, \
                 and on an unnamed one only a handshake does"
            );
        }
    }

    #[test]
    fn only_a_card_in_a_handshake_or_done_is_asked_its_point() {
        for step in ["4way_handshake", "group_handshake", "completed"] {
            assert!(needs_point(step), "{step} is a rekey's step too");
        }
        for step in [
            "authenticating",
            "associating",
            "associated",
            "disconnected",
            "scanning",
            "inactive",
            "",
        ] {
            assert!(!needs_point(step), "{step} says whether it left alone");
        }
    }

    #[test]
    fn a_card_has_left_its_point_as_it_joins_another_or_gives_up() {
        assert!(has_left("authenticating"), "joining another");
        assert!(has_left("disconnected"), "given up");
        assert!(!has_left("completed"), "not while on its point");
        assert!(!has_left("4way_handshake"), "nor as it rekeys there");
    }

    #[test]
    fn a_way_back_joined_where_it_can_t_be_named_ends_there() {
        let mut arrivals = Arrivals::default();
        let now = Instant::now();
        arrivals.back("wlan0", DESK, now);

        let unnamed = arrivals.joined("wlan0", None, now + ms(300));
        let stranded = arrivals.stranded(now + BACK_FOR);

        assert_eq!(
            unnamed,
            Some(Arrival::Unnamed {
                to: DESK.into(),
                asked: ms(300)
            }),
            "the join is said"
        );
        assert!(!arrivals.moving("wlan0"), "and the way back ended");
        assert_eq!(stranded, [], "so it's never stranded");
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
        // Judged as late as a busy loop might.
        let stranded = arrivals.stranded(now + ms(189) + BACK_FOR + ms(40));
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
            [("wlan0".to_owned(), DESK.to_owned(), BACK_FOR + ms(40))],
            "stranded short of the point it was sent to, as long as it went"
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
