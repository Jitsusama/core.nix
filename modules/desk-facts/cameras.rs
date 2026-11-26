//! Cameras, from V4L2 and their USB descriptors.

use std::fs::{self, File};
use std::path::Path;

use crate::usb::{release, speed};
use crate::{Facts, ask, fact, word};

/// V4L2's requests, each `_IOWR('V', number, size)` as videodev2.h has it.
const VIDIOC_ENUM_FMT: u64 = 0xC040_5602;
const VIDIOC_G_FMT: u64 = 0xC0D0_5604;
const VIDIOC_G_PARM: u64 = 0xC0CC_5615;
const VIDIOC_ENUM_FRAMESIZES: u64 = 0xC02C_564A;
const VIDIOC_ENUM_FRAMEINTERVALS: u64 = 0xC034_564B;
/// uvcvideo's extension unit query, `_IOWR('u', 0x21, 16)`.
const UVCIOC_CTRL_QUERY: u64 = 0xC010_7521;
const CAPTURE: u32 = 1;
const UVC_GET_CUR: u8 = 0x81;
/// The Logitech Brio's own extension unit, as its descriptor gives the GUID
/// 49e40215-f434-47fe-b158-0e885023e51b; selector 5 is its field of view.
const BRIO_UNIT: [u8; 16] = [
    0x15, 0x02, 0xE4, 0x49, 0x34, 0xF4, 0xFE, 0x47, 0xB1, 0x58, 0x0E, 0x88, 0x50, 0x23, 0xE5, 0x1B,
];

/// A pixel format's name, from its four letters.
fn format_name(fourcc: u32) -> String {
    let letters: String = fourcc
        .to_le_bytes()
        .iter()
        .map(|byte| char::from(*byte))
        .collect();
    match letters.trim() {
        "MJPG" => "MJPEG".to_owned(),
        "H264" => "H.264".to_owned(),
        "HEVC" | "H265" => "H.265".to_owned(),
        other => other.to_owned(),
    }
}

/// A mode, as its width, height and frames a second.
type Mode = (u32, u32, u32);

fn mode(width: u32, height: u32, rate: u32) -> String {
    format!("{width} \u{d7} {height} at {rate}")
}

/// A format's largest mode and its fastest, or the one when they're one.
fn summary(modes: &[Mode]) -> String {
    let pixels = |(width, height, _): &Mode| u64::from(*width) * u64::from(*height);
    let Some(largest) = modes.iter().max_by_key(|each| (pixels(each), each.2)) else {
        return String::new();
    };
    let fastest = modes
        .iter()
        .max_by_key(|each| (each.2, pixels(each)))
        .unwrap_or(largest);
    if fastest.2 == largest.2 {
        mode(largest.0, largest.1, largest.2)
    } else {
        format!(
            "{}; {}",
            mode(largest.0, largest.1, largest.2),
            mode(fastest.0, fastest.1, fastest.2)
        )
    }
}

/// Frames a second from a frame interval, rounded.
fn rate(numerator: u32, denominator: u32) -> u32 {
    denominator
        .saturating_add(numerator / 2)
        .checked_div(numerator)
        .unwrap_or(0)
}

/// The fastest a node offers a format at a size.
fn fastest(node: &File, format: u32, width: u32, height: u32) -> u32 {
    let mut best = 0;
    for index in 0..64u32 {
        let mut asked = [0u8; 52];
        for (at, value) in [(0, index), (4, format), (8, width), (12, height)] {
            asked[at..at + 4].copy_from_slice(&value.to_ne_bytes());
        }
        // SAFETY: a frame interval's struct holds no pointer.
        if !unsafe { ask(node, VIDIOC_ENUM_FRAMEINTERVALS, &mut asked) } {
            break;
        }
        // A discrete interval, or a range's shortest.
        best = best.max(rate(word(&asked, 20), word(&asked, 24)));
        if word(&asked, 16) != 1 {
            break;
        }
    }
    best
}

