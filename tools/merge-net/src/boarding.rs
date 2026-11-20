//! Whether a change that passed earlier boards the merge train, and what
//! `ready` says when it can't.
//!
//! The train merges a head as it stands, as a merge commit, and builds what
//! its riders moved over main in one run, so a head main moved beside
//! boards it whether it sits on main or behind it. On 20 November four
//! changes ready to merge were sent to rebase by hand and run again, three
//! of them missing their times, while main moved about every five minutes.
//! Only a head that can't board is sent to run again, or to rebase, and
//! told why, since a rebase alone fixes only some of the reasons.

use crate::state::short;

/// Why a head can't board the merge train.
#[derive(Debug, PartialEq, Eq)]
pub enum Off {
    /// A reader of the terminal kit owed its turn goes first, as it says.
    KitFirst(String),
    /// No one heard which VM tests run before it merges.
    Unheard,
    /// It waits for an earlier gate sharing its files, which `turn` said.
    Held,
    /// It can't merge with main as main stands.
    Conflicts,
    /// `ready` was asked from no branch, and the train merges a branch.
    NoBranch,
    /// Its pull request is a draft, as it says.
    Draft(String),
    /// The train didn't take it: it fell off at this head, as it says, or
    /// the train wouldn't start.
    Train(Option<String>),
}

/// What `boarding` asks, each only once the one before it let the head
/// on, so a head held or unheard is never offered.
pub struct Asks<'a> {
    /// The kit reader that goes first, if one does.
    pub kit_first: &'a dyn Fn() -> Option<String>,
    /// Whether `merge-net needs` was heard for it.
    pub heard: &'a dyn Fn() -> bool,
    /// Whether an earlier gate on its files holds it.
    pub held: &'a dyn Fn() -> bool,
    /// Whether it conflicts with main.
    pub conflicts: &'a dyn Fn() -> bool,
    /// The branch `ready` was asked from, or nothing.
    pub branch: &'a dyn Fn() -> String,
    /// Why the branch's pull request is a draft, if it is.
    pub drafted: &'a dyn Fn(&str) -> Option<String>,
    /// What the train said once it took the head on, or nothing.
    pub offer: &'a dyn Fn(&str) -> Option<String>,
    /// Why the head fell off the train, if it did.
    pub fell: &'a dyn Fn() -> Option<String>,
}

/// What the train said once the head boarded, or why it didn't.
pub fn boarding(asks: &Asks) -> Result<String, Off> {
    if let Some(first) = (asks.kit_first)() {
        return Err(Off::KitFirst(first));
    }
    if !(asks.heard)() {
        return Err(Off::Unheard);
    }
    if (asks.held)() {
        return Err(Off::Held);
    }
    if (asks.conflicts)() {
        return Err(Off::Conflicts);
    }
    let branch = (asks.branch)();
    if branch.is_empty() {
        return Err(Off::NoBranch);
    }
    if let Some(said) = (asks.drafted)(&branch) {
        return Err(Off::Draft(said));
    }
    (asks.offer)(&branch).ok_or_else(|| Off::Train((asks.fell)()))
}

