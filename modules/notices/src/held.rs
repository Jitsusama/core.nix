//! Every notice held, newest first, and what becomes of each: arriving,
//! said again in place, resting off the screen, leaving it, and read.
//! These are the shell's Notices.qml's ways, so the history reads as it
//! did once the daemon holds it.
//!
//! One leaves the screen in one of three ways: put away; withdrawn by its
//! program, as a chat app does once the message is read there; or by
//! itself when its time is up. Only one that left by itself was missed,
//! and one its program withdrew was read there. Apart from that, each is
//! unread until it's acted on or marked read.

use std::collections::BTreeSet;

use serde_json::Value;

use crate::notice::Notice;

/// How far past what Joel would scroll the history keeps: every unread,
/// and the newest of the rest up to this many in all (VKCF3Z, 2026-11-09).
pub const KEEP: usize = 1000;

/// Why a notice closed, by the code the specification's
/// `NotificationClosed` says it with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Closed {
    /// Its time was up.
    Expired = 1,
    /// Joel put it away, or did what it offered.
    Dismissed = 2,
    /// Its program withdrew it, `CloseNotification`.
    Withdrawn = 3,
}

impl Closed {
    /// How the history says it left.
    fn said(self) -> &'static str {
        match self {
            Closed::Expired => "expired",
            Closed::Dismissed => "dismissed",
            Closed::Withdrawn => "withdrawn",
        }
    }
}

/// The notices held, newest first, and the id the next one gets.
#[derive(Debug)]
pub struct Holding {
    notices: Vec<Notice>,
    next: u32,
}

impl Default for Holding {
    fn default() -> Self {
        Holding {
            notices: Vec::new(),
            next: 1,
        }
    }
}

impl Holding {
    /// Those kept, newest first, and the id the next one gets, which is
    /// past every id kept so none is given twice.
    pub fn new(notices: Vec<Notice>, next: u32) -> Holding {
        let past = notices.iter().map(|notice| notice.id).max().unwrap_or(0);
        Holding {
            notices,
            next: next.max(past.saturating_add(1)).max(1),
        }
    }

    /// Every notice held, newest first.
    pub fn notices(&self) -> &[Notice] {
        &self.notices
    }

    /// The id the next notice gets.
    pub fn next(&self) -> u32 {
        self.next
    }

    /// An id never given before.
    pub fn new_id(&mut self) -> u32 {
        let id = self.next;
        self.next = self.next.checked_add(1).unwrap_or(1);
        id
    }

    pub fn get(&self, id: u32) -> Option<&Notice> {
        self.notices.iter().find(|notice| notice.id == id)
    }

    fn get_mut(&mut self, id: u32) -> Option<&mut Notice> {
        self.notices.iter_mut().find(|notice| notice.id == id)
    }

    /// Whether a notice is still open.
    pub fn is_live(&self, id: u32) -> bool {
        self.get(id).is_some_and(|notice| notice.live)
    }

    /// Takes a notice as it arrives, already judged. One whose id is open
    /// already is said again in place: its words, actions and hints are
    /// the new ones, and its place, its judging and whether it's read stay,
    /// as the specification's `replaces_id` means. One held back and
    /// transient is closed at once and never kept, as its sender asked it
    /// pass, and that's answered so its sender is told. Every other is kept
    /// open, the newest, resting from the start where it's held back; the
    /// caller trims what's kept after (`trim`), to take away the pictures
    /// of those that went.
    pub fn arrive(&mut self, notice: Notice) -> Option<Closed> {
        if let Some(open) = self.get_mut(notice.id).filter(|open| open.live) {
            open.app = notice.app;
            open.icon = notice.icon;
            open.entry = notice.entry;
            open.summary = notice.summary;
            open.body = notice.body;
            open.actions = notice.actions;
            open.reply = notice.reply;
            open.urgency = notice.urgency;
            open.category = notice.category;
            open.hints = notice.hints;
            open.expire = notice.expire;
            open.transient = notice.transient;
            open.resident = notice.resident;
            open.picture = notice.picture;
            return None;
        }
        if notice.transient && notice.held {
            return Some(Closed::Dismissed);
        }
        let mut notice = notice;
        notice.live = true;
        notice.resting = notice.held;
        self.notices.retain(|kept| kept.id != notice.id);
        self.notices.insert(0, notice);
        None
    }

