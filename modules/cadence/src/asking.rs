//! Whom cadence asks about the targets `watch.rs` finds missed, and what
//! it remembers between ticks so each is asked once: the session keeping
//! the cycle fast, or the one switching the machine for an alarm only a
//! switch answers, and the orchestrator above them when the one asked
//! doesn't answer or isn't there. A budget nearing its allowance is told
//! to the orchestrator alone, as a finding nothing waits on.

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
/// How long a budget's alarm is remembered once it stops standing: one is
/// told again only once it has stood clear this long, so a share hovering
/// at a level isn't told at each crossing, while a rolling window that
/// eased overnight is told when it climbs back, and a spend-watch stopped
/// again the next day is told again.
const BUDGET_REST: i64 = 6 * 3600;
/// How long no budget may go watched before the orchestrator is told:
/// spend-watch times its passes by a clock that stops while the machine
/// sleeps, so after a wake it may take five minutes to write again.
const UNWATCHED_FOR: i64 = 10 * 60;

/// A question cadence asked, waiting on its answer.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Asked {
    pub id: String,
    pub at: i64,
    pub text: String,
    /// The switcher it went to, or none for the watcher: kept, since who
    /// owns switching can change while it waits, and only the one asked
    /// answers it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub by: Option<String>,
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
    /// Since when no budget has been watched, as spend-watch's standing
    /// couldn't be read.
    #[serde(default)]
    pub unwatched_since: Option<i64>,
}

/// Who a question goes to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum To {
    Watcher,
    /// Whoever switches the machine, for an alarm only a switch answers:
    /// on 18 November one went to the watcher, which could only pass it on.
    Switcher,
    /// Whoever sent the hold the net is held by, for an alarm only ending
    /// that hold answers.
    Holder,
    Orchestrator,
}

impl To {
    /// How the mesh is to carry it. The watcher's, the switcher's and the
    /// holder's answers are waited on, read from what they sent; the
    /// orchestrator is told as a finding, which wants no answer, since one
    /// sent back to cadence's address reaches no session, and on 9 November
    /// a question to the orchestrator sat unanswered on the mesh.
    #[must_use]
    pub fn intent(self) -> &'static str {
        match self {
            To::Watcher | To::Switcher | To::Holder => "question",
            To::Orchestrator => "finding",
        }
    }

    /// The address it goes to, and the session it was asked of by name, whose
    /// silence is then said as that session's. The switcher or the holder
    /// the snapshot doesn't name is asked as the watcher, so the watcher's
    /// silence isn't said as theirs.
    #[must_use]
    pub fn addressed<'a>(
        self,
        s: &'a Snapshot,
        watcher: &'a str,
        orchestrator: &'a str,
    ) -> (&'a str, Option<&'a str>) {
        let named = match self {
            To::Watcher => None,
            To::Switcher => s.switcher.as_deref(),
            To::Holder => s
                .net_held_since
                .as_ref()
                .and_then(|held| held.from.as_deref()),
            To::Orchestrator => return (orchestrator, None),
        };
        (named.unwrap_or(watcher), named)
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
/// before. Each question to the watcher or the switcher is remembered by
/// its key once it has gone, with [`State::asked_as`], so its answer can
/// be waited on. An answer from the one asked settles it, and a held
/// net's question is settled too once its hold is seen to end or pass to
/// another.
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

    forget_ended_holds(s, state);
    state.asked.retain(|_, q| {
        let answers = match &q.by {
            Some(by) => s.answered_by.get(by).map_or(&[][..], Vec::as_slice),
            None => &s.answered,
        };
        !answers.contains(&q.id)
    });
    let silent: Vec<(String, Asked)> = state
        .asked
        .iter()
        .filter(|(_, q)| s.now - q.at > ANSWER_WITHIN)
        .map(|(k, q)| (k.clone(), q.clone()))
        .collect();
    for (key, q) in silent {
        // The switcher's silence is said whether the watcher is there or
        // not; the watcher's only while it is, as its being gone is said
        // on its own.
        let who = match &q.by {
            Some(by) if holding(&key) => {
                format!("{by}, which sent the hold the net is held by,")
            }
            Some(by) => format!("{by}, the session that switches the machine,"),
            None if s.watcher_alive => "The session keeping the delivery cycle fast".to_owned(),
            _ => continue,
        };
        escalate(
            state,
            format!("silent:{key}"),
            format!(
                "{who} hasn't answered cadence for {} min: \"{}\" Pivot or restart it.",
                (s.now - q.at) / 60,
                q.text
            ),
        );
    }

    let mut ask = |state: &mut State, key: String, text: String| {
        if !state.seen.contains_key(&key) {
            state.seen.insert(key.clone(), s.now);
            let to = if holding(&key) {
                To::Holder
            } else if s.switcher.is_some() && switching(&key) {
                To::Switcher
            } else if budgeting(&key) {
                To::Orchestrator
            } else {
                To::Watcher
            };
            out.push(Question {
                to,
                // The orchestrator is told, and nothing waits on its answer.
                key: (to != To::Orchestrator).then_some(key),
                text,
            });
        }
    };
    for breach in standing(s, state) {
        ask(
            state,
            format!("alarm:{}", breach.key),
            format!("Cadence alarm: {}.", breach.text),
        );
        remember_budget(state, &breach.key, s.now);
    }

    let keep = |at: &i64| s.now - *at < REMEMBER;
    state
        .seen
        .retain(|key, at| keep(at) && (!budgeting(key) || s.now - *at < BUDGET_REST));
    state.escalated.retain(|_, at| keep(at));
    state.asked.retain(|_, q| keep(&q.at));
    out
}

