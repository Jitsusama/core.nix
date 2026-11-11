//! How loud a node is set, read from the Props it says and written into
//! the Props or the Route an act sets: each channel's volume and whether
//! it's muted. A Props names only what it says, so either may be missing,
//! and a missing one keeps what was said before. A card's device says
//! which of its routes plays each of its profile's devices, the profile
//! it's in, and each route it has, a port, read here too, and a Route
//! written to choose one.

use std::io::Cursor;

use pipewire::spa::pod::deserialize::PodDeserializer;
use pipewire::spa::pod::serialize::PodSerializer;
use pipewire::spa::pod::{Object, Property, PropertyFlags, Value, ValueArray};
use pipewire::spa::sys;
use pipewire::spa::utils::Id;

/// Each channel's volume and the mute a Props pod says, from its bytes.
pub fn levels(pod: &[u8]) -> (Option<Vec<f32>>, Option<bool>) {
    let Ok((_, Value::Object(object))) = PodDeserializer::deserialize_any_from(pod) else {
        return (None, None);
    };
    let mut volumes = None;
    let mut muted = None;
    for property in object.properties {
        match property.value {
            Value::ValueArray(ValueArray::Float(said))
                if property.key == sys::SPA_PROP_channelVolumes =>
            {
                volumes = Some(said);
            }
            Value::Bool(said) if property.key == sys::SPA_PROP_mute => muted = Some(said),
            _ => {}
        }
    }
    (volumes, muted)
}

/// The route a card's Route pod says it plays, and the profile's device
/// it plays it for: `(index, device)`.
pub fn route(pod: &[u8]) -> Option<(i32, i32)> {
    let Ok((_, Value::Object(object))) = PodDeserializer::deserialize_any_from(pod) else {
        return None;
    };
    let mut index = None;
    let mut device = None;
    for property in object.properties {
        match property.value {
            Value::Int(n) if property.key == sys::SPA_PARAM_ROUTE_index => index = Some(n),
            Value::Int(n) if property.key == sys::SPA_PARAM_ROUTE_device => device = Some(n),
            _ => {}
        }
    }
    Some((index?, device?))
}

/// One route a card has, a port: a jack, the speakers, a display's output.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Port {
    pub index: i32,
    /// Whether it's played through (true) or listened with (false), or
    /// neither said.
    pub sink: Option<bool>,
    /// What the card calls it, "Headphones", by its description first.
    pub name: String,
    /// The profile's devices it can play.
    pub devices: Vec<i32>,
    /// The profiles it's in, or none said for every profile.
    pub profiles: Vec<i32>,
    /// Whether the card says anything is plugged into it: "yes", "no",
    /// or "unknown" for one it can't tell of, as the speakers are.
    pub available: &'static str,
}

impl Port {
    /// Whether the card says nothing is plugged into it, as a jack with
    /// nothing in it and a display's output with no display are. One it
    /// can't tell of is plugged in.
    pub fn unplugged(&self) -> bool {
        self.available == "no"
    }
}

fn availability(said: u32) -> &'static str {
    match said {
        sys::SPA_PARAM_AVAILABILITY_no => "no",
        sys::SPA_PARAM_AVAILABILITY_yes => "yes",
        _ => "unknown",
    }
}

/// What a card's `EnumRoute` pod says of one route it has.
pub fn offered(pod: &[u8]) -> Option<Port> {
    let Ok((_, Value::Object(object))) = PodDeserializer::deserialize_any_from(pod) else {
        return None;
    };
    let mut index = None;
    let mut port = Port {
        available: "unknown",
        ..Port::default()
    };
    let mut named = String::new();
    for property in object.properties {
        match (property.key, property.value) {
            (sys::SPA_PARAM_ROUTE_index, Value::Int(n)) => index = Some(n),
            (sys::SPA_PARAM_ROUTE_direction, Value::Id(Id(way))) => {
                port.sink = Some(way == sys::SPA_DIRECTION_OUTPUT);
            }
            (sys::SPA_PARAM_ROUTE_name, Value::String(said)) => named = said,
            (sys::SPA_PARAM_ROUTE_description, Value::String(said)) => port.name = said,
            (sys::SPA_PARAM_ROUTE_devices, Value::ValueArray(ValueArray::Int(said))) => {
                port.devices = said;
            }
            (sys::SPA_PARAM_ROUTE_profiles, Value::ValueArray(ValueArray::Int(said))) => {
                port.profiles = said;
            }
            (sys::SPA_PARAM_ROUTE_available, Value::Id(Id(said))) => {
                port.available = availability(said);
            }
            _ => {}
        }
    }
    if port.name.is_empty() {
        port.name = named;
    }
    port.index = index?;
    Some(port)
}

