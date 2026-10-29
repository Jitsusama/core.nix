//! What kept Panther Lake's package from resting: the power unit's second
//! region, "PTL fixed telemetry in Punit", counts each millisecond in which
//! something kept the package out of its deep idle states, which save the
//! most power when the laptop does little. The places are Intel's own
//! published layout for the region, Intel-PMT's `xml/PTL/1/ptl_aggregator.xml`
//! at 92aff2b00f67, whose sample groups are the region's eight-byte words in
//! order.
//!
//! Each counter is 32 bits wide, and counts the milliseconds in which its
//! part kept the package from resting at least once. Several parts have
//! more than one counter, numbered, and the layout doesn't say whether
//! they overlap, so a part is said as its longest, never their sum, which
//! could say more than the part did.

use crate::layout::Field;

/// The region's id, its `guid` file's number.
pub const GUID: u32 = 0x0308_6100;

/// How many eight-byte words the region holds.
pub const WORDS: usize = 93;

/// A part that can keep the package from resting.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Keeper {
    /// VPU: the NPU.
    Npu,
    /// IPU: the image processor, behind the cameras.
    ImageProcessor,
    Media,
    /// GT, and the link to the graphics die (`D2D_GDIE_CFI`).
    Graphics,
    /// DE: the display engine, which drives the screens.
    Display,
    /// The chipset's parts: USB, storage, audio and the rest.
    Chipset,
    /// CLUSTER0 and CLUSTER1: cores still busy.
    Cores,
    /// The power unit's own internal reasons, which Intel doesn't name.
    Internal,
    /// `OTHER_REASONS`.
    Other,
}

/// A 32-bit counter at its word's low or high half.
const fn low(word: usize) -> Field {
    Field {
        word,
        lsb: 0,
        msb: 31,
    }
}

const fn high(word: usize) -> Field {
    Field {
        word,
        lsb: 32,
        msb: 63,
    }
}

/// `PACKAGE_CSTATE_BLOCK_REFCNT`: the milliseconds in which anything kept
/// the package from resting.
pub const KEPT_AT_ALL: Field = low(1);

/// Each part's `PREVENT_PKGC_<part>_<n>` counters.
pub const KEEPERS: [(Keeper, &[Field]); 9] = [
    (Keeper::Npu, &[high(2), low(13)]),
    (Keeper::ImageProcessor, &[high(7), high(13), high(14)]),
    (Keeper::Media, &[high(4), low(5)]),
    (Keeper::Graphics, &[high(5), high(10)]),
    (Keeper::Display, &[low(9), low(14), high(16)]),
    (Keeper::Chipset, &[high(3), low(4), high(6), low(7)]),
    (Keeper::Cores, &[high(9), low(10), low(15), high(15)]),
    (
        Keeper::Internal,
        &[low(2), low(6), low(8), high(8), low(11), high(11), low(12)],
    ),
    (Keeper::Other, &[high(17)]),
];

/// The region's words at one moment.
#[derive(Clone, Debug, PartialEq)]
pub struct Region {
    words: Vec<u64>,
}

impl Region {
    /// The region from its file's bytes, refused unless it's the size the
    /// layout is for.
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() != WORDS * 8 {
            return None;
        }
        let (words, _) = bytes.as_chunks::<8>();
        let words = words.iter().map(|&word| u64::from_le_bytes(word)).collect();
        Some(Self { words })
    }

    #[cfg(test)]
    pub fn from_words(words: Vec<u64>) -> Self {
        Self { words }
    }

    fn read(&self, field: Field) -> u32 {
        u32::try_from(field.read(&self.words)).unwrap_or(u32::MAX)
    }
}

/// What kept the package from resting, and for what share of the time.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Kept {
    pub by: Keeper,
    pub share: f64,
}

/// What kept the package from resting between two readings.
#[derive(Clone, Debug, PartialEq)]
pub struct Resting {
    /// The share of the time anything did.
    pub kept: f64,
    /// Each part that did for at least a twentieth, the longest first.
    pub by: Vec<Kept>,
}

/// The share of `seconds` the milliseconds a counter counted between two
/// readings make. It wraps after 49 days, so it's read across a wrap; a
/// count more than the time allows, as when the power unit starts its
/// counters again after a resume, is read as nothing.
fn share(before: u32, after: u32, seconds: f64) -> f64 {
    let milliseconds = seconds * 1000.0;
    let counted = f64::from(after.wrapping_sub(before));
    if milliseconds <= 0.0 || counted > milliseconds * 1.5 {
        return 0.0;
    }
    (counted / milliseconds).clamp(0.0, 1.0)
}

