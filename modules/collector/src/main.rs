//! collector keeps the traces an account's programs send it, so a moment
//! can be read back afterwards: what each program was doing then, and how
//! long each step took.
//!
//! - `collector serve` takes spans in OpenTelemetry's JSON form, one
//!   export request a line, on `$XDG_RUNTIME_DIR/collector/lines.sock`, and
//!   keeps them an hour a file in `$XDG_STATE_HOME/collector`, for seven
//!   days and in at most 2 GiB. Beside them it keeps the kernel's own:
//!   each time the machine, or the account's app or session slice, made
//!   something wait for the processor, memory or the disk
//!   (src/pressure.rs says which), and every ten seconds what the account
//!   and its slices with memory floors hold (src/held.rs).
//! - `collector at 09:01:57 [--around 5s]` lists every span open within
//!   that much of the moment, by when it began.
//! - `collector trace <id>` shows one trace's spans as the tree they make.
//!
//! It takes the shape every command of ours does (docs/commands.md),
//! through the terminal kit: `--json` answers `collector.spans/1`, one
//! object holding the spans, and `--format jsonl` a span a line. `--store`,
//! `--socket` and `--pressure` (where /proc and /sys are) name other places.

mod held;
mod otlp;
mod pressure;
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
use terminal_kit::{Context, Exit, Failure, Format, Help, Room, flags, layout};

use otlp::Span;
use store::{Keep, Store};

/// What it does, by the first word.
const COMMANDS: [&str; 3] = ["at", "trace", "serve"];

/// What it says of itself, set as help and read by the check.
fn help() -> Help {
    Help::new(
        "collector",
        "keeps the traces programs send, to read a moment back",
    )
    .example("collector at 09:01:57 --around 5s", "every span open then")
    .example("collector trace 4bf92f35", "one trace's spans, as a tree")
    .example("collector at now --json", "the spans open now, as JSON")
    .subcommand(
        reads("at", "the spans open at a moment, by when each began")
            .example("collector at 09:01:57", "every span open then")
            .example("collector at now --around 5s", "five seconds either side")
            .argument("<time>", "a moment, as 09:01:57 or now")
            .option("--around LENGTH", "that much either side, as 500ms or 5s"),
    )
    .subcommand(
        reads("trace", "one trace's spans, each below its parent")
            .example("collector trace 4bf92f35", "that trace, as a tree")
            .argument("<id>", "the trace's id, or its start")
            .exit(Exit::NotFound, "no such trace is kept"),
    )
    .subcommand(
        Help::new("serve", "keeps what's sent, as its unit runs it")
            .example("collector serve --socket S", "keeps what S is sent")
            .option("--store FOLDER", "where the spans are kept")
            .option("--socket PATH", "where spans are sent")
            .option("--pressure ROOT", "where /proc and /sys are")
            .exit(Exit::Failed, "the store couldn't be kept")
            .exit(Exit::Config, "neither XDG_STATE_HOME nor HOME is set"),
    )
    .option("--store FOLDER", "where the spans are kept")
    .exit(Exit::Failed, "the store couldn't be read or kept")
    .exit(Exit::NotFound, "no such trace is kept")
    .exit(Exit::Config, "neither XDG_STATE_HOME nor HOME is set")
}

/// A subcommand that reads the store.
fn reads(name: &str, about: &str) -> Help {
    Help::new(name, about)
        .option("--store FOLDER", "where the spans are kept")
        .exit(Exit::Failed, "the store couldn't be read")
        .exit(Exit::Config, "neither XDG_STATE_HOME nor HOME is set")
}

fn main() -> ExitCode {
    terminal_kit::main(&help(), env!("CARGO_PKG_VERSION"), body)
}

/// A command line read: the words and the places it names.
struct Asked {
    words: Vec<String>,
    around: i64,
    store: PathBuf,
    socket: PathBuf,
    /// Where the machine's /proc and /sys are, which a test moves.
    pressure: PathBuf,
}