/// Every format a node offers, with each of its modes.
fn formats(node: &File) -> Vec<(u32, Vec<Mode>)> {
    let mut found = Vec::new();
    for index in 0..32u32 {
        let mut asked = [0u8; 64];
        asked[..4].copy_from_slice(&index.to_ne_bytes());
        asked[4..8].copy_from_slice(&CAPTURE.to_ne_bytes());
        // SAFETY: a format's description holds no pointer.
        if !unsafe { ask(node, VIDIOC_ENUM_FMT, &mut asked) } {
            break;
        }
        let format = word(&asked, 44);
        let mut modes = Vec::new();
        for size in 0..128u32 {
            let mut sizes = [0u8; 44];
            sizes[..4].copy_from_slice(&size.to_ne_bytes());
            sizes[4..8].copy_from_slice(&format.to_ne_bytes());
            // SAFETY: a frame size's struct holds no pointer.
            if !unsafe { ask(node, VIDIOC_ENUM_FRAMESIZES, &mut sizes) } {
                break;
            }
            // A discrete size, or a range's largest.
            let discrete = word(&sizes, 8) == 1;
            let (width, height) = if discrete {
                (word(&sizes, 12), word(&sizes, 16))
            } else {
                (word(&sizes, 16), word(&sizes, 28))
            };
            modes.push((width, height, fastest(node, format, width, height)));
            if !discrete {
                break;
            }
        }
        found.push((format, modes));
    }
    found
}

/// What a node streams now: its size, format and rate.
fn now_streaming(node: &File) -> Option<String> {
    let mut format = [0u8; 208];
    format[..4].copy_from_slice(&CAPTURE.to_ne_bytes());
    let mut parameters = [0u8; 204];
    parameters[..4].copy_from_slice(&CAPTURE.to_ne_bytes());
    // SAFETY: a capture's format and its parameters hold no pointer; only
    // an overlay's format does.
    if !unsafe { ask(node, VIDIOC_G_FMT, &mut format) && ask(node, VIDIOC_G_PARM, &mut parameters) }
    {
        return None;
    }
    Some(format!(
        "{} {}",
        format_name(word(&format, 16)),
        mode(
            word(&format, 8),
            word(&format, 12),
            rate(word(&parameters, 12), word(&parameters, 16))
        )
    ))
}

/// The id of the extension unit a camera's descriptors give this GUID.
fn unit(descriptors: &[u8], guid: &[u8; 16]) -> Option<u8> {
    let mut rest = descriptors;
    while let [length, ..] = rest {
        let length = usize::from(*length);
        if length < 2 || length > rest.len() {
            return None;
        }
        let (descriptor, after) = rest.split_at(length);
        // A class-specific interface descriptor that is an extension unit.
        if descriptor.len() >= 20
            && descriptor[1] == 0x24
            && descriptor[2] == 0x06
            && &descriptor[4..20] == guid
        {
            return Some(descriptor[3]);
        }
        rest = after;
    }
    None
}

/// A Brio's field of view, as its own unit has it set.
fn field_of_view(node: &File, unit: u8) -> Option<&'static str> {
    let mut value = [0u8; 1];
    let mut query = [0u8; 16];
    query[..3].copy_from_slice(&[unit, 5, UVC_GET_CUR]);
    query[4..6].copy_from_slice(&1u16.to_ne_bytes());
    query[8..].copy_from_slice(&(value.as_mut_ptr() as usize as u64).to_ne_bytes());
    // SAFETY: the query points at `value`, as long as its size says, which
    // lasts the call.
    if !unsafe { ask(node, UVCIOC_CTRL_QUERY, &mut query) } {
        return None;
    }
    match value[0] {
        0 => Some("90\u{b0}"),
        1 => Some("78\u{b0}"),
        2 => Some("65\u{b0}"),
        _ => None,
    }
}

/// A camera's name as its holders file is named: trimmed, cut to 64
/// characters, one plain file name, or its node's where nothing is left
/// (modules/v4l2loopback/holders' `camera_file`).
fn holders_name(name: &str, node: &str) -> String {
    let called: String = name
        .trim()
        .chars()
        .map(|c| if c.is_control() { '?' } else { c })
        .take(64)
        .collect();
    let called = called.replace('/', "-");
    match called.trim_start_matches('.') {
        "" => node.to_owned(),
        called => called.to_owned(),
    }
}

