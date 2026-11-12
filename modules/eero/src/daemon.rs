//! `eero daemon`: the home network kept current for the shell and the CLI.
//!
//! It keeps `$XDG_RUNTIME_DIR/eero/home-network.json`, written whole and
//! only when it changes, for the CLI, and tells its socket's listeners
//! each change (listeners.rs), the shell among them. It reads each part
//! of the home network as often as its figures move (schedule.rs), and
//! sleeps between, woken early only by a listener. When eero can't be
//! reached the last model stays, its cloud saying so, since the internet
//! being down is what the shell most needs to show. After each read it
//! records what moves in the history (history/), in eero's folder in the
//! account's state. With each read
//! of the network, and every 10 seconds while eero can't be reached, it
//! takes its own look at the internet (probe.rs), so an outage is seen and
//! timed even when eero can't say; then it asks eero again as often, so a
//! machine that starts before its network is up has the model at once.
//! And it tells Joel what changed on the home network while it watched, as
//! desktop notices (notices.rs, desktop.rs): the internet out and back, a
//! node offline and back, a device new to the home network, firmware, and
//! eero's own events. When eero's session ages out, it asks Joel to sign
//! in again in For You (waiting.rs).

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use jiff::tz::TimeZone;
use serde_json::Value;

use crate::api;
use crate::cloud::{self, Failure};
use crate::day;
use crate::desktop::Notifier;
use crate::history::History;
use crate::listeners::{Call, Listeners};
use crate::model::{Cloud, CloudState, HomeNetwork};
use crate::notices::{self, Watch};
use crate::probe::{self, Targets};
use crate::read::{self, Answers};
use crate::schedule::{Looks, Read, Schedule};
use crate::session;
use crate::time;
use crate::waiting;

/// Keeps the home network until it's stopped.
pub fn run() -> Result<(), String> {
    let folder = runtime()?.join("eero");
    fs::create_dir_all(&folder).map_err(|e| format!("{}: {e}", folder.display()))?;
    let file = folder.join("home-network.json");
    let (wake, woken) = mpsc::channel();
    let listeners = Listeners::listen(&folder.join("socket"), wake).map_err(|e| {
        format!(
            "couldn't listen at {}: {e}",
            folder.join("socket").display()
        )
    })?;
    let mut daemon = Daemon::new(earlier(&file), history());
    if let Some(model) = &daemon.model {
        listeners.send(&value(model));
    }
    loop {
        let listening = listeners.listening();
        if let Some(model) = daemon.cycle(Instant::now(), listening) {
            write(&file, &line(&model)).map_err(|e| format!("{}: {e}", file.display()))?;
            listeners.send(&value(&model));
        }
        match woken.recv_timeout(daemon.wait(Instant::now(), listeners.listening())) {
            Ok(Call::Read) => daemon.schedule.ask(),
            Ok(Call::Joined | Call::Left | Call::Looked) | Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return Err("the socket stopped".into()),
        }
    }
}

/// What eero said of a cycle's reads, as the model's cloud tells it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Heard {
    Answered,
    Unreachable(String),
    SignedOut(String),
}

/// The cloud after hearing eero at `at`: its state, and since when it's
/// been in it, which a state held carries over.
pub fn after(cloud: Option<&Cloud>, heard: &Heard, at: &str) -> Cloud {
    let (state, reason) = match heard {
        Heard::Answered => (CloudState::Answering, None),
        Heard::Unreachable(reason) => (CloudState::Unreachable, Some(reason.clone())),
        Heard::SignedOut(reason) => (CloudState::SignedOut, Some(reason.clone())),
    };
    let since = match cloud {
        Some(cloud) if cloud.state == state => cloud.since.clone(),
        _ => at.to_owned(),
    };
    Cloud {
        state,
        since,
        reason,
    }
}

