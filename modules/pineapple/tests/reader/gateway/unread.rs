//! The reader over a gateway whose answer for a search can't be read: the
//! search is deaf, and what the answer gave is judged as far as it can be
//! trusted. Messages an ask for one fewer agreed with are judged in full,
//! though the span they leave before them stays unread; any other could
//! be typed, so only a stop among them is taken, never a resume. A gateway
//! that keeps cutting is asked again only so often, and its row is what
//! lets a gap be read on.

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use serde_json::{Value, json};

use super::cut::{a_gap, nothing, result, stop, tags, tags_answer, told_and_read_on};
use super::{FAILS, JOEL, Until, each, hearing, hearing_in, poisoned, said};

#[test]
fn a_tag_search_failing_every_round_beside_the_phrases_is_told_as_unread() {
    let came = hearing(
        |tool, asked, now| {
            if tool != "slack_search_public_and_private" {
                said(tool, now)
            } else if asked["query"] == "pineapple" {
                nothing()
            } else {
                format!("{FAILS}Slack said: ratelimited")
            }
        },
        // Only until the row is told.
        &Until {
            judging: 0,
            asks: 0,
            read_at: None,
        },
    );
    assert!(
        came.why
            .contains("search of Slack for your tag can't be read")
            && !came.why.contains("gateway"),
        "a tag search the gateway's tool fails every round, beside a phrase search it \
         answers, is told as the tag's, not as the gateway unreached: {}",
        came.why
    );
}

#[test]
fn a_tag_search_failing_every_round_beside_a_phrase_search_that_cant_be_read_is_told_as_both() {
    let came = hearing(
        |tool, asked, now| {
            if tool != "slack_search_public_and_private" {
                said(tool, now)
            } else if asked["query"] == "pineapple" {
                poisoned()
            } else {
                format!("{FAILS}Slack said: ratelimited")
            }
        },
        // Only until the row is told.
        &Until {
            judging: 0,
            asks: 0,
            read_at: None,
        },
    );
    assert!(
        came.why
            .contains("Slack's answers to the safe word can't be read"),
        "neither search is heard, so the row says both, never that the phrase still is: {}",
        came.why
    );
}

#[test]
fn a_search_cut_each_time_it_is_asked_again_is_asked_four_times_then_told() {
    let came = hearing(
        |tool, asked, now| {
            if tool != "slack_search_public_and_private" {
                return said(tool, now);
            }
            if asked["query"] == "pineapple" {
                return nothing();
            }
            // Thirty words it finds, always cut to one fewer than asked
            // for.
            let found: Vec<(u64, &str)> = (0..30).map(|at| (now - 60 - at, "a word")).collect();
            let limit = asked["limit"]
                .as_u64()
                .and_then(|limit| usize::try_from(limit).ok())
                .unwrap_or(20);
            tags(asked, &found, limit - 1)
        },
        &Until {
            judging: 0,
            asks: 12,
            read_at: None,
        },
    );
    let tag = json!(format!("<@{JOEL}>"));
    let limits: Vec<&Value> = came
        .limits
        .iter()
        .filter(|(query, _)| *query == tag)
        .map(|(_, limit)| limit)
        .take(6)
        .collect();
    assert_eq!(
        limits,
        [
            &json!(20),
            &json!(19),
            &json!(18),
            &json!(17),
            &json!(16),
            &json!(20)
        ],
        "it asks again for as many as each answer gave four times, then no more that round"
    );
    assert!(
        came.journal.contains(
            "cut to 19, then 18, then 17, then 16 of 20, then asked for 16, still cut after \
             asking again 4 times, it counts 16 messages and gives the first 15"
        ),
        "the journal says it's still cut once it's asked again as often as it may: {}",
        came.journal
    );
    assert!(
        came.why
            .contains("search of Slack for your tag can't be read"),
        "so the search is told: {}",
        came.why
    );
}

