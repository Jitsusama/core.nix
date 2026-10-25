//! The load proof: whether what Joel uses still feels idle while the work
//! behind him fills every core and the line.
//!
//! It grows a signal at a time. Now it has one:
//!
//! - `frames`: whether niri shows a frame at every refresh (frames.rs).
//!
//! Usage: load-proof frames [SECONDS]

mod frames;

use std::process::ExitCode;
use std::time::Duration;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) != Some("frames") {
        eprintln!("usage: load-proof frames [SECONDS]");
        return ExitCode::from(2);
    }
    let secs = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(10);
    match frames::watch(Duration::from_secs(secs)) {
        Ok(shown) => {
            let t = frames::Tally::of(&shown);
            println!(
                "shown {} missed {} late {} discarded {} longest {:.1} ms refresh {:.2} ms",
                t.shown, t.missed, t.late, t.discarded, t.longest_ms, t.refresh_ms
            );
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("load-proof: {e}");
            ExitCode::FAILURE
        }
    }
}
