//! The socket an ask comes in on. Only root and the config's group may
//! write it, so the kernel decides who may ask. An ask is one line,
//! `update`, or `stop`, which ends an update still fetching or building,
//! or one of the asks that pause updates or hold a commit back
//! (src/held.rs), and anything else is refused. An ask while an update runs
//! joins it and hears it from its first line, so a flood of asks is one
//! update. Once an update has switched, the program starts itself again
//! from the running system's path, so the newest one serves the next. A
//! switch made some other way, by hand or by the system's own tools,
//! doesn't restart it, so it also starts again once the system's path
//! holds another program than it was started as and nothing is running
//! or being heard.
//!
//! Each update's ending is also kept in a file the group reads, so a
//! machine's node can say how the last update ended though the switch
//! restarted it before it heard. While one runs, its latest line is kept
//! in another, so a program can tell an update is running, which an ask
//! over the socket doesn't show in the process list.

use crate::config::Config;
use crate::follows::Followed;
use crate::line::{Applied, Line};
use crate::{git, held, run, update};
use std::ffi::CString;
use std::fs;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::Duration;

/// A person's ask.
pub const ASK: &str = "update";

/// What follows a person's ask to name a followed input's commit, as
/// `update at INPUT COMMIT`, for main with that input there rather than at
/// its main's head (src/follows.rs says what the commit must be).
pub const AT: &str = "at";

/// The follower's ask: an update like any other, except that once built
/// its switch waits while someone is at the machine. A person asking
/// meanwhile joins it and ends the wait.
pub const FOLLOW: &str = "follow";

/// What a person sends to stop an update still fetching or building, one
/// a hold was given for too: on 11 November a granted switch compiled the
/// kernel on the laptop, and the stop asked of it was refused.
pub const STOP: &str = "stop";

/// What stops an update still fetching or building once main has moved
/// past the head it builds: on 8 November one built a kernel two hours
/// after a revert made it moot, and every ask joined it. One a hold was
/// given for runs on, since someone waits on its switch.
pub const STOP_FOR_NEWER: &str = "stop-for-newer";

/// How long an asker has to say its ask, and the most it may send.
const ASK_WITHIN: Duration = Duration::from_secs(5);
const ASK_AT_MOST: u64 = 128;

/// How long a finished update waits for its askers to hear the ending
/// before the program starts itself again.
const HEARD_WITHIN: Duration = Duration::from_secs(10);

/// How often the program checks whether the system's path holds another
/// program than it was started as.
const MOVED_EVERY: Duration = Duration::from_secs(5);

/// One update's answer as it grows, heard by every asker who joined it.
#[derive(Default)]
struct Answer {
    state: Mutex<Said>,
    grew: Condvar,
}

#[derive(Default)]
struct Said {
    lines: Vec<String>,
    ended: Option<Line>,
    hearing: usize,
    /// Whether only the follower has asked for it.
    following: bool,
    /// The followed inputs' commits it was asked at, none for their heads.
    at: Followed,
    /// The thread running the update, whose commands a stop ends.
    worker: Option<thread::ThreadId>,
}

/// Serves asks on the config's socket until the program starts again.
pub fn serve(config: Config) -> Result<(), String> {
    let listener = listen(&config)?;
    let config = Arc::new(config);
    let current: Arc<Mutex<Option<Arc<Answer>>>> = Arc::default();
    if let Some(program) = config.program.clone() {
        let started = fs::canonicalize(&program).ok();
        let current = Arc::clone(&current);
        thread::spawn(move || follow_the_system(&program, started.as_deref(), &current));
    }
    for stream in listener.incoming() {
        let Ok(stream) = stream else { continue };
        let config = Arc::clone(&config);
        let current = Arc::clone(&current);
        thread::spawn(move || {
            let _ = answer(&config, &current, &stream);
        });
    }
    Ok(())
}

