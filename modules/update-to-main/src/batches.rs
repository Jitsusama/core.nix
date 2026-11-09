//! Building a system here a batch of pieces at a time, each batch its own
//! build, and each started only once the machine has the memory for it.
//! The Nix daemon's worker serving a build grows with what that build is
//! given: on 9 November one build of 1,757 pieces grew it to about 8 GB,
//! most of it in swap, on a laptop with 7.4 GB of memory, and systemd-oomd
//! stopped the build. Builds of about 70 pieces on the same laptop left
//! over 3 GB of memory free throughout. A batch's build ends before the
//! next starts, so each worker starts small.

use crate::config::{Batches, Config};
use crate::run::{self, Failure};
use crate::system;
use std::collections::BTreeMap;
use std::fs;
use std::thread;
use std::time::{Duration, Instant};

/// How often the memory is read again while a batch waits for room. A
/// fixed wait, since Linux tells nothing when memory frees, and a few
/// seconds late costs a build nothing.
const LOOK_EVERY: Duration = Duration::from_secs(10);

/// Builds what the system needs, a batch at a time while it needs more
/// pieces than a batch holds, then waits for room for the build that
/// follows, which builds what's left and evaluates the system again.
pub fn build(
    config: &Config,
    batches: &Batches,
    flake: &str,
    overrides: &[String],
) -> Result<(), Failure> {
    let needed = system::needed(config, flake, overrides)?;
    if needed.len() > batches.pieces {
        let mut show = system::nix(config);
        show.args(["derivation", "show"]);
        let shown = run::output(show, "reading what the system needs", &needed)?;
        let inputs: BTreeMap<String, Vec<String>> = system::pieces(&shown)?
            .into_iter()
            .map(|(drv, piece)| (drv, piece.inputs))
            .collect();
        let batched = in_batches(&needed, &inputs, batches.pieces);
        let count = batched.len();
        eprintln!(
            "the system needs {} pieces built, so they build here in {count} batches",
            needed.len()
        );
        for (at, batch) in batched.iter().enumerate() {
            room(config, batches)?;
            let mut build = system::nix(config);
            build.args(["build", "--no-link"]);
            let what = format!("building batch {} of {count} here", at + 1);
            system::told(config, build, &what, batch)?;
        }
    }
    room(config, batches)
}

/// The needed pieces in batches of at most `size`, each batch built from
/// pieces in it or before it: they're ordered by how many needed pieces
/// deep they're built, the pieces built from none of the others first.
pub fn in_batches(
    needed: &[String],
    inputs: &BTreeMap<String, Vec<String>>,
    size: usize,
) -> Vec<Vec<String>> {
    let mut depths = BTreeMap::new();
    for drv in needed {
        depth(drv, inputs, &mut depths);
    }
    let mut ordered: Vec<&String> = needed.iter().collect();
    ordered.sort_by_key(|drv| (depths[drv.as_str()], drv.as_str()));
    ordered
        .chunks(size.max(1))
        .map(|batch| batch.iter().map(|drv| (*drv).clone()).collect())
        .collect()
}

/// How deep a piece sits among the needed ones: none for one built from
/// none of them, else one more than the deepest it's built from. Only the
/// needed pieces' own inputs are known, so one known of no needed piece
/// sits at the bottom.
fn depth(
    drv: &str,
    inputs: &BTreeMap<String, Vec<String>>,
    depths: &mut BTreeMap<String, usize>,
) -> usize {
    if let Some(known) = depths.get(drv) {
        return *known;
    }
    // Set before the inputs are read, so a cycle, which Nix can't build
    // anyway, ends rather than recursing for ever.
    depths.insert(drv.to_owned(), 0);
    let below = inputs.get(drv).map_or(0, |of| {
        of.iter()
            .filter(|input| inputs.contains_key(input.as_str()))
            .map(|input| depth(input, inputs, depths) + 1)
            .max()
            .unwrap_or(0)
    });
    depths.insert(drv.to_owned(), below);
    below
}

