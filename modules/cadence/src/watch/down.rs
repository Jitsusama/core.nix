//! Each realm's other machines gone down while they hold work: one its node
//! read ready within the hour before, on mains power with a session on it
//! busy, and that has read unreachable for ten minutes of readings since,
//! is told to Joel by a desktop notice and to the orchestrator and its owner
//! as a finding, once, and again once it's back. On 17 November this machine
//! powered off by itself and no one heard for minutes; the node knows each
//! link's state, so the tick reads it rather than wait on another session
//! to. A machine with no work sleeps as its node lets it, and one with a
//! battery or a Mac may sleep when its lid shuts whatever it holds, so
//! neither is news: a laptop shut on mains and then unplugged sleeps
//! before its node hears the battery, and until its node says it's going
//! to sleep, nothing tells that from a crash. With
//! none ready, this machine may be the one cut off: unless every network
//! its node probes for it reads reached, and no two machines down were
//! last heard within minutes of each other, as a lost link loses every
//! peer at once, nothing is told. Where it can't tell, as when a realm
//! doesn't answer, nothing new is told: a false critical notice misleads
//! Joel, where a late one only waits.

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
/// How long a word on who is busy holds when a tick doesn't say, so a
/// realm whose sessions never answer can't keep a machine busy for good.
const SAID_FOR: i64 = READ_GAP;
/// How long a machine no realm lists any more is remembered.
const FORGET: i64 = 2 * 86_400;
/// How long a node may take to answer: one stalled on a peer it can't reach
/// would otherwise hold the whole tick, and every alarm with it.
const ASK_WITHIN: Duration = Duration::from_secs(30);
/// How close together machines must have last been heard from to have
/// dropped at once: a lost link loses each peer as its own link times out,
/// tens of seconds apart, where machines sleeping or failing on their own
/// rarely land this close.
const TOGETHER: i64 = 3 * 60;

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
    /// Whether a lid shut may sleep it however busy: one with a battery,
    /// which a lid shut on mains and then unplugged sleeps unheard, or a
    /// Mac, whose lid sleeps it on any power; for one down, as its node
    /// last heard.
    pub battery: bool,
    /// When its node last heard from it, in seconds.
    pub heard: Option<i64>,
}

/// What a node names a Mac's system.
const MAC: &str = "macos";

/// The probe every node adds to say whether a call is on here, which says
/// nothing of what this machine reaches.
const CALL: &str = "call";

/// Whether this machine's own record, in what `mycelium machines` answers,
/// says it reaches every network its node probes for it (`Some(true)`) or
/// not (`Some(false)`); `None` when it declares none, or has no record.
#[must_use]
pub fn reach(machines: &str, this: &str) -> Option<bool> {
    crate::elsewhere::machines(machines)
        .unwrap_or_default()
        .iter()
        .filter(|machine| machine["name"] == this)
        .filter_map(|machine| machine["reached"].as_object())
        .find_map(|reached| {
            let mut networks = reached
                .iter()
                .filter(|(probe, _)| *probe != CALL)
                .peekable();
            networks.peek()?;
            Some(networks.all(|(_, one)| one.as_bool() == Some(true)))
        })
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
                // A node gives a charge only for a machine with a battery.
                battery: machine["power"]["on_battery"].as_bool() == Some(true)
                    || machine["power"]["charge"].is_u64()
                    || machine["system"] == MAC,
                heard: machine["measured_at"]
                    .as_str()
                    .and_then(|at| at.parse::<jiff::Timestamp>().ok())
                    .map(jiff::Timestamp::as_second),
            })
        })
        .collect()
}

/// What cadence remembers of each machine between ticks.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Told {
    #[serde(default)]
    pub machines: BTreeMap<String, Spell>,
}