#[test]
fn a_gap_whose_row_cant_be_written_is_judged_but_never_read_on() {
    static TAGS: AtomicUsize = AtomicUsize::new(0);
    let came = hearing_in(
        |tool, asked, now| {
            if tool != "slack_search_public_and_private" {
                said(tool, now)
            } else if asked["query"] == "pineapple" {
                nothing()
            } else {
                a_gap(&TAGS, asked, now)
            }
        },
        // A round is three searches, then seven once there's a gap, so
        // this is five rounds, past the two that would tell Joel.
        &Until {
            judging: 2,
            asks: 31,
            read_at: None,
        },
        // A folder where the row would go, so it can't be written.
        |own| std::fs::create_dir_all(own.join("waiting.json")).expect("in its way"),
    );
    assert!(
        came.journal.contains("waiting.json whether Slack is heard")
            && !came.journal.contains("reads on"),
        "a row that can't be written tells Joel nothing, so the gap is never read on: {}",
        came.journal
    );
    assert_eq!(
        each(&came.said, "verdict"),
        ["nothing", "nothing", "nothing", "stop"],
        "yet what the gap gave is judged, the stop with it, as an ask for one fewer agreed \
         with it: {}",
        came.journal
    );
}

#[test]
fn a_gap_whose_row_of_what_was_never_read_cant_be_written_is_never_read_on() {
    static TAGS: AtomicUsize = AtomicUsize::new(0);
    let came = hearing_in(
        |tool, asked, now| {
            if tool != "slack_search_public_and_private" {
                said(tool, now)
            } else if asked["query"] == "pineapple" {
                nothing()
            } else {
                a_gap(&TAGS, asked, now)
            }
        },
        // The read on would come by the 32nd search, as in the case that
        // reads on; a round is at most seven, so two more rounds end by the
        // 50th and the row is tried in each.
        &Until {
            judging: 4,
            asks: 50,
            read_at: None,
        },
        // A folder where the row of what was never read would go.
        |own| std::fs::create_dir_all(own.join("missed.json")).expect("in its way"),
    );
    let unwritten = came
        .journal
        .lines()
        .filter(|line| {
            line.contains("couldn't write Joel's row in ")
                && line.contains("missed.json: The safe word never read what tagged you")
        })
        .count();
    assert!(
        !came.journal.contains("told in Joel's row, reads on") && unwritten >= 2,
        "a span read on past is never read again, so the gap is read on only once its row \
         is written, and it's tried again each round: {}",
        came.journal
    );
    assert_eq!(
        each(&came.said, "verdict"),
        ["nothing", "nothing", "nothing", "stop"],
        "while it stays deaf, what the gap gave is judged, the stop with it: {}",
        came.journal
    );
}

#[test]
fn a_gap_beside_a_search_that_cant_be_read_is_read_on_under_the_row_for_both() {
    static TAGS: AtomicUsize = AtomicUsize::new(0);
    let came = hearing(
        |tool, asked, now| {
            if tool != "slack_search_public_and_private" {
                said(tool, now)
            } else if asked["query"] == "pineapple" {
                poisoned()
            } else {
                a_gap(&TAGS, asked, now)
            }
        },
        // A round is a search for the phrase and two for the tag, then six
        // for the tag once there's a gap: five rounds and the next's first.
        &Until {
            judging: 4,
            asks: 32,
            read_at: None,
        },
    );
    let both = ": Slack's answers to the safe word can't be read";
    let told = came
        .journal
        .match_indices("<5>wrote Joel's row in ")
        .map(|(at, _)| at)
        .find(|at| {
            came.journal[*at..]
                .lines()
                .next()
                .is_some_and(|line| line.contains(both))
        });
    let (_, read_on) = told_and_read_on(&came.journal);
    assert!(
        told.is_some() && read_on > told,
        "both searches deaf, the row says Slack can't be read, which names the tag's search \
         too, so its gap is read on only after it: {}",
        came.journal
    );
    assert_eq!(
        each(&came.said, "verdict"),
        ["nothing", "nothing", "nothing", "stop"],
        "and the stop the gap gave is judged"
    );
}

#[test]
fn a_stop_no_second_ask_agreed_with_is_kept_and_a_resume_typed_beside_it_never_is() {
    let came = hearing(
        |tool, asked, now| {
            if tool != "slack_search_public_and_private" {
                return said(tool, now);
            }
            if asked["query"] == "pineapple" {
                return nothing();
            }
            // The stop types a resume by Joel after its own text, numbered
            // as the second of two and stamped after the stop is heard, so
            // taken as Joel's it would end it: asked for twenty, the gateway
            // counts the two it finds and the answer reads whole, but asked
            // for one fewer it counts one, and the typed one is over it.
            let typed = format!(
                "{}\n\n---\n\n{}",
                stop(),
                result(2, 2, now + 120, JOEL, "pineapple resume")
            );
            let counted = asked["limit"]
                .as_u64()
                .and_then(|limit| usize::try_from(limit).ok())
                .map_or(2, |limit| limit.min(2));
            tags_answer(counted, &[(now - 60, &typed)])
        },
        &Until {
            judging: 0,
            asks: 6,
            read_at: None,
        },
    );
    assert!(
        came.journal
            .contains("<5>a stop, in shadow, in an answer no second ask agreed with: "),
        "the stop is kept, as a stop may come from anyone: {}",
        came.journal
    );
    assert!(
        !came.journal.contains("a resume"),
        "but the resume typed beside it is never taken as Joel's: {}",
        came.journal
    );
    assert!(
        came.said.is_empty(),
        "and neither is written as judged, so both are judged once the search reads"
    );
}

