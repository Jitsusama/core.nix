//! How hard the machine's disks are written, and how worn they are. A
//! laptop's disk wrote 4.2 TB in its first 16 hours of one boot on 18
//! November, nearly all of it builds, which at that pace reaches its rated
//! wear in about a year; nothing said so until someone measured it. The
//! system guard reads what each disk wrote since boot from the kernel at
//! every step, and its health log, which only root may ask for, once an
//! hour awake, and says both in its status.

use crate::clock::Tick;
use crate::units::float;
use serde::Serialize;
use std::collections::{BTreeMap, VecDeque};

/// What a disk's own count of written blocks is in, from the kernel: 512
/// bytes whatever the disk's own block.
const SECTOR: u64 = 512;

/// What an NVMe health log counts data units in: a thousand blocks of 512
/// bytes.
const DATA_UNIT: u128 = 512_000;

/// The span the rate is over: the last hour awake.
const RATE_OVER: f64 = 3600.0;

/// The least span a rate is said over, so a guard just started doesn't
/// say the burst it started in.
const RATE_FIRST: f64 = 600.0;

/// How long awake between asks of a disk's health. It moves a percent in
/// weeks, and each ask is a command to the disk.
const HEALTH_EVERY: f64 = 3600.0;

/// The least time into a boot its pace is said from.
const PACE_FIRST: f64 = 3600.0;

/// What a disk wrote since boot, in bytes, from its `stat` in sysfs, whose
/// seventh field counts sectors written.
pub fn written_since_boot(stat: &str) -> Option<u64> {
    let sectors: u64 = stat.split_whitespace().nth(6)?.parse().ok()?;
    sectors.checked_mul(SECTOR)
}

/// How worn a disk says it is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Wear {
    /// The share of its rated endurance used, in percent, which passes
    /// 100 once it's past its rating.
    pub used: u8,
    /// What it wrote in its life, in bytes.
    pub written: u64,
}

/// A disk's wear from its NVMe health log (log page 2): byte 5 is the
/// percentage used and bytes 48 to 63 the data units written, little
/// endian.
pub fn health(log: &[u8]) -> Option<Wear> {
    let used = *log.get(5)?;
    let units = u128::from_le_bytes(log.get(48..64)?.try_into().ok()?);
    let written = u64::try_from(units.saturating_mul(DATA_UNIT)).unwrap_or(u64::MAX);
    Some(Wear { used, written })
}

/// One disk as read now.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reading {
    /// Its name in the kernel, such as `nvme0n1`.
    pub name: String,
    /// Bytes written since boot.
    pub written: u64,
    /// Its wear, when its health was asked for and it answered.
    pub wear: Option<Wear>,
}

/// Where a guard reads its disks: the kernel, or a stand-in.
pub trait Disks {
    /// Every disk, with its wear too when `health` is asked for.
    fn read(&mut self, health: bool) -> Vec<Reading>;
}

/// A disk as the status file says it.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct DiskStatus {
    pub name: String,
    /// Bytes written since boot.
    pub written_this_boot: u64,
    /// Bytes an hour awake over the last hour; none until ten minutes are
    /// watched.
    pub written_per_hour: Option<f64>,
    /// The share of its rated endurance used, in percent, once its health
    /// has been read.
    pub used_percent: Option<u8>,
    /// Bytes written in its life, once its health has been read.
    pub written_in_life: Option<u64>,
    /// Seconds until it reaches its rated endurance at the pace it was
    /// written since boot, asleep and all; none until it's worn a percent
    /// and an hour of the boot has passed. A percent is coarse, so a disk
    /// 2% worn could be 1.5% or 2.5%, and this a quarter either way.
    pub rated_wear_in: Option<f64>,
}

/// What a guard has seen of one disk.
#[derive(Default)]
struct Seen {
    /// Each count at an awake moment, oldest first, over the last hour.
    counts: VecDeque<(f64, u64)>,
    written: u64,
    wear: Option<Wear>,
}

/// A guard's watch on its disks, from step to step.
pub struct Watch {
    disks: Box<dyn Disks>,
    seen: BTreeMap<String, Seen>,
    health_at: Option<f64>,
    /// Seconds since boot at the last look, asleep and all.
    booted_for: f64,
}

impl Watch {
    pub fn new(disks: Box<dyn Disks>) -> Watch {
        Watch {
            disks,
            seen: BTreeMap::new(),
            health_at: None,
            booted_for: 0.0,
        }
    }

    /// Reads every disk, asking its health when an hour awake has passed
    /// since the last ask.
    pub fn observe(&mut self, tick: &Tick) {
        let awake = tick.monotonic;
        let health = self
            .health_at
            .is_none_or(|at| awake - at >= HEALTH_EVERY || awake < at);
        if health {
            self.health_at = Some(awake);
        }
        self.booted_for = tick.monotonic + tick.suspended;
        let readings = self.disks.read(health);
        self.seen
            .retain(|name, _| readings.iter().any(|reading| reading.name == *name));
        for reading in readings {
            let seen = self.seen.entry(reading.name).or_default();
            // A count that went back is another boot's or another disk's.
            if reading.written < seen.written {
                seen.counts.clear();
            }
            seen.written = reading.written;
            seen.counts.push_back((awake, reading.written));
            while seen
                .counts
                .front()
                .is_some_and(|(at, _)| awake - at > RATE_OVER)
            {
                seen.counts.pop_front();
            }
            if let Some(wear) = reading.wear {
                seen.wear = Some(wear);
            }
        }
    }

