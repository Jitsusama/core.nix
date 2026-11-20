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
//! fifteen times, since pi started elsewhere would ask to fork it. A
//! record last kept more than four days before the boot brings nothing
//! back, since what it held is from long ago. A realm's `.off` file beside
//! its record keeps its sessions from coming back while it's there: the
//! record is removed and none is kept, so a file removed again later
//! brings back nothing from before it.
//!
//!   sessions-come-back MYCELIUM REALM...    keeps, or brings back, each realm's sessions
//!
//! It takes the shape every command of ours does (docs/commands.md): its
//! help, and a failure that says what to try.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use terminal_kit::{Exit, Failure, Help};

mod machine;
use machine::{Unheard, era, runs, within};
#[cfg(test)]
use machine::{console_login, era_of};

/// How many runs a session waits to be started again before it's left:
/// past that, the node isn't coming, or the folder isn't. At one a minute
/// that's a quarter of an hour when each try is refused, and up to about
/// 75 minutes when each goes unanswered, since each of those waits QUIET.
const TRIES: u32 = 15;

/// How many runs in a row a session goes unseen before it counts as ended
/// while the machine ran, rather than by the shutdown a run caught midway.
const GONE: u32 = 3;

/// How long before a new boot or login the record may last have been kept
/// and still bring its sessions back: a long weekend with the machine off.
/// Past it, a record left from before a rollback or a long outage would
/// start sessions from long ago.
const STALE: u64 = 4 * 24 * 60 * 60;

/// How long a start the node gave no answer to waits before it's asked
/// again, since the node may have started it and not yet say it holds it.
const QUIET: u64 = 5 * 60;

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

/// One waiting to be started again, with the runs it has waited, and
/// when a start of it last went unanswered.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Waiting {
    session: Session,
    tries: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    unanswered: Option<u64>,
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
    /// When they were seen, in seconds since 1970.
    #[serde(default)]
    looked: u64,
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
    /// What the last run passed over, so each is said once, not each minute.
    #[serde(default)]
    passed: Vec<String>,
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
/// and the pid it runs as, 0 while it's parked or gone, as cadence reads
/// the same lines.
#[derive(Debug, Deserialize)]
struct Hold {
    id: String,
    command: Vec<String>,
    #[serde(default)]
    folder: String,
    #[serde(default)]
    pid: u32,
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
    #[serde(default)]
    state: Option<String>,
}

/// A listing's answer, which says `Sessions` whenever it's one.
#[derive(Debug, Deserialize)]
struct Listing {
    #[serde(rename = "Sessions")]
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

/// Whether a word can only be read as one, since it's passed to a command
/// on another's behalf: letters, digits and `-`, `_` and `.`, not first.
fn plain(word: &str) -> bool {
    !word.is_empty()
        && !word.starts_with(['-', '.'])
        && word
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
}

/// Whether a word can only be read as a session's id, as pi makes one:
/// letters, digits and `-`, not first. Narrower than `plain`, since an id
/// with a `.` in it, as a file's name, `pi --session` would read as a path.
fn an_id(word: &str) -> bool {
    !word.is_empty()
        && !word.starts_with('-')
        && word.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

/// Whether a hold's pi still runs: one parked holds its session, and one
/// whose pid is 0, or no longer runs, holds nothing.
fn holds_its_pi(hold: &Hold, runs: impl Fn(u32) -> bool) -> bool {
    hold.parked || (hold.pid > 0 && runs(hold.pid))
}

/// The machine most of the sessions held here name, since a hold's id is
/// joined to a listing every machine of the realm writes to, and one
/// entry, mistaken or not, mustn't say where every start goes. None when
/// no machine is named by more than any other.
fn most_named<'a>(named: impl Iterator<Item = &'a str>) -> Option<String> {
    let mut counts = BTreeMap::<&str, usize>::new();
    for machine in named {
        *counts.entry(machine).or_default() += 1;
    }
    let most = *counts.values().max()?;
    let mut leaders = counts.into_iter().filter(|&(_, count)| count == most);
    match (leaders.next(), leaders.next()) {
        (Some((machine, _)), None) => Some(machine.into()),
        _ => None,
    }
}

/// The pi sessions held here, joining the holds this machine keeps to the
/// node's live sessions by the hold's id, for the session's own id. A hold
/// whose pi has died and isn't parked holds nothing. A line that isn't a
/// hold, a hold with no folder, one more than one live session names, an
/// id or machine that isn't plain, and a session naming another machine
/// than most do are passed over and said, so the rest are kept.
fn held(holds: &str, sessions: &str, runs: impl Fn(u32) -> bool) -> Result<Held, String> {
    let listing = match serde_json::from_str::<Answer>(sessions).map_err(|e| e.to_string())? {
        Answer::Ok(listing) => listing,
        Answer::Err(why) => return Err(format!("the node said {why}")),
    };
    let live: Vec<&Listed> = listing
        .sessions
        .iter()
        .filter(|listed| listed.state.as_deref() != Some("ended"))
        .collect();
    let mut found = Held::default();
    let mut joined = Vec::new();
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
        if !holds_its_pi(&hold, &runs) {
            continue;
        }
        if hold.folder.is_empty() {
            found
                .passed
                .push(format!("hold {} names no folder", hold.id));
            continue;
        }
        let named: Vec<&Listed> = live
            .iter()
            .copied()
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
        if !an_id(&listed.id) {
            found
                .passed
                .push(format!("hold {}'s session id isn't plain", hold.id));
            continue;
        }
        let Some(machine) = listed.machine.as_deref().filter(|m| plain(m)) else {
            found
                .passed
                .push(format!("{} names no plain machine", listed.id));
            continue;
        };
        let session = Session {
            id: listed.id.clone(),
            folder: hold.folder,
            program: program.into(),
        };
        joined.push((machine, session));
    }
    found.machine = most_named(joined.iter().map(|(machine, _)| *machine));
    for (machine, session) in joined {
        match &found.machine {
            Some(here) if here == machine => found.sessions.push(session),
            Some(here) => found.passed.push(format!(
                "{} names {machine}, not {here}, as its machine",
                session.id
            )),
            None => found.passed.push(format!(
                "{} names {machine}, and no machine is named by most of them",
                session.id
            )),
        }
    }
    Ok(found)
}

