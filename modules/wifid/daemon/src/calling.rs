//! Whether Joel's on a call, as his session tells wifid on the launcher's
//! rows socket, so wifid keeps the card on its access point for the call
//! rather than move it to a quicker one and stall the call while it does,
//! unless its link fades: then it gives way at once to another point of
//! its network, or looks for one every two minutes (fading.rs).
//!
//! ```text
//! {"id": 1, "ask": "act", "act": "calling", "on": true}
//! {"id": 1, "done": true}
//! ```
//!
//! A call is said on at its start and once a minute while it lasts, and
//! over at its end. One said on and then never again lapses after ten
//! minutes, so a teller that stops can't hold the card forever. Any
//! process of the wifi group's can say it, every agent of Joel's among
//! them. One said falsely over a link it loads till echoes go unanswered
//! moves the card to another point of the network at most once each two
//! minutes, as loss on both is taken as the network's (`fading::AWAY`).
//! So each change is logged with the process and cgroup that said it, and
//! anything but exactly that shape is refused, logged by what was
//! wrong with it and never by what it said (2026-11-14).
//!
//! Long because its cases, over half of it, stay beside the private
//! readings of the kernel's word on an asker they prove, as every module
//! in wifid keeps them.

use std::ffi::OsStr;
use std::fmt;
use std::io;
use std::mem;
use std::os::fd::{AsRawFd, BorrowedFd, FromRawFd, OwnedFd};
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

use serde_json::Value;
use tokio::time::{Duration, Instant};

/// How long a call said on stays on with nothing more said of it.
pub const LAPSE: Duration = Duration::from_secs(600);

/// The longest a calling ask's line may be, in bytes: the longest the
/// shape can take with an id of a few digits.
pub const LONGEST: usize = 128;

/// What's last been said of a call, and when.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Calling {
    word: Option<(bool, Instant)>,
}

impl Calling {
    /// Takes what was said, answering whether it changed whether a call is
    /// on.
    pub fn told(&mut self, on: bool, now: Instant) -> bool {
        let was = self.on(now);
        self.word = Some((on, now));
        was != on
    }

    /// Whether a call is on now.
    pub fn on(&self, now: Instant) -> bool {
        self.word
            .is_some_and(|(on, at)| on && now.duration_since(at) <= LAPSE)
    }
}

/// Whether a calling ask says a call is on, read from its line strictly:
/// or what's wrong with it, to be logged in place of what it said.
pub fn asked(line: &str, said: &Value) -> Result<bool, &'static str> {
    if line.len() > LONGEST {
        return Err("too long");
    }
    let Some(fields) = said.as_object() else {
        return Err("not an object");
    };
    let own = ["id", "ask", "act", "on"];
    if fields.len() != own.len() || !own.iter().all(|field| fields.contains_key(*field)) {
        return Err("fields beyond its own");
    }
    if said["ask"] != "act" || said["act"] != "calling" {
        return Err("not a calling act");
    }
    if !(said["id"].is_number() || said["id"].is_string()) {
        return Err("an id neither a number nor a string");
    }
    said["on"].as_bool().ok_or("on isn't true or false")
}

/// Who said something on a connection, as the kernel tells it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Peer {
    pid: Option<i32>,
    /// The effective uid the process connected with, as the kernel took
    /// it then: a set-uid program's owner, not who ran it. No cgroup nested
    /// too deep to read can hide it.
    uid: Option<u32>,
    /// Where it runs, or why that went unsaid.
    cgroup: Cgroup,
}

/// Where an asker runs, or why it couldn't be said. Only `cgroup_of`
/// makes a path, so nothing unescaped reaches the journal through one.
#[derive(Clone, Debug, Default, PartialEq)]
enum Cgroup {
    /// The cgroup's path from the root, as `cgroup_of` says it, read
    /// while the process that connected was still there.
    Path(String),
    /// The path read where the kernel gives no pidfd for a socket, so
    /// nothing says the process read is the one that connected.
    Unproven(String),
    /// The process had exited by the time its path was read, so the pid it
    /// connected with may since be another's, and no path is said for it.
    Gone,
    /// Read, but with no unified line, as a host with no unified hierarchy
    /// gives.
    Unnamed,
    /// Not read, with the kernel's errno for why when it gave one: a path
    /// past `PATH_MAX`, which a process may nest on purpose to hide, is
    /// `ENAMETOOLONG`, told apart from a process merely gone.
    Unread(Option<i32>),
    /// No pid to read it by.
    #[default]
    Unasked,
}

impl Peer {
    /// The peer on a connection, as the kernel took its pid and uid when
    /// it connected, with the cgroup it's in. The path is read by that pid
    /// as wifid answers, which may be up to a minute on while connections
    /// queue through a restart, so the pid could by then be another's. The
    /// kernel's pidfd for the peer is asked after the read: a process that
    /// hasn't exited then is the one that connected, as a live process
    /// keeps its pid, and one that has is said as gone rather than named
    /// by whatever may have taken its number. A pid of 0
    /// is the kernel saying the peer is outside wifid's pid namespace,
    /// which it can't name, so it's said as none. A kernel older than 6.5
    /// gives no pidfd for a socket (`ENOPROTOOPT`), as the builders' did
    /// on 26 Nov 2026, so there the path is read as before and said as
    /// unproven, rather than said of no asker at all.
    pub fn on(socket: BorrowedFd<'_>, pid: Option<i32>, uid: Option<u32>) -> Peer {
        let pidfd = || pidfd_of(socket);
        Peer::read_by(pidfd, pid, uid, |number| {
            std::fs::read(format!("/proc/{number}/cgroup"))
        })
    }

    /// The peer on a connection as `on` says it, its pidfd had from
    /// `pidfd` and its cgroup file read by `read` from its pid, so a test
    /// can give a pidfd any kernel makes and have the asker exit as it's
    /// read. The pidfd is asked after the read, each a statement of its
    /// own: the read as an argument beside `held` would come after the
    /// poll, and an asker exiting between them would have another's path
    /// said as its own, proven. When the pidfd is had needn't matter, as
    /// the socket keeps the connecting process's pid: had after a reap,
    /// it's refused before 6.16 and polls exited from 6.16, as `held` says.
    fn read_by(
        pidfd: impl FnOnce() -> io::Result<OwnedFd>,
        pid: Option<i32>,
        uid: Option<u32>,
        read: impl FnOnce(i32) -> io::Result<Vec<u8>>,
    ) -> Peer {
        let pid = pid.filter(|&pid| pid > 0);
        let Some(number) = pid else {
            let cgroup = Cgroup::Unasked;
            return Peer { pid, uid, cgroup };
        };
        let pidfd = pidfd();
        let read = read(number);
        let cgroup = judged(held(pidfd), read, |path| {
            named(Path::new("/sys/fs/cgroup"), path)
        });
        Peer { pid, uid, cgroup }
    }
}

