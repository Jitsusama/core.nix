//! Each realm's other machines gone down: one its node read ready within
//! the hour before, and that has read unreachable or refused for ten
//! minutes since, is told to Joel by a desktop notice and to the
//! orchestrator and its owner as a finding, once, and again once it's
//! back. On 17 November this machine powered off by itself and no one
//! heard for minutes; the node knows each link's state, so the tick reads
//! it rather than wait on another session to.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::Path;
use std::process::{Command, Stdio};

use serde::{Deserialize, Serialize};

use crate::mesh::Mesh;

/// How long a machine may read down before it's told: a link that drops
/// for a moment, as on a network change, is back within a tick or two.
const DOWN_FOR: i64 = 10 * 60;
/// How recently before it went down a machine must have read ready to be
/// told: one off for days, or one this machine slept away from, isn't news.
const READY_WITHIN: i64 = 60 * 60;
/// How long a machine no realm lists any more is remembered.
const FORGET: i64 = 2 * 86_400;

/// A machine as its realm's node reads it now.
#[derive(Debug, Clone, PartialEq)]
pub struct Seen {
    pub machine: String,
    pub realm: String,
    pub ready: bool,
    /// Why its node refuses it, when it does rather than only not reaching
    /// it, as for speaking another version.
    pub refused: Option<String>,
}

/// Each machine the realm's node lists but this one, from what
/// `mycelium machines` and `mycelium health` answer; none when the node
/// doesn't answer, so a node that's down reads no machine as down.
#[must_use]
pub fn seen(realm: &str, machines: &str, health: &str, this: &str) -> Vec<Seen> {
    let said = |answer: &str| {
        serde_json::Deserializer::from_str(answer)
            .into_iter::<serde_json::Value>()
            .map_while(Result::ok)
            .collect::<Vec<_>>()
    };
    let refused: BTreeMap<String, String> = said(health)
        .iter()
        .filter_map(|line| line["links"].as_array())
        .flatten()
        .filter(|link| link["state"] == "refused")
        .filter_map(|link| {
            let why = link["why"].as_str().unwrap_or("it gave no reason");
            Some((link["machine"].as_str()?.to_owned(), why.to_owned()))
        })
        .collect();
    said(machines)
        .iter()
        .find_map(|line| line["Ok"]["Machines"].as_array().cloned())
        .unwrap_or_default()
        .iter()
        .filter_map(|machine| {
            let name = machine["name"].as_str().filter(|name| *name != this)?;
            let ready = machine["state"] == "ready";
            Some(Seen {
                machine: name.to_owned(),
                realm: realm.to_owned(),
                ready,
                refused: if ready {
                    None
                } else {
                    refused.get(name).cloned()
                },
            })
        })
        .collect()
}

/// What cadence remembers of each machine between ticks.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Told {
    #[serde(default)]
    pub machines: BTreeMap<String, Spell>,
}

/// One machine's last reading.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Spell {
    /// When it last read ready.
    #[serde(default)]
    pub ready_at: Option<i64>,
    /// Since when it has read down, if it does.
    #[serde(default)]
    pub down_since: Option<i64>,
    /// Whether its going down was told, so its coming back is.
    #[serde(default)]
    pub told: bool,
    /// Whether Joel had the notice of its going down, so a finding sent
    /// again doesn't show it him twice.
    #[serde(default)]
    pub noticed: bool,
    /// When any realm last listed it.
    #[serde(default)]
    pub listed_at: i64,
}

/// What to tell of one machine.
#[derive(Debug, Clone, PartialEq)]
pub enum Tell {
    Down {
        machine: String,
        realm: String,
        minutes: i64,
        refused: Option<String>,
    },
    Back {
        machine: String,
        minutes: i64,
    },
}