/// The model after hearing eero at `at`: drawn afresh from the answers
/// when eero answered, else the last one with its cloud saying why it's
/// no newer. There's none until eero first answers.
pub fn next_model(
    last: Option<&HomeNetwork>,
    answers: &Answers,
    heard: &Heard,
    at: &str,
) -> Option<HomeNetwork> {
    let cloud = after(last.map(|model| &model.cloud), heard, at);
    let drawn = match heard {
        Heard::Answered => read::home_network(answers, at, cloud.clone()),
        Heard::Unreachable(_) | Heard::SignedOut(_) => None,
    };
    drawn.or_else(|| {
        last.map(|model| HomeNetwork {
            cloud,
            ..model.clone()
        })
    })
}

/// The history in eero's state folder, or one in memory when that can't
/// be opened, so a full disk costs the history and not the live figures.
fn history() -> History {
    let opened = session::folder()
        .map_err(|e| e.to_string())
        .and_then(|folder| {
            fs::create_dir_all(&folder).map_err(|e| format!("{}: {e}", folder.display()))?;
            History::open(&folder.join("history.sqlite"))
        });
    opened.unwrap_or_else(|reason| {
        eprintln!("eero: keeping the history in memory alone: {reason}");
        History::in_memory()
    })
}

/// What the daemon holds between cycles.
struct Daemon {
    cloud: Option<cloud::Cloud>,
    /// The home network's path, such as `/2.2/networks/1001`: the
    /// account's first network.
    network: Option<String>,
    answers: Answers,
    model: Option<HomeNetwork>,
    schedule: Schedule,
    history: History,
    looks: Looks,
    /// The model the notices were last worked out against.
    watch: Watch,
    notifier: Notifier,
    /// Where the daemon asks Joel to sign in again (waiting.rs).
    waiting: Option<PathBuf>,
    /// Where each day's look at the home network is written (day.rs).
    days: Option<PathBuf>,
}

impl Daemon {
    fn new(model: Option<HomeNetwork>, history: History) -> Daemon {
        Daemon {
            cloud: None,
            network: None,
            answers: Answers::default(),
            model,
            schedule: Schedule::default(),
            history,
            looks: Looks::default(),
            watch: Watch::default(),
            notifier: Notifier::default(),
            waiting: waiting::file()
                .inspect_err(|reason| eprintln!("eero: For You can't be asked: {reason}"))
                .ok(),
            days: session::folder()
                .inspect_err(|reason| eprintln!("eero: no day is written: {reason}"))
                .ok()
                .map(|folder| folder.join("day")),
        }
    }

    /// Raises the notices a model calls for, and records each change they
    /// tell, at `at`. A notice the desktop can't take is said to the
    /// journal and still recorded, since the change it told is past.
    fn tell(&mut self, next: &notices::Next, at: i64) {
        for told in &next.settled {
            if let Err(reason) = self.history.record_told(told, at) {
                eprintln!("eero: {reason}");
            }
        }
        for notice in &next.notices {
            if let Err(reason) = self.notifier.tell(notice) {
                eprintln!("eero: couldn't raise \"{}\": {reason}", notice.summary);
            }
            if let Err(reason) = self.history.record_notice(notice, at) {
                eprintln!("eero: {reason}");
            }
        }
    }

    /// Asks Joel to sign in again while eero refuses the session, and stops
    /// asking once eero answers. Unreachable, eero can't say which, so the
    /// ask stays as it was.
    fn ask_to_sign_in(&self, heard: Option<&Heard>, at: i64) {
        let asks = match heard {
            Some(Heard::SignedOut(_)) => true,
            Some(Heard::Answered) => false,
            Some(Heard::Unreachable(_)) | None => return,
        };
        if let Some(file) = &self.waiting
            && let Err(reason) = waiting::ask(file, asks, at)
        {
            eprintln!("eero: couldn't change {}: {reason}", file.display());
        }
    }

    /// How long to sleep from `now` until the next read or look is due.
    fn wait(&self, now: Instant, listening: bool) -> Duration {
        let read = self.schedule.wait(now, listening);
        self.looks
            .wait(now, self.unreachable())
            .map_or(read, |look| look.min(read))
    }

    fn unreachable(&self) -> bool {
        self.model
            .as_ref()
            .is_some_and(|model| model.cloud.state == CloudState::Unreachable)
    }

