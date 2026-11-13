//! pi-packages: each of pi's packages built into a version of its own and
//! swapped in whole (src/lib.rs says how).
//!
//! It takes the shape every command of ours does (docs/commands.md),
//! through the terminal kit: its help says each command, a bare line is
//! `status`, and `--json` answers each as one object naming its schema,
//! `pi-packages.<command>/1`, with `--format jsonl` a package a line. The
//! account's `pi-packages` names the settings with `--config FILE`, which
//! the unit's runs name themselves (home.nix).

use std::path::Path;
use std::process::ExitCode;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};
use terminal_kit::{Context, Exit, Failure, Format, Help};

use pi_packages::ask::{Asked, asked};
use pi_packages::config::{Config, Package};
use pi_packages::hold::{self, Hold};
use pi_packages::run::{self, Done, Outcome, Pace};
use pi_packages::version::Place;

/// Characters of a commit worth showing.
const SHORT: usize = 8;

/// What it says of itself, set as help and read by the check.
fn help() -> Help {
    Help::new(
        "pi-packages",
        "builds pi's packages as versions and swaps each in whole",
    )
    .example("pi-packages", "where each package is, and any hold")
    .example(
        "pi-packages update --now",
        "each to its branch's head, swapped in now",
    )
    .example("pi-packages hold a demo", "swaps nothing until released")
    .command("status", "where each package's current is, and any hold")
    .command("update", "each to its branch, swapping hourly at most")
    .command("sweep", "removes the versions no pi runs")
    .command("hold WHY...", "swaps nothing, however an update runs")
    .command("release", "ends the hold")
    .option("--now", "update: swaps in now, unless a hold stands")
    .option("--for LENGTH", "hold: only that long, as 90m or 2h")
    .option(
        "--config FILE",
        "the settings file the account's command gives",
    )
    .exit(
        Exit::Failed,
        "a package wasn't updated or swept, or a hold failed",
    )
    .exit(Exit::Config, "the settings couldn't be read")
}

fn main() -> ExitCode {
    terminal_kit::main(&help(), env!("CARGO_PKG_VERSION"), body)
}

fn body(context: &Context) -> Result<(), Failure> {
    let words: Vec<&str> = context.args.iter().map(String::as_str).collect();
    match asked(&words)? {
        Asked::Help if context.format().is_json() => said(context.answer(&help().json())),
        Asked::Help => said(context.say(&help().text(&context.out, context.room))),
        Asked::Update { file, pace } => update(context, &read(file)?, pace),
        Asked::Sweep(file) => sweep(context, &read(file)?),
        Asked::Status(file) => status(context, &read(file)?),
        Asked::Hold { file, why, length } => put_hold(context, &read(file)?, why, length),
        Asked::Release(file) => release(context, &read(file)?),
    }
}

fn read(file: &Path) -> Result<Config, Failure> {
    Config::read(file)
        .map_err(|why| Failure::new(Exit::Config, "the settings couldn't be read").because(why))
}

/// An answer said, or why it couldn't be.
fn said(done: std::io::Result<()>) -> Result<(), Failure> {
    done.map_err(|error| {
        Failure::new(Exit::Failed, "the answer couldn't be said").because(error.to_string())
    })
}

/// One answer in the form asked: its lines for a person, its facts as one
/// object for `json`, and for `jsonl` a line of each kind it holds.
fn answer(
    context: &Context,
    text: &[String],
    whole: &Value,
    lines: &[(&str, Value)],
) -> Result<(), Failure> {
    said(match context.format() {
        Format::Jsonl => lines
            .iter()
            .try_for_each(|(kind, fields)| context.line(kind, fields)),
        format if format.is_json() => context.answer(whole),
        _ => context.say(
            &text
                .iter()
                .fold(String::new(), |said, line| said + line + "\n"),
        ),
    })
}

/// A moment in RFC 3339.
fn instant(at: SystemTime) -> String {
    let seconds = at
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_secs();
    terminal_kit::time::rfc3339(i64::try_from(seconds).unwrap_or(i64::MAX))
}

/// How long a hold lasts, as a person reads it.
fn how_long(now: SystemTime, hold: &Hold) -> String {
    hold.until.map_or_else(
        || "until released".to_owned(),
        |until| format!("for {} more minutes", minutes_until(now, until)),
    )
}

/// A hold as facts: why, and until when, null when until released.
fn hold_fact(hold: &Hold) -> Value {
    json!({ "why": hold.why, "until": hold.until.map(instant) })
}