    pub fn status(&self) -> Vec<DiskStatus> {
        self.seen
            .iter()
            .map(|(name, seen)| DiskStatus {
                name: name.clone(),
                written_this_boot: seen.written,
                written_per_hour: rate(&seen.counts),
                used_percent: seen.wear.map(|wear| wear.used),
                written_in_life: seen.wear.map(|wear| wear.written),
                rated_wear_in: seen
                    .wear
                    .and_then(|wear| rated_wear_in(wear, seen.written, self.booted_for)),
            })
            .collect()
    }
}

/// Bytes an hour between the oldest count kept and the newest.
fn rate(counts: &VecDeque<(f64, u64)>) -> Option<f64> {
    let (first, last) = (counts.front()?, counts.back()?);
    let span = last.0 - first.0;
    (span >= RATE_FIRST).then(|| float(last.1 - first.1) / span * 3600.0)
}

/// Seconds until `wear` reaches 100% at the pace since boot: what each
/// percent took over its life, times the percents left, over what the
/// boot wrote a second.
fn rated_wear_in(wear: Wear, this_boot: u64, booted_for: f64) -> Option<f64> {
    if wear.used == 0 || booted_for < PACE_FIRST || this_boot == 0 {
        return None;
    }
    if wear.used >= 100 {
        return Some(0.0);
    }
    let per_percent = float(wear.written) / f64::from(wear.used);
    let left = per_percent * f64::from(100 - wear.used);
    Some(left / (float(this_boot) / booted_for))
}

/// The kernel's disks: each entry of `/sys/block` with a device behind
/// it, which leaves out loops, mappers and memory, and for an NVMe disk
/// the health log its controller in `/dev` gives root.
#[cfg(target_os = "linux")]
pub struct Sysfs {
    block: std::path::PathBuf,
    dev: std::path::PathBuf,
}

#[cfg(target_os = "linux")]
impl Sysfs {
    pub fn new(block: std::path::PathBuf, dev: std::path::PathBuf) -> Sysfs {
        Sysfs { block, dev }
    }

    /// The machine's own.
    pub fn machine() -> Sysfs {
        Sysfs::new("/sys/block".into(), "/dev".into())
    }

    /// The health log of the controller `entry` names as its device, as
    /// `nvme0` for `nvme0n1`.
    fn wear(&self, entry: &std::path::Path) -> Option<Wear> {
        let device = std::fs::read_link(entry.join("device")).ok()?;
        let controller = device.file_name()?.to_str()?;
        if !controller.starts_with("nvme") {
            return None;
        }
        health(&nvme_health_log(&self.dev.join(controller))?)
    }
}

#[cfg(target_os = "linux")]
impl Disks for Sysfs {
    fn read(&mut self, health: bool) -> Vec<Reading> {
        let Ok(entries) = std::fs::read_dir(&self.block) else {
            return Vec::new();
        };
        let mut readings: Vec<Reading> = entries
            .flatten()
            .filter(|entry| entry.path().join("device").symlink_metadata().is_ok())
            .filter_map(|entry| {
                let path = entry.path();
                let stat = std::fs::read_to_string(path.join("stat")).ok()?;
                Some(Reading {
                    name: entry.file_name().to_str()?.to_owned(),
                    written: written_since_boot(&stat)?,
                    wear: health.then(|| self.wear(&path)).flatten(),
                })
            })
            .collect();
        readings.sort_by(|one, other| one.name.cmp(&other.name));
        readings
    }
}

/// The kernel's admin command passthrough, `struct nvme_passthru_cmd`.
#[cfg(target_os = "linux")]
#[repr(C)]
#[derive(Default)]
struct AdminCommand {
    opcode: u8,
    flags: u8,
    rsvd1: u16,
    nsid: u32,
    cdw2: u32,
    cdw3: u32,
    metadata: u64,
    addr: u64,
    metadata_len: u32,
    data_len: u32,
    cdw10: u32,
    cdw11: u32,
    cdw12: u32,
    cdw13: u32,
    cdw14: u32,
    cdw15: u32,
    timeout_ms: u32,
    result: u32,
}

/// `NVME_IOCTL_ADMIN_CMD`: `_IOWR('N', 0x41, struct nvme_admin_cmd)`.
#[cfg(target_os = "linux")]
const NVME_IOCTL_ADMIN_CMD: libc::c_ulong = 0xC048_4E41;

/// The 512-byte health log of the NVMe controller at `controller`, asked
/// with Get Log Page for every namespace, as `nvme smart-log` asks it.
/// Only root may; anything else, or a file that isn't a controller, has
/// none.
#[cfg(target_os = "linux")]
fn nvme_health_log(controller: &std::path::Path) -> Option<Vec<u8>> {
    use std::os::fd::AsRawFd;
    const GET_LOG_PAGE: u8 = 0x02;
    const HEALTH: u32 = 0x02;
    const LENGTH: u32 = 512;
    let file = std::fs::File::open(controller).ok()?;
    let mut log = vec![0u8; LENGTH as usize];
    let mut command = AdminCommand {
        opcode: GET_LOG_PAGE,
        nsid: u32::MAX,
        addr: log.as_mut_ptr() as u64,
        data_len: LENGTH,
        // The log's id, and how many dwords less one in the upper half.
        cdw10: HEALTH | ((LENGTH / 4 - 1) << 16),
        timeout_ms: 5000,
        ..AdminCommand::default()
    };
    // SAFETY: the command is the kernel's layout and points at `log`,
    // which outlives the call and is as long as it says.
    let answered = unsafe { libc::ioctl(file.as_raw_fd(), NVME_IOCTL_ADMIN_CMD, &raw mut command) };
    (answered == 0).then_some(log)
}
