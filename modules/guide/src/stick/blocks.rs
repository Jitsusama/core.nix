//! Reaching a stick a partition at a time, in whole aligned blocks. macOS's
//! raw disks refuse any read or write that isn't sector aligned, and the
//! FAT library writes 32-byte directory entries, so every write goes through
//! cached blocks, read whole, changed and written back whole.
//!
//! The cache holds many blocks, not one: FAT puts a cluster's data, its
//! entry in each of the two tables and the file's directory entry in
//! different blocks, so one block held would be read and written back for
//! nearly every cluster, and every new file would read the whole directory
//! off the stick again.

use std::collections::BTreeMap;
use std::io::{self, Read, Seek, SeekFrom, Write};

use super::gpt::SECTOR;

/// What a stick is reached through: a disk or an image file.
pub trait Disk: Read + Write + Seek {}
impl<T: Read + Write + Seek> Disk for T {}

/// 64 KiB: whole sectors on every stick, and large enough that a file
/// written in order costs one write per block. A partition's last block is
/// shorter when it ends short of one, still whole sectors.
pub const BLOCK: u64 = 64 * 1024;

/// 512 blocks, 32 MiB: the tables' blocks a vault's files are written
/// through and a directory of thousands of files, with room to spare for
/// data passing through in order.
pub const CACHED: usize = 512;

/// A block held: its bytes, whether they changed since they were read, and
/// when it was last reached, for the least lately reached to go first.
struct Held {
    bytes: Vec<u8>,
    changed: bool,
    reached: u64,
}

/// One partition of a disk, from its own byte 0, reached in whole blocks.
pub struct Partition<'d, D: Disk> {
    disk: &'d mut D,
    start: u64,
    bytes: u64,
    position: u64,
    /// The blocks held, by their offsets in the partition.
    held: BTreeMap<u64, Held>,
    /// How many times a block has been reached, as a clock.
    clock: u64,
}

impl<'d, D: Disk> Partition<'d, D> {
    /// The partition at `start` bytes into the disk, `bytes` long, both in
    /// whole sectors.
    pub fn new(disk: &'d mut D, start: u64, bytes: u64) -> io::Result<Partition<'d, D>> {
        if !start.is_multiple_of(SECTOR) || !bytes.is_multiple_of(SECTOR) || bytes == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "a partition has to start and end on a sector",
            ));
        }
        Ok(Partition {
            disk,
            start,
            bytes,
            position: 0,
            held: BTreeMap::new(),
            clock: 0,
        })
    }

    pub fn bytes(&self) -> u64 {
        self.bytes
    }

    /// How long the block at `block` is: whole, or what's left.
    fn length(&self, block: u64) -> usize {
        BLOCK.min(self.bytes - block) as usize
    }

    /// The block `at` lies in, read off the disk unless it's held, or unless
    /// `whole` says it's about to be written over entirely.
    fn hold(&mut self, at: u64, whole: bool) -> io::Result<&mut Held> {
        let block = at / BLOCK * BLOCK;
        self.clock += 1;
        if !self.held.contains_key(&block) {
            if self.held.len() >= CACHED {
                self.let_go()?;
            }
            let length = self.length(block);
            let mut bytes = vec![0; length];
            if !whole {
                self.disk.seek(SeekFrom::Start(self.start + block))?;
                self.disk.read_exact(&mut bytes)?;
            }
            self.held.insert(
                block,
                Held {
                    bytes,
                    changed: false,
                    reached: 0,
                },
            );
        }
        let held = self.held.get_mut(&block).expect("held just now");
        held.reached = self.clock;
        Ok(held)
    }

    /// Lets go of the block least lately reached, written back if it changed.
    fn let_go(&mut self) -> io::Result<()> {
        let Some(block) = self
            .held
            .iter()
            .min_by_key(|(_, held)| held.reached)
            .map(|(block, _)| *block)
        else {
            return Ok(());
        };
        self.write_back(block)?;
        self.held.remove(&block);
        Ok(())
    }

    fn write_back(&mut self, block: u64) -> io::Result<()> {
        if let Some(held) = self.held.get_mut(&block)
            && held.changed
        {
            self.disk.seek(SeekFrom::Start(self.start + block))?;
            self.disk.write_all(&held.bytes)?;
            held.changed = false;
        }
        Ok(())
    }

    /// Writes back every block that changed, in the order they lie on the
    /// disk.
    fn write_back_all(&mut self) -> io::Result<()> {
        let blocks: Vec<u64> = self.held.keys().copied().collect();
        for block in blocks {
            self.write_back(block)?;
        }
        Ok(())
    }
}