fn put_hold(
    context: &Context,
    config: &Config,
    why: String,
    length: Option<Duration>,
) -> Result<(), Failure> {
    let now = SystemTime::now();
    let hold = Hold {
        why,
        until: length.map(|length| now + length),
    };
    hold::put(config, &hold)
        .map_err(|why| Failure::new(Exit::Failed, "the hold couldn't be kept").because(why))?;
    let fact = hold_fact(&hold);
    answer(
        context,
        &[format!(
            "swaps are held {}: {}",
            how_long(now, &hold),
            hold.why
        )],
        &json!({ "schema": "pi-packages.hold/1", "hold": fact }),
        &[("hold", fact.clone())],
    )
}

fn release(context: &Context, config: &Config) -> Result<(), Failure> {
    let released = hold::release(config)
        .map_err(|why| Failure::new(Exit::Failed, "the hold couldn't be ended").because(why))?;
    let text = if released {
        "the hold is over; the next update swaps in what waited"
    } else {
        "no hold stood"
    };
    answer(
        context,
        &[text.to_owned()],
        &json!({ "schema": "pi-packages.release/1", "released": released }),
        &[("release", json!({ "released": released }))],
    )
}

fn short(commit: &str) -> &str {
    commit.get(..SHORT).unwrap_or(commit)
}

fn name(package: &Package) -> String {
    package.key.to_string_lossy().into_owned()
}

/// Whole minutes from `now` to `until`, rounded up, so a wait never reads
/// as none.
fn minutes_until(now: SystemTime, until: SystemTime) -> u64 {
    until
        .duration_since(now)
        .unwrap_or(Duration::ZERO)
        .as_secs()
        .div_ceil(60)
}

/// What an update did for one package, as a person reads it.
fn outcome_lines(name: &str, outcome: &Outcome, now: SystemTime) -> Vec<String> {
    let mut lines = vec![match &outcome.done {
        Done::Current { commit } => format!("{name}: at {}, its branch's head", short(commit)),
        Done::Moved {
            from: Some(from),
            to,
        } => format!("{name}: moved from {} to {}", short(from), short(to)),
        Done::Moved { from: None, to } => format!("{name}: installed at {}", short(to)),
        Done::Busy => format!("{name}: another update holds it, so this one left it"),
        Done::OnHold { commit, hold } => format!(
            "{name}: {} waits, since swaps are held {}: {}; `pi-packages release` ends the hold",
            short(commit),
            how_long(now, hold),
            hold.why
        ),
        Done::Held { commit, until } => format!(
            "{name}: {} waits {} more minutes, since installs swap in at most once an hour; \
             `pi-packages update --now` swaps it in now",
            short(commit),
            minutes_until(now, *until)
        ),
        Done::Failed { commit, why } => format!(
            "{name}: {} not swapped in, current left as it was: {why}",
            short(commit)
        ),
    }];
    if let Some(why) = &outcome.unfetched {
        lines.push(format!(
            "{name}: could not fetch, so used the last commit seen: {why}"
        ));
    }
    for why in &outcome.unwarmed {
        lines.push(format!(
            "{name}: a pi's cache was not warmed before the swap: {why}"
        ));
    }
    if outcome.removed > 0 {
        lines.push(format!(
            "{name}: removed {} old versions nothing uses",
            outcome.removed
        ));
    }
    lines
}

/// What an update did for one package, as facts.
fn outcome_fact(name: &str, outcome: &Outcome) -> Value {
    let (done, commit, from, why, waits_until, hold) = match &outcome.done {
        Done::Current { commit } => ("current", Some(commit), None, None, None, None),
        Done::Moved { from, to } => {
            let done = if from.is_some() { "moved" } else { "installed" };
            (done, Some(to), from.as_ref(), None, None, None)
        }
        Done::Busy => ("busy", None, None, None, None, None),
        Done::Held { commit, until } => (
            "spaced",
            Some(commit),
            None,
            None,
            Some(instant(*until)),
            None,
        ),
        Done::OnHold { commit, hold } => (
            "on_hold",
            Some(commit),
            None,
            None,
            None,
            Some(hold_fact(hold)),
        ),
        Done::Failed { commit, why } => ("failed", Some(commit), None, Some(why), None, None),
    };
    json!({
        "package": name,
        "done": done,
        "commit": commit,
        "from": from,
        "why": why,
        "waits_until": waits_until,
        "hold": hold,
        "unfetched": outcome.unfetched,
        "unwarmed": outcome.unwarmed,
        "removed_versions": outcome.removed,
    })
}

