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
    assert_eq!(answer, 42);
    assert!(beats.get() > 0);
    crate::assert_exactly(pacer.rested(), 0.0);
}

#[test]
fn a_hurried_pacer_beats_while_it_waits_too() {
    let (mut pacer, _, beats) = hurried();
    pacer.waiting(|| std::thread::sleep(Duration::from_millis(50)));
    assert!(beats.get() > 0);
}

#[test]
#[should_panic(expected = "the work broke")]
fn work_that_panics_panics_its_waiter() {
    let (mut pacer, _, _) = hurried();
    pacer.waiting(|| panic!("the work broke"));
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