impl<D: Disk> Read for Partition<'_, D> {
    fn read(&mut self, into: &mut [u8]) -> io::Result<usize> {
        if self.position >= self.bytes || into.is_empty() {
            return Ok(0);
        }
        let within = (self.position % BLOCK) as usize;
        let count = into
            .len()
            .min(self.length(self.position - within as u64) - within);
        let held = self.hold(self.position, false)?;
        into[..count].copy_from_slice(&held.bytes[within..within + count]);
        self.position += count as u64;
        Ok(count)
    }
}

impl<D: Disk> Write for Partition<'_, D> {
    fn write(&mut self, from: &[u8]) -> io::Result<usize> {
        if from.is_empty() {
            return Ok(0);
        }
        if self.position >= self.bytes {
            return Err(io::Error::new(
                io::ErrorKind::WriteZero,
                "past the partition's end",
            ));
        }
        let within = (self.position % BLOCK) as usize;
        let length = self.length(self.position - within as u64);
        let count = from.len().min(length - within);
        // A whole block replaces what's there, so it needn't be read.
        let held = self.hold(self.position, within == 0 && count == length)?;
        held.bytes[within..within + count].copy_from_slice(&from[..count]);
        held.changed = true;
        self.position += count as u64;
        Ok(count)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.write_back_all()?;
        self.disk.flush()
    }
}

impl<D: Disk> Seek for Partition<'_, D> {
    fn seek(&mut self, to: SeekFrom) -> io::Result<u64> {
        let position = match to {
            SeekFrom::Start(at) => Some(at),
            SeekFrom::End(by) => self.bytes.checked_add_signed(by),
            SeekFrom::Current(by) => self.position.checked_add_signed(by),
        }
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "before the partition"))?;
        self.position = position;
        Ok(position)
    }
}

/// A partition whose flushes wait for its owner's: FAT flushes its disk
/// each time it closes a file, which would write back every table and
/// directory block once per file. The owner flushes the partition itself
/// once the file system is closed.
pub struct Unflushed<'p, P: Disk>(pub &'p mut P);

impl<P: Disk> Read for Unflushed<'_, P> {
    fn read(&mut self, into: &mut [u8]) -> io::Result<usize> {
        self.0.read(into)
    }
}

impl<P: Disk> Write for Unflushed<'_, P> {
    fn write(&mut self, from: &[u8]) -> io::Result<usize> {
        self.0.write(from)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl<P: Disk> Seek for Unflushed<'_, P> {
    fn seek(&mut self, to: SeekFrom) -> io::Result<u64> {
        self.0.seek(to)
    }
}

