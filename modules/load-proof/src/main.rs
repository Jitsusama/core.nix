//! The load proof: whether what Joel uses still feels idle while the work
//! behind him fills every core and the line.
//!
//! It has six signals, each read on its own or all at once (signals.rs):
//!
//! - `call`: how long a call's packets take to cross the line and back
//!   (call.rs).
//! - `frames`: whether niri shows a frame at every refresh (frames.rs).
//! - `keys`: how quickly a key typed in a terminal comes back (keys.rs).
//! - `save`: how quickly an app's careful save reaches the disk (save.rs).
//! - `sound`: whether any sound in PipeWire misses its turn (sound.rs).
//! - `typed`: how long Joel's own keys in the launcher take to reach the
//!   screen, from what the launcher keeps of them (typed.rs).
//!
//! `all` reads the six at once and prints a line each, which is a
//! reading; `judge` says whether a loaded reading felt as an idle one did
//! (judge.rs). `run` takes an idle reading, loads the machine in steps up
//! to twenty builds with the line full both ways, and judges a reading in
//! each step against idle (run.rs).
//!
//! It takes the shape every command of ours does (docs/commands.md): its
//! help, named exits, and `--json` for the same facts as the words. A
//! reading's lines are what `judge` and a run read back, so they keep
//! their words.

mod call;
mod frames;
mod judge;
mod keys;
mod run;
mod save;
mod signals;
mod sound;
mod spread;
mod typed;

use serde_json::{Value, json};
use signals::SIGNALS;
use std::ffi::OsStr;
use std::fmt::Write;
use std::fs;
use std::io;
use std::path::Path;
use std::process::ExitCode;
use std::time::Duration;
use terminal_kit::{Context, Exit, Failure, Format, Help, Room, flags, layout};

/// How long a signal is read for when it isn't told.
const LENGTH: Duration = Duration::from_secs(10);

/// What it says of itself, set as help and read by the check.
fn help() -> Help {
    Help::new(
        "load-proof",
        "says whether what Joel uses still feels idle under load",
    )
    .example(
        "load-proof keys 30",
        "how quickly keys echo, over 30 s, not 10",
    )
    .example(
        "load-proof all > idle",
        "a reading of all six signals, a line each",
    )
    .example(
        "load-proof judge idle busy",
        "whether the busy reading felt as the idle one did",
    )
    .example(
        "load-proof run",
        "load up to twenty builds, judging each step",
    )
    .example(
        "load-proof run --json",
        "the run's verdict, as load-proof.run/1",
    )
    .command("call", "a call's round trips to a STUN server")
    .command("frames", "whether niri shows a frame at every refresh")
    .command("keys", "how quickly a key typed in a terminal echoes")
    .command("save", "how quickly an app's careful save reaches disk")
    .command("sound", "whether any sound in PipeWire misses its turn")
    .command("typed", "how fast the launcher's keys reach the screen")
    .command("all", "all six at once, a line each: a reading")
    .command("judge", "whether a loaded reading felt as an idle one did")
    .command("run", "the whole proof, in under four minutes")
    .exit(Exit::Failed, "a loaded reading didn't feel as idle did")
    .exit(Exit::NotFound, "a reading to judge isn't there")
    .exit(
        Exit::Unavailable,
        "a signal couldn't be read, or the load couldn't run",
    )
}

fn main() -> ExitCode {
    let help = help();
    let commands = help.command_names();
    terminal_kit::main(&help, env!("CARGO_PKG_VERSION"), |context| {
        act(context, &commands)
    })
}

