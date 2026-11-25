//! `pineapple serve`: the safe word's daemon for the launcher. It holds
//! the rows src/tell.rs makes of what's kept under the state folder, and
//! serves them on its socket in the shell's daemon interface
//! (docs/daemons.md), for Needs You; it takes Joel's presses back as
//! asks (src/acts.rs), only from his shell (src/peer.rs); and as a row
//! first appears, it sends a notice that knocks even in a call, and
//! closes it once the row stops standing, resumed or left, so Joel's
//! alarm doesn't outlive what it was about.
//!
//! It takes Joel's Pause All Agents the same way, as the ask `pause`, and
//! while that stop stands it freezes agents' work started since, each
//! time it looks; once the stop is resumed, however, it undoes what the
//! stop paused (src/pause.rs).
//!
//! On a thread of its own, it gives each other machine running agents
//! every live stop the readers keep here, its resume, and its end once
//! it is kept here no longer (src/pause/carry.rs), so a stop said in Slack or on GitHub holds the
//! sessions there too; a machine slow to answer never holds up a row.
//!
//! While the safe word is in shadow, every stop and alert is too, and
//! the daemon serves none of them and sends no notice: it writes each row
//! it would have shown, once, to would-tell.jsonl, with the message it
//! would send to Joel's own Slack, which nothing sends yet either.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::sync::mpsc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use daemon_serve::asks::Domain;
use daemon_serve::serve::{Shared, accept, listen};
use serde_json::{Value, json};

use crate::acts::{self, Refused};
use crate::pause;
use crate::peer::{Peer, may_act};
use crate::phone::{self, Alert};
use crate::record;
use crate::tell::{self, KIND};
use crate::vocabulary::Words;

/// The daemon as its clients know it.
const DOMAIN: Domain = Domain {
    name: "pineapple",
    kinds: &[KIND],
    sole: &[],
    named_only: &[],
};

/// How often what's kept is looked at for a change, unless a test says
/// otherwise: a fixed wait, since nothing here is told of a write, and
/// two seconds puts a stop in front of Joel well inside the five minutes
/// a stop has to land in, for a look at four files' sizes and times.
pub const EVERY: Duration = Duration::from_secs(2);

/// How often the rows are made again however nothing moved, so an alert
/// a week old goes.
const REMADE: Duration = Duration::from_secs(60);

/// What the daemon is started with.
#[derive(Debug)]
pub struct Serving {
    /// The socket it listens on, unless systemd hands it one.
    pub socket: PathBuf,
    /// The state folder: stops/, decisions.jsonl, acted.jsonl and what
    /// it writes itself.
    pub state: PathBuf,
    /// The launcher's vocabulary, the rows' words.
    pub words: Words,
    /// notify-send, to knock with; none sends no notice.
    pub notify: Option<PathBuf>,
    /// busctl, to close a notice with once its row stops standing; none
    /// leaves it for Joel to dismiss.
    pub busctl: Option<PathBuf>,
    /// How often it looks for a change.
    pub every: Duration,
    /// What a stop on everything Joel asks for here pauses; none, and his
    /// Pause All Agents is refused.
    pub pause: Option<pause::Plan>,
    /// This machine's name, as Joel is told of it.
    pub host: String,
}

/// Every ask a press makes, Pause All Agents's beside the rows'.
const ASKS: [&str; 6] = ["resume", "widen", "stop", "approve", "dismiss", PAUSE];

/// The ask Pause All Agents makes.
const PAUSE: &str = "pause";

/// How long each step of a pause is given, so it answers the launcher in
/// time however a machine named fails to.
const STEP: Duration = Duration::from_secs(6);

/// How often the other machines are given the stops they lack: a stop
/// heard reaches them within this and a run, and a run more for each
/// machine that hangs, which then waits a minute, well inside the five
/// minutes a stop has to land in.
const CARRY_EVERY: Duration = Duration::from_secs(5);

/// How long each machine is given to take a stop, longer than a pause's
/// step since nobody waits on its answer.
const CARRY_STEP: Duration = Duration::from_secs(30);

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| {
            u64::try_from(since.as_millis()).unwrap_or(u64::MAX)
        })
}

