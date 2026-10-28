//! What Linux says about the machine.

use crate::band::{BtrfsSpace, Space};
use crate::pace::Settling;
use std::os::fd::AsRawFd;
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::time::Duration;

const BTRFS_SUPER_MAGIC: u64 = 0x9123_683e;

/// `BTRFS_IOC_FS_INFO`, `_IOR(0x94, 31, struct btrfs_ioctl_fs_info_args)`:
/// the filesystem's id, which names its sysfs directory. Any account may ask.
const BTRFS_IOC_FS_INFO: libc::c_ulong = 0x8400_941f;

/// Where sysfs keeps each Btrfs filesystem's directory.
const BTRFS_SYSFS: &str = "/sys/fs/btrfs";

/// When the machine booted, in wall seconds, and how many clock ticks a
/// second process start times count in.
pub fn boot(proc_stat: &std::path::Path) -> io::Result<crate::snapshots::Boot> {
    let text = fs::read_to_string(proc_stat)?;
    let wall: f64 = text
        .lines()
        .find_map(|line| line.strip_prefix("btime "))
        .and_then(|value| value.trim().parse().ok())
        .ok_or_else(|| io::Error::other("no boot time in the kernel's statistics"))?;
    // SAFETY: sysconf only reads a constant.
    let ticks = unsafe { libc::sysconf(libc::_SC_CLK_TCK) };
    let Some(ticks) = u64::try_from(ticks).ok().filter(|&ticks| ticks > 0) else {
        return Err(io::Error::last_os_error());
    };
    Ok(crate::snapshots::Boot {
        wall,
        ticks: crate::units::float(ticks),
    })
}

