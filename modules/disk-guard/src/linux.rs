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

/// The stalls PSI reports (`/proc/pressure`), each over its last ten
/// seconds. A resource whose file is missing, or a line an older kernel
/// doesn't write, reads as no stall.
pub fn pressure(proc_pressure: &Path) -> io::Result<crate::moments::Pressure> {
    let stall = |resource: &str| -> io::Result<crate::moments::Stall> {
        let text = match fs::read_to_string(proc_pressure.join(resource)) {
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Default::default()),
            other => other?,
        };
        let mut stall = crate::moments::Stall::default();
        for line in text.lines() {
            let mut words = line.split_whitespace();
            let which = words.next();
            let average = words
                .find_map(|word| word.strip_prefix("avg10="))
                .and_then(|value| value.parse().ok());
            match (which, average) {
                (Some("some"), Some(value)) => stall.some = value,
                (Some("full"), Some(value)) => stall.full = value,
                _ => {}
            }
        }
        Ok(stall)
    };
    Ok(crate::moments::Pressure {
        io: stall("io")?,
        memory: stall("memory")?,
        cpu: stall("cpu")?,
    })
}

/// Where power comes from, by the supplies in `/sys/class/power_supply`.
/// Supplies scoped to a device, such as a mouse's battery, say nothing
/// about the machine. A battery's status can read Discharging while mains
/// is online, so only the supplies' own online state counts. A machine with
/// no supplies at all is a desktop, on mains.
pub fn power(supplies: &Path) -> io::Result<crate::moments::Power> {
    let read = |path: PathBuf| fs::read_to_string(path).map(|text| text.trim().to_owned());
    let (mut sources, mut online, mut charges) = (0, false, Vec::new());
    for supply in fs::read_dir(supplies)? {
        let supply = supply?.path();
        if read(supply.join("scope")).is_ok_and(|scope| scope == "Device") {
            continue;
        }
        match read(supply.join("type"))?.as_str() {
            "Battery" => {
                if let Ok(charge) = read(supply.join("capacity"))?.parse::<u8>() {
                    charges.push(charge);
                }
            }
            _ => {
                sources += 1;
                online |= read(supply.join("online")).is_ok_and(|state| state == "1");
            }
        }
    }
    let battery = (!charges.is_empty())
        .then(|| (charges.iter().map(|&c| c as u32).sum::<u32>() / charges.len() as u32) as u8);
    Ok(crate::moments::Power {
        mains: online || (sources == 0 && battery.is_none()),
        battery,
    })
}

/// `PF_KTHREAD` in a process's flags: a kernel thread, which holds no files.
const PF_KTHREAD: u64 = 0x0020_0000;

/// The environment variables that point a build somewhere, which are all
/// the guard keeps of a process's environment.
const BUILD_VARIABLES: [&str; 3] = ["CARGO_TARGET_DIR", "CARGO_BUILD_BUILD_DIR", "TMPDIR"];

/// Every process `/proc` shows, but kernel threads and zombies. A process
/// that exits mid-scan is left out. One whose cwd, files or maps are hidden,
/// as another account's are, reads as unreadable rather than as idle.
pub fn processes(proc: &Path) -> io::Result<Vec<crate::activity::Process>> {
    let mut all = Vec::new();
    for entry in fs::read_dir(proc)? {
        let entry = entry?;
        let Some(pid) = entry
            .file_name()
            .to_str()
            .and_then(|name| name.parse().ok())
        else {
            continue;
        };
        if let Some(process) = process(&entry.path(), pid) {
            all.push(process);
        }
    }
    Ok(all)
}