fn usage(what: impl Into<String>) -> Failure {
    Failure::new(Exit::Usage, what).try_this("collector --help")
}

fn asked(args: &[String]) -> Result<Asked, Failure> {
    let mut asked = Asked {
        words: Vec::new(),
        around: 0,
        store: home("XDG_STATE_HOME", ".local/state")?.join("collector"),
        socket: env::var_os("XDG_RUNTIME_DIR")
            .map_or_else(|| PathBuf::from("/run/user/unknown"), PathBuf::from)
            .join("collector/lines.sock"),
        pressure: PathBuf::from("/"),
    };
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        let mut value = || {
            args.next()
                .cloned()
                .ok_or_else(|| usage(format!("{arg} needs a value")))
        };
        match arg.as_str() {
            "--around" => asked.around = when::length(&value()?).map_err(usage)?,
            "--store" => asked.store = PathBuf::from(value()?),
            "--socket" => asked.socket = PathBuf::from(value()?),
            "--pressure" => asked.pressure = PathBuf::from(value()?),
            flag if flag.starts_with("--") => {
                return Err(flags::unknown(
                    "option",
                    flag,
                    &["--around", "--store", "--socket", "--pressure"],
                ));
            }
            _ => asked.words.push(arg.clone()),
        }
    }
    Ok(asked)
}

/// A folder from the environment, or its usual place under home.
fn home(variable: &str, usual: &str) -> Result<PathBuf, Failure> {
    if let Some(dir) = env::var_os(variable).filter(|dir| !dir.is_empty()) {
        return Ok(PathBuf::from(dir));
    }
    env::var_os("HOME")
        .map(|home| PathBuf::from(home).join(usual))
        .ok_or_else(|| {
            Failure::new(Exit::Config, "there's nowhere the spans are kept")
                .because(format!("neither {variable} nor HOME is set"))
                .try_this("collector --store FOLDER at now")
        })
}

/// The store, opened to read or keep.
fn store(asked: &Asked) -> Result<Store, Failure> {
    Store::new(&asked.store, Keep::USUAL)
        .map_err(|why| Failure::new(Exit::Failed, "the store couldn't be opened").because(why))
}

fn unread(why: String) -> Failure {
    Failure::new(Exit::Failed, "the store couldn't be read").because(why)
}

fn body(context: &Context) -> Result<(), Failure> {
    let asked = asked(&context.args)?;
    let zone = TimeZone::system();
    match asked.words.iter().map(String::as_str).collect::<Vec<_>>()[..] {
        // It keeps no state to show bare, so it says how it's asked.
        [] if context.format().is_json() => context.answer(&help().json()).map_err(|error| {
            Failure::new(Exit::Failed, "help couldn't be said").because(error.to_string())
        }),
        [] => said(context, &help().brief(&context.out, context.room)),
        ["serve"] => serve::serve(&asked.socket, store(&asked)?, &asked.pressure).map_err(|why| {
            Failure::new(Exit::Failed, "the collector stopped keeping spans").because(why)
        }),
        ["at", moment] => {
            let at = when::moment(moment, &zone, Timestamp::now()).map_err(usage)?;
            let spans = store(&asked)?
                .during(at - asked.around, at + asked.around)
                .map_err(unread)?;
            let ordered: Vec<(&Span, usize)> = spans.iter().map(|span| (span, 0)).collect();
            if spans.is_empty() && *context.format() == Format::Text {
                return said(context, "No span was open then.\n");
            }
            answered(context, &ordered, &zone)
        }
        ["trace", id] => {
            let spans = store(&asked)?.trace(id).map_err(unread)?;
            if spans.is_empty() {
                return Err(
                    Failure::new(Exit::NotFound, format!("no trace {id} is kept"))
                        .because("it's past seven days, or was never sent")
                        .try_this("collector at TIME"),
                );
            }
            answered(context, &tree(&spans), &zone)
        }
        ["at"] => Err(usage("at needs a time, as 09:01:57 or now")),
        ["trace"] => Err(usage("trace needs a trace's id")),
        [word, ..] if !COMMANDS.contains(&word) => Err(flags::unknown("command", word, &COMMANDS)),
        [word, ..] => Err(usage(format!("{word} takes {}", takes(word)))),
    }
}

