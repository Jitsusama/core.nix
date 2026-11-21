//! The notices daemon: the session bus's notifications server, holding
//! every notice and its history, and serving each to the shell over the
//! socket systemd holds for it as one record and its rows, each watch
//! told only those that changed.
//!
//! Its unit alone starts it, in the terminal kit's shape, as
//! `notices --help` says (`help()`): `--state` keeps the history and the
//! pictures sent as pixels, `--levels` is each sender's level as home.nix
//! writes it, `--senders` is senders.json, `--quiet` is Quiet.qml's file,
//! `--vocabulary` the launcher's words.json, `--marks` each program's
//! mark as home.nix writes them, `--addressed` what's to Joel from each
//! chat program the machine names, and `--shells` Quickshell's `by-shell`
//! state folder, whose history is taken the first time it starts.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError, mpsc};

use daemon_serve::asks::Domain;
use daemon_serve::serve::{self, Own, Shared};
use notices::addressed::Asked;
use notices::bus::{self, Server, now_ms};
use notices::daemon::State;
use notices::front;
use notices::held::Holding;
use notices::judged::Judge;
use notices::posted;
use notices::row::{self, Rows};
use notices::sender::{Marks, Senders, Signs};
use notices::store;
use notices::words::Words;
use ready_rows::kept::Kept;
use serde_json::{Value, json};
use terminal_kit::{Exit, Failure, Help, flags};
use windows::entries::{self, Entries};

/// The daemon as the shell knows it: a record for each recent notice
/// (notice.rs's recent), the one in front (front.rs), the rows of every
/// one, and beside the mixed place's page and History's the count each
/// has rows for, sent only to a watch that names it, as the launcher's
/// does.
const DOMAIN: Domain = Domain {
    name: "notices",
    kinds: &["front", "notice", "row", "summary"],
    sole: &[],
    named_only: &["summary"],
};

/// The flags its unit hands it, each with a value, those it can't start
/// without first.
const FLAGS: [&str; 9] = [
    "--socket",
    "--state",
    "--levels",
    "--senders",
    "--quiet",
    "--vocabulary",
    "--marks",
    "--addressed",
    "--shells",
];

/// How many of `FLAGS` it can't start without.
const NEEDED: usize = 7;

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

/// What `notices --help` says: how its unit runs it, and how it ends.
fn help() -> Help {
    Help::new("notices", "holds the desktop's notices for the shell")
        .example(
            "notices --socket SOCKET --state FOLDER ...",
            "as its unit runs it, with every flag below",
        )
        .option(
            "--socket SOCKET",
            "where the shell asks, unless systemd hands one",
        )
        .option(
            "--state FOLDER",
            "where the history and its pictures are kept",
        )
        .option(
            "--levels FILE",
            "each sender's level, as home.nix writes it",
        )
        .option("--senders FILE", "the desktop's own senders, senders.json")
        .option(
            "--quiet FILE",
            "Quiet.qml's file, saying whether it's quiet",
        )
        .option(
            "--vocabulary FILE",
            "the launcher's words, which rows are said in",
        )
        .option(
            "--marks FILE",
            "each program's mark, as home.nix writes them",
        )
        .option("--addressed FILE", "what's to Joel from each chat program")
        .option(
            "--shells FOLDER",
            "the shell's history, taken the first time",
        )
        .exit(Exit::Failed, "its history, the bus or its socket failed it")
        .exit(Exit::Config, "its levels or senders couldn't be read")
}

