//! What keeps Choose by Ear from recording: commands Nix lists, each of
//! which means nothing may be recorded while it exits 0, such as one
//! saying a call is on or a meeting is near. The trial asks each before
//! it records (trial.rs).

use std::process::{Command, Stdio};

use serde_json::Value;

/// Something that, while it exits 0, means nothing may be recorded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Refusal {
    pub command: Vec<String>,
    pub why: String,
}

impl Refusal {
    /// Each refusal a file's JSON lists, as `[{"command": [...], "why": "..."}]`.
    pub fn listed(text: &str) -> Result<Vec<Self>, String> {
        let listed: Value =
            serde_json::from_str(text).map_err(|why| format!("reading the refusals: {why}"))?;
        let strings = |value: &Value| -> Option<Vec<String>> {
            value
                .as_array()?
                .iter()
                .map(|word| word.as_str().map(str::to_owned))
                .collect()
        };
        listed
            .as_array()
            .ok_or_else(|| "the refusals aren't a list".to_owned())?
            .iter()
            .map(|refusal| {
                let command = strings(&refusal["command"]);
                let why = refusal["why"].as_str().map(str::to_owned);
                command
                    .zip(why)
                    .map(|(command, why)| Self { command, why })
                    .ok_or_else(|| format!("a refusal needs a command and a why: {refusal}"))
            })
            .collect()
    }

    /// Whether the refusal holds now: its command ran and exited 0.
    pub fn holds(&self) -> bool {
        let Some((program, arguments)) = self.command.split_first() else {
            return false;
        };
        Command::new(program)
            .args(arguments)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|status| status.success())
    }
}
