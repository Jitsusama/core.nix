//! Each realm's other machines gone down while they hold work: one its node
//! read ready within the hour before, with a session on it busy, and that
//! has read unreachable or turned away for ten minutes of readings since,
//! is told to Joel by a desktop notice and to the orchestrator and its owner
//! as a finding, once, and again once it's back. On 17 November this machine
//! powered off by itself and no one heard for minutes; the node knows each
//! link's state, so the tick reads it rather than wait on another session
//! to. A machine with no work sleeps as its node lets it, which isn't news,
//! and every other machine dropping at once is this one cut off.

use std::collections::{BTreeMap, BTreeSet};
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
/// A gap between readings past this starts a machine's time down again, as
/// across a sleep of this machine or a node not answering: one tick missed
/// is absorbed, and only readings in a row count as being down.
const READ_GAP: i64 = 15 * 60;
/// How long a machine no realm lists any more is remembered.
const FORGET: i64 = 2 * 86_400;
/// The most of a node's words a finding carries.
const SAID: usize = 120;

/// What a realm's node reads of a machine now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reading {
    Ready,
    /// Unreachable, or turned away.
    Down,
    /// In a state that's neither, or listed by a realm that didn't answer.
    Unread,
}

/// A machine as its realm's node reads it now.
#[derive(Debug, Clone, PartialEq)]
pub struct Seen {
    pub machine: String,
    pub realm: String,
    pub reading: Reading,
    /// Whether a session on it is busy, so its node keeps it awake.
    pub working: bool,
    /// Why its node turns it away, when it does rather than only not
    /// reaching it, as for speaking another version.
    pub refused: Option<String>,
}

