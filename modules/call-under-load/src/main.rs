//! Whether Joel's calls and apps stay quick while the machine is under
//! everything it carries, measured each hour.
//!
//! Joel asked to run insanely high quality video while the machine is
//! under all its pressures, with Chrome and Slack staying responsive. A
//! check that runs only when someone remembers finds a slide a day late,
//! so each hour `call-under-load run` makes a test call shaped as Meet's
//! between two Chromiums on this machine, on Chromium's fake camera, never a
//! real one, and clicks the call's tab every five seconds. It writes what
//! the call did, with the hour's Chrome and Slack drawing waits from
//! lag-watch's history and the launcher's openings from the journal, to
//! `$XDG_STATE_HOME/apps-under-load/hour.json`, which cadence judges.
//!
//! It keeps out of Joel's calls: it asks `a-call-is-on --here`, which hears
//! a call on this machine by its microphone, before it starts and every five seconds after,
//! and the calendar, keeping ten minutes clear of any meeting there. A run
//! stopped or skipped writes nothing.
//!
//! `call-under-load`, or `call-under-load now`, says the last hour's
//! figures, in the form `--format` asks for (docs/commands.md).

mod call;
mod cdp;
mod figures;
mod gate;
mod history;
mod hour;
mod managed;

use serde_json::{Value, json};
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::time::{SystemTime, UNIX_EPOCH};
use terminal_kit::{Context, Exit, Failure, Format, Help, flags};

/// How long the call runs: 140 samples past its first ten seconds, read
/// as about 135 overlapping five-second windows, and short enough to
/// leave the hour to Joel.
const SECONDS: u64 = 150;

/// The longest a run may take, its unit's start timeout: a meeting must be
/// ten minutes clear of all of it, since starting Chrome under load can
/// take minutes before the call's own 150 s.
const LONGEST: u64 = 10 * 60;

fn env_dir(var: &str, fallback: &str) -> PathBuf {
    std::env::var_os(var).map_or_else(
        || PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(fallback),
        PathBuf::from,
    )
}

fn state() -> PathBuf {
    env_dir("XDG_STATE_HOME", ".local/state")
}

fn folder() -> PathBuf {
    state().join("apps-under-load")
}

/// What it says of itself, set as help and read by the check.
fn help() -> Help {
    Help::new("call-under-load", "calls and apps under load, each hour")
        .example("call-under-load", "the last hour's figures")
        .example("call-under-load --json", "the same, for a script")
        .subcommand(
            Help::new("now", "the last hour's figures, as a bare call-under-load")
                .example("call-under-load now", "the last hour's figures")
                .exit(Exit::NotFound, "no hour has been measured yet")
                .exit(Exit::BadInput, "the hour can't be read"),
        )
        .subcommand(
            Help::new("run", "the hour's test call, as its timer runs it")
                .example(
                    "call-under-load run",
                    "measures the hour, unless Joel is in a call",
                )
                .exit(Exit::Done, "measured, or skipped for a call or a meeting")
                .exit(
                    Exit::Unavailable,
                    "Chromium, the calendar or the call check isn't there",
                )
                .exit(
                    Exit::Refused,
                    "a test browser enrolled, or Chromium's policy could enrol one",
                )
                .exit(Exit::Failed, "the call couldn't be made"),
        )
        .exit(Exit::NotFound, "no hour has been measured yet")
        .exit(Exit::BadInput, "the hour can't be read")
}

fn main() -> ExitCode {
    let help = help();
    let commands = help.command_names();
    terminal_kit::main(&help, env!("CARGO_PKG_VERSION"), |context| {
        let (command, rest) = match context.args.split_first() {
            Some((command, rest)) => (command.as_str(), rest),
            None => ("now", &[][..]),
        };
        match command {
            "now" => none_after(rest).and_then(|()| now(context)),
            "run" => none_after(rest).and_then(|()| run(context)),
            word => Err(flags::unknown("command", word, &commands)),
        }
    })
}

/// Nothing more on the line; `--verbose` is the kit's.
fn none_after(rest: &[String]) -> Result<(), Failure> {
    match rest
        .iter()
        .find(|arg| !matches!(arg.as_str(), "--verbose" | "-v"))
    {
        Some(arg) => Err(Failure::new(Exit::Usage, format!("{arg} isn't taken here"))
            .because("now and run take nothing more")
            .try_this("call-under-load --help")),
        None => Ok(()),
    }
}

