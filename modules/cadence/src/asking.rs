//! Whom cadence asks about the targets `watch.rs` finds missed, and what
//! it remembers between ticks so each is asked once: the session keeping
//! the cycle fast first, and the orchestrator above it when that session
//! doesn't answer or isn't there.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::watch::{Snapshot, breaches};

/// The card is overdue when neither the watcher nor cadence sent one this
/// long, in seconds; cadence then sends its own, since asking the watcher
/// waits on that session's turn, and on 7 November three cards in a row
/// went late that way.
pub const CARD_EVERY: i64 = 65 * 60;
/// How long the watcher has to answer before the orchestrator is asked.
pub const ANSWER_WITHIN: i64 = 20 * 60;
/// How long the watcher may be gone before the orchestrator is asked.
pub const DOWN_FOR: i64 = 10 * 60;
/// How long a key is remembered, so a target missed all day is said once.
const REMEMBER: i64 = 2 * 86_400;

/// A question cadence asked, waiting on its answer.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Asked {
    pub id: String,
    pub at: i64,
    pub text: String,
}

/// What cadence remembers between ticks.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct State {
    #[serde(default)]
    pub asked: BTreeMap<String, Asked>,
    #[serde(default)]
    pub seen: BTreeMap<String, i64>,
    #[serde(default)]
    pub escalated: BTreeMap<String, i64>,
    #[serde(default)]
    pub down_since: Option<i64>,
    /// When cadence last sent the orchestrator the card itself.
    #[serde(default)]
    pub carded: Option<i64>,
}

/// Who a question goes to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum To {
    Watcher,
    Orchestrator,
}

impl To {
    /// How the mesh is to carry it. Only the watcher's answers are waited
    /// on, read from what it sent; the orchestrator is told as a finding, which
    /// wants no answer, since one sent back to cadence's address reaches no
    /// session, and on 9 November a question to the orchestrator sat unanswered
    /// on the mesh.
    #[must_use]
    pub fn intent(self) -> &'static str {
        match self {
            To::Watcher => "question",
            To::Orchestrator => "finding",
        }
    }
}

/// A question to send, and the key it was asked under, when it waits on an
/// answer.
#[derive(Debug, Clone, PartialEq)]
pub struct Question {
    pub to: To,
    pub key: Option<String>,
    pub text: String,
}

/// One tick: what to ask whom, given the snapshot and what was asked
/// before. Each question to the watcher is remembered by its key once it
/// has gone, with [`State::asked_as`], so its answer can be waited on.
pub fn tick(s: &Snapshot, state: &mut State) -> Vec<Question> {
    let mut out = Vec::new();
    let mut escalate = |state: &mut State, key: String, text: String| {
        if let std::collections::btree_map::Entry::Vacant(slot) = state.escalated.entry(key) {
            slot.insert(s.now);
            out.push(Question {
                to: To::Orchestrator,
                key: None,
                text,
            });
        }
    };

    if s.watcher_alive {
        state.down_since = None;
    } else {
        let down = *state.down_since.get_or_insert(s.now);
        if s.now - down >= DOWN_FOR {
            let restart = s
                .watcher_session
                .as_ref()
                .map_or_else(String::new, |session| {
                    format!(" Its session is {session}, which `pi --session` starts again.")
                });
            escalate(
                state,
                format!("down:{down}"),
                format!(
                    "The session keeping the delivery cycle fast has been gone {} min, so nobody \
                     answers the cycle's alarms.{restart} The latest card is in cadence's state folder.",
                    (s.now - down) / 60
                ),
            );
        }
    }

    state.asked.retain(|_, q| !s.answered.contains(&q.id));
    let silent: Vec<(String, Asked)> = state
        .asked
        .iter()
        .filter(|(_, q)| s.now - q.at > ANSWER_WITHIN)
        .map(|(k, q)| (k.clone(), q.clone()))
        .collect();
    if s.watcher_alive {
        for (key, q) in silent {
            escalate(
                state,
                format!("silent:{key}"),
                format!(
                    "The session keeping the delivery cycle fast hasn't answered cadence for {} min: \
                     \"{}\" Pivot or restart it.",
                    (s.now - q.at) / 60,
                    q.text
                ),
            );
        }
    }

    let mut ask = |state: &mut State, key: String, text: String| {
        if !state.seen.contains_key(&key) {
            state.seen.insert(key.clone(), s.now);
            out.push(Question {
                to: To::Watcher,
                key: Some(key),
                text,
            });
        }
    };
    for breach in breaches(s) {
        ask(
            state,
            format!("alarm:{}", breach.key),
            format!("Cadence alarm: {}.", breach.text),
        );
    }

    let keep = |at: &i64| s.now - *at < REMEMBER;
    state.seen.retain(|_, at| keep(at));
    state.escalated.retain(|_, at| keep(at));
    state.asked.retain(|_, q| keep(&q.at));
    out
}

