//! Each signal read for a while, as the line a reading keeps and as its
//! facts. The line is what `judge` and a run read back, so its words stay
//! as they are; the facts are the same figures, named with their units.

use crate::spread::Spread;
use crate::{call, frames, keys, save, sound, typed};
use serde_json::{Value, json};
use std::fmt::Write;
use std::thread;
use std::time::Duration;

/// The signals, in the order a reading prints them.
pub const SIGNALS: [&str; 6] = ["call", "sound", "frames", "keys", "save", "typed"];

/// One signal read: its line and its facts.
pub struct Read {
    pub line: String,
    pub facts: Value,
}

/// Reads `signal` for `length`.
pub fn read(signal: &str, length: Duration) -> Result<Read, String> {
    match signal {
        "call" => call::watch(call::SERVER, length).map(|calls| {
            let t = Spread::of(&calls.times);
            Read {
                line: format!(
                    "calls {} lost {} p50 {:.2} ms p95 {:.2} ms worst {:.2} ms",
                    t.count, calls.lost, t.p50_ms, t.p95_ms, t.worst_ms
                ),
                facts: json!({
                    "calls": t.count,
                    "lost": calls.lost,
                    "p50_ms": t.p50_ms,
                    "p95_ms": t.p95_ms,
                    "worst_ms": t.worst_ms,
                }),
            }
        }),
        "frames" => frames::watch(length).map(|shown| {
            let t = frames::Tally::of(&shown);
            Read {
                line: format!(
                    "shown {} missed {} late {} discarded {} longest {:.1} ms refresh {:.2} ms",
                    t.shown, t.missed, t.late, t.discarded, t.longest_ms, t.refresh_ms
                ),
                facts: json!({
                    "shown": t.shown,
                    "missed": t.missed,
                    "late": t.late,
                    "discarded": t.discarded,
                    "longest_ms": t.longest_ms,
                    "refresh_ms": t.refresh_ms,
                }),
            }
        }),
        "keys" => keys::watch(length).map(|echoes| timed("keys", "echo", &echoes)),
        "save" => {
            let cache = std::env::var_os("XDG_CACHE_HOME").map(Into::into);
            let home = std::env::var_os("HOME").map(Into::into);
            let folder =
                save::folder(cache, home).ok_or("no HOME or XDG_CACHE_HOME to save under")?;
            save::watch(&folder, length).map(|times| timed("saves", "took", &times))
        }
        "sound" => sound::watch(length).map(|rose| {
            let errors: u64 = rose.iter().map(|(_, count)| count).sum();
            let mut line = format!("sound errors {errors}");
            for (node, count) in &rose {
                let _ = write!(line, ", {node} {count}");
            }
            let nodes: Vec<Value> = rose
                .iter()
                .map(|(node, count)| json!({"node": node, "errors": count}))
                .collect();
            Read {
                line,
                facts: json!({"errors": errors, "nodes": nodes}),
            }
        }),
        "typed" => typed::watch(length).map(|keys| timed("typed", "frame", &keys)),
        _ => Err(format!("no signal {signal}")),
    }
}

/// A signal timed one by one, as `word` and how many, then what `took`
/// at the middle, for 99 in 100 and at worst.
fn timed(word: &str, took: &str, times: &[Duration]) -> Read {
    let t = Spread::of(times);
    Read {
        line: format!(
            "{word} {} {took} p50 {:.2} ms p99 {:.2} ms worst {:.2} ms",
            t.count, t.p50_ms, t.p99_ms, t.worst_ms
        ),
        facts: json!({
            word: t.count,
            "p50_ms": t.p50_ms,
            "p99_ms": t.p99_ms,
            "worst_ms": t.worst_ms,
        }),
    }
}

/// Every signal read at once for `length`, in order: each one's read, or
/// why it couldn't be.
pub fn all(length: Duration) -> Vec<(&'static str, Result<Read, String>)> {
    let readers = SIGNALS.map(|signal| thread::spawn(move || read(signal, length)));
    SIGNALS
        .into_iter()
        .zip(readers)
        .map(|(signal, reader)| {
            let read = reader
                .join()
                .unwrap_or_else(|_| Err("it panicked".to_owned()));
            (signal, read)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_timed_signals_line_is_the_one_judge_reads() {
        let times = [Duration::from_millis(2), Duration::from_millis(4)];
        let read = timed("keys", "echo", &times);
        assert_eq!(
            read.line,
            "keys 2 echo p50 2.00 ms p99 4.00 ms worst 4.00 ms"
        );
        assert_eq!(
            read.facts,
            json!({"keys": 2, "p50_ms": 2.0, "p99_ms": 4.0, "worst_ms": 4.0})
        );
    }
}
