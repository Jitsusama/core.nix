//! Keyboards and mice not on Bluetooth, from the input and USB layers.

use std::fs;
use std::path::{Path, PathBuf};

use crate::bluetooth::BLUETOOTH_BUS;
use crate::input::{FEWEST_KEYS, Heard, KEYBOARD_KEYS, MOUSE_BUTTONS, set_bits};
use crate::usb::release;
use crate::{Facts, fact};

/// The relative axes a mouse moves on, `REL_X` and `REL_Y`.
const REL_X: usize = 0x00;
const REL_Y: usize = 0x01;
/// A mouse's first button, `BTN_LEFT`.
const BTN_LEFT: usize = 0x110;
/// The USB bus, as the input layer numbers it.
const USB_BUS: &str = "0003";

/// How often a USB endpoint is polled, from its interval as sysfs writes
/// it ("1ms", "125us"): "1000 a second".
fn polling_rate(interval: &str) -> String {
    let interval = interval.trim();
    let micros = if let Some(us) = interval.strip_suffix("us") {
        us.parse::<u32>().ok()
    } else if let Some(ms) = interval.strip_suffix("ms") {
        ms.parse::<u32>().ok().and_then(|ms| ms.checked_mul(1000))
    } else {
        None
    };
    match micros {
        Some(micros) if micros > 0 => format!("{} a second", 1_000_000 / micros),
        _ => String::new(),
    }
}

/// The polling rate of the first interrupt endpoint that reads from a USB
/// interface, which is how often a HID device is asked for its reports.
fn interface_rate(interface: &Path) -> String {
    let Ok(entries) = fs::read_dir(interface) else {
        return String::new();
    };
    let mut endpoints: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .is_some_and(|name| name.to_string_lossy().starts_with("ep_"))
        })
        .collect();
    endpoints.sort();
    endpoints
        .iter()
        .find(|endpoint| {
            let read = |name: &str| fs::read_to_string(endpoint.join(name)).unwrap_or_default();
            read("type").trim() == "Interrupt" && read("direction").trim() == "in"
        })
        .map(|endpoint| {
            polling_rate(&fs::read_to_string(endpoint.join("interval")).unwrap_or_default())
        })
        .unwrap_or_default()
}

/// One input device's facts, from its folder in /sys/class/input: a
/// keyboard's polling rate, keys and firmware, and a USB mouse's polling
/// rate, buttons, wheel and firmware. Nothing for a device on Bluetooth,
/// which `bluetooth` tells, one made by a program, such as keyd's, or one
/// that's neither a keyboard nor a USB mouse.
fn input_device(input: &Path) -> Option<(String, Facts)> {
    let read = |name: &str| fs::read_to_string(input.join(name)).unwrap_or_default();
    let bus = read("id/bustype");
    let bus = bus.trim();
    let home = fs::canonicalize(input).ok()?;
    if bus == BLUETOOTH_BUS || home.components().any(|part| part.as_os_str() == "virtual") {
        return None;
    }
    let keys = set_bits(&read("capabilities/key"));
    let axes = set_bits(&read("capabilities/rel"));
    let keyboard = keys
        .iter()
        .filter(|key| KEYBOARD_KEYS.contains(key))
        .count();
    let mouse = bus == USB_BUS
        && keys.contains(&BTN_LEFT)
        && axes.contains(&REL_X)
        && axes.contains(&REL_Y);
    if keyboard < FEWEST_KEYS && !mouse {
        return None;
    }
    let name = read("name").trim().to_owned();
    if name.is_empty() {
        return None;
    }
    let mut facts = Facts::new();
    // An input's device is its HID device, whose parent is the USB
    // interface the reports come through, and that interface's parent the
    // USB device.
    let interface = input.join("device/..");
    if bus == USB_BUS {
        fact(&mut facts, "polling rate", interface_rate(&interface));
    }
    // A keyboard is told by its keys alone, though its firmware may offer
    // a mouse's buttons beside them, and a mouse by its buttons and wheel.
    let heard = if mouse {
        Heard {
            keys: keys
                .into_iter()
                .filter(|key| MOUSE_BUTTONS.contains(key))
                .collect(),
            axes,
            ..Heard::default()
        }
    } else {
        Heard {
            keys: keys
                .into_iter()
                .filter(|key| KEYBOARD_KEYS.contains(key))
                .collect(),
            ..Heard::default()
        }
    };
    facts.extend(heard.facts());
    let firmware = if bus == USB_BUS {
        fs::read_to_string(interface.join("../bcdDevice")).unwrap_or_default()
    } else if bus == "0018" {
        // An I2C HID device's version is its firmware's, from its HID
        // descriptor.
        read("id/version")
    } else {
        String::new()
    };
    if !firmware.trim().is_empty() {
        fact(
            &mut facts,
            "firmware",
            format!("release {}", release(&firmware)),
        );
    }
    Some((name, facts))
}