/// Waits until Linux says the machine has the room a batch needs, or fails
/// once it has waited `within` seconds, or the update is stopped.
fn room(config: &Config, batches: &Batches) -> Result<(), Failure> {
    let started = Instant::now();
    let within = Duration::from_secs(batches.within);
    let mut said = false;
    loop {
        let free = fs::read_to_string(&config.memory)
            .ok()
            .and_then(|text| free_mib(&text));
        if free.is_some_and(|free| free >= batches.room_mib) {
            return Ok(());
        }
        let free = free.map_or_else(|| "an unknown amount".to_owned(), |free| format!("{free}"));
        if run::stopping() {
            return Err(Failure::new("the update was stopped"));
        }
        if started.elapsed() >= within {
            return Err(Failure::new(format!(
                "only {free} MiB of memory and swap was free here, not the {} a build \
                 needs, for {} minutes, so it didn't start",
                batches.room_mib,
                within.as_secs() / 60
            )));
        }
        if !said {
            eprintln!(
                "{free} MiB of memory and swap is free here, not the {} a build needs, \
                 so it waits",
                batches.room_mib
            );
            said = true;
        }
        thread::sleep(LOOK_EVERY.min(within.saturating_sub(started.elapsed())));
    }
}

/// How much memory is available and swap free together, in MiB, from
/// Linux's meminfo, when both can be read. Both count, since systemd-oomd
/// stops a build only once memory and swap are both nearly used.
pub fn free_mib(meminfo: &str) -> Option<u64> {
    let field = |name: &str| {
        meminfo.lines().find_map(|line| {
            let rest = line.strip_prefix(name)?.strip_prefix(':')?;
            rest.split_whitespace().next()?.parse::<u64>().ok()
        })
    };
    Some((field("MemAvailable")? + field("SwapFree")?) / 1024)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn graph(edges: &[(&str, &[&str])]) -> BTreeMap<String, Vec<String>> {
        edges
            .iter()
            .map(|(drv, of)| {
                (
                    (*drv).to_owned(),
                    of.iter().map(|s| (*s).to_owned()).collect(),
                )
            })
            .collect()
    }

    fn names(needed: &[&str]) -> Vec<String> {
        needed.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn each_batch_is_built_only_from_itself_and_those_before() {
        // d is built from c, c from b, and b from a; e from none.
        let inputs = graph(&[
            ("d", &["c", "outside"]),
            ("c", &["b"]),
            ("b", &["a"]),
            ("a", &[]),
            ("e", &[]),
        ]);
        let batched = in_batches(&names(&["d", "c", "b", "a", "e"]), &inputs, 2);
        assert_eq!(
            batched,
            [names(&["a", "e"]), names(&["b", "c"]), names(&["d"])],
            "the pieces built from none come first, and no batch is built from a later one"
        );
    }

    #[test]
    fn a_batch_holds_no_more_than_its_size_and_every_piece_is_in_one() {
        let needed: Vec<String> = (0..250).map(|n| format!("p{n:03}")).collect();
        let batched = in_batches(&needed, &BTreeMap::new(), 100);
        let sizes: Vec<usize> = batched.iter().map(Vec::len).collect();
        assert_eq!(
            sizes,
            [100, 100, 50],
            "250 pieces make two full batches and a half"
        );
    }

    #[test]
    fn a_cycle_still_ends() {
        let inputs = graph(&[("a", &["b"]), ("b", &["a"])]);
        let batched = in_batches(&names(&["a", "b"]), &inputs, 10);
        assert_eq!(
            batched.concat().len(),
            2,
            "both pieces are batched once: {batched:?}"
        );
    }

    #[test]
    fn the_room_is_memory_available_and_swap_free_together() {
        let meminfo = "MemTotal:        7559036 kB\nMemFree:          200000 kB\n\
                       MemAvailable:    3924472 kB\nSwapTotal:       8388604 kB\n\
                       SwapFree:        7009640 kB\n";
        assert_eq!(
            free_mib(meminfo),
            Some(10_677),
            "(3924472 + 7009640) kB in whole MiB"
        );
        assert_eq!(
            free_mib("MemAvailable: 100 kB\n"),
            None,
            "with no swap line the room can't be told"
        );
    }
}
