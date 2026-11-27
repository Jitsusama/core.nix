use disk_guard::pool::{apfs_container, pool};
use std::fs;
use std::os::fd::AsRawFd;
use std::os::unix::fs::MetadataExt;
use std::path::Path;

const BTRFS_SUPER_MAGIC: u64 = 0x9123_683e;
/// `BTRFS_IOC_SUBVOL_CREATE`, `_IOW(0x94, 14, struct btrfs_ioctl_vol_args)`,
/// which is 4096 bytes: a descriptor, then the name.
const BTRFS_IOC_SUBVOL_CREATE: libc::c_ulong = 0x5000_940e;

/// Whether `path` is on Btrfs, where subvolumes can be made.
pub(crate) fn on_btrfs(path: &Path) -> bool {
    // Signed on Linux and unsigned on a Mac, the type is widened to compare.
    i128::from(rustix::fs::statfs(path).unwrap().f_type) == i128::from(BTRFS_SUPER_MAGIC)
}

/// `BTRFS_IOC_SUBVOL_SETFLAGS`, `_IOW(0x94, 26, __u64)`, and the flag it
/// sets to make a subvolume read-only, as a snapshot taken with `-r` is.
const BTRFS_IOC_SUBVOL_SETFLAGS: libc::c_ulong = 0x4008_941a;
const BTRFS_SUBVOL_RDONLY: u64 = 1 << 1;

/// Makes the subvolume at `path` read-only, or writable again, which its
/// owner may.
pub(crate) fn read_only(path: &Path, on: bool) {
    let file = fs::File::open(path).unwrap();
    let flags: u64 = if on { BTRFS_SUBVOL_RDONLY } else { 0 };
    // SAFETY: the kernel reads one u64, which outlives the call.
    let set = unsafe {
        libc::ioctl(
            file.as_raw_fd(),
            BTRFS_IOC_SUBVOL_SETFLAGS as _,
            &raw const flags,
        )
    };
    assert_eq!(set, 0, "{}", std::io::Error::last_os_error());
}

/// `FICLONE`, `_IOW(0x94, 9, int)`: makes a file share all another's
/// extents, as `cp --reflink` does.
const FICLONE: libc::c_ulong = 0x4004_9409;

/// Makes `to` a clone of `from`, sharing its blocks rather than copying.
pub(crate) fn clone_file(from: &Path, to: &Path) {
    let source = fs::File::open(from).unwrap();
    let target = fs::File::create(to).unwrap();
    // SAFETY: the kernel reads one descriptor, passed by value.
    let cloned = unsafe { libc::ioctl(target.as_raw_fd(), FICLONE as _, source.as_raw_fd()) };
    assert_eq!(cloned, 0, "{}", std::io::Error::last_os_error());
}

/// Makes the subvolume `name` in `dir`, which any account owning `dir` may.
pub(crate) fn subvolume(dir: &Path, name: &str) {
    let file = fs::File::open(dir).unwrap();
    let mut args = [0u8; 4096];
    args[8..8 + name.len()].copy_from_slice(name.as_bytes());
    // SAFETY: args is the size the kernel reads, and outlives the call.
    let made = unsafe {
        libc::ioctl(
            file.as_raw_fd(),
            BTRFS_IOC_SUBVOL_CREATE as _,
            args.as_ptr(),
        )
    };
    assert_eq!(made, 0, "{}", std::io::Error::last_os_error());
}

#[test]
fn a_filesystem_of_one_volume_is_known_by_its_device() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir(dir.path().join("inside")).unwrap();
    assert_eq!(
        pool(dir.path()).unwrap(),
        pool(&dir.path().join("inside")).unwrap()
    );
    // A Btrfs pool or an APFS container is known by a number of its own,
    // with the top bit set; anything else by its device.
    let known = pool(dir.path()).unwrap();
    if known & 1 << 63 == 0 {
        assert_eq!(known, fs::metadata(dir.path()).unwrap().dev());
    }
    // Another filesystem on both systems: a Mac has no /proc.
    let elsewhere = if cfg!(target_os = "macos") {
        "/dev"
    } else {
        "/proc"
    };
    assert_ne!(pool(Path::new(elsewhere)).unwrap(), known);
    assert!(pool(&dir.path().join("gone")).is_err());
}

/// Btrfs gives each subvolume a device number of its own, though all of
/// them share the pool's free space, so the pool is what's known.
#[test]
fn the_subvolumes_of_one_btrfs_pool_are_one_filesystem() {
    let dir = tempfile::tempdir().unwrap();
    if !on_btrfs(dir.path()) {
        eprintln!("skipped: {} isn't on Btrfs", dir.path().display());
        return;
    }
    subvolume(dir.path(), "sub");
    let sub = dir.path().join("sub");
    let device = |path: &Path| fs::metadata(path).unwrap().dev();
    assert_ne!(device(dir.path()), device(&sub), "a subvolume of its own");
    assert_eq!(pool(dir.path()).unwrap(), pool(&sub).unwrap());
    assert_ne!(
        pool(&sub).unwrap(),
        device(&sub),
        "no device number is a pool's"
    );
}

