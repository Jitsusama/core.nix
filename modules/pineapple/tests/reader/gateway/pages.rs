//! The reader over a gateway that keeps to the bounds it's asked, reading
//! a search back to the last read in pages: each page is the one before
//! the oldest message of the one after it, which it must hold, so none
//! between the two goes unread. A page that doesn't, that can't be read,
//! or five pages short of it, is a gap: deaf until Joel is told, then read
//! on, with what the pages agreed on read. A page that goes unanswered
//! counts neither way, what the pages before it gave judged, until the
//! tenth round in a row makes it a gap, as it is at once in a gap already.
//! Asked with no bound after, a message Slack finds late is read while
//! it's among the newest.

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use serde_json::{Value, json};

use super::cut::{nothing, opens, stop, tags, tags_answer, told_and_read_on};
use super::{FAILS, JOEL, Until, each, hearing, poisoned, said};

mod wait;

/// When a case began, in seconds, set by its first ask, so a second
/// passing mid-case moves no message's time.
fn began(at: &AtomicU64, now: u64) -> u64 {
    at.compare_exchange(0, now, Ordering::SeqCst, Ordering::SeqCst)
        .map_or_else(|began| began, |_| now)
}

/// A tag search's answer to `asked` from `said`, newest first, as a
/// gateway keeping to its bounds gives it, cut to `kept` alike for every
/// ask: those said from its `after` second, when it's asked one, and to
/// its `before` second, or, unless `meets`, only before that second, so a
/// page never holds the oldest of the one after it. Asked with the
/// replies it would add, it answers in words that can't be read.
fn bounded(asked: &Value, said: &[(u64, &str)], kept: usize, meets: bool) -> String {
    if asked["include_context"] != json!(false) {
        return json!({ "results": "## Messages (" }).to_string();
    }
    let second = |key: &str| asked[key].as_str().and_then(|at| at.parse::<u64>().ok());
    let (after, before) = (second("after"), second("before"));
    let found: Vec<(u64, &str)> = said
        .iter()
        .copied()
        .filter(|(at, _)| after.is_none_or(|after| *at >= after))
        .filter(|(at, _)| {
            before.is_none_or(|before| {
                if meets {
                    *at <= before
                } else {
                    at.saturating_add(1) < before
                }
            })
        })
        .collect();
    tags(asked, &found, kept)
}

/// The messages a gap's pages find after the first round, newest first,
/// said from the second a case `began`: the stop, two words the first
/// page holds, the word read in the first round, one Slack found late in
/// the minute before it, and an older one.
fn paged(began: u64, stop: &str) -> [(u64, &str); 6] {
    [
        (began - 60, stop),
        (began - 200, "a newer word"),
        (began - 250, "a word between"),
        (began - 300, "a word"),
        (began - 330, "a word found late"),
        (began - 400, "an older word"),
    ]
}

/// The tag search's answers from the second a case `began`: its first
/// round finds two words, whole, the newer said 300 seconds before; each
/// round after keeps to its bounds over `found`, cut to `kept`, its pages
/// meeting when `meets`. Each round opens asking for twenty, which
/// `rounds` counts.
fn in_pages(
    rounds: &AtomicUsize,
    asked: &Value,
    began: u64,
    found: &[(u64, &str)],
    kept: usize,
    meets: bool,
) -> String {
    let round = if opens(asked) {
        rounds.fetch_add(1, Ordering::SeqCst) + 1
    } else {
        rounds.load(Ordering::SeqCst)
    };
    if round <= 1 {
        let first = [(began - 300, "a word"), (began - 400, "an older word")];
        return tags(asked, &first, first.len());
    }
    bounded(asked, found, kept, meets)
}

/// Each bound, after and before, the tag's searches were asked between.
fn bounds(searched: &[Value]) -> Vec<(Value, Value)> {
    let tag = format!("<@{JOEL}>");
    searched
        .iter()
        .filter(|asked| asked["query"] == tag.as_str())
        .map(|asked| (asked["after"].clone(), asked["before"].clone()))
        .collect()
}

