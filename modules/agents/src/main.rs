//! The agents daemon: follows what each source says of Joel's sessions,
//! makes Agents' rows from all of it in one loop, and serves them to the
//! launcher over the socket systemd holds for it, each watch told only
//! the rows that moved; and does a row's act as the shell asks by it,
//! steering its session through the node (acts.rs); and serves the
//! sessions there are to reopen, keeping what Joel says of them
//! (reopen.rs), what the sessions landed, keeping what of it he has
//! read (landed.rs), the closed orchestrators (closed.rs), and what the
//! sessions answered of his talks, saying where he read it (answered.rs);
//! and says while agents' starts are stopped, letting them start again
//! as the shell asks (`starts_stopped.rs`).
//!
//! Usage: `agents --vocabulary WORDS [--node NODE] --to-do TODO --windows
//! WINDOWS --spend SPEND --orchestrators ORCHESTRATORS --restore RESTORE
//! --kept KEPT --landed LANDED --steer STEER --shells SHELLS [--switched
//! SWITCHED --clone CLONE] [--harness HARNESS] [--switches SWITCHES]
//! --socket SOCKET`, where
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
//! landings; SWITCHES the
//! integration's record of switches, a line each, which says when each
//! of core's merges reached the machine. A machine with no realm runs no node,
//! so it's started with none, and serves the sessions to reopen alone.
//! Its help, its failures and its exits take the terminal kit's shape
//! (`help()`).

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::rc::Rc;
use std::sync::mpsc::RecvTimeoutError;
use std::sync::{Arc, mpsc};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use terminal_kit::{Exit, Failure, Help};

use agents::acts;
use agents::arrangement::{Arrangement, Still};
use agents::asker;
use agents::cells;
use agents::clock;
use agents::feedback::{self, Landings};
use agents::inputs::Inputs;
use agents::kept_places::KeptPlaces;
use agents::landed::Reading;
use agents::offers;
use agents::places;
use agents::reopen::Kept;
use agents::row::Words;
use agents::said::Said;
use agents::served::Served;
use agents::session::Session;
use agents::sources::{self, Heard, Paths};
use agents::sprout;
use agents::starts::{self, Keeping, Starts};
use agents::starts_stopped;
use agents::tell::{self, Routes};
use agents::wants::{self, PUT_OFF_FOR_MS, PutOff, Wants};
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
        "talks",
        cells::KIND,
        starts::KIND,
    ],
    sole: &[],
    named_only: &["landed-drawer"],
};

/// What `agents --help` says: each file, socket and folder it reads,
/// as its unit names them.
fn help() -> Help {
    Help::new("agents", "makes Agents' rows ready for the launcher")
        .example(
            "agents --vocabulary FILE --socket FILE",
            "and the rest, as its unit runs it",
        )
        .option("--vocabulary FILE", "the launcher's words")
        .option("--node SOCKET", "mycelium's node, where there's one")
        .option("--to-do SOCKET", "the to-do daemon's")
        .option("--windows SOCKET", "the windows daemon's")
        .option("--spend FILE", "spend-watch's collection")
        .option("--orchestrators FILE", "the harness's orchestrators")
        .option("--restore FILE", "the sessions there are to reopen")
        .option("--kept FILE", "what was said of those, its own")
        .option("--landed FILE", "the landings read, its own")
        .option("--steer FOLDER", "the sessions' inboxes")
        .option("--shells FOLDER", "each shell's own state")
        .option("--switched FILE", "what of core runs, with --clone")
        .option("--clone FOLDER", "core's bare clone, with --switched")
        .option("--harness FOLDER", "where the harness is installed")
        .option("--switches FILE", "the switches made, with --clone")
        .option("--socket FILE", "where it serves the rows")
        .exit(Exit::Failed, "it couldn't serve or hear its sources")
        .exit(Exit::Config, "the vocabulary couldn't be read")
}

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
    switches: Option<PathBuf>,
    socket: PathBuf,
}

/// What it was started with, every path named.
///
/// # Errors
///
/// Asked wrongly, 64, naming the first flag it needs and wasn't given.
fn args(args: &[String]) -> Result<Args, Failure> {
    let named = |flag: &str| {
        let at = args.iter().position(|one| one == flag)?;
        args.get(at + 1).map(PathBuf::from)
    };
    let needed = |flag: &str| {
        named(flag).ok_or_else(|| {
            Failure::new(Exit::Usage, format!("agents needs {flag}")).try_this("agents --help")
        })
    };
    Ok(Args {
        vocabulary: needed("--vocabulary")?,
        node: named("--node"),
        to_do: needed("--to-do")?,
        windows: needed("--windows")?,
        spend: needed("--spend")?,
        orchestrators: needed("--orchestrators")?,
        restore: needed("--restore")?,
        kept: needed("--kept")?,
        landed: needed("--landed")?,
        steer: needed("--steer")?,
        shells: needed("--shells")?,
        switched: named("--switched"),
        clone: named("--clone"),
        harness: named("--harness"),
        switches: named("--switches"),
        socket: needed("--socket")?,
    })
}