/// What the kernel's pidfd for an asker says of it once its path is read.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Held {
    /// It hasn't exited, so the path read is its own.
    Still,
    /// It has exited, or was reaped before wifid asked for its pidfd.
    Exited,
    /// The kernel gives no pidfd for a socket.
    NoPidfd,
    /// The pidfd couldn't be had or asked, with the kernel's errno.
    Failed(Option<i32>),
}

/// What the asker's pidfd, as `pidfd_of` gave it, says of it now. From
/// 6.16 the kernel gives a unix socket's peer a pidfd even once it's been
/// reaped (`PIDFD_STALE` in `net/core/sock.c`), so there it's the poll in
/// `exited`, not an errno, that says it's gone. Before, a peer reaped
/// before it's asked for gets `EINVAL`; `ESRCH` is taken as gone too, as
/// `pidfd_prepare` says it of a pid with no task where nothing turns it
/// into `EINVAL`.
fn held(pidfd: io::Result<OwnedFd>) -> Held {
    match pidfd {
        Ok(pidfd) => match exited(&pidfd) {
            Ok(false) => Held::Still,
            Ok(true) => Held::Exited,
            Err(error) => Held::Failed(error.raw_os_error()),
        },
        Err(error) => match error.raw_os_error() {
            Some(libc::EINVAL | libc::ESRCH) => Held::Exited,
            Some(libc::ENOPROTOOPT) => Held::NoPidfd,
            errno => Held::Failed(errno),
        },
    }
}

/// Whether a cgroup of that path from the root exists, read from its
/// bytes as the kernel wrote them. Only one found not there counts as
/// none: one wifid may not look into, as a user may close their own
/// delegated cgroups to it, is taken as there, so a live asker's isn't
/// said as removed.
fn named(root: &Path, path: &[u8]) -> bool {
    let at = root.join(OsStr::from_bytes(path));
    !matches!(std::fs::symlink_metadata(at), Err(error) if error.kind() == io::ErrorKind::NotFound)
}

/// The kernel's pidfd for the process on the other end of a socket, as it
/// was when that process connected.
fn pidfd_of(socket: BorrowedFd<'_>) -> io::Result<OwnedFd> {
    let mut fd: libc::c_int = -1;
    let mut length =
        libc::socklen_t::try_from(mem::size_of::<libc::c_int>()).map_err(io::Error::other)?;
    // SAFETY: the socket is borrowed for the call, and the option writes
    // an int into `fd`, whose size `length` holds.
    let got = unsafe {
        libc::getsockopt(
            socket.as_raw_fd(),
            libc::SOL_SOCKET,
            libc::SO_PEERPIDFD,
            (&raw mut fd).cast(),
            &raw mut length,
        )
    };
    if got < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: the kernel made the descriptor for this call, and nothing
    // else holds it.
    Ok(unsafe { OwnedFd::from_raw_fd(fd) })
}

/// Whether the process a pidfd names has exited, as a pidfd polls
/// readable once it has. Polling asks no permission, so no filter or
/// missing capability can make a process gone read as there. A zombie
/// still holds its pid but counts as exited, which only says less. Any
/// answer at all is taken as exited, not only `POLLIN`, since a live
/// pidfd answers none: one that answers only `POLLHUP`, `POLLERR` or
/// `POLLNVAL` isn't trusted to prove the path.
fn exited(pidfd: &OwnedFd) -> io::Result<bool> {
    let mut polled = libc::pollfd {
        fd: pidfd.as_raw_fd(),
        events: libc::POLLIN,
        revents: 0,
    };
    // SAFETY: it reads and writes the one pollfd, for one descriptor held
    // for the call, and waits for nothing.
    let ready = unsafe { libc::poll(&raw mut polled, 1, 0) };
    if ready < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(answered(polled.revents))
}

/// Whether a pidfd's poll answered at all, as `exited` takes any answer
/// to say the process is gone.
fn answered(revents: libc::c_short) -> bool {
    revents != 0
}

/// Where an asker runs, from what its pidfd said once its cgroup file was
/// read and what that read gave, `named` saying whether a cgroup of a
/// path exists.
///
/// An asker still there when the read was done can't have been reaped
/// before it, so a file not found says only that wifid's `/proc` doesn't
/// show it, as `ProtectProc=invisible` would, and is said as unread, not
/// gone. Its path's ` (deleted)` ending is taken as its cgroup's name,
/// with no lookup that a wrong root or a long path could spoil. The
/// kernel refuses to remove a cgroup while a task that hasn't exited is
/// in it (`cgroup_destroy_locked` asks `cgroup_is_populated`, which an
/// exited task no longer counts toward), and `pidfd_poll` answers only
/// once a process's every thread has exited, holding back a leader
/// whose other threads run, as it has since a pidfd could be polled, in
/// 5.3. So an asker still there reads so only as a threaded group's
/// leader, exited in a cgroup since removed while its threads run in
/// another, and that path is still one the asker ran in.
/// With no pidfd, the file not found says the asker was reaped, and the
/// ending is looked up by `named`.
fn judged(held: Held, read: io::Result<Vec<u8>>, named: impl Fn(&[u8]) -> bool) -> Cgroup {
    let cgroup = match (held, read) {
        (Held::Exited, _) => return Cgroup::Gone,
        // Without its pidfd where the kernel gives one, nothing says the
        // path read is the asker's.
        (Held::Failed(errno), _) => return Cgroup::Unread(errno),
        (Held::Still, Ok(cgroup)) => cgroup_of(&cgroup, |_| true),
        (Held::NoPidfd, Ok(cgroup)) => cgroup_of(&cgroup, named),
        // A process reaped before its file is opened leaves none, and one
        // reaped as it's read leaves it without its task.
        (Held::NoPidfd, Err(error))
            if error.kind() == io::ErrorKind::NotFound
                || error.raw_os_error() == Some(libc::ESRCH) =>
        {
            return Cgroup::Gone;
        }
        (Held::Still | Held::NoPidfd, Err(error)) => {
            return Cgroup::Unread(error.raw_os_error());
        }
    };
    match (cgroup, held) {
        (None, _) => Cgroup::Unnamed,
        (Some(path), Held::NoPidfd) => Cgroup::Unproven(path),
        (Some(path), _) => Cgroup::Path(path),
    }
}

