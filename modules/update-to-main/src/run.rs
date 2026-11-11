//! Running the programs an update needs, with nothing of the environment
//! root was started in: no git configuration but what's given here, no
//! prompts, and a fixed path, so what runs depends only on the config.

use std::collections::HashMap;
use std::ffi::{CStr, CString, OsStr};
use std::io::{BufRead, BufReader, Read};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, LazyLock, Mutex, MutexGuard, PoisonError};
use std::thread::{self, ThreadId};
use std::time::{Duration, Instant};

use crate::progress;

/// By the thread running each update, the process groups of the commands
/// it runs now and whether it's being stopped, so a stop can end what runs
/// and start nothing more. An update runs its commands on its own thread,
/// or on a helper's, which a stop of the update stops too.
static RUNNING: LazyLock<Mutex<HashMap<ThreadId, Running>>> = LazyLock::new(Mutex::default);

#[derive(Default)]
struct Running {
    groups: Vec<libc::pid_t>,
    stopping: bool,
    granted: bool,
    /// The threads that run commands for this update beside its own.
    helpers: Vec<ThreadId>,
}

fn running() -> MutexGuard<'static, HashMap<ThreadId, Running>> {
    RUNNING.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Whether a stop ends an update a hold was given for too.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Granted {
    Ends,
    RunsOn,
}

/// Ends every command the update on `thread` runs now, each with its
/// whole process group, and lets it start none until it has `ended`, so
/// the update fails at once. Says how many it ended; none when a hold was
/// given for it and `granted` lets it run on.
pub fn stop(thread: ThreadId, granted: Granted) -> Option<usize> {
    stop_in(&mut running(), thread, granted)
}

fn stop_in(
    running: &mut HashMap<ThreadId, Running>,
    thread: ThreadId,
    granted: Granted,
) -> Option<usize> {
    let update = running.entry(thread).or_default();
    if update.granted && granted == Granted::RunsOn {
        return None;
    }
    update.stopping = true;
    for group in &update.groups {
        // SAFETY: kill only sends a signal; a group already gone is ESRCH.
        unsafe { libc::kill(-group, libc::SIGTERM) };
    }
    let mut ended = update.groups.len();
    for helper in update.helpers.clone() {
        ended += stop_in(running, helper, granted).unwrap_or_default();
    }
    Some(ended)
}

/// Makes this thread a helper of the update on `update`, running commands
/// beside it, so a stop of the update stops it too; stopped already if
/// the update is. Its own stop, by its thread's id, stops it alone.
pub fn helps(update: ThreadId) {
    let me = thread::current().id();
    let mut running = running();
    let owner = running.entry(update).or_default();
    owner.helpers.push(me);
    let stopping = owner.stopping;
    running.entry(me).or_default().stopping |= stopping;
}

/// Forgets a helper of the update on `update` once it has ended.
pub fn helped(update: ThreadId, helper: ThreadId) {
    let mut running = running();
    running.remove(&helper);
    if let Some(owner) = running.get_mut(&update) {
        owner.helpers.retain(|h| *h != helper);
    }
}

/// Marks this thread's update as one a hold was given for, which a stop
/// for a newer head or a pause leaves to run, since someone waits on its
/// switch and a newer head would only start it over. A person's stop
/// still ends it.
pub fn granted() {
    running().entry(thread::current().id()).or_default().granted = true;
}

/// Whether a hold was given for the update on `thread`.
pub fn was_granted(thread: ThreadId) -> bool {
    running().get(&thread).is_some_and(|update| update.granted)
}

/// Whether this thread's update is being stopped.
pub fn stopping() -> bool {
    running()
        .get(&thread::current().id())
        .is_some_and(|update| update.stopping)
}

/// Forgets this thread's update once it has ended.
pub fn ended() {
    running().remove(&thread::current().id());
}

/// The path every command starts with: the system's own programs.
pub const PATH: &str = "/run/current-system/sw/bin:/usr/bin:/bin:/usr/sbin:/sbin";

/// How many of a failed command's last lines an answer carries.
pub const TAIL: usize = 20;

