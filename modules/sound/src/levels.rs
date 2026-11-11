//! How loud a node is set, read from the Props it says: each channel's
//! volume and whether it's muted. A Props names only what it says, so
//! either may be missing, and a missing one keeps what was said before.

use pipewire::spa::pod::deserialize::PodDeserializer;
use pipewire::spa::pod::{Value, ValueArray};
use pipewire::spa::sys::{SPA_PROP_channelVolumes, SPA_PROP_mute};

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
                if property.key == SPA_PROP_channelVolumes =>
            {
                volumes = Some(said);
            }
            Value::Bool(said) if property.key == SPA_PROP_mute => muted = Some(said),
            _ => {}
        }
    }
    (volumes, muted)
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use pipewire::spa::pod::serialize::PodSerializer;
    use pipewire::spa::pod::{Object, Property, PropertyFlags};
    use pipewire::spa::sys::{SPA_PARAM_Props, SPA_PROP_volume, SPA_TYPE_OBJECT_Props};

    use super::*;

    fn props(properties: Vec<(u32, Value)>) -> Vec<u8> {
        let object = Value::Object(Object {
            type_: SPA_TYPE_OBJECT_Props,
            id: SPA_PARAM_Props,
            properties: properties
                .into_iter()
                .map(|(key, value)| Property {
                    key,
                    flags: PropertyFlags::empty(),
                    value,
                })
                .collect(),
        });
        PodSerializer::serialize(Cursor::new(Vec::new()), &object)
            .expect("the pod is written")
            .0
            .into_inner()
    }

    #[test]
    fn a_props_says_each_channels_volume_and_the_mute() {
        let pod = props(vec![
            (SPA_PROP_volume, Value::Float(1.0)),
            (SPA_PROP_mute, Value::Bool(true)),
            (
                SPA_PROP_channelVolumes,
                Value::ValueArray(ValueArray::Float(vec![0.25, 0.5])),
            ),
        ]);
        assert_eq!(levels(&pod), (Some(vec![0.25, 0.5]), Some(true)));
    }

    #[test]
    fn what_a_props_leaves_out_is_none() {
        assert_eq!(
            levels(&props(vec![(SPA_PROP_mute, Value::Bool(false))])),
            (None, Some(false))
        );
        assert_eq!(levels(&[1, 2, 3]), (None, None));
    }
}
