//! How short of space a filesystem is, and what each level allows.

use serde::{Deserialize, Serialize};

/// A level of free space, least severe first, so `Ord` compares severity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Band {
    Healthy,
    Pressure,
    Urgent,
    Critical,
}

/// A filesystem's space as the guard reads it: `free` is what an
/// unprivileged writer may still use (`f_bavail`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Space {
    pub total: u64,
    pub free: u64,
    pub btrfs: Option<BtrfsSpace>,
}

/// What Btrfs has left to allocate, which `f_bavail` doesn't show.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BtrfsSpace {
    /// Device space no chunk holds yet.
    pub unallocated: u64,
    /// The device space one more metadata chunk takes, with every copy its
    /// profile keeps (two for dup).
    pub metadata_chunk: u64,
    /// Metadata chunks' room not yet used, reserved, pinned or read-only.
    pub metadata_free: u64,
    pub global_reserve: u64,
}

/// The configured bands in bytes on one filesystem.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Thresholds {
    pub pressure: u64,
    pub urgent: u64,
    pub critical: u64,
    pub target: u64,
    pub margin: u64,
}

impl Thresholds {
    /// A band is left only once free space passes its threshold by this
    /// share of the filesystem, so a pass that stops at the line doesn't
    /// flap in and out of it with every write.
    const MARGIN_PERCENT: f64 = 1.0;

    /// The bands in bytes on a filesystem of `total` bytes. A config can
    /// mix bytes and shares, whose order shows only here, so bands out of
    /// order fold into the one before, and the target lies at least where
    /// pressure is left: a pass that reached a target under it would leave
    /// the disk under pressure, and run again every step.
    pub fn new(bands: &crate::config::Bands, total: u64) -> Thresholds {
        let margin = crate::units::Size::Percent(Self::MARGIN_PERCENT).of(total);
        let pressure = bands.pressure.of(total);
        let urgent = bands.urgent.of(total).min(pressure);
        Thresholds {
            pressure,
            urgent,
            critical: bands.critical.of(total).min(urgent),
            target: bands.target.of(total).max(pressure.saturating_add(margin)),
            margin,
        }
    }

    /// Where each band starts, deepest first.
    fn starts(&self) -> [(Band, u64); 3] {
        [
            (Band::Critical, self.critical),
            (Band::Urgent, self.urgent),
            (Band::Pressure, self.pressure),
        ]
    }

    /// The band `space` is in, given the band it was in. A deeper band is
    /// entered at once; a band is left only past its threshold and the
    /// margin.
    pub fn band(&self, space: &Space, previous: Band) -> Band {
        if space
            .btrfs
            .is_some_and(|btrfs| btrfs.short_of_metadata_room())
        {
            return Band::Critical;
        }
        let free = space.free;
        let raw = self
            .starts()
            .iter()
            .find(|(_, start)| free < *start)
            .map_or(Band::Healthy, |(band, _)| *band);
        if raw >= previous {
            return raw;
        }
        self.starts()
            .iter()
            .find(|(band, start)| *band > raw && *band <= previous && free < start + self.margin)
            .map_or(raw, |(band, _)| *band)
    }
}

impl BtrfsSpace {
    /// Btrfs reserves metadata room of its own, and once it can allocate
    /// no new metadata chunk, metadata can run out and fail every write
    /// with ENOSPC while data space still shows free.
    pub fn short_of_metadata_room(&self) -> bool {
        self.unallocated < self.metadata_chunk && self.metadata_free < 2 * self.global_reserve
    }
}
