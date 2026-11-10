//! The keyboards connected now, whether caps lock is on, and the layer a
//! board is on, each as the program that knows it writes it, so the shell
//! reads none of their files itself (Joel, 2026-11-10: "the daemons need
//! to be the data layer").
//!
//! - `keyboards:connected`: the kernel's input devices that are keyboards,
//!   in its own order, read from `/proc/bus/input/devices` each time the
//!   keyboard-lights watcher says the keyboards it holds have changed
//!   (`/run/keyboard-lights/keyboards`, modules/quickshell/nixos.nix). The
//!   kernel's file can't be watched, and the watcher's list changes only
//!   as a board comes or goes, so a board plugged in or out is heard at
//!   once and nothing is read on a clock.
//! - `capslock:keyboard`: whether caps lock's light is on, as the watcher
//!   writes it in `/run/keyboard-lights/capslock`; never on without it, as
//!   on a Mac.
//! - `layer:keyboard`: the layer a ZSA board is on and whether it's locked
//!   there, as its watcher writes it in `$XDG_RUNTIME_DIR/keyboard-layer`
//!   (modules/ergodox/nixos.nix); none while no board is plugged in, when
//!   it says "-", or with no watcher.

use std::collections::HashSet;

use serde_json::{Value, json};

use crate::Root;

/// The keyboard-lights watcher's folder, and caps lock's file in it.
pub const LIGHTS: &str = "/run/keyboard-lights";
const CAPSLOCK: &str = "/run/keyboard-lights/capslock";
/// The kernel's list of input devices.
const DEVICES: &str = "/proc/bus/input/devices";
/// The layer's file, in the session's runtime folder.
const LAYER: &str = "keyboard-layer";
/// The kernel's number for USB, as its list says the bus.
const USB: &str = "0003";

/// Whether a file of this name in the watcher's folder is one it says,
/// rather than its list being written beside it.
pub fn names(name: &str) -> bool {
    name == "keyboards" || name == "capslock"
}

/// Whether a file of this name in the runtime folder is the layer's.
pub fn names_the_layer(name: &str) -> bool {
    name == LAYER
}

/// The layer's file for this session, or none without a runtime folder.
pub fn layer_file() -> Option<String> {
    std::env::var("XDG_RUNTIME_DIR")
        .ok()
        .filter(|folder| !folder.is_empty())
        .map(|folder| format!("{folder}/{LAYER}"))
}

/// What a device's lines say after a field's mark, or "".
fn field<'a>(block: &'a str, mark: &str) -> &'a str {
    block
        .lines()
        .find_map(|line| line.strip_prefix(mark))
        .unwrap_or_default()
}

/// One word of the kernel's `I:` line, as "Bus=0003" says it, lowered.
fn identity(block: &str, word: &str) -> String {
    field(block, "I: ")
        .split_whitespace()
        .find_map(|said| said.strip_prefix(word)?.strip_prefix('='))
        .unwrap_or_default()
        .to_lowercase()
}

/// Each device in the kernel's list, with what it says of itself, as the
/// shell's Keyboards.js reads each.
fn devices(text: &str) -> Vec<Value> {
    text.split("\n\n")
        .filter(|block| !block.trim().is_empty())
        .map(|block| {
            let name = field(block, "N: Name=");
            let name = name
                .strip_prefix('"')
                .and_then(|name| name.strip_suffix('"'))
                .unwrap_or(name);
            let leds = block
                .lines()
                .find_map(|line| line.strip_prefix("B: LED="))
                .and_then(|mask| u64::from_str_radix(mask.trim(), 16).ok())
                .unwrap_or(0);
            json!({
                "name": name,
                "phys": field(block, "P: Phys="),
                "sysfs": field(block, "S: Sysfs="),
                "uniq": field(block, "U: Uniq="),
                "handlers": field(block, "H: Handlers="),
                "bus": identity(block, "Bus"),
                "vendor": identity(block, "Vendor"),
                "product": identity(block, "Product"),
                "leds": leds,
            })
        })
        .collect()
}

/// Whether a list of handlers names this one, as a word.
fn handles(handlers: &str, word: &str) -> bool {
    handlers.split_whitespace().any(|handler| handler == word)
}