/// The space of the filesystem `path` is on: `free` is what an
/// unprivileged writer may still use. On Btrfs its allocation comes too,
/// unless there's no sysfs to read it from, as in a build sandbox.
pub fn space(path: &Path) -> io::Result<Space> {
    let vfs = rustix::fs::statvfs(path)?;
    let btrfs = if is_btrfs(&rustix::fs::statfs(path)?) && Path::new(BTRFS_SYSFS).exists() {
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

/// Hands the allocator's free memory back to the system. A pass builds and
/// drops tens of thousands of small things, the names of a store's garbage
/// among them, and glibc keeps that memory for the daemon's whole life
/// otherwise: 22 MiB of batching 80,000 dead paths, held between passes
/// hours apart.
pub fn give_back() {
    #[cfg(target_env = "gnu")]
    // SAFETY: malloc_trim only returns free pages; nothing in use moves.
    unsafe {
        libc::malloc_trim(0);
    }
}

/// Waits until what was deleted on `path`'s filesystem shows as free.
/// Btrfs frees extents only when its transaction commits, within its
/// commit interval of 30 s. A patient pass waits for that commit and asks
/// nothing sooner, since a commit asked for is a commit of everyone's
/// writes that every save on the disk then waits behind. A hurried one
/// asks for it at once (`commit_now`). Anywhere else, `syncfs` does it.
pub fn settle(path: &Path, settling: Settling) -> io::Result<()> {
    let flags = rustix::fs::OFlags::RDONLY | rustix::fs::OFlags::CLOEXEC;
    let fd = rustix::fs::open(path, flags, rustix::fs::Mode::empty())?;
    if !is_btrfs(&rustix::fs::fstatfs(&fd)?) {
        return sync_filesystem(&fd);
    }
    match settling {
        Settling::Patient => wait_for_commit(path, &fd),
        Settling::Hurried => commit_now(&fd),
    }
}

/// How often a patient settle reads the commit count, and looks for a stop.
const COMMIT_LOOK: Duration = Duration::from_millis(200);

/// The longest a patient settle waits, three of Btrfs's 30 s commit
/// intervals: with nothing left to commit, the count never moves, and
/// what was deleted shows at the next commit anyway.
const COMMIT_LONGEST: Duration = Duration::from_secs(90);

/// Waits for the transaction running now to commit in its own time, by
/// reading the filesystem's commit count rather than waiting in the
/// kernel. A thread waiting in `BTRFS_IOC_WAIT_SYNC` can't be ended, and a
/// process can't exit while one of its threads is in it: a stop at a
/// switch was taken at once and still held the unit 23 s, until Btrfs
/// committed under load. A kernel without the count (before 6.0) waits
/// in the kernel as before.
fn wait_for_commit(path: &Path, fd: &rustix::fd::OwnedFd) -> io::Result<()> {
    let stats = Path::new(BTRFS_SYSFS)
        .join(btrfs_fsid(path)?)
        .join("commit_stats");
    if !stats.exists() {
        return wait_in_the_kernel(fd);
    }
    committed(
        || commits(&stats),
        crate::commands::stop_waiting,
        COMMIT_LOOK,
        COMMIT_LONGEST,
    )
}

/// What a Btrfs filesystem's `commit_stats` says of its commits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Commits {
    /// How many have finished since it was mounted.
    pub done: u64,
    /// Whether one is under way.
    pub underway: bool,
}

/// Reads a Btrfs filesystem's `commit_stats`. Btrfs counts a commit once
/// its superblocks are written, and shows a commit under way as the time it
/// has taken so far, and nothing between commits.
///
/// # Errors
///
/// When it can't be read, or doesn't say how many commits finished.
pub fn commits(stats: &Path) -> io::Result<Commits> {
    let text = fs::read_to_string(stats)?;
    let field = |name: &str| {
        text.lines()
            .find_map(|line| line.strip_prefix(name)?.trim().parse::<u64>().ok())
    };
    let done = field("commits ")
        .ok_or_else(|| io::Error::other(format!("{} doesn't count commits", stats.display())))?;
    Ok(Commits {
        done,
        underway: field("cur_commit_ms ").is_some_and(|pending| pending > 0),
    })
}

/// Waits, reading `read` every `look`, until the transaction running when
/// it's called has committed: the next commit to finish, or the one after
/// it when one was under way, since that one may have closed before what
/// was deleted. A stop, which `stopping` says of, ends it at the next look;
/// past `longest` it ends as settled.
///
/// # Errors
///
/// `Interrupted` when a stop came first, and whatever `read` fails with.
pub fn committed(
    mut read: impl FnMut() -> io::Result<Commits>,
    stopping: impl Fn() -> bool,
    look: Duration,
    longest: Duration,
) -> io::Result<()> {
    let start = read()?;
    let wanted = start.done + if start.underway { 2 } else { 1 };
    let began = std::time::Instant::now();
    loop {
        if stopping() {
            return Err(io::Error::new(io::ErrorKind::Interrupted, "stopping"));
        }
        if began.elapsed() >= longest {
            return Ok(());
        }
        std::thread::sleep(look);
        if read()?.done >= wanted {
            return Ok(());
        }
    }
}

/// Waits in the kernel for the transaction running now to commit. Only
/// `BTRFS_IOC_START_SYNC` makes Btrfs commit early; `BTRFS_IOC_WAIT_SYNC`
/// on a transaction's id only waits for it, and on one already committed
/// returns at once. The filesystem's generation names either the running
/// transaction or the last committed, so the one after it is waited for
/// first, and when Btrfs knows no such transaction, the generation itself.
fn wait_in_the_kernel(fd: &rustix::fd::OwnedFd) -> io::Result<()> {
    let generation = btrfs_generation(fd)?;
    for transaction in [generation + 1, generation] {
        // SAFETY: WAIT_SYNC only reads the u64, which outlives the call.
        let waited =
            unsafe { libc::ioctl(fd.as_raw_fd(), BTRFS_IOC_WAIT_SYNC as _, &transaction) } == 0;
        if waited {
            return Ok(());
        }
        let error = io::Error::last_os_error();
        if error.raw_os_error() != Some(libc::EINVAL) {
            return Err(error);
        }
    }
    // Nothing by either id is waiting, so nothing deleted is.
    Ok(())
}

/// Asks Btrfs to commit now and waits for it. That took 6 to 10 ms on a
/// laptop's NVMe disk under a 3 GB write, against 33 ms for `syncfs`.
///
/// Btrfs's commit is begun by its transaction thread, and a request that
/// reaches the thread while it's busy with another commit can wait out
/// the whole commit interval: two settling at once waited 10 to 30 s in
/// half of ten runs, in `btrfs_commit_transaction_async`. So a commit not
/// back within a second gives way to `syncfs`, which commits in the
/// caller's own time and so frees the one waiting too.
fn commit_now(fd: &rustix::fd::OwnedFd) -> io::Result<()> {
    let committing = rustix::io::fcntl_dupfd_cloexec(fd, 0)?;
    patiently(
        &COMMITTING,
        COMMIT_PATIENCE,
        move || {
            let mut transaction: u64 = 0;
            // SAFETY: the kernel writes one u64 into transaction, which
            // outlives the call; WAIT_SYNC only reads it.
            let done = unsafe {
                libc::ioctl(
                    committing.as_raw_fd(),
                    BTRFS_IOC_START_SYNC as _,
                    &mut transaction,
                ) == 0
                    && libc::ioctl(
                        committing.as_raw_fd(),
                        BTRFS_IOC_WAIT_SYNC as _,
                        &transaction,
                    ) == 0
            };
            if done {
                Ok(())
            } else {
                Err(io::Error::last_os_error())
            }
        },
        || sync_filesystem(fd),
    )
}

/// `syncfs`, or nothing where it isn't offered: Nix's build sandbox answers
/// it with ENOSYS, and a scenario settling there failed its pass whenever a
/// Btrfs commit took past its second. Unsettled, the frees show at the
/// filesystem's next commit instead.
fn sync_filesystem(fd: &rustix::fd::OwnedFd) -> io::Result<()> {
    match rustix::fs::syncfs(fd) {
        Err(rustix::io::Errno::NOSYS) => Ok(()),
        other => Ok(other?),
    }
}

/// Whether a Btrfs commit this process asked for hasn't come back.
static COMMITTING: std::sync::LazyLock<Arc<AtomicBool>> =
    std::sync::LazyLock::new(|| Arc::new(AtomicBool::new(false)));

/// How long a Btrfs commit is waited for, against the 6 to 45 ms one takes.
const COMMIT_PATIENCE: Duration = Duration::from_secs(1);

/// Runs `commit` on a thread of its own and waits `patience` for it,
/// running `fallback` instead once that's past, or at once while an
/// earlier commit is still out, which `stuck` says. A commit given up on
/// finishes in its own time and clears `stuck`.
pub fn patiently(
    stuck: &Arc<AtomicBool>,
    patience: Duration,
    commit: impl FnOnce() -> io::Result<()> + Send + 'static,
    fallback: impl FnOnce() -> io::Result<()>,
) -> io::Result<()> {
    if stuck
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return fallback();
    }
    let (sender, answer) = mpsc::channel();
    let out = Arc::clone(stuck);
    let spawned = std::thread::Builder::new()
        .name("settle".to_owned())
        .spawn(move || {
            let done = commit();
            out.store(false, Ordering::SeqCst);
            // Nobody hears a commit that came back too late.
            let _ = sender.send(done);
        });
    if spawned.is_err() {
        stuck.store(false, Ordering::SeqCst);
        return fallback();
    }
    match answer.recv_timeout(patience) {
        Ok(done) => done,
        Err(_) => fallback(),
    }
}

