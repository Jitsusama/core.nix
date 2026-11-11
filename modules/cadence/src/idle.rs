//! Telling an orchestrator of a session of its that has sat idle. A
//! session that finished its piece and waits to be told its next one does
//! nothing all night unless someone notices, so each one listening, its
//! pi running this boot, two hours past its last turn (or its start, when
//! it has taken none since) is named to the orchestrator its presence
//! names, once for each idle spell. Everything here is a decision over
//! what was read; main.rs reads and sends.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::mesh::Presence;

/// How long a session may sit idle before its orchestrator hears of it.
pub const IDLE_FOR: i64 = 2 * 3600;

/// A session, as far as telling of its idleness needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Quiet {
    pub address: String,
    pub session: String,
    pub quest: Option<String>,
    pub orchestrated_by: Option<String>,
    /// Waiting for something to do: listening, not working and not parked.
    pub listening: bool,
    /// Whether its pi runs, and ran during this boot.
    pub running: bool,
    /// When its idle spell began, in seconds: the later of its last turn's
    /// end and its pi's start.
    pub since: Option<i64>,
    /// Whether it has taken no turn since its pi started.
    pub unstarted: bool,
}

impl Quiet {
    /// Reads one from a presence, given when its session's log was last
    /// written, the boot running now and whether its process runs (see
    /// `Presence::runs`). A
    /// presence's last turn is gone once its day's usage starts again at
    /// midnight, so the log, written at each turn, counts too: on 9
    /// November one was said to have taken no turn since 22:32 when its
    /// last ended at 23:57.
    #[must_use]
    pub fn from(
        p: &Presence,
        logged: Option<i64>,
        boot: Option<&str>,
        parked: bool,
        runs: impl Fn(&Presence) -> bool,
    ) -> Option<Self> {
        let address = p.addresses.first()?.clone();
        let owner = p.owner.as_ref();
        let this_boot = boot.is_some() && owner.and_then(|o| o.boot_id.as_deref()) == boot;
        let started = owner.and_then(|o| o.started_at).map(|ms| ms / 1000);
        let turned = p.last_turn_ended().max(logged);
        Some(Self {
            address,
            session: p.session.clone(),
            quest: p.quest.clone(),
            orchestrated_by: p.orchestrated_by.clone(),
            listening: p.state == "listening" && !parked,
            running: this_boot && runs(p),
            since: turned.max(started),
            unstarted: started.is_some() && turned < started,
        })
    }
}

/// The spells already told, by session, so each is told once.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Told {
    #[serde(default)]
    pub spells: BTreeMap<String, i64>,
}

/// A line for an orchestrator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tell {
    pub session: String,
    pub to: String,
    pub text: String,
}

