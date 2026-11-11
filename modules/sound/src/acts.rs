//! What an act asks of the sound server, and how it's done: a device's
//! volume or mute set, the device chosen, or the port its card plays it
//! through, as `wpctl set-route` sets one. A card's own device is set
//! through the route that plays it, as the session manager saves a route's
//! levels and as Quickshell sets them; any other device through its node's
//! own Props. The choice goes into the default metadata's configured key,
//! which the session manager keeps and follows, and a program's stream is
//! sent to a device by its `target.object` there, which the session
//! manager moves it by, live. A program's volume and mute are set on
//! each of its streams' own Props, as its row names them.

use serde_json::{Value, json};

use crate::devices::Device;
use crate::levels::{ported, props, routed};
use crate::streams::Streams;

/// One act, as asked.
#[derive(Clone, Debug, PartialEq)]
pub enum Act {
    /// A volume as the shell shows it, and the mute with it where given.
    Volume {
        key: String,
        volume: f32,
        muted: Option<bool>,
    },
    Mute {
        key: String,
        muted: bool,
    },
    /// The device to play through or listen with.
    Use {
        key: String,
    },
    /// The port its card plays it through, by the port's index.
    Port {
        key: String,
        index: i32,
    },
    /// A program's streams sent to a device, by the device's node name,
    /// or back to the default, given none, which they then follow.
    Send {
        key: String,
        streams: Vec<u32>,
        target: Option<String>,
    },
    /// A program's volume or mute, set on each of its streams.
    Program {
        key: String,
        streams: Vec<u32>,
        volume: Option<f32>,
        muted: Option<bool>,
    },
}

/// Why an act isn't done, by the interface's codes.
#[derive(Clone, Debug, PartialEq)]
pub struct Refused {
    pub code: &'static str,
    pub message: String,
}

fn refused(code: &'static str, message: impl Into<String>) -> Refused {
    Refused {
        code,
        message: message.into(),
    }
}

/// How one act is done on the server.
#[derive(Clone, Debug, PartialEq)]
pub enum Move {
    /// Set on the node's own Props, by the node's id.
    Node { id: u32, pod: Vec<u8> },
    /// Set on a card's device, on the route playing the node.
    Card { id: u32, pod: Vec<u8> },
    /// Said in the default metadata.
    Choose { key: &'static str, value: String },
    /// A stream's target said in the default metadata, or let go of.
    Send { stream: u32, target: Option<String> },
    /// Several moves, done in turn, as a program's streams are each set.
    Each(Vec<Move>),
}

#[expect(clippy::cast_possible_truncation, reason = "a volume fits an f32")]
fn share(volume: f64) -> f32 {
    volume as f32
}

/// The act named, with the rest of what was said as its arguments.
pub fn act(name: &str, said: &Value) -> Result<Act, Refused> {
    if name == "send" {
        return sent(said);
    }
    if said["key"]
        .as_str()
        .is_some_and(|key| key.starts_with("program:"))
    {
        return program(name, said);
    }
    let key = said["key"]
        .as_str()
        .filter(|key| key.starts_with("device:"))
        .ok_or_else(|| refused("bad-act", "an act names its device's key"))?
        .to_owned();
    match name {
        "volume" => {
            // No further than full, as the shell shows it: any process of
            // the account can ask, and a volume of 10 would be 1,000%.
            let volume = said["volume"]
                .as_f64()
                .filter(|volume| (0.0..=1.0).contains(volume))
                .ok_or_else(|| refused("bad-act", "a volume act says a volume from 0 to 1"))?;
            Ok(Act::Volume {
                key,
                volume: share(volume),
                muted: said["muted"].as_bool(),
            })
        }
        "mute" => {
            let muted = said["muted"]
                .as_bool()
                .ok_or_else(|| refused("bad-act", "a mute act says whether it's muted"))?;
            Ok(Act::Mute { key, muted })
        }
        "use" => Ok(Act::Use { key }),
        "port" => {
            let index = said["index"]
                .as_i64()
                .and_then(|index| i32::try_from(index).ok())
                .ok_or_else(|| refused("bad-act", "a port act says the port's index"))?;
            Ok(Act::Port { key, index })
        }
        _ => Err(refused("unknown-act", format!("no act {name:?} here"))),
    }
}

/// The streams a program's act names, by their ids, at least one.
fn streams_of(said: &Value) -> Result<Vec<u32>, Refused> {
    let ids: Option<Vec<u32>> = said["streams"].as_array().map(|ids| {
        ids.iter()
            .filter_map(|id| id.as_u64().and_then(|id| u32::try_from(id).ok()))
            .collect()
    });
    ids.filter(|ids| !ids.is_empty())
        .ok_or_else(|| refused("bad-act", "a program's act names its streams"))
}

/// A send: the streams it moves, one by its key, `stream:<id>` as
/// PipeWire numbers it, or a program's by their ids, and the device they
/// go to, by its node name, or "" for the default.
fn sent(said: &Value) -> Result<Act, Refused> {
    let key = said["key"].as_str().unwrap_or_default();
    let streams = match key.strip_prefix("stream:") {
        Some(id) => id.parse().ok().map(|id| vec![id]),
        None => key
            .starts_with("program:")
            .then(|| streams_of(said).ok())
            .flatten(),
    }
    .ok_or_else(|| {
        refused(
            "bad-act",
            "a send names its stream's key, or a program's streams",
        )
    })?;
    let target = said["target"]
        .as_str()
        .ok_or_else(|| refused("bad-act", "a send says its device, or \"\" for the default"))?;
    Ok(Act::Send {
        key: key.to_owned(),
        streams,
        target: (!target.is_empty()).then(|| target.to_owned()),
    })
}

/// A program's volume or mute, on the streams it names.
fn program(name: &str, said: &Value) -> Result<Act, Refused> {
    let key = said["key"].as_str().unwrap_or_default().to_owned();
    let streams = streams_of(said)?;
    let (volume, muted) = match name {
        "volume" => {
            let volume = said["volume"]
                .as_f64()
                .filter(|volume| (0.0..=1.0).contains(volume))
                .ok_or_else(|| refused("bad-act", "a volume act says a volume from 0 to 1"))?;
            (Some(share(volume)), said["muted"].as_bool())
        }
        "mute" => {
            let muted = said["muted"]
                .as_bool()
                .ok_or_else(|| refused("bad-act", "a mute act says whether it's muted"))?;
            (None, Some(muted))
        }
        _ => {
            return Err(refused(
                "unknown-act",
                format!("no act {name:?} for a program"),
            ));
        }
    };
    Ok(Act::Program {
        key,
        streams,
        volume,
        muted,
    })
}

impl Act {
    /// What it acts on, by its key: a device's record, or a stream.
    pub fn key(&self) -> &str {
        match self {
            Act::Volume { key, .. }
            | Act::Mute { key, .. }
            | Act::Use { key }
            | Act::Port { key, .. }
            | Act::Send { key, .. }
            | Act::Program { key, .. } => key,
        }
    }

