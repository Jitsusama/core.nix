//! Whether a client runs in pi's fence, and for which session, from what
//! the kernel says rather than what the client claims.
//!
//! Every fenced command runs as the person, so its own environment is
//! only a claim: one command can name another session, or say it is
//! fenced when it isn't. What it can't change is where it sits. pi runs
//! each command under bubblewrap, which makes it a pid namespace of its
//! own; the bubblewrap that made it stays outside, in this door's
//! namespace, holding the environment pi gave it. Nothing in a fence can
//! make a process outside it, nor reach that bubblewrap's memory. So a
//! client is answered when the kernel puts it in a pid namespace other
//! than the door's, its nearest ancestor in the door's own is
//! bubblewrap, and both run as the account the door does; its session is
//! the `PI_SESSION_ID` that bubblewrap was started with.
//!
//! A command outside the fence is refused even though it could reach
//! the clipboard itself: the door is the fence's, and answering only
//! fenced commands keeps its journal a record of theirs alone.

use std::fs;
use std::os::fd::AsRawFd;
use std::os::unix::net::UnixStream;
use std::path::PathBuf;

/// What a line says when the bubblewrap's session can't be read.
pub const UNKNOWN: &str = "unknown";

/// The longest session id said, longer than any pi mints.
const LONGEST: usize = 64;

/// How many ancestors are walked before a client is taken as outside,
/// far deeper than a fence nests.
const DEEPEST: usize = 64;

/// The program pi fences commands with, as `/proc/PID/exe` names it.
const BUBBLEWRAP: &str = "bwrap";

/// A client in a fence of this account, and the session it runs for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fenced {
    pub session: String,
}

/// Why a client was taken as outside the fence, as the one word its
/// journal line carries.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outside {
    /// The kernel couldn't say who the client is, or it has gone.
    Gone,
    /// The client, or the bubblewrap above it, runs as another account.
    OtherAccount,
    /// The client shares the door's pid namespace: it isn't fenced.
    Unfenced,
    /// Its nearest ancestor in the door's namespace isn't bubblewrap.
    NoBubblewrap,
}

impl Outside {
    /// The word a journal line carries for this.
    pub fn word(self) -> &'static str {
        match self {
            Outside::Gone => "gone",
            Outside::OtherAccount => "other-account",
            Outside::Unfenced => "unfenced",
            Outside::NoBubblewrap => "no-bubblewrap",
        }
    }
}

/// The process table, as `/proc` holds it, or a stand-in for one.
pub struct Processes {
    root: PathBuf,
}

impl Processes {
    /// The system's own, from the door's pid namespace.
    pub fn system() -> Self {
        Processes::at("/proc")
    }

    /// One laid out as `/proc` is, under `root`.
    pub fn at(root: impl Into<PathBuf>) -> Self {
        Processes { root: root.into() }
    }

    /// Whether the process `pid`, running as `uid`, is in a fence of the
    /// account `own_uid`, and which session it runs for.
    pub fn fenced(&self, pid: u32, uid: u32, own_uid: u32) -> Result<Fenced, Outside> {
        if uid != own_uid {
            return Err(Outside::OtherAccount);
        }
        let own = self.namespace("self").ok_or(Outside::Gone)?;
        let theirs = self.namespace(&pid.to_string()).ok_or(Outside::Gone)?;
        if theirs == own {
            return Err(Outside::Unfenced);
        }
        let mut process = pid;
        for _ in 0..DEEPEST {
            process = self.parent(process).ok_or(Outside::Gone)?;
            if process == 0 {
                return Err(Outside::NoBubblewrap);
            }
            let name = process.to_string();
            if self.namespace(&name).ok_or(Outside::Gone)? != own {
                continue;
            }
            if !self.runs_bubblewrap(&name) {
                return Err(Outside::NoBubblewrap);
            }
            if self.uid(&name) != Some(own_uid) {
                return Err(Outside::OtherAccount);
            }
            let session = self
                .file(&name, "environ")
                .and_then(|environ| session_in(&environ))
                .unwrap_or_else(|| UNKNOWN.to_owned());
            return Ok(Fenced { session });
        }
        Err(Outside::NoBubblewrap)
    }

    fn path(&self, pid: &str, entry: &str) -> PathBuf {
        self.root.join(pid).join(entry)
    }

    fn file(&self, pid: &str, entry: &str) -> Option<Vec<u8>> {
        fs::read(self.path(pid, entry)).ok()
    }

