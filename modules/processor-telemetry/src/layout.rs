//! Where Panther Lake's power unit keeps each reading this reader uses, in
//! the telemetry region Intel's PMT driver shows as a `telem` file. The places
//! are Intel's own published layout for the region, Intel-PMT's
//! `xml/PTL/0/ptl_aggregator.xml` at 92aff2b00f67, whose sample groups are the
//! region's eight-byte words in order: a reading is the bits from its least to
//! its most significant within its word.

/// The region's id, its `guid` file's number: "PTL normal telemetry in Punit".
pub const GUID: u32 = 0x0308_6000;

/// How many eight-byte words the region holds, so a region of another size
/// is refused rather than misread.
pub const WORDS: usize = 419;

/// One reading's place: its word, and its bits within it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Field {
    pub word: usize,
    pub lsb: u32,
    pub msb: u32,
}

impl Field {
    /// A whole word, as every counter is.
    const fn whole(word: usize) -> Self {
        Self {
            word,
            lsb: 0,
            msb: 63,
        }
    }

    /// The reading, from the region's words.
    pub fn read(self, words: &[u64]) -> u64 {
        let width = self.msb - self.lsb + 1;
        let shifted = words[self.word] >> self.lsb;
        if width == 64 {
            shifted
        } else {
            shifted & ((1 << width) - 1)
        }
    }
}

/// Which crystal the counters tick at, as `XTAL_FREQ`'s code: 0 for 24 MHz,
/// 1 for 19.2, 2 for 38.4 and 3 for 25.
pub const CRYSTAL: Field = Field {
    word: 0,
    lsb: 32,
    msb: 33,
};

/// The crystal's ticks a second, by its code.
pub fn ticks_per_second(code: u64) -> f64 {
    match code {
        0 => 24.0e6,
        1 => 19.2e6,
        3 => 25.0e6,
        _ => 38.4e6,
    }
}

/// A part with a temperature sensor no hwmon chip shows, its reading a
/// signed whole degree Celsius.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sensor {
    Npu,
    ImageProcessor,
    Media,
    Display,
    SystemAgent,
    Chipset,
}

/// Each sensor's place: `VPU_TEMP`, `IPU_TEMP`, `Media_TEMP`, `DE_TEMP`,
/// `SA_TEMP` and `PCH_TEMP`, all in word 15.
pub const TEMPERATURES: [(Sensor, Field); 6] = [
    (
        Sensor::Npu,
        Field {
            word: 15,
            lsb: 32,
            msb: 39,
        },
    ),
    (
        Sensor::ImageProcessor,
        Field {
            word: 15,
            lsb: 16,
            msb: 23,
        },
    ),
    (
        Sensor::Media,
        Field {
            word: 15,
            lsb: 40,
            msb: 47,
        },
    ),
    (
        Sensor::Display,
        Field {
            word: 15,
            lsb: 24,
            msb: 31,
        },
    ),
    (
        Sensor::SystemAgent,
        Field {
            word: 15,
            lsb: 8,
            msb: 15,
        },
    ),
    (
        Sensor::Chipset,
        Field {
            word: 15,
            lsb: 0,
            msb: 7,
        },
    ),
];

/// How many core clusters the region counts for: `CCP_0` to `CCP_6`.
pub const CLUSTERS: usize = 7;

/// Why a part ran slower than it was asked to, as the power unit sorts its
/// "slow limits".
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// THERM: a temperature limit.
    Heat,
    /// POWER: a power limit.
    Power,
    /// EDP: the most current the part may draw.
    Current,
    /// OTHER: anything else.
    Other,
}

/// The first of each kind's seven cluster counters,
/// `PERF_LIMIT_<kind>_CCP_RESIDENCY_0`, the rest following in order.
pub const CLUSTER_LIMITS: [(Kind, usize); 4] = [
    (Kind::Heat, 99),
    (Kind::Power, 111),
    (Kind::Current, 122),
    (Kind::Other, 133),
];