/// One reading of a clock, in seconds.
fn clock(id: libc::clockid_t) -> io::Result<f64> {
    let mut time = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    // SAFETY: time is a valid timespec the kernel fills in.
    if unsafe { libc::clock_gettime(id, &raw mut time) } != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(crate::units::wall(time.tv_sec, time.tv_nsec))
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
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Ok(crate::moments::Stall::default());
            }
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
/// about the machine. A battery's status can read Discharging while the
/// charger is online, so only the supplies' own online state counts. A
/// machine with no supplies at all is a desktop, taken as on the charger.
pub fn power(supplies: &Path) -> io::Result<crate::moments::Power> {
    let read = |path: PathBuf| fs::read_to_string(path).map(|text| text.trim().to_owned());
    let (mut sources, mut online, mut charges) = (0, false, Vec::new());
    for supply in fs::read_dir(supplies)? {
        let supply = supply?.path();
        if read(supply.join("scope")).is_ok_and(|scope| scope == "Device") {
            continue;
        }
        if read(supply.join("type"))? == "Battery" {
            if let Ok(charge) = read(supply.join("capacity"))?.parse::<u8>() {
                charges.push(charge);
            }
        } else {
            sources += 1;
            online |= read(supply.join("online")).is_ok_and(|state| state == "1");
        }
    }
    // The mean of charges each under 256 is under 256 too, so it fits.
    let total: usize = charges.iter().map(|&charge| usize::from(charge)).sum();
    let battery = total
        .checked_div(charges.len())
        .and_then(|mean| u8::try_from(mean).ok());
    Ok(crate::moments::Power {
        on_charger: online || (sources == 0 && battery.is_none()),
        battery,
    })
}

