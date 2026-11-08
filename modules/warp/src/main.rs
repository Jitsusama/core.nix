//! warp: how WARP's link stands, read from warp-cli, WARP's own client,
//! every figure it gives in its own unit.
//!
//! Usage:
//!   warp           the page: the link, the tunnel, TLS, the settings, DNS
//!   warp --json    the same as one record (model.rs; warp.json beside this)
//!
//! `WARP_CLI` names the warp-cli to ask, as the shell names the one the
//! system runs; without it, it's the one on the PATH.
//!
//! It asks warp-cli each time it's run and keeps nothing, rather than
//! running as a daemon: WARP's service is the daemon, keeping every
//! figure itself and answering each question in about 20 ms
//! (2026-11-07, on a laptop), so a copy kept here would only ever be
//! older. The shell asks it every couple of seconds while WARP is shown,
//! and nothing otherwise.
//!
//! It exits 0 having read WARP, 2 on a word it doesn't know, 3 when
//! there's no warp-cli, so no WARP to read, 4 when WARP's service isn't
//! answering, and 5 when warp-cli said what it can't read, each said in a
//! line, never a trace; with --json, as { error, said } too.

mod ask;
#[cfg(test)]
mod contract;
mod model;
mod page;
mod read;

use std::io::Write as _;
use std::process::ExitCode;

use ask::{Ask, WarpCli};
use read::Failure;

const USAGE: &str = "usage: warp [--json]";

/// What a run prints, and how it ends.
#[derive(Debug, PartialEq)]
struct Ran {
    code: u8,
    out: String,
    err: String,
}

fn run(args: &[String], cli: &impl Ask) -> Ran {
    let json = args.iter().any(|arg| arg == "--json");
    if let Some(word) = args.iter().find(|arg| *arg != "--json") {
        return Ran {
            code: 2,
            out: String::new(),
            err: format!("warp: {word} isn't a word warp knows\n{USAGE}\n"),
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

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
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
        let ran = run(&args(&["connect"]), &Contract::whole());
        assert_eq!(
            (ran.code, ran.out.as_str(), ran.err.as_str()),
            (
                2,
                "",
                "warp: connect isn't a word warp knows\nusage: warp [--json]\n"
            )
        );
    }
}
