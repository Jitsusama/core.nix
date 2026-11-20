//! Brings a Mac's pi sessions back after it reboots or its login restarts.
//!
//! A reboot, or a console login starting over, ends every session the
//! node holds, since the node is an agent of the login, and nothing in
//! mycelium starts them again, so on a Mac each came back only when
//! someone typed `pi --session` for it. This program is run each minute.
//! Each run asks the node of every realm it's given which pi sessions it
//! holds here, and keeps them, with the folder each runs in, under
//! `$XDG_STATE_HOME/sessions-come-back/<realm>.json`, beside the boot and
//! the login it saw them in. The first run in a newer boot or login finds
//! what it kept from before and starts each one again, through `mycelium
//! start` on this machine, as `pi --session <id>` in its own folder, which
//! is the form the node takes as a session brought back.
//!
//! A session that ended while the machine ran isn't held at the next runs,
//! so it drops out of what's kept and doesn't come back: only a reboot or
//! a new login brings anything back. It drops out only once three runs in
//! a row haven't seen it, since a shutdown ends sessions before it ends
//! this agent, and a run in between mustn't read them as ended by hand. A
//! hold whose pi is no longer running holds nothing, as cadence reads it.
//! A session whose folder isn't there, or that the node won't start yet,
//! as while it's still linking after the boot, is tried again each run,
//! for a quarter of an hour, since pi started elsewhere would ask to fork
//! it.
//!
//!   sessions-come-back MYCELIUM REALM...    keeps, or brings back, each realm's sessions
//!
//! It takes the shape every command of ours does (docs/commands.md): its
//! help, and a failure that says what to try.

use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use terminal_kit::{Exit, Failure, Help};

/// How many runs a session waits to be started again before it's left,
/// a quarter of an hour at one a minute: past that, the node isn't coming,
/// or the folder isn't.
const TRIES: u32 = 15;

/// How many runs in a row a session goes unseen before it counts as ended
/// while the machine ran, rather than by the shutdown a run caught midway.
const GONE: u32 = 3;

/// How long the node has to say what it holds: a run is due again in a
/// minute, and launchd starts none while one still runs.
const LOOK_WITHIN: Duration = Duration::from_secs(20);

/// How long the node has to start one session.
const START_WITHIN: Duration = Duration::from_secs(30);

/// The programs a session is held as, which come back as they were held:
/// pi, and pi on the person's own Claude plan.
const PIS: [&str; 2] = ["pi", "pi-claude"];

/// A session the node held here: its id, the folder it runs in, and the
/// program it runs as.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Session {
    id: String,
    folder: String,
    #[serde(default = "plain_pi")]
    program: String,
}

fn plain_pi() -> String {
    "pi".into()
}

/// One waiting to be started again, with the runs it has waited.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Waiting {
    session: Session,
    tries: u32,
}

/// One held lately but not at the last runs, with how many in a row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Unseen {
    session: Session,
    runs: u32,
}

/// What a realm's run keeps between runs.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
struct Kept {
    /// Which boot and which login the sessions were seen in.
    era: String,
    /// The machine's name in the realm, which a start names to land here.
    machine: Option<String>,
    /// The sessions held here at the last run.
    sessions: Vec<Session>,
    /// The sessions held before the last run but not since, for fewer
    /// than GONE runs.
    #[serde(default)]
    unseen: Vec<Unseen>,
    /// The sessions a reboot or a new login ended, still to be started.
    #[serde(default)]
    waiting: Vec<Waiting>,
}

/// What the node holds here now: the machine's name and its pi sessions,
/// and what in its answer was passed over, and why.
#[derive(Debug, Default, PartialEq, Eq)]
struct Held {
    machine: Option<String>,
    sessions: Vec<Session>,
    passed: Vec<String>,
}

/// A line of `mycelium holds`: a program a holder keeps on this machine,
/// and the pid it runs as, which a holder whose program died keeps.
#[derive(Debug, Deserialize)]
struct Hold {
    id: String,
    #[serde(default)]
    command: Vec<String>,
    #[serde(default)]
    folder: String,
    #[serde(default)]
    pid: Option<u32>,
    #[serde(default)]
    parked: bool,
}

