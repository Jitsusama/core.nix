//! A warned card's look for the points its request names, from the
//! answer that begins it to the end wpa_supplicant says, and what a newer
//! ask or a move of the card's own meanwhile does to it.
//!
//! wpa_supplicant says each look's end as `ScanDone`, its own looks and
//! anyone's alike, and a look under way when the card asked for its own
//! ends first, so a card waits for an end said after the answer that
//! began its look. wpa_supplicant numbers its answers and signals in the
//! order it sends them on its one connection, and the bus keeps that
//! order. Whether it's still scanning can't tell them apart: it sets that
//! only once the radio begins, a turn after it answers.
//!
//! A look asked but queued behind other radio work, not yet begun, as the
//! card's own is asked, ends after the card's answer and is taken for its
//! own: wpa_supplicant refuses a look only once one has begun (scan.c,
//! `wpa_supplicant_trigger_scan`), and says no more of which look ended.
//! The card is then moved only to a point it heard, or let go, never kept
//! to a point that drops it.
//!
//! A look begun may never be said to end: wpa_supplicant drops one whose
//! random address it can't set without a word (`wpas_trigger_scan_cb`).
//! So a card waits on a look `LOOK_AT_MOST`, and a point's next warning
//! after that is heeded afresh.

use std::collections::HashMap;

use tokio::time::{Duration, Instant};

use crate::transition::Request;

/// The longest a card waits on a look. One over every channel takes 5 to
/// 6 s (`Heed::Look`), and a card waits on at most one of those under way
/// and then its own over a few channels.
const LOOK_AT_MOST: Duration = Duration::from_secs(15);

/// How far a card has looked for the points a request names, so it looks
/// once and is then moved or let go, rather than looking until it's
/// dropped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Looked {
    /// Not yet: the request was just said.
    Not,
    /// A look already under way, maybe over other channels, was stopped,
    /// or the card looked over others than a newer ask names, so it looks
    /// for them once that look ends.
    Stopped,
    /// The card looked over the request's channels.
    Done,
    /// The card's look over them failed, which says nothing of whether
    /// the points are there.
    Failed,
}

impl Looked {
    /// Whether the card has had its one look, so it's moved or let go.
    pub fn over(self) -> bool {
        matches!(self, Looked::Done | Looked::Failed)
    }
}

/// The cards waiting on a look for the points a request names, each with
/// the request, the point that asked, how far it's looked and the number
/// of wpa_supplicant's answer that began the look, so the request is
/// heeded again as that look ends.
#[derive(Debug, Default)]
pub struct Looks(HashMap<String, Waiting>);

#[derive(Debug)]
struct Waiting {
    request: Request,
    from: Option<String>,
    looked: Looked,
    after: u32,
    since: Instant,
}

/// What a newer ask does to a card's look.
#[derive(Debug, PartialEq, Eq)]
pub enum Again {
    /// The card waits on no look of that point's, or the newer ask is one
    /// heeded without a look, so it's heeded as any.
    Fresh,
    /// The card waited on a look `LOOK_AT_MOST` and was never said its
    /// end, so the newer ask is heeded afresh.
    Lapsed,
    /// The look waits on with the warning it had, as the newer ask warns
    /// nothing, and a point that warned still drops the card.
    Kept,
    /// The look ends with the newer ask heeded, as a point warns again
    /// until the card is gone.
    Taken,
}

/// What a look ending says of a card waiting on one.
#[derive(Debug, PartialEq, Eq)]
pub enum Ended {
    /// It waits on none.
    Unwaited,
    /// It's still looking: the look that ended was said to before the
    /// card's own began, behind the signal that had it ask for its own.
    Earlier,
    /// It's no longer on the point that asked, so what that point asked
    /// is past.
    Moved(Option<String>),
    /// Its look is over, so the request is heeded again, this far looked.
    Heed(Request, Looked),
}

impl Looks {
    /// Has a card on `from` wait on its look for what `request` names,
    /// begun by wpa_supplicant's answer numbered `after`, at `since`.
    pub fn wait(
        &mut self,
        card: &str,
        request: Request,
        from: Option<String>,
        (looked, after): (Looked, u32),
        since: Instant,
    ) {
        let waiting = Waiting {
            request,
            from,
            looked,
            after,
            since,
        };
        self.0.insert(card.to_owned(), waiting);
    }

    /// Whether a card waits on a look. wifid itself only asks what a
    /// look's end or a newer ask does, so only the cases read it.
    #[cfg(test)]
    fn waits(&self, card: &str) -> bool {
        self.0.contains_key(card)
    }