/// The processes waiting on locks the guard (`me`) holds on these inodes,
/// by `/proc/locks`, where an OFD lock's pid reads -1. Its lines name a
/// device by numbers stat doesn't give on Btrfs (00:23 where stat says 0:52),
/// so the device is taken from the guard's own lock's line.
pub fn lock_waiters(proc_locks: &Path, me: u32, inodes: &[u64]) -> io::Result<Vec<i32>> {
    // The kernel prints a held lock and its waiters as one record, but
    // lists the records a page at a time, letting go of them in between,
    // so locks coming and going elsewhere can print a record twice or skip
    // one. A listing missing one of the guard's own locks is read again,
    // and one that keeps missing it can't say who waits.
    for _ in 0..LISTING_TRIES {
        if let Some(waiters) = waiters_in(&fs::read_to_string(proc_locks)?, me, inodes) {
            return Ok(waiters);
        }
    }
    Err(io::Error::other(
        "the lock listing keeps missing a lock the guard holds",
    ))
}

const LISTING_TRIES: usize = 3;

fn waiters_in(text: &str, me: u32, inodes: &[u64]) -> Option<Vec<i32>> {
    // Each line: id, an arrow for a waiter, kind, mode, access, pid, then
    // major:minor:inode.
    let parse = |line: &str| -> Option<(bool, String, i32, String)> {
        let mut words = line.split_whitespace().skip(1).peekable();
        let waiting = words.next_if_eq(&"->").is_some();
        let kind = words.next()?.to_owned();
        let pid = words.nth(2)?.parse().ok()?;
        Some((waiting, kind, pid, words.next()?.to_owned()))
    };
    let lines: Vec<_> = text.lines().filter_map(parse).collect();
    let held: Vec<&str> = lines
        .iter()
        .filter(|(waiting, kind, pid, file)| {
            let mine = *pid == me.cast_signed() || (kind == "OFDLCK" && *pid == -1);
            let inode = file.rsplit(':').next().and_then(|inode| inode.parse().ok());
            !waiting && mine && inode.is_some_and(|inode: u64| inodes.contains(&inode))
        })
        .map(|(_, _, _, file)| file.as_str())
        .collect();
    let shown = |inode: &u64| {
        held.iter()
            .any(|file| file.rsplit(':').next() == Some(inode.to_string().as_str()))
    };
    if !inodes.iter().all(shown) {
        return None;
    }
    let mut waiters = Vec::new();
    for (waiting, _, pid, file) in &lines {
        if *waiting && held.contains(&file.as_str()) && !waiters.contains(pid) {
            waiters.push(*pid);
        }
    }
    Some(waiters)
}