/// Does what the command line asks, of the `commands` help names.
fn act(context: &Context, commands: &[&str]) -> Result<(), Failure> {
    let Some((command, rest)) = context.args.split_first() else {
        return Err(Failure::new(
            Exit::Usage,
            "it needs a signal to read, or a reading to judge",
        )
        .try_this("load-proof all"));
    };
    match command.as_str() {
        // The keys signal runs this program again as the shell's
        // stand-in, and a run runs it again for each part of its load
        // in jobs.slice; none is for typing, so help leaves them out.
        keys::ECHO => match rest {
            [path] => keys::echo(OsStr::new(path)).map_err(|why| broke(keys::ECHO, why)),
            _ => Err(takes(keys::ECHO, "a pty's path")),
        },
        run::BUILD => match rest {
            [folder, length] => run::build(Path::new(folder), seconds(length)?)
                .map_err(|why| broke(run::BUILD, why)),
            _ => Err(takes(run::BUILD, "a folder and its seconds")),
        },
        run::LINE => match rest {
            [length] => run::line(seconds(length)?).map_err(|why| broke(run::LINE, why)),
            _ => Err(takes(run::LINE, "its seconds")),
        },
        "judge" => match rest {
            [idle, loaded] => judged(context, idle, loaded),
            _ => Err(takes("judge", "an idle reading and a loaded one")
                .try_this("load-proof judge idle busy")),
        },
        // A mistyped `run` would otherwise load the machine for
        // minutes, so it takes nothing.
        "run" => match rest {
            [] => ran(context),
            [word, ..] => Err(
                Failure::new(Exit::Usage, format!("run takes nothing, not {word}"))
                    .try_this("load-proof run"),
            ),
        },
        "all" => length(rest).and_then(|length| all(context, length)),
        signal if SIGNALS.contains(&signal) => {
            length(rest).and_then(|length| one(context, signal, length))
        }
        word => Err(flags::unknown("command", word, commands)),
    }
}

/// How long to read for: ten seconds, or the whole number given.
fn length(rest: &[String]) -> Result<Duration, Failure> {
    match rest {
        [] => Ok(LENGTH),
        [length] => seconds(length),
        [_, word, ..] => Err(
            Failure::new(Exit::Usage, format!("{word} isn't taken here"))
                .because("a signal takes only its seconds")
                .try_this("load-proof keys 30"),
        ),
    }
}

/// A length given as a whole number of seconds.
fn seconds(word: &str) -> Result<Duration, Failure> {
    word.parse().map(Duration::from_secs).map_err(|_| {
        Failure::new(
            Exit::Usage,
            format!("{word} isn't a whole number of seconds"),
        )
        .try_this("load-proof keys 30")
    })
}

fn takes(command: &str, what: &str) -> Failure {
    Failure::new(Exit::Usage, format!("{command} takes {what}"))
}

/// A part of a run's load that couldn't do its part.
fn broke(command: &str, why: String) -> Failure {
    Failure::new(Exit::Failed, format!("{command} couldn't do its part")).because(why)
}

/// The schema's name beside the facts.
fn answered(schema: &str, facts: Value) -> Value {
    let mut answer = json!({"schema": schema});
    if let (Some(answer), Value::Object(facts)) = (answer.as_object_mut(), facts) {
        answer.extend(facts);
    }
    answer
}

/// A line for a person, its words set after `lead` and wrapped under
/// themselves, so a reading or a failure never runs past the terminal.
/// Into a pipe it stays one line, as a saved reading is read back.
fn hung(lead: &str, words: &str, room: Room) -> String {
    let words = layout::hung(words, layout::width(lead), room.for_prose());
    format!("{lead}{words}\n")
}

/// A reading's line, after `inset`, hung after the word naming its signal.
fn reading_line(inset: &str, line: &str, room: Room) -> String {
    match line.split_once(' ') {
        Some((name, figures)) => hung(&format!("{inset}{name} "), figures, room),
        None => format!("{inset}{line}\n"),
    }
}

fn said(result: io::Result<()>) -> Result<(), Failure> {
    result.map_err(|e| {
        Failure::new(Exit::Failed, "it couldn't write its answer").because(e.to_string())
    })
}

/// Reads one signal for `length`, and says its line or its facts.
fn one(context: &Context, signal: &str, length: Duration) -> Result<(), Failure> {
    let read = signals::read(signal, length).map_err(|why| {
        Failure::new(Exit::Unavailable, format!("it couldn't read {signal}")).because(why)
    })?;
    if context.format().is_json() {
        said(context.answer(&answered(&format!("load-proof.{signal}/1"), read.facts)))
    } else {
        said(context.say(&reading_line("", &read.line, context.room)))
    }
}

/// Reads every signal at once for `length`, and says each one's line in
/// order, or their facts. One that couldn't be read is told of, and the
/// rest are still said.
fn all(context: &Context, length: Duration) -> Result<(), Failure> {
    let mut answer = json!({"schema": "load-proof.reading/1"});
    let mut lines = String::new();
    let mut unread = Vec::new();
    for (signal, read) in signals::all(length) {
        match read {
            Ok(read) => {
                lines.push_str(&reading_line("", &read.line, context.room));
                answer[signal] = read.facts;
            }
            Err(why) => {
                answer[signal] = Value::Null;
                unread.push((signal, why));
            }
        }
    }
    if context.format().is_json() {
        said(context.answer(&answer))?;
    } else {
        said(context.say(&lines))?;
    }
    if unread.is_empty() {
        return Ok(());
    }
    let names: Vec<&str> = unread.iter().map(|(signal, _)| *signal).collect();
    let why: Vec<String> = unread
        .iter()
        .map(|(signal, why)| format!("{signal}: {why}"))
        .collect();
    Err(Failure::new(
        Exit::Unavailable,
        format!("it couldn't read {}", flags::and(&names)),
    )
    .because(why.join("; ")))
}