fn listen(config: &Config) -> Result<UnixListener, String> {
    let socket = &config.socket;
    let named = |e: io::Error| format!("{}: {e}", socket.display());
    match fs::remove_file(socket) {
        Err(e) if e.kind() != io::ErrorKind::NotFound => return Err(named(e)),
        _ => {}
    }
    // Made under another name and moved into place once it's only the
    // group's, so whoever finds the socket finds it as it stays.
    let making = beside(socket);
    let _ = fs::remove_file(&making);
    let listener = UnixListener::bind(&making).map_err(named)?;
    the_groups(&making, &config.group, 0o660)?;
    fs::rename(&making, socket).map_err(named)?;
    Ok(listener)
}

/// Where a file is made before it's moved into place at `path`.
fn beside(path: &Path) -> PathBuf {
    let mut beside = path.as_os_str().to_owned();
    beside.push(".new");
    PathBuf::from(beside)
}

/// Gives `path` to `group` with `mode`. Only the group changes: the file
/// is its maker's, root's when it serves a machine.
fn the_groups(path: &Path, group: &str, mode: u32) -> Result<(), String> {
    let named = |e: io::Error| format!("{}: {e}", path.display());
    let gid = group_id(group)?;
    let c = CString::new(path.as_os_str().as_encoded_bytes()).map_err(|e| e.to_string())?;
    // SAFETY: c is a valid C string.
    if unsafe { libc::chown(c.as_ptr(), libc::uid_t::MAX, gid) } != 0 {
        return Err(named(io::Error::last_os_error()));
    }
    fs::set_permissions(path, fs::Permissions::from_mode(mode)).map_err(named)
}

/// Keeps `ending` in the config's ended file, whole: written beside it,
/// then moved into place, so a reader never finds half of one.
fn keep(config: &Config, ending: &Line) -> Result<(), String> {
    keep_in(config, &config.ended, ending)
}

/// Keeps `line` in `file` for the config's group, whole, as `keep` does.
/// Tells the update's askers `line`, then `keeps` it for other programs:
/// in that order, since a stop asked the moment another program sees the
/// update building reads what the askers were told, to say what it ended.
fn told(heard: &Answer, line: &Line, keeps: impl FnOnce(&Line)) {
    {
        let mut said = heard.state.lock().expect("the answer's lock");
        said.lines.push(line.json());
        heard.grew.notify_all();
    }
    keeps(line);
}

fn keep_in(config: &Config, file: &Path, line: &Line) -> Result<(), String> {
    let making = beside(file);
    let kept = fs::write(&making, format!("{}\n", line.json()))
        .map_err(|e| format!("{}: {e}", making.display()))
        .and_then(|()| the_groups(&making, &config.group, 0o640))
        .and_then(|()| fs::rename(&making, file).map_err(|e| format!("{}: {e}", file.display())));
    if kept.is_err() {
        let _ = fs::remove_file(&making);
    }
    kept
}

fn group_id(name: &str) -> Result<libc::gid_t, String> {
    let c = CString::new(name).map_err(|e| e.to_string())?;
    // SAFETY: c is a valid C string; getgrnam's result is read at once,
    // before anything else could call it.
    let group = unsafe { libc::getgrnam(c.as_ptr()) };
    if group.is_null() {
        return Err(format!("there's no group {name}"));
    }
    // SAFETY: group is non-null and points at getgrnam's static record.
    Ok(unsafe { (*group).gr_gid })
}

