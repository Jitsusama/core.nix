//! Running the programs an update needs, with nothing of the environment
//! root was started in: no git configuration but what's given here, no
//! prompts, and a fixed path, so what runs depends only on the config.

use std::ffi::{CStr, CString, OsStr};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

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
pub fn output<I, S>(mut command: Command, what: &str, args: I) -> Result<String, Failure>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let out = command
        .args(args)
        .output()
        .map_err(|e| Failure::new(format!("{what}: {e}")))?;
    if out.status.success() {
        return Ok(String::from_utf8_lossy(&out.stdout).trim().to_owned());
    }
    let mut said = String::from_utf8_lossy(&out.stdout).into_owned();
    said.push_str(&String::from_utf8_lossy(&out.stderr));
    Err(Failure {
        why: format!("{what} failed ({})", out.status),
        lines: tail(&said),
    })
}

/// The last lines of what a command said, blank ones left out.
pub fn tail(said: &str) -> Vec<String> {
    let lines: Vec<&str> = said.lines().filter(|l| !l.trim().is_empty()).collect();
    let from = lines.len().saturating_sub(TAIL);
    lines[from..].iter().map(|l| (*l).to_owned()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