/// A loopback camera a relay fills from a sensor that isn't on USB: its
/// holders file's name, the loopback's own name, and the sensor's I2C
/// device as /sys/bus/i2c/devices names it, where it's known.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Loopback {
    name: String,
    card: String,
    sensor: String,
}

/// The loopbacks a file lists, a line each: the holders file's name, the
/// loopback's name and the sensor, apart by tabs.
pub(crate) fn loopbacks(text: &str) -> Vec<Loopback> {
    text.lines()
        .filter_map(|line| {
            let mut fields = line.split('\t');
            let (name, card) = (fields.next()?, fields.next()?);
            Some(Loopback {
                name: name.to_owned(),
                card: card.to_owned(),
                sensor: fields.next().unwrap_or_default().to_owned(),
            })
        })
        .filter(|loopback| !loopback.name.is_empty() && !loopback.card.is_empty())
        .collect()
}

/// A loopback's format and mode, as v4l2loopback's `format` says them
/// ("NV12:1920x1080@300000/10000" is NV12 at 1920 \u{d7} 1080 and 30).
fn looped_mode(format: &str) -> Option<(String, String)> {
    let (fourcc, rest) = format.trim().split_once(':')?;
    let (size, rate) = rest.split_once('@')?;
    let (width, height) = size.split_once('x')?;
    let (frames, seconds) = rate.split_once('/').unwrap_or((rate, "1"));
    let number = |text: &str| text.parse::<u32>().ok();
    let rate = self::rate(number(seconds)?, number(frames)?);
    Some((
        format_name(u32::from_le_bytes(fourcc.as_bytes().try_into().ok()?)),
        mode(number(width)?, number(height)?, rate),
    ))
}