fn answer(
    config: &Arc<Config>,
    current: &Arc<Mutex<Option<Arc<Answer>>>>,
    stream: &UnixStream,
) -> io::Result<()> {
    // The account asking, by what the kernel says of the socket's other end.
    let uid = peer_uid(stream);
    let asker = uid.map_or_else(|| "unknown".to_owned(), |uid| uid.to_string());
    stream.set_read_timeout(Some(ASK_WITHIN))?;
    let mut ask = String::new();
    BufReader::new(stream.take(ASK_AT_MOST)).read_line(&mut ask)?;
    let mut out = stream;
    if let Some(said) = held::ask(config, ask.trim(), uid) {
        // A pause stops what the follower alone asked for too, so it
        // switches nothing after it; what a person asked for runs on.
        let why = match said {
            Ok(said) if ask.trim() == held::PAUSE => {
                format!("{}; {said}", stop(current, Asked::ByTheFollower))
            }
            Ok(said) => said,
            Err(why) => why,
        };
        eprintln!("uid {asker} asks to {}: {why}", ask.trim());
        let answered = Line::Refused {
            commit: update::last(config),
            why,
        };
        return writeln!(out, "{}", answered.json());
    }
    if ask.trim() == STOP || ask.trim() == STOP_FOR_NEWER {
        eprintln!("uid {asker} asks to {}", ask.trim());
        let by = if ask.trim() == STOP {
            Asked::ByAPerson
        } else {
            Asked::ForANewerHead
        };
        let stopped = Line::Refused {
            commit: update::last(config),
            why: stop(current, by),
        };
        return writeln!(out, "{}", stopped.json());
    }
    let following = ask.trim() == FOLLOW;
    let at = if following {
        Some(Ok(Followed::new()))
    } else {
        asked_at(ask.trim())
    };
    let at = match at {
        Some(Ok(at)) => at,
        Some(Err(why)) => {
            eprintln!("refused an ask from uid {asker}: {why}");
            let refused = Line::Refused {
                commit: update::last(config),
                why,
            };
            return writeln!(out, "{}", refused.json());
        }
        None => {
            eprintln!("refused an ask from uid {asker}: {:?}", ask.trim());
            let refused = Line::Refused {
                commit: update::last(config),
                why: format!(
                    "the only asks are `{ASK}`, `{ASK} {AT} INPUT COMMIT`, `{FOLLOW}`, `{STOP}`, \
                     `{STOP_FOR_NEWER}`, `{}`, `{}`, `{} COMMIT` and `{} COMMIT`",
                    held::PAUSE,
                    held::RESUME,
                    held::HOLD,
                    held::LIFT
                ),
            };
            return writeln!(out, "{}", refused.json());
        }
    };
    eprintln!("uid {asker} asks for an update{}", at_said(&at));
    // An asker that stops reading is let go rather than kept waiting on.
    stream.set_write_timeout(Some(HEARD_WITHIN))?;
    let heard = match join(config, current, following, &at) {
        Ok(heard) => heard,
        Err(why) => {
            let refused = Line::Refused {
                commit: update::last(config),
                why,
            };
            return writeln!(out, "{}", refused.json());
        }
    };
    heard.state.lock().expect("the answer's lock").hearing += 1;
    let result = hear(&heard, &mut out);
    heard.state.lock().expect("the answer's lock").hearing -= 1;
    heard.grew.notify_all();
    result
}

/// Writes an answer's lines to an asker as they come, until its ending.
/// The lock is never held while writing, so a slow asker slows only
/// itself.
fn hear(heard: &Answer, out: &mut impl Write) -> io::Result<()> {
    let mut next = 0;
    loop {
        let (pending, ended) = {
            let mut said = heard.state.lock().expect("the answer's lock");
            while next == said.lines.len() && said.ended.is_none() {
                said = heard.grew.wait(said).expect("the answer's lock");
            }
            (said.lines[next..].to_vec(), said.ended.is_some())
        };
        for line in &pending {
            writeln!(out, "{line}")?;
        }
        next += pending.len();
        // The ending is pushed with `ended` set, under one lock, so an
        // answer seen ended was seen whole.
        if ended {
            return Ok(());
        }
    }
}

/// Which updates a stop ends: a person's any, even one a hold was given
/// for; one for a newer head any but those; and a pause only one the
/// follower alone asked for.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Asked {
    ByAPerson,
    ForANewerHead,
    ByTheFollower,
}

