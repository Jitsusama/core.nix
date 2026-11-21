//! Reads why the last boot ended from the platform health table, PHAT.
//!
//! ACPI 6.5 (5.2.31) lays the table out as its 36-byte header, then
//! records, each opening with its type (u16), its length (u16) and its
//! revision. A health record (type 1) goes on with two reserved bytes, the
//! device's health, the device's GUID and the offset, from the record's
//! start, of data the device defines. The device whose GUID is the reset
//! reason's (5.2.31.5.1) gives there which sources the platform can name,
//! the source of the last reset, its sub-source, its reason and how many
//! vendors' entries follow, each a GUID, its length with its 20-byte
//! header, its revision and its data.
//!
//! Intel's entry holds four little-endian words. The first is the power
//! management controller's global reset cause: on the XPS 14's two resets
//! of 20 November it read 0x400 where their Crash Logs read the cause
//! `PMC_FW`, and 0x40 on 17 November where it read `MEGBL`, so its bits are
//! named as the Crash Log's register layout for the controller names them.
//! That layout is the XPS 14's processor family's, Panther Lake's, so the
//! names are given only on it; elsewhere a set bit is shown by its number,
//! since another family may lay the register out otherwise. What the other
//! three words mean isn't published, so they're shown as they are.

/// The processor family whose controller's layout names the bits: Intel's
/// family 6, model 0xcc, Panther Lake, as /proc/cpuinfo gives them.
const NAMED_ON: [(&str, &str); 3] = [
    ("vendor_id", "GenuineIntel"),
    ("cpu family", "6"),
    ("model", "204"),
];

/// Whether the processor /proc/cpuinfo describes is the family whose
/// layout names the global reset cause's bits.
pub fn names_its_causes(cpuinfo: &str) -> bool {
    let first = cpuinfo.split("\n\n").next().unwrap_or_default();
    NAMED_ON.iter().all(|(field, value)| {
        first.lines().any(|line| {
            line.split_once(':')
                .is_some_and(|(name, given)| name.trim() == *field && given.trim() == *value)
        })
    })
}

/// The table header's length, before the first record.
const HEADER: usize = 36;
/// A record's type, length and revision.
const RECORD: usize = 5;
/// The type of a health record.
const HEALTH: u16 = 1;
/// A health record's fields before its device path.
const HEALTH_FIELDS: usize = 28;
/// The reset reason's GUID, 7a014ce2-f263-4b77-b88a-e6336b782c14, as it
/// lies in the table.
const RESET_REASON: [u8; 16] = [
    0xe2, 0x4c, 0x01, 0x7a, 0x63, 0xf2, 0x77, 0x4b, 0xb8, 0x8a, 0xe6, 0x33, 0x6b, 0x78, 0x2c, 0x14,
];
/// The reset data's fields before the vendors' entries.
const RESET_FIELDS: usize = 6;
/// A vendor entry's GUID, length and revision.
const VENDOR_HEADER: usize = 20;
/// Intel's vendor GUID, 808601bf-44b2-4d67-9b27-cca2642285ad, as it lies in
/// the table.
const INTEL: [u8; 16] = [
    0xbf, 0x01, 0x86, 0x80, 0xb2, 0x44, 0x67, 0x4d, 0x9b, 0x27, 0xcc, 0xa2, 0x64, 0x22, 0x85, 0xad,
];

/// The global reset cause's bits as the controller's Crash Log layout
/// names them, from bit 0; a bit it leaves unnamed is empty.
const GLOBAL_RESET_CAUSES: [&str; 31] = [
    "SECURE_POLICY_ERR",
    "PBO",
    "PMC_UNC_ERR",
    "SOCS_THRM",
    "ME_PBO",
    "SOCN_THRM",
    "MEGBL",
    "LT_RESET",
    "PMC_WDT",
    "ME_WDT",
    "PMC_FW",
    "PCHPWR_FLR",
    "SYSPWR_FLR",
    "ISH",
    "MIA_UXS_ERR",
    "MIA_UX_ERR",
    "CPU_THRM_WDT",
    "ME_UNC_ERR",
    "ADR_GPIO",
    "OCWDT_NOICC",
    "OCWDT_ICC",
    "CSE_HEC_UNC_ERR",
    "PMC_SRAM_UNC_ERR",
    "PMC_IROM_PARITY",
    "PMC_RF_FUSA_ERR",
    "COMPUTE_THRM",
    "GFX_THRM",
    "BASE_THRM",
    "GPSB_LINK_INT_ERR",
    "",
    "ESE_GBLRST_REQ",
];

