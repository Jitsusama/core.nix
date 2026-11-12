//! What a go at something in pi lost Joel, from the turns read from its
//! log: the time the provider failed him and pi's own share (its queue,
//! tools and the preparing of calls) past a second, the whole of a go he
//! stopped or had to say again, and never the model's thinking or writing,
//! which is no fault.

use super::{Stop, Turn};
use crate::pain::{Painful, PiSigns, Signs, tenths};

/// Past a second of pi's own share of a turn (its queue, tools and the
/// preparing of calls) his flow is broken; that second is the spec, and
/// every one past it is lost. The model's thinking and writing isn't.
pub const REPLY_MS: i64 = 1_000;
/// A go in pi lost him this at most: past two minutes he was most likely
/// doing something else meanwhile, as with a search he left in the
/// launcher, so a reply that hung for ten loses him two.
pub const MOST_MS: i64 = 120_000;
/// pi hands a command still running at a minute to the background, so
/// the session is his again; a tool that holds it between calls longer
/// than that is pi's fault past the minute, and what runs within it is
/// the work he asked for.
pub const HANDED_MS: i64 = 60_000;

/// The day's turns that lost him at least a second, as the day file says
/// them: each a turn, or turns in a row where each said the last again.
/// `logs` are each session's turns, in order; only goes begun in
/// [since, until) count.
pub fn painful(logs: &[&[Turn]], since: i64, until: i64) -> Vec<Painful> {
    let mut named = Vec::new();
    for turns in logs {
        let mut goes: Vec<Vec<&Turn>> = Vec::new();
        for turn in *turns {
            match goes.last_mut() {
                Some(go) if turn.again => go.push(turn),
                _ => goes.push(vec![turn]),
            }
        }
        named.extend(
            goes.iter()
                .filter(|go| (since..until).contains(&go[0].enter))
                .map(|go| interaction(go))
                .filter(|p| p.lost_seconds >= 1.0),
        );
    }
    named
}