    /// How it's done where it needs no device found first: a send, whose
    /// streams are the session manager's to find, each in turn.
    pub fn alone(&self) -> Option<Move> {
        let Act::Send {
            streams, target, ..
        } = self
        else {
            return None;
        };
        let mut moves: Vec<Move> = streams
            .iter()
            .map(|stream| Move::Send {
                stream: *stream,
                target: target.clone(),
            })
            .collect();
        Some(if moves.len() == 1 {
            moves.remove(0)
        } else {
            Move::Each(moves)
        })
    }
}

/// How a program's act is done on each of its streams, every one of
/// which must still be heard, so no act reaches another program's.
pub fn on_streams(act: &Act, heard: &Streams) -> Result<Move, Refused> {
    let Act::Program {
        streams,
        volume,
        muted,
        ..
    } = act
    else {
        return Err(refused(
            "bad-act",
            "only a program's act is done on its streams",
        ));
    };
    let mut moves = Vec::new();
    for id in streams {
        let Some(stream) = heard.stream(*id) else {
            return Err(refused("no-stream", format!("no stream {id} here")));
        };
        let volumes = match volume {
            Some(_) if stream.volumes.is_empty() => {
                let message = format!("stream {id} hasn't said its volume yet");
                return Err(refused("no-volume", message));
            }
            Some(volume) => Some(leveled(&stream.volumes, *volume)),
            None => None,
        };
        moves.push(Move::Node {
            id: *id,
            pod: props(volumes.as_deref(), *muted),
        });
    }
    Ok(Move::Each(moves))
}

/// Each channel's share for a volume as the shell shows it, keeping how
/// the channels stand to each other, as Quickshell's own setting does: the
/// cube roots are scaled to the mean asked for, then cubed back.
pub fn leveled(volumes: &[f32], wanted: f32) -> Vec<f32> {
    let roots: Vec<f32> = volumes.iter().map(|v| v.max(0.0).cbrt()).collect();
    let count = f32::from(u16::try_from(roots.len()).unwrap_or(u16::MAX));
    let mean = roots.iter().sum::<f32>() / count;
    // From silence there's nothing to keep, so each channel takes it.
    if mean <= 0.0 {
        return vec![wanted.powi(3); roots.len()];
    }
    roots
        .iter()
        .map(|root| (root * wanted / mean).powi(3))
        .collect()
}

/// How an act is done on this device.
pub fn planned(act: &Act, device: &Device) -> Result<Move, Refused> {
    let (volumes, muted) = match act {
        Act::Use { .. } => {
            let key = if device.sink {
                "default.configured.audio.sink"
            } else {
                "default.configured.audio.source"
            };
            let value = json!({"name": device.name}).to_string();
            return Ok(Move::Choose { key, value });
        }
        Act::Volume { volume, muted, .. } => {
            if device.volumes.is_empty() {
                let message = format!("{} hasn't said its volume yet", device.name);
                return Err(refused("no-volume", message));
            }
            (Some(leveled(device.volumes, *volume)), *muted)
        }
        Act::Mute { muted, .. } => (None, Some(*muted)),
        // Done alone, by `alone`, or on its streams, by `on_streams`.
        Act::Send { .. } | Act::Program { .. } => {
            return Err(refused(
                "bad-act",
                "a program's act moves streams, not a device",
            ));
        }
        Act::Port { index, .. } => {
            // Only a port its card offers it, so no act moves another
            // device's.
            let (Some((card, _, played)), true) = (device.route, device.ports.contains(index))
            else {
                let message = format!("{} has no port {index}", device.name);
                return Err(refused("no-port", message));
            };
            return Ok(Move::Card {
                id: card,
                pod: ported(*index, played),
            });
        }
    };
    Ok(match device.route {
        Some((card, index, profile)) => Move::Card {
            id: card,
            pod: routed(index, profile, volumes.as_deref(), muted),
        },
        None => Move::Node {
            id: device.id,
            pod: props(volumes.as_deref(), muted),
        },
    })
}

#[cfg(test)]
mod tests {
    use crate::levels::{levels, route};

