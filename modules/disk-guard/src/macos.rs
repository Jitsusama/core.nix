//! What macOS says about the machine, through sysctl and libproc. Each
//! function takes what Linux's does, so the daemon asks either the same
//! way; the places Linux reads from mean nothing here.

use crate::band::Space;
use std::ffi::{CStr, OsStr, c_int};
use std::mem::{MaybeUninit, size_of};
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::{io, ptr};

/// A process's start counts microseconds since the epoch, as the kernel
/// keeps it. Counted from boot instead, it would move whenever the clock
/// is set and kern.boottime with it, and a process would seem to have
/// started again.
const TICKS: f64 = 1e6;

/// When the machine booted, in wall seconds, and how many ticks a second
/// process start times count in. Starts are counted from the epoch, so
/// boot is its zero.
pub fn boot(_proc_stat: &Path) -> io::Result<crate::snapshots::Boot> {
    Ok(crate::snapshots::Boot {
        wall: 0.0,
        ticks: TICKS,
    })
}

/// The space of the filesystem `path` is on: `free` is what an
/// unprivileged writer may still use.
pub fn space(path: &Path) -> io::Result<Space> {
    let vfs = rustix::fs::statvfs(path)?;
    Ok(Space {
        total: vfs.f_blocks * vfs.f_frsize,
        free: vfs.f_bavail * vfs.f_frsize,
        btrfs: None,
    })
}

/// Nothing to wait for: APFS shows what was deleted as free at once.
pub fn settle(_path: &Path) -> io::Result<()> {
    Ok(())
}

/// Nothing to hand back: macOS's allocator returns what's idle itself.
pub fn give_back() {}