fn unwritten(error: &std::io::Error) -> Failure {
    Failure::new(Exit::Failed, "couldn't write the answer").because(error.to_string())
}

fn seconds_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// The last hour's figures.
fn now(context: &Context) -> Result<(), Failure> {
    let path = folder().join("hour.json");
    let text = fs::read_to_string(&path).map_err(|error| {
        Failure::new(Exit::NotFound, "no hour has been measured yet")
            .because(format!("{} can't be read: {error}", path.display()))
            .try_this("systemctl --user start call-under-load")
    })?;
    let hour: Value = serde_json::from_str(&text).map_err(|error| {
        Failure::new(Exit::BadInput, "the hour can't be read")
            .because(format!("{}: {error}", path.display()))
            .try_this("systemctl --user start call-under-load")
    })?;
    let age = seconds_now().saturating_sub(hour["at"].as_u64().unwrap_or(0));
    match context.format() {
        Format::Text => context.say(&said(&hour, age)),
        _ => context.answer(&json!({
            "schema": "call-under-load.hour/1",
            "at": hour["at"],
            "age_s": age,
            "measures": hour["measures"],
        })),
    }
    .map_err(|error| unwritten(&error))
}

/// The hour as lines for a person: each figure against its target.
fn said(hour: &Value, age: u64) -> String {
    let mut text = format!("measured {} min ago\n", age / 60);
    for m in hour["measures"].as_array().into_iter().flatten() {
        let now = m["now"]
            .as_f64()
            .map_or_else(|| "unread".to_string(), |x| format!("{x}"));
        let target = m["target"].as_f64().unwrap_or(0.0);
        let way = if m["lower_is_better"] == true {
            "at most"
        } else {
            "at least"
        };
        // Writing to a String can't fail.
        let _ = writeln!(
            text,
            "{}: {now}, {way} {target}",
            m["measure"].as_str().unwrap_or("?")
        );
    }
    text
}

/// Refuses the call where a test browser could join anyone's management
/// (`managed`): once one has enrolled, until someone has read what it was
/// given, or where Chromium's own machine policy could enrol it.
fn unmanaged() -> Result<(), Failure> {
    if let Some(why) = managed::stays_off(&state()) {
        let kept = managed::kept(&state());
        return Err(Failure::new(Exit::Refused, "the test call stays off")
            .because(why.trim_end())
            .try_this(format!("ls -R {0}; rm -r {0}", kept.display())));
    }
    if let Some(why) = managed::could_enrol(Path::new(managed::POLICIES)) {
        return Err(
            Failure::new(Exit::Refused, "the test call could enrol its browsers")
                .because(why)
                .try_this(format!("ls -R {}", managed::POLICIES)),
        );
    }
    Ok(())
}

