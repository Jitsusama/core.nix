//! google-rows WORDS: the ready rows and drawers a state's meetings make,
//! as google's daemon serves them (src/ready.rs), so a stand-in for the
//! daemon serves Calendar's rows from this code rather than a copy of it
//! that drifts (tests/desktop/stand-in-google.py). It reads one object on
//! its input, `{"meetings": [...], "held": [...], "now": <ms>}`, each
//! meeting as the daemon tells one, and writes every record it makes, as
//! the daemon would hold it, as one JSON array, in the launcher's words at
//! WORDS, on the machine's clock's zone.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use google::calendar::Meeting;
use google::ready::Ready;
use google::rows::Vocabulary;
use google::state::State;
use serde::Deserialize;
use serde_json::Value;

/// What it's asked to make rows of.
#[derive(Deserialize)]
struct Asked {
    meetings: Vec<Meeting>,
    held: Vec<Meeting>,
    now: i64,
}

fn main() -> ExitCode {
    let Some(words) = std::env::args_os().nth(1).map(PathBuf::from) else {
        eprintln!("google-rows WORDS: name the launcher's words");
        return ExitCode::FAILURE;
    };
    match made(&words) {
        Ok(records) => {
            let mut out = std::io::stdout().lock();
            if writeln!(out, "{records}").is_err() {
                return ExitCode::FAILURE;
            }
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("google-rows: {error}");
            ExitCode::FAILURE
        }
    }
}

/// Every record the asked meetings make, said in the words at `words`.
fn made(words: &Path) -> Result<Value, String> {
    let text = std::fs::read_to_string(words).map_err(|error| error.to_string())?;
    let whole: Value = serde_json::from_str(&text).map_err(|error| error.to_string())?;
    let (vocabulary, lacking) = Vocabulary::read(&whole);
    if let Some(lack) = lacking.first() {
        return Err(format!("the words lack {lack}"));
    }
    let mut input = String::new();
    std::io::stdin()
        .read_to_string(&mut input)
        .map_err(|error| error.to_string())?;
    let asked: Asked = serde_json::from_str(&input).map_err(|error| error.to_string())?;
    let state = State {
        meetings: asked.meetings,
        held: asked.held,
        ..State::default()
    };
    let records = Ready::default().made(&state, &vocabulary, asked.now);
    Ok(Value::Array(
        records.into_iter().map(|(_, record)| record).collect(),
    ))
}