    /// One left the screen and closed, if it was open: one that went by
    /// itself stays missed, and one its program withdrew was read there. A
    /// transient one, never kept, goes. Answers whether it was open, so
    /// its sender is told only of one that was.
    pub fn close(&mut self, id: u32, why: Closed, now: i64) -> bool {
        let Some(notice) = self.get_mut(id).filter(|notice| notice.live) else {
            return false;
        };
        notice.live = false;
        notice.resting = false;
        notice.left = Some(why.said().to_owned());
        notice.left_at = Some(now);
        if why != Closed::Expired {
            notice.missed = false;
        }
        if why == Closed::Withdrawn {
            notice.read = true;
            notice.read_how = None;
        }
        let transient = notice.transient;
        if transient {
            self.notices.retain(|notice| notice.id != id);
        }
        true
    }

    /// One whose time on the screen is up rests off it, still open.
    pub fn rest(&mut self, id: u32) -> bool {
        self.change(
            id,
            |notice| notice.live && !notice.resting,
            |notice| {
                notice.resting = true;
            },
        )
    }

    /// One resting comes back to the screen.
    pub fn wake(&mut self, id: u32) -> bool {
        self.change(id, |notice| notice.resting, |notice| notice.resting = false)
    }

    /// One said again in place that tells something new comes back to the
    /// screen unread, the same card rather than another.
    pub fn raise(&mut self, id: u32, now: i64) -> bool {
        self.change(
            id,
            |notice| notice.live,
            |notice| {
                notice.read = false;
                notice.read_how = None;
                notice.resting = false;
                notice.raised_at = Some(now);
            },
        )
    }

    /// Marks those of `ids` read, or unread again. Read, one's no longer
    /// missed, and `how` says where it was read where that wasn't here, as
    /// `readInSlack`, or `withdrawn` where what it said stopped standing,
    /// so History never says Joel read what he never saw; one already read
    /// keeps how it was. Answers whether any changed.
    pub fn mark(&mut self, ids: &[u32], read: bool, how: Option<&str>, now: i64) -> bool {
        let mut changed = false;
        for notice in self
            .notices
            .iter_mut()
            .filter(|notice| ids.contains(&notice.id))
        {
            let was = notice.clone();
            if read && notice.read {
                notice.missed = false;
            } else if read {
                notice.read = true;
                notice.missed = false;
                notice.read_at = Some(now);
                notice.withdrawn = how == Some("withdrawn");
                notice.read_how = how.filter(|how| *how != "withdrawn").map(str::to_owned);
            } else {
                notice.read = false;
                notice.read_at = None;
                notice.read_how = None;
                notice.withdrawn = false;
            }
            changed |= *notice != was;
        }
        changed
    }

    /// Every notice of a to-do that no longer stands is read, so one that
    /// came and went unseen isn't missed once its item is done or gone:
    /// `standing` holds the items Joel can still do.
    pub fn settle(&mut self, standing: &BTreeSet<String>) -> bool {
        let mut changed = false;
        for notice in &mut self.notices {
            let waits = notice.hint("x-waiting").to_owned();
            if !waits.is_empty() && !standing.contains(&waits) && (notice.missed || !notice.read) {
                notice.read = true;
                notice.missed = false;
                notice.read_how = None;
                changed = true;
            }
        }
        changed
    }

    /// Everything held has been seen, though not all of it read.
    pub fn seen(&mut self) -> bool {
        let mut changed = false;
        for notice in self.notices.iter_mut().filter(|notice| notice.missed) {
            notice.missed = false;
            changed = true;
        }
        changed
    }

    /// Every record served, one a notice.
    pub fn records(&self) -> Vec<Value> {
        self.notices.iter().map(Notice::record).collect()
    }

    /// Those kept on disk: all but the transient, which pass unkept.
    pub fn kept(&self) -> impl Iterator<Item = &Notice> {
        self.notices.iter().filter(|notice| !notice.transient)
    }

    fn change(
        &mut self,
        id: u32,
        when: impl Fn(&Notice) -> bool,
        how: impl FnOnce(&mut Notice),
    ) -> bool {
        match self.get_mut(id).filter(|notice| when(notice)) {
            Some(notice) => {
                how(notice);
                true
            }
            None => false,
        }
    }

