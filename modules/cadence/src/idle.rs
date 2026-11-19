//! Waking a session of an orchestrator's that has sat idle, and telling
//! the orchestrator when waking it didn't take. A session that finished
//! its piece and waits to be told its next one does nothing all night
//! unless someone notices. So each one listening, its pi running this
//! boot, that names a next step of its own, or names none on a quest, is
//! asked itself twenty minutes past its last turn, with a question, which
//! starts its turn, unless background work of its own has run for less
//! than an hour of it; and each one still idle two hours past it is named to
//! the orchestrator its presence names, once the question has had twenty
//! minutes to take. A session is asked once for each step and told of
//! once for each idle spell. On 18 November OX4WFV sat idle from 07:52 to
//! 10:11 past a 09:00 due: only its orchestrator was ever to hear, after
//! two hours, and each host restart began its spell again. That evening
//! SEYNT2 sat idle from 20:25 to 23:11 unwoken, naming no step and with
//! two alarm loops running for hours. Everything here is a decision over
//! what was read; tell.rs reads and sends.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::ANSWERS_GO;
use crate::mesh::Presence;

pub mod dated;
mod tell;

pub use tell::tell_idle;

/// Whether a quest's README says it routes: `hears: addressed` is the
/// harness's mark for a quest whose sessions hear in full only what is
/// put to them, as Joel's router does. Its cell is idle by design, waiting
/// on his lines, so it is never woken and no orchestrator hears of it: on
/// 19 November 8GYNJ6, the void, was woken at 03:10 and 05:10 to no end.
#[must_use]
pub fn routes(readme: &str) -> bool {
    let front = crate::axes::plans::front(readme);
    crate::axes::plans::field(front, "hears") == Some("addressed")
}

/// How long a session with a next step may sit idle before it is woken:
/// the hour's cells measure names one idle this long, by its
/// orchestrator's overnight rule of 16 November.
pub const WAKE_AFTER: i64 = 20 * 60;

/// How long a session may sit idle before its orchestrator hears of it.
pub const IDLE_FOR: i64 = 2 * 3600;

/// How many are woken in one tick. A host's restart brings dozens back at
/// once, and each woken starts a turn, so the longest idle go first and
/// the rest wait a tick: the background host ran out of memory at 03:23
/// on 18 November.
pub const WAKE_AT_MOST: usize = 4;

/// How long background work of its own holds back a session's wake past
/// its last turn. Its end starts a turn, but a loop sleeping until an
/// alarm hides an idle session for as long as it sleeps: on 18 November
/// SEYNT2 held one sleeping from 12:51 until 01:30, and sat idle from
/// 20:25 with nobody asking.
pub const WORKS_AT_MOST: i64 = 3600;

/// How much of a next step a wake quotes back to its session.
const QUOTED: usize = 160;

/// A session, as far as waking it or telling of its idleness needs.
#[derive(Debug, Clone, PartialEq, Eq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "each is a separate fact read of its session"
)]
pub struct Quiet {
    pub address: String,
    pub session: String,
    pub quest: Option<String>,
    pub orchestrated_by: Option<String>,
    /// Waiting for something to do: listening, not working and not parked.
    pub listening: bool,
    /// Whether its pi runs, and ran during this boot.
    pub running: bool,
    /// When its last turn ended, in seconds, as its presence says; none
    /// once the presence's day began again at midnight.
    pub turned: Option<i64>,
    /// When its idle spell began, as far as what was read says: its last
    /// turn's end, else its log's last write, else its pi's start. A
    /// restart begins no spell, since a session resumed idle is as idle as
    /// before.
    pub since: Option<i64>,
    /// When its pi started, in seconds.
    pub started: Option<i64>,
    /// Whether it has taken no turn since its pi started.
    pub unstarted: bool,
    /// The next step it names, the work it would wake to. None while it
    /// waits on an answer it asked for, or while its step names Joel,
    /// which only he can move, as the hour's cells measure has it.
    pub next: Option<String>,
    /// Whether it names no next step at all on a quest it holds, waiting
    /// on no answer: it is woken to the quest itself.
    pub stepless: bool,
    /// Whether background work of its own runs, whose end starts its turn
    /// without a word from anyone.
    pub working: bool,
    /// Whether its quest routes (see `routes`), so it is left alone.
    pub routes: bool,
    /// When its next step says it starts (see `dated`), in seconds: its
    /// spell begins no earlier.
    pub starts: Option<i64>,
}