/// What `ready` says of `commit`, behind origin's main at `main`, which
/// since moved `shared` of its files unseen by its run at `checked`, once
/// it boarded or didn't. A run again at a head behind main would check it
/// against the same old base, so one that can't board must rebase, unless
/// what keeps it off is something a rebase doesn't end: a hold, a draft, a
/// kit reader going first, unheard needs or no branch, each of which says
/// its own next step and then boards it as it stands.
pub fn behind_said(
    commit: &str,
    main: &str,
    checked: &str,
    shared: &[String],
    boarded: &Result<String, Off>,
) -> String {
    let moved = format!(
        "the same change passed at {}, but origin's main, {}, has since moved {}",
        short(checked),
        short(main),
        shared.join(", ")
    );
    let why = match boarded {
        Ok(said) | Err(Off::Draft(said) | Off::KitFirst(said)) => {
            return format!("{moved}; {said}");
        }
        Err(Off::Held) => {
            return format!("{moved}; it waits for that gate, so merge again once it ends");
        }
        Err(Off::Unheard) => {
            return format!(
                "{moved}; merge-net needs wasn't heard for it, so run merge-net needs and \
                 merge again"
            );
        }
        Err(Off::NoBranch) => {
            return format!(
                "{moved}; ready was asked from no branch, so ask it again from the pull \
                 request's branch"
            );
        }
        Err(Off::Conflicts) => "it conflicts with main".to_owned(),
        Err(Off::Train(Some(fell))) => fell.clone(),
        Err(Off::Train(None)) => "the train wouldn't start".to_owned(),
    };
    format!(
        "{moved}, which its run didn't see, and it can't board the merge train, as {why}; \
         {} must rebase onto origin/main, push, and merge again",
        short(commit)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};

    fn nothing() -> Option<String> {
        None
    }

    fn yes() -> bool {
        true
    }

    fn no() -> bool {
        false
    }

    fn branch() -> String {
        "main-w".to_owned()
    }

    fn not_drafted(_: &str) -> Option<String> {
        None
    }

    /// Asks that let a head on at every step, from `main-w`, to `offer`.
    fn on(offer: &dyn Fn(&str) -> Option<String>) -> Asks<'_> {
        Asks {
            kit_first: &nothing,
            heard: &yes,
            held: &no,
            conflicts: &no,
            branch: &branch,
            drafted: &not_drafted,
            offer,
            fell: &nothing,
        }
    }

    #[test]
    fn a_head_that_passes_every_ask_boards_from_its_branch() {
        let offered = Cell::new(false);
        let offer = |branch: &str| {
            offered.set(true);
            Some(format!("on the train from {branch}"))
        };
        assert_eq!(
            boarding(&on(&offer)),
            Ok("on the train from main-w".to_owned())
        );
        assert!(
            offered.get(),
            "a head kept off by nothing was never offered"
        );
    }

    // may_merge asks of a head on main in this order, and a head behind it
    // is asked the same, so a hold is found before a conflict, and nothing
    // is offered before every other ask lets it on.
    #[test]
    fn every_ask_is_asked_in_may_merges_order() {
        let asked = RefCell::new(Vec::new());
        let note = |ask: &'static str| asked.borrow_mut().push(ask);
        let offer = |_: &str| {
            note("offer");
            Some(String::new())
        };
        let asks = Asks {
            kit_first: &|| {
                note("kit_first");
                None
            },
            heard: &|| {
                note("heard");
                true
            },
            held: &|| {
                note("held");
                false
            },
            conflicts: &|| {
                note("conflicts");
                false
            },
            branch: &|| {
                note("branch");
                branch()
            },
            drafted: &|_| {
                note("drafted");
                None
            },
            offer: &offer,
            fell: &nothing,
        };
        assert_eq!(boarding(&asks), Ok(String::new()));
        assert_eq!(
            *asked.borrow(),
            [
                "kit_first",
                "heard",
                "held",
                "conflicts",
                "branch",
                "drafted",
                "offer"
            ]
        );
    }

    #[test]
    fn a_head_kept_off_is_never_offered_and_says_why() {
        let offered = Cell::new(false);
        let offer = |_: &str| {
            offered.set(true);
            Some(String::new())
        };
        let kept: [(Asks, Off); 6] = [
            (
                Asks {
                    kit_first: &|| Some("a reader goes first".to_owned()),
                    ..on(&offer)
                },
                Off::KitFirst("a reader goes first".to_owned()),
            ),
            (
                Asks {
                    heard: &no,
                    ..on(&offer)
                },
                Off::Unheard,
            ),
            (
                Asks {
                    held: &yes,
                    ..on(&offer)
                },
                Off::Held,
            ),
            (
                Asks {
                    conflicts: &yes,
                    ..on(&offer)
                },
                Off::Conflicts,
            ),
            (
                Asks {
                    branch: &String::new,
                    ..on(&offer)
                },
                Off::NoBranch,
            ),
            (
                Asks {
                    drafted: &|_| Some("it's a draft".to_owned()),
                    ..on(&offer)
                },
                Off::Draft("it's a draft".to_owned()),
            ),
        ];
        for (asks, off) in kept {
            assert_eq!(boarding(&asks), Err(off));
        }
        assert!(!offered.get(), "a head kept off was offered");
    }

    #[test]
    fn a_head_the_train_refuses_says_why_it_fell() {
        let refused = |_: &str| None;
        let fell = Asks {
            fell: &|| Some("it fell off the merge train: it conflicts".to_owned()),
            ..on(&refused)
        };
        assert_eq!(
            boarding(&fell),
            Err(Off::Train(Some(
                "it fell off the merge train: it conflicts".to_owned()
            )))
        );
        assert_eq!(boarding(&on(&refused)), Err(Off::Train(None)));
    }

    /// What `behind_said` says of a head behind main, `boarded` so.
    fn said(boarded: &Result<String, Off>) -> String {
        let shared = ["flake.nix".to_owned()];
        behind_said("a".repeat(40).as_str(), "main", "checked", &shared, boarded)
    }

    // On 20 November a head behind main was sent to rebase though the
    // train would have merged it as it stood.
    #[test]
    fn a_head_behind_main_that_boards_isnt_told_to_rebase() {
        let words = said(&Ok("merge-net put it on the merge train".to_owned()));
        assert!(
            words.contains("merge-net put it on the merge train"),
            "{words}"
        );
        assert!(!words.contains("rebase"), "{words}");
    }

    #[test]
    fn a_head_behind_main_kept_off_by_what_a_rebase_doesnt_end_is_told_its_own_step() {
        for (off, step) in [
            (Off::Held, "merge again once it ends"),
            (Off::Draft("mark it ready".to_owned()), "mark it ready"),
            (
                Off::KitFirst("merge again once it has merged".to_owned()),
                "merge again once it has merged",
            ),
            (Off::Unheard, "run merge-net needs and merge again"),
            (Off::NoBranch, "from the pull request's branch"),
        ] {
            let words = said(&Err(off));
            assert!(words.contains(step), "{words}");
            assert!(!words.contains("rebase"), "{words}");
        }
    }

    #[test]
    fn a_head_behind_main_that_cant_board_is_told_why_and_to_rebase() {
        for (off, why) in [
            (Off::Conflicts, "it conflicts with main"),
            (Off::Train(Some("it fell off".to_owned())), "it fell off"),
            (Off::Train(None), "the train wouldn't start"),
        ] {
            let words = said(&Err(off));
            assert!(words.contains(why), "{words}");
            assert!(words.contains("must rebase onto origin/main"), "{words}");
        }
    }
}