    /// Past [`KEEP`], the oldest go first, never one awaiting Joel or still
    /// open, so what he hasn't seen outlasts what he has. Answers the
    /// pictures of those that went, for the caller to take away.
    pub fn trim(&mut self) -> Vec<String> {
        let mut over = self.notices.len().saturating_sub(KEEP);
        let mut gone = BTreeSet::new();
        for (at, notice) in self.notices.iter().enumerate().rev() {
            if over == 0 {
                break;
            }
            if !notice.awaits() && !notice.live {
                gone.insert(at);
                over -= 1;
            }
        }
        let mut pictures = Vec::new();
        let mut at = 0;
        self.notices.retain(|notice| {
            let going = gone.contains(&at);
            at += 1;
            if going && !notice.picture.is_empty() {
                pictures.push(notice.picture.clone());
            }
            !going
        });
        pictures
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::notice::Action;

    fn notice(id: u32, summary: &str) -> Notice {
        Notice {
            id,
            summary: summary.into(),
            missed: true,
            ..Notice::default()
        }
    }

    fn held(holding: &Holding) -> Vec<u32> {
        holding.notices().iter().map(|notice| notice.id).collect()
    }

    #[test]
    fn each_arrives_open_and_newest_first() {
        let mut holding = Holding::default();
        assert_eq!(holding.arrive(notice(1, "first")), None);
        assert_eq!(holding.arrive(notice(2, "second")), None);
        assert_eq!(held(&holding), [2, 1]);
        assert!(holding.is_live(1) && holding.is_live(2));
        assert!(!holding.get(1).expect("held").resting);
    }

    #[test]
    fn one_held_back_rests_from_the_start() {
        let mut holding = Holding::default();
        holding.arrive(Notice {
            held: true,
            ..notice(1, "quiet")
        });
        assert!(holding.get(1).expect("held").resting);
    }

    #[test]
    fn one_held_back_and_transient_closes_at_once_unkept() {
        let mut holding = Holding::default();
        let closed = holding.arrive(Notice {
            held: true,
            transient: true,
            ..notice(1, "passing")
        });
        assert_eq!(closed, Some(Closed::Dismissed));
        assert_eq!(
            holding.notices().len(),
            0,
            "a held passing notice isn't kept"
        );
    }

    #[test]
    fn one_said_again_in_place_keeps_its_place_and_whether_its_read() {
        let mut holding = Holding::default();
        holding.arrive(notice(1, "Tokens at 80%"));
        holding.arrive(notice(2, "other"));
        holding.mark(&[1], true, None, 50);
        holding.arrive(Notice {
            actions: vec![Action {
                id: "default".into(),
                text: "Open".into(),
            }],
            missed: false,
            ..notice(1, "Tokens at 90%")
        });
        assert_eq!(held(&holding), [2, 1]);
        let said = holding.get(1).expect("held");
        assert_eq!(said.summary, "Tokens at 90%");
        assert_eq!(said.actions.len(), 1);
        assert!(said.read);
    }

    #[test]
    fn one_that_went_by_itself_stays_missed_and_one_put_away_doesnt() {
        let mut holding = Holding::default();
        holding.arrive(notice(1, "expired"));
        holding.arrive(notice(2, "put away"));
        assert!(holding.close(1, Closed::Expired, 10));
        assert!(holding.close(2, Closed::Dismissed, 10));
        let (expired, away) = (holding.get(1).expect("held"), holding.get(2).expect("held"));
        assert!(expired.missed && !expired.live);
        assert_eq!(expired.left.as_deref(), Some("expired"));
        assert_eq!(expired.left_at, Some(10));
        assert!(!away.missed && !away.read);
    }

    #[test]
    fn one_withdrawn_by_its_program_was_read_there() {
        let mut holding = Holding::default();
        holding.arrive(notice(1, "read in chat"));
        assert!(holding.close(1, Closed::Withdrawn, 10));
        let withdrawn = holding.get(1).expect("held");
        assert!(withdrawn.read && !withdrawn.missed);
        assert!(
            !holding.close(1, Closed::Withdrawn, 20),
            "one closed isn't again"
        );
    }

    #[test]
    fn a_transient_one_goes_as_it_closes() {
        let mut holding = Holding::default();
        holding.arrive(Notice {
            transient: true,
            ..notice(1, "passing")
        });
        assert_eq!(holding.kept().count(), 0);
        assert!(holding.close(1, Closed::Expired, 10));
        assert_eq!(
            holding.notices().len(),
            0,
            "a passing notice goes as it closes"
        );
    }

    #[test]
    fn one_rests_wakes_and_is_raised_unread() {
        let mut holding = Holding::default();
        holding.arrive(notice(1, "resting"));
        assert!(holding.rest(1));
        assert!(!holding.rest(1), "one resting doesn't rest again");
        assert!(holding.wake(1));
        holding.mark(&[1], true, Some("readInSlack"), 5);
        holding.rest(1);
        assert!(holding.raise(1, 9));
        let raised = holding.get(1).expect("held");
        assert!(!raised.read && !raised.resting);
        assert_eq!(raised.read_how, None);
        assert_eq!(raised.raised_at, Some(9));
    }

    #[test]
    fn marked_read_it_says_how_and_when_and_unread_forgets_both() {
        let mut holding = Holding::default();
        holding.arrive(notice(1, "in slack"));
        holding.arrive(notice(2, "gone stale"));
        assert!(holding.mark(&[1], true, Some("readInSlack"), 7));
        assert!(holding.mark(&[2], true, Some("withdrawn"), 8));
        let (slack, stale) = (holding.get(1).expect("held"), holding.get(2).expect("held"));
        assert_eq!(slack.read_how.as_deref(), Some("readInSlack"));
        assert_eq!(slack.read_at, Some(7));
        assert!(stale.withdrawn && stale.read_how.is_none());
        assert!(
            !holding.mark(&[1], true, Some("other"), 9),
            "one read keeps how it was"
        );
        assert!(holding.mark(&[1], false, None, 9));
        let unread = holding.get(1).expect("held");
        assert!(!unread.read && unread.read_at.is_none() && unread.read_how.is_none());
    }

    #[test]
    fn seen_clears_whats_missed_and_settling_reads_what_no_longer_stands() {
        let mut holding = Holding::default();
        let mut waiting = notice(1, "a to-do");
        waiting.hints.insert("x-waiting".into(), "todo:1".into());
        let mut standing = notice(2, "another");
        standing.hints.insert("x-waiting".into(), "todo:2".into());
        holding.arrive(waiting);
        holding.arrive(standing);
        holding.arrive(notice(3, "plain"));
        let still = BTreeSet::from(["todo:2".to_owned()]);
        assert!(holding.settle(&still));
        assert!(holding.get(1).expect("held").read);
        assert!(!holding.get(2).expect("held").read);
        assert!(holding.seen());
        assert!(holding.notices().iter().all(|notice| !notice.missed));
        assert!(!holding.seen(), "nothing left to see");
    }

    #[test]
    fn past_what_it_keeps_the_oldest_read_go_never_one_unread_or_open() {
        let mut kept: Vec<Notice> = (1..=u32::try_from(KEEP).expect("small") + 2)
            .rev()
            .map(|id| Notice {
                read: true,
                picture: format!("file:///pictures/{id}.png"),
                ..notice(id, "read")
            })
            .collect();
        // The oldest is unread, and the next oldest still open.
        let oldest = kept.len() - 1;
        kept[oldest].read = false;
        kept[oldest - 1].live = true;
        let mut holding = Holding::new(kept, 1);
        let pictures = holding.trim();
        assert_eq!(holding.notices().len(), KEEP);
        let ids = held(&holding);
        assert!(
            ids.contains(&1) && ids.contains(&2),
            "the unread and the open stay"
        );
        assert!(!ids.contains(&3) && !ids.contains(&4), "the oldest read go");
        assert_eq!(
            pictures,
            ["file:///pictures/4.png", "file:///pictures/3.png"]
        );
    }

    #[test]
    fn ids_are_never_given_twice_even_past_those_kept() {
        let mut holding = Holding::new(vec![notice(41, "kept")], 7);
        assert_eq!(holding.new_id(), 42);
        assert_eq!(holding.new_id(), 43);
        assert_eq!(holding.next(), 44);
    }
}