/// The profile a card's Profile pod says it's in, by its index.
pub fn profile(pod: &[u8]) -> Option<i32> {
    let Ok((_, Value::Object(object))) = PodDeserializer::deserialize_any_from(pod) else {
        return None;
    };
    object
        .properties
        .into_iter()
        .find_map(|property| match (property.key, property.value) {
            (sys::SPA_PARAM_PROFILE_index, Value::Int(n)) => Some(n),
            _ => None,
        })
}

/// A Route pod choosing the port a card plays one of its profile's
/// devices through, saved, as `wpctl set-route` sets it.
pub fn ported(index: i32, device: i32) -> Vec<u8> {
    written(&Value::Object(Object {
        type_: sys::SPA_TYPE_OBJECT_ParamRoute,
        id: sys::SPA_PARAM_Route,
        properties: vec![
            keyed(sys::SPA_PARAM_ROUTE_index, Value::Int(index)),
            keyed(sys::SPA_PARAM_ROUTE_device, Value::Int(device)),
            keyed(sys::SPA_PARAM_ROUTE_save, Value::Bool(true)),
        ],
    }))
}

fn keyed(key: u32, value: Value) -> Property {
    Property {
        key,
        flags: PropertyFlags::empty(),
        value,
    }
}

/// A Props object setting what's given and nothing else.
fn props_object(volumes: Option<&[f32]>, muted: Option<bool>) -> Value {
    let mut properties = Vec::new();
    if let Some(volumes) = volumes {
        properties.push(keyed(
            sys::SPA_PROP_channelVolumes,
            Value::ValueArray(ValueArray::Float(volumes.to_vec())),
        ));
    }
    if let Some(muted) = muted {
        properties.push(keyed(sys::SPA_PROP_mute, Value::Bool(muted)));
    }
    Value::Object(Object {
        type_: sys::SPA_TYPE_OBJECT_Props,
        id: sys::SPA_PARAM_Props,
        properties,
    })
}

fn written(value: &Value) -> Vec<u8> {
    PodSerializer::serialize(Cursor::new(Vec::new()), value)
        .map(|(written, _)| written.into_inner())
        .unwrap_or_default()
}

/// A Props pod for a node's own volumes and mute.
pub fn props(volumes: Option<&[f32]>, muted: Option<bool>) -> Vec<u8> {
    written(&props_object(volumes, muted))
}