#[test]
fn a_gap_read_in_pages_back_to_the_last_read_is_read_untold() {
    static TAGS: AtomicUsize = AtomicUsize::new(0);
    static BEGAN: AtomicU64 = AtomicU64::new(0);
    let came = hearing(
        |tool, asked, now| {
            let began = began(&BEGAN, now);
            if tool != "slack_search_public_and_private" {
                said(tool, now)
            } else if asked["query"] == "pineapple" {
                // Can't be read, so a row is told and the case can stop
                // waiting; the tag's gap is never told.
                poisoned()
            } else {
                let stop = stop();
                in_pages(&TAGS, asked, began, &paged(began, &stop), 3, true)
            }
        },
        &Until {
            judging: 6,
            asks: 0,
            read_at: None,
        },
    );
    assert_eq!(
        each(&came.said, "verdict"),
        [
            "nothing", "nothing", "nothing", "nothing", "nothing", "stop"
        ],
        "the two words read whole, then from the pages each word the first page cut and \
         the one found late, then the stop, each judged once: {}",
        came.journal
    );
    assert!(
        !came
            .journal
            .contains(&format!("search for \"<@{JOEL}>\" can't be read"))
            && came.journal.contains("read page 2, before")
            && came.journal.contains("read page 3, before")
            && came
                .journal
                .contains("read 6 messages in pages, back to the last read"),
        "read in three pages back to the last read, the second cut, the gap is no deafness \
         and the journal says so: {}",
        came.journal
    );
    assert!(
        came.missed.is_null(),
        "nothing was read on past, so no row says so: {:?}",
        came.missed
    );
    let began = BEGAN.load(Ordering::SeqCst);
    let bounds = bounds(&came.searched);
    assert!(
        bounds.iter().all(|(after, _)| after.is_null())
            && bounds.contains(&(Value::Null, Value::Null))
            && bounds.contains(&(Value::Null, json!((began - 249).to_string())))
            && bounds.contains(&(Value::Null, json!((began - 329).to_string()))),
        "asked for the newest with no bound after, then for each page to the second after \
         the oldest of the one after it: {bounds:?}"
    );
}

#[test]
fn a_page_that_doesnt_hold_the_oldest_of_the_one_after_it_is_a_gap() {
    static TAGS: AtomicUsize = AtomicUsize::new(0);
    static BEGAN: AtomicU64 = AtomicU64::new(0);
    let came = hearing(
        |tool, asked, now| {
            let began = began(&BEGAN, now);
            if tool != "slack_search_public_and_private" {
                said(tool, now)
            } else if asked["query"] == "pineapple" {
                nothing()
            } else {
                let stop = stop();
                in_pages(&TAGS, asked, began, &paged(began, &stop), 3, false)
            }
        },
        // A round is three searches, then six once there's a gap: five
        // rounds and the next's first, past the read on, as what the pages
        // gave is judged before it.
        &Until {
            judging: 6,
            asks: 28,
            read_at: None,
        },
    );
    let (told, read_on) = told_and_read_on(&came.journal);
    assert!(
        came.journal.contains("doesn't hold")
            && came.journal.contains("the oldest of the one after it")
            && told.is_some()
            && read_on > told,
        "a page that doesn't meet the one after it leaves what lies between unread, so it's \
         a gap, told, then read on: {}",
        came.journal
    );
    assert_eq!(
        each(&came.said, "verdict"),
        [
            "nothing", "nothing", "nothing", "nothing", "nothing", "stop"
        ],
        "what both pages gave was agreed, so each is judged while the span between them is \
         unread, the one Slack found late among them: {}",
        came.journal
    );
    assert_eq!(
        came.missed["items"].as_array().map(Vec::len),
        Some(1),
        "what it read on past is a row of its own: {:?}",
        came.missed
    );
}