impl fmt::Display for Peer {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        // A uid is said even with no pid, since it alone still says whose
        // the asker is.
        let Some(pid) = self.pid else {
            return match self.uid {
                Some(uid) => write!(out, "a process of uid {uid} the kernel gave no pid"),
                None => write!(out, "a process the kernel didn't name"),
            };
        };
        write!(out, "pid {pid}")?;
        if let Some(uid) = self.uid {
            write!(out, ", uid {uid}")?;
        }
        // Each way a path goes unsaid is said apart, so a process that
        // went away can be told from one that made itself unreadable, and
        // its uid still says whose it is.
        match &self.cgroup {
            // The path from the root, as a process names its own unit
            // under its user's manager: a unit called wifid-calls.service
            // there still reads as under user.slice, not system.slice.
            Cgroup::Path(cgroup) => write!(out, " ({cgroup})"),
            Cgroup::Unproven(cgroup) => write!(out, " ({cgroup}, unproven)"),
            Cgroup::Gone => write!(out, " (gone as its cgroup was read)"),
            Cgroup::Unnamed => write!(out, " (in no unified cgroup)"),
            Cgroup::Unread(Some(errno)) => write!(out, " (its cgroup unread, errno {errno})"),
            Cgroup::Unread(None) | Cgroup::Unasked => write!(out, " (its cgroup unread)"),
        }
    }
}

/// The path of the cgroup a process runs in, from the root, each character
/// no unit's name may hold said as `?`. A process names its own cgroup
/// under its user's manager, so the name could otherwise hold a control
/// character, which has journalctl show the whole line as a blob, or a
/// `) asks a look; pid 2211 (` writing a second asker into the line. Bytes
/// that aren't UTF-8 are said as `?` too. No name holds a line break, since
/// the kernel's `cgroup_mkdir` refuses one to keep this file readable, so
/// lines are split at that alone: a carriage return ending a name is the
/// name's, said as `?`. A host with no unified hierarchy has no `0::`
/// line, and gives none, said as in no unified cgroup.
///
/// The kernel ends the path with ` (deleted)` once a process's cgroup is
/// removed, as for a zombie whose unit has stopped, so that's said as the
/// cgroup it was, since removed. A zombie's pidfd polls exited, so this
/// is reached only where there's none, as `judged` says. A live cgroup
/// may be named to end so too, so a path `named`, one whose cgroup exists
/// by that whole name, keeps it as the name's, escaped as the rest.
/// It's looked up by the bytes the kernel wrote, before any is said as
/// `?`, so a name that isn't UTF-8 is found too. A name removed between
/// the read and that look is said as removed, which it then is.
fn cgroup_of(cgroup: &[u8], named: impl Fn(&[u8]) -> bool) -> Option<String> {
    let path = cgroup
        .split(|&byte| byte == b'\n')
        .find_map(|line| line.strip_prefix(b"0::/"))?;
    let (path, removed) = match path.strip_suffix(b" (deleted)") {
        Some(stripped) if !named(path) => (stripped, ", since removed"),
        _ => (path, ""),
    };
    let path = String::from_utf8_lossy(path);
    let allowed = |said: char| said.is_ascii_alphanumeric() || ":-_.\\@/".contains(said);
    let said: String = path
        .chars()
        .map(|said| if allowed(said) { said } else { '?' })
        .collect();
    // The root's path is empty, said as itself so it doesn't read as one
    // that went unread.
    let said = if said.is_empty() {
        "/".into()
    } else {
        cut(&said)
    };
    Some(format!("{said}{removed}"))
}

/// The most of a cgroup's path from its root an ask's line says, enough
/// for `user.slice/user-1000.slice/user@1000.service/`, which says whose
/// manager runs it.
const SLICE_MOST: usize = 64;

/// The most of a cgroup's path at its end an ask's line says: a unit's
/// name is at most 255 characters, and the `/` before it one more.
const UNIT_MOST: usize = 256;

/// What stands for the middle of a path too long to say whole.
const CUT: &str = "...";

/// A path as an ask's line says it: whole up to `SLICE_MOST`, `CUT` and
/// `UNIT_MOST` together, and past that its start and end around `CUT`. A
/// process may nest cgroups under its own manager until the path nears
/// `PATH_MAX`, past which the kernel refuses to say it, and 4 KB of
/// `a/a/a/` on each line it asks would bury who asked. The path is ASCII
/// by now, so a character is a byte.
fn cut(path: &str) -> String {
    if path.len() <= SLICE_MOST + CUT.len() + UNIT_MOST {
        return path.to_owned();
    }
    let (start, end) = (&path[..SLICE_MOST], &path[path.len() - UNIT_MOST..]);
    format!("{start}{CUT}{end}")
}

#[cfg(test)]
mod tests {
    use std::os::fd::AsFd;
    use std::os::unix::process::CommandExt;

    use serde_json::json;

    use super::*;

    /// A cgroup file as `cgroup_of` reads it where no cgroup is named to
    /// end in ` (deleted)`.
    fn cgroup_of(cgroup: &[u8]) -> Option<String> {
        super::cgroup_of(cgroup, |_| false)
    }

