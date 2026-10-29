//! What two readings of the region a moment apart say: each sensor's
//! temperature, and what held the processor, each core cluster and the NPU
//! back between them, as the share of that time each limit was holding.

use crate::layout::{
    self, CLUSTER_LIMITS, CLUSTERS, CRYSTAL, Field, Kind, Limit, NPU_LIMITS, PACKAGE_LIMITS,
    Sensor, TEMPERATURES,
};

/// The least share of the time a limit has to hold to count as holding the
/// processor back: a twentieth, so a limit touched for a moment in a
/// second isn't said.
pub const COUNTS: f64 = 0.05;

/// The region's words at one moment.
#[derive(Clone, Debug, PartialEq)]
pub struct Region {
    words: Vec<u64>,
}

impl Region {
    /// The region from its file's bytes, refused unless it's the size the
    /// layout is for.
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() != layout::WORDS * 8 {
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

    fn read(&self, field: Field) -> u64 {
        field.read(&self.words)
    }

    /// Each sensor's temperature, in whole degrees Celsius.
    pub fn temperatures(&self) -> Vec<(Sensor, i8)> {
        TEMPERATURES
            .iter()
            .map(|&(sensor, field)| {
                let [byte, ..] = self.read(field).to_le_bytes();
                (sensor, i8::from_le_bytes([byte]))
            })
            .collect()
    }

    fn ticks(&self, words: &[usize]) -> u64 {
        words.iter().fold(0, |sum: u64, &word| {
            sum.wrapping_add(self.read(Field {
                word,
                lsb: 0,
                msb: 63,
            }))
        })
    }
}

/// What held something back, and for what share of the time.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Held<T> {
    pub by: T,
    pub share: f64,
}

/// Why the processor as a whole was held back: a package limit, or a kind
/// of limit on its clusters that no package limit names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cause {
    Limit(Limit),
    Kind(Kind),
}

/// What held everything back between two readings.
#[derive(Clone, Debug, PartialEq)]
pub struct HeldBack {
    /// The processor as a whole, the longest-holding first.
    pub processor: Vec<Held<Cause>>,
    /// Each core cluster, in the region's order.
    pub clusters: Vec<Vec<Held<Kind>>>,
    pub npu: Vec<Held<Kind>>,
}

/// The share of `seconds` a counter's ticks between two readings make,
/// from 0 to 1. A counter that went backwards, as one does when the
/// machine resumes, is read as nothing.
fn share(before: u64, after: u64, ticks_per_second: f64, seconds: f64) -> f64 {
    if after < before || seconds <= 0.0 {
        return 0.0;
    }
    #[expect(
        clippy::cast_precision_loss,
        reason = "a second's ticks are tens of millions, far inside a float's exact range"
    )]
    let ticks = (after - before) as f64;
    (ticks / (ticks_per_second * seconds)).clamp(0.0, 1.0)
}

/// Those that held for at least `COUNTS`, the longest first.
fn counted<T: Copy>(held: impl Iterator<Item = Held<T>>) -> Vec<Held<T>> {
    let mut held: Vec<_> = held.filter(|held| held.share >= COUNTS).collect();
    held.sort_by(|a, b| b.share.total_cmp(&a.share));
    held
}

