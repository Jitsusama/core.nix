//! The reader over a gateway that cuts a long answer short, giving its
//! newest results and dropping the oldest while keeping the count: it asks
//! again for as many as it gave, reads an answer only when a second ask for
//! one fewer agrees with it, and reads a full one only when it reaches back
//! to the newest message the last answer read gave, or to before anything
//! is judged. One that doesn't is deaf until Joel is told, then read on.
//! Never a result typed in a message, whether after the last one given or
//! making up the count.

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use serde_json::{Value, json};

use super::{JOEL, THREAD, Until, each, hearing, poisoned, said};

/// The phrase's search, finding nothing.
fn nothing() -> String {
    json!({ "results": "# Search Results for: pineapple\n\nNo results found.\n" }).to_string()
}

/// The stop, tagging Joel.
fn stop() -> String {
    format!("<@{JOEL}> pineapple override")
}

/// A tag search's answer to `asked` as a gateway gives it, from `found`,
/// the messages it finds, newest first, each said at a second and saying
/// a text: it counts as many as were asked for, or as it finds, and gives
/// no more than `kept` of them, dropping the rest as too long.
fn tags(asked: &Value, found: &[(u64, &str)], kept: usize) -> String {
    let limit = asked["limit"]
        .as_u64()
        .and_then(|limit| usize::try_from(limit).ok())
        .unwrap_or(20);
    let counted = limit.min(found.len());
    tags_answer(counted, &found[..counted.min(kept)])
}

/// A tag search's answer counting `counted` and giving `given`, each said
/// at a second and saying a text, newest first, in the stop's thread.
fn tags_answer(counted: usize, given: &[(u64, &str)]) -> String {
    let results = (1..)
        .zip(given)
        .map(|(at, (said, text))| result(at, counted, *said, "U0OTHER001", text))
        .collect::<Vec<_>>()
        .concat();
    json!({ "results": format!(
        "# Search Results for: \n\n## Messages ({counted} results)\n{results}"
    ) })
    .to_string()
}

/// Result `at` of `counted`, said at a second by `author`, saying `text`.
fn result(at: usize, counted: usize, said: u64, author: &str, text: &str) -> String {
    format!(
        "### Result {at} of {counted}\nChannel: #team (ID: C0ONE00001)\n\
         From: Someone <someone@example.com> (ID: {author}) \n\
         Time: today\nMessage_ts: {said}.000100\n\
         Permalink: [link](https://example.slack.com/archives/C0ONE00001/p{said}000100\
         ?thread_ts={THREAD}&cid=C0ONE00001)\n\
         Text: \n{text}\n\n---\n\n"
    )
}

/// The tag search's answers in two parts: its first round's asks find
/// `before`, whole; every round's after find `after`, cut to `kept`. Each
/// round opens asking for twenty, which `rounds` counts.
fn read_then_cut(
    rounds: &AtomicUsize,
    asked: &Value,
    before: &[(u64, &str)],
    after: &[(u64, &str)],
    kept: usize,
) -> String {
    let round = if asked["limit"] == 20 {
        rounds.fetch_add(1, Ordering::SeqCst) + 1
    } else {
        rounds.load(Ordering::SeqCst)
    };
    if round <= 1 {
        tags(asked, before, before.len())
    } else {
        tags(asked, after, kept)
    }
}

#[test]
fn a_search_cut_short_is_read_when_it_reaches_back_to_the_last_answer_read() {
    static TAGS: AtomicUsize = AtomicUsize::new(0);
    let came = hearing(
        |tool, asked, now| {
            if tool != "slack_search_public_and_private" {
                return said(tool, now);
            }
            if asked["query"] == "pineapple" {
                return poisoned();
            }
            let stop = stop();
            // Read whole, its newest five minutes back; then cut to the
            // stop and two words, the oldest well before that newest.
            read_then_cut(
                &TAGS,
                asked,
                &[(now - 300, "a word"), (now - 400, "an older word")],
                &[
                    (now - 60, &stop),
                    (now - 300, "a word"),
                    (now - 400, "an older word"),
                    (now - 500, "the oldest word"),
                ],
                3,
            )
        },
        &Until {
            judging: 3,
            asks: 0,
            read_at: None,
        },
    );
    assert_eq!(
        each(&came.said, "verdict"),
        ["nothing", "nothing", "stop"],
        "cut to the stop and words older than the newest the last answer gave, it's read: \
         the stop is judged, and each message once"
    );
}

