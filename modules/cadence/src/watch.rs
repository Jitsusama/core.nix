//! What the cycle's records say now, which targets that misses, and whom
//! to ask about it: the session keeping the cycle fast first, and the
//! steward above it when that session doesn't answer or isn't there.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::records::{Follower, Look, Merge, NetHold, Run};
use crate::stats::spread;

/// A merge not on the machine this long after it merged, in minutes.
pub const NOT_ON_MIN: i64 = 20;
/// A VM run's median wait for a builder over the last hour, past which it
/// is said, in minutes; the target is 2, and a few slow runs aren't a trend.
pub const VM_WAIT_MEDIAN_MIN: f64 = 5.0;
/// CPU pressure over five minutes in what Joel uses, his apps' or his
/// session's slice, past which it is said. The machine-wide figure stays on
/// the card alone: on 10 November it was asked about at 53% and 41%, both
/// times carried by the agents' jobs, which rank behind him by design,
/// while his own slices waited under 1%, so no answer changed anything.
pub const JOEL_CPU_AVG300: f64 = 10.0;
/// The net's round this many hours behind main's head.
pub const NET_BEHIND_H: f64 = 3.0;
/// The net holding main with no revert under way this long, in minutes:
/// every merge waits off the machine until the merge that broke a test is
/// fixed forward or reverted by hand, and on 6 November a revert that
/// conflicted left it so with only a log line said.
pub const HELD_UNREVERTED_MIN: i64 = 10;
/// The card is overdue when neither the watcher nor cadence sent one this
/// long, in seconds; cadence then sends its own, since asking the watcher
/// waits on that session's turn, and on 7 November three cards in a row
/// went late that way.
pub const CARD_EVERY: i64 = 65 * 60;
/// How long the watcher has to answer before the steward is asked.
pub const ANSWER_WITHIN: i64 = 20 * 60;
/// How long the watcher may be gone before the steward is asked.
pub const DOWN_FOR: i64 = 10 * 60;
/// How long a key is remembered, so a target missed all day is said once.
const REMEMBER: i64 = 2 * 86_400;

/// A merge, and the look that put it on the machine, when one has.
#[derive(Debug, Clone, PartialEq)]
pub struct Landing {
    pub merge: Merge,
    pub on: Option<Look>,
}

/// The net's round: the head it tests, when that head merged, and how many
/// merges main has past it.
#[derive(Debug, Clone, PartialEq)]
pub struct Round {
    pub head: String,
    pub merged: i64,
    pub behind: usize,
    /// Whether a round tests `head` now, or the net, between rounds, last
    /// saw it: on 10 November the net tested nothing past 19:56's head for
    /// hours while a search ran, and the card said it was caught up.
    pub testing: bool,
}

/// Everything the watch and the card read, gathered at one moment.
#[derive(Debug, Clone, Default)]
pub struct Snapshot {
    pub now: i64,
    pub landings: Vec<Landing>,
    pub follower: Follower,
    pub runs: Vec<Run>,
    pub cpu_avg300: Option<f64>,
    /// The higher of Joel's apps' and session's CPU pressure over five
    /// minutes.
    pub joel_cpu_avg300: Option<f64>,
    pub available_gib: f64,
    pub swap_gib: f64,
    /// When the newest machine-wide out-of-memory kill in the last week was.
    pub last_oom_kill: Option<i64>,
    pub round: Option<Round>,
    /// The net's hold on switching, while a test it broke is unfixed.
    pub net_hold: Option<NetHold>,
    /// Whether a watcher was named, so its being there is read at all.
    pub watched: bool,
    pub watcher_alive: bool,
    /// The watcher's session, for the steward to restart it by.
    pub watcher_session: Option<String>,
    /// When the watcher last sent the steward a card.
    pub last_card: Option<i64>,
    /// The ids the watcher's messages answer.
    pub answered: Vec<String>,
}

/// A target missed: a key it is said under once, and what to say.
#[derive(Debug, Clone, PartialEq)]
pub struct Breach {
    pub key: String,
    pub text: String,
}

