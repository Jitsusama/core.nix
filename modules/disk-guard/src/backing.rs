//! Which file a filesystem lives in, when it's an image on another disk
//! that gives that disk back each block it frees. A loop device passes a
//! discard on to its file as a punched hole, and a filesystem mounted to
//! discard sends one for each block it frees as its journal commits, so
//! a build deleted in the image frees the disk the image is on. One that
//! doesn't discard frees nothing there until it's trimmed, which takes
//! root, so the guard counts it as a disk of its own alone.

use std::fs;
use std::path::{Path, PathBuf};

/// The file the filesystem on block device `major:minor` lives in: the
/// backing file `block` (the kernel's `/sys/dev/block`) names for it as a
/// loop device, when `mountinfo`, as `/proc/self/mountinfo` reads, shows
/// it mounted to discard. A file deleted since it was attached, which the
/// kernel marks so, is on no disk the guard can free.
pub fn image(block: &Path, mountinfo: &str, (major, minor): (u32, u32)) -> Option<PathBuf> {
    let device = format!("{major}:{minor}");
    let named = fs::read_to_string(block.join(&device).join("loop/backing_file")).ok()?;
    let file = named.trim_end_matches('\n');
    if file.is_empty() || file.ends_with(" (deleted)") {
        return None;
    }
    discards(mountinfo, &device).then(|| PathBuf::from(file))
}

/// Whether a mount of `device` in `mountinfo` discards, as its own options
/// or its filesystem's say: ext4 keeps `discard` with its superblock's.
fn discards(mountinfo: &str, device: &str) -> bool {
    mountinfo.lines().any(|line| {
        let (mount, filesystem) = line.split_once(" - ").unwrap_or((line, ""));
        let fields: Vec<&str> = mount.split(' ').collect();
        let options = [
            fields.get(5).copied().unwrap_or(""),
            filesystem.split(' ').nth(2).unwrap_or(""),
        ];
        fields.get(2) == Some(&device)
            && options
                .iter()
                .any(|options| options.split(',').any(|option| option == "discard"))
    })
}