    /// Takes a newer ask of a card now on `from`, at `now`. One from
    /// another point than asked before is heeded afresh, as is a warning
    /// heeded `at_once`, needing no look: one whose whole network drops
    /// the card, or naming a point it hears now. A newer warning naming a
    /// channel the card's look didn't cover has it look again.
    pub fn asked_again(
        &mut self,
        card: &str,
        request: &Request,
        from: Option<&str>,
        at_once: bool,
        now: Instant,
    ) -> Again {
        let Some(waiting) = self.0.get_mut(card) else {
            return Again::Fresh;
        };
        if !same_point(waiting.from.as_deref(), from) {
            self.0.remove(card);
            return Again::Fresh;
        }
        if now.saturating_duration_since(waiting.since) >= LOOK_AT_MOST {
            self.0.remove(card);
            return Again::Lapsed;
        }
        if !request.warns_any() && waiting.request.warns_any() {
            return Again::Kept;
        }
        if at_once && request.warns_any() {
            self.0.remove(card);
            return Again::Fresh;
        }
        let looked_over = waiting.request.channels();
        if waiting.looked.over()
            && !request
                .channels()
                .iter()
                .all(|channel| looked_over.contains(channel))
        {
            waiting.looked = Looked::Stopped;
        }
        waiting.request = request.clone();
        Again::Taken
    }

    /// Takes the end of a card's look, as wpa_supplicant says one ended:
    /// the number of its signal, the point the card is on now, and whether
    /// the look that ended `succeeded`. A signal numbered before the
    /// answer that began the card's look ended another; the number wraps
    /// only past four billion messages.
    pub fn ended(&mut self, card: &str, serial: u32, on: Option<&str>, succeeded: bool) -> Ended {
        match self.0.get(card) {
            None => return Ended::Unwaited,
            Some(waiting) if serial <= waiting.after => return Ended::Earlier,
            Some(_) => {}
        }
        let Some(waiting) = self.0.remove(card) else {
            return Ended::Unwaited;
        };
        if !same_point(waiting.from.as_deref(), on) {
            return Ended::Moved(waiting.from);
        }
        let looked = match waiting.looked {
            Looked::Done if !succeeded => Looked::Failed,
            looked => looked,
        };
        Ended::Heed(waiting.request, looked)
    }

    /// Forgets a card's look, as the card came off its network or went.
    pub fn forget(&mut self, card: &str) {
        self.0.remove(card);
    }

    /// Forgets every card's look, as wpa_supplicant changed hands: the old
    /// one's looks are never said to end, and the new one numbers its
    /// signals afresh, so a look kept would take each of its ends for one
    /// said before the card's own.
    pub fn clear(&mut self) {
        self.0.clear();
    }
}

