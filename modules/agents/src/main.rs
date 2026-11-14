//! The agents daemon: follows what each source says of Joel's sessions,
//! makes Agents' rows from all of it in one loop, and serves them to the
//! launcher over the socket systemd holds for it, each watch told only
//! the rows that moved; and does a row's act as the shell asks by it,
//! steering its session through the node (acts.rs); and serves the
//! sessions there are to reopen, keeping what Joel says of them
//! (reopen.rs), what the sessions landed, keeping what of it he has
//! read (landed.rs), the closed orchestrators (closed.rs), and what the
//! sessions answered of his talks, saying where he read it (answered.rs).
//!
//! Usage: `agents --vocabulary WORDS [--node NODE] --to-do TODO --windows
//! WINDOWS --spend SPEND --orchestrators ORCHESTRATORS --restore RESTORE
//! --kept KEPT --landed LANDED --steer STEER --shells SHELLS [--switched
//! SWITCHED --clone CLONE] [--harness HARNESS] --socket SOCKET`, where
//! WORDS is the launcher's
//! words.json, NODE the mycelium node's socket, TODO the to-do daemon's,
//! WINDOWS the windows daemon's, SPEND spend-watch's collection (its
//! budgets.json read from beside it), ORCHESTRATORS the harness's list of
//! them, RESTORE its list of sessions to reopen, KEPT the daemon's own
//! file of what Joel said of those (where each session's window sat is
//! kept beside it, in arrangement.json, and where those moving onto pi's
//! host sat, in kept-places.json), LANDED its file of the landings he
//! has read, STEER the harness's steer folders, which keep each
//! session's talks, SHELLS the launcher's own folders, where it kept them
//! before the daemon did, SWITCHED update-to-main's answer to its last
//! ask and CLONE a clone of core's, which together say what of core's
//! and mycelium's merges run, so a landing of theirs waits for its
//! switch before To Try holds it (carried.rs; the core is kept beside
//! KEPT, in carried.json), and with neither nothing waits; HARNESS is
//! where pi-packages installs the harness, which says the same of its
//! landings. A machine with no realm runs no node,
//! so it's started with none, and serves the sessions to reopen alone.

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::mpsc::RecvTimeoutError;
use std::sync::{Arc, mpsc};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use agents::acts;
use agents::arrangement::{Arrangement, Still};
use agents::feedback::{self, Landings};
use agents::inputs::Inputs;
use agents::kept_places::KeptPlaces;
use agents::landed::Reading;
use agents::reopen::Kept;
use agents::row::Words;
use agents::served::Served;
use agents::session::Session;
use agents::sources::{self, Heard, Paths};
use agents::tell::{self, Routes};
use agents::wants::Wants;
use daemon_serve::asks::Domain;
use daemon_serve::serve::{self, Own, Shared};
use serde_json::{Value, json};

/// The daemon as the launcher knows it: Agents' ready rows, how its
/// sessions stand together, each session's drawer, and the sessions
/// there are to reopen, with Sessions to Reopen's own words. A landing's
/// drawer is told only to a watch naming it, as the shell does for the
/// few it's about to draw (landed.rs).
const DOMAIN: Domain = Domain {
    name: "agents",
    kinds: &[
        "row",
        "summary",
        "drawer",
        "reopen",
        "restoring",
        "to-reopen",
        "recent",
        "landed",
        "landed-drawer",
        "landed-days",
        "try",
        "closed",
        "agent-window",
        "arrangement",
        "kept-places",
        "want",
    ],
    sole: &[],
    named_only: &["landed-drawer"],
};

const USAGE: &str = "usage: agents --vocabulary WORDS [--node NODE] --to-do TODO \
                     --windows WINDOWS --spend SPEND --orchestrators ORCHESTRATORS \
                     --restore RESTORE --kept KEPT --landed LANDED --steer STEER \
                     --shells SHELLS [--switched SWITCHED --clone CLONE] \
                     [--harness HARNESS] --socket SOCKET";

