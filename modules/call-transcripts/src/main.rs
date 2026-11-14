//! transcripts-people: the calls call-transcripts wrote down, as one of
//! People's sources, which the people daemon runs on People's line
//! protocol (modules/people/src/main.rs). Each line it's written
//! is an address, `?` before one asked ahead of Joel stepping in; it
//! answers each at once with the calls with whoever has that address,
//! each one thing between them, its row ready (rows.rs), and says them
//! again whenever a call with them changes, as call-transcripts' daemon
//! tells it over calls.sock. It names nobody and knows nobody, so each
//! answer is `known` false: everyone a call was with is someone another
//! source knows already. People draws what it serves as served, with no
//! QML of its own (Y2YBTS, 67770006).
//!
//! It reads the launcher's words from the file `LAUNCHER_WORDS` names,
//! as slack-people does, since the daemon runs a source's command as it's
//! given.

mod rows;
mod styled;
mod when;

use std::collections::{BTreeMap, BTreeSet};
use std::io::{BufRead, Write};
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::mpsc;
use std::time::{SystemTime, UNIX_EPOCH};

use daemon_serve::watch::{self, Held};
use serde_json::{Value, json};
use terminal_kit::{Exit, Failure, Help};

use rows::Words;

fn help() -> Help {
    Help::new(
        "transcripts-people",
        "the calls written down with someone, for People",
    )
    .command(
        "[CALLS]",
        "reads an address a line on stdin, says the calls with each",
    )
    .example(
        "echo ada@example.com | transcripts-people",
        "the calls with that address, as a line of JSON, from calls.sock",
    )
}

fn main() -> ExitCode {
    terminal_kit::main(
        &help(),
        env!("CARGO_PKG_VERSION"),
        |context| match context.args.as_slice() {
            [] => {
                answer(calls_here(), &words()?);
                Ok(())
            }
            [calls] => {
                answer(PathBuf::from(calls), &words()?);
                Ok(())
            }
            _ => Err(Failure::new(
                Exit::Usage,
                "transcripts-people takes call-transcripts' socket, or none for its own",
            )
            .try_this("transcripts-people --help")),
        },
    )
}

/// call-transcripts' socket in this session.
fn calls_here() -> PathBuf {
    let runtime = std::env::var_os("XDG_RUNTIME_DIR").unwrap_or_else(|| "/run/user".into());
    PathBuf::from(runtime).join("daemons/calls.sock")
}

/// The launcher's words a call is said in, from the file `LAUNCHER_WORDS`
/// names.
fn words() -> Result<Words, Failure> {
    let path = std::env::var_os("LAUNCHER_WORDS").ok_or_else(|| {
        Failure::new(
            Exit::Config,
            "LAUNCHER_WORDS names no file of the launcher's words",
        )
    })?;
    let read = std::fs::read_to_string(&path)
        .map_err(|why| why.to_string())
        .and_then(|text| serde_json::from_str(&text).map_err(|why| why.to_string()))
        .and_then(|whole| Words::read(&whole));
    read.map_err(|why| {
        Failure::new(Exit::Config, "the launcher's words say no call")
            .because(format!("{}: {why}", PathBuf::from(path).display()))
    })
}

/// What's heard: an address asked, an answer of call-transcripts', or the
/// end of what the people daemon writes.
enum Heard {
    Asked(String),
    Calls(Value),
    Ended,
}

/// Answers each address written on stdin with the calls with it, and
/// again as they change, until the daemon stops writing or reading.
fn answer(calls: PathBuf, words: &Words) {
    let (to, heard) = mpsc::channel();
    let lines = to.clone();
    std::thread::spawn(move || {
        for line in std::io::stdin().lock().lines() {
            let Ok(line) = line else { break };
            if lines.send(Heard::Asked(line)).is_err() {
                return;
            }
        }
        let _ = lines.send(Heard::Ended);
    });
    watch::follow(calls, "call", Heard::Calls, to);
    let mut held = Held::default();
    // Each address asked, with what it was last said, so a change to a
    // call says again only what reads differently.
    let mut said: BTreeMap<String, String> = BTreeMap::new();
    let stdout = std::io::stdout();
    for one in heard {
        let asked: BTreeSet<String> = match one {
            Heard::Asked(line) => {
                let address = line.trim().trim_start_matches('?').trim().to_lowercase();
                if address.is_empty() {
                    continue;
                }
                // Every ask is answered, the same or not, as the daemon
                // waits on each source's answer as Joel steps in.
                said.remove(&address);
                BTreeSet::from([address])
            }
            // Everyone asked is said again at each change, each by looking
            // through every call, rather than indexing calls by who they
            // were with, since a call changes only a few dozen times: as
            // it starts and ends, as its transcribing starts and stops, as
            // its summary comes, and as its opening grows, which stops at
            // four paragraphs or 400 characters, a minute or two in
            // (book.rs).
            Heard::Calls(answer) => {
                if held.took(&answer).is_empty() {
                    continue;
                }
                said.keys().cloned().collect()
            }
            Heard::Ended => return,
        };
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |since| {
                i64::try_from(since.as_millis()).unwrap_or(i64::MAX)
            });
        let this_year = when::local(now).year;
        let mut out = stdout.lock();
        for address in asked {
            let line = of(&held, &address, words, this_year).to_string();
            if said.get(&address) == Some(&line) {
                continue;
            }
            if writeln!(out, "{line}").and_then(|()| out.flush()).is_err() {
                return;
            }
            said.insert(address, line);
        }
    }
}

/// What's said of an address: the calls with it, newest first.
fn of(held: &Held, address: &str, words: &Words, this_year: i32) -> Value {
    let mut calls: Vec<&Value> = held
        .records()
        .map(|(_, call)| call)
        .filter(|call| call["kind"] == "call" && rows::with(call, address))
        .collect();
    calls.sort_by_key(|call| std::cmp::Reverse(call["started"].as_i64().unwrap_or(0)));
    let between: Vec<Value> = calls
        .into_iter()
        .map(|call| rows::met(call, words, when::local, this_year))
        .collect();
    json!({"email": address, "known": false, "between": between})
}
