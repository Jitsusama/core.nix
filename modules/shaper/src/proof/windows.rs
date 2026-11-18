//! How a proof's measuring spreads across Joel's idle windows, and how
//! it ends before its verdict counts: each window reading the shaper
//! afresh, a window that couldn't put it back ending the run, an earlier
//! proof's debt paid first, and a shaper left wrong outranking a stop.

use super::stop::tell;
use super::verdict::Block;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};
use terminal_kit::{Exit, Failure};

/// How a proof's windows went.
pub struct Windows {
    /// The pairs kept, in the order measured.
    pub pairs: Vec<[Block; 2]>,
    /// Whether every pair asked for was measured.
    pub complete: bool,
    /// Whether the shaper went back after every window.
    pub back: bool,
    /// How the last window found the shaper, or None when none ran.
    pub found: Option<bool>,
    /// Whether a window was kept from opening because systemd didn't
    /// say whether the shaper was on.
    pub unread: bool,
}

impl Windows {
    /// How the proof ends before its verdict counts, `paid` being how an
    /// earlier proof's debt put the shaper back, if one was owed: a
    /// window's own finding is the later word. 75 when a window was kept
    /// from opening, since the shaper was left as it stood.
    pub fn ending(&self, paid: Option<bool>, stopped: bool) -> Result<(), Failure> {
        put_back_or_stopped(self.back, stopped, self.found.or(paid))?;
        if self.unread {
            return Err(
                Failure::new(Exit::TryAgain, "couldn't read the shaper's state")
                    .because("systemd didn't say whether it was on, so it was left as it stood"),
            );
        }
        Ok(())
    }
}

/// Gathers the pairs across Joel's idle windows, `opened` waiting for the
/// next and `window` measuring from the pair it's given until the gate
/// closes, with the shaper as `shaper_on` reads it as the window opens,
/// and saying whether it put the shaper back. Each window starts after
/// the last pair kept, since a pair's arms run back to back and so
/// compare alike in any window.
pub fn across_windows(
    pairs: u32,
    mut opened: impl FnMut() -> bool,
    mut shaper_on: impl FnMut() -> Option<bool>,
    mut window: impl FnMut(u32, bool) -> (Vec<[Block; 2]>, bool),
) -> Windows {
    let mut done = Windows {
        pairs: Vec::new(),
        complete: false,
        back: true,
        found: None,
        unread: false,
    };
    while done.pairs.len() < pairs as usize {
        // A window that couldn't put the shaper back leaves it for Joel,
        // so no other reads its wrong state as the shaper's own.
        if !done.back || !opened() {
            return done;
        }
        // A state that can't be read is neither on nor off: a window that
        // took it for off would stop a shaper that was on, for good.
        let Some(on) = shaper_on() else {
            tell("systemd didn't say whether the shaper is on: leaving it as it stands");
            done.unread = true;
            return done;
        };
        done.found = Some(on);
        let from = u32::try_from(done.pairs.len()).unwrap_or(pairs);
        let (got, back) = window(from, on);
        done.pairs.extend(got);
        done.back = back;
    }
    done.complete = true;
    done
}

/// Pays an earlier proof's debt, `owed`, with `put_back`: how the shaper
/// was put back, or 69 when it couldn't be.
pub fn pay(
    owed: Option<bool>,
    put_back: impl FnOnce(bool) -> bool,
) -> Result<Option<bool>, Failure> {
    let Some(on) = owed else {
        return Ok(None);
    };
    put_back_or_stopped(put_back(on), false, Some(on))?;
    Ok(Some(on))
}

/// How a proof ends before its verdict counts: 69 when the shaper isn't
/// back as it was `found`, which outranks a stop since it's Joel's to put
/// right, else 130 when `stopped`. `found` is None when no window ran and
/// nothing was owed, the shaper never touched.
fn put_back_or_stopped(back: bool, stopped: bool, found: Option<bool>) -> Result<(), Failure> {
    let state = |on: bool| if on { "on" } else { "off" };
    if !back && let Some(on) = found {
        return Err(
            Failure::new(Exit::Unavailable, "couldn't put the shaper back").because(format!(
                "it should be {}: systemctl {} shaper.service",
                state(on),
                if on { "start" } else { "stop" }
            )),
        );
    }
    if stopped {
        let shaper = found.map_or_else(
            || "no window ran, so the shaper is as it was".to_string(),
            // Said so, not "put back", since a window stopped before any
            // arm moved the shaper left it where it was.
            |on| format!("the shaper is {} as it was", state(on)),
        );
        return Err(Failure::new(Exit::Interrupted, "stopped")
            .because(format!("a signal asked it to; {shaper}, its load ended")));
    }
    Ok(())
}

/// Waits for `open` to say the gate is open, until `deadline` or a stop.
pub fn opened(open: impl Fn() -> bool, deadline: Instant, stopped: &AtomicBool) -> bool {
    while !stopped.load(Ordering::Relaxed) && !open() {
        if Instant::now() >= deadline {
            return false;
        }
        // The gate is asked once a second while it's waited for.
        thread::sleep(Duration::from_secs(1));
    }
    !stopped.load(Ordering::Relaxed)
}

#[cfg(test)]
mod tests {
    use super::super::verdict::tests::pairs;
    use super::*;
    use std::sync::Arc;

    /// Windows that ran with nothing gone wrong, the last finding `found`.
    fn ran(found: Option<bool>) -> Windows {
        Windows {
            pairs: Vec::new(),
            complete: false,
            back: true,
            found,
            unread: false,
        }
    }

