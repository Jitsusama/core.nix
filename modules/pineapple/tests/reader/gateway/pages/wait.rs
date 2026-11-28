//! A search whose later page goes unanswered beside a first page that
//! blinks: only a round that waited on a later page is heard beside a
//! search that can't be read, the phrase's wait as the tag's is in
//! pages.rs, and a blink between two waits neither counts nor ends their
//! row, so the tenth still comes. A search waiting beside one that
//! blinked reads neither whole, so a row saying it can't be read stands.

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use super::super::cut::{nothing, opens, tags, told_and_read_on};
use super::super::{FAILS, Until, hearing, poisoned, said};
use super::{began, bounded, in_pages, paged};

#[test]
fn a_first_page_blinking_after_a_wait_leaves_the_row_as_it_was() {
    static TAGS: AtomicUsize = AtomicUsize::new(0);
    static BEGAN: AtomicU64 = AtomicU64::new(0);
    let came = hearing(
        |tool, asked, now| {
            let began = began(&BEGAN, now);
            if tool != "slack_search_public_and_private" {
                return said(tool, now);
            } else if asked["query"] == "pineapple" {
                return poisoned();
            } else if asked.get("before").is_some() {
                return format!("{FAILS}Slack said: ratelimited");
            }
            let round = if opens(asked) {
                TAGS.fetch_add(1, Ordering::SeqCst) + 1
            } else {
                TAGS.load(Ordering::SeqCst)
            };
            // Read whole, then waiting on its second page a round, then
            // unreadable three rounds, so the row says both can't be read,
            // then blinking at its first page every round.
            match round {
                1 => tags(asked, &[(began - 300, "a word")], 1),
                2 => bounded(asked, &paged(began, "a word"), 3, true),
                3..=5 => poisoned(),
                _ => format!("{FAILS}Slack said: ratelimited"),
            }
        },
        // Asks enough for the blinks to come after the row says both.
        &Until {
            judging: 0,
            asks: 30,
            read_at: None,
        },
    );
    // The row says both by the fourth round; the eighth opening means the
    // sixth and seventh blinked whole, as a stale wait takes two to narrow.
    assert!(
        TAGS.load(Ordering::SeqCst) >= 8,
        "two rounds blinked after the row said both: {}",
        came.journal
    );
    assert!(
        came.journal.contains("1 of 10 rounds in a row")
            && came
                .why
                .contains("Slack's answers to the safe word can't be read"),
        "a wait rounds ago doesn't make a first page's blink a wait, so the row still says \
         both can't be read: {}\n{}",
        came.why,
        came.journal
    );
}

#[test]
fn a_search_that_cant_be_read_is_told_while_the_phrase_waits_on_a_later_page() {
    static PHRASES: AtomicUsize = AtomicUsize::new(0);
    static BEGAN: AtomicU64 = AtomicU64::new(0);
    let came = hearing(
        |tool, asked, now| {
            let began = began(&BEGAN, now);
            if tool != "slack_search_public_and_private" {
                said(tool, now)
            } else if asked["query"] != "pineapple" {
                poisoned()
            } else if asked.get("before").is_some() {
                format!("{FAILS}Slack said: ratelimited")
            } else {
                in_pages(&PHRASES, asked, began, &paged(began, "a word"), 3, true)
            }
        },
        // Only until the row is told.
        &Until {
            judging: 0,
            asks: 0,
            read_at: None,
        },
    );
    let told = came.journal.find("<5>wrote Joel's row in ");
    let began_waiting = came.journal.find("1 of 10 rounds in a row");
    let waiting = came.journal.find("3 of 10 rounds in a row");
    assert!(
        came.why
            .contains("search of Slack for your tag can't be read")
            && began_waiting.is_some()
            && told > began_waiting
            && (waiting.is_none() || told < waiting),
        "the tag's search, unreadable, is told in its rounds while the phrase's waits on its \
         second page: {}",
        came.journal
    );
}

#[test]
fn the_phrases_first_page_blinking_beside_a_tag_that_cant_be_read_leaves_the_row_as_it_was() {
    static PHRASES: AtomicUsize = AtomicUsize::new(0);
    static BLINKS: AtomicUsize = AtomicUsize::new(0);
    let came = hearing(
        |tool, asked, now| {
            if tool != "slack_search_public_and_private" {
                said(tool, now)
            } else if asked["query"] != "pineapple" {
                poisoned()
            } else if PHRASES.fetch_add(1, Ordering::SeqCst) < 3 {
                // Both can't be read until the row says so, then the
                // phrase's first page blinks in every round.
                poisoned()
            } else {
                if opens(asked) {
                    BLINKS.fetch_add(1, Ordering::SeqCst);
                }
                format!("{FAILS}Slack said: ratelimited")
            }
        },
        // Told, then rounds enough to tell it anew.
        &Until {
            judging: 0,
            asks: 30,
            read_at: None,
        },
    );
    // A blink heard narrows the row in its second round, so four blinks
    // leave it room to.
    assert!(
        BLINKS.load(Ordering::SeqCst) >= 4,
        "four rounds blinked after the row said both: {}",
        came.journal
    );
    assert!(
        came.why
            .contains("Slack's answers to the safe word can't be read"),
        "the phrase's search whose first page blinks, waiting on no later page, isn't heard, \
         so the row still says both can't be read: {}",
        came.why
    );
}

