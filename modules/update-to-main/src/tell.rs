//! What the follower tells the person: that the machine can't update, once
//! each time it stops being able to, so a machine that keeps itself
//! current says so when it no longer does. An update that failed at new
//! heads is told, and a refusal only a person's ask gets past; a refusal
//! for being busy isn't, since it passes by itself, nor a build that lost
//! its builders while it's still asked for again soon.

use crate::follows::Followed;

/// The failures already told and the one still to tell.
#[derive(Debug, Default)]
pub struct Telling {
    /// The heads whose failure has been told, so heads that keep failing
    /// are told once however often they're asked for.
    told: Option<Followed>,
    /// Why the machine can't update, until it's been said.
    telling: Option<String>,
}

impl Telling {
    /// Counts an update at `heads` that failed for `why`.
    pub fn failed(&mut self, heads: &Followed, why: &str) {
        if self.told.as_ref() != Some(heads) {
            self.told = Some(heads.clone());
            self.telling = Some(why.to_owned());
        }
    }

    /// Counts a refusal for `why` that waits for a person's ask.
    pub fn waits(&mut self, why: &str) {
        self.telling = Some(why.to_owned());
    }

    /// Counts an update that went through, so the next failure is told.
    pub fn went_through(&mut self) {
        self.told = None;
    }

    /// Why the machine can't update, once.
    pub fn take(&mut self) -> Option<String> {
        self.telling.take()
    }
}

#[cfg(test)]
mod tests {
    use crate::follow::tests::{at, refused};
    use crate::follow::{DROPPED_AGAIN, DROPPED_TRIES, FAILED_AGAIN, Follower};
    use crate::follows::Followed;
    use crate::line::Line;
    use crate::update::NEEDS_HOLD;
    use std::time::Instant;

    fn failed(why: &str, line: &str) -> Line {
        Line::Failed {
            commit: None,
            why: why.to_owned(),
            lines: vec![line.to_owned()],
        }
    }

    #[test]
    fn a_failed_update_is_told_once_until_its_heads_move() {
        let mut f = Follower::default();
        let t = Instant::now();
        let broke = failed("it didn't evaluate", "error: undefined variable");
        f.asked(at("a"), &broke, "", t);
        assert_eq!(
            f.to_tell(),
            Some("it didn't evaluate".to_owned()),
            "a failure went untold"
        );
        assert_eq!(f.to_tell(), None, "what was told was told again");
        f.asked(at("a"), &broke, "", t + FAILED_AGAIN);
        assert_eq!(
            f.to_tell(),
            None,
            "the same heads failing again were told again"
        );
        f.asked(at("b"), &broke, "", t + FAILED_AGAIN);
        assert!(f.to_tell().is_some(), "new heads failing went untold");
    }

    #[test]
    fn heads_that_fail_after_an_update_went_through_are_told_again() {
        let mut f = Follower::default();
        let t = Instant::now();
        let broke = failed("it didn't evaluate", "error: undefined variable");
        f.asked(at("a"), &broke, "", t);
        f.to_tell();
        let through = Line::Unchanged {
            commit: "c".to_owned(),
            followed: Followed::new(),
        };
        f.asked(at("a"), &through, "", t + FAILED_AGAIN);
        assert_eq!(f.to_tell(), None, "an update that went through was told");
        f.asked(at("a"), &broke, "", t + FAILED_AGAIN * 2);
        assert!(
            f.to_tell().is_some(),
            "a failure after one that went through went untold"
        );
    }

    #[test]
    fn lost_builders_are_told_only_once_asking_again_soon_gives_up() {
        let mut f = Follower::default();
        let mut t = Instant::now();
        let lost = failed(
            "building the system on the machine's builders failed (exit status: 1)",
            "error: Nix daemon disconnected unexpectedly (maybe it crashed?)",
        );
        for _ in 0..DROPPED_TRIES {
            f.asked(at("a"), &lost, "", t);
            assert_eq!(f.to_tell(), None, "a build asked for again soon was told");
            t += DROPPED_AGAIN;
        }
        f.asked(at("a"), &lost, "", t);
        assert!(
            f.to_tell().is_some(),
            "builders lost past its tries went untold"
        );
    }

    #[test]
    fn a_busy_machine_is_never_told_and_one_waiting_for_a_person_is() {
        let mut f = Follower::default();
        let t = Instant::now();
        f.asked(at("a"), &refused("a call is on"), "", t);
        assert_eq!(
            f.to_tell(),
            None,
            "a refusal that passes by itself was told"
        );
        let why = format!("{NEEDS_HOLD} nix-daemon.service, which waits for a hold");
        f.asked(at("a"), &refused(&why), "refused", t);
        assert_eq!(f.to_tell(), Some(why), "a wait for a person went untold");
    }
}