/// Remembers a budget's alarm while it stands, a level with those beneath
/// it, so a week past 90% is told once however long it stays, and isn't
/// told as new past 80% as it eases.
fn remember_budget(state: &mut State, key: &str, now: i64) {
    for key in crate::watch::budgets::stands_for(key) {
        state.seen.insert(format!("alarm:{key}"), now);
    }
}

/// The breaches standing now, but for no budget watched until it has stood
/// for `UNWATCHED_FOR`, so a machine just woken isn't told of it.
fn standing(s: &Snapshot, state: &mut State) -> Vec<crate::watch::Breach> {
    let unwatched_for = if s.budgets_unwatched.is_some() {
        s.now - *state.unwatched_since.get_or_insert(s.now)
    } else {
        state.unwatched_since = None;
        0
    };
    breaches(s)
        .into_iter()
        .filter(|b| b.key != crate::watch::budgets::UNWATCHED || unwatched_for >= UNWATCHED_FOR)
        .collect()
}

/// Ends each held net's question whose hold has ended, as ending it is the
/// answer in deed, so a holder isn't named silent about a hold it has
/// ended. Its key is forgotten with it, so a hold its sender sends again in
/// the same stop is asked again. An alarm that only stopped standing, as
/// its merges left the day, keeps its question while the hold does.
fn forget_ended_holds(s: &Snapshot, state: &mut State) {
    let ended: Vec<String> = state
        .seen
        .keys()
        .chain(state.asked.keys())
        .filter(|key| {
            holding(key)
                && key
                    .strip_prefix("alarm:")
                    .is_some_and(|alarm| crate::watch::held::ended(s, alarm))
        })
        .cloned()
        .collect();
    for key in &ended {
        state.seen.remove(key);
        state.asked.remove(key);
    }
}

/// Whether an alarm's key says the machine isn't running what merged: a
/// merge not on, updates failing, or the follower held. Only a switch
/// answers those.
fn switching(key: &str) -> bool {
    ["alarm:noton:", "alarm:failing:", "alarm:heldfor:"]
        .iter()
        .any(|alarm| key.starts_with(alarm))
}

