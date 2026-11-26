//! The train's right of way over changes that would merge alone. On 16
//! November two riders ran five times from 20:27, 22 to 30 minutes each,
//! and every time main moved beside them before the run ended, as changes
//! merged alone about every ten minutes: nothing landed off the train for
//! two hours. Once main moves beside a candidate that passed, the train
//! holds the right of way, `train/right-of-way` saying since when, and a
//! change `ready` would merge alone boards it instead, so main stays where
//! the train's next run is cut. A landing that leaves riders aboard, those
//! that boarded meanwhile, holds on to it for their run, from when it
//! began. It ends when the train goes red, can't reach main, lands the
//! last of its riders or stops for any reason, a train starting afresh
//! holding none, and `RIGHT_OF_WAY` after it began whatever happens: it's
//! given again only by a miss after a landing, so neither a train that
//! can't land nor one that lands under steady traffic holds main back for
//! longer than that at a time.
//!
//! Apart from the right of way, a train running a candidate with riders
//! aboard takes a change `ready` would merge alone too (`yields`), since
//! that merge would move main beside its run. It does so only while the
//! run is clear, searching no red and waiting on no builder, as
//! `train/runs` says from when it began, and for `RIGHT_OF_WAY` of the
//! run at most: a change boarding a red's search could fall for a red not
//! its own, and a run that hangs lets changes merge alone again once that
//! time is up. A run that passed holds main through its landing too,
//! however long the run took and whether or not it was clear, as `train/runs`
//! then says it lands. Each run begins its time afresh, so a train that keeps
//! running clear holds main for as long as it keeps running; that's what
//! keeps main still under it, since a miss happens only when main moves.
//! Beside either, a change still merges alone when the train can't take
//! it (`gives_way`), as before.

use std::fs;
use std::path::{Path, PathBuf};

/// How long the train holds the right of way it was given, and how long
/// one clear run of it holds main: longer than the 22 to 30 minutes its
/// runs took on 16 November, short enough that one that never lands, or
/// a run that hangs, keeps changes from merging alone for less than an
/// hour at a time.
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
    /// The train holds on to it from when it began, for the riders still
    /// aboard, and it's taken back once its time is up.
    Keep,
    /// Changes merge alone again.
    TakeBack,
}

/// What a run that `ended` so does to the right of way.
#[must_use]
pub fn after(ended: &Ended) -> Way {
    match ended {
        Ended::Moved => Way::Give,
        Ended::Landed { left: true } => Way::Keep,
        Ended::Landed { left: false } | Ended::Otherwise => Way::TakeBack,
    }
}

/// Do to the right of way of the train under `root`, at `now`, what a
/// run that `ended` so does, saying what that was for the train's log.
pub fn follow(root: &Path, ended: &Ended, now: u64) -> Option<String> {
    let held = fs::metadata(file(root)).is_ok();
    match after(ended) {
        Way::Give if !held => {
            give(root, now);
            Some(format!(
                "the train holds the right of way for {} minutes, so changes that would \
                 merge alone board it",
                RIGHT_OF_WAY / 60
            ))
        }
        Way::Keep if holds(root, now).is_some() => {
            Some("the train holds on to the right of way for the riders still aboard".into())
        }
        Way::TakeBack | Way::Keep if held => {
            take_back(root);
            Some("the train gives back the right of way".into())
        }
        // Given while held, or taken back while not, nothing changes.
        Way::Give | Way::TakeBack | Way::Keep => None,
    }
}

/// Why a change that would merge alone boards the train instead.
#[derive(Debug, PartialEq, Eq)]
pub enum Yields {
    /// The train has held the right of way since then.
    Held(u64),
    /// The train runs a clear candidate with this many riders aboard.
    Runs(usize),
}

/// Whether a change that would merge alone boards the train instead, by
/// the right of way the train holds since `held`, the clear run it began
/// at `runs` and the `aboard` riders: while a train runs with riders, a
/// change merged alone moves main beside its run, and once main moves
/// files a rider reads the train runs again. On 25 November the right of
/// way given at 05:19 ran out at 06:04 while the train still ran, so
/// #6593 merged alone at 07:31 and the train, 18 riders aboard, ran again
/// at 4b0fe54b742f. Velocity's ruling of that morning already had no
/// change merge alone while a train ran; this makes `ready` hold to it.
#[must_use]
pub fn yields(held: Option<u64>, runs: Option<u64>, aboard: usize) -> Option<Yields> {
    match (held, runs) {
        (Some(since), _) => Some(Yields::Held(since)),
        (None, Some(_)) if aboard > 0 => Some(Yields::Runs(aboard)),
        (None, _) => None,
    }
}

