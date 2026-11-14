//! slack-unreads CREDENTIALS SOCKET WAITING [ORIGIN [PACE]]
//!
//! Slack's daemon: what in Slack is unread and what Joel read last, for
//! the shell's Now and Slack's program (lib's `unreads` says what it is
//! and how it's read), held on SOCKET as the shell's daemon interface
//! says (lib's `daemon`), for any watch to hear as it changes. Slack is
//! asked every half minute. Slack refusing the sign-in is said at once;
//! Slack out of reach only once two asks in a row have failed, so one
//! lost answer doesn't blank the count. With no sign-in in CREDENTIALS,
//! the harness's file, nothing is held and nothing is asked until there
//! is one. ORIGIN is Slack's and PACE, in milliseconds, half a minute,
//! unless a test names a stand-in and a quicker pace.
//!
//! In the same round, each message wanting Joel is read from its
//! conversation's history once its unreads change, and said done once
//! Slack's read mark passes it (lib's `messages`), so nothing more is
//! asked of Slack than once a round, and nothing while nothing comes.
//! The account says when those held are all there are, so one missing
//! after a restart is known read. A few more of the conversations he's
//! read are named each round too, so each one's read mark is said by its
//! name and Slack's own notices of it are read once he's read past them.
//!
//! While Slack refuses the sign-in, signing in again waits on Joel in
//! For You, through WAITING, a file of what waits on him as each
//! quest's is (lib's `sign_in_waits`): written once Slack refuses, and
//! taken away once Slack answers again or there's no sign-in, so it clears
//! itself. Slack out of reach leaves it as it stood, since that says
//! nothing of the sign-in.
//!
//! Run by systemd, it tells its watchdog after each ask, so one that hangs
//! is restarted; a count said "ok" never outlives the program that read
//! it, since the shell lets what it held go once the socket stops
//! answering. It searches Slack when Joel asks too, so nothing is started
//! to search.
//!
//! Given the launcher's vocabulary, its store path in `LAUNCHER_WORDS`
//! as the shell's own is, it serves each conversation as the row Slack's
//! Unread and Recent show (lib's `rows`); without it, or with one it
//! can't read, it says so and serves no rows, and the shell makes its
//! own.
//!
//! It takes the shape every command of ours does (docs/commands.md),
//! through the terminal kit: `--help` and `--version` answer without
//! serving anything, and a wrong command line exits 64. It writes nothing
//! to stdout, so `--format` says only how a failure is told.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use slack_people::messages::Messages;
use slack_people::rows::Words;
use slack_people::unreads::{
    self, Conversation, Counted, Counts, Kind, Said, State, link, sign_in_waits,
};
use slack_people::{Account, Refused, Slack};
use terminal_kit::{Exit, Failure, Help};

/// How often Slack is asked, unless a test says otherwise.
const EVERY: Duration = Duration::from_secs(30);

/// How many asks in a row may fail before Slack is said to be out of
/// reach.
const MISSES: u32 = 2;

/// How many conversations he's read are named each round beyond those
/// unread or recent, so their read marks can be said by name: a few
/// hundred are named within some minutes of a start, while each round
/// asks Slack at most twice this many more times, well within what
/// `conversations.info` and `users.info` allow a minute.
const NAMED_EACH_ROUND: usize = 10;

fn help() -> Help {
    Help::new(
        "slack-unreads",
        "what in Slack wants Joel, for the launcher",
    )
    .command(
        "CREDENTIALS SOCKET WAITING [ORIGIN [PACE]]",
        "serves what's unread on SOCKET, asking Joel through WAITING to sign in",
    )
    .example(
        "slack-unreads creds.json slack.sock waits.json",
        "serves Slack's unreads, as its unit does",
    )
}

fn main() -> ExitCode {
    terminal_kit::main(&help(), env!("CARGO_PKG_VERSION"), |context| {
        let (credentials, socket, waiting, rest) = match context.args.as_slice() {
            [credentials, socket, waiting, rest @ ..] if rest.len() <= 2 => {
                (credentials, socket, waiting, rest)
            }
            _ => {
                return Err(Failure::new(
                    Exit::Usage,
                    "slack-unreads takes CREDENTIALS, SOCKET and WAITING, then ORIGIN and PACE",
                )
                .try_this("slack-unreads --help"));
            }
        };
        let pace = match rest.get(1) {
            Some(pace) => Some(pace.parse().map_err(|_| {
                Failure::new(Exit::Usage, format!("PACE is milliseconds, not {pace}"))
                    .try_this("slack-unreads --help")
            })?),
            None => None,
        };
        serve(
            credentials,
            socket,
            Path::new(waiting),
            rest.first()
                .cloned()
                .unwrap_or_else(|| "https://slack.com".into()),
            pace.map_or(EVERY, Duration::from_millis),
        )
    })
}

