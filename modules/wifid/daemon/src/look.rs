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

use std::collections::HashMap;

use crate::transition::Request;

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
}

/// What a newer ask does to a card's look.
#[derive(Debug, PartialEq, Eq)]
pub enum Again {
    /// The card waits on no look of that point's, so it's heeded as any.
    Fresh,
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
    /// begun by wpa_supplicant's answer numbered `after`.
    pub fn wait(
        &mut self,
        card: &str,
        request: Request,
        from: Option<String>,
        looked: Looked,
        after: u32,
    ) {
        let waiting = Waiting {
            request,
            from,
            looked,
            after,
        };
        self.0.insert(card.to_owned(), waiting);
    }

    /// Whether a card waits on a look. wifid itself only asks what a
    /// look's end or a newer ask does, so only the cases read it.
    #[cfg(test)]
    fn waits(&self, card: &str) -> bool {
        self.0.contains_key(card)
    }

    /// Takes a newer ask of a card now on `from`. One from another point
    /// than asked before is heeded afresh. A newer warning naming a
    /// channel the card's look didn't cover has it look again.
    pub fn asked_again(&mut self, card: &str, request: &Request, from: Option<&str>) -> Again {
        let Some(waiting) = self.0.get_mut(card) else {
            return Again::Fresh;
        };
        if !same_point(waiting.from.as_deref(), from) {
            self.0.remove(card);
            return Again::Fresh;
        }
        if !request.warns_any() && waiting.request.warns_any() {
            return Again::Kept;
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
    /// by wpa_supplicant's answer numbered 10.
    fn waiting(request: &Request, looked: Looked) -> Looks {
        let mut looks = Looks::default();
        looks.wait("wlan2", request.clone(), Some(ON.to_owned()), looked, 10);
        looks
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
        assert_eq!(looks.asked_again("wlan2", &plain, Some(ON)), Again::Kept);
        assert_eq!(
            looks.ended("wlan2", 11, Some(ON), true),
            Ended::Heed(first.clone(), Looked::Done),
            "a plain ask between warnings doesn't keep a warned card on a point that drops it",
        );

        let again = Request {
            in_ms: 1024,
            ..first.clone()
        };
        let mut looks = waiting(&first, Looked::Done);
        assert_eq!(looks.asked_again("wlan2", &again, Some(ON)), Again::Taken);
        assert_eq!(
            looks.ended("wlan2", 11, Some(ON), true),
            Ended::Heed(again, Looked::Done),
            "a warning said again over the channels looked is heeded with that look",
        );

        let mut revised = warning(FAR);
        revised.candidates[0].frequency = Some(5180);
        let mut looks = waiting(&first, Looked::Done);
        assert_eq!(looks.asked_again("wlan2", &revised, Some(ON)), Again::Taken);
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
            looks.asked_again("wlan2", &request, Some(FAR)),
            Again::Fresh,
            "the point the card is on now asks for itself",
        );
        assert!(!looks.waits("wlan2"), "and the old point's look is past");
        assert_eq!(looks.asked_again("wlan3", &request, Some(ON)), Again::Fresh);
        let mut looks = waiting(&request, Looked::Done);
        looks.forget("wlan2");
        assert!(
            !looks.waits("wlan2"),
            "a card off its network looks for nothing"
        );
    }
}
