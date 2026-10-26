//! disk-loss-panic: panics the kernel once the disk the system runs from stops answering, so
//! pstore keeps the kernel's last messages.
//!
//! When a disk goes, the journal goes with it, and the kernel writes its log to pstore only
//! when it panics. A disk that fails its reads blocks no task, so neither the hung-task
//! detector nor a filesystem going read-only panics. This reads the kernel's log from
//! /dev/kmsg, from the start of the boot on. Each warning or worse that names the disk's
//! drivers or btrfs sends it to read the disk's first block straight from the device,
//! bypassing the page cache. A read that fails, or doesn't finish within `PROBE_LIMIT`, panics
//! the kernel through sysrq; one that answers was a false alarm, and it reads on.
//!
//! It panics only once per record: while pstore still holds an earlier record, which the next
//! boot's systemd-pstore would have archived and cleared had the disk come back, it says what
//! it would have done instead, so a disk that is gone for good can't fill the firmware's
//! variables one boot after another.
//!
//! It's started before anything reads the disk much and locks itself into memory, so once
//! running it needs nothing from the disk it watches.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::os::unix::fs::{FileExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

/// How long the disk may take to answer one read of its first block. An NVMe drive answers
/// in well under a millisecond; nvme's own timeout is 30 s, and a reset after it takes more.
const PROBE_LIMIT: Duration = Duration::from_secs(10);

/// How much is read, and the alignment a direct read needs.
const BLOCK: usize = 4096;

/// The kernel log's levels, from /dev/kmsg's prefix: warning is 4, and lower is worse.
const WARNING: u8 = 4;

/// Words in a kernel message that say it comes from the path between btrfs and the disk, or
/// that a task has waited on something, the disk perhaps, past the hung-task timeout.
const DISK_WORDS: [&str; 7] = [
    "blocked for more than",
    "nvme",
    "BTRFS",
    "I/O error",
    "blk_update_request",
    "device-mapper",
    "dm-crypt",
];

/// Where pstore's records are, and how a kernel log record is named there.
const PSTORE: &str = "/sys/fs/pstore";
const KERNEL_LOG_RECORD: &str = "dmesg-";

#[cfg(target_arch = "x86_64")]
const O_DIRECT: i32 = 0o40000;
#[cfg(target_arch = "aarch64")]
const O_DIRECT: i32 = 0o200000;

const MCL_CURRENT: i32 = 1;
const MCL_FUTURE: i32 = 2;

unsafe extern "C" {
    fn mlockall(flags: i32) -> i32;
}

/// One record of the kernel's log: its level and its message.
#[derive(Debug, PartialEq, Eq)]
struct Record<'a> {
    level: u8,
    message: &'a str,
}

/// Reads a /dev/kmsg record, "prio,seq,usec,flags[,...];message", followed by its
/// continuation lines, each starting with a space, which are left out.
fn parse_record(raw: &str) -> Option<Record<'_>> {
    let (fields, rest) = raw.split_once(';')?;
    let priority: u32 = fields.split(',').next()?.parse().ok()?;
    let message = rest.lines().next().unwrap_or("");
    let level = u8::try_from(priority & 7).ok()?;
    Some(Record { level, message })
}

/// Whether a record could mean the disk is going: a warning or worse from its path.
fn names_disk_trouble(record: &Record<'_>) -> bool {
    record.level <= WARNING && DISK_WORDS.iter().any(|word| record.message.contains(word))
}

/// Whether pstore already holds a kernel log record nobody has archived.
fn holds_a_kernel_log(pstore: &Path) -> bool {
    fs::read_dir(pstore).is_ok_and(|entries| {
        entries.flatten().any(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with(KERNEL_LOG_RECORD)
        })
    })
}

/// A block of memory aligned for a direct read.
struct AlignedBlock {
    pointer: *mut u8,
    layout: std::alloc::Layout,
}

