//! What a key waited on at the windows of each install of pi's packages.
//!
//! One sample pools every pi window Joel was at, so a rise between
//! samples can't say whether the machine's load or a newer install
//! moved it. Sessions load their packages when they start and keep them
//! until they reload, so within one sample windows run different
//! installs under the same load, and splitting the keys by install
//! compares them fairly.

use crate::model::{self, Path};
use std::collections::BTreeMap;

/// A stretch Joel spent at a pi window, in the monotonic clock's
/// milliseconds, and the pi process the window ran.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Held {
    pub from: f64,
    pub to: f64,
    pub pi: u32,
}

/// What keys waited on at the windows that ran one install.
#[derive(Debug, Clone, PartialEq)]
pub struct Install {
    /// The packages the windows' pi loaded, by name and version.
    pub packages: Vec<String>,
    pub seconds: f64,
    pub summary: model::Summary,
}

/// The keys of each install, modelled over its own stretches, with
/// `installs` naming what each pi process loaded.
pub fn split(
    path: &Path,
    held: &[Held],
    installs: impl Fn(u32) -> Vec<String>,
    frame: f64,
    keys: usize,
    seed: u64,
) -> Vec<Install> {
    let mut by: BTreeMap<Vec<String>, Vec<(f64, f64)>> = BTreeMap::new();
    for stretch in held {
        by.entry(installs(stretch.pi))
            .or_default()
            .push((stretch.from, stretch.to));
    }
    by.into_iter()
        .filter_map(|(packages, stretches)| {
            let modelled = model::keys(path, &stretches, frame, keys, &mut model::Draw::new(seed));
            let summary = model::summary(&modelled)?;
            let seconds = stretches.iter().map(|(a, b)| b - a).sum::<f64>() / 1000.0;
            Some(Install {
                packages,
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
        Held { from, to, pi }
    }

    fn named(pi: u32) -> Vec<String> {
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
        let split = split(
            &path(Vec::new()),
            &[held(0.0, 1000.0, 1), held(2000.0, 3500.0, 2)],
            named,
            FRAME,
            1000,
            7,
        );
        assert_eq!(split.len(), 1, "both windows ran one install: {split:?}");
        assert_eq!(split[0].packages, named(1));
        assert_eq!(split[0].seconds, 2.5, "a second and a second and a half");
    }

    #[test]
    fn a_stalled_install_shows_apart_from_one_that_wasn_t() {
        // pi was busy through the older install's stretch alone.
        let split = split(
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
                .find(|install| install.packages == named(pi))
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
        let split = split(
            &path(Vec::new()),
            &[held(0.0, 1000.0, 3), held(2000.0, 3000.0, 1)],
            named,
            FRAME,
            100,
            7,
        );
        let names: Vec<_> = split.iter().map(|i| i.packages.clone()).collect();
        assert_eq!(names, vec![named(3), named(1)], "3d8c sorts before a4c7");
    }
}