/// What the shell says for Joel, beside what the node takes as his
/// ([`as_joel`]), each answered only for the shell, as it says what he
/// sees and read: handed `told` to the daemon's loop, and Not Now puts
/// off the one of `wants` its session asks.
fn own(told: mpsc::Sender<Heard>, wants: Wants) -> Own {
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
        // Joel's Not Now on the ask a session's row showed him, refused
        // once it asks anew: kept for an hour, which the want's record
        // says, ringing for nothing.
        wants.put_off(said).map(|named| match named {
            Ok(known) => {
                let _ = told.send(Heard::PutOffWant(known.clone(), now));
                json!({"putOff": known, "forMs": u64::try_from(PUT_OFF_FOR_MS).unwrap_or(0)})
            }
            Err(why) => json!({"error": {"code": "not-an-ask", "message": why}}),
        })
    })
}

/// What Joel does through the daemon that the node takes as his, answered
/// only for the shell ([`for_the_shell_alone`]): a row's act, done by
/// asking the node at `node`, refused where there's none, a line he
/// tells whoever `routes` says hears it, and his feedback on one of
/// `landings`, kept in the file at `kept`, whose talk is handed `told`.
fn as_joel(
    told: mpsc::Sender<Heard>,
    node: Option<PathBuf>,
    routes: Routes,
    landings: Landings,
    kept: PathBuf,
) -> Own {
    Box::new(move |said| {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |since| since.as_millis());
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
        acts::steer_of(said, now).map(|steer| match &node {
            Some(node) => acts::done(steer, node),
            None => json!({"error": {"code": "unsteered",
                "message": "this machine runs no node to steer it through"}}),
        })
    })
}

/// Start New Work, where it can begin, whether the cloud can take it, Let
/// Them Start, an answer to what a session asks and all Joel's (`as_joel`: a row's act, a line he
/// tells, his feedback, and what the shell says he saw, read, put off or
/// forgot), asked of `node` only for the shell, told apart by the
/// connection each came on, so no agent lifts the stop Joel put on their
/// starts, reads his places, answers one of `wants`, steers, tells or
/// talks to a session as him, nor hides from him what he'd see; what the
/// node says of the starts once they're let start is `stood` at once,
/// rather than at the next poll. Each but which are kept asks the shell's
/// systemd who the shell is, the first time a shell asks (asker.rs's
/// `TAKEN`); Joel's acts are answered in the order he did them, and the
/// rest of [`SLOW`] on threads of their own, holding up no ask after them. Each
/// start the shell names by a
/// token is kept on disk before the node is asked, its end before the
/// shell hears it, and its told before that's answered, until the shell
/// says it told Joel its end (starts.rs), so neither a restart of the
/// shell nor one of this daemon ends it untold or tells it twice. The
/// loop is told each change, to serve the starts' records anew.
fn for_the_shell_alone(
    shared: &Shared,
    (node, wants): (Option<PathBuf>, Wants),
    keeping: Keeping,
    told: mpsc::Sender<Heard>,
    (stood, words): (Arc<starts_stopped::Told>, Said),
    as_joel: Own,
) {
    shared.peered(
        &SHELLS_ALONE,
        Box::new(move |said, stream| {
            if said["ask"]
                .as_str()
                .is_some_and(|ask| AS_JOEL.contains(&ask))
            {
                return match asker::the_shell(stream) {
                    Ok(()) => as_joel(said),
                    Err(why) => Some(not_as_joel(&why, said)),
                };
            }
            if said["ask"] == places::ASK {
                return Some(places::answer(|| asker::the_shell(stream), node.as_deref()));
            }
            if said["ask"] == offers::ASK {
                return Some(offers::answer(|| asker::the_shell(stream), node.as_deref()));
            }
            if said["ask"] == wants::ASK {
                let asker = asker::the_shell(stream);
                return Some(answered(asker, &wants, said, node.as_deref()));
            }
            if said["ask"] == starts_stopped::ASK {
                let (answer, now) =
                    starts_stopped::answer(asker::the_shell(stream), node.as_deref(), &words);
                if let Some(now) = now {
                    stood.tell(now);
                }
                return Some(answer);
            }
            // Whether the change reached the disk: a start or its end is
            // answered either way, the shell being told as it would be with
            // no file, but a told isn't taken, so the shell sends it again
            // rather than a restart serving it a second time.
            let keep = |change: &dyn Fn(&mut Starts)| {
                let kept = keeping.change(change);
                if let Err(why) = &kept {
                    eprintln!("agents: can't keep {}: {why}", starts::FILE);
                }
                let _ = told.send(Heard::StartsMoved);
                kept
            };
            if said["ask"] == starts::KEPT {
                return Some(json!({"kept": keeping.kept(&starts::told_of(said))}));
            }
            let asker = asker::the_shell(stream);
            if said["ask"] == starts::TOLD {
                return Some(match asker {
                    Ok(()) => {
                        let tokens = starts::told_of(said);
                        match keep(&|kept: &mut Starts| kept.told(&tokens)) {
                            Ok(()) => json!({"told": tokens.len()}),
                            Err(why) => json!({"error": {"code": "unkept",
                                "message": format!("can't keep {}: {why}", starts::FILE)}}),
                        }
                    }
                    Err(why) => json!({"error": {"code": "untold", "message": why}}),
                });
            }
            // Only the shell's are kept, so no other program fills the file.
            let token = asker.as_ref().ok().and_then(|()| starts::token_of(said));
            if let Some(token) = &token {
                let at = clock::now_ms();
                let _ = keep(&|kept: &mut Starts| kept.begun(token, at));
            }
            let answer = sprout::answer(said, asker, node.as_deref());
            if let Some(token) = &token {
                let _ = keep(&|kept: &mut Starts| kept.ended(token, &answer));
            }
            Some(answer)
        }),
    );
    shared.slow(&SLOW);
}