/// What held the processor, its clusters and the NPU back between two
/// readings `seconds` apart.
pub fn held_back(before: &Region, after: &Region, seconds: f64) -> HeldBack {
    let rate = layout::ticks_per_second(after.read(CRYSTAL));
    let between = |words: &[usize]| share(before.ticks(words), after.ticks(words), rate, seconds);

    let clusters: Vec<Vec<Held<Kind>>> = (0..CLUSTERS)
        .map(|cluster| {
            counted(CLUSTER_LIMITS.iter().map(|&(kind, first)| Held {
                by: kind,
                share: between(&[layout::cluster_limit(first, cluster).word]),
            }))
        })
        .collect();
    let npu = counted(NPU_LIMITS.iter().map(|&(kind, field)| Held {
        by: kind,
        share: between(&[field.word]),
    }));

    let limits = counted(PACKAGE_LIMITS.iter().map(|&(limit, words)| Held {
        by: Cause::Limit(limit),
        share: between(words),
    }));
    // A kind that held a cluster back is said for the processor as the
    // longest it held any cluster, unless a package limit already names it:
    // a power limit on the cores is PL1's or PL2's when either held, and
    // heat is PROCHOT's when that did.
    let named = |kind: Kind| {
        limits.iter().any(|held| match (kind, held.by) {
            (Kind::Power, Cause::Limit(limit)) => {
                !matches!(limit, Limit::Prochot | Limit::RegulatorHot)
            }
            (Kind::Heat, Cause::Limit(limit)) => {
                matches!(limit, Limit::Prochot | Limit::RegulatorHot)
            }
            _ => false,
        })
    };
    let kinds = CLUSTER_LIMITS
        .iter()
        .filter(|&&(kind, _)| !named(kind))
        .map(|&(kind, _)| Held {
            by: Cause::Kind(kind),
            share: clusters
                .iter()
                .flat_map(|held| {
                    held.iter()
                        .filter(|held| held.by == kind)
                        .map(|held| held.share)
                })
                .fold(0.0, f64::max),
        });
    let mut processor = limits.clone();
    processor.extend(counted(kinds));
    processor.sort_by(|a, b| b.share.total_cmp(&a.share));

    HeldBack {
        processor,
        clusters,
        npu,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECOND: u64 = 38_400_000;

    fn region(set: &[(usize, u64)]) -> Region {
        let mut words = vec![0; layout::WORDS];
        words[0] = 2 << 32;
        for &(word, value) in set {
            words[word] = value;
        }
        Region::from_words(words)
    }

    #[test]
    fn a_region_of_another_size_is_refused() {
        assert!(Region::from_bytes(&[0; 744]).is_none());
        assert!(Region::from_bytes(&[0; layout::WORDS * 8]).is_some());
    }

    #[test]
    fn the_words_are_little_endian() {
        let mut bytes = vec![0; layout::WORDS * 8];
        bytes[15 * 8 + 4] = 41;
        let region = Region::from_bytes(&bytes);
        let npu = region.and_then(|r| r.temperatures().first().copied());
        assert_eq!(npu, Some((Sensor::Npu, 41)));
    }

    #[test]
    fn a_temperature_below_freezing_reads_below_zero() {
        let temperatures = region(&[(15, 0xf6)]).temperatures();
        assert!(temperatures.contains(&(Sensor::Chipset, -10)));
    }

    #[test]
    fn nothing_holding_says_nothing() {
        let held = held_back(&region(&[]), &region(&[]), 1.0);
        assert!(held.processor.is_empty());
        assert!(held.npu.is_empty());
        assert!(held.clusters.iter().all(Vec::is_empty));
        assert_eq!(held.clusters.len(), CLUSTERS);
    }

    #[test]
    fn pl1_holding_most_of_a_second_names_pl1_not_power() {
        let before = region(&[(150, 1000), (111, 0)]);
        let after = region(&[(150, 1000 + SECOND * 8 / 10), (111, SECOND * 7 / 10)]);
        let held = held_back(&before, &after, 1.0);
        assert_eq!(held.processor.len(), 1);
        assert_eq!(held.processor[0].by, Cause::Limit(Limit::Pl1));
        assert!((held.processor[0].share - 0.8).abs() < 1e-9);
        assert_eq!(held.clusters[0][0].by, Kind::Power);
    }

    #[test]
    fn heat_on_one_cluster_alone_is_said_as_heat() {
        let after = region(&[(99 + 4, SECOND / 2)]);
        let held = held_back(&region(&[]), &after, 1.0);
        assert_eq!(held.processor.len(), 1);
        assert_eq!(held.processor[0].by, Cause::Kind(Kind::Heat));
        assert!(held.clusters[4].iter().any(|h| h.by == Kind::Heat));
        assert!(held.clusters[3].is_empty());
    }

    #[test]
    fn prochot_names_the_heat() {
        let after = region(&[(144, SECOND), (99, SECOND)]);
        let held = held_back(&region(&[]), &after, 1.0);
        assert_eq!(held.processor.len(), 1);
        assert_eq!(held.processor[0].by, Cause::Limit(Limit::Prochot));
    }

    #[test]
    fn the_regulators_count_together() {
        let after = region(&[(145, SECOND / 20), (147, SECOND / 20)]);
        let held = held_back(&region(&[]), &after, 1.0);
        assert_eq!(held.processor[0].by, Cause::Limit(Limit::RegulatorHot));
        assert!((held.processor[0].share - 0.1).abs() < 1e-9);
    }

    #[test]
    fn a_moment_under_a_twentieth_isnt_said() {
        let after = region(&[(150, SECOND / 25)]);
        assert!(held_back(&region(&[]), &after, 1.0).processor.is_empty());
    }

    #[test]
    fn the_longest_holding_comes_first() {
        let after = region(&[(149, SECOND / 5), (150, SECOND / 2), (122 + 1, SECOND / 3)]);
        let held = held_back(&region(&[]), &after, 1.0);
        let by: Vec<_> = held.processor.iter().map(|h| h.by).collect();
        assert_eq!(
            by,
            [
                Cause::Limit(Limit::Pl1),
                Cause::Kind(Kind::Current),
                Cause::Limit(Limit::Pl2)
            ]
        );
    }

    #[test]
    fn a_counter_going_backwards_reads_as_nothing() {
        let held = held_back(&region(&[(150, SECOND)]), &region(&[(150, 5)]), 1.0);
        assert!(held.processor.is_empty());
    }

    #[test]
    fn the_npu_says_its_own_limits() {
        let after = region(&[(109, SECOND)]);
        let held = held_back(&region(&[]), &after, 2.0);
        assert_eq!(held.npu.len(), 1);
        assert!((held.npu[0].share - 0.5).abs() < 1e-9);
    }
}