/// A USB board's own device in sysfs, as "1-2" or "3-1.4" names it, above
/// its interfaces, or "" for one that isn't on USB.
fn usb_device(device: &Value) -> String {
    if device["bus"] != USB {
        return String::new();
    }
    let sysfs = device["sysfs"].as_str().unwrap_or_default();
    let steps: Vec<&str> = sysfs.split('/').collect();
    let is_device = |step: &&str| {
        step.split_once('-').is_some_and(|(port, path)| {
            !port.is_empty()
                && port.bytes().all(|byte| byte.is_ascii_digit())
                && !path.is_empty()
                && path.bytes().all(|byte| byte.is_ascii_digit() || byte == b'.')
        })
    };
    steps
        .iter()
        .rposition(|step| is_device(step))
        .map(|last| format!("/sys{}", steps[..=last].join("/")))
        .unwrap_or_default()
}

/// The keyboards among the kernel's devices: one the kernel gives both
/// keys and lights, which leaves out the power button, the lid and the
/// hotkeys, and not keyd's own, which only passes on the others' keys. A
/// board is listed once however many keyboards it says it is: the ErgoDox
/// says it twice over one USB device, so the first of a USB device's is
/// the board (Joel saw two, 2026-10-26).
fn keyboards(text: &str) -> Vec<Value> {
    let mut seen = HashSet::new();
    devices(text)
        .into_iter()
        .filter(|device| {
            let handlers = device["handlers"].as_str().unwrap_or_default();
            let name = device["name"].as_str().unwrap_or_default();
            handles(handlers, "kbd")
                && handles(handlers, "leds")
                && name.split_whitespace().next() != Some("keyd")
        })
        .filter(|device| {
            let usb = usb_device(device);
            let board = if usb.is_empty() {
                device["sysfs"].as_str().unwrap_or_default().to_owned()
            } else {
                usb
            };
            seen.insert(board)
        })
        .collect()
}

/// The keyboards connected now, in the kernel's order.
pub fn connected(root: &Root) -> Value {
    let listed = root.read(DEVICES).map_or_else(Vec::new, |text| keyboards(&text));
    json!({"key": "keyboards:connected", "kind": "keyboards", "listed": listed})
}

/// Whether caps lock is on, as the watcher last wrote it.
pub fn capslock(root: &Root) -> Value {
    let on = root.said(CAPSLOCK).is_some_and(|said| said == "1");
    json!({"key": "capslock:keyboard", "kind": "capslock", "on": on})
}

