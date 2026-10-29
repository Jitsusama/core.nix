//! What macOS says about each process through libproc: the folder it
//! stands in, the files it holds open and the files it maps. Another
//! account's processes can't be asked, and read as using nothing.

use std::ffi::{OsStr, c_int};
use std::mem::{MaybeUninit, size_of};
use std::os::unix::ffi::OsStrExt;
use std::path::PathBuf;
use std::ptr;

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
const _: () = assert!(size_of::<ProcFileInfo>() == 24);
const _: () = assert!(size_of::<VnodeFdInfoWithPath>() == 1200);
const _: () = assert!(size_of::<ProcRegionInfo>() == 96);
const _: () = assert!(size_of::<ProcRegionWithPathInfo>() == 1272);

/// `PROC_PIDFDVNODEPATHINFO`: an open file's vnode and path.
const PROC_PIDFDVNODEPATHINFO: c_int = 2;

/// `PROC_PIDREGIONPATHINFO2`: the first region at or after an address that
/// a file backs, and its path, from xnu's `proc_info_private.h`, so memory
/// no file backs is never asked about.
const PROC_PIDREGIONPATHINFO2: c_int = 22;

/// How many regions are walked at most, against a process whose map keeps
/// changing under the walk.
const REGIONS_AT_MOST: usize = 100_000;

/// Whether any process uses the version by what it stands in, holds
/// open or maps, at paths `inside` says are in it, and by what `said`
/// says it loaded (`super::uses`). Its maps are walked only when it said
/// nothing, since they can't change the answer otherwise.
pub fn any_process(inside: &dyn Fn(&str) -> bool, said: &dyn Fn(u32) -> Option<bool>) -> bool {
    let within = |path: Option<PathBuf>| {
        path.as_deref()
            .and_then(|path| path.to_str())
            .is_some_and(inside)
    };
    pids().into_iter().filter(|&pid| pid > 0).any(|pid| {
        let told = u32::try_from(pid).ok().and_then(said);
        let seen = super::Seen {
            stands: within(cwd(pid)),
            holds: files(pid).into_iter().any(|file| within(Some(file))),
            maps: told.is_none() && maps(pid, &within),
        };
        super::uses(seen, told)
    })
}

/// When process `pid` started, from its BSD information.
pub fn started(pid: u32) -> Option<std::time::SystemTime> {
    let pid = c_int::try_from(pid).ok()?;
    let info = pidinfo::<libc::proc_bsdinfo>(pid, libc::PROC_PIDTBSDINFO, 0)?;
    let since_epoch = std::time::Duration::from_secs(info.pbi_start_tvsec)
        + std::time::Duration::from_micros(info.pbi_start_tvusec);
    std::time::SystemTime::UNIX_EPOCH.checked_add(since_epoch)
}

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

/// libproc's answer about `pid` of exactly `T`'s size, or none.
fn pidinfo<T: Copy>(pid: c_int, flavor: c_int, arg: u64) -> Option<T> {
    let mut value = MaybeUninit::<T>::zeroed();
    let size = c_int::try_from(size_of::<T>()).ok()?;
    // SAFETY: value has room for size bytes, which the kernel writes at most.
    let got = unsafe { libc::proc_pidinfo(pid, flavor, arg, value.as_mut_ptr().cast(), size) };
    // SAFETY: the kernel filled every byte of a plain C structure, as it
    // answered its size.
    (got == size).then(|| unsafe { value.assume_init() })
}

/// Every process there is, by pid, with room for those started meanwhile.
fn pids() -> Vec<libc::pid_t> {
    // SAFETY: a null buffer asks only how many there are.
    let count = unsafe { libc::proc_listallpids(ptr::null_mut(), 0) };
    let Ok(count) = usize::try_from(count) else {
        return Vec::new();
    };
    let mut pids: Vec<libc::pid_t> = vec![0; count + 64];
    let Ok(room) = c_int::try_from(pids.len() * size_of::<libc::pid_t>()) else {
        return Vec::new();
    };
    // SAFETY: pids has room for room bytes.
    let count = unsafe { libc::proc_listallpids(pids.as_mut_ptr().cast(), room) };
    pids.truncate(usize::try_from(count).unwrap_or(0));
    pids
}

/// The folder it stands in.
fn cwd(pid: libc::pid_t) -> Option<PathBuf> {
    pidinfo::<libc::proc_vnodepathinfo>(pid, libc::PROC_PIDVNODEPATHINFO, 0)
        .and_then(|paths| path(&paths.pvi_cdir))
}

/// The files it holds open: of its descriptors, those that are vnodes.
fn files(pid: libc::pid_t) -> Vec<PathBuf> {
    // SAFETY: a null buffer asks only for the size.
    let size = unsafe { libc::proc_pidinfo(pid, libc::PROC_PIDLISTFDS, 0, ptr::null_mut(), 0) };
    let Some(size) = usize::try_from(size).ok().filter(|&size| size > 0) else {
        return Vec::new();
    };
    let none = libc::proc_fdinfo {
        proc_fd: 0,
        proc_fdtype: 0,
    };
    let mut fds = vec![none; size / size_of::<libc::proc_fdinfo>() + 16];
    let Ok(room) = c_int::try_from(fds.len() * size_of::<libc::proc_fdinfo>()) else {
        return Vec::new();
    };
    // SAFETY: fds has room for room bytes.
    let got =
        unsafe { libc::proc_pidinfo(pid, libc::PROC_PIDLISTFDS, 0, fds.as_mut_ptr().cast(), room) };
    fds.truncate(usize::try_from(got).unwrap_or(0) / size_of::<libc::proc_fdinfo>());
    fds.iter()
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
        .collect()
}

/// Whether a file it maps is one `within` takes, walked region by region
/// over only the memory files back, and stopping at the first.
fn maps(pid: libc::pid_t, within: &dyn Fn(Option<PathBuf>) -> bool) -> bool {
    let mut address = 0u64;
    for _ in 0..REGIONS_AT_MOST {
        let Some(region) = pidinfo::<ProcRegionWithPathInfo>(pid, PROC_PIDREGIONPATHINFO2, address)
        else {
            return false;
        };
        if within(path(&region.prp_vip)) {
            return true;
        }
        let end = region
            .prp_prinfo
            .pri_address
            .saturating_add(region.prp_prinfo.pri_size);
        if end <= address {
            return false;
        }
        address = end;
    }
    false
}