/// One machine's readings.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Spell {
    /// When it last read ready.
    #[serde(default)]
    pub ready_at: Option<i64>,
    /// Whether a session on it was busy when it last read ready, as last
    /// said: a tick the node didn't say keeps what the one before did, for
    /// as long as `SAID_FOR`.
    #[serde(default)]
    pub working: bool,
    /// When a realm last said whether a session on it was busy.
    #[serde(default)]
    pub said_at: Option<i64>,
    /// Whether a lid shut sleeps it, as `Seen::battery` says, when it last
    /// read ready, or as its node last heard while it reads down.
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
        // Read down by two realms, it was last heard when either last
        // heard it, with a battery if either heard one.
        let both_down = kept.reading == Reading::Down && one.reading == Reading::Down;
        let (heard, battery) = (kept.heard.max(one.heard), kept.battery || one.battery);
        // A realm that didn't say keeps the word unsaid, unless another
        // saw a session busy: its sessions may be the ones it didn't say.
        let working = match (kept.working, one.working) {
            (Some(true), _) | (_, Some(true)) => Some(true),
            (Some(false), Some(false)) => Some(false),
            _ => None,
        };
        if rank(one.reading) > rank(kept.reading) {
            *kept = one.clone();
        }
        kept.working = working;
        if both_down {
            kept.heard = heard;
            kept.battery = battery;
        }
    }
    by
}

/// Whether this machine reads as cut off, or can't tell: none ready, and
/// either it isn't `sure` it reaches what its node probes, or a machine
/// `recent`ly ready is down with no time heard, or two are down heard
/// within `TOGETHER` of each other within the hour. It reads only what the
/// node heard, so a tick that read nothing, or peers lost either side of a
/// tick, change nothing, and one clock running fast can't part the rest.
/// A time heard ahead of now counts as now. One gone past the hour can't
/// be told, so says nothing of this drop.
fn cut_off(
    by: &BTreeMap<&str, (Seen, BTreeSet<String>)>,
    now: i64,
    sure: bool,
    recent: &BTreeSet<&str>,
) -> bool {
    if by.values().any(|(one, _)| one.reading == Reading::Ready) {
        return false;
    }
    if !sure {
        return true;
    }
    let downs: Vec<Option<i64>> = by
        .iter()
        .filter(|(machine, (one, _))| {
            one.reading == Reading::Down && (one.heard.is_some() || recent.contains(*machine))
        })
        .map(|(_, (one, _))| one.heard.map(|heard| heard.min(now)))
        .collect();
    if downs.contains(&None) {
        return true;
    }
    let mut heard: Vec<i64> = downs
        .into_iter()
        .flatten()
        .filter(|heard| now - heard <= READY_WITHIN)
        .collect();
    heard.sort_unstable();
    heard.windows(2).any(|pair| pair[1] - pair[0] <= TOGETHER)
}