/// Serves until it's stopped.
///
/// # Errors
///
/// Why the socket couldn't be listened on.
pub fn serve(serving: Serving) -> std::io::Result<()> {
    let Serving {
        socket,
        state,
        words,
        notify,
        busctl,
        every,
        pause,
        host,
    } = serving;
    let words = Arc::new(words);
    let pause = Arc::new(pause);
    let shared = Shared::new(DOMAIN, Box::new(|_| None));
    let (poke, poked) = mpsc::channel::<()>();
    {
        let state = state.clone();
        let words = Arc::clone(&words);
        let pause = Arc::clone(&pause);
        let host = host.clone();
        shared.slow(&[PAUSE]);
        shared.peered(
            &ASKS,
            Box::new(move |said, stream| {
                let answer = if said["ask"] == PAUSE {
                    paused(&state, said, stream, pause.as_ref().as_ref(), &host)
                } else {
                    pressed(&state, said, stream, &words)
                };
                // Looked at again at once, so the row a press changed is
                // told without waiting for the next look.
                let _ = poke.send(());
                Some(answer)
            }),
        );
    }
    shared.change(Vec::new(), json!({"stops": "answering"}));
    if let Some(plan) = pause
        .as_ref()
        .clone()
        .filter(|plan| plan.mycelium.is_some())
    {
        let (state, host) = (state.clone(), host.clone());
        std::thread::spawn(move || carry_each(&plan, &state, &host));
    }
    let listener = listen(&socket)?;
    accept(Arc::clone(&shared), listener);
    let mut told = read_told(&state);
    let mut looked: Option<Vec<(String, u64, u128)>> = None;
    let mut made = Instant::now();
    loop {
        if let Some(plan) = pause.as_ref() {
            hold_paused(plan, &state, &host);
        }
        let mark = mark(&state);
        if looked.as_ref() != Some(&mark) || made.elapsed() >= REMADE {
            let rows = tell::rows(&tell::kept(&state), now_ms(), &words);
            let (served, shadow): (Vec<Value>, Vec<Value>) =
                rows.into_iter().partition(|row| row["shadow"] == false);
            let knocking = Knocking {
                notify: notify.as_deref(),
                busctl: busctl.as_deref(),
            };
            tell_new(&state, &served, &shadow, &knocking, &mut told);
            shared.change(served, json!({"stops": "answering"}));
            looked = Some(mark);
            made = Instant::now();
        }
        // Woken early by a press; a closed channel can't happen while the
        // daemon serves, and would only mean looking at the usual pace.
        let _ = poked.recv_timeout(every);
    }
}

/// While a stop on everything stands, freezes agents' work started since;
/// once it's resumed, undoes what it paused. Each is journalled.
fn hold_paused(plan: &pause::Plan, state: &Path, host: &str) {
    let mut runs = pause::System { limit: STEP };
    match pause::tend(plan, state, host, now_ms(), &mut runs) {
        Ok(tended) => {
            if !tended.froze.is_empty() {
                eprintln!(
                    "paused agents' work started since: {}",
                    tended.froze.join(", ")
                );
            }
            if !tended.took.is_empty() {
                eprintln!("{}", pause::said("resuming", &tended.took, None));
            }
        }
        Err(error) => eprintln!("<4>couldn't tend what a stop paused: {error}"),
    }
}

/// Gives the other machines the stops they lack, as long as the daemon
/// serves, journalling each it gave or couldn't, and why the machines
/// couldn't all be listed each time that changes, not each time it looks.
fn carry_each(plan: &pause::Plan, state: &Path, host: &str) {
    let mut carrier = pause::carry::Carrier::default();
    let mut unlisted = Vec::new();
    loop {
        let mut runs = pause::System { limit: CARRY_STEP };
        let (gives, listing) = carrier.carry(plan, state, host, now_ms(), &mut runs);
        if listing != unlisted {
            for why in &listing {
                eprintln!("<4>couldn't list every machine to give a stop: {why}");
            }
            unlisted = listing;
        }
        for one in gives {
            let what = one.gift.said();
            match one.failed {
                None => eprintln!("gave {} {what} {}", one.machine, one.name),
                Some(why) if one.machine.is_empty() => {
                    eprintln!("<4>gave {what} {}, but {why}", one.name);
                }
                Some(why) => eprintln!(
                    "<4>couldn't give {} {what} {}, tried again in a minute: {why}",
                    one.machine, one.name
                ),
            }
        }
        std::thread::sleep(CARRY_EVERY);
    }
}

