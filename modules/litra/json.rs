//! The lights as JSON, which `status --format json` says once and `watch`
//! again each time they change.

use std::path::Path;

use crate::found::About;
use crate::light::{BRIGHTEST, DIMMEST, State, percent};

/// Text as a JSON string holds it.
fn quoted(text: &str) -> String {
    use std::fmt::Write as _;
    let mut said = String::with_capacity(text.len() + 2);
    said.push('"');
    for c in text.chars() {
        match c {
            '"' => said.push_str("\\\""),
            '\\' => said.push_str("\\\\"),
            c if u32::from(c) < 0x20 => {
                let _ = write!(said, "\\u{:04x}", u32::from(c));
            }
            c => said.push(c),
        }
    }
    said.push('"');
    said
}

/// The lights that have said everything about themselves, as JSON: each
/// one's node, whether it's on, its share of the brightest and the lumens
/// that is, which its own units are, its warmth, and its name and serial
/// as it gives them.
pub(crate) fn json<'a>(
    lights: impl IntoIterator<Item = (&'a Path, &'a State, &'a About)>,
) -> String {
    let said: Vec<String> = lights
        .into_iter()
        .filter_map(|(node, state, about)| {
            let (on, units, kelvin) = (state.on?, state.units?, state.kelvin?);
            let range = if about.glow {
                format!(r#","dimmest":{DIMMEST},"brightest":{BRIGHTEST}"#)
            } else {
                String::new()
            };
            Some(format!(
                r#"{{"node":{},"on":{on},"percent":{},"lumens":{units}{range},"kelvin":{kelvin},"name":{},"serial":{}}}"#,
                quoted(&node.display().to_string()),
                percent(units),
                quoted(&about.name),
                quoted(&about.serial)
            ))
        })
        .collect();
    format!("[{}]", said.join(","))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_says_only_the_lights_that_have_said_everything() {
        let whole = State {
            on: Some(false),
            units: Some(DIMMEST + 39),
            kelvin: Some(5000),
        };
        let partial = State {
            on: Some(true),
            ..State::default()
        };
        let glow = About {
            name: "Logitech Litra Glow".to_owned(),
            serial: "2213FE40".to_owned(),
            glow: true,
        };
        let unknown = About::default();
        assert_eq!(json([]), "[]");
        assert_eq!(
            json([
                (Path::new("/dev/hidraw1"), &whole, &glow),
                (Path::new("/dev/hidraw4"), &partial, &glow),
            ]),
            r#"[{"node":"/dev/hidraw1","on":false,"percent":17,"lumens":59,"dimmest":20,"brightest":250,"kelvin":5000,"name":"Logitech Litra Glow","serial":"2213FE40"}]"#
        );
        assert_eq!(
            json([
                (Path::new("/dev/a\"b"), &whole, &unknown),
                (Path::new("/dev/c\\\u{1}"), &whole, &unknown)
            ]),
            r#"[{"node":"/dev/a\"b","on":false,"percent":17,"lumens":59,"kelvin":5000,"name":"","serial":""},{"node":"/dev/c\\\u0001","on":false,"percent":17,"lumens":59,"kelvin":5000,"name":"","serial":""}]"#
        );
    }
}
