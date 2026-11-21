//! A boot's ring: a header on a page of its own, then a fixed number of
//! slots, each holding one second's record, written over in turn, so the
//! file never grows and the newest seconds before a cut are always in it.
//!
//! What it costs the disk decides how it's written. Flushing a record
//! with fdatasync makes btrfs log the file's changed time at each flush: on
//! the XPS 14, 36 KB written a second, about 3 GB a day, copy-on-write or
//! not, for 128 bytes of reading. So the file's blocks are made and
//! written once, when the ring is made, and with copy-on-write off, so each
//! record after is written where its slot already lies on the disk, and
//! only that page is sent to it with `sync_file_range`: 4 KB a second,
//! measured the same way over two minutes. The file's times still change
//! with each write, but only in memory: they reach the disk with the
//! filesystem's own commit every 30 seconds, which the two minutes took in,
//! and a later boot needs none of them to read the records. Where the
//! filesystem can't turn copy-on-write off, the page is still sent; a
//! record may then be lost to a cut until the filesystem commits, never
//! read wrong, since each carries its own CRC. Only btrfs has it here, so
//! only btrfs is asked.
//!
//! A cut while the disk writes a page can garble the whole page, so
//! seconds that follow each other go to different pages: a garbled page
//! loses one second in every nineteen, never the run of seconds before the
//! cut.

use crate::record::{self, Record, crc32};
use std::fs::{File, OpenOptions};
use std::io;
use std::os::fd::AsRawFd;
use std::os::unix::fs::FileExt;
use std::path::Path;

/// What opens a ring's header.
const MAGIC: [u8; 4] = *b"LSH1";
/// The layout its header and records are in.
const VERSION: u32 = 1;
/// How many bytes the header takes: one record's room.
const HEADER: usize = record::SIZE;
/// How many bytes a page the disk is sent holds, and where the slots
/// begin, so the header never shares a page with a record.
const PAGE: usize = 4096;
/// How many records a page holds.
const PER_PAGE: u32 = 32;
/// The longest rest state's name kept, in bytes.
const NAME: usize = 8;
/// Where the header's CRC begins.
const CRC_AT: usize = HEADER - 4;
/// The inode flag that turns copy-on-write off, as linux/fs.h has it.
const FS_NOCOW_FL: libc::c_int = 0x0080_0000;

/// How many seconds a ring keeps: nineteen pages of them, ten minutes and
/// eight seconds, in 80 KB.
pub const SLOTS: u32 = 19 * PER_PAGE;

/// What a ring says of itself and of the machine it was made on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Header {
    /// How many records it holds before it writes over the oldest.
    pub slots: u32,
    /// When the boot began, in seconds since 1970, as the kernel said.
    pub began: i64,
    /// How many processors the rest states' times are summed over.
    pub processors: u16,
    /// Each rest state's name, in the order the records give their times.
    pub rests: Vec<String>,
}

impl Header {
    fn encode(&self) -> [u8; HEADER] {
        let mut bytes = [0; HEADER];
        bytes[..4].copy_from_slice(&MAGIC);
        bytes[4..8].copy_from_slice(&VERSION.to_le_bytes());
        bytes[8..12].copy_from_slice(&u32::try_from(record::SIZE).unwrap_or(0).to_le_bytes());
        bytes[12..16].copy_from_slice(&self.slots.to_le_bytes());
        bytes[16..24].copy_from_slice(&self.began.to_le_bytes());
        bytes[24..26].copy_from_slice(&self.processors.to_le_bytes());
        let rests = self.rests.len().min(record::RESTS);
        bytes[26] = u8::try_from(rests).unwrap_or(0);
        for (at, name) in self.rests.iter().take(rests).enumerate() {
            let name = &name.as_bytes()[..name.len().min(NAME)];
            let start = 28 + at * NAME;
            bytes[start..start + name.len()].copy_from_slice(name);
        }
        let crc = crc32(&bytes[..CRC_AT]);
        bytes[CRC_AT..].copy_from_slice(&crc.to_le_bytes());
        bytes
    }

    fn decode(bytes: &[u8; HEADER]) -> Option<Self> {
        let word = |at: usize| u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap_or([0; 4]));
        if bytes[..4] != MAGIC
            || word(CRC_AT) != crc32(&bytes[..CRC_AT])
            || word(4) != VERSION
            || usize::try_from(word(8)).ok()? != record::SIZE
            || word(12) == 0
        {
            return None;
        }
        let rests = usize::from(bytes[26]).min(record::RESTS);
        let rests = (0..rests)
            .map(|at| {
                let name = &bytes[28 + at * NAME..28 + (at + 1) * NAME];
                let end = name.iter().position(|byte| *byte == 0).unwrap_or(NAME);
                String::from_utf8_lossy(&name[..end]).into_owned()
            })
            .collect();
        Some(Self {
            slots: word(12),
            began: i64::from_le_bytes(bytes[16..24].try_into().ok()?),
            processors: u16::from_le_bytes([bytes[24], bytes[25]]),
            rests,
        })
    }
}