/// Joel's Pause All Agents, refused unless it comes from his own shell:
/// `{"ask": "pause", "scope": "all"}`, answered `{"said": <line>}`.
fn paused(
    state: &Path,
    said: &Value,
    stream: &std::os::unix::net::UnixStream,
    plan: Option<&pause::Plan>,
    host: &str,
) -> Value {
    if let Err(refused) = from_joel(PAUSE, stream) {
        return refused.answer();
    }
    let refuse = |code: &'static str, message: String| {
        eprintln!("<4>refused pause: {message}");
        Refused { code, message }.answer()
    };
    if said["scope"] != "all" {
        return refuse("bad-scope", "Pause All Agents pauses all".to_owned());
    }
    let Some(plan) = plan else {
        return refuse("no-plan", "this machine names nothing to pause".to_owned());
    };
    if let Some(id) = pause::stands(state) {
        return refuse(
            "paused-already",
            format!("agents are already stopped, by {id}"),
        );
    }
    let at = now_ms();
    let mut runs = pause::System { limit: STEP };
    match pause::stop(plan, state, &pause::id(at), host, at, &mut runs) {
        Ok(took) => {
            let sessions = pause::sessions(plan, &pause::recorded(&took), &mut runs);
            let line = pause::said("stopping", &took, sessions);
            eprintln!("Joel paused all agents: {line}");
            json!({"said": line})
        }
        Err(error) => refuse("failed", format!("couldn't write the stop: {error}")),
    }
}

/// Whether the process on `stream` is Joel's own shell, or the refusal
/// saying why not.
fn from_joel(verb: &str, stream: &std::os::unix::net::UnixStream) -> Result<(), Refused> {
    let peer = match Peer::of(stream) {
        Ok(peer) => peer,
        Err(error) => {
            eprintln!("<4>refused {verb} from a process it couldn't name: {error}");
            return Err(Refused {
                code: "no-peer",
                message: format!("the process asking can't be named: {error}"),
            });
        }
    };
    if !may_act(&peer.cgroup, &peer.exe) {
        eprintln!(
            "<4>refused {verb} from pid {} ({}), which isn't Joel's shell",
            peer.pid, peer.exe
        );
        return Err(Refused {
            code: "not-joel",
            message: "only Joel's own launcher may act on a stop".to_owned(),
        });
    }
    Ok(())
}

/// A press, refused unless it comes from Joel's own shell.
fn pressed(
    state: &Path,
    said: &Value,
    stream: &std::os::unix::net::UnixStream,
    words: &Words,
) -> Value {
    let verb = said["ask"].as_str().unwrap_or_default();
    if let Err(refused) = from_joel(verb, stream) {
        return refused.answer();
    }
    match acts::press(state, said, now_ms(), words) {
        Ok(done) => {
            eprintln!("Joel pressed {verb} on {}", said["key"]);
            done
        }
        Err(refused) => {
            eprintln!("<4>refused {verb}: {}", refused.message);
            refused.answer()
        }
    }
}

/// What's kept, by each file's name, size and time, so a change to any
/// of them is seen: what each other machine was given of a stop too, so
/// a row names one that couldn't take it as soon as it's known.
fn mark(state: &Path) -> Vec<(String, u64, u128)> {
    let one = |path: &Path| {
        let meta = fs::metadata(path).ok();
        let size = meta.as_ref().map_or(0, fs::Metadata::len);
        let at = meta
            .and_then(|meta| meta.modified().ok())
            .and_then(|at| at.duration_since(UNIX_EPOCH).ok())
            .map_or(0, |at| at.as_nanos());
        (path.to_string_lossy().into_owned(), size, at)
    };
    let mut marked: Vec<_> = ["decisions.jsonl", "acted.jsonl"]
        .iter()
        .map(|name| one(&state.join(name)))
        .collect();
    for folder in ["stops", pause::carry::CARRIED] {
        let files = fs::read_dir(state.join(folder))
            .into_iter()
            .flatten()
            .flatten()
            .map(|entry| one(&entry.path()));
        marked.extend(files);
    }
    marked.sort();
    marked
}

