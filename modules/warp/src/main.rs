//! warp: how WARP's link stands, read from warp-cli, WARP's own client,
//! every figure it gives in its own unit.
//!
//! Usage:
//!   warp             the page: the link, the tunnel, TLS, the settings, DNS
//!   warp --json      the same as one record (model.rs; warp.json beside this)
//!   warp connect     bring WARP up, as the launcher's :connect does
//!   warp disconnect  take it down, as :disconnect does (switch.rs says when
//!                    each is left as it is), with --json as a record
//!   warp serve SOCKET  hold the link for the shell, which reads it and
//!                    switches it over SOCKET (serve.rs)
//!
//! `WARP_CLI` names the warp-cli to ask, as the shell names the one the
//! system runs; without it, it's the one on the PATH.
//!
//! Run by hand, it asks warp-cli each time and keeps nothing: WARP's
//! service keeps every figure itself, answering each question in about
//! 20 ms (2026-11-07, on a laptop). Serving the shell, it asks only while
//! the link is watched, every two seconds, so the launcher starts no
//! process to read WARP or to switch it.
//!
//! It exits 0 having read WARP or switched it, 2 on a word it doesn't
//! know, 3 when there's no warp-cli, so no WARP to read, 4 when WARP's
//! service isn't answering, 5 when warp-cli said what it can't read, 6
//! when the organisation holds the switch and 7 when warp-cli wouldn't
//! switch it, each said in a line, never a trace; with --json, as
//! { error, said } or the switch's record too. Serving, it runs until
//! stopped, and ends 1 only when it can't listen on its socket.

mod ask;
#[cfg(test)]
mod contract;
mod model;
mod page;
mod read;
mod serve;
mod switch;

use std::io::Write as _;
use std::process::ExitCode;

use ask::{Ask, WarpCli};
use read::Failure;

const USAGE: &str = "usage: warp [connect | disconnect] [--json] | warp serve SOCKET";

/// What a run prints, and how it ends.
#[derive(Debug, PartialEq)]
struct Ran {
    code: u8,
    out: String,
    err: String,
}

fn run(args: &[String], cli: &impl Ask) -> Ran {
    let json = args.iter().any(|arg| arg == "--json");
    let words: Vec<&str> = args
        .iter()
        .map(String::as_str)
        .filter(|arg| *arg != "--json")
        .collect();
    let act = match words.as_slice() {
        [] => None,
        ["connect"] => Some(switch::Act::Connect),
        ["disconnect"] => Some(switch::Act::Disconnect),
        [word] => {
            return Ran {
                code: 2,
                out: String::new(),
                err: format!("warp: {word} isn't a word warp knows\n{USAGE}\n"),
            };
        }
        _ => {
            return Ran {
                code: 2,
                out: String::new(),
                err: format!("warp: one word at a time\n{USAGE}\n"),
            };
        }
    };
    if let Some(act) = act {
        return match switch::switch(cli, act) {
            Ok(switched) => switched_ran(&switched, json),
            Err(failure) => failed(&failure, json),
        };
    }
    match read::read(cli) {
        Ok(warp) if json => Ran {
            code: 0,
            out: format!(
                "{}\n",
                serde_json::to_string(&warp).expect("the record is JSON")
            ),
            err: String::new(),
        },
        Ok(warp) => Ran {
            code: 0,
            out: page::page(&warp),
            err: String::new(),
        },
        Err(failure) => failed(&failure, json),
    }
}

/// A failure, said in a line, and with --json as a record too, ending with
/// its code.
fn failed(failure: &Failure, json: bool) -> Ran {
    let (code, kind, said, line) = match failure {
        Failure::NotInstalled(at) => (
            3,
            "not-installed",
            at,
            format!("{at} isn't installed, so there's no WARP here to read"),
        ),
        Failure::NotAnswering(why) => (
            4,
            "not-answering",
            why,
            format!("WARP's service isn't answering: {why}"),
        ),
        Failure::Unreadable(why) => (5, "unreadable", why, why.clone()),
    };
    Ran {
        code,
        out: if json {
            format!("{}\n", serde_json::json!({ "error": kind, "said": said }))
        } else {
            String::new()
        },
        err: format!("warp: {line}\n"),
    }
}

