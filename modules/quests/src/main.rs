//! The quests daemon: follows the node's two lists of quests, the open
//! and the ended, and its sessions, makes Quests' rows from all of it in
//! one loop, and serves the open ones' to the launcher over the socket
//! systemd holds for it, each watch told only the rows that moved. A
//! search finds among every quest, ended ones too (search.rs). Beside
//! them it says what moved for each open quest since Joel last looked
//! (moved.rs), from the asks file, until `looked` gives back what he was
//! shown of it.
//!
//! Its unit alone starts it, in the terminal kit's shape, as
//! `quests --help` says (`help()`): `--vocabulary` is the launcher's
//! `words.json` and `--node` the mycelium node's socket; `--asks` and
//! `--looked`, given together or not at all, the asks file and where what
//! he last looked at is kept. A quest with no folder of its own starts in
//! `$HOME`.

use std::fs::{self, File, TryLockError};
use std::io;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::{Arc, Mutex, PoisonError, mpsc};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use daemon_serve::asks::Domain;
use daemon_serve::serve::{self, Own, Shared};
use daemon_serve::watch::{Held, follow};
use quests::folders;
use quests::moved::{self, Ask, Looked};
use quests::quest::Quest;
use quests::search::Searched;
use quests::served::{self, Inputs};
use quests::words::Words;
use ready_rows::kept::Kept;
use serde_json::{Value, json};
use terminal_kit::{Exit, Failure, Help, flags};

/// The daemon as the launcher knows it: Quests' ready rows, how many are
/// open, and what moved.
const DOMAIN: Domain = Domain {
    name: "quests",
    kinds: &["row", "summary", "folders", moved::KIND],
    sole: &[],
    named_only: &[],
};

/// The flags its unit hands it, each with a value: the first three
/// always, the last two together where the machine keeps asks.
const FLAGS: [&str; 5] = ["--vocabulary", "--node", "--socket", "--asks", "--looked"];

/// How often the asks file is looked at for a change: a stat, which costs
/// nothing, and an ask's landing said within the time it takes Joel to
/// open the launcher.
const ASKS_EVERY: Duration = Duration::from_secs(2);

/// How many looks running the asks file may go unread before it's said:
/// six seconds, past any moment its keeper holds the lock to write.
const UNREAD_SAID: u32 = 3;

/// Something heard, of the node's watches, the asks file or the launcher.
enum Heard {
    Open(Value),
    Ended(Value),
    Sessions(Value),
    Asks(Vec<Ask>),
    /// He looked, and what he was shown is kept.
    Looked,
}

/// Where it reads and serves.
struct Args {
    vocabulary: PathBuf,
    node: PathBuf,
    socket: PathBuf,
    /// The asks file and where what he last looked at is kept.
    asks: Option<(PathBuf, PathBuf)>,
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
        .option("--asks FILE", "what's asked of each quest, with --looked")
        .option("--looked FILE", "where it keeps what Joel last looked at")
        .exit(Exit::Failed, "its socket couldn't be listened on")
        .exit(Exit::Usage, "asked other than its unit asks")
        .exit(Exit::Config, "the launcher's words couldn't be read")
}

/// Where it reads and serves, from what its unit hands it.
fn asked(args: &[String]) -> Result<Args, Failure> {
    let wrongly = |what: String| Failure::new(Exit::Usage, what).try_this("quests --help");
    let mut given: [Option<PathBuf>; 5] = Default::default();
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        let Some(at) = FLAGS.iter().position(|flag| flag == arg) else {
            if arg.starts_with('-') {
                return Err(flags::unknown("option", arg, &FLAGS));
            }
            return Err(wrongly(format!("quests takes no word like {arg}")));
        };
        let value = args
            .next()
            .ok_or_else(|| wrongly(format!("{arg} needs a value")))?;
        given[at] = Some(PathBuf::from(value));
    }
    let [vocabulary, node, socket, asks, looked] = given;
    let missing: Vec<&str> = FLAGS[..3]
        .iter()
        .copied()
        .zip([&vocabulary, &node, &socket])
        .filter_map(|(flag, value)| value.is_none().then_some(flag))
        .collect();
    let asks = match (asks, looked) {
        (Some(asks), Some(looked)) => Some((asks, looked)),
        (None, None) => None,
        _ => {
            return Err(wrongly(
                "quests needs --asks and --looked together".to_owned(),
            ));
        }
    };
    match (vocabulary, node, socket) {
        (Some(vocabulary), Some(node), Some(socket)) => Ok(Args {
            vocabulary,
            node,
            socket,
            asks,
        }),
        _ => Err(wrongly(format!("quests needs {}", flags::and(&missing)))),
    }
}

