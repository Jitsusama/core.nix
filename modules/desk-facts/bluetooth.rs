//! Bluetooth input devices, from the kernel's HID and input layers.

use std::fs;
use std::path::Path;

use crate::Facts;
use crate::input::{BLUETOOTH_BUS, Heard, set_bits};
use crate::usb::release;

/// A HID device on the Bluetooth bus: its address, as bluetoothd writes it,
/// and how it's carried, its ids, and what its inputs take. One that
/// bluetoothd made through uhid is HID over GATT, on LE; the kernel's own
/// HIDP is Bluetooth Classic's.
fn hid_device(hid: &Path) -> Option<(String, Heard)> {
    let uevent = fs::read_to_string(hid.join("uevent")).ok()?;
    let field = |name: &str| {
        uevent
            .lines()
            .find_map(|line| line.strip_prefix(name)?.strip_prefix('='))
            .map(str::trim)
    };
    let address = field("HID_UNIQ")?.to_uppercase();
    // Its bus, vendor and product, as "0005:000025A7:0000FAA0".
    let mut ids = field("HID_ID")?.split(':');
    let (Some(BLUETOOTH_BUS), Some(vendor), Some(product)) = (ids.next(), ids.next(), ids.next())
    else {
        return None;
    };
    if address.is_empty() {
        return None;
    }
    let last = |id: &str| {
        id.get(id.len().saturating_sub(4)..)
            .unwrap_or(id)
            .to_lowercase()
    };
    let over_gatt = fs::canonicalize(hid)
        .is_ok_and(|real| real.components().any(|part| part.as_os_str() == "uhid"));
    let mut heard = Heard {
        link: if over_gatt {
            "LE, HID over GATT"
        } else {
            "Classic, HIDP"
        }
        .to_owned(),
        id: format!("{}:{}", last(vendor), last(product)),
        ..Heard::default()
    };
    let inputs = fs::read_dir(hid.join("input"))
        .into_iter()
        .flatten()
        .flatten();
    for input in inputs.map(|entry| entry.path()) {
        let read = |name: &str| fs::read_to_string(input.join(name)).unwrap_or_default();
        if heard.release.is_empty() {
            let version = read("id/version");
            if !version.trim().is_empty() {
                heard.release = release(&version);
            }
        }
        heard.keys.extend(set_bits(&read("capabilities/key")));
        heard.axes.extend(set_bits(&read("capabilities/rel")));
    }
    Some((address, heard))
}

/// Each Bluetooth input device connected, by its address, from its HID
/// devices: a device with several, as a keyboard with a touchpad has, is
/// told as one.
pub(crate) fn bluetooth(sys: &Path) -> Vec<(String, Facts)> {
    let Ok(entries) = fs::read_dir(sys.join("bus/hid/devices")) else {
        return Vec::new();
    };
    let mut devices: Vec<(String, Heard)> = Vec::new();
    for (address, heard) in entries
        .flatten()
        .filter_map(|entry| hid_device(&entry.path()))
    {
        match devices.iter_mut().find(|(each, _)| *each == address) {
            Some((_, was)) => {
                was.keys.extend(heard.keys);
                was.axes.extend(heard.axes);
                if was.release.is_empty() {
                    was.release = heard.release;
                }
            }
            None => devices.push((address, heard)),
        }
    }
    let mut found: Vec<(String, Facts)> = devices
        .into_iter()
        .map(|(address, heard)| (address, heard.facts()))
        .collect();
    found.sort();
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::*;

    #[test]
    fn a_bluetooth_mouse_says_how_it_s_carried_and_what_it_takes() {
        let sys = Sys::new("mouse");
        sys.hid(
            "devices/virtual/misc/uhid",
            "0005:25A7:FAA0.0001",
            "DRIVER=hid-generic\nHID_ID=0005:000025A7:0000FAA0\nHID_NAME=ProtoArc EM11 NL\n\
             HID_PHYS=b4:8c:9d:00:00:01\nHID_UNIQ=c3:9c:e3:2a:f3:6e\n",
            ["0508\n", "1f0000 0 0 0 0\n", "903\n"],
        );
        let said = |pairs: &[(&str, &str)]| -> Facts {
            pairs
                .iter()
                .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
                .collect()
        };
        assert_eq!(
            bluetooth(&sys.0),
            vec![(
                "C3:9C:E3:2A:F3:6E".to_owned(),
                said(&[
                    ("connection", "LE, HID over GATT"),
                    ("buttons", "5"),
                    ("wheel", "vertical"),
                    ("id", "25a7:faa0"),
                    ("release", "5.08"),
                ])
            )]
        );
    }

    #[test]
    fn a_classic_keyboard_says_its_keys_and_a_usb_device_is_left_out() {
        // The USB keyboard has a serial, so only its bus leaves it out.
        let sys = Sys::new("keyboard");
        sys.hid(
            "devices/pci0000:00/0000:00:14.7/bluetooth/hci0/hci0:3",
            "0005:05AC:0255.0002",
            "HID_ID=0005:000005AC:00000255\nHID_UNIQ=aa:bb:cc:dd:ee:ff\n",
            ["0050\n", "fffffffffffffffe ffffffffffffffff\n", "0\n"],
        );
        sys.hid(
            "devices/pci0000:00/0000:00:14.0/usb3/3-1",
            "0003:3297:4976.0003",
            "HID_ID=0003:00003297:00004976\nHID_UNIQ=0C2A8F31\n",
            ["0100\n", "fffffffffffffffe ffffffffffffffff\n", "0\n"],
        );
        let found = bluetooth(&sys.0);
        assert_eq!(found.len(), 1, "{found:?}");
        let (address, facts) = &found[0];
        assert_eq!(address, "AA:BB:CC:DD:EE:FF");
        assert_eq!(
            facts[0],
            ("connection".to_owned(), "Classic, HIDP".to_owned())
        );
        // Bits 0 to 63 and 65 to 127, of which 126 are keys from KEY_ESC.
        assert!(
            facts.contains(&("keys".to_owned(), "126".to_owned())),
            "{facts:?}"
        );
        assert!(
            !facts
                .iter()
                .any(|(name, _)| name == "buttons" || name == "wheel"),
            "a keyboard says no buttons or wheel: {facts:?}"
        );
    }

    #[test]
    fn a_device_with_several_hid_devices_is_told_as_one() {
        let sys = Sys::new("several");
        let home = "devices/pci0000:00/0000:00:14.7/bluetooth/hci0/hci0:3";
        let uevent = "HID_ID=0005:000005AC:00000265\nHID_UNIQ=aa:bb:cc:dd:ee:ff\n";
        // A keyboard's keys, and a touchpad's five buttons and both wheels.
        sys.hid(
            home,
            "0005:05AC:0265.0004",
            uevent,
            ["0110\n", "fffffffffffffffe ffffffffffffffff\n", "0\n"],
        );
        sys.hid(
            home,
            "0005:05AC:0265.0005",
            uevent,
            ["0110\n", "1f0000 0 0 0 0\n", "943\n"],
        );
        let said: Vec<(String, String)> = [
            ("connection", "Classic, HIDP"),
            ("buttons", "5"),
            ("wheel", "vertical and horizontal"),
            ("keys", "126"),
            ("id", "05ac:0265"),
            ("release", "1.10"),
        ]
        .into_iter()
        .map(|(name, value)| (name.to_owned(), value.to_owned()))
        .collect();
        assert_eq!(
            bluetooth(&sys.0),
            vec![("AA:BB:CC:DD:EE:FF".to_owned(), said)],
            "a device's HID devices weren't told as one"
        );
    }
}