/// What each command takes, for a line that gave it too much.
fn takes(command: &str) -> &'static str {
    match command {
        "at" => "one time",
        "trace" => "one trace's id",
        _ => "no words",
    }
}

fn said(context: &Context, text: &str) -> Result<(), Failure> {
    context.say(text).map_err(|error| {
        Failure::new(Exit::Failed, "the answer couldn't be said").because(error.to_string())
    })
}

/// The spans in the form asked: a row each for a person, one object for
/// `json`, a line each for `jsonl`.
fn answered(context: &Context, spans: &[(&Span, usize)], zone: &TimeZone) -> Result<(), Failure> {
    let done = match context.format() {
        Format::Jsonl => spans
            .iter()
            .try_for_each(|(span, _)| context.line("span", &fact(span))),
        format if format.is_json() => context.answer(&json!({
            "schema": "collector.spans/1",
            "spans": spans.iter().map(|(span, _)| fact(span)).collect::<Vec<_>>(),
        })),
        _ => {
            let rows: String = spans
                .iter()
                .map(|(span, depth)| row(span, *depth, zone, context.room) + "\n")
                .collect();
            context.say(&rows)
        }
    };
    done.map_err(|error| {
        Failure::new(Exit::Failed, "the spans couldn't be said").because(error.to_string())
    })
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

/// A moment in RFC 3339, to the nanosecond the span gave, or null when
/// it's past what a time can say.
fn instant(nanos: i64) -> Value {
    Timestamp::from_nanosecond(i128::from(nanos))
        .map_or(Value::Null, |at| Value::String(at.to_string()))
}

/// How long a span took, in milliseconds.
fn took_ms(span: &Span) -> f64 {
    #[expect(
        clippy::cast_precision_loss,
        reason = "a span's length said to well under a microsecond"
    )]
    let took = (span.end - span.start) as f64 / 1e6;
    took
}

/// A span as facts.
fn fact(span: &Span) -> Value {
    json!({
        "trace_id": span.trace,
        "span_id": span.id,
        "parent_id": span.parent,
        "name": span.name,
        "service": span.service,
        "started_at": instant(span.start),
        "ended_at": instant(span.end),
        "took_ms": took_ms(span),
        "status": span.status,
        "attributes": span.attributes,
    })
}

/// How far in a span's words go on below its head, when the head takes
/// more than half the room.
const INSET: usize = 4;

/// A span as a person reads it: when, how long, its name indented by how
/// deep it sits, its service and trace, then its status and attributes.
/// In a terminal the words after its head wrap beside it, under the name,
/// or below it when the head takes more than half the room; into a pipe
/// it's one line.
fn row(span: &Span, depth: usize, zone: &TimeZone, room: Room) -> String {
    let head = format!(
        "{}  {:>9.1} ms  ",
        when::shown(span.start, zone),
        took_ms(span)
    );
    let indent = "  ".repeat(depth);
    let mut pieces = vec![
        ("", span.name.clone()),
        ("  ", span.service.clone()),
        ("  ", span.trace[..8].to_owned()),
    ];
    pieces.extend(span.status.iter().map(|status| ("  ", status.clone())));
    pieces.extend(span.attributes.iter().enumerate().map(|(n, (key, value))| {
        let said = match value {
            Value::String(text) => format!("{key}={text}"),
            other => format!("{key}={other}"),
        };
        (if n == 0 { "  " } else { " " }, said)
    }));
    let whole = pieces
        .iter()
        .fold(String::new(), |whole, (gap, piece)| whole + gap + piece);
    let line = format!("{head}{indent}{whole}");
    let Some(cells) = room.cells().filter(|cells| layout::width(&line) > *cells) else {
        return line;
    };
    let beside = layout::width(&head) + indent.len();
    if beside <= cells / 2 {
        let pad = " ".repeat(beside);
        let lines = filled(&pieces, cells - beside);
        format!("{head}{indent}{}", lines.join(&format!("\n{pad}")))
    } else {
        let pad = format!("{}{indent}", " ".repeat(INSET));
        let lines = filled(&pieces, cells.saturating_sub(pad.len()));
        format!(
            "{}\n{pad}{}",
            head.trim_end(),
            lines.join(&format!("\n{pad}"))
        )
    }
}