/// The folder macOS keeps for the running account's temporary files, under
/// /var/folders, whatever TMPDIR says: a program the account starts is told
/// it through TMPDIR, but a harness may set its own, and the guard's plan
/// read there should name the folder its daemon sweeps.
#[must_use]
pub fn account_temp() -> Option<PathBuf> {
    // SAFETY: a null buffer of no length asks only for the size needed.
    let size = unsafe { libc::confstr(libc::_CS_DARWIN_USER_TEMP_DIR, ptr::null_mut(), 0) };
    if size == 0 {
        return None;
    }
    let mut folder = vec![0_u8; size];
    // SAFETY: folder holds size bytes, which confstr writes at most.
    let written = unsafe {
        libc::confstr(
            libc::_CS_DARWIN_USER_TEMP_DIR,
            folder.as_mut_ptr().cast(),
            size,
        )
    };
    if written == 0 || written > size {
        return None;
    }
    // What it wrote ends in a NUL, which a path doesn't hold.
    folder.truncate(written - 1);
    Some(PathBuf::from(OsStr::from_bytes(&folder)))
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

/// The clocks awake time is counted from. On macOS uptime stops while the
/// machine sleeps and monotonic time doesn't, the other way round from
/// Linux's names, so suspended time is the gap between them; the boot
/// session's id tells one boot from the next.
pub fn tick() -> io::Result<crate::clock::Tick> {
    let awake = clock(libc::CLOCK_UPTIME_RAW)?;
    let since_boot = clock(libc::CLOCK_MONOTONIC)?;
    let wall = clock(libc::CLOCK_REALTIME)?;
    let boot = sysctl_bytes(c"kern.bootsessionuuid")?;
    let boot = String::from_utf8_lossy(&boot);
    Ok(crate::clock::Tick {
        wall,
        monotonic: awake,
        suspended: (since_boot - awake).max(0.0),
        boot: boot.trim_end_matches('\0').trim().to_owned(),
    })
}

/// macOS keeps no stall averages, so it reads as none.
pub fn pressure(_proc_pressure: &Path) -> io::Result<crate::moments::Pressure> {
    Ok(crate::moments::Pressure::default())
}

/// Where power comes from, as `pmset` says.
pub fn power(_supplies: &Path) -> io::Result<crate::moments::Power> {
    let output = std::process::Command::new("/usr/bin/pmset")
        .args(["-g", "batt"])
        .output()?;
    crate::pmset::power(&String::from_utf8_lossy(&output.stdout))
        .ok_or_else(|| io::Error::other("pmset didn't say where power comes from"))
}

/// The processes waiting on the guard's locks. macOS lists no one's locks,
/// so it's none: a build waiting on a directory the guard has just moved
/// aside builds again in a new one.
pub fn lock_waiters(_proc_locks: &Path, _me: u32, _inodes: &[u64]) -> io::Result<Vec<i32>> {
    Ok(Vec::new())
}

/// A value of a fixed size, by its sysctl name.
fn sysctl_value<T: Copy>(name: &CStr) -> io::Result<T> {
    let mut value = MaybeUninit::<T>::zeroed();
    let mut size = size_of::<T>();
    // SAFETY: the kernel writes at most size bytes into value, and says how
    // many it wrote.
    let failed = unsafe {
        libc::sysctlbyname(
            name.as_ptr(),
            value.as_mut_ptr().cast(),
            &raw mut size,
            ptr::null_mut(),
            0,
        )
    } != 0;
    if failed {
        return Err(io::Error::last_os_error());
    }
    if size != size_of::<T>() {
        return Err(io::Error::other(format!("{name:?} is {size} bytes")));
    }
    // SAFETY: the kernel filled every byte of a plain C value.
    Ok(unsafe { value.assume_init() })
}

/// A value of any size, by its sysctl name.
fn sysctl_bytes(name: &CStr) -> io::Result<Vec<u8>> {
    let mut size = 0;
    // SAFETY: a null buffer asks only for the size.
    let asked = unsafe {
        libc::sysctlbyname(
            name.as_ptr(),
            ptr::null_mut(),
            &raw mut size,
            ptr::null_mut(),
            0,
        )
    };
    if asked != 0 {
        return Err(io::Error::last_os_error());
    }
    let mut bytes = vec![0u8; size];
    // SAFETY: bytes has room for size bytes.
    let read = unsafe {
        libc::sysctlbyname(
            name.as_ptr(),
            bytes.as_mut_ptr().cast(),
            &raw mut size,
            ptr::null_mut(),
            0,
        )
    };
    if read != 0 {
        return Err(io::Error::last_os_error());
    }
    bytes.truncate(size);
    Ok(bytes)
}

/// `struct proc_fileinfo`, from xnu's `sys/proc_info.h`, which libc lacks.
#[repr(C)]
#[derive(Clone, Copy)]
#[expect(
    clippy::struct_field_names,
    reason = "its fields are named as xnu's `sys/proc_info.h` names them"
)]
struct ProcFileInfo {
    fi_openflags: u32,
    fi_status: u32,
    fi_offset: libc::off_t,
    fi_type: i32,
    fi_guardflags: u32,
}

/// `struct vnode_fdinfowithpath`: an open file and the path it's at.
#[repr(C)]
#[derive(Clone, Copy)]
struct VnodeFdInfoWithPath {
    pfi: ProcFileInfo,
    pvip: libc::vnode_info_path,
}

/// `struct proc_regioninfo`: one region of a process's memory.
#[repr(C)]
#[derive(Clone, Copy)]
#[expect(
    clippy::struct_field_names,
    reason = "its fields are named as xnu's `sys/proc_info.h` names them"
)]
struct ProcRegionInfo {
    pri_protection: u32,
    pri_max_protection: u32,
    pri_inheritance: u32,
    pri_flags: u32,
    pri_offset: u64,
    pri_behavior: u32,
    pri_user_wired_count: u32,
    pri_user_tag: u32,
    pri_pages_resident: u32,
    pri_pages_shared_now_private: u32,
    pri_pages_swapped_out: u32,
    pri_pages_dirtied: u32,
    pri_ref_count: u32,
    pri_shadow_depth: u32,
    pri_share_mode: u32,
    pri_private_pages_resident: u32,
    pri_shared_pages_resident: u32,
    pri_obj_id: u32,
    pri_depth: u32,
    pri_address: u64,
    pri_size: u64,
}

