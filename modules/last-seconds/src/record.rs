//! One second's reading as the ring keeps it: 128 bytes, little-endian,
//! ending in a CRC-32 of the rest, so a record the power cut while it was
//! being written reads as no record rather than as a wrong one.
//!
//! A value the machine didn't give, or gave none of yet, is kept as its
//! type's sentinel (`None` here), never as a zero that would read as a
//! reading.

/// How many bytes a record takes in the ring.
pub const SIZE: usize = 128;
/// What opens every record, so a slot never written reads as empty.
const MAGIC: [u8; 4] = *b"LSR1";
/// Where the CRC begins: it covers every byte before it.
const CRC_AT: usize = SIZE - 4;
/// How many rest states a record has room for.
pub const RESTS: usize = 8;
/// How many fans a record has room for.
pub const FANS: usize = 4;

/// Its flag that the recorder was stopped after writing it, as at a
/// shutdown, so a boot whose last record has it ended as it was asked to.
const STOPPED: u8 = 1;
/// Its flag that the machine went to sleep after it was written, so a
/// boot whose last record has it ended asleep or waking.
const ASLEEP: u8 = 2;
/// Its flag that the fans, the battery and the chargers were read for it,
/// rather than carried from a record before.
const FIRMWARE_READ: u8 = 4;

/// What the processor and the machine were doing over one second.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Record {
    /// Its place in the boot's records, from 0.
    pub sequence: u32,
    /// When it was read, in milliseconds since 1970.
    pub at_ms: i64,
    /// When it was read, in milliseconds since the boot, sleep included.
    pub since_boot_ms: u64,
    /// What the processor package drew over the second, in milliwatts.
    pub package_mw: Option<u32>,
    /// What the whole platform drew over the second, in milliwatts.
    pub platform_mw: Option<u32>,
    /// The package's temperature, in thousandths of a degree Celsius.
    pub package_mc: Option<i32>,
    /// Each fan's speed, in revolutions a minute.
    pub fans: [Option<u16>; FANS],
    /// The load average over the last minute, in hundredths.
    pub load: Option<u32>,
    /// How many tasks were running, and how many there were.
    pub tasks: Option<(u16, u16)>,
    /// Microseconds over the second that some task waited on the
    /// processor, that some and that every task waited on memory, and
    /// that some and that every task waited on the disk.
    pub stalls: Stalls,
    /// Memory still to be had without swapping, in KiB.
    pub available_kib: Option<u32>,
    /// Swap in use, in KiB.
    pub swapped_kib: Option<u32>,
    /// The battery's voltage, in millivolts.
    pub battery_mv: Option<u32>,
    /// The battery's current as its driver gives it, in milliamps.
    pub battery_ma: Option<i32>,
    /// How full the battery is, in percent.
    pub battery_percent: Option<u8>,
    /// What the battery says it's doing.
    pub battery: Battery,
    /// Whether a charger or another outside supply is giving power.
    pub outside_power: Option<bool>,
    /// Microseconds over the second the processors spent in each rest
    /// state, summed over them, in the order the ring's header names.
    pub rests: [Option<u32>; RESTS],
    /// Whether the recorder was stopped after writing it.
    pub stopped: bool,
    /// Whether the machine went to sleep after it was written.
    pub asleep: bool,
    /// Whether its fans, battery and chargers were read for it, rather
    /// than carried from the last record that read them.
    pub firmware_read: bool,
}

/// How long tasks waited over a second, in microseconds, by what on.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Stalls {
    pub cpu_some: Option<u32>,
    pub memory_some: Option<u32>,
    pub memory_full: Option<u32>,
    pub io_some: Option<u32>,
    pub io_full: Option<u32>,
}

/// What a battery's driver says it's doing, as the kernel names it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Battery {
    /// There's no battery, or it didn't say.
    #[default]
    Unsaid,
    Charging,
    Discharging,
    NotCharging,
    Full,
}

impl Battery {
    /// What the kernel's status file says, as one of these.
    pub fn of_status(status: &str) -> Self {
        match status.trim() {
            "Charging" => Self::Charging,
            "Discharging" => Self::Discharging,
            "Not charging" => Self::NotCharging,
            "Full" => Self::Full,
            _ => Self::Unsaid,
        }
    }

    /// Its name, for an answer's JSON.
    pub fn name(self) -> Option<&'static str> {
        match self {
            Self::Unsaid => None,
            Self::Charging => Some("charging"),
            Self::Discharging => Some("discharging"),
            Self::NotCharging => Some("not-charging"),
            Self::Full => Some("full"),
        }
    }

    fn byte(self) -> u8 {
        match self {
            Self::Unsaid => 0,
            Self::Charging => 1,
            Self::Discharging => 2,
            Self::NotCharging => 3,
            Self::Full => 4,
        }
    }

    fn of_byte(byte: u8) -> Self {
        match byte {
            1 => Self::Charging,
            2 => Self::Discharging,
            3 => Self::NotCharging,
            4 => Self::Full,
            _ => Self::Unsaid,
        }
    }
}