fn main() -> ExitCode {
    terminal_kit::main(&help(), env!("CARGO_PKG_VERSION"), |context| {
        serve_quests(&asked(&context.args)?)
    })
}

/// What Joel last looked at, as kept: none where nothing is yet, so the
/// first asks heard are taken as shown and a first start says nothing
/// until something moves. One that can't be read, or isn't what the
/// daemon keeps, is said and taken as nothing looked at, so every ask is
/// said once rather than whatever moved meanwhile hidden as shown.
fn looked_kept(kept: &Path) -> Option<Looked> {
    let text = match fs::read_to_string(kept) {
        Ok(text) => text,
        Err(why) if why.kind() == io::ErrorKind::NotFound => return None,
        Err(why) => {
            eprintln!(
                "quests: {} couldn't be read, so every ask is said: {why}",
                kept.display()
            );
            return Some(Looked::new());
        }
    };
    moved::looked_from(&text).or_else(|| {
        eprintln!(
            "quests: {} isn't what was looked at, so every ask is said",
            kept.display()
        );
        Some(Looked::new())
    })
}

/// The moment now, in milliseconds since the epoch, which the next date
/// still owed is told from.
fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| {
            i64::try_from(since.as_millis()).unwrap_or(i64::MAX)
        })
}

/// Keeps what he looked at, written whole and moved into place, so a
/// crash mid-write leaves what was kept before rather than none.
fn keep_looked(kept: &Path, looked: &Looked) -> io::Result<()> {
    let next = kept.with_extension("next");
    if let Some(folder) = kept.parent() {
        fs::create_dir_all(folder)?;
    }
    fs::write(&next, moved::kept(looked))?;
    fs::rename(&next, kept)
}

/// The asks file's text, read under a shared lock on `<file>.lock`, which
/// its keeper takes alone (a flock) as it writes the file anew and moves
/// it into place, so a file half written is never read. A lock the keeper
/// holds is tried again at the next look rather than waited on, so a
/// keeper stuck holding it is said rather than freezing What Moved. A
/// byte that isn't UTF-8 spoils only its own line, which isn't JSON.
fn read_asks(asks: &Path) -> io::Result<String> {
    let lock = asks.with_extension("jsonl.lock");
    // No lock file is a keeper that never wrote, whose file is whole.
    let held = File::open(&lock).ok();
    if let Some(lock) = &held {
        match lock.try_lock_shared() {
            Ok(()) => {}
            Err(TryLockError::WouldBlock) => {
                return Err(io::Error::new(
                    io::ErrorKind::WouldBlock,
                    "its keeper holds its lock",
                ));
            }
            Err(TryLockError::Error(why)) => return Err(why),
        }
    }
    let text = fs::read(asks).map(|bytes| String::from_utf8_lossy(&bytes).into_owned());
    if let Some(lock) = &held {
        let _ = lock.unlock();
    }
    text
}