    /// Takes every read due, and a look if one is due, and gives the model
    /// when it changed.
    fn cycle(&mut self, now: Instant, listening: bool) -> Option<HomeNetwork> {
        let due = self.schedule.due(now, listening);
        let look = self
            .looks
            .due(now, due.contains(&Read::Network), self.unreachable());
        if due.is_empty() && !look {
            return None;
        }
        let was_answering = self
            .model
            .as_ref()
            .is_some_and(|model| model.cloud.state == CloudState::Answering);
        let mut heard = None;
        for read in &due {
            self.schedule.done(*read, now);
            if heard.as_ref().is_some_and(|h| *h != Heard::Answered) {
                continue;
            }
            heard = Some(match self.read(*read) {
                Ok(()) => {
                    self.schedule.reached(true);
                    Heard::Answered
                }
                Err(failure) => self.failed(failure),
            });
            if *read == Read::Network && heard == Some(Heard::Answered) && !was_answering {
                self.schedule.catch_up();
            }
        }
        let at = now_unix();
        self.ask_to_sign_in(heard.as_ref(), at);
        let mut next = match &heard {
            Some(heard) => next_model(
                self.model.as_ref(),
                &self.answers,
                heard,
                &time::from_unix(at),
            )?,
            None => self.model.clone()?,
        };
        read::mark_this_machine(
            &mut next,
            &probe::own_macs(),
            probe::own_hostname().as_deref(),
        );
        if heard == Some(Heard::Answered) {
            let network_read = due.contains(&Read::Network);
            if let Err(reason) = self.history.record(&next, at, network_read) {
                eprintln!("eero: {reason}");
            }
        }
        // A look takes up to its timeout, so what it found is timed as it
        // ends, and the history drawn as of then.
        if look {
            self.looks.done(now);
            let reading = look_from(&next);
            if let Err(reason) = self.history.record_probe(&reading, now_unix()) {
                eprintln!("eero: {reason}");
            }
            if let Some(days) = &self.days
                && let Err(reason) =
                    day::write(&self.history, now_unix(), &TimeZone::system(), days)
            {
                eprintln!("eero: the day: {reason}");
            }
        }
        if let Err(reason) = self.history.draw(&mut next, now_unix()) {
            eprintln!("eero: {reason}");
        }
        let told = self
            .watch
            .next(&next, heard == Some(Heard::Answered), &self.history);
        self.tell(&told, now_unix());
        if !told.notices.is_empty()
            && let Err(reason) = self.history.draw_told(&mut next, now_unix())
        {
            eprintln!("eero: {reason}");
        }
        if self.model.as_ref() == Some(&next) {
            return None;
        }
        self.model = Some(next.clone());
        Some(next)
    }

    /// What a failure says of the cloud, said to the journal too. A
    /// signed-out session is let go, so the next read opens the sealed one
    /// afresh, which `eero sign-in` may have replaced.
    fn failed(&mut self, failure: Failure) -> Heard {
        eprintln!("eero: {failure}");
        self.schedule
            .reached(!matches!(failure, Failure::Unreachable(_)));
        match failure {
            Failure::SignedOut(reason) => {
                self.cloud = None;
                Heard::SignedOut(reason)
            }
            Failure::Unreachable(reason) | Failure::Refused(reason) => Heard::Unreachable(reason),
        }
    }

