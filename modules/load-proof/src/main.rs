//! The load proof: whether what Joel uses still feels idle while the work
//! behind him fills every core and the line.
//!
//! It has five signals, each read on its own or all at once:
//!
//! - `call`: how long a call's packets take to cross the line and back
//!   (call.rs).
//! - `frames`: whether niri shows a frame at every refresh (frames.rs).
//! - `keys`: how quickly a key typed in a terminal comes back (keys.rs).
//! - `save`: how quickly an app's careful save reaches the disk (save.rs).
//! - `sound`: whether any sound in PipeWire misses its turn (sound.rs).
//!
//! `all` reads the five at once and prints a line each, which is a
//! reading; `judge` says whether a loaded reading felt as an idle one did
//! (judge.rs). `run` takes an idle reading, loads the machine in steps up
//! to twenty builds with the line full both ways, and judges a reading in
//! each step against idle (run.rs).
//!
//! Usage: load-proof call|frames|keys|save|sound|all [SECONDS]
//!        load-proof judge IDLE LOADED
//!        load-proof run

mod call;
mod frames;
mod judge;
mod keys;
mod run;
mod save;
mod sound;
mod spread;

use spread::Spread;

use std::ffi::OsString;
use std::fmt::Write;
use std::process::ExitCode;
use std::thread;
use std::time::Duration;

const USAGE: &str = "usage: load-proof call|frames|keys|save|sound|all [SECONDS]
       load-proof judge IDLE LOADED
       load-proof run";

/// The signals, in the order a reading prints them.
const SIGNALS: [&str; 5] = ["call", "sound", "frames", "keys", "save"];

fn main() -> ExitCode {
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();
    let command = args.first().and_then(|arg| arg.to_str());
    // The keys signal runs this program again as the shell's stand-in.
    if command == Some(keys::ECHO) {
        let Some(path) = args.get(1) else {
            eprintln!("usage: load-proof {} PATH", keys::ECHO);
            return ExitCode::from(2);
        };
        return match keys::echo(path) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => failed(&e),
        };
    }
    if command == Some("judge") {
        let (Some(idle), Some(loaded)) = (args.get(1), args.get(2)) else {
            eprintln!("{USAGE}");
            return ExitCode::from(2);
        };
        return judged(idle, loaded);
    }
    if command == Some("run") {
        return ran();
    }
    // A run's load runs this program again in jobs.slice.
    if command == Some(run::BUILD) {
        let (Some(folder), Some(length)) = (args.get(1), seconds(args.get(2))) else {
            eprintln!("usage: load-proof {} FOLDER SECONDS", run::BUILD);
            return ExitCode::from(2);
        };
        return done(run::build(folder.as_ref(), length));
    }
    if command == Some(run::LINE) {
        let Some(length) = seconds(args.get(1)) else {
            eprintln!("usage: load-proof {} SECONDS", run::LINE);
            return ExitCode::from(2);
        };
        return done(run::line(length));
    }
    let length = seconds(args.get(1)).unwrap_or(Duration::from_secs(10));
    match command {
        Some("all") => all(length),
        Some(signal) if SIGNALS.contains(&signal) => match read(signal, length) {
            Ok(line) => {
                println!("{line}");
                ExitCode::SUCCESS
            }
            Err(e) => failed(&e),
        },
        _ => {
            eprintln!("{USAGE}");
            ExitCode::from(2)
        }
    }
}

/// Reads every signal at once for `length`, and prints each one's line in
/// order. One that couldn't be read is told of, and the rest still print.
fn all(length: Duration) -> ExitCode {
    let reading: Vec<_> = SIGNALS
        .map(|signal| thread::spawn(move || read(signal, length)))
        .into_iter()
        .map(|reader| {
            reader
                .join()
                .unwrap_or_else(|_| Err("it panicked".to_owned()))
        })
        .collect();
    let mut code = ExitCode::SUCCESS;
    for (signal, line) in SIGNALS.iter().zip(reading) {
        match line {
            Ok(line) => println!("{line}"),
            Err(e) => {
                eprintln!("load-proof: {signal}: {e}");
                code = ExitCode::FAILURE;
            }
        }
    }
    code
}