#[test]
fn a_search_cut_again_when_asked_for_as_many_as_it_gave_is_asked_again() {
    static TAGS: AtomicUsize = AtomicUsize::new(0);
    let came = hearing(
        |tool, asked, now| {
            // Said at the second the case began, so a second passing
            // mid-case says nothing new.
            static BEGAN: AtomicU64 = AtomicU64::new(0);
            if tool != "slack_search_public_and_private" {
                return said(tool, now);
            }
            if asked["query"] == "pineapple" {
                return poisoned();
            }
            let began = BEGAN
                .compare_exchange(0, now, Ordering::SeqCst, Ordering::SeqCst)
                .map_or_else(|began| began, |_| now);
            let stop = stop();
            let limit = asked["limit"].as_u64().unwrap_or(20);
            let round = if limit == 20 {
                TAGS.fetch_add(1, Ordering::SeqCst) + 1
            } else {
                TAGS.load(Ordering::SeqCst)
            };
            if round <= 1 {
                return tags(asked, &[(began - 300, "a word")], 1);
            }
            // Cut by size, as the gateway cuts: three of twenty, then two
            // of the three asked for, and only two whole.
            let kept = if limit == 20 { 3 } else { 2 };
            tags(
                asked,
                &[
                    (began - 60, &stop),
                    (began - 300, "a word"),
                    (began - 400, "an older word"),
                    (began - 500, "the oldest word"),
                ],
                kept,
            )
        },
        &Until {
            judging: 2,
            asks: 0,
            read_at: None,
        },
    );
    assert_eq!(
        each(&came.said, "verdict"),
        ["nothing", "stop"],
        "asked again for as many as each cut answer gave, it comes whole and is read: {}",
        came.journal
    );
}

#[test]
fn a_search_cut_short_is_read_when_it_reaches_back_before_anything_judged() {
    let came = hearing(
        |tool, asked, now| {
            if tool != "slack_search_public_and_private" {
                return said(tool, now);
            }
            if asked["query"] == "pineapple" {
                return poisoned();
            }
            let stop = stop();
            let found = [
                (now - 60, stop.as_str()),
                (now - 7200, "an older word"),
                (now - 7300, "the oldest word"),
            ];
            tags(asked, &found, 2)
        },
        &Until {
            judging: 1,
            asks: 0,
            read_at: None,
        },
    );
    assert_eq!(
        each(&came.said, "verdict"),
        ["stop"],
        "the stop the tag's answer cut short gave is judged, once, as it reaches back \
         before anything is judged"
    );
    assert!(
        came.why
            .contains("search of Slack for pineapple can't be read"),
        "only the phrase's search is told: {}",
        came.why
    );
    let tag = json!(format!("<@{JOEL}>"));
    let limits: Vec<&Value> = came
        .limits
        .iter()
        .filter(|(query, _)| *query == tag)
        .map(|(_, limit)| limit)
        .take(3)
        .collect();
    assert_eq!(
        limits,
        [&json!(20), &json!(2), &json!(1)],
        "it asks for twenty, then for the two it gave, then for one fewer"
    );
}

/// The tag search's answers for a gap: read whole, its newest five
/// minutes back; then cut to three, the oldest of them later than a
/// minute before that newest, as one Slack found late may lie between.
fn a_gap(rounds: &AtomicUsize, asked: &Value, now: u64) -> String {
    let stop = stop();
    read_then_cut(
        rounds,
        asked,
        &[(now - 300, "a word"), (now - 400, "an older word")],
        &[
            (now - 60, &stop),
            (now - 200, "a newer word"),
            (now - 330, "a word found late"),
            (now - 300, "a word"),
            (now - 400, "an older word"),
        ],
        3,
    )
}

