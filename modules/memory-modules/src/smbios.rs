//! A memory device, SMBIOS structure type 17, read from the bytes the
//! kernel gives for it: its formatted area, then its strings. The SMBIOS
//! specification (DSP0134, 7.18) gives each field's place and encoding.

use serde::Serialize;

/// The structure type of a memory device.
pub const MEMORY_DEVICE: u8 = 17;

/// Every structure opens with its type, its formatted area's length and its
/// handle.
const HEADER: usize = 4;

// Where each field sits in a memory device's formatted area, by the
// specification's offsets.
const SIZE: usize = 0x0C;
const DEVICE_LOCATOR: usize = 0x10;
const BANK_LOCATOR: usize = 0x11;
const MEMORY_TYPE: usize = 0x12;
const SPEED: usize = 0x15;
const MANUFACTURER: usize = 0x17;
const PART_NUMBER: usize = 0x1A;
const EXTENDED_SIZE: usize = 0x1C;
const CONFIGURED_SPEED: usize = 0x20;
const EXTENDED_SPEED: usize = 0x54;
const EXTENDED_CONFIGURED_SPEED: usize = 0x58;

// A size of 0 is an empty slot, and all ones an unknown size. 0x7FFF says
// the size is in the extended field, in mebibytes; otherwise the top bit
// says kibibytes rather than mebibytes.
const EMPTY_SLOT: u64 = 0;
const UNKNOWN_WORD: u64 = 0xFFFF;
const SIZE_IS_EXTENDED: u64 = 0x7FFF;
const SIZE_IN_KIBIBYTES: u64 = 0x8000;
const SIZE_BITS: u64 = 0x7FFF;
const EXTENDED_SIZE_BITS: u64 = 0x7FFF_FFFF;
const KIBIBYTES_IN_A_MEBIBYTE: u64 = 1024;
const KIBIBYTES_IN_A_GIBIBYTE: u64 = 1024 * 1024;

// SMBIOS has no code for LPDDR5X, so firmware lists it as LPDDR5. JEDEC's
// LPDDR5 runs at 6400 MT/s at most, so LPDDR5 rated faster than that can
// only be LPDDR5X.
const LPDDR5: &str = "LPDDR5";
const LPDDR5X: &str = "LPDDR5X";
const LPDDR5_FASTEST: u64 = 6400;

/// What firmware writes in a string it has nothing to put in, which says
/// nothing either.
const NOTHING_SAID: [&str; 6] = [
    "",
    "not specified",
    "unknown",
    "undefined",
    "none",
    "to be filled by o.e.m.",
];

/// A memory module as the launcher reads it, each field empty where the
/// firmware says nothing.
#[derive(Serialize, Debug, PartialEq, Eq)]
pub struct Module {
    pub locator: String,
    pub size: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub speed: String,
    pub manufacturer: String,
    pub part: String,
}

/// The memory types the specification names, by their codes, as dmidecode
/// spells them. Other and Unknown say nothing a person can use, so they
/// read as nothing said.
fn memory_type(code: u64) -> &'static str {
    match code {
        0x03 => "DRAM",
        0x04 => "EDRAM",
        0x05 => "VRAM",
        0x06 => "SRAM",
        0x07 => "RAM",
        0x08 => "ROM",
        0x09 => "Flash",
        0x0A => "EEPROM",
        0x0B => "FEPROM",
        0x0C => "EPROM",
        0x0D => "CDRAM",
        0x0E => "3DRAM",
        0x0F => "SDRAM",
        0x10 => "SGRAM",
        0x11 => "RDRAM",
        0x12 => "DDR",
        0x13 => "DDR2",
        0x14 => "DDR2 FB-DIMM",
        0x18 => "DDR3",
        0x19 => "FBD2",
        0x1A => "DDR4",
        0x1B => "LPDDR",
        0x1C => "LPDDR2",
        0x1D => "LPDDR3",
        0x1E => "LPDDR4",
        0x1F => "Logical non-volatile device",
        0x20 => "HBM",
        0x21 => "HBM2",
        0x22 => "DDR5",
        0x23 => LPDDR5,
        0x24 => "HBM3",
        _ => "",
    }
}

/// The little-endian number at an offset of the formatted area, or none past
/// its end.
fn number(raw: &[u8], offset: usize, width: usize) -> Option<u64> {
    let length = usize::from(*raw.get(1)?);
    if offset + width > length {
        return None;
    }
    let bytes = raw.get(offset..offset + width)?;
    Some(
        bytes
            .iter()
            .rev()
            .fold(0, |sum, byte| (sum << 8) | u64::from(*byte)),
    )
}

/// The strings that follow the formatted area, in order, each trimmed.
fn strings_of(raw: &[u8]) -> Vec<String> {
    let start = raw.get(1).map_or(raw.len(), |length| usize::from(*length));
    raw.get(start..)
        .unwrap_or_default()
        .split(|byte| *byte == 0)
        .take_while(|piece| !piece.is_empty())
        .map(|piece| String::from_utf8_lossy(piece).trim().to_owned())
        .collect()
}