/// The CRC-32 every zip and Ethernet frame uses, worked a bit at a time:
/// a record's 124 bytes once a second don't need its table.
pub fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = !0u32;
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            let low = crc & 1;
            crc >>= 1;
            if low == 1 {
                crc ^= 0xEDB8_8320;
            }
        }
    }
    !crc
}

/// Writes little-endian values one after another into a record's bytes.
struct Writer<'a> {
    bytes: &'a mut [u8; SIZE],
    at: usize,
}

impl Writer<'_> {
    fn put(&mut self, value: &[u8]) {
        self.bytes[self.at..self.at + value.len()].copy_from_slice(value);
        self.at += value.len();
    }
}

/// Reads little-endian values one after another from a record's bytes.
struct Reader<'a> {
    bytes: &'a [u8; SIZE],
    at: usize,
}

impl Reader<'_> {
    fn take<const N: usize>(&mut self) -> [u8; N] {
        let mut value = [0; N];
        value.copy_from_slice(&self.bytes[self.at..self.at + N]);
        self.at += N;
        value
    }

    fn u8(&mut self) -> u8 {
        self.take::<1>()[0]
    }

    fn u16(&mut self) -> Option<u16> {
        Some(u16::from_le_bytes(self.take())).filter(|value| *value != u16::MAX)
    }

    fn u32(&mut self) -> Option<u32> {
        Some(u32::from_le_bytes(self.take())).filter(|value| *value != u32::MAX)
    }

    fn i32(&mut self) -> Option<i32> {
        Some(i32::from_le_bytes(self.take())).filter(|value| *value != i32::MIN)
    }
}

impl Record {
    /// Its bytes as the ring keeps them.
    pub fn encode(&self) -> [u8; SIZE] {
        let mut bytes = [0; SIZE];
        let mut out = Writer {
            bytes: &mut bytes,
            at: 0,
        };
        let u16_of = |value: Option<u16>| value.unwrap_or(u16::MAX).to_le_bytes();
        let u32_of = |value: Option<u32>| value.unwrap_or(u32::MAX).to_le_bytes();
        let i32_of = |value: Option<i32>| value.unwrap_or(i32::MIN).to_le_bytes();
        out.put(&MAGIC);
        out.put(&self.sequence.to_le_bytes());
        out.put(&self.at_ms.to_le_bytes());
        out.put(&self.since_boot_ms.to_le_bytes());
        out.put(&u32_of(self.package_mw));
        out.put(&u32_of(self.platform_mw));
        out.put(&i32_of(self.package_mc));
        for fan in self.fans {
            out.put(&u16_of(fan));
        }
        out.put(&u32_of(self.load));
        out.put(&u16_of(self.tasks.map(|(running, _)| running)));
        out.put(&u16_of(self.tasks.map(|(_, all)| all)));
        let stalls = self.stalls;
        for stall in [
            stalls.cpu_some,
            stalls.memory_some,
            stalls.memory_full,
            stalls.io_some,
            stalls.io_full,
        ] {
            out.put(&u32_of(stall));
        }
        out.put(&u32_of(self.available_kib));
        out.put(&u32_of(self.swapped_kib));
        out.put(&u32_of(self.battery_mv));
        out.put(&i32_of(self.battery_ma));
        out.put(&[
            self.battery_percent.unwrap_or(u8::MAX),
            self.battery.byte(),
            self.outside_power.map_or(u8::MAX, u8::from),
            [
                (self.stopped, STOPPED),
                (self.asleep, ASLEEP),
                (self.firmware_read, FIRMWARE_READ),
            ]
            .iter()
            .filter(|(set, _)| *set)
            .fold(0, |flags, (_, flag)| flags | flag),
        ]);
        for rest in self.rests {
            out.put(&u32_of(rest));
        }
        // In every build, tests' included: a field taken out of both encode
        // and decode would leave a gap no round trip shows.
        assert_eq!(out.at, CRC_AT, "a record's fields don't fill it");
        let crc = crc32(&bytes[..CRC_AT]);
        bytes[CRC_AT..].copy_from_slice(&crc.to_le_bytes());
        bytes
    }

