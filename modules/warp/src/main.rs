//! warp: how WARP's link stands, read from warp-cli, WARP's own client,
//! every figure it gives in its own unit, in the terminal kit's shape, as
//! `warp --help` says (`help()`):
//!
//!   warp             the page: the link, the tunnel, TLS, the settings, DNS
//!   warp --json      the same as one record (model.rs; warp.json beside this)
//!   warp connect     bring WARP up, as the launcher's :connect does
//!   warp disconnect  take it down, as :disconnect does (switch.rs says when
//!                    each is left as it is), with --json as a record
//!   warp serve SOCKET  hold the link for the shell, which reads it and
//!                    switches it over SOCKET (serve.rs)
//!
//! `WARP_CLI` names the warp-cli to ask, as the shell names the one the
//! system runs; without it, it's the one on the PATH. Serving, `WARP_SHAPER`
//! names the network shaper's status.json, so the link says what waits on
//! the way through it (waiting.rs), and how WARP stands says when that's
//! slow (slow.rs); without it, it says nothing of either.
//!
//! Run by hand, it asks warp-cli each time and keeps nothing: WARP's
//! service keeps every figure itself, answering each question in about
//! 20 ms (2026-11-07, on a laptop). Serving the shell, it asks only while
//! the link is watched, every two seconds, so the launcher starts no
//! process to read WARP or to switch it.
//!
//! It ends with the kit's codes (docs/commands.md): 0 having read WARP or
//! switched it, 64 on a word it doesn't take, 66 when there's no warp-cli,
//! so no WARP to read, 69 when WARP's service isn't answering, 65 when
//! warp-cli said what it can't read, 77 when the organisation holds the
//! switch and 1 when warp-cli wouldn't switch it, each said on stderr,
//! never a trace. With --json, a failure prints its record too, as
//! { error, said } or the switch's, which serve.rs hands the shell. Serving, it runs until stopped, and ends 1 only when it can't
//! listen on its socket. Nothing reads these codes: the shell asks
//! `warp serve` over its socket instead.

mod ask;
#[cfg(test)]
mod contract;
mod model;
mod page;
mod read;
mod serve;
mod slow;
mod switch;
mod waiting;

use std::io::Write as _;
use std::process::ExitCode;

use ask::{Ask, WarpCli};
use terminal_kit::{Exit, Failure, Help};

/// What a run prints on stdout, and how it ends.
#[derive(Debug, PartialEq)]
struct Ran {
    out: String,
    ended: Result<(), Failure>,
}

