//! What a round of the GitHub reader couldn't read, owed and read again
//! from where it was to be until it is. So one that keeps failing, or a
//! flood past Joel's rate limit, holds back neither how far the reader
//! has read nor anything else it names. One that fails is asked less
//! often each time, so a broken thread costs his shared limit little, and
//! after [`TRIES`] failed asks it's let go, the journal naming the span
//! nobody read so he can look there himself.

use pineapple::github::{self, Thread};
use pineapple::progress::Owed;

/// How many failed asks a pull request or issue is owed before it's let
/// go. With each wait doubling up to [`LONGEST`] rounds, that's about four
/// hours of rounds that tried it: never time asleep, rounds that couldn't
/// start, or asks a spent limit kept back.
const TRIES: u32 = 12;

/// How many doublings an owed thread's wait between asks grows by, so the
/// longest is 31 rounds, about half an hour.
const LONGEST: u32 = 5;

/// A pull request or issue a round reads, with when what's judged there is
/// from and when it's read from.
pub(super) type Listed = (Thread, u64, u64);

/// What a round reads: the search's `hits` first, as likeliest to hold
/// Joel's stop, then what's `owed` and due, fewest failed asks and oldest
/// first, so neither a flood of what `notified` him, last, nor a thread
/// that keeps failing holds back one never asked. One listed twice is read
/// once from the farther back of the two, so nothing owed is skipped; one
/// owed that changed is due at once.
pub(super) fn list(hits: Vec<Listed>, notified: Vec<Listed>, owed: &[Owed]) -> Vec<Listed> {
    let mut threads = hits;
    let mut due = Vec::new();
    for owed in owed {
        if merged(&mut threads, &owed.thread, owed.from, owed.read_from) {
            continue;
        }
        if owed.skip == 0 || notified.iter().any(|(thread, _, _)| *thread == owed.thread) {
            due.push(owed);
        }
    }
    due.sort_by_key(|owed| (owed.tries, owed.from));
    for owed in due {
        threads.push((owed.thread.clone(), owed.from, owed.read_from));
    }
    for (thread, from, read_from) in notified {
        if !merged(&mut threads, &thread, from, read_from) {
            threads.push((thread, from, read_from));
        }
    }
    threads
}

/// Whether `thread` is in `threads` already, then read from the farther
/// back of where it was and `from` and `read_from`.
fn merged(threads: &mut [Listed], thread: &Thread, from: u64, read_from: u64) -> bool {
    let Some((_, judged, read)) = threads.iter_mut().find(|(listed, _, _)| listed == thread) else {
        return false;
    };
    *judged = (*judged).min(from);
    *read = (*read).min(read_from);
    true
}

/// What a round on the site `name` that `began` then owes, of the
/// `threads` it listed, as it `read` them: those whose read failed, asked
/// again later each time, and those from where it stopped asking, which a
/// spent limit or too many failures kept back, due next round as they
/// were; and what it owed `before` and didn't list, a round nearer due.
pub(super) fn kept(
    name: &str,
    threads: &[Listed],
    read: (&[usize], usize),
    before: &[Owed],
    began: u64,
) -> Vec<Owed> {
    let (failed, unasked) = read;
    if !failed.is_empty() || unasked < threads.len() {
        eprintln!(
            "<4>{name} read {} of the {} it named, so the rest is read again later",
            unasked - failed.len(),
            threads.len()
        );
    }
    let unlisted = before
        .iter()
        .filter(|owed| !threads.iter().any(|(thread, _, _)| *thread == owed.thread));
    let mut owed: Vec<Owed> = unlisted
        .map(|owed| Owed {
            skip: owed.skip.saturating_sub(1),
            ..owed.clone()
        })
        .collect();
    for (place, (thread, from, read_from)) in threads.iter().enumerate() {
        let asked = failed.contains(&place);
        if place < unasked && !asked {
            continue;
        }
        let was = before.iter().find(|owed| owed.thread == *thread);
        let at = was.map_or(began, |owed| owed.at);
        let tries = was.map_or(0, |owed| owed.tries) + u32::from(asked);
        if tries >= TRIES {
            eprintln!(
                "<3>couldn't read {} in {TRIES} tries since {}, so what was said there from {} on \
                 goes unjudged: look there for a stop",
                thread.key(),
                github::stamp(at),
                github::stamp(*from)
            );
            continue;
        }
        // Asked the round after a first failure, then after waits doubling.
        let skip = if asked {
            (1 << tries.saturating_sub(1).min(LONGEST)) - 1
        } else {
            0
        };
        owed.push(Owed {
            thread: thread.clone(),
            from: *from,
            read_from: *read_from,
            at,
            tries,
            skip,
        });
    }
    owed
}