/// The string a field names by its number, counted from one, or none.
fn string_at(raw: &[u8], offset: usize, texts: &[String]) -> String {
    let text = number(raw, offset, 1)
        .and_then(|index| usize::try_from(index).ok())
        .and_then(|index| index.checked_sub(1))
        .and_then(|at| texts.get(at));
    match text {
        Some(text) if !NOTHING_SAID.contains(&text.to_lowercase().as_str()) => text.clone(),
        _ => String::new(),
    }
}

/// A module's size in kibibytes, 0 for an empty slot, or none when it isn't
/// known.
fn kibibytes_of(raw: &[u8]) -> Option<u64> {
    let size = number(raw, SIZE, 2)?;
    if size == UNKNOWN_WORD {
        return None;
    }
    if size == SIZE_IS_EXTENDED {
        return number(raw, EXTENDED_SIZE, 4)
            .map(|extended| (extended & EXTENDED_SIZE_BITS) * KIBIBYTES_IN_A_MEBIBYTE);
    }
    if size & SIZE_IN_KIBIBYTES != 0 {
        return Some(size & SIZE_BITS);
    }
    Some(size * KIBIBYTES_IN_A_MEBIBYTE)
}

/// A size in the largest unit it's a whole number of, as dmidecode says it.
fn size_said(kibibytes: Option<u64>) -> String {
    match kibibytes {
        None => String::new(),
        Some(kibibytes) if kibibytes % KIBIBYTES_IN_A_GIBIBYTE == 0 => {
            format!("{} GB", kibibytes / KIBIBYTES_IN_A_GIBIBYTE)
        }
        Some(kibibytes) if kibibytes % KIBIBYTES_IN_A_MEBIBYTE == 0 => {
            format!("{} MB", kibibytes / KIBIBYTES_IN_A_MEBIBYTE)
        }
        Some(kibibytes) => format!("{kibibytes} kB"),
    }
}

/// A speed in MT/s, from its extended field when the word is all ones, or
/// none when it's not said.
fn speed_at(raw: &[u8], offset: usize, extended: usize) -> Option<u64> {
    let speed = match number(raw, offset, 2) {
        Some(UNKNOWN_WORD) => number(raw, extended, 4),
        speed => speed,
    };
    speed.filter(|speed| *speed != 0)
}

fn configured(raw: &[u8]) -> Option<u64> {
    speed_at(raw, CONFIGURED_SPEED, EXTENDED_CONFIGURED_SPEED)
}

fn rated(raw: &[u8]) -> Option<u64> {
    speed_at(raw, SPEED, EXTENDED_SPEED)
}

/// The speed the memory runs at, or the fastest it's rated for when that's
/// not said.
fn speed_said(raw: &[u8]) -> String {
    configured(raw)
        .or_else(|| rated(raw))
        .map_or_else(String::new, |speed| format!("{speed} MT/s"))
}

/// The memory's type as the specification names it, LPDDR5X where it can
/// only be that.
fn type_said(raw: &[u8]) -> String {
    let said = memory_type(number(raw, MEMORY_TYPE, 1).unwrap_or(0));
    let fastest = rated(raw).or_else(|| configured(raw));
    if said == LPDDR5 && fastest.is_some_and(|speed| speed > LPDDR5_FASTEST) {
        return LPDDR5X.to_owned();
    }
    said.to_owned()
}

