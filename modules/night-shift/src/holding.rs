//! Who holds the turn: the note that names the job holding it, for those
//! waiting to say who they wait for, and the kernel's own word on which
//! process holds the turn's lock, which alone decides whether a turn asked
//! for inside a turn's command runs in it.

use crate::ahead;
use std::fs::{self, File};
use std::path::Path;
use std::time::SystemTime;

/// Notes in `runtime` that `name`, this process, has the turn from now,
/// for those waiting to say who they wait for. It's written whole beside
/// the note and then named, so a reader never finds half of it, and a
/// tab or line break in the name, which would shift the note's fields, is
/// written as a space. Who holds the turn is the kernel's word alone
/// ([`held_above`]), so a note that can't be written only leaves those
/// waiting naming nobody, and is said.
pub fn note(runtime: &Path, name: &str) {
    let since = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |since| since.as_secs());
    let pid = std::process::id();
    let shown = name.replace(['\t', '\n', '\r'], " ");
    let writing = runtime.join(format!(".holder-{pid}"));
    let written = fs::write(&writing, format!("{shown}\t{pid}\t{since}\n"))
        .and_then(|()| fs::rename(&writing, runtime.join("holder")));
    if let Err(error) = written {
        let _ = fs::remove_file(&writing);
        eprintln!(
            "night-shift {name}: can't note that it has the turn ({error}), so those waiting \
             for it can't say who has it"
        );
    }
}

/// Who has the turn, as the last to take it noted, while its process
/// still runs.
pub fn holder(runtime: &Path) -> String {
    noted(runtime).map_or_else(
        || "a job that doesn't say".into(),
        |(name, pid, minutes)| format!("{name} (process {pid}, for {minutes} min)"),
    )
}

/// The note's name, process and minutes held, while its process runs.
/// Only its line break is trimmed, so an empty name shifts no field.
fn noted(runtime: &Path) -> Option<(String, i32, u64)> {
    let noted = fs::read_to_string(runtime.join("holder")).ok()?;
    let mut fields = noted.trim_end_matches('\n').split('\t');
    let (Some(name), Some(pid), Some(since)) = (fields.next(), fields.next(), fields.next()) else {
        return None;
    };
    let pid = pid.parse::<i32>().ok().filter(|pid| *pid > 0)?;
    if !Path::new("/proc").join(pid.to_string()).exists() {
        return None;
    }
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |now| now.as_secs());
    let minutes = now.saturating_sub(since.parse().unwrap_or(now)) / 60;
    Some((name.to_owned(), pid, minutes))
}

/// Who holds `lock`, as far as the kernel says.
pub enum Held {
    /// This process, or one of its parents, as the process named.
    Above(String),
    /// Another process, or nobody.
    Elsewhere,
    /// The kernel's word couldn't be read, or named nobody, as given.
    Unread(String),
}

/// Whether the turn in `runtime` is held by this process or one above it,
/// by the kernel's own word on who holds `lock`: a turn asked for inside a
/// turn's command has the turn already, and waiting for the lock its own
/// parent holds would wait for ever, as a cargo test an agent ran in its
/// turn did once nix-offload asked for a turn of its own. The note plays
/// no part but to name the holder, so one left standing, missing or
/// unwritten neither runs a turn beside another job nor makes one wait on
/// its own parent. A holder that let the lock go while it lives, as a
/// night job paused for a job Joel waits on does, holds nothing the
/// kernel names. "Above" is this process's parents alone: a turn asked
/// for by work a turn's command handed elsewhere, to a unit or a terminal
/// server started before it, waits like any other.
pub fn held_above(runtime: &Path, lock: &File) -> Held {
    let holders = match lock_holders(lock) {
        Ok(holders) if holders.is_empty() => {
            // Nobody, or a device the kernel names otherwise, which a
            // caller that found the lock taken tells apart.
            return Held::Unread("the kernel names no holder of its lock".into());
        }
        Ok(holders) => holders,
        Err(why) => return Held::Unread(why),
    };
    let Ok(mut at) = i32::try_from(std::process::id()) else {
        return Held::Elsewhere;
    };
    // pid 1 and the kernel's 0 hold no turn, and a chain longer than any
    // real one is a loop read mid-change.
    for _ in 0..4096 {
        if at <= 1 {
            break;
        }
        if holders.contains(&at) {
            let named = noted(runtime)
                .filter(|(_, pid, _)| *pid == at)
                .map_or_else(|| format!("process {at}"), |_| holder(runtime));
            return Held::Above(named);
        }
        let Some(parent) = ahead::stat(&at.to_string()).and_then(|stat| stat.get(1)?.parse().ok())
        else {
            break;
        };
        at = parent;
    }
    Held::Elsewhere
}

