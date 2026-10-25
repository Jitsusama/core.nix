use disk_guard::config::Pacing;
use disk_guard::pace::{Clock, Pacer, Probe};
use disk_guard::units::Span;
use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

/// A clock that moves only when the pacer sleeps, or when a test moves it.
#[derive(Clone, Default)]
pub struct FakeClock(Rc<Cell<f64>>);

impl FakeClock {
    fn at(&self) -> f64 {
        self.0.get()
    }

    fn pass(&self, seconds: f64) {
        self.0.set(self.0.get() + seconds);
    }
}

impl Clock for FakeClock {
    fn now(&self) -> f64 {
        self.at()
    }

    fn sleep(&mut self, how_long: Duration) {
        self.pass(how_long.as_secs_f64());
    }
}

/// A probe whose syncs take as long as `took` says at the clock's time.
struct FakeProbe {
    clock: FakeClock,
    took: fn(f64) -> Duration,
}

impl Probe for FakeProbe {
    fn sync(&mut self) -> Option<Duration> {
        Some((self.took)(self.clock.at()))
    }
}

const SLOW: Duration = Duration::from_millis(150);
const QUICK: Duration = Duration::from_millis(2);

fn pacing(files: usize) -> Pacing {
    Pacing {
        files,
        rest: Span(Duration::from_secs(30)),
    }
}

/// A paced pacer on a fake clock, feeling the disk as `took` says, and
/// counting its beats.
fn paced(
    files: usize,
    took: Option<fn(f64) -> Duration>,
    pressed: fn(f64) -> bool,
) -> (Pacer, FakeClock, Rc<Cell<u32>>) {
    let clock = FakeClock::default();
    let beats = Rc::new(Cell::new(0));
    let probe = took.map(|took| {
        Box::new(FakeProbe {
            clock: clock.clone(),
            took,
        }) as Box<dyn Probe>
    });
    let counted = Rc::clone(&beats);
    let at = clock.clone();
    let pacer = Pacer::paced(
        pacing(files),
        Box::new(clock.clone()),
        probe,
        Box::new(move || counted.set(counted.get() + 1)),
        Box::new(move || pressed(at.at())),
    );
    (pacer, clock, beats)
}

fn never(_: f64) -> bool {
    false
}

#[test]
fn a_paced_pass_rests_after_each_batch_of_files() {
    let (mut pacer, clock, _) = paced(3, Some(|_| QUICK), never);
    for _ in 0..7 {
        pacer.unlinked();
    }
    // Two batches of three, and a rest after each.
    crate::assert_exactly(clock.at(), 60.0);
    // And one asked for.
    pacer.rest();
    crate::assert_exactly(clock.at(), 90.0);
    crate::assert_exactly(pacer.rested(), 90.0);
}

#[test]
fn a_rest_lasts_while_the_disk_is_strained_until_two_minutes_pass_without_a_slow_sync() {
    // Slow for the first hundred seconds, as a commit of a big delete is.
    let (mut pacer, clock, _) = paced(1, Some(|at| if at <= 100.0 { SLOW } else { QUICK }), never);
    pacer.rest();
    // The last slow sync at 100, then two calm minutes.
    crate::assert_exactly(clock.at(), 221.0);
}

#[test]
fn three_slow_syncs_in_two_minutes_dont_strain_the_disk() {
    let (mut pacer, clock, _) = paced(
        1,
        Some(|at| {
            if [5.0, 10.0, 15.0].contains(&at) {
                SLOW
            } else {
                QUICK
            }
        }),
        never,
    );
    pacer.rest();
    crate::assert_exactly(clock.at(), 30.0);
}

#[test]
fn a_pacer_with_no_probe_rests_as_its_pacing_says() {
    let (mut pacer, clock, _) = paced(1, None, never);
    pacer.rest();
    crate::assert_exactly(clock.at(), 30.0);
}

#[test]
fn a_disk_fallen_to_urgent_ends_the_rest_and_every_one_after_it() {
    let (mut pacer, clock, _) = paced(2, Some(|_| SLOW), |at| at >= 10.0);
    // Strained, but urgent can't wait.
    pacer.rest();
    crate::assert_exactly(clock.at(), 10.0);
    // And it never rests again.
    for _ in 0..10 {
        pacer.unlinked();
    }
    crate::assert_exactly(clock.at(), 10.0);
}