#[test]
fn a_page_that_cant_be_read_is_a_gap_read_on_once_told() {
    static TAGS: AtomicUsize = AtomicUsize::new(0);
    static BEGAN: AtomicU64 = AtomicU64::new(0);
    let came = hearing(
        |tool, asked, now| {
            let began = began(&BEGAN, now);
            if tool != "slack_search_public_and_private" {
                said(tool, now)
            } else if asked["query"] == "pineapple" {
                nothing()
            } else if asked.get("before").is_some() {
                // The second page, typed into in the gap's first round,
                // then failing in every round after, which keeps the gap
                // rather than waiting anew.
                if TAGS.load(Ordering::SeqCst) <= 2 {
                    poisoned()
                } else {
                    format!("{FAILS}Slack said: ratelimited")
                }
            } else {
                let stop = stop();
                in_pages(&TAGS, asked, began, &paged(began, &stop), 3, true)
            }
        },
        // A round is three searches, then five once there's a gap: four
        // rounds and the next's first, past the read on, as what the
        // first page gave is judged before it.
        &Until {
            judging: 5,
            asks: 19,
            read_at: None,
        },
    );
    let began = BEGAN.load(Ordering::SeqCst);
    let (told, read_on) = told_and_read_on(&came.journal);
    assert!(
        came.journal
            .contains(&format!("its page before {} can't be read", began - 249))
            && came
                .journal
                .contains(&format!("its page before {} goes unanswered", began - 249))
            && !came.journal.contains("rounds in a row")
            && told.is_some()
            && read_on > told,
        "a page that can't be read leaves what lies before the pages read unread, so it's \
         a gap, told, then read on, as it stays once that page goes unanswered: {}",
        came.journal
    );
    assert_eq!(
        each(&came.said, "verdict"),
        ["nothing", "nothing", "nothing", "nothing", "stop"],
        "read on, the first page, agreed, is judged, its stop with it: {}",
        came.journal
    );
}

#[test]
fn a_page_unanswered_ten_rounds_in_a_row_is_a_gap_while_the_pages_before_it_are_judged() {
    static TAGS: AtomicUsize = AtomicUsize::new(0);
    static BEGAN: AtomicU64 = AtomicU64::new(0);
    let came = hearing(
        |tool, asked, now| {
            let began = began(&BEGAN, now);
            if tool != "slack_search_public_and_private" {
                said(tool, now)
            } else if asked["query"] == "pineapple" {
                nothing()
            } else if asked.get("before").is_some() {
                // The second page, rate-limited in every round.
                format!("{FAILS}Slack said: ratelimited")
            } else {
                let stop = stop();
                in_pages(&TAGS, asked, began, &paged(began, &stop), 3, true)
            }
        },
        // A round is three searches, then five once the second page is
        // asked: nine rounds waiting, the gap's three, and the next's
        // first, past the read on.
        &Until {
            judging: 5,
            asks: 64,
            read_at: None,
        },
    );
    let began = BEGAN.load(Ordering::SeqCst);
    let unanswered = |rounds: u32| {
        came.journal.find(&format!(
            "but its page before {} goes unanswered, {rounds} of 10 rounds in a row",
            began - 249
        ))
    };
    let stopped = came.journal.find("<5>a stop, in shadow: ");
    assert!(
        unanswered(1).is_some() && stopped > unanswered(1) && stopped < unanswered(2),
        "the stop on the first page is judged in the round its second page first goes \
         unanswered, the journal saying so: {}",
        came.journal
    );
    let (told, read_on) = told_and_read_on(&came.journal);
    assert!(
        unanswered(9).is_some()
            && unanswered(10).is_none()
            && told > unanswered(9)
            && read_on > told
            && came.journal.contains("goes unanswered, 10 rounds in a row"),
        "a page unanswered for ten rounds in a row, and only then, is a gap, told, then \
         read on: {}",
        came.journal
    );
    assert_eq!(
        each(&came.said, "verdict"),
        ["nothing", "nothing", "nothing", "nothing", "stop"],
        "what the first page gave is judged once, its stop with it: {}",
        came.journal
    );
    // The read on reaches back a minute before the last read, the first
    // round's newest, which was said 300 seconds before the case began, as
    // a gap told at once would: the wait moves no last read. A second
    // write of the same span would fold into the one row, so it's the
    // journal that says the row was written once.
    let written = came
        .journal
        .lines()
        .filter(|line| {
            line.contains("<5>wrote Joel's row in ")
                && line.contains("missed.json: The safe word never read what tagged you")
        })
        .count();
    assert!(
        written == 1,
        "the row of what it read on past is written once, not also while it waited, {written} \
         times: {}",
        came.journal
    );
    let rows = came.missed["items"].as_array().cloned().unwrap_or_default();
    let id = format!(
        "pineapple-slack-missed-tag-{}-{}",
        (began - 360) * 1000,
        (began - 250) * 1000
    );
    assert!(
        rows.len() == 1 && rows[0]["id"] == id.as_str() && rows[0]["whole"] == true,
        "what it read on past after waiting is one row, whole, from a minute before the last \
         read to the oldest the first page gave, {id}: {:?}",
        came.missed
    );
}

