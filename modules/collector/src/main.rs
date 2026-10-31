//! collector keeps the traces an account's programs send it, so a moment
//! can be read back afterwards: what each program was doing then, and how
//! long each step took.
//!
//! - `collector serve` takes spans in OpenTelemetry's JSON form, one
//!   export request a line, on `$XDG_RUNTIME_DIR/collector/lines.sock`, and
//!   keeps them an hour a file in `$XDG_STATE_HOME/collector`, for seven
//!   days and in at most 2 GiB.
//! - `collector at 09:01:57 [--around 5s]` lists every span open within
//!   that much of the moment, by when it began.
//! - `collector trace <id>` shows one trace's spans as the tree they make.
//!
//! `--json` prints a span a line instead, and `--store` and `--socket`
//! name other places.

mod otlp;
mod serve;
mod store;
mod when;

use std::collections::HashMap;
use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

use jiff::Timestamp;
use jiff::tz::TimeZone;
use serde_json::{Value, json};

use otlp::Span;
use store::{Keep, Store};

const USAGE: &str = "usage: collector serve | at <time> [--around <length>] | trace <id> \
                     [--json] [--store <folder>] [--socket <path>]";

fn main() -> ExitCode {
    match run(env::args().skip(1).collect()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("collector: {e}");
            ExitCode::FAILURE
        }
    }
}

struct Asked {
    words: Vec<String>,
    around: i64,
    json: bool,
    store: PathBuf,
    socket: PathBuf,
}

fn asked(args: Vec<String>) -> Result<Asked, String> {
    let mut asked = Asked {
        words: Vec::new(),
        around: 0,
        json: false,
        store: home("XDG_STATE_HOME", ".local/state")?.join("collector"),
        socket: env::var_os("XDG_RUNTIME_DIR")
            .map_or_else(|| PathBuf::from("/run/user/unknown"), PathBuf::from)
            .join("collector/lines.sock"),
    };
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        let mut value = || args.next().ok_or_else(|| format!("{arg} needs a value"));
        match arg.as_str() {
            "--around" => asked.around = when::length(&value()?)?,
            "--json" => asked.json = true,
            "--store" => asked.store = PathBuf::from(value()?),
            "--socket" => asked.socket = PathBuf::from(value()?),
            flag if flag.starts_with("--") => return Err(format!("{flag}? {USAGE}")),
            _ => asked.words.push(arg),
        }
    }
    Ok(asked)
}

/// A folder from the environment, or its usual place under home.
fn home(variable: &str, usual: &str) -> Result<PathBuf, String> {
    if let Some(dir) = env::var_os(variable).filter(|dir| !dir.is_empty()) {
        return Ok(PathBuf::from(dir));
    }
    env::var_os("HOME")
        .map(|home| PathBuf::from(home).join(usual))
        .ok_or_else(|| format!("neither {variable} nor HOME is set"))
}

fn run(args: Vec<String>) -> Result<(), String> {
    let asked = asked(args)?;
    let zone = TimeZone::system();
    match asked.words.iter().map(String::as_str).collect::<Vec<_>>()[..] {
        ["serve"] => serve::serve(&asked.socket, Store::new(&asked.store, Keep::USUAL)?),
        ["at", moment] => {
            let at = when::moment(moment, &zone, Timestamp::now())?;
            let spans = Store::new(&asked.store, Keep::USUAL)?
                .during(at - asked.around, at + asked.around)?;
            if spans.is_empty() && !asked.json {
                println!("No span was open then.");
            }
            for span in &spans {
                show(span, 0, asked.json, &zone);
            }
            Ok(())
        }
        ["trace", id] => {
            let spans = Store::new(&asked.store, Keep::USUAL)?.trace(id)?;
            if spans.is_empty() {
                return Err(format!("no trace {id} is kept"));
            }
            for (span, depth) in tree(&spans) {
                show(span, depth, asked.json, &zone);
            }
            Ok(())
        }
        _ => Err(USAGE.to_owned()),
    }
}

/// The spans in the order a tree reads, each with how deep it sits: every
/// span below its parent, and those whose parent isn't kept at the top.
fn tree(spans: &[Span]) -> Vec<(&Span, usize)> {
    let mut children: HashMap<Option<&str>, Vec<&Span>> = HashMap::new();
    let kept: Vec<&str> = spans.iter().map(|s| s.id.as_str()).collect();
    for span in spans {
        let parent = span.parent.as_deref().filter(|p| kept.contains(p));
        children.entry(parent).or_default().push(span);
    }
    let mut ordered = Vec::new();
    let mut waiting: Vec<(&Span, usize)> = children
        .get(&None)
        .into_iter()
        .flatten()
        .rev()
        .map(|s| (*s, 0))
        .collect();
    while let Some((span, depth)) = waiting.pop() {
        ordered.push((span, depth));
        if let Some(below) = children.get(&Some(span.id.as_str())) {
            waiting.extend(below.iter().rev().map(|s| (*s, depth + 1)));
        }
    }
    ordered
}

fn show(span: &Span, depth: usize, as_json: bool, zone: &TimeZone) {
    if as_json {
        let line = json!({
            "trace": span.trace,
            "span": span.id,
            "parent": span.parent,
            "name": span.name,
            "service": span.service,
            "start": span.start,
            "end": span.end,
            "status": span.status,
            "attributes": span.attributes,
        });
        println!("{line}");
        return;
    }
    let attributes: Vec<String> = span
        .attributes
        .iter()
        .map(|(key, value)| match value {
            Value::String(text) => format!("{key}={text}"),
            other => format!("{key}={other}"),
        })
        .collect();
    #[expect(
        clippy::cast_precision_loss,
        reason = "a span's length shown to 0.1 ms"
    )]
    let took = (span.end - span.start) as f64 / 1e6;
    println!(
        "{}  {:>9.1} ms  {}{}  {}  {}{}{}",
        when::shown(span.start, zone),
        took,
        "  ".repeat(depth),
        span.name,
        span.service,
        &span.trace[..8],
        span.status
            .as_deref()
            .map_or(String::new(), |s| format!("  {s}")),
        if attributes.is_empty() {
            String::new()
        } else {
            format!("  {}", attributes.join(" "))
        },
    );
}