/// A sensor's maker and model: its maker from its ACPI id's first four
/// letters, its model from the name its driver gives its subdevice
/// ("ov08x40 17-0036").
fn sensor_name(sensor: &Path) -> String {
    let id = sensor
        .file_name()
        .map(|name| name.to_string_lossy().trim_start_matches("i2c-").to_owned())
        .unwrap_or_default();
    let maker = match id.get(..4) {
        Some("OVTI") => "OmniVision",
        Some("HIMX") => "Himax",
        Some("SONY") => "Sony",
        Some("GCTI") => "GalaxyCore",
        _ => "",
    };
    let model = fs::read_dir(sensor.join("video4linux"))
        .ok()
        .and_then(|mut entries| entries.next()?.ok())
        .and_then(|entry| fs::read_to_string(entry.path().join("name")).ok())
        .and_then(|name| Some(name.split(' ').next()?.trim().to_uppercase()))
        .unwrap_or_default();
    [maker, model.as_str()]
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// A loopback camera's facts, from its node in `folder` and its sensor in
/// `sys`, none of which opens it: what it streams while it's held, its one
/// format and mode, its sensor, which way the sensor faces, and whether
/// it's powered.
fn looped(folder: &Path, sys: &Path, loopback: &Loopback, held: bool) -> Facts {
    let mut facts = Facts::new();
    let said = fs::read_to_string(folder.join("format")).unwrap_or_default();
    if let Some((format, mode)) = looped_mode(&said) {
        if held {
            fact(&mut facts, "streaming", format!("{format} {mode}"));
        }
        fact(&mut facts, &format, mode);
    }
    if loopback.sensor.is_empty() {
        return facts;
    }
    let sensor = sys.join("bus/i2c/devices").join(&loopback.sensor);
    let read = |name: &str| {
        fs::read_to_string(sensor.join(name))
            .unwrap_or_default()
            .trim()
            .to_owned()
    };
    fact(&mut facts, "sensor", sensor_name(&sensor));
    let facing = match read("physical_location/panel").as_str() {
        "front" => "you",
        "back" => "away from you",
        _ => "",
    };
    fact(&mut facts, "facing", facing);
    let power = match read("power/runtime_status").as_str() {
        "active" => "on",
        "suspended" => "off",
        _ => "",
    };
    fact(&mut facts, "power", power);
    facts
}

/// Every camera's facts, by its holders file's name, from the video nodes
/// `sys` lists, their nodes in `dev` and their holders in `holders`: a USB
/// camera's from the camera, and a loopback's that `looped` lists from its
/// sysfs and its sensor's.
pub(crate) fn cameras(
    sys: &Path,
    dev: &Path,
    holders: &Path,
    looped_from: &[Loopback],
) -> Vec<(String, Facts)> {
    let Ok(entries) = fs::read_dir(sys.join("class/video4linux")) else {
        return Vec::new();
    };
    let mut found = Vec::new();
    for entry in entries.flatten() {
        let folder = entry.path();
        let read = |name: &str| fs::read_to_string(folder.join(name)).unwrap_or_default();
        // A UVC camera's first node is the one it streams pictures through.
        let usb = folder.join("device/..");
        if read("index").trim() != "0" {
            continue;
        }
        if !usb.join("idVendor").exists() {
            let card = read("name");
            if let Some(loopback) = looped_from.iter().find(|each| each.card == card.trim()) {
                let held = fs::read_to_string(holders.join(&loopback.name)).unwrap_or_default();
                let facts = looped(&folder, sys, loopback, !held.trim().is_empty());
                found.push((loopback.name.clone(), facts));
            }
            continue;
        }
        let name = holders_name(&read("name"), &entry.file_name().to_string_lossy());
        let Ok(node) = File::open(dev.join(entry.file_name())) else {
            continue;
        };
        let mut facts = Facts::new();
        let held = fs::read_to_string(holders.join(&name)).unwrap_or_default();
        if !held.trim().is_empty() {
            fact(
                &mut facts,
                "streaming",
                now_streaming(&node).unwrap_or_default(),
            );
        }
        for (format, modes) in formats(&node) {
            let said = summary(&modes);
            fact(&mut facts, &format_name(format), said);
        }
        let descriptors = fs::read(usb.join("descriptors")).unwrap_or_default();
        if let Some(view) = unit(&descriptors, &BRIO_UNIT).and_then(|id| field_of_view(&node, id)) {
            fact(&mut facts, "field of view", view);
        }
        let at = |name: &str| fs::read_to_string(usb.join(name)).unwrap_or_default();
        fact(&mut facts, "USB", speed(&at("speed")));
        let firmware = at("bcdDevice");
        if !firmware.trim().is_empty() {
            fact(
                &mut facts,
                "firmware",
                format!("release {}", release(&firmware)),
            );
        }
        found.push((name, facts));
    }
    found.sort();
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::*;

    #[test]
    fn a_format_says_its_largest_mode_and_its_fastest() {
        // The MX Brio 705's MJPEG modes, some of them.
        let modes = [
            (640, 480, 30),
            (1920, 1080, 60),
            (2560, 1440, 30),
            (3840, 2160, 30),
        ];
        assert_eq!(
            summary(&modes),
            "3840 \u{d7} 2160 at 30; 1920 \u{d7} 1080 at 60"
        );
        assert_eq!(summary(&[(1920, 1080, 30)]), "1920 \u{d7} 1080 at 30");
        assert_eq!(summary(&[]), "");
        assert_eq!(rate(1, 30), 30);
        assert_eq!(rate(1001, 60000), 60);
        assert_eq!(rate(0, 30), 0);
    }

    #[test]
    fn a_pixel_format_is_named_as_people_say_it() {
        assert_eq!(format_name(u32::from_le_bytes(*b"MJPG")), "MJPEG");
        assert_eq!(format_name(u32::from_le_bytes(*b"YUYV")), "YUYV");
        assert_eq!(format_name(u32::from_le_bytes(*b"NV12")), "NV12");
        assert_eq!(format_name(u32::from_le_bytes(*b"H264")), "H.264");
    }

    #[test]
    fn the_brio_s_unit_is_found_among_its_descriptors() {
        // An interface descriptor, then an extension unit with id 10.
        let mut descriptors = vec![9, 0x04, 0, 0, 0, 0x0E, 0x01, 0, 0];
        let mut unit_descriptor = vec![27, 0x24, 0x06, 10];
        unit_descriptor.extend_from_slice(&BRIO_UNIT);
        unit_descriptor.resize(27, 0);
        descriptors.extend(unit_descriptor);
        assert_eq!(unit(&descriptors, &BRIO_UNIT), Some(10));
        assert_eq!(unit(&descriptors, &[0; 16]), None);
        assert_eq!(unit(&[0, 1, 2], &BRIO_UNIT), None, "a zero length ends it");
    }

    #[test]
    fn a_camera_is_keyed_as_its_holders_file_is_named() {
        assert_eq!(
            holders_name("MX Brio 705 for Business\n", "video0"),
            "MX Brio 705 for Business"
        );
        assert_eq!(holders_name("A/B .cam", "video0"), "A-B .cam");
        assert_eq!(holders_name(".hidden", "video0"), "hidden");
        assert_eq!(
            holders_name("..", "video2"),
            "video2",
            "a name of dots alone"
        );
        assert_eq!(
            holders_name(&"x".repeat(100), "video0").len(),
            64,
            "a name past the longest holders writes"
        );
    }

    #[test]
    fn a_loopback_says_its_mode_its_sensor_and_whether_it_s_powered() {
        assert_eq!(
            loopbacks("camera\tBuilt-in Camera\ti2c-OVTI08F4:00\nir-camera\tIR Camera\n\n"),
            vec![
                Loopback {
                    name: "camera".to_owned(),
                    card: "Built-in Camera".to_owned(),
                    sensor: "i2c-OVTI08F4:00".to_owned(),
                },
                Loopback {
                    name: "ir-camera".to_owned(),
                    card: "IR Camera".to_owned(),
                    sensor: String::new(),
                },
            ]
        );
        assert_eq!(
            looped_mode("NV12:1920x1080@300000/10000\n"),
            Some(("NV12".to_owned(), "1920 \u{d7} 1080 at 30".to_owned()))
        );
        assert_eq!(
            looped_mode("GREY:648x368@30"),
            Some(("GREY".to_owned(), "648 \u{d7} 368 at 30".to_owned()))
        );
        assert_eq!(looped_mode(""), None);
        let sys = Sys::new("loopback");
        let node = sys.0.join("class/video4linux/video33");
        put(&node, "name", "Built-in Camera\n");
        put(&node, "index", "0\n");
        put(&node, "format", "NV12:1920x1080@300000/10000\n");
        let sensor = sys.0.join("bus/i2c/devices/i2c-OVTI08F4:00");
        put(&sensor, "video4linux/v4l-subdev5/name", "ov08x40 17-0036\n");
        put(&sensor, "physical_location/panel", "front\n");
        put(&sensor, "power/runtime_status", "active\n");
        let holders = sys.0.join("holders");
        put(&holders, "camera", "firefox\n");
        let listed = loopbacks("camera\tBuilt-in Camera\ti2c-OVTI08F4:00\n");
        let found = cameras(&sys.0, &sys.0.join("dev"), &holders, &listed);
        assert_eq!(
            found.len(),
            1,
            "one camera, the loopback told of: {found:?}"
        );
        let said: Vec<(&str, &str)> = found[0]
            .1
            .iter()
            .map(|(name, value)| (name.as_str(), value.as_str()))
            .collect();
        assert_eq!(found[0].0, "camera");
        assert_eq!(
            said,
            [
                ("streaming", "NV12 1920 \u{d7} 1080 at 30"),
                ("NV12", "1920 \u{d7} 1080 at 30"),
                ("sensor", "OmniVision OV08X40"),
                ("facing", "you"),
                ("power", "on")
            ]
        );
        // A loopback it isn't told of says nothing.
        assert_eq!(
            cameras(&sys.0, &sys.0.join("dev"), &holders, &[]),
            [] as [(
                std::string::String,
                std::vec::Vec<(std::string::String, std::string::String)>
            ); 0]
        );
    }
}
