//! The socket an ask comes in on. Only root and the config's group may
//! write it, so the kernel decides who may ask. An ask is one line,
//! `update`, and anything else is refused. An ask while an update runs
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
//! restarted it before it heard.

use crate::config::Config;
use crate::line::{Applied, Line};
use crate::update;
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

/// The one ask.
pub const ASK: &str = "update";

/// How long an asker has to say its ask, and the most it may send.
const ASK_WITHIN: Duration = Duration::from_secs(5);
const ASK_AT_MOST: u64 = 64;

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
    let file = &config.ended;
    let making = beside(file);
    let kept = fs::write(&making, format!("{}\n", ending.json()))
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
    let asker = asker(stream);
    stream.set_read_timeout(Some(ASK_WITHIN))?;
    let mut ask = String::new();
    BufReader::new(stream.take(ASK_AT_MOST)).read_line(&mut ask)?;
    let mut out = stream;
    if ask.trim() != ASK {
        eprintln!("refused an ask from uid {asker}: {:?}", ask.trim());
        let refused = Line::Refused {
            commit: update::last(config),
            why: format!("the only ask is `{ASK}`"),
        };
        return writeln!(out, "{}", refused.json());
    }
    eprintln!("uid {asker} asks for an update");
    // An asker that stops reading is let go rather than kept waiting on.
    stream.set_write_timeout(Some(HEARD_WITHIN))?;
    let heard = join(config, current);
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

/// The update running now, or a new one started for this ask.
fn join(config: &Arc<Config>, current: &Arc<Mutex<Option<Arc<Answer>>>>) -> Arc<Answer> {
    let mut current = current.lock().expect("the current update's lock");
    if let Some(running) = current.as_ref()
        && running
            .state
            .lock()
            .expect("the answer's lock")
            .ended
            .is_none()
    {
        return Arc::clone(running);
    }
    let heard = Arc::new(Answer::default());
    *current = Some(Arc::clone(&heard));
    let config = Arc::clone(config);
    let growing = Arc::clone(&heard);
    thread::spawn(move || run(&config, &growing));
    heard
}

fn run(config: &Config, heard: &Answer) {
    let push = |line: &Line| {
        let mut said = heard.state.lock().expect("the answer's lock");
        said.lines.push(line.json());
        heard.grew.notify_all();
    };
    let ending = update::update(config, &mut |line| push(line));
    eprintln!("{}", ending.json());
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

/// The account asking, by what the kernel says of the socket's other end.
fn asker(stream: &UnixStream) -> String {
    peer_uid(stream).map_or_else(|| "unknown".to_owned(), |uid| uid.to_string())
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