/// Each machine the realm's node lists but this one, from what `mycelium
/// machines`, `sessions` and `health` answer; none when the node doesn't
/// answer `machines`.
#[must_use]
pub fn seen(realm: &str, machines: &str, sessions: &str, health: &str, this: &str) -> Vec<Seen> {
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
    let working: BTreeSet<String> = said(sessions)
        .iter()
        .filter_map(|line| line["Ok"]["Sessions"].as_array())
        .flatten()
        .filter(|session| session["state"] == "busy")
        .filter_map(|session| session["machine"].as_str().map(str::to_owned))
        .collect();
    crate::elsewhere::machines(machines)
        .unwrap_or_default()
        .iter()
        .filter_map(|machine| {
            let name = machine["name"].as_str().filter(|name| *name != this)?;
            let turned_away = refused.get(name).cloned();
            let reading = match machine["state"].as_str() {
                Some("ready") => Reading::Ready,
                Some("unreachable") => Reading::Down,
                _ if turned_away.is_some() => Reading::Down,
                _ => Reading::Unread,
            };
            Some(Seen {
                machine: name.to_owned(),
                realm: realm.to_owned(),
                reading,
                working: working.contains(name),
                refused: turned_away.filter(|_| reading == Reading::Down),
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

/// One machine's readings.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Spell {
    /// When it last read ready.
    #[serde(default)]
    pub ready_at: Option<i64>,
    /// Whether a session on it was busy when it last read ready.
    #[serde(default)]
    pub working: bool,
    /// When it was last read ready or down.
    #[serde(default)]
    pub read_at: Option<i64>,
    /// Since when it has read down, if it does.
    #[serde(default)]
    pub down_since: Option<i64>,
    /// Who has had the finding of its going down, so one not sent is sent
    /// again alone, and its coming back goes to each who had it.
    #[serde(default)]
    pub sent: BTreeSet<String>,
    /// Joel's notice of its going down, by the id its back replaces it by.
    #[serde(default)]
    pub notice: Option<u32>,
    /// Whether its going down has gone to everyone, notice and findings.
    #[serde(default)]
    pub told: bool,
    /// Which realms list it, so one that doesn't answer leaves it unread.
    #[serde(default)]
    pub realms: BTreeSet<String>,
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
        /// Who had the finding of its going down.
        to: BTreeSet<String>,
        /// Joel's notice of its going down, to replace.
        notice: Option<u32>,
    },
}

/// Each machine's readings merged across realms: ready when any reaches
/// it, down when one reads it down and none ready.
fn merged(seen: &[Seen]) -> BTreeMap<&str, (Seen, BTreeSet<String>)> {
    let mut by: BTreeMap<&str, (Seen, BTreeSet<String>)> = BTreeMap::new();
    for one in seen {
        let (kept, realms) = by
            .entry(&one.machine)
            .or_insert_with(|| (one.clone(), BTreeSet::new()));
        realms.insert(one.realm.clone());
        kept.working |= one.working;
        let rank = |reading| match reading {
            Reading::Ready => 2,
            Reading::Down => 1,
            Reading::Unread => 0,
        };
        if rank(one.reading) > rank(kept.reading) {
            let working = kept.working;
            *kept = Seen {
                working,
                ..one.clone()
            };
        }
    }
    by
}

/// What's newly down or back, given what each of the `answered` realms
/// reads now, keeping each machine's spell in `told`.
pub fn due(seen: &[Seen], answered: &[String], told: &mut Told, now: i64) -> Vec<Tell> {
    let mut by = merged(seen);
    for (machine, (one, realms)) in &mut by {
        let spell = told.machines.entry((*machine).to_owned()).or_default();
        spell.listed_at = now;
        let unanswered = spell.realms.iter().any(|realm| !answered.contains(realm));
        if unanswered && one.reading != Reading::Ready {
            one.reading = Reading::Unread;
        }
        if unanswered {
            spell.realms.append(realms);
        } else {
            spell.realms = std::mem::take(realms);
        }
    }
    // Every machine up within the hour reading down at once, two or more,
    // is this machine cut off; with one, the two can't be told apart.
    let lately_up: Vec<Reading> = by
        .iter()
        .filter(|(machine, _)| {
            told.machines[**machine]
                .ready_at
                .is_some_and(|ready| now - ready <= READY_WITHIN)
        })
        .map(|(_, (one, _))| one.reading)
        .collect();
    let cut_off = lately_up.len() >= 2 && lately_up.iter().all(|r| *r == Reading::Down);
    let mut tells = Vec::new();
    for (machine, (one, _)) in by {
        let spell = told
            .machines
            .get_mut(machine)
            .expect("each machine was kept above");
        match one.reading {
            Reading::Unread => {}
            Reading::Down if cut_off => {}
            Reading::Ready => {
                if let Some(since) = spell.down_since
                    && (spell.notice.is_some() || !spell.sent.is_empty())
                {
                    tells.push(Tell::Back {
                        machine: machine.to_owned(),
                        minutes: (now - since) / 60,
                        to: std::mem::take(&mut spell.sent),
                        notice: spell.notice,
                    });
                }
                *spell = Spell {
                    ready_at: Some(now),
                    working: one.working,
                    read_at: Some(now),
                    realms: std::mem::take(&mut spell.realms),
                    listed_at: now,
                    ..Spell::default()
                };
            }
            Reading::Down => {
                let begun = spell.notice.is_some() || !spell.sent.is_empty();
                let gap = spell.read_at.is_none_or(|read| now - read > READ_GAP);
                if spell.down_since.is_none() || (gap && !begun) {
                    spell.down_since = Some(now);
                }
                spell.read_at = Some(now);
                let since = spell.down_since.unwrap_or(now);
                let lately = spell
                    .ready_at
                    .is_some_and(|ready| since - ready <= READY_WITHIN);
                if !spell.told && spell.working && lately && now - since >= DOWN_FOR {
                    tells.push(Tell::Down {
                        machine: machine.to_owned(),
                        realm: one.realm.clone(),
                        minutes: (now - since) / 60,
                        refused: one.refused.clone(),
                    });
                }
            }
        }
    }
    told.machines
        .retain(|_, spell| now - spell.listed_at < FORGET);
    tells
}

/// Up to `SAID` of a node's words, on one line and quoted, so a finding
/// shows them as the node's rather than cadence's.
fn quoted(words: &str) -> String {
    let line: String = words
        .chars()
        .map(|c| match c {
            '"' => '\'',
            c if c.is_control() => ' ',
            c => c,
        })
        .take(SAID)
        .collect();
    format!("\"{line}\"")
}

impl Tell {
    /// The machine it's about.
    #[must_use]
    pub fn machine(&self) -> &str {
        match self {
            Self::Down { machine, .. } | Self::Back { machine, .. } => machine,
        }
    }

    /// How urgent Joel's notice is: going down stops its work, so it waits
    /// in For You until read; coming back needs nothing of him.
    #[must_use]
    pub fn urgency(&self) -> &'static str {
        match self {
            Self::Down { .. } => "critical",
            Self::Back { .. } => "normal",
        }
    }

    /// The notice's words, beneath the machine's name: what's true, and
    /// what Joel can do about it. The node's own words stay in the finding.
    #[must_use]
    pub fn notice(&self) -> String {
        match self {
            Self::Down {
                minutes,
                refused: None,
                ..
            } => format!(
                "Can't be reached for {minutes} min, and its work waits until it's back. \
                 Check it's awake and online."
            ),
            Self::Down { minutes, .. } => format!(
                "Won't link with this machine for {minutes} min, and its work waits. The \
                 orchestrator has been told why."
            ),
            Self::Back { minutes, .. } => {
                format!("Back after {minutes} min, and its work carries on.")
            }
        }
    }

    /// The finding's words, for the orchestrator and the machine's owner,
    /// saying whether Joel has a notice of it.
    #[must_use]
    pub fn finding(&self, noticed: bool) -> String {
        match self {
            Self::Down {
                machine,
                realm,
                minutes,
                refused,
            } => {
                let state = refused.as_ref().map_or_else(
                    || "unreachable".to_owned(),
                    |why| format!("turned away, its node saying {}", quoted(why)),
                );
                let joel = if noticed {
                    "Joel has a notice of it."
                } else {
                    "Joel's notice of it couldn't be shown."
                };
                format!(
                    "Cadence alarm: {machine} has read {state} in the {realm} realm for \
                     {minutes} min, after reading ready within the hour before with a \
                     session on it busy. {joel}"
                )
            }
            Self::Back {
                machine, minutes, ..
            } => format!("{machine} reads ready again, after {minutes} min down."),
        }
    }
}

/// Tells each of `tells`: a going down by `notify` to Joel once a spell and
/// by `send` as a finding to the orchestrator and the owner `owner_of`
/// names for `down:<machine>`, each who hasn't had it; a coming back to
/// each who had its going down, replacing Joel's notice. A going down not
/// told to all is left untold in `told`, so the next tick tells the rest.
///
/// # Errors
///
/// The last error sending one, once each is tried.
pub fn tell<E>(
    tells: &[Tell],
    told: &mut Told,
    orchestrator: &str,
    owner_of: impl Fn(&str) -> Option<String>,
    notify: impl Fn(&Tell) -> Option<u32>,
    send: impl Fn(&str, String) -> Result<(), E>,
) -> Result<(), E> {
    let mut failed = None;
    for one in tells {
        match one {
            Tell::Down { machine, .. } => {
                let mut to = vec![orchestrator.to_owned()];
                to.extend(
                    owner_of(&format!("down:{machine}"))
                        .filter(|owner| !owner.eq_ignore_ascii_case(orchestrator)),
                );
                let spell = told.machines.entry(machine.clone()).or_default();
                if spell.notice.is_none() {
                    spell.notice = notify(one);
                }
                for address in &to {
                    if spell.sent.contains(address) {
                        continue;
                    }
                    match send(address, one.finding(spell.notice.is_some())) {
                        Ok(()) => {
                            spell.sent.insert(address.clone());
                        }
                        Err(err) => failed = Some(err),
                    }
                }
                spell.told = spell.notice.is_some() && to.iter().all(|a| spell.sent.contains(a));
            }
            Tell::Back { to, notice, .. } => {
                if notice.is_some() {
                    notify(one);
                }
                // A coming back not sent is let go, as the machine is back.
                for address in to {
                    if let Err(err) = send(address, one.finding(notice.is_some())) {
                        failed = Some(err);
                    }
                }
            }
        }
    }
    failed.map_or(Ok(()), Err)
}

/// The notice's body as markup reads it, so nothing in it is taken for a
/// tag or a link.
fn escaped(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Shows Joel `one`'s notice, a coming back in place of its going down,
/// with its id when it was shown.
fn notify(one: &Tell) -> Option<u32> {
    let mut args = vec!["-p", "-a", "Machines", "-u", one.urgency()];
    let replaces = match one {
        Tell::Back {
            notice: Some(id), ..
        } => id.to_string(),
        _ => String::new(),
    };
    if !replaces.is_empty() {
        args.extend(["-r", &replaces]);
    }
    let body = escaped(&one.notice());
    args.extend(["--", one.machine(), &body]);
    let out = Command::new("notify-send")
        .args(&args)
        .stdin(Stdio::null())
        .output()
        .ok()
        .filter(|out| out.status.success())?;
    Some(
        String::from_utf8_lossy(&out.stdout)
            .trim()
            .parse()
            .unwrap_or(0),
    )
}

/// One tick's watch: reads each of `realms`, tells what's newly down or
/// back, and keeps each machine's spell in `down.json` in `folder`. The
/// spells are written as told before anything is, so a write that fails
/// tells nothing rather than the same each tick.
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
    let mut answered = Vec::new();
    let mut read = Vec::new();
    for realm in realms {
        let Ok(machines) = ask(realm, "machines") else {
            continue;
        };
        let sessions = ask(realm, "sessions").unwrap_or_default();
        let health = ask(realm, "health").unwrap_or_default();
        answered.push(realm.clone());
        read.extend(seen(realm, &machines, &sessions, &health, &this));
    }
    let tells = due(&read, &answered, &mut told, now);
    let owner_of = |key: &str| crate::findings::owner_of(key, owners);
    let write = |told: &Told| crate::write_whole(&path, &serde_json::to_string_pretty(told)?);
    let mut promised = told.clone();
    tell::<io::Error>(
        &tells,
        &mut promised,
        orchestrator,
        owner_of,
        |_| Some(0),
        |_, _| Ok(()),
    )?;
    write(&promised)?;
    let told_all = tell(
        &tells,
        &mut told,
        orchestrator,
        owner_of,
        notify,
        |to, text| mesh.send(&crate::message(to, "finding", text)),
    );
    for one in &tells {
        println!("{}", one.finding(true));
    }
    write(&told)?;
    told_all
}

#[cfg(test)]
mod tests;