/// Whether an alarm's key says a budget passed a share of an allowance,
/// which only the orchestrator can act on, by moving work to another
/// budget or asking Joel, as it asked to be told on 20 November.
fn budgeting(key: &str) -> bool {
    key.starts_with("alarm:budget:")
}

/// Whether an alarm's key says the net is held by a hold on the machine,
/// which only its sender ends.
fn holding(key: &str) -> bool {
    key.starts_with("alarm:netheld:")
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
    /// Remembers that the question asked under `key` went as `id`, to the
    /// switcher `by` or else the watcher, so its answer is known.
    pub fn asked_as(&mut self, key: &str, id: &str, at: i64, text: &str, by: Option<&str>) {
        self.asked.insert(
            key.to_owned(),
            Asked {
                id: id.to_owned(),
                at,
                text: text.to_owned(),
                by: by.map(str::to_owned),
            },
        );
    }

    /// The sessions other than the watcher that a question waits on, whose
    /// answers are to be read.
    pub fn askers(&self) -> impl Iterator<Item = &str> {
        self.asked.values().filter_map(|q| q.by.as_deref())
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
    fn an_alarm_only_a_switch_answers_is_one_of_three() {
        assert!(switching("alarm:noton:6a811d78d19a"), "a merge not on");
        assert!(
            switching("alarm:noton:after:1795000000"),
            "merges not on after one"
        );
        assert!(switching("alarm:failing:1795000000"), "updates failing");
        assert!(switching("alarm:heldfor:1795000000"), "the follower held");
        assert!(!switching("alarm:cpu:498611"), "his processor");
        assert!(!switching("noton:6a811d78d19a"), "only an alarm");
    }

    #[test]
    fn the_watcher_and_the_switcher_are_asked_and_the_orchestrator_is_told() {
        assert_eq!(To::Watcher.intent(), "question", "waited on");
        assert_eq!(To::Switcher.intent(), "question", "waited on");
        assert_eq!(To::Orchestrator.intent(), "finding", "wants nothing back");
    }

    /// A switch alarm asked 25 minutes ago of `switcher`, or of the watcher
    /// with none, and a snapshot of now with that switcher named.
    fn a_switch_alarm_asked(switcher: Option<&str>) -> (Snapshot, State) {
        let mut state = State::default();
        let at = 100_000 - ANSWER_WITHIN - 300;
        state.asked_as(
            "alarm:noton:abc",
            "1-CADENCE-1-aa",
            at,
            "#1 isn't on.",
            switcher,
        );
        let mut s = calm(100_000);
        s.switcher = switcher.map(Into::into);
        (s, state)
    }

    /// `s` with `by` having answered the question asked under `id`.
    fn answered_by(s: &mut Snapshot, by: &str, id: &str) {
        s.answered_by.insert(by.into(), vec![id.into()]);
    }

    #[test]
    fn a_switch_alarm_goes_to_the_switcher_or_with_none_named_to_the_watcher() {
        let mut s = calm(100_000);
        s.landings = vec![crate::watch::Landing {
            merge: crate::watch::tests::merge("abc", 100_000 - 25 * 60),
            on: None,
        }];
        let asked = tick(&s, &mut State::default());
        assert_eq!(asked.len(), 1, "one alarm: {asked:?}");
        assert_eq!(asked[0].to, To::Watcher, "no switcher named");
        s.switcher = Some("SWITCH".into());
        let asked = tick(&s, &mut State::default());
        assert_eq!(asked[0].to, To::Switcher, "the switcher named");
    }

    #[test]
    fn a_budget_past_a_level_is_told_to_the_orchestrator_once_and_waits_on_no_answer() {
        let mut s = calm(100_000);
        s.budgets = vec![crate::watch::budgets::Allowance {
            budget: "proxy".into(),
            label: "Your AI proxy token".into(),
            name: "weekly".into(),
            window: "Week".into(),
            share: 0.91,
            standing: "$36,261 of $40,000".into(),
            resets: None,
        }];
        let mut state = State::default();
        let told = tick(&s, &mut state);
        assert_eq!(
            told,
            [Question {
                to: To::Orchestrator,
                key: None,
                text: "Cadence alarm: Your AI proxy token's Week allowance stands at \
                       $36,261 of $40,000, 91%: past 90%, the last level it's told."
                    .into(),
            }],
            "a budget at 91% wasn't told to the orchestrator, or was waited on"
        );
        s.now += 5 * 60;
        assert!(
            tick(&s, &mut state).is_empty(),
            "one standing was told twice"
        );
        // It stands past 90% all week, then eases past 80%.
        for _ in 0..4 {
            s.now += REMEMBER;
            assert!(
                tick(&s, &mut state).is_empty(),
                "a week standing past 90% was told again as it stood"
            );
        }
        s.budgets[0].share = 0.85;
        assert!(
            tick(&s, &mut state).is_empty(),
            "a budget easing from past 90% to past 80% was told as nearing its allowance"
        );
        // Hovering at a level isn't told at each crossing, but a rolling
        // window that eased for longer than BUDGET_REST is told as it
        // climbs back.
        s.budgets[0].share = 0.92;
        s.now += 3600;
        assert!(
            tick(&s, &mut state).is_empty(),
            "a share back past 90% an hour after easing was told again"
        );
        s.budgets[0].share = 0.85;
        s.now += BUDGET_REST + 60;
        assert!(
            tick(&s, &mut state).is_empty(),
            "a share standing past 80% was told"
        );
        s.budgets[0].share = 0.92;
        assert_eq!(
            told_whom(&tick(&s, &mut state)),
            [(To::Orchestrator, false)],
            "a rolling window climbing back past 90% after a night's easing went untold"
        );

        s.budgets.clear();
        for stop in 1..=2 {
            s.budgets_unwatched = Some("is missing".into());
            // spend-watch may take five minutes after a wake to write.
            for _ in 0..2 {
                assert!(
                    tick(&s, &mut state).is_empty(),
                    "a spend-watch not yet written after a wake was told as stopped"
                );
                s.now += 5 * 60;
            }
            assert_eq!(
                told_whom(&tick(&s, &mut state)),
                [(To::Orchestrator, false)],
                "stop {stop}: no budget watched for ten minutes wasn't told to the \
                 orchestrator, or was waited on"
            );
            s.now += REMEMBER;
            assert!(
                tick(&s, &mut state).is_empty(),
                "stop {stop}: a standing stop was told again"
            );
            s.budgets_unwatched = None;
            s.now += BUDGET_REST + 60;
            tick(&s, &mut state);
        }
    }

    fn told_whom(questions: &[Question]) -> Vec<(To, bool)> {
        questions.iter().map(|q| (q.to, q.key.is_some())).collect()
    }

    #[test]
    fn a_held_net_is_asked_of_the_hold_and_its_silence_named_as_the_holds() {
        let since = 100_000 - 31 * 60;
        let key = format!("alarm:netheld:{since}:P9R1KI");
        assert!(holding(&key), "the net held");
        assert!(!holding("alarm:heldfor:1795000000"), "the follower held");
        assert_eq!(To::Holder.intent(), "question", "waited on");
        let held = crate::watch::held::tests::held_for(31, &[100_000 - 60]);
        let held_by = |s: &Snapshot| {
            let mut s = s.clone();
            s.net_held_since.as_mut().expect("a hold").from = Some("OSPHT4".into());
            s
        };
        // Each holder in one stop is asked once, of the hold.
        let mut state = State::default();
        let to_holder = |asked: &[Question]| -> Vec<String> {
            asked
                .iter()
                .filter(|q| q.to == To::Holder)
                .filter_map(|q| q.key.clone())
                .collect()
        };
        assert_eq!(
            to_holder(&tick(&held, &mut state)),
            std::slice::from_ref(&key),
            "the first holder"
        );
        assert_eq!(
            to_holder(&tick(&held_by(&held), &mut state)),
            [format!("alarm:netheld:{since}:OSPHT4")],
            "the second holder in the same stop wasn't asked"
        );
        let again = tick(&held_by(&held), &mut state);
        assert!(to_holder(&again).is_empty(), "asked again: {again:?}");
        assert_eq!(
            to_holder(&tick(&held, &mut state)),
            std::slice::from_ref(&key),
            "a holder back in the same stop wasn't asked of its new hold"
        );
        // A holder silent while its hold stands is named as its sender.
        let mut state = State::default();
        let at = 100_000 - ANSWER_WITHIN - 300;
        state.asked_as(&key, "1-CADENCE-1-aa", at, "held.", Some("P9R1KI"));
        let named = "P9R1KI, which sent the hold the net is held by,";
        let told = tick(&held, &mut state);
        assert!(
            told.iter().any(|q| q.text.starts_with(named)),
            "the hold's sender wasn't named as such: {told:?}"
        );
        // Once its hold has ended, or another holds the net, it isn't.
        for (now, case) in [(calm(100_000), "ended"), (held_by(&held), "taken over")] {
            let mut state = State::default();
            state.asked_as(&key, "1-CADENCE-1-aa", at, "held.", Some("P9R1KI"));
            let told = tick(&now, &mut state);
            assert!(
                !told.iter().any(|q| q.text.starts_with(named)),
                "a hold {case} still named its sender silent: {told:?}"
            );
        }
        // A held.json the net hasn't rewritten lately says nothing ended.
        let mut unread = held.clone();
        unread.net_held = Some(unread.now - crate::watch::HELD_FRESH_S - 60);
        let mut state = State::default();
        state.asked_as(&key, "1-CADENCE-1-aa", at, "held.", Some("P9R1KI"));
        let told = tick(&unread, &mut state);
        assert!(
            told.iter().any(|q| q.text.starts_with(named)),
            "one stale read ended a silent holder's question: {told:?}"
        );
    }

    #[test]
    fn each_is_asked_where_it_answers_and_only_one_named_is_asked_by_name() {
        fn at(to: To, s: &Snapshot) -> (&str, Option<&str>) {
            to.addressed(s, "WATCH", "@ORCH")
        }
        let mut s = crate::watch::held::tests::held_for(31, &[]);
        s.switcher = Some("SWITCH".into());
        assert_eq!(
            at(To::Holder, &s),
            ("P9R1KI", Some("P9R1KI")),
            "the hold's sender"
        );
        assert_eq!(
            at(To::Switcher, &s),
            ("SWITCH", Some("SWITCH")),
            "the switcher"
        );
        assert_eq!(at(To::Watcher, &s), ("WATCH", None), "the watcher");
        assert_eq!(
            at(To::Orchestrator, &s),
            ("@ORCH", None),
            "the orchestrator"
        );
        s.switcher = None;
        if let Some(held) = s.net_held_since.as_mut() {
            held.from = None;
        }
        assert_eq!(
            at(To::Holder, &s),
            ("WATCH", None),
            "a hold naming no sender was asked of the watcher as the hold's sender"
        );
        assert_eq!(
            at(To::Switcher, &s),
            ("WATCH", None),
            "no switcher known was asked of the watcher as the switcher"
        );
    }

    #[test]
    fn only_the_one_asked_settles_a_question() {
        let (mut s, mut state) = a_switch_alarm_asked(Some("SWITCH"));
        s.answered = vec!["1-CADENCE-1-aa".into()];
        tick(&s, &mut state);
        assert!(
            state.asked.contains_key("alarm:noton:abc"),
            "the watcher's reply settles no question the switcher was asked"
        );
        s.answered.clear();
        s.switcher = Some("LATER".into());
        answered_by(&mut s, "LATER", "1-CADENCE-1-aa");
        tick(&s, &mut state);
        assert!(
            state.asked.contains_key("alarm:noton:abc"),
            "nor does a later switcher's, which was never asked"
        );
        answered_by(&mut s, "SWITCH", "1-CADENCE-1-aa");
        tick(&s, &mut state);
        assert!(state.asked.is_empty(), "the switcher asked does");

        let (mut s, mut state) = a_switch_alarm_asked(None);
        s.switcher = Some("SWITCH".into());
        answered_by(&mut s, "SWITCH", "1-CADENCE-1-aa");
        tick(&s, &mut state);
        assert!(
            state.asked.contains_key("alarm:noton:abc"),
            "the switcher's reply settles no question the watcher was asked"
        );
        s.answered = vec!["1-CADENCE-1-aa".into()];
        tick(&s, &mut state);
        assert!(state.asked.is_empty(), "the watcher's own does");
    }

    #[test]
    fn only_those_with_a_question_waiting_are_read_for_answers() {
        let (_, mut state) = a_switch_alarm_asked(Some("SWITCH"));
        state.asked_as("alarm:cpu:1", "1-CADENCE-1-bb", 100_000, "hot", None);
        assert_eq!(
            state.askers().collect::<Vec<_>>(),
            ["SWITCH"],
            "the watcher aside"
        );
    }

    #[test]
    fn a_watcher_gone_is_said_as_gone_not_also_as_silent() {
        let (mut s, mut state) = a_switch_alarm_asked(None);
        s.watcher_alive = false;
        assert_eq!(
            tick(&s, &mut state),
            [] as [Question; 0],
            "not silent, and not yet gone"
        );
    }

    #[test]
    fn each_alarm_only_a_switch_answers_goes_to_the_switcher() {
        let mut s = calm(100_000);
        s.switcher = Some("SWITCH".into());
        s.landings = vec![crate::watch::Landing {
            merge: crate::watch::tests::merge("abc", 100_000 - 25 * 60),
            on: None,
        }];
        s.follower.failed = Some((100_000 - 3600, "no space left".into()));
        s.follower.held = Some((100_000 - 3600, "switching would change x".into()));
        let asked = tick(&s, &mut State::default());
        let keys: Vec<_> = asked.iter().filter_map(|q| q.key.as_deref()).collect();
        assert!(
            keys.len() >= 3,
            "a merge not on, failing and held: {keys:?}"
        );
        assert!(
            asked.iter().all(|q| q.to == To::Switcher),
            "every one to the switcher: {asked:?}"
        );
    }

    #[test]
    fn a_silent_switcher_is_named_to_the_orchestrator_though_the_watcher_is_gone() {
        let (mut s, mut state) = a_switch_alarm_asked(Some("SWITCH"));
        s.watcher_alive = false;
        let asked = tick(&s, &mut state);
        assert_eq!(
            asked.len(),
            1,
            "the silence, not yet the watcher's going: {asked:?}"
        );
        assert_eq!(asked[0].to, To::Orchestrator, "told");
        assert!(
            asked[0]
                .text
                .starts_with("SWITCH, the session that switches the machine,"),
            "named: {}",
            asked[0].text
        );
        let (s, mut state) = a_switch_alarm_asked(None);
        let asked = tick(&s, &mut state);
        assert!(
            asked[0]
                .text
                .starts_with("The session keeping the delivery cycle fast"),
            "with no switcher the watcher was asked, and is named: {}",
            asked[0].text
        );
    }

    #[test]
    fn a_question_unanswered_past_twenty_minutes_goes_to_the_orchestrator_once() {
        let mut state = State::default();
        state.asked_as(
            "alarm:cpu:1",
            "1-CADENCE-1-aa",
            100_000 - ANSWER_WITHIN - 1,
            "Cadence alarm: hot.",
            None,
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
            None,
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
