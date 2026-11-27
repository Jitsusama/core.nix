//! Finding the lights plugged in, by the hidraw nodes sysfs lists, and what
//! each one's HID device says of it.

use std::fs;
use std::path::{Path, PathBuf};

/// The lights this speaks to, by the `HID_ID` hidraw's uevent gives them:
/// the Glow, and the Beam, which takes the same reports.
const LIGHTS: [&str; 2] = [
    "HID_ID=0003:0000046D:0000C900",
    "HID_ID=0003:0000046D:0000C901",
];

/// Every light plugged in, as its node in `dev`, by the hidraw nodes `sys`
/// lists.
pub(crate) fn lights(sys: &Path, dev: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(sys.join("class/hidraw")) else {
        return Vec::new();
    };
    let mut names: Vec<_> = entries.flatten().map(|entry| entry.file_name()).collect();
    names.sort();
    names
        .into_iter()
        .filter(|name| {
            let uevent = sys.join("class/hidraw").join(name).join("device/uevent");
            fs::read_to_string(uevent)
                .is_ok_and(|text| text.lines().any(|line| LIGHTS.contains(&line)))
        })
        .map(|name| dev.join(name))
        .collect()
}

/// What a light says of itself when it's plugged in, by the HID device
/// behind its node: its name ("Logitech Litra Glow") and its serial, each
/// empty where it gives none, and whether it's a Glow, whose range in
/// lumens this knows. A Beam's is wider, 30 to 400, and this maps it as a
/// Glow's, so it says no range for one rather than a wrong one.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct About {
    pub(crate) name: String,
    pub(crate) serial: String,
    pub(crate) glow: bool,
}

pub(crate) fn about(sys: &Path, node: &Path) -> About {
    let Some(name) = node.file_name() else {
        return About::default();
    };
    let uevent = sys.join("class/hidraw").join(name).join("device/uevent");
    let text = fs::read_to_string(uevent).unwrap_or_default();
    let field = |key: &str| {
        text.lines()
            .find_map(|line| line.strip_prefix(key)?.strip_prefix('='))
            .unwrap_or_default()
            .trim()
            .to_owned()
    };
    About {
        name: field("HID_NAME"),
        serial: field("HID_UNIQ"),
        glow: text.lines().any(|line| line == LIGHTS[0]),
    }
}

/// Every light plugged in, each with the HID device behind its node. hidraw
/// gives a light plugged back in the name it had before, as nothing took it
/// meanwhile, but the HID device is made anew at every plug, so it tells a
/// light just plugged in, which comes up dark, from one that never left.
pub(crate) fn plugged(sys: &Path, dev: &Path) -> Vec<(PathBuf, Option<PathBuf>)> {
    lights(sys, dev)
        .into_iter()
        .map(|node| {
            let device = node.file_name().and_then(|name| {
                fs::canonicalize(sys.join("class/hidraw").join(name).join("device")).ok()
            });
            (node, device)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::*;

    fn node(root: &Path, name: &str, id: &str) {
        let device = root.join("sys/class/hidraw").join(name).join("device");
        fs::create_dir_all(&device).unwrap();
        fs::write(
            device.join("uevent"),
            format!("DRIVER=hid-generic\nHID_ID={id}\n"),
        )
        .unwrap();
    }

    #[test]
    fn the_lights_are_found_by_their_ids_and_nothing_else() {
        let root = root("found");
        node(&root, "hidraw0", "0003:0000046D:0000091D");
        node(&root, "hidraw1", "0003:0000046D:0000C900");
        node(&root, "hidraw2", "0003:00003297:00004976");
        node(&root, "hidraw3", "0003:0000046D:0000C901");
        assert_eq!(
            lights(&root.join("sys"), Path::new("/dev")),
            [PathBuf::from("/dev/hidraw1"), PathBuf::from("/dev/hidraw3")]
        );
        assert_eq!(
            lights(&root.join("nothing"), Path::new("/dev")),
            [] as [std::path::PathBuf; 0]
        );
    }

    #[test]
    fn a_light_says_its_name_and_serial_as_its_hid_device_gives_them() {
        let root = root("about");
        let device = root.join("sys/class/hidraw/hidraw1/device");
        fs::create_dir_all(&device).unwrap();
        fs::write(
            device.join("uevent"),
            "HID_ID=0003:0000046D:0000C900\nHID_NAME=Logitech Litra Glow\nHID_UNIQ=2213FE40\n",
        )
        .unwrap();
        assert_eq!(
            about(&root.join("sys"), Path::new("/dev/hidraw1")),
            About {
                name: "Logitech Litra Glow".to_owned(),
                serial: "2213FE40".to_owned(),
                glow: true,
            }
        );
        node(&root, "hidraw2", "0003:0000046D:0000C901");
        assert_eq!(
            about(&root.join("sys"), Path::new("/dev/hidraw2")),
            About::default()
        );
    }

    /// A light's node as the kernel makes it: hidraw's entry linked to the
    /// HID device behind it, which is numbered anew at each plug.
    fn plugged_in(root: &Path, name: &str, device: &str) {
        let hid = root.join("sys/devices/virtual/misc/uhid").join(device);
        fs::create_dir_all(&hid).unwrap();
        fs::write(hid.join("uevent"), "HID_ID=0003:0000046D:0000C900\n").unwrap();
        let entry = root.join("sys/class/hidraw").join(name);
        let _ = fs::remove_dir_all(&entry);
        fs::create_dir_all(&entry).unwrap();
        std::os::unix::fs::symlink(&hid, entry.join("device")).unwrap();
    }

    #[test]
    fn a_light_plugged_back_in_under_its_old_name_is_told_from_one_that_stayed() {
        let root = root("replugged");
        plugged_in(&root, "hidraw0", "0003:046D:C900.0001");
        let before = plugged(&root.join("sys"), Path::new("/dev"));
        assert_eq!(before, plugged(&root.join("sys"), Path::new("/dev")));
        plugged_in(&root, "hidraw0", "0003:046D:C900.0002");
        let after = plugged(&root.join("sys"), Path::new("/dev"));
        assert_eq!(
            lights(&root.join("sys"), Path::new("/dev")),
            [PathBuf::from("/dev/hidraw0")]
        );
        assert_ne!(before, after);
    }
}