/// Whether a record kept in another era was last kept too long before it
/// to bring back what it held.
fn stale(before: &Kept, era: &str, at: u64) -> bool {
    before.era != era && at.saturating_sub(before.looked) > STALE
}

/// What to keep after this run's look, at `at`, before any start: what's
/// held now, and what's waiting, which a newer boot or login fills with
/// what was held before it, unless that was kept too long before. A
/// session held again no longer waits.
fn next(kept: Option<Kept>, era: &str, now: &Held, at: u64) -> Kept {
    let Some(before) = kept else {
        return Kept {
            era: era.into(),
            looked: at,
            machine: now.machine.clone(),
            sessions: now.sessions.clone(),
            passed: now.passed.clone(),
            ..Kept::default()
        };
    };
    let held_now = |id: &str| now.sessions.iter().any(|s| s.id == id);
    let is_stale = stale(&before, era, at);
    let mut waiting = if is_stale { Vec::new() } else { before.waiting };
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
    } else if !is_stale {
        for waiting in &mut waiting {
            waiting.tries = 0;
            waiting.unanswered = None;
        }
        let lately = before.unseen.into_iter().map(|u| u.session);
        for session in before.sessions.into_iter().chain(lately) {
            if !waiting.iter().any(|w| w.session.id == session.id) {
                waiting.push(Waiting {
                    session,
                    tries: 0,
                    unanswered: None,
                });
            }
        }
    }
    waiting.retain(|w| !held_now(&w.session.id));
    Kept {
        era: era.into(),
        looked: at,
        machine: now.machine.clone().or(before.machine),
        sessions: now.sessions.clone(),
        unseen,
        waiting,
        passed: now.passed.clone(),
    }
}

/// What a start's ask came to.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Start {
    Started,
    /// The node said no, or the folder wasn't there: try again next run.
    Refused(String),
    /// The node's client gave no answer, in time or at all, so the node may
    /// have started it.
    Unanswered(String),
}

