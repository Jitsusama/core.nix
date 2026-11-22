//! Every act done to the home network, kept as a JSON line each in eero's
//! state folder as `changes.jsonl`: when, what, which process asked and
//! the unit it ran in, and whether eero took it. It holds no session, so
//! it can be read anywhere the account can, and it is how a change made
//! by anyone is found again and undone.

use std::fs::OpenOptions;
use std::io::{self, Write};
use std::os::unix::fs::OpenOptionsExt;

use serde_json::{Value, json};

use crate::acts::Act;
use crate::listeners::Asked;
use crate::session;

/// Appends what `asked` did, at `at`, to the log.
pub fn log(asked: &Asked, done: &Result<(), String>, at: i64) -> io::Result<()> {
    let unit = asked.peer.and_then(|peer| unit(peer.pid));
    let mut out = OpenOptions::new()
        .create(true)
        .append(true)
        .mode(0o600)
        .open(session::folder()?.join("changes.jsonl"))?;
    writeln!(out, "{}", line(asked, unit.as_deref(), done, at))
}

/// One act's line: what it changed, who asked, in which unit, and how it
/// went.
pub fn line(asked: &Asked, unit: Option<&str>, done: &Result<(), String>, at: i64) -> Value {
    let mut line = json!({
        "at": at,
        "pid": asked.peer.map(|peer| peer.pid),
        "uid": asked.peer.map(|peer| peer.uid),
        "unit": unit,
    });
    match asked.act {
        Act::SpeedTest => line["act"] = json!("speedtest"),
        Act::Conferencing(on) => {
            line["act"] = json!("sqm");
            line["on"] = json!(on);
        }
        Act::Reserve { mac, address } => {
            line["act"] = json!("reserve");
            line["mac"] = json!(mac.to_string());
            line["address"] = json!(address.to_string());
            line["undo"] = json!(format!("eero devices {mac} unreserve"));
        }
        Act::Unreserve { mac } => {
            line["act"] = json!("unreserve");
            line["mac"] = json!(mac.to_string());
        }
    }
    match done {
        Ok(()) => line["done"] = json!(true),
        Err(why) => line["refused"] = json!(why),
    }
    line
}

/// The unit or scope the process `pid` runs in, the last part of its
/// cgroup, which names the session that asked.
fn unit(pid: i32) -> Option<String> {
    let cgroup = std::fs::read_to_string(format!("/proc/{pid}/cgroup")).ok()?;
    cgroup
        .lines()
        .find_map(|line| line.strip_prefix("0::"))
        .and_then(|path| path.rsplit('/').next())
        .filter(|unit| !unit.is_empty())
        .map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::acts::Mac;
    use crate::listeners::Peer;

    fn asked(act: Act) -> Asked {
        Asked {
            listener: 1,
            id: json!(1),
            act,
            peer: Some(Peer {
                pid: 4242,
                uid: 1000,
            }),
        }
    }

    #[test]
    fn a_reservation_is_logged_with_who_asked_and_its_undo() {
        let mac = Mac::parse("02:00:00:00:00:01").expect("a MAC");
        let act = Act::Reserve {
            mac,
            address: "10.0.0.31".parse().expect("an address"),
        };
        assert_eq!(
            line(&asked(act), Some("pi-1.scope"), &Ok(()), 1_794_702_033),
            json!({
                "at": 1_794_702_033, "pid": 4242, "uid": 1000, "unit": "pi-1.scope",
                "act": "reserve", "mac": "02:00:00:00:00:01", "address": "10.0.0.31",
                "undo": "eero devices 02:00:00:00:00:01 unreserve", "done": true,
            }),
            "the reservation's line misses what it did, who asked or its undo"
        );
    }

    #[test]
    fn a_refused_act_is_logged_with_why() {
        let mac = Mac::parse("02:00:00:00:00:01").expect("a MAC");
        let said = line(
            &asked(Act::Unreserve { mac }),
            None,
            &Err("eero answered 403".into()),
            1,
        );
        assert_eq!(
            (
                said["act"].as_str(),
                said["refused"].as_str(),
                said.get("done")
            ),
            (Some("unreserve"), Some("eero answered 403"), None),
            "a refusal was logged as done, or without why"
        );
    }
}