/// This boot, as Linux names it, or none where it isn't said, when no
/// boot's sessions are offered as the last boot's.
const BOOT_ID: &str = "/proc/sys/kernel/random/boot_id";

/// What Joel said of landings, kept beside what he has read, never
/// rewritten, for the daily loop that reads on from where it left off
/// (80ECO2, 2026-11-11), and read as the daemon starts for how each talk
/// of it went.
const FEEDBACK: &str = "feedback.jsonl";

/// Where everything it reads and serves is.
struct Args {
    vocabulary: PathBuf,
    node: Option<PathBuf>,
    to_do: PathBuf,
    windows: PathBuf,
    spend: PathBuf,
    orchestrators: PathBuf,
    restore: PathBuf,
    kept: PathBuf,
    landed: PathBuf,
    steer: PathBuf,
    shells: PathBuf,
    switched: Option<PathBuf>,
    clone: Option<PathBuf>,
    harness: Option<PathBuf>,
    socket: PathBuf,
}

/// What it was started with, every path named.
fn args() -> Option<Args> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let named = |flag: &str| {
        let at = args.iter().position(|one| one == flag)?;
        args.get(at + 1).map(PathBuf::from)
    };
    Some(Args {
        vocabulary: named("--vocabulary")?,
        node: named("--node"),
        to_do: named("--to-do")?,
        windows: named("--windows")?,
        spend: named("--spend")?,
        orchestrators: named("--orchestrators")?,
        restore: named("--restore")?,
        kept: named("--kept")?,
        landed: named("--landed")?,
        steer: named("--steer")?,
        shells: named("--shells")?,
        switched: named("--switched"),
        clone: named("--clone"),
        harness: named("--harness"),
        socket: named("--socket")?,
    })
}

/// What the daemon answers of its own, beside its records: what the shell
/// says, handed `told` to the daemon's loop, and a row's act, done by
/// asking the node at `node`, refused where there's none, and a line Joel
/// tells whoever `routes` says hears it. Joel's feedback on one of
/// `landings` is kept in the file at `kept`. An answer goes to the one
/// of `wants` its session asks.
fn own(
    told: mpsc::Sender<Heard>,
    node: Option<PathBuf>,
    routes: Routes,
    (landings, wants): (Landings, Wants),
    kept: PathBuf,
) -> Own {
    Box::new(move |said| {
        // The shell says whether the screen is shared, as Quiet.qml knows
        // it from niri's casts, so no amount is served while it is.
        if said["ask"] == "sharing" {
            let sharing = said["sharing"].as_bool().unwrap_or(false);
            let _ = told.send(Heard::Sharing(sharing));
            return Some(json!({"sharing": sharing}));
        }
        // What Joel says of the sessions to reopen: Not Now on the last
        // boot's, or Forget on one of them.
        if said["ask"] == "put-off" {
            let _ = told.send(Heard::PutOff);
            return Some(json!({"put-off": true}));
        }
        // The session is named by pi's id for it as `session`, since `id`
        // is the ask's own.
        if said["ask"] == "forget" {
            let Some(id) = said["session"].as_str().filter(|id| !id.is_empty()) else {
                return Some(json!({"error": {"code": "not-an-ask",
                    "message": "a forget names no `session`"}}));
            };
            let _ = told.send(Heard::Forget(id.to_owned()));
            return Some(json!({"forgot": id}));
        }
        // Where the windows of sessions moving onto pi's host sat, all of
        // them, as the shell says each time one's kept or let go.
        if said["ask"] == "keep-places" {
            let Some(kept) = KeptPlaces::said(&said["places"]) else {
                return Some(json!({"error": {"code": "not-an-ask",
                    "message": "a keep-places names no `places`, each a place by a session"}}));
            };
            let count = kept.places.len();
            let _ = told.send(Heard::KeepPlaces(kept));
            return Some(json!({"kept": count}));
        }
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |since| since.as_millis());
        // Joel reads landings, by their keys, as he opens one or its drawer.
        if said["ask"] == "read" {
            let Some(keys) = said["keys"].as_array() else {
                return Some(json!({"error": {"code": "not-an-ask",
                    "message": "a read names no `keys`"}}));
            };
            let keys: Vec<String> = keys
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect();
            let count = keys.len();
            let _ = told.send(Heard::Read(keys, i64::try_from(now).unwrap_or(0)));
            return Some(json!({"read": count}));
        }
        // Joel opens a session's drawer, by its address, so he's read what
        // it answered him, if anything.
        if said["ask"] == "heard" {
            let Some(address) = said["address"].as_str().filter(|one| !one.is_empty()) else {
                return Some(json!({"error": {"code": "not-an-ask",
                    "message": "a heard names no `address`"}}));
            };
            let now = i64::try_from(now).unwrap_or(0);
            let _ = told.send(Heard::Opened(address.to_owned(), now));
            return Some(json!({"heard": address}));
        }
        if said["ask"] == "tell" {
            return Some(tell::answer(said, &routes, node.as_deref(), now));
        }
        // Joel's words on a landing, sent to whoever landed it, and the
        // line kept of a talk told the loop, whose drawer says how it went.
        if said["ask"] == "feedback" {
            let (answer, line) =
                feedback::answer(said, &landings, &routes, node.as_deref(), &kept, now);
            if let Some(line) = line.filter(|line| line["by"] == "talk") {
                let _ = told.send(Heard::FedBack(line));
            }
            return Some(answer);
        }
        if let Some(answered) = wants.answer(said, node.as_deref(), now) {
            return Some(answered);
        }
        acts::steer_of(said, now).map(|steer| match &node {
            Some(node) => acts::done(steer, node),
            None => json!({"error": {"code": "unsteered",
                "message": "this machine runs no node to steer it through"}}),
        })
    })
}