/// A command that didn't succeed: what it was, and the end of what it said.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Failure {
    pub why: String,
    pub lines: Vec<String>,
}

impl Failure {
    pub fn new(why: impl Into<String>) -> Failure {
        Failure {
            why: why.into(),
            lines: Vec::new(),
        }
    }
}

/// A command with a clean environment: `home` for programs that want one,
/// and a path that finds the system's own programs.
pub fn command(program: &Path, home: &Path) -> Command {
    let mut command = Command::new(program);
    command
        .env_clear()
        .env("HOME", home)
        .env("PATH", PATH)
        .env("LANG", "C.UTF-8")
        // Nix builds through the machine's daemon, as anyone else's do, so
        // a build takes its builders, its place and its limits, rather than
        // root building beside it.
        .env("NIX_REMOTE", "daemon")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_TERMINAL_PROMPT", "0")
        .stdin(Stdio::null());
    // The system's certificates, where both NixOS and nix-darwin keep
    // them, for git's and Nix's fetches over HTTPS.
    if Path::new(CERTIFICATES).exists() {
        command
            .env("SSL_CERT_FILE", CERTIFICATES)
            .env("NIX_SSL_CERT_FILE", CERTIFICATES);
    }
    command
}

const CERTIFICATES: &str = "/etc/ssl/certs/ca-certificates.crt";

/// An account a command runs as.
#[derive(Clone, Debug)]
pub struct Account {
    name: CString,
    uid: libc::uid_t,
    gid: libc::gid_t,
    home: PathBuf,
}

impl Account {
    pub fn uid(&self) -> libc::uid_t {
        self.uid
    }

    pub fn gid(&self) -> libc::gid_t {
        self.gid
    }
}

/// Looks an account up by its name.
pub fn account(name: &str) -> Result<Account, Failure> {
    let unknown = || Failure::new(format!("there's no account named {name}"));
    let c_name = CString::new(name).map_err(|_| unknown())?;
    let mut record = zeroed_passwd();
    let mut buffer = vec![0 as libc::c_char; 16 * 1024];
    let mut found: *mut libc::passwd = std::ptr::null_mut();
    // SAFETY: every pointer is to memory this frame owns, sized as given,
    // and the record's strings point into `buffer`, read before it goes.
    let status = unsafe {
        libc::getpwnam_r(
            c_name.as_ptr(),
            &raw mut record,
            buffer.as_mut_ptr(),
            buffer.len(),
            &raw mut found,
        )
    };
    if status != 0 || found.is_null() {
        return Err(unknown());
    }
    // SAFETY: getpwnam_r found the account, so pw_dir is a C string in
    // `buffer`.
    let home = unsafe { CStr::from_ptr(record.pw_dir) };
    Ok(Account {
        name: c_name,
        uid: record.pw_uid,
        gid: record.pw_gid,
        home: PathBuf::from(home.to_string_lossy().into_owned()),
    })
}

fn zeroed_passwd() -> libc::passwd {
    // SAFETY: passwd is plain data, for which all zeroes is a valid value:
    // null pointers and zero numbers, all overwritten by getpwnam_r.
    unsafe { std::mem::zeroed() }
}