    /// The record its bytes hold; none where they hold none or were cut.
    #[expect(
        clippy::similar_names,
        reason = "each binding is its field, named by its unit: mW beside mC, mV beside mA"
    )]
    pub fn decode(bytes: &[u8; SIZE]) -> Option<Self> {
        if bytes[..4] != MAGIC {
            return None;
        }
        let crc = u32::from_le_bytes(bytes[CRC_AT..].try_into().ok()?);
        if crc != crc32(&bytes[..CRC_AT]) {
            return None;
        }
        let mut read = Reader { bytes, at: 4 };
        let sequence = u32::from_le_bytes(read.take());
        let at_ms = i64::from_le_bytes(read.take());
        let since_boot_ms = u64::from_le_bytes(read.take());
        let package_mw = read.u32();
        let platform_mw = read.u32();
        let package_mc = read.i32();
        let fans = [read.u16(), read.u16(), read.u16(), read.u16()];
        let load = read.u32();
        let tasks = read.u16().zip(read.u16());
        let stalls = Stalls {
            cpu_some: read.u32(),
            memory_some: read.u32(),
            memory_full: read.u32(),
            io_some: read.u32(),
            io_full: read.u32(),
        };
        let available_kib = read.u32();
        let swapped_kib = read.u32();
        let battery_mv = read.u32();
        let battery_ma = read.i32();
        let battery_percent = Some(read.u8()).filter(|percent| *percent != u8::MAX);
        let battery = Battery::of_byte(read.u8());
        let outside_power = match read.u8() {
            0 => Some(false),
            1 => Some(true),
            _ => None,
        };
        let flags = read.u8();
        let mut rests = [None; RESTS];
        for rest in &mut rests {
            *rest = read.u32();
        }
        Some(Self {
            sequence,
            at_ms,
            since_boot_ms,
            package_mw,
            platform_mw,
            package_mc,
            fans,
            load,
            tasks,
            stalls,
            available_kib,
            swapped_kib,
            battery_mv,
            battery_ma,
            battery_percent,
            battery,
            outside_power,
            rests,
            stopped: flags & STOPPED != 0,
            asleep: flags & ASLEEP != 0,
            firmware_read: flags & FIRMWARE_READ != 0,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A record with every field given, none at its sentinel.
    fn full() -> Record {
        Record {
            sequence: 7,
            at_ms: 1_795_241_514_250,
            since_boot_ms: 61_000,
            package_mw: Some(14_250),
            platform_mw: Some(21_900),
            package_mc: Some(71_000),
            fans: [Some(4_200), Some(4_100), None, None],
            load: Some(312),
            tasks: Some((3, 1_204)),
            stalls: Stalls {
                cpu_some: Some(12_000),
                memory_some: Some(0),
                memory_full: Some(0),
                io_some: Some(800),
                io_full: Some(100),
            },
            available_kib: Some(20_123_456),
            swapped_kib: Some(0),
            battery_mv: Some(12_356),
            battery_ma: Some(-1_250),
            battery_percent: Some(87),
            battery: Battery::Discharging,
            outside_power: Some(false),
            rests: [
                Some(1),
                Some(20),
                Some(300),
                Some(4_000),
                Some(11_000_000),
                None,
                None,
                None,
            ],
            stopped: false,
            asleep: false,
            firmware_read: true,
        }
    }

    #[test]
    fn a_record_reads_back_as_it_was_written() {
        let record = full();
        assert_eq!(Record::decode(&record.encode()), Some(record));
        let stopped = Record {
            stopped: true,
            ..Record::default()
        };
        assert_eq!(Record::decode(&stopped.encode()), Some(stopped));
        let asleep = Record {
            asleep: true,
            ..Record::default()
        };
        assert_eq!(Record::decode(&asleep.encode()), Some(asleep));
    }

    #[test]
    fn a_value_not_given_reads_back_as_not_given_never_as_zero() {
        let empty = Record::default();
        let read = Record::decode(&empty.encode()).expect("a record");
        assert_eq!(
            (
                read.package_mw,
                read.package_mc,
                read.fans[0],
                read.outside_power
            ),
            (None, None, None, None)
        );
    }

    #[test]
    fn a_record_cut_or_changed_anywhere_is_no_record() {
        let bytes = full().encode();
        for at in 0..SIZE {
            let mut changed = bytes;
            changed[at] ^= 0x10;
            assert_eq!(
                Record::decode(&changed),
                None,
                "a byte changed at {at} read"
            );
        }
        // Half of it written over a slot never written, as a cut leaves it.
        let mut torn = [0; SIZE];
        torn[..SIZE / 2].copy_from_slice(&bytes[..SIZE / 2]);
        assert_eq!(Record::decode(&torn), None);
        assert_eq!(Record::decode(&[0; SIZE]), None);
    }

    #[test]
    fn the_crc_is_the_one_every_zip_uses() {
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    }
}