/// The processes holding a flock on `lock`, as the kernel's `/proc/locks`
/// says: lines `ID: FLOCK ADVISORY WRITE PID MAJ:MIN:INODE` for the lock's
/// file, a waiter's lines being marked `->`. The kernel names the file by
/// its filesystem's device, which stat's needn't be: on a btrfs subvolume
/// stat gives the subvolume's own (on 23 November, 0:533 for a file the
/// kernel named 00:22), so the device is the mount's, as mountinfo gives
/// it, or stat's where that can't be read.
fn lock_holders(lock: &File) -> Result<Vec<i32>, String> {
    use std::os::unix::fs::MetadataExt;
    let held = lock
        .metadata()
        .map_err(|error| format!("the lock can't be read: {error}"))?;
    let (major, minor) = mounted_device(lock).unwrap_or_else(|| {
        // Linux's own encoding of a device in 32 bits, 12 for the major
        // number and 20 for the minor, split around it.
        let dev = held.dev();
        ((dev >> 8) & 0xfff, (dev & 0xff) | ((dev >> 12) & 0xf_ff00))
    });
    let file = format!("{major:02x}:{minor:02x}:{}", held.ino());
    let locks = fs::read_to_string("/proc/locks")
        .map_err(|error| format!("/proc/locks can't be read: {error}"))?;
    Ok(locks
        .lines()
        .filter_map(|line| {
            let words: Vec<&str> = line.split_whitespace().collect();
            let theirs = words.get(1) == Some(&"FLOCK") && words.get(5) == Some(&file.as_str());
            words.get(4).filter(|_| theirs)?.parse().ok()
        })
        .collect())
}

/// The device of the filesystem `lock` is on, as `/proc/self/mountinfo`
/// gives its mount's: the one `/proc/locks` names it by.
fn mounted_device(lock: &File) -> Option<(u64, u64)> {
    use std::os::fd::AsRawFd;
    let info = fs::read_to_string(format!("/proc/self/fdinfo/{}", lock.as_raw_fd())).ok()?;
    let mount = info
        .lines()
        .find_map(|line| line.strip_prefix("mnt_id:"))?
        .trim();
    let mounts = fs::read_to_string("/proc/self/mountinfo").ok()?;
    let device = mounts.lines().find_map(|line| {
        let mut words = line.split(' ');
        (words.next() == Some(mount)).then(|| words.nth(1))?
    })?;
    let (major, minor) = device.split_once(':')?;
    Some((major.parse().ok()?, minor.parse().ok()?))
}

#[cfg(test)]
mod tests {
    use super::*;

    // The kernel names a held lock's file by its filesystem's device,
    // which is read from the mount rather than from stat, so this process
    // is found holding it on whatever filesystem the temp folder is.
    #[test]
    fn a_lock_held_here_is_held_by_this_process_as_the_kernel_says() {
        let folder = std::env::temp_dir().join(format!("night-shift-held-{}", std::process::id()));
        fs::create_dir_all(&folder).unwrap();
        let lock = File::create(folder.join("lock")).unwrap();
        lock.lock().unwrap();
        let holders = lock_holders(&lock);
        let unheld = lock_holders(&File::create(folder.join("other")).unwrap());
        drop(lock);
        fs::remove_dir_all(&folder).unwrap();
        let own = i32::try_from(std::process::id()).unwrap();
        assert_eq!(
            holders,
            Ok(vec![own]),
            "the kernel's word on the lock was misread"
        );
        assert_eq!(unheld, Ok(vec![]), "a file nobody locked was read as held");
    }

    // A tab or line break in a name, as a night job's from its config may
    // hold, and an empty name, shift none of the note's fields.
    #[test]
    fn a_note_names_its_process_whatever_its_name_holds() {
        let folder = std::env::temp_dir().join(format!("night-shift-note-{}", std::process::id()));
        fs::create_dir_all(&folder).unwrap();
        let own = i32::try_from(std::process::id()).unwrap();
        let mut read = Vec::new();
        for name in ["x\t1", "x\n1", ""] {
            note(&folder, name);
            read.push(noted(&folder));
        }
        fs::remove_dir_all(&folder).unwrap();
        assert_eq!(
            read,
            [
                Some(("x 1".to_owned(), own, 0)),
                Some(("x 1".to_owned(), own, 0)),
                Some((String::new(), own, 0)),
            ],
            "a name shifted the note's fields"
        );
    }
}
