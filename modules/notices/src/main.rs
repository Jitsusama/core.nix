//! The notices daemon: the session bus's notifications server, holding
//! every notice and its history, and serving each to the shell over the
//! socket systemd holds for it as one record and its rows, each watch
//! told only those that changed.
//!
//! Usage: `notices --socket SOCKET --state FOLDER --levels LEVELS
//! --senders SENDERS --quiet QUIET --vocabulary WORDS --marks MARKS
//! [--addressed ADDRESSED] [--shells SHELLS]`, where FOLDER keeps the
//! history and the pictures
//! sent as pixels, LEVELS is each sender's level as home.nix writes it,
//! SENDERS is senders.json, QUIET is Quiet.qml's file, WORDS is the
//! launcher's words.json, MARKS each program's mark as home.nix writes
//! them, ADDRESSED what's to Joel from each chat program the machine
//! names, and SHELLS is Quickshell's `by-shell` state folder, whose
//! history is taken the first time it starts.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError, mpsc};

use daemon_serve::asks::Domain;
use daemon_serve::serve::{self, Own, Shared};
use notices::addressed::Asked;
use notices::bus::{self, Server, now_ms};
use notices::daemon::State;
use notices::held::Holding;
use notices::judged::Judge;
use notices::posted;
use notices::row::{self, Rows};
use notices::sender::{Marks, Senders, Signs};
use notices::store;
use notices::words::Words;
use ready_rows::kept::Kept;
use serde_json::{Value, json};
use windows::entries::{self, Entries};

/// The daemon as the shell knows it: a record for each notice, its rows,
/// and beside the mixed place's page the count it has rows for, sent
/// only to a watch that names it, as the launcher's does.
const DOMAIN: Domain = Domain {
    name: "notices",
    kinds: &["notice", "row", "summary"],
    sole: &[],
    named_only: &["summary"],
};

const USAGE: &str = "usage: notices --socket SOCKET --state FOLDER --levels LEVELS \
                     --senders SENDERS --quiet QUIET --vocabulary WORDS --marks MARKS \
                     [--addressed ADDRESSED] [--shells SHELLS]";

/// Where it serves, keeps and reads.
struct Args {
    socket: PathBuf,
    state: PathBuf,
    levels: PathBuf,
    senders: PathBuf,
    quiet: PathBuf,
    vocabulary: PathBuf,
    marks: PathBuf,
    addressed: Option<PathBuf>,
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
        vocabulary: named("--vocabulary")?,
        marks: named("--marks")?,
        addressed: named("--addressed"),
        shells: named("--shells"),
    })
}

fn read(file: &Path) -> Result<String, String> {
    std::fs::read_to_string(file).map_err(|why| format!("{}: {why}", file.display()))
}

/// Each sender's level and the senders' own words, or why they can't be
/// read.
fn judge(args: &Args) -> Result<Judge, String> {
    let judge =
        Judge::read(&read(&args.levels)?, &read(&args.senders)?).map_err(|why| why.to_string())?;
    match &args.addressed {
        Some(file) => judge
            .addressing(&read(file)?)
            .map_err(|why| format!("{}: {why}", file.display())),
        None => Ok(judge),
    }
}

/// What a notice's rows are said and signed in: the desktop's own
/// senders, each program's mark and the launcher's words. One that can't
/// be read is said and none is used, so the rows say less, rather than
/// the daemon stopping, which would leave nothing to take a notice.
fn signing(args: &Args) -> (Senders, Marks, Words) {
    fn or_none<T: Default>(file: &Path, made: impl FnOnce(&str) -> serde_json::Result<T>) -> T {
        let made = read(file).and_then(|text| made(&text).map_err(|why| why.to_string()));
        made.unwrap_or_else(|why| {
            eprintln!(
                "notices: {} can't be read, so none is used: {why}",
                file.display()
            );
            T::default()
        })
    }
    (
        or_none(&args.senders, |text| serde_json::from_str(text)),
        or_none(&args.marks, Marks::read),
        or_none(&args.vocabulary, Words::from_vocabulary),
    )
}

/// The rows as served: one place's kept, so each row keeps its order.
#[derive(Default)]
struct Served {
    rows: Rows,
    places: [Kept; 3],
    /// The notices Desktop showed when its rows were last made, kept there
    /// once read for as long as it's watched.
    looked: BTreeSet<u32>,
}

impl Served {
    /// Every record, its notices' and their rows, as they stand: each
    /// notice's with its card's sender and words beside it.
    fn records(&mut self, holding: &Holding, signs: &Signs, desktop_watched: bool) -> Vec<Value> {
        let looked = if desktop_watched {
            std::mem::take(&mut self.looked)
        } else {
            BTreeSet::new()
        };
        let [mixed, desktop, history] = self.rows.make(holding.notices().iter(), signs, &looked);
        self.looked = row::shown(&desktop);
        // The mixed place serves a page, saying how many it has rows for.
        let summary = json!({ "key": row::SUMMARY, "kind": "summary",
            "summary": { "count": mixed.len() } });
        self.places[0].take(row::paged(mixed), vec![summary]);
        self.places[1].take(desktop, Vec::new());
        self.places[2].take(history, Vec::new());
        let mut records: Vec<Value> = holding
            .notices()
            .iter()
            .map(|notice| {
                let mut record = notice.record();
                if let Some(card) = self.rows.card(notice.id).and_then(Value::as_object) {
                    for (field, value) in card {
                        record[field] = value.clone();
                    }
                }
                record
            })
            .collect();
        records.extend(self.places.iter().flat_map(Kept::all).cloned());
        records
    }

