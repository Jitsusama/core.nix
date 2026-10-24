//! What a monitor says of itself over DDC/CI, for the launcher's Displays
//! facts (modules/desk-facts).
//!
//! Its capabilities string says which VCP codes it takes and which MCCS
//! version it speaks, and only the codes it lists are asked for: its
//! contrast, colour preset, panel, display controller and firmware level.
//! A colour temperature isn't, since a monitor in a preset answers its
//! unused request (3000 K on a Z27k G3). With them go its brightness as it
//! follows the panel and, for a Z27k G3, its hot-plug detection. They're
//! kept in `$XDG_RUNTIME_DIR/desk/ddc-<identity>`, one fact a line, its
//! name, a tab and its value, written whole through a copy renamed over it
//! whenever what it says changes, and removed when the monitor goes. The
//! identity is the hex of the EDID's bytes 8 to 17, which desk-facts reads
//! from the connector, since a bus behind an MST hub names no connector.

use std::fmt::Write as _;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Where the facts are kept, beside desk-facts' own.
const FOLDER: &str = "desk";
/// The order facts are shown in; any other comes last.
const ORDER: [&str; 8] = [
    "brightness",
    "contrast",
    "colour preset",
    "panel",
    "controller",
    "hot-plug detection",
    "firmware level",
    "MCCS",
];

/// The data a capabilities answer carries, if it's one for this offset:
/// none once the string has ended.
pub fn fragment(reply: &[u8], offset: u16) -> Result<&[u8], String> {
    let [0x6E, length, 0xE3, high, low, ..] = *reply else {
        return Err(format!("it answered {reply:02x?}"));
    };
    let size = usize::from(length & 0x7F);
    if length & 0x80 == 0 || size < 3 || reply.len() < size + 3 {
        return Err(format!("its answer's length is wrong: {reply:02x?}"));
    }
    let sum = reply[..size + 2].iter().fold(0x50, |sum, byte| sum ^ byte);
    if sum != reply[size + 2] {
        return Err(format!("its answer's checksum is wrong: {reply:02x?}"));
    }
    if u16::from_be_bytes([high, low]) != offset {
        return Err(format!("it answered for another offset: {reply:02x?}"));
    }
    Ok(&reply[5..size + 2])
}

/// The VCP codes a capabilities string lists, not those its values name.
pub fn listed(capabilities: &str) -> Vec<u8> {
    let Some((_, rest)) = capabilities.split_once("vcp(") else {
        return Vec::new();
    };
    let mut codes = Vec::new();
    let mut depth = 1;
    let mut token = String::new();
    let mut flush = |token: &mut String, depth: usize| {
        if depth == 1
            && token.len() == 2
            && let Ok(code) = u8::from_str_radix(token, 16)
        {
            codes.push(code);
        }
        token.clear();
    };
    for c in rest.chars() {
        match c {
            '(' => {
                flush(&mut token, depth);
                depth += 1;
            }
            ')' => {
                flush(&mut token, depth);
                depth -= 1;
                if depth == 0 {
                    break;
                }
            }
            c if c.is_whitespace() => flush(&mut token, depth),
            c => token.push(c),
        }
    }
    codes
}

/// The MCCS version a capabilities string gives.
pub fn mccs_version(capabilities: &str) -> Option<String> {
    let (_, rest) = capabilities.split_once("mccs_ver(")?;
    let (version, _) = rest.split_once(')')?;
    let version = version.trim();
    (!version.is_empty()).then(|| version.to_owned())
}

/// A colour preset, as MCCS 2.2 names its values.
pub fn preset(value: u16) -> String {
    let [_, low] = value.to_be_bytes();
    match low {
        0x01 => "sRGB".to_owned(),
        0x02 => "native".to_owned(),
        0x03 => "4000 K".to_owned(),
        0x04 => "5000 K".to_owned(),
        0x05 => "6500 K".to_owned(),
        0x06 => "7500 K".to_owned(),
        0x07 => "8200 K".to_owned(),
        0x08 => "9300 K".to_owned(),
        0x09 => "10000 K".to_owned(),
        0x0A => "11500 K".to_owned(),
        0x0B => "user 1".to_owned(),
        0x0C => "user 2".to_owned(),
        0x0D => "user 3".to_owned(),
        other => format!("{other:#04x}"),
    }
}

/// A panel's kind, as MCCS 2.2 names its display technologies.
pub fn panel(value: u16) -> String {
    let [_, low] = value.to_be_bytes();
    match low {
        0x01 => "CRT, shadow mask".to_owned(),
        0x02 => "CRT, aperture grille".to_owned(),
        0x03 => "LCD".to_owned(),
        0x04 => "LCoS".to_owned(),
        0x05 => "plasma".to_owned(),
        0x06 => "OLED".to_owned(),
        0x07 => "electroluminescent".to_owned(),
        0x08 | 0x09 => "MEMS".to_owned(),
        other => format!("{other:#04x}"),
    }
}

