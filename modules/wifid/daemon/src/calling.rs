//! Whether Joel's on a call, as his session tells wifid on the launcher's
//! rows socket, so wifid keeps the card on its access point for the call
//! rather than move it to a quicker one and stall the call while it does.
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
//! them, so each change is logged with the process and cgroup that said it,
//! and anything but exactly that shape is refused, logged by what was
//! wrong with it and never by what it said (2026-11-14).

use std::fmt;
use std::io;
use std::mem;
use std::os::fd::{AsRawFd, BorrowedFd, FromRawFd, OwnedFd};
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
    /// The cgroup's path from the root, as `cgroup_of` says it.
    Path(String),
    /// The process had gone by the time its path was read, so the pid it
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
    /// kernel's pidfd for the peer is taken first and asked after the
    /// read: a process still holding its pid then is the one that
    /// connected, as a live process keeps its pid, and one that has gone
    /// is said as gone rather than named by whatever took its number. A
    /// pid of 0 is the kernel saying the peer is outside wifid's pid
    /// namespace, which it can't name, so it's said as none.
    pub fn on(socket: BorrowedFd<'_>, pid: Option<i32>, uid: Option<u32>) -> Peer {
        let pid = pid.filter(|&pid| pid > 0);
        let Some(number) = pid else {
            let cgroup = Cgroup::Unasked;
            return Peer { pid, uid, cgroup };
        };
        let held = pidfd_of(socket);
        let read = std::fs::read(format!("/proc/{number}/cgroup"));
        let cgroup = match held {
            Ok(held) if !holds_its_pid(&held) => Cgroup::Gone,
            Ok(_) => judged(read, |path| Path::new("/sys/fs/cgroup").join(path).exists()),
            Err(error) if error.raw_os_error() == Some(libc::ESRCH) => Cgroup::Gone,
            // Without its pidfd, nothing says the path read is the asker's.
            Err(error) => Cgroup::Unread(error.raw_os_error()),
        };
        Peer { pid, uid, cgroup }
    }
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

/// Whether the process a pidfd names still holds its pid: running, or
/// exited and not yet reaped. Signal 0 sends nothing, and a process wifid
/// may not signal, having no capability to, is there all the same.
fn holds_its_pid(held: &OwnedFd) -> bool {
    // SAFETY: it takes a descriptor held for the call, signal 0, no
    // signal's details and no flags.
    let sent = unsafe {
        libc::syscall(
            libc::SYS_pidfd_send_signal,
            held.as_raw_fd(),
            0,
            std::ptr::null::<libc::siginfo_t>(),
            0,
        )
    };
    sent == 0 || io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
}

/// Where a process runs, from what reading its cgroup file gave, `named`
/// saying whether a cgroup of a path exists.
fn judged(read: io::Result<Vec<u8>>, named: impl Fn(&str) -> bool) -> Cgroup {
    match read {
        Ok(cgroup) => cgroup_of(&cgroup, named).map_or(Cgroup::Unnamed, Cgroup::Path),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Cgroup::Gone,
        Err(error) => Cgroup::Unread(error.raw_os_error()),
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
            Cgroup::Gone => write!(out, " (gone before its cgroup was read)"),
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
/// line, and reads as unread.
///
/// The kernel ends the path with ` (deleted)` once a process's cgroup is
/// removed, as for a zombie whose unit has stopped, so that's said as the
/// cgroup it was, since removed. A live cgroup may be named to end so too,
/// so a path `named`, one whose cgroup exists by that whole name, keeps
/// it as the name's, escaped as the rest. A name removed between the read
/// and that look is said as removed, which it then is.
fn cgroup_of(cgroup: &[u8], named: impl Fn(&str) -> bool) -> Option<String> {
    let cgroup = String::from_utf8_lossy(cgroup);
    let path = cgroup
        .split('\n')
        .find_map(|line| line.strip_prefix("0::/"))?;
    let (path, removed) = match path.strip_suffix(" (deleted)") {
        Some(stripped) if !named(path) => (stripped, ", since removed"),
        _ => (path, ""),
    };
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
            (Cgroup::Gone, "gone before its cgroup was read"),
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
        use std::os::fd::AsFd;

        // A pair's other end is this test's own process, alive.
        let (socket, _other) = std::os::unix::net::UnixStream::pair().expect("a pair");
        let own = i32::try_from(std::process::id()).expect("a pid fits");
        let here = Peer::on(socket.as_fd(), Some(own), Some(1000));
        // No pid reaches past pid_max, 2^22 at most, so none is there.
        let gone = Peer::on(socket.as_fd(), Some(i32::MAX), Some(1000));

        assert!(
            matches!(here.cgroup, Cgroup::Path(_)),
            "this test's own cgroup went unread: {here}"
        );
        assert_eq!(
            gone.to_string(),
            format!(
                "pid {}, uid 1000 (gone before its cgroup was read)",
                i32::MAX
            ),
            "a pid that isn't there named a cgroup"
        );
        assert_eq!(
            Peer::on(socket.as_fd(), Some(0), Some(1000)).to_string(),
            "a process of uid 1000 the kernel gave no pid",
            "a peer outside wifid's pid namespace was read as pid 0, the kernel's own"
        );
    }

    #[test]
    fn a_process_holds_its_pid_until_it_is_reaped() {
        let mut child = std::process::Command::new("sleep")
            .arg("60")
            .spawn()
            .expect("sleep starts");
        let pid = libc::pid_t::try_from(child.id()).expect("a pid fits");
        // SAFETY: it takes only numbers, and the descriptor it gives is
        // taken straight into an OwnedFd.
        let fd = unsafe { libc::syscall(libc::SYS_pidfd_open, pid, 0) };
        let fd = i32::try_from(fd).expect("a descriptor fits");
        assert!(fd >= 0, "no pidfd: {}", io::Error::last_os_error());
        // SAFETY: the kernel made it for this call, and nothing else holds
        // it.
        let held = unsafe { OwnedFd::from_raw_fd(fd) };

        assert!(holds_its_pid(&held), "a running process let its pid go");
        child.kill().expect("sleep is killed");
        assert!(
            holds_its_pid(&held),
            "a process killed but not yet reaped let its pid go"
        );
        child.wait().expect("sleep is reaped");
        assert!(
            !holds_its_pid(&held),
            "a reaped process still held its pid, so another's path could be said as its"
        );
    }

    #[test]
    fn a_cgroup_unread_says_why() {
        assert_eq!(
            judged(Err(io::ErrorKind::NotFound.into()), |_| false),
            Cgroup::Gone,
            "a process gone read as one unreadable"
        );
        assert_eq!(
            judged(
                Err(io::Error::from_raw_os_error(libc::ENAMETOOLONG)),
                |_| false
            ),
            Cgroup::Unread(Some(libc::ENAMETOOLONG)),
            "a path nested past PATH_MAX read as a process gone"
        );
        assert_eq!(
            judged(Ok(b"1:name=systemd:/a\n".to_vec()), |_| false),
            Cgroup::Unnamed,
            "a host with no unified line read as a process gone"
        );
        assert_eq!(
            super::cgroup_of(b"0::/system.slice/x.service (deleted)\n", |path| {
                path == "system.slice/x.service (deleted)"
            })
            .as_deref(),
            Some("system.slice/x.service??deleted?"),
            "a live cgroup named to end as a removed one's does was said as removed"
        );
    }
}
