//! The quests daemon: follows the node's two lists of quests, the open
//! and the ended, and its sessions, makes Quests' rows from all of it in
//! one loop, and serves the open ones' to the launcher over the socket
//! systemd holds for it, each watch told only the rows that moved. A
//! search finds among every quest, ended ones too (search.rs).
//!
//! Its unit alone starts it, in the terminal kit's shape, as
//! `quests --help` says (`help()`): `--vocabulary` is the launcher's
//! `words.json` and `--node` the mycelium node's socket. A quest with no
//! folder of its own starts in `$HOME`.

use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::{Arc, Mutex, PoisonError, mpsc};

use daemon_serve::asks::Domain;
use daemon_serve::serve::{self, Own, Shared};
use daemon_serve::watch::{Held, follow};
use quests::search::Searched;
use quests::served::{self, Inputs};
use quests::words::Words;
use ready_rows::kept::Kept;
use serde_json::{Value, json};
use terminal_kit::{Exit, Failure, Help, flags};

/// The daemon as the launcher knows it: Quests' ready rows, and how many
/// are open.
const DOMAIN: Domain = Domain {
    name: "quests",
    kinds: &["row", "summary"],
    sole: &[],
    named_only: &[],
};

/// The flags its unit hands it, each with a value.
const FLAGS: [&str; 3] = ["--vocabulary", "--node", "--socket"];

/// Something the node said, of one of its watches.
enum Heard {
    Open(Value),
    Ended(Value),
    Sessions(Value),
}

/// Where it reads and serves.
struct Args {
    vocabulary: PathBuf,
    node: PathBuf,
    socket: PathBuf,
}

/// What `quests --help` says: how its unit runs it, and how it ends.
fn help() -> Help {
    Help::new("quests", "makes Quests' rows ready for the launcher")
        .example(
            "quests --vocabulary FILE --node SOCKET --socket SOCKET",
            "as its unit runs it",
        )
        .option(
            "--vocabulary FILE",
            "the launcher's words, which rows are said in",
        )
        .option("--node SOCKET", "the mycelium node, which says the quests")
        .option(
            "--socket SOCKET",
            "where the launcher asks, unless systemd hands one",
        )
        .exit(Exit::Failed, "its socket couldn't be listened on")
        .exit(Exit::Config, "the launcher's words couldn't be read")
}

/// Where it reads and serves, from what its unit hands it.
fn asked(args: &[String]) -> Result<Args, Failure> {
    let wrongly = |what: String| Failure::new(Exit::Usage, what).try_this("quests --help");
    let [mut vocabulary, mut node, mut socket] = [None, None, None];
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        let held = match arg.as_str() {
            "--vocabulary" => &mut vocabulary,
            "--node" => &mut node,
            "--socket" => &mut socket,
            flag if flag.starts_with('-') => return Err(flags::unknown("option", flag, &FLAGS)),
            word => return Err(wrongly(format!("quests takes no word like {word}"))),
        };
        let value = args
            .next()
            .ok_or_else(|| wrongly(format!("{arg} needs a value")))?;
        *held = Some(PathBuf::from(value));
    }
    let missing: Vec<&str> = FLAGS
        .into_iter()
        .zip([&vocabulary, &node, &socket])
        .filter_map(|(flag, value)| value.is_none().then_some(flag))
        .collect();
    match (vocabulary, node, socket) {
        (Some(vocabulary), Some(node), Some(socket)) => Ok(Args {
            vocabulary,
            node,
            socket,
        }),
        _ => Err(wrongly(format!("quests needs {}", flags::and(&missing)))),
    }
}

fn main() -> ExitCode {
    terminal_kit::main(&help(), env!("CARGO_PKG_VERSION"), |context| {
        serve_quests(&asked(&context.args)?)
    })
}

/// Follows the node's quests and serves their rows for as long as it
/// runs.
fn serve_quests(args: &Args) -> Result<(), Failure> {
    // Started without the launcher's words, it would serve a summary
    // saying nothing, so it says why and stops for systemd to start again.
    let read = std::fs::read_to_string(&args.vocabulary).map_err(|why| why.to_string());
    let words = read
        .and_then(|text| Words::from_vocabulary(&text).map_err(|why| why.to_string()))
        .map_err(|why| {
            let path = args.vocabulary.display();
            Failure::new(
                Exit::Config,
                format!("it couldn't read the words in {path}"),
            )
            .because(why)
        })?;
    let home = std::env::var("HOME").unwrap_or_default();
    // What a search finds among, every quest as last made, shared with
    // the asks each connection answers.
    let searched = Arc::new(Mutex::new(Searched::default()));
    let asked = Arc::clone(&searched);
    let own: Own = Box::new(move |said| {
        let searched = || asked.lock().unwrap_or_else(PoisonError::into_inner);
        let mut answer = match said["ask"].as_str() {
            Some("search") => searched().answer(said),
            Some("titles") => searched().titles(said),
            _ => return None,
        };
        answer["id"] = said["id"].clone();
        Some(answer)
    });
    let shared = Shared::new(DOMAIN, own);
    let listener = serve::listen(&args.socket).map_err(|why| {
        let socket = args.socket.display();
        Failure::new(Exit::Failed, format!("it couldn't listen on {socket}"))
            .because(why.to_string())
    })?;
    serve::accept(Arc::clone(&shared), listener);
    let (to, heard) = mpsc::channel();
    follow(args.node.clone(), "quest", Heard::Open, to.clone());
    follow(args.node.clone(), "ended-quest", Heard::Ended, to.clone());
    follow(args.node.clone(), "session", Heard::Sessions, to);
    let (mut open, mut ended, mut sessions) = (Held::default(), Held::default(), Held::default());
    let mut kept = Kept::default();
    while let Ok(first) = heard.recv() {
        // Everything said meanwhile is taken before the rows are made, so
        // the node writing both lists at once makes the rows once.
        let mut moved = false;
        for one in std::iter::once(first).chain(heard.try_iter()) {
            let took = match one {
                Heard::Open(answer) => open.took(&answer),
                Heard::Ended(answer) => ended.took(&answer),
                Heard::Sessions(answer) => sessions.took(&answer),
            };
            moved |= !took.is_empty();
        }
        if !moved {
            continue;
        }
        let made = served::made(&Inputs {
            open: &open,
            ended: &ended,
            sessions: &sessions,
            home: &home,
            words: &words,
        });
        *searched.lock().unwrap_or_else(PoisonError::into_inner) = Searched::of(&made);
        let summary = served::summary_record(&made.said);
        if !kept.take(made.open, vec![summary]).is_empty() {
            shared.change(kept.all().cloned().collect(), json!({}));
        }
    }
    Ok(())
}