/// Serves what in Slack wants Joel on `socket`, asking Slack every
/// `every` as the account signed in at `credentials`, until it's stopped.
fn serve(
    credentials: &str,
    socket: &str,
    waiting: &Path,
    origin: String,
    every: Duration,
) -> Result<(), Failure> {
    let daemon = slack_people::daemon::listen(
        Path::new(&socket),
        PathBuf::from(&credentials),
        origin.clone(),
    );
    if let Some(path) = std::env::var_os("LAUNCHER_WORDS") {
        match vocabulary(Path::new(&path)) {
            Ok(words) => daemon.words(words),
            Err(why) => eprintln!("<3>serving no rows, {}: {why}", path.display()),
        }
    }
    let slack = Slack::new(origin);
    let credentials = PathBuf::from(credentials);
    let mut names = HashMap::new();
    // Those Slack named nothing, so each is asked once a run.
    let mut nameless = HashSet::new();
    // Who each person is, shared with the daemon's searches.
    let mut messages = Messages::sharing(daemon.people());
    let mut misses = 0;
    loop {
        // The file is read each time, so signing in again from pi is
        // taken without a restart. Waiting on one is no hang, so the
        // watchdog is told then too.
        let Some(account) = Account::read(&credentials) else {
            daemon.say(None, millis());
            ask_joel(waiting, false);
            alive();
            std::thread::sleep(every);
            continue;
        };
        let checked = millis();
        let asked = slack.counts(&account).and_then(
            |Counts {
                 unread,
                 recent,
                 read,
             }| {
                let conversations = named(&slack, &account, &mut names, unread)?;
                let recent = named(&slack, &account, &mut names, recent)?;
                messages.heard(&slack, &account, &conversations, &read, checked)?;
                name_more(&slack, &account, &mut names, &mut nameless, &read)?;
                Ok((conversations, recent, unreads::seen(&read, &names)))
            },
        );
        // The messages wanting Joel are kept through Slack refusing or out
        // of reach, since none was seen read (lib's `daemon::records`), but
        // not said whole, since what came meanwhile isn't known.
        let none = |state| Said {
            state,
            conversations: Vec::new(),
            recent: Vec::new(),
            messages: messages.raised(),
            marks: messages.marks(),
            seen: Vec::new(),
            whole: false,
        };
        let said = match asked {
            Ok((conversations, recent, seen)) => {
                misses = 0;
                Some(Said {
                    state: State::Ok,
                    conversations,
                    recent,
                    messages: messages.raised(),
                    marks: messages.marks(),
                    seen,
                    whole: messages.whole(),
                })
            }
            Err(Refused::SignedOut) => Some(none(State::SignedOut)),
            Err(Refused::Unreachable(why)) => {
                eprintln!("<4>{why}");
                misses += 1;
                (misses >= MISSES).then(|| none(State::Unreachable))
            }
        };
        match said.as_ref().map(|said| said.state) {
            Some(State::SignedOut) => ask_joel(waiting, true),
            Some(State::Ok) => ask_joel(waiting, false),
            _ => {}
        }
        if let Some(said) = &said {
            daemon.say(Some(said), checked);
        }
        alive();
        std::thread::sleep(every);
    }
}

/// Tells systemd's watchdog it's still asking, when systemd runs it.
fn alive() {
    use std::os::linux::net::SocketAddrExt;
    use std::os::unix::net::{SocketAddr, UnixDatagram};
    let Some(socket) = std::env::var_os("NOTIFY_SOCKET") else {
        return;
    };
    let socket = socket.to_string_lossy().into_owned();
    // A name starting with @ is in the abstract namespace.
    let address = match socket.strip_prefix('@') {
        Some(name) => SocketAddr::from_abstract_name(name.as_bytes()),
        None => SocketAddr::from_pathname(&socket),
    };
    let told =
        address.and_then(|address| UnixDatagram::unbound()?.send_to_addr(b"WATCHDOG=1", &address));
    if let Err(error) = told {
        eprintln!("<4>couldn't tell systemd: {error}");
    }
}