/// What's newly down or back, given what each realm reads now, keeping
/// each machine's spell in `told`. A machine two realms list reads ready
/// when either reaches it.
pub fn due(seen: &[Seen], told: &mut Told, now: i64) -> Vec<Tell> {
    let mut by_machine: BTreeMap<&str, &Seen> = BTreeMap::new();
    for one in seen {
        by_machine
            .entry(&one.machine)
            .and_modify(|kept| {
                if one.ready && !kept.ready {
                    *kept = one;
                }
            })
            .or_insert(one);
    }
    let mut tells = Vec::new();
    for (machine, one) in by_machine {
        let spell = told.machines.entry(machine.to_owned()).or_default();
        spell.listed_at = now;
        if one.ready {
            if spell.told
                && let Some(since) = spell.down_since
            {
                tells.push(Tell::Back {
                    machine: machine.to_owned(),
                    minutes: (now - since) / 60,
                });
            }
            *spell = Spell {
                ready_at: Some(now),
                listed_at: now,
                ..Spell::default()
            };
            continue;
        }
        let since = *spell.down_since.get_or_insert(now);
        let lately = spell
            .ready_at
            .is_some_and(|ready| since - ready <= READY_WITHIN);
        if !spell.told && lately && now - since >= DOWN_FOR {
            spell.told = true;
            tells.push(Tell::Down {
                machine: machine.to_owned(),
                realm: one.realm.clone(),
                minutes: (now - since) / 60,
                refused: one.refused.clone(),
            });
        }
    }
    told.machines
        .retain(|_, spell| now - spell.listed_at < FORGET);
    tells
}

impl Tell {
    /// The machine it's about.
    #[must_use]
    pub fn machine(&self) -> &str {
        match self {
            Self::Down { machine, .. } | Self::Back { machine, .. } => machine,
        }
    }

    /// The notice's words, beneath the machine's name: what's true, and
    /// what Joel can do about it.
    #[must_use]
    pub fn notice(&self) -> String {
        match self {
            Self::Down {
                minutes,
                refused: None,
                ..
            } => format!(
                "Can't be reached for {minutes} min. Its work waits until it's back: check \
                 it's awake and online."
            ),
            Self::Down {
                minutes,
                refused: Some(why),
                ..
            } => format!("Turned away for {minutes} min: {why}."),
            Self::Back { minutes, .. } => format!("Back after {minutes} min down."),
        }
    }

    /// The finding's words, for the orchestrator and the machine's owner.
    #[must_use]
    pub fn finding(&self) -> String {
        match self {
            Self::Down {
                machine,
                realm,
                minutes,
                refused,
            } => {
                let state = refused.as_ref().map_or_else(
                    || "unreachable".to_owned(),
                    |why| format!("refused ({why})"),
                );
                format!(
                    "Cadence alarm: {machine} has read {state} in the {realm} realm for \
                     {minutes} min, after reading ready within the hour before. Joel has a \
                     notice of it."
                )
            }
            Self::Back { machine, minutes } => {
                format!("{machine} reads ready again, after {minutes} min down.")
            }
        }
    }
}

/// Tells each of `tells`, by `notify` to Joel once a spell, and by `send`
/// as a finding to the orchestrator and the owner `owner_of` names for
/// `down:<machine>`.
/// One not sent is left untold in `told`, so the next tick tells it again.
///
/// # Errors
///
/// The last error sending one, once each is tried.
pub fn tell<E>(
    tells: &[Tell],
    told: &mut Told,
    orchestrator: &str,
    owner_of: impl Fn(&str) -> Option<String>,
    notify: impl Fn(&Tell),
    send: impl Fn(&str, String) -> Result<(), E>,
) -> Result<(), E> {
    let mut failed = None;
    for tell in tells {
        let mut to = vec![orchestrator.to_owned()];
        to.extend(
            owner_of(&format!("down:{}", tell.machine()))
                .filter(|owner| !owner.eq_ignore_ascii_case(orchestrator)),
        );
        let mut delivered = true;
        for address in &to {
            if let Err(err) = send(address, tell.finding()) {
                failed = Some(err);
                delivered = false;
            }
        }
        let spell = told.machines.entry(tell.machine().to_owned()).or_default();
        if !delivered {
            // A going down not told is told next tick; a coming back not
            // told is let go, as the machine is back.
            spell.told = false;
        }
        if !spell.noticed {
            notify(tell);
        }
        // Only a going down is kept as noticed: a machine back starts
        // its next spell afresh.
        spell.noticed = matches!(tell, Tell::Down { .. });
    }
    failed.map_or(Ok(()), Err)
}