/// One process, or none when it's gone, a kernel thread or a zombie.
fn process(dir: &Path, pid: u32) -> Option<crate::activity::Process> {
    use std::os::unix::ffi::OsStrExt;
    let stat = fs::read_to_string(dir.join("stat")).ok()?;
    // The name sits in parentheses and may hold any of them itself.
    let (open, close) = (stat.find('(')?, stat.rfind(')')?);
    let name = stat.get(open + 1..close)?.to_owned();
    // Fields from the state (the third) on; field n is at n - 3.
    let fields: Vec<&str> = stat.get(close + 1..)?.split_whitespace().collect();
    let flags: u64 = fields.get(6)?.parse().ok()?;
    if fields.first() == Some(&"Z") || flags & PF_KTHREAD != 0 {
        return None;
    }
    let start_ticks = fields.get(19)?.parse().ok()?;
    let status = fs::read_to_string(dir.join("status")).ok()?;
    let uid = status
        .lines()
        .find_map(|line| line.strip_prefix("Uid:"))
        .and_then(|ids| ids.split_whitespace().next())
        .and_then(|id| id.parse().ok())?;

    let words = |bytes: Vec<u8>| -> Vec<OsString> {
        bytes
            .split(|&byte| byte == 0)
            .filter(|word| !word.is_empty())
            .map(|word| std::ffi::OsStr::from_bytes(word).to_owned())
            .collect()
    };
    let cmdline = fs::read(dir.join("cmdline")).map(words).unwrap_or_default();
    let env = fs::read(dir.join("environ"))
        .map(words)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|pair| {
            let bytes = pair.as_bytes();
            let equals = bytes.iter().position(|&byte| byte == b'=')?;
            let key = std::ffi::OsStr::from_bytes(&bytes[..equals]);
            BUILD_VARIABLES.iter().any(|name| key == *name).then(|| {
                (
                    key.to_owned(),
                    std::ffi::OsStr::from_bytes(&bytes[equals + 1..]).to_owned(),
                )
            })
        })
        .collect();

    let cwd = fs::read_link(dir.join("cwd"));
    let exe = fs::read_link(dir.join("exe")).ok();
    let files = fs::read_dir(dir.join("fd")).map(|fds| {
        let mut files: Vec<PathBuf> = fds
            .filter_map(|fd| fs::read_link(fd.ok()?.path()).ok())
            .filter(|target| target.is_absolute())
            .collect();
        files.sort();
        files.dedup();
        files
    });
    let maps = fs::read_to_string(dir.join("maps")).map(|text| {
        let mut maps: Vec<PathBuf> = Vec::new();
        for line in text.lines() {
            if let Some(path) = mapped_path(line)
                && !maps.contains(&path)
            {
                maps.push(path);
            }
        }
        maps
    });
    let readable = cwd.is_ok() && files.is_ok() && maps.is_ok();
    let write_bytes = fs::read_to_string(dir.join("io")).ok().and_then(|io| {
        io.lines()
            .find_map(|line| line.strip_prefix("write_bytes:")?.trim().parse().ok())
    });
    let cgroup = fs::read_to_string(dir.join("cgroup"))
        .ok()
        .and_then(|text| {
            text.lines()
                .find_map(|line| line.strip_prefix("0::").map(str::to_owned))
        });
    Some(crate::activity::Process {
        pid,
        uid,
        start_ticks,
        name,
        exe,
        cwd: cwd.ok(),
        cmdline,
        env,
        files: files.unwrap_or_default(),
        maps: maps.unwrap_or_default(),
        readable,
        write_bytes,
        cgroup,
    })
}

/// The file a line of `/proc/<pid>/maps` maps, if it maps one. The path is
/// everything after the fifth field, spaces and all, less the kernel's
/// " (deleted)" for a file since unlinked.
fn mapped_path(line: &str) -> Option<PathBuf> {
    let mut rest = line;
    for _ in 0..5 {
        rest = rest.trim_start();
        rest = &rest[rest.find(' ')?..];
    }
    let path = rest.trim_start();
    let path = path.strip_suffix(" (deleted)").unwrap_or(path);
    path.starts_with('/').then(|| PathBuf::from(path))
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
use std::ffi::OsString;
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
