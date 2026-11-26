//! An account's command run in its login session, on a Mac. macOS opens an
//! account's keychain only to the processes of its login session, so a
//! command root runs as the account, its own home and all, finds the
//! logins kept there locked: dev's git credential helper and gh both keep
//! GitHub's there. A launchd job in the session's domain, `gui/<uid>`,
//! runs where they answer, and launchd lets the account start one there
//! itself.
//!
//! Root writes the command down in a folder of its own, made in the jobs
//! folder the config names, and gives the folder to the account, which starts a job running `update-to-main job` on it;
//! `job` runs the command and writes down what it said and how it ended,
//! and root waits for that. With no launchctl, or nobody logged in as the
//! account, the command runs as it would anywhere else.
//!
//! Long because root's half of the hand-off and the job's half share its
//! files and their limits, and are tested together: most of it is tests.

use crate::config::Session;
use crate::run::{self, Account, Failure};
use serde_json::{Value, json};
use std::ffi::{CString, OsStr};
use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// How long a job may take before it's stopped. It fetches an input, which
/// may be a large repository.
const WITHIN: Duration = Duration::from_hours(1);

/// How often root looks whether the job has ended.
const EVERY: Duration = Duration::from_millis(250);

/// How long an empty status may stay empty before it's told as the job's
/// end with its number lost. A status written in place lands within a
/// write; one still empty after this was left so by a write that failed,
/// as on a full disk, so root says so rather than waiting out its hour.
const LANDS: Duration = Duration::from_secs(10);

/// How much of the end of a job's file root reads: to tell why it failed,
/// and of what it printed when it ended well, which neither caller reads.
const KEPT: u64 = 64 * 1024;

/// How much of a job's status root reads: a number and its line's end.
const STATUS_KEPT: u64 = 64;

/// What a job's folder holds: the command, the job launchd is given, and
/// what the command printed, said and ended with.
const COMMAND: &str = "command.json";
const PLIST: &str = "job.plist";
const OUT: &str = "out";
const ERR: &str = "err";
const STATUS: &str = "status";

/// Runs `command`, made by `run::command_as` for `account`, as
/// `run::output` would: in the account's login session when `session` is
/// given and the login session is open, and otherwise as it is.
pub fn output<I, S>(
    session: Option<&Session>,
    account: &Account,
    mut command: Command,
    what: &str,
    args: I,
) -> Result<String, Failure>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    command.args(args);
    match session {
        Some(session) if open(&session.launchctl, account) => {
            in_session(session, account, &command, what)
        }
        _ => run::output(command, what, std::iter::empty::<&str>()),
    }
}

/// The launchd domain of the account's login session.
fn domain(account: &Account) -> String {
    format!("gui/{}", account.uid())
}