/// The targets the snapshot misses now.
#[must_use]
pub fn breaches(s: &Snapshot) -> Vec<Breach> {
    let hour = s.now / 3600;
    let mut out = Vec::new();
    if let Some(breach) = not_on(s) {
        out.push(breach);
    }
    if let Some(breach) = switch_failing(s) {
        out.push(breach);
    }
    // Said at once, unlike a merge not on yet: after this refusal the
    // follower asks no more, so nothing moves until someone asks with a
    // hold sent, and sessions-kitty joined the held daemons on 9 November.
    if let Some((at, why)) = &s.follower.held {
        let why: String = why.chars().take(240).collect();
        out.push(Breach {
            key: format!("heldfor:{at}"),
            text: format!(
                "an update was refused {} min ago, and the follower asks no more until someone \
                 asks with a hold sent: {why}",
                (s.now - at) / 60
            ),
        });
    }
    let waits: Vec<f64> = s
        .runs
        .iter()
        .filter(|r| r.vm() && r.at >= s.now - 3600)
        .filter_map(|r| r.waited.map(|w| w / 60.0))
        .collect();
    if let Some(w) = spread(&waits).filter(|w| w.n >= 4 && w.median > VM_WAIT_MEDIAN_MIN) {
        out.push(Breach {
            key: format!("vmwait:{hour}"),
            text: format!(
                "VM runs waited a median {:.0} min for a builder over the last hour (n {}, target 2)",
                w.median, w.n
            ),
        });
    }
    if let Some(cpu) = s.joel_cpu_avg300.filter(|&c| c > JOEL_CPU_AVG300) {
        out.push(Breach {
            key: format!("cpu:{hour}"),
            text: format!(
                "what Joel uses waits on the CPU {cpu:.0}% of five minutes (his apps' or \
                 session's slice, said past {JOEL_CPU_AVG300:.0})"
            ),
        });
    }
    if let Some(round) = &s.round {
        #[expect(clippy::cast_precision_loss, reason = "seconds within a lifetime")]
        let hours = (s.now - round.merged) as f64 / 3600.0;
        if hours > NET_BEHIND_H {
            // Said at three hours and again each time its age doubles: a long
            // search is known once it's said.
            let mut tier = NET_BEHIND_H;
            while tier * 2.0 <= hours {
                tier *= 2.0;
            }
            let short: String = round.head.chars().take(9).collect();
            let what = if round.testing {
                format!("the net's round at {short}")
            } else {
                format!("the net, between rounds, last saw {short}, which")
            };
            out.push(Breach {
                key: format!("net:{short}:{tier}"),
                text: format!(
                    "{what} is {hours:.1} h old, {} merges behind main",
                    round.behind
                ),
            });
        }
    }
    if let Some(hold) = s
        .net_hold
        .as_ref()
        .filter(|h| !h.reverting && s.now - h.at >= HELD_UNREVERTED_MIN * 60)
    {
        let culprit = hold.bad_pr.map_or_else(
            || "it couldn't tell which merge broke it".to_owned(),
            |pr| format!("it blames #{pr} and couldn't revert it"),
        );
        out.push(Breach {
            key: format!("hold:{}", hold.at),
            text: format!(
                "the net has held main {} min on {}: {culprit}, so every merge waits off the \
                 machine until it's fixed forward or reverted by hand",
                (s.now - hold.at) / 60,
                hold.test
            ),
        });
    }
    if let Some(at) = s.last_oom_kill.filter(|&at| s.now - at < 3600) {
        out.push(Breach {
            key: format!("oom:{at}"),
            text: format!(
                "a machine-wide out-of-memory kill {} min ago; the week without one starts again",
                (s.now - at) / 60
            ),
        });
    }
    out
}

/// How long a switch under way keeps a waiting merge quiet. A switch past
/// it is stalled or slow enough to say: on 8 November one waited 20
/// minutes on a builder that didn't answer, and nothing said so.
pub const GOING_QUIET_MIN: i64 = 15;

/// The least time between two switches, in minutes, as update-to-main's
/// follower keeps it (its `SWITCH_EVERY`): a merge that lands soon after a
/// switch waits for the next by design. On 10 November a merge 22 minutes
/// after a switch was said not on, eight minutes before the follower
/// could ask for it, and the follower was restarted for nothing.
pub const SWITCH_EVERY_MIN: i64 = 30;