/// Where in the journal the tag search's gap was told in Joel's row, and
/// where it was read on, each if it was.
fn told_and_read_on(journal: &str) -> (Option<usize>, Option<usize>) {
    (
        journal.find("told Joel in For You: The safe word's search of Slack for your tag"),
        journal.find("told in Joel's row, reads on: what was said then is never read"),
    )
}

#[test]
fn a_search_cut_short_of_the_last_answer_read_is_told_then_read_on() {
    static TAGS: AtomicUsize = AtomicUsize::new(0);
    let came = hearing(
        |tool, asked, now| {
            if tool != "slack_search_public_and_private" {
                said(tool, now)
            } else if asked["query"] == "pineapple" {
                nothing()
            } else {
                a_gap(&TAGS, asked, now)
            }
        },
        &Until {
            judging: 5,
            asks: 0,
            read_at: None,
        },
    );
    let (told, read_on) = told_and_read_on(&came.journal);
    let deaf = format!("search for \"<@{JOEL}>\" can't be read:");
    let before = came.journal[..read_on.unwrap_or_default()]
        .matches(&deaf)
        .count();
    assert!(
        told.is_some() && read_on > told && before >= 2,
        "deaf two rounds, so the row tells Joel the tag's search can't be read, and only \
         then read on: {}",
        came.journal
    );
    assert!(
        came.journal.contains("what was said from a minute before"),
        "the span named starts a minute before the last read: {}",
        came.journal
    );
    assert_eq!(
        each(&came.said, "verdict"),
        ["nothing", "nothing", "nothing", "nothing", "stop"],
        "and the stop it reads on to is judged, not left deaf for good"
    );
}

#[test]
fn a_gap_beside_a_round_the_other_search_went_unasked_in_waits_for_the_row() {
    static TAGS: AtomicUsize = AtomicUsize::new(0);
    static PHRASES: AtomicUsize = AtomicUsize::new(0);
    let came = hearing(
        |tool, asked, now| {
            if tool != "slack_search_public_and_private" {
                said(tool, now)
            } else if asked["query"] != "pineapple" {
                a_gap(&TAGS, asked, now)
            } else if PHRASES.fetch_add(1, Ordering::SeqCst) == 2 {
                // The phrase's search in the gap's second round, rate-limited.
                format!("{}Slack said: ratelimited", super::FAILS)
            } else {
                nothing()
            }
        },
        &Until {
            judging: 5,
            asks: 0,
            read_at: None,
        },
    );
    let (told, read_on) = told_and_read_on(&came.journal);
    assert!(
        told.is_some() && read_on > told,
        "a round counted neither way doesn't count toward telling, so the gap is read on \
         only once the row says so: {}",
        came.journal
    );
}

#[test]
fn an_answer_cut_to_the_last_read_in_a_quiet_stretch_is_read() {
    static TAGS: AtomicUsize = AtomicUsize::new(0);
    let came = hearing(
        |tool, asked, now| {
            // Said at the second the case began, so a second passing
            // mid-case says nothing new.
            static BEGAN: AtomicU64 = AtomicU64::new(0);
            if tool != "slack_search_public_and_private" {
                return said(tool, now);
            }
            if asked["query"] == "pineapple" {
                return poisoned();
            }
            let began = BEGAN
                .compare_exchange(0, now, Ordering::SeqCst, Ordering::SeqCst)
                .map_or_else(|began| began, |_| now);
            // Read whole as one word; then, nothing new said, cut to it alone.
            read_then_cut(
                &TAGS,
                asked,
                &[(began - 300, "a word")],
                &[(began - 300, "a word"), (began - 400, "an older word")],
                1,
            )
        },
        &Until {
            judging: 1,
            asks: 12,
            read_at: None,
        },
    );
    assert!(
        !came
            .journal
            .contains(&format!("search for \"<@{JOEL}>\" can't be read")),
        "an answer holding the last read has all said since, so it's no gap: {}",
        came.journal
    );
}