/// `PF_KTHREAD` in a process's flags: a kernel thread, which holds no files.
const PF_KTHREAD: u64 = 0x0020_0000;

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

/// When process `pid` started, in clock ticks since boot, or none when it's
/// gone or a zombie, which holds no files. Its stat is anybody's to read.
pub fn started(proc: &Path, pid: u32) -> Option<u64> {
    let stat = fs::read_to_string(proc.join(pid.to_string()).join("stat")).ok()?;
    let (_, fields) = stat_fields(&stat)?;
    if fields.first() == Some(&"Z") {
        return None;
    }
    fields.get(19)?.parse().ok()
}

/// A stat's name and its fields from the state (the third) on, so field n
/// is at n - 3. The name sits in parentheses and may hold any of them.
fn stat_fields(stat: &str) -> Option<(&str, Vec<&str>)> {
    let (open, close) = (stat.find('(')?, stat.rfind(')')?);
    Some((
        stat.get(open + 1..close)?,
        stat.get(close + 1..)?.split_whitespace().collect(),
    ))
}

/// One process, or none when it's gone, a kernel thread or a zombie.
fn process(dir: &Path, pid: u32) -> Option<crate::activity::Process> {
    use std::os::unix::ffi::OsStrExt;
    let stat = fs::read_to_string(dir.join("stat")).ok()?;
    let (name, fields) = stat_fields(&stat)?;
    let name = name.to_owned();
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
            crate::activity::kept(key.as_bytes()).then(|| {
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

/// Which filesystem `path` is on, as its free space is shared: on Btrfs
/// the pool, whose subvolumes each have a device number of their own,
/// and elsewhere the device. A pool's number has its top bit set, which
/// no device number has, so the two never meet.
pub fn pool(path: &Path) -> io::Result<u64> {
    let device = fs::metadata(path)?.dev();
    if !is_btrfs(&rustix::fs::statfs(path)?) {
        return Ok(device);
    }
    let id = btrfs_id(path)?;
    let halves = [&id[..8], &id[8..]].map(|half| u64::from_le_bytes(half.try_into().unwrap()));
    Ok((halves[0] ^ halves[1]) | 1 << 63)
}

/// Whether `path`, a directory beneath `parent` with a device number of
/// its own, is a Btrfs subvolume of the same pool, reached through the
/// same mount and writable, so a walk may go on into it as into any
/// directory. A mount is another filesystem, or this one seen again, and
/// a read-only snapshot can't be emptied; neither is entered. A kernel
/// that doesn't say which mount a path is on enters nothing.
pub fn subvolume_within(parent: &Path, path: &Path) -> bool {
    let btrfs = |dir: &Path| rustix::fs::statfs(dir).is_ok_and(|stats| is_btrfs(&stats));
    let mount = |dir: &Path| {
        let flags = rustix::fs::AtFlags::SYMLINK_NOFOLLOW;
        let wanted = rustix::fs::StatxFlags::MNT_ID;
        rustix::fs::statx(rustix::fs::CWD, dir, flags, wanted)
            .ok()
            .filter(|stat| stat.stx_mask & wanted.bits() != 0)
            .map(|stat| stat.stx_mnt_id)
    };
    btrfs(parent)
        && btrfs(path)
        && btrfs_id(parent).is_ok_and(|id| btrfs_id(path).is_ok_and(|other| other == id))
        && mount(parent).is_some_and(|id| mount(path) == Some(id))
        && rustix::fs::access(path, rustix::fs::Access::WRITE_OK).is_ok()
}

/// Whether `child`, a directory opened beneath `parent` with a device
/// number of its own, is a writable Btrfs subvolume reached through the
/// same mount: what `subvolume_within` asks of two paths, asked of open
/// directories, so nothing can be swapped in between. One mount is one
/// filesystem, so the pool needn't be asked. A kernel that doesn't say
/// which mount a directory is on lets nothing through.
pub fn subvolume_beneath(parent: &std::os::fd::OwnedFd, child: &std::os::fd::OwnedFd) -> bool {
    let btrfs = |fd| rustix::fs::fstatfs(fd).is_ok_and(|stats| is_btrfs(&stats));
    let mount = |fd| {
        let wanted = rustix::fs::StatxFlags::MNT_ID;
        rustix::fs::statx(fd, "", rustix::fs::AtFlags::EMPTY_PATH, wanted)
            .ok()
            .filter(|stat| stat.stx_mask & wanted.bits() != 0)
            .map(|stat| stat.stx_mnt_id)
    };
    let writable = rustix::fs::accessat(
        child,
        ".",
        rustix::fs::Access::WRITE_OK,
        rustix::fs::AtFlags::empty(),
    );
    btrfs(parent)
        && btrfs(child)
        && mount(parent).is_some_and(|id| mount(child) == Some(id))
        && writable.is_ok()
}

/// Whether a filesystem's statistics say it's Btrfs. The kernel's type is
/// signed, so its bits are read as the unsigned magic number they are.
fn is_btrfs(stats: &rustix::fs::StatFs) -> bool {
    stats.f_type.cast_unsigned() == BTRFS_SUPER_MAGIC
}

/// A Btrfs filesystem's id, as its sysfs directory is named.
fn btrfs_fsid(path: &Path) -> io::Result<PathBuf> {
    let hex = btrfs_id(path)?.iter().fold(String::new(), |mut hex, byte| {
        // Writing to a String can't fail.
        _ = write!(hex, "{byte:02x}");
        hex
    });
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

/// A Btrfs filesystem's 16-byte id, the same from every subvolume.
fn btrfs_id(path: &Path) -> io::Result<[u8; 16]> {
    let flags = rustix::fs::OFlags::RDONLY | rustix::fs::OFlags::CLOEXEC;
    let fd = rustix::fs::open(path, flags, rustix::fs::Mode::empty())?;
    // struct btrfs_ioctl_fs_info_args is 1024 bytes: max_id and
    // num_devices, then the 16-byte fsid.
    let args = btrfs_fs_info(&fd, 0)?;
    Ok(args[16..32].try_into().unwrap())
}

/// `BTRFS_FS_INFO_FLAG_GENERATION`: asks `BTRFS_IOC_FS_INFO` for the
/// filesystem's generation too.
const BTRFS_FS_INFO_FLAG_GENERATION: u64 = 1 << 1;

/// A Btrfs filesystem's generation, the id of its newest transaction.
fn btrfs_generation(fd: &rustix::fd::OwnedFd) -> io::Result<u64> {
    // The flags asked for and answered are the u64 at 48, after the fsid
    // and the sizes, and the generation the u64 after them.
    let args = btrfs_fs_info(fd, BTRFS_FS_INFO_FLAG_GENERATION)?;
    let answered = u64::from_ne_bytes(args[48..56].try_into().unwrap());
    if answered & BTRFS_FS_INFO_FLAG_GENERATION == 0 {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "the kernel doesn't say a Btrfs filesystem's generation",
        ));
    }
    Ok(u64::from_ne_bytes(args[56..64].try_into().unwrap()))
}

/// `struct btrfs_ioctl_fs_info_args`, all 1024 bytes of it, asked for with
/// `flags`.
fn btrfs_fs_info(fd: &rustix::fd::OwnedFd, flags: u64) -> io::Result<[u8; 1024]> {
    let mut args = [0u8; 1024];
    args[48..56].copy_from_slice(&flags.to_ne_bytes());
    // SAFETY: args is as large as the kernel writes, and outlives the call.
    if unsafe { libc::ioctl(fd.as_raw_fd(), BTRFS_IOC_FS_INFO as _, args.as_mut_ptr()) } != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(args)
}
use std::ffi::OsString;
use std::fmt::Write as _;
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
