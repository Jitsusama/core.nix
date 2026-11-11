//! The quests daemon: follows the node's two lists of quests, the open
//! and the ended, and its sessions, makes Quests' rows from all of it in
//! one loop, and serves them to the launcher over the socket systemd
//! holds for it, each watch told only the rows that moved.
//!
//! Usage: `quests --node NODE --socket SOCKET`, where NODE is the
//! mycelium node's socket. A quest with no folder of its own starts in
//! `$HOME`.

use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::{Arc, mpsc};

use daemon_serve::asks::Domain;
use daemon_serve::serve::{self, Own, Shared};
use daemon_serve::watch::{Held, follow};
use quests::served::{self, Inputs};
use ready_rows::kept::Kept;
use serde_json::{Value, json};

/// The daemon as the launcher knows it: Quests' ready rows, and how many
/// are open.
const DOMAIN: Domain = Domain {
    name: "quests",
    kinds: &["row", "summary"],
};

const USAGE: &str = "usage: quests --node NODE --socket SOCKET";

/// Something the node said, of one of its watches.
enum Heard {
    Open(Value),
    Ended(Value),
    Sessions(Value),
}

/// Where it reads and serves.
struct Args {
    node: PathBuf,
    socket: PathBuf,
}

fn args() -> Option<Args> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let named = |flag: &str| {
        let at = args.iter().position(|one| one == flag)?;
        args.get(at + 1).map(PathBuf::from)
    };
    Some(Args {
        node: named("--node")?,
        socket: named("--socket")?,
    })
}

fn main() -> ExitCode {
    let Some(args) = args() else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    let home = std::env::var("HOME").unwrap_or_default();
    let own: Own = Box::new(|_| None);
    let shared = Shared::new(DOMAIN, own);
    match serve::listen(&args.socket) {
        Ok(listener) => serve::accept(Arc::clone(&shared), listener),
        Err(why) => {
            eprintln!("quests: can't listen on {}: {why}", args.socket.display());
            return ExitCode::FAILURE;
        }
    }
    let (to, heard) = mpsc::channel();
    follow(args.node.clone(), "quest", Heard::Open, to.clone());
    follow(args.node.clone(), "ended-quest", Heard::Ended, to.clone());
    follow(args.node, "session", Heard::Sessions, to);
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
        let (rows, said) = served::made(&Inputs {
            open: &open,
            ended: &ended,
            sessions: &sessions,
            home: &home,
        });
        if !kept
            .take(rows, vec![served::summary_record(&said)])
            .is_empty()
        {
            shared.change(kept.all().cloned().collect(), json!({}));
        }
    }
    ExitCode::SUCCESS
}