/// One go at something in a session, as the day file says it.
fn interaction(go: &[&Turn]) -> Painful {
    let (first, last) = (go[0], go[go.len() - 1]);
    let mut signs = PiSigns::default();
    let ms = |ms: i64| crate::pain::whole(crate::pain::float(ms));
    for turn in go {
        let waited = turn.calls.first().map_or(0, |c| c.start - turn.enter);
        signs.waited_ms = signs.waited_ms.max(ms(waited));
        signs.queued_ms = signs.queued_ms.max(ms(turn.delivered - turn.enter));
        let errors = turn.calls.iter().filter(|c| c.stop == Stop::Error).count();
        signs.errors += u32::try_from(errors).unwrap_or(u32::MAX);
        signs.stopped += u32::from(turn.calls.iter().any(|c| c.stop == Stop::Aborted));
        signs.corrections += u32::from(turn.correction);
        signs.status += u32::from(turn.status);
    }
    // Each go but the last didn't do it, so its time up to the next went
    // for nothing; the last loses what it waited on that wasn't the model
    // thinking or writing, which is no fault: the provider failing, and
    // pi's own share, its queue, tools and preparing of calls.
    let again: i64 = go.windows(2).map(|w| w[1].enter - w[0].enter).sum();
    let calls = &last.calls;
    let model: i64 = calls
        .iter()
        .filter(|c| c.stop != Stop::Error)
        .map(|c| c.ms)
        .sum();
    let error: i64 = calls
        .iter()
        .enumerate()
        .filter(|(_, c)| c.stop == Stop::Error)
        .map(|(n, c)| calls.get(n + 1).map_or(c.start + c.ms, |next| next.start) - c.start)
        .sum();
    let own = (last.end() - last.enter - model - error - last.his_ms).max(0);
    let waiting = (own - REPLY_MS).max(0);
    signs.model_ms = ms(model);
    signs.provider_ms = ms(error);
    signs.pi_ms = ms(own);
    signs.tools_ms = ms(last.tools_ms);
    let stopped = calls
        .iter()
        .find(|c| c.stop == Stop::Aborted)
        .map_or(0, |c| {
            (c.start + c.ms - last.enter - waiting - error).max(0)
        });
    let part = [
        (waiting, "pi"),
        (error, "provider"),
        (stopped, "stopped"),
        (again, "again"),
    ]
    .into_iter()
    .fold(
        (i64::MIN, "pi"),
        |best, it| if it.0 > best.0 { it } else { best },
    )
    .1;
    let slowed = match part {
        _ if go.len() > 1 => "tried again",
        "stopped" => "stopped it",
        "provider" => "an error",
        _ => "waiting",
    };
    let float = crate::pain::float;
    Painful {
        at: first.enter / 1000,
        place: "pi",
        trying: "a session".to_string(),
        seconds: tenths(float(last.end() - first.enter)),
        usual_seconds: tenths(float(REPLY_MS)),
        lost_seconds: tenths(float((again + waiting + error + stopped).min(MOST_MS))),
        rank: 0,
        slowed,
        part,
        openings: go.len(),
        signs: Signs::Pi(signs),
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::{AT, turn};
    use super::*;

    #[test]
    fn waiting_past_a_second_for_the_reply_is_lost() {
        let turns = [turn(AT, &[(AT + 21_000, 4000, Stop::Done)])];
        let named = painful(&[&turns], AT - 1, AT + 1);
        assert_eq!(named.len(), 1, "a 21 s wait is named");
        let p = &named[0];
        assert_eq!((p.place, p.trying.as_str()), ("pi", "a session"));
        assert_eq!(
            (p.lost_seconds, p.seconds, p.usual_seconds),
            (20.0, 25.0, 1.0)
        );
        assert_eq!((p.slowed, p.part, p.openings), ("waiting", "pi", 1));
        let quick = [turn(AT, &[(AT + 900, 30_000, Stop::Done)])];
        assert!(
            painful(&[&quick], AT - 1, AT + 1).is_empty(),
            "a reply that began within a second lost him nothing, however long it ran"
        );
    }

    #[test]
    fn tools_between_calls_lose_only_what_they_hold_past_a_minute() {
        let work = [turn(
            AT,
            &[
                (AT + 500, 1000, Stop::Tool),
                (AT + 51_500, 1000, Stop::Tool),
                (AT + 52_900, 1000, Stop::Done),
            ],
        )];
        assert!(
            painful(&[&work], AT - 1, AT + 1).is_empty(),
            "tools within a minute of a call are the work he asked for"
        );
        let held = [turn(
            AT,
            &[
                (AT + 500, 1000, Stop::Tool),
                (AT + 91_500, 1000, Stop::Done),
            ],
        )];
        let p = &painful(&[&held], AT - 1, AT + 1)[0];
        assert_eq!(
            (p.lost_seconds, p.part),
            (30.0, "pi"),
            "held 30 s past the minute"
        );
        let mut asked = held[0].clone();
        asked.his_ms = 90_000;
        assert!(
            painful(&[&[asked]], AT - 1, AT + 1).is_empty(),
            "a question put to him is his time"
        );
    }

    #[test]
    fn a_nudge_loses_only_what_its_first_reply_kept_him_waiting() {
        let mut nudge = turn(
            AT,
            &[
                (AT + 5000, 1000, Stop::Tool),
                (AT + 600_000, 1000, Stop::Aborted),
            ],
        );
        nudge.nudge = true;
        let p = &painful(&[&[nudge.clone()]], AT - 1, AT + 1)[0];
        assert_eq!(
            (p.lost_seconds, p.seconds, p.part),
            (4.0, 6.0, "pi"),
            "what the session went on to do was handed to it"
        );
        nudge.calls[0].start = AT + 500;
        assert!(
            painful(&[&[nudge]], AT - 1, AT + 1).is_empty(),
            "answered within a second"
        );
    }

    #[test]
    fn an_error_loses_the_time_until_the_next_try() {
        let turns = [turn(
            AT,
            &[
                (AT + 500, 2000, Stop::Error),
                (AT + 12_500, 3000, Stop::Done),
            ],
        )];
        let p = &painful(&[&turns], AT - 1, AT + 1)[0];
        assert_eq!(
            (p.lost_seconds, p.slowed, p.part),
            (12.0, "an error", "provider")
        );
        let Signs::Pi(signs) = &p.signs else {
            panic!("a pi moment carries pi's signs");
        };
        assert_eq!(signs.errors, 1);
        assert_eq!(
            crate::pain::float(MOST_MS),
            crate::pain::GAVE_UP_MOST_MS,
            "pi and the launcher hold a go to the same two minutes"
        );
        let hung = [turn(AT, &[(AT + 500, 600_000, Stop::Error)])];
        assert_eq!(
            painful(&[&hung], AT - 1, AT + 1)[0].lost_seconds,
            120.0,
            "past two minutes he was doing something else"
        );
    }

    #[test]
    fn a_stopped_turn_loses_what_it_ran_at_most_two_minutes() {
        let short = [turn(AT, &[(AT + 500, 30_000, Stop::Aborted)])];
        let p = &painful(&[&short], AT - 1, AT + 1)[0];
        assert_eq!(
            (p.lost_seconds, p.slowed, p.part),
            (30.5, "stopped it", "stopped")
        );
        let long = [turn(AT, &[(AT + 500, 600_000, Stop::Aborted)])];
        assert_eq!(painful(&[&long], AT - 1, AT + 1)[0].lost_seconds, 120.0);
    }

    #[test]
    fn saying_it_again_loses_the_first_go() {
        let mut second = turn(AT + 90_000, &[(AT + 90_500, 5000, Stop::Done)]);
        second.again = true;
        let turns = [turn(AT, &[(AT + 6000, 5000, Stop::Done)]), second];
        let named = painful(&[&turns], AT - 1, AT + 1);
        assert_eq!(named.len(), 1, "the two are one go");
        let p = &named[0];
        assert_eq!(
            (p.lost_seconds, p.slowed, p.part, p.openings),
            (90.0, "tried again", "again", 2)
        );
        assert_eq!(p.seconds, 95.5);
    }

    #[test]
    fn corrections_and_status_questions_are_signs_not_lost_time() {
        let mut asked = turn(AT, &[(AT + 3000, 1000, Stop::Done)]);
        asked.status = true;
        asked.correction = true;
        let p = &painful(&[&[asked]], AT - 1, AT + 1)[0];
        assert_eq!(p.lost_seconds, 2.0, "only its wait is lost");
        let Signs::Pi(signs) = &p.signs else {
            panic!("a pi moment carries pi's signs");
        };
        assert_eq!((signs.corrections, signs.status), (1, 1));
    }

    #[test]
    fn only_goes_begun_in_the_day_count() {
        let turns = [turn(AT, &[(AT + 21_000, 4000, Stop::Done)])];
        assert!(
            painful(&[&turns], AT + 1, AT + 2).is_empty(),
            "before the day"
        );
        assert!(painful(&[&turns], AT - 2, AT).is_empty(), "after it");
    }
}
