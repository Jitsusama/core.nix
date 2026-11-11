//! What Screens asks of the screen's light, done here so the shell spawns
//! nothing (Y2YBTS, 2026-11-10): `brightness` sets a backlight to a share
//! of its most through logind's session, as `brightnessctl
//! --class=backlight set N%` did, and `night-light` and `follow-the-room`
//! switch the backlight program's own properties on the session's bus
//! (modules/backlight's switch.rs), as `busctl --user set-property` did.

use std::time::Duration;

use serde_json::Value;

use crate::Root;
use crate::acts::{Refused, refused};
use crate::light;

/// How long logind or the backlight program is given to answer. Each
/// answers from what it holds, in a millisecond.
const ASKING: Duration = Duration::from_secs(2);

/// Where the backlight program answers on the session's bus: its name, its
/// object and its interface.
const BACKLIGHT: [&str; 3] = [
    "io.jitsusama.Backlight",
    "/io/jitsusama/Backlight",
    "io.jitsusama.Backlight",
];

/// The backlight a brightness act sets and the steps it sets it to: the
/// device named, or the first the kernel lists, as brightnessctl took;
/// and the share of its most, never below one step, since none leaves
/// the screen black.
pub fn aimed(root: &Root, said: &Value) -> Result<(String, u32), Refused> {
    let Some(share) = said["share"].as_u64().filter(|share| *share <= 100) else {
        return Err(refused(
            "bad-act",
            "a brightness needs its share, a whole number from 0 to 100",
        ));
    };
    let named = said["device"].as_str();
    let lights = light::backlights(root);
    let Some(light) = lights
        .iter()
        .find(|light| named.is_none_or(|name| light["device"] == name))
    else {
        return Err(refused(
            "bad-act",
            format!(
                "no backlight {}here",
                named.map(|name| format!("{name} ")).unwrap_or_default()
            ),
        ));
    };
    let most = light["max_brightness"].as_u64().unwrap_or(1);
    let steps = (share * most).div_ceil(100).max(1);
    Ok((
        light["device"].as_str().unwrap_or_default().to_owned(),
        u32::try_from(steps).unwrap_or(u32::MAX),
    ))
}

/// Sets the backlight to its steps through logind's session, which lets
/// the session's own programs set its seat's backlight without asking.
pub fn brighten(device: &str, steps: u32) -> Result<(), Refused> {
    let unreached = |error: zbus::Error| refused("refused", error.to_string());
    let bus = zbus::blocking::connection::Builder::system()
        .and_then(|builder| builder.method_timeout(ASKING).build())
        .map_err(unreached)?;
    let session = zbus::blocking::Proxy::new(
        &bus,
        "org.freedesktop.login1",
        "/org/freedesktop/login1/session/auto",
        "org.freedesktop.login1.Session",
    )
    .map_err(unreached)?;
    session
        .call::<_, _, ()>("SetBrightness", &("backlight", device, steps))
        .map_err(unreached)
}

/// The backlight program's property a switch sets, and to what: the night
/// light by `on`, following the room by `follows`.
pub fn switched(act: &str, said: &Value) -> Result<(&'static str, bool), Refused> {
    let (property, given) = match act {
        "night-light" => ("NightLight", "on"),
        "follow-the-room" => ("FollowsTheRoom", "follows"),
        _ => return Err(refused("unknown-act", format!("no act {act:?} here"))),
    };
    said[given]
        .as_bool()
        .map(|on| (property, on))
        .ok_or_else(|| refused("bad-act", format!("{act} needs {given}, true or false")))
}

/// Sets the backlight program's property, which writes its warmth file or
/// steers the screen again as it changes, so a watch hears of it there.
pub fn switch(property: &str, on: bool) -> Result<(), Refused> {
    let [service, path, interface] = BACKLIGHT;
    let bus = zbus::blocking::connection::Builder::session()
        .and_then(|builder| builder.method_timeout(ASKING).build())
        .map_err(|error| refused("refused", error.to_string()))?;
    let backlight = zbus::blocking::Proxy::new(&bus, service, path, interface)
        .map_err(|error| refused("refused", error.to_string()))?;
    backlight
        .set_property(property, on)
        .map_err(|error| refused("refused", error.to_string()))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use serde_json::json;

    use super::*;

    /// A machine with two backlights, the panel's first as the kernel lists
    /// them, each by its steps now and its most.
    fn lit(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("machine-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        for (device, now, most) in [("intel_backlight", 96_000, 120_000), ("z_monitor", 3, 7)] {
            let folder = root.join("sys/class/backlight").join(device);
            fs::create_dir_all(&folder).expect("a backlight's folder is made");
            fs::write(folder.join("brightness"), format!("{now}\n"))
                .expect("its steps are written");
            fs::write(folder.join("max_brightness"), format!("{most}\n"))
                .expect("its most is written");
        }
        root
    }

    #[test]
    fn a_share_sets_the_first_backlight_and_never_below_a_step() {
        let root = Root::new(lit("share"));
        assert_eq!(
            aimed(&root, &json!({"share": 50})),
            Ok(("intel_backlight".to_owned(), 60_000)),
            "half of the panel's most"
        );
        assert_eq!(
            aimed(&root, &json!({"share": 0})),
            Ok(("intel_backlight".to_owned(), 1)),
            "none still lights the screen a step"
        );
        assert_eq!(
            aimed(&root, &json!({"share": 30, "device": "z_monitor"})),
            Ok(("z_monitor".to_owned(), 3)),
            "a share of a few steps rounds up, as a share asked is never darker"
        );
    }

    #[test]
    fn a_brightness_without_its_share_or_backlight_is_refused() {
        let root = Root::new(lit("refused"));
        for said in [json!({}), json!({"share": 101}), json!({"share": "half"})] {
            let refusal = aimed(&root, &said).expect_err("a brightness needs a share");
            assert_eq!(refusal.code, "bad-act", "{said} isn't a share");
        }
        let unknown = aimed(&root, &json!({"share": 50, "device": "acpi_video0"}))
            .expect_err("no such backlight");
        assert_eq!(unknown.message, "no backlight acpi_video0 here");
        let none = aimed(
            &Root::new(std::env::temp_dir().join("machine-unlit")),
            &json!({"share": 50}),
        )
        .expect_err("a machine with no backlight");
        assert_eq!(none.message, "no backlight here");
    }

    #[test]
    fn a_switch_names_the_backlight_programs_property() {
        assert_eq!(
            switched("night-light", &json!({"on": true})),
            Ok(("NightLight", true)),
            "the night light by on"
        );
        assert_eq!(
            switched("follow-the-room", &json!({"follows": false})),
            Ok(("FollowsTheRoom", false)),
            "following the room by follows"
        );
        let unsaid = switched("night-light", &json!({"follows": true}))
            .expect_err("the night light needs on");
        assert_eq!(unsaid.code, "bad-act");
    }
}