/// A pipe holds none of its disk's bytes, so its pool is its own device,
/// read without opening it, since an open of a pipe with no writer waits
/// for one: by its name, as any account may ask the machine's guard, and
/// through a link to it, as a process the guard weighs for a freeze holds
/// one, which the kernel follows rather than the name the link says, as
/// that may be gone. Only on Btrfs was a pipe ever opened, so only there
/// does this case fail on code that opens it, by waiting.
#[cfg(target_os = "linux")]
#[test]
fn a_pipe_with_no_writer_s_pool_is_its_own_device_read_without_opening_it() {
    use std::os::unix::ffi::OsStrExt;
    use std::os::unix::fs::OpenOptionsExt;
    use std::path::PathBuf;
    let dir = tempfile::tempdir().unwrap();
    let pipe = dir.path().join("pipe");
    let name = std::ffi::CString::new(pipe.as_os_str().as_bytes()).unwrap();
    // SAFETY: a plain system call on a name this test owns.
    assert_eq!(
        unsafe { libc::mkfifo(name.as_ptr(), 0o600) },
        0,
        "no pipe made"
    );
    let device = fs::metadata(&pipe).unwrap().dev();
    // Read within five seconds, on a thread of its own, since an open
    // would never end.
    let within = |path: PathBuf| {
        let (sent, heard) = std::sync::mpsc::channel();
        std::thread::spawn(move || sent.send(pool(&path).ok()));
        heard.recv_timeout(std::time::Duration::from_secs(5)).ok()
    };
    assert_eq!(
        within(pipe.clone()),
        Some(Some(device)),
        "a pipe's pool, read by its name within five seconds, as its own device"
    );
    // Held for reading only, without waiting, so it still has no writer.
    let held = fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NONBLOCK)
        .open(&pipe)
        .unwrap();
    fs::remove_file(&pipe).unwrap();
    let link = Path::new("/proc/self/fd").join(held.as_raw_fd().to_string());
    assert_eq!(
        within(link),
        Some(Some(device)),
        "a held pipe's pool, read through its link within five seconds, as its own device"
    );
    drop(held);
}

/// A file its writer holds a lease on refuses the guard's reopen, and the
/// writer may lease it again after each break; so its pool is read as its
/// subvolume's, without waiting the lease out, or a writer could keep its
/// file off every disk and so out of every freeze.
#[cfg(target_os = "linux")]
#[test]
#[ignore = "needs TMPDIR on Btrfs, which the sandbox and the shell's tmpfs aren't: \
            run with --ignored and TMPDIR on a Btrfs folder"]
fn a_leased_file_s_pool_is_its_subvolume_s_read_without_waiting_out_the_lease() {
    let dir = tempfile::tempdir().unwrap();
    assert!(
        on_btrfs(dir.path()),
        "{} isn't on Btrfs",
        dir.path().display()
    );
    let expected = pool(dir.path()).unwrap();
    let file = dir.path().join("leased");
    let held = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&file)
        .unwrap();
    // SAFETY: plain system calls; a broken lease signals SIGIO, whose
    // default would end the test, so it's ignored.
    let leased = unsafe {
        libc::signal(libc::SIGIO, libc::SIG_IGN);
        libc::fcntl(held.as_raw_fd(), libc::F_SETLEASE, libc::F_WRLCK)
    };
    assert_eq!(leased, 0, "no lease: {}", std::io::Error::last_os_error());
    let (sent, heard) = std::sync::mpsc::channel();
    std::thread::spawn(move || sent.send(pool(&file).ok()));
    assert_eq!(
        heard.recv_timeout(std::time::Duration::from_secs(5)).ok(),
        Some(Some(expected)),
        "a leased file's pool, read within five seconds, as its subvolume's"
    );
    drop(held);
}

/// A writer may give its file a mode that lets nobody read it, which a
/// reading open would meet; so its pool is read without one, or a writer
/// could keep its file off every disk and so out of every freeze.
#[cfg(target_os = "linux")]
#[test]
#[ignore = "needs TMPDIR on Btrfs, which the sandbox and the shell's tmpfs aren't: \
            run with --ignored and TMPDIR on a Btrfs folder"]
fn a_file_nobody_may_read_s_pool_is_its_subvolume_s() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    assert!(
        on_btrfs(dir.path()),
        "{} isn't on Btrfs",
        dir.path().display()
    );
    let file = dir.path().join("unreadable");
    fs::write(&file, "written").unwrap();
    fs::set_permissions(&file, fs::Permissions::from_mode(0o000)).unwrap();
    assert_eq!(
        pool(&file).ok(),
        pool(dir.path()).ok(),
        "a file nobody may read, on its subvolume's pool"
    );
}

#[test]
fn the_volumes_of_one_apfs_container_share_its_pool() {
    let data = apfs_container("apfs", "/dev/disk3s5").unwrap();
    // The sealed system volume mounts a snapshot of its own slice.
    assert_eq!(apfs_container("apfs", "/dev/disk3s1s1"), Some(data));
    assert_eq!(apfs_container("apfs", "/dev/disk3s2"), Some(data));
    let other = apfs_container("apfs", "/dev/disk10s1").unwrap();
    assert_ne!(other, data);
    assert_ne!(data & 1 << 63, 0, "no device number is a pool's");
}

#[test]
fn what_isnt_an_apfs_volume_has_no_container() {
    assert_eq!(apfs_container("hfs", "/dev/disk4s2"), None);
    assert_eq!(apfs_container("apfs", "/dev/disk4"), None);
    assert_eq!(apfs_container("apfs", "disk3s5"), None);
    assert_eq!(apfs_container("autofs", "map auto_home"), None);
    assert_eq!(apfs_container("apfs", "/dev/diskXs1"), None);
}