/// What `mycelium sessions` says of a session, of every machine's.
#[derive(Debug, Deserialize)]
struct Listed {
    id: String,
    #[serde(default)]
    held: Option<String>,
    #[serde(default)]
    machine: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Listing {
    #[serde(rename = "Sessions", default)]
    sessions: Vec<Listed>,
}

#[derive(Debug, Deserialize)]
enum Answer {
    Ok(Listing),
    Err(serde_json::Value),
}

/// The pi a hold runs, by its program's file name, since a session may be
/// held by name or by path. Any other program, such as a sprout's wrapper,
/// would start its work over, so it isn't brought back.
fn pi_of(command: &[String]) -> Option<&'static str> {
    let name = Path::new(command.first()?).file_name()?;
    PIS.into_iter().find(|pi| name == *pi)
}

/// Whether an id can only be read as one, since it's passed to a command
/// on another's behalf: letters, digits and `-`, `_` and `.`, not first.
fn plain(id: &str) -> bool {
    !id.is_empty()
        && !id.starts_with(['-', '.'])
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
}

/// The pi sessions held here, joining the holds this machine keeps to the
/// node's sessions by the hold's id, for the session's own id. A hold
/// whose pi has died and isn't parked holds nothing. A line that isn't a
/// hold, a hold with no folder, one more than one session names, one
/// naming another machine than the rest and an id that isn't plain are
/// passed over and said, so the rest are kept.
fn held(holds: &str, sessions: &str, runs: impl Fn(u32) -> bool) -> Result<Held, String> {
    let listing = match serde_json::from_str::<Answer>(sessions).map_err(|e| e.to_string())? {
        Answer::Ok(listing) => listing,
        Answer::Err(why) => return Err(format!("the node said {why}")),
    };
    let mut found = Held::default();
    for line in holds.lines().filter(|line| !line.trim().is_empty()) {
        let hold: Hold = match serde_json::from_str(line) {
            Ok(hold) => hold,
            Err(why) => {
                found
                    .passed
                    .push(format!("a line of its holds isn't one ({why})"));
                continue;
            }
        };
        let Some(program) = pi_of(&hold.command) else {
            continue;
        };
        if !hold.parked && hold.pid.is_some_and(|pid| !runs(pid)) {
            continue;
        }
        if hold.folder.is_empty() {
            found
                .passed
                .push(format!("hold {} names no folder", hold.id));
            continue;
        }
        let named: Vec<&Listed> = listing
            .sessions
            .iter()
            .filter(|listed| listed.held.as_deref() == Some(hold.id.as_str()))
            .collect();
        let listed = match named.as_slice() {
            [] => continue,
            [listed] => *listed,
            _ => {
                found
                    .passed
                    .push(format!("{} sessions name hold {}", named.len(), hold.id));
                continue;
            }
        };
        if !plain(&listed.id) {
            found
                .passed
                .push(format!("hold {}'s session id isn't plain", hold.id));
            continue;
        }
        match (&found.machine, &listed.machine) {
            (None, machine) => found.machine.clone_from(machine),
            (Some(here), Some(there)) if here != there => {
                found.passed.push(format!(
                    "{} names {there}, not {here}, as its machine",
                    listed.id
                ));
                continue;
            }
            _ => {}
        }
        found.sessions.push(Session {
            id: listed.id.clone(),
            folder: hold.folder,
            program: program.into(),
        });
    }
    Ok(found)
}