/// A command run as `account` in `dir`: its own groups, its own home, so
/// its own logins, `path` to find programs in, and nothing else of root's
/// environment.
pub fn command_as(program: &OsStr, account: &Account, path: &[PathBuf], dir: &Path) -> Command {
    let mut command = Command::new(program);
    command
        .env_clear()
        .env("HOME", &account.home)
        .env("USER", account.name.to_string_lossy().as_ref())
        .env("LOGNAME", account.name.to_string_lossy().as_ref())
        .env("PATH", std::env::join_paths(path).unwrap_or_default())
        .env("LANG", "C.UTF-8")
        .env("NIX_REMOTE", "daemon")
        .env("GIT_TERMINAL_PROMPT", "0")
        .current_dir(dir)
        .stdin(Stdio::null());
    if Path::new(CERTIFICATES).exists() {
        command
            .env("SSL_CERT_FILE", CERTIFICATES)
            .env("NIX_SSL_CERT_FILE", CERTIFICATES);
    }
    let runtime = PathBuf::from(format!("/run/user/{}", account.uid));
    if runtime.is_dir() {
        command.env("XDG_RUNTIME_DIR", runtime);
    }
    // Root becomes the account, its groups first, as a login would; anyone
    // else runs it as themselves, which is all they can do.
    let (name, uid, gid) = (account.name.clone(), account.uid, account.gid);
    // SAFETY: what runs between fork and exec is geteuid and the calls a
    // child makes to change who it is, with nothing allocated; initgroups
    // reads the group database, as login programs do at this point.
    unsafe {
        command.pre_exec(move || {
            if libc::geteuid() != 0 || uid == 0 {
                return Ok(());
            }
            #[cfg(target_os = "macos")]
            let gid_arg = gid.cast_signed();
            #[cfg(not(target_os = "macos"))]
            let gid_arg = gid;
            if libc::initgroups(name.as_ptr(), gid_arg) != 0
                || libc::setgid(gid) != 0
                || libc::setuid(uid) != 0
            {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    command
}

/// Runs a command to its end, giving what it printed, or a failure naming
/// `what` with the last lines of everything it said.
pub fn output<I, S>(command: Command, what: &str, args: I) -> Result<String, Failure>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    ran(command, what, args, false)
}

/// Runs a command to its end as `output` does, but giving everything it
/// said on both its streams, for one that reports on its error stream.
pub fn said<I, S>(command: Command, what: &str, args: I) -> Result<String, Failure>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    ran(command, what, args, true)
}

fn ran<I, S>(mut command: Command, what: &str, args: I, errors_too: bool) -> Result<String, Failure>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    // Each runs in a process group of its own, so a stop ends what it
    // started too. Its input is what the caller gave, or the daemon's own,
    // which systemd and launchd make empty.
    command
        .args(args)
        .process_group(0)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let me = thread::current().id();
    let stopped = || Failure::new(format!("{what}: the update was stopped"));
    let child = {
        let mut running = running();
        let update = running.entry(me).or_default();
        if update.stopping {
            return Err(stopped());
        }
        let child = command
            .spawn()
            .map_err(|e| Failure::new(format!("{what}: {e}")))?;
        update.groups.push(child.id().cast_signed());
        child
    };
    let group = child.id().cast_signed();
    let out = child.wait_with_output();
    {
        let mut running = running();
        let update = running.entry(me).or_default();
        update.groups.retain(|g| *g != group);
        if update.stopping {
            return Err(stopped());
        }
    }
    let out = out.map_err(|e| Failure::new(format!("{what}: {e}")))?;
    if out.status.success() {
        let mut said = String::from_utf8_lossy(&out.stdout).into_owned();
        if errors_too {
            said.push_str(&String::from_utf8_lossy(&out.stderr));
        }
        return Ok(said.trim().to_owned());
    }
    let mut said = String::from_utf8_lossy(&out.stdout).into_owned();
    said.push_str(&String::from_utf8_lossy(&out.stderr));
    Err(Failure {
        why: format!("{what} failed ({})", out.status),
        lines: tail(&said),
    })
}

/// Runs a build to its end as `said` does, but stops it, with its process
/// group, once it has said nothing for `silent`, giving a failure that
/// says so. Given `-L`, nix says each build's log as it goes, so a build
/// that goes quiet that long is one waiting on a builder that doesn't
/// answer: on 8 November one waited 20 minutes on a lent builder whose
/// tunnel reset every 15 seconds, with the machine idle. The log's own
/// lines, which nix starts with the piece's name and `> `, count as
/// something said but are left out of what's given, which only needs
/// nix's own. While nix's newest line says it is copying a path back
/// from a builder, or building a piece, it may stay quiet `COPYING` times
/// as long, since a copy says nothing until it ends and a compile may
/// say nothing for minutes as it works: on 8 November a switch was
/// stopped four minutes into copying two pieces back, and on 9 November
/// another 84 minutes in, while ONNX Runtime compiled quietly. Quiet
/// alone isn't stuck, so a build past its limit is stopped only if its
/// pieces did no work since the last look either (`progress::done`), and
/// never runs quiet past `PATIENT_MOST` limits whatever they did, since a
/// builder's figures count other builds than this one's.
pub fn built<I, S>(
    command: Command,
    what: &str,
    args: I,
    silent: Duration,
) -> Result<String, Failure>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let roots = progress::Roots::machine();
    built_watching(command, what, args, silent, &|group| {
        progress::done(group, &roots)
    })
}