/// What the daemon is asked as Joel's: what the node takes as his, a
/// row's act (acts.rs), a line he tells and his words on a landing; and
/// what the shell says of him (`own`), a Not Now, a put-off or forgotten
/// reopening, a landing read, a drawer opened, a shared screen and his
/// windows' places. Said by an agent, the first would steer a session as
/// him, and the rest would hide from him what he should see, or show a
/// shared screen what he'd keep off it (ISXEG8, 26 November).
const AS_JOEL: [&str; 15] = [
    "pause",
    "resume",
    "stop",
    "close",
    "wake",
    "talk",
    "tell",
    "feedback",
    "not-now",
    "put-off",
    "forget",
    "read",
    "heard",
    "sharing",
    "keep-places",
];

/// Every ask the daemon answers only as it's told who asked
/// (`for_the_shell_alone`).
const SHELLS_ALONE: [&str; 22] = [
    sprout::ASK,
    places::ASK,
    offers::ASK,
    starts::TOLD,
    starts::KEPT,
    starts_stopped::ASK,
    wants::ASK,
    "pause",
    "resume",
    "stop",
    "close",
    "wake",
    "talk",
    "tell",
    "feedback",
    "not-now",
    "put-off",
    "forget",
    "read",
    "heard",
    "sharing",
    "keep-places",
];

/// Which of [`SHELLS_ALONE`] are answered each on a thread of its own: a
/// start, which waits on the node a long while, its places, whether the
/// cloud can take it, a start told,
/// an answer, which waits on the node up to acts.rs's `PATIENCE` and so
/// queued could come back after the Talk lets a second go
/// (TalkAnswers.js's `SENT_STANDS_MS`), and feedback, which waits on the
/// quest it goes to. Joel's other acts are answered in the order the
/// shell asks them, on its connection's thread, as each but feedback was
/// before the shell's check: a share's start and end, or the whole of his
/// windows' places as each is kept, answered apart could land out of
/// order, and the daemon keep the one he'd left (round 000056 on #6796,
/// F16 and F30).
const SLOW: [&str; 7] = [
    sprout::ASK,
    places::ASK,
    offers::ASK,
    starts::TOLD,
    starts_stopped::ASK,
    wants::ASK,
    "feedback",
];

/// What one of [`AS_JOEL`] is told where the shell's check refused it,
/// in the form each is read: an act as the shell reads a steer
/// refused (Sessions.qml's steered), journalled with what it named, so
/// an agent acting as Joel can be traced to it.
fn not_as_joel(why: &str, said: &Value) -> Value {
    eprintln!("{}", asker::refused_line(why, said));
    json!({"error": {"code": "refused", "message": NOT_AS_JOEL}})
}