/// What to keep after this run's look, before any start: what's held now,
/// and what's waiting, which a newer boot or login fills with what was
/// held before it. A session held again no longer waits.
fn next(kept: Option<Kept>, era: &str, now: &Held) -> Kept {
    let Some(before) = kept else {
        return Kept {
            era: era.into(),
            machine: now.machine.clone(),
            sessions: now.sessions.clone(),
            unseen: Vec::new(),
            waiting: Vec::new(),
        };
    };
    let held_now = |id: &str| now.sessions.iter().any(|s| s.id == id);
    let mut waiting = before.waiting;
    let mut unseen = Vec::new();
    if before.era == era {
        let missed = before
            .sessions
            .into_iter()
            .map(|session| Unseen { session, runs: 0 })
            .chain(before.unseen);
        for Unseen { session, runs } in missed {
            if !held_now(&session.id) && runs + 1 < GONE {
                unseen.push(Unseen {
                    session,
                    runs: runs + 1,
                });
            }
        }
    } else {
        let lately = before.unseen.into_iter().map(|u| u.session);
        for session in before.sessions.into_iter().chain(lately) {
            if !waiting.iter().any(|w| w.session.id == session.id) {
                waiting.push(Waiting { session, tries: 0 });
            }
        }
    }
    waiting.retain(|w| !held_now(&w.session.id));
    Kept {
        era: era.into(),
        machine: now.machine.clone().or(before.machine),
        sessions: now.sessions.clone(),
        unseen,
        waiting,
    }
}

/// Starts each waiting session whose folder is there, and says what it
/// did with each. One started waits no more; one whose folder isn't there
/// or that the node wouldn't start waits for another run, up to TRIES.
fn bring_back(
    kept: &mut Kept,
    there: impl Fn(&str) -> bool,
    mut start: impl FnMut(&str, &Session) -> Result<(), String>,
    mut say: impl FnMut(String),
) {
    let Some(machine) = kept.machine.clone() else {
        if !kept.waiting.is_empty() {
            say("no session has said which machine this is, so none is started yet".into());
        }
        return;
    };
    kept.waiting.retain_mut(|waiting| {
        let session = &waiting.session;
        let why = if there(&session.folder) {
            match start(&machine, session) {
                Ok(()) => {
                    say(format!(
                        "{} started again in {}",
                        session.id, session.folder
                    ));
                    return false;
                }
                Err(why) => why,
            }
        } else {
            format!("its folder, {}, isn't there", session.folder)
        };
        waiting.tries += 1;
        let left = waiting.tries < TRIES;
        say(format!(
            "{} wasn't started again ({why}); {}",
            waiting.session.id,
            if left {
                "trying again next run"
            } else {
                "leaving it"
            }
        ));
        left
    });
}

/// The words that start a session again on this machine, in its folder,
/// as the program it was held as.
fn start_words(machine: &str, session: &Session) -> Vec<String> {
    [
        "start",
        "--need",
        &format!("machine={machine}"),
        "--folder",
        &session.folder,
        "--",
        &session.program,
        "--session",
        &session.id,
    ]
    .map(String::from)
    .to_vec()
}

/// Whether the node's answer to a start says it started: `{"Ok":{"Started":...}}`.
fn started(said: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(said)
        .ok()
        .and_then(|answer| answer.get("Ok").cloned())
        .is_some_and(|ok| ok.get("Started").is_some())
}

/// What a kept record says, or nothing on a first run; a record that
/// can't be read is said rather than taken as a first run unheard.
fn recall(text: Option<&str>) -> Result<Option<Kept>, String> {
    text.map(serde_json::from_str::<Kept>)
        .transpose()
        .map_err(|e| e.to_string())
}

/// Which boot and login this is: a Mac's boot session uuid and its
/// console login, from `who`, or Linux's boot id. A clock step changes
/// neither, and a login starting over changes the second.
fn era() -> Result<String, Failure> {
    let unread = |why: String| {
        Failure::new(Exit::Unavailable, "it couldn't read which boot this is").because(why)
    };
    if let Ok(id) = fs::read_to_string("/proc/sys/kernel/random/boot_id") {
        return Ok(id.trim().into());
    }
    let uuid = within(
        Command::new("/usr/sbin/sysctl").args(["-n", "kern.bootsessionuuid"]),
        LOOK_WITHIN,
    )
    .map_err(unread)?;
    let who = within(&mut Command::new("/usr/bin/who"), LOOK_WITHIN).map_err(unread)?;
    let user = env::var("USER").unwrap_or_default();
    let login = console_login(&who, &user).unwrap_or_default();
    Ok(format!("{} {login}", uuid.trim()))
}

