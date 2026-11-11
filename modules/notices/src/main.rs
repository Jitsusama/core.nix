//! The notices daemon: the session bus's notifications server, holding
//! every notice and its history, and serving each to the shell over the
//! socket systemd holds for it, each watch told only the notices that
//! changed.
//!
//! Usage: `notices --socket SOCKET --state FOLDER --levels LEVELS
//! --senders SENDERS --quiet QUIET [--shells SHELLS]`, where FOLDER keeps
//! the history and the pictures sent as pixels, LEVELS is each sender's
//! level as home.nix writes it, SENDERS is senders.json, QUIET is
//! Quiet.qml's file, and SHELLS is Quickshell's `by-shell` state folder,
//! whose history is taken the first time it starts.

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::{Arc, Mutex, PoisonError, mpsc};

use daemon_serve::asks::Domain;
use daemon_serve::serve::{self, Own, Shared};
use notices::bus::{self, Server, now_ms};
use notices::daemon::State;
use notices::held::Holding;
use notices::judged::Judge;
use notices::store;
use serde_json::json;

/// The daemon as the shell knows it: a record for each notice.
const DOMAIN: Domain = Domain {
    name: "notices",
    kinds: &["notice"],
};

const USAGE: &str = "usage: notices --socket SOCKET --state FOLDER --levels LEVELS \
                     --senders SENDERS --quiet QUIET [--shells SHELLS]";

/// Where it serves, keeps and reads.
struct Args {
    socket: PathBuf,
    state: PathBuf,
    levels: PathBuf,
    senders: PathBuf,
    quiet: PathBuf,
    shells: Option<PathBuf>,
}

fn args() -> Option<Args> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let named = |flag: &str| {
        let at = args.iter().position(|one| one == flag)?;
        args.get(at + 1).map(PathBuf::from)
    };
    Some(Args {
        socket: named("--socket")?,
        state: named("--state")?,
        levels: named("--levels")?,
        senders: named("--senders")?,
        quiet: named("--quiet")?,
        shells: named("--shells"),
    })
}

/// Each sender's level and the senders' own words, or why they can't be
/// read.
fn judge(args: &Args) -> Result<Judge, String> {
    let read = |file: &PathBuf| {
        std::fs::read_to_string(file).map_err(|why| format!("{}: {why}", file.display()))
    };
    Judge::read(&read(&args.levels)?, &read(&args.senders)?).map_err(|why| why.to_string())
}

/// The history kept, or the shell's taken where none is yet; or why what
/// was kept can't be read, so nothing writes over it.
fn holding(args: &Args, file: &Path) -> Result<Holding, String> {
    if let Some(saved) = store::read(file)? {
        return Ok(Holding::new(saved.notices, saved.next));
    }
    let Some(shells) = args.shells.as_deref().and_then(store::shells) else {
        return Ok(Holding::default());
    };
    let mut fresh = Holding::default();
    let taken = std::fs::read_to_string(&shells)
        .map_err(|why| why.to_string())
        .and_then(|text| store::taken(&text, &mut fresh));
    match taken {
        Ok(notices) => {
            eprintln!("notices: took {} from {}", notices.len(), shells.display());
            Ok(Holding::new(notices, fresh.next()))
        }
        // The shell's history is the shell's to keep, so one that doesn't
        // read is left where it is and this starts with none.
        Err(why) => {
            eprintln!("notices: can't take {}: {why}", shells.display());
            Ok(Holding::default())
        }
    }
}

fn main() -> ExitCode {
    let Some(args) = args() else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    let kept = args.state.join("notices.json");
    let (judge, holding) = match judge(&args).and_then(|judge| Ok((judge, holding(&args, &kept)?)))
    {
        Ok(both) => both,
        Err(why) => {
            eprintln!("notices: {why}");
            return ExitCode::FAILURE;
        }
    };
    // Those still open when it last stopped stay open: their senders may
    // still hear what's done with them.
    let state = Arc::new(Mutex::new(State::new(
        holding,
        judge,
        args.quiet.clone(),
        args.state.join("pictures"),
    )));
    let (changed, heard) = mpsc::channel();
    let own: Own = {
        let (state, changed) = (Arc::clone(&state), changed.clone());
        Box::new(move |said| {
            let mut held = state.lock().unwrap_or_else(PoisonError::into_inner);
            let (answer, moved) = held.ask(said, now_ms());
            if moved || !held.owed.is_empty() {
                let _ = changed.send(());
            }
            Some(answer)
        })
    };
    let shared = Shared::new(DOMAIN, own);
    match serve::listen(&args.socket) {
        Ok(listener) => serve::accept(Arc::clone(&shared), listener),
        Err(why) => {
            eprintln!("notices: can't listen on {}: {why}", args.socket.display());
            return ExitCode::FAILURE;
        }
    }
    let connection = match bus::serve(Server {
        state: Arc::clone(&state),
        changed,
    }) {
        Ok(connection) => connection,
        Err(why) => {
            eprintln!("notices: {why}");
            return ExitCode::FAILURE;
        }
    };
    let records = state
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .holding
        .records();
    shared.change(records, json!({}));
    // Everything changed meanwhile is taken at once, so a burst is told
    // and written once; each burst is written as it's told, so a notice
    // that's arrived is never lost to a stop.
    while let Ok(()) = heard.recv() {
        let _ = heard.try_iter().count();
        let (records, owed) = {
            let mut held = state.lock().unwrap_or_else(PoisonError::into_inner);
            if let Err(why) = store::write(&kept, &held.holding) {
                eprintln!("notices: can't write {}: {why}", kept.display());
            }
            (held.holding.records(), std::mem::take(&mut held.owed))
        };
        shared.change(records, json!({}));
        for signal in &owed {
            if let Err(why) = bus::send(&connection, signal) {
                eprintln!("notices: can't send {signal:?}: {why}");
            }
        }
    }
    ExitCode::SUCCESS
}