/// Each kind's counter for the NPU, `PERF_LIMIT_<kind>_VPU_RESIDENCY`.
pub const NPU_LIMITS: [(Kind, Field); 4] = [
    (Kind::Heat, Field::whole(109)),
    (Kind::Power, Field::whole(121)),
    (Kind::Current, Field::whole(132)),
    (Kind::Other, Field::whole(143)),
];

/// A limit on the whole package.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Limit {
    /// PL1, the power it may draw for long.
    Pl1,
    /// PL2, the power it may draw in a burst.
    Pl2,
    /// PL3, the peak power it may draw at all.
    Pl3,
    /// The platform's PL1, the whole laptop's power over the long run.
    PlatformPl1,
    /// The platform's PL2.
    PlatformPl2,
    /// The platform's PL3.
    PlatformPl3,
    /// PROCHOT: told to slow down, by itself or the board, for heat.
    Prochot,
    /// A voltage regulator for the cores, graphics or system agent too hot.
    RegulatorHot,
}

/// Each package limit's counters: `PROCHOT_RESIDENCY`, the three
/// `VCC*_VRHOT_RESIDENCY` counters taken together, PL3, PL2 and PL1, then
/// PSYS's PL3, PL2 and PL1.
pub const PACKAGE_LIMITS: [(Limit, &[usize]); 8] = [
    (Limit::Prochot, &[144]),
    (Limit::RegulatorHot, &[145, 146, 147]),
    (Limit::Pl3, &[148]),
    (Limit::Pl2, &[149]),
    (Limit::Pl1, &[150]),
    (Limit::PlatformPl3, &[151]),
    (Limit::PlatformPl2, &[152]),
    (Limit::PlatformPl1, &[153]),
];

/// The counter for a kind of limit on one cluster.
pub fn cluster_limit(first: usize, cluster: usize) -> Field {
    Field::whole(first + cluster)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_field_takes_only_its_own_bits() {
        let mut words = vec![0; WORDS];
        words[15] = 0x0000_2a33_2f30_3132;
        assert_eq!(TEMPERATURES[0].1.read(&words), 0x33);
        assert_eq!(TEMPERATURES[5].1.read(&words), 0x32);
        assert_eq!(TEMPERATURES[2].1.read(&words), 0x2a);
    }

    #[test]
    fn a_whole_word_reads_all_sixty_four_bits() {
        let mut words = vec![0; WORDS];
        words[150] = u64::MAX;
        assert_eq!(Field::whole(150).read(&words), u64::MAX);
    }

    #[test]
    fn the_crystal_code_names_its_rate() {
        let mut words = vec![0; WORDS];
        words[0] = 2 << 32;
        assert!((ticks_per_second(CRYSTAL.read(&words)) - 38.4e6).abs() < 1.0);
    }

    #[test]
    fn every_counter_lies_inside_the_region() {
        let last = CLUSTER_LIMITS
            .iter()
            .map(|(_, first)| first + CLUSTERS - 1)
            .max();
        assert!(last.is_some_and(|last| last < WORDS));
        assert!(
            PACKAGE_LIMITS
                .iter()
                .flat_map(|(_, words)| words.iter())
                .all(|&w| w < WORDS)
        );
    }

    #[test]
    fn no_two_readings_share_a_counter() {
        let mut words: Vec<usize> = CLUSTER_LIMITS
            .iter()
            .flat_map(|&(_, first)| (0..CLUSTERS).map(move |c| first + c))
            .chain(NPU_LIMITS.iter().map(|(_, field)| field.word))
            .chain(
                PACKAGE_LIMITS
                    .iter()
                    .flat_map(|(_, words)| words.iter().copied()),
            )
            .collect();
        let all = words.len();
        words.sort_unstable();
        words.dedup();
        assert_eq!(words.len(), all);
    }
}
