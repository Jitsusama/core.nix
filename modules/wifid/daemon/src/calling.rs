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
    /// The account the kernel says connected, which no cgroup nested too
    /// deep to read can hide.
    uid: Option<u32>,
    /// The cgroup's path, as `cgroup_of` says it, or none when it couldn't
    /// be read. Only `cgroup_of` makes one, so nothing unescaped reaches
    /// the journal through it.
    cgroup: Option<String>,
}

impl Peer {
    /// The peer at a pid, with the cgroup it's in, read as bytes so a path
    /// that isn't UTF-8 is still said. The pid is read as wifid answers,
    /// so a process that connected and exited before then could have its
    /// pid taken by another, and the path name that one's cgroup: the
    /// slice holds only while the pid isn't reused. The uid can't move
    /// that way, since the kernel took it as the connection was made.
    pub fn at(pid: Option<i32>, uid: Option<u32>) -> Peer {
        let cgroup = pid
            .and_then(|pid| std::fs::read(format!("/proc/{pid}/cgroup")).ok())
            .and_then(|cgroup| cgroup_of(&cgroup));
        Peer { pid, uid, cgroup }
    }
}

impl fmt::Display for Peer {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Some(pid) = self.pid else {
            return write!(out, "a process the kernel didn't name");
        };
        write!(out, "pid {pid}")?;
        if let Some(uid) = self.uid {
            write!(out, ", uid {uid}")?;
        }
        match &self.cgroup {
            // The path from the root, as a process names its own unit
            // under its user's manager: a unit called wifid-calls.service
            // there still reads as under user.slice, not system.slice.
            Some(cgroup) => write!(out, " ({cgroup})"),
            // A process may have gone, or nest its cgroup too deep for
            // the kernel to say, so a line it asks for says the path went
            // unread rather than passing as one that has none, and its
            // uid still says whose it is.
            None => write!(out, " (its cgroup unread)"),
        }
    }
}

/// The path of the cgroup a process runs in, from the root, each character
/// no unit's name may hold said as `?`. A process names its own cgroup
/// under its user's manager, so the name could otherwise hold a control
/// character, which has journalctl show the whole line as a blob, or a
/// `) asks a look; pid 2211 (` writing a second asker into the line. Bytes
/// that aren't UTF-8 are said as `?` too. No name holds a line break, since
/// the kernel's `cgroup_mkdir` refuses one to keep this file readable. A
/// host with no unified hierarchy has no `0::` line, and reads as unread.
fn cgroup_of(cgroup: &[u8]) -> Option<String> {
    let cgroup = String::from_utf8_lossy(cgroup);
    let path = cgroup.lines().find_map(|line| line.strip_prefix("0::/"))?;
    let allowed = |said: char| said.is_ascii_alphanumeric() || ":-_.\\@/".contains(said);
    let said: String = path
        .chars()
        .map(|said| if allowed(said) { said } else { '?' })
        .collect();
    // The root's path is empty, said as itself so it doesn't read as one
    // that went unread.
    Some(if said.is_empty() {
        "/".into()
    } else {
        cut(&said)
    })
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
        assert_eq!(cgroup_of(b""), None, "nothing read names none");
        assert_eq!(
            cgroup_of(b"0::/system.slice/x\xff\xc3\xa9.service\n").as_deref(),
            Some("system.slice/x??.service"),
            "a path not in UTF-8 named no cgroup, or said a character as more than one ?"
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
    }

    #[test]
    fn a_peer_says_what_the_kernel_named() {
        let named = Peer {
            pid: Some(42),
            uid: Some(1000),
            cgroup: Some("system.slice/wifid-calls.service".into()),
        };
        assert_eq!(
            named.to_string(),
            "pid 42, uid 1000 (system.slice/wifid-calls.service)"
        );
        assert_eq!(
            Peer {
                cgroup: None,
                ..named.clone()
            }
            .to_string(),
            "pid 42, uid 1000 (its cgroup unread)",
            "a process whose cgroup went unread passed as one in none"
        );
        assert_eq!(
            Peer { uid: None, ..named }.to_string(),
            "pid 42 (system.slice/wifid-calls.service)",
            "a peer with no uid said one"
        );
        assert_eq!(
            Peer::default().to_string(),
            "a process the kernel didn't name"
        );
    }

    #[test]
    fn a_peer_is_read_from_the_kernel_and_says_when_it_couldnt_be() {
        // No pid reaches past pid_max, 2^22 at most, so none is there.
        let gone = Peer::at(Some(i32::MAX), Some(1000));
        let here = Peer::at(
            Some(i32::try_from(std::process::id()).expect("a pid fits")),
            Some(1000),
        );

        assert_eq!(
            gone.to_string(),
            format!("pid {}, uid 1000 (its cgroup unread)", i32::MAX),
            "a pid that isn't there named a cgroup"
        );
        assert!(
            here.cgroup.is_some(),
            "this test's own cgroup went unread: {here}"
        );
    }
}
