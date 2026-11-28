//! The hourly marks walking a cargo root again only once it changed since
//! the last walk of it, the survey's or their own, or that walk is six
//! hours old. On 27 November three hourly marking walks read about 12.7 GB
//! each, walking the roots the survey had just walked.

use crate::daemon::{CALM, Home, tick, walks, wall};
use disk_guard::clock::Tick;
use disk_guard::daemon::Guard;
use disk_guard::marks::WALK_AGAIN_AFTER;
use disk_guard::opens::Watch;
use disk_guard::sizes::SETTLED;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

/// A tick `seconds` on from now.
fn after(seconds: f64) -> Tick {
    Tick {
        wall: wall() + seconds,
        ..tick()
    }
}

/// A guard that has marked the build `deps` holds once, by a walk of its
/// root journaled as such, and its home.
struct Marked {
    home: Home,
    guard: Guard,
    deps: PathBuf,
}

/// A home holding one build under its cargo root, settled, so a walk of
/// the root keeps a stamp that can match the next; nothing when there's
/// no fanotify here.
fn settled_build() -> Option<(Watch, Home, Guard, PathBuf)> {
    let Ok(watch) = Watch::new() else {
        eprintln!("no fanotify here, so nothing to check");
        return None;
    };
    let home = Home::new();
    let (config, deps) = home.replaced_build("0%");
    // Past the time a stamp must settle for, so the walk keeps one.
    std::thread::sleep(std::time::Duration::from_secs_f64(2.0 * SETTLED));
    let guard = home.guard(config);
    Some((watch, home, guard, deps))
}

/// A guard listening on a settled build, after the step that first marked
/// it.
fn marked() -> Option<Marked> {
    let (watch, home, mut guard, deps) = settled_build()?;
    guard.listen(Arc::new(watch));
    guard.step(&tick(), &CALM).unwrap();
    let walked = walks(&home, "marks");
    assert_eq!(
        walked.len(),
        1,
        "the first marks didn't walk the root once: {walked:?}"
    );
    Some(Marked { home, guard, deps })
}

/// Whether the guard hears the profile and `deps` of the build `deps`
/// names, as its marks on them stand now.
fn hears_build(guard: &Guard, deps: &std::path::Path) -> bool {
    let hears = guard.hears();
    hears.contains(&deps.to_path_buf()) && hears.contains(&deps.parent().unwrap().to_path_buf())
}

#[test]
fn an_hour_on_an_unchanged_root_is_marked_again_without_a_walk() {
    let Some(Marked {
        home,
        mut guard,
        deps,
    }) = marked()
    else {
        return;
    };
    // Cleaned and built again three levels below the root, where its stamp
    // doesn't read: only marking every target kept puts the mark back.
    fs::remove_dir_all(&deps).unwrap();
    home.replaced_crate("app");
    guard.step(&after(3601.0), &CALM).unwrap();
    let walked = walks(&home, "marks");
    assert_eq!(
        walked.len(),
        1,
        "an hour on, the marks walked a root nothing near its top changed in: {walked:?}"
    );
    assert!(
        hears_build(&guard, &deps),
        "the marks kept didn't mark again a build made anew: {:?}",
        guard.hears()
    );
}

#[test]
fn a_target_made_in_a_changed_root_is_walked_and_marked_at_the_next_marks() {
    let Some(Marked {
        home, mut guard, ..
    }) = marked()
    else {
        return;
    };
    let other = home.replaced_crate("other");
    guard.step(&after(3601.0), &CALM).unwrap();
    let walked = walks(&home, "marks");
    assert_eq!(
        walked.len(),
        2,
        "a root a checkout was made in wasn't walked at the next marks: {walked:?}"
    );
    assert!(
        hears_build(&guard, &other),
        "the target made in the changed root went unmarked: {:?}",
        guard.hears()
    );
}

#[test]
fn a_root_walked_six_hours_ago_is_walked_again_though_unchanged() {
    let Some(Marked {
        home,
        mut guard,
        deps,
    }) = marked()
    else {
        return;
    };
    guard.step(&after(WALK_AGAIN_AFTER + 1.0), &CALM).unwrap();
    let walked = walks(&home, "marks");
    assert_eq!(
        walked.len(),
        2,
        "a root last walked six hours before wasn't walked again: {walked:?}"
    );
    assert!(
        hears_build(&guard, &deps),
        "the build went unmarked after the walk: {:?}",
        guard.hears()
    );
}

#[test]
fn a_survey_s_finished_walk_spares_the_next_marks_their_walk() {
    let Some((watch, home, mut guard, deps)) = settled_build() else {
        return;
    };
    // Not yet listening, so only the survey walks.
    guard.step(&tick(), &CALM).unwrap();
    guard.listen(Arc::new(watch));
    guard.step(&after(1.0), &CALM).unwrap();
    let walked = walks(&home, "marks");
    assert!(
        walked.is_empty(),
        "the marks walked a root the survey before had walked: {walked:?}"
    );
    assert!(
        hears_build(&guard, &deps),
        "the targets the survey found weren't marked: {:?}",
        guard.hears()
    );
}

#[test]
fn a_survey_a_stop_cut_short_spares_the_marks_nothing() {
    let Some((watch, home, mut guard, deps)) = settled_build() else {
        return;
    };
    // The survey asks once before the source, and its walk once for each
    // folder it reads, so the third ask stops the walk after the root.
    let asked = std::rc::Rc::new(std::cell::Cell::new(0));
    let asks = std::rc::Rc::clone(&asked);
    guard.stop_when(Box::new(move || {
        asks.set(asks.get() + 1);
        asks.get() >= 3
    }));
    let stepped = guard.step(&tick(), &CALM);
    assert_eq!(
        stepped.map_err(|error| error.kind()).err(),
        Some(std::io::ErrorKind::Interrupted),
        "the stop didn't cut the survey short"
    );
    assert!(
        asked.get() >= 3,
        "the survey never walked into the root, asked {} times",
        asked.get()
    );
    guard.stop_when(Box::new(|| false));
    guard.listen(Arc::new(watch));
    guard.step(&after(1.0), &CALM).unwrap();
    let walked = walks(&home, "marks");
    assert_eq!(
        walked.len(),
        1,
        "the marks trusted a walk a stop cut short: {walked:?}"
    );
    assert!(
        hears_build(&guard, &deps),
        "the build went unmarked: {:?}",
        guard.hears()
    );
}