/// The hour's test call, and the figures written for cadence.
fn run(context: &Context) -> Result<(), Failure> {
    unmanaged()?;
    let skip = |why: String| {
        context
            .say(&format!("no test call this hour: {why}\n"))
            .map_err(|error| unwritten(&error))
    };
    match gate::a_call() {
        gate::Call::Off => {}
        gate::Call::On => return skip("Joel is in a call".into()),
        gate::Call::Unknown(why) => {
            return Err(
                Failure::new(Exit::Unavailable, "whether Joel is in a call can't be told")
                    .because(why)
                    .try_this("a-call-is-on --here"),
            );
        }
    }
    let runtime = env_dir("XDG_RUNTIME_DIR", ".cache");
    let socket = runtime.join("daemons/google.sock");
    let calendar = gate::ask(&socket).and_then(|answer| gate::calendar(&answer));
    let calendar = calendar.map_err(|why| {
        Failure::new(Exit::Unavailable, "the calendar can't be read")
            .because(why)
            .try_this("systemctl --user status google")
    })?;
    let started = seconds_now();
    if let Some(why) = gate::kept_clear(&calendar, started * 1000, LONGEST * 1000) {
        return skip(why);
    }
    // Chromium, never Google Chrome, which a managed machine enrols with
    // its owner's management at every fresh profile (package.nix).
    let chrome = on_path("chromium").ok_or_else(|| {
        Failure::new(Exit::Unavailable, "there's no Chromium to call with")
            .because("chromium isn't on the program's PATH")
            .try_this("nix build .#call-under-load")
    })?;
    let page = std::env::current_exe()
        .ok()
        .and_then(|exe| page_beside(&exe))
        .filter(|page| page.exists())
        .ok_or_else(|| {
            Failure::new(Exit::Failed, "the call's page isn't installed")
                .because("share/call-under-load/page.html isn't beside the program")
                .try_this("nix build .#call-under-load")
        })?;
    let cache = env_dir("XDG_CACHE_HOME", ".cache").join("call-under-load");
    let ended = call::run(&chrome, &page, SECONDS, &runtime, &cache, &state()).map_err(|why| {
        Failure::new(Exit::Failed, "the test call couldn't be made")
            .because(why)
            .try_this("journalctl --user -u call-under-load")
    })?;
    let (mine, far, clicks) = match ended {
        call::Ended::Stopped(why) => return skip(why),
        call::Ended::Ran { mine, far, clicks } => (mine, far, clicks),
    };
    let call = figures::call(&mine, &far, &clicks);
    let at = seconds_now();
    let watched = state().join("lag-watch");
    let chrome_waits = history::waits(&watched, "chrome-ui", at);
    let slack_waits = history::waits(&watched, "slack-ui", at);
    let measures = hour::measures(
        &call,
        call::STAND_IN,
        &chrome_waits,
        &slack_waits,
        &launcher_opens(),
    );
    let run = json!({"started": started, "seconds": SECONDS, "call": call});
    hour::write(&folder(), at, &measures, &run).map_err(|error| {
        Failure::new(Exit::Failed, "the hour couldn't be written")
            .because(error.to_string())
            .try_this(format!("ls -ld {}", folder().display()))
    })?;
    context
        .say(&said(&json!({"measures": measures}), 0))
        .map_err(|error| unwritten(&error))
}

/// The first program named `name` on PATH.
fn on_path(name: &str) -> Option<PathBuf> {
    std::env::split_paths(&std::env::var_os("PATH")?)
        .map(|dir| dir.join(name))
        .find(|path| path.is_file())
}

/// How long the launcher took to open over the last hour, in ms, each
/// from the line it writes to the shell's journal as it's on screen; read
/// from the shell's unit alone, as cadence reads it, so no other program's
/// line passes for one.
fn launcher_opens() -> Vec<f64> {
    Command::new("journalctl")
        .args(["--user", "-u", "quickshell", "--since", "-1h"])
        .args(["--no-pager", "-o", "cat"])
        .args(["--grep", "launcher: opening presented"])
        .output()
        .map(|out| opens(&String::from_utf8_lossy(&out.stdout)))
        .unwrap_or_default()
}

/// The openings' times in `lines`, each `... opening presented N ms ...`.
fn opens(lines: &str) -> Vec<f64> {
    lines
        .lines()
        .filter_map(|line| {
            let after = line.split_once("opening presented ")?.1;
            after
                .strip_suffix(" ms")
                .unwrap_or(after)
                .split(' ')
                .next()?
                .parse()
                .ok()
        })
        .collect()
}

/// Where the call's page is installed beside the program at `exe`.
fn page_beside(exe: &Path) -> Option<PathBuf> {
    Some(
        exe.parent()?
            .parent()?
            .join("share/call-under-load/page.html"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_launcher_s_openings_are_read_from_its_journal_lines() {
        let lines = "  INFO qml: launcher: opening presented 41 ms after the chord\n\
                     something else\n\
                     launcher: opening presented 7 ms\n";
        assert_eq!(opens(lines), [41.0, 7.0]);
    }

    #[test]
    fn the_hour_is_said_figure_by_figure_against_its_target() {
        let hour = json!({"measures": [
            {"measure": "call sent fps p10", "now": 29.8, "target": 30.0, "lower_is_better": false},
            {"measure": "launcher open ms p99", "now": null, "target": 50.0, "lower_is_better": true},
        ]});
        assert_eq!(
            said(&hour, 125),
            "measured 2 min ago\ncall sent fps p10: 29.8, at least 30\n\
             launcher open ms p99: unread, at most 50\n"
        );
    }

    #[test]
    fn the_page_is_looked_for_in_the_package_s_share() {
        assert_eq!(
            page_beside(Path::new("/nix/store/x/bin/call-under-load")),
            Some(PathBuf::from(
                "/nix/store/x/share/call-under-load/page.html"
            ))
        );
    }
}
