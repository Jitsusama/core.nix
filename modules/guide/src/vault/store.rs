//! Where a vault's files are kept: the stick's FAT partition, or a
//! directory, which a vault is staged in and the tests use.
//!
//! Every file is at the top, with no directories. fatfs 0.3.6 writes a
//! long name before every entry, `.` and `..` included, which puts a new
//! directory's dot entries out of the slots FAT gives them; fsck.fat then
//! "repairs" them by renaming. Files alone it writes as FAT allows.

use std::fs;
use std::io::{Read, Write};
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
use std::path::PathBuf;

use crate::Fault;

/// How the guide opens a FAT partition: with files stamped by the clock.
/// Left to itself fatfs 0.3.6, without its chrono feature, stamps every
/// file with a zero date, which reads as month 0, day 0, a date DOS
/// doesn't have, though its docs promise 1980-01-01.
pub fn fat_options() -> fatfs::FsOptions {
    fatfs::FsOptions::new().time_provider(&CLOCK)
}

static CLOCK: Clock = Clock;

/// The time now, in UTC, since a stick has no time zone of its own.
#[derive(Debug)]
struct Clock;

impl fatfs::TimeProvider for Clock {
    fn get_current_date(&self) -> fatfs::Date {
        self.get_current_date_time().date
    }

    fn get_current_date_time(&self) -> fatfs::DateTime {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |since| since.as_secs());
        dos_time(now)
    }
}

/// Seconds since 1970 as a DOS date and time, held to the years FAT can
/// say, 1980 to 2107. The calendar is Howard Hinnant's `civil_from_days`.
fn dos_time(seconds: u64) -> fatfs::DateTime {
    const FIRST: u64 = 315_532_800; // 1980-01-01
    const LAST: u64 = 4_354_819_199; // 2107-12-31 23:59:59
    let seconds = seconds.clamp(FIRST, LAST);
    let (days, time) = (seconds / 86_400, seconds % 86_400);
    let z = days + 719_468;
    let era = z / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + u64::from(month <= 2);
    let small = |n: u64| u16::try_from(n).unwrap_or(u16::MAX);
    fatfs::DateTime {
        date: fatfs::Date {
            year: small(year),
            month: small(month),
            day: small(day),
        },
        time: fatfs::Time {
            hour: small(time / 3600),
            min: small(time / 60 % 60),
            sec: small(time % 60),
            millis: 0,
        },
    }
}

pub trait Store {
    /// A new file, replacing one of that name. Names have no `/`.
    fn create(&mut self, name: &str) -> Result<Box<dyn Write + '_>, Fault>;

    /// A file's bytes as they're read.
    fn open(&mut self, name: &str) -> Result<Box<dyn Read + '_>, Fault>;

    /// Whether there's a file of that name.
    fn has(&mut self, name: &str) -> Result<bool, Fault>;

    /// The names of every file it keeps.
    fn names(&mut self) -> Result<Vec<String>, Fault>;

    /// Puts the file `from` in place of `to`, whatever `to` held: in one
    /// rename where the store can, else `to` taken away just before.
    fn replace(&mut self, from: &str, to: &str) -> Result<(), Fault>;
}

/// A vault in a directory, every file its owner's alone.
pub struct Directory {
    root: PathBuf,
}

impl Directory {
    pub fn new(root: impl Into<PathBuf>) -> Directory {
        Directory { root: root.into() }
    }
}

fn unkept(path: &std::path::Path) -> impl Fn(std::io::Error) -> Fault + '_ {
    move |why| Fault::Unkept {
        path: path.display().to_string(),
        why: why.to_string(),
    }
}

/// A name a store keeps a file by: one part, never a path.
fn flat(name: &str) -> Result<&str, Fault> {
    if name.is_empty() || name.contains('/') || name == "." || name == ".." {
        Err(Fault::Unkept {
            path: name.into(),
            why: "a vault keeps its files by name, not by path".into(),
        })
    } else {
        Ok(name)
    }
}