/// A run over the words the kit left, the record rather than the page
/// where `json`.
fn run(words: &[String], json: bool, cli: &impl Ask) -> Ran {
    let act = match words {
        [] => None,
        [word] if word == "connect" => Some(switch::Act::Connect),
        [word] if word == "disconnect" => Some(switch::Act::Disconnect),
        _ => {
            return Ran {
                out: String::new(),
                ended: Err(Failure::new(Exit::Usage, "warp takes one word at a time")
                    .try_this("warp --help")),
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
            out: format!(
                "{}\n",
                serde_json::to_string(&warp).expect("the record is JSON")
            ),
            ended: Ok(()),
        },
        Ok(warp) => Ran {
            out: page::page(&warp),
            ended: Ok(()),
        },
        Err(failure) => failed(&failure, json),
    }
}

/// A reading that failed, as the kit says a failure, and with --json as
/// the `{ error, said }` record the shell reads through serve.rs too.
fn failed(failure: &read::Failure, json: bool) -> Ran {
    let (kind, said, ended) = match failure {
        read::Failure::NotInstalled(at) => (
            "not-installed",
            at,
            Failure::new(
                Exit::NotFound,
                format!("{at} isn't installed, so there's no WARP here to read"),
            ),
        ),
        read::Failure::NotAnswering(why) => (
            "not-answering",
            why,
            Failure::new(Exit::Unavailable, "WARP's service isn't answering").because(why),
        ),
        read::Failure::Unreadable(why) => (
            "unreadable",
            why,
            Failure::new(Exit::BadInput, "warp-cli said what warp can't read").because(why),
        ),
    };
    Ran {
        out: if json {
            format!("{}\n", serde_json::json!({ "error": kind, "said": said }))
        } else {
            String::new()
        },
        ended: Err(ended),
    }
}

/// A switch, said in a line on stdout where it was done and as a failure
/// where it wasn't, with --json as its record either way.
fn switched_ran(switched: &switch::Switched, json: bool) -> Ran {
    let out = if json {
        format!(
            "{}\n",
            serde_json::to_string(switched).expect("the record is JSON")
        )
    } else if switched.refused.is_none() {
        format!("{}\n", switched.said)
    } else {
        String::new()
    };
    let ended = match switched.refused {
        None => Ok(()),
        Some("held") => Err(Failure::new(Exit::Refused, &switched.said)),
        Some(_) => Err(Failure::new(Exit::Failed, &switched.said)),
    };
    Ran { out, ended }
}

/// What `warp --help` says, and the words the kit walks.
fn help() -> Help {
    let reading = |help: Help| {
        help.exit(Exit::NotFound, "there's no warp-cli to ask")
            .exit(Exit::Unavailable, "WARP's service isn't answering")
            .exit(Exit::BadInput, "warp-cli said what it can't read")
    };
    let switching = |name: &str, about: &str, does: &str| {
        reading(
            Help::new(name, about)
                .example(&format!("warp {name}"), does)
                .example(&format!("warp {name} --json"), "the switch, as a record")
                .exit(Exit::Refused, "the organisation holds the switch")
                .exit(Exit::Failed, "warp-cli wouldn't switch it"),
        )
    };
    reading(
        Help::new("warp", "how WARP's link stands, read from warp-cli")
            .example("warp", "the link, tunnel, TLS, settings and DNS")
            .example("warp --json", "the same, as one record")
            .subcommand(switching("connect", "brings WARP up", "bring WARP up"))
            .subcommand(switching("disconnect", "takes WARP down", "take WARP down"))
            .subcommand(
                Help::new("serve", "holds the link for the shell, over a socket")
                    .argument("<socket>", "where the shell asks")
                    .example(
                        "warp serve $XDG_RUNTIME_DIR/daemons/warp.sock",
                        "what its unit runs",
                    )
                    .exit(Exit::Failed, "the socket couldn't be listened on"),
            ),
    )
}

/// The link held for the shell over `socket`, until stopped.
fn serving(socket: &str) -> Result<(), Failure> {
    let shaper = std::env::var_os("WARP_SHAPER")
        .map(|status| serve::Shaper::at(std::path::PathBuf::from(status)));
    serve::serve(std::path::Path::new(socket), WarpCli::from_env(), shaper).map_err(|why| {
        Failure::new(Exit::Failed, format!("{socket} couldn't be listened on"))
            .because(why.to_string())
    })
}

fn main() -> ExitCode {
    terminal_kit::main(&help(), env!("CARGO_PKG_VERSION"), |context| {
        match context.args.as_slice() {
            [serve, socket] if serve == "serve" => return serving(socket),
            [serve, ..] if serve == "serve" => {
                return Err(Failure::new(Exit::Usage, "serve takes one socket")
                    .try_this("warp serve --help"));
            }
            _ => {}
        }
        let ran = run(
            &context.args,
            context.format().is_json(),
            &WarpCli::from_env(),
        );
        // A closed pipe, as `warp | head` gives, changes nothing about the
        // reading, so it isn't said.
        let _ = std::io::stdout().write_all(ran.out.as_bytes());
        ran.ended
    })
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

    /// The code a run ends with, as the kit would end it.
    fn code(ran: &Ran) -> u8 {
        ran.ended
            .as_ref()
            .map_or_else(|failure| failure.exit.code(), |()| 0)
    }

    /// What a failed run says failed, and why where it says.
    fn said_failed(ran: &Ran) -> (&str, Option<&str>) {
        let failure = ran.ended.as_ref().expect_err("the run failed");
        (failure.what.as_str(), failure.why.as_deref())
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
        let page = run(&args(&[]), false, &Contract::whole());
        assert_eq!(page.ended, Ok(()), "the reading failed");
        assert!(
            page.out.starts_with("WARP \u{b7} connected"),
            "{}",
            page.out
        );
        let record = run(&args(&[]), true, &Contract::whole());
        assert_eq!(record.ended, Ok(()), "the reading failed");
        let printed: Value = serde_json::from_str(&record.out).expect("it prints JSON");
        let golden: Value = serde_json::from_str(
            &std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/warp.json"))
                .expect("warp.json is there"),
        )
        .expect("warp.json is JSON");
        assert_eq!(printed, golden, "the record isn't warp.json");
    }

    #[test]
    fn each_failure_is_said_and_ends_with_the_kits_code_for_it() {
        let none = run(&args(&[]), false, &missing());
        assert_eq!(
            (code(&none), none.out.as_str(), said_failed(&none)),
            (
                66,
                "",
                (
                    "warp-cli isn't installed, so there's no WARP here to read",
                    None
                )
            ),
            "no warp-cli is said wrongly"
        );
        let quiet = run(&args(&[]), false, &down());
        assert_eq!(
            (code(&quiet), quiet.out.as_str(), said_failed(&quiet)),
            (
                69,
                "",
                (
                    "WARP's service isn't answering",
                    Some("No such file or directory (os error 2)")
                )
            ),
            "a quiet service is said wrongly"
        );
        let odd = run(&args(&[]), false, &refusing());
        assert_eq!(
            (code(&odd), odd.out.as_str(), said_failed(&odd)),
            (
                65,
                "",
                (
                    "warp-cli said what warp can't read",
                    Some("warp-cli status said InvalidAccountType: Invalid account type.")
                )
            ),
            "an unreadable answer is said wrongly"
        );
    }

    #[test]
    fn a_failure_with_json_is_a_record_saying_which_and_why() {
        let quiet = run(&args(&[]), true, &down());
        assert_eq!(code(&quiet), 69, "a quiet service ended wrongly");
        let said: Value = serde_json::from_str(&quiet.out).expect("it prints JSON");
        assert_eq!(
            said,
            serde_json::json!({
                "error": "not-answering",
                "said": "No such file or directory (os error 2)"
            }),
            "a quiet service's record is wrong"
        );
        let none = run(&args(&[]), true, &missing());
        assert_eq!(code(&none), 66, "no warp-cli ended wrongly");
        let said: Value = serde_json::from_str(&none.out).expect("it prints JSON");
        assert_eq!(
            said["error"], "not-installed",
            "no warp-cli's record is wrong"
        );
    }

    #[test]
    fn more_than_one_word_is_refused_as_usage() {
        let two = run(&args(&["connect", "disconnect"]), false, &Contract::whole());
        assert_eq!(code(&two), 64, "two words weren't refused");
        assert_eq!(
            said_failed(&two).0,
            "warp takes one word at a time",
            "the refusal says the wrong thing"
        );
    }

    #[test]
    fn a_word_it_doesnt_know_is_refused_before_anything_is_asked() {
        let ending = terminal_kit::run(
            &help(),
            "0",
            &terminal_kit::Environment::default(),
            args(&["reconnect"]),
            |_| panic!("an unknown word reached warp's body"),
        );
        assert_eq!(ending.exit, Exit::Usage, "{}", ending.stderr);
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
        let ran = run(&args(&["disconnect"]), false, &cli);
        assert_eq!(
            (code(&ran), ran.out.as_str()),
            (
                0,
                "WARP is disconnected, and comes back by itself in 2 min\n"
            ),
            "the disconnect is said wrongly"
        );
        let record = run(&args(&["disconnect"]), true, &cli);
        assert_eq!(code(&record), 0, "the disconnect failed");
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
            ),
            "the disconnect's record is wrong"
        );
    }

    #[test]
    fn connect_brings_warp_up() {
        let cli = Contract::whole()
            .with("status", disconnected())
            .with("connect", done());
        let ran = run(&args(&["connect"]), false, &cli);
        assert_eq!(
            (code(&ran), ran.out.as_str()),
            (0, "WARP is connecting\n"),
            "the connect is said wrongly"
        );
    }

    #[test]
    fn a_switch_the_organisation_holds_is_refused() {
        let cli = Contract::whole()
            .with("settings", held())
            .with("disconnect", done());
        let ran = run(&args(&["disconnect"]), false, &cli);
        assert_eq!(
            (code(&ran), ran.out.as_str(), said_failed(&ran)),
            (
                77,
                "",
                ("example holds WARP's switch, so it's left connected", None)
            ),
            "a held switch is said wrongly"
        );
        let record = run(&args(&["disconnect"]), true, &cli);
        assert_eq!(code(&record), 77, "a held switch wasn't refused");
        let said: Value = serde_json::from_str(&record.out).expect("it prints JSON");
        assert_eq!(
            (&said["refused"], &said["organisation"]),
            (&Value::from("held"), &Value::from("example")),
            "a held switch's record doesn't say who holds it"
        );
    }

    #[test]
    fn a_switch_warp_cli_refuses_fails() {
        // The contract refuses a question it isn't given.
        let cli = Contract::whole().with("status", disconnected());
        let ran = run(&args(&["connect"]), false, &cli);
        assert_eq!(
            (code(&ran), ran.out.as_str(), said_failed(&ran)),
            (
                1,
                "",
                ("warp-cli wouldn't connect: Invalid account type.", None)
            ),
            "a refused switch is said wrongly"
        );
    }

    #[test]
    fn a_switch_with_no_warp_to_ask_fails_as_a_reading_does() {
        let ran = run(&args(&["disconnect"]), false, &missing());
        assert_eq!(code(&ran), 66, "a switch with no warp-cli ended wrongly");
    }
}