#[test]
fn a_stop_on_a_page_whose_check_goes_unanswered_is_judged_while_the_search_waits() {
    static TAGS: AtomicUsize = AtomicUsize::new(0);
    static BEGAN: AtomicU64 = AtomicU64::new(0);
    let came = hearing(
        |tool, asked, now| {
            let began = began(&BEGAN, now);
            let stop = stop();
            // The stop lies on the second page, past the first's three,
            // and newer than the word the first round read last.
            let found = [
                (began - 60, "a newer word"),
                (began - 200, "a word between"),
                (began - 250, "another word"),
                (began - 280, stop.as_str()),
                (began - 330, "a word found late"),
                (began - 400, "an older word"),
            ];
            if tool != "slack_search_public_and_private" {
                said(tool, now)
            } else if asked["query"] == "pineapple" {
                nothing()
            } else if asked.get("before").is_some() && asked["limit"] == 2 {
                // The second page's ask for one fewer, in every round.
                format!("{FAILS}Slack said: ratelimited")
            } else {
                in_pages(&TAGS, asked, began, &found, 3, true)
            }
        },
        // Its first round's two words and page one's three, over rounds
        // enough to wait twice; a stop no second ask agreed with is
        // journaled, not decided.
        &Until {
            judging: 5,
            asks: 64,
            read_at: None,
        },
    );
    let stopped = came
        .journal
        .find("<5>a stop, in shadow, in an answer no second ask agreed with: ");
    let waits = |rounds: u32| {
        came.journal
            .find(&format!("{rounds} of 10 rounds in a row"))
    };
    assert!(
        waits(1).is_some() && stopped > waits(1) && stopped < waits(2),
        "a stop the second page showed before its check went unanswered is judged in that \
         round: {}",
        came.journal
    );
}