/// Why one of [`AS_JOEL`] the shell's check refused wasn't done, which
/// Joel reads where his own act was refused because the check couldn't
/// be made.
const NOT_AS_JOEL: &str =
    "the agents daemon acts for you only from the launcher, and couldn't tell this came from it";

/// What an answer the shell's check refused is told, as Let Them Start
/// says its own. Joel reads it where his own Return was refused because
/// the check couldn't be made, so it says what the daemon couldn't tell
/// rather than who may answer.
const ONLY_THE_SHELL: &str =
    "the agents daemon takes an answer only from the launcher, and couldn't tell this came from it";

/// Joel's answer to one of `wants`, steered through `node` only where
/// `asker` says the shell asked it: the node takes it as his, so an
/// agent answering its own ask through this daemon would answer as him
/// (ISXEG8's R-agent-answers-own-ask).
fn answered(asker: Result<(), String>, wants: &Wants, said: &Value, node: Option<&Path>) -> Value {
    if let Err(why) = asker {
        // Why it was refused is journalled with what it named, beside the
        // line asker.rs writes of who asked, so an agent answering its own
        // ask can be traced to it. What it named is the asker's own, cut
        // short, so no ask writes more than a line.
        eprintln!(
            "agents: an answer to {} for {} wasn't taken: {}",
            asker::named(&said["want"]),
            asker::named(&said["address"]),
            asker::journalled(&why)
        );
        return json!({"error": {"code": "refused", "message": ONLY_THE_SHELL}});
    }
    // Only an answer reaches here, so the wants always have one to give.
    wants
        .answer(said, node, clock::now_ms())
        .unwrap_or_else(|| json!({"error": {"code": "not-an-ask", "message": "not an answer"}}))
}

