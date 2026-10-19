use disk_guard::pool::{apfs_container, pool};
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::Path;

const BTRFS_SUPER_MAGIC: u64 = 0x9123_683e;
/// `BTRFS_IOC_SUBVOL_CREATE`, `_IOW(0x94, 14, struct btrfs_ioctl_vol_args)`,
/// which is 4096 bytes: a descriptor, then the name.
const BTRFS_IOC_SUBVOL_CREATE: libc::c_ulong = 0x5000_940e;

/// Whether `path` is on Btrfs, where subvolumes can be made.
pub(crate) fn on_btrfs(path: &Path) -> bool {
    rustix::fs::statfs(path).unwrap().f_type as u64 == BTRFS_SUPER_MAGIC
}

/// Makes the subvolume `name` in `dir`, which any account owning `dir` may.
pub(crate) fn subvolume(dir: &Path, name: &str) {
    let file = fs::File::open(dir).unwrap();
    let mut args = [0u8; 4096];
    args[8..8 + name.len()].copy_from_slice(name.as_bytes());
    use std::os::fd::AsRawFd;
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

/// On Linux a filesystem that isn't Btrfs is its device, and `/proc` is
/// another.
#[cfg(target_os = "linux")]
#[test]
fn a_filesystem_of_one_volume_is_known_by_its_device() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir(dir.path().join("inside")).unwrap();
    assert_eq!(
        pool(dir.path()).unwrap(),
        pool(&dir.path().join("inside")).unwrap()
    );
    if !on_btrfs(dir.path()) {
        assert_eq!(
            pool(dir.path()).unwrap(),
            fs::metadata(dir.path()).unwrap().dev()
        );
    }
    assert_ne!(pool(Path::new("/proc")).unwrap(), pool(dir.path()).unwrap());
    assert!(pool(&dir.path().join("gone")).is_err());
}

/// On a Mac the sealed system volume, the data volume the temporary
/// folder lies on and the rest of the startup disk's container share its
/// free space, so they're one pool, and devfs, which isn't APFS, is its
/// device.
#[cfg(target_os = "macos")]
#[test]
fn the_volumes_of_a_macs_startup_container_are_one_pool() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir(dir.path().join("inside")).unwrap();
    let data = pool(dir.path()).unwrap();
    assert_eq!(data, pool(&dir.path().join("inside")).unwrap());
    assert_ne!(data & 1 << 63, 0, "the container's, not a device's");
    assert_eq!(pool(Path::new("/")).unwrap(), data, "the system volume");
    assert_eq!(pool(Path::new("/System/Volumes/Data")).unwrap(), data);
    let devfs = pool(Path::new("/dev")).unwrap();
    assert_eq!(devfs, fs::metadata("/dev").unwrap().dev());
    assert_ne!(devfs, data);
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