impl Quiet {
    /// Reads one from a presence, given when its session's log was last
    /// written, the boot running now, whether it is parked, whether its
    /// background work runs and whether its process runs (see
    /// `Presence::runs`). A presence's last turn is gone once its day's
    /// usage starts again at midnight, so the log, written at each turn,
    /// stands in then: on 9 November one was said to have taken no turn
    /// since 22:32 when its last ended at 23:57. Only then, since a host
    /// writes every log it resumes: at 10:18 on 18 November one whose
    /// last turn was at 07:20 had its log written.
    #[must_use]
    pub fn from(
        p: &Presence,
        logged: Option<i64>,
        boot: Option<&str>,
        (parked, working): (bool, bool),
        runs: impl Fn(&Presence) -> bool,
    ) -> Option<Self> {
        let address = p.addresses.first()?.clone();
        let owner = p.owner.as_ref();
        let this_boot = boot.is_some() && owner.and_then(|o| o.boot_id.as_deref()) == boot;
        let started = owner.and_then(|o| o.started_at).map(|ms| ms / 1000);
        let turned = p.last_turn_ended();
        let ended = turned.or(logged);
        let unasked = p.wanting.is_null();
        Some(Self {
            address,
            session: p.session.clone(),
            quest: p.quest.clone(),
            orchestrated_by: p.orchestrated_by.clone(),
            listening: p.state == "listening" && !parked,
            running: this_boot && runs(p),
            turned,
            since: ended.or(started),
            started,
            unstarted: started.is_some() && ended < started,
            next: p
                .next_step()
                .filter(|next| unasked && !next.contains("Joel"))
                .map(str::to_owned),
            stepless: unasked
                && p.quest.as_deref().is_some_and(|q| !q.is_empty())
                && p.next_step().is_none(),
            working,
            routes: false,
            starts: None,
        })
    }

    /// The step it would be woken to at `now`, its spell begun at `since`:
    /// its own next step, or none named on its quest, said as the empty
    /// step. None while background work of its own has run under
    /// `WORKS_AT_MOST` of the spell, since that work's end wakes it.
    fn step(&self, since: i64, now: i64) -> Option<&str> {
        if self.working && now - since < WORKS_AT_MOST {
            return None;
        }
        self.next.as_deref().or(self.stepless.then_some(""))
    }

    /// Its orchestrator, when it is one of an orchestrator's, listening
    /// and running.
    fn led(&self) -> Option<&str> {
        let by = self.orchestrated_by.as_deref()?;
        (!by.is_empty()
            && !self.routes
            && !by.eq_ignore_ascii_case(&self.address)
            && self.listening
            && self.running)
            .then_some(by)
    }

    /// How its spell, begun at `since`, began, with `whose` for its own or
    /// its pi's.
    fn how(&self, since: i64, whose: &str, clock: &impl Fn(i64) -> String) -> String {
        match self.started.filter(|s| self.unstarted && *s != since) {
            Some(started) => format!(
                "{whose} last turn ended at {}, before {whose} pi started again at {}, \
                 with none since",
                clock(since),
                clock(started)
            ),
            None if self.unstarted => format!(
                "{whose} pi started at {} and has taken no turn since",
                clock(since)
            ),
            None => format!("{whose} last turn ended at {}", clock(since)),
        }
    }
}