/// Each conversation, named, each name asked once a run. One that wants
/// Joel that Slack won't name now is said as Slack's, so its count isn't
/// lost; any other is left out until it's named, so neither Unread nor
/// Recent lists a row nobody can tell apart. Both are asked again next
/// time. Slack too busy to name one is said once a round, not once a
/// conversation, so a first round naming dozens doesn't flood the
/// journal.
fn named(
    slack: &Slack,
    account: &Account,
    names: &mut HashMap<String, String>,
    counted: Vec<Counted>,
) -> Result<Vec<Conversation>, Refused> {
    let mut unnamed = None;
    let mut listed = Vec::new();
    for counted in counted {
        let name = match counted.kind {
            Kind::Thread => Some("Threads".to_owned()),
            _ => match names.get(&counted.id) {
                Some(name) => Some(name.clone()),
                None => match slack.called(account, &counted.id) {
                    Ok(name) if !name.is_empty() => {
                        names.insert(counted.id.clone(), name.clone());
                        Some(name)
                    }
                    Ok(_) => None,
                    Err(Refused::SignedOut) => return Err(Refused::SignedOut),
                    Err(Refused::Unreachable(why)) => {
                        unnamed.get_or_insert(why);
                        None
                    }
                },
            },
        };
        let name = match name {
            Some(name) => name,
            None if counted.kind.wants() => "Slack".to_owned(),
            None => continue,
        };
        listed.push(Conversation {
            link: link(&account.team, &counted),
            id: counted.id,
            name,
            kind: counted.kind,
            count: counted.count,
            at: counted.at,
        });
    }
    if let Some(why) = unnamed {
        eprintln!("<4>{why}");
    }
    Ok(listed)
}

/// Names a few more of the conversations he's read each round, the last
/// read first (lib's `unreads::unnamed`), so each one's read mark can be
/// said by the name Slack's own notices give it. One Slack names nothing
/// isn't asked again this run; Slack too busy ends this round's naming,
/// said once, and it carries on next round.
fn name_more(
    slack: &Slack,
    account: &Account,
    names: &mut HashMap<String, String>,
    nameless: &mut HashSet<String>,
    read: &BTreeMap<String, String>,
) -> Result<(), Refused> {
    let next: Vec<String> = unreads::unnamed(
        read,
        |id| names.contains_key(id) || nameless.contains(id),
        NAMED_EACH_ROUND,
    )
    .into_iter()
    .map(str::to_owned)
    .collect();
    for id in next {
        match slack.called(account, &id) {
            Ok(name) if !name.is_empty() => {
                names.insert(id, name);
            }
            Ok(_) => {
                nameless.insert(id);
            }
            Err(Refused::SignedOut) => return Err(Refused::SignedOut),
            Err(Refused::Unreachable(why)) => {
                eprintln!("<4>{why}");
                break;
            }
        }
    }
    Ok(())
}

/// Asks Joel to sign in again in For You, or stops asking, writing or
/// taking away `waiting` only when that changes, so the shell hears it
/// once; its folder is made at login, and here too should it be missing.
fn ask_joel(waiting: &Path, asks: bool) {
    if asks == waiting.exists() {
        return;
    }
    let done = if asks {
        waiting
            .parent()
            .map_or(Ok(()), std::fs::create_dir_all)
            .and_then(|()| whole(waiting, &sign_in_waits(millis()).to_string()))
    } else {
        std::fs::remove_file(waiting)
    };
    if let Err(error) = done {
        eprintln!("<4>couldn't change {}: {error}", waiting.display());
    }
}

/// Writes `text` in a `.writing` folder beside `file` and then renames it
/// over `file`, so no reader sees half, and a shell watching the folder
/// hears only the rename (#1515 does the same for Google's).
fn whole(file: &Path, text: &str) -> std::io::Result<()> {
    let aside = file
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(".writing");
    std::fs::create_dir_all(&aside)?;
    let writing = aside.join(file.file_name().unwrap_or_default());
    std::fs::write(&writing, text)?;
    std::fs::rename(&writing, file)
}

/// The words rows are said in, read once from the launcher's vocabulary.
fn vocabulary(path: &Path) -> Result<Words, String> {
    let text = std::fs::read_to_string(path).map_err(|why| why.to_string())?;
    let whole = serde_json::from_str(&text).map_err(|why| why.to_string())?;
    Words::read(&whole)
}

/// Now, in milliseconds since the epoch.
fn millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| {
            u64::try_from(since.as_millis()).unwrap_or(u64::MAX)
        })
}