/// Where it serves, keeps and reads, from what its unit hands it.
///
/// # Errors
///
/// Asked wrongly, 64, for a flag it doesn't know, a flag with no value, a
/// word that isn't a flag's, or short of one it can't start without.
fn asked(args: &[String]) -> Result<Args, Failure> {
    let wrongly = |what: String| Failure::new(Exit::Usage, what).try_this("notices --help");
    let mut given: HashMap<&str, PathBuf> = HashMap::new();
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        let flag = match FLAGS.iter().find(|flag| **flag == arg) {
            Some(flag) => *flag,
            None if arg.starts_with('-') => return Err(flags::unknown("option", arg, &FLAGS)),
            None => return Err(wrongly(format!("notices takes no word like {arg}"))),
        };
        let value = args
            .next()
            .ok_or_else(|| wrongly(format!("{flag} needs a value")))?;
        given.insert(flag, PathBuf::from(value));
    }
    let missing: Vec<&str> = FLAGS[..NEEDED]
        .iter()
        .copied()
        .filter(|flag| !given.contains_key(flag))
        .collect();
    if !missing.is_empty() {
        return Err(wrongly(format!("notices needs {}", flags::and(&missing))));
    }
    // Each needed flag is there, since none is missing.
    let mut take = |flag: &str| given.remove(flag);
    Ok(Args {
        socket: take("--socket").unwrap_or_default(),
        state: take("--state").unwrap_or_default(),
        levels: take("--levels").unwrap_or_default(),
        senders: take("--senders").unwrap_or_default(),
        quiet: take("--quiet").unwrap_or_default(),
        vocabulary: take("--vocabulary").unwrap_or_default(),
        marks: take("--marks").unwrap_or_default(),
        addressed: take("--addressed"),
        shells: take("--shells"),
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
    /// The machine it runs on, by its host name, which each notice's cell
    /// is stamped with when its sender named none (notice.rs's Cell).
    host: String,
    rows: Rows,
    places: [Kept; 3],
    /// The notices Desktop showed when its rows were last made, kept there
    /// once read for as long as it's watched.
    looked: BTreeSet<u32>,
}

impl Served {
    /// Every record, its notices' and their rows, as they stand: each
    /// notice's with its card's sender and words beside it, of those
    /// still recent now (notice.rs's recent), while the rows are of every
    /// one. One passing a day done with goes at the next change, as one
    /// comes or is read, rather than on a clock of its own.
    fn records(&mut self, holding: &Holding, signs: &Signs, desktop_watched: bool) -> Vec<Value> {
        let now = now_ms();
        let looked = if desktop_watched {
            std::mem::take(&mut self.looked)
        } else {
            BTreeSet::new()
        };
        let made = self.rows.make(holding.notices().iter(), signs, &looked);
        self.looked = row::shown(&made[1]);
        for (kept, (rows, beside)) in self.places.iter_mut().zip(row::served(made)) {
            kept.take(rows, beside);
        }
        let mut records: Vec<Value> = holding
            .notices()
            .iter()
            .filter(|notice| notice.recent(now))
            .map(|notice| {
                let mut record = notice.record(&self.host);
                if let Some(card) = self.rows.card(notice.id).and_then(Value::as_object) {
                    for (field, value) in card {
                        record[field] = value.clone();
                    }
                }
                record
            })
            .collect();
        records.push(front::record(holding.notices()));
        records.extend(self.places.iter().flat_map(Kept::all).cloned());
        records
    }

    /// The paged places' rows their watches aren't served, by key, for a
    /// search or a `rows` ask to answer with: those read past each page,
    /// the mixed place's and History's.
    fn unserved(&self) -> BTreeMap<String, Value> {
        let mixed = self.rows.mixed().map(|row| (row, &self.places[0]));
        let history = self.rows.history().map(|row| (row, &self.places[2]));
        mixed
            .chain(history)
            .filter_map(|(row, kept)| {
                let key = row["key"].as_str()?;
                kept.get(key)
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
    terminal_kit::main(&help(), env!("CARGO_PKG_VERSION"), |context| {
        serve_notices(&asked(&context.args)?)
    })
}

/// The machine's host name, as the kernel holds it as the daemon starts,
/// or "" where it can't be read, which stamps no machine. Said once in
/// the journal when it stamps none, so links with no machine have a why.
fn this_host() -> String {
    let host = std::fs::read_to_string("/proc/sys/kernel/hostname")
        .map(|name| name.trim().to_owned())
        .unwrap_or_default();
    if host.is_empty() {
        eprintln!("notices: no host name to stamp a notice's cell with");
    }
    host
}

/// Takes the bus and serves the shell for as long as it runs.
///
/// # Errors
///
/// Its configuration, 78, where its levels or senders can't be read;
/// failed, 1, where its history, the bus or its socket fails it.
fn serve_notices(args: &Args) -> Result<(), Failure> {
    let kept = args.state.join("notices.json");
    let judge = judge(args).map_err(|why| {
        Failure::new(Exit::Config, "notices couldn't read its levels or senders").because(why)
    })?;
    let holding = holding(args, &kept).map_err(|why| {
        Failure::new(Exit::Failed, "notices couldn't read its history").because(why)
    })?;
    // Written at once, so the shell's history, where it was taken, is
    // read only the once.
    if let Err(why) = store::write(&kept, &holding) {
        eprintln!("notices: can't write {}: {why}", kept.display());
    }
    let (senders, marks, words) = signing(args);
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
    let mut served = Served {
        host: this_host(),
        ..Served::default()
    };
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
    // The first records, the history kept, are made before the socket is
    // listened on, or under systemd before what it handed over is accepted
    // on: an ask answered before this making, a watch or a get, was told
    // no records at all, the history as if it were empty, for as long as
    // taking the bus name took (bus.rs's
    // one_done_with_a_day_ago_is_historys_row_but_no_record_the_shell_watches,
    // failing on a builder, 2026-11-21). A notice sent on the bus is kept
    // in `state` and told by the loop below, whenever it comes.
    let first = records(&mut state.lock().unwrap_or_else(PoisonError::into_inner));
    shared.change(first, json!({}));
    let listener = serve::listen(&args.socket).map_err(|why| {
        let socket = args.socket.display();
        Failure::new(Exit::Failed, format!("notices couldn't listen on {socket}"))
            .because(why.to_string())
    })?;
    serve::accept(Arc::clone(&shared), listener);
    let asker = changed.clone();
    let connection = bus::serve(Server {
        state: Arc::clone(&state),
        changed,
    })
    .map_err(|why| Failure::new(Exit::Failed, "notices couldn't take the bus").because(why))?;
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
    Ok(())
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