/// A switch, said in a line, on the output where it was done and as an
/// error where it wasn't, and with --json as its record, ending with its
/// code.
fn switched_ran(switched: &switch::Switched, json: bool) -> Ran {
    let code = match switched.refused {
        None => 0,
        Some("held") => 6,
        Some(_) => 7,
    };
    let line = format!("{}\n", switched.said);
    Ran {
        code,
        out: if json {
            format!(
                "{}\n",
                serde_json::to_string(switched).expect("the record is JSON")
            )
        } else if code == 0 {
            line.clone()
        } else {
            String::new()
        },
        err: if code == 0 {
            String::new()
        } else {
            format!("warp: {line}")
        },
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let [serving, socket] = args.as_slice()
        && serving == "serve"
    {
        return match serve::serve(std::path::Path::new(socket), WarpCli::from_env()) {
            Ok(()) => ExitCode::SUCCESS,
            Err(why) => {
                eprintln!("warp: {socket} couldn't be listened on: {why}");
                ExitCode::from(1)
            }
        };
    }
    let ran = run(&args, &WarpCli::from_env());
    // A closed pipe, as `warp | head` gives, changes nothing about the
    // reading, so it isn't said.
    let _ = std::io::stdout().write_all(ran.out.as_bytes());
    let _ = std::io::stderr().write_all(ran.err.as_bytes());
    ExitCode::from(ran.code)
}

#[cfg(test)]
mod tests {
    use super::*;

    use ask::Unasked;
    use contract::{Contract, said};
    use serde_json::Value;

    fn args(words: &[&str]) -> Vec<String> {
        words.iter().map(|word| (*word).to_string()).collect()
    }

    fn down() -> Contract {
        Contract::none().with("status", said(false, "daemon-down.json"))
    }

    fn missing() -> Contract {
        Contract::none().with("status", Err(Unasked::Missing("warp-cli".into())))
    }

    fn refusing() -> Contract {
        Contract::none().with("status", said(false, "refused.json"))
    }

    #[test]
    fn the_page_is_printed_by_default_and_the_record_with_json() {
        let page = run(&args(&[]), &Contract::whole());
        assert_eq!(page.code, 0);
        assert!(
            page.out.starts_with("WARP \u{b7} connected"),
            "{}",
            page.out
        );
        assert_eq!(page.err, "");
        let record = run(&args(&["--json"]), &Contract::whole());
        assert_eq!(record.code, 0);
        let printed: Value = serde_json::from_str(&record.out).expect("it prints JSON");
        let golden: Value = serde_json::from_str(
            &std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/warp.json"))
                .expect("warp.json is there"),
        )
        .expect("warp.json is JSON");
        assert_eq!(printed, golden);
    }

    #[test]
    fn each_failure_is_said_in_a_line_and_ends_with_its_own_code() {
        let none = run(&args(&[]), &missing());
        assert_eq!(
            (none.code, none.out.as_str(), none.err.as_str()),
            (
                3,
                "",
                "warp: warp-cli isn't installed, so there's no WARP here to read\n"
            )
        );
        let quiet = run(&args(&[]), &down());
        assert_eq!(
            (quiet.code, quiet.out.as_str(), quiet.err.as_str()),
            (
                4,
                "",
                "warp: WARP's service isn't answering: No such file or directory (os error 2)\n"
            )
        );
        let odd = run(&args(&[]), &refusing());
        assert_eq!(
            (odd.code, odd.out.as_str(), odd.err.as_str()),
            (
                5,
                "",
                "warp: warp-cli status said InvalidAccountType: Invalid account type.\n"
            )
        );
    }

    #[test]
    fn a_failure_with_json_is_a_record_saying_which_and_why() {
        let quiet = run(&args(&["--json"]), &down());
        assert_eq!(quiet.code, 4);
        let said: Value = serde_json::from_str(&quiet.out).expect("it prints JSON");
        assert_eq!(
            said,
            serde_json::json!({
                "error": "not-answering",
                "said": "No such file or directory (os error 2)"
            })
        );
        let none = run(&args(&["--json"]), &missing());
        assert_eq!(none.code, 3);
        let said: Value = serde_json::from_str(&none.out).expect("it prints JSON");
        assert_eq!(said["error"], "not-installed");
    }

    #[test]
    fn a_word_it_doesnt_know_is_refused_with_its_usage() {
        let ran = run(&args(&["reconnect"]), &Contract::whole());
        assert_eq!(
            (ran.code, ran.out.as_str(), ran.err.as_str()),
            (
                2,
                "",
                "warp: reconnect isn't a word warp knows\n\
                 usage: warp [connect | disconnect] [--json]\n"
            )
        );
        let two = run(&args(&["connect", "disconnect"]), &Contract::whole());
        assert_eq!(two.code, 2);
    }

    #[expect(
        clippy::unnecessary_wraps,
        reason = "it's an answer as `Contract::with` takes one"
    )]
    fn done() -> Result<ask::Said, Unasked> {
        Ok(ask::Said {
            ok: true,
            text: "Success\n".into(),
        })
    }

    #[expect(
        clippy::unnecessary_wraps,
        reason = "it's an answer as `Contract::with` takes one"
    )]
    fn disconnected() -> Result<ask::Said, Unasked> {
        Ok(ask::Said {
            ok: true,
            text: r#"{"status": "Disconnected", "reason": "ManualDisconnection"}"#.into(),
        })
    }

    #[expect(
        clippy::unnecessary_wraps,
        reason = "it's an answer as `Contract::with` takes one"
    )]
    fn held() -> Result<ask::Said, Unasked> {
        let mut said: Value =
            serde_json::from_str(&contract::file("settings.json")).expect("it's JSON");
        said["settings"]["switch_locked"] = Value::Bool(true);
        Ok(ask::Said {
            ok: true,
            text: said.to_string(),
        })
    }

    #[test]
    fn disconnect_takes_warp_down_and_says_when_it_comes_back() {
        let cli = Contract::whole().with("disconnect", done());
        let ran = run(&args(&["disconnect"]), &cli);
        assert_eq!(
            (ran.code, ran.out.as_str(), ran.err.as_str()),
            (
                0,
                "WARP is disconnected, and comes back by itself in 2 min\n",
                ""
            )
        );
        let record = run(&args(&["disconnect", "--json"]), &cli);
        assert_eq!(record.code, 0);
        let said: Value = serde_json::from_str(&record.out).expect("it prints JSON");
        assert_eq!(
            (
                &said["act"],
                &said["done"],
                &said["refused"],
                &said["comes_back_after_min"]
            ),
            (
                &Value::from("disconnect"),
                &Value::Bool(true),
                &Value::Null,
                &Value::from(2)
            )
        );
    }

    #[test]
    fn connect_brings_warp_up() {
        let cli = Contract::whole()
            .with("status", disconnected())
            .with("connect", done());
        let ran = run(&args(&["connect"]), &cli);
        assert_eq!(
            (ran.code, ran.out.as_str(), ran.err.as_str()),
            (0, "WARP is connecting\n", "")
        );
    }

    #[test]
    fn a_switch_the_organisation_holds_ends_with_its_own_code() {
        let cli = Contract::whole()
            .with("settings", held())
            .with("disconnect", done());
        let ran = run(&args(&["disconnect"]), &cli);
        assert_eq!(
            (ran.code, ran.out.as_str(), ran.err.as_str()),
            (
                6,
                "",
                "warp: example holds WARP's switch, so it's left connected\n"
            )
        );
        let record = run(&args(&["disconnect", "--json"]), &cli);
        assert_eq!(record.code, 6);
        let said: Value = serde_json::from_str(&record.out).expect("it prints JSON");
        assert_eq!(
            (&said["refused"], &said["organisation"]),
            (&Value::from("held"), &Value::from("example"))
        );
    }

    #[test]
    fn a_switch_warp_cli_refuses_ends_with_its_own_code() {
        // The contract refuses a question it isn't given.
        let cli = Contract::whole().with("status", disconnected());
        let ran = run(&args(&["connect"]), &cli);
        assert_eq!(
            (ran.code, ran.out.as_str(), ran.err.as_str()),
            (
                7,
                "",
                "warp: warp-cli wouldn't connect: Invalid account type.\n"
            )
        );
    }

    #[test]
    fn a_switch_with_no_warp_to_ask_fails_as_a_reading_does() {
        let ran = run(&args(&["disconnect"]), &missing());
        assert_eq!(ran.code, 3);
    }
}