/// `struct proc_regionwithpathinfo`: a region and the file it maps.
#[repr(C)]
#[derive(Clone, Copy)]
struct ProcRegionWithPathInfo {
    prp_prinfo: ProcRegionInfo,
    prp_vip: libc::vnode_info_path,
}

// The kernel answers only a buffer of exactly its own size, so a layout
// that drifted from xnu's fails here rather than reading nothing.
const _: () = assert!(size_of::<libc::proc_bsdinfo>() == 136);
const _: () = assert!(size_of::<ProcFileInfo>() == 24);
const _: () = assert!(size_of::<VnodeFdInfoWithPath>() == 1200);
const _: () = assert!(size_of::<ProcRegionInfo>() == 96);
const _: () = assert!(size_of::<ProcRegionWithPathInfo>() == 1272);

/// `PROC_PIDFDVNODEPATHINFO`: an open file's vnode and path.
const PROC_PIDFDVNODEPATHINFO: c_int = 2;

/// `PROC_PIDREGIONPATHINFO`: the region at or after an address, and its path.
const PROC_PIDREGIONPATHINFO: c_int = 8;

/// How many regions are walked at most, against a process whose map keeps
/// changing under the walk.
const REGIONS_AT_MOST: usize = 100_000;

/// The C string in a fixed buffer, which a NUL or its end closes.
fn bytes(chars: &[libc::c_char]) -> Vec<u8> {
    chars
        .iter()
        .take_while(|&&c| c != 0)
        .map(|&c| c.cast_unsigned())
        .collect()
}

/// The absolute path in a vnode's path buffer, if it holds one.
fn path(vnode: &libc::vnode_info_path) -> Option<PathBuf> {
    let path = bytes(vnode.vip_path.as_flattened());
    path.starts_with(b"/")
        .then(|| PathBuf::from(OsStr::from_bytes(&path)))
}

/// Clears errno, since libproc answers 0 both for nothing and for failing.
fn clear_errno() {
    // SAFETY: __error points at this thread's errno.
    unsafe { *libc::__error() = 0 };
}

/// libproc's answer about `pid` of exactly `T`'s size.
fn pidinfo<T: Copy>(pid: c_int, flavor: c_int, arg: u64) -> io::Result<T> {
    let mut value = MaybeUninit::<T>::zeroed();
    let size = c_int::try_from(size_of::<T>()).map_err(io::Error::other)?;
    clear_errno();
    // SAFETY: value has room for size bytes, which the kernel writes at most.
    let got = unsafe { libc::proc_pidinfo(pid, flavor, arg, value.as_mut_ptr().cast(), size) };
    if got != size {
        let error = io::Error::last_os_error();
        return Err(match error.raw_os_error() {
            Some(0) | None => io::Error::other(format!("{got} bytes where {size} were asked")),
            _ => error,
        });
    }
    // SAFETY: the kernel filled every byte of a plain C structure.
    Ok(unsafe { value.assume_init() })
}

/// Every process there is, by pid, with room for those started meanwhile.
fn pids() -> io::Result<Vec<libc::pid_t>> {
    // SAFETY: a null buffer asks only how many there are.
    let count = unsafe { libc::proc_listallpids(ptr::null_mut(), 0) };
    // Negative, the call failed.
    let Ok(count) = usize::try_from(count) else {
        return Err(io::Error::last_os_error());
    };
    let mut pids: Vec<libc::pid_t> = vec![0; count + 64];
    let room = c_int::try_from(pids.len() * size_of::<libc::pid_t>()).map_err(io::Error::other)?;
    // SAFETY: pids has room for room bytes.
    let count = unsafe { libc::proc_listallpids(pids.as_mut_ptr().cast(), room) };
    let Ok(count) = usize::try_from(count) else {
        return Err(io::Error::last_os_error());
    };
    pids.truncate(count);
    Ok(pids)
}