/// A step a session was asked to take up: the spell it was asked in, when,
/// and the step, so it isn't asked of the same step twice.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Woken {
    pub since: i64,
    pub at: i64,
    /// The step, or empty when it was woken to its quest, naming none.
    #[serde(default)]
    pub next: String,
    /// When it was last woken to its quest, so one going back and forth
    /// between a step and none is woken to its quest at most once in
    /// `IDLE_FOR`, and its orchestrator hears of it in time.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to_quest: Option<i64>,
}

/// The spells already told and steps already woken to, by session, so
/// each is done once.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Told {
    #[serde(default)]
    pub spells: BTreeMap<String, i64>,
    #[serde(default)]
    pub woken: BTreeMap<String, Woken>,
}

/// A line for a session itself, to wake it, or for its orchestrator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tell {
    pub session: String,
    pub to: String,
    pub text: String,
    pub wakes: bool,
}

/// When `s`'s spell began. Its own last turn says so when its presence
/// names one. Without one, its day began again at midnight, or a host
/// wrote its log on resuming it, and neither starts a spell: so the spell
/// already told or woken in stands, unless what was read began later.
fn spell(s: &Quiet, told: &Told) -> Option<i64> {
    let read = read_spell(s, told)?;
    Some(s.starts.map_or(read, |starts| read.max(starts)))
}

/// When `s`'s spell began as what was read says, before its step's start.
fn read_spell(s: &Quiet, told: &Told) -> Option<i64> {
    let since = s.since?;
    if s.turned.is_some() {
        return Some(since);
    }
    let kept = told
        .spells
        .get(&s.session)
        .copied()
        .into_iter()
        .chain(told.woken.get(&s.session).map(|w| w.since))
        .filter(|kept| *kept <= since)
        .max();
    Some(kept.unwrap_or(since))
}

/// Takes back what `tell` counted said, when it couldn't be sent, so the
/// next tick says it again.
pub fn unsay(told: &mut Told, tell: &Tell) {
    if tell.wakes {
        told.woken.remove(&tell.session);
    } else {
        told.spells.remove(&tell.session);
    }
}

/// What to say at `now`, minding what was said already and counting it
/// said, with `clock` writing a moment for a person. A line that can't be
/// sent is taken back with `unsay` by its caller.
pub fn due(
    sessions: &[Quiet],
    told: &mut Told,
    now: i64,
    clock: impl Fn(i64) -> String,
) -> Vec<Tell> {
    // A spell of a session no longer seen needn't be remembered.
    let seen = |session: &String| sessions.iter().any(|s| &s.session == session);
    told.spells.retain(|session, _| seen(session));
    told.woken.retain(|session, _| seen(session));
    let mut tells = wakes(sessions, told, now, &clock);
    for s in sessions {
        let (Some(by), Some(since)) = (s.led(), spell(s, told)) else {
            continue;
        };
        if now - since < IDLE_FOR || told.spells.get(&s.session) == Some(&since) {
            continue;
        }
        let step = s.step(since, now);
        let woken = told
            .woken
            .get(&s.session)
            .filter(|w| step == Some(w.next.as_str()))
            .cloned();
        // One with a step to take up is asked first, and its orchestrator
        // hears only once the question has had time to take; one first seen
        // past two hours is asked this tick and told of in twenty minutes.
        if step.is_some() && woken.as_ref().is_none_or(|w| now - w.at < WAKE_AFTER) {
            continue;
        }
        told.spells.insert(s.session.clone(), since);
        let minutes = (now - since) / 60;
        let on = s
            .quest
            .as_deref()
            .map_or_else(String::new, |q| format!(" on {q}"));
        let woken = woken.map_or_else(String::new, |w| {
            let what = if w.next.is_empty() {
                "its quest"
            } else {
                "its next step"
            };
            if w.since == since {
                format!(
                    " It was asked at {} to take up {what}, and has taken no turn since.",
                    clock(w.at)
                )
            } else {
                format!(
                    " It was asked at {} to take up {what}, and the turns since haven't \
                     moved it.",
                    clock(w.at)
                )
            }
        });
        tells.push(Tell {
            session: s.session.clone(),
            to: by.to_owned(),
            text: format!(
                "{address}{on} has sat idle {h} h {m:02} min: {how} (session {session}).{woken} \
                 Give it its next piece, or park it if it has none.",
                address = s.address,
                h = minutes / 60,
                m = minutes % 60,
                how = s.how(since, "its", &clock),
                session = s.session,
            ),
            wakes: false,
        });
    }
    tells
}