impl AlignedBlock {
    fn new() -> Self {
        let layout = std::alloc::Layout::from_size_align(BLOCK, BLOCK)
            .expect("a block's size is a power of two");
        // SAFETY: the layout's size isn't zero.
        let pointer = unsafe { std::alloc::alloc_zeroed(layout) };
        if pointer.is_null() {
            std::alloc::handle_alloc_error(layout);
        }
        Self { pointer, layout }
    }

    fn as_mut_slice(&mut self) -> &mut [u8] {
        // SAFETY: the pointer holds `layout.size()` bytes this block alone owns.
        unsafe { std::slice::from_raw_parts_mut(self.pointer, self.layout.size()) }
    }
}

impl Drop for AlignedBlock {
    fn drop(&mut self) {
        // SAFETY: the pointer came from `alloc_zeroed` with this layout.
        unsafe { std::alloc::dealloc(self.pointer, self.layout) }
    }
}

/// Reads the disk's first block straight from the device, past the page cache.
fn read_first_block(disk: &Path) -> io::Result<()> {
    let device = OpenOptions::new()
        .read(true)
        .custom_flags(O_DIRECT)
        .open(disk)?;
    let mut block = AlignedBlock::new();
    let read = device.read_at(block.as_mut_slice(), 0)?;
    if read == BLOCK {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            format!("read {read} of {BLOCK} bytes"),
        ))
    }
}

/// What a read of the disk found.
#[derive(Debug, PartialEq, Eq)]
enum Answer {
    Answered,
    Failed(String),
    Silent,
}

/// Reads the disk's first block on a thread of its own, since a read of a disk that has
/// stopped answering may never come back, and gives up after `limit`.
fn probe(disk: &Path, limit: Duration) -> Answer {
    let (sender, receiver) = mpsc::channel();
    let disk = disk.to_path_buf();
    thread::spawn(move || {
        let _ = sender.send(read_first_block(&disk));
    });
    match receiver.recv_timeout(limit) {
        Ok(Ok(())) => Answer::Answered,
        Ok(Err(error)) => Answer::Failed(error.to_string()),
        Err(_) => Answer::Silent,
    }
}

/// Writes a line to the kernel's log at the given level, where pstore will keep it.
fn say(level: u8, line: &str) {
    if let Ok(mut kmsg) = OpenOptions::new().write(true).open("/dev/kmsg") {
        let _ = kmsg.write_all(format!("<{level}>disk-loss-panic: {line}\n").as_bytes());
    }
}

/// Panics the kernel, through sysrq's crash, which pstore records.
fn panic_the_kernel() {
    if let Ok(mut trigger) = OpenOptions::new().write(true).open("/proc/sysrq-trigger") {
        let _ = trigger.write_all(b"c");
    }
}

fn watch(disk: &Path) -> io::Result<()> {
    let mut kmsg = File::open("/dev/kmsg")?;
    // One read gives one record, and no record is longer than this.
    let mut buffer = vec![0_u8; 8192];
    loop {
        let length = match kmsg.read(&mut buffer) {
            Ok(0) => return Ok(()),
            Ok(length) => length,
            // The kernel overwrote records before they were read; read on from the oldest left.
            Err(error) if error.raw_os_error() == Some(32) => continue,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error),
        };
        let raw = String::from_utf8_lossy(&buffer[..length]);
        let Some(record) = parse_record(&raw) else {
            continue;
        };
        if record.message.starts_with("disk-loss-panic:") || !names_disk_trouble(&record) {
            continue;
        }
        let why = match probe(disk, PROBE_LIMIT) {
            Answer::Answered => {
                say(
                    6,
                    &format!("{} answered after \"{}\"", disk.display(), record.message),
                );
                continue;
            }
            Answer::Failed(error) => format!("its read failed: {error}"),
            Answer::Silent => format!("its read didn't finish in {} s", PROBE_LIMIT.as_secs()),
        };
        let disk = disk.display();
        let message = record.message;
        if holds_a_kernel_log(Path::new(PSTORE)) {
            say(
                3,
                &format!(
                    "{disk} stopped answering after \"{message}\" ({why}), but pstore still holds \
                     an earlier kernel log, so the kernel carries on"
                ),
            );
        } else {
            say(
                2,
                &format!(
                    "{disk} stopped answering after \"{message}\" ({why}); panicking so pstore \
                     keeps the kernel log"
                ),
            );
            panic_the_kernel();
        }
    }
}

