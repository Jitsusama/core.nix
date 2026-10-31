//! slack-unreads CREDENTIALS FILE SOCKET [ORIGIN [PACE]]
//!
//! What in Slack wants Joel, kept in FILE for the shell's Now (lib's
//! `unreads` says what it is and how it's read): Slack is asked every
//! half minute, and FILE is written whole, through a file beside it, only
//! when what it says changes, or every five minutes so the shell can tell
//! a quiet Slack from this program gone. Slack refusing the sign-in is
//! said at once; Slack out of reach only once two asks in a row have
//! failed, so one lost answer doesn't blank the count. With no sign-in in
//! CREDENTIALS, the harness's file, FILE is taken away and nothing is
//! asked until there is one. ORIGIN is Slack's and PACE, in milliseconds,
//! half a minute, unless a test names a stand-in and a quicker pace.
//!
//! Run by systemd, it tells its watchdog after each ask, so one that hangs
//! is restarted, and its unit takes FILE away however it stops: a count
//! said "ok" never outlives the program that read it.
//!
//! It's Slack's daemon too: on SOCKET it answers the shell's daemon
//! interface, searching Slack when Joel asks (answers.rs), so nothing is
//! started to search.

mod answers;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use slack_people::unreads::{Counted, Kind, Said, State, Unread, due, link};
use slack_people::{Account, Refused, Slack};

/// How often Slack is asked, unless a test says otherwise.
const EVERY: Duration = Duration::from_secs(30);

/// How old the file may grow while nothing changes, in milliseconds: the
/// shell takes one ten minutes old as unknown.
const STALE: u64 = 5 * 60 * 1000;

/// How many asks in a row may fail before Slack is said to be out of
/// reach.
const MISSES: u32 = 2;

fn main() {
    let mut args = std::env::args().skip(1);
    let (Some(credentials), Some(file), Some(socket)) = (args.next(), args.next(), args.next())
    else {
        eprintln!("usage: slack-unreads CREDENTIALS FILE SOCKET [ORIGIN [PACE]]");
        std::process::exit(64);
    };
    let origin = args.next().unwrap_or_else(|| "https://slack.com".into());
    answers::listen(
        Path::new(&socket),
        PathBuf::from(&credentials),
        origin.clone(),
    );
    let slack = Slack::new(origin);
    let every = args
        .next()
        .and_then(|every| every.parse().ok())
        .map_or(EVERY, Duration::from_millis);
    let (credentials, file) = (PathBuf::from(credentials), PathBuf::from(file));
    let mut names = HashMap::new();
    let mut written: Option<Said> = None;
    let mut misses = 0;
    loop {
        // The file is read each time, so signing in again from pi is
        // taken without a restart.
        let Some(account) = Account::read(&credentials) else {
            if written.take().is_some() || file.exists() {
                let _ = std::fs::remove_file(&file);
            }
            std::thread::sleep(every);
            continue;
        };
        let checked = millis();
        let asked = slack
            .counts(&account)
            .and_then(|counted| named(&slack, &account, &mut names, counted));
        let said = match asked {
            Ok(conversations) => {
                misses = 0;
                Some(Said {
                    checked,
                    state: State::Ok,
                    conversations,
                })
            }
            Err(Refused::SignedOut) => Some(Said {
                checked,
                state: State::SignedOut,
                conversations: Vec::new(),
            }),
            Err(Refused::Unreachable(why)) => {
                eprintln!("<4>{why}");
                misses += 1;
                (misses >= MISSES).then(|| Said {
                    checked,
                    state: State::Unreachable,
                    conversations: Vec::new(),
                })
            }
        };
        if let Some(said) = said
            && due(written.as_ref(), &said, STALE)
        {
            match write(&file, &said) {
                Ok(()) => written = Some(said),
                Err(error) => eprintln!("<4>couldn't write {}: {error}", file.display()),
            }
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

/// Each conversation that wants Joel, named, each name asked once a run;
/// one Slack won't name now is said as Slack's, and asked again next time.
fn named(
    slack: &Slack,
    account: &Account,
    names: &mut HashMap<String, String>,
    counted: Vec<Counted>,
) -> Result<Vec<Unread>, Refused> {
    counted
        .into_iter()
        .map(|counted| {
            let name = match counted.kind {
                Kind::Thread => "Threads".to_owned(),
                _ => match names.get(&counted.id) {
                    Some(name) => name.clone(),
                    None => match slack.called(account, &counted.id) {
                        Ok(name) if !name.is_empty() => {
                            names.insert(counted.id.clone(), name.clone());
                            name
                        }
                        Ok(_) => "Slack".to_owned(),
                        Err(Refused::SignedOut) => return Err(Refused::SignedOut),
                        Err(Refused::Unreachable(why)) => {
                            eprintln!("<4>{why}");
                            "Slack".to_owned()
                        }
                    },
                },
            };
            Ok(Unread {
                link: link(&account.team, &counted),
                id: counted.id,
                name,
                kind: counted.kind,
                count: counted.count,
            })
        })
        .collect()
}

/// Writes what's said whole, beside the file and then over it, so the
/// shell never reads half of it.
fn write(file: &Path, said: &Said) -> std::io::Result<()> {
    let text = serde_json::to_string(said).map_err(std::io::Error::other)?;
    let beside = file.with_extension("new");
    std::fs::write(&beside, text)?;
    std::fs::rename(&beside, file)
}

/// Now, in milliseconds since the epoch.
fn millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| {
            u64::try_from(since.as_millis()).unwrap_or(u64::MAX)
        })
}