/// Where a record lies in the file. Where the slots fill whole pages, each
/// second goes to the page after the last one's, so the seconds on a page
/// lie a page count apart; a ring of fewer, as the tests make, is in turn.
fn slot_at(sequence: u32, slots: u32) -> u64 {
    let size = u64::try_from(record::SIZE).unwrap_or(u64::MAX);
    let turn = sequence % slots;
    let slot = if slots.is_multiple_of(PER_PAGE) {
        let pages = slots / PER_PAGE;
        (turn % pages) * PER_PAGE + turn / pages
    } else {
        turn
    };
    u64::try_from(PAGE).unwrap_or(u64::MAX) + u64::from(slot) * size
}

/// How long a ring of `slots` is, made whole.
fn length(slots: u32) -> u64 {
    u64::try_from(PAGE + usize::try_from(slots).unwrap_or(0) * record::SIZE).unwrap_or(0)
}

/// A ring open to write into.
pub struct Ring {
    file: File,
    slots: u32,
    next: u32,
}

impl Ring {
    /// The boot's ring at `path`, going on after its newest record, or
    /// made anew where there is none or it was made to another layout.
    pub fn open(path: &Path, header: &Header) -> io::Result<Self> {
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(path)?;
        held(&file)?;
        // A ring cut short while being made is made again, so no record
        // is written past what was made whole.
        if let Ok((kept, records)) = read(path)
            && kept.slots == header.slots
            && file.metadata()?.len() == length(kept.slots)
        {
            let next = records.last().map_or(0, |newest| newest.sequence + 1);
            return Ok(Self {
                file,
                slots: kept.slots,
                next,
            });
        }
        // Emptied, since copy-on-write only goes off for a file with nothing
        // in it.
        file.set_len(0)?;
        if let Err(error) = without_copy_on_write(&file) {
            eprintln!(
                "last-seconds-keep: copy-on-write stays on for {}: {error}; each record is \
                 still sent to the disk, and may be lost to a cut until the filesystem commits",
                path.display()
            );
        }
        let whole = usize::try_from(length(header.slots)).unwrap_or(0);
        let mut made = vec![0; whole];
        made[..HEADER].copy_from_slice(&header.encode());
        file.write_all_at(&made, 0)?;
        // Made whole once, file and folder, so no later write needs to
        // change anything about it but its contents.
        file.sync_all()?;
        if let Some(folder) = path.parent() {
            File::open(folder)?.sync_all()?;
        }
        Ok(Self {
            file,
            slots: header.slots,
            next: 0,
        })
    }

    /// Writes the next record in its slot and sends that page to the
    /// disk, giving it its place in the boot's records.
    pub fn put(&mut self, record: &mut Record) -> io::Result<()> {
        record.sequence = self.next;
        let at = slot_at(self.next, self.slots);
        self.file.write_all_at(&record.encode(), at)?;
        sent(&self.file, at)?;
        self.next += 1;
        Ok(())
    }
}

/// Holds the ring for this recorder alone, so a second one, run by hand,
/// can't write the same records into the same slots beside it. The hold
/// goes with the recorder, however it ends.
fn held(file: &File) -> io::Result<()> {
    // SAFETY: it takes a descriptor this owns and plain numbers.
    if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } == 0 {
        return Ok(());
    }
    let error = io::Error::last_os_error();
    if error.kind() == io::ErrorKind::WouldBlock {
        Err(io::Error::new(
            io::ErrorKind::WouldBlock,
            "another recorder is writing it",
        ))
    } else {
        Err(error)
    }
}