#[test]
fn a_search_that_cant_be_read_is_told_while_the_other_waits_on_a_later_page() {
    static TAGS: AtomicUsize = AtomicUsize::new(0);
    static BEGAN: AtomicU64 = AtomicU64::new(0);
    let came = hearing(
        |tool, asked, now| {
            let began = began(&BEGAN, now);
            if tool != "slack_search_public_and_private" {
                said(tool, now)
            } else if asked["query"] == "pineapple" {
                poisoned()
            } else if asked.get("before").is_some() {
                format!("{FAILS}Slack said: ratelimited")
            } else {
                in_pages(&TAGS, asked, began, &paged(began, "a word"), 3, true)
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
            .contains("search of Slack for pineapple can't be read")
            && began_waiting.is_some()
            && told > began_waiting
            && (waiting.is_none() || told < waiting),
        "the phrase's search, unreadable, is told in its rounds, not held back while the \
         tag's waits on its second page: {}",
        came.journal
    );
}

#[test]
fn a_stop_the_phrase_finds_is_judged_while_the_tags_second_page_goes_unanswered() {
    static TAGS: AtomicUsize = AtomicUsize::new(0);
    static BEGAN: AtomicU64 = AtomicU64::new(0);
    let came = hearing(
        |tool, asked, now| {
            let began = began(&BEGAN, now);
            if tool != "slack_search_public_and_private" {
                said(tool, now)
            } else if asked["query"] == "pineapple" {
                // Found by the phrase alone, from the second round, the
                // first the tag's second page goes unanswered in.
                if TAGS.load(Ordering::SeqCst) == 0 {
                    nothing()
                } else {
                    let stop = stop();
                    tags_answer(1, &[(began - 30, &stop)])
                }
            } else if asked.get("before").is_some() {
                format!("{FAILS}Slack said: ratelimited")
            } else {
                in_pages(&TAGS, asked, began, &paged(began, "a word"), 3, true)
            }
        },
        // As the case before, the phrase's stop judged with the rest.
        &Until {
            judging: 6,
            asks: 64,
            read_at: None,
        },
    );
    let stopped = came.journal.find("<5>a stop, in shadow: ");
    let waits = |rounds: u32| {
        came.journal
            .find(&format!("{rounds} of 10 rounds in a row"))
    };
    assert!(
        waits(1).is_some() && stopped > waits(1) && stopped < waits(2),
        "a stop the phrase's search finds is judged while the tag's search waits on its \
         second page, not once that page is a gap: {}",
        came.journal
    );
}

#[test]
fn an_answer_cut_then_counting_fewer_is_read_in_pages_rather_than_whole() {
    static TAGS: AtomicUsize = AtomicUsize::new(0);
    static BEGAN: AtomicU64 = AtomicU64::new(0);
    let came = hearing(
        |tool, asked, now| {
            let began = began(&BEGAN, now);
            if tool != "slack_search_public_and_private" {
                return said(tool, now);
            }
            if asked["query"] == "pineapple" {
                return nothing();
            }
            let stop = stop();
            let found = [
                (began - 60, stop.as_str()),
                (began - 100, "a newer word"),
                (began - 200, "a word the short answer hides"),
                (began - 300, "a word"),
                (began - 400, "an older word"),
            ];
            let round = TAGS.load(Ordering::SeqCst) + usize::from(opens(asked));
            if round <= 1 || asked.get("before").is_some() {
                return in_pages(&TAGS, asked, began, &found, 20, true);
            }
            if opens(asked) {
                // Counting twenty and giving three, then, asked for the
                // three, counting and giving only the newest two.
                TAGS.fetch_add(1, Ordering::SeqCst);
                tags_answer(20, &found[..3])
            } else {
                tags(asked, &found[..2], 2)
            }
        },
        &Until {
            judging: 5,
            asks: 0,
            read_at: None,
        },
    );
    assert_eq!(
        each(&came.said, "verdict"),
        ["nothing", "nothing", "nothing", "nothing", "stop"],
        "an answer counting fewer than the one it was cut from is no whole answer, so the \
         page before it is read and the word it hid is judged: {}",
        came.journal
    );
    assert!(
        came.journal.contains("read page 2, before")
            && !came
                .journal
                .contains(&format!("search for \"<@{JOEL}>\" can't be read")),
        "read in pages back to the last read, it's no gap: {}",
        came.journal
    );
}

#[test]
fn five_pages_short_of_the_last_read_are_a_gap_told_then_read_on() {
    static TAGS: AtomicUsize = AtomicUsize::new(0);
    static BEGAN: AtomicU64 = AtomicU64::new(0);
    let came = hearing(
        |tool, asked, now| {
            let began = began(&BEGAN, now);
            if tool != "slack_search_public_and_private" {
                return said(tool, now);
            }
            if asked["query"] == "pineapple" {
                return nothing();
            }
            // Eight words since the last read, each page cut to two, so
            // each page past the first adds one and five reach six.
            let flood: Vec<(u64, String)> = (0..8)
                .map(|at| (began - 60 - 10 * at, format!("word {at}")))
                .collect();
            let flood: Vec<(u64, &str)> = flood
                .iter()
                .map(|(at, text)| (*at, text.as_str()))
                .chain([(began - 300, "a word"), (began - 400, "an older word")])
                .collect();
            in_pages(&TAGS, asked, began, &flood, 2, true)
        },
        // A round is three searches, then sixteen once there's a gap:
        // four rounds and the next's first, past the read on, as what the
        // pages gave is judged before it.
        &Until {
            judging: 8,
            asks: 52,
            read_at: None,
        },
    );
    let (told, read_on) = told_and_read_on(&came.journal);
    assert!(
        came.journal.contains("read page 5, before")
            && came.journal.contains("5 pages don't reach it")
            && told.is_some()
            && read_on > told,
        "five pages read, each cut, and still short of the last read, it's a gap, told, \
         then read on: {}",
        came.journal
    );
    assert_eq!(
        each(&came.said, "verdict"),
        ["nothing"; 8],
        "read on, the two words read whole and the six the pages gave are judged, never \
         the two they didn't reach: {}",
        came.journal
    );
}

#[test]
fn a_message_slack_finds_late_well_before_the_last_read_is_judged() {
    static TAGS: AtomicUsize = AtomicUsize::new(0);
    static BEGAN: AtomicU64 = AtomicU64::new(0);
    let came = hearing(
        |tool, asked, now| {
            let began = began(&BEGAN, now);
            if tool != "slack_search_public_and_private" {
                return said(tool, now);
            }
            if asked["query"] == "pineapple" {
                return nothing();
            }
            // Found only after the first round, said three minutes
            // before the last read.
            let stop = stop();
            let found = [
                (began - 60, stop.as_str()),
                (began - 300, "a word"),
                (began - 400, "an older word"),
                (began - 480, "a word found late"),
            ];
            in_pages(&TAGS, asked, began, &found, 20, true)
        },
        &Until {
            judging: 4,
            asks: 0,
            read_at: None,
        },
    );
    assert_eq!(
        each(&came.said, "verdict"),
        ["nothing", "nothing", "nothing", "stop"],
        "the word Slack found late, said well before the last read, is judged beside the \
         stop, as no ask bounds how long after the last read it is found: {}",
        came.journal
    );
    assert!(
        bounds(&came.searched)
            .iter()
            .all(|(after, _)| after.is_null()),
        "no search is bounded after"
    );
}

#[test]
fn a_whole_answer_of_as_many_as_asked_for_is_read_in_pages() {
    static TAGS: AtomicUsize = AtomicUsize::new(0);
    static BEGAN: AtomicU64 = AtomicU64::new(0);
    let came = hearing(
        |tool, asked, now| {
            let began = began(&BEGAN, now);
            if tool != "slack_search_public_and_private" {
                return said(tool, now);
            }
            if asked["query"] == "pineapple" {
                return nothing();
            }
            // Twenty-five words since the last read, each answer giving
            // twenty of twenty, never cut.
            let found: Vec<(u64, &str)> = (0..25).map(|at| (began - 60 - at, "a word")).collect();
            in_pages(&TAGS, asked, began, &found, 20, true)
        },
        &Until {
            judging: 0,
            asks: 8,
            read_at: None,
        },
    );
    assert!(
        bounds(&came.searched)
            .iter()
            .any(|(_, before)| !before.is_null()),
        "an answer giving all twenty it was asked for may not hold all it finds, so the \
         page before it is asked: {:?}",
        came.searched
    );
}

#[test]
fn a_stop_an_unreadable_page_first_gives_in_the_round_that_reads_on_is_kept() {
    static TAGS: AtomicUsize = AtomicUsize::new(0);
    static BEGAN: AtomicU64 = AtomicU64::new(0);
    let came = hearing(
        |tool, asked, now| {
            let began = began(&BEGAN, now);
            if tool != "slack_search_public_and_private" {
                said(tool, now)
            } else if asked["query"] == "pineapple" {
                nothing()
            } else if asked.get("before").is_some() {
                // The second page: typed into while the gap is told, then
                // giving more than it counts, a stop among them, in the
                // round that reads on past it.
                if TAGS.load(Ordering::SeqCst) <= 3 {
                    poisoned()
                } else {
                    let stop = stop();
                    tags_answer(1, &[(began - 260, &stop), (began - 270, "hi")])
                }
            } else {
                let stop = stop();
                in_pages(&TAGS, asked, began, &paged(began, &stop), 3, true)
            }
        },
        // Four rounds and the next's first, as in the case of a page that
        // can't be read.
        &Until {
            judging: 0,
            asks: 19,
            read_at: None,
        },
    );
    let (told, read_on) = told_and_read_on(&came.journal);
    assert!(
        told.is_some() && read_on > told,
        "the gap is told, then read on: {}",
        came.journal
    );
    assert!(
        came.journal
            .contains("<5>a stop, in shadow, in an answer no second ask agreed with: "),
        "and the stop the page it reads on past gave is kept: {}",
        came.journal
    );
}