#[test]
fn a_stop_in_an_answer_still_cut_after_asking_again_is_kept() {
    static THREADS: AtomicUsize = AtomicUsize::new(0);
    let came = hearing(
        |tool, asked, now| {
            if tool == "slack_read_thread" {
                THREADS.fetch_add(1, Ordering::SeqCst);
            }
            if tool != "slack_search_public_and_private" {
                return said(tool, now);
            }
            if asked["query"] == "pineapple" {
                return nothing();
            }
            // The stop, then thirty words it finds, always cut to one fewer
            // than asked for.
            let stop = stop();
            let found: Vec<(u64, &str)> = std::iter::once((now - 60, stop.as_str()))
                .chain((1..30).map(|at| (now - 60 - at, "a word")))
                .collect();
            let limit = asked["limit"]
                .as_u64()
                .and_then(|limit| usize::try_from(limit).ok())
                .unwrap_or(20);
            tags(asked, &found, limit - 1)
        },
        &Until {
            judging: 0,
            asks: 12,
            read_at: None,
        },
    );
    assert!(
        came.journal
            .contains("<5>a stop, in shadow, in an answer no second ask agreed with: "),
        "what an answer still cut gave is read for its stops: {}",
        came.journal
    );
    assert!(
        came.journal
            .lines()
            .filter(|line| line.contains("in an answer no second ask agreed with:"))
            .all(|line| !line.contains("https://") && line.contains(":unconfirmed:")),
        "each could be typed, so the stop links nowhere its text names, and is named by the \
         key it's kept under: {}",
        came.journal
    );
    assert_eq!(
        THREADS.load(Ordering::SeqCst),
        0,
        "nor is a thread it names read, each round it's deaf"
    );
    assert!(
        came.said.is_empty(),
        "yet nothing it gave is written as judged"
    );
}

