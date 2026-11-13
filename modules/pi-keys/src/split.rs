//! What a key waited on, split by the install of pi's packages each
//! window ran, and by the window itself.
//!
//! One sample pools every pi window Joel was at, so a rise between
//! samples can't say whether the machine's load, a newer install or one
//! window's session moved it. Sessions load their packages when they
//! start and keep them until they reload, so within one sample windows
//! run different installs under the same load, and splitting the keys by
//! install compares them fairly. A long session can slow its own window
//! whatever it runs, so the keys split by window too, each with its
//! install, which says whether one window held the sample's slow keys.

use crate::model::{self, Path};
use std::collections::BTreeMap;

/// A stretch Joel spent at a pi window, in the monotonic clock's
/// milliseconds, the window and the pi process it ran.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Held {
    pub from: f64,
    pub to: f64,
    pub window: u64,
    pub pi: u32,
}

/// What keys waited on over the stretches that share one key.
#[derive(Debug, Clone, PartialEq)]
pub struct Part<K> {
    pub of: K,
    pub seconds: f64,
    pub summary: model::Summary,
}

/// The packages a pi process loaded, by name and version.
pub type Packages = Vec<String>;

/// The keys of each install, with `installs` naming what each pi process
/// loaded.
pub fn installs(
    path: &Path,
    held: &[Held],
    installs: impl Fn(u32) -> Packages,
    frame: f64,
    keys: usize,
    seed: u64,
) -> Vec<Part<Packages>> {
    parts(path, held, |h| installs(h.pi), frame, keys, seed)
}

/// The keys of each window, with the install its pi loaded.
pub fn windows(
    path: &Path,
    held: &[Held],
    installs: impl Fn(u32) -> Packages,
    frame: f64,
    keys: usize,
    seed: u64,
) -> Vec<Part<(u64, Packages)>> {
    let _ = (path, held, installs, frame, keys, seed);
    Vec::new()
}

/// The keys of the stretches grouped by `key`, each group modelled over
/// its own stretches, in the order of their keys.
fn parts<K: Ord>(
    path: &Path,
    held: &[Held],
    key: impl Fn(&Held) -> K,
    frame: f64,
    keys: usize,
    seed: u64,
) -> Vec<Part<K>> {
    let mut by: BTreeMap<K, Vec<(f64, f64)>> = BTreeMap::new();
    for stretch in held {
        by.entry(key(stretch))
            .or_default()
            .push((stretch.from, stretch.to));
    }
    by.into_iter()
        .filter_map(|(of, stretches)| {
            let modelled = model::keys(path, &stretches, frame, keys, &mut model::Draw::new(seed));
            let summary = model::summary(&modelled)?;
            let seconds = stretches.iter().map(|(a, b)| b - a).sum::<f64>() / 1000.0;
            Some(Part {
                of,
                seconds,
                summary,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const FRAME: f64 = 16.67;

    fn held(from: f64, to: f64, pi: u32) -> Held {
        Held {
            from,
            to,
            window: u64::from(pi) * 10,
            pi,
        }
    }

    fn named(pi: u32) -> Packages {
        match pi {
            1 | 2 => vec!["agentic-harness.pi@a4c73cef8e36".to_string()],
            _ => vec!["agentic-harness.pi@3d8c1dc3a206".to_string()],
        }
    }

    fn path(pi: Vec<(f64, f64)>) -> Path {
        Path {
            niri: model::Spans::new(Vec::new()),
            kitty: model::Spans::new(Vec::new()),
            pi: model::Spans::new(pi),
        }
    }

    #[test]
    fn windows_of_one_install_pool_their_time() {
        let split = installs(
            &path(Vec::new()),
            &[held(0.0, 1000.0, 1), held(2000.0, 3500.0, 2)],
            named,
            FRAME,
            1000,
            7,
        );
        assert_eq!(split.len(), 1, "both windows ran one install: {split:?}");
        assert_eq!(split[0].of, named(1));
        assert_eq!(split[0].seconds, 2.5, "a second and a second and a half");
    }

    #[test]
    fn a_stalled_install_shows_apart_from_one_that_wasn_t() {
        // pi was busy through the older install's stretch alone.
        let split = installs(
            &path(vec![(0.0, 1000.0)]),
            &[held(0.0, 1000.0, 1), held(5000.0, 6000.0, 3)],
            named,
            FRAME,
            1000,
            7,
        );
        assert_eq!(split.len(), 2, "two installs: {split:?}");
        let of = |pi: u32| {
            split
                .iter()
                .find(|install| install.of == named(pi))
                .expect("each install has its keys")
        };
        assert!(
            of(1).summary.p99 > 100.0,
            "keys at the stalled install waited it out: {:?}",
            of(1).summary
        );
        assert!(
            of(3).summary.p99 < model::LATE,
            "keys at the other were on time: {:?}",
            of(3).summary
        );
    }

    #[test]
    fn installs_come_in_the_order_of_their_names() {
        let split = installs(
            &path(Vec::new()),
            &[held(0.0, 1000.0, 3), held(2000.0, 3000.0, 1)],
            named,
            FRAME,
            100,
            7,
        );
        let names: Vec<_> = split.iter().map(|i| i.of.clone()).collect();
        assert_eq!(names, vec![named(3), named(1)], "3d8c sorts before a4c7");
    }

    #[test]
    fn a_stalled_window_shows_apart_from_one_of_the_same_install() {
        // Windows 10 and 20 both ran a4c73cef; pi was busy at 10 alone.
        let split = windows(
            &path(vec![(0.0, 1000.0)]),
            &[held(0.0, 1000.0, 1), held(5000.0, 6000.0, 2)],
            named,
            FRAME,
            1000,
            7,
        );
        assert_eq!(split.len(), 2, "two windows: {split:?}");
        assert_eq!(split[0].of, (10, named(1)), "each window names its install");
        assert_eq!(split[1].of, (20, named(2)));
        assert!(
            split[0].summary.p99 > 100.0,
            "keys at the stalled window waited it out: {:?}",
            split[0].summary
        );
        assert!(
            split[1].summary.p99 < model::LATE,
            "keys at the other were on time: {:?}",
            split[1].summary
        );
    }

    #[test]
    fn a_window_s_stretches_pool_their_time() {
        let split = windows(
            &path(Vec::new()),
            &[held(0.0, 1000.0, 1), held(4000.0, 4500.0, 1)],
            named,
            FRAME,
            100,
            7,
        );
        assert_eq!(split.len(), 1, "one window, left and come back to: {split:?}");
        assert_eq!(split[0].seconds, 1.5, "a second and half a second");
    }
}