impl Store for Directory {
    fn create(&mut self, name: &str) -> Result<Box<dyn Write + '_>, Fault> {
        let path = self.root.join(flat(name)?);
        fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(&self.root)
            .map_err(unkept(&self.root))?;
        let file = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(&path)
            .map_err(unkept(&path))?;
        Ok(Box::new(file))
    }

    fn open(&mut self, name: &str) -> Result<Box<dyn Read + '_>, Fault> {
        let path = self.root.join(flat(name)?);
        Ok(Box::new(fs::File::open(&path).map_err(unkept(&path))?))
    }

    fn has(&mut self, name: &str) -> Result<bool, Fault> {
        let path = self.root.join(flat(name)?);
        match fs::metadata(&path) {
            Ok(found) => Ok(found.is_file()),
            Err(why) if why.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(why) => Err(unkept(&path)(why)),
        }
    }

    fn names(&mut self) -> Result<Vec<String>, Fault> {
        let entries = match fs::read_dir(&self.root) {
            Ok(entries) => entries,
            Err(why) if why.kind() == std::io::ErrorKind::NotFound => return Ok(vec![]),
            Err(why) => return Err(Fault::unreadable(self.root.display().to_string(), why)),
        };
        let mut names = vec![];
        for entry in entries {
            let entry =
                entry.map_err(|why| Fault::unreadable(self.root.display().to_string(), why))?;
            if entry.file_type().is_ok_and(|kind| kind.is_file()) {
                names.push(entry.file_name().to_string_lossy().into_owned());
            }
        }
        Ok(names)
    }

    fn replace(&mut self, from: &str, to: &str) -> Result<(), Fault> {
        let to = self.root.join(flat(to)?);
        fs::rename(self.root.join(flat(from)?), &to).map_err(unkept(&to))
    }
}

/// A vault on a FAT filesystem, as the stick's last partition holds it.
pub struct Fat<IO: fatfs::ReadWriteSeek> {
    filesystem: fatfs::FileSystem<IO>,
}

impl<IO: fatfs::ReadWriteSeek> Fat<IO> {
    /// Formats the partition as FAT32, labelled as the guide's vault.
    pub fn format(mut partition: IO) -> Result<Fat<IO>, Fault> {
        fatfs::format_volume(
            &mut partition,
            fatfs::FormatVolumeOptions::new()
                .fat_type(fatfs::FatType::Fat32)
                .volume_label(*b"GUIDE VAULT"),
        )
        .map_err(|why| Fault::Unkept {
            path: "the vault's partition".into(),
            why: why.to_string(),
        })?;
        Fat::open(partition)
    }

    pub fn open(partition: IO) -> Result<Fat<IO>, Fault> {
        Ok(Fat {
            filesystem: fatfs::FileSystem::new(partition, fat_options())
                .map_err(|why| Fault::unreadable("the vault's partition", why))?,
        })
    }

    /// Writes everything back to the partition, which a vault has to be
    /// before the stick is read back or pulled. Counting the free clusters
    /// first records them, which a fresh FAT32 leaves unknown.
    pub fn close(self) -> Result<(), Fault> {
        let unkept = |why: std::io::Error| Fault::Unkept {
            path: "the vault's partition".into(),
            why: why.to_string(),
        };
        self.filesystem.stats().map_err(unkept)?;
        self.filesystem.unmount().map_err(unkept)
    }
}