/// When the account logged in at the console, from `who`'s line for it:
/// `someone      console      Nov 19 12:42`.
fn console_login(who: &str, user: &str) -> Option<String> {
    who.lines().find_map(|line| {
        let mut words = line.split_whitespace();
        (words.next() == Some(user) && words.next() == Some("console"))
            .then(|| words.collect::<Vec<_>>().join(" "))
    })
}

/// Whether a process runs: Linux's /proc, or a Mac's `kill -0`.
fn runs(pid: u32) -> bool {
    if Path::new("/proc/self").exists() {
        return Path::new(&format!("/proc/{pid}")).exists();
    }
    within(
        Command::new("/bin/kill").args(["-0", &pid.to_string()]),
        LOOK_WITHIN,
    )
    .is_ok()
}

/// Where each realm's record is kept.
fn state() -> Result<PathBuf, Failure> {
    let base = env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/state")))
        .ok_or_else(|| {
            Failure::new(Exit::Config, "there's nowhere to keep the sessions")
                .because("neither XDG_STATE_HOME nor HOME is set")
        })?;
    Ok(base.join("sessions-come-back"))
}

/// What a command printed, or why it couldn't say: it failed, or gave no
/// answer in time, and was stopped, so a node that hangs holds up no run.
fn within(command: &mut Command, limit: Duration) -> Result<String, String> {
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;
    let drain = |pipe: Option<Box<dyn Read + Send>>| {
        thread::spawn(move || {
            let mut text = String::new();
            if let Some(mut pipe) = pipe {
                let _ = pipe.read_to_string(&mut text);
            }
            text
        })
    };
    let out = drain(
        child
            .stdout
            .take()
            .map(|p| Box::new(p) as Box<dyn Read + Send>),
    );
    let err = drain(
        child
            .stderr
            .take()
            .map(|p| Box::new(p) as Box<dyn Read + Send>),
    );
    let deadline = Instant::now() + limit;
    let status = loop {
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
            break status;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err(format!("no answer within {} s", limit.as_secs()));
        }
        thread::sleep(Duration::from_millis(50));
    };
    let out = out.join().unwrap_or_default();
    if status.success() {
        Ok(out)
    } else {
        Err(err.join().unwrap_or_default().trim().to_string())
    }
}

/// A mycelium command's answer in a realm.
fn ask(mycelium: &str, realm: &str, words: &[String], limit: Duration) -> Result<String, String> {
    within(
        Command::new(mycelium).args(["--realm", realm]).args(words),
        limit,
    )
}

/// Now, as a log line's stamp: `2026-11-19T23:14:01Z`.
fn stamp(seconds: u64) -> String {
    let (days, rest) = (seconds / 86_400, seconds % 86_400);
    // Days since 1970-01-01 to a civil date, by Howard Hinnant's method.
    let z = days + 719_468;
    let era = z / 146_097;
    let doe = z % 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + u64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        rest / 3600,
        rest % 3600 / 60,
        rest % 60
    )
}

/// Says a line in the agent's log, stamped and named by its realm.
fn say(realm: &str, line: &str) {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs());
    eprintln!("{} {realm}: {line}", stamp(now));
}

/// Writes a realm's record whole, or not at all.
fn keep(file: &Path, kept: &Kept) -> Result<(), Failure> {
    let unkept = |why: String| {
        Failure::new(Exit::Failed, "it couldn't keep the sessions")
            .because(why)
            .try_this(format!("check that {} can be written", file.display()))
    };
    if let Some(folder) = file.parent() {
        fs::create_dir_all(folder).map_err(|e| unkept(e.to_string()))?;
    }
    let text = serde_json::to_string_pretty(kept).map_err(|e| unkept(e.to_string()))?;
    let partial = file.with_extension("json.partial");
    fs::write(&partial, text).map_err(|e| unkept(e.to_string()))?;
    fs::rename(&partial, file).map_err(|e| unkept(e.to_string()))
}