    use super::*;

    fn near(got: &[f32], wanted: &[f32]) -> bool {
        got.len() == wanted.len() && got.iter().zip(wanted).all(|(a, b)| (a - b).abs() < 1e-5)
    }

    fn speakers(route: Option<(u32, i32, i32)>) -> Device<'static> {
        Device {
            id: 40,
            name: "speakers",
            sink: true,
            volumes: &[0.125, 0.015_625],
            route,
            ports: vec![0, 1],
        }
    }

    #[test]
    fn an_act_says_its_device_and_what_to_set() {
        let said = json!({"key": "device:speakers", "volume": 0.5, "muted": false});
        assert_eq!(
            act("volume", &said),
            Ok(Act::Volume {
                key: "device:speakers".into(),
                volume: 0.5,
                muted: Some(false)
            })
        );
        let said = json!({"key": "device:speakers", "muted": true});
        let key = act("mute", &said).map(|act| act.key().to_owned());
        assert_eq!(key, Ok("device:speakers".into()));
    }

    #[test]
    fn an_act_missing_what_it_needs_is_refused() {
        let code = |name: &str, said: Value| act(name, &said).map_err(|refused| refused.code);
        let below = json!({"key": "device:a", "volume": -1});
        let above = json!({"key": "device:a", "volume": 10});
        assert_eq!(code("volume", json!({"key": "device:a"})), Err("bad-act"));
        assert_eq!(code("mute", json!({"key": "device:a"})), Err("bad-act"));
        assert_eq!(code("use", json!({"key": "window:1"})), Err("bad-act"));
        assert_eq!(code("volume", below), Err("bad-act"));
        assert_eq!(code("volume", above), Err("bad-act"));
        assert_eq!(
            code("louder", json!({"key": "device:a"})),
            Err("unknown-act")
        );
        let at = |key: &str| json!({"key": key, "target": "speakers"});
        assert_eq!(code("send", at("device:a")), Err("bad-act"));
        assert_eq!(code("send", at("stream:x")), Err("bad-act"));
        assert_eq!(code("send", json!({"key": "stream:93"})), Err("bad-act"));
    }

    #[test]
    fn a_send_says_a_streams_target_or_lets_it_go() {
        let said = json!({"key": "stream:93", "target": "headphones"});
        let target = Some("headphones".to_owned());
        let sending = act("send", &said).map(|act| act.alone());
        assert_eq!(sending, Ok(Some(Move::Send { stream: 93, target })));
        let said = json!({"key": "stream:93", "target": ""});
        let back = act("send", &said).map(|act| act.alone());
        let target = None;
        assert_eq!(back, Ok(Some(Move::Send { stream: 93, target })));
        let said = json!({"key": "device:speakers", "muted": true});
        assert_eq!(act("mute", &said).map(|act| act.alone()), Ok(None));
    }

    #[test]
    fn a_programs_send_moves_each_of_its_streams() {
        let said = json!({"key": "program:speakers:Firefox", "streams": [70, 71],
            "target": "headphones"});
        let target = Some("headphones".to_owned());
        let moves = vec![
            Move::Send {
                stream: 70,
                target: target.clone(),
            },
            Move::Send { stream: 71, target },
        ];
        let sending = act("send", &said).map(|act| act.alone());
        assert_eq!(sending, Ok(Some(Move::Each(moves))));
        let none = json!({"key": "program:speakers:Firefox", "streams": [], "target": ""});
        let refused = act("send", &none).map_err(|refused| refused.code);
        assert_eq!(
            refused,
            Err("bad-act"),
            "a program's send names its streams"
        );
    }