/// Judges the reading in the file `loaded` against the one in `idle`,
/// saying what failed, or that it passed.
fn judged(context: &Context, idle: &str, loaded: &str) -> Result<(), Failure> {
    let contents = |path: &str| {
        fs::read_to_string(path).map_err(|e| {
            let exit = if e.kind() == io::ErrorKind::NotFound {
                Exit::NotFound
            } else {
                Exit::Failed
            };
            Failure::new(exit, format!("it couldn't read {path}"))
                .because(e.to_string())
                .try_this(format!("load-proof all > {path}"))
        })
    };
    let (idle, loaded) = (
        judge::read(&contents(idle)?),
        judge::read(&contents(loaded)?),
    );
    let failed = judge::judge(&idle, &loaded);
    if context.format().is_json() {
        let facts: Vec<Value> = failed.iter().map(judge::Failed::facts).collect();
        said(context.answer(&json!({
            "schema": "load-proof.judge/1",
            "passed": failed.is_empty(),
            "failed": facts,
        })))?;
    } else if failed.is_empty() {
        said(context.say("passes: everything felt as it does idle\n"))?;
    } else {
        let mut lines = String::new();
        for failure in &failed {
            lines.push_str(&hung("fails: ", &failure.to_string(), context.room));
        }
        said(context.say(&lines))?;
    }
    if failed.is_empty() {
        Ok(())
    } else {
        Err(Failure::new(
            Exit::Failed,
            "the loaded reading didn't feel as the idle one did",
        ))
    }
}

/// What a step found, as facts.
fn step_facts(step: &run::Step) -> Value {
    let failed: Vec<Value> = step.failed.iter().map(judge::Failed::facts).collect();
    json!({"builds": step.builds, "passed": failed.is_empty(), "failed": failed})
}

/// Says one of a run's readings as it comes: its lines, under what it was
/// taken in, for a person; a step's line for a stream; nothing for one
/// answer, which says it all at the end.
fn say_as_it_comes(context: &Context, said: run::Said<'_>) {
    let (what, reading, step) = match said {
        run::Said::Idle(reading) => ("idle".to_owned(), reading, None),
        run::Said::Step(step, reading) => (format!("{} builds", step.builds), reading, Some(step)),
    };
    match context.format() {
        Format::Jsonl => {
            if let Some(step) = step {
                let _ = context.line("step", &step_facts(step));
            }
        }
        format if format.is_json() => {}
        _ => {
            let mut lines = format!("{what}:\n");
            for line in reading.lines() {
                lines.push_str(&reading_line("  ", line, context.room));
            }
            for failure in step.map(|step| step.failed.as_slice()).unwrap_or_default() {
                lines.push_str(&hung("  fails: ", &failure.to_string(), context.room));
            }
            let _ = context.say(&lines);
        }
    }
}

