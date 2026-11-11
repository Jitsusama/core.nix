//! How loud a node is set, read from the Props it says and written into
//! the Props or the Route an act sets: each channel's volume and whether
//! it's muted. A Props names only what it says, so either may be missing,
//! and a missing one keeps what was said before. A card's device says
//! which of its routes plays each of its profile's devices, read here too.

use std::io::Cursor;

use pipewire::spa::pod::deserialize::PodDeserializer;
use pipewire::spa::pod::serialize::PodSerializer;
use pipewire::spa::pod::{Object, Property, PropertyFlags, Value, ValueArray};
use pipewire::spa::sys;

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
}
