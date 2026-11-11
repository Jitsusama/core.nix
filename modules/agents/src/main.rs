//! The agents daemon: follows what each source says of Joel's sessions,
//! makes Agents' rows from all of it in one loop, and serves them to the
//! launcher over the socket systemd holds for it, each watch told only
//! the rows that moved.
//!
//! Usage: `agents --vocabulary WORDS --node NODE --to-do TODO --windows
//! WINDOWS --spend SPEND --orchestrators ORCHESTRATORS --socket SOCKET`,
//! where WORDS is the launcher's words.json, NODE the mycelium node's
//! socket, TODO the to-do daemon's, WINDOWS the windows daemon's, SPEND
//! spend-watch's collection (its budgets.json read from beside it) and
//! ORCHESTRATORS the harness's list of them.

use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::{Arc, mpsc};

use agents::inputs::Inputs;
use agents::row::Words;
use agents::served::Served;
use agents::sources::{self, Heard, Paths};
use daemon_serve::asks::Domain;
use daemon_serve::serve::{self, Own, Shared};
use serde_json::json;

/// The daemon as the launcher knows it: Agents' ready rows, how its
/// sessions stand together, and each session's drawer.
const DOMAIN: Domain = Domain {
    name: "agents",
    kinds: &["row", "summary", "drawer"],
};

const USAGE: &str = "usage: agents --vocabulary WORDS --node NODE --to-do TODO \
                     --windows WINDOWS --spend SPEND --orchestrators ORCHESTRATORS \
                     --socket SOCKET";

/// Where everything it reads and serves is.
struct Args {
    vocabulary: PathBuf,
    node: PathBuf,
    to_do: PathBuf,
    windows: PathBuf,
    spend: PathBuf,
    orchestrators: PathBuf,
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
        node: named("--node")?,
        to_do: named("--to-do")?,
        windows: named("--windows")?,
        spend: named("--spend")?,
        orchestrators: named("--orchestrators")?,
        socket: named("--socket")?,
    })
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
    // The shell says whether the screen is shared, as Quiet.qml knows it
    // from niri's casts, so no amount is served while it is.
    let told = to.clone();
    let own: Own = Box::new(move |said| {
        (said["ask"] == "sharing").then(|| {
            let sharing = said["sharing"].as_bool().unwrap_or(false);
            let _ = told.send(Heard::Sharing(sharing));
            json!({"sharing": sharing})
        })
    });
    let shared = Shared::new(DOMAIN, own);
    match serve::listen(&args.socket) {
        Ok(listener) => serve::accept(Arc::clone(&shared), listener),
        Err(why) => {
            eprintln!("agents: can't listen on {}: {why}", args.socket.display());
            return ExitCode::FAILURE;
        }
    }
    sources::follow(args.node.clone(), "session", Heard::Sessions, to.clone());
    sources::follow(args.node, "job", Heard::Jobs, to.clone());
    sources::follow(args.to_do, "to-do", Heard::ToDos, to.clone());
    sources::follow(args.windows, "window", Heard::Windows, to.clone());
    let paths = Paths {
        budgets: args.spend.with_file_name("budgets.json"),
        spend: args.spend,
        orchestrators: args.orchestrators,
    };
    if let Err(why) = sources::follow_files(&paths, to) {
        eprintln!("agents: can't hear the files it reads: {why}");
        return ExitCode::FAILURE;
    }
    let mut inputs = Inputs {
        words,
        ..Inputs::default()
    };
    let mut served = Served::default();
    while let Ok(first) = heard.recv() {
        inputs.hear(first, &paths);
        // Everything said meanwhile is taken before the rows are made, so
        // sources speaking at once make the rows once.
        while let Ok(more) = heard.try_recv() {
            inputs.hear(more, &paths);
        }
        if !served.take(inputs.made()).is_empty() {
            shared.change(served.all().cloned().collect(), json!({}));
        }
    }
    ExitCode::SUCCESS
}
