//! The screen's light and the room's: each backlight the kernel lists in
//! `/sys/class/backlight` (`backlight:<device>`), its steps as set and as
//! the hardware reads them back, how its steps map to light, what drives
//! it and which screen it lights; and each ambient light sensor in
//! `/sys/bus/iio/devices` (`light:<device>`), its raw count, the scale
//! and offset that make it lux, and the lux they make. Displays reads
//! both from here, so the shell reads no file on its own thread, and the
//! sensor's slow raw read is the daemon's to wait on.

use serde_json::{Value, json};

use crate::Root;

const BACKLIGHTS: &str = "/sys/class/backlight";
const SENSORS: &str = "/sys/bus/iio/devices";

/// Every backlight's record, in the kernel's order of names.
pub fn backlights(root: &Root) -> Vec<Value> {
    root.names(BACKLIGHTS)
        .iter()
        .filter_map(|name| backlight(root, name))
        .collect()
}

/// A backlight's record, or none for one that says no most it can be.
fn backlight(root: &Root, name: &str) -> Option<Value> {
    let folder = format!("{BACKLIGHTS}/{name}");
    let file = |leaf: &str| format!("{folder}/{leaf}");
    let most = root
        .number(&file("max_brightness"))
        .filter(|most| *most > 0)?;
    Some(json!({
        "key": format!("backlight:{name}"),
        "kind": "backlight",
        "device": name,
        "brightness": root.number(&file("brightness")),
        "actual_brightness": root.number(&file("actual_brightness")),
        "max_brightness": most,
        // "raw", "platform" or "firmware", as the kernel says what sets it.
        "type": root.said(&file("type")),
        // "linear" or "non-linear"; the kernel's "unknown" says nothing.
        "scale": root.said(&file("scale")).filter(|scale| scale != "unknown"),
        "connector": connector(root, &file("device")),
    }))
}

/// The screen a backlight lights, where its device is a connector of a
/// graphics card, as the kernel names `card1-eDP-1`: `eDP-1`, as niri
/// names the output.
fn connector(root: &Root, device: &str) -> Option<String> {
    let path = root.resolved(device)?;
    let leaf = path.rsplit('/').next()?;
    let (card, output) = leaf.split_once('-')?;
    let numbered = card
        .strip_prefix("card")
        .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()));
    (numbered && !output.is_empty()).then(|| output.to_owned())
}

/// Every ambient light sensor's record, a sensor being a device the
/// kernel names `als` that says its illuminance's scale, as Displays
/// has always found it.
pub fn sensors(root: &Root) -> Vec<Value> {
    root.names(SENSORS)
        .iter()
        .filter_map(|device| sensor(root, device))
        .collect()
}

fn sensor(root: &Root, device: &str) -> Option<Value> {
    let folder = format!("{SENSORS}/{device}");
    let file = |leaf: &str| format!("{folder}/{leaf}");
    let name = root.said(&file("name")).filter(|name| name == "als")?;
    let scale = figure(root.said(&file("in_illuminance_scale")))?;
    let offset = figure(root.said(&file("in_illuminance_offset"))).unwrap_or(0.0);
    let raw = figure(root.said(&file("in_illuminance_raw")));
    Some(json!({
        "key": format!("light:{device}"),
        "kind": "light",
        "device": device,
        "name": name,
        "raw": raw,
        "scale": scale,
        "offset": offset,
        "lux": raw.map(|raw| (raw + offset) * scale),
    }))
}

