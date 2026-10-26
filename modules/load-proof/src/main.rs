//! The load proof: whether what Joel uses still feels idle while the work
//! behind him fills every core and the line.
//!
//! It grows a signal at a time. Now it has two:
//!
//! - `frames`: whether niri shows a frame at every refresh (frames.rs).
//! - `keys`: how quickly a key typed in a terminal comes back (keys.rs).
//!
//! Usage: load-proof frames|keys [SECONDS]

mod frames;
mod keys;

use std::ffi::OsString;
use std::process::ExitCode;
use std::time::Duration;

const USAGE: &str = "usage: load-proof frames|keys [SECONDS]";

fn main() -> ExitCode {
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();
    let signal = args.first().and_then(|arg| arg.to_str());
    // The keys signal runs this program again as the shell's stand-in.
    if signal == Some(keys::ECHO) {
        let Some(path) = args.get(1) else {
            eprintln!("usage: load-proof {} PATH", keys::ECHO);
            return ExitCode::from(2);
        };
        return report(keys::echo(path).map(|()| None));
    }
    let secs = args
        .get(1)
        .and_then(|arg| arg.to_str())
        .and_then(|arg| arg.parse().ok())
        .unwrap_or(10);
    let length = Duration::from_secs(secs);
    match signal {
        Some("frames") => report(frames::watch(length).map(|shown| {
            let t = frames::Tally::of(&shown);
            Some(format!(
                "shown {} missed {} late {} discarded {} longest {:.1} ms refresh {:.2} ms",
                t.shown, t.missed, t.late, t.discarded, t.longest_ms, t.refresh_ms
            ))
        })),
        Some("keys") => report(keys::watch(length).map(|echoes| {
            let t = keys::Tally::of(&echoes);
            Some(format!(
                "keys {} echo p50 {:.2} ms p99 {:.2} ms worst {:.2} ms",
                t.keys, t.p50_ms, t.p99_ms, t.worst_ms
            ))
        })),
        _ => {
            eprintln!("{USAGE}");
            ExitCode::from(2)
        }
    }
}

/// Prints what a signal found, or why it couldn't say.
fn report(found: Result<Option<String>, String>) -> ExitCode {
    match found {
        Ok(line) => {
            if let Some(line) = line {
                println!("{line}");
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("load-proof: {e}");
            ExitCode::FAILURE
        }
    }
}
