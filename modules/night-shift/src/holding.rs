//! Who holds the turn: the note that names the job holding it, for those
//! waiting to say who they wait for, and the kernel's own word on which
//! process holds the turn's lock, which alone decides whether a turn asked
//! for inside a turn's command runs in it, and which the note is named by.

use crate::ahead;
use std::fs::{self, File};
use std::path::Path;
use std::time::SystemTime;

/// A job's name as a note or a mark writes it: every control character
/// a space, so a tab or line break can't shift the fields read after it,
/// and an escape can't reach the terminal of a job that names it.
pub fn shown(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect()
}

/// Notes in `runtime` that `name`, this process, has the turn from now,
/// for those waiting to say who they wait for. It's written whole beside
/// the note and then named, so a reader never finds half of it. Who holds
/// the turn is the kernel's word alone ([`held_above`]), and a note is
/// named only while its process holds the lock, so a note that can't be
/// written, which leaves the last one in place, names nobody, and is said.
pub fn note(runtime: &Path, name: &str) {
    let since = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |since| since.as_secs());
    let pid = std::process::id();
    let writing = runtime.join(format!(".holder-{pid}"));
    let written = fs::write(&writing, format!("{}\t{pid}\t{since}\n", shown(name)))
        .and_then(|()| fs::rename(&writing, runtime.join("holder")));
    if let Err(error) = written {
        let _ = fs::remove_file(&writing);
        eprintln!(
            "night-shift {}: can't note that it has the turn ({error}), so those waiting for \
             it can't say who has it",
            shown(name)
        );
    }
}

/// Who holds `lock` in `runtime`, as the kernel names them: the note's
/// job when its process is among them, or the processes themselves.
pub fn holder_of(runtime: &Path, lock: &File) -> String {
    match lock_holders(lock) {
        Ok(holders) => named(runtime, &holders),
        Err(_) => named(runtime, &[]),
    }
}

/// The holders `holders` as a waiting line says them: the note's job
/// when its process is one of them, since a note left by a job that let
/// go, or by one whose number a later process took, names the wrong one.
fn named(runtime: &Path, holders: &[i32]) -> String {
    if let Some(noted) = noted(runtime).filter(|(_, pid, _)| holders.contains(pid)) {
        return described(&noted);
    }
    match holders {
        [] => "a job that doesn't say".into(),
        [one] => format!("process {one}"),
        many => {
            let pids: Vec<String> = many.iter().map(i32::to_string).collect();
            format!("processes {}", pids.join(", "))
        }
    }
}