/// Watches the asks file, telling its asks once at the start and again
/// each time its size or the moment it was written changes: none while
/// it isn't there, and the asks told last kept while it can't be read,
/// read again at the next look. Unread for `UNREAD_SAID` looks running, it
/// says so once, since a keeper writing holds the lock for a moment.
fn watch_asks(asks: PathBuf, to: mpsc::Sender<Heard>) {
    std::thread::spawn(move || {
        let mut was = None;
        let mut unread = 0_u32;
        loop {
            let now = fs::metadata(&asks)
                .ok()
                .map(|m| (m.len(), m.modified().ok()));
            if was != Some(now) {
                let text = if now.is_some() {
                    read_asks(&asks)
                } else {
                    Ok(String::new())
                };
                match text {
                    Ok(text) => {
                        was = Some(now);
                        unread = 0;
                        if to.send(Heard::Asks(moved::asks(&text))).is_err() {
                            return;
                        }
                    }
                    Err(why) => {
                        unread = unread.saturating_add(1);
                        if unread == UNREAD_SAID {
                            eprintln!("quests: {} couldn't be read: {why}", asks.display());
                        }
                    }
                }
            }
            // A stat every ASKS_EVERY, since the file's writers are other
            // programs' and a missing file has nothing to watch.
            std::thread::sleep(ASKS_EVERY);
        }
    });
}

/// Takes the first asks heard as shown where nothing was kept of what he
/// looked at, and keeps that, so a first start says nothing of the asks
/// already there: once, as `due` says, whatever a look gave meanwhile. An
/// asks file not there yet, or holding none, takes nothing, so the asks
/// it comes to hold aren't each said as taken on.
fn first_shown(seen: &Mutex<Option<Looked>>, asks: &[Ask], kept: Option<&Path>, due: &mut bool) {
    let Some(kept) = kept.filter(|_| *due && !asks.is_empty()) else {
        return;
    };
    *due = false;
    let mut held = seen.lock().unwrap_or_else(PoisonError::into_inner);
    let mut shown = held.clone().unwrap_or_default();
    shown.extend(moved::shown(asks));
    if let Err(why) = keep_looked(kept, &shown) {
        eprintln!("quests: what's shown at the start couldn't be kept: {why}");
    }
    *held = Some(shown);
}

/// Lets go of what he was shown of asks no longer in the file, put down
/// or let go by its keeper, so what's kept is only what can still move.
/// An asks file holding none lets go of nothing, as one not there yet.
fn let_go(seen: &Mutex<Option<Looked>>, asks: &[Ask], kept: Option<&Path>) {
    let Some(kept) = kept.filter(|_| !asks.is_empty()) else {
        return;
    };
    let mut held = seen.lock().unwrap_or_else(PoisonError::into_inner);
    let Some(looked) = held.as_mut() else {
        return;
    };
    let before = looked.len();
    looked.retain(|key, _| asks.iter().any(|ask| &ask.key == key));
    if looked.len() != before
        && let Err(why) = keep_looked(kept, looked)
    {
        eprintln!("quests: what was looked at couldn't be kept: {why}");
    }
}

/// Answers `looked`: what he was shown, given back as the launcher was
/// served it, is kept and taken as looked at, so what moves after goes on
/// being said. Nothing is taken unless it's kept, so a restart never says
/// again what he was told was looked at.
fn looked(said: &Value, kept: &Path, looked: &Mutex<Option<Looked>>) -> Value {
    let Some(seen) = moved::seen_in(&said["seen"]) else {
        return json!({"error": {"code": "bad-input",
            "message": "looked needs seen, a list of each ask's key, landed and expected"}});
    };
    let mut held = looked.lock().unwrap_or_else(PoisonError::into_inner);
    let mut next = held.clone().unwrap_or_default();
    next.extend(seen);
    if let Err(why) = keep_looked(kept, &next) {
        return json!({"error": {"code": "failed",
            "message": format!("what was looked at couldn't be kept: {why}")}});
    }
    *held = Some(next);
    json!({"done": true})
}

