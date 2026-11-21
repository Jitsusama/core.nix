//! Reads the Crash Log an Intel firmware leaves in the boot error region
//! when it resets the machine, with Intel's own decoder.
//!
//! The region is the generic error status block BERT points at, as the
//! kernel shows it in /sys/firmware/acpi/tables/data/BERT. The decoder
//! takes it after the four bytes "BERR", decodes each record against the
//! register layouts of its processor, built into the decoder, and says
//! what it finds as tags, such as `RESET_CAUSE.GLOBAL_RESET.PMC_FW` or
//! `MCA.BANK3.INTERNAL_TIMER_ERROR.MSCOD_E184H`, the likeliest first.
//!
//! The block's own lengths are read first: a block that holds no entries
//! is no error at all, and one whose entries run past it was cut, which
//! the decoder would otherwise report as it reports entries of another
//! kind. The kernel clears the block's status word once it has read the
//! region at boot (drivers/acpi/apei/bert.c), so every copy read after
//! reads zero there, Crash Log or not, and only the entries' length says
//! whether it holds any. Only the entries are decoded, since a firmware may
//! reserve more than they take.
//!
//! The decoder says the same when the entries hold no Crash Log and when
//! they hold one whose records it can't decode, so the region is searched
//! for the Crash Log's record GUID to tell the two apart. The decoder
//! comes from elsewhere and reads bytes the firmware wrote, so a panic in
//! it is said as a region it couldn't read, and the other boots are still
//! listed. An abort isn't a panic: one that ran it out of memory would
//! still end the command.

use intel_crashlog::prelude::{Analyzer, CollateralManager, CrashLog, Error};
use std::cell::Cell;
use std::panic::{self, AssertUnwindSafe};
use std::sync::Once;

thread_local! {
    /// Whether this thread is in the decoder, where a panic is said in the
    /// row rather than printed.
    static DECODING: Cell<bool> = const { Cell::new(false) };
}

/// The panic hook is the whole program's, so it's set once.
static QUIET_WHILE_DECODING: Once = Once::new();

/// The generic error status block's fields before its entries: its
/// status, its raw data's offset and length, its entries' length and its
/// severity (ACPI 6.5, 18.3.2.7.1).
const STATUS_BLOCK: usize = 20;
/// Where the block gives its entries' length.
const ENTRIES_LENGTH: usize = 12;
/// The firmware error record's GUID for an Intel Crash Log,
/// 8f87f311-c998-4d9e-a0c4-6065518c4f6d, as it lies in the region.
const CRASH_LOG_RECORD: [u8; 16] = [
    0x11, 0xf3, 0x87, 0x8f, 0x98, 0xc9, 0x9e, 0x4d, 0xa0, 0xc4, 0x60, 0x65, 0x51, 0x8c, 0x4f, 0x6d,
];

/// What the region holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Crash {
    /// A Crash Log, and its tags, which may be none where the analyzer
    /// names nothing in it.
    Tags(Vec<String>),
    /// No error kept: the block holds no entries.
    None,
    /// Error entries, none of them a Crash Log the decoder can read.
    Other,
    /// A region cut short or the decoder couldn't read, and why.
    Unreadable(String),
}

/// What the region holds, by its bytes.
pub fn crash_of(region: &[u8]) -> Crash {
    let Some(length) = region
        .get(ENTRIES_LENGTH..ENTRIES_LENGTH + 4)
        .and_then(|bytes| bytes.try_into().ok())
        .map(u32::from_le_bytes)
        .and_then(|length| usize::try_from(length).ok())
    else {
        return Crash::Unreadable("the region is shorter than its status block".to_owned());
    };
    if length == 0 {
        return Crash::None;
    }
    if region.len() < STATUS_BLOCK.saturating_add(length) {
        return Crash::Unreadable(format!(
            "the region is cut short: its entries take {length} bytes, and {} are kept",
            region.len().saturating_sub(STATUS_BLOCK)
        ));
    }
    let entries = &region[..STATUS_BLOCK + length];
    guarded(|| decoded(entries))
}

/// What `decode` gives, or a region the decoder failed on where it
/// panicked. The hook that would print the panic over the listing is kept
/// quiet on the decoding thread alone: swapped for each decode, the hook
/// of tests run beside it on other threads would be lost, and with it what
/// their failures say.
fn guarded(decode: impl FnOnce() -> Crash) -> Crash {
    QUIET_WHILE_DECODING.call_once(|| {
        let said = panic::take_hook();
        panic::set_hook(Box::new(move |info| {
            if !DECODING.with(Cell::get) {
                said(info);
            }
        }));
    });
    DECODING.with(|decoding| decoding.set(true));
    let read = panic::catch_unwind(AssertUnwindSafe(decode));
    DECODING.with(|decoding| decoding.set(false));
    read.unwrap_or_else(|_| Crash::Unreadable("the decoder failed on it".to_owned()))
}