/// Whether the card is overdue, so cadence sends the orchestrator its own: no
/// card from the watcher or from cadence within [`CARD_EVERY`].
#[must_use]
pub fn card_due(s: &Snapshot, state: &State) -> bool {
    s.last_card
        .max(state.carded)
        .is_none_or(|at| s.now - at > CARD_EVERY)
}

impl State {
    /// Remembers that the question asked under `key` went as `id`, so its
    /// answer is known.
    pub fn asked_as(&mut self, key: &str, id: &str, at: i64, text: &str) {
        self.asked.insert(
            key.to_owned(),
            Asked {
                id: id.to_owned(),
                at,
                text: text.to_owned(),
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::watch::tests::calm;

    #[test]
    fn an_overdue_card_is_sent_by_cadence_once_an_hour_rather_than_asked_for() {
        let mut s = calm(100_000);
        s.last_card = Some(100_000 - CARD_EVERY - 1);
        let mut state = State::default();
        assert!(tick(&s, &mut state).is_empty(), "the watcher isn't asked");
        assert!(card_due(&s, &state), "the watcher's card is late");
        state.carded = Some(s.now);
        s.now += 5 * 60;
        assert!(!card_due(&s, &state), "cadence sent one this hour");
        s.now += CARD_EVERY;
        assert!(card_due(&s, &state), "and none since");
        s.last_card = Some(s.now - 60);
        assert!(!card_due(&s, &state), "the watcher's own card counts");
    }

    #[test]
    fn only_the_watcher_is_asked_and_the_orchestrator_is_told() {
        assert_eq!(To::Watcher.intent(), "question");
        assert_eq!(To::Orchestrator.intent(), "finding");
    }

    #[test]
    fn a_question_unanswered_past_twenty_minutes_goes_to_the_orchestrator_once() {
        let mut state = State::default();
        state.asked_as(
            "alarm:cpu:1",
            "1-CADENCE-1-aa",
            100_000 - ANSWER_WITHIN - 1,
            "Cadence alarm: hot.",
        );
        let s = calm(100_000);
        let asked = tick(&s, &mut state);
        assert_eq!(asked.len(), 1);
        assert_eq!(asked[0].to, To::Orchestrator);
        assert!(
            asked[0].text.contains("Cadence alarm: hot."),
            "the orchestrator wasn't told what went unanswered: {}",
            asked[0].text
        );
        assert_eq!(tick(&s, &mut state), [] as [Question; 0]);
    }

    #[test]
    fn an_answered_question_is_let_go() {
        let mut state = State::default();
        state.asked_as(
            "alarm:cpu:1",
            "1-CADENCE-1-aa",
            100_000 - ANSWER_WITHIN - 1,
            "hot",
        );
        let mut s = calm(100_000);
        s.answered = vec!["1-CADENCE-1-aa".into()];
        assert_eq!(tick(&s, &mut state), [] as [Question; 0]);
        assert!(
            state.asked.is_empty(),
            "an answered question was kept: {:?}",
            state.asked
        );
    }

    #[test]
    fn a_watcher_gone_ten_minutes_is_told_to_the_orchestrator_with_its_session() {
        let mut state = State::default();
        let mut s = calm(100_000);
        s.watcher_alive = false;
        s.watcher_session = Some("01a1ae37".into());
        assert_eq!(tick(&s, &mut state), [] as [Question; 0]);
        s.now += DOWN_FOR;
        s.last_card = Some(s.now);
        let asked = tick(&s, &mut state);
        assert_eq!(asked.len(), 1);
        assert_eq!(asked[0].to, To::Orchestrator);
        assert!(
            asked[0].text.contains("01a1ae37"),
            "the gone watcher's session wasn't named: {}",
            asked[0].text
        );
    }
}