    // A program's volume is set on each stream's own Props, keeping how
    // its channels stand; a stream no longer heard refuses the act.
    #[test]
    fn a_programs_volume_is_set_on_each_stream() {
        let mut heard = Streams::default();
        let class = |class: &str| {
            std::collections::BTreeMap::from([("media.class".to_owned(), class.to_owned())])
        };
        heard.node(70, class("Stream/Output/Audio"));
        heard.levels(70, Some(vec![0.125, 0.015_625]), Some(true));
        let said = json!({"key": "program:speakers:Firefox", "streams": [70],
            "volume": 0.75, "muted": false});
        let asked = act("volume", &said).expect("a program's volume is an act");
        let wanted = Move::Each(vec![Move::Node {
            id: 70,
            pod: props(
                Some(leveled(&[0.125, 0.015_625], 0.75).as_slice()),
                Some(false),
            ),
        }]);
        assert_eq!(on_streams(&asked, &heard), Ok(wanted));
        let gone = json!({"key": "program:speakers:Firefox", "streams": [71], "muted": true});
        let asked = act("mute", &gone).expect("a program's mute is an act");
        let refused = on_streams(&asked, &heard).map_err(|refused| refused.code);
        assert_eq!(refused, Err("no-stream"), "a stream gone");
    }

    #[test]
    fn a_volume_keeps_how_the_channels_stand() {
        // Cube roots of 0.5 and 0.25, a mean of 0.375, doubled to 0.75.
        assert!(
            near(&leveled(&[0.125, 0.015_625], 0.75), &[1.0, 0.125]),
            "each channel is scaled alike"
        );
        assert!(
            near(&leveled(&[0.0, 0.0], 0.5), &[0.125, 0.125]),
            "from silence, every channel takes the volume"
        );
    }

    #[test]
    fn a_device_a_card_plays_is_set_on_its_route() {
        let asked = Act::Mute {
            key: "device:speakers".into(),
            muted: true,
        };
        let Ok(Move::Card { id, pod }) = planned(&asked, &speakers(Some((7, 2, 1)))) else {
            panic!("a card's device is set on its route");
        };
        assert_eq!((id, route(&pod)), (7, Some((2, 1))));
    }

    #[test]
    fn any_other_device_is_set_on_its_node() {
        let asked = Act::Volume {
            key: "device:speakers".into(),
            volume: 0.75,
            muted: Some(false),
        };
        let Ok(Move::Node { id, pod }) = planned(&asked, &speakers(None)) else {
            panic!("a device no card plays is set on its node");
        };
        let (volumes, muted) = levels(&pod);
        assert_eq!((id, muted), (40, Some(false)));
        assert!(
            near(&volumes.unwrap_or_default(), &[1.0, 0.125]),
            "the volumes are set"
        );
    }

    #[test]
    fn using_a_device_configures_the_default() {
        let asked = Act::Use {
            key: "device:speakers".into(),
        };
        assert_eq!(
            planned(&asked, &speakers(None)),
            Ok(Move::Choose {
                key: "default.configured.audio.sink",
                value: r#"{"name":"speakers"}"#.into()
            })
        );
    }

    // The port is chosen on the card, for the profile's device it plays,
    // and only among the ports it offers that device.
    #[test]
    fn a_port_is_chosen_on_its_card() {
        let asked = |index: i32| Act::Port {
            key: "device:speakers".into(),
            index,
        };
        let Ok(Move::Card { id, pod }) = planned(&asked(1), &speakers(Some((7, 0, 3)))) else {
            panic!("a port is chosen on its card");
        };
        assert_eq!((id, route(&pod)), (7, Some((1, 3))));
        let refusal = |device: &Device| planned(&asked(5), device).map_err(|it| it.code);
        assert_eq!(refusal(&speakers(Some((7, 0, 3)))), Err("no-port"));
        assert_eq!(refusal(&speakers(None)), Err("no-port"));
        let said = json!({"key": "device:speakers", "index": 1});
        assert_eq!(act("port", &said), Ok(asked(1)));
        let code = act("port", &json!({"key": "device:speakers"})).map_err(|it| it.code);
        assert_eq!(code, Err("bad-act"));
    }

    #[test]
    fn a_volume_before_the_device_says_its_own_is_refused() {
        let asked = Act::Volume {
            key: "device:speakers".into(),
            volume: 0.5,
            muted: None,
        };
        let quiet = Device {
            volumes: &[],
            ..speakers(None)
        };
        let refusal = planned(&asked, &quiet).map_err(|refused| refused.code);
        assert_eq!(refusal, Err("no-volume"));
    }
}
