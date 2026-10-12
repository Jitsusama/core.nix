//! What Linux says about the machine.

use crate::band::BtrfsSpace;
use std::fs;
use std::io;
use std::path::Path;

/// One number from sysfs.
fn number(path: &Path) -> io::Result<u64> {
    let text = fs::read_to_string(path)?;
    text.trim().parse().map_err(|_| {
        let what = format!("{} isn't a number: {text:?}", path.display());
        io::Error::new(io::ErrorKind::InvalidData, what)
    })
}

/// A count newer kernels give and older ones don't, which is then none.
fn number_or_none(path: &Path) -> io::Result<u64> {
    match number(path) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(0),
        other => other,
    }
}

/// What a Btrfs filesystem has left to allocate, from its sysfs directory
/// (`/sys/fs/btrfs/<fsid>`), which any account may read.
pub fn btrfs_allocation(sysfs: &Path) -> io::Result<BtrfsSpace> {
    let allocation = sysfs.join("allocation");
    let mut devices = 0;
    for device in fs::read_dir(sysfs.join("devices"))? {
        // The block device's size, in 512-byte sectors whatever its own.
        devices += number(&device?.path().join("size"))? * 512;
    }
    let mut allocated = 0;
    for kind in ["data", "metadata", "system"] {
        allocated += number(&allocation.join(kind).join("disk_total"))?;
    }
    let metadata = allocation.join("metadata");
    let total = number(&metadata.join("total_bytes"))?;
    let copies = (number(&metadata.join("disk_total"))? / total.max(1)).max(1);
    // bytes_may_use is room promised to writes in flight, which Btrfs
    // overcommits and hands back, so it isn't taken from what's free.
    let mut taken = 0;
    for count in [
        "bytes_used",
        "bytes_reserved",
        "bytes_pinned",
        "bytes_readonly",
    ] {
        taken += number(&metadata.join(count))?;
    }
    taken += number_or_none(&metadata.join("bytes_zone_unusable"))?;
    Ok(BtrfsSpace {
        unallocated: devices.saturating_sub(allocated),
        metadata_chunk: number(&metadata.join("chunk_size"))? * copies,
        metadata_free: total.saturating_sub(taken),
        global_reserve: number(&allocation.join("global_rsv_size"))?,
    })
}
