//! What niri says is taken of the screen, apart from the windows: each
//! screen or window it casts, kept as a `cast` record, and the last
//! screenshot, kept as one `screenshot` record, for the few readers that
//! draw them (the recorder, Sharing, Quiet and calls).

use std::collections::BTreeMap;

use serde_json::{Value, json};

use crate::desktop::Heard;

/// Every cast and the last screenshot, as niri's events describe them.
#[derive(Debug, Default)]
pub struct Captures {
    /// Every cast by its stream's id.
    casts: BTreeMap<u64, Value>,
    /// The last screenshot's path, and how many have been taken, so a
    /// reader hears of a new one even where it's saved to the same path.
    screenshot: Value,
    screenshots: u64,
}

impl Captures {
    /// Hears one event of niri's stream, by its name and what it said.
    pub fn hear(&mut self, name: &str, said: &Value) -> Heard {
        match name {
            "CastsChanged" => {
                self.casts = said["casts"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter(|cast| cast.is_object())
                    .filter_map(|cast| Some((cast["stream_id"].as_u64()?, cast.clone())))
                    .collect();
            }
            "CastStartedOrChanged" => {
                let cast = &said["cast"];
                let Some(id) = cast["stream_id"].as_u64().filter(|_| cast.is_object()) else {
                    return Heard::Nothing;
                };
                self.casts.insert(id, cast.clone());
            }
            "CastStopped" => {
                if let Some(id) = said["stream_id"].as_u64() {
                    self.casts.remove(&id);
                }
            }
            "ScreenshotCaptured" => {
                self.screenshot = said["path"].clone();
                self.screenshots += 1;
            }
            _ => return Heard::Nothing,
        }
        Heard::Changed
    }

    /// Each cast's record, as niri says it, and the screenshot's.
    pub fn records(&self, records: &mut Vec<Value>) {
        records.extend(self.casts.iter().map(|(id, cast)| {
            let mut record = cast.clone();
            record["key"] = json!(format!("cast:{id}"));
            record["order"] = json!(id);
            record
        }));
        records.push(json!({
            "key": "screenshot",
            "path": self.screenshot,
            "taken": self.screenshots,
        }));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hear(captures: &mut Captures, event: &str) -> Heard {
        let event: Value = serde_json::from_str(event).expect("an event is JSON");
        let (name, said) = event
            .as_object()
            .and_then(|event| event.iter().next())
            .expect("an event names itself");
        captures.hear(name, said)
    }

    fn records(captures: &Captures) -> BTreeMap<String, Value> {
        let mut records = Vec::new();
        captures.records(&mut records);
        records
            .into_iter()
            .map(|record| (record["key"].as_str().unwrap().to_owned(), record))
            .collect()
    }

    #[test]
    fn each_cast_is_kept_as_niri_says_it() {
        let mut captures = Captures::default();
        hear(
            &mut captures,
            r#"{"CastsChanged":{"casts":[{"stream_id":3,"kind":"PipeWire","is_active":true,"target":{"Output":{"name":"eDP-1"}},"pid":200}]}}"#,
        );
        let said = records(&captures);
        assert_eq!(said["cast:3"]["kind"], "PipeWire");
        assert_eq!(said["cast:3"]["target"]["Output"]["name"], "eDP-1");
        hear(
            &mut captures,
            r#"{"CastStartedOrChanged":{"cast":{"stream_id":4,"kind":"WlrScreencopy","is_active":true,"target":{"Window":{"id":11}}}}}"#,
        );
        hear(&mut captures, r#"{"CastStopped":{"stream_id":3}}"#);
        let after = records(&captures);
        assert_eq!(
            after.keys().collect::<Vec<_>>(),
            ["cast:4", "screenshot"],
            "the cast stopped is gone, the one started kept"
        );
        assert_eq!(after["cast:4"]["target"]["Window"]["id"], 11);
        assert_eq!(
            hear(&mut captures, r#"{"CastStartedOrChanged":{"cast":null}}"#),
            Heard::Nothing
        );
    }

    #[test]
    fn each_screenshot_is_counted_even_at_the_same_path() {
        let mut captures = Captures::default();
        assert_eq!(records(&captures)["screenshot"]["taken"], 0);
        hear(
            &mut captures,
            r#"{"ScreenshotCaptured":{"path":"/tmp/a.png"}}"#,
        );
        let once = records(&captures);
        assert_eq!(once["screenshot"]["path"], "/tmp/a.png");
        hear(
            &mut captures,
            r#"{"ScreenshotCaptured":{"path":"/tmp/a.png"}}"#,
        );
        let twice = records(&captures);
        assert_ne!(once["screenshot"], twice["screenshot"]);
        assert_eq!(twice["screenshot"]["taken"], 2);
    }
}