/// Why a change that would merge alone boards the train under `root`
/// instead, at `now`, if it does, as its files say and whether its unit
/// is `going`: a run marked by a train that was killed isn't running.
#[must_use]
pub fn why(root: &Path, now: u64, going: bool) -> Option<Yields> {
    let runs = runs(root, now).filter(|_| going);
    yields(holds(root, now), runs, super::riders(root).len())
}

/// What `ready` says of the change at `head`, which would merge alone,
/// while the train holds the right of way or runs with riders aboard:
/// that it boards the train instead, so main stays where the train's run
/// is cut; nothing while the train neither holds the way nor runs, or
/// can't take the change, or the change waits on a run of its own, as
/// `ready` boards none then either.
#[must_use]
pub fn gives_way(head: &str) -> Option<String> {
    let root = crate::state::State::from_env().root;
    let now = crate::state::now();
    let why = why(&root, now, super::going())?;
    if crate::turn::waits_to_run(head).is_some() {
        return None;
    }
    let branch = crate::git(&["symbolic-ref", "--quiet", "--short", "HEAD"]).ok()?;
    let boarded = super::offer(head, (!branch.is_empty()).then_some(branch)?.as_str())?;
    let since = match why {
        Yields::Held(since) => format!(
            "has held the right of way for {} minutes, since main moved beside a run of it \
             that passed",
            now.saturating_sub(since) / 60
        ),
        Yields::Runs(1) => "runs with 1 rider aboard".to_owned(),
        Yields::Runs(aboard) => format!("runs with {aboard} riders aboard"),
    };
    Some(format!(
        "the merge train {since}, so {} rides it rather than merge alone and move main \
         beside it: {boarded}",
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

fn runs_file(root: &Path) -> PathBuf {
    super::folder(root).join("runs")
}

/// Say that the train under `root` began a clear run at `began`, whole,
/// since a `ready` reading a file half written would find no run and let
/// its change merge alone beside it.
pub fn run_begins(root: &Path, began: u64) {
    let _ = crate::whole::write(&runs_file(root), &format!("{began}\n"));
}

/// Say that the train under `root`, its run begun at `began`, lands it,
/// whole, whether or not the run was clear: a change merged alone between
/// the train's merges leaves main a tree the train never tested, and on
/// 25 November that reverted all three of a run's merges. A landing is
/// held however long the run took before it, since a run past
/// `RIGHT_OF_WAY` lands as surely as a quick one; a train that was killed
/// holds nothing, as `why` asks whether its unit is going.
pub fn landing_begins(root: &Path, began: u64) {
    let _ = crate::whole::write(&runs_file(root), &format!("{began} {LANDS}\n"));
}

/// What `train/runs` adds after a run's start while the run lands.
const LANDS: &str = "lands";

/// Say that the train under `root` runs nothing clear, and lands nothing.
pub fn run_ends(root: &Path) {
    let _ = fs::remove_file(runs_file(root));
}

/// Since when the train under `root` has run a clear candidate, at `now`,
/// if it began less than `RIGHT_OF_WAY` ago or lands it: one said to
/// begin after `now` isn't running, as for the right of way.
#[must_use]
pub fn runs(root: &Path, now: u64) -> Option<u64> {
    let said = fs::read_to_string(runs_file(root)).ok()?;
    let mut words = said.split_whitespace();
    let began: u64 = words.next()?.parse().ok()?;
    let lands = words.next() == Some(LANDS);
    let running = now.checked_sub(began)?;
    (lands || running < RIGHT_OF_WAY).then_some(began)
}

/// Since when the train under `root` has held the right of way, at `now`,
/// if it still does: one said to begin after `now`, as a clock stepped
/// back would make it, isn't held.
#[must_use]
pub fn holds(root: &Path, now: u64) -> Option<u64> {
    let since: u64 = fs::read_to_string(file(root)).ok()?.trim().parse().ok()?;
    let held = now.checked_sub(since)?;
    (held < RIGHT_OF_WAY).then_some(since)
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
            "a right of way said to begin later than now held"
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
        assert!(
            follow(&root, &Ended::Moved, up + 20).is_some(),
            "a miss after the train landed didn't give it the right of way again"
        );
        assert_eq!(
            holds(&root, up + 30),
            Some(up + 20),
            "the right of way given again didn't begin at the miss"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_landing_that_leaves_riders_aboard_holds_on_from_when_it_began() {
        let root = root("keeps");
        follow(&root, &Ended::Moved, 100);
        follow(&root, &Ended::Landed { left: true }, 200);
        assert_eq!(
            holds(&root, 300),
            Some(100),
            "a landing began the right of way afresh, so traffic could keep it for ever"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_run_that_ends_otherwise_gives_the_right_of_way_back() {
        let root = root("back");
        follow(&root, &Ended::Moved, 100);
        assert!(
            follow(&root, &Ended::Otherwise, 150).is_some(),
            "giving it back wasn't said"
        );
        assert!(
            !file(&root).exists(),
            "the right of way given back was left on disk"
        );
        assert!(
            follow(&root, &Ended::Otherwise, 160).is_none(),
            "giving back one not held said it was given back"
        );
        let _ = fs::remove_dir_all(&root);
    }

    fn aboard(root: &Path, riders: usize) {
        for rider in 0..riders {
            let rider = super::super::Rider {
                head: format!("{rider:040}"),
                branch: format!("rider-{rider}"),
                at: 100,
            };
            super::super::board(root, &rider).unwrap();
        }
    }

    #[test]
    fn a_change_boards_a_clear_run_after_the_right_of_way_ran_out() {
        // As on 25 November: the right of way began at 05:19 and ran out
        // at 06:04, and the train began another clear run, 18 aboard.
        let root = root("runs");
        give(&root, 100);
        let now = 100 + RIGHT_OF_WAY + 600;
        run_begins(&root, now - 300);
        aboard(&root, 18);
        assert_eq!(
            why(&root, now, true),
            Some(Yields::Runs(18)),
            "a change would merge alone beside a clear run with riders, once the right of way \
             ran out"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_landing_holds_main_however_long_its_run_took() {
        // A run past its time still lands, and a change merged alone
        // between its merges leaves main a tree nobody tested.
        let root = root("lands");
        let now = 100 + RIGHT_OF_WAY + 600;
        landing_begins(&root, 100);
        aboard(&root, 3);
        assert_eq!(
            why(&root, now, true),
            Some(Yields::Runs(3)),
            "a change would merge alone beside a landing whose run outlasted the right of way"
        );
        assert_eq!(
            why(&root, now, false),
            None,
            "a landing marked by a train that was killed held main"
        );
        run_ends(&root);
        assert_eq!(
            why(&root, now, true),
            None,
            "a landing that ended still held main"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_run_past_its_time_that_isnt_landing_holds_main_no_longer() {
        let root = root("past");
        run_begins(&root, 100);
        aboard(&root, 3);
        assert_eq!(
            why(&root, 100 + RIGHT_OF_WAY, true),
            None,
            "a clear run that ran out its time, not landing, still held main"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_change_merges_alone_beside_a_run_that_isnt_clear_running_or_ridden() {
        let root = root("alone");
        aboard(&root, 3);
        assert_eq!(
            why(&root, 1000, true),
            None,
            "a change would board a red's search or an outage, which no clear run marks"
        );
        run_begins(&root, 1000);
        assert_eq!(
            why(&root, 1000, false),
            None,
            "a change would board a run a train that was killed marked"
        );
        assert_eq!(
            why(&root, 1000 + RIGHT_OF_WAY, true),
            None,
            "a change would board a run that hung past its time"
        );
        run_ends(&root);
        assert_eq!(
            why(&root, 1000, true),
            None,
            "a change would board a run that ended"
        );
        let empty = self::root("empty");
        run_begins(&empty, 1000);
        assert_eq!(
            why(&empty, 1000, true),
            None,
            "a change would board a run with no one aboard"
        );
        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_dir_all(&empty);
    }

    #[test]
    fn the_right_of_way_is_said_as_why_even_with_no_run() {
        let root = root("held");
        give(&root, 100);
        assert_eq!(
            why(&root, 200, false),
            Some(Yields::Held(100)),
            "a change would merge alone while the train held the right of way"
        );
        run_begins(&root, 150);
        aboard(&root, 2);
        assert_eq!(
            why(&root, 200, true),
            Some(Yields::Held(100)),
            "a clear run was said as why while the train held the right of way"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_run_gives_keeps_or_takes_back_the_right_of_way_by_how_it_ended() {
        for (ended, way) in [
            (Ended::Moved, Way::Give),
            (Ended::Landed { left: true }, Way::Keep),
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