/// Pieces set into lines of `measure` cells, each kept whole and parted
/// from the one before by its gap, but for one too long for any line,
/// which wraps.
fn filled(pieces: &[(&str, String)], measure: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();
    for (gap, piece) in pieces {
        if !line.is_empty() {
            if layout::width(&line) + layout::width(gap) + layout::width(piece) <= measure {
                line.push_str(gap);
                line.push_str(piece);
                continue;
            }
            lines.push(std::mem::take(&mut line));
        }
        let mut parts = layout::wrapped(piece, measure.max(1));
        line = parts.pop().unwrap_or_default();
        lines.extend(parts);
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    fn span() -> Span {
        let mut attributes = serde_json::Map::new();
        attributes.insert("row".to_owned(), json!("the launcher's quests"));
        attributes.insert("shown".to_owned(), json!(14));
        Span {
            trace: "4bf92f3577b34da6a3ce929d0e0e4736".to_owned(),
            id: "00f067aa0ba902b7".to_owned(),
            parent: None,
            name: "rows drawn".to_owned(),
            service: "launcher".to_owned(),
            start: 1_763_024_517_123_000_000,
            end: 1_763_024_517_135_400_000,
            status: Some("ok".to_owned()),
            attributes,
        }
    }

    fn lines(room: Room, depth: usize) -> Vec<String> {
        row(&span(), depth, &TimeZone::UTC, room)
            .lines()
            .map(str::to_owned)
            .collect()
    }

    #[test]
    fn into_a_pipe_a_span_is_one_line() {
        let said = lines(Room::Unbounded, 1);
        assert_eq!(
            said,
            [
                "2025-11-13 09:01:57.123       12.4 ms    rows drawn  launcher  4bf92f35  ok  \
              row=the launcher's quests shown=14"
            ],
            "a pipe should take the row whole"
        );
    }

    #[test]
    fn with_room_the_words_wrap_beside_the_head_under_the_name() {
        let said = lines(Room::Cells(80), 0);
        assert!(said.len() > 1, "it should wrap at 80: {said:?}");
        let under = said[0]
            .find("rows drawn")
            .expect("the first line should hold the name");
        for line in &said {
            assert!(layout::width(line) <= 80, "{line:?} runs past 80");
        }
        for line in &said[1..] {
            assert_eq!(
                line.len() - line.trim_start().len(),
                under,
                "{line:?} should start under the name"
            );
        }
    }

    #[test]
    fn narrow_the_words_go_below_the_head_inset_by_its_depth() {
        let said = lines(Room::Cells(60), 1);
        assert_eq!(
            said[0], "2025-11-13 09:01:57.123       12.4 ms",
            "the head should stand alone"
        );
        for line in &said[1..] {
            assert!(layout::width(line) <= 60, "{line:?} runs past 60");
            assert!(
                line.starts_with("      ") && !line.starts_with("       "),
                "{line:?} should start 6 in"
            );
        }
        assert!(
            said[1].starts_with("      rows drawn  launcher"),
            "the name should lead: {said:?}"
        );
    }

    #[test]
    fn a_row_that_fits_is_one_line() {
        assert_eq!(
            lines(Room::Cells(140), 0).len(),
            1,
            "at 140 the row should fit"
        );
    }
}