/// The layer the board is on, and whether it's locked there, or none.
pub fn layer(root: &Root, file: Option<&str>) -> Option<Value> {
    let said = root.read(file?)?;
    let line = said.lines().next().unwrap_or_default().trim();
    if line.is_empty() || line == "-" {
        return None;
    }
    let (name, locked) = line
        .strip_suffix(" locked")
        .map_or((line, false), |name| (name, true));
    Some(json!({"key": "layer:keyboard", "kind": "layer", "name": name, "locked": locked}))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The kernel's list, as a laptop with an ErgoDox says it: the lid,
    /// the built-in keyboard, keyd's, and the board twice over one USB
    /// device.
    const LISTED: &str = "\
I: Bus=0019 Vendor=0000 Product=0005 Version=0000
N: Name=\"Lid Switch\"
P: Phys=PNP0C0D/button/input0
S: Sysfs=/devices/LNXSYSTM:00/input/input0
U: Uniq=
H: Handlers=event0
B: EV=21

I: Bus=0011 Vendor=0001 Product=0001 Version=ab83
N: Name=\"AT Translated Set 2 keyboard\"
P: Phys=isa0060/serio0/input0
S: Sysfs=/devices/platform/i8042/serio0/input/input3
U: Uniq=
H: Handlers=sysrq kbd leds event3
B: LED=7

I: Bus=0019 Vendor=0fac Product=0ade Version=0001
N: Name=\"keyd virtual keyboard\"
P: Phys=
S: Sysfs=/devices/virtual/input/input9
U: Uniq=
H: Handlers=sysrq kbd leds event9
B: LED=7

I: Bus=0003 Vendor=3297 Product=4975 Version=0111
N: Name=\"ZSA Technology Labs ErgoDox EZ\"
P: Phys=usb-0000:00:14.0-2/input0
S: Sysfs=/devices/pci0000:00/0000:00:14.0/usb1/1-2/1-2:1.0/0003:3297:4975.0001/input/input12
U: Uniq=
H: Handlers=sysrq kbd leds event12
B: LED=1f

I: Bus=0003 Vendor=3297 Product=4975 Version=0111
N: Name=\"ZSA Technology Labs ErgoDox EZ Keyboard\"
P: Phys=usb-0000:00:14.0-2/input2
S: Sysfs=/devices/pci0000:00/0000:00:14.0/usb1/1-2/1-2:1.2/0003:3297:4975.0003/input/input14
U: Uniq=
H: Handlers=sysrq kbd leds event14
B: LED=1f
";

    fn rooted(name: &str) -> (Root, std::path::PathBuf) {
        let folder =
            std::env::temp_dir().join(format!("machine-keyboards-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(folder.join("proc/bus/input")).expect("a folder is made");
        std::fs::create_dir_all(folder.join("run/keyboard-lights")).expect("a folder is made");
        (Root::new(&folder), folder)
    }

    #[test]
    fn a_keyboard_is_one_with_keys_and_lights_and_a_board_is_listed_once() {
        let listed = keyboards(LISTED);
        let names: Vec<&str> = listed
            .iter()
            .map(|device| device["name"].as_str().unwrap_or_default())
            .collect();
        assert_eq!(
            names,
            ["AT Translated Set 2 keyboard", "ZSA Technology Labs ErgoDox EZ"]
        );
        assert_eq!(listed[1]["bus"], "0003");
        assert_eq!(listed[1]["vendor"], "3297");
        assert_eq!(listed[1]["product"], "4975");
        assert_eq!(listed[1]["leds"], 31);
        assert_eq!(listed[1]["phys"], "usb-0000:00:14.0-2/input0");
        assert_eq!(
            usb_device(&listed[1]),
            "/sys/devices/pci0000:00/0000:00:14.0/usb1/1-2"
        );
        assert_eq!(usb_device(&listed[0]), "", "a board off USB has no USB device");
    }

    #[test]
    fn whats_connected_and_caps_lock_are_read_from_their_files() {
        let (root, folder) = rooted("connected");
        assert_eq!(connected(&root)["listed"], json!([]), "no list, no keyboards");
        assert_eq!(capslock(&root)["on"], false, "no watcher, never on");
        std::fs::write(folder.join("proc/bus/input/devices"), LISTED).expect("it's written");
        std::fs::write(folder.join("run/keyboard-lights/capslock"), "1\n").expect("it's written");
        assert_eq!(
            connected(&root)["listed"]
                .as_array()
                .map(Vec::len),
            Some(2)
        );
        assert_eq!(capslock(&root)["on"], true);
        assert!(names("keyboards") && names("capslock"), "the watcher's files are its");
        assert!(!names("keyboards.new"), "a list being written is none yet");
        let _ = std::fs::remove_dir_all(folder);
    }

    #[test]
    fn the_layer_says_its_name_and_whether_its_locked_and_none_without_a_board() {
        let (root, folder) = rooted("layer");
        let file = "/run/user/1000/keyboard-layer";
        assert_eq!(layer(&root, Some(file)), None, "no file, no layer");
        assert_eq!(layer(&root, None), None, "no runtime folder, no layer");
        std::fs::create_dir_all(folder.join("run/user/1000")).expect("a folder is made");
        let written = folder.join("run/user/1000/keyboard-layer");
        std::fs::write(&written, "Numb locked\n").expect("it's written");
        assert_eq!(
            layer(&root, Some(file)),
            Some(json!({"key": "layer:keyboard", "kind": "layer", "name": "Numb", "locked": true}))
        );
        std::fs::write(&written, "Punk\n").expect("it's written");
        assert_eq!(layer(&root, Some(file)).map(|layer| layer["locked"].clone()), Some(json!(false)));
        std::fs::write(&written, "-\n").expect("it's written");
        assert_eq!(layer(&root, Some(file)), None, "no board plugged in, no layer");
        let _ = std::fs::remove_dir_all(folder);
    }
}