/// Whether the account has a login session open, which launchd says by
/// knowing its domain.
fn open(launchctl: &Path, account: &Account) -> bool {
    launchctl_as(launchctl, account)
        .args(["print", &domain(account)])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

/// launchctl run as the account, which may start and stop jobs in its own
/// session's domain.
fn launchctl_as(launchctl: &Path, account: &Account) -> Command {
    run::command_as(launchctl.as_os_str(), account, &[], Path::new("/"))
}

fn in_session(
    session: &Session,
    account: &Account,
    command: &Command,
    what: &str,
) -> Result<String, Failure> {
    let launchctl = session.launchctl.as_path();
    let failed = |e: std::io::Error| Failure::new(format!("{what} in the login session: {e}"));
    let folder = folder(&session.jobs).map_err(failed)?;
    let label = format!(
        "org.jitsusama.update-to-main.{}",
        folder.file_name().unwrap_or_default().to_string_lossy()
    );
    let ended = start(launchctl, account, command, &folder, &label)
        .and_then(|()| wait(&folder, what, WITHIN, LANDS));
    // Read before the job is stopped, so a job that ran out its hour is told
    // by what it was doing, not by Nix's own word on being stopped.
    let (out, err) = (kept(&folder.join(OUT), KEPT), kept(&folder.join(ERR), KEPT));
    let _ = launchctl_as(launchctl, account)
        .args(["bootout", &format!("{}/{label}", domain(account))])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    let _ = fs::remove_dir_all(&folder);
    said_by(ended, what, out.as_deref(), err.as_deref())
}

/// The last `most` bytes of one of a job's files. The account owns the
/// job's folder and may have put anything there, so root opens the file
/// without following a link or waiting on a pipe, and reads only a plain
/// file the folder's owner owns, with no other name, since a hard link
/// would let the account have root read a file it can't. A hard link to
/// a file whose other name is gone has no other name, but keeps its own
/// owner, so the owner is what refuses it.
fn read_end(path: &Path, most: u64) -> std::io::Result<Vec<u8>> {
    let folder = path.parent().unwrap_or(Path::new("/"));
    read_owned(path, most, fs::symlink_metadata(folder)?.uid())
}

/// `read_end` of a file that must be `owner`'s.
fn read_owned(path: &Path, most: u64, owner: u32) -> std::io::Result<Vec<u8>> {
    use std::io::{Read, Seek, SeekFrom};
    use std::os::unix::fs::OpenOptionsExt;
    let mut file = fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)?;
    let found = file.metadata()?;
    if !found.is_file() {
        return Err(std::io::Error::other("it isn't a plain file"));
    }
    if found.nlink() > 1 {
        return Err(std::io::Error::other("it has another name"));
    }
    if found.uid() != owner {
        return Err(std::io::Error::other("it isn't its folder's owner's"));
    }
    file.seek(SeekFrom::Start(found.len().saturating_sub(most)))?;
    let mut bytes = Vec::new();
    file.take(most).read_to_end(&mut bytes)?;
    Ok(bytes)
}

