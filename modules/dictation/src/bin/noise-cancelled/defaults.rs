//! What the sound server holds as its defaults, for the trial: what
//! PipeWire holds now, as pw-dump says it, the output the clips play on,
//! and making the twin of a pick the microphone calls hear. The default
//! microphone itself is found in dictation's library, which dictation
//! shares.

use std::process::{Command, Stdio};

use serde_json::{Value, json};

/// What PipeWire holds now, as pw-dump says it.
pub fn dumped(dump: &str) -> Result<Value, String> {
    let out = Command::new(dump)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .map_err(|why| format!("asking PipeWire: {why}"))?;
    serde_json::from_slice(&out.stdout).map_err(|why| format!("reading PipeWire: {why}"))
}

/// The default output in a dump, by its description: what the clips
/// play on.
pub fn default_output(dump: &Value) -> Option<String> {
    let objects = dump.as_array()?;
    let default = objects
        .iter()
        .filter(|object| object["props"]["metadata.name"] == "default")
        .filter_map(|object| object["metadata"].as_array())
        .flatten()
        .find(|entry| entry["key"] == "default.audio.sink")?;
    let name = default["value"]["name"].as_str()?;
    objects.iter().find_map(|object| {
        let props = &object["info"]["props"];
        (props["node.name"] == name && props["media.class"] == "Audio/Sink").then(|| {
            props["node.description"]
                .as_str()
                .unwrap_or(name)
                .to_owned()
        })
    })
}

/// Makes a node the default microphone, as choosing it in Sound does:
/// `WirePlumber`'s own configured default, which it keeps.
pub fn default_microphone_is(metadata: &str, name: &str) -> Result<(), String> {
    let value = json!({"name": name}).to_string();
    let status = Command::new(metadata)
        .args([
            "-n",
            "default",
            "0",
            "default.configured.audio.source",
            &value,
        ])
        .arg("Spa:String:JSON")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|why| format!("choosing the microphone: {why}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "choosing the microphone: pw-metadata said {status}"
        ))
    }
}
