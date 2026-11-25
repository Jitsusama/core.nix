//! What a round of the GitHub reader couldn't read, owed and read again
//! each round from where it was to be until it is. So one that keeps
//! failing, or a flood past Joel's rate limit, holds back neither how far
//! the reader has read nor anything else it names; one owed past
//! [`OWED_FOR`] is let go, said loudly, and read again once it changes.

use pineapple::github::Thread;
use pineapple::progress::Owed;

/// How long a pull request or issue is owed before it's let go: past the
/// hour a spent rate limit lasts, so only one that keeps failing goes.
const OWED_FOR: u64 = 2 * 60 * 60;

/// Lists what's `owed` after the `threads` a round reads, each with when
/// what's judged there is from and when it's read from; one listed already
/// is read from the farther back of the two, so nothing owed is skipped.
pub(super) fn list(threads: &mut Vec<(Thread, u64, u64)>, owed: &[Owed]) {
    for owed in owed {
        let listed = threads
            .iter_mut()
            .find(|(listed, _, _)| *listed == owed.thread);
        match listed {
            Some((_, from, read_from)) => {
                *from = (*from).min(owed.from);
                *read_from = (*read_from).min(owed.read_from);
            }
            None => threads.push((owed.thread.clone(), owed.from, owed.read_from)),
        }
    }
}

/// What a round on the site `name` that `began` then owes: each of the
/// `threads` it listed whose place is in `unread`, owed since it first was
/// in what it owed `before`. One owed past [`OWED_FOR`] the journal names
/// as let go.
pub(super) fn kept(
    name: &str,
    threads: &[(Thread, u64, u64)],
    unread: &[usize],
    before: &[Owed],
    began: u64,
) -> Vec<Owed> {
    if !unread.is_empty() {
        eprintln!(
            "<4>{name} read {} of the {} it named, so the rest is read again next round",
            threads.len() - unread.len(),
            threads.len()
        );
    }
    let mut owed = Vec::new();
    for &place in unread {
        let (thread, from, read_from) = &threads[place];
        let at = before
            .iter()
            .find(|owed| owed.thread == *thread)
            .map_or(began, |owed| owed.at);
        if began.saturating_sub(at) > OWED_FOR {
            eprintln!(
                "<3>couldn't read {} for two hours, so it's read again only once it changes",
                thread.key()
            );
            continue;
        }
        owed.push(Owed {
            thread: thread.clone(),
            from: *from,
            read_from: *read_from,
            at,
        });
    }
    owed
}