/// The asks the daemon answers itself: a search and titles among every
/// quest, and `looked`, which keeps what he was shown and tells the loop.
fn own_asks(
    searched: Arc<Mutex<Searched>>,
    kept_looked: Option<PathBuf>,
    seen: Arc<Mutex<Option<Looked>>>,
    to: mpsc::Sender<Heard>,
) -> Own {
    let told = Mutex::new(to);
    Box::new(move |said| {
        let searched = || searched.lock().unwrap_or_else(PoisonError::into_inner);
        let mut answer = match said["ask"].as_str() {
            Some("search") => searched().answer(said),
            Some("titles") => searched().titles(said),
            // He's looked at what moved: what he was shown goes until it
            // moves again.
            Some("looked") => match &kept_looked {
                Some(kept) => {
                    let answer = looked(said, kept, &seen);
                    if answer.get("done").is_some() {
                        let _ = told
                            .lock()
                            .unwrap_or_else(PoisonError::into_inner)
                            .send(Heard::Looked);
                    }
                    answer
                }
                None => json!({"error": {"code": "no-asks",
                    "message": "this machine keeps no asks"}}),
            },
            _ => return None,
        };
        answer["id"] = said["id"].clone();
        Some(answer)
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
    let (to, heard) = mpsc::channel();
    // What a search finds among, every quest as last made, shared with
    // the asks each connection answers.
    let searched = Arc::new(Mutex::new(Searched::default()));
    let kept_looked = args.asks.as_ref().map(|(_, looked)| looked.clone());
    // What he last looked at, kept, shared with the asks that give it.
    let seen = Arc::new(Mutex::new(kept_looked.as_deref().and_then(looked_kept)));
    let own = own_asks(
        Arc::clone(&searched),
        kept_looked,
        Arc::clone(&seen),
        to.clone(),
    );
    let shared = Shared::new(DOMAIN, own);
    let listener = serve::listen(&args.socket).map_err(|why| {
        let socket = args.socket.display();
        Failure::new(Exit::Failed, format!("it couldn't listen on {socket}"))
            .because(why.to_string())
    })?;
    serve::accept(Arc::clone(&shared), listener);
    follow(args.node.clone(), "quest", Heard::Open, to.clone());
    follow(args.node.clone(), "ended-quest", Heard::Ended, to.clone());
    follow(args.node.clone(), "session", Heard::Sessions, to.clone());
    if let Some((asks, _)) = &args.asks {
        watch_asks(asks.clone(), to);
    }
    let (mut open, mut ended, mut sessions) = (Held::default(), Held::default(), Held::default());
    let mut asks: Vec<Ask> = Vec::new();
    let mut kept = Kept::default();
    let looked_at = args.asks.as_ref().map(|(_, kept)| kept.as_path());
    // Whether the first asks are still to be taken as shown: only where
    // nothing was kept of what he looked at.
    let mut baseline = seen
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .is_none();
    while let Ok(first) = heard.recv() {
        // Everything said meanwhile is taken before the rows are made, so
        // the node writing both lists at once makes the rows once.
        let mut moved = false;
        for one in std::iter::once(first).chain(heard.try_iter()) {
            moved |= match one {
                Heard::Open(answer) => !open.took(&answer).is_empty(),
                Heard::Ended(answer) => !ended.took(&answer).is_empty(),
                Heard::Sessions(answer) => !sessions.took(&answer).is_empty(),
                Heard::Asks(now) => {
                    first_shown(&seen, &now, looked_at, &mut baseline);
                    std::mem::replace(&mut asks, now) != asks
                }
                Heard::Looked => {
                    let_go(&seen, &asks, looked_at);
                    true
                }
            };
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
        let quests: Vec<Quest> = open.records().map(|(_, record)| Quest(record)).collect();
        let beside = vec![
            served::summary_record(&made.said),
            folders::record(&made.open, &home),
            moved::record(
                &asks,
                &seen
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .clone()
                    .unwrap_or_default(),
                &quests,
                &words,
                now_ms(),
            ),
        ];
        if !kept.take(made.open, beside).is_empty() {
            shared.change(kept.all().cloned().collect(), json!({}));
        }
    }
    Ok(())
}