/// One realm's run: look, keep, and start what a reboot or a new login
/// ended. What's waiting is kept before any start, so a run that couldn't
/// keep it starts nothing a later run would start again.
fn run_realm(mycelium: &str, realm: &str, era: &str, folder: &Path) -> Result<(), Failure> {
    let unanswered = |why: String| {
        Failure::new(
            Exit::TryAgain,
            format!("the {realm} realm's node didn't say what it holds"),
        )
        .because(why)
        .try_this(format!("mycelium --realm {realm} holds"))
    };
    let holds = ask(mycelium, realm, &["holds".into()], LOOK_WITHIN).map_err(unanswered)?;
    let sessions = ask(mycelium, realm, &["sessions".into()], LOOK_WITHIN).map_err(unanswered)?;
    let now = held(&holds, &sessions, runs).map_err(|why| {
        Failure::new(Exit::BadInput, "the node's sessions couldn't be read").because(why)
    })?;
    for line in &now.passed {
        say(realm, &format!("passed over: {line}"));
    }
    let file = folder.join(format!("{realm}.json"));
    let text = fs::read_to_string(&file).ok();
    let before = recall(text.as_deref()).unwrap_or_else(|why| {
        let aside = file.with_extension("json.unread");
        say(
            realm,
            &format!(
                "its record couldn't be read ({why}), so it's set aside as {} and kept afresh",
                aside.display()
            ),
        );
        let _ = fs::rename(&file, &aside);
        None
    });
    let mut kept = next(before, era, &now);
    keep(&file, &kept)?;
    if kept.waiting.is_empty() {
        return Ok(());
    }
    bring_back(
        &mut kept,
        |folder| Path::new(folder).is_dir(),
        |machine, session| {
            let said = ask(
                mycelium,
                realm,
                &start_words(machine, session),
                START_WITHIN,
            )?;
            if started(&said) {
                Ok(())
            } else {
                Err(said.trim().to_string())
            }
        },
        |line| say(realm, &line),
    );
    keep(&file, &kept)
}

/// Every realm's run, each on its own, so one realm's node being away
/// keeps none of the others' sessions from coming back. Each failure is
/// said; the first is what the run ends with.
fn run(mycelium: &str, realms: &[String]) -> Result<(), Failure> {
    let era = era()?;
    let folder = state()?;
    let mut failed = None;
    for realm in realms {
        if let Err(failure) = run_realm(mycelium, realm, &era, &folder) {
            if failed.is_some() {
                say(realm, &failure.to_string());
            }
            failed.get_or_insert(failure);
        }
    }
    failed.map_or(Ok(()), Err)
}

/// What it says of itself, set as help and read by the check.
fn help() -> Help {
    Help::new(
        "sessions-come-back",
        "brings a Mac's pi sessions back after it reboots or its login restarts",
    )
    .example(
        "sessions-come-back mycelium work",
        "what its agent runs each minute, keeping or bringing back the work realm's sessions",
    )
    .exit(Exit::Failed, "it couldn't keep the sessions")
    .exit(Exit::BadInput, "the node's sessions couldn't be read")
    .exit(Exit::Unavailable, "it couldn't read which boot this is")
    .exit(Exit::TryAgain, "a realm's node didn't say what it holds")
    .exit(Exit::Config, "there's nowhere to keep the sessions")
}

fn main() -> ExitCode {
    terminal_kit::main(&help(), env!("CARGO_PKG_VERSION"), |context| {
        let [mycelium, realms @ ..] = context.args.as_slice() else {
            return Err(
                Failure::new(Exit::Usage, "it takes mycelium, as a path, and realms")
                    .try_this("sessions-come-back mycelium work"),
            );
        };
        if realms.is_empty() {
            return Err(Failure::new(Exit::Usage, "it takes at least one realm")
                .try_this("sessions-come-back mycelium work"));
        }
        run(mycelium, realms)
    })
}

#[cfg(test)]
mod tests;