/// Follows the windows daemon: its windows, its workspaces, and Screens'
/// rows alone, for what each screen is, since a place names its screen
/// by that rather than by a connector a dock can change.
fn follow_windows(windows: &Path, to: &mpsc::Sender<Heard>) {
    sources::follow(windows.to_owned(), "window", Heard::Windows, to.clone());
    sources::follow(
        windows.to_owned(),
        "workspace",
        Heard::Workspaces,
        to.clone(),
    );
    sources::follow_asking(
        windows.to_owned(),
        json!({"kind": "row", "place": "displays"}),
        Heard::Screens,
        to.clone(),
    );
}

/// Everything said, at least one, waiting on nothing but what's said
/// unless places wait to be kept once the windows are still; none once
/// that wait ends, and an error once nothing can be said any more.
fn hear(heard: &mpsc::Receiver<Heard>, still: &Still) -> Result<Vec<Heard>, RecvTimeoutError> {
    let first = match still.wait(Instant::now()) {
        None => Some(heard.recv().map_err(|_| RecvTimeoutError::Disconnected)?),
        Some(wait) => match heard.recv_timeout(wait) {
            Ok(first) => Some(first),
            Err(RecvTimeoutError::Timeout) => None,
            Err(gone) => return Err(gone),
        },
    };
    let mut said: Vec<Heard> = first.into_iter().collect();
    // Everything said meanwhile is taken before the rows are made, so
    // sources speaking at once make the rows once.
    while let Ok(more) = heard.try_recv() {
        said.push(more);
    }
    Ok(said)
}

/// The inputs as the daemon starts: what it keeps read from its own
/// files, or from the shell's where it has kept none yet.
fn started(words: Words, paths: &Paths) -> Inputs {
    Inputs {
        words,
        boot: std::fs::read_to_string(BOOT_ID)
            .map(|id| id.trim().to_owned())
            .unwrap_or_default(),
        kept: Kept::read(&paths.kept),
        home: std::env::var("HOME").unwrap_or_default(),
        read: Reading::read(&paths.landed).unwrap_or_else(|| Reading::from_shell(&paths.shells)),
        arrangement: Arrangement::read(&paths.arrangement)
            .unwrap_or_else(|| Arrangement::from_shell(&paths.shells)),
        kept_places: KeptPlaces::read(&paths.kept_places)
            .unwrap_or_else(|| KeptPlaces::from_shell(&paths.shells)),
        fed_back: feedback::kept(&paths.landed.with_file_name(FEEDBACK)),
        ..Inputs::default()
    }
}

