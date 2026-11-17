//! The train's right of way over changes that would merge alone. On 16
//! November two riders ran five times from 20:27, 22 to 30 minutes each,
//! and every time main moved beside them before the run ended, as changes
//! merged alone about every ten minutes: nothing landed off the train for
//! two hours. Once main moves beside a candidate that passed, the train
//! holds the right of way, `train/right-of-way` saying since when, and a
//! change `ready` would merge alone boards it instead, so main stays where
//! the train's next run is cut. A landing that leaves riders aboard, those
//! that boarded meanwhile, holds it afresh for their run. It ends when the
//! train goes red, can't reach main, lands the last of its riders or stops
//! for any reason, and `RIGHT_OF_WAY` after it began whatever happens: a
//! train that keeps missing gets it again only once it lands, so one that
//! can't land never holds main back for long.

use std::fs;
use std::path::{Path, PathBuf};

/// How long the train holds the right of way it was given: longer than
/// the 22 to 30 minutes its runs took on 16 November, short enough that
/// one that never lands keeps changes from merging alone for less than
/// an hour.
pub const RIGHT_OF_WAY: u64 = 45 * 60;

/// How a run of the train ended, as the right of way reads it.
#[derive(Debug, PartialEq, Eq)]
pub enum Ended {
    /// Main moved beside a candidate that passed.
    Moved,
    /// It landed, and riders that boarded meanwhile are still aboard, or
    /// none are.
    Landed { left: bool },
    /// It failed, main refused it, or the train stopped.
    Otherwise,
}

/// What the end of a run does to the right of way.
#[derive(Debug, PartialEq, Eq)]
pub enum Way {
    /// The train holds it from now, unless it already held one since it
    /// last landed.
    Give,
    /// The train holds it afresh from now, for the riders still aboard.
    Renew,
    /// Changes merge alone again.
    TakeBack,
}

/// What a run that `ended` so does to the right of way.
#[must_use]
pub fn after(_ended: &Ended) -> Way {
    Way::TakeBack
}

/// Do to the right of way of the train under `root`, at `now`, what a
/// run that `ended` so does, saying what that was for the train's log.
pub fn follow(root: &Path, ended: &Ended, now: u64) -> Option<String> {
    match after(ended) {
        Way::Give if fs::metadata(file(root)).is_err() => {
            give(root, now);
            Some(format!(
                "the train holds the right of way for {} minutes, so changes that would \
                 merge alone board it",
                RIGHT_OF_WAY / 60
            ))
        }
        Way::Give => None,
        Way::Renew => {
            take_back(root);
            give(root, now);
            Some("the train holds the right of way afresh for the riders still aboard".into())
        }
        Way::TakeBack if fs::metadata(file(root)).is_ok() => {
            take_back(root);
            Some("the train gives back the right of way".into())
        }
        Way::TakeBack => None,
    }
}

/// What `ready` says of the change at `head`, which would merge alone,
/// while the train holds the right of way: that it boards the train
/// instead, so main stays where the train's run is cut; nothing while the
/// train holds none, or can't take the change, or the change waits on a
/// run of its own, as `ready` boards none then either.
#[must_use]
pub fn gives_way(head: &str) -> Option<String> {
    let root = crate::state::State::from_env().root;
    let now = crate::state::now();
    let since = holds(&root, now)?;
    if crate::turn::waits_to_run(head).is_some() {
        return None;
    }
    let branch = crate::git(&["symbolic-ref", "--quiet", "--short", "HEAD"]).ok()?;
    let boarded = super::offer(head, (!branch.is_empty()).then_some(branch)?.as_str())?;
    Some(format!(
        "the merge train has held the right of way for {} minutes, since main moved beside \
         a run of it that passed, so {} rides it rather than merge alone and move main \
         again: {boarded}",
        now.saturating_sub(since) / 60,
        crate::state::short(head)
    ))
}

fn file(root: &Path) -> PathBuf {
    super::folder(root).join("right-of-way")
}

/// Give the train under `root` the right of way at `now`, keeping when it
/// began while it holds one, and leaving one whose time is up ended until
/// it's taken back.
pub fn give(root: &Path, now: u64) {
    if holds(root, now).is_none() {
        let _ = fs::write(file(root), format!("{now}\n"));
    }
}

/// Take the right of way back from the train under `root`.
pub fn take_back(root: &Path) {
    let _ = fs::remove_file(file(root));
}

/// Since when the train under `root` has held the right of way, at `now`,
/// if it still does: one said to begin after `now`, as a clock stepped
/// back would make it, isn't held.
#[must_use]
pub fn holds(root: &Path, now: u64) -> Option<u64> {
    let since: u64 = fs::read_to_string(file(root)).ok()?.trim().parse().ok()?;
    (now.saturating_sub(since) < RIGHT_OF_WAY).then_some(since)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root(name: &str) -> PathBuf {
        let root =
            std::env::temp_dir().join(format!("merge-net-way-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(super::super::folder(&root)).unwrap();
        root
    }

    #[test]
    fn a_train_given_the_right_of_way_holds_it_from_then() {
        let root = root("given");
        assert_eq!(
            holds(&root, 100),
            None,
            "a train given nothing held the right of way"
        );
        give(&root, 100);
        assert_eq!(
            holds(&root, 160),
            Some(100),
            "the train given the right of way didn't hold it"
        );
        give(&root, 200);
        assert_eq!(
            holds(&root, 260),
            Some(100),
            "giving it again moved when the right of way began"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn the_right_of_way_ends_when_taken_back_or_when_its_time_is_up() {
        let root = root("ends");
        give(&root, 100);
        take_back(&root);
        assert_eq!(
            holds(&root, 160),
            None,
            "a right of way taken back still held"
        );
        give(&root, 100);
        assert_eq!(
            holds(&root, 100 + RIGHT_OF_WAY - 1),
            Some(100),
            "the right of way ended before its time"
        );
        assert_eq!(
            holds(&root, 100 + RIGHT_OF_WAY),
            None,
            "the right of way outlasted its time"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_right_of_way_said_to_begin_later_than_now_isnt_held() {
        let root = root("later");
        give(&root, 1000);
        assert_eq!(
            holds(&root, 400),
            None,
            "a right of way from a clock stepped back held, past its time"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_train_that_keeps_missing_gets_the_right_of_way_again_only_once_it_lands() {
        let root = root("missing");
        let up = 100 + RIGHT_OF_WAY;
        assert!(
            follow(&root, &Ended::Moved, 100).is_some(),
            "giving it wasn't said"
        );
        assert!(
            follow(&root, &Ended::Moved, up).is_none(),
            "a miss after its time said more"
        );
        assert_eq!(
            holds(&root, up),
            None,
            "a train that kept missing held the right of way past its time"
        );
        follow(&root, &Ended::Landed { left: true }, up + 10);
        assert_eq!(
            holds(&root, up + 20),
            Some(up + 10),
            "a landing that left riders aboard didn't hold it afresh for them"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_run_gives_renews_or_takes_back_the_right_of_way_by_how_it_ended() {
        for (ended, way) in [
            (Ended::Moved, Way::Give),
            (Ended::Landed { left: true }, Way::Renew),
            (Ended::Landed { left: false }, Way::TakeBack),
            (Ended::Otherwise, Way::TakeBack),
        ] {
            assert_eq!(
                after(&ended),
                way,
                "a run that ended {ended:?} did the wrong thing"
            );
        }
    }
}