fn update(context: &Context, config: &Config, pace: Pace) -> Result<(), Failure> {
    let mut failed = Vec::new();
    let mut text = Vec::new();
    let mut lines = Vec::new();
    let now = SystemTime::now();
    for (package, outcome) in run::update(config, now, pace) {
        let name = name(&package);
        let fact = match outcome {
            Err(why) => {
                text.push(format!("{name}: not updated: {why}"));
                failed.push(name.clone());
                json!({ "package": name, "done": "not_updated", "why": why })
            }
            Ok(outcome) => {
                text.extend(outcome_lines(&name, &outcome, now));
                if matches!(outcome.done, Done::Failed { .. }) {
                    failed.push(name.clone());
                }
                outcome_fact(&name, &outcome)
            }
        };
        lines.push(("package", fact));
    }
    // One line each age-out, which the daily disk figure counts.
    let aged = run::age_jiti(config, now).map(|aged| {
        let days = run::JITI_KEEPS.as_secs() / (24 * 3600);
        text.push(format!(
            "jiti: removed {} files, {} bytes, older than {days} days",
            aged.files, aged.bytes
        ));
        json!({ "removed_files": aged.files, "removed_bytes": aged.bytes, "older_than_days": days })
    });
    let packages: Vec<Value> = lines.iter().map(|(_, fact)| fact.clone()).collect();
    if let Some(aged) = &aged {
        lines.push(("jiti", aged.clone()));
    }
    answer(
        context,
        &text,
        &json!({ "schema": "pi-packages.update/1", "packages": packages, "jiti_aged": aged }),
        &lines,
    )?;
    unless_failed(&failed, "not every package was updated", "not updated")
}

/// A failure naming each package that failed, if any did.
fn unless_failed(failed: &[String], what: &str, said: &str) -> Result<(), Failure> {
    if failed.is_empty() {
        Ok(())
    } else {
        Err(Failure::new(Exit::Failed, what)
            .because(format!("{said}: {}", failed.join(", ")))
            .try_this("pi-packages update"))
    }
}

fn sweep(context: &Context, config: &Config) -> Result<(), Failure> {
    let mut failed = Vec::new();
    let mut text = Vec::new();
    let mut lines = Vec::new();
    for (package, removed) in run::sweep(config) {
        let name = name(&package);
        let fact = match removed {
            Ok(Some(removed)) => {
                if removed > 0 {
                    text.push(format!(
                        "{name}: removed {removed} old versions nothing uses"
                    ));
                }
                json!({ "package": name, "done": "swept", "removed_versions": removed })
            }
            Ok(None) => {
                text.push(format!(
                    "{name}: an update holds it, and removes what it can"
                ));
                json!({ "package": name, "done": "busy", "removed_versions": null })
            }
            Err(why) => {
                text.push(format!("{name}: nothing removed: {why}"));
                failed.push(name.clone());
                json!({ "package": name, "done": "failed", "removed_versions": 0, "why": why })
            }
        };
        lines.push(("package", fact));
    }
    let packages: Vec<Value> = lines.iter().map(|(_, fact)| fact.clone()).collect();
    answer(
        context,
        &text,
        &json!({ "schema": "pi-packages.sweep/1", "packages": packages }),
        &lines,
    )?;
    unless_failed(&failed, "not every package was swept", "nothing removed")
}

fn status(context: &Context, config: &Config) -> Result<(), Failure> {
    let now = SystemTime::now();
    let standing = hold::standing(config, now);
    let mut text = Vec::new();
    let mut lines = Vec::new();
    if let Some(hold) = &standing {
        text.push(format!(
            "swaps are held {}: {}",
            how_long(now, hold),
            hold.why
        ));
        lines.push(("hold", hold_fact(hold)));
    }
    for package in &config.packages {
        let name = name(package);
        let current = Place::of(config, package).current_commit();
        text.push(match &current {
            Some(commit) => format!("{name}: current is {}", short(commit)),
            None => format!("{name}: nothing installed yet"),
        });
        lines.push(("package", json!({ "package": name, "current": current })));
    }
    let packages: Vec<Value> = lines
        .iter()
        .filter(|(kind, _)| *kind == "package")
        .map(|(_, fact)| fact.clone())
        .collect();
    answer(
        context,
        &text,
        &json!({
            "schema": "pi-packages.status/1",
            "hold": standing.as_ref().map(hold_fact),
            "packages": packages,
        }),
        &lines,
    )
}
