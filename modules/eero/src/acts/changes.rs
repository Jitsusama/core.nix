//! Every act asked of the home network, kept as JSON lines in eero's state
//! folder as `changes.jsonl`: when, the act and the MAC and address it
//! names, which process asked and the unit it ran in, and its result. An
//! act is logged as asked before the daemon does it and again once it
//! has, so one cut short still leaves a line; one let go because its asker
//! left first is logged as `gone`, and an ask refused as no act at all is
//! logged too, by its name cut short. An unreservation's last line names
//! the address eero kept for it, its undo, whatever it came to. It holds
//! no session and nothing eero answered, so it can be read anywhere the
//! account can. Nothing trims it: it grows by a line or two an act.
//!
//! It's a record kept by the daemon, not an audit: any process running as
//! the account can write to the file, and a process can ask through
//! another, so it says who asked as the kernel saw it, nothing more.

use std::fs::OpenOptions;
use std::io::{self, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};

use crate::acts::{self, Act, Came, Done};
use crate::cloud::Failure;
use crate::listeners::{Asked, Peer};
use crate::session;

/// Logs to `log` that `asked` is about to be done, at `at`.
pub fn asked(log: &Path, asked: &Asked, at: i64) -> io::Result<()> {
    append(
        log,
        &line(asked.peer.as_ref(), Some(&asked.act), "asked", at),
    )
}

/// Logs to `log` what `asked` came to, at `at`, with the address an
/// unreservation found kept, done or not.
pub fn ended(log: &Path, asked: &Asked, came: &Came, at: i64) -> io::Result<()> {
    let mut line = line(
        asked.peer.as_ref(),
        Some(&asked.act),
        result(&came.done),
        at,
    );
    if let Some(held) = came.held {
        line["address"] = json!(held.to_string());
    }
    append(log, &line)
}

/// Logs to `log` that `asked` was let go undone, at `at`, since who asked
/// it left before the daemon reached it.
pub fn gone(log: &Path, asked: &Asked, at: i64) -> io::Result<()> {
    append(
        log,
        &line(asked.peer.as_ref(), Some(&asked.act), "gone", at),
    )
}

/// Logs to `log` an ask the daemon refused as no act it does, naming the
/// act it said, cut short, since that's what an agent reaching past the
/// acts it may ask for would show.
pub fn refused(log: &Path, peer: Option<&Peer>, said: &Value, at: i64) -> io::Result<()> {
    let mut line = line(peer, None, "bad-act", at);
    line["act"] = json!(said["act"].as_str().map(acts::named));
    append(log, &line)
}

/// Now, in seconds since the epoch, as a line's `at` says it.
pub fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs().try_into().unwrap_or(i64::MAX))
}

/// The log's file.
pub fn file() -> io::Result<PathBuf> {
    Ok(session::folder()?.join("changes.jsonl"))
}

/// Adds `line` to the log at `path`, which is the account's alone.
fn append(path: &Path, line: &Value) -> io::Result<()> {
    let mut out = OpenOptions::new()
        .create(true)
        .append(true)
        .mode(0o600)
        .open(path)?;
    writeln!(out, "{line}")
}

/// One line: who asked, in which unit, the act and what it names, and
/// `result`.
fn line(peer: Option<&Peer>, act: Option<&Act>, result: &str, at: i64) -> Value {
    let mut line = json!({
        "at": at,
        "pid": peer.and_then(|peer| peer.pid),
        "uid": peer.map(|peer| peer.uid),
        "unit": peer.and_then(|peer| peer.unit.as_deref()),
    });
    match act {
        Some(Act::SpeedTest) => line["act"] = json!("speedtest"),
        Some(Act::Conferencing(on)) => {
            line["act"] = json!("sqm");
            line["on"] = json!(on);
        }
        Some(Act::Reserve { mac, address }) => {
            line["act"] = json!("reserve");
            line["mac"] = json!(mac.to_string());
            line["address"] = json!(address.to_string());
        }
        Some(Act::Unreserve { mac }) => {
            line["act"] = json!("unreserve");
            line["mac"] = json!(mac.to_string());
        }
        None => {}
    }
    line["result"] = json!(result);
    line
}