/// The last `most` bytes a job wrote to one of its files, as text, read as
/// `read_end` reads. Bytes that aren't UTF-8 are read as replacements
/// rather than losing the file, since a job stopped after its hour may
/// have stopped in the middle of a character. A file that isn't there is
/// none, since the job may never have started; one that can't be read
/// says so in its place.
fn kept(path: &Path, most: u64) -> Option<String> {
    match read_end(path, most) {
        Ok(bytes) => Some(String::from_utf8_lossy(&bytes).into_owned()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => Some(format!(
            "the job's {} was left unread: {e}\n",
            path.file_name().unwrap_or_default().to_string_lossy()
        )),
    }
}

/// What a job's end gives back: its output alone when it ended well, and
/// otherwise the failure with what the job said, each file read only to
/// its last `KEPT` bytes. A job that failed is told from Nix's first error
/// in those bytes, as any command is. Any other failure that brought no
/// lines of its own, a job that didn't end above all, is told with the
/// last lines the job had said by then, the newest nearest what it waited
/// on; or that it had written nothing, which fits a prompt or a fetch that
/// prints nothing without a terminal, rather than Nix failing; or, when
/// neither of its files is there, that it left none, which a job that
/// never started leaves too. On 24 November three that waited an hour each
/// left only "didn't end", and whether they waited on a prompt or on Nix
/// couldn't be read afterwards. A failure with lines of its own, as
/// launchctl's refusal to start the job, keeps them.
fn said_by(
    ended: Result<i32, Failure>,
    what: &str,
    out: Option<&str>,
    err: Option<&str>,
) -> Result<String, Failure> {
    let said = format!("{}{}", out.unwrap_or_default(), err.unwrap_or_default());
    match ended {
        Ok(0) => Ok(out.unwrap_or_default().trim().to_owned()),
        Ok(status) => Err(Failure {
            why: format!("{what} failed (exit status: {status})"),
            lines: run::tail(&said),
        }),
        Err(failure) if failure.lines.is_empty() => {
            let lines = last(&said);
            let why = if out.is_none() && err.is_none() {
                format!("{}, the job having left no output or errors", failure.why)
            } else if lines.is_empty() {
                format!("{}, the job having written nothing", failure.why)
            } else {
                failure.why
            };
            Err(Failure { why, lines })
        }
        Err(failure) => Err(failure),
    }
}

/// The last lines of what a job said, blank ones left out.
fn last(said: &str) -> Vec<String> {
    let lines: Vec<&str> = said.lines().filter(|l| !l.trim().is_empty()).collect();
    lines[lines.len().saturating_sub(run::TAIL)..]
        .iter()
        .map(|l| (*l).to_owned())
        .collect()
}

/// A new folder only root can reach, in `jobs`, given to the account once
/// root has written the command in it. Root then only reads what the job
/// leaves. `jobs` is made root's and passable by anyone, so the account can
/// reach the folder; a folder there that isn't root's, or a link, is
/// refused rather than trusted.
fn folder(jobs: &Path) -> std::io::Result<PathBuf> {
    fs::create_dir_all(jobs)?;
    let found = fs::symlink_metadata(jobs)?;
    // SAFETY: geteuid only reads.
    if !found.is_dir() || found.uid() != unsafe { libc::geteuid() } {
        return Err(std::io::Error::other(format!(
            "{} isn't a folder of this account's own",
            jobs.display()
        )));
    }
    fs::set_permissions(jobs, fs::Permissions::from_mode(0o711))?;
    let template = jobs.join("update-to-main.XXXXXX");
    let template = CString::new(template.as_os_str().as_bytes())?;
    let raw = template.into_raw();
    // SAFETY: `raw` is a C string this frame owns, which mkdtemp rewrites in
    // place, and which is taken back below whatever it returns.
    let made = unsafe { libc::mkdtemp(raw) };
    // SAFETY: `raw` came from CString::into_raw just above.
    let template = unsafe { CString::from_raw(raw) };
    if made.is_null() {
        return Err(std::io::Error::last_os_error());
    }
    Ok(PathBuf::from(OsStr::from_bytes(template.as_bytes())))
}

/// Writes the command and its job down, gives the folder to the account,
/// and has the account start the job in its session.
fn start(
    launchctl: &Path,
    account: &Account,
    command: &Command,
    folder: &Path,
    label: &str,
) -> Result<(), Failure> {
    let io = |e: std::io::Error| Failure::new(format!("{}: {e}", folder.display()));
    let this = std::env::current_exe().map_err(io)?;
    fs::write(folder.join(COMMAND), described(command).to_string()).map_err(io)?;
    fs::write(folder.join(PLIST), plist(label, &this, folder)).map_err(io)?;
    for path in [folder.to_owned(), folder.join(COMMAND), folder.join(PLIST)] {
        give(&path, account).map_err(io)?;
    }
    run::output(
        launchctl_as(launchctl, account),
        "starting a job in the login session",
        [
            OsStr::new("bootstrap"),
            OsStr::new(&domain(account)),
            folder.join(PLIST).as_os_str(),
        ],
    )?;
    Ok(())
}

fn give(path: &Path, account: &Account) -> std::io::Result<()> {
    let path = CString::new(path.as_os_str().as_bytes())?;
    // SAFETY: `path` is a C string this frame owns.
    if unsafe { libc::chown(path.as_ptr(), account.uid(), account.gid()) } != 0 {
        let e = std::io::Error::last_os_error();
        // Anyone but root runs the job as themselves, and owns it already.
        // SAFETY: geteuid only reads.
        if unsafe { libc::geteuid() } == 0 {
            return Err(e);
        }
    }
    Ok(())
}

/// Waits for the job to write down how it ended, giving its status, for
/// at most `within`. The status is read as the job's other files are, and
/// only to its last few bytes, so one the account made a link, a pipe or a
/// large file is refused rather than followed, waited on or read whole.
/// An empty one is a status written in place but not yet, so it's waited
/// on as one not there, for `lands` at most: one empty longer was left so
/// by a failed write, and the job has ended.
fn wait(folder: &Path, what: &str, within: Duration, lands: Duration) -> Result<i32, Failure> {
    let since = Instant::now();
    let mut empty: Option<Instant> = None;
    loop {
        match read_end(&folder.join(STATUS), STATUS_KEPT) {
            Ok(bytes) if bytes.is_empty() => {
                if empty.get_or_insert_with(Instant::now).elapsed() >= lands {
                    return Err(Failure::new(format!(
                        "{what} in the login session ended, but its status stayed empty, \
                         as a write on a full disk leaves it"
                    )));
                }
            }
            Ok(bytes) => {
                let status = String::from_utf8_lossy(&bytes);
                return status.trim().parse().map_err(|_| {
                    Failure::new(format!(
                        "{what} in the login session ended saying {status:?}"
                    ))
                });
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => {
                return Err(Failure::new(format!(
                    "{what} in the login session left its status unread: {e}"
                )));
            }
        }
        if since.elapsed() > within {
            return Err(Failure::new(format!(
                "{what} in the login session didn't end within {} minutes",
                within.as_secs() / 60
            )));
        }
        std::thread::sleep(EVERY);
    }
}

/// The command as the job runs it: its program, arguments, environment and
/// folder, which `run::command_as` set and nothing else did.
fn described(command: &Command) -> Value {
    let text = |s: &OsStr| s.to_string_lossy().into_owned();
    let env: serde_json::Map<String, Value> = command
        .get_envs()
        .filter_map(|(k, v)| Some((text(k), Value::String(text(v?)))))
        .collect();
    json!({
        "program": text(command.get_program()),
        "args": command.get_args().map(text).collect::<Vec<_>>(),
        "env": env,
        "dir": command.get_current_dir().map_or_else(|| "/".to_owned(), |d| text(d.as_os_str())),
    })
}

/// The job launchd runs once, at once: `update-to-main job FOLDER`.
fn plist(label: &str, program: &Path, folder: &Path) -> String {
    let string = |s: &str| format!("\t\t<string>{}</string>\n", escaped(s));
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \
         \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
         <plist version=\"1.0\">\n<dict>\n\
         \t<key>Label</key>\n\t<string>{}</string>\n\
         \t<key>ProgramArguments</key>\n\t<array>\n{}{}{}\t</array>\n\
         \t<key>RunAtLoad</key>\n\t<true/>\n\
         </dict>\n</plist>\n",
        escaped(label),
        string(&program.to_string_lossy()),
        string("job"),
        string(&folder.to_string_lossy()),
    )
}

fn escaped(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// `update-to-main job FOLDER`, the job's own end: runs the command written
/// in the folder, its output and errors to files there, and writes down
/// its status last, as the sign it has ended, which root reads. main.rs
/// ends 0 for a status of 0 and 1 for any other.
pub fn job(folder: &Path) -> i32 {
    let status = ran(folder).unwrap_or_else(|e| {
        let _ = fs::write(folder.join(ERR), format!("update-to-main job: {e}\n"));
        127
    });
    // Written aside and moved into place, so root reads it whole, but for
    // the fallback below.
    let aside = folder.join(format!("{STATUS}.new"));
    let said = format!("{status}\n");
    if fs::write(&aside, &said)
        .and_then(|()| fs::rename(&aside, folder.join(STATUS)))
        .is_err()
    {
        // One that can't be moved into place, as on a full disk, is written
        // where it goes, rather than root waiting out its hour on a job that
        // ended: root waits on an empty one, as when it's caught before its
        // bytes land, and reads whatever lands as the job's end, or tells
        // one still empty after `LANDS` as an end whose number was lost.
        let _ = fs::write(folder.join(STATUS), said);
    }
    status
}

fn ran(folder: &Path) -> Result<i32, String> {
    let text = fs::read_to_string(folder.join(COMMAND)).map_err(|e| e.to_string())?;
    let described: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    let field = |name: &str| described[name].as_str().map(str::to_owned);
    let program = field("program").ok_or("no program")?;
    let mut command = Command::new(program);
    command
        .env_clear()
        .current_dir(field("dir").ok_or("no folder")?)
        .stdin(Stdio::null());
    for arg in described["args"].as_array().into_iter().flatten() {
        command.arg(arg.as_str().ok_or("an argument isn't text")?);
    }
    for (key, value) in described["env"].as_object().into_iter().flatten() {
        command.env(key, value.as_str().ok_or("a variable isn't text")?);
    }
    let file = |name| fs::File::create(folder.join(name)).map_err(|e| e.to_string());
    let status = command
        .stdout(file(OUT)?)
        .stderr(file(ERR)?)
        .status()
        .map_err(|e| e.to_string())?;
    Ok(status.code().unwrap_or(128))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shell() -> String {
        std::env::var("STAND_IN_SHELL").unwrap_or_else(|_| "/bin/sh".to_owned())
    }

    #[test]
    fn a_job_runs_its_command_with_only_its_environment_and_writes_how_it_ended() {
        let folder = tempfile::tempdir().unwrap();
        let mut command = Command::new(shell());
        command
            .env_clear()
            .env("SAYING", "a & b")
            .current_dir(folder.path())
            .args([
                "-c",
                "echo \"$SAYING in $PWD, HOME=${HOME-unset}\"; echo said >&2; exit 3",
            ]);
        fs::write(folder.path().join(COMMAND), described(&command).to_string()).unwrap();
        assert_eq!(job(folder.path()), 3);
        let read = |name| fs::read_to_string(folder.path().join(name)).unwrap();
        assert_eq!(read(STATUS), "3\n");
        let here = folder.path().canonicalize().unwrap();
        assert_eq!(
            read(OUT),
            format!("a & b in {}, HOME=unset\n", here.display())
        );
        assert_eq!(read(ERR), "said\n");
    }

    #[test]
    fn a_jobs_folder_is_made_passable_and_a_jobs_own_folder_in_it_private() {
        let dir = tempfile::tempdir().unwrap();
        let jobs = dir.path().join("jobs");
        let own = folder(&jobs).unwrap();
        assert_eq!(own.parent(), Some(jobs.as_path()));
        let mode = |p: &Path| fs::metadata(p).unwrap().permissions().mode() & 0o777;
        assert_eq!((mode(&jobs), mode(&own)), (0o711, 0o700));
    }

    fn lines(said: &[&str]) -> Vec<String> {
        said.iter().map(|l| (*l).to_owned()).collect()
    }

    #[test]
    fn a_job_that_didnt_end_is_told_with_the_last_it_had_said() {
        let folder = tempfile::tempdir().unwrap();
        let waited = wait(
            folder.path(),
            "fetching the input secrets",
            Duration::ZERO,
            LANDS,
        );
        let out = "waiting for lock on /nix/store/x.lock\n";
        let err = "error: interrupted by the user\n";
        let told = said_by(waited, "fetching", Some(out), Some(err));
        let told = told.unwrap_err();
        assert_eq!(
            (told.why.as_str(), told.lines),
            (
                "fetching the input secrets in the login session didn't end within 0 minutes",
                lines(&[
                    "waiting for lock on /nix/store/x.lock",
                    "error: interrupted by the user"
                ])
            )
        );
    }

    #[test]
    fn a_job_that_didnt_end_keeps_only_its_newest_lines() {
        let said = (1..=30)
            .map(|n| format!("line {n}"))
            .collect::<Vec<_>>()
            .join("\n");
        let told = said_by(
            Err(Failure::new("didn't end")),
            "fetching",
            None,
            Some(&said),
        );
        let told = told.unwrap_err().lines;
        assert_eq!(
            (told.len(), told.first(), told.last()),
            (
                run::TAIL,
                Some(&"line 11".to_owned()),
                Some(&"line 30".to_owned())
            )
        );
    }

    #[test]
    fn a_job_that_didnt_end_having_written_nothing_says_so() {
        let told = said_by(
            Err(Failure::new("didn't end")),
            "fetching",
            Some(""),
            Some("\n"),
        );
        let told = told.unwrap_err();
        assert_eq!(
            (told.why.as_str(), told.lines),
            ("didn't end, the job having written nothing", Vec::new())
        );
    }

    #[test]
    fn a_job_that_didnt_end_having_left_one_file_empty_says_it_wrote_nothing() {
        // A job stopped between making its output and its errors leaves
        // only the first, which is still a job that started.
        let told = said_by(Err(Failure::new("didn't end")), "fetching", Some(""), None);
        assert_eq!(
            told.unwrap_err().why,
            "didn't end, the job having written nothing",
            "a job that left one of its files was told as having left none"
        );
    }

    #[test]
    fn a_job_that_didnt_end_having_left_no_files_says_so() {
        let folder = tempfile::tempdir().unwrap();
        let (out, err) = (
            kept(&folder.path().join(OUT), KEPT),
            kept(&folder.path().join(ERR), KEPT),
        );
        let told = said_by(
            Err(Failure::new("didn't end")),
            "fetching",
            out.as_deref(),
            err.as_deref(),
        );
        assert_eq!(
            told.unwrap_err().why,
            "didn't end, the job having left no output or errors"
        );
    }

    #[test]
    fn a_job_whose_status_isnt_a_number_is_told_with_what_it_said() {
        let folder = tempfile::tempdir().unwrap();
        fs::write(folder.path().join(STATUS), "x\n").unwrap();
        let ended = wait(folder.path(), "fetching", WITHIN, LANDS);
        let told = said_by(ended, "fetching", Some(""), Some("said\n")).unwrap_err();
        assert_eq!(
            (told.why.as_str(), told.lines),
            (
                "fetching in the login session ended saying \"x\\n\"",
                lines(&["said"])
            )
        );
    }

    #[test]
    fn an_empty_status_is_waited_on_as_one_not_written() {
        let folder = tempfile::tempdir().unwrap();
        fs::write(folder.path().join(STATUS), "").unwrap();
        let told = wait(folder.path(), "fetching", Duration::ZERO, LANDS).unwrap_err();
        assert_eq!(
            told.why, "fetching in the login session didn't end within 0 minutes",
            "an empty status, as one written in place but not yet, was read as the job's end"
        );
    }

    #[test]
    fn a_jobs_status_is_read_only_to_its_last_bytes() {
        let folder = tempfile::tempdir().unwrap();
        let kept = usize::try_from(STATUS_KEPT).unwrap();
        // One x ahead of the last bytes, so a read of the first bytes
        // shows apart from one of the last.
        fs::write(folder.path().join(STATUS), format!("x{}", "y".repeat(kept))).unwrap();
        let told = wait(folder.path(), "fetching", Duration::ZERO, LANDS).unwrap_err();
        assert_eq!(
            told.why,
            format!(
                "fetching in the login session ended saying {:?}",
                "y".repeat(kept)
            ),
            "a status wasn't read to its last {kept} bytes alone"
        );
    }

    #[test]
    fn a_job_that_couldnt_start_keeps_what_launchctl_said() {
        let refused = Failure {
            why: "starting a job in the login session failed".to_owned(),
            lines: lines(&["Bootstrap failed: 5"]),
        };
        let left = Some("left over\n");
        let told = said_by(Err(refused), "fetching", left, left).unwrap_err();
        assert_eq!(
            (told.why.as_str(), told.lines),
            (
                "starting a job in the login session failed",
                lines(&["Bootstrap failed: 5"])
            )
        );
    }

    #[test]
    fn a_job_that_ends_well_gives_its_output_alone() {
        let given = said_by(Ok(0), "fetching", Some("out\n"), Some("warning: noise\n"));
        assert_eq!(given.unwrap(), "out");
    }

    #[test]
    fn a_job_that_ends_well_is_read_only_to_the_end_of_what_it_printed() {
        let folder = tempfile::tempdir().unwrap();
        let file = folder.path().join(OUT);
        let printed = usize::try_from(KEPT).unwrap();
        fs::write(&file, format!("{}{}", "o".repeat(printed), "newest\n")).unwrap();
        let read = kept(&file, KEPT).unwrap();
        assert!(
            read.len() == printed && read.ends_with("newest\n"),
            "a good job's output was read past its last {KEPT} bytes: {} bytes",
            read.len()
        );
    }

    #[test]
    fn a_job_that_fails_gives_its_status_and_what_it_printed_then_said() {
        let failed = said_by(Ok(3), "fetching", Some("printed\n"), Some("said\n")).unwrap_err();
        assert_eq!(
            (failed.why.as_str(), failed.lines),
            (
                "fetching failed (exit status: 3)",
                lines(&["printed", "said"])
            )
        );
    }

    #[test]
    fn a_jobs_file_is_read_past_bytes_that_arent_text() {
        let folder = tempfile::tempdir().unwrap();
        let file = folder.path().join(ERR);
        fs::write(&file, b"cut \xe2\x80\nwaiting for lock\n").unwrap();
        assert_eq!(
            kept(&file, KEPT).as_deref(),
            Some("cut \u{fffd}\nwaiting for lock\n")
        );
    }

    #[test]
    fn a_jobs_file_is_read_only_to_its_end() {
        let folder = tempfile::tempdir().unwrap();
        let file = folder.path().join(ERR);
        fs::write(&file, "older\nnewest\n").unwrap();
        assert_eq!(kept(&file, 7).as_deref(), Some("newest\n"));
    }

    #[test]
    fn a_jobs_file_that_is_a_link_is_left_unread() {
        let folder = tempfile::tempdir().unwrap();
        let secret = folder.path().join("secret");
        fs::write(&secret, "root's own\n").unwrap();
        let file = folder.path().join(ERR);
        std::os::unix::fs::symlink(&secret, &file).unwrap();
        let read = kept(&file, KEPT).unwrap_or_default();
        assert!(
            read.starts_with("the job's err was left unread:") && !read.contains("root's own"),
            "a link in the job's folder was followed: {read}"
        );
    }

    #[test]
    fn a_jobs_file_that_is_another_files_hard_link_is_left_unread() {
        let folder = tempfile::tempdir().unwrap();
        let secret = folder.path().join("secret");
        fs::write(&secret, "root's own\n").unwrap();
        let file = folder.path().join(ERR);
        fs::hard_link(&secret, &file).unwrap();
        assert_eq!(
            kept(&file, KEPT).as_deref(),
            Some("the job's err was left unread: it has another name\n")
        );
    }

    #[test]
    fn a_jobs_file_not_its_folders_owners_is_left_unread() {
        // A hard link whose other name is gone has one name, so only its
        // owner, root's and not the account's, tells it apart. A test
        // can't make a file another account owns, so it names another
        // owner instead. Which owner `read_end` takes, its folder's, is
        // held by every read of a job's file here, run by an account that
        // isn't root.
        let folder = tempfile::tempdir().unwrap();
        let file = folder.path().join(ERR);
        fs::write(&file, "the account's own\n").unwrap();
        let owner = fs::metadata(folder.path()).unwrap().uid();
        assert_eq!(
            read_owned(&file, KEPT, owner).unwrap(),
            b"the account's own\n"
        );
        let refused = read_owned(&file, KEPT, owner.wrapping_add(1)).unwrap_err();
        assert_eq!(refused.to_string(), "it isn't its folder's owner's");
    }

    #[test]
    fn a_status_left_empty_is_told_as_an_end_whose_number_was_lost() {
        let folder = tempfile::tempdir().unwrap();
        fs::write(folder.path().join(STATUS), "").unwrap();
        // A bound of four looks, so one whose clock starts again at each
        // look never ends the wait, within a wait short enough that one
        // which forgets the bound fails as a job that didn't end, and one
        // held to that wait instead ends too late.
        let (within, lands, late) = (Duration::from_secs(5), 4 * EVERY, Duration::from_secs(4));
        let since = Instant::now();
        let told = wait(folder.path(), "fetching", within, lands).unwrap_err();
        assert_eq!(
            told.why,
            "fetching in the login session ended, but its status stayed empty, as a write \
             on a full disk leaves it",
            "an empty status that stayed empty was waited on for the hour"
        );
        let waited = since.elapsed();
        assert!(
            waited >= lands && waited < late,
            "an empty status was told lost after {waited:?}, not once it had stayed empty for \
             its bound"
        );
    }

    #[test]
    fn an_empty_status_that_lands_within_its_bound_is_read_as_its_number() {
        // The status of a job caught between its write's opening and its
        // bytes, seen empty for a few looks and then landing.
        let folder = tempfile::tempdir().unwrap();
        let status = folder.path().join(STATUS);
        fs::write(&status, "").unwrap();
        let landing = std::thread::spawn(move || {
            std::thread::sleep(2 * EVERY);
            fs::write(status, "0\n").unwrap();
        });
        let ended = wait(folder.path(), "fetching", Duration::from_secs(5), 8 * EVERY);
        landing.join().unwrap();
        assert_eq!(
            ended.map_err(|told| told.why),
            Ok(0),
            "a status that landed within its bound wasn't read as the job's end"
        );
    }

    #[test]
    fn a_jobs_status_that_is_a_link_is_refused_rather_than_followed() {
        let folder = tempfile::tempdir().unwrap();
        let elsewhere = folder.path().join("elsewhere");
        fs::write(&elsewhere, "0\n").unwrap();
        std::os::unix::fs::symlink(&elsewhere, folder.path().join(STATUS)).unwrap();
        // No wait, so a refusal taken as not yet there fails at once.
        let told = wait(folder.path(), "fetching", Duration::ZERO, LANDS).unwrap_err();
        assert!(
            told.why
                .starts_with("fetching in the login session left its status unread:"),
            "a status that is a link was read as the job's end: {}",
            told.why
        );
    }

    #[test]
    fn a_jobs_status_that_is_a_pipe_is_refused_at_once() {
        let folder = tempfile::tempdir().unwrap();
        let path = CString::new(folder.path().join(STATUS).as_os_str().as_bytes()).unwrap();
        // SAFETY: `path` is a C string this frame owns.
        let made = unsafe { libc::mkfifo(path.as_ptr(), 0o600) };
        assert_eq!(made, 0, "no pipe made");
        let told = wait(folder.path(), "fetching", Duration::ZERO, LANDS).unwrap_err();
        assert_eq!(
            told.why,
            "fetching in the login session left its status unread: it isn't a plain file"
        );
    }

    #[test]
    fn a_jobs_file_that_is_a_pipe_is_left_unread_at_once() {
        let folder = tempfile::tempdir().unwrap();
        let file = folder.path().join(OUT);
        let path = CString::new(file.as_os_str().as_bytes()).unwrap();
        // SAFETY: `path` is a C string this frame owns.
        let made = unsafe { libc::mkfifo(path.as_ptr(), 0o600) };
        assert_eq!(made, 0, "no pipe made");
        assert_eq!(
            kept(&file, KEPT).as_deref(),
            Some("the job's out was left unread: it isn't a plain file\n")
        );
    }

    #[test]
    fn a_jobs_file_that_isnt_there_is_none() {
        let folder = tempfile::tempdir().unwrap();
        assert_eq!(kept(&folder.path().join(OUT), KEPT), None);
    }

    #[test]
    fn a_jobs_folder_that_is_a_link_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let elsewhere = dir.path().join("elsewhere");
        fs::create_dir(&elsewhere).unwrap();
        let jobs = dir.path().join("jobs");
        std::os::unix::fs::symlink(&elsewhere, &jobs).unwrap();
        assert!(folder(&jobs).is_err());
        assert_eq!(fs::read_dir(&elsewhere).unwrap().count(), 0);
    }

    #[test]
    fn a_job_that_cant_run_still_says_it_ended() {
        let folder = tempfile::tempdir().unwrap();
        // One left from before is there, so a status written in place
        // rather than moved from aside leaves it and fails.
        fs::write(folder.path().join("status.new"), "0\n").unwrap();
        assert_eq!(job(folder.path()), 127);
        assert_eq!(
            fs::read_to_string(folder.path().join(STATUS)).unwrap(),
            "127\n"
        );
        assert!(
            !folder.path().join("status.new").exists(),
            "the status written aside was left beside it"
        );
    }

    #[test]
    fn a_status_that_cant_be_written_aside_is_written_in_place() {
        let folder = tempfile::tempdir().unwrap();
        // A folder where the status goes aside, so writing it there fails.
        fs::create_dir(folder.path().join("status.new")).unwrap();
        assert_eq!(job(folder.path()), 127);
        assert_eq!(
            fs::read_to_string(folder.path().join(STATUS))
                .ok()
                .as_deref(),
            Some("127\n"),
            "a status that couldn't be written aside wasn't written in place"
        );
    }

    #[test]
    fn a_plist_escapes_what_xml_would_read_as_markup() {
        let plist = plist(
            "a<b",
            Path::new("/x&y/update-to-main"),
            Path::new("/tmp/f>"),
        );
        assert!(plist.contains("<string>a&lt;b</string>"), "{plist}");
        assert!(
            plist.contains("<string>/x&amp;y/update-to-main</string>"),
            "{plist}"
        );
        assert!(plist.contains("<string>/tmp/f&gt;</string>"), "{plist}");
    }
}