/// What a region's whole entries hold, as Intel's decoder reads them.
fn decoded(entries: &[u8]) -> Crash {
    let mut bert = b"BERR".to_vec();
    bert.extend_from_slice(entries);
    let crashlog = match CrashLog::from_slice(&bert) {
        Ok(crashlog) => crashlog,
        // The decoder says a Crash Log is invalid both when the entries
        // hold none and when it can't decode the one they hold.
        Err(Error::NoCrashLogFound | Error::InvalidCrashLog) => {
            return if entries
                .windows(CRASH_LOG_RECORD.len())
                .any(|window| window == CRASH_LOG_RECORD)
            {
                Crash::Unreadable(
                    "a Crash Log is kept, but its records can't be decoded".to_owned(),
                )
            } else {
                Crash::Other
            };
        }
        Err(error) => return Crash::Unreadable(error.to_string()),
    };
    let mut layouts = match CollateralManager::embedded_tree() {
        Ok(layouts) => layouts,
        Err(error) => return Crash::Unreadable(error.to_string()),
    };
    let nodes = crashlog.decode(&mut layouts);
    let tags = Analyzer::default().with_input(&nodes).analyze().tags;
    Crash::Tags(tags.iter().map(ToString::to_string).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Intel's own sample of a core's three-strike timeout, from its
    /// decoder's tests (tests/README.md says where).
    const SAMPLE: &[u8] = include_bytes!("../tests/three-strike-timeout.crashlog");

    /// How long a BERT table is: ACPI's 36-byte header, the region's
    /// length in 4 and its address in 8. Intel's decoder writes a stand-in
    /// header whose own length field isn't this, so it's counted here.
    const BERT_TABLE: usize = 48;

    /// The sample as the kernel would show the region: Intel's decoder
    /// writes it as a BERT table and then its region.
    fn region() -> Vec<u8> {
        let bert = CrashLog::from_slice(SAMPLE)
            .expect("the sample decodes")
            .to_bert();
        assert_eq!(&bert[..4], b"BERT", "the decoder writes a BERT table first");
        bert[BERT_TABLE..].to_vec()
    }

    #[test]
    fn a_core_timeout_is_named() {
        let Crash::Tags(tags) = crash_of(&region()) else {
            panic!("the sample's region holds no Crash Log");
        };
        assert!(
            tags.iter()
                .any(|tag| tag == "MCA.BANK3.INTERNAL_TIMER_ERROR.MSCOD_E184H"),
            "the tags don't name the core's timer: {tags:?}"
        );
        assert!(
            tags.iter().any(|tag| tag.starts_with("CORE_TIMEOUT")),
            "the tags don't name a core timeout: {tags:?}"
        );
    }

    #[test]
    fn a_region_reserved_past_its_entries_is_read_to_their_end() {
        let mut padded = region();
        padded.extend_from_slice(&[0; 4096]);
        let Crash::Tags(tags) = crash_of(&padded) else {
            panic!("the padded region holds no Crash Log");
        };
        assert!(
            tags.iter().any(|tag| tag.starts_with("CORE_TIMEOUT")),
            "the padding hid the core timeout: {tags:?}"
        );
    }

    /// A region holding one entry: revision 3.0's 72-byte header for the
    /// section named, then its data.
    fn region_of(section: [u8; 16], data: &[u8]) -> Vec<u8> {
        let mut entry = vec![0; 72];
        entry[..16].copy_from_slice(&section);
        entry[20..22].copy_from_slice(&0x300u16.to_le_bytes());
        let length = u32::try_from(data.len()).expect("a short entry");
        entry[24..28].copy_from_slice(&length.to_le_bytes());
        entry.extend_from_slice(data);
        let mut region = vec![0; STATUS_BLOCK];
        let entries = u32::try_from(entry.len()).expect("a short region");
        region[ENTRIES_LENGTH..ENTRIES_LENGTH + 4].copy_from_slice(&entries.to_le_bytes());
        region.extend_from_slice(&entry);
        region
    }

    #[test]
    fn a_crash_log_that_wont_decode_is_said_unreadable_not_another_kind() {
        // The firmware error record section, 81212a96-09ed-4996-9471-
        // 8d729c8e69ed, holding a revision 2 record for a Crash Log whose
        // first record's header names a type the decoder doesn't know.
        let section = [
            0x96, 0x2a, 0x21, 0x81, 0xed, 0x09, 0x96, 0x49, 0x94, 0x71, 0x8d, 0x72, 0x9c, 0x8e,
            0x69, 0xed,
        ];
        let mut record = vec![0; 16];
        record[1] = 2;
        record.extend_from_slice(&CRASH_LOG_RECORD);
        record.extend_from_slice(&[0xff; 16]);
        let read = crash_of(&region_of(section, &record));
        assert!(
            matches!(read, Crash::Unreadable(ref why) if why.contains("can't be decoded")),
            "a Crash Log that won't decode was said as {read:?}"
        );
    }

    #[test]
    fn a_region_with_nothing_in_it_is_no_error() {
        // A generic error status block that holds no entries.
        assert_eq!(crash_of(&[0; 64]), Crash::None);
        assert_eq!(crash_of(&[0; STATUS_BLOCK]), Crash::None);
    }

    #[test]
    fn a_cut_region_is_said_to_be_cut() {
        let region = region();
        for cut in [
            0,
            ENTRIES_LENGTH + 2,
            STATUS_BLOCK,
            region.len() / 2,
            region.len() - 1,
        ] {
            let read = crash_of(&region[..cut]);
            assert!(
                matches!(read, Crash::Unreadable(ref why) if why.contains("short")),
                "a region cut at {cut} of {} wasn't said to be cut",
                region.len()
            );
        }
    }

    #[test]
    fn an_entry_of_another_kind_is_no_crash_log() {
        // One entry of a section nobody names, and no data.
        assert_eq!(crash_of(&region_of([0; 16], &[])), Crash::Other);
    }

    #[test]
    fn a_panic_in_the_decoder_is_said_and_the_thread_left_as_it_was() {
        let said = guarded(|| panic!("a decoder that fails"));
        assert_eq!(
            said,
            Crash::Unreadable("the decoder failed on it".to_owned())
        );
        assert!(
            !DECODING.with(Cell::get),
            "a later panic on this thread would go unsaid"
        );
        assert_eq!(guarded(|| Crash::None), Crash::None);
    }
}