/// Why the last boot ended, as the table says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reset {
    /// The source's bit: 0 unknown, 1 hardware, 2 firmware, 3 software, 4
    /// a supervisor.
    pub source: u8,
    /// The reason, by the specification's number.
    pub reason: u8,
    /// Intel's four words, where Intel's entry is there.
    pub intel: Option<[u32; 4]>,
}

/// The reasons a reset is weighed by on its own, whatever Intel's cause
/// says: heat, lost power and the power button, which say better than the
/// controller's bits why it happened.
pub const SAID_REASONS: [u8; 3] = [0x23, 0x24, 0x25];
/// The first reason the specification counts as unasked.
pub const FIRST_UNASKED: u8 = 0x20;
/// The reset source's bit that says software asked for it.
pub const SOFTWARE: u8 = 0b1000;

/// The global reset cause's bits that say heat, lost power or the power
/// button reset the machine, each with the reason it means, in the order
/// the reasons are weighed: SOCS, SOCN, CPU watchdog, compute, graphics and
/// base heat; the controller's and the system's power failing; the power
/// button held, and the management engine's own.
const CAUSES_SAID: [(u32, u8); 3] = [
    (
        (1 << 3) | (1 << 5) | (1 << 16) | (1 << 25) | (1 << 26) | (1 << 27),
        0x23,
    ),
    ((1 << 11) | (1 << 12), 0x24),
    ((1 << 1) | (1 << 4), 0x25),
];

/// A reason, as a word a program matches; none where the table says it
/// doesn't know, or gives a value the specification doesn't name.
pub fn reason_name(reason: u8) -> Option<&'static str> {
    Some(match reason {
        1 => "cold-boot",
        2 => "cold-reset",
        3 => "warm-reset",
        4 => "update",
        0x20 => "unexpected-reset",
        0x21 => "fault",
        0x22 => "timeout",
        0x23 => "heat",
        0x24 => "power-loss",
        0x25 => "power-button",
        _ => return None,
    })
}

/// A reason, in words; none where the table says it doesn't know.
pub fn reason_words(reason: u8) -> Option<&'static str> {
    Some(match reason {
        1 => "a cold boot",
        2 => "a cold reset",
        3 => "a warm reset",
        4 => "an update",
        0x20 => "an unexpected reset",
        0x21 => "a fault",
        0x22 => "a timeout",
        0x23 => "heat",
        0x24 => "lost power",
        0x25 => "the power button",
        _ => return None,
    })
}

impl Reset {
    /// Where the reset came from, as a word a program matches; none for a
    /// value the specification doesn't name.
    pub fn source_name(&self) -> Option<&'static str> {
        Some(match self.source {
            0b1 => "unknown",
            0b10 => "hardware",
            0b100 => "firmware",
            0b1000 => "software",
            0b1_0000 => "supervisor",
            _ => return None,
        })
    }

    /// Where the reset came from, in words; none for a value the
    /// specification doesn't name.
    pub fn source_words(&self) -> Option<&'static str> {
        Some(match self.source {
            0b1 => "unknown",
            0b10 => "the hardware",
            0b100 => "the firmware",
            0b1000 => "software",
            0b1_0000 => "a supervisor",
            _ => return None,
        })
    }

    /// Why, as a word a program matches; none where the table says it
    /// doesn't know, or gives a value the specification doesn't name.
    pub fn reason_name(&self) -> Option<&'static str> {
        reason_name(self.reason)
    }

    /// Why, in words; none where the table says it doesn't know.
    pub fn reason_words(&self) -> Option<&'static str> {
        reason_words(self.reason)
    }

    /// The reason Intel's cause says, where `named` says the machine is the
    /// platform whose layout names the bits and a bit says heat, lost power
    /// or the power button: the controller resets the machine for those
    /// too, and saying so is truer than saying the firmware did.
    pub fn said_by_cause(&self, named: bool) -> Option<u8> {
        let [cause, ..] = self.intel?;
        if !named {
            return None;
        }
        CAUSES_SAID
            .iter()
            .find(|(bits, _)| cause & bits != 0)
            .map(|&(_, reason)| reason)
    }

    /// The global reset cause's set bits, by name where `named` says the
    /// machine is the platform whose layout names them, and otherwise, or
    /// where the layout leaves one unnamed, as `BIT29`.
    pub fn global_causes(&self, named: bool) -> Vec<String> {
        let Some([cause, ..]) = self.intel else {
            return Vec::new();
        };
        (0..32)
            .filter(|bit| cause & (1 << bit) != 0)
            .map(|bit| match GLOBAL_RESET_CAUSES.get(bit) {
                Some(name) if named && !name.is_empty() => (*name).to_owned(),
                _ => format!("BIT{bit}"),
            })
            .collect()
    }
}