/// What an act came to, in one word: a refusal by what mends it alone,
/// since eero's refusal can carry the whole body it answered.
fn result(done: &Result<Done, Failure>) -> &'static str {
    match done {
        Ok(Done::Made) => "made",
        Ok(Done::Unchanged) => "unchanged",
        Err(Failure::SignedOut(_)) => "signed-out",
        Err(Failure::Unreachable(_)) => "unreachable",
        Err(Failure::Refused(_)) => "refused",
        Err(Failure::Unknown(_)) => "unknown",
    }
}

/// The unit or scope the process `pid` runs in, which names the session
/// that asked.
pub fn unit(pid: i32) -> Option<String> {
    unit_in(&std::fs::read_to_string(format!("/proc/{pid}/cgroup")).ok()?)
}

/// The unit a process's `cgroup` file names: the last part of its unified
/// hierarchy's path, or none at the root or with no unified line.
fn unit_in(cgroup: &str) -> Option<String> {
    cgroup
        .lines()
        .find_map(|line| line.strip_prefix("0::"))
        .and_then(|path| path.rsplit('/').next())
        .filter(|unit| !unit.is_empty())
        .map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use std::os::unix::fs::PermissionsExt;

    use super::*;
    use crate::acts::Mac;

    fn peer() -> Peer {
        Peer {
            pid: Some(4242),
            uid: 1000,
            unit: Some("pi-1.scope".into()),
        }
    }

    fn mac() -> Mac {
        Mac::parse("02:00:00:00:00:01").expect("a MAC")
    }

    #[test]
    fn a_reservation_is_logged_with_who_asked() {
        let act = Act::Reserve {
            mac: mac(),
            address: "10.0.0.31".parse().expect("an address"),
        };
        assert_eq!(
            line(Some(&peer()), Some(&act), "asked", 1_794_702_033),
            json!({
                "at": 1_794_702_033, "pid": 4242, "uid": 1000, "unit": "pi-1.scope",
                "act": "reserve", "mac": "02:00:00:00:00:01", "address": "10.0.0.31",
                "result": "asked",
            }),
            "the reservation's line misses what it did or who asked, or keeps more"
        );
    }

    #[test]
    fn a_line_with_no_peer_says_none_asked_it() {
        assert_eq!(
            line(None, Some(&Act::SpeedTest), "made", 1),
            json!({"at": 1, "pid": null, "uid": null, "unit": null,
                "act": "speedtest", "result": "made"}),
            "a line with no peer named someone"
        );
    }

    #[test]
    fn each_result_is_logged_by_its_kind_and_nothing_eero_said() {
        let said = r#"eero answered 403 to /2.2/networks/1001: {"meta":{"code":403}}"#;
        let results = [
            Ok(Done::Made),
            Ok(Done::Unchanged),
            Err(Failure::SignedOut("aged".into())),
            Err(Failure::Unreachable(said.into())),
            Err(Failure::Refused(said.into())),
            Err(Failure::Unknown(said.into())),
        ]
        .map(|done| result(&done));
        assert_eq!(
            results,
            [
                "made",
                "unchanged",
                "signed-out",
                "unreachable",
                "refused",
                "unknown"
            ],
            "a result was logged under another's name"
        );
    }

    #[test]
    fn the_unit_is_the_unified_hierarchys_last_part() {
        let scope = "0::/user.slice/user-1000.slice/user@1000.service/app.slice/pi-1.scope\n";
        let hybrid = "12:pids:/user.slice\n1:name=systemd:/user.slice\n0::/app.slice/a.service\n";
        assert_eq!(
            [
                unit_in(scope),
                unit_in(hybrid),
                unit_in("0::/\n"),
                unit_in("1:name=systemd:/x\n"),
            ],
            [
                Some("pi-1.scope".to_owned()),
                Some("a.service".to_owned()),
                None,
                None,
            ],
            "a process's unit was misread"
        );
    }

    #[test]
    fn a_refused_ask_is_logged_by_its_name_cut_short_and_nothing_else() {
        let folder = std::env::temp_dir().join(format!("eero-refused-{}", std::process::id()));
        std::fs::create_dir_all(&folder).expect("the test's folder is made");
        let log = folder.join("changes.jsonl");
        let _ = std::fs::remove_file(&log);
        let said = json!({"ask": "act", "act": "forward".repeat(10), "port": 22});
        refused(&log, Some(&peer()), &said, 7).expect("the line is written");
        let logged: Value = serde_json::from_str(&std::fs::read_to_string(&log).expect("the log"))
            .expect("the line is JSON");
        assert_eq!(
            logged,
            json!({"at": 7, "pid": 4242, "uid": 1000, "unit": "pi-1.scope",
                "act": &"forward".repeat(10)[..32], "result": "bad-act"}),
            "a refused ask was logged whole, or without who asked"
        );
        let _ = std::fs::remove_dir_all(&folder);
    }

    /// The lines a test's log holds, each read as JSON.
    fn logged(log: &Path) -> Vec<Value> {
        std::fs::read_to_string(log)
            .expect("the log")
            .lines()
            .map(|line| serde_json::from_str(line).expect("a line is JSON"))
            .collect()
    }

    #[test]
    fn an_unreservations_last_line_names_the_address_it_kept_whatever_it_came_to() {
        let folder = std::env::temp_dir().join(format!("eero-ended-{}", std::process::id()));
        std::fs::create_dir_all(&folder).expect("the test's folder is made");
        let log = folder.join("changes.jsonl");
        let _ = std::fs::remove_file(&log);
        let unreserve = Asked {
            listener: 1,
            id: json!(1),
            act: Act::Unreserve { mac: mac() },
            peer: Some(peer()),
        };
        let held = Some("10.0.0.208".parse().expect("an address"));
        asked(&log, &unreserve, 1).expect("the ask is logged");
        let made = Came {
            done: Ok(Done::Made),
            held,
        };
        ended(&log, &unreserve, &made, 2).expect("the end is logged");
        let unknown = Came {
            done: Err(Failure::Unknown("eero didn't say".into())),
            held,
        };
        ended(&log, &unreserve, &unknown, 3).expect("the end is logged");
        gone(&log, &unreserve, 4).expect("the let-go is logged");
        let who = json!({"pid": 4242, "uid": 1000, "unit": "pi-1.scope",
            "act": "unreserve", "mac": "02:00:00:00:00:01"});
        let with = |extra: Value| {
            let mut line = who.clone();
            for (key, value) in extra.as_object().expect("the test's fields") {
                line[key] = value.clone();
            }
            line
        };
        assert_eq!(
            logged(&log),
            [
                with(json!({"at": 1, "result": "asked"})),
                with(json!({"at": 2, "result": "made", "address": "10.0.0.208"})),
                with(json!({"at": 3, "result": "unknown", "address": "10.0.0.208"})),
                with(json!({"at": 4, "result": "gone"})),
            ],
            "an unreservation's lines lost the address that undoes it, or who asked"
        );
        let _ = std::fs::remove_dir_all(&folder);
    }

    #[test]
    fn conferencing_is_logged_by_whether_it_was_asked_on() {
        assert_eq!(
            line(None, Some(&Act::Conferencing(false)), "made", 1),
            json!({"at": 1, "pid": null, "uid": null, "unit": null,
                "act": "sqm", "on": false, "result": "made"}),
            "conferencing's line misses whether it was turned on or off"
        );
    }

    #[test]
    fn the_log_is_the_accounts_alone_and_only_grows() {
        let folder = std::env::temp_dir().join(format!("eero-changes-{}", std::process::id()));
        std::fs::create_dir_all(&folder).expect("the test's folder is made");
        let file = folder.join("changes.jsonl");
        let _ = std::fs::remove_file(&file);
        for at in [1, 2] {
            append(&file, &json!({"at": at})).expect("the line is written");
        }
        let mode = std::fs::metadata(&file)
            .expect("the log")
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600, "the log is open to other accounts");
        assert_eq!(
            std::fs::read_to_string(&file).expect("the log reads"),
            "{\"at\":1}\n{\"at\":2}\n",
            "a line overwrote another"
        );
        let _ = std::fs::remove_dir_all(&folder);
    }
}
