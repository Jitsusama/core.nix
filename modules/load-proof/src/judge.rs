//! The verdict: whether what Joel uses felt as it does idle while the
//! machine was loaded, read from what the signals printed in each.
//!
//! Each signal prints one line of words and figures, such as `keys 1000
//! echo p50 0.03 ms p99 0.64 ms worst 1.67 ms`, so a reading is kept as
//! the lines themselves, and judged by reading them back. The thresholds
//! are the load proof's (PLAN-20261025-H5CHVF):
//!
//! - a call's p95 round trip within 10 ms of idle's, and none lost that
//!   idle didn't lose; and whatever idle did, a p95 no longer than 300 ms
//!   and no more than 1% lost, since an idle that was poor itself proves
//!   nothing about a call;
//! - no sound that misses a turn;
//! - niri missing no more frames than idle;
//! - a key's echo p99 within 2 ms of idle's, and none over 16 ms;
//! - an app's save p99 within 20 ms of idle's;
//! - where Joel typed in the launcher, his keys reaching their frame in
//!   50 ms at the middle and 100 ms for 99 in 100, the launcher's own
//!   budget, whatever idle did, since he may type in one and not the
//!   other; a step he typed nothing in isn't judged for it.

use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::fmt;

/// A call's p95 round trip may grow this much under load, in ms.
const CALL_MARGIN: f64 = 10.0;
/// The longest p95 round trip a call bears, in ms: ITU-T G.114 accepts a
/// one-way delay of 150 ms for nearly every call.
const CALL_WORST: f64 = 300.0;
/// The most of a call's packets that may go missing, as a share: about
/// where a voice codec stops sounding clear.
const CALL_LOSS: f64 = 0.01;
/// A key's p99 echo may grow this much under load, in ms.
const KEYS_MARGIN: f64 = 2.0;
/// No key's echo may take longer than a frame at 60 Hz, in ms.
const KEYS_WORST: f64 = 16.0;
/// A save's p99 may grow this much under load, in ms.
const SAVE_MARGIN: f64 = 20.0;
/// The launcher's budget from a key's press to its frame, in ms: at the
/// middle, and for 99 keys in 100.
const TYPED_MIDDLE: f64 = 50.0;
const TYPED_MOST: f64 = 100.0;

/// The signals, by the word each line opens with, and what to call each.
const SIGNALS: [(&str, &str); 5] = [
    ("calls", "call"),
    ("sound", "sound"),
    ("shown", "frames"),
    ("keys", "keys"),
    ("saves", "save"),
];

/// A reading: each signal's figures by name, under the word its line opens
/// with.
pub type Reading = BTreeMap<String, BTreeMap<String, f64>>;

/// Reads what the signals printed. A word followed by a figure names it;
/// units and other words are passed over.
pub fn read(lines: &str) -> Reading {
    let mut reading = Reading::new();
    for line in lines.lines() {
        let words: Vec<&str> = line
            .split_whitespace()
            .map(|word| word.trim_end_matches(','))
            .collect();
        let Some(signal) = words.first() else {
            continue;
        };
        let figures = words
            .windows(2)
            .filter_map(|pair| Some((pair[0].to_owned(), pair[1].parse::<f64>().ok()?)))
            .collect();
        reading.insert((*signal).to_owned(), figures);
    }
    reading
}

/// A signal that didn't feel as it does idle: each figure of it that
/// failed, or the reading that had no line of it.
#[derive(Debug, Clone, PartialEq)]
pub struct Failed {
    /// The signal, as its command is named.
    pub signal: &'static str,
    /// The reading with no line of the signal: `idle` or `loaded`.
    pub missing: Option<&'static str>,
    /// Each figure that failed, by the name its JSON gives it, and in
    /// words.
    pub figures: Vec<(&'static str, String)>,
}

impl Failed {
    fn of(signal: &'static str) -> Failed {
        Failed {
            signal,
            missing: None,
            figures: Vec::new(),
        }
    }

    fn says(&mut self, figure: &'static str, words: String) {
        self.figures.push((figure, words));
    }

    /// What failed, as facts: the signal, the figures by name, and the
    /// reading that had none of it.
    pub fn facts(&self) -> Value {
        let figures: Vec<&str> = self.figures.iter().map(|(figure, _)| *figure).collect();
        json!({"signal": self.signal, "figures": figures, "missing": self.missing})
    }
}

impl fmt::Display for Failed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(missing) = self.missing {
            return write!(f, "{}: no reading {missing}", self.signal);
        }
        let words: Vec<&str> = self
            .figures
            .iter()
            .map(|(_, words)| words.as_str())
            .collect();
        write!(f, "{}: {}", self.signal, words.join("; "))
    }
}