/// A Route pod setting a card's route's volumes and mute, saved, as the
/// session manager keeps a route's levels from one start to the next.
pub fn routed(index: i32, device: i32, volumes: Option<&[f32]>, muted: Option<bool>) -> Vec<u8> {
    written(&Value::Object(Object {
        type_: sys::SPA_TYPE_OBJECT_ParamRoute,
        id: sys::SPA_PARAM_Route,
        properties: vec![
            keyed(sys::SPA_PARAM_ROUTE_index, Value::Int(index)),
            keyed(sys::SPA_PARAM_ROUTE_device, Value::Int(device)),
            keyed(sys::SPA_PARAM_ROUTE_props, props_object(volumes, muted)),
            keyed(sys::SPA_PARAM_ROUTE_save, Value::Bool(true)),
        ],
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_props_says_each_channels_volume_and_the_mute() {
        let pod = written(&Value::Object(Object {
            type_: sys::SPA_TYPE_OBJECT_Props,
            id: sys::SPA_PARAM_Props,
            properties: vec![
                keyed(sys::SPA_PROP_volume, Value::Float(1.0)),
                keyed(sys::SPA_PROP_mute, Value::Bool(true)),
                keyed(
                    sys::SPA_PROP_channelVolumes,
                    Value::ValueArray(ValueArray::Float(vec![0.25, 0.5])),
                ),
            ],
        }));
        assert_eq!(levels(&pod), (Some(vec![0.25, 0.5]), Some(true)));
    }

    #[test]
    fn what_a_props_leaves_out_is_none() {
        assert_eq!(levels(&props(None, Some(false))), (None, Some(false)));
        assert_eq!(levels(&[1, 2, 3]), (None, None));
    }

    #[test]
    fn a_props_written_reads_back_the_same() {
        let pod = props(Some(&[0.125, 1.0]), None);
        assert_eq!(levels(&pod), (Some(vec![0.125, 1.0]), None));
    }

    #[test]
    fn a_route_says_its_index_and_the_profiles_device() {
        let pod = routed(3, 1, Some(&[0.5]), Some(true));
        assert_eq!(route(&pod), Some((3, 1)));
        assert_eq!(route(&props(None, Some(true))), None);
    }

    fn enumerated(index: i32, description: &str, available: u32) -> Vec<u8> {
        let ints = |said: &[i32]| Value::ValueArray(ValueArray::Int(said.to_vec()));
        let mut properties = vec![
            keyed(sys::SPA_PARAM_ROUTE_index, Value::Int(index)),
            keyed(
                sys::SPA_PARAM_ROUTE_direction,
                Value::Id(Id(sys::SPA_DIRECTION_OUTPUT)),
            ),
            keyed(
                sys::SPA_PARAM_ROUTE_name,
                Value::String("analog-output".into()),
            ),
            keyed(sys::SPA_PARAM_ROUTE_devices, ints(&[index])),
            keyed(sys::SPA_PARAM_ROUTE_profiles, ints(&[1, 2])),
            keyed(sys::SPA_PARAM_ROUTE_available, Value::Id(Id(available))),
        ];
        if !description.is_empty() {
            let said = Value::String(description.into());
            properties.push(keyed(sys::SPA_PARAM_ROUTE_description, said));
        }
        written(&Value::Object(Object {
            type_: sys::SPA_TYPE_OBJECT_ParamRoute,
            id: sys::SPA_PARAM_EnumRoute,
            properties,
        }))
    }

    // The headphones' jack with nothing in it, as the laptop's card says
    // while none are plugged in, and the speakers, which it can't tell of.
    #[test]
    fn a_route_offered_says_its_port() {
        let headphones = enumerated(1, "Headphones", sys::SPA_PARAM_AVAILABILITY_no);
        let expected = Port {
            index: 1,
            sink: Some(true),
            name: "Headphones".into(),
            devices: vec![1],
            profiles: vec![1, 2],
            available: "no",
        };
        assert_eq!(offered(&headphones), Some(expected));
        let speakers = offered(&enumerated(0, "", sys::SPA_PARAM_AVAILABILITY_unknown));
        let said = speakers.map(|port| (port.name.clone(), port.available, port.unplugged()));
        assert_eq!(said, Some(("analog-output".into(), "unknown", false)));
        let plugged = offered(&enumerated(1, "", sys::SPA_PARAM_AVAILABILITY_yes));
        assert_eq!(plugged.map(|port| port.unplugged()), Some(false));
        assert_eq!(offered(&props(None, Some(true))), None);
    }

    #[test]
    fn a_profile_says_its_index() {
        let pod = written(&Value::Object(Object {
            type_: sys::SPA_TYPE_OBJECT_ParamProfile,
            id: sys::SPA_PARAM_Profile,
            properties: vec![keyed(sys::SPA_PARAM_PROFILE_index, Value::Int(2))],
        }));
        assert_eq!(profile(&pod), Some(2));
        assert_eq!(profile(&props(None, Some(true))), None);
    }

    #[test]
    fn a_port_chosen_reads_back_as_its_route() {
        assert_eq!(route(&ported(4, 1)), Some((4, 1)));
    }
}
