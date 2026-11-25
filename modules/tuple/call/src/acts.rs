//! The call's acts, as asks on tuple-call's socket: each done through
//! Tuple's own command on a thread of its own, answered once it's done or
//! once Tuple has been slow to answer, and how Joel stands set ahead of
//! Tuple's stream once one is done, while the call stands as it was asked
//! in.

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, mpsc};
use std::time::Duration;

use daemon_serve::serve::Own;
use serde_json::{Value, json};

use tuple_call::ask::{self, Act, refused, unanswered};
use tuple_call::stream::Person;

use super::{Daemon, Shared, changed, lock, now, write};

/// What the acts need: the tuple command, and where the room is kept.
pub(super) struct Doing {
    pub(super) tuple: String,
    pub(super) room: PathBuf,
}

/// How the call stood as an act was asked: whether one was held, which,
/// by when it connected, and how often the stream had said Joel's camera
/// turned. An act done is how Joel stands ahead of the stream only while
/// the call still stands so, since one done late may otherwise mark off a
/// camera turned on again by hand, or land on another call.
#[derive(Clone, Copy, PartialEq, Eq)]
struct Turn {
    calling: bool,
    since: Option<i64>,
    camera: u64,
}

impl Turn {
    fn of(daemon: &Daemon) -> Turn {
        Turn {
            calling: daemon.held.live.is_some(),
            since: daemon.held.live.as_ref().and_then(|live| live.since),
            camera: daemon.camera_turns,
        }
    }
}

/// Sets how Joel stands in the call held now, ahead of Tuple's stream,
/// and says it to those watching, while the call stands as it did when
/// the act was asked (`Turn`); otherwise the stream says how he stands.
fn ahead(shared: &Shared, asked: Turn, set: impl FnOnce(&mut Person)) {
    let mut daemon = lock(shared);
    if Turn::of(&daemon) != asked {
        return;
    }
    if let Some(live) = &mut daemon.held.live {
        set(&mut live.joel);
    }
    changed(shared, &mut daemon);
}

/// What answers the call's acts, each on a thread of its own (ask's
/// `ACTS`), which daemon-serve asks as a client asks one; it answers
/// none of the interface's own asks, which daemon-serve answers.
pub(super) fn acts(shared: &Shared, doing: Doing) -> Own {
    let shared = Arc::clone(shared);
    Box::new(move |said| ask::act(said).map(|asked| act(&shared, &doing, &asked)))
}

/// How long an act waits on Tuple's command before answering that it
/// hasn't answered, so a command that never ends still has its ask told.
/// Five seconds is far past what Tuple's app takes to answer one that
/// waits on nobody; a share waits on Joel picking a screen, so it's
/// answered once done however long that takes, and opening the room once
/// `tuple new` prints its link, which it does as it starts. The command
/// runs on, what it does once it ends still counts, and its answer goes
/// to the journal.
const ANSWERS_WITHIN: Duration = Duration::from_secs(5);