impl<D: Disk> Drop for Partition<'_, D> {
    fn drop(&mut self) {
        // A caller that cares flushes and sees the error; this only keeps
        // one that forgot from losing what's held.
        let _ = self.write_back_all();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    /// A disk that refuses anything but whole sectors, as a raw disk does.
    struct Strict(Cursor<Vec<u8>>);

    impl Read for Strict {
        fn read(&mut self, into: &mut [u8]) -> io::Result<usize> {
            assert_eq!(self.0.position() % SECTOR, 0, "an unaligned read");
            assert_eq!(into.len() as u64 % SECTOR, 0, "a partial read");
            self.0.read(into)
        }
    }

    impl Write for Strict {
        fn write(&mut self, from: &[u8]) -> io::Result<usize> {
            assert_eq!(self.0.position() % SECTOR, 0, "an unaligned write");
            assert_eq!(from.len() as u64 % SECTOR, 0, "a partial write");
            self.0.write(from)
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    impl Seek for Strict {
        fn seek(&mut self, to: SeekFrom) -> io::Result<u64> {
            self.0.seek(to)
        }
    }

    #[test]
    fn small_writes_reach_the_disk_in_whole_blocks() {
        let mut disk = Strict(Cursor::new(vec![0xaa; 4 * BLOCK as usize]));
        {
            let mut partition = Partition::new(&mut disk, BLOCK, 2 * BLOCK).unwrap();
            partition.seek(SeekFrom::Start(BLOCK - 3)).unwrap();
            partition.write_all(b"across").unwrap();
            partition.seek(SeekFrom::Start(10)).unwrap();
            partition.write_all(b"entry").unwrap();
            partition.flush().unwrap();
        }
        let bytes = disk.0.into_inner();
        let at = BLOCK as usize;
        assert_eq!(&bytes[at + 10..at + 15], b"entry");
        assert_eq!(&bytes[2 * at - 3..2 * at + 3], b"across");
        assert_eq!(bytes[at - 1], 0xaa, "before the partition");
        assert_eq!(bytes[3 * at], 0xaa, "after it");
    }

    #[test]
    fn what_was_written_reads_back_through_another_view() {
        let mut disk = Cursor::new(vec![0; 3 * BLOCK as usize]);
        let message: Vec<u8> = (0..BLOCK as usize + 100).map(|n| n as u8).collect();
        {
            let mut partition = Partition::new(&mut disk, BLOCK, 2 * BLOCK).unwrap();
            partition.seek(SeekFrom::Start(7)).unwrap();
            partition.write_all(&message).unwrap();
            partition.flush().unwrap();
        }
        let mut partition = Partition::new(&mut disk, BLOCK, 2 * BLOCK).unwrap();
        partition.seek(SeekFrom::Start(7)).unwrap();
        let mut back = vec![0; message.len()];
        partition.read_exact(&mut back).unwrap();
        assert_eq!(back, message);
    }

    #[test]
    fn a_short_last_block_is_reached_in_whole_sectors() {
        let bytes = BLOCK + 3 * SECTOR;
        let mut disk = Strict(Cursor::new(vec![0; (bytes + 2 * SECTOR) as usize]));
        {
            let mut partition = Partition::new(&mut disk, SECTOR, bytes).unwrap();
            partition.seek(SeekFrom::End(-4)).unwrap();
            partition.write_all(b"tail").unwrap();
            partition.flush().unwrap();
        }
        let written = disk.0.into_inner();
        let end = (SECTOR + bytes) as usize;
        assert_eq!(&written[end - 4..end], b"tail");
        assert_eq!(written[end], 0, "past the partition");
    }

    /// A disk that counts the bytes read off it and written to it.
    #[derive(Default)]
    struct Counting {
        disk: Cursor<Vec<u8>>,
        read: u64,
        written: u64,
    }

    impl Read for Counting {
        fn read(&mut self, into: &mut [u8]) -> io::Result<usize> {
            let count = self.disk.read(into)?;
            self.read += count as u64;
            Ok(count)
        }
    }

    impl Write for Counting {
        fn write(&mut self, from: &[u8]) -> io::Result<usize> {
            let count = self.disk.write(from)?;
            self.written += count as u64;
            Ok(count)
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    impl Seek for Counting {
        fn seek(&mut self, to: SeekFrom) -> io::Result<u64> {
            self.disk.seek(to)
        }
    }

    #[test]
    fn a_vault_of_many_files_costs_the_disk_about_what_they_hold() {
        use crate::vault::store::{Fat, Store};
        // FAT32's least size, and more.
        let bytes = 1024 * BLOCK;
        let mut disk = Counting {
            disk: Cursor::new(vec![0; bytes as usize]),
            ..Counting::default()
        };
        // As a binary cache is: a thousand files at the top, most small.
        let files: Vec<Vec<u8>> = (0..1000)
            .map(|n| vec![n as u8; 1000 + (n % 7) * 9000])
            .collect();
        let held: u64 = files.iter().map(|file| file.len() as u64).sum();
        {
            let mut partition = Partition::new(&mut disk, 0, bytes).unwrap();
            let mut store = Fat::format(Unflushed(&mut partition)).unwrap();
            for (n, file) in files.iter().enumerate() {
                store
                    .create(&format!("{n:04}.nar.zst"))
                    .unwrap()
                    .write_all(file)
                    .unwrap();
            }
            store.close().unwrap();
            partition.flush().unwrap();
        }
        // Each file takes whole clusters, and the tables and the directory
        // come on top.
        assert!(
            disk.written < 2 * held,
            "{} bytes written for {held} held",
            disk.written
        );
        // A block written in part is read first, once, and not again.
        assert!(
            disk.read < 2 * held,
            "{} bytes read for {held} written",
            disk.read
        );
    }

    #[test]
    fn nothing_is_written_past_the_partitions_end() {
        let mut disk = Cursor::new(vec![0; 3 * BLOCK as usize]);
        let mut partition = Partition::new(&mut disk, 0, BLOCK).unwrap();
        partition.seek(SeekFrom::Start(BLOCK - 2)).unwrap();
        assert_eq!(partition.write(b"abcd").unwrap(), 2);
        assert!(partition.write(b"cd").is_err());
        partition.flush().unwrap();
        drop(partition);
        assert_eq!(disk.into_inner()[BLOCK as usize], 0);
    }
}