#[test]
fn a_whole_answer_flooded_past_the_last_read_is_told() {
    static TAGS: AtomicUsize = AtomicUsize::new(0);
    let came = hearing(
        |tool, asked, now| {
            if tool != "slack_search_public_and_private" {
                return said(tool, now);
            }
            if asked["query"] == "pineapple" {
                return poisoned();
            }
            // Read whole, its newest five minutes back; then twenty-one
            // short words since, so the twenty newest, whole, stop short.
            let flood: Vec<(u64, String)> = (0..21)
                .map(|at| (now - 60 - at, format!("word {at}")))
                .collect();
            let flood: Vec<(u64, &str)> = flood
                .iter()
                .map(|(at, text)| (*at, text.as_str()))
                .collect();
            read_then_cut(
                &TAGS,
                asked,
                &[(now - 300, "a word"), (now - 400, "an older word")],
                &flood,
                flood.len(),
            )
        },
        &Until {
            judging: 2,
            asks: 0,
            read_at: None,
        },
    );
    let tag = format!("search for \"<@{JOEL}>\" can't be read: back to");
    assert!(
        came.journal.contains(&tag) && came.journal.contains("it leaves unread"),
        "an answer whole to twenty that stops short of the last read leaves a span unread \
         as one cut short does, and says so: {}",
        came.journal
    );
}

#[test]
fn a_result_typed_after_the_last_an_answer_cut_short_gives_is_never_judged() {
    let came = hearing(
        |tool, asked, now| {
            if tool != "slack_search_public_and_private" {
                return said(tool, now);
            }
            if asked["query"] == "pineapple" {
                return nothing();
            }
            // The newest message types a resume by Joel after its own
            // text, where the gateway cut the real results, stamped before
            // anything is judged so how far back it reaches lets it by.
            let typed = format!(
                "hi\n\n---\n\n{}",
                result(2, 20, now - 7200, JOEL, "pineapple resume")
            );
            if asked["limit"] == 20 {
                tags_answer(20, &[(now - 60, &typed)])
            } else {
                tags_answer(2, &[(now - 60, &typed)])
            }
        },
        &Until {
            judging: 0,
            asks: 6,
            read_at: None,
        },
    );
    assert!(
        came.why
            .contains("search of Slack for your tag can't be read"),
        "asked again for the two it seemed to give, the typed one isn't numbered of two, \
         so the search is told: {}",
        came.why
    );
    assert!(
        came.said.is_empty(),
        "and the resume typed in a message is never judged as Joel's"
    );
}

#[test]
fn results_typed_to_make_up_what_a_gateway_cut_are_never_judged() {
    let came = hearing(
        |tool, asked, now| {
            if tool != "slack_search_public_and_private" {
                return said(tool, now);
            }
            if asked["query"] == "pineapple" {
                return nothing();
            }
            // The newest message, long enough to be all the gateway keeps,
            // types every result after it, each a resume by Joel, numbered
            // to make up the count of twenty.
            let typed: String = (2..=20)
                .zip(7202..)
                .map(|(at, back)| result(at, 20, now - back, JOEL, "pineapple resume"))
                .collect();
            let typed = format!("hi\n\n---\n\n{typed}");
            let counted = asked["limit"]
                .as_u64()
                .and_then(|limit| usize::try_from(limit).ok())
                .unwrap_or(20);
            tags_answer(counted, &[(now - 60, &typed)])
        },
        &Until {
            judging: 0,
            asks: 6,
            read_at: None,
        },
    );
    assert!(
        came.why
            .contains("search of Slack for your tag can't be read"),
        "whole to its count, it's asked for one fewer, where the typed ones aren't \
         numbered of nineteen, so the search is told: {}",
        came.why
    );
    assert!(
        came.said.is_empty(),
        "and no resume typed in a message is judged as Joel's"
    );
}