/// Runs the load proof whole, saying each step's reading as it comes, and
/// passes only when every step felt as idle did.
fn ran(context: &Context) -> Result<(), Failure> {
    let unable = |why: String| {
        Failure::new(Exit::Unavailable, "it couldn't run the load proof").because(why)
    };
    let program =
        std::env::current_exe().map_err(|e| unable(format!("finding this program: {e}")))?;
    let cache = std::env::var_os("XDG_CACHE_HOME").map(Into::into);
    let home = std::env::var_os("HOME").map(Into::into);
    let folder = save::folder(cache, home)
        .ok_or_else(|| unable("no HOME or XDG_CACHE_HOME to build under".to_owned()))?;
    let plan = run::Plan::default();
    let json = context.format().is_json();
    if !json {
        said(context.say(&format!("a run takes {} s\n", plan.length().as_secs())))?;
    }
    let run = run::run(&plan, &program, &folder.join("run"), &mut |said| {
        say_as_it_comes(context, said);
    })
    .map_err(unable)?;
    if json {
        let steps: Vec<Value> = run.steps.iter().map(step_facts).collect();
        said(context.answer(&json!({
            "schema": "load-proof.run/1",
            "passed": run.passed(),
            "planned_seconds": plan.length().as_secs(),
            "steps": steps,
            "first_failed_builds": run.first_failed(),
            "stopped_under_free_bytes": run.stopped_under,
        })))?;
    } else if run.passed() {
        said(context.say("passes: everything felt as it does idle, up to twenty builds\n"))?;
    } else {
        let mut lines = String::new();
        if let Some(bytes) = run.stopped_under {
            let _ = writeln!(lines, "stopped: free memory fell under {} GiB", bytes >> 30);
        }
        match run.first_failed() {
            Some(builds) => {
                let _ = writeln!(lines, "fails: from {builds} builds on");
            }
            None => lines.push_str("fails: the load didn't run to the end\n"),
        }
        said(context.say(&lines))?;
    }
    if run.passed() {
        Ok(())
    } else {
        Err(Failure::new(
            Exit::Failed,
            "a loaded step didn't feel as idle did",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use terminal_kit::{Ending, Environment};

    /// Runs the command line `line` as load-proof would. Each case here
    /// is refused before anything is read or loaded.
    fn ran(line: &str) -> Ending {
        let help = help();
        let commands = help.command_names();
        let args = line.split_whitespace().map(str::to_owned).collect();
        terminal_kit::run(
            &help,
            "1.0",
            &Environment::of([false; 3], &[]),
            args,
            |context| act(context, &commands),
        )
    }

    const FRAMES: &str = "shown 3600 missed 0 late 0 discarded 0 longest 8.3 ms refresh 8.33 ms";

    #[test]
    fn a_reading_wraps_under_its_figures_in_a_narrow_terminal() {
        let said = reading_line("  ", FRAMES, Room::Cells(40));
        assert!(
            said.lines().all(|line| layout::width(line) <= 40),
            "a line ran past 40 cells:\n{said}"
        );
        assert!(
            said.lines().skip(1).all(|line| line.starts_with("        ")),
            "a wrapped line didn't sit under the figures:\n{said}"
        );
    }

    #[test]
    fn a_reading_stays_one_line_into_a_pipe() {
        assert_eq!(
            reading_line("", FRAMES, Room::Unbounded),
            format!("{FRAMES}\n"),
            "a reading saved through a pipe changed, so judge can't read it back"
        );
    }

    #[test]
    fn help_is_answered_before_a_run_loads_anything() {
        let ending = ran("run --help");
        assert_eq!(ending.exit, Exit::Done, "{}", ending.stderr);
        assert!(ending.stdout.contains("load-proof"), "{}", ending.stdout);
    }

    #[test]
    fn a_run_given_anything_is_refused_before_it_loads() {
        let ending = ran("run now");
        assert_eq!(ending.exit, Exit::Usage, "{}", ending.stderr);
    }

    #[test]
    fn each_command_takes_no_more_than_its_own() {
        for line in [
            "",
            "frames 30 40",
            "frames soon",
            "all 10 20",
            "judge idle",
            "judge idle busy again",
            "load-build folder",
            "load-line 30 40",
            "frams",
        ] {
            let ending = ran(line);
            assert_eq!(ending.exit, Exit::Usage, "{line}: {}", ending.stderr);
        }
    }

    #[test]
    fn a_reading_to_judge_that_isnt_there_is_not_found() {
        let missing = std::env::temp_dir().join(format!("load-proof-none-{}", std::process::id()));
        let missing = missing.display();
        let ending = ran(&format!("judge {missing} {missing}"));
        assert_eq!(ending.exit, Exit::NotFound, "{}", ending.stderr);
    }

    #[test]
    fn a_reading_is_answered_with_its_schema_beside_its_facts() {
        let answer = answered("load-proof.keys/1", json!({"keys": 2, "p99_ms": 4.0}));
        assert_eq!(
            answer,
            json!({"schema": "load-proof.keys/1", "keys": 2, "p99_ms": 4.0})
        );
    }

    #[test]
    fn a_step_says_which_figures_failed() {
        let step = run::Step {
            builds: 8,
            failed: vec![judge::Failed {
                signal: "save",
                missing: None,
                figures: vec![("p99_ms", "p99 40.0 ms against 9.0 ms idle".to_owned())],
            }],
        };
        assert_eq!(
            step_facts(&step),
            json!({
                "builds": 8,
                "passed": false,
                "failed": [{"signal": "save", "figures": ["p99_ms"], "missing": null}],
            })
        );
    }
}