fn main() {
    let Some(disk) = std::env::args_os().nth(1).map(PathBuf::from) else {
        eprintln!("usage: disk-loss-panic <disk>");
        std::process::exit(2);
    };
    // SAFETY: mlockall takes only flags and touches no memory of ours.
    if unsafe { mlockall(MCL_CURRENT | MCL_FUTURE) } != 0 {
        say(
            4,
            &format!(
                "can't lock itself into memory: {}",
                io::Error::last_os_error()
            ),
        );
    }
    // Resolved once, while the disk still answers, so a by-id link that vanishes with the disk
    // still leads to its device.
    let disk = fs::canonicalize(&disk).unwrap_or(disk);
    say(6, &format!("watching {}", disk.display()));
    if let Err(error) = watch(&disk) {
        eprintln!("disk-loss-panic: can't read the kernel log: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_record_gives_its_level_and_first_line() {
        let raw = "3,1203,16195991,-;nvme nvme0: I/O tag 12 QID 3 timeout, aborting\n \
                   SUBSYSTEM=nvme\n";
        assert_eq!(
            parse_record(raw),
            Some(Record {
                level: 3,
                message: "nvme nvme0: I/O tag 12 QID 3 timeout, aborting"
            })
        );
    }

    #[test]
    fn a_record_from_userspace_keeps_its_level_past_its_facility() {
        assert_eq!(
            parse_record("11,5,100,-;nvme test").map(|r| r.level),
            Some(3)
        );
    }

    #[test]
    fn a_line_without_a_prefix_is_no_record() {
        assert_eq!(parse_record("no prefix here"), None);
        assert_eq!(parse_record("x,1,2,-;words"), None);
    }

    #[test]
    fn btrfs_errors_and_nvme_warnings_are_trouble() {
        let btrfs = Record {
            level: 3,
            message: "BTRFS error (device dm-0 state EA): bdev /dev/mapper/system errs: wr 59",
        };
        let nvme = Record {
            level: 4,
            message: "nvme nvme0: controller is down; will reset: CSTS=0xffffffff",
        };
        let hung = Record {
            level: 3,
            message: "INFO: task btrfs-transacti:612 blocked for more than 120 seconds.",
        };
        assert!(names_disk_trouble(&btrfs));
        assert!(names_disk_trouble(&nvme));
        assert!(names_disk_trouble(&hung));
    }

    #[test]
    fn notices_and_other_drivers_are_not_trouble() {
        let notice = Record {
            level: 6,
            message: "nvme nvme0: 12/0/0 default/read/poll queues",
        };
        let wifi = Record {
            level: 3,
            message: "iwlwifi 0000:02:00.0: Microcode SW error detected",
        };
        assert!(!names_disk_trouble(&notice));
        assert!(!names_disk_trouble(&wifi));
    }

    #[test]
    fn pstore_holding_a_kernel_log_is_seen() {
        let pstore = std::env::temp_dir().join(format!("pstore-{}", std::process::id()));
        fs::create_dir_all(&pstore).unwrap();
        assert!(!holds_a_kernel_log(&pstore));
        fs::write(pstore.join("pmsg-ramoops-0"), "").unwrap();
        assert!(!holds_a_kernel_log(&pstore));
        fs::write(pstore.join("dmesg-efi-178000000001"), "").unwrap();
        assert!(holds_a_kernel_log(&pstore));
        fs::remove_dir_all(&pstore).unwrap();
    }

    #[test]
    fn no_pstore_holds_no_kernel_log() {
        assert!(!holds_a_kernel_log(Path::new("/nonexistent/pstore")));
    }

    #[test]
    fn a_disk_that_is_gone_fails_its_read() {
        let answer = probe(Path::new("/dev/disk-loss-panic-test-none"), PROBE_LIMIT);
        assert!(matches!(answer, Answer::Failed(_)), "{answer:?}");
    }
}
