//! Who holds the turn: the note that names the job holding it, for those
//! waiting to say who they wait for, and the kernel's own word on which
//! process holds the turn's lock, which alone decides whether a turn asked
//! for inside a turn's command runs in it, and which the note is named by.

use crate::ahead;
use std::fs::{self, File};
use std::path::Path;
use std::time::SystemTime;

/// A job's name as a note or a mark writes it, and as one read back is
/// said, since anything of the account can write those files: every
/// control character, bidi override and bidi isolate a space, so a tab or
/// line break can't shift the fields read after it, an escape can't reach
/// the terminal of a job that names it, and a name can't turn the rest of
/// a waiting line around.
pub fn shown(name: &str) -> String {
    name.chars()
        .map(|c| {
            let turns = matches!(c, '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}');
            if c.is_control() || turns { ' ' } else { c }
        })
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

/// How a waiting line names a holder the kernel names nobody for, the
/// one wording each line that says so uses.
pub const NOBODY: &str = "a holder the kernel doesn't name";

/// Who holds a turn, as a waiting line says them, and why they can't be
/// named where they can't, said after the line rather than inside it.
#[derive(Debug, PartialEq, Eq)]
pub struct Holder {
    pub who: String,
    pub unnamed_as: Option<String>,
}

impl Holder {
    /// A holder the kernel names nobody for, looked at twice.
    pub fn nobody() -> Self {
        Self {
            who: NOBODY.into(),
            unnamed_as: None,
        }
    }

    /// Why a job that waits for nobody isn't started, and the line one
    /// that waits says, with why the holder can't be named at the end.
    pub fn said(&self) -> (String, String) {
        let who = &self.who;
        let because = self
            .unnamed_as
            .as_ref()
            .map_or_else(String::new, |why| format!(", as {why}"));
        (
            format!("another heavy job is running, {who}, and only one runs at a time{because}"),
            format!("waiting for the turn, which {who} has{because}"),
        )
    }
}

/// Who holds `lock` in `runtime`, as the kernel names them: the note's
/// job when its process is among them, or the processes themselves. It's
/// nothing when the kernel names no holder this process can see, as when
/// the holder let go between the caller's look and this read, so the
/// caller looks again before it says anyone has the turn.
pub fn holder_of(runtime: &Path, lock: &File) -> Option<Holder> {
    match lock_holders(lock) {
        Ok(holders) if holders.is_empty() => None,
        Ok(holders) => Some(Holder {
            who: named(runtime, &holders),
            unnamed_as: None,
        }),
        Err(why) => Some(Holder {
            who: "a holder the kernel can't name".into(),
            unnamed_as: Some(why),
        }),
    }
}

/// A wait's looks at who holds the turn: a look naming nobody is taken
/// again at once, since its holder may have let go between the look and
/// the kernel's word, and only a second in a row is said. Any look that
/// names someone, or is said, starts the next pair afresh, so each wait
/// loop says the same looks the same way.
#[derive(Debug, Default)]
pub struct Looks {
    looked_again: bool,
}

impl Looks {
    /// Whether a look that named nobody is taken again rather than said.
    pub fn again(&mut self) -> bool {
        self.looked_again = !self.looked_again;
        self.looked_again
    }

    /// That a look named someone, or was said, so the next naming nobody
    /// is taken again.
    pub fn named(&mut self) {
        self.looked_again = false;
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
        [] => NOBODY.into(),
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
    Some((shown(name), pid, minutes))
}

/// Who holds `lock`, as far as the kernel says.
#[derive(Debug, PartialEq)]
pub enum Held {
    /// This process, or one of its parents, as the holder named.
    Above(String),
    /// Another process, as named, and none of this process's parents.
    Elsewhere(String),
    /// Whether a parent holds it couldn't be told, as `why` says, and
    /// `holder` holds it, as the kernel named them.
    Unread { why: String, holder: String },
    /// The kernel named no holder this process can see, as given: one
    /// that let go between the look and the read, one in a pid namespace
    /// this process can't see, or a file id the kernel names otherwise.
    Unnamed(String),
}

/// Where a walk up from a process found a holder.
#[derive(Debug, PartialEq)]
enum Walked {
    /// At this process, the start or one of its parents.
    Above(i32),
    /// Nowhere above it: the walk reached the first process.
    Elsewhere,
    /// It couldn't tell, as given.
    Unread(String),
}

/// Walks up from `start` by `parent` until it meets one of `holders`. The
/// kernel names only live holders this process can see, so even pid 1, a
/// pid namespace's first process, is asked; a parent of 0 or less ends
/// the walk there. A chain longer than any real one is a loop read
/// mid-change, and a parent that can't be read is said, never taken for
/// the walk's end.
fn walk(start: i32, holders: &[i32], parent: impl Fn(i32) -> Option<i32>) -> Walked {
    let mut at = start;
    for _ in 0..4096 {
        if holders.contains(&at) {
            return Walked::Above(at);
        }
        if at <= 1 {
            return Walked::Elsewhere;
        }
        let Some(next) = parent(at) else {
            return Walked::Unread(format!("process {at}'s parent can't be read"));
        };
        at = next;
    }
    Walked::Unread("this process's parents never ended".into())
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
            // Only the ids looked for can tell the causes apart.
            return Held::Unnamed(format!(
                "the kernel names no holder of its lock this process can see, looking for {}",
                file_ids(lock).map_or_else(|why| why, |ids| ids.join(" or "))
            ));
        }
        Ok(holders) => holders,
        Err(why) => {
            return Held::Unread {
                why,
                holder: "a holder the kernel can't name".into(),
            };
        }
    };
    let Ok(own) = i32::try_from(std::process::id()) else {
        return Held::Unread {
            why: "this process's number can't be read".into(),
            holder: named(runtime, &holders),
        };
    };
    let parent = |at: i32| ahead::stat(&at.to_string()).and_then(|stat| stat.get(1)?.parse().ok());
    match walk(own, &holders, parent) {
        Walked::Above(at) => Held::Above(
            noted(runtime)
                .filter(|(_, pid, _)| *pid == at)
                .map_or_else(|| format!("process {at}"), |noted| described(&noted)),
        ),
        Walked::Elsewhere => Held::Elsewhere(named(runtime, &holders)),
        Walked::Unread(why) => Held::Unread {
            why,
            holder: named(runtime, &holders),
        },
    }
}