fn main() -> ExitCode {
    let Some(args) = args() else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    let words = std::fs::read_to_string(&args.vocabulary)
        .map_err(|why| why.to_string())
        .and_then(|text| Words::from_vocabulary(&text).map_err(|why| why.to_string()));
    let words = match words {
        Ok(words) => words,
        Err(why) => {
            eprintln!(
                "agents: can't read the vocabulary {}: {why}",
                args.vocabulary.display()
            );
            return ExitCode::FAILURE;
        }
    };
    let (to, heard) = mpsc::channel();
    let routes = Routes::default();
    let landings = Landings::default();
    let wants = Wants::default();
    // Kept beside what Joel has read, never rewritten, for the daily loop
    // that reads on from where it left off (80ECO2, 2026-11-11).
    let kept = args.landed.with_file_name(FEEDBACK);
    let own = own(
        to.clone(),
        args.node.clone(),
        routes.clone(),
        (landings.clone(), wants.clone()),
        kept,
    );
    let shared = Shared::new(DOMAIN, own);
    // Feedback waits on the node, so it holds up no ask after it.
    shared.slow(&["feedback"]);
    match serve::listen(&args.socket) {
        Ok(listener) => serve::accept(Arc::clone(&shared), listener),
        Err(why) => {
            eprintln!("agents: can't listen on {}: {why}", args.socket.display());
            return ExitCode::FAILURE;
        }
    }
    if let Some(node) = args.node {
        sources::follow(node.clone(), "session", Heard::Sessions, to.clone());
        sources::follow(node.clone(), "job", Heard::Jobs, to.clone());
        sources::follow(node.clone(), "quest", Heard::Quests, to.clone());
        sources::follow(node.clone(), "talk", Heard::Talks, to.clone());
        sources::follow(node, "landing", Heard::Landings, to.clone());
    }
    sources::follow(args.to_do, "to-do", Heard::ToDos, to.clone());
    follow_windows(&args.windows, &to);
    let paths = Paths {
        budgets: args.spend.with_file_name("budgets.json"),
        spend: args.spend,
        orchestrators: args.orchestrators,
        restore: args.restore,
        arrangement: args.kept.with_file_name("arrangement.json"),
        kept_places: args.kept.with_file_name("kept-places.json"),
        carried: args.kept.with_file_name("carried.json"),
        kept: args.kept,
        switched: args.switched,
        clone: args.clone,
        harness: args.harness,
        landed: args.landed,
        steer: args.steer,
        shells: args.shells,
    };
    if let Err(why) = sources::follow_inboxes(&paths.steer, to.clone()) {
        eprintln!("agents: can't hear the sessions' inboxes: {why}");
        return ExitCode::FAILURE;
    }
    if let Err(why) = sources::follow_files(&paths, to) {
        eprintln!("agents: can't hear the files it reads: {why}");
        return ExitCode::FAILURE;
    }
    let mut inputs = started(words, &paths);
    let mut served = Served::default();
    let mut still = Still::default();
    while let Ok(said) = hear(&heard, &still) {
        for one in said {
            if one.moves_places() {
                still.moved(Instant::now());
            }
            inputs.hear(one, &paths);
        }
        if still.due(Instant::now()) {
            inputs.keep_places(&paths.arrangement);
        }
        if !served.take(inputs.made()).is_empty() {
            landings.hold(served.all());
            wants.hold(served.all());
            shared.change(served.all().cloned().collect(), json!({}));
        }
        // A quest can change whom a line reaches with no row moving.
        routes.hold(
            inputs.sessions.records().map(|(_, one)| Session(one)),
            inputs.quests.records().map(|(_, one)| one),
            served.all(),
        );
    }
    ExitCode::SUCCESS
}