    /// The mixed place's rows its watch doesn't serve, by key, for a
    /// search or a `rows` ask to answer with: those read past its page.
    fn unserved(&self) -> BTreeMap<String, Value> {
        self.rows
            .mixed()
            .filter_map(|row| {
                let key = row["key"].as_str()?;
                self.places[0]
                    .get(key)
                    .is_none()
                    .then(|| (key.to_owned(), row.clone()))
            })
            .collect()
    }
}

/// The history kept, or the shell's taken where none is yet; or why what
/// was kept can't be opened, so nothing writes over it. One that doesn't
/// read was set aside, and none is kept, rather than the shell's taken
/// again, which it was the first time.
fn holding(args: &Args, file: &Path) -> Result<Holding, String> {
    match store::read(file, now_ms())? {
        store::Kept::Saved(saved) => return Ok(Holding::new(saved.notices, saved.next)),
        store::Kept::SetAside(aside, why) => {
            eprintln!(
                "notices: {} didn't read ({why}), so it's set aside as {} and none is kept",
                file.display(),
                aside.display()
            );
            return Ok(Holding::default());
        }
        store::Kept::None => {}
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

/// The desktop entries, read again as they change, which may sign any
/// notice anew; and whether they have since the rows were last made.
fn followed(changed: mpsc::Sender<()>) -> (Arc<Mutex<Entries>>, Arc<AtomicBool>) {
    let folders = programs::xdg::folders_here();
    let (read_entries, looked) = Entries::read(&folders);
    let entries = Arc::new(Mutex::new(read_entries));
    let resigned = Arc::new(AtomicBool::new(false));
    let (reading, rereading, told) = (folders.clone(), Arc::clone(&entries), Arc::clone(&resigned));
    let following = entries::follow(&folders, looked, move || {
        let (read_entries, _) = Entries::read(&reading);
        *rereading.lock().unwrap_or_else(PoisonError::into_inner) = read_entries;
        told.store(true, Ordering::Release);
        let _ = changed.send(());
    });
    if let Err(why) = following {
        eprintln!("notices: inotify isn't there, so entries changing aren't heard: {why}");
    }
    (entries, resigned)
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
    // Written at once, so the shell's history, where it was taken, is
    // read only the once.
    if let Err(why) = store::write(&kept, &holding) {
        eprintln!("notices: can't write {}: {why}", kept.display());
    }
    let (senders, marks, words) = signing(&args);
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
    {
        let changed = changed.clone();
        shared.on_watches(Box::new(move || {
            let _ = changed.send(());
        }));
    }
    let (entries, resigned) = followed(changed.clone());
    let mut served = Served::default();
    // What's served of what `held` holds, handing it the rows a search
    // answers with.
    let mut records = |held: &mut State| {
        if resigned.swap(false, Ordering::Acquire) {
            served.rows.forget();
        }
        let entries = entries.lock().unwrap_or_else(PoisonError::into_inner);
        let signs = Signs {
            senders: &senders,
            marks: &marks,
            entries: &entries,
            words: &words,
        };
        let told = served.records(&held.holding, &signs, shared.watches("row", row::DESKTOP));
        held.unserved(served.unserved());
        told
    };
    match serve::listen(&args.socket) {
        Ok(listener) => serve::accept(Arc::clone(&shared), listener),
        Err(why) => {
            eprintln!("notices: can't listen on {}: {why}", args.socket.display());
            return ExitCode::FAILURE;
        }
    }
    let asker = changed.clone();
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
    let first = records(&mut state.lock().unwrap_or_else(PoisonError::into_inner));
    shared.change(first, json!({}));
    // Everything changed meanwhile is taken at once, so a burst is told
    // and written once; each burst is written as it's told, so a notice
    // that's arrived is never lost to a stop. It's written first, so a
    // notice the shell has been told of is already kept.
    while let Ok(()) = heard.recv() {
        let _ = heard.try_iter().count();
        let (told, owed, asking) = {
            let mut held = state.lock().unwrap_or_else(PoisonError::into_inner);
            if let Err(why) = store::write(&kept, &held.holding) {
                eprintln!("notices: can't write {}: {why}", kept.display());
            }
            let owed = std::mem::take(&mut held.owed);
            (records(&mut held), owed, std::mem::take(&mut held.asking))
        };
        shared.change(told, json!({}));
        for (id, asked, at) in asking {
            ask(Arc::clone(&state), asker.clone(), id, asked, at);
        }
        for signal in &owed {
            if let Err(why) = bus::send(&connection, signal) {
                eprintln!("notices: can't send {signal:?}: {why}");
            }
        }
    }
    ExitCode::SUCCESS
}

/// Asks a sender's daemon, on a thread of its own, whether a notice it
/// batched was a post, and judges it to Joel where it wasn't or no answer
/// came in time, telling the loop it changed.
fn ask(state: Arc<Mutex<State>>, changed: mpsc::Sender<()>, id: u32, asked: Asked, at: i64) {
    std::thread::spawn(move || {
        let runtime = std::env::var_os("XDG_RUNTIME_DIR")
            .map(PathBuf::from)
            .unwrap_or_default();
        let socket = posted::socket(&asked.socket, &runtime);
        if posted::post(&socket, &asked, at, posted::PATIENCE) == Some(true) {
            return;
        }
        let moved = state
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .replied(id, now_ms());
        if moved {
            let _ = changed.send(());
        }
    });
}

#[cfg(test)]
mod tests {
    use super::DOMAIN;
    use daemon_serve::asks::Ask;

    #[test]
    fn the_launcher_watches_the_notices_rows_with_their_summary() {
        let ask = DOMAIN.ask(r#"{"id":1,"ask":"watch","kind":["row","summary"]}"#);
        assert!(
            matches!(&ask, Ask::Watch { kinds, .. } if kinds == &["row", "summary"]),
            "the watch ReadyRows.qml sends was answered {ask:?}"
        );
    }
}