/// Follows the node: every kind of record it holds that Agents reads.
fn follow_node(node: &Path, to: &mpsc::Sender<Heard>) {
    sources::follow(node.to_owned(), "session", Heard::Sessions, to.clone());
    sources::follow(node.to_owned(), "job", Heard::Jobs, to.clone());
    sources::follow(node.to_owned(), "quest", Heard::Quests, to.clone());
    sources::follow(node.to_owned(), "talk", Heard::Talks, to.clone());
    sources::follow(node.to_owned(), "team", Heard::Teams, to.clone());
    sources::follow(node.to_owned(), "network", Heard::Networks, to.clone());
    sources::follow(node.to_owned(), "carry", Heard::Carries, to.clone());
    sources::follow(node.to_owned(), "landing", Heard::Landings, to.clone());
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

/// The sooner of two waits, either none.
fn soonest(one: Option<Duration>, other: Option<Duration>) -> Option<Duration> {
    match (one, other) {
        (Some(one), Some(other)) => Some(one.min(other)),
        (one, other) => one.or(other),
    }
}

/// Everything said, at least one, waiting on nothing but what's said
/// unless something waits for a time, places to be kept once the windows
/// are still or a want's hour to end; none once that wait ends, and an
/// error once nothing can be said any more.
fn hear(
    heard: &mpsc::Receiver<Heard>,
    still: &Still,
    put_off: &PutOff,
) -> Result<Vec<Heard>, RecvTimeoutError> {
    let wait = soonest(still.wait(Instant::now()), put_off.wait(clock::now_ms()));
    let first = match wait {
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
fn started(words: Words, paths: &Paths, starts: Keeping) -> Inputs {
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
        put_off: PutOff::read(
            &paths.kept.with_file_name(wants::PUT_OFF_FILE),
            clock::now_ms(),
        ),
        starts,
        ..Inputs::default()
    }
}

fn main() -> ExitCode {
    terminal_kit::main(&help(), env!("CARGO_PKG_VERSION"), |context| {
        serve(args(&context.args)?)
    })
}

/// The launcher's words at `path`, or 78 when they can't be read.
fn vocabulary(path: &Path) -> Result<Words, Failure> {
    std::fs::read_to_string(path)
        .map_err(|why| why.to_string())
        .and_then(|text| Words::from_vocabulary(&text).map_err(|why| why.to_string()))
        .map_err(|why| {
            Failure::new(
                Exit::Config,
                format!("agents can't read the vocabulary {}", path.display()),
            )
            .because(why)
        })
}

/// Serves the rows until the sources it hears from are gone.
///
/// # Errors
///
/// 78 when the vocabulary can't be read, and 1 when it can't listen on
/// its socket or hear the files it reads.
fn serve(args: Args) -> Result<(), Failure> {
    // Its judge is the walks' weak one, so a walks build that got installed
    // says so at every start rather than acting as Joel for any process
    // named as the shell, unsaid (round 000011 on #6796, F109).
    #[cfg(feature = "walks")]
    eprintln!(
        "agents: built for the niri walks, taking any process running quickshell's program \
         for the shell; no machine should run this build"
    );
    let words = vocabulary(&args.vocabulary)?;
    let (to, heard) = mpsc::channel();
    // What the poll and Let Them Start last told of agents' starts, so
    // each tells only what moved since either.
    let starts_told = Arc::new(starts_stopped::Told::new(to.clone()));
    let starts_said = words.starts.clone();
    let routes = Routes::default();
    let landings = Landings::default();
    let wants = Wants::default();
    // Kept beside what Joel has read, never rewritten, for the daily loop
    // that reads on from where it left off (80ECO2, 2026-11-11).
    let kept = args.landed.with_file_name(FEEDBACK);
    let acts = as_joel(
        to.clone(),
        args.node.clone(),
        routes.clone(),
        landings.clone(),
        kept,
    );
    let says = own(to.clone(), wants.clone());
    let as_joel: Own = Box::new(move |said| says(said).or_else(|| acts(said)));
    // Every ask of its own is Joel's, answered only for the shell
    // (`for_the_shell_alone`), so none is left to answer anyone else.
    let shared = Shared::new(DOMAIN, Box::new(|_| None));
    // Read before the socket listens, so a shell's first ask of which are
    // kept is answered from them, though no record is served yet.
    let keeping = Keeping::read(&args.kept.with_file_name(starts::FILE), clock::now_ms());
    for_the_shell_alone(
        &shared,
        (args.node.clone(), wants.clone()),
        keeping.clone(),
        to.clone(),
        (Arc::clone(&starts_told), starts_said),
        as_joel,
    );
    match serve::listen(&args.socket) {
        Ok(listener) => serve::accept(Arc::clone(&shared), listener),
        Err(why) => {
            return Err(Failure::new(
                Exit::Failed,
                format!("agents can't listen on {}", args.socket.display()),
            )
            .because(why.to_string()));
        }
    }
    if let Some(node) = args.node {
        follow_node(&node, &to);
        starts_stopped::follow(node.clone(), starts_told);
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
        switches: args.switches,
        landed: args.landed,
        steer: args.steer,
        shells: args.shells,
    };
    // Without them only talks not yet taken go unshown, so every row is
    // still served.
    if let Err(why) = sources::follow_inboxes(&paths.steer, to.clone()) {
        eprintln!(
            "agents: can't hear the sessions' inboxes, so talks not yet taken go unshown: {why}"
        );
    }
    sources::follow_files(&paths, to).map_err(|why| {
        Failure::new(Exit::Failed, "agents can't hear the files it reads").because(why.to_string())
    })?;
    let mut inputs = started(words, &paths, keeping);
    let mut served = Served::default();
    let mut held_landed = Rc::clone(served.landed());
    let mut still = Still::default();
    while let Ok(said) = hear(&heard, &still, &inputs.put_off) {
        for one in said {
            if one.moves_places() {
                still.moved(Instant::now());
            }
            inputs.hear(one, &paths);
        }
        inputs.keep_when_still(&mut still, &paths, Instant::now());
        let moved = served.take(inputs.made());
        // The landings are read again only once they were made again, and
        // only what moved is told: 4,997 of them, read and cloned at every
        // move, kept the daemon at 49% of a core (perf, 2026-11-16). Made
        // again while no served record moved, feedback holds them as they
        // were; the newest are kept even so, freeing the older set rather
        // than holding two of about 3 MB each.
        if !Rc::ptr_eq(&held_landed, served.landed()) {
            held_landed = Rc::clone(served.landed());
            if !moved.is_empty() {
                landings.hold(held_landed.iter());
            }
        }
        if !moved.is_empty() {
            wants.hold(served.all());
            shared.mend(moved.changed, &moved.gone, json!({}));
        }
        // A quest can change whom a line reaches with no row moving.
        routes.hold(
            inputs.sessions.records().map(|(_, one)| Session(one)),
            inputs.quests.records().map(|(_, one)| one),
            served.all(),
        );
    }
    Ok(())
}

// Its socket's asks, as `serve` serves them, in a file of their own
// beside this one, so neither passes 800 lines.
#[cfg(test)]
#[path = "main_tests.rs"]
mod sprouts;