    fn read(said: &Value) -> Result<bool, &'static str> {
        asked(&said.to_string(), said)
    }

    #[test]
    fn a_call_said_on_is_on_and_said_over_is_over() {
        let (mut calling, start) = (Calling::default(), Instant::now());
        assert!(!calling.on(start), "nothing said, no call");
        assert!(calling.told(true, start), "a call starting changes it");
        assert!(calling.on(start), "a call said on is on");
        let later = start + Duration::from_secs(60);
        assert!(!calling.told(true, later), "said again, it's no change");
        assert!(calling.told(false, later), "a call ending changes it");
        assert!(!calling.on(later), "a call said over is over");
    }

    #[test]
    fn a_call_said_on_and_never_again_lapses() {
        let (mut calling, start) = (Calling::default(), Instant::now());
        calling.told(true, start);
        assert!(calling.on(start + LAPSE), "on until the lapse");
        assert!(
            !calling.on(start + LAPSE + Duration::from_secs(1)),
            "a teller gone quiet holds nothing"
        );
    }

    #[test]
    fn a_calling_ask_of_its_own_shape_is_read() {
        let on = json!({"id": 1, "ask": "act", "act": "calling", "on": true});
        let over = json!({"id": "a", "ask": "act", "act": "calling", "on": false});
        assert_eq!(read(&on), Ok(true), "a call on");
        assert_eq!(read(&over), Ok(false), "a call over");
    }

    #[test]
    fn a_calling_ask_of_any_other_shape_is_refused_by_what_is_wrong() {
        let more = json!({"id": 1, "ask": "act", "act": "calling", "on": true, "for": "x"});
        let unsaid = json!({"id": 1, "ask": "act", "act": "calling"});
        let worded = json!({"id": 1, "ask": "act", "act": "calling", "on": "yes"});
        let odd = json!({"id": [1], "ask": "act", "act": "calling", "on": true});
        assert_eq!(
            read(&more),
            Err("fields beyond its own"),
            "one field too many"
        );
        assert_eq!(read(&unsaid), Err("fields beyond its own"), "one too few");
        assert_eq!(read(&worded), Err("on isn't true or false"), "on as a word");
        assert_eq!(
            read(&odd),
            Err("an id neither a number nor a string"),
            "a list for an id"
        );
        let long = format!("{{\"id\": \"{}\"}}", "x".repeat(LONGEST));
        assert_eq!(
            asked(&long, &serde_json::from_str(&long).expect("JSON")),
            Err("too long"),
            "the line's length is capped"
        );
    }

    #[test]
    fn a_process_is_named_by_its_cgroups_whole_path() {
        let path = "user.slice/user-1000.slice/user@1000.service/app.slice/wifid-calls.service";
        assert_eq!(
            cgroup_of(format!("0::/{path}\n").as_bytes()).as_deref(),
            Some(path),
            "the path"
        );
        assert_eq!(cgroup_of(b"0::/\n").as_deref(), Some("/"), "the root");
        assert_eq!(
            cgroup_of(b"1:name=systemd:/a\n0::/system.slice/x.service\n").as_deref(),
            Some("system.slice/x.service"),
            "a hybrid host's unified line, after its controllers', went unread"
        );
        assert_eq!(
            cgroup_of(b"1:name=systemd:/system.slice/x.service\n"),
            None,
            "a host with no unified hierarchy named a path from a controller's line"
        );
        assert_eq!(cgroup_of(b""), None, "nothing read names none");
        assert_eq!(
            cgroup_of(b"0::/system.slice/x\xff\xc3\xa9.service\n").as_deref(),
            Some("system.slice/x??.service"),
            "a path not in UTF-8 named no cgroup, or said a character as more than one ?"
        );
        assert_eq!(
            cgroup_of(b"0::/system.slice/x.service\r\n").as_deref(),
            Some("system.slice/x.service?"),
            "a carriage return ending a name was dropped, passing it as another unit"
        );
        assert_eq!(
            cgroup_of(b"0::/system.slice/x.service (deleted)\n").as_deref(),
            Some("system.slice/x.service, since removed"),
            "a removed cgroup's path was said with the kernel's mark escaped into it"
        );
    }

    #[test]
    fn a_cgroup_nested_deep_says_its_slice_and_its_unit() {
        let user = "user.slice/user-1000.slice/user@1000.service/app.slice/";
        // A unit's name as long as one may be, so the end kept must hold
        // all of it, and a start and end swapped would lose it.
        let unit = format!("{}.service", "u".repeat(255 - ".service".len()));
        let deep = format!("{user}{}{unit}", "a/".repeat(2000));

        let said = cgroup_of(format!("0::/{deep}\n").as_bytes()).expect("a path");

        assert_eq!(
            said,
            format!(
                "{}...{}",
                &deep[..SLICE_MOST],
                &deep[deep.len() - UNIT_MOST..]
            ),
            "a deep path wasn't cut to its start and end"
        );
        assert!(
            said.starts_with("user.slice/user-1000.slice/user@1000.service/"),
            "a cut path lost whose manager runs it: {said}"
        );
        assert!(
            said.ends_with(&format!("/{unit}")),
            "a cut path lost its unit's whole name: {said}"
        );
        let most = SLICE_MOST + "...".len() + UNIT_MOST;
        let whole = format!("{user}{}", "x".repeat(most - user.len()));
        let past = format!("{whole}y");
        assert_eq!(
            cgroup_of(format!("0::/{whole}\n").as_bytes()),
            Some(whole),
            "a path no longer than its cut would be was cut"
        );
        assert_eq!(
            cgroup_of(format!("0::/{past}\n").as_bytes()).map(|said| said.len()),
            Some(most),
            "a path one past the most wasn't cut"
        );
        // Two-byte characters from an odd byte on, so one straddles the
        // cut's first edge as read: the path is escaped whole before it's
        // cut, so the cut falls between characters.
        let wide = format!("a{}/{}{unit}", "é".repeat(80), "é/".repeat(300));
        let said = cgroup_of(format!("0::/{wide}\n").as_bytes()).expect("a path");
        assert!(
            said.is_ascii() && said.len() == most,
            "a path of wide characters wasn't escaped whole and then cut: {said}"
        );
        assert!(
            said.starts_with(&format!("a{}", "?".repeat(SLICE_MOST - 1))),
            "a wide character before the cut wasn't said as one ?: {said}"
        );
        assert!(
            said.ends_with(&format!("/{unit}")),
            "a path of wide characters lost its unit's name: {said}"
        );
    }

    #[test]
    fn a_cgroup_says_no_byte_a_units_name_cant_hold() {
        let user = "user.slice/user-1000.slice/user@1000.service/app.slice";
        let escaping = format!("0::/{user}/\u{1b}[2Jx.service\n");
        let forging = format!("0::/{user}/init.scope) asks a look; pid 2211 (quickshell.service\n");

        assert_eq!(
            cgroup_of(escaping.as_bytes()),
            Some(format!("{user}/??2Jx.service")),
            "a unit's control byte reached the journal raw"
        );
        assert_eq!(
            cgroup_of(forging.as_bytes()),
            Some(format!(
                "{user}/init.scope??asks?a?look??pid?2211??quickshell.service"
            )),
            "a unit named to close its own bracket wrote a second asker into the line"
        );
        assert_eq!(
            cgroup_of(format!("0::/{user}/a\u{7}b.slice/x.service\n").as_bytes()),
            Some(format!("{user}/a?b.slice/x.service")),
            "a control byte in a slice before the unit reached the journal raw"
        );
    }

    #[test]
    fn a_peer_says_what_the_kernel_named() {
        let named = Peer {
            pid: Some(42),
            uid: Some(1000),
            cgroup: Cgroup::Path("system.slice/wifid-calls.service".into()),
        };
        assert_eq!(
            named.to_string(),
            "pid 42, uid 1000 (system.slice/wifid-calls.service)"
        );
        for (cgroup, said) in [
            (
                Cgroup::Unproven("system.slice/wifid-calls.service".into()),
                "system.slice/wifid-calls.service, unproven",
            ),
            (Cgroup::Gone, "gone as its cgroup was read"),
            (Cgroup::Unnamed, "in no unified cgroup"),
            (
                Cgroup::Unread(Some(libc::ENAMETOOLONG)),
                "its cgroup unread, errno 36",
            ),
            (Cgroup::Unread(None), "its cgroup unread"),
        ] {
            assert_eq!(
                Peer {
                    cgroup,
                    ..named.clone()
                }
                .to_string(),
                format!("pid 42, uid 1000 ({said})"),
                "a process whose cgroup went unsaid passed as one in none, or as unsaid another way"
            );
        }
        assert_eq!(
            Peer {
                uid: None,
                ..named.clone()
            }
            .to_string(),
            "pid 42 (system.slice/wifid-calls.service)",
            "a peer with no uid said one"
        );
        assert_eq!(
            Peer {
                pid: None,
                cgroup: Cgroup::Unasked,
                ..named
            }
            .to_string(),
            "a process of uid 1000 the kernel gave no pid",
            "a peer with no pid lost the uid that says whose it is"
        );
        assert_eq!(
            Peer::default().to_string(),
            "a process the kernel didn't name"
        );
    }

    #[test]
    fn a_peer_is_read_from_the_kernel_and_says_when_it_couldnt_be() {
        // A pair's other end is this test's own process, alive.
        let (socket, _other) = std::os::unix::net::UnixStream::pair().expect("a pair");
        let own = i32::try_from(std::process::id()).expect("a pid fits");
        let here = Peer::on(socket.as_fd(), Some(own), Some(1000));
        // No pid reaches past pid_max, 2^22 at most, so none is there.
        let gone = Peer::on(socket.as_fd(), Some(i32::MAX), Some(1000));
        let pidfd = pidfd_of(socket.as_fd());

        // Proven only where the kernel gives a pidfd, and said unproven
        // only where it gives none, so a proof that never ran shows.
        // The pid read past pid_max is beside this test's own live
        // socket, so with a pidfd it's an asker still there whose file
        // wifid can't see, as a hidden /proc gives, and with none it's
        // one reaped.
        let unseen = match pidfd.as_ref().map_err(io::Error::raw_os_error) {
            Ok(_) => {
                assert!(
                    matches!(here.cgroup, Cgroup::Path(_)),
                    "this test's own cgroup went unproven or unread: {here}"
                );
                format!("its cgroup unread, errno {}", libc::ENOENT)
            }
            Err(Some(libc::ENOPROTOOPT)) => {
                assert!(
                    matches!(here.cgroup, Cgroup::Unproven(_)),
                    "a kernel with no pidfd for a socket proved this test's own cgroup: {here}"
                );
                "gone as its cgroup was read".to_owned()
            }
            Err(errno) => panic!("this test's own pidfd went unread, errno {errno:?}"),
        };
        assert_eq!(
            gone.to_string(),
            format!("pid {}, uid 1000 ({unseen})", i32::MAX),
            "a pid that isn't there named a cgroup, or a live asker's unseen file was said gone"
        );
        assert_eq!(
            Peer::on(socket.as_fd(), Some(0), Some(1000)).to_string(),
            "a process of uid 1000 the kernel gave no pid",
            "a peer outside wifid's pid namespace was read as pid 0, the kernel's own"
        );
    }

    /// A `sleep` child, killed and reaped when dropped, so a case that
    /// fails before it does leaves none running. A child a case already
    /// reaped isn't signalled again: `Child::kill` sends nothing once the
    /// standard library holds the child's status, which `wait` keeps
    /// (`Process::send_signal` in `library/std/src/sys/process/unix/unix.rs`,
    /// as rust-lang/rust's master read on 27 Nov 2026), so its pid, which
    /// may since be another's, is never killed.
    struct Sleeper(std::process::Child);

    impl Drop for Sleeper {
        fn drop(&mut self) {
            // Killed, then reaped whether or not the kill failed, as a
            // tuple's parts are evaluated in order. It's one line, so it
            // isn't the copy of other tests' guards that the rule
            // every-test-function-lives-once refuses.
            let _ = (self.0.kill(), self.0.wait());
        }
    }

    /// A `sleep` child, with its pidfd from `pidfd_open`, which every
    /// kernel since 5.3 gives, so a case built on it runs the same where
    /// a socket gives no pidfd, as the builders' kernels did. It dies with
    /// the thread that started it (`dies_with`), so a test runner killed
    /// without unwinding leaves none running either.
    fn a_sleeper() -> (Sleeper, libc::pid_t, OwnedFd) {
        // SAFETY: it takes nothing and gives a number.
        let parent = unsafe { libc::getpid() };
        let mut command = std::process::Command::new("sleep");
        command.arg("60");
        // SAFETY: the hook runs in the child between its fork and its exec,
        // and calls only what `dies_with` says is safe there. The ask
        // outlasts the exec, as `sleep` isn't set-uid.
        unsafe {
            command.pre_exec(move || {
                dies_with(parent);
                Ok(())
            });
        }
        let child = Sleeper(command.spawn().expect("sleep starts"));
        let pid = libc::pid_t::try_from(child.0.id()).expect("a pid fits");
        (child, pid, pidfd_open(pid))
    }

    /// Asks the kernel to kill this child once the thread that forked it
    /// ends, as `parent`'s: a test process that dies without unwinding
    /// never drops the child, so the kernel kills it then instead. The
    /// thread's end comes after the drop, so a case that passes reaps its
    /// child first. Called in a child straight after its fork, it calls
    /// only `prctl`, `getppid` and `_exit`, each safe there in a process
    /// of many threads.
    fn dies_with(parent: libc::pid_t) {
        // SAFETY: they take only numbers, and end only this child.
        unsafe {
            libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL);
            // A parent gone before the ask never kills the child, so it
            // exits itself. It exits 3 so that whoever reaps the orphan,
            // init or a subreaper, can tell why: the case that forked it
            // is gone, so no `left` of a case ever sees it, and only the
            // case that pins this exit does.
            if libc::getppid() != parent {
                libc::_exit(3);
            }
        }
    }

    /// A process's pidfd from `pidfd_open`, which any process may ask of
    /// another's pid, not only its parent.
    fn pidfd_open(pid: libc::pid_t) -> OwnedFd {
        // SAFETY: it takes only numbers, and the descriptor it gives is
        // taken straight into an OwnedFd.
        let fd = unsafe { libc::syscall(libc::SYS_pidfd_open, pid, 0) };
        let fd = i32::try_from(fd).expect("a descriptor fits");
        assert!(fd >= 0, "no pidfd: {}", io::Error::last_os_error());
        // SAFETY: the kernel made it for this call, and nothing else holds
        // it.
        unsafe { OwnedFd::from_raw_fd(fd) }
    }

    /// Whether the process a pidfd names exits within 10 s, as a pidfd
    /// polls readable once it has: far past a kill landing on the busiest
    /// builder, and bounded, so a process that never exits fails the case
    /// rather than hang the run.
    fn exits_soon(pidfd: &OwnedFd) -> bool {
        let mut polled = libc::pollfd {
            fd: pidfd.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        // SAFETY: one pollfd, for a descriptor held for the call.
        let ready = unsafe { libc::poll(&raw mut polled, 1, 10_000) };
        ready == 1
    }

    #[test]
    fn a_child_asked_to_die_with_its_parent_goes_once_its_parent_does() {
        let mut ends = [0; 2];
        // SAFETY: it writes the two ints given.
        let piped = unsafe { libc::pipe2(ends.as_mut_ptr(), libc::O_CLOEXEC) };
        assert_eq!(piped, 0, "no pipe: {}", io::Error::last_os_error());
        // SAFETY: the kernel made it for this call, and nothing else holds
        // it.
        let reading = unsafe { OwnedFd::from_raw_fd(ends[0]) };
        // SAFETY: as above.
        let writing = unsafe { OwnedFd::from_raw_fd(ends[1]) };
        // SAFETY: it takes nothing and gives a number.
        let runner = unsafe { libc::getpid() };
        // SAFETY: the child calls only what `dies_with` does, getpid, fork
        // and pause, and its own child what `dies_with` does, getpid, write
        // and pause, each safe after a fork in a process of many threads,
        // and neither returns into the test.
        let pid = unsafe { libc::fork() };
        if pid == 0 {
            dies_with(runner);
            // SAFETY: as above.
            unsafe {
                let parent = libc::getpid();
                if libc::fork() == 0 {
                    dies_with(parent);
                    // Its pid, said only once it's asked, so the parent
                    // can't go before the ask and leave the case passing
                    // on the exit that `dies_with` makes for that.
                    let own = libc::getpid();
                    let size = mem::size_of::<libc::pid_t>();
                    libc::write(writing.as_raw_fd(), (&raw const own).cast(), size);
                }
                loop {
                    libc::pause();
                }
            }
        }
        assert!(pid > 0, "no fork: {}", io::Error::last_os_error());
        let parent = Forked {
            pid,
            reaped: std::cell::Cell::new(false),
        };
        drop(writing);
        let mut waiting = libc::pollfd {
            fd: reading.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        // SAFETY: one pollfd, for a descriptor held for the call. A child
        // that never forks says nothing, so the wait is bounded, at 10 s,
        // far past a fork on the busiest builder.
        let ready = unsafe { libc::poll(&raw mut waiting, 1, 10_000) };
        assert_eq!(ready, 1, "the child never said it had asked");
        let mut child: libc::pid_t = 0;
        let size = mem::size_of::<libc::pid_t>();
        // SAFETY: it writes at most the size of the one pid given.
        let got = unsafe { libc::read(reading.as_raw_fd(), (&raw mut child).cast(), size) };
        assert_eq!(
            usize::try_from(got).ok(),
            Some(size),
            "the child's pid was cut"
        );
        // Had while its parent lives and doesn't wait, so the pid is still
        // the child's.
        let pidfd = pidfd_open(child);

        parent.gone();
        let went = exits_soon(&pidfd);
        if !went {
            // SAFETY: it takes only numbers and the pidfd held, which
            // names the child however long it's been orphaned.
            unsafe {
                libc::syscall(
                    libc::SYS_pidfd_send_signal,
                    pidfd.as_raw_fd(),
                    libc::SIGKILL,
                    std::ptr::null::<libc::siginfo_t>(),
                    0,
                );
            }
        }

        assert!(
            went,
            "a child asked to die with its parent outlived it, so a killed runner leaves it"
        );
    }

    #[test]
    fn a_child_whose_parent_went_before_it_asked_to_die_with_it_exits() {
        // SAFETY: the child calls only getpid, what `dies_with` does and
        // pause, each safe after a fork in a process of many threads, and
        // never returns into the test.
        let pid = unsafe { libc::fork() };
        if pid == 0 {
            // SAFETY: as above. Its own pid, never its parent's, stands for
            // a parent gone before the ask, whose orphan's parent is by
            // then init or a subreaper.
            unsafe {
                dies_with(libc::getpid());
                loop {
                    libc::pause();
                }
            }
        }
        assert!(pid > 0, "no fork: {}", io::Error::last_os_error());
        let child = Forked {
            pid,
            reaped: std::cell::Cell::new(false),
        };
        // Had before the wait, so the pid is still the child's, exited
        // or not.
        let went = exits_soon(&pidfd_open(pid));
        let mut status = 0;
        // SAFETY: it takes only numbers and the one int written, and
        // waits for nothing.
        let reaped = unsafe { libc::waitpid(pid, &raw mut status, libc::WNOHANG) };
        child.reaped.set(reaped == pid);

        assert!(
            went,
            "a child whose parent went before it asked kept running"
        );
        assert!(
            reaped == pid && libc::WIFEXITED(status) && libc::WEXITSTATUS(status) == 3,
            "a child whose parent went before it asked didn't exit 3, status {status:#x}"
        );
    }

    #[test]
    fn a_process_is_said_exited_once_it_has() {
        let (mut child, pid, pidfd) = a_sleeper();
        let asked = || held(Ok(pidfd.try_clone().expect("a pidfd copies")));

        let running = asked();
        child.0.kill().expect("sleep is killed");
        // It waits for the kill to land.
        let landed = exits_soon(&pidfd);
        let killed = asked();
        // A zombie's file still reads, so only the poll keeps its path
        // from being said as a proven asker's.
        let zombie_read = std::fs::read(format!("/proc/{pid}/cgroup"));
        child.0.wait().expect("sleep is reaped");
        let reaped = asked();

        assert_eq!(running, Held::Still, "a running process was said exited");
        assert!(landed, "a killed process never polled readable");
        assert_eq!(
            killed,
            Held::Exited,
            "a killed process not yet reaped was said there"
        );
        let zombie_read = zombie_read.expect("a zombie's cgroup file still reads");
        assert!(
            matches!(
                judged(Held::Still, Ok(zombie_read), |_| true),
                Cgroup::Path(_)
            ),
            "a zombie's file named no path, so its poll wasn't what kept it unsaid"
        );
        assert_eq!(
            reaped,
            Held::Exited,
            "a reaped process was said there, so another's path could be said as its"
        );
    }

    #[test]
    fn any_answer_from_a_pidfd_s_poll_says_its_process_exited() {
        assert!(
            !answered(0),
            "a live pidfd, which answers none, was said exited"
        );
        for revents in [libc::POLLIN, libc::POLLHUP, libc::POLLERR, libc::POLLNVAL] {
            assert!(
                answered(revents),
                "a pidfd answering {revents:#x} was taken as live, so its asker's path was proven"
            );
        }
    }

    /// A child a case forked, killed and reaped by `gone`, or when dropped,
    /// so a test that fails leaves no child paused, and killed by the
    /// kernel should the thread that forked it end first (`dies_with`).
    /// Dropped, it kills its pid unless `gone` or `left` reaped it, which
    /// is safe only while no test waits on any child or ignores `SIGCHLD`:
    /// one that did could reap it first and let its pid go to another.
    struct Forked {
        pid: libc::pid_t,
        reaped: std::cell::Cell<bool>,
    }

    impl Forked {
        /// Kill the child, and reap it.
        fn gone(&self) {
            let mut status = 0;
            // SAFETY: they take only numbers and the one int written.
            let (killed, reaped) = unsafe {
                (
                    libc::kill(self.pid, libc::SIGKILL),
                    libc::waitpid(self.pid, &raw mut status, 0),
                )
            };
            // A wait that fails, as it does on a child reaped elsewhere, is
            // taken as reaped too, as `left` takes it: the pid may since be
            // another process's, which the drop mustn't kill.
            if reaped == self.pid || reaped < 0 {
                self.reaped.set(true);
            }
            assert_eq!(
                (killed, reaped),
                (0, self.pid),
                "the child wasn't killed and reaped"
            );
        }

        /// Fails the test if the child has exited, with how it did, as a
        /// child that can't connect exits 1 for no socket and 2 for a
        /// connect refused. Fails it too if the child can't be waited on,
        /// as one reaped elsewhere would otherwise cost the whole wait.
        fn left(&self) {
            let mut status = 0;
            // SAFETY: it takes only numbers and the one int written, and
            // waits for nothing.
            let reaped = unsafe { libc::waitpid(self.pid, &raw mut status, libc::WNOHANG) };
            if reaped < 0 {
                let error = io::Error::last_os_error();
                self.reaped.set(true);
                panic!("the child can't be waited on, so another reaped it: {error}");
            }
            if reaped == self.pid {
                self.reaped.set(true);
                let how = if libc::WIFEXITED(status) {
                    format!(
                        "exit {} (1: no socket, 2: refused)",
                        libc::WEXITSTATUS(status)
                    )
                } else {
                    format!("killed by signal {}", libc::WTERMSIG(status))
                };
                panic!("the child exited before it connected, {how}");
            }
        }
    }

    impl Drop for Forked {
        fn drop(&mut self) {
            // Only a child not yet reaped is killed, as a reaped one's pid
            // may since be another's.
            if !self.reaped.get() {
                let mut status = 0;
                // SAFETY: they take only numbers and the one int written.
                unsafe {
                    libc::kill(self.pid, libc::SIGKILL);
                    libc::waitpid(self.pid, &raw mut status, 0);
                }
            }
        }
    }

    /// A child process connected to a listener of this test's, with the
    /// connection's end here. Once connected it waits on signals, so the
    /// test says when it goes, by `gone`.
    fn a_child_connected() -> (std::os::unix::net::UnixStream, Forked) {
        let folder = tempfile::tempdir().expect("a folder");
        let at = folder.path().join("s");
        let listener = std::os::unix::net::UnixListener::bind(&at).expect("a listener");
        // SAFETY: a sockaddr_un is plain numbers, all zero a valid one.
        let mut address: libc::sockaddr_un = unsafe { mem::zeroed() };
        address.sun_family = libc::sa_family_t::try_from(libc::AF_UNIX).expect("a family");
        let bytes = at.as_os_str().as_bytes();
        assert!(
            bytes.len() < address.sun_path.len(),
            "a socket path too long: {at:?}"
        );
        for (slot, &byte) in address.sun_path.iter_mut().zip(bytes) {
            *slot = libc::c_char::from_ne_bytes([byte]);
        }
        let length = libc::socklen_t::try_from(mem::size_of::<libc::sockaddr_un>())
            .expect("an address's length fits");
        // SAFETY: it takes nothing and gives a number.
        let parent = unsafe { libc::getpid() };
        // SAFETY: the child calls only what `dies_with` does, socket,
        // connect, pause and _exit, each safe after a fork in a process of
        // many threads, and never returns into the test.
        let pid = unsafe { libc::fork() };
        if pid == 0 {
            dies_with(parent);
            // SAFETY: as above; the address was made before the fork.
            unsafe {
                let fd = libc::socket(libc::AF_UNIX, libc::SOCK_STREAM, 0);
                if fd < 0 {
                    libc::_exit(1);
                }
                if libc::connect(fd, (&raw const address).cast(), length) < 0 {
                    libc::_exit(2);
                }
                loop {
                    libc::pause();
                }
            }
        }
        assert!(pid > 0, "no fork: {}", io::Error::last_os_error());
        let child = Forked {
            pid,
            reaped: std::cell::Cell::new(false),
        };
        let mut waiting = libc::pollfd {
            fd: listener.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        // A child that can't connect exits, so the wait is bounded rather
        // than left to hang the run in accept, and looks for that exit
        // every 0.1 s rather than wait the bound out: ten seconds in all
        // is far past a connect on the busiest builder.
        for _ in 0..100 {
            // SAFETY: one pollfd, for a descriptor held for the call.
            let ready = unsafe { libc::poll(&raw mut waiting, 1, 100) };
            assert!(
                ready >= 0,
                "the wait for the child's connection failed: {}",
                io::Error::last_os_error()
            );
            if ready == 1 {
                let (connected, _) = listener.accept().expect("the child connects");
                return (connected, child);
            }
            child.left();
        }
        panic!("the child neither connected nor exited in 10 s");
    }

    #[test]
    fn a_child_reaped_elsewhere_is_taken_as_reaped_so_its_pid_is_never_killed() {
        let (_connected, child) = a_child_connected();
        let mut status = 0;
        // SAFETY: they take only numbers and the one int written, and the
        // wait reaps only this case's own child.
        let reaped = unsafe {
            libc::kill(child.pid, libc::SIGKILL);
            libc::waitpid(child.pid, &raw mut status, 0)
        };
        assert_eq!(reaped, child.pid, "the child wasn't reaped out of band");

        // Its kill and its wait each fail on a child already reaped, so
        // `gone` fails the case it's in, which this one catches.
        let gone = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| child.gone()));
        let taken = child.reaped.get();
        // Set whatever `gone` did, so one that forgets never has the drop
        // kill a pid that may since be another process's.
        child.reaped.set(true);

        assert!(
            gone.is_err(),
            "a child reaped elsewhere was said killed and reaped"
        );
        assert!(
            taken,
            "a child whose wait failed wasn't taken as reaped, so the drop would kill its pid"
        );
    }

    #[test]
    fn an_asker_reaped_before_it_s_read_is_said_gone() {
        let (connected, child) = a_child_connected();
        child.gone();

        // Gone whatever the kernel: from 6.16 its stale pidfd polls
        // exited, from 6.5 it's refused one, and before, its file's gone.
        let peer = Peer::on(connected.as_fd(), Some(child.pid), Some(1000));

        assert_eq!(
            peer.cgroup,
            Cgroup::Gone,
            "an asker reaped before it was read named a cgroup: {peer}"
        );
    }

    #[test]
    fn an_asker_s_file_is_read_before_its_pidfd_is_asked() {
        let (mut child, pid, pidfd) = a_sleeper();

        // The asker goes as its file is read, so a poll before the read
        // would find it still there and prove the path read after.
        let peer = Peer::read_by(
            || Ok(pidfd),
            Some(pid),
            Some(1000),
            |_| {
                child.0.kill().expect("sleep is killed");
                child.0.wait().expect("sleep is reaped");
                Ok(b"0::/system.slice/x.service\n".to_vec())
            },
        );

        assert_eq!(
            peer.cgroup,
            Cgroup::Gone,
            "a path read as its asker went was proven, so the poll came first: {peer}"
        );
    }

    #[test]
    fn an_asker_s_socket_gives_the_pidfd_that_proves_its_path() {
        let (connected, child) = a_child_connected();
        let pidfd = pidfd_of(connected.as_fd());

        // Where the kernel gives a socket's pidfd, it's that one which
        // proves the path; the order alone is pinned on every kernel by
        // the case above, through `read_by` too.
        let socket = connected.as_fd();
        let peer = Peer::read_by(
            || pidfd_of(socket),
            Some(child.pid),
            Some(1000),
            |_| {
                child.gone();
                Ok(b"0::/system.slice/x.service\n".to_vec())
            },
        );

        let (want, wrong) = match pidfd.as_ref().map_err(io::Error::raw_os_error) {
            Ok(_) => (
                Cgroup::Gone,
                "a path read as its asker went was proven, so the socket's pidfd wasn't asked",
            ),
            Err(Some(libc::ENOPROTOOPT)) => (
                Cgroup::Unproven("system.slice/x.service".into()),
                "a kernel with no pidfd for a socket didn't say the path read unproven",
            ),
            Err(errno) => panic!("the child's pidfd went unread, errno {errno:?}"),
        };
        assert_eq!(peer.cgroup, want, "{wrong}: {peer}");
    }

    #[test]
    fn an_asker_s_pidfd_says_whether_its_path_is_its_own() {
        let read = || Ok(b"0::/system.slice/x.service\n".to_vec());
        let path = "system.slice/x.service".to_owned();
        let errno = |errno| Err(io::Error::from_raw_os_error(errno));

        assert_eq!(
            judged(Held::Still, read(), |_| true),
            Cgroup::Path(path.clone()),
            "a path read while its asker was there went unproven"
        );
        assert_eq!(
            judged(Held::Exited, read(), |_| true),
            Cgroup::Gone,
            "a path read once its asker had exited was said as its"
        );
        assert_eq!(
            judged(Held::NoPidfd, read(), |_| true),
            Cgroup::Unproven(path),
            "a path read with no pidfd to prove it read as proven"
        );
        assert_eq!(
            judged(Held::Failed(Some(libc::EMFILE)), read(), |_| true),
            Cgroup::Unread(Some(libc::EMFILE)),
            "a path said where its pidfd couldn't be had"
        );
        assert_eq!(
            held(errno(libc::EINVAL)),
            Held::Exited,
            "reaped, 6.5 to 6.15"
        );
        assert_eq!(
            held(errno(libc::ESRCH)),
            Held::Exited,
            "a pid with no task, said so by pidfd_prepare"
        );
        assert_eq!(held(errno(libc::ENOPROTOOPT)), Held::NoPidfd, "before 6.5");
        assert_eq!(
            held(errno(libc::EMFILE)),
            Held::Failed(Some(libc::EMFILE)),
            "a pidfd that couldn't be had passed as a proof"
        );
    }

    #[test]
    fn a_removed_cgroups_mark_is_a_live_name_for_an_asker_still_there() {
        let read = || Ok(b"0::/system.slice/x.service (deleted)\n".to_vec());
        assert_eq!(
            judged(Held::Still, read(), |_| false),
            Cgroup::Path("system.slice/x.service??deleted?".into()),
            "an asker still there had its path's mark looked up, not kept as its cgroup's name"
        );
        assert_eq!(
            judged(Held::NoPidfd, read(), |_| false),
            Cgroup::Unproven("system.slice/x.service, since removed".into()),
            "with no pidfd, a cgroup not found by its marked name wasn't said as removed"
        );
        assert_eq!(
            judged(Held::NoPidfd, read(), |_| true),
            Cgroup::Unproven("system.slice/x.service??deleted?".into()),
            "with no pidfd, a live cgroup named to end so was said as removed"
        );
    }

    #[test]
    fn a_cgroup_unread_says_why() {
        let unread = |read| judged(Held::NoPidfd, read, |_| false);
        assert_eq!(
            unread(Err(io::ErrorKind::NotFound.into())),
            Cgroup::Gone,
            "a process reaped before its file was opened read as one unreadable"
        );
        assert_eq!(
            unread(Err(io::Error::from_raw_os_error(libc::ESRCH))),
            Cgroup::Gone,
            "a process reaped as its file was read read as one unreadable"
        );
        for errno in [libc::ENOENT, libc::ESRCH] {
            assert_eq!(
                judged(
                    Held::Still,
                    Err(io::Error::from_raw_os_error(errno)),
                    |_| true
                ),
                Cgroup::Unread(Some(errno)),
                "an asker its pidfd held there, its file unseen, was said gone"
            );
        }
        // Both with a pidfd, as production has, and without.
        for held in [Held::Still, Held::NoPidfd] {
            assert_eq!(
                judged(
                    held,
                    Err(io::Error::from_raw_os_error(libc::ENAMETOOLONG)),
                    |_| false
                ),
                Cgroup::Unread(Some(libc::ENAMETOOLONG)),
                "a path nested past PATH_MAX read as a process gone, held {held:?}"
            );
            assert_eq!(
                judged(held, Ok(b"1:name=systemd:/a\n".to_vec()), |_| false),
                Cgroup::Unnamed,
                "a host with no unified line read as a process gone, held {held:?}"
            );
        }
        assert_eq!(
            super::cgroup_of(b"0::/system.slice/x\xff.service (deleted)\n", |path| {
                path == b"system.slice/x\xff.service (deleted)"
            })
            .as_deref(),
            Some("system.slice/x?.service??deleted?"),
            "a live cgroup not UTF-8, named to end as a removed one's does, was said as removed"
        );
        // A hierarchy of its own, as a build's sandbox may have none.
        let root = tempfile::tempdir().expect("a hierarchy");
        let live = b"x.slice/y\xff.service (deleted)";
        std::fs::create_dir_all(root.path().join(OsStr::from_bytes(live))).expect("a cgroup");
        assert!(
            named(root.path(), live),
            "a live cgroup not UTF-8 was said as none"
        );
        assert!(
            !named(root.path(), b"x.slice/z.service (deleted)"),
            "a cgroup not there was said as there"
        );
        // A lookup that fails but not as not found, here ENOTDIR under a
        // file, as root or not, is taken as there.
        std::fs::write(root.path().join("f"), b"").expect("a file");
        assert!(
            named(root.path(), b"f/x (deleted)"),
            "a cgroup wifid couldn't look into was said as none"
        );
    }
}