/// Every keyboard, and every USB mouse, not on Bluetooth, by its name as
/// the kernel gives it, from the input devices `sys` lists. Two inputs of
/// one name, as a keyboard's several interfaces can be, are told as the
/// first.
pub(crate) fn keyboards(sys: &Path) -> Vec<(String, Facts)> {
    let Ok(entries) = fs::read_dir(sys.join("class/input")) else {
        return Vec::new();
    };
    let mut inputs: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .is_some_and(|name| name.to_string_lossy().starts_with("input"))
        })
        .collect();
    inputs.sort();
    let mut found: Vec<(String, Facts)> = Vec::new();
    for (name, facts) in inputs.iter().filter_map(|input| input_device(input)) {
        if !found.iter().any(|(each, _)| *each == name) {
            found.push((name, facts));
        }
    }
    found.sort();
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::*;

    /// An input device where the kernel keeps it, under `home`, with its
    /// name, bus and capabilities, linked from /sys/class/input and to its
    /// HID device as the kernel links them.
    fn input_at(sys: &Path, home: &str, number: u32, said: [&str; 4]) -> PathBuf {
        let [name, bus, keys, axes] = said;
        let real = sys.join(home).join(format!("input/input{number}"));
        put(&real, "name", &format!("{name}\n"));
        put(&real, "id/bustype", &format!("{bus}\n"));
        put(&real, "id/version", "0100\n");
        put(&real, "capabilities/key", &format!("{keys}\n"));
        put(&real, "capabilities/rel", &format!("{axes}\n"));
        std::os::unix::fs::symlink("../..", real.join("device")).unwrap();
        fs::create_dir_all(sys.join("class/input")).unwrap();
        std::os::unix::fs::symlink(&real, sys.join(format!("class/input/input{number}"))).unwrap();
        real
    }

    /// A USB device's interrupt endpoint, polled at an interval, on one of
    /// its interfaces, and the device's release.
    fn endpoint(sys: &Path, usb: &str, interface: &str, said: [&str; 2]) {
        let [interval, release] = said;
        let endpoint = format!("{usb}/{interface}/ep_81");
        put(sys, &format!("{endpoint}/type"), "Interrupt\n");
        put(sys, &format!("{endpoint}/direction"), "in\n");
        put(
            sys,
            &format!("{endpoint}/interval"),
            &format!("{interval}\n"),
        );
        put(sys, &format!("{usb}/bcdDevice"), &format!("{release}\n"));
    }

    #[test]
    fn a_keyboard_and_a_usb_mouse_say_how_often_they_re_polled_and_what_they_take() {
        let sys = Sys::new("keyboards");
        let usb = "devices/pci0000:00/usb3/3-1";
        // The 63 keys from KEY_ESC, bits 1 to 63, and BTN_LEFT, which a
        // keyboard's key count leaves out.
        let keys = "10000 0 0 0 fffffffffffffffe";
        let ergodox = format!("{usb}/3-1:1.0/0003:3297:4976.0001");
        input_at(
            &sys.0,
            &ergodox,
            5,
            ["ZSA Technology Labs ErgoDox EZ", "0003", keys, "0"],
        );
        endpoint(&sys.0, usb, "3-1:1.0", ["1ms", "0001"]);
        // The same keyboard's second interface, of the same name.
        let second = format!("{usb}/3-1:1.1/0003:3297:4976.0002");
        input_at(
            &sys.0,
            &second,
            6,
            ["ZSA Technology Labs ErgoDox EZ", "0003", keys, "0"],
        );
        // A mouse: BTN_LEFT to BTN_EXTRA, REL_X, REL_Y and REL_WHEEL.
        let mouse = "devices/pci0000:00/usb3/3-2/3-2:1.0/0003:046D:C077.0003";
        input_at(
            &sys.0,
            mouse,
            7,
            [
                "Logitech USB Optical Mouse",
                "0003",
                "1f0000 0 0 0 0",
                "103",
            ],
        );
        endpoint(
            &sys.0,
            "devices/pci0000:00/usb3/3-2",
            "3-2:1.0",
            ["8ms", "7200"],
        );
        let builtin = "devices/platform/i8042/serio0";
        input_at(
            &sys.0,
            builtin,
            0,
            ["AT Translated Set 2 keyboard", "0011", keys, "0"],
        );
        // Left out: one made by a program, one on Bluetooth and the
        // touchpad's mouse, which isn't on USB.
        input_at(
            &sys.0,
            "devices/virtual/input",
            42,
            ["keyd virtual keyboard", "0003", keys, "0"],
        );
        input_at(
            &sys.0,
            "devices/virtual/misc/uhid/0005:25A7:FAA0.0009",
            43,
            ["EM11", "0005", keys, "0"],
        );
        input_at(
            &sys.0,
            "devices/i2c-14/0018:2C2F:0034.0004",
            44,
            ["Touchpad Mouse", "0018", "1f0000 0 0 0 0", "3"],
        );
        let found = keyboards(&sys.0);
        let said: Vec<(&str, Vec<String>)> = found
            .iter()
            .map(|(name, facts)| {
                let facts = facts
                    .iter()
                    .map(|(fact, value)| format!("{fact}: {value}"))
                    .collect();
                (name.as_str(), facts)
            })
            .collect();
        assert_eq!(
            said,
            [
                ("AT Translated Set 2 keyboard", vec!["keys: 63".to_owned()]),
                (
                    "Logitech USB Optical Mouse",
                    vec![
                        "polling rate: 125 a second".to_owned(),
                        "buttons: 5".to_owned(),
                        "wheel: vertical".to_owned(),
                        "firmware: release 72.00".to_owned()
                    ]
                ),
                (
                    "ZSA Technology Labs ErgoDox EZ",
                    vec![
                        "polling rate: 1000 a second".to_owned(),
                        "keys: 63".to_owned(),
                        "firmware: release 0.01".to_owned()
                    ]
                ),
            ]
        );
    }

    #[test]
    fn a_polling_interval_is_said_as_a_rate() {
        assert_eq!(polling_rate("1ms\n"), "1000 a second");
        assert_eq!(polling_rate("125us"), "8000 a second");
        assert_eq!(polling_rate("0ms"), "");
    }
}