/// The processes holding a flock on `lock`, as the kernel's `/proc/locks`
/// says: lines `ID: FLOCK ADVISORY WRITE PID MAJ:MIN:INODE` for the lock's
/// file, a waiter's lines being marked `->`. Each process is named once,
/// since a read torn across the kernel's pages may say a line twice.
fn lock_holders(lock: &File) -> Result<Vec<i32>, String> {
    let ids = file_ids(lock)?;
    let locks = kernel_locks()?;
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

/// `/proc/locks` as the kernel gives it, read to its end. The kernel
/// gives it a page a read, whatever is asked for, and lets the locks
/// change between pages, so a line can be said twice or not at all: what
/// one look reads is good for that look alone, and a look that finds no
/// holder is taken again ([`Looks`]) before it's said.
fn kernel_locks() -> Result<String, String> {
    fs::read_to_string("/proc/locks").map_err(|error| format!("/proc/locks can't be read: {error}"))
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
fn file_ids(lock: &File) -> Result<Vec<String>, String> {
    use std::os::unix::fs::MetadataExt;
    let held = lock
        .metadata()
        .map_err(|error| format!("the lock can't be read: {error}"))?;
    let mut devices = Vec::new();
    devices.extend(mounted_device(lock));
    devices.push(stat_device(held.dev()));
    devices.dedup();
    Ok(devices
        .into_iter()
        .map(|(major, minor)| format!("{major:02x}:{minor:02x}:{}", held.ino()))
        .collect())
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
        let ids = file_ids(&lock).unwrap();
        let said = locks_naming(&ids);
        let by_mount = locks_naming(&ids[..1]);
        let mounted = mounted_device(&lock);
        let above = held_above(&folder, &lock);
        let other = File::create(folder.join("other")).unwrap();
        let unheld = lock_holders(&other);
        let unread = held_above(&folder, &other);
        let nobody = holder_of(&folder, &other);
        drop(lock);
        fs::remove_dir_all(&folder).unwrap();
        let own = i32::try_from(std::process::id()).unwrap();
        assert_eq!(
            holders,
            Ok(vec![own]),
            "the kernel's word on {ids:?} was misread: {said:?}"
        );
        let Some((major, minor)) = mounted else {
            panic!("the lock's mount gave no device");
        };
        assert!(
            ids[0].starts_with(&format!("{major:02x}:{minor:02x}:")),
            "the first id looked for, {}, isn't the mount's device",
            ids[0]
        );
        assert!(
            !by_mount.is_empty(),
            "the mount's id alone, {}, doesn't name the lock: {said:?}",
            ids[0]
        );
        assert_eq!(
            above,
            Held::Above(format!("process {own}")),
            "this process wasn't found holding its own lock"
        );
        assert_eq!(unheld, Ok(vec![]), "a file nobody locked was read as held");
        assert_eq!(
            nobody, None,
            "a file nobody locked was said to have a holder"
        );
        assert!(
            matches!(&unread, Held::Unnamed(why) if why.contains("no holder")),
            "a lock nobody holds wasn't said to be unexplained: {unread:?}"
        );
    }

    // Why a holder can't be named is said after the line, never inside
    // it, so the line still reads as a sentence.
    #[test]
    fn a_holder_that_cant_be_named_is_said_with_why_at_the_end() {
        let unread = Holder {
            who: "a holder the kernel can't name".into(),
            unnamed_as: Some("/proc/locks can't be read: denied".into()),
        };
        assert_eq!(
            unread.said().1,
            "waiting for the turn, which a holder the kernel can't name has, as /proc/locks \
             can't be read: denied",
            "why the holder can't be named was put inside the line"
        );
        assert_eq!(
            Holder::nobody().said().0,
            format!("another heavy job is running, {NOBODY}, and only one runs at a time"),
            "a holder nobody names was said otherwise"
        );
    }

    // Of looks naming nobody, one alone is taken again and a second in a
    // row is said, and any look naming someone, between, starts afresh.
    #[test]
    fn one_look_naming_nobody_is_taken_again_and_a_second_said() {
        let mut looks = Looks::default();
        assert!(looks.again(), "a first look naming nobody was said");
        assert!(
            !looks.again(),
            "a second look in a row naming nobody wasn't said"
        );
        looks.named();
        assert!(
            looks.again(),
            "a look naming nobody after one said wasn't taken again"
        );
        looks.named();
        assert!(
            looks.again(),
            "a look naming nobody after one naming someone wasn't taken again"
        );
    }

    // A walk up the parents finds a holder wherever it is, the first
    // process included, and says rather than guesses when a parent can't
    // be read or the parents run round in a loop.
    #[test]
    fn a_walk_finds_a_holder_above_or_says_why_it_cant() {
        let parents = |at: i32| match at {
            9 => Some(8),
            8 => Some(1),
            5 => Some(6),
            6 => Some(5),
            4 => Some(0),
            _ => None,
        };
        assert_eq!(
            walk(9, &[1], parents),
            Walked::Above(1),
            "a holder at the first process wasn't found"
        );
        assert_eq!(
            walk(9, &[8], parents),
            Walked::Above(8),
            "a parent holding it wasn't found"
        );
        assert_eq!(
            walk(9, &[7], parents),
            Walked::Elsewhere,
            "a holder no parent is was taken for one"
        );
        assert_eq!(
            walk(4, &[7], parents),
            Walked::Elsewhere,
            "a parent of 0 didn't end the walk"
        );
        assert_eq!(
            walk(3, &[7], parents),
            Walked::Unread("process 3's parent can't be read".into()),
            "a parent that can't be read was taken for the walk's end"
        );
        assert_eq!(
            walk(5, &[7], parents),
            Walked::Unread("this process's parents never ended".into()),
            "parents in a loop weren't said"
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
        for name in ["x\t1", "x\n1", "x\u{1b}1", "x\u{202e}1", "x\u{2066}1", ""] {
            note(&folder, name);
            read.push(noted(&folder));
        }
        // A note written by anything of the account but this program is
        // made safe as it's read.
        fs::write(
            folder.join("holder"),
            format!("x\u{1b}[2J\u{202e}1\t{own}\t0\n"),
        )
        .unwrap();
        let planted = noted(&folder).map(|(name, _, _)| name);
        fs::remove_dir_all(&folder).unwrap();
        assert_eq!(
            planted.as_deref(),
            Some("x [2J 1"),
            "a planted note's name was said as written"
        );
        assert_eq!(
            read,
            [
                Some(("x 1".to_owned(), own, 0)),
                Some(("x 1".to_owned(), own, 0)),
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
        assert_eq!(
            nobody, "a holder the kernel doesn't name",
            "no holder at all was said as one"
        );
    }
}