/// A stop waits, blocked, for the guard to take it between passes, and
/// systemd kills a guard that keeps it waiting, so a rest ends for it
/// however strained the disk, and none follows. The stop is this thread's
/// alone, so no other test sees it, and it's taken back before the end.
#[cfg(target_os = "linux")]
#[test]
fn a_stop_ends_the_rest_and_every_one_after_it() {
    let (rested, after) = std::thread::spawn(|| {
        // Strained for five minutes, as a repack strains it, so a rest that
        // ignored the stop would end only after them.
        let (mut pacer, clock, _) =
            paced(2, Some(|at| if at <= 300.0 { SLOW } else { QUICK }), never);
        // SAFETY: set is a valid sigset_t; the signal goes to this thread,
        // which blocks it, and is taken back before the thread ends.
        unsafe {
            let mut set = std::mem::zeroed::<libc::sigset_t>();
            libc::sigemptyset(&raw mut set);
            libc::sigaddset(&raw mut set, libc::SIGTERM);
            libc::pthread_sigmask(libc::SIG_BLOCK, &raw const set, std::ptr::null_mut());
            libc::pthread_kill(libc::pthread_self(), libc::SIGTERM);
            pacer.rest();
            let rested = clock.at();
            for _ in 0..10 {
                pacer.unlinked();
            }
            let after = clock.at();
            let mut taken = 0;
            libc::sigwait(&raw const set, &raw mut taken);
            (rested, after)
        }
    })
    .join()
    .unwrap();
    assert!(rested < 2.0, "rested {rested} s with a stop waiting");
    crate::assert_exactly(after, rested);
}

#[test]
fn a_resting_pacer_beats_every_ten_seconds() {
    let (mut pacer, _, beats) = paced(1, Some(|_| QUICK), never);
    pacer.rest();
    assert_eq!(beats.get(), 3);
}

/// A hurried pacer on a fake clock, counting its beats.
pub fn hurried() -> (Pacer, FakeClock, Rc<Cell<u32>>) {
    let clock = FakeClock::default();
    let beats = Rc::new(Cell::new(0));
    let counted = Rc::clone(&beats);
    let pacer = Pacer::hurried(
        Box::new(clock.clone()),
        Box::new(move || counted.set(counted.get() + 1)),
    );
    (pacer, clock, beats)
}

#[test]
fn a_pacer_beats_while_it_waits_on_work_and_hands_back_its_answer() {
    // The fake clock moves a tenth of a second at each look, which is
    // far quicker than the work's real fiftieth, so it beats many times.
    let (mut pacer, _, beats) = paced(2_000, None, never);
    let answer = pacer.waiting(|| {
        std::thread::sleep(Duration::from_millis(50));
        42
    });
    assert_eq!(answer, Ok(42));
    assert!(beats.get() > 0);
    crate::assert_exactly(pacer.rested(), 0.0);
}

#[test]
fn a_hurried_pacer_beats_while_it_waits_too() {
    let (mut pacer, _, beats) = hurried();
    let waited = pacer.waiting(|| std::thread::sleep(Duration::from_millis(50)));
    assert_eq!(waited, Ok(()));
    assert!(beats.get() > 0);
}

/// A stop that comes while the guard waits on work, an answer from the
/// system daemon or a commit, ends the wait at once, and the work is left
/// to end with the guard. The work here waits on a channel for up to five
/// seconds, sent to only once the wait is over, so a wait that ignored the
/// stop ends late rather than never. The stop is this thread's alone, as
/// in the rest's own test, and taken back before the end.
#[cfg(target_os = "linux")]
#[test]
fn a_stop_ends_a_wait_on_work_that_has_not_ended() {
    use disk_guard::pace::Stopped;

    let (waited, ended_first) = std::thread::spawn(|| {
        let (mut pacer, _, _) = hurried();
        let (release, wait) = std::sync::mpsc::channel::<()>();
        // SAFETY: set is a valid sigset_t; the signal goes to this thread,
        // which blocks it, and is taken back before the thread ends.
        unsafe {
            let mut set = std::mem::zeroed::<libc::sigset_t>();
            libc::sigemptyset(&raw mut set);
            libc::sigaddset(&raw mut set, libc::SIGTERM);
            libc::pthread_sigmask(libc::SIG_BLOCK, &raw const set, std::ptr::null_mut());
            libc::pthread_kill(libc::pthread_self(), libc::SIGTERM);
            let waited = pacer.waiting(move || wait.recv_timeout(Duration::from_secs(5)).is_ok());
            // The work still waits on the channel, so a send reaches it.
            let ended_first = release.send(()).is_err();
            let mut taken = 0;
            libc::sigwait(&raw const set, &raw mut taken);
            (waited, ended_first)
        }
    })
    .join()
    .unwrap();
    assert_eq!(waited, Err(Stopped), "the wait went on past a stop");
    assert!(!ended_first, "the work ended before the wait did");
}

#[test]
#[should_panic(expected = "the work broke")]
fn work_that_panics_panics_its_waiter() {
    let (mut pacer, _, _) = hurried();
    let _ = pacer.waiting(|| panic!("the work broke"));
}

#[test]
fn a_hurried_pacer_never_rests_but_keeps_beating() {
    let (mut pacer, clock, beats) = hurried();
    for _ in 0..30 {
        pacer.unlinked();
        clock.pass(1.0);
    }
    pacer.rest();
    // Only the test moved the clock.
    crate::assert_exactly(clock.at(), 30.0);
    assert_eq!(beats.get(), 2, "at ten and twenty seconds");
    crate::assert_exactly(pacer.rested(), 0.0);
}