/// Does one of the call's acts through Tuple's command, answering once
/// it's done, or once `ANSWERS_WITHIN` has passed without it. One asked
/// again while Tuple's command for it still runs is answered at once
/// rather than run twice, so a Tuple that never answers holds one command
/// for each act, however often Joel asks.
fn act(shared: &Shared, doing: &Doing, asked: &Act) -> Value {
    let arguments: Vec<&str> = match asked {
        Act::Join { link } if link.is_empty() => return refused("that isn't a call's link"),
        Act::Join { link } => vec!["call", "join", link],
        Act::Leave => vec!["call", "leave"],
        Act::Mute { muted: true } => vec!["call", "mute"],
        Act::Mute { muted: false } => vec!["call", "unmute"],
        Act::Share => vec!["share"],
        Act::Camera { on: true } => vec!["webcam", "start"],
        Act::Camera { on: false } => vec!["webcam", "stop"],
        Act::Accept => vec!["call", "accept"],
        Act::Open => return open(shared, doing),
    };
    let arguments: Vec<String> = arguments.into_iter().map(str::to_owned).collect();
    let command = arguments.join(" ");
    let turn = {
        let mut daemon = lock(shared);
        if daemon.acting.contains(&arguments) {
            eprintln!("<4>tuple {command} was asked again before Tuple answered the last");
            return unanswered("Tuple hasn't answered the last time it was asked");
        }
        daemon.acting.push(arguments.clone());
        Turn::of(&daemon)
    };
    let (answer, answered) = mpsc::channel();
    let shared = Arc::clone(shared);
    let tuple = doing.tuple.clone();
    let acted = asked.clone();
    let late = command.clone();
    std::thread::spawn(move || {
        let said = run(&shared, &tuple, &arguments, &acted, turn);
        lock(&shared).acting.retain(|one| *one != arguments);
        // Nobody hears it once the ask was answered as unanswered, so
        // it's journaled instead, Tuple's reason for a refusal with it.
        if let Err(unheard) = answer.send(said) {
            eprintln!("<5>tuple {late} answered late: {}", unheard.0);
        }
    });
    if *asked == Act::Share {
        return answered
            .recv()
            .unwrap_or_else(|_| refused("tuple's command went unanswered"));
    }
    answered.recv_timeout(ANSWERS_WITHIN).unwrap_or_else(|_| {
        let within = ANSWERS_WITHIN.as_secs();
        eprintln!("<4>tuple {command} hasn't answered in {within} seconds");
        unanswered(&format!("Tuple hasn't answered in {within} seconds"))
    })
}

/// Runs Tuple's command for an act, and answers how it ended.
fn run(shared: &Shared, tuple: &str, arguments: &[String], asked: &Act, turn: Turn) -> Value {
    let output = Command::new(tuple)
        .args(arguments)
        .stdin(Stdio::null())
        .output();
    match output {
        Ok(output) if output.status.success() => {
            // Once Tuple says it's done, which it does as well when he
            // already was, that's how Joel stands, ahead of its stream. A
            // camera stopped is off as well, so the launcher offers the
            // picker again at once; a picker opened shares nothing until
            // he picks, so only the stream says his camera is on.
            match asked {
                Act::Mute { muted } => ahead(shared, turn, |joel| joel.muted = *muted),
                Act::Camera { on: false } => ahead(shared, turn, |joel| joel.camera = false),
                _ => {}
            }
            json!({"done": true})
        }
        Ok(output) => {
            let said = String::from_utf8_lossy(&output.stderr);
            let why = said.lines().rev().find(|line| !line.trim().is_empty());
            refused(
                why.map_or_else(|| format!("tuple ended: {}", output.status), str::to_owned)
                    .as_str(),
            )
        }
        Err(why) => refused(&format!("couldn't run tuple: {why}")),
    }
}

/// Opens Joel's room with `tuple new`, answering with the link it prints
/// as it does, and keeping it.
fn open(shared: &Shared, doing: &Doing) -> Value {
    let mut child = match Command::new(&doing.tuple)
        .arg("new")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(child) => child,
        Err(why) => return refused(&format!("couldn't run tuple: {why}")),
    };
    let Some(stdout) = child.stdout.take() else {
        let _ = child.kill();
        let _ = child.wait();
        return refused("tuple new printed nothing");
    };
    let mut lines = BufReader::new(stdout).lines();
    let found = lines
        .by_ref()
        .map_while(Result::ok)
        .map(|line| ask::link(&line).to_owned())
        .find(|link| !link.is_empty());
    // Whatever else it prints is read and let go while it runs on.
    std::thread::spawn(move || {
        lines.for_each(drop);
        let _ = child.wait();
    });
    let Some(link) = found else {
        return refused("tuple new printed no link");
    };
    keep(&doing.room, &link);
    let mut daemon = lock(shared);
    daemon.held.room = Some(link.clone());
    // `tuple new` starts a call in Joel's room, so whoever comes to it
    // came to his call.
    daemon.held.began(String::new(), now());
    changed(shared, &mut daemon);
    json!({"done": true, "link": link})
}

/// Keeps the room's link.
fn keep(room: &Path, link: &str) {
    if let Err(why) = write(room, &format!("{link}\n")) {
        eprintln!("<4>couldn't keep the room's link: {why}");
    }
}