/// `built`, reading the work its pieces have done with `done`, given
/// the build's process group.
fn built_watching<I, S>(
    mut command: Command,
    what: &str,
    args: I,
    silent: Duration,
    done: &dyn Fn(libc::pid_t) -> u64,
) -> Result<String, Failure>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    command
        .args(args)
        .process_group(0)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let me = thread::current().id();
    let stopped = || Failure::new(format!("{what}: the update was stopped"));
    let mut child = {
        let mut running = running();
        let update = running.entry(me).or_default();
        if update.stopping {
            return Err(stopped());
        }
        let child = command
            .spawn()
            .map_err(|e| Failure::new(format!("{what}: {e}")))?;
        update.groups.push(child.id().cast_signed());
        child
    };
    let group = child.id().cast_signed();
    let heard: Heard = Arc::new(Mutex::new((Instant::now(), String::new(), false)));
    let readers: Vec<_> = [
        child
            .stdout
            .take()
            .map(|out| Box::new(out) as Box<dyn Read + Send>),
        child
            .stderr
            .take()
            .map(|err| Box::new(err) as Box<dyn Read + Send>),
    ]
    .into_iter()
    .flatten()
    .map(|stream| {
        let heard = Arc::clone(&heard);
        thread::spawn(move || {
            for line in BufReader::new(stream).lines().map_while(Result::ok) {
                let mut heard = heard.lock().unwrap_or_else(PoisonError::into_inner);
                heard.0 = Instant::now();
                if !is_log(&line) {
                    heard.2 = is_copy_back(&line) || is_building(&line);
                    heard.1.push_str(&line);
                    heard.1.push('\n');
                }
            }
        })
    })
    .collect();
    let (status, quiet) = watched(&mut child, &heard, silent, done);
    for reader in readers {
        let _ = reader.join();
    }
    {
        let mut running = running();
        let update = running.entry(me).or_default();
        update.groups.retain(|g| *g != group);
        if update.stopping {
            return Err(stopped());
        }
    }
    let said = std::mem::take(&mut heard.lock().unwrap_or_else(PoisonError::into_inner).1);
    let status = status.map_err(|e| Failure::new(format!("{what}: {e}")))?;
    if let Some((quiet, worked)) = quiet {
        return Err(Failure {
            why: format!(
                "{what} said nothing for {} minutes{}, so it was stopped",
                quiet.as_secs() / 60,
                if worked {
                    ""
                } else {
                    " and its pieces did no work"
                }
            ),
            lines: tail(&said),
        });
    }
    if status.success() {
        return Ok(said.trim().to_owned());
    }
    Err(Failure {
        why: format!("{what} failed ({status})"),
        lines: tail(&said),
    })
}

/// What nix has said, as `built` hears it: when it last said anything,
/// what it said of its own, and whether its newest line has it building
/// or copying back.
type Heard = Arc<Mutex<(Instant, String, bool)>>;