/// A notice on screen: the number it was given, and the notices server
/// that gave it, by its name on the bus. The number means that notice only
/// while the same server runs, since one started again, as each switch
/// starts the shell again, numbers its notices from the start (2RCBOJ).
#[derive(Debug, Clone, PartialEq, Eq)]
struct Shown {
    id: u32,
    server: String,
}

impl Shown {
    fn kept(&self) -> Value {
        json!({"id": self.id, "server": self.server})
    }

    fn read(kept: &Value) -> Option<Self> {
        Some(Self {
            id: u32::try_from(kept["id"].as_u64()?).ok()?,
            server: kept["server"].as_str()?.to_owned(),
        })
    }
}

/// What was told, each with the notice it put on screen, if one it can
/// close. A list, as told.json was kept before notices were closed, is
/// read as told with no notice to close.
fn read_told(state: &Path) -> BTreeMap<String, Option<Shown>> {
    let kept = fs::read_to_string(state.join("told.json"))
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok());
    match kept {
        Some(Value::Array(keys)) => keys
            .iter()
            .filter_map(Value::as_str)
            .map(|key| (key.to_owned(), None))
            .collect(),
        Some(Value::Object(told)) => told
            .into_iter()
            .map(|(key, shown)| (key, Shown::read(&shown)))
            .collect(),
        _ => BTreeMap::new(),
    }
}

/// The programs a notice is sent and closed with.
struct Knocking<'a> {
    notify: Option<&'a Path>,
    busctl: Option<&'a Path>,
}

/// What a row is told as: its key and the saying it shows, so an alert
/// dismissed and then said again in its thread is told again.
fn told_as(row: &Value) -> String {
    format!(
        "{}@{}",
        row["key"].as_str().unwrap_or_default(),
        row["saying"].as_str().unwrap_or_default()
    )
}

/// Tells each row new since the last: a notice for a live one, and for
/// every one a line in would-tell.jsonl with the message Joel's own Slack
/// would be sent, which nothing sends yet. What's told is kept only for
/// rows standing, so it never grows past them, and the notice of one that
/// no longer stands is closed.
fn tell_new(
    state: &Path,
    served: &[Value],
    shadow: &[Value],
    knocking: &Knocking,
    told: &mut BTreeMap<String, Option<Shown>>,
) {
    let standing: BTreeSet<String> = served.iter().chain(shadow).map(told_as).collect();
    let before = told.len();
    told.retain(|key, shown| {
        let stands = standing.contains(key);
        if let (false, Some(shown), Some(busctl)) = (stands, shown.as_ref(), knocking.busctl) {
            close(busctl, shown);
        }
        stands
    });
    let mut moved = told.len() != before;
    for row in served.iter().chain(shadow) {
        let key = told_as(row);
        if told.contains_key(&key) {
            continue;
        }
        let live = row["shadow"] == false;
        let knock = match knocking.notify {
            Some(program) if live => notice(program, row),
            _ => Knock::Missed,
        };
        let knocked = matches!(knock, Knock::Sent(_));
        let shown = match (knock, knocking.busctl) {
            (Knock::Sent(Some(id)), Some(busctl)) => {
                server(busctl).map(|server| Shown { id, server })
            }
            _ => None,
        };
        let would = json!({
            "at": now_ms(),
            "key": row["key"],
            "saying": row["saying"],
            "shadow": !live,
            "knocked": knocked,
            "title": row["title"],
            "words": row["words"],
            "link": row["link"],
            "dm": dm(row),
        });
        if let Err(error) = record::append(&state.join("would-tell.jsonl"), &would) {
            eprintln!("<4>couldn't write what it would tell: {error}");
        }
        told.insert(key, shown);
        moved = true;
    }
    if moved {
        let kept: serde_json::Map<String, Value> = told
            .iter()
            .map(|(key, shown)| (key.clone(), shown.as_ref().map_or(Value::Null, Shown::kept)))
            .collect();
        let written = state.join("told.new");
        let saved = fs::write(&written, Value::Object(kept).to_string())
            .and_then(|()| fs::rename(&written, state.join("told.json")));
        if let Err(error) = saved {
            eprintln!("<4>couldn't keep what it told: {error}");
        }
    }
}