/// What failed, a signal each; nothing when everything passed.
pub fn judge(idle: &Reading, loaded: &Reading) -> Vec<Failed> {
    let mut failed = Vec::new();
    for (word, name) in SIGNALS {
        let (Some(idle), Some(loaded)) = (idle.get(word), loaded.get(word)) else {
            let missing = if idle.contains_key(word) {
                "loaded"
            } else {
                "idle"
            };
            failed.push(Failed {
                missing: Some(missing),
                ..Failed::of(name)
            });
            continue;
        };
        let figure = |figures: &BTreeMap<String, f64>, key: &str| {
            figures.get(key).copied().unwrap_or(f64::NAN)
        };
        let mut says = Failed::of(name);
        match word {
            "calls" => {
                let (was, now) = (figure(idle, "p95"), figure(loaded, "p95"));
                // Each figure is said once: against idle, or else against
                // what a call bears, which a poor idle may have passed.
                if !within(now, was + CALL_MARGIN) {
                    says.says(
                        "p95_ms",
                        format!("p95 {now:.1} ms against {was:.1} ms idle"),
                    );
                } else if !within(now, CALL_WORST) {
                    says.says("p95_ms", format!("p95 {now:.1} ms, past {CALL_WORST} ms"));
                }
                let (was, now) = (figure(idle, "lost"), figure(loaded, "lost"));
                let calls = figure(loaded, "calls");
                if !within(now, was) {
                    says.says("lost", format!("{now} lost against {was} idle"));
                } else if !within(now, calls * CALL_LOSS) {
                    let share = CALL_LOSS * 100.0;
                    says.says("lost", format!("{now} lost of {calls}, past {share}%"));
                }
            }
            "sound" => {
                let now = figure(loaded, "errors");
                if !within(now, 0.0) {
                    says.says("errors", format!("{now} missed turns"));
                }
            }
            "shown" => {
                let (was, now) = (figure(idle, "missed"), figure(loaded, "missed"));
                if !within(now, was) {
                    says.says("missed", format!("{now} frames missed against {was} idle"));
                }
            }
            "keys" => {
                let (was, now) = (figure(idle, "p99"), figure(loaded, "p99"));
                if !within(now, was + KEYS_MARGIN) {
                    says.says(
                        "p99_ms",
                        format!("p99 {now:.2} ms against {was:.2} ms idle"),
                    );
                }
                let worst = figure(loaded, "worst");
                if !within(worst, KEYS_WORST) {
                    says.says("worst_ms", format!("worst {worst:.1} ms"));
                }
            }
            _ => {
                let (was, now) = (figure(idle, "p99"), figure(loaded, "p99"));
                if !within(now, was + SAVE_MARGIN) {
                    says.says(
                        "p99_ms",
                        format!("p99 {now:.1} ms against {was:.1} ms idle"),
                    );
                }
            }
        }
        if !says.figures.is_empty() {
            failed.push(says);
        }
    }
    if let Some(typed) = loaded
        .get("typed")
        .filter(|typed| typed.get("typed") > Some(&0.0))
    {
        let figure = |key: &str| typed.get(key).copied().unwrap_or(f64::NAN);
        let mut says = Failed::of("typed");
        let (middle, most) = (figure("p50"), figure("p99"));
        if !within(middle, TYPED_MIDDLE) {
            says.says(
                "p50_ms",
                format!("p50 {middle:.1} ms, past {TYPED_MIDDLE} ms"),
            );
        }
        if !within(most, TYPED_MOST) {
            says.says("p99_ms", format!("p99 {most:.1} ms, past {TYPED_MOST} ms"));
        }
        if !says.figures.is_empty() {
            failed.push(says);
        }
    }
    failed
}

/// Whether `figure` is no more than `limit`. A figure that went missing is
/// read as not a number, which is within nothing, so it fails.
fn within(figure: f64, limit: f64) -> bool {
    figure <= limit
}

#[cfg(test)]
mod tests {
    use super::*;

    const IDLE: &str = "\
calls 1500 lost 0 p50 20.50 ms p95 23.00 ms worst 29.00 ms
sound errors 0
shown 3600 missed 0 late 0 discarded 0 longest 8.3 ms refresh 8.33 ms
keys 1000 echo p50 0.03 ms p99 0.60 ms worst 1.20 ms
saves 120 took p50 4.00 ms p99 9.00 ms worst 12.00 ms
";

    /// What failed, in the words each failure is said in.
    fn judge(idle: &Reading, loaded: &Reading) -> Vec<String> {
        super::judge(idle, loaded)
            .iter()
            .map(ToString::to_string)
            .collect()
    }

    fn loaded(change: &[(&str, &str)]) -> Reading {
        let mut lines = IDLE.to_owned();
        for (from, to) in change {
            lines = lines.replace(from, to);
        }
        read(&lines)
    }

    #[test]
    fn a_line_is_read_as_its_words_figures() {
        let reading = read("calls 1500 lost 2 p50 20.50 ms p95 23.00 ms worst 29.00 ms\n");
        let calls = &reading["calls"];
        assert!((calls["calls"] - 1500.0).abs() < 1e-9, "{calls:?}");
        assert!((calls["lost"] - 2.0).abs() < 1e-9, "{calls:?}");
        assert!((calls["p95"] - 23.0).abs() < 1e-9, "{calls:?}");
        assert!(!calls.contains_key("ms"), "{calls:?}");
    }