#[test]
fn a_tag_waiting_on_a_later_page_beside_the_phrase_blinking_leaves_its_row_standing() {
    static PHRASES: AtomicUsize = AtomicUsize::new(0);
    static TAGS: AtomicUsize = AtomicUsize::new(0);
    static BEGAN: AtomicU64 = AtomicU64::new(0);
    let came = hearing(
        |tool, asked, now| {
            let began = began(&BEGAN, now);
            if tool != "slack_search_public_and_private" {
                return said(tool, now);
            } else if asked.get("before").is_some() {
                return format!("{FAILS}Slack said: ratelimited");
            }
            // Each search counts its own rounds, so they keep in step.
            let rounds = if asked["query"] == "pineapple" {
                &PHRASES
            } else {
                &TAGS
            };
            let round = if opens(asked) {
                rounds.fetch_add(1, Ordering::SeqCst) + 1
            } else {
                rounds.load(Ordering::SeqCst)
            };
            // The tag can't be read three rounds while the phrase is read
            // whole, so the row says the tag can't be read; then the tag
            // waits on its second page every round as the phrase's first
            // page blinks.
            match (asked["query"] == "pineapple", round) {
                (true, 1..=3) => nothing(),
                (true, _) => format!("{FAILS}Slack said: ratelimited"),
                (false, 1..=3) => poisoned(),
                (false, _) => bounded(asked, &paged(began, "a word"), 3, true),
            }
        },
        // Told, then rounds enough for the waits to have untold it.
        &Until {
            judging: 0,
            asks: 24,
            read_at: None,
        },
    );
    assert!(
        TAGS.load(Ordering::SeqCst) >= 6,
        "three rounds waited after the row was told: {}",
        came.journal
    );
    assert!(
        came.why
            .contains("search of Slack for your tag can't be read"),
        "a tag read only to a later page beside a phrase that blinked reads neither whole, \
         so the row saying the tag can't be read stands: {:?}\n{}",
        came.why,
        came.journal
    );
}

#[test]
fn a_first_page_blinking_between_two_waits_neither_counts_nor_ends_their_row() {
    static TAGS: AtomicUsize = AtomicUsize::new(0);
    static BEGAN: AtomicU64 = AtomicU64::new(0);
    let came = hearing(
        |tool, asked, now| {
            let began = began(&BEGAN, now);
            if tool != "slack_search_public_and_private" {
                return said(tool, now);
            } else if asked["query"] == "pineapple" {
                return nothing();
            } else if asked.get("before").is_some() {
                return format!("{FAILS}Slack said: ratelimited");
            }
            let round = if opens(asked) {
                TAGS.fetch_add(1, Ordering::SeqCst) + 1
            } else {
                TAGS.load(Ordering::SeqCst)
            };
            // Read whole, then waiting on its second page every even
            // round and blinking at its first page every odd one.
            match round {
                1 => tags(asked, &[(began - 300, "a word")], 1),
                _ if round % 2 == 0 => bounded(asked, &paged(began, "a word"), 3, true),
                _ => format!("{FAILS}Slack said: ratelimited"),
            }
        },
        // Until the row names the gap the tenth wait makes, two deaf
        // rounds after it, about the twenty-second.
        &Until {
            judging: 0,
            asks: 0,
            read_at: None,
        },
    );
    assert!(
        came.journal.contains("2 of 10 rounds in a row"),
        "a blink between two waits neither counts nor ends their row, so the second wait is \
         the second in a row: {}",
        came.journal
    );
    // Read from the journal, as the round after may read on past the gap
    // and take the row away.
    let tenth = came.journal.find("goes unanswered, 10 rounds in a row");
    let (told, _) = told_and_read_on(&came.journal);
    assert!(
        tenth.is_some() && told > tenth,
        "a gateway whose pages take turns failing still reaches its tenth, and Joel's row \
         tells of the gap: {}",
        came.journal
    );
    // The tenth wait is the twentieth opening; blinks counted as waits
    // would bring it about the eleventh.
    assert!(
        TAGS.load(Ordering::SeqCst) >= 20,
        "blinks between waits aren't counted, so the tenth comes only at the twentieth \
         opening: {}",
        came.journal
    );
}