/// Waits for `child` to end, stopping its group once it is quiet past
/// its limit with its pieces doing no work, or past the most whatever
/// they do. Gives how it ended and, if it was stopped, how long it had
/// been quiet and whether its pieces still worked.
fn watched(
    child: &mut std::process::Child,
    heard: &Heard,
    silent: Duration,
    done: &dyn Fn(libc::pid_t) -> u64,
) -> (
    std::io::Result<std::process::ExitStatus>,
    Option<(Duration, bool)>,
) {
    let group = child.id().cast_signed();
    let mut quiet = None;
    let mut seen = 0;
    // When the pieces' work last started the clock again.
    let mut working: Option<Instant> = None;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return (Ok(status), quiet),
            Ok(None) => {}
            Err(e) => return (Err(e), quiet),
        }
        let (last, patient) = {
            let heard = heard.lock().unwrap_or_else(PoisonError::into_inner);
            (heard.0, heard.2)
        };
        let limit = if patient { silent * COPYING } else { silent };
        let clock = working.map_or(last, |working| working.max(last));
        if quiet.is_none() && clock.elapsed() >= limit {
            let now = done(group);
            let moved = now != seen;
            seen = now;
            if moved && last.elapsed() < limit * PATIENT_MOST {
                // Its pieces work, so the clock starts again, but the
                // quiet spell it is in still counts towards the most.
                working = Some(Instant::now());
            } else {
                // SAFETY: kill only sends a signal; a group already gone is ESRCH.
                unsafe { libc::kill(-group, libc::SIGTERM) };
                quiet = Some((last.elapsed(), moved));
            }
        }
        thread::sleep(Duration::from_millis(200));
    }
}

/// How many times as long a copy back from a builder may stay quiet as a
/// build may.
const COPYING: u32 = 3;

/// How many limits a build may run quiet, its pieces working, before it
/// is stopped all the same: an hour for a twelve-minute limit, where a
/// kernel or a browser says something every few seconds.
const PATIENT_MOST: u32 = 5;

/// Whether a line is nix starting to copy a path back from a builder,
/// as in `copying path '/nix/store/...' from 'ssh-ng://...'...`.
fn is_copy_back(line: &str) -> bool {
    line.starts_with("copying path ") && line.contains(" from '")
}

/// Whether a line is nix starting to build a piece, here or on a builder,
/// as in `building '/nix/store/...drv' on 'ssh-ng://...'...`.
fn is_building(line: &str) -> bool {
    line.starts_with("building '/nix/store/")
}

/// Whether a line is a build's own log, as nix's `-L` gives it: the
/// piece's name, then `> `.
fn is_log(line: &str) -> bool {
    line.split_once("> ").is_some_and(|(name, _)| {
        !name.is_empty()
            && name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "+-._".contains(c))
    })
}

/// The lines of what a command said that tell why it failed, blank ones
/// left out: from Nix's first error that is a piece's own, or else the
/// last lines.
pub fn tail(said: &str) -> Vec<String> {
    let lines: Vec<&str> = said.lines().filter(|l| !l.trim().is_empty()).collect();
    let from = first_cause(&lines).unwrap_or_else(|| lines.len().saturating_sub(TAIL));
    lines[from..]
        .iter()
        .take(TAIL)
        .map(|l| (*l).to_owned())
        .collect()
}

/// Where the first error that isn't only another piece's failing starts.
/// Nix gives each error as an `error:` line and the indented lines under
/// it, the piece that failed first and then every piece waiting on it, so
/// on 9 November a system's last 20 lines named five waiting pieces and
/// never the one that failed, for ten updates in a row.
fn first_cause(lines: &[&str]) -> Option<usize> {
    lines
        .iter()
        .enumerate()
        .filter(|(_, line)| line.starts_with("error:"))
        .map(|(start, _)| start)
        .find(|&start| {
            let under = lines[start + 1..]
                .iter()
                .take_while(|line| line.starts_with(char::is_whitespace));
            !std::iter::once(&lines[start])
                .chain(under)
                .copied()
                .any(is_waiting)
        })
}

