//! Whether the NPU is free for what nobody waits on: nothing records from
//! a microphone, as modules/calls counts it, so a call's transcript and
//! dictation, which hear what's recorded on the NPU, never queue behind
//! embedding (M4RL61, 2026-11-07: a call's Parakeet and dictation come
//! before indexing).
//!
//! It asks PipeWire once with `pw-dump`, only when there's something to
//! embed, and keeps the answer a few seconds, so a burst asks once, and
//! nothing is asked while there's nothing to do.

use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use calls::Recording;
use serde_json::Value;

/// How long an answer is kept, and how long embedding waits before
/// asking again while something records.
pub const KEPT: Duration = Duration::from_secs(10);

/// The last answer, and when it was asked.
#[derive(Default)]
pub struct Quiet {
    asked: Option<(Instant, bool)>,
    /// Whether pw-dump has failed, so it's said once.
    said: bool,
}

impl Quiet {
    /// Whether something records now, asked again once the last answer
    /// is older than `KEPT`. When PipeWire can't be asked, nothing is
    /// held back, since nothing hears on the NPU without it either.
    pub fn recording(&mut self) -> bool {
        if let Some((when, answer)) = self.asked
            && when.elapsed() < KEPT
        {
            return answer;
        }
        let answer = match ask() {
            Ok(answer) => answer,
            Err(why) => {
                if !self.said {
                    eprintln!("meaning: can't tell whether anything records: {why}");
                    self.said = true;
                }
                false
            }
        };
        self.asked = Some((Instant::now(), answer));
        answer
    }
}

fn ask() -> Result<bool, String> {
    let dumped = Command::new("pw-dump")
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .map_err(|why| format!("running pw-dump: {why}"))?;
    if !dumped.status.success() {
        return Err(format!("pw-dump ended {}", dumped.status));
    }
    let graph: Value =
        serde_json::from_slice(&dumped.stdout).map_err(|why| format!("reading pw-dump: {why}"))?;
    Ok(records(&graph))
}

/// Whether anything in a pw-dump of the graph records from a microphone.
pub fn records(graph: &Value) -> bool {
    let mut recording = Recording::default();
    recording.take(graph);
    recording.any()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn node(binary: &str) -> Value {
        json!({
            "id": 40, "type": "PipeWire:Interface:Node",
            "info": {"state": "running", "props": {
                "media.class": "Stream/Input/Audio",
                "application.process.binary": binary,
            }},
        })
    }

    #[test]
    fn dictation_or_a_call_recording_holds_embedding_back() {
        assert!(records(&json!([node("dictation")])));
        assert!(records(&json!([node("chrome")])));
    }

    #[test]
    fn a_graph_with_nothing_recording_lets_it_run() {
        assert!(!records(&json!([])));
        assert!(!records(&json!([node("noise-cancelled")])));
    }
}