    #[test]
    fn the_nodes_after_a_sounds_errors_dont_hide_them() {
        let reading = read("sound errors 2, speaker_sink 1, speaker_tuning 1\n");
        assert!(
            (reading["sound"]["errors"] - 2.0).abs() < 1e-9,
            "{reading:?}"
        );
    }

    #[test]
    fn a_load_that_felt_idle_passes() {
        let idle = read(IDLE);
        assert_eq!(judge(&idle, &idle), Vec::<String>::new());
    }

    #[test]
    fn each_signal_within_its_margin_passes() {
        let loaded = loaded(&[
            ("p95 23.00", "p95 33.00"),
            ("p99 0.60", "p99 2.50"),
            ("p99 9.00", "p99 29.00"),
        ]);
        assert_eq!(judge(&read(IDLE), &loaded), Vec::<String>::new());
    }

    #[test]
    fn each_signal_past_its_margin_fails_with_its_figures() {
        let loaded = loaded(&[
            ("p95 23.00", "p95 33.50"),
            ("lost 0", "lost 3"),
            ("errors 0", "errors 2"),
            ("missed 0", "missed 4"),
            ("p99 0.60", "p99 2.70"),
            ("worst 1.20", "worst 17.00"),
            ("p99 9.00", "p99 288.00"),
        ]);
        assert_eq!(
            judge(&read(IDLE), &loaded),
            [
                "call: p95 33.5 ms against 23.0 ms idle; 3 lost against 0 idle",
                "sound: 2 missed turns",
                "frames: 4 frames missed against 0 idle",
                "keys: p99 2.70 ms against 0.60 ms idle; worst 17.0 ms",
                "save: p99 288.0 ms against 9.0 ms idle",
            ]
        );
    }

    #[test]
    fn what_failed_names_its_figures_as_its_json_does() {
        let loaded = loaded(&[("p99 0.60", "p99 2.70"), ("worst 1.20", "worst 17.00")]);
        let without_sound: String = IDLE
            .lines()
            .filter(|l| !l.starts_with("sound"))
            .collect::<Vec<_>>()
            .join("\n");
        let facts: Vec<Value> = super::judge(&read(IDLE), &loaded)
            .into_iter()
            .chain(super::judge(&read(IDLE), &read(&without_sound)))
            .map(|failed| failed.facts())
            .collect();
        assert_eq!(
            facts,
            [
                json!({"signal": "keys", "figures": ["p99_ms", "worst_ms"], "missing": null}),
                json!({"signal": "sound", "figures": [], "missing": "loaded"}),
            ]
        );
    }

    #[test]
    fn a_call_past_what_a_call_bears_fails_however_bad_idle_was() {
        let poor = IDLE.replace(
            "calls 1500 lost 0 p50 20.50 ms p95 23.00 ms",
            "calls 1500 lost 60 p50 118.00 ms p95 330.00 ms",
        );
        let loaded = poor.replace("p95 330.00", "p95 335.00");
        assert_eq!(
            judge(&read(&poor), &read(&loaded)),
            ["call: p95 335.0 ms, past 300 ms; 60 lost of 1500, past 1%"]
        );
    }

    #[test]
    fn keys_typed_within_the_launchers_budget_pass() {
        let loaded = format!("{IDLE}typed 40 frame p50 19.00 ms p99 82.00 ms worst 82.00 ms\n");
        assert_eq!(judge(&read(IDLE), &read(&loaded)), Vec::<String>::new());
    }

    #[test]
    fn keys_typed_past_the_launchers_budget_fail_whatever_idle_was() {
        let loaded = format!("{IDLE}typed 40 frame p50 60.00 ms p99 140.00 ms worst 150.00 ms\n");
        assert_eq!(
            judge(&read(IDLE), &read(&loaded)),
            ["typed: p50 60.0 ms, past 50 ms; p99 140.0 ms, past 100 ms"]
        );
    }

    #[test]
    fn a_step_nobody_typed_in_is_not_judged_for_it() {
        let none = format!("{IDLE}typed 0 frame p50 0.00 ms p99 0.00 ms worst 0.00 ms\n");
        assert_eq!(judge(&read(IDLE), &read(&none)), Vec::<String>::new());
        assert_eq!(judge(&read(IDLE), &read(IDLE)), Vec::<String>::new());
    }

    #[test]
    fn a_signal_with_no_reading_fails() {
        let without_sound: String = IDLE
            .lines()
            .filter(|l| !l.starts_with("sound"))
            .collect::<Vec<_>>()
            .join("\n");
        let failed = judge(&read(IDLE), &read(&without_sound));
        assert_eq!(failed, ["sound: no reading loaded"]);
        let failed = judge(&read(&without_sound), &read(IDLE));
        assert_eq!(failed, ["sound: no reading idle"]);
    }

    #[test]
    fn a_figure_that_went_missing_fails() {
        let loaded = loaded(&[("p95 23.00 ms ", "")]);
        assert_eq!(
            judge(&read(IDLE), &loaded),
            ["call: p95 NaN ms against 23.0 ms idle"]
        );
    }
}