/// Reads `signal` for `length`, as the line it prints.
fn read(signal: &str, length: Duration) -> Result<String, String> {
    match signal {
        "call" => call::watch(call::SERVER, length).map(|calls| {
            let t = Spread::of(&calls.times);
            format!(
                "calls {} lost {} p50 {:.2} ms p95 {:.2} ms worst {:.2} ms",
                t.count, calls.lost, t.p50_ms, t.p95_ms, t.worst_ms
            )
        }),
        "frames" => frames::watch(length).map(|shown| {
            let t = frames::Tally::of(&shown);
            format!(
                "shown {} missed {} late {} discarded {} longest {:.1} ms refresh {:.2} ms",
                t.shown, t.missed, t.late, t.discarded, t.longest_ms, t.refresh_ms
            )
        }),
        "keys" => keys::watch(length).map(|echoes| {
            let t = Spread::of(&echoes);
            format!(
                "keys {} echo p50 {:.2} ms p99 {:.2} ms worst {:.2} ms",
                t.count, t.p50_ms, t.p99_ms, t.worst_ms
            )
        }),
        "save" => {
            let cache = std::env::var_os("XDG_CACHE_HOME").map(Into::into);
            let home = std::env::var_os("HOME").map(Into::into);
            let folder =
                save::folder(cache, home).ok_or("no HOME or XDG_CACHE_HOME to save under")?;
            save::watch(&folder, length).map(|times| {
                let t = Spread::of(&times);
                format!(
                    "saves {} took p50 {:.2} ms p99 {:.2} ms worst {:.2} ms",
                    t.count, t.p50_ms, t.p99_ms, t.worst_ms
                )
            })
        }
        "sound" => sound::watch(length).map(|rose| {
            let errors: u64 = rose.iter().map(|(_, count)| count).sum();
            let mut line = format!("sound errors {errors}");
            for (node, count) in &rose {
                let _ = write!(line, ", {node} {count}");
            }
            line
        }),
        _ => Err(format!("no signal {signal}")),
    }
}

/// Judges the reading in the file `loaded` against the one in `idle`,
/// printing what failed, or that it passed.
fn judged(idle: &OsString, loaded: &OsString) -> ExitCode {
    let contents = |path: &OsString| {
        std::fs::read_to_string(path).map_err(|e| format!("reading {}: {e}", path.display()))
    };
    let (idle, loaded) = match (contents(idle), contents(loaded)) {
        (Ok(idle), Ok(loaded)) => (judge::read(&idle), judge::read(&loaded)),
        (Err(e), _) | (_, Err(e)) => return failed(&e),
    };
    let failures = judge::judge(&idle, &loaded);
    if failures.is_empty() {
        println!("passes: everything felt as it does idle");
        return ExitCode::SUCCESS;
    }
    for failure in &failures {
        println!("fails: {failure}");
    }
    ExitCode::FAILURE
}

/// A length given as a whole number of seconds.
fn seconds(arg: Option<&OsString>) -> Option<Duration> {
    let secs = arg?.to_str()?.parse().ok()?;
    Some(Duration::from_secs(secs))
}

/// Runs the load proof whole, saying each step's reading as it comes, and
/// passes only when every step felt as idle did.
fn ran() -> ExitCode {
    let program = match std::env::current_exe() {
        Ok(program) => program,
        Err(e) => return failed(&format!("finding this program: {e}")),
    };
    let cache = std::env::var_os("XDG_CACHE_HOME").map(Into::into);
    let home = std::env::var_os("HOME").map(Into::into);
    let Some(folder) = save::folder(cache, home) else {
        return failed("no HOME or XDG_CACHE_HOME to build under");
    };
    let plan = run::Plan::default();
    println!("a run takes {} s", plan.length().as_secs());
    match run::run(&plan, &program, &folder.join("run"), &mut std::io::stdout()) {
        Ok(run) if run.passed() => {
            println!("passes: everything felt as it does idle, up to twenty builds");
            ExitCode::SUCCESS
        }
        Ok(run) => {
            if let Some(why) = &run.stopped {
                println!("stopped: {why}");
            }
            match run.first_failed() {
                Some(builds) => println!("fails: from {builds} builds on"),
                None => println!("fails: the load didn't run to the end"),
            }
            ExitCode::FAILURE
        }
        Err(e) => failed(&e),
    }
}

fn done(result: Result<(), String>) -> ExitCode {
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => failed(&e),
    }
}

/// Says why the load proof couldn't do what it was asked.
fn failed(why: &str) -> ExitCode {
    eprintln!("load-proof: {why}");
    ExitCode::FAILURE
}