impl<IO: fatfs::ReadWriteSeek> Store for Fat<IO> {
    fn create(&mut self, name: &str) -> Result<Box<dyn Write + '_>, Fault> {
        let fault = |why: std::io::Error| Fault::Unkept {
            path: format!("{name} in the vault"),
            why: why.to_string(),
        };
        let mut created = self
            .filesystem
            .root_dir()
            .create_file(flat(name)?)
            .map_err(fault)?;
        created.truncate().map_err(fault)?;
        Ok(Box::new(created))
    }

    fn open(&mut self, name: &str) -> Result<Box<dyn Read + '_>, Fault> {
        Ok(Box::new(
            self.filesystem
                .root_dir()
                .open_file(flat(name)?)
                .map_err(|why| Fault::unreadable(format!("{name} in the vault"), why))?,
        ))
    }

    fn has(&mut self, name: &str) -> Result<bool, Fault> {
        match self.filesystem.root_dir().open_file(flat(name)?) {
            Ok(_) => Ok(true),
            Err(why) if why.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(why) => Err(Fault::unreadable(format!("{name} in the vault"), why)),
        }
    }

    fn names(&mut self) -> Result<Vec<String>, Fault> {
        let mut names = vec![];
        for entry in self.filesystem.root_dir().iter() {
            let entry = entry.map_err(|why| Fault::unreadable("the vault", why))?;
            if entry.is_file() {
                names.push(entry.file_name());
            }
        }
        Ok(names)
    }

    /// FAT has no rename over a file, so `to` goes first, and a reader that
    /// finds no `to` has to look for `from`.
    fn replace(&mut self, from: &str, to: &str) -> Result<(), Fault> {
        let fault = |why: std::io::Error| Fault::Unkept {
            path: format!("{to} in the vault"),
            why: why.to_string(),
        };
        let root = self.filesystem.root_dir();
        if let Err(why) = root.remove(flat(to)?)
            && why.kind() != std::io::ErrorKind::NotFound
        {
            return Err(fault(why));
        }
        root.rename(flat(from)?, &root, to).map_err(fault)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(store: &mut dyn Store, name: &str) -> Vec<u8> {
        let mut bytes = vec![];
        store.open(name).unwrap().read_to_end(&mut bytes).unwrap();
        bytes
    }

    #[test]
    fn a_file_replaces_another_on_fat_which_has_no_rename_over_one() {
        let mut fat = Fat::format(std::io::Cursor::new(vec![0u8; 40 << 20])).unwrap();
        for (name, bytes) in [("index.age", b"old"), ("index-new.age", b"new")] {
            let mut file = fat.create(name).unwrap();
            file.write_all(bytes).and_then(|()| file.flush()).unwrap();
        }
        fat.replace("index-new.age", "index.age").unwrap();
        assert_eq!(read(&mut fat, "index.age"), b"new");
        assert!(!fat.has("index-new.age").unwrap());
        // With nothing to replace, it's only named.
        let mut file = fat.create("index-new.age").unwrap();
        file.write_all(b"newer")
            .and_then(|()| file.flush())
            .unwrap();
        drop(file);
        fat.replace("index-new.age", "other.age").unwrap();
        assert_eq!(read(&mut fat, "other.age"), b"newer");
    }

    fn said(time: fatfs::DateTime) -> String {
        let (d, t) = (time.date, time.time);
        format!(
            "{}-{:02}-{:02} {:02}:{:02}:{:02}",
            d.year, d.month, d.day, t.hour, t.min, t.sec
        )
    }

    #[test]
    fn the_clock_says_a_date_fat_can_hold() {
        assert_eq!(said(dos_time(951_868_799)), "2000-02-29 23:59:59");
        assert_eq!(said(dos_time(1_792_657_945)), "2026-10-22 08:32:25");
        assert_eq!(said(dos_time(4_354_819_198)), "2107-12-31 23:59:58");
        assert_eq!(said(dos_time(0)), "1980-01-01 00:00:00");
        assert_eq!(said(dos_time(u64::MAX)), "2107-12-31 23:59:59");
    }

    #[test]
    fn a_file_written_to_fat_carries_a_real_date() {
        let mut fat = Fat::format(std::io::Cursor::new(vec![0u8; 40 << 20])).unwrap();
        let mut file = fat.create("index.age").unwrap();
        file.write_all(b"index")
            .and_then(|()| file.flush())
            .unwrap();
        drop(file);
        let entry = fat
            .filesystem
            .root_dir()
            .iter()
            .map(Result::unwrap)
            .find(|entry| entry.file_name() == "index.age")
            .unwrap();
        let date = entry.modified().date;
        assert!(
            date.year >= 2026 && (1..=12).contains(&date.month),
            "{date:?}"
        );
    }
}