    fn read(&mut self, read: Read) -> Result<(), Failure> {
        let opened = match self.cloud.take() {
            Some(cloud) => cloud,
            None => cloud::Cloud::signed_in()?,
        };
        let cloud = self.cloud.insert(opened);
        if self.network.is_none() {
            self.network = api::networks(&cloud.get(api::ACCOUNT)?).into_iter().next();
        }
        let network = self
            .network
            .clone()
            .ok_or_else(|| Failure::Refused("eero names no network on the account".into()))?;
        let answers = &mut self.answers;
        let nodes = read::node_ids(&answers.network);
        let profiles = read::profile_ids(&answers.profiles);
        match read {
            Read::Network => {
                answers.network = cloud.get(&network)?;
                answers.devices = cloud.get(&format!("{network}/devices"))?;
            }
            Read::Connections => {
                answers.connections = each_node(cloud, &nodes, &answers.connections, |id| {
                    format!("/2.2/eeros/{id}/connections")
                })?;
            }
            Read::Usage => {
                let window = usage_window(now_unix());
                kept(
                    cloud.get(&format!("{network}/data_usage?{window}")),
                    &mut answers.usage,
                )?;
                answers.node_usage = each_node(cloud, &nodes, &answers.node_usage, |id| {
                    format!("{network}/data_usage/eeros/{id}?{window}")
                })?;
                answers.profile_usage =
                    each_node(cloud, &profiles, &answers.profile_usage, |id| {
                        format!("{network}/data_usage/profiles/{id}?{window}")
                    })?;
            }
            Read::Rest => {
                for (path, answer) in [
                    ("speedtest", &mut answers.speed_tests),
                    ("thread", &mut answers.thread),
                    ("cellular_backup_events", &mut answers.outages),
                    ("profiles", &mut answers.profiles),
                    ("app_events", &mut answers.events),
                    ("subnets_config", &mut answers.subnets),
                    ("routing", &mut answers.routing),
                    ("wpa3_per_band", &mut answers.security),
                    ("ac_compat", &mut answers.ac_compatibility),
                    ("network_scan", &mut answers.scan),
                ] {
                    kept(cloud.get(&format!("{network}/{path}")), answer)?;
                }
            }
        }
        Ok(())
    }
}

/// A look at the internet from this machine, at the gateway node of the
/// home network `model` is when this machine is on it.
fn look_from(model: &HomeNetwork) -> probe::Reading {
    let home = probe::gateway(model)
        .filter(|gateway| probe::at_home(model, &probe::own_macs(), &probe::routes(), *gateway));
    probe::look(&Targets::usual(home), probe::TIMEOUT)
}

/// Each node's answer at the path `path` names for it, keeping a node's
/// last answer where eero refuses this one. A profile's answer is asked
/// for the same way.
fn each_node(
    cloud: &mut cloud::Cloud,
    nodes: &[String],
    last: &BTreeMap<String, Value>,
    path: impl Fn(&str) -> String,
) -> Result<BTreeMap<String, Value>, Failure> {
    let mut answers = BTreeMap::new();
    for id in nodes {
        let mut answer = last.get(id).cloned().unwrap_or_default();
        kept(cloud.get(&path(id)), &mut answer)?;
        answers.insert(id.clone(), answer);
    }
    Ok(answers)
}

/// Keeps an answer, or the last one where eero refuses the read: a read
/// this network's tier or firmware doesn't give costs its figures, not
/// the cycle.
fn kept(answer: Result<Value, Failure>, into: &mut Value) -> Result<(), Failure> {
    match answer {
        Ok(answer) => *into = answer,
        Err(Failure::Refused(reason)) => eprintln!("eero: {reason}"),
        Err(failure) => return Err(failure),
    }
    Ok(())
}

/// The query for the last day's usage, by hour, ending at `now`.
fn usage_window(now: i64) -> String {
    let day = 24 * 60 * 60;
    format!(
        "start={}&end={}&cadence=hourly",
        time::from_unix(now - day),
        time::from_unix(now)
    )
}

/// The model a daemon before this one kept, so a restart has something to
/// show before eero first answers.
fn earlier(file: &Path) -> Option<HomeNetwork> {
    serde_json::from_str(&fs::read_to_string(file).ok()?).ok()
}

fn line(model: &HomeNetwork) -> String {
    serde_json::to_string(model).expect("the model is plain data")
}

fn value(model: &HomeNetwork) -> Value {
    serde_json::to_value(model).expect("the model is plain data")
}

fn runtime() -> Result<PathBuf, String> {
    std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .ok_or_else(|| "XDG_RUNTIME_DIR isn't set".into())
}

