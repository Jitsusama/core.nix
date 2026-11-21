//! Each realm's other machines gone down while they hold work: one its node
//! read ready within the hour before, on mains power with a session on it
//! busy, and that has read unreachable for ten minutes of readings since,
//! is told to Joel by a desktop notice and to the orchestrator and its owner
//! as a finding, once, and again once it's back. On 17 November this machine
//! powered off by itself and no one heard for minutes; the node knows each
//! link's state, so the tick reads it rather than wait on another session
//! to. A machine with no work sleeps as its node lets it, and one on battery
//! sleeps when its lid shuts whatever it holds, so neither is news. Two or
//! more machines that read ready a tick before all dropping at once is this
//! one cut off; with one other machine, the two can't be told apart.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{self, Read};
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

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
/// How long a node may take to answer: one stalled on a peer it can't reach
/// would otherwise hold the whole tick, and every alarm with it.
const ASK_WITHIN: Duration = Duration::from_secs(30);

/// What a realm's node reads of a machine now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reading {
    Ready,
    Down,
    /// In a state that's neither, or listed by a realm that didn't answer.
    Unread,
}

/// A machine as its realm's node reads it now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Seen {
    pub machine: String,
    pub realm: String,
    pub reading: Reading,
    /// Whether a session on it is busy, so its node keeps it awake; none
    /// when the node didn't say which sessions are.
    pub working: Option<bool>,
    /// Whether it runs on battery, where a lid shut sleeps it however busy.
    pub battery: bool,
}

/// Each machine the realm's node lists but this one, from what `mycelium
/// machines` and `sessions` answer; none when the node doesn't answer
/// `machines`.
#[must_use]
pub fn seen(realm: &str, machines: &str, sessions: &str, this: &str) -> Vec<Seen> {
    let listed: Option<Vec<serde_json::Value>> = serde_json::Deserializer::from_str(sessions)
        .into_iter::<serde_json::Value>()
        .map_while(Result::ok)
        .find_map(|said| said["Ok"]["Sessions"].as_array().cloned());
    let working: Option<BTreeSet<String>> = listed.map(|sessions| {
        sessions
            .iter()
            .filter(|session| session["state"] == "busy")
            .filter_map(|session| session["machine"].as_str().map(str::to_owned))
            .collect()
    });
    crate::elsewhere::machines(machines)
        .unwrap_or_default()
        .iter()
        .filter_map(|machine| {
            let name = machine["name"].as_str().filter(|name| *name != this)?;
            let reading = match machine["state"].as_str() {
                Some("ready") => Reading::Ready,
                Some("unreachable") => Reading::Down,
                _ => Reading::Unread,
            };
            Some(Seen {
                machine: name.to_owned(),
                realm: realm.to_owned(),
                reading,
                working: working.as_ref().map(|busy| busy.contains(name)),
                battery: machine["power"]["on_battery"].as_bool() == Some(true),
            })
        })
        .collect()
}

/// What cadence remembers of each machine between ticks.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Told {
    #[serde(default)]
    pub machines: BTreeMap<String, Spell>,
    /// When the machines were last read, so those ready then are told
    /// from those that went down earlier.
    #[serde(default)]
    pub read_at: Option<i64>,
    /// Whether this machine reads as cut off: it holds until any machine
    /// reads ready again, however long that takes.
    #[serde(default)]
    pub cut_off: bool,
}