    /// The pid namespace `pid` is in, as the link naming it reads.
    fn namespace(&self, pid: &str) -> Option<PathBuf> {
        fs::read_link(self.path(pid, "ns/pid")).ok()
    }

    /// The parent of `pid`, as `stat` names it in the door's namespace:
    /// its fourth field, after a name that may hold spaces and brackets.
    fn parent(&self, pid: u32) -> Option<u32> {
        let stat = self.file(&pid.to_string(), "stat")?;
        let stat = String::from_utf8_lossy(&stat);
        let (_, after) = stat.rsplit_once(')')?;
        after.split_whitespace().nth(1)?.parse().ok()
    }

    fn runs_bubblewrap(&self, pid: &str) -> bool {
        fs::read_link(self.path(pid, "exe"))
            .is_ok_and(|exe| exe.file_name().and_then(|name| name.to_str()) == Some(BUBBLEWRAP))
    }

    /// The account `pid` runs as, its real uid as `status` says it.
    fn uid(&self, pid: &str) -> Option<u32> {
        let status = self.file(pid, "status")?;
        let status = String::from_utf8_lossy(&status);
        status
            .lines()
            .find_map(|line| line.strip_prefix("Uid:"))?
            .split_whitespace()
            .next()?
            .parse()
            .ok()
    }
}

/// The process on the far end of `client` and the account it runs as,
/// as the kernel says.
pub fn credentials(client: &UnixStream) -> Option<(u32, u32)> {
    let mut credentials = libc::ucred {
        pid: 0,
        uid: 0,
        gid: 0,
    };
    let mut length = libc::socklen_t::try_from(std::mem::size_of::<libc::ucred>()).ok()?;
    // SAFETY: the socket is open for as long as `client` is borrowed, and
    // the kernel writes at most `length` bytes into `credentials`.
    let read = unsafe {
        libc::getsockopt(
            client.as_raw_fd(),
            libc::SOL_SOCKET,
            libc::SO_PEERCRED,
            (&raw mut credentials).cast(),
            &raw mut length,
        )
    };
    let pid = u32::try_from(credentials.pid).ok().filter(|pid| *pid > 0);
    (read == 0).then_some(())?;
    Some((pid?, credentials.uid))
}

/// The account this process runs as.
pub fn own_uid() -> u32 {
    // SAFETY: geteuid takes nothing and can't fail.
    unsafe { libc::geteuid() }
}

/// Whether `client` is in a fence of this account, by the system's own
/// process table.
pub fn fenced(client: &UnixStream) -> Result<Fenced, Outside> {
    let (pid, uid) = credentials(client).ok_or(Outside::Gone)?;
    Processes::system().fenced(pid, uid, own_uid())
}