/// Whether a notice went, and the number it was given, which closes it.
enum Knock {
    Missed,
    Sent(Option<u32>),
}

/// Knocks: a notice that sounds even while notices are held or Joel is in
/// a call, which the notices daemon gives this category (2RCBOJ).
fn notice(notify: &Path, row: &Value) -> Knock {
    let title = row["title"].as_str().unwrap_or_default();
    let body = format!(
        "{}\n{}",
        row["words"].as_str().unwrap_or_default(),
        row["link"].as_str().unwrap_or_default()
    );
    let sent = Command::new(notify)
        .args([
            "--urgency=critical",
            "--category=pineapple.stop",
            "--app-name=pineapple",
            "--print-id",
            title,
            &body,
        ])
        .output();
    match sent {
        Ok(output) if output.status.success() => {
            Knock::Sent(String::from_utf8_lossy(&output.stdout).trim().parse().ok())
        }
        Ok(output) => {
            eprintln!("<4>notify-send ended {}", output.status);
            Knock::Missed
        }
        Err(error) => {
            eprintln!("<4>couldn't run notify-send: {error}");
            Knock::Missed
        }
    }
}

/// The notices server's name on the bus now, which changes whenever it
/// starts again.
fn server(busctl: &Path) -> Option<String> {
    let asked = Command::new(busctl)
        .args([
            "--user",
            "call",
            "org.freedesktop.DBus",
            "/org/freedesktop/DBus",
            "org.freedesktop.DBus",
            "GetNameOwner",
            "s",
            "org.freedesktop.Notifications",
        ])
        .output();
    match asked {
        // busctl says a string as `s ":1.42"`.
        Ok(output) if output.status.success() => String::from_utf8_lossy(&output.stdout)
            .trim()
            .strip_prefix("s ")
            .map(|name| name.trim_matches('"').to_owned())
            .filter(|name| !name.is_empty()),
        Ok(output) => {
            eprintln!("<5>no notices server on the bus: {}", output.status);
            None
        }
        Err(error) => {
            eprintln!("<4>couldn't run busctl: {error}");
            None
        }
    }
}

/// Closes a notice by its number, as its sender may, once what it knocked
/// for no longer stands, but only while the server that numbered it still
/// runs: a number from one since started again would close another
/// program's notice. One the notices daemon has already let go is refused,
/// which changes nothing.
fn close(busctl: &Path, shown: &Shown) {
    let Shown { id, server: gave } = shown;
    if server(busctl).as_ref() != Some(gave) {
        eprintln!("<6>notice {id} went with the notices server that showed it");
        return;
    }
    let closed = Command::new(busctl)
        .args([
            "--user",
            "call",
            "org.freedesktop.Notifications",
            "/org/freedesktop/Notifications",
            "org.freedesktop.Notifications",
            "CloseNotification",
            "u",
            &id.to_string(),
        ])
        .output();
    match closed {
        Ok(output) if output.status.success() => {}
        Ok(output) => eprintln!("<5>notice {id} wasn't closed: {}", output.status),
        Err(error) => eprintln!("<4>couldn't run busctl: {error}"),
    }
}

/// The message Joel's own Slack is sent for a row, in the alert's one
/// form (src/phone.rs), which the safe word's reader sends.
fn dm(row: &Value) -> String {
    phone::text(Alert {
        title: row["title"].as_str().unwrap_or_default(),
        words: row["words"].as_str().unwrap_or_default(),
        link: row["link"].as_str().unwrap_or_default(),
    })
}