/// One machine's readings.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Spell {
    /// When it last read ready.
    #[serde(default)]
    pub ready_at: Option<i64>,
    /// Whether a session on it was busy when it last read ready, as last
    /// said: a tick the node didn't say keeps what the one before did.
    #[serde(default)]
    pub working: bool,
    /// Whether it ran on battery when it last read ready.
    #[serde(default)]
    pub battery: bool,
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Tell {
    Down {
        machine: String,
        realm: String,
        minutes: i64,
    },
    Back {
        machine: String,
        minutes: i64,
        /// Whether it went unread a while before it was back, so `minutes`
        /// counts only to its last reading down.
        at_least: bool,
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
    let rank = |reading| match reading {
        Reading::Ready => 2,
        Reading::Down => 1,
        Reading::Unread => 0,
    };
    for one in seen {
        let (kept, realms) = by
            .entry(&one.machine)
            .or_insert_with(|| (one.clone(), BTreeSet::new()));
        realms.insert(one.realm.clone());
        let working = match (kept.working, one.working) {
            (None, None) => None,
            (a, b) => Some(a == Some(true) || b == Some(true)),
        };
        if rank(one.reading) > rank(kept.reading) {
            *kept = one.clone();
        }
        kept.working = working;
    }
    by
}

/// Whether this machine reads as cut off, kept in `told`: two or more
/// machines ready at the last reading and none ready now, until one reads
/// ready again. A machine that went down earlier wasn't ready then, so it
/// doesn't count toward dropping at once.
fn cut_off(by: &BTreeMap<&str, (Seen, BTreeSet<String>)>, told: &mut Told, now: i64) -> bool {
    let last = told.read_at.replace(now);
    let just_up = by
        .keys()
        .filter(|machine| last.is_some() && told.machines[**machine].ready_at == last)
        .count();
    if by.values().any(|(one, _)| one.reading == Reading::Ready) {
        told.cut_off = false;
    } else if just_up >= 2 {
        told.cut_off = true;
    }
    told.cut_off
}

/// What's newly down or back, given what each of the `answered` realms of
/// those `asked` reads now, keeping each machine's spell in `told`.
pub fn due(
    seen: &[Seen],
    asked: &[String],
    answered: &[String],
    told: &mut Told,
    now: i64,
) -> Vec<Tell> {
    let mut by = merged(seen);
    for (machine, (one, realms)) in &mut by {
        let spell = told.machines.entry((*machine).to_owned()).or_default();
        spell.listed_at = now;
        // A realm no longer asked never answers again, so it's let go.
        spell.realms.retain(|realm| asked.contains(realm));
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
    let cut_off = cut_off(&by, told, now);
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
                    let last_down = spell.read_at.unwrap_or(now);
                    let at_least = now - last_down > READ_GAP;
                    let until = if at_least { last_down } else { now };
                    tells.push(Tell::Back {
                        machine: machine.to_owned(),
                        minutes: (until - since) / 60,
                        at_least,
                        to: std::mem::take(&mut spell.sent),
                        notice: spell.notice,
                    });
                }
                *spell = Spell {
                    ready_at: Some(now),
                    working: one.working.unwrap_or(spell.working),
                    battery: one.battery,
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
                let holds = spell.working && !spell.battery;
                if !spell.told && holds && lately && now - since >= DOWN_FOR {
                    tells.push(Tell::Down {
                        machine: machine.to_owned(),
                        realm: one.realm.clone(),
                        minutes: (now - since) / 60,
                    });
                }
            }
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
    /// what Joel can do about it.
    #[must_use]
    pub fn notice(&self) -> String {
        match self {
            Self::Down { minutes, .. } => format!(
                "Can't be reached for {minutes} min, and its work waits until it's back. \
                 Check it's awake and online."
            ),
            Self::Back {
                minutes,
                at_least: false,
                ..
            } => format!("Back after {minutes} min, and its work carries on."),
            Self::Back { minutes, .. } => {
                format!("Back after at least {minutes} min, and its work carries on.")
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
            } => {
                let joel = if noticed {
                    "Joel has a notice of it."
                } else {
                    "Joel's notice of it couldn't be shown."
                };
                format!(
                    "Cadence alarm: {machine} has read unreachable in the {realm} realm for \
                     {minutes} min, after reading ready within the hour before, on mains \
                     power with a session on it busy. {joel}"
                )
            }
            Self::Back {
                machine,
                minutes,
                at_least,
                ..
            } => {
                let least = if *at_least { "at least " } else { "" };
                format!("{machine} reads ready again, after {least}{minutes} min down.")
            }
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

/// What `notify-send` is given to show `one`: printing the id it's shown
/// by, and a coming back replacing its going down.
fn notice_args(one: &Tell) -> Vec<String> {
    let mut args: Vec<String> = ["-p", "-a", "Machines", "-u", one.urgency()]
        .map(str::to_owned)
        .into();
    if let Tell::Back {
        notice: Some(id), ..
    } = one
    {
        args.extend(["-r".to_owned(), id.to_string()]);
    }
    args.extend([
        "--".to_owned(),
        one.machine().to_owned(),
        escaped(&one.notice()),
    ]);
    args
}

/// Shows Joel `one`'s notice, with its id when it was shown.
fn notify(one: &Tell) -> Option<u32> {
    let out = Command::new("notify-send")
        .args(notice_args(one))
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

/// What `program` prints given `args`, or an error when it fails or is
/// still running after `within`, when it's stopped.
fn answer(program: &str, args: &[&str], within: Duration) -> io::Result<String> {
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    let mut out = child.stdout.take().expect("its output was piped");
    // Read beside the wait, so an answer past a pipe's buffer can't stall it.
    let reader = thread::spawn(move || {
        let mut said = String::new();
        out.read_to_string(&mut said).map(|_| said)
    });
    let deadline = Instant::now() + within;
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if Instant::now() >= deadline {
            child.kill()?;
            child.wait()?;
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                format!("{program} {} took over {within:?}", args.join(" ")),
            ));
        }
        thread::sleep(Duration::from_millis(100));
    };
    let said = reader
        .join()
        .map_err(|_| io::Error::other("reading its answer failed"))??;
    if status.success() {
        Ok(said)
    } else {
        Err(io::Error::other(format!(
            "{program} {} failed",
            args.join(" ")
        )))
    }
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
    let ask = |realm: &str, what: &str| answer("mycelium", &["--realm", realm, what], ASK_WITHIN);
    let mut answered = Vec::new();
    let mut read = Vec::new();
    for realm in realms {
        // An answer naming no machines, as an error, says nothing of them.
        let Some(machines) = ask(realm, "machines")
            .ok()
            .filter(|said| crate::elsewhere::machines(said).is_some())
        else {
            continue;
        };
        let sessions = ask(realm, "sessions").unwrap_or_default();
        answered.push(realm.clone());
        read.extend(seen(realm, &machines, &sessions, &this));
    }
    let tells = due(&read, realms, &answered, &mut told, now);
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
        let noticed = match one {
            Tell::Down { machine, .. } => told
                .machines
                .get(machine)
                .is_some_and(|spell| spell.notice.is_some()),
            Tell::Back { notice, .. } => notice.is_some(),
        };
        println!("{}", one.finding(noticed));
    }
    write(&told)?;
    told_all
}

#[cfg(test)]
mod tests;