/// Every process but the kernel's and zombies. A process that exits
/// mid-scan is left out. One whose cwd, files or maps are hidden, as
/// another account's are, reads as unreadable rather than as idle.
pub fn processes(_proc: &Path) -> io::Result<Vec<crate::activity::Process>> {
    let argmax: c_int = sysctl_value(c"kern.argmax")?;
    // A negative maximum leaves no room, as it says nothing.
    let mut args = vec![0u8; usize::try_from(argmax).unwrap_or(0)];
    Ok(pids()?
        .into_iter()
        .filter(|&pid| pid > 0)
        .filter_map(|pid| process(pid, &mut args))
        .collect())
}

/// When process `pid` started, in the microseconds `processes` gives as its
/// start, or none when it's gone or a zombie, which holds no files. Its BSD
/// info is anybody's to read.
pub fn started(_proc: &Path, pid: u32) -> Option<u64> {
    let info: libc::proc_bsdinfo = pidinfo(pid.cast_signed(), libc::PROC_PIDTBSDINFO, 0).ok()?;
    (info.pbi_status != libc::SZOMB).then(|| start_of(&info))
}

fn start_of(info: &libc::proc_bsdinfo) -> u64 {
    info.pbi_start_tvsec
        .saturating_mul(1_000_000)
        .saturating_add(info.pbi_start_tvusec)
}

/// One process, or none when it's gone or a zombie. `args` is room for its
/// arguments, shared between processes.
fn process(pid: libc::pid_t, args: &mut [u8]) -> Option<crate::activity::Process> {
    let info: libc::proc_bsdinfo = pidinfo(pid, libc::PROC_PIDTBSDINFO, 0).ok()?;
    if info.pbi_status == libc::SZOMB {
        return None;
    }
    let name = if info.pbi_name[0] != 0 {
        bytes(&info.pbi_name)
    } else {
        bytes(&info.pbi_comm)
    };
    let start_ticks = start_of(&info);

    let cwd: io::Result<Option<PathBuf>> =
        pidinfo::<libc::proc_vnodepathinfo>(pid, libc::PROC_PIDVNODEPATHINFO, 0)
            .map(|paths| path(&paths.pvi_cdir));
    let files = files(pid);
    let maps = maps(pid);
    let readable = cwd.is_ok() && files.is_ok() && maps.is_ok();
    let crate::procargs::Args { argv, env } = arguments(pid, args).unwrap_or_default();

    Some(crate::activity::Process {
        pid: u32::try_from(pid).ok()?,
        uid: info.pbi_ruid,
        start_ticks,
        name: String::from_utf8_lossy(&name).into_owned(),
        exe: exe(pid),
        cwd: cwd.ok().flatten(),
        cmdline: argv,
        env,
        files: files.unwrap_or_default(),
        maps: maps.unwrap_or_default(),
        readable,
        write_bytes: written(pid),
        cgroup: None,
    })
}

/// The executable's path.
fn exe(pid: libc::pid_t) -> Option<PathBuf> {
    let mut path = vec![0u8; libc::PROC_PIDPATHINFO_MAXSIZE as usize];
    let room = u32::try_from(path.len()).ok()?;
    // SAFETY: path has room for the size given.
    let length = unsafe { libc::proc_pidpath(pid, path.as_mut_ptr().cast(), room) };
    // Nothing, or negative, it failed.
    let length = usize::try_from(length).ok().filter(|&length| length > 0)?;
    path.truncate(length);
    Some(PathBuf::from(OsStr::from_bytes(&path)))
}

