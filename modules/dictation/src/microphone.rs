//! The microphone Joel picked, as PipeWire names it, and its
//! noise-cancelled twin, which noise-cancelled makes for each microphone
//! and cleans as he chose by ear. Dictation records from the twin, so what
//! it hears is what a call hears, and the trial records from the
//! microphone itself.

use serde_json::Value;

/// The start of every twin's node name, before its microphone's.
pub const OURS: &str = "noise-cancelled.";

/// A microphone as PipeWire names it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Microphone {
    /// Its node's name, which a recording targets.
    pub name: String,
    /// What it's called where Joel reads it.
    pub description: String,
}

impl Microphone {
    /// Its noise-cancelled twin's node name.
    pub fn twin(&self) -> String {
        format!("{OURS}{}", self.name)
    }
}

/// The default microphone in a pw-dump, found through its twin when the
/// twin is the default, since the twin is only ever the default for it.
pub fn default_microphone(dump: &Value) -> Option<Microphone> {
    let objects = dump.as_array()?;
    let default = objects
        .iter()
        .filter(|object| object["props"]["metadata.name"] == "default")
        .filter_map(|object| object["metadata"].as_array())
        .flatten()
        .find(|entry| entry["key"] == "default.audio.source")?;
    let named = default["value"]["name"].as_str()?;
    let name = named.strip_prefix(OURS).unwrap_or(named);
    objects.iter().find_map(|object| {
        let props = &object["info"]["props"];
        (props["node.name"] == name && props["media.class"] == "Audio/Source").then(|| Microphone {
            name: name.to_owned(),
            description: props["node.description"]
                .as_str()
                .unwrap_or(name)
                .to_owned(),
        })
    })
}

/// A microphone in a pw-dump by its node name, or by its twin's, so
/// Choose by Ear records the one it was opened for, the default or not.
pub fn named(dump: &Value, _name: &str) -> Option<Microphone> {
    default_microphone(dump)
}

/// What to record the default microphone through: its twin when the dump
/// holds one, else the microphone itself, else nothing, which leaves the
/// choice to the sound server.
pub fn to_record(dump: &Value) -> Option<String> {
    let microphone = default_microphone(dump)?;
    let twin = microphone.twin();
    let has_twin = dump.as_array()?.iter().any(|object| {
        let props = &object["info"]["props"];
        props["node.name"] == twin.as_str() && props["media.class"] == "Audio/Source"
    });
    Some(if has_twin { twin } else { microphone.name })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn dump(default: &str, twin: bool) -> Value {
        let mut objects = vec![
            json!({"type": "PipeWire:Interface:Metadata", "props": {"metadata.name": "default"},
                   "metadata": [{"subject": 0, "key": "default.audio.source",
                                 "value": {"name": default}}]}),
            json!({"info": {"props": {"node.name": "mv7", "node.description": "Shure MV7",
                                      "media.class": "Audio/Source"}}}),
            json!({"info": {"props": {"node.name": "noise-cancelled.mv7.capture",
                                      "media.class": "Stream/Input/Audio"}}}),
        ];
        if twin {
            objects.push(
                json!({"info": {"props": {"node.name": "noise-cancelled.mv7",
                                                   "media.class": "Audio/Source"}}}),
            );
        }
        Value::Array(objects)
    }

    #[test]
    fn the_default_is_found_through_its_twin() {
        let microphone = default_microphone(&dump("noise-cancelled.mv7", true)).expect("one");
        assert_eq!(microphone.name, "mv7");
        assert_eq!(microphone.description, "Shure MV7");
        assert_eq!(microphone.twin(), "noise-cancelled.mv7");
    }

    #[test]
    fn a_microphone_is_found_by_its_name_or_its_twins() {
        let dumped = dump("someone-else", true);
        let description = named(&dumped, "mv7").map(|microphone| microphone.description);
        assert_eq!(description.as_deref(), Some("Shure MV7"));
        let name = named(&dumped, "noise-cancelled.mv7").map(|microphone| microphone.name);
        assert_eq!(name.as_deref(), Some("mv7"));
        assert_eq!(named(&dumped, "gone"), None);
    }

    #[test]
    fn dictation_records_through_the_twin() {
        assert_eq!(
            to_record(&dump("mv7", true)).as_deref(),
            Some("noise-cancelled.mv7")
        );
        assert_eq!(
            to_record(&dump("noise-cancelled.mv7", true)).as_deref(),
            Some("noise-cancelled.mv7")
        );
    }

    #[test]
    fn with_no_twin_it_records_the_microphone() {
        assert_eq!(to_record(&dump("mv7", false)).as_deref(), Some("mv7"));
    }

    #[test]
    fn with_no_default_the_sound_server_chooses() {
        assert_eq!(to_record(&json!([])), None);
        assert_eq!(to_record(&dump("gone", true)), None);
    }
}
