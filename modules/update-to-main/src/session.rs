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
    let ended =
        start(launchctl, account, command, &folder, &label).and_then(|()| wait(&folder, what));
    let _ = launchctl_as(launchctl, account)
        .args(["bootout", &format!("{}/{label}", domain(account))])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    let said = |name| fs::read_to_string(folder.join(name)).unwrap_or_default();
    let (out, err) = (said(OUT), said(ERR));
    let _ = fs::remove_dir_all(&folder);
    let status = ended?;
    if status == 0 {
        return Ok(out.trim().to_owned());
    }
    Err(Failure {
        why: format!("{what} failed (exit status: {status})"),
        lines: run::tail(&format!("{out}{err}")),
    })
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

/// Waits for the job to write down how it ended, giving its status.
fn wait(folder: &Path, what: &str) -> Result<i32, Failure> {
    let since = Instant::now();
    loop {
        if let Ok(status) = fs::read_to_string(folder.join(STATUS)) {
            return status.trim().parse().map_err(|_| {
                Failure::new(format!(
                    "{what} in the login session ended saying {status:?}"
                ))
            });
        }
        if since.elapsed() > WITHIN {
            return Err(Failure::new(format!(
                "{what} in the login session didn't end within {} minutes",
                WITHIN.as_secs() / 60
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
/// its status last, as the sign it has ended. The status is also what it
/// exits with.
pub fn job(folder: &Path) -> i32 {
    let status = ran(folder).unwrap_or_else(|e| {
        let _ = fs::write(folder.join(ERR), format!("update-to-main job: {e}\n"));
        127
    });
    let _ = fs::write(folder.join(STATUS), format!("{status}\n"));
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
        let made = folder(&jobs).unwrap();
        assert_eq!(made.parent(), Some(jobs.as_path()));
        let mode = |p: &Path| fs::metadata(p).unwrap().permissions().mode() & 0o777;
        assert_eq!((mode(&jobs), mode(&made)), (0o711, 0o700));
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
        assert_eq!(job(folder.path()), 127);
        assert_eq!(
            fs::read_to_string(folder.path().join(STATUS)).unwrap(),
            "127\n"
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