/// What's newly down or back, given what each of the `answered` realms of
/// those `asked` reads now, keeping each machine's spell in `told`, and
/// whether this machine is `sure` it reaches what its node probes.
pub fn due(
    seen: &[Seen],
    asked: &[String],
    answered: &[String],
    told: &mut Told,
    now: i64,
    sure: bool,
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
    let recent: BTreeSet<&str> = by
        .keys()
        .filter(|machine| {
            told.machines
                .get(**machine)
                .and_then(|spell| spell.ready_at)
                .is_some_and(|at| now - at <= READY_WITHIN)
        })
        .copied()
        .collect();
    let cut_off = cut_off(&by, now, sure, &recent);
    // A realm not answering may hold the machines that show this one cut
    // off, so nothing new is told until each answers.
    let all_answered = asked.iter().all(|realm| answered.contains(realm));
    let mut tells = Vec::new();
    for (machine, (one, _)) in by {
        let spell = told
            .machines
            .get_mut(machine)
            .expect("each machine was kept above");
        match one.reading {
            Reading::Unread => {}
            Reading::Down if cut_off => {
                // What this machine read while cut off says nothing of
                // it, so a count not yet told starts afresh after.
                if spell.notice.is_none() && spell.sent.is_empty() {
                    spell.down_since = None;
                }
            }
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
                    working: one.working.unwrap_or(
                        spell.working && spell.said_at.is_some_and(|at| now - at <= SAID_FOR),
                    ),
                    said_at: if one.working.is_some() {
                        Some(now)
                    } else {
                        spell.said_at
                    },
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
                // A battery heard in any read of the spell stands, as one
                // realm's node may hear it when another's doesn't.
                spell.battery |= one.battery;
                let since = spell.down_since.unwrap_or(now);
                let lately = spell
                    .ready_at
                    .is_some_and(|ready| since - ready <= READY_WITHIN);
                let holds = spell.working && !spell.battery;
                if !spell.told && all_answered && holds && lately && now - since >= DOWN_FOR {
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
/// by, and a coming back replacing its going down. An id kept for hours
/// stays its notice's across a restart of the shell, since the notices
/// daemon keeps its next id with its store and replaces only a notice
/// still open, giving a new id otherwise; only a store it can't read,
/// which it sets aside, numbers from 1 again.
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

/// What each of `realms`' nodes says of its machines, read once a tick for
/// every use of it, each within `ASK_WITHIN`, so a node that never answers
/// can't stall the tick; a realm whose node didn't answer, or answered
/// naming no machines, as with an error, is left out.
pub fn machines(realms: &[String]) -> BTreeMap<String, String> {
    realms
        .iter()
        .filter_map(|realm| {
            let said = answer("mycelium", &["--realm", realm, "machines"], ASK_WITHIN).ok()?;
            crate::elsewhere::machines(&said)?;
            Some((realm.clone(), said))
        })
        .collect()
}

/// One tick's watch: reads each of `realms`, by its node's answer in
/// `machines`, tells what's newly down or back, and keeps each machine's
/// spell in `down.json` in `folder`. The spells are written as told before
/// anything is, so a write that fails tells nothing rather than the same
/// each tick.
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
    machines: &BTreeMap<String, String>,
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
    let mut here = None;
    for realm in realms {
        let Some(machines) = machines.get(realm) else {
            continue;
        };
        let sessions = ask(realm, "sessions").unwrap_or_default();
        answered.push(realm.clone());
        // A realm whose record says this machine misses a network outranks
        // one saying it reaches all.
        here = match (here, reach(machines, &this)) {
            (Some(false), _) | (_, Some(false)) => Some(false),
            (_, Some(true)) => Some(true),
            (was, None) => was,
        };
        read.extend(seen(realm, machines, &sessions, &this));
    }
    let sure = here == Some(true);
    if !sure {
        println!(
            "{this} doesn't read every network its node probes as reached, so with none \
             ready no machine is told down"
        );
    }
    for realm in realms.iter().filter(|realm| !answered.contains(realm)) {
        println!("The {realm} realm's node didn't answer, so no machine is told down");
    }
    let tells = due(&read, realms, &answered, &mut told, now, sure);
    promise_then_tell(
        &path,
        &tells,
        told,
        |key: &str| crate::findings::owner_of(key, owners),
        orchestrator,
        notify,
        |to, text| mesh.send(&crate::message(to, "finding", text)),
    )
}

/// Tells each of `tells` as `tell` does, with the spells in `told` written
/// to `path` as though each were told before any is, so a write that fails
/// tells nothing rather than the same each tick, and as told once each is.
/// Each is printed for the journal, saying Joel has a notice only where
/// one was shown.
fn promise_then_tell(
    path: &Path,
    tells: &[Tell],
    mut told: Told,
    owner_of: impl Fn(&str) -> Option<String>,
    orchestrator: &str,
    notify: impl Fn(&Tell) -> Option<u32>,
    send: impl Fn(&str, String) -> io::Result<()>,
) -> io::Result<()> {
    let write = |told: &Told| crate::write_whole(path, &serde_json::to_string_pretty(told)?);
    let mut promised = told.clone();
    tell::<io::Error>(
        tells,
        &mut promised,
        orchestrator,
        &owner_of,
        |_| Some(0),
        |_, _| Ok(()),
    )?;
    write(&promised)?;
    let told_all = tell(tells, &mut told, orchestrator, &owner_of, notify, send);
    for one in tells {
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