/// A memory device as the launcher reads it, or none for an empty slot or
/// bytes that aren't a whole memory device.
pub fn module_of(raw: &[u8]) -> Option<Module> {
    if raw.len() < HEADER || raw[0] != MEMORY_DEVICE || raw.len() < usize::from(raw[1]) {
        return None;
    }
    let kibibytes = kibibytes_of(raw);
    if kibibytes == Some(EMPTY_SLOT) {
        return None;
    }
    let texts = strings_of(raw);
    let locator = string_at(raw, DEVICE_LOCATOR, &texts);
    Some(Module {
        locator: if locator.is_empty() {
            string_at(raw, BANK_LOCATOR, &texts)
        } else {
            locator
        },
        size: size_said(kibibytes),
        kind: type_said(raw),
        speed: speed_said(raw),
        manufacturer: string_at(raw, MANUFACTURER, &texts),
        part: string_at(raw, PART_NUMBER, &texts),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The length of a memory device's formatted area since SMBIOS 3.7.
    const WHOLE: u8 = 0x5C;

    /// A memory device of a formatted area this long, with these fields set
    /// little-endian, then these strings.
    fn device(length: u8, fields: &[(usize, u64, usize)], strings: &[&str]) -> Vec<u8> {
        let mut raw = vec![0; usize::from(length)];
        raw[0] = MEMORY_DEVICE;
        raw[1] = length;
        for &(offset, value, width) in fields {
            raw[offset..offset + width].copy_from_slice(&value.to_le_bytes()[..width]);
        }
        for text in strings {
            raw.extend_from_slice(text.as_bytes());
            raw.push(0);
        }
        raw.push(0);
        if strings.is_empty() {
            raw.push(0);
        }
        raw
    }

    fn module(raw: &[u8]) -> Module {
        module_of(raw).expect("a module")
    }

    #[test]
    fn lpddr5_rated_past_6400_is_lpddr5x() {
        let raw = device(
            WHOLE,
            &[
                (SIZE, SIZE_IS_EXTENDED, 2),
                (EXTENDED_SIZE, 8192, 4),
                (DEVICE_LOCATOR, 1, 1),
                (BANK_LOCATOR, 2, 1),
                (MEMORY_TYPE, 0x23, 1),
                (SPEED, 9600, 2),
                (MANUFACTURER, 3, 1),
                (PART_NUMBER, 4, 1),
                (CONFIGURED_SPEED, 9600, 2),
            ],
            &[
                "Controller0-ChannelA",
                "BANK 0",
                "SK Hynix",
                "  H58G56CK8BX  ",
            ],
        );
        assert_eq!(
            module(&raw),
            Module {
                locator: "Controller0-ChannelA".to_owned(),
                size: "8 GB".to_owned(),
                kind: "LPDDR5X".to_owned(),
                speed: "9600 MT/s".to_owned(),
                manufacturer: "SK Hynix".to_owned(),
                part: "H58G56CK8BX".to_owned(),
            }
        );
    }

    #[test]
    fn lpddr5_at_6400_stays_lpddr5() {
        let raw = device(
            WHOLE,
            &[(SIZE, 4096, 2), (MEMORY_TYPE, 0x23, 1), (SPEED, 6400, 2)],
            &[],
        );
        assert_eq!(module(&raw).kind, "LPDDR5");
    }

    #[test]
    fn an_empty_slot_is_no_module() {
        assert_eq!(module_of(&device(WHOLE, &[(SIZE, 0, 2)], &[])), None);
    }

    #[test]
    fn bytes_of_another_structure_are_no_module() {
        let mut raw = device(WHOLE, &[(SIZE, 4096, 2)], &[]);
        raw[0] = 16;
        assert_eq!(module_of(&raw), None);
        assert_eq!(module_of(&raw[..2]), None);
    }

    #[test]
    fn an_older_structure_says_its_rated_speed() {
        // SMBIOS 2.7's area ends before the configured speed.
        let raw = device(
            0x1C,
            &[(SIZE, 16384, 2), (MEMORY_TYPE, 0x1A, 1), (SPEED, 3200, 2)],
            &[],
        );
        let read = module(&raw);
        assert_eq!((read.size.as_str(), read.kind.as_str()), ("16 GB", "DDR4"));
        assert_eq!(read.speed, "3200 MT/s");
    }

    #[test]
    fn a_speed_of_all_ones_is_read_from_its_extended_field() {
        let raw = device(
            WHOLE,
            &[
                (SIZE, 4096, 2),
                (CONFIGURED_SPEED, UNKNOWN_WORD, 2),
                (EXTENDED_CONFIGURED_SPEED, 70_000, 4),
            ],
            &[],
        );
        assert_eq!(module(&raw).speed, "70000 MT/s");
    }

    #[test]
    fn sizes_read_in_their_largest_whole_unit() {
        let kibibytes = device(WHOLE, &[(SIZE, SIZE_IN_KIBIBYTES | 512, 2)], &[]);
        assert_eq!(module(&kibibytes).size, "512 kB");
        let mebibytes = device(WHOLE, &[(SIZE, 1536, 2)], &[]);
        assert_eq!(module(&mebibytes).size, "1536 MB");
        let unknown = device(WHOLE, &[(SIZE, UNKNOWN_WORD, 2)], &[]);
        assert_eq!(module(&unknown).size, "");
    }

    #[test]
    fn what_says_nothing_reads_as_nothing() {
        let raw = device(
            WHOLE,
            &[
                (SIZE, 4096, 2),
                (DEVICE_LOCATOR, 1, 1),
                (BANK_LOCATOR, 2, 1),
                (MANUFACTURER, 3, 1),
                (PART_NUMBER, 9, 1),
            ],
            &["Not Specified", "BANK 1", "To Be Filled By O.E.M."],
        );
        let read = module(&raw);
        assert_eq!(read.locator, "BANK 1");
        assert_eq!((read.manufacturer.as_str(), read.part.as_str()), ("", ""));
        assert_eq!((read.kind.as_str(), read.speed.as_str()), ("", ""));
    }

    #[test]
    fn it_writes_the_fields_the_launcher_reads() {
        let raw = device(WHOLE, &[(SIZE, 4096, 2), (MEMORY_TYPE, 0x22, 1)], &[]);
        let said = serde_json::to_string(&module(&raw)).expect("json");
        assert_eq!(
            said,
            r#"{"locator":"","size":"4 GB","type":"DDR5","speed":"","manufacturer":"","part":""}"#
        );
    }
}
