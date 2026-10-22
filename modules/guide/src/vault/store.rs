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

pub trait Store {
    /// A new file, replacing one of that name. Names have no `/`.
    fn create(&mut self, name: &str) -> Result<Box<dyn Write + '_>, Fault>;

    /// A file's bytes as they're read.
    fn open(&mut self, name: &str) -> Result<Box<dyn Read + '_>, Fault>;

    /// Whether there's a file of that name.
    fn has(&mut self, name: &str) -> Result<bool, Fault>;

    /// The names of every file it keeps.
    fn names(&mut self) -> Result<Vec<String>, Fault>;
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
            filesystem: fatfs::FileSystem::new(partition, fatfs::FsOptions::new())
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
}
