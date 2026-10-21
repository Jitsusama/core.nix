//! Reaching a stick a partition at a time, in whole aligned blocks. macOS's
//! raw disks refuse any read or write that isn't sector aligned, and the
//! FAT library writes 32-byte directory entries, so every write goes through
//! one cached block, read whole, changed and written back whole.

use std::io::{self, Read, Seek, SeekFrom, Write};

use super::gpt::SECTOR;

/// What a stick is reached through: a disk or an image file.
pub trait Disk: Read + Write + Seek {}
impl<T: Read + Write + Seek> Disk for T {}

/// 64 KiB: whole sectors on every stick, and large enough that a file
/// written in order costs one write per block. A partition's last block is
/// shorter when it ends short of one, still whole sectors.
pub const BLOCK: u64 = 64 * 1024;

/// One partition of a disk, from its own byte 0, reached in whole blocks.
pub struct Partition<'d, D: Disk> {
    disk: &'d mut D,
    start: u64,
    bytes: u64,
    position: u64,
    /// The block held, by its offset in the partition, and whether it has
    /// changed since it was read.
    held: Option<(u64, bool)>,
    buffer: Vec<u8>,
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
            held: None,
            buffer: vec![0; BLOCK as usize],
        })
    }

    pub fn bytes(&self) -> u64 {
        self.bytes
    }

    /// How long the block at `block` is: whole, or what's left.
    fn length(&self, block: u64) -> usize {
        BLOCK.min(self.bytes - block) as usize
    }

    /// Holds the block `at` lies in, writing back the one held before.
    fn hold(&mut self, at: u64) -> io::Result<()> {
        let block = at / BLOCK * BLOCK;
        if self.held.map(|(held, _)| held) == Some(block) {
            return Ok(());
        }
        self.write_back()?;
        let length = self.length(block);
        self.disk.seek(SeekFrom::Start(self.start + block))?;
        self.disk.read_exact(&mut self.buffer[..length])?;
        self.held = Some((block, false));
        Ok(())
    }

    fn write_back(&mut self) -> io::Result<()> {
        if let Some((block, true)) = self.held {
            let length = self.length(block);
            self.disk.seek(SeekFrom::Start(self.start + block))?;
            self.disk.write_all(&self.buffer[..length])?;
            self.held = Some((block, false));
        }
        Ok(())
    }
}

impl<D: Disk> Read for Partition<'_, D> {
    fn read(&mut self, into: &mut [u8]) -> io::Result<usize> {
        if self.position >= self.bytes || into.is_empty() {
            return Ok(0);
        }
        self.hold(self.position)?;
        let within = (self.position % BLOCK) as usize;
        let count = into
            .len()
            .min(self.length(self.position - within as u64) - within);
        into[..count].copy_from_slice(&self.buffer[within..within + count]);
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
        if within == 0 && count == length {
            // A whole block replaces what's there, so it needn't be read.
            self.write_back()?;
            self.buffer[..count].copy_from_slice(&from[..count]);
            self.held = Some((self.position, true));
        } else {
            self.hold(self.position)?;
            self.buffer[within..within + count].copy_from_slice(&from[..count]);
            if let Some((_, changed)) = &mut self.held {
                *changed = true;
            }
        }
        self.position += count as u64;
        Ok(count)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.write_back()?;
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

impl<D: Disk> Drop for Partition<'_, D> {
    fn drop(&mut self) {
        // A caller that cares flushes and sees the error; this only keeps
        // one that forgot from losing the last block.
        let _ = self.write_back();
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