/// Stops the running update while it fetches or builds, saying what it
/// did. One switching runs on, since a switch cut short leaves the machine
/// between two systems.
fn stop(current: &Mutex<Option<Arc<Answer>>>, asked: Asked) -> String {
    let current = current.lock().expect("the current update's lock");
    let Some(running) = current.as_ref() else {
        return "no update is running, so nothing was stopped".to_owned();
    };
    let said = running.state.lock().expect("the answer's lock");
    let (None, Some(worker)) = (&said.ended, said.worker) else {
        return "no update is running, so nothing was stopped".to_owned();
    };
    if asked == Asked::ByTheFollower && !said.following {
        return "the update running was asked for by a person, so it runs on".to_owned();
    }
    let latest = said
        .lines
        .last()
        .and_then(|line| serde_json::from_str::<Line>(line).ok());
    let granted = if asked == Asked::ByAPerson {
        run::Granted::Ends
    } else {
        run::Granted::RunsOn
    };
    let was_granted = run::was_granted(worker);
    let stopped = match latest {
        Some(Line::Switching { commit }) => {
            return format!("the update is switching to {commit}, so it runs on");
        }
        _ if run::stop(worker, granted).is_none() => {
            return "the update switches what a hold was given for, so it runs on".to_owned();
        }
        Some(Line::Building { commit }) => format!("stopped the update building {commit}"),
        _ => "stopped the update".to_owned(),
    };
    if was_granted {
        format!("{stopped}; the hold given for it is spent, so its switch needs one given again")
    } else {
        stopped
    }
}

/// The followed inputs' commits a person's ask names: none for `update`,
/// one for `update at INPUT COMMIT`, and why for an `at` without both;
/// nothing when the ask is another.
fn asked_at(ask: &str) -> Option<Result<Followed, String>> {
    match ask.split_whitespace().collect::<Vec<_>>().as_slice() {
        [ASK] => Some(Ok(Followed::new())),
        [ASK, AT, name, commit] if held::is_commit(commit) => Some(Ok(Followed::from([(
            (*name).to_owned(),
            commit.to_ascii_lowercase(),
        )]))),
        [ASK, AT, ..] => Some(Err(format!(
            "`{ASK} {AT}` takes a followed input's name and one commit's full id"
        ))),
        _ => None,
    }
}

/// The commits an update was asked at, as its line in the journal ends.
fn at_said(at: &Followed) -> String {
    at.iter()
        .map(|(name, commit)| format!(" with {name} at {}", git::short(commit)))
        .collect::<Vec<_>>()
        .concat()
}

/// The update running now, or a new one started for this ask. An ask
/// joins only one asked at the same commits, since joining another would
/// hear it switch to what it didn't ask for; it's refused, saying what
/// runs.
fn join(
    config: &Arc<Config>,
    current: &Arc<Mutex<Option<Arc<Answer>>>>,
    following: bool,
    at: &Followed,
) -> Result<Arc<Answer>, String> {
    let mut current = current.lock().expect("the current update's lock");
    if let Some(running) = current.as_ref() {
        let mut said = running.state.lock().expect("the answer's lock");
        if said.ended.is_none() {
            if said.at != *at {
                let runs = if said.at.is_empty() {
                    " at its inputs' heads".to_owned()
                } else {
                    at_said(&said.at)
                };
                return Err(format!(
                    "an update{runs} is running, and an ask joins only one asked at the \
                     same commits; `update-to-main {STOP}` ends it"
                ));
            }
            said.following &= following;
            return Ok(Arc::clone(running));
        }
    }
    let heard = Arc::new(Answer::default());
    {
        let mut said = heard.state.lock().expect("the answer's lock");
        said.following = following;
        said.at = at.clone();
    }
    *current = Some(Arc::clone(&heard));
    let config = Arc::clone(config);
    let growing = Arc::clone(&heard);
    thread::spawn(move || run(&config, &growing));
    Ok(heard)
}