/// `PI_SESSION_ID` in an environment as `/proc/PID/environ` holds one,
/// when it's plain: letters, digits, `-` and `_`.
fn session_in(environ: &[u8]) -> Option<String> {
    let value = environ
        .split(|byte| *byte == 0)
        .find_map(|pair| pair.strip_prefix(b"PI_SESSION_ID="))?;
    let plain = !value.is_empty()
        && value.len() <= LONGEST
        && value
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || *byte == b'-' || *byte == b'_');
    plain.then(|| String::from_utf8_lossy(value).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    const ME: u32 = 1000;
    const HOST: &str = "4026531836";
    const FENCE: &str = "4026532001";
    const BWRAP: &str = "/nix/store/x-bubblewrap/bin/bwrap";

    /// Lays out one process in a stand-in for `/proc`: its namespace,
    /// parent, program, account and environment.
    fn lay_out(
        root: &Path,
        pid: &str,
        namespace: &str,
        parent: u32,
        exe: &str,
        uid: u32,
        environ: &[u8],
    ) -> std::io::Result<()> {
        let folder = root.join(pid);
        fs::create_dir_all(folder.join("ns"))?;
        std::os::unix::fs::symlink(format!("pid:[{namespace}]"), folder.join("ns/pid"))?;
        std::os::unix::fs::symlink(exe, folder.join("exe"))?;
        fs::write(
            folder.join("stat"),
            format!("{pid} (a (b) c) S {parent} 1 1 0 -1\n"),
        )?;
        fs::write(
            folder.join("status"),
            format!("Name:\tx\nUid:\t{uid}\t{uid}\t{uid}\t{uid}\n"),
        )?;
        fs::write(folder.join("environ"), environ)
    }

    /// A process table with the door, a shell outside, and a fenced
    /// command three levels below the bubblewrap pi started.
    fn table(name: &str) -> (Processes, PathBuf) {
        let root =
            std::env::temp_dir().join(format!("pi-clipboard-peer-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        lay_out(&root, "self", HOST, 1, "/bin/door", ME, b"").unwrap();
        lay_out(
            &root,
            "10",
            HOST,
            1,
            "/bin/bash",
            ME,
            b"PI_SESSION_ID=outer\0",
        )
        .unwrap();
        lay_out(
            &root,
            "20",
            HOST,
            10,
            BWRAP,
            ME,
            b"HOME=/h\0PI_SESSION_ID=01a1-s_2\0",
        )
        .unwrap();
        lay_out(&root, "21", FENCE, 20, BWRAP, ME, b"").unwrap();
        lay_out(&root, "22", FENCE, 21, "/bin/landrun", ME, b"").unwrap();
        lay_out(
            &root,
            "23",
            FENCE,
            22,
            "/bin/bash",
            ME,
            b"PI_SESSION_ID=forged\0",
        )
        .unwrap();
        (Processes::at(&root), root)
    }

    #[test]
    fn a_fenced_command_is_answered_for_its_bubblewraps_session() {
        let (processes, root) = table("answered");
        let fenced = processes.fenced(23, ME, ME);
        assert_eq!(
            fenced,
            Ok(Fenced {
                session: "01a1-s_2".into()
            })
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn a_command_outside_the_fence_is_refused() {
        let (processes, root) = table("outside");
        assert_eq!(processes.fenced(10, ME, ME), Err(Outside::Unfenced));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn another_account_is_refused() {
        let (processes, root) = table("account");
        assert_eq!(processes.fenced(23, 1001, ME), Err(Outside::OtherAccount));
        lay_out(&root, "30", HOST, 1, BWRAP, 1001, b"").unwrap();
        lay_out(&root, "31", FENCE, 30, "/bin/sh", ME, b"").unwrap();
        assert_eq!(processes.fenced(31, ME, ME), Err(Outside::OtherAccount));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn a_namespace_not_made_by_bubblewrap_is_refused() {
        let (processes, root) = table("container");
        lay_out(&root, "40", HOST, 1, "/bin/crun", ME, b"PI_SESSION_ID=x\0").unwrap();
        lay_out(&root, "41", FENCE, 40, "/bin/sh", ME, b"").unwrap();
        assert_eq!(processes.fenced(41, ME, ME), Err(Outside::NoBubblewrap));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn a_bubblewrap_inside_the_fence_names_no_session() {
        // A fenced command running bubblewrap itself, with a session of
        // its choosing: its bubblewrap is in the fence's namespace, so
        // the walk passes it for pi's own outside.
        let (processes, root) = table("nested");
        lay_out(&root, "50", FENCE, 23, BWRAP, ME, b"PI_SESSION_ID=forged\0").unwrap();
        lay_out(&root, "51", "4026532002", 50, "/bin/sh", ME, b"").unwrap();
        assert_eq!(
            processes.fenced(51, ME, ME),
            Ok(Fenced {
                session: "01a1-s_2".into()
            })
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn a_bubblewrap_whose_session_could_forge_a_field_is_unknown() {
        let (processes, root) = table("unknown");
        for (environ, at) in [
            &b"PI_SESSION_ID=a b\0"[..],
            b"PI_SESSION_ID=a\nrefused why=x\0",
            b"PI_SESSION_ID=\0",
            b"NOT_PI_SESSION_ID=a\0",
        ]
        .into_iter()
        .zip((60_u32..).step_by(2))
        {
            let fenced = at + 1;
            lay_out(&root, &at.to_string(), HOST, 1, BWRAP, ME, environ).unwrap();
            lay_out(&root, &fenced.to_string(), FENCE, at, "/bin/sh", ME, b"").unwrap();
            assert_eq!(
                processes.fenced(fenced, ME, ME),
                Ok(Fenced {
                    session: UNKNOWN.into()
                }),
                "{environ:?}"
            );
        }
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn a_client_gone_is_refused() {
        let (processes, root) = table("gone");
        assert_eq!(processes.fenced(99, ME, ME), Err(Outside::Gone));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn its_own_end_of_a_pair_is_this_process_and_outside() {
        let (ours, _theirs) = UnixStream::pair().expect("a pair");
        assert_eq!(credentials(&ours), Some((std::process::id(), own_uid())));
        assert_eq!(fenced(&ours), Err(Outside::Unfenced));
    }
}
