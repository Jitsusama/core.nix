//! The reader over a gateway whose answer for a search can't be read: the
//! search is deaf, and what the answer gave is judged as far as it can be
//! trusted. Messages an ask for one fewer agreed with are judged in full,
//! though the span they leave before them stays unread; any other could
//! be typed, so only a stop among them is taken, never a resume. A gateway
//! that keeps cutting is asked again only so often, and its row is what
//! lets a gap be read on.

use std::sync::atomic::AtomicUsize;

use serde_json::{Value, json};

use super::cut::{a_gap, nothing, result, stop, tags, tags_answer, told_and_read_on};
use super::{JOEL, Until, each, hearing, hearing_in, poisoned, said};

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
        came.journal
            .contains("couldn't tell Joel in For You whether Slack is heard")
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
            // as the second of two: asked for twenty, the gateway counts the
            // two it finds and the answer reads whole, but asked for one
            // fewer it counts one, and the typed one is over it.
            let typed = format!(
                "{}\n\n---\n\n{}",
                stop(),
                result(2, 2, now - 30, JOEL, "pineapple resume")
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