/// The files it holds open: of its descriptors, those that are vnodes.
fn files(pid: libc::pid_t) -> io::Result<Vec<PathBuf>> {
    clear_errno();
    // SAFETY: a null buffer asks only for the size.
    let size = unsafe { libc::proc_pidinfo(pid, libc::PROC_PIDLISTFDS, 0, ptr::null_mut(), 0) };
    // Nothing, or negative, it has no descriptors or the call failed.
    let Some(size) = usize::try_from(size).ok().filter(|&size| size > 0) else {
        let error = io::Error::last_os_error();
        return match error.raw_os_error() {
            Some(0) | None => Ok(Vec::new()),
            _ => Err(error),
        };
    };
    let none = libc::proc_fdinfo {
        proc_fd: 0,
        proc_fdtype: 0,
    };
    let mut fds = vec![none; size / size_of::<libc::proc_fdinfo>() + 16];
    let room =
        c_int::try_from(fds.len() * size_of::<libc::proc_fdinfo>()).map_err(io::Error::other)?;
    // SAFETY: fds has room for room bytes.
    let got =
        unsafe { libc::proc_pidinfo(pid, libc::PROC_PIDLISTFDS, 0, fds.as_mut_ptr().cast(), room) };
    let Some(got) = usize::try_from(got).ok().filter(|&got| got > 0) else {
        return Err(io::Error::last_os_error());
    };
    fds.truncate(got / size_of::<libc::proc_fdinfo>());
    let mut files: Vec<PathBuf> = fds
        .iter()
        .filter(|fd| fd.proc_fdtype == libc::PROX_FDTYPE_VNODE as u32)
        .filter_map(|fd| {
            let mut vnode = MaybeUninit::<VnodeFdInfoWithPath>::zeroed();
            let size = c_int::try_from(size_of::<VnodeFdInfoWithPath>()).ok()?;
            // SAFETY: vnode has room for size bytes.
            let got = unsafe {
                libc::proc_pidfdinfo(
                    pid,
                    fd.proc_fd,
                    PROC_PIDFDVNODEPATHINFO,
                    vnode.as_mut_ptr().cast(),
                    size,
                )
            };
            // SAFETY: the kernel filled every byte, as it answered size.
            (got == size).then(|| path(&unsafe { vnode.assume_init() }.pvip))?
        })
        .collect();
    files.sort();
    files.dedup();
    Ok(files)
}

/// The files it has mapped, region by region. A process that can't be
/// asked about its first region is hidden; past that, the walk ends at
/// the last region.
fn maps(pid: libc::pid_t) -> io::Result<Vec<PathBuf>> {
    let mut maps = Vec::new();
    let mut address = 0u64;
    for walked in 0..REGIONS_AT_MOST {
        let region = match pidinfo::<ProcRegionWithPathInfo>(pid, PROC_PIDREGIONPATHINFO, address) {
            Ok(region) => region,
            Err(error) if walked == 0 => return Err(error),
            Err(_) => break,
        };
        if let Some(mapped) = path(&region.prp_vip)
            && !maps.contains(&mapped)
        {
            maps.push(mapped);
        }
        let end = region
            .prp_prinfo
            .pri_address
            .saturating_add(region.prp_prinfo.pri_size);
        if end <= address {
            break;
        }
        address = end;
    }
    Ok(maps)
}

/// Its arguments and the environment variables the guard keeps, through
/// `KERN_PROCARGS2`, which only its own account and root may read.
fn arguments(pid: libc::pid_t, room: &mut [u8]) -> Option<crate::procargs::Args> {
    let mut name = [libc::CTL_KERN, libc::KERN_PROCARGS2, pid];
    let mut size = room.len();
    // SAFETY: room has room for size bytes, and the kernel says how many
    // it wrote.
    let read = unsafe {
        libc::sysctl(
            name.as_mut_ptr(),
            u32::try_from(name.len()).ok()?,
            room.as_mut_ptr().cast(),
            &raw mut size,
            ptr::null_mut(),
            0,
        )
    };
    if read != 0 {
        return None;
    }
    crate::procargs::parse(room.get(..size)?, crate::activity::kept)
}

/// How many bytes it has written to disk.
fn written(pid: libc::pid_t) -> Option<u64> {
    let mut usage = MaybeUninit::<libc::rusage_info_v2>::zeroed();
    // SAFETY: usage is the structure RUSAGE_INFO_V2 asks for.
    let got =
        unsafe { libc::proc_pid_rusage(pid, libc::RUSAGE_INFO_V2, usage.as_mut_ptr().cast()) };
    // SAFETY: the kernel filled it, as it answered 0.
    (got == 0).then(|| unsafe { usage.assume_init() }.ri_diskio_byteswritten)
}