fn run(config: &Config, heard: &Answer) {
    heard.state.lock().expect("the answer's lock").worker = Some(thread::current().id());
    let push = |line: &Line| {
        told(heard, line, |line| {
            // Not keeping it costs only another program's sight of it.
            if let Err(why) = keep_in(config, &config.running, line) {
                eprintln!("couldn't keep what the update is doing: {why}");
            }
        });
    };
    let following = || heard.state.lock().expect("the answer's lock").following;
    let at = heard.state.lock().expect("the answer's lock").at.clone();
    let ending = update::update_at(config, &following, &at, &mut |line| push(line));
    run::ended();
    eprintln!("{}", ending.json());
    let _ = fs::remove_file(&config.running);
    // Kept before the askers hear it, so one that reads the file after the
    // ending finds it there. Not keeping it costs only the node's word.
    if let Err(why) = keep(config, &ending) {
        eprintln!("couldn't keep how the update ended: {why}");
    }
    let switched = matches!(
        ending,
        Line::Done {
            applied: Applied::Now,
            ..
        }
    );
    {
        let mut said = heard.state.lock().expect("the answer's lock");
        said.lines.push(ending.json());
        said.ended = Some(ending);
        heard.grew.notify_all();
    }
    if let (true, Some(program)) = (switched, &config.program) {
        start_again(program, heard);
    }
}

/// Waits for the askers to hear the ending, then becomes the running
/// system's update-to-main, with the same arguments.
fn start_again(program: &Path, heard: &Answer) {
    let said = heard.state.lock().expect("the answer's lock");
    let _ = heard
        .grew
        .wait_timeout_while(said, HEARD_WITHIN, |s| s.hearing > 0);
    eprintln!(
        "starting {} again, from the system it switched to",
        program.display()
    );
    let error = Command::new(program)
        .args(std::env::args_os().skip(1))
        .exec();
    eprintln!(
        "couldn't start {} again ({error}); this one serves on",
        program.display()
    );
}

/// Starts the program again from the system's path once that holds
/// another program than `started`, and no update is running or being
/// heard. The current update's lock is held to the end, so no ask joins
/// between looking and starting. If it can't start, this one serves on and
/// stops looking, rather than trying again every few seconds.
fn follow_the_system(program: &Path, started: Option<&Path>, current: &Mutex<Option<Arc<Answer>>>) {
    loop {
        thread::sleep(MOVED_EVERY);
        if !moved(program, started) {
            continue;
        }
        let current = current.lock().expect("the current update's lock");
        if !current.as_deref().is_none_or(idle) {
            continue;
        }
        eprintln!(
            "{} is another program now, so starting it",
            program.display()
        );
        let error = Command::new(program)
            .args(std::env::args_os().skip(1))
            .exec();
        eprintln!(
            "couldn't start {} again ({error}); this one serves on",
            program.display()
        );
        return;
    }
}

/// Whether the system's path to the program leads to another than
/// `started`. A path that leads nowhere now, as while a system is half
/// made, hasn't moved; one that led nowhere when it started has, once it
/// leads somewhere.
fn moved(program: &Path, started: Option<&Path>) -> bool {
    fs::canonicalize(program).is_ok_and(|now| Some(now.as_path()) != started)
}

/// Whether an update has ended and every asker has heard it.
fn idle(update: &Answer) -> bool {
    let said = update.state.lock().expect("the answer's lock");
    said.ended.is_some() && said.hearing == 0
}