/// The oldest merge that should be on the machine by now and isn't, unless
/// a switch that began after it is under way, for less than
/// `GOING_QUIET_MIN`, or one waits out a call. A merge is counted late
/// from when it could first go on: when it merged, or once the switches'
/// spacing since the last one has passed, whichever is later.
fn not_on(s: &Snapshot) -> Option<Breach> {
    if s.follower.meeting {
        return None;
    }
    let spaced = s
        .follower
        .looks
        .iter()
        .filter(|look| look.switched)
        .map(|look| look.at + SWITCH_EVERY_MIN * 60)
        .max();
    let due = |l: &Landing| spaced.map_or(l.merge.at, |spaced| spaced.max(l.merge.at));
    let mut waiting: Vec<&Landing> = s
        .landings
        .iter()
        .filter(|l| l.on.is_none() && !l.merge.tests_only())
        .filter(|l| s.now - due(l) >= NOT_ON_MIN * 60 && s.now - l.merge.at < 6 * 3600)
        .collect();
    waiting.sort_by_key(|l| l.merge.at);
    let oldest = waiting.first()?;
    let going = s.follower.going.filter(|began| *began >= oldest.merge.at);
    if going.is_some_and(|began| s.now - began < GOING_QUIET_MIN * 60) {
        return None;
    }
    let held = if s.net_hold.is_some() {
        " (the net holds switches)".to_owned()
    } else if let Some(began) = going {
        format!(
            ", though a switch has been under way {} min",
            (s.now - began) / 60
        )
    } else {
        String::new()
    };
    Some(Breach {
        key: format!(
            "noton:{}",
            oldest.merge.sha.chars().take(12).collect::<String>()
        ),
        text: format!(
            "{} merged {} min ago and isn't on the machine yet{held}",
            oldest.merge.name(),
            (s.now - oldest.merge.at) / 60
        ),
    })
}

/// How long updates may keep failing with a merge waiting before it's
/// said, in minutes: past the follower's quick tries after a builder goes
/// away, which mostly get a switch on within that. On 8 November a switch
/// Joel waited on failed at 12:57 and 13:12 and nothing said so, since
/// the merge it carried had already been said not on during a hold.
pub const FAILING_MIN: i64 = 5;

/// Updates failing since one first failed `FAILING_MIN` ago, with a merge
/// that should go on waiting, said once for each run of failures.
fn switch_failing(s: &Snapshot) -> Option<Breach> {
    let (first, why) = s.follower.failed.as_ref()?;
    if s.now - first < FAILING_MIN * 60 {
        return None;
    }
    let waiting = s
        .landings
        .iter()
        .filter(|l| l.on.is_none() && !l.merge.tests_only())
        .count();
    if waiting == 0 {
        return None;
    }
    let why: String = why.chars().take(240).collect();
    let merges = if waiting == 1 { "merge" } else { "merges" };
    Some(Breach {
        key: format!("failing:{first}"),
        text: format!(
            "updating the machine has failed since {} min ago, with {waiting} {merges} waiting \
             to go on; the newest failure: {why}",
            (s.now - first) / 60
        ),
    })
}

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
    /// When cadence last sent the steward the card itself.
    #[serde(default)]
    pub carded: Option<i64>,
}

/// Who a question goes to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum To {
    Watcher,
    Steward,
}

