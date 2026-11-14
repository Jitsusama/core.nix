//! google-rows WORDS: the ready rows and drawers a state's meetings make,
//! as google's daemon serves them (src/ready.rs), so a stand-in for the
//! daemon serves Calendar's rows from this code rather than a copy of it
//! that drifts (tests/desktop/stand-in-google.py). It reads one object on
//! its input, `{"meetings": [...], "held": [...], "now": <ms>}`, each
//! meeting as the daemon tells one, and writes every record it makes, as
//! the daemon would hold it, as one JSON array, in the launcher's words at
//! WORDS, on the machine's clock's zone. Given a `search` too, the ask a
//! shell sent, it writes what the daemon answers it with instead, or null
//! where the daemon refuses it.
//!
//! It takes the shape every command of ours does (docs/commands.md),
//! through the terminal kit. The stand-in parses that array, so it stays
//! as it is; with `--json` it's the `records` of a
//! `google-rows.records/1` object instead, or its `answer` under a search.

use std::io::{Read, Write};
use std::path::Path;
use std::process::ExitCode;

use google::calendar::Meeting;
use google::ready::{self, Ready};
use google::rows::Vocabulary;
use google::state::State;
use serde::Deserialize;
use serde_json::{Value, json};
use terminal_kit::{Context, Exit, Failure, Help};

/// What it's asked to make rows of.
#[derive(Deserialize)]
struct Asked {
    meetings: Vec<Meeting>,
    held: Vec<Meeting>,
    now: i64,
    #[serde(default)]
    search: Option<Value>,
}

fn help() -> Help {
    Help::new(
        "google-rows",
        "the rows a state's meetings make, as google's are",
    )
    .command(
        "WORDS",
        "reads the meetings on stdin, says their rows in the words at WORDS",
    )
    .example(
        "google-rows words.json < state.json",
        "every record, as one JSON array",
    )
    .exit(Exit::Failed, "the records couldn't be written")
    .exit(Exit::BadInput, "stdin or WORDS isn't what it reads")
    .exit(Exit::NotFound, "there's no file at WORDS")
}

fn main() -> ExitCode {
    terminal_kit::main(&help(), env!("CARGO_PKG_VERSION"), |context| {
        let [words] = context.args.as_slice() else {
            return Err(
                Failure::new(Exit::Usage, "google-rows takes the launcher's words alone")
                    .try_this("google-rows --help"),
            );
        };
        let (asked, searched) = made(Path::new(words))?;
        said(context, &asked, searched).map_err(|error| {
            Failure::new(Exit::Failed, "the records couldn't be written").because(error.to_string())
        })
    })
}

/// Says what was made: the bare array or answer the stand-in parses, or
/// the same as a `google-rows.records/1` object under `--json`.
fn said(context: &Context, made: &Value, searched: bool) -> std::io::Result<()> {
    if !context.format().is_json() {
        return writeln!(std::io::stdout().lock(), "{made}");
    }
    let (records, answer) = if searched {
        (Value::Null, made.clone())
    } else {
        (made.clone(), Value::Null)
    };
    context.answer(&json!({
        "schema": "google-rows.records/1",
        "records": records,
        "answer": answer,
    }))
}

/// Every record the asked meetings make, said in the words at `words`, and
/// whether it's a search's answer rather than records.
fn made(words: &Path) -> Result<(Value, bool), Failure> {
    let text = std::fs::read_to_string(words).map_err(|error| {
        Failure::new(Exit::NotFound, "the launcher's words couldn't be read")
            .because(format!("{}: {error}", words.display()))
    })?;
    let unread = |why: String| {
        Failure::new(Exit::BadInput, "the launcher's words aren't what it reads")
            .because(format!("{}: {why}", words.display()))
    };
    let whole: Value = serde_json::from_str(&text).map_err(|error| unread(error.to_string()))?;
    let (vocabulary, lacking) = Vocabulary::read(&whole);
    if let Some(lack) = lacking.first() {
        return Err(unread(format!("the words lack {lack}")));
    }
    let unasked = |why: String| {
        Failure::new(Exit::BadInput, "what it was asked on stdin isn't a state").because(why)
    };
    let mut input = String::new();
    std::io::stdin()
        .read_to_string(&mut input)
        .map_err(|error| unasked(error.to_string()))?;
    let asked: Asked = serde_json::from_str(&input).map_err(|error| unasked(error.to_string()))?;
    let state = State {
        meetings: asked.meetings,
        held: asked.held,
        ..State::default()
    };
    if let Some(search) = asked.search {
        let answer = ready::typed(&state, &vocabulary, &search, asked.now).unwrap_or(Value::Null);
        return Ok((answer, true));
    }
    let records = Ready::default().made(&state, &vocabulary, asked.now);
    Ok((
        Value::Array(records.into_iter().map(|(_, record)| record).collect()),
        false,
    ))
}