/// Who made a display controller, as MCCS 2.2 names them in its low byte.
pub fn controller(value: u16) -> String {
    let [_, low] = value.to_be_bytes();
    let maker = match low {
        0x01 => "Conexant",
        0x02 => "Genesis",
        0x03 => "Macronix",
        0x04 => "IDT",
        0x05 => "Mstar",
        0x06 => "Myson",
        0x07 => "Philips",
        0x08 => "PixelWorks",
        0x09 => "RealTek",
        0x0A => "Sage",
        0x0B => "Silicon Image",
        0x0C => "SmartASIC",
        0x0D => "STMicroelectronics",
        0x0E => "Topro",
        0x0F => "Trumpion",
        0x10 => "Welltrend",
        0x11 => "Samsung",
        0x12 => "Novatek",
        0x13 => "STK",
        other => return format!("{other:#04x}"),
    };
    maker.to_owned()
}

/// A firmware level, its major version in the high byte.
pub fn firmware(value: u16) -> String {
    let [major, minor] = value.to_be_bytes();
    format!("{major}.{minor}")
}

/// The hex of an EDID's identity bytes, which names the file.
fn named(identity: &[u8]) -> String {
    identity.iter().fold(String::new(), |mut hex, byte| {
        let _ = write!(hex, "{byte:02x}");
        hex
    })
}