/// Whether a line of a Nix error says its piece failed only because one
/// it needs did, in the words of Nix before 2.29 and after.
fn is_waiting(line: &str) -> bool {
    let line = line.trim();
    (line.starts_with("Reason:") && line.contains(" failed") && line.contains("dependenc"))
        || (line.starts_with("error:") && line.contains("dependencies of derivation"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_build_that_goes_quiet_is_stopped() {
        let mut sh = Command::new(std::env::var_os("BASH").unwrap_or_else(|| "sh".into()));
        sh.args([
            "-c",
            "echo 'building x on y'; echo 'x> compiling'; sleep 30",
        ]);
        let started = Instant::now();
        let failed = built(
            sh,
            "building",
            std::iter::empty::<&str>(),
            Duration::from_secs(1),
        )
        .unwrap_err();
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "it waited for the end"
        );
        assert_eq!(
            failed.why,
            "building said nothing for 0 minutes and its pieces did no work, so it was stopped"
        );
        assert_eq!(
            failed.lines,
            ["building x on y"],
            "the log's lines are left out"
        );
    }

    #[test]
    fn a_quiet_build_whose_pieces_work_runs_on_but_not_for_ever() {
        let quiet = |then: &str, done: &dyn Fn(libc::pid_t) -> u64| {
            let mut sh = Command::new(std::env::var_os("BASH").unwrap_or_else(|| "sh".into()));
            sh.args([
                "-c",
                &format!("echo 'these 2 derivations will be built:'; {then}"),
            ]);
            built_watching(
                sh,
                "building",
                std::iter::empty::<&str>(),
                Duration::from_millis(500),
                done,
            )
        };
        let work = std::sync::atomic::AtomicU64::new(0);
        let working = |_| work.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
        assert!(quiet("sleep 1.6; echo done", &working).is_ok());
        // Its pieces stopped working, so it's stopped at the next look.
        let started = Instant::now();
        let failed = quiet("sleep 30", &|_| 7).unwrap_err();
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "it waited for the end"
        );
        assert!(failed.why.contains("did no work"), "{}", failed.why);
        // Working however long, it's quiet no more than the most.
        let started = Instant::now();
        let failed = quiet("sleep 30", &working).unwrap_err();
        let took = started.elapsed();
        assert!(
            took >= Duration::from_millis(2500) && took < Duration::from_secs(10),
            "{took:?}"
        );
        assert!(failed.why.contains("said nothing for"), "{}", failed.why);
        assert!(!failed.why.contains("did no work"), "{}", failed.why);
    }

    #[test]
    fn a_build_that_keeps_talking_runs_to_its_end() {
        let mut sh = Command::new(std::env::var_os("BASH").unwrap_or_else(|| "sh".into()));
        sh.args([
            "-c",
            "for i in 1 2 3 4; do echo \"x> $i\" >&2; sleep 0.4; done; echo done",
        ]);
        let said = built(
            sh,
            "building",
            std::iter::empty::<&str>(),
            Duration::from_secs(1),
        );
        assert_eq!(said.unwrap(), "done");
    }

    #[test]
    fn a_copy_back_may_stay_quiet_longer_but_not_for_ever() {
        let copying = |then: &str| {
            let mut sh = Command::new(std::env::var_os("BASH").unwrap_or_else(|| "sh".into()));
            sh.args([
                "-c",
                &format!(
                    "echo \"copying path '/nix/store/a-x' from 'ssh-ng://builder'...\"; {then}"
                ),
            ]);
            built(
                sh,
                "building",
                std::iter::empty::<&str>(),
                Duration::from_secs(1),
            )
        };
        assert_eq!(
            copying("sleep 1.6; echo done").unwrap(),
            "copying path '/nix/store/a-x' from 'ssh-ng://builder'...\ndone"
        );
        let started = Instant::now();
        let failed = copying("sleep 30").unwrap_err();
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "it waited for the end"
        );
        assert!(failed.why.contains("said nothing"), "{}", failed.why);
        assert!(!is_copy_back(
            "copying path '/nix/store/a-x' to 'ssh-ng://builder'..."
        ));
    }

    #[test]
    fn a_piece_building_may_stay_quiet_longer_but_waiting_for_one_may_not() {
        let after = |first: &str| {
            let mut sh = Command::new(std::env::var_os("BASH").unwrap_or_else(|| "sh".into()));
            sh.args(["-c", &format!("echo \"{first}\"; sleep 1.6; echo done")]);
            built(
                sh,
                "building",
                std::iter::empty::<&str>(),
                Duration::from_secs(1),
            )
        };
        // A compile that says nothing for a while still works.
        assert!(after("building '/nix/store/a-x.drv' on 'ssh-ng://builder'...").is_ok());
        assert!(after("building '/nix/store/a-x.drv'...").is_ok());
        // Before any piece builds, a quiet build is one waiting on a
        // builder that doesn't answer.
        let failed = after("these 2 derivations will be built:").unwrap_err();
        assert!(failed.why.contains("said nothing"), "{}", failed.why);
    }

    #[test]
    fn a_build_log_line_is_told_from_nix_s_own() {
        assert!(is_log("linux-7.3.1> CC kernel/fork.o"));
        assert!(!is_log(
            "building '/nix/store/a-b.drv' on 'ssh-ng://builder'..."
        ));
        assert!(!is_log("error: builder failed > 1"));
    }

    #[test]
    fn said_gives_both_streams_and_output_only_what_was_printed() {
        let both = || {
            // The shell the integration tests' stand-ins run on, which a
            // build's sandbox has where it has no /bin/sh.
            let shell = std::env::var("STAND_IN_SHELL").unwrap_or_else(|_| "/bin/sh".to_owned());
            let mut sh = Command::new(shell);
            sh.arg("-c").arg("echo printed; echo said >&2");
            sh
        };
        assert_eq!(output(both(), "saying", [""; 0]).unwrap(), "printed");
        assert_eq!(said(both(), "saying", [""; 0]).unwrap(), "printed\nsaid");
    }

    #[test]
    fn a_tail_keeps_the_last_lines_that_say_something() {
        let said = (1..=30).fold(String::new(), |mut said, n| {
            said.push_str(&n.to_string());
            said.push_str("\n\n");
            said
        });
        let kept = tail(&said);
        assert_eq!(kept.len(), TAIL);
        assert_eq!(kept.first().map(String::as_str), Some("11"));
        assert_eq!(kept.last().map(String::as_str), Some("30"));
    }

    /// One of Nix's errors for a piece, its reason and its output.
    fn cannot_build(piece: &str, reason: &str) -> String {
        format!(
            "error: Cannot build '/nix/store/{piece}.drv'.\n       Reason: {reason}.\n       \
             Output paths:\n         /nix/store/{piece}\n"
        )
    }

    #[test]
    fn a_tail_starts_at_the_piece_that_failed_not_those_waiting_on_it() {
        let mut said = "building the system\n".to_owned();
        said.push_str(&cannot_build("guard", "builder failed with exit code 101"));
        said.push_str("       Last 25 log lines:\n       > test a_guard_holds ... FAILED\n");
        for piece in ["guard-user", "home-path", "system-path", "unit", "system"] {
            said.push_str(&cannot_build(piece, "1 dependency failed"));
        }
        let kept = tail(&said);
        assert_eq!(
            kept.first().map(String::as_str),
            Some("error: Cannot build '/nix/store/guard.drv'.")
        );
        assert!(kept.iter().any(|l| l.contains("a_guard_holds ... FAILED")));
        assert_eq!(kept.len(), TAIL);
    }

    #[test]
    fn a_tail_starts_at_an_older_nix_builder_failing() {
        let said = "error: builder for '/nix/store/guard.drv' failed with exit code 1;\n       \
                    last 25 log lines:\n       > it broke\n\
                    error: 1 dependencies of derivation '/nix/store/system.drv' failed to build\n";
        let kept = tail(said);
        assert_eq!(kept.len(), 4);
        assert!(kept[0].starts_with("error: builder for '/nix/store/guard.drv'"));
    }

    #[test]
    fn a_tail_of_only_waiting_pieces_keeps_the_last_lines() {
        let said = (0..8).fold(String::new(), |said, n| {
            said + &cannot_build(&format!("piece{n}"), "1 dependency failed")
        });
        let kept = tail(&said);
        assert_eq!(kept.len(), TAIL);
        assert_eq!(
            kept.last().map(String::as_str).map(str::trim),
            Some("/nix/store/piece7")
        );
    }
}