/// Writes a file whole, so the shell never reads half of one.
fn write(file: &Path, contents: &str) -> io::Result<()> {
    let fresh = file.with_extension("json.new");
    fs::write(&fresh, contents)?;
    fs::rename(fresh, file)
}

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs().try_into().unwrap_or(i64::MAX))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn contract(file: &str) -> Value {
        let path = format!("{}/contract/{file}", env!("CARGO_MANIFEST_DIR"));
        let answer: Value = serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
        answer["data"].clone()
    }

    fn answers() -> Answers {
        Answers {
            network: contract("network.json"),
            devices: contract("net.devices.json"),
            ..Answers::default()
        }
    }

    const FIRST: &str = "2026-11-07T22:54:43Z";
    const LATER: &str = "2026-11-07T23:00:00Z";
    const LATER_STILL: &str = "2026-11-07T23:05:00Z";

    fn unreachable() -> Heard {
        Heard::Unreachable("couldn't reach eero: timed out".into())
    }

    #[test]
    fn there_is_no_model_until_eero_first_answers() {
        assert_eq!(next_model(None, &answers(), &unreachable(), FIRST), None);
        let model = next_model(None, &answers(), &Heard::Answered, FIRST).unwrap();
        assert_eq!(model.read, FIRST);
        assert_eq!(model.cloud.state, CloudState::Answering);
        assert_eq!(model.cloud.since, FIRST);
    }

    #[test]
    fn answering_on_keeps_since_and_moves_read() {
        let first = next_model(None, &answers(), &Heard::Answered, FIRST).unwrap();

        let later = next_model(Some(&first), &answers(), &Heard::Answered, LATER).unwrap();

        assert_eq!(later.read, LATER);
        assert_eq!(later.cloud.since, FIRST);
    }

    #[test]
    fn unreachable_keeps_the_last_figures_and_says_since_when_and_why() {
        let first = next_model(None, &answers(), &Heard::Answered, FIRST).unwrap();

        let out = next_model(Some(&first), &Answers::default(), &unreachable(), LATER).unwrap();
        let still = next_model(Some(&out), &Answers::default(), &unreachable(), LATER_STILL);

        assert_eq!(out.read, FIRST, "read is when eero last answered");
        assert_eq!(out.nodes, first.nodes);
        assert_eq!(out.cloud.state, CloudState::Unreachable);
        assert_eq!(out.cloud.since, LATER);
        assert_eq!(
            out.cloud.reason.as_deref(),
            Some("couldn't reach eero: timed out")
        );
        assert_eq!(
            still.as_ref(),
            Some(&out),
            "nothing changed, so nothing is written"
        );
    }

    #[test]
    fn answering_again_after_an_outage_starts_a_new_since() {
        let first = next_model(None, &answers(), &Heard::Answered, FIRST).unwrap();
        let out = next_model(Some(&first), &answers(), &unreachable(), LATER).unwrap();

        let back = next_model(Some(&out), &answers(), &Heard::Answered, LATER_STILL).unwrap();

        assert_eq!(back.cloud.state, CloudState::Answering);
        assert_eq!(back.cloud.since, LATER_STILL);
        assert_eq!(back.cloud.reason, None);
    }

    #[test]
    fn signed_out_keeps_the_last_figures_and_says_how_to_sign_in() {
        let first = next_model(None, &answers(), &Heard::Answered, FIRST).unwrap();
        let reason = "eero's session aged and couldn't be refreshed: run `eero sign-in`";

        let out = next_model(
            Some(&first),
            &answers(),
            &Heard::SignedOut(reason.into()),
            LATER,
        );

        let out = out.unwrap();
        assert_eq!(out.read, FIRST);
        assert_eq!(out.cloud.state, CloudState::SignedOut);
        assert_eq!(out.cloud.since, LATER);
        assert_eq!(out.cloud.reason.as_deref(), Some(reason));
    }

    #[test]
    fn the_usage_window_is_the_last_day_by_hour() {
        assert_eq!(
            usage_window(1_794_091_483),
            "start=2026-11-06T22:44:43Z&end=2026-11-07T22:44:43Z&cadence=hourly"
        );
    }
}