fn u16_at(bytes: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(bytes.get(at..at + 2)?.try_into().ok()?))
}

fn u32_at(bytes: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(bytes.get(at..at + 4)?.try_into().ok()?))
}

/// Why the last boot ended, from the table's bytes; none where it isn't a
/// health table or has no reset reason the table's own lengths hold.
pub fn reset_of(table: &[u8]) -> Option<Reset> {
    if table.get(..4)? != b"PHAT" {
        return None;
    }
    let end = usize::try_from(u32_at(table, 4)?).ok()?.min(table.len());
    let mut at = HEADER;
    while at + RECORD <= end {
        let length = usize::from(u16_at(table, at + 2)?);
        if length < RECORD || at + length > end {
            return None;
        }
        let record = &table[at..at + length];
        if u16_at(record, 0)? == HEALTH
            && record.len() >= HEALTH_FIELDS
            && record[8..24] == RESET_REASON
        {
            let offset = usize::try_from(u32_at(record, 24)?).ok()?;
            if offset >= HEALTH_FIELDS {
                return reset_in(record.get(offset..)?);
            }
        }
        at += length;
    }
    None
}

/// The reset data, from its first byte to the record's end. A vendor's
/// entry the record doesn't hold whole ends the reading of entries, but
/// keeps the source and reason before it.
fn reset_in(data: &[u8]) -> Option<Reset> {
    let fields = data.get(..RESET_FIELDS)?;
    let mut intel = None;
    let mut at = RESET_FIELDS;
    for _ in 0..u16_at(fields, 4)? {
        let Some(length) = u16_at(data, at + 16).map(usize::from) else {
            break;
        };
        let Some(entry) = data.get(at..at + length) else {
            break;
        };
        if length < VENDOR_HEADER {
            break;
        }
        if entry[..16] == INTEL {
            let word = |n: usize| u32_at(entry, VENDOR_HEADER + 4 * n);
            if let (Some(a), Some(b), Some(c), Some(d)) = (word(0), word(1), word(2), word(3)) {
                intel = Some([a, b, c, d]);
            }
        }
        at += length;
    }
    Some(Reset {
        source: fields[1],
        reason: fields[3],
        intel,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A table holding a version record and then a health record for the
    /// reset reason, laid out as the XPS 14's of 20 November 22:00.
    fn table(source: u8, reason: u8, intel: Option<[u32; 4]>) -> Vec<u8> {
        let entries: Vec<Vec<u8>> = intel
            .map(|words| entry(INTEL, &words))
            .into_iter()
            .collect();
        table_of(source, reason, &entries)
    }

    /// A vendor's entry: its GUID, its length, revision 1, and its words.
    fn entry(vendor: [u8; 16], words: &[u32]) -> Vec<u8> {
        let mut entry = vendor.to_vec();
        let length = u16::try_from(VENDOR_HEADER + 4 * words.len()).unwrap();
        entry.extend_from_slice(&length.to_le_bytes());
        entry.extend_from_slice(&1u16.to_le_bytes());
        for word in words {
            entry.extend_from_slice(&word.to_le_bytes());
        }
        entry
    }

    /// The same table, holding the vendors' entries given.
    fn table_of(source: u8, reason: u8, entries: &[Vec<u8>]) -> Vec<u8> {
        let mut data = vec![0b111, source, 0, reason];
        data.extend_from_slice(&u16::try_from(entries.len()).unwrap().to_le_bytes());
        for entry in entries {
            data.extend_from_slice(entry);
        }
        // The device path, in UTF-16 as the firmware writes it.
        let path: Vec<u8> = "VenHw(0)\0"
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect();
        let offset = HEALTH_FIELDS + path.len();
        let mut health = vec![0; RECORD];
        health[0..2].copy_from_slice(&HEALTH.to_le_bytes());
        health[2..4].copy_from_slice(&u16::try_from(offset + data.len()).unwrap().to_le_bytes());
        health[4] = 1;
        health.extend_from_slice(&[0, 0, 2]);
        health.extend_from_slice(&RESET_REASON);
        health.extend_from_slice(&u32::try_from(offset).unwrap().to_le_bytes());
        health.extend_from_slice(&path);
        health.extend_from_slice(&data);
        // A version record with nothing in it comes first, as on the laptop.
        let version = [0, 0, 8, 0, 1, 0, 0, 0];
        let mut table = b"PHAT".to_vec();
        let length = HEADER + version.len() + health.len();
        table.extend_from_slice(&u32::try_from(length).unwrap().to_le_bytes());
        table.resize(HEADER, 0);
        table.extend_from_slice(&version);
        table.extend_from_slice(&health);
        table
    }

    const TWENTIETH: [u32; 4] = [0x400, 0, 1, 0x1039_0000];

    #[test]
    fn another_vendors_entry_before_intels_is_walked_past() {
        let maker = entry([0x11; 16], &[0xdead_beef]);
        let table = table_of(0b10, 0, &[maker, entry(INTEL, &TWENTIETH)]);
        assert_eq!(
            reset_of(&table).and_then(|reset| reset.intel),
            Some(TWENTIETH),
            "Intel's entry after another vendor's wasn't found"
        );
    }

    #[test]
    fn a_reset_record_pointing_inside_its_own_fields_is_passed_over() {
        let mut table = table(0b10, 0x24, None);
        // A reset reason record whose data offset points at its own header,
        // put before the real one.
        let mut bogus = vec![0; RECORD];
        bogus[0..2].copy_from_slice(&HEALTH.to_le_bytes());
        bogus[2..4].copy_from_slice(&u16::try_from(HEALTH_FIELDS).unwrap().to_le_bytes());
        bogus.extend_from_slice(&[0, 0, 2]);
        bogus.extend_from_slice(&RESET_REASON);
        bogus.extend_from_slice(&0u32.to_le_bytes());
        let at = HEADER + 8;
        table.splice(at..at, bogus);
        let length = u32::try_from(table.len()).unwrap();
        table[4..8].copy_from_slice(&length.to_le_bytes());
        assert_eq!(
            reset_of(&table).map(|reset| reset.reason),
            Some(0x24),
            "the record pointing at itself hid the one after it"
        );
    }

    #[test]
    fn the_reset_reason_is_read_past_other_records() {
        assert_eq!(
            reset_of(&table(1, 0, Some(TWENTIETH))),
            Some(Reset {
                source: 1,
                reason: 0,
                intel: Some(TWENTIETH)
            })
        );
    }

    #[test]
    fn intels_first_word_is_named_by_its_bits() {
        let reset = reset_of(&table(1, 0, Some(TWENTIETH))).expect("a reset reason");
        assert_eq!(reset.global_causes(true), ["PMC_FW"]);
        let seventeenth =
            reset_of(&table(1, 0, Some([0x40, 0, 0, 0x1039_0000]))).expect("a reason");
        assert_eq!(seventeenth.global_causes(true), ["MEGBL"]);
        let unnamed = reset_of(&table(1, 0, Some([1 << 29, 0, 0, 0]))).expect("a reset reason");
        assert_eq!(unnamed.global_causes(true), ["BIT29"]);
    }

    #[test]
    fn another_familys_bits_are_given_by_number() {
        let reset = reset_of(&table(1, 0, Some([0x402, 0, 0, 0]))).expect("a reset reason");
        assert_eq!(reset.global_causes(false), ["BIT1", "BIT10"]);
    }

    #[test]
    fn only_panther_lake_names_the_bits() {
        let panther = "processor\t: 0\nvendor_id\t: GenuineIntel\ncpu family\t: 6\n\
                       model\t\t: 204\nmodel name\t: Intel(R) Core(TM) Ultra X7 358H\n\n\
                       processor\t: 1\n";
        assert!(
            names_its_causes(panther),
            "Panther Lake's bits weren't named"
        );
        let meteor = panther.replace(": 204", ": 170");
        assert!(!names_its_causes(&meteor), "Meteor Lake's bits were named");
        let amd = panther.replace("GenuineIntel", "AuthenticAMD");
        assert!(!names_its_causes(&amd), "another maker's bits were named");
        assert!(!names_its_causes(""), "nothing read named the bits");
    }

    #[test]
    fn a_reason_and_source_are_said_in_words_and_names() {
        let reset = reset_of(&table(0b100, 0x22, None)).expect("a reset reason");
        assert_eq!(reset.source_words(), Some("the firmware"));
        assert_eq!(reset.source_name(), Some("firmware"));
        assert_eq!(reset.reason_words(), Some("a timeout"));
        assert_eq!(reset.reason_name(), Some("timeout"));
        assert_eq!(reset.intel, None);
        let unknown = reset_of(&table(1, 0, None)).expect("a reset reason");
        assert_eq!(
            (unknown.source_name(), unknown.reason_name()),
            (Some("unknown"), None)
        );
        let unlisted = reset_of(&table(0b110, 0x26, None)).expect("a reset reason");
        assert_eq!(
            (unlisted.source_words(), unlisted.reason_words()),
            (None, None)
        );
    }

    #[test]
    fn hostile_lengths_give_nothing_and_never_panic() {
        // A table claiming to run 4 GiB, past the bytes there are.
        let mut long = table(1, 0, Some(TWENTIETH));
        long[4..8].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(reset_of(&long).map(|reset| reset.reason), Some(0));
        // A health record whose data starts past its end.
        let mut past = table(1, 0, None);
        let offset = HEADER + 8 + 24;
        past[offset..offset + 4].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(reset_of(&past), None);
        // A vendor entry claiming fewer bytes than its own header.
        let mut short = table(1, 0, Some(TWENTIETH));
        let entry = short.len() - 36 + 16;
        short[entry..entry + 2].copy_from_slice(&3u16.to_le_bytes());
        assert_eq!(reset_of(&short).and_then(|reset| reset.intel), None);
    }

    #[test]
    fn a_table_that_isnt_whole_gives_nothing() {
        let whole = table(1, 0, Some(TWENTIETH));
        for cut in [0, 4, HEADER, HEADER + 3, whole.len() - 30] {
            assert_eq!(reset_of(&whole[..cut]), None, "cut at {cut}");
        }
        let mut other = whole.clone();
        other[..4].copy_from_slice(b"BERT");
        assert_eq!(reset_of(&other), None);
    }

    #[test]
    fn a_record_claiming_no_length_ends_the_reading() {
        let mut table = table(1, 0, None);
        table[HEADER + 2..HEADER + 4].copy_from_slice(&0u16.to_le_bytes());
        assert_eq!(reset_of(&table), None);
    }

    #[test]
    fn a_record_running_past_the_table_gives_nothing() {
        let mut table = table(1, 0, Some(TWENTIETH));
        let length = table.len();
        table.truncate(length - 8);
        let shortened = u32::try_from(table.len()).unwrap();
        table[4..8].copy_from_slice(&shortened.to_le_bytes());
        assert_eq!(reset_of(&table), None);
    }

    #[test]
    fn a_vendor_entry_the_record_lacks_keeps_what_came_before() {
        let mut table = table(0b10, 0x24, Some(TWENTIETH));
        // The reset data, its six fields and Intel's 36-byte entry, ends
        // the table; its vendor count now claims a second entry.
        let count = table.len() - (RESET_FIELDS + 36) + 4;
        table[count..count + 2].copy_from_slice(&2u16.to_le_bytes());
        assert_eq!(
            reset_of(&table),
            Some(Reset {
                source: 0b10,
                reason: 0x24,
                intel: Some(TWENTIETH)
            })
        );
    }
}
