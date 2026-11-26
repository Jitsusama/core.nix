//! What a round of the GitHub reader couldn't read, owed and read again
//! from where it was to be until it is. So one that keeps failing, or a
//! flood past Joel's rate limit, holds back neither how far the reader
//! has read nor anything else it names. One that fails is asked less
//! often each time, so a broken thread costs his shared limit little, and
//! after [`TRIES`] failed asks it's let go, the journal and a row in For
//! You naming the span nobody read, with its link, so he can look there
//! himself. One his search or the watch lists again is asked early, and
//! both list one for an hour after it changed, so for that hour it's
//! asked each round: early asks cost no try, and a round's
//! [`super::FAILS`] bound what they cost.

use pineapple::github::{self, Thread};
use pineapple::progress::Owed;

use super::Left;

/// How many failed asks a pull request or issue is owed before it's let
/// go. With each wait doubling up to [`LONGEST`] rounds, that's 224 rounds
/// that tried it, about two hours at a round each half minute, four on a
/// site searched by author, a round a minute: never time asleep, rounds that
/// couldn't start, asks a spent limit kept back, or asks made early
/// because it changed, so a busy thread is let go no sooner.
const TRIES: u32 = 12;

/// How many doublings an owed thread's wait between asks grows by, so the
/// longest is 31 rounds, about a quarter hour, half an hour by author.
const LONGEST: u32 = 5;

/// A pull request or issue a round reads, with when what's judged there is
/// from and when it's read from.
pub(super) type Listed = (Thread, u64, u64);

