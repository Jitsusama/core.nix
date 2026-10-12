//! What Linux says about the machine.

use crate::band::{BtrfsSpace, Space};
use std::os::fd::AsRawFd;
use std::path::PathBuf;

const BTRFS_SUPER_MAGIC: u64 = 0x9123_683e;

/// `BTRFS_IOC_FS_INFO`, `_IOR(0x94, 31, struct btrfs_ioctl_fs_info_args)`:
/// the filesystem's id, which names its sysfs directory. Any account may ask.
const BTRFS_IOC_FS_INFO: libc::c_ulong = 0x8400_941f;

/// Where sysfs keeps each Btrfs filesystem's directory.
const BTRFS_SYSFS: &str = "/sys/fs/btrfs";

/// The space of the filesystem `path` is on: `free` is what an
/// unprivileged writer may still use. On Btrfs its allocation comes too,
/// unless there's no sysfs to read it from, as in a build sandbox.
pub fn space(path: &Path) -> io::Result<Space> {
    let vfs = rustix::fs::statvfs(path)?;
    let btrfs = if rustix::fs::statfs(path)?.f_type as u64 == BTRFS_SUPER_MAGIC
        && Path::new(BTRFS_SYSFS).exists()
    {
        Some(btrfs_allocation(
            &Path::new(BTRFS_SYSFS).join(btrfs_fsid(path)?),
        )?)
    } else {
        None
    };
    Ok(Space {
        total: vfs.f_blocks * vfs.f_frsize,
        free: vfs.f_bavail * vfs.f_frsize,
        btrfs,
    })
}

/// `BTRFS_IOC_START_SYNC`, `_IOR(0x94, 24, __u64)`: starts committing the
/// running transaction and says which one it is.
const BTRFS_IOC_START_SYNC: libc::c_ulong = 0x8008_9418;

/// `BTRFS_IOC_WAIT_SYNC`, `_IOW(0x94, 22, __u64)`: waits for that commit.
const BTRFS_IOC_WAIT_SYNC: libc::c_ulong = 0x4008_9416;

/// Waits until what was deleted on `path`'s filesystem shows as free.
/// Btrfs frees extents only when its transaction commits, about 30 s
/// later, so it's asked to commit: 6 to 10 ms on optimus, and it writes no
/// one else's dirty data. Anywhere else, `syncfs` does it.
pub fn settle(path: &Path) -> io::Result<()> {
    let flags = rustix::fs::OFlags::RDONLY | rustix::fs::OFlags::CLOEXEC;
    let fd = rustix::fs::open(path, flags, rustix::fs::Mode::empty())?;
    if rustix::fs::fstatfs(&fd)?.f_type as u64 != BTRFS_SUPER_MAGIC {
        return Ok(rustix::fs::syncfs(&fd)?);
    }
    let mut transaction: u64 = 0;
    // SAFETY: the kernel writes one u64 into transaction, which outlives
    // the call; WAIT_SYNC only reads it.
    unsafe {
        if libc::ioctl(fd.as_raw_fd(), BTRFS_IOC_START_SYNC as _, &mut transaction) != 0
            || libc::ioctl(fd.as_raw_fd(), BTRFS_IOC_WAIT_SYNC as _, &transaction) != 0
        {
            return Err(io::Error::last_os_error());
        }
    }
    Ok(())
}

/// One reading of a clock, in seconds.
fn clock(id: libc::clockid_t) -> io::Result<f64> {
    let mut time = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    // SAFETY: time is a valid timespec the kernel fills in.
    if unsafe { libc::clock_gettime(id, &mut time) } != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(time.tv_sec as f64 + time.tv_nsec as f64 / 1e9)
}

/// The clocks awake time is counted from. Monotonic time stops while the
/// machine sleeps and boot time doesn't, so suspended time is the gap
/// between them; the boot id tells one boot from the next.
pub fn tick() -> io::Result<crate::clock::Tick> {
    let monotonic = clock(libc::CLOCK_MONOTONIC)?;
    let boottime = clock(libc::CLOCK_BOOTTIME)?;
    let wall = clock(libc::CLOCK_REALTIME)?;
    let boot = fs::read_to_string("/proc/sys/kernel/random/boot_id")?;
    Ok(crate::clock::Tick {
        wall,
        monotonic,
        suspended: (boottime - monotonic).max(0.0),
        boot: boot.trim().to_owned(),
    })
}

/// A Btrfs filesystem's id, as its sysfs directory is named.
fn btrfs_fsid(path: &Path) -> io::Result<PathBuf> {
    let flags = rustix::fs::OFlags::RDONLY | rustix::fs::OFlags::CLOEXEC;
    let fd = rustix::fs::open(path, flags, rustix::fs::Mode::empty())?;
    // struct btrfs_ioctl_fs_info_args is 1024 bytes: max_id and
    // num_devices, then the 16-byte fsid.
    let mut args = [0u8; 1024];
    // SAFETY: args is as large as the kernel writes, and outlives the call.
    if unsafe { libc::ioctl(fd.as_raw_fd(), BTRFS_IOC_FS_INFO as _, args.as_mut_ptr()) } != 0 {
        return Err(io::Error::last_os_error());
    }
    let hex: String = args[16..32]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    let fsid = format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    );
    Ok(PathBuf::from(fsid))
}
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
