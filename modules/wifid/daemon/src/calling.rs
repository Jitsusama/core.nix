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
    pub pid: Option<i32>,
    /// The cgroup's whole path, as `cgroup_of` says it.
    pub cgroup: Option<String>,
}

impl Peer {
    /// The peer at a pid, with the cgroup it's in. A path the kernel
    /// gives in bytes that aren't UTF-8 is read all the same, those bytes
    /// said as `?`, so the line never reads as a process in no cgroup.
    pub fn at(pid: Option<i32>) -> Peer {
        let cgroup = pid
            .and_then(|pid| std::fs::read(format!("/proc/{pid}/cgroup")).ok())
            .and_then(|cgroup| cgroup_of(&String::from_utf8_lossy(&cgroup)));
        Peer { pid, cgroup }
    }
}

impl fmt::Display for Peer {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (self.pid, &self.cgroup) {
            // The whole path, as a process names its own unit under its
            // user's manager: a unit called wifid-calls.service there
            // still reads as under user.slice, not system.slice.
            (Some(pid), Some(cgroup)) => write!(out, "pid {pid} ({cgroup})"),
            (Some(pid), None) => write!(out, "pid {pid}"),
            (None, _) => write!(out, "a process the kernel didn't name"),
        }
    }
}

/// The path of the cgroup a process runs in, from the root, each byte no
/// unit's name may hold said as `?`. A process names its own cgroup under
/// its user's manager, so the name could otherwise hold a control byte,
/// which has journalctl show the whole line as a blob, or a `) asks a
/// look; pid 2211 (` writing a second asker into the line.
fn cgroup_of(cgroup: &str) -> Option<String> {
    let path = cgroup.lines().find_map(|line| line.strip_prefix("0::/"))?;
    let allowed = |byte: char| byte.is_ascii_alphanumeric() || ":-_.\\@/".contains(byte);
    let said: String = path
        .chars()
        .map(|byte| if allowed(byte) { byte } else { '?' })
        .collect();
    Some(said).filter(|path| !path.is_empty())
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
            cgroup_of(&format!("0::/{path}\n")).as_deref(),
            Some(path),
            "the path"
        );
        assert_eq!(cgroup_of("0::/\n"), None, "the root names none");
        assert_eq!(cgroup_of(""), None, "nothing read names none");
        let lossy = String::from_utf8_lossy(b"0::/system.slice/x\xff.service\n");
        assert_eq!(
            cgroup_of(&lossy).as_deref(),
            Some("system.slice/x?.service"),
            "a path not in UTF-8 named no cgroup"
        );
    }

    #[test]
    fn a_cgroup_says_no_byte_a_units_name_cant_hold() {
        let user = "user.slice/user-1000.slice/user@1000.service/app.slice";
        let escaping = format!("0::/{user}/\u{1b}[2Jx.service\n");
        let forging = format!("0::/{user}/init.scope) asks a look; pid 2211 (quickshell.service\n");

        assert_eq!(
            cgroup_of(&escaping),
            Some(format!("{user}/??2Jx.service")),
            "a unit's control byte reached the journal raw"
        );
        assert_eq!(
            cgroup_of(&forging),
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
            cgroup: Some("system.slice/wifid-calls.service".into()),
        };
        assert_eq!(
            named.to_string(),
            "pid 42 (system.slice/wifid-calls.service)"
        );
        assert_eq!(
            Peer {
                cgroup: None,
                ..named
            }
            .to_string(),
            "pid 42",
            "a process in no cgroup read is named by its pid"
        );
        assert_eq!(
            Peer::default().to_string(),
            "a process the kernel didn't name"
        );
    }
}