/// Facts as the file holds them: a line a fact, in their order.
fn rendered(facts: &[(&'static str, String)]) -> String {
    let mut sorted: Vec<&(&str, String)> = facts.iter().collect();
    sorted.sort_by_key(|(name, _)| {
        ORDER
            .iter()
            .position(|each| each == name)
            .unwrap_or(ORDER.len())
    });
    sorted
        .iter()
        .fold(String::new(), |mut text, (name, value)| {
            let _ = writeln!(text, "{name}\t{}", value.replace(['\t', '\n'], " "));
            text
        })
}

/// One monitor's facts, and the file they're kept in, if there's a runtime
/// folder and an identity to name it by.
pub struct Told {
    path: Option<PathBuf>,
    facts: Vec<(&'static str, String)>,
    said: String,
    failing: bool,
}

impl Told {
    pub fn new(runtime: Option<&Path>, identity: Option<&[u8]>) -> Told {
        let path = runtime.zip(identity).map(|(runtime, identity)| {
            runtime
                .join(FOLDER)
                .join(format!("ddc-{}", named(identity)))
        });
        Told {
            path,
            facts: Vec::new(),
            said: String::new(),
            failing: false,
        }
    }

    /// Whether the facts are kept anywhere.
    pub fn kept(&self) -> bool {
        self.path.is_some()
    }

    pub fn set(&mut self, name: &'static str, value: String) {
        match self.facts.iter_mut().find(|(each, _)| *each == name) {
            Some((_, was)) => *was = value,
            None => self.facts.push((name, value)),
        }
    }

    /// Whether the file doesn't yet say what's known.
    pub fn unsaid(&self) -> bool {
        self.path.is_some() && rendered(&self.facts) != self.said
    }

    /// Writes the file whole, if what it says has changed.
    pub fn say(&mut self) {
        let Some(path) = &self.path else {
            return;
        };
        let text = rendered(&self.facts);
        if text == self.said {
            return;
        }
        match keep(path, &text) {
            Ok(()) => {
                self.said = text;
                self.failing = false;
            }
            Err(error) => {
                if !self.failing {
                    eprintln!(
                        "<4>can't keep a monitor's facts in {}: {error}",
                        path.display()
                    );
                }
                self.failing = true;
                // Once: it isn't tried again until something changes.
                self.said = text;
            }
        }
    }
}

impl Drop for Told {
    fn drop(&mut self) {
        if let Some(path) = &self.path {
            let _ = fs::remove_file(path);
        }
    }
}

fn keep(path: &Path, text: &str) -> io::Result<()> {
    if let Some(folder) = path.parent() {
        fs::create_dir_all(folder)?;
    }
    let copy = path.with_extension("new");
    fs::write(&copy, text)?;
    fs::rename(&copy, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A capabilities answer, summed as a monitor sends it.
    fn answer(offset: u16, data: &[u8]) -> Vec<u8> {
        let [high, low] = offset.to_be_bytes();
        let length = u8::try_from(data.len() + 3).unwrap();
        let mut reply = vec![0x6E, 0x80 | length, 0xE3, high, low];
        reply.extend_from_slice(data);
        reply.push(reply.iter().fold(0x50, |sum, byte| sum ^ byte));
        // A read takes the whole buffer, padded past the answer.
        reply.resize(38, 0);
        reply
    }

    #[test]
    fn a_capabilities_answer_gives_its_piece_of_the_string() {
        assert_eq!(
            fragment(&answer(0, b"(prot(monitor)"), 0).unwrap(),
            b"(prot(monitor)"
        );
        assert_eq!(fragment(&answer(64, b""), 64).unwrap(), b"");
    }

    #[test]
    fn a_capabilities_answer_that_isn_t_right_is_refused() {
        let mut summed = answer(0, b"(prot");
        summed[10] ^= 1;
        assert!(fragment(&summed, 0).is_err());
        assert!(
            fragment(&answer(32, b"(prot"), 0).is_err(),
            "another offset"
        );
        assert!(fragment(&[0x6E, 0x80, 0xBE], 0).is_err(), "a null answer");
    }

    /// A capabilities string as an MCCS 2.2 monitor gives one.
    const CAPABILITIES: &str = "(prot(monitor)type(LCD)model(HP Z27k G3)cmds(01 02 03 07 0C E3 F3)\
        vcp(02 04 05 08 10 12 14(01 05 08 0B) 16 18 1A 52 60(0F 11 1B) 62 AC AE B6 C0 C6 C8 C9 \
        CA(01 02) CC(02 0A) D6(01 04 05) DF E7 FF)mswhql(1)asset_eep(40)mccs_ver(2.2))";

    #[test]
    fn the_codes_a_monitor_lists_are_read_and_not_the_values_they_name() {
        let codes = listed(CAPABILITIES);
        for code in [0x10, 0x14, 0xC9, 0xDF, 0xE7] {
            assert!(codes.contains(&code), "{code:#04x} is listed");
        }
        assert!(!codes.contains(&0x0B), "a value of 14's, not a code");
        assert!(!codes.contains(&0x11), "a value of 60's, not a code");
        assert!(!codes.contains(&0x0C), "a command, not a code");
        assert!(listed("(prot(monitor))").is_empty());
    }

    #[test]
    fn its_mccs_version_is_read_from_the_string() {
        assert_eq!(mccs_version(CAPABILITIES).as_deref(), Some("2.2"));
        assert_eq!(mccs_version("(prot(monitor))"), None);
    }

    #[test]
    fn presets_and_firmware_levels_are_named() {
        assert_eq!(preset(0x0001), "sRGB");
        assert_eq!(preset(0x0005), "6500 K");
        assert_eq!(preset(0x000B), "user 1");
        assert_eq!(preset(0x0042), "0x42");
        assert_eq!(panel(0x0003), "LCD");
        assert_eq!(panel(0x0006), "OLED");
        assert_eq!(controller(0xFF05), "Mstar");
        assert_eq!(controller(0x0042), "0x42");
        assert_eq!(firmware(0x0100), "1.0");
        assert_eq!(firmware(0x010E), "1.14");
    }

    #[test]
    fn facts_are_kept_in_their_order_a_line_each() {
        let facts = vec![
            ("MCCS", "2.2".to_owned()),
            ("brightness", "40%".to_owned()),
            ("colour preset", "sRGB\tbut tabbed".to_owned()),
        ];
        assert_eq!(
            rendered(&facts),
            "brightness\t40%\ncolour preset\tsRGB but tabbed\nMCCS\t2.2\n"
        );
    }

    #[test]
    fn a_monitor_s_file_is_written_whole_and_gone_with_it() {
        let runtime = std::env::temp_dir().join(format!("told-{}", std::process::id()));
        let identity = [0x22, 0x0E, 0xD3, 0x36, 0, 0, 0, 0, 0x1C, 0x1F];
        let file = runtime.join("desk/ddc-220ed336000000001c1f");
        {
            let mut told = Told::new(Some(&runtime), Some(&identity));
            assert!(told.kept());
            told.set("brightness", "40%".to_owned());
            assert!(told.unsaid());
            told.say();
            assert!(!told.unsaid());
            assert_eq!(fs::read_to_string(&file).unwrap(), "brightness\t40%\n");
            told.set("brightness", "41%".to_owned());
            told.say();
            assert_eq!(fs::read_to_string(&file).unwrap(), "brightness\t41%\n");
        }
        assert!(!file.exists(), "removed when the monitor goes");
        fs::remove_dir_all(&runtime).unwrap();
    }

    #[test]
    fn nothing_is_kept_without_a_runtime_folder_or_an_identity() {
        assert!(!Told::new(None, Some(&[0; 10])).kept());
        assert!(!Told::new(Some(Path::new("/run/user/1000")), None).kept());
    }
}