/// What to tell at `now`, minding what was told already and counting it
/// told, with `clock` writing a moment for a person. A line that can't be
/// sent is taken back out of `told` by its caller.
pub fn due(
    sessions: &[Quiet],
    told: &mut Told,
    now: i64,
    clock: impl Fn(i64) -> String,
) -> Vec<Tell> {
    // A spell of a session no longer seen needn't be remembered.
    told.spells
        .retain(|session, _| sessions.iter().any(|s| &s.session == session));
    let mut tells = Vec::new();
    for s in sessions {
        let (Some(by), Some(since)) = (&s.orchestrated_by, s.since) else {
            continue;
        };
        if by.is_empty()
            || by.eq_ignore_ascii_case(&s.address)
            || !s.listening
            || !s.running
            || now - since < IDLE_FOR
            || told.spells.get(&s.session) == Some(&since)
        {
            continue;
        }
        told.spells.insert(s.session.clone(), since);
        let minutes = (now - since) / 60;
        let how = if s.unstarted {
            format!(
                "its pi started at {} and has taken no turn since",
                clock(since)
            )
        } else {
            format!("its last turn ended at {}", clock(since))
        };
        let on = s
            .quest
            .as_deref()
            .map_or_else(String::new, |q| format!(" on {q}"));
        tells.push(Tell {
            session: s.session.clone(),
            to: by.clone(),
            text: format!(
                "{address}{on} has sat idle {h} h {m:02} min: {how} (session {session}). \
                 Give it its next piece, or park it if it has none.",
                address = s.address,
                h = minutes / 60,
                m = minutes % 60,
                session = s.session,
            ),
        });
    }
    tells
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i64 = 100_000;

    fn quiet(idle: i64) -> Quiet {
        Quiet {
            address: "8C7D8C31".into(),
            session: "s-1".into(),
            quest: Some("QEST-1".into()),
            orchestrated_by: Some("VP50PC".into()),
            listening: true,
            running: true,
            since: Some(NOW - idle),
            unstarted: false,
        }
    }

    fn clock(at: i64) -> String {
        format!("t{at}")
    }

    #[test]
    fn a_session_idle_two_hours_is_told_to_its_orchestrator_once() {
        let mut told = Told::default();
        let sessions = [quiet(IDLE_FOR + 5 * 60)];
        let tells = due(&sessions, &mut told, NOW, clock);
        assert_eq!(tells.len(), 1);
        assert_eq!(tells[0].to, "VP50PC");
        assert!(
            tells[0]
                .text
                .starts_with("8C7D8C31 on QEST-1 has sat idle 2 h 05 min: its last turn ended at"),
            "{}",
            tells[0].text
        );
        assert!(tells[0].text.contains("(session s-1)"));
        assert_eq!(
            due(&sessions, &mut told, NOW + 3600, clock),
            Vec::<Tell>::new()
        );
    }

    #[test]
    fn a_new_turn_starts_a_new_spell() {
        let mut told = Told::default();
        assert_eq!(due(&[quiet(IDLE_FOR)], &mut told, NOW, clock).len(), 1);
        let again = quiet(IDLE_FOR - 60);
        assert_eq!(
            due(std::slice::from_ref(&again), &mut told, NOW, clock),
            Vec::<Tell>::new()
        );
        assert_eq!(due(&[again], &mut told, NOW + 60, clock).len(), 1);
    }

    #[test]
    fn one_that_never_turned_since_its_start_says_so() {
        let mut told = Told::default();
        let s = Quiet {
            unstarted: true,
            ..quiet(IDLE_FOR)
        };
        let tells = due(&[s], &mut told, NOW, clock);
        assert!(
            tells[0]
                .text
                .contains("its pi started at t92800 and has taken no turn since")
        );
    }

    #[test]
    fn busy_parked_dead_unled_and_recent_sessions_are_not_told() {
        let long = IDLE_FOR * 3;
        let cases = [
            quiet(IDLE_FOR - 1),
            Quiet {
                listening: false,
                ..quiet(long)
            },
            Quiet {
                running: false,
                ..quiet(long)
            },
            Quiet {
                orchestrated_by: None,
                ..quiet(long)
            },
            Quiet {
                orchestrated_by: Some("8c7d8c31".into()),
                ..quiet(long)
            },
            Quiet {
                since: None,
                ..quiet(long)
            },
        ];
        for case in cases {
            let mut told = Told::default();
            assert!(
                due(std::slice::from_ref(&case), &mut told, NOW, clock).is_empty(),
                "{case:?}"
            );
        }
    }

    #[test]
    fn a_presence_counts_from_the_later_of_its_last_turn_and_its_start() {
        let presence = |turned: &str, started_ms: i64| -> Presence {
            serde_json::from_value(serde_json::json!({
                "session": "s-1",
                "addresses": ["8C7D8C31"],
                "state": "listening",
                "orchestratedBy": "VP50PC",
                "owner": {"pid": 7, "startedAt": started_ms, "bootId": "b"},
                "usage": {"turns": {"last": {"at": turned, "duration_ms": 60_000}}},
            }))
            .unwrap()
        };
        // 1_000_000 s is 1970-01-12T13:46:40Z.
        let turned_later = presence("1970-01-12T13:46:40Z", 900_000_000);
        let seven = |p: &Presence| p.owner.as_ref().and_then(|o| o.pid) == Some(7);
        let q = Quiet::from(&turned_later, None, Some("b"), false, seven).unwrap();
        assert_eq!(q.since, Some(1_000_060));
        assert!(!q.unstarted);
        assert!(q.running && q.listening);

        let started_later = presence("1970-01-12T13:46:40Z", 1_100_000_000);
        let q = Quiet::from(&started_later, None, Some("b"), false, |_| true).unwrap();
        assert_eq!(q.since, Some(1_100_000));
        assert!(q.unstarted);

        let other_boot = Quiet::from(&started_later, None, Some("c"), false, |_| true).unwrap();
        assert!(!other_boot.running);
        let parked = Quiet::from(&started_later, None, Some("b"), true, |_| true).unwrap();
        assert!(!parked.listening);
    }

    #[test]
    fn a_presence_whose_day_began_again_counts_from_its_log() {
        // Past midnight a presence's usage holds no last turn.
        let presence: Presence = serde_json::from_value(serde_json::json!({
            "session": "s-1",
            "addresses": ["1H56ZJ"],
            "state": "listening",
            "orchestratedBy": "WCJQSG",
            "owner": {"pid": 7, "startedAt": 1_000_000_000, "bootId": "b"},
            "usage": {"day": "1970-01-13", "turns": {"today": 0}},
        }))
        .unwrap();
        let q = Quiet::from(&presence, Some(1_005_100), Some("b"), false, |_| true).unwrap();
        assert_eq!(q.since, Some(1_005_100));
        assert!(!q.unstarted, "it took turns after its start");
        let q = Quiet::from(&presence, None, Some("b"), false, |_| true).unwrap();
        assert!(q.unstarted);
    }
}