/// Whether two points, maybe none, are one, however written.
fn same_point(one: Option<&str>, other: Option<&str>) -> bool {
    match (one, other) {
        (Some(one), Some(other)) => one.eq_ignore_ascii_case(other),
        (None, None) => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transition::Candidate;

    const ON: &str = "02:00:00:00:00:00";
    const NEAR: &str = "02:00:00:00:05:00";
    const FAR: &str = "02:00:00:00:06:00";

    /// A request warning it drops the card, naming `bssid` on channel 6
    /// as the one the point would most rather.
    fn warning(bssid: &str) -> Request {
        Request {
            warns: true,
            leaves_network: false,
            protected: true,
            in_ms: 2048,
            valid_ms: 25_600,
            candidates: vec![Candidate {
                bssid: bssid.to_owned(),
                preference: Some(255),
                frequency: Some(2437),
            }],
        }
    }

    /// A card on `ON` waiting on its look for the request, the look begun
    /// by wpa_supplicant's answer numbered 10, at `START`.
    fn waiting(request: &Request, looked: Looked) -> Looks {
        let mut looks = Looks::default();
        let begun = (looked, 10);
        looks.wait(
            "wlan2",
            request.clone(),
            Some(ON.to_owned()),
            begun,
            start(),
        );
        looks
    }

    /// When each case's look began.
    fn start() -> Instant {
        static START: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();
        *START.get_or_init(Instant::now)
    }

    /// A second into a look.
    fn soon() -> Instant {
        start() + Duration::from_secs(1)
    }

    /// A newer ask of the card on `ON` a second into its look, needing it.
    fn again(looks: &mut Looks, request: &Request) -> Again {
        looks.asked_again("wlan2", request, Some(ON), false, soon())
    }

    #[test]
    fn a_look_s_end_said_before_the_card_s_own_began_is_another_s() {
        let request = warning(NEAR);
        let mut looks = waiting(&request, Looked::Done);
        assert_eq!(
            looks.ended("wlan2", 9, Some(ON), true),
            Ended::Earlier,
            "a look that ended as the card asked for its own is said before the answer",
        );
        assert_eq!(
            looks.ended("wlan2", 10, Some(ON), true),
            Ended::Earlier,
            "nor is the answer itself the look's end",
        );
        assert!(looks.waits("wlan2"), "the card still waits on its own");
        assert_eq!(
            looks.ended("wlan2", 11, Some(ON), true),
            Ended::Heed(request, Looked::Done),
            "its own look's end has the request heeded again",
        );
        assert!(!looks.waits("wlan2"), "and it waits no longer");
        assert_eq!(looks.ended("wlan2", 12, Some(ON), true), Ended::Unwaited);
    }

    #[test]
    fn a_look_that_failed_is_said_so_and_a_stopped_one_looks_again() {
        let request = warning(NEAR);
        assert_eq!(
            waiting(&request, Looked::Done).ended("wlan2", 11, Some(ON), false),
            Ended::Heed(request.clone(), Looked::Failed),
            "a look that failed says nothing of whether the points are there",
        );
        assert!(Looked::Failed.over(), "but it was the card's one look");
        assert_eq!(
            waiting(&request, Looked::Stopped).ended("wlan2", 11, Some(ON), false),
            Ended::Heed(request, Looked::Stopped),
            "the stopped look ending, however, is the card's turn to look",
        );
        assert!(
            !Looked::Stopped.over(),
            "a stopped look isn't the card's own"
        );
    }

    #[test]
    fn a_card_that_left_the_point_as_it_looked_heeds_nothing_it_asked() {
        let request = warning(NEAR);
        let mut looks = waiting(&request, Looked::Done);
        assert_eq!(
            looks.ended("wlan2", 11, Some(FAR), true),
            Ended::Moved(Some(ON.to_owned())),
            "a card moved by its chooser mid-look isn't the one that was warned",
        );
        assert!(!looks.waits("wlan2"), "and it waits on nothing more");
        let mut looks = waiting(&request, Looked::Done);
        assert!(
            matches!(
                looks.ended("wlan2", 11, Some(&ON.to_uppercase()), true),
                Ended::Heed(..)
            ),
            "its own point however written is the one it was on"
        );
    }

    #[test]
    fn a_newer_ask_mid_look_keeps_a_warning_and_looks_where_it_names_anew() {
        let first = warning(NEAR);
        let plain = Request {
            warns: false,
            ..first.clone()
        };
        let mut looks = waiting(&first, Looked::Done);
        assert_eq!(again(&mut looks, &plain), Again::Kept);
        assert_eq!(
            looks.ended("wlan2", 11, Some(ON), true),
            Ended::Heed(first.clone(), Looked::Done),
            "a plain ask between warnings doesn't keep a warned card on a point that drops it",
        );

        let sooner = Request {
            in_ms: 1024,
            ..first.clone()
        };
        let mut looks = waiting(&first, Looked::Done);
        assert_eq!(again(&mut looks, &sooner), Again::Taken);
        assert_eq!(
            looks.ended("wlan2", 11, Some(ON), true),
            Ended::Heed(sooner, Looked::Done),
            "a warning said again over the channels looked is heeded with that look",
        );

        let mut revised = warning(FAR);
        revised.candidates[0].frequency = Some(5180);
        let mut looks = waiting(&first, Looked::Done);
        assert_eq!(again(&mut looks, &revised), Again::Taken);
        assert_eq!(
            looks.ended("wlan2", 11, Some(ON), true),
            Ended::Heed(revised, Looked::Stopped),
            "a point named on a channel the look didn't cover is looked for",
        );
    }

    #[test]
    fn an_ask_from_another_point_or_of_a_card_not_looking_is_heeded_afresh() {
        let request = warning(NEAR);
        let mut looks = waiting(&request, Looked::Done);
        assert_eq!(
            looks.asked_again("wlan2", &request, Some(FAR), false, soon()),
            Again::Fresh,
            "the point the card is on now asks for itself",
        );
        assert!(!looks.waits("wlan2"), "and the old point's look is past");
        assert_eq!(
            looks.asked_again("wlan3", &request, Some(ON), false, soon()),
            Again::Fresh
        );
        let mut looks = waiting(&request, Looked::Done);
        looks.forget("wlan2");
        assert!(
            !looks.waits("wlan2"),
            "a card off its network looks for nothing"
        );
    }

    #[test]
    fn a_warning_heeded_without_a_look_is_heeded_at_once_mid_look() {
        let first = warning(NEAR);
        let whole = Request {
            leaves_network: true,
            ..first.clone()
        };
        let mut looks = waiting(&first, Looked::Done);
        assert_eq!(
            looks.asked_again("wlan2", &whole, Some(ON), true, soon()),
            Again::Fresh,
            "a whole network dropping the card waited on a look it needs none of",
        );
        assert!(!looks.waits("wlan2"), "and the look is past");
        let plain = Request {
            warns: false,
            ..first.clone()
        };
        let mut looks = waiting(&first, Looked::Done);
        assert_eq!(
            looks.asked_again("wlan2", &plain, Some(ON), true, soon()),
            Again::Kept,
            "a plain ask naming a point heard ended the look for a warning",
        );
    }

    #[test]
    fn a_look_never_said_to_end_lapses_and_a_new_hand_ends_every_look() {
        let request = warning(NEAR);
        let mut looks = waiting(&request, Looked::Done);
        let lapsed = start() + LOOK_AT_MOST;
        assert_eq!(
            looks.asked_again("wlan2", &request, Some(ON), false, lapsed),
            Again::Lapsed,
            "a look never said to end swallowed the point's next warning",
        );
        assert!(!looks.waits("wlan2"), "and the look is past");

        let mut looks = waiting(&request, Looked::Done);
        looks.clear();
        assert_eq!(
            looks.ended("wlan2", 3, Some(ON), true),
            Ended::Unwaited,
            "a new wpa_supplicant's first ends were taken for ones before the card's look",
        );
        assert_eq!(again(&mut looks, &request), Again::Fresh);
    }
}