/// One tick's watch: reads each of `realms`, tells what's newly down or
/// back, and keeps each machine's spell in `down.json` in `folder`.
///
/// # Errors
///
/// When the spells can't be written, or a finding can't be sent.
pub fn run(
    mesh: &Mesh,
    folder: &Path,
    orchestrator: &str,
    owners: &BTreeMap<String, String>,
    realms: &[String],
    now: i64,
) -> io::Result<()> {
    let path = folder.join("down.json");
    let mut told: Told = fs::read(&path)
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default();
    let this = terminal_kit::environment::machine();
    let ask = |realm: &str, what: &str| crate::command("mycelium", &["--realm", realm, what]);
    let read: Vec<Seen> = realms
        .iter()
        .filter_map(|realm| {
            let machines = ask(realm, "machines").ok()?;
            let health = ask(realm, "health").unwrap_or_default();
            Some(seen(realm, &machines, &health, &this))
        })
        .flatten()
        .collect();
    let tells = due(&read, &mut told, now);
    let told_all = tell(
        &tells,
        &mut told,
        orchestrator,
        |key| crate::findings::owner_of(key, owners),
        |one| {
            // Going down waits on Joel, so it stays in For You until read;
            // coming back needs nothing of him.
            let urgency = match one {
                Tell::Down { .. } => "critical",
                Tell::Back { .. } => "normal",
            };
            let _ = Command::new("notify-send")
                .args([
                    "-a",
                    "Machines",
                    "-u",
                    urgency,
                    one.machine(),
                    &one.notice(),
                ])
                .stdin(Stdio::null())
                .status();
        },
        |to, text| mesh.send(&crate::message(to, "finding", text)),
    );
    for one in &tells {
        println!("{}", one.finding());
    }
    crate::write_whole(&path, &serde_json::to_string_pretty(&told)?)?;
    told_all
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;

    const NOW: i64 = 1_795_200_000;

    fn one(machine: &str, ready: bool) -> Seen {
        Seen {
            machine: machine.into(),
            realm: "work".into(),
            ready,
            refused: None,
        }
    }

    #[test]
    fn the_nodes_answers_are_read_for_each_other_machine() {
        let machines = concat!(
            r#"{"Ok":{"Machines":[{"name":"here","state":"ready"},"#,
            r#"{"name":"yonder","state":"unreachable"},{"name":"far","state":"unreachable"},"#,
            r#"{"name":"near","state":"ready"}]}}"#,
            "\n",
            r#"{"Ok":{"Builders":[]}}"#,
        );
        let health = r#"{"links":[{"machine":"far","state":"refused","why":"it speaks v3"},
            {"machine":"yonder","state":"unreachable"},{"machine":"near","state":"up"}]}"#;
        let read = seen("work", machines, health, "here");
        assert_eq!(
            read,
            [
                one("yonder", false),
                Seen {
                    refused: Some("it speaks v3".into()),
                    ..one("far", false)
                },
                one("near", true),
            ],
            "the machines weren't read as the node says, or this one wasn't left out"
        );
        assert!(
            seen("work", "", "", "here").is_empty(),
            "a node that doesn't answer read machines"
        );
    }

    #[test]
    fn a_machine_down_ten_minutes_is_told_once_and_again_once_back() {
        let mut told = Told::default();
        assert!(
            due(&[one("yonder", true)], &mut told, NOW).is_empty(),
            "a ready machine was told"
        );
        let mut now = NOW + 5 * 60;
        assert!(
            due(&[one("yonder", false)], &mut told, now).is_empty(),
            "a machine down a moment was told"
        );
        // Ten minutes from when it first read down.
        now += 5 * 60;
        assert!(
            due(&[one("yonder", false)], &mut told, now).is_empty(),
            "a machine down five minutes was told"
        );
        now += 5 * 60;
        assert_eq!(
            due(&[one("yonder", false)], &mut told, now),
            [Tell::Down {
                machine: "yonder".into(),
                realm: "work".into(),
                minutes: 10,
                refused: None,
            }],
            "a machine down ten minutes wasn't told"
        );
        now += 25 * 60;
        assert!(
            due(&[one("yonder", false)], &mut told, now).is_empty(),
            "a machine still down was told again"
        );
        now += 5 * 60;
        assert_eq!(
            due(&[one("yonder", true)], &mut told, now),
            [Tell::Back {
                machine: "yonder".into(),
                minutes: 40,
            }],
            "a machine back wasn't told"
        );
        now += 5 * 60;
        assert!(
            due(&[one("yonder", true)], &mut told, now).is_empty(),
            "a machine back was told twice"
        );
    }

    #[test]
    fn a_machine_long_off_or_back_in_moments_is_never_told() {
        let mut told = Told::default();
        // Off since before cadence first read it.
        for minutes in [0, 5, 10, 60] {
            assert!(
                due(&[one("yonder", false)], &mut told, NOW + minutes * 60).is_empty(),
                "a machine never read ready was told at {minutes} min"
            );
        }
        // Ready, then this machine slept eight hours and its link is slow
        // to come back.
        due(&[one("near", true)], &mut told, NOW);
        let woke = NOW + 8 * 3600;
        for minutes in [0, 5, 15] {
            assert!(
                due(&[one("near", false)], &mut told, woke + minutes * 60).is_empty(),
                "a machine last ready before this one slept was told at {minutes} min"
            );
        }
        // A link that drops for a tick.
        due(&[one("far", true)], &mut told, NOW);
        due(&[one("far", false)], &mut told, NOW + 5 * 60);
        assert!(
            due(&[one("far", true)], &mut told, NOW + 10 * 60).is_empty(),
            "a machine back within ten minutes was told"
        );
    }

    #[test]
    fn a_machine_either_realm_reaches_is_up_and_one_unlisted_is_forgotten() {
        let mut told = Told::default();
        let personal = Seen {
            realm: "personal".into(),
            ..one("yonder", true)
        };
        for minutes in [0, 5, 10, 15] {
            assert!(
                due(
                    &[one("yonder", minutes == 0), personal.clone()],
                    &mut told,
                    NOW + minutes * 60
                )
                .is_empty(),
                "a machine one realm still reaches was told at {minutes} min"
            );
        }
        due(&[], &mut told, NOW + FORGET + 15 * 60);
        assert!(
            told.machines.is_empty(),
            "a machine no realm lists was kept"
        );
    }

    #[test]
    fn each_is_told_to_joel_the_orchestrator_and_its_owner() {
        let down = Tell::Down {
            machine: "yonder".into(),
            realm: "work".into(),
            minutes: 10,
            refused: Some("it speaks v3".into()),
        };
        let mut told = Told::default();
        told.machines.insert(
            "yonder".into(),
            Spell {
                ready_at: Some(NOW - 20 * 60),
                down_since: Some(NOW - 10 * 60),
                told: true,
                ..Spell::default()
            },
        );
        let noticed = RefCell::new(Vec::new());
        let findings = RefCell::new(Vec::new());
        let owner = |key: &str| (key == "down:yonder").then(|| "OWNER".to_owned());
        let result = tell(
            &[down],
            &mut told,
            "ORCH",
            owner,
            |one| {
                let line = format!("{}: {}", one.machine(), one.notice());
                noticed.borrow_mut().push(line);
            },
            |to, text| {
                findings.borrow_mut().push((to.to_owned(), text));
                if to == "OWNER" { Err("gone") } else { Ok(()) }
            },
        );
        assert_eq!(result, Err("gone"), "a send that failed wasn't said");
        assert_eq!(
            noticed.into_inner(),
            ["yonder: Turned away for 10 min: it speaks v3."],
            "Joel's notice wasn't as written"
        );
        let to: Vec<String> = findings
            .into_inner()
            .into_iter()
            .map(|(to, _)| to)
            .collect();
        assert_eq!(to, ["ORCH", "OWNER"], "the finding didn't go to both");
        assert!(
            !told.machines["yonder"].told,
            "a going down not sent was kept as told"
        );
        let again = due(&[one("yonder", false)], &mut told, NOW);
        let noticed = RefCell::new(0);
        let result: Result<(), &str> = tell(
            &again,
            &mut told,
            "ORCH",
            owner,
            |_| *noticed.borrow_mut() += 1,
            |_, _| Ok(()),
        );
        assert_eq!(
            (again.len(), result, noticed.into_inner()),
            (1, Ok(()), 0),
            "a finding sent again wasn't sent, or showed Joel the notice twice"
        );
    }
}