impl To {
    /// How the mesh is to carry it. Only the watcher's answers are waited
    /// on, read from what it sent; the steward is told as a finding, which
    /// wants no answer, since one sent back to cadence's address reaches no
    /// session, and on 9 November a question to the steward sat unanswered
    /// on the mesh.
    #[must_use]
    pub fn intent(self) -> &'static str {
        match self {
            To::Watcher => "question",
            To::Steward => "finding",
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
                to: To::Steward,
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

/// Whether the card is overdue, so cadence sends the steward its own: no
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

    fn merge(sha: &str, at: i64) -> Merge {
        Merge {
            sha: sha.into(),
            at,
            pr: Some(1),
            last_commit: None,
            files: vec!["modules/a/home.nix".into()],
        }
    }

    fn calm(now: i64) -> Snapshot {
        Snapshot {
            now,
            watcher_alive: true,
            last_card: Some(now - 60),
            ..Snapshot::default()
        }
    }

    #[test]
    fn a_calm_cycle_asks_nothing() {
        let mut state = State::default();
        assert_eq!(
            tick(&calm(100_000), &mut state),
            [] as [crate::watch::Question; 0]
        );
    }

    #[test]
    fn a_merge_off_the_machine_past_twenty_minutes_is_said_once() {
        let mut s = calm(100_000);
        s.landings = vec![Landing {
            merge: merge("abc", 100_000 - 25 * 60),
            on: None,
        }];
        let mut state = State::default();
        let first = tick(&s, &mut state);
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].to, To::Watcher);
        assert!(
            first[0].text.contains("#1 merged 25 min ago"),
            "{}",
            first[0].text
        );
        assert_eq!(tick(&s, &mut state), [] as [crate::watch::Question; 0]);
    }

    #[test]
    fn a_merge_waits_quietly_behind_a_switch_under_way_or_a_call() {
        let mut s = calm(100_000);
        s.landings = vec![Landing {
            merge: merge("abc", 100_000 - 25 * 60),
            on: None,
        }];
        s.follower.going = Some(100_000 - 60);
        assert_eq!(breaches(&s), [] as [crate::watch::Breach; 0]);
        s.follower.going = None;
        s.follower.meeting = true;
        assert_eq!(breaches(&s), [] as [crate::watch::Breach; 0]);
    }

    #[test]
    fn updates_failing_with_a_merge_waiting_are_said_once_after_five_minutes() {
        let mut s = calm(100_000);
        s.landings = vec![Landing {
            merge: merge("abc", 100_000 - 8 * 60),
            on: None,
        }];
        s.follower.failed = Some((100_000 - 4 * 60, "the builders went away".into()));
        assert_eq!(breaches(&s), Vec::<Breach>::new());
        s.follower.failed = Some((100_000 - FAILING_MIN * 60, "the builders went away".into()));
        let mut state = State::default();
        let said = tick(&s, &mut state);
        assert_eq!(said.len(), 1, "{said:?}");
        assert!(
            said[0]
                .text
                .contains("failed since 5 min ago, with 1 merge waiting"),
            "{}",
            said[0].text
        );
        assert!(
            said[0]
                .text
                .ends_with("the newest failure: the builders went away."),
            "{}",
            said[0].text
        );
        assert_eq!(tick(&s, &mut state), Vec::<Question>::new());
        // With nothing waiting, a failure isn't said.
        s.landings.clear();
        assert_eq!(breaches(&s), Vec::<Breach>::new());
    }

    #[test]
    fn an_update_refused_for_a_held_daemon_is_said_at_once_and_once() {
        let mut s = calm(100_000);
        s.follower.held = Some((100_000 - 60, "switching would change sessions-kitty".into()));
        let mut state = State::default();
        let said = tick(&s, &mut state);
        assert_eq!(said.len(), 1, "{said:?}");
        assert!(
            said[0]
                .text
                .contains("refused 1 min ago, and the follower asks no more"),
            "{}",
            said[0].text
        );
        assert!(
            said[0]
                .text
                .ends_with("switching would change sessions-kitty."),
            "{}",
            said[0].text
        );
        assert_eq!(tick(&s, &mut state), Vec::<Question>::new());
    }

    #[test]
    fn a_switch_under_way_too_long_is_said() {
        let mut s = calm(100_000);
        s.landings = vec![Landing {
            merge: merge("abc", 100_000 - 40 * 60),
            on: None,
        }];
        s.follower.going = Some(100_000 - GOING_QUIET_MIN * 60);
        let said = breaches(&s);
        assert_eq!(said.len(), 1, "{said:?}");
        assert!(
            said[0]
                .text
                .ends_with("though a switch has been under way 15 min"),
            "{}",
            said[0].text
        );
    }

    #[test]
    fn a_merge_waiting_out_the_switches_spacing_is_late_only_from_its_end() {
        let now = 100_000;
        let mut s = calm(now);
        // The 10 November case: a switch at 06:32, a merge at 06:34, said
        // not on at 06:55, though the follower could ask only from 07:02.
        s.follower.looks = vec![Look {
            at: now - 23 * 60,
            core: "old".into(),
            switched: true,
        }];
        s.landings = vec![Landing {
            merge: merge("abc", now - 21 * 60),
            on: None,
        }];
        assert_eq!(breaches(&s), [] as [crate::watch::Breach; 0]);
        // Twenty minutes past the spacing's end it's late after all.
        s.now = now - 23 * 60 + (SWITCH_EVERY_MIN + NOT_ON_MIN) * 60;
        let said = breaches(&s);
        assert_eq!(said.len(), 1, "{said:?}");
        assert!(
            said[0].text.contains("#1 merged 48 min ago"),
            "{}",
            said[0].text
        );
    }

    #[test]
    fn a_merge_on_the_machine_or_of_docs_alone_isnt_said() {
        let mut s = calm(100_000);
        let mut docs = merge("doc", 100_000 - 30 * 60);
        docs.files = vec!["docs/delivery.md".into()];
        let look = Look {
            at: 100_000 - 60,
            core: "abc".into(),
            switched: true,
        };
        s.landings = vec![
            Landing {
                merge: merge("abc", 100_000 - 30 * 60),
                on: Some(look),
            },
            Landing {
                merge: docs,
                on: None,
            },
        ];
        assert_eq!(breaches(&s), [] as [crate::watch::Breach; 0]);
    }

    #[test]
    fn a_hold_with_no_revert_is_said_once_after_ten_minutes() {
        let mut s = calm(100_000);
        let hold = NetHold {
            at: 100_000 - 5 * 60,
            test: "vm-a".into(),
            bad_pr: Some(2059),
            reverting: false,
        };
        s.net_hold = Some(hold.clone());
        assert_eq!(breaches(&s), [] as [crate::watch::Breach; 0]);
        s.net_hold = Some(NetHold {
            at: 100_000 - 12 * 60,
            ..hold.clone()
        });
        let said = breaches(&s);
        assert_eq!(said[0].key, format!("hold:{}", 100_000 - 12 * 60));
        assert!(
            said[0].text.contains("12 min on vm-a: it blames #2059"),
            "{}",
            said[0].text
        );
        s.net_hold = Some(NetHold {
            at: 100_000 - 12 * 60,
            reverting: true,
            ..hold
        });
        assert_eq!(breaches(&s), [] as [crate::watch::Breach; 0]);
    }

    #[test]
    fn the_nets_lag_is_said_at_three_hours_then_at_each_doubling() {
        let mut s = calm(100_000);
        s.round = Some(Round {
            head: "cb5b9b12dce0".into(),
            merged: 100_000 - 4 * 3600,
            behind: 5,
            testing: true,
        });
        assert_eq!(breaches(&s)[0].key, "net:cb5b9b12d:3");
        s.round = Some(Round {
            head: "cb5b9b12dce0".into(),
            merged: 100_000 - 7 * 3600,
            behind: 9,
            testing: false,
        });
        let said = &breaches(&s)[0];
        assert_eq!(said.key, "net:cb5b9b12d:6");
        assert!(
            said.text
                .starts_with("the net, between rounds, last saw cb5b9b12d"),
            "a net testing nothing was said to be in a round: {}",
            said.text
        );
    }

    #[test]
    fn slow_waits_and_high_pressure_and_a_kill_are_each_said() {
        let mut s = calm(100_000);
        let run = |w| Run {
            at: 99_900,
            on: "nix-offload-vm".into(),
            seconds: 600.0,
            waited: Some(w),
            status: 0,
        };
        s.runs = vec![run(400.0), run(400.0), run(400.0), run(400.0)];
        s.cpu_avg300 = Some(45.0);
        s.last_oom_kill = Some(99_000);
        let keys: Vec<String> = breaches(&s).into_iter().map(|b| b.key).collect();
        assert_eq!(
            keys,
            ["vmwait:27", "oom:99000"],
            "the agents' own pressure is the card's alone"
        );
        s.joel_cpu_avg300 = Some(12.0);
        let keys: Vec<String> = breaches(&s).into_iter().map(|b| b.key).collect();
        assert_eq!(keys, ["vmwait:27", "cpu:27", "oom:99000"]);
        s.last_oom_kill = Some(100_000 - 3600);
        assert_eq!(breaches(&s).len(), 2);
    }

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
    fn only_the_watcher_is_asked_and_the_steward_is_told() {
        assert_eq!(To::Watcher.intent(), "question");
        assert_eq!(To::Steward.intent(), "finding");
    }

    #[test]
    fn a_question_unanswered_past_twenty_minutes_goes_to_the_steward_once() {
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
        assert_eq!(asked[0].to, To::Steward);
        assert!(asked[0].text.contains("Cadence alarm: hot."));
        assert_eq!(tick(&s, &mut state), [] as [crate::watch::Question; 0]);
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
        assert_eq!(tick(&s, &mut state), [] as [crate::watch::Question; 0]);
        assert!(state.asked.is_empty());
    }

    #[test]
    fn a_watcher_gone_ten_minutes_is_told_to_the_steward_with_its_session() {
        let mut state = State::default();
        let mut s = calm(100_000);
        s.watcher_alive = false;
        s.watcher_session = Some("01a1ae37".into());
        assert_eq!(tick(&s, &mut state), [] as [crate::watch::Question; 0]);
        s.now += DOWN_FOR;
        s.last_card = Some(s.now);
        let asked = tick(&s, &mut state);
        assert_eq!(asked.len(), 1);
        assert_eq!(asked[0].to, To::Steward);
        assert!(asked[0].text.contains("01a1ae37"));
    }
}