/// The sessions to wake now, the longest idle first, at most
/// `WAKE_AT_MOST` of them. A session is asked once of each step: one that
/// answers without moving it is its orchestrator's to hear of, at two
/// hours, rather than asked again every twenty minutes all night.
fn wakes(
    sessions: &[Quiet],
    told: &mut Told,
    now: i64,
    clock: &impl Fn(i64) -> String,
) -> Vec<Tell> {
    let mut idle: Vec<(&Quiet, &str, i64, &str)> = sessions
        .iter()
        .filter_map(|s| {
            let by = s.led()?;
            let since = spell(s, told)?;
            let next = s.step(since, now)?;
            let asked = told.woken.get(&s.session).is_some_and(|w| {
                if next.is_empty() {
                    (w.next.is_empty() && w.since == since)
                        || w.to_quest.is_some_and(|at| now - at < IDLE_FOR)
                } else {
                    w.next == next
                }
            });
            (!asked && now - since >= WAKE_AFTER).then_some((s, by, since, next))
        })
        .collect();
    idle.sort_by_key(|(s, _, since, _)| (*since, s.session.clone()));
    let mut tells = Vec::new();
    for (s, by, since, next) in idle.into_iter().take(WAKE_AT_MOST) {
        let to_quest = if next.is_empty() {
            Some(now)
        } else {
            told.woken.get(&s.session).and_then(|w| w.to_quest)
        };
        told.woken.insert(
            s.session.clone(),
            Woken {
                since,
                at: now,
                next: next.to_owned(),
                to_quest,
            },
        );
        let minutes = (now - since) / 60;
        let (h, m) = (minutes / 60, minutes % 60);
        // Its work's end would have woken it, so one still running past
        // the hour is said, in case it is what the session waits on.
        let how = if s.working {
            format!(
                "{}, and background work of its own has run all that while, which may be \
                 what it waits on",
                s.how(since, "its", clock)
            )
        } else {
            s.how(since, "its", clock)
        };
        let text = if next.is_empty() {
            format!(
                "{address}: this session has sat idle {h} h {m:02} min on {quest}, naming no \
                 next step: {how}. Take up its next piece now, and focus a plan whose next \
                 item says it, so it can be woken to it. If it waits on someone, park saying \
                 on whom or on what; if nothing is left, park saying so. Answer this with \
                 re, saying which. {ANSWERS_GO} {by} hears of it at two hours idle.",
                address = s.address,
                quest = quoted(s.quest.as_deref().unwrap_or("its quest")),
            )
        } else {
            format!(
                "{address}: this session has sat idle {h} h {m:02} min with a next step of \
                 its own, \"{next}\": {how}. Take it up now. If it waits on someone or on its \
                 work, park saying on what; if it is another's, hand it to them by name. \
                 Answer this with re, saying which. {ANSWERS_GO} {by} hears of it at two \
                 hours idle.",
                address = s.address,
                next = quoted(next),
            )
        };
        tells.push(Tell {
            session: s.session.clone(),
            to: s.address.clone(),
            text,
            wakes: true,
        });
    }
    tells
}

/// A next step as a wake quotes it: its first `QUOTED` characters, since
/// it is the session's own words, sent back to it under cadence's name.
fn quoted(next: &str) -> String {
    let mut quoted: String = next.chars().take(QUOTED).collect();
    if quoted.len() < next.len() {
        quoted.push_str("...");
    }
    quoted
}

#[cfg(test)]
mod tests;