/// A figure a sensor's file says, which may be a fraction, as a scale is.
fn figure(said: Option<String>) -> Option<f64> {
    said?.parse::<f64>().ok().filter(|value| value.is_finite())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use super::*;

    fn write(root: &Path, path: &str, text: &str) {
        let path = root.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    #[test]
    fn a_backlight_says_its_steps_its_curve_and_its_screen() {
        let root = std::env::temp_dir().join(format!("machine-light-{}", std::process::id()));
        let card = "sys/devices/pci0000:00/0000:00:02.0/drm/card1/card1-eDP-1";
        write(
            &root,
            &format!("{card}/intel_backlight/brightness"),
            "96000\n",
        );
        write(
            &root,
            &format!("{card}/intel_backlight/actual_brightness"),
            "95800\n",
        );
        write(
            &root,
            &format!("{card}/intel_backlight/max_brightness"),
            "120000\n",
        );
        write(&root, &format!("{card}/intel_backlight/type"), "raw\n");
        write(
            &root,
            &format!("{card}/intel_backlight/scale"),
            "non-linear\n",
        );
        fs::create_dir_all(root.join("sys/class/backlight")).unwrap();
        std::os::unix::fs::symlink(
            root.join(format!("{card}/intel_backlight")),
            root.join("sys/class/backlight/intel_backlight"),
        )
        .unwrap();
        std::os::unix::fs::symlink(
            root.join(card),
            root.join(format!("{card}/intel_backlight/device")),
        )
        .unwrap();
        // A backlight with no most it can be says nothing.
        write(&root, "sys/class/backlight/broken/brightness", "3\n");
        let records = backlights(&Root::new(&root));
        fs::remove_dir_all(&root).unwrap();
        assert_eq!(records.len(), 1, "one backlight says its most: {records:?}");
        let light = &records[0];
        assert_eq!(light["key"], "backlight:intel_backlight");
        assert_eq!(light["brightness"], json!(96000));
        assert_eq!(light["actual_brightness"], json!(95800));
        assert_eq!(light["max_brightness"], json!(120_000));
        assert_eq!(light["type"], "raw");
        assert_eq!(light["scale"], "non-linear");
        assert_eq!(
            light["connector"], "eDP-1",
            "the card's connector names its screen"
        );
    }

    #[test]
    fn a_backlight_with_no_screen_or_curve_says_neither() {
        let root = std::env::temp_dir().join(format!("machine-plain-{}", std::process::id()));
        write(&root, "sys/class/backlight/acpi_video0/brightness", "5\n");
        write(
            &root,
            "sys/class/backlight/acpi_video0/max_brightness",
            "10\n",
        );
        write(&root, "sys/class/backlight/acpi_video0/scale", "unknown\n");
        write(&root, "sys/class/backlight/acpi_video0/type", "firmware\n");
        let records = backlights(&Root::new(&root));
        fs::remove_dir_all(&root).unwrap();
        assert_eq!(records[0]["type"], "firmware");
        assert_eq!(records[0]["scale"], Value::Null, "unknown says no curve");
        assert_eq!(records[0]["connector"], Value::Null, "no device, no screen");
        assert_eq!(records[0]["actual_brightness"], Value::Null);
    }

    #[test]
    fn the_room_is_its_count_made_lux() {
        let root = std::env::temp_dir().join(format!("machine-room-{}", std::process::id()));
        let als = "sys/bus/iio/devices/iio:device0";
        write(&root, &format!("{als}/name"), "als\n");
        write(&root, &format!("{als}/in_illuminance_raw"), "1234\n");
        write(&root, &format!("{als}/in_illuminance_scale"), "0.1\n");
        write(&root, &format!("{als}/in_illuminance_offset"), "6\n");
        // An accelerometer is no light sensor.
        write(&root, "sys/bus/iio/devices/iio:device1/name", "accel_3d\n");
        let records = sensors(&Root::new(&root));
        fs::remove_dir_all(&root).unwrap();
        assert_eq!(records.len(), 1, "only the light sensor: {records:?}");
        let room = &records[0];
        assert_eq!(room["key"], "light:iio:device0");
        assert_eq!(room["name"], "als");
        assert_eq!(room["raw"], json!(1234.0));
        assert_eq!(room["scale"], json!(0.1));
        assert_eq!(room["offset"], json!(6.0));
        let lux = room["lux"].as_f64().unwrap();
        assert!(
            (lux - 124.0).abs() < 1e-9,
            "(1234 + 6) x 0.1 is 124 lx, not {lux}"
        );
    }

    #[test]
    fn a_sensor_unread_says_no_lux() {
        let root = std::env::temp_dir().join(format!("machine-unread-{}", std::process::id()));
        let als = "sys/bus/iio/devices/iio:device0";
        write(&root, &format!("{als}/name"), "als\n");
        write(&root, &format!("{als}/in_illuminance_scale"), "0.1\n");
        let records = sensors(&Root::new(&root));
        fs::remove_dir_all(&root).unwrap();
        assert_eq!(records[0]["raw"], Value::Null);
        assert_eq!(records[0]["lux"], Value::Null, "no count, no lux");
        assert_eq!(records[0]["offset"], json!(0.0), "no offset is none");
    }
}