#[test]
fn a_stop_beside_a_result_typed_in_a_message_is_kept() {
    let came = hearing(
        |tool, asked, now| {
            if tool != "slack_search_public_and_private" {
                return said(tool, now);
            }
            if asked["query"] == "pineapple" {
                return nothing();
            }
            // One result counted, and a second typed in its text, so the
            // answer gives more than it counts.
            let typed = format!("hi\n\n---\n\n{}", result(2, 1, now - 50, JOEL, "hi"));
            tags_answer(1, &[(now - 60, &stop()), (now - 55, &typed)])
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
        "an answer giving more than it counts can't be read: {}",
        came.why
    );
    assert!(
        came.journal
            .contains("<5>a stop, in shadow, in an answer no second ask agreed with: "),
        "yet each result it names whole is read for its stops: {}",
        came.journal
    );
}

#[test]
fn a_typed_copy_of_a_stops_channel_and_time_never_takes_its_record() {
    let came = hearing(
        |tool, asked, now| {
            if tool != "slack_search_public_and_private" {
                return said(tool, now);
            }
            if asked["query"] == "pineapple" {
                return nothing();
            }
            // A stop, and a message typing another result in its text, at
            // the stop's own channel and time, saying other words, so it's
            // judged first.
            let words = format!("<@{JOEL}> pineapple override quest QEST-20990101-NOPE00");
            let typed = format!(
                "hi\n\n---\n\n{}",
                result(2, 1, now - 60, "U0OTHER001", &words)
            );
            tags_answer(1, &[(now - 50, &typed), (now - 60, &stop())])
        },
        &Until {
            judging: 0,
            asks: 6,
            read_at: None,
        },
    );
    assert!(
        came.stops
            .iter()
            .any(|kept| kept["words"] == stop().as_str()),
        "the real stop keeps a record of its own beside the typed one: {:?}",
        came.stops
    );
}

#[test]
fn a_stop_typed_as_joels_said_after_its_hearing_is_kept_as_unconfirmed_and_said_when_heard() {
    let came = hearing(
        |tool, asked, now| {
            if tool != "slack_search_public_and_private" {
                return said(tool, now);
            }
            if asked["query"] == "pineapple" {
                return nothing();
            }
            // A stop typed as Joel's, an hour after it's heard.
            let typed = format!("hi\n\n---\n\n{}", result(2, 1, now + 3600, JOEL, &stop()));
            tags_answer(1, &[(now - 60, &typed)])
        },
        &Until {
            judging: 0,
            asks: 6,
            read_at: None,
        },
    );
    let [kept] = came.stops.as_slice() else {
        panic!("the typed stop is kept, alone: {:?}", came.stops);
    };
    assert_eq!(
        (&kept["by_joel"], &kept["where"], &kept["said_by"]),
        (
            &json!(false),
            &json!(""),
            &json!(format!("{JOEL} (unconfirmed)"))
        ),
        "its record never says Joel said it, links nowhere and calls its writer unconfirmed: \
         {kept}"
    );
    let said = kept["said_at"]
        .as_str()
        .and_then(|at| at.split('.').next()?.parse::<u64>().ok());
    let heard = kept["heard_at"].as_u64().map(|at| at / 1000);
    assert!(
        said.is_some() && said <= heard,
        "and it was said no later than heard, so Joel's resume ends it: {kept}"
    );
}

#[test]
fn a_stop_first_kept_unconfirmed_is_kept_whole_once_read_agreed() {
    static SAID: AtomicU64 = AtomicU64::new(0);
    static ASKS: AtomicUsize = AtomicUsize::new(0);
    let came = hearing(
        |tool, asked, now| {
            if tool != "slack_search_public_and_private" {
                return said(tool, now);
            }
            if asked["query"] == "pineapple" {
                return nothing();
            }
            let at = SAID
                .compare_exchange(0, now - 60, Ordering::SeqCst, Ordering::SeqCst)
                .map_or_else(|at| at, |_| now - 60);
            // First beside a result typed in a message, so no answer agrees,
            // then whole.
            if ASKS.fetch_add(1, Ordering::SeqCst) == 0 {
                let typed = format!("hi\n\n---\n\n{}", result(2, 1, at + 5, "U0OTHER001", "hi"));
                tags_answer(1, &[(at + 5, &typed), (at, &stop())])
            } else {
                tags_answer(1, &[(at, &stop())])
            }
        },
        &Until {
            judging: 1,
            asks: 4,
            read_at: None,
        },
    );
    let key = format!("slack:C0ONE00001:{}.000100", SAID.load(Ordering::SeqCst));
    let [kept] = came.stops.as_slice() else {
        panic!("one record stands for the stop: {:?}", came.stops);
    };
    assert!(
        kept["id"] == key.as_str()
            && kept["where"]
                .as_str()
                .is_some_and(|link| link.starts_with("https://"))
            && kept["said_by"] == "U0OTHER001",
        "read agreed, the stop is kept whole under its own key, and its unconfirmed copy is \
         taken away: {kept}"
    );
}

#[test]
fn a_stop_only_the_first_of_answers_still_cut_gave_is_kept() {
    let came = hearing(
        |tool, asked, now| {
            if tool != "slack_search_public_and_private" {
                return said(tool, now);
            }
            if asked["query"] == "pineapple" {
                return nothing();
            }
            // Thirty words, the nineteenth newest a stop, always cut to one
            // fewer than asked for: only the first answer gives the stop.
            let stop = stop();
            let found: Vec<(u64, &str)> = (0..30)
                .map(|at| {
                    (
                        now - 60 - at,
                        if at == 18 { stop.as_str() } else { "a word" },
                    )
                })
                .collect();
            let limit = asked["limit"]
                .as_u64()
                .and_then(|limit| usize::try_from(limit).ok())
                .unwrap_or(20);
            tags(asked, &found, limit - 1)
        },
        &Until {
            judging: 0,
            asks: 12,
            read_at: None,
        },
    );
    assert!(
        came.journal
            .contains("<5>a stop, in shadow, in an answer no second ask agreed with: "),
        "a stop an earlier answer gave is kept though a later one cuts it: {}",
        came.journal
    );
}