    #[test]
    fn a_stop_ends_the_wait_at_once() {
        let stopped = AtomicBool::new(true);
        let deadline = Instant::now() + Duration::from_secs(5);
        assert!(
            !opened(|| false, deadline, &stopped),
            "opened while stopped"
        );
        // A stopped proof finding Joel away still opens no window.
        assert!(!opened(|| true, deadline, &stopped), "opened once stopped");
        // And a stop that comes while it waits.
        let stopped = Arc::new(AtomicBool::new(false));
        let later = Arc::clone(&stopped);
        let raiser = thread::spawn(move || {
            thread::sleep(Duration::from_millis(300));
            later.store(true, Ordering::Relaxed);
        });
        let began = Instant::now();
        assert!(
            !opened(|| false, began + Duration::from_secs(15), &stopped),
            "opened once stopped"
        );
        let _ = raiser.join();
        assert!(
            began.elapsed() < Duration::from_secs(2),
            "a stop on the way waited {:?}",
            began.elapsed()
        );
    }

    #[test]
    fn pairs_measured_in_three_windows_are_kept_as_one_run() {
        let mut froms = Vec::new();
        let mut windows = [5, 0, 7].into_iter();
        let done = across_windows(
            12,
            || true,
            || Some(true),
            |from, _| {
                froms.push(from);
                (pairs(windows.next().unwrap_or(0), 0.8, 0.05), true)
            },
        );
        assert_eq!((done.pairs.len(), done.complete), (12, true));
        assert_eq!(
            froms,
            [0, 5, 5],
            "a window didn't start after the last pair kept"
        );
    }

    #[test]
    fn a_wait_that_runs_out_keeps_the_pairs_measured() {
        let mut opens = [true, false].into_iter();
        let done = across_windows(
            12,
            || opens.next().unwrap_or(false),
            || Some(true),
            |_, _| (pairs(4, 0.8, 0.05), true),
        );
        assert_eq!((done.pairs.len(), done.complete), (4, false));
    }

    #[test]
    fn each_window_reads_the_shaper_afresh() {
        // A switch or Joel turns the shaper off between the two windows.
        let mut states = [true, false].into_iter();
        let mut found = Vec::new();
        let done = across_windows(
            4,
            || true,
            || states.next(),
            |_, on| {
                found.push(on);
                (pairs(2, 0.8, 0.05), true)
            },
        );
        assert_eq!(found, [true, false], "a window took an earlier one's state");
        assert_eq!((done.back, done.found), (true, Some(false)));
    }

    #[test]
    fn a_shaper_whose_state_cant_be_read_is_left_alone() {
        // The first window ran; the second couldn't read the shaper.
        let mut states = [Some(false), None].into_iter();
        let mut windows = 0;
        let done = across_windows(
            12,
            || true,
            || states.next().flatten(),
            |_, _| {
                windows += 1;
                (pairs(2, 0.8, 0.05), true)
            },
        );
        assert_eq!(windows, 1, "a window ran on a state it couldn't read");
        assert_eq!(
            (done.unread, done.complete, done.found),
            (true, false, Some(false))
        );
        assert_eq!(done.ending(None, false).map_err(|f| f.code()), Err(75));
        // A stop still says so first.
        assert_eq!(done.ending(None, true).map_err(|f| f.code()), Err(130));
    }

    #[test]
    fn a_window_that_couldnt_put_the_shaper_back_ends_the_run() {
        let mut opens = 0;
        let done = across_windows(
            12,
            || {
                opens += 1;
                true
            },
            || Some(true),
            |_, _| (pairs(2, 0.8, 0.05), false),
        );
        assert_eq!(opens, 1, "another window read the shaper left wrong");
        assert_eq!(
            (done.pairs.len(), done.complete, done.back),
            (2, false, false)
        );
        assert_eq!(done.ending(None, true).map_err(|f| f.code()), Err(69));
    }

    #[test]
    fn an_earlier_proofs_debt_is_paid_or_said() {
        assert!(
            matches!(pay(None, |_| panic!("paid nothing owed")), Ok(None)),
            "nothing owed"
        );
        let mut asked = None;
        let paid = pay(Some(false), |on| {
            asked = Some(on);
            true
        });
        assert!(matches!(paid, Ok(Some(false))), "a debt paid unsaid");
        assert_eq!(asked, Some(false), "put back otherwise than owed");
        assert_eq!(pay(Some(true), |_| false).map_err(|f| f.code()), Err(69));
        // A stop before any window, after paying, says how this proof left
        // the shaper.
        let stopped = ran(None).ending(Some(true), true).expect_err("a stop");
        assert!(
            stopped.to_string().contains("is on as it was"),
            "a stop after a debt said nothing moved: {stopped}"
        );
        // A window's finding is the later word.
        let stopped = ran(Some(false))
            .ending(Some(true), true)
            .expect_err("a stop");
        assert!(
            stopped.to_string().contains("is off as it was"),
            "a stop said the debt's state over the window's: {stopped}"
        );
    }

    #[test]
    fn a_shaper_left_wrong_outranks_a_stop() {
        let code =
            |back, stopped, found| put_back_or_stopped(back, stopped, found).map_err(|f| f.code());
        assert_eq!(
            code(false, true, Some(true)),
            Err(69),
            "a stop hid a shaper left off"
        );
        assert_eq!(code(true, true, Some(true)), Err(130));
        assert_eq!(code(true, false, Some(true)), Ok(()));
        // Stopped before any window, the shaper was never touched.
        let untouched = ran(None).ending(None, true).expect_err("a stop");
        assert!(
            untouched.to_string().contains("no window ran"),
            "said it put back a shaper it never touched: {untouched}"
        );
        assert!(
            ran(Some(true)).ending(None, false).is_ok(),
            "a clean run failed"
        );
    }
}
