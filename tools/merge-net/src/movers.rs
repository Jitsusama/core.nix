//! Which merges in a range changed a test's derivation, the only ones a
//! search for what broke it need run. Each answer is a `nix eval` of the
//! flake at that merge, about 16 seconds on a lent builder; on 10 November
//! two searches asked them one after another across 27 merges, nine
//! minutes each before the search's first run. So they are asked several
//! at once, a few at a time across every search the net runs, since each
//! holds up to two GiB on the builder that evaluates it.

use std::sync::{Condvar, Mutex, MutexGuard, PoisonError};

use crate::watch::World;

/// How many evaluations the net runs at once, across all its searches.
const EVALUATIONS: usize = 4;

/// The turns every search's evaluations take.
static NET: Turns = Turns::new(EVALUATIONS);

/// The merges among `merges` (oldest first, on main's first parents) worth
/// searching for what broke `test`: those that changed its derivation from
/// the commit before them, or that it couldn't be told for. Only these can
/// have broken it, since a test whose derivation is the same at two
/// commits does there what it does here, and each is seconds' evaluation
/// against minutes a run: on 7 November the search among 16 merges took
/// 73 minutes. The first merge stays all the same, so a test failing from the range's start, which no merge changed,
/// is still told apart as broken outside the code rather than blamed on
/// the next merge.
pub fn movers(world: &dyn World, merges: &[String], test: &str) -> Vec<String> {
    let drvs = drvs(&NET, merges, |merge| world.drv(merge, test).ok());
    merges
        .iter()
        .enumerate()
        .filter(|&(at, _)| at == 0 || drvs[at].is_none() || drvs[at] != drvs[at - 1])
        .map(|(_, merge)| merge.clone())
        .collect()
}

/// What `ask` says of each of `merges`, in their order, asked as `turns`
/// allows.
fn drvs(
    turns: &Turns,
    merges: &[String],
    ask: impl Fn(&str) -> Option<String> + Sync,
) -> Vec<Option<String>> {
    let ask = &ask;
    std::thread::scope(|scope| {
        let asked: Vec<_> = merges
            .iter()
            .map(|merge| {
                scope.spawn(move || {
                    let _turn = turns.take();
                    ask(merge)
                })
            })
            .collect();
        asked
            .into_iter()
            .map(|one| one.join().ok().flatten())
            .collect()
    })
}

/// At most `most` places, each taken until it's dropped.
struct Turns {
    most: usize,
    under_way: Mutex<usize>,
    room: Condvar,
}

impl Turns {
    const fn new(most: usize) -> Self {
        Self {
            most,
            under_way: Mutex::new(0),
            room: Condvar::new(),
        }
    }

    fn take(&self) -> Turn<'_> {
        let mut under_way = self.lock();
        while *under_way >= self.most {
            under_way = self
                .room
                .wait(under_way)
                .unwrap_or_else(PoisonError::into_inner);
        }
        *under_way += 1;
        Turn(self)
    }

    fn lock(&self) -> MutexGuard<'_, usize> {
        self.under_way
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }
}

/// A place among the turns, given back when dropped.
struct Turn<'a>(&'a Turns);

impl Drop for Turn<'_> {
    fn drop(&mut self) {
        *self.0.lock() -= 1;
        self.0.room.notify_one();
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    fn names(merges: &str) -> Vec<String> {
        merges.split_whitespace().map(str::to_owned).collect()
    }

    #[test]
    fn evaluations_run_side_by_side_up_to_the_bound_and_keep_their_order() {
        let turns = Turns::new(3);
        let at_once = Mutex::new((0_usize, 0_usize));
        let filled = Condvar::new();
        let drvs = drvs(&turns, &names("a b c d e f"), |merge| {
            let mut held = at_once.lock().unwrap();
            held.0 += 1;
            held.1 = held.1.max(held.0);
            filled.notify_all();
            // Each waits for the bound's worth beside it, at most a few
            // seconds, so evaluations asked one after another would show
            // as one at once rather than hang the test.
            held = filled
                .wait_timeout_while(held, Duration::from_secs(5), |held| held.0 < 3)
                .unwrap()
                .0;
            held.0 -= 1;
            drop(held);
            (merge != "d").then(|| format!("/nix/store/{merge}.drv"))
        });
        let most = at_once.lock().unwrap().1;
        assert_eq!(most, 3, "evaluations ran {most} at once, not three");
        assert_eq!(drvs[0].as_deref(), Some("/nix/store/a.drv"));
        assert_eq!(drvs[3], None, "d didn't evaluate");
        assert_eq!(drvs[5].as_deref(), Some("/nix/store/f.drv"));
    }
}