#[cfg(target_os = "linux")]
fn peer_uid(stream: &UnixStream) -> Option<libc::uid_t> {
    use std::os::fd::AsRawFd;
    let mut cred = libc::ucred {
        pid: 0,
        uid: 0,
        gid: 0,
    };
    let mut len = libc::socklen_t::try_from(std::mem::size_of::<libc::ucred>())
        .expect("a ucred's size fits a socklen_t");
    // SAFETY: cred and len are valid for getsockopt to write.
    let done = unsafe {
        libc::getsockopt(
            stream.as_raw_fd(),
            libc::SOL_SOCKET,
            libc::SO_PEERCRED,
            (&raw mut cred).cast(),
            &raw mut len,
        )
    };
    (done == 0).then_some(cred.uid)
}

#[cfg(not(target_os = "linux"))]
fn peer_uid(stream: &UnixStream) -> Option<libc::uid_t> {
    use std::os::fd::AsRawFd;
    let mut uid = 0;
    let mut gid = 0;
    // SAFETY: uid and gid are valid for getpeereid to write.
    let done = unsafe { libc::getpeereid(stream.as_raw_fd(), &raw mut uid, &raw mut gid) };
    (done == 0).then_some(uid)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;

    #[test]
    fn an_ask_at_a_commit_names_one_input_and_its_full_id() {
        let commit = "B2BE6AE26C8FCB7F68B687145BAFCBCEEC102640";
        assert_eq!(
            asked_at(&format!("update at core {commit}")),
            Some(Ok(Followed::from([(
                "core".to_owned(),
                commit.to_ascii_lowercase()
            )]))),
            "an ask at core's commit wasn't read as one"
        );
        assert_eq!(
            asked_at("update"),
            Some(Ok(Followed::new())),
            "a plain ask named a commit"
        );
        for ask in ["update at", "update at core", "update at core b2be6ae26c8f"] {
            assert!(
                asked_at(ask).is_some_and(|at| at.is_err()),
                "{ask:?} wasn't refused"
            );
        }
        for ask in ["update /nix/store/something", "follow", "stop"] {
            assert_eq!(asked_at(ask), None, "{ask:?} was taken as an update");
        }
    }

    #[test]
    fn the_program_moves_when_the_systems_path_leads_to_another() {
        let dir = tempfile::tempdir().unwrap();
        let (old, new, path) = (
            dir.path().join("old"),
            dir.path().join("new"),
            dir.path().join("path"),
        );
        fs::write(&old, "").unwrap();
        fs::write(&new, "").unwrap();
        symlink(&old, &path).unwrap();
        let started = fs::canonicalize(&path).unwrap();
        assert!(
            !moved(&path, Some(&started)),
            "still the one it was started as"
        );
        fs::remove_file(&path).unwrap();
        assert!(
            !moved(&path, Some(&started)),
            "a path leading nowhere hasn't moved"
        );
        symlink(&new, &path).unwrap();
        assert!(
            moved(&path, Some(&started)),
            "a switch made by hand moves it"
        );
        assert!(
            moved(&path, None),
            "one that led nowhere at the start moves once it leads somewhere"
        );
    }

    #[test]
    fn what_an_update_does_is_told_before_another_program_can_see_it() {
        // On 12 November a person's stop, asked the moment the running
        // file said building, once answered as though nothing were built.
        let update = Answer::default();
        let building = Line::Building {
            commit: "one".to_owned(),
        };
        told(&update, &building, |line| {
            let said = update.state.lock().unwrap();
            assert_eq!(
                said.lines.last(),
                Some(&line.json()),
                "another program saw what the update does before its askers were told"
            );
        });
    }

    #[test]
    fn only_an_update_ended_and_heard_lets_the_program_start_again() {
        let update = Answer::default();
        assert!(!idle(&update), "running");
        update.state.lock().unwrap().ended = Some(Line::Refused {
            commit: None,
            why: String::new(),
        });
        update.state.lock().unwrap().hearing = 1;
        assert!(!idle(&update), "ended, but an asker is still hearing it");
        update.state.lock().unwrap().hearing = 0;
        assert!(idle(&update), "ended and heard");
    }
}