/// What kept the package from resting between two readings `seconds`
/// apart.
pub fn resting(before: &Region, after: &Region, seconds: f64) -> Resting {
    let between = |field: Field| share(before.read(field), after.read(field), seconds);
    let mut by: Vec<Kept> = KEEPERS
        .iter()
        .map(|&(keeper, fields)| Kept {
            by: keeper,
            share: fields
                .iter()
                .map(|&field| between(field))
                .fold(0.0, f64::max),
        })
        .filter(|kept| kept.share >= crate::reading::COUNTS)
        .collect();
    by.sort_by(|a, b| b.share.total_cmp(&a.share));
    Resting {
        kept: between(KEPT_AT_ALL),
        by,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn region(set: &[(Field, u32)]) -> Region {
        let mut words = vec![0; WORDS];
        for &(field, value) in set {
            words[field.word] |= u64::from(value) << field.lsb;
        }
        Region::from_words(words)
    }

    #[test]
    fn a_region_of_another_size_is_refused() {
        assert!(Region::from_bytes(&[0; 3352]).is_none());
        assert!(Region::from_bytes(&[0; WORDS * 8]).is_some());
    }

    #[test]
    fn every_counter_lies_inside_the_region_and_none_is_shared() {
        let mut fields: Vec<(usize, u32)> = KEEPERS
            .iter()
            .flat_map(|(_, fields)| fields.iter())
            .chain([&KEPT_AT_ALL])
            .map(|field| (field.word, field.lsb))
            .collect();
        assert!(fields.iter().all(|&(word, _)| word < WORDS));
        let all = fields.len();
        fields.sort_unstable();
        fields.dedup();
        assert_eq!(fields.len(), all);
    }

    #[test]
    fn resting_all_second_says_nothing_kept_it() {
        let rested = resting(&region(&[]), &region(&[]), 1.0);
        assert!(rested.kept.abs() < 1e-9);
        assert!(rested.by.is_empty());
    }

    #[test]
    fn the_display_engine_keeping_it_a_quarter_of_a_second_is_said() {
        let before = region(&[(KEPT_AT_ALL, 100), (low(9), 40)]);
        let after = region(&[(KEPT_AT_ALL, 400), (low(9), 290)]);
        let rested = resting(&before, &after, 1.0);
        assert!((rested.kept - 0.3).abs() < 1e-9);
        assert_eq!(rested.by.len(), 1);
        assert_eq!(rested.by[0].by, Keeper::Display);
        assert!((rested.by[0].share - 0.25).abs() < 1e-9);
    }

    #[test]
    fn a_part_with_several_counters_is_said_as_its_longest() {
        let after = region(&[(high(7), 200), (high(13), 600), (high(14), 100)]);
        let rested = resting(&region(&[]), &after, 1.0);
        assert_eq!(rested.by[0].by, Keeper::ImageProcessor);
        assert!((rested.by[0].share - 0.6).abs() < 1e-9);
    }

    #[test]
    fn the_longest_keeping_comes_first_and_a_moment_isnt_said() {
        let after = region(&[(high(2), 100), (high(9), 700), (high(3), 30)]);
        let rested = resting(&region(&[]), &after, 1.0);
        let by: Vec<_> = rested.by.iter().map(|kept| kept.by).collect();
        assert_eq!(by, [Keeper::Cores, Keeper::Npu]);
    }

    #[test]
    fn a_counter_is_read_across_its_wrap() {
        let before = region(&[(KEPT_AT_ALL, u32::MAX - 99)]);
        let after = region(&[(KEPT_AT_ALL, 400)]);
        assert!((resting(&before, &after, 1.0).kept - 0.5).abs() < 1e-9);
    }

    #[test]
    fn counters_started_again_read_as_nothing() {
        let before = region(&[(KEPT_AT_ALL, 5_000_000), (low(9), 5_000_000)]);
        let after = region(&[(KEPT_AT_ALL, 10), (low(9), 10)]);
        let rested = resting(&before, &after, 1.0);
        assert!(rested.kept.abs() < 1e-9);
        assert!(rested.by.is_empty());
    }

    #[test]
    fn the_share_is_of_the_time_between() {
        let after = region(&[(KEPT_AT_ALL, 1000)]);
        assert!((resting(&region(&[]), &after, 2.0).kept - 0.5).abs() < 1e-9);
    }
}