/// A note's name, process and minutes as a line says them.
fn described((name, pid, minutes): &(String, i32, u64)) -> String {
    format!("{name} (process {pid}, for {minutes} min)")
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
#[derive(Debug, PartialEq)]
pub enum Held {
    /// This process, or one of its parents, as the holder named.
    Above(String),
    /// Another process, as named, and none of this process's parents.
    Elsewhere(String),
    /// Whether a parent holds it couldn't be told, as given.
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
/// server started before it, waits like any other. A walk that can't read
/// a parent, or never ends, says it couldn't tell rather than that the
/// holder is elsewhere, so a wait on its own parent is said as one.
pub fn held_above(runtime: &Path, lock: &File) -> Held {
    let holders = match lock_holders(lock) {
        Ok(holders) if holders.is_empty() => {
            // Nobody, a holder in a pid namespace this process can't see,
            // or a file id the kernel names otherwise, which only the ids
            // looked for can tell apart.
            return Held::Unread(format!(
                "the kernel names no holder of its lock this process can see, looking for {}",
                file_ids(lock).join(" or ")
            ));
        }
        Ok(holders) => holders,
        Err(why) => return Held::Unread(why),
    };
    let Ok(mut at) = i32::try_from(std::process::id()) else {
        return Held::Unread("this process's number can't be read".into());
    };
    // A chain longer than any real one is a loop read mid-change.
    for _ in 0..4096 {
        // The kernel names only live holders this process can see, so
        // even pid 1, a pid namespace's first process, is asked.
        if holders.contains(&at) {
            let named = noted(runtime)
                .filter(|(_, pid, _)| *pid == at)
                .map_or_else(|| format!("process {at}"), |noted| described(&noted));
            return Held::Above(named);
        }
        if at <= 1 {
            return Held::Elsewhere(named(runtime, &holders));
        }
        let Some(parent) = ahead::stat(&at.to_string()).and_then(|stat| stat.get(1)?.parse().ok())
        else {
            return Held::Unread(format!("process {at}'s parent can't be read"));
        };
        at = parent;
    }
    Held::Unread("this process's parents never ended".into())
}

/// The processes holding a flock on `lock`, as the kernel's `/proc/locks`
/// says: lines `ID: FLOCK ADVISORY WRITE PID MAJ:MIN:INODE` for the lock's
/// file, a waiter's lines being marked `->`. Each process is named once.
fn lock_holders(lock: &File) -> Result<Vec<i32>, String> {
    let ids = file_ids(lock);
    if ids.is_empty() {
        return Err("the lock can't be read".into());
    }
    let locks = fs::read_to_string("/proc/locks")
        .map_err(|error| format!("/proc/locks can't be read: {error}"))?;
    let mut holders: Vec<i32> = locks
        .lines()
        .filter_map(|line| {
            let words: Vec<&str> = line.split_whitespace().collect();
            let theirs = words.get(1) == Some(&"FLOCK")
                && words
                    .get(5)
                    .is_some_and(|file| ids.iter().any(|id| id == file));
            words.get(4).filter(|_| theirs)?.parse().ok()
        })
        .collect();
    holders.sort_unstable();
    holders.dedup();
    Ok(holders)
}

/// The ids `/proc/locks` may name `lock`'s file by, `MAJ:MIN:INODE`, as
/// it prints them. The kernel names a file by its filesystem's device,
/// which stat's needn't be: on a btrfs subvolume stat gives the
/// subvolume's own (on 23 November, 0:533 for a file the kernel named
/// 00:22). So the mount's device, as mountinfo gives it, comes first, and
/// stat's after it where it differs: a device is given to one filesystem
/// alone, so neither can name another's file. On an overlay without its
/// inode numbers kept apart, stat's inode may differ from the kernel's
/// too, which no id here covers; the lock lives in the runtime folder, a
/// tmpfs, so it isn't met.
fn file_ids(lock: &File) -> Vec<String> {
    use std::os::unix::fs::MetadataExt;
    let Ok(held) = lock.metadata() else {
        return Vec::new();
    };
    let mut devices = Vec::new();
    devices.extend(mounted_device(lock));
    devices.push(stat_device(held.dev()));
    devices.dedup();
    devices
        .into_iter()
        .map(|(major, minor)| format!("{major:02x}:{minor:02x}:{}", held.ino()))
        .collect()
}

/// A device number as stat gives it, split by Linux's own encoding into
/// its major number's 12 bits and its minor number's 20, around it.
fn stat_device(dev: u64) -> (u64, u64) {
    ((dev >> 8) & 0xfff, (dev & 0xff) | ((dev >> 12) & 0xf_ff00))
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

    /// The lines of `/proc/locks` naming any of `ids`, to say what the
    /// kernel said when a reading goes wrong.
    fn locks_naming(ids: &[String]) -> Vec<String> {
        fs::read_to_string("/proc/locks")
            .unwrap_or_default()
            .lines()
            .filter(|line| ids.iter().any(|id| line.contains(id.as_str())))
            .map(str::to_owned)
            .collect()
    }

    // The kernel names a held lock's file by its filesystem's device,
    // which is read from the mount, so this process is found holding it,
    // once, on whatever filesystem the temp folder is, and the mount's
    // device is read rather than left to stat's.
    #[test]
    fn a_lock_held_here_is_held_by_this_process_as_the_kernel_says() {
        let folder = std::env::temp_dir().join(format!("night-shift-held-{}", std::process::id()));
        fs::create_dir_all(&folder).unwrap();
        let lock = File::create(folder.join("lock")).unwrap();
        lock.lock().unwrap();
        let holders = lock_holders(&lock);
        let ids = file_ids(&lock);
        let said = locks_naming(&ids);
        let mounted = mounted_device(&lock);
        let other = File::create(folder.join("other")).unwrap();
        let unheld = lock_holders(&other);
        let unread = held_above(&folder, &other);
        drop(lock);
        fs::remove_dir_all(&folder).unwrap();
        let own = i32::try_from(std::process::id()).unwrap();
        assert_eq!(
            holders,
            Ok(vec![own]),
            "the kernel's word on {ids:?} was misread: {said:?}"
        );
        assert!(mounted.is_some(), "the lock's mount gave no device");
        assert_eq!(unheld, Ok(vec![]), "a file nobody locked was read as held");
        assert!(
            matches!(&unread, Held::Unread(why) if why.contains("no holder")),
            "a lock nobody holds wasn't said to be unexplained: {unread:?}"
        );
    }

    // Linux keeps a device's major number in bits 8 to 19 and its minor
    // number around it, as a btrfs subvolume's 0:533 shows.
    #[test]
    fn a_device_splits_as_linux_encodes_it() {
        assert_eq!(stat_device(0x812), (8, 18), "a disk's device was misread");
        assert_eq!(
            stat_device(0x20_0015),
            (0, 533),
            "a minor number past 255 was misread"
        );
    }

    // Every control character in a name, as a night job's from its config
    // may hold, and an empty name, shift none of the note's fields.
    #[test]
    fn a_note_names_its_process_whatever_its_name_holds() {
        let folder = std::env::temp_dir().join(format!("night-shift-note-{}", std::process::id()));
        fs::create_dir_all(&folder).unwrap();
        let own = i32::try_from(std::process::id()).unwrap();
        let mut read = Vec::new();
        for name in ["x\t1", "x\n1", "x\u{1b}1", ""] {
            note(&folder, name);
            read.push(noted(&folder));
        }
        fs::remove_dir_all(&folder).unwrap();
        assert_eq!(
            read,
            [
                Some(("x 1".to_owned(), own, 0)),
                Some(("x 1".to_owned(), own, 0)),
                Some(("x 1".to_owned(), own, 0)),
                Some((String::new(), own, 0)),
            ],
            "a name shifted the note's fields"
        );
    }

    // A note whose process holds nothing the kernel names, as a job that
    // let go leaves, isn't named as the holder.
    #[test]
    fn a_note_is_named_only_while_its_process_holds_the_lock() {
        let folder = std::env::temp_dir().join(format!("night-shift-named-{}", std::process::id()));
        fs::create_dir_all(&folder).unwrap();
        note(&folder, "paused");
        let own = i32::try_from(std::process::id()).unwrap();
        let held = named(&folder, &[own]);
        let elsewhere = named(&folder, &[own + 1]);
        let nobody = named(&folder, &[]);
        fs::remove_dir_all(&folder).unwrap();
        assert!(
            held.starts_with("paused (process"),
            "its holder went unnamed: {held}"
        );
        assert_eq!(
            elsewhere,
            format!("process {}", own + 1),
            "a note was named for a process that doesn't hold the lock"
        );
        assert_eq!(nobody, "a job that doesn't say");
    }
}