/// Starts each waiting session whose folder is there, at `at`, and says
/// what it did with each. One started waits no more; one whose folder
/// isn't there or that the node wouldn't start waits for another run, up
/// to TRIES; one the node gave no answer for waits QUIET before it's asked
/// again. What the record says is checked again before it's run, as what
/// the node said was.
fn bring_back(
    kept: &mut Kept,
    at: u64,
    there: impl Fn(&str) -> bool,
    mut start: impl FnMut(&str, &Session) -> Start,
    mut say: impl FnMut(String),
) {
    let Some(machine) = kept.machine.clone().filter(|m| plain(m)) else {
        if !kept.waiting.is_empty() {
            say("no session has said which machine this is, so none is started yet".into());
        }
        return;
    };
    kept.waiting.retain_mut(|waiting| {
        let session = &waiting.session;
        if !an_id(&session.id)
            || !PIS.contains(&session.program.as_str())
            || !Path::new(&session.folder).is_absolute()
        {
            say(format!(
                "passed over: the record's {:?}, as {:?} in {:?}, isn't a pi session",
                session.id, session.program, session.folder
            ));
            return false;
        }
        if waiting
            .unanswered
            .is_some_and(|asked| at.saturating_sub(asked) < QUIET)
        {
            return true;
        }
        let outcome = if there(&session.folder) {
            start(&machine, session)
        } else {
            Start::Refused(format!("its folder, {}, isn't there", session.folder))
        };
        let why = match outcome {
            Start::Started => {
                say(format!(
                    "{} started again in {}",
                    session.id, session.folder
                ));
                return false;
            }
            Start::Refused(why) => {
                waiting.unanswered = None;
                why
            }
            Start::Unanswered(why) => {
                waiting.unanswered = Some(at);
                format!(
                    "{why}, so it may yet start; it's asked again in {} min",
                    QUIET / 60
                )
            }
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

/// A realm's record as text, none only when there is none: a record that
/// can't be read, as one a crash left in bytes that aren't text, is said.
fn read_record(file: &Path) -> Result<Option<String>, String> {
    match fs::read_to_string(file) {
        Ok(text) => Ok(Some(text)),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.to_string()),
    }
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

/// Now, in seconds since 1970.
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs())
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

/// A log line as one line, whatever the node or a peer put in it: a
/// control character, a line break among them, is written as its escape,
/// so nothing said can pass for a line of the log's own.
fn one_line(line: &str) -> String {
    line.chars()
        .map(|c| {
            if c.is_control() {
                c.escape_default().to_string()
            } else {
                c.to_string()
            }
        })
        .collect()
}

/// Says a line in the agent's log, stamped and named by its realm.
fn say(realm: &str, line: &str) {
    eprintln!("{} {}: {}", stamp(now()), one_line(realm), one_line(line));
}

/// Writes a realm's record whole, or not at all, and on the disk before
/// it takes the old one's place, since the restarts it's for include
/// those with no clean shutdown. One unchanged since this run kept it, as
/// after a run that started nothing, isn't written again.
fn keep(file: &Path, kept: &Kept) -> Result<(), Failure> {
    let unkept = |why: String| {
        Failure::new(Exit::Failed, "it couldn't keep the sessions")
            .because(why)
            .try_this(format!("check that {} can be written", file.display()))
    };
    let text = serde_json::to_string_pretty(kept).map_err(|e| unkept(e.to_string()))?;
    if fs::read_to_string(file).is_ok_and(|was| was == text) {
        return Ok(());
    }
    if let Some(folder) = file.parent() {
        fs::create_dir_all(folder).map_err(|e| unkept(e.to_string()))?;
    }
    let partial = file.with_extension("json.partial");
    let mut written = fs::File::create(&partial).map_err(|e| unkept(e.to_string()))?;
    written
        .write_all(text.as_bytes())
        .and_then(|()| written.sync_all())
        .map_err(|e| unkept(e.to_string()))?;
    fs::rename(&partial, file).map_err(|e| unkept(e.to_string()))
}

/// Asks a realm's node, within a limit, what the words ask.
type Ask<'a> = dyn Fn(&[String], Duration) -> Result<String, Unheard> + 'a;

/// What a realm's run reaches beyond itself, so a test can stand in for
/// the node, the processes and the folders.
struct World<'a> {
    ask: &'a Ask<'a>,
    runs: &'a dyn Fn(u32) -> bool,
    there: &'a dyn Fn(&str) -> bool,
    say: &'a dyn Fn(&str),
}

/// One realm's run, at `at`: look, keep, and start what a reboot or a new
/// login ended. What's waiting is kept before any start, so a run that
/// couldn't keep it starts nothing a later run would start again. A run
/// beside it on the same realm, as one by hand, leaves it to that one.
fn run_realm(realm: &str, era: &str, folder: &Path, at: u64, world: &World) -> Result<(), Failure> {
    let failed = |exit: Exit, what: &str, why: String| {
        Failure::new(exit, format!("{what} in the {realm} realm")).because(why)
    };
    fs::create_dir_all(folder)
        .map_err(|e| failed(Exit::Failed, "it couldn't keep the sessions", e.to_string()))?;
    let lock = fs::File::create(folder.join(format!("{realm}.lock")))
        .map_err(|e| failed(Exit::Failed, "it couldn't keep the sessions", e.to_string()))?;
    match lock.try_lock() {
        Ok(()) => {}
        Err(fs::TryLockError::WouldBlock) => {
            (world.say)("another run is keeping it, so this one leaves it");
            return Ok(());
        }
        Err(fs::TryLockError::Error(e)) => {
            return Err(failed(
                Exit::Failed,
                "it couldn't keep the sessions",
                format!("its lock couldn't be taken: {e}"),
            ));
        }
    }
    let file = folder.join(format!("{realm}.json"));
    if folder.join(format!("{realm}.off")).exists() {
        if fs::remove_file(&file).is_ok() {
            (world.say)(&format!(
                "it's off while {realm}.off is there, so its record is removed"
            ));
        }
        return Ok(());
    }
    let unanswered = |why: Unheard| {
        failed(
            Exit::TryAgain,
            "the node didn't say what it holds",
            why.to_string(),
        )
        .try_this(format!("mycelium --realm {realm} holds"))
    };
    let holds = (world.ask)(&["holds".into()], LOOK_WITHIN).map_err(unanswered)?;
    let sessions = (world.ask)(&["sessions".into()], LOOK_WITHIN).map_err(unanswered)?;
    let now = held(&holds, &sessions, world.runs)
        .map_err(|why| failed(Exit::BadInput, "the node's sessions couldn't be read", why))?;
    let before = read_record(&file)
        .and_then(|text| recall(text.as_deref()))
        .unwrap_or_else(|why| {
            let aside = file.with_extension("json.unread");
            let moved = match fs::rename(&file, &aside) {
                Ok(()) => format!("it's set aside as {}", aside.display()),
                Err(e) => format!("it couldn't be set aside ({e})"),
            };
            (world.say)(&format!(
                "its record couldn't be read ({why}), so {moved}, and it's kept afresh"
            ));
            None
        });
    let said_before = before
        .as_ref()
        .map(|b| b.passed.clone())
        .unwrap_or_default();
    for line in now.passed.iter().filter(|line| !said_before.contains(line)) {
        (world.say)(&format!("passed over: {line}"));
    }
    if let Some(before) = before.as_ref().filter(|before| stale(before, era, at)) {
        (world.say)(&format!(
            "its record was last kept {} h before this boot or login, too long to bring back \
             what it held",
            at.saturating_sub(before.looked) / 3600
        ));
    }
    let mut kept = next(before, era, &now, at);
    keep(&file, &kept).map_err(|f| f.because(format!("in the {realm} realm")))?;
    if kept.waiting.is_empty() {
        return Ok(());
    }
    bring_back(
        &mut kept,
        at,
        world.there,
        |machine, session| match (world.ask)(&start_words(machine, session), START_WITHIN) {
            Ok(said) if started(&said) => Start::Started,
            Ok(said) => Start::Refused(said.trim().to_string()),
            Err(unheard) => Start::Unanswered(unheard.to_string()),
        },
        |line| (world.say)(&line),
    );
    keep(&file, &kept).map_err(|f| f.because(format!("in the {realm} realm")))
}

/// Every realm's run, each on its own, so one realm's node being away
/// keeps none of the others' sessions from coming back. Each failure is
/// said, stamped and named by its realm; the first is what the run ends
/// with.
fn run(mycelium: &str, realms: &[String]) -> Result<(), Failure> {
    let era = era()?;
    let folder = state()?;
    let mut failed = None;
    for realm in realms {
        let ask = |words: &[String], limit: Duration| {
            within(
                Command::new(mycelium).args(["--realm", realm]).args(words),
                limit,
            )
        };
        let speak = |line: &str| say(realm, line);
        let world = World {
            ask: &ask,
            runs: &runs,
            there: &|folder| Path::new(folder).is_dir(),
            say: &speak,
        };
        if let Err(failure) = run_realm(realm, &era, &folder, now(), &world) {
            say(realm, &failure.to_string());
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
    .example(
        "touch ~/.local/state/sessions-come-back/work.off",
        "keeps the work realm's sessions from coming back until the file is removed",
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
        if let Some(realm) = realms.iter().find(|realm| !plain(realm)) {
            return Err(Failure::new(Exit::Usage, "a realm is named plainly")
                .because(format!("{realm:?} would name a file elsewhere"))
                .try_this("sessions-come-back mycelium work"));
        }
        run(mycelium, realms)
    })
}

#[cfg(test)]
mod tests;
