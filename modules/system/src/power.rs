//! Whether this machine lost power before its boot, from the firmware's own
//! record of it. A laptop whose embedded controller cuts the power leaves no
//! word in the journal, since nothing ran to write one; the firmware keeps
//! an error record across the cut instead, its boot error region (ACPI
//! BERT), which modules/firmware-errors copies where an administrator may
//! read it at every boot; for any other account it can't be read, and this
//! says nothing. No command owns that record, so this reads it, as the
//! machine's node will once it says it of every machine.
//!
//! It is worth a look for a day after the boot: long enough to be seen the
//! morning after a cut in the night, and no longer, since a cut a week ago
//! is history rather than news.

use std::path::Path;
use std::time::SystemTime;

/// Where modules/firmware-errors keeps each boot's records.
pub const RECORDS: &str = "/var/lib/firmware-errors";

/// How long after the boot a power loss before it is worth a look.
pub const NEWS_FOR: u64 = 24 * 60 * 60;

/// How long ago this machine booted after losing power, in seconds, when
/// the firmware recorded an error from the boot before and it was within a
/// day; none when it didn't, when it was longer ago, or when the record
/// can't be read.
#[must_use]
pub fn lost_before_boot(records: &Path, boot_id: &str, now: SystemTime) -> Option<u64> {
    let region = records.join(boot_id.replace('-', "")).join("bert-region");
    let head = std::fs::read(&region).ok()?;
    let written = std::fs::metadata(&region).ok()?.modified().ok()?;
    let ago = now.duration_since(written).ok()?.as_secs();
    judged(&head, ago)
}

/// The judgement alone: the region's first bytes and how long ago it was
/// copied, at the boot. The region opens with a generic error status
/// block, whose first four bytes, the block status, are zero while it
/// holds no error (ACPI's Generic Error Status Block), so a firmware that keeps an
/// empty region at every boot says nothing.
#[must_use]
pub fn judged(head: &[u8], ago: u64) -> Option<u64> {
    let status = u32::from_le_bytes(head.get(..4)?.try_into().ok()?);
    (status != 0 && ago <= NEWS_FOR).then_some(ago)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A region whose block status says one uncorrectable error.
    const AN_ERROR: [u8; 8] = [0x11, 0, 0, 0, 0, 0, 0, 0];

    #[test]
    fn an_error_from_the_boot_before_is_news_for_a_day() {
        assert_eq!(judged(&AN_ERROR, 3_600), Some(3_600));
        assert_eq!(judged(&AN_ERROR, NEWS_FOR), Some(NEWS_FOR));
        assert_eq!(
            judged(&AN_ERROR, NEWS_FOR + 1),
            None,
            "a day on it's history"
        );
    }

    #[test]
    fn an_empty_or_short_region_says_nothing() {
        assert_eq!(
            judged(&[0; 8], 60),
            None,
            "a firmware may keep an empty one"
        );
        assert_eq!(judged(&[0x11, 0], 60), None, "too short to hold a status");
        assert_eq!(judged(&[], 60), None);
    }

    #[test]
    fn a_boot_with_no_record_says_nothing() {
        let none = Path::new("/nonexistent/firmware-errors");
        assert_eq!(lost_before_boot(none, "a-b", SystemTime::now()), None);
    }
}
