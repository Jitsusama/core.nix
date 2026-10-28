//! Which filesystem a path is on, as its free space is shared, which is
//! how the daemons tell their filesystems apart. Walks stop at a change of
//! device, so they never cross into a mount; the walk that finds targets
//! goes on into a Btrfs subvolume, which `within` tells from a mount.

use std::io;
use std::path::Path;

/// On Linux a Btrfs pool counts once, whatever subvolumes it holds.
#[cfg(target_os = "linux")]
pub fn pool(path: &Path) -> io::Result<u64> {
    crate::linux::pool(path)
}

/// Whether a walk may go on from `parent` into `path`, a directory beneath
/// it with a device number of its own: on Linux, when it's a subvolume of
/// the same Btrfs pool, not mounted and not read-only. The work broker
/// clones each tree as a subvolume, and its builds are in it.
#[cfg(target_os = "linux")]
pub fn within(parent: &Path, path: &Path) -> bool {
    crate::linux::subvolume_within(parent, path)
}

/// Anywhere else a change of device is a mount, or an APFS volume, which
/// is mounted as one.
#[cfg(not(target_os = "linux"))]
pub fn within(_parent: &Path, _path: &Path) -> bool {
    false
}

/// The APFS container a volume is in, by what it's mounted from: volume
/// `s<n>` of `/dev/disk<container>`, or a snapshot of one, as the sealed
/// system volume is. Its volumes share the container's free space, as a
/// Btrfs pool's subvolumes do, and its number has the same top bit set.
pub fn apfs_container(kind: &str, from: &str) -> Option<u64> {
    if kind != "apfs" {
        return None;
    }
    let rest = from.strip_prefix("/dev/disk")?;
    let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
    let container: u64 = rest[..digits].parse().ok()?;
    let slice = rest[digits..].strip_prefix('s')?;
    slice
        .starts_with(|c: char| c.is_ascii_digit())
        .then_some(container | 1 << 63)
}

/// On macOS an APFS container counts once, whatever volumes it holds.
#[cfg(target_os = "macos")]
pub fn pool(path: &Path) -> io::Result<u64> {
    use std::os::unix::fs::MetadataExt;
    let stat = rustix::fs::statfs(path)?;
    let text = |chars: &[libc::c_char]| -> String {
        let bytes: Vec<u8> = chars
            .iter()
            .take_while(|&&c| c != 0)
            .map(|&c| c.cast_unsigned())
            .collect();
        String::from_utf8_lossy(&bytes).into_owned()
    };
    match apfs_container(&text(&stat.f_fstypename), &text(&stat.f_mntfromname)) {
        Some(container) => Ok(container),
        None => Ok(std::fs::metadata(path)?.dev()),
    }
}

/// Anywhere else, the device.
#[cfg(not(any(target_os = "linux", target_os = "macos")))]
pub fn pool(path: &Path) -> io::Result<u64> {
    use std::os::unix::fs::MetadataExt;
    Ok(std::fs::metadata(path)?.dev())
}