/// What a round reads: the search's `hits` first, as likeliest to hold
/// Joel's stop, then what's `owed` and due, fewest failed asks and oldest
/// first, so neither a flood of what `notified` him, last, nor a thread
/// that keeps failing holds back one never asked. One owed that a search
/// hits again takes its turn among what's owed, so three that keep failing
/// and changing can't spend every round's failures before the rest are
/// asked. One listed twice is read once from the farther back of the two,
/// so nothing owed is skipped; one owed that changed is due at once.
pub(super) fn list(hits: Vec<Listed>, notified: Vec<Listed>, owed: &[Owed]) -> Vec<Listed> {
    let mut threads = hits;
    let mut due = Vec::new();
    for owed in owed {
        let mut listed = (owed.thread.clone(), owed.from, owed.read_from);
        let hit = threads
            .iter()
            .position(|(thread, _, _)| *thread == owed.thread);
        if let Some(place) = hit {
            let (_, from, read_from) = threads.remove(place);
            listed.1 = listed.1.min(from);
            listed.2 = listed.2.min(read_from);
        }
        let changed = hit.is_some() || notified.iter().any(|(thread, _, _)| *thread == owed.thread);
        if owed.skip == 0 || changed {
            due.push((owed.tries, listed));
        }
    }
    due.sort_by_key(|(tries, (_, from, _))| (*tries, *from));
    threads.extend(due.into_iter().map(|(_, listed)| listed));
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

/// What a round on the site whose rows `tell` writes, which `began` then
/// owes, of the `threads` it listed, as it `read` them (where they failed,
/// where GitHub said they were gone, and where it stopped asking): those
/// that failed, asked again later each time; those from where it stopped,
/// which a spent limit or too many failures kept back, and what it owed
/// `before` and didn't list, each a round nearer due. One owed that
/// GitHub says is gone is asked again as one that failed, as it says so
/// of one that's there when an organization's sign-in lapses, and one
/// truly gone is let go after its tries, its span named to Joel. One gone
/// that wasn't owed goes, as it did before anything was owed: what notified
/// Joel loses a round's span, but a search hit loses from its reach on,
/// as the search gives it again only once it changes, and while a sign-in
/// lapses each new one there goes so, under a warning each.
pub(super) fn kept(
    tell: &Left,
    threads: &[Listed],
    read: (&[usize], &[usize], usize),
    before: &[Owed],
    began: u64,
) -> Vec<Owed> {
    let (failed, gone, unasked) = read;
    let was = |thread: &Thread| before.iter().find(|owed| owed.thread == *thread);
    let unread = |place: usize| {
        failed.contains(&place)
            || (gone.contains(&place)
                && threads
                    .get(place)
                    .is_some_and(|(it, _, _)| was(it).is_some()))
    };
    let missed = (0..unasked).filter(|place| unread(*place)).count();
    if missed > 0 || unasked < threads.len() {
        eprintln!(
            "<4>{} read {} of the {} it named, so the rest is read again later",
            tell.name,
            unasked - missed,
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
        let asked = place < unasked;
        if asked && !unread(place) {
            continue;
        }
        let was = was(thread);
        // Only an ask its wait chose is a try: one asked early, as it
        // changed, costs nothing, so a busy thread isn't let go sooner.
        let chosen = asked && was.is_none_or(|owed| owed.skip == 0);
        let at = was.map_or(began, |owed| owed.at);
        let tries = was.map_or(0, |owed| owed.tries) + u32::from(chosen);
        if tries >= TRIES {
            eprintln!(
                "<3>couldn't read {} in {TRIES} tries since {}, each failing or answered gone, so \
                 what was said there from {} on goes unjudged: look there for a stop",
                thread.key(),
                github::stamp(at),
                github::stamp(*from)
            );
            tell.thread(thread, *from);
            continue;
        }
        // Asked the round after a first failure, then after waits doubling;
        // one asked early, or not at all, is a round nearer due.
        let skip = if chosen {
            (1 << tries.saturating_sub(1).min(LONGEST)) - 1
        } else {
            was.map_or(0, |owed| owed.skip.saturating_sub(1))
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

#[cfg(test)]
mod tests {
    use super::*;

    /// GitHub's rows in shadow, in a folder of this test run's own, which
    /// only one let go writes.
    fn left() -> Left {
        let home = std::env::temp_dir().join(format!("pineapple-owed-{}", std::process::id()));
        Left::new("github", "GitHub", "https://api.github.com", true, &home)
    }

    /// example/shop's pull request `number`.
    fn thread(number: u64) -> Thread {
        Thread {
            site: "github".to_owned(),
            repo: "example/shop".to_owned(),
            number,
            pull: true,
        }
    }

    /// Pull request `number` owed from `from`, failed `tries` times and
    /// waiting `skip` rounds.
    fn owed(number: u64, from: u64, tries: u32, skip: u32) -> Owed {
        Owed {
            thread: thread(number),
            from,
            read_from: from,
            at: 0,
            tries,
            skip,
        }
    }

    /// The rounds of the first `rounds` in which one failing every ask is
    /// asked, notified in each if `busy` and only the first if not, and
    /// whether it's owed after them.
    fn asked_while_failing(rounds: u64, busy: bool) -> (Vec<u64>, bool) {
        let (mut owing, mut asked) = (Vec::new(), Vec::new());
        for round in 1..=rounds {
            let notified = (busy || round == 1).then(|| (thread(14), 100, 100));
            let threads = list(Vec::new(), notified.into_iter().collect(), &owing);
            let failed: Vec<usize> = (0..threads.len()).collect();
            if !threads.is_empty() {
                asked.push(round);
            }
            owing = kept(
                &left(),
                &threads,
                (&failed, &[], threads.len()),
                &owing,
                round,
            );
        }
        (asked, !owing.is_empty())
    }

    #[test]
    fn one_failing_every_ask_is_asked_after_waits_doubling_to_31_rounds_and_let_go_at_its_twelfth()
    {
        let (asked, owing) = asked_while_failing(224, false);
        assert_eq!(
            asked,
            [1, 2, 4, 8, 16, 32, 64, 96, 128, 160, 192, 224],
            "its waits double to 31 rounds, no longer"
        );
        assert!(!owing, "its twelfth failed ask lets it go");
    }

    #[test]
    fn one_failing_every_ask_that_changes_each_round_is_asked_each_round_and_let_go_no_sooner() {
        let (asked, owing) = asked_while_failing(223, true);
        assert_eq!(
            asked.len(),
            223,
            "one that changes is asked at once, each round"
        );
        assert!(
            owing,
            "asks its changes brought early aren't tries, so it's still owed"
        );
        let (_, owing) = asked_while_failing(224, true);
        assert!(!owing, "it's let go when one untouched is");
    }

    #[test]
    fn a_round_reads_fresh_hits_then_whats_owed_and_due_fewest_tries_first_then_what_notified_joel()
    {
        let hits = vec![(thread(1), 300, 200), (thread(2), 300, 200)];
        let notified = vec![(thread(3), 400, 400), (thread(4), 400, 400)];
        let owing = [
            owed(2, 250, 3, 7),
            owed(4, 350, 1, 5),
            owed(5, 100, 1, 0),
            owed(6, 90, 0, 0),
            owed(7, 50, 1, 3),
        ];
        let threads = list(hits, notified, &owing);

        let numbers: Vec<u64> = threads.iter().map(|(thread, _, _)| thread.number).collect();
        assert_eq!(
            numbers,
            [1, 6, 5, 4, 2, 3],
            "a fresh hit, then what's due by tries and from, a hit or a notified one owed among \
             them though it waits, then the rest notified; one waiting and unchanged isn't read"
        );
        assert_eq!(
            (threads[4].clone(), threads[3].clone()),
            ((thread(2), 250, 200), (thread(4), 350, 350)),
            "one listed twice is judged and read from the farther back of each"
        );
    }

    #[test]
    fn what_a_spent_limit_kept_back_neither_ages_nor_counts_a_try() {
        let before = [Owed {
            at: 5,
            ..owed(14, 100, 11, 0)
        }];
        let threads = list(vec![(thread(12), 300, 300)], Vec::new(), &before);
        let owing = kept(&left(), &threads, (&[], &[], 0), &before, 900);

        let fresh = Owed {
            at: 900,
            ..owed(12, 300, 0, 0)
        };
        assert_eq!(
            owing,
            [fresh, before[0].clone()],
            "both are owed as they were, the one a try from let go not let go"
        );
    }

    #[test]
    fn what_a_spent_limit_kept_back_while_it_waited_is_a_round_nearer_due() {
        // Its search hit lists it early; the limit keeps it unasked.
        let before = [owed(15, 100, 4, 3)];
        let threads = list(vec![(thread(15), 300, 300)], Vec::new(), &before);
        let owing = kept(&left(), &threads, (&[], &[], 0), &before, 900);

        assert_eq!(
            owing,
            [owed(15, 100, 4, 2)],
            "its wait counts the round down, its tries and span kept"
        );
    }

    #[test]
    fn one_owed_that_github_says_is_gone_stays_owed_as_one_failed_and_one_new_goes() {
        let before = [owed(14, 100, 1, 0)];
        let threads = list(vec![(thread(12), 300, 300)], Vec::new(), &before);
        let owing = kept(&left(), &threads, (&[], &[0, 1], 2), &before, 900);

        assert_eq!(
            owing,
            [owed(14, 100, 2, 1)],
            "the owed one is asked again later, the new one goes"
        );
    }
}