/// Turns copy-on-write off for a file still empty, on btrfs, the one
/// filesystem here that has it; any other writes in place already.
fn without_copy_on_write(file: &File) -> io::Result<()> {
    let mut filesystem = std::mem::MaybeUninit::<libc::statfs>::uninit();
    // SAFETY: it writes the filesystem's figures into memory this owns,
    // read only once it says it did.
    if unsafe { libc::fstatfs(file.as_raw_fd(), filesystem.as_mut_ptr()) } != 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: fstatfs said it filled it.
    if unsafe { filesystem.assume_init() }.f_type != libc::BTRFS_SUPER_MAGIC {
        return Ok(());
    }
    let mut flags: libc::c_int = 0;
    // SAFETY: both ask about and set the inode's flags through a pointer to
    // an int, which is what the kernel reads and writes there whatever the
    // request's name says, on a file this owns.
    let done = unsafe {
        libc::ioctl(file.as_raw_fd(), libc::FS_IOC_GETFLAGS, &raw mut flags) == 0 && {
            flags |= FS_NOCOW_FL;
            libc::ioctl(file.as_raw_fd(), libc::FS_IOC_SETFLAGS, &raw const flags) == 0
        }
    };
    if done {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

/// Sends the page a record was written to to the disk and waits for it.
fn sent(file: &File, at: u64) -> io::Result<()> {
    let offset = libc::off64_t::try_from(at).map_err(|_| io::ErrorKind::InvalidInput)?;
    let length = libc::off64_t::try_from(record::SIZE).map_err(|_| io::ErrorKind::InvalidInput)?;
    let flags = libc::SYNC_FILE_RANGE_WAIT_BEFORE
        | libc::SYNC_FILE_RANGE_WRITE
        | libc::SYNC_FILE_RANGE_WAIT_AFTER;
    // SAFETY: it takes a descriptor this owns and plain numbers.
    if unsafe { libc::sync_file_range(file.as_raw_fd(), offset, length, flags) } == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

/// What a ring holds: its header, and every whole record in it, oldest
/// first. A slot never written, or one a cut left half written, is passed
/// over by its CRC.
pub fn read(path: &Path) -> io::Result<(Header, Vec<Record>)> {
    let bytes = std::fs::read(path)?;
    let not_a_ring = || io::Error::new(io::ErrorKind::InvalidData, "it isn't a ring");
    let header: &[u8; HEADER] = bytes
        .get(..HEADER)
        .and_then(|bytes| bytes.try_into().ok())
        .ok_or_else(not_a_ring)?;
    let header = Header::decode(header).ok_or_else(not_a_ring)?;
    let mut records: Vec<Record> = bytes
        .get(PAGE..)
        .unwrap_or_default()
        .as_chunks::<{ record::SIZE }>()
        .0
        .iter()
        .take(usize::try_from(header.slots).unwrap_or(usize::MAX))
        .filter_map(Record::decode)
        .collect();
    records.sort_by_key(|record| record.sequence);
    Ok((header, records))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header(slots: u32) -> Header {
        Header {
            slots,
            began: 1_795_241_514,
            processors: 16,
            rests: ["POLL", "C1", "C1E", "C6S", "C10"]
                .map(str::to_owned)
                .to_vec(),
        }
    }

    fn ring_path(name: &str) -> std::path::PathBuf {
        let folder =
            std::env::temp_dir().join(format!("last-seconds-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).expect("a folder");
        folder.join("ring")
    }

    fn put(ring: &mut Ring, at_ms: i64) {
        let mut record = Record {
            at_ms,
            ..Record::default()
        };
        ring.put(&mut record).expect("a record written");
    }

    #[test]
    fn a_header_reads_back_as_it_was_written() {
        let made = header(SLOTS);
        assert_eq!(Header::decode(&made.encode()), Some(made));
    }

    #[test]
    fn the_ring_keeps_its_newest_seconds_and_writes_over_the_oldest() {
        let path = ring_path("wraps");
        let mut ring = Ring::open(&path, &header(4)).expect("a ring");
        for second in 0..6 {
            put(&mut ring, second);
        }
        let (_, records) = read(&path).expect("the ring");
        let kept: Vec<(u32, i64)> = records
            .iter()
            .map(|record| (record.sequence, record.at_ms))
            .collect();
        assert_eq!(kept, [(2, 2), (3, 3), (4, 4), (5, 5)]);
        let size = std::fs::metadata(&path).expect("the ring").len();
        assert_eq!(size, 4096 + 4 * 128, "the ring grew past its slots");
    }

    #[test]
    fn seconds_that_follow_each_other_lie_on_different_pages() {
        let pages: Vec<u64> = (0..=SLOTS)
            .map(|sequence| slot_at(sequence, SLOTS) / 4096)
            .collect();
        assert!(
            pages.windows(2).all(|pair| pair[0] != pair[1]),
            "two seconds in a row shared a page"
        );
        let mut slots: Vec<u64> = (0..SLOTS)
            .map(|sequence| slot_at(sequence, SLOTS))
            .collect();
        slots.sort_unstable();
        slots.dedup();
        assert_eq!(slots.len(), 608, "two seconds of one turn shared a slot");
        assert_eq!(
            slot_at(SLOTS, SLOTS),
            slot_at(0, SLOTS),
            "the turn didn't wrap"
        );
        assert!(
            slots.last().is_some_and(|last| last + 128 <= length(SLOTS)),
            "a slot lay past the ring's end"
        );
    }

    #[test]
    fn a_garbled_page_loses_one_second_in_nineteen_never_the_last_few() {
        let path = ring_path("garbled");
        let mut ring = Ring::open(&path, &header(SLOTS)).expect("a ring");
        for second in 0..700 {
            put(&mut ring, second);
        }
        let newest = slot_at(699, SLOTS);
        let page = newest - newest % 4096;
        let file = OpenOptions::new()
            .write(true)
            .open(&path)
            .expect("the ring");
        file.write_all_at(&[0xa5; 4096], page)
            .expect("a garbled page");
        let (_, records) = read(&path).expect("the ring");
        let kept: Vec<u32> = records.iter().map(|record| record.sequence).collect();
        assert_eq!(
            kept.len(),
            608 - 32,
            "a garbled page took more than its own"
        );
        assert_eq!(
            kept[kept.len() - 3..],
            [696, 697, 698],
            "the seconds before the newest went with it"
        );
    }

    #[test]
    fn a_ring_cut_short_while_made_is_made_again() {
        let path = ring_path("short");
        Ring::open(&path, &header(4)).expect("a ring");
        let file = OpenOptions::new()
            .write(true)
            .open(&path)
            .expect("the ring");
        file.set_len(4096).expect("a ring cut short");
        let mut ring = Ring::open(&path, &header(4)).expect("the ring again");
        put(&mut ring, 1);
        let size = std::fs::metadata(&path).expect("the ring").len();
        assert_eq!(
            size,
            4096 + 4 * 128,
            "a short ring was written past its end"
        );
    }

    #[test]
    fn a_record_a_cut_left_half_written_is_passed_over() {
        let path = ring_path("torn");
        let mut ring = Ring::open(&path, &header(4)).expect("a ring");
        for second in 0..3 {
            put(&mut ring, second);
        }
        // The fourth slot half written, as a cut leaves it.
        let half = Record {
            sequence: 3,
            ..Record::default()
        }
        .encode();
        let file = OpenOptions::new()
            .write(true)
            .open(&path)
            .expect("the ring");
        file.write_all_at(&half[..64], slot_at(3, 4))
            .expect("half a record");
        let (_, records) = read(&path).expect("the ring");
        assert_eq!(records.last().map(|record| record.sequence), Some(2));
    }

    #[test]
    fn a_ring_opened_again_goes_on_after_its_newest_record() {
        let path = ring_path("again");
        let mut ring = Ring::open(&path, &header(4)).expect("a ring");
        put(&mut ring, 10);
        put(&mut ring, 11);
        drop(ring);
        let mut ring = Ring::open(&path, &header(4)).expect("the ring again");
        put(&mut ring, 12);
        let (_, records) = read(&path).expect("the ring");
        let kept: Vec<(u32, i64)> = records
            .iter()
            .map(|record| (record.sequence, record.at_ms))
            .collect();
        assert_eq!(kept, [(0, 10), (1, 11), (2, 12)]);
    }

    #[test]
    fn a_second_recorder_is_refused_the_ring_one_is_writing() {
        let path = ring_path("held");
        let mut ring = Ring::open(&path, &header(4)).expect("a ring");
        put(&mut ring, 10);
        let second = Ring::open(&path, &header(4))
            .err()
            .map(|error| error.kind());
        drop(ring);
        let after = Ring::open(&path, &header(4)).map(|ring| ring.next);
        assert_eq!(second, Some(io::ErrorKind::WouldBlock));
        assert_eq!(
            after.ok(),
            Some(1),
            "the ring wasn't let go with its recorder"
        );
    }

    #[test]
    fn a_ring_of_another_layout_is_made_anew() {
        let path = ring_path("anew");
        let mut ring = Ring::open(&path, &header(4)).expect("a ring");
        put(&mut ring, 10);
        drop(ring);
        Ring::open(&path, &header(8)).expect("a ring of another size");
        let (kept, records) = read(&path).expect("the ring");
        assert_eq!((kept.slots, records.len()), (8, 0));
    }

    #[test]
    fn a_file_that_isnt_a_ring_is_said_as_one() {
        let path = ring_path("not");
        std::fs::write(&path, b"not a ring").expect("a file");
        let error = read(&path).expect_err("it read as a ring");
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    }
}
