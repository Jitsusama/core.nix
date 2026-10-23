//! Writing a disk no further ahead of it than it keeps up with. Linux holds
//! what's written to a disk in memory and writes it out as it can, so a
//! count of what was handed over runs ahead of a slow stick by as much as a
//! gigabyte, and the last of it is waited for at the end in a long silence
//! with nothing on the screen moving. Waiting for each 32 MiB one window
//! behind keeps the count within seconds of what the stick holds, while it
//! always has the next window to write. macOS's raw disks hold nothing
//! back, so there nothing is waited for.

use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom, Write};

/// 32 MiB: a couple of seconds of a slow stick's writing.
pub const WINDOW: u64 = 32 << 20;

/// Bytes of a disk, from the first to just past the last.
pub type Range = (u64, u64);

/// Tells the system to write out what it holds of a range of the disk.
pub trait Settle {
    /// Starts writing the range out, and doesn't wait.
    fn start(&mut self, range: Range) -> io::Result<()>;
    /// Waits until the range is written out.
    fn wait(&mut self, range: Range) -> io::Result<()>;
}

/// A disk written a window at a time, each waited for once the next is
/// handed over.
pub struct Behind<D: Read + Write + Seek, S: Settle> {
    disk: D,
    settle: S,
    position: u64,
    /// What's been written since the last window was handed over.
    current: Option<Range>,
    written: u64,
    /// The window handed over last, and not yet waited for.
    previous: Option<Range>,
}

impl<D: Read + Write + Seek, S: Settle> Behind<D, S> {
    pub fn new(disk: D, settle: S) -> Behind<D, S> {
        Behind {
            disk,
            settle,
            position: 0,
            current: None,
            written: 0,
            previous: None,
        }
    }

    /// Hands over what's been written since the last window, then waits for
    /// that one.
    fn hand_over(&mut self) -> io::Result<()> {
        self.written = 0;
        if let Some(current) = self.current.take() {
            self.settle.start(current)?;
            if let Some(previous) = self.previous.replace(current) {
                self.settle.wait(previous)?;
            }
        }
        Ok(())
    }

    /// Hands over what's left and waits for all of it.
    pub fn finish(&mut self) -> io::Result<()> {
        self.hand_over()?;
        if let Some(previous) = self.previous.take() {
            self.settle.wait(previous)?;
        }
        Ok(())
    }
}

impl<D: Read + Write + Seek, S: Settle> Write for Behind<D, S> {
    fn write(&mut self, from: &[u8]) -> io::Result<usize> {
        let count = self.disk.write(from)?;
        let (start, end) = (self.position, self.position + count as u64);
        self.current = Some(self.current.map_or((start, end), |(first, last)| {
            (first.min(start), last.max(end))
        }));
        self.position = end;
        self.written += count as u64;
        if self.written >= WINDOW {
            self.hand_over()?;
        }
        Ok(count)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.disk.flush()
    }
}

impl<D: Read + Write + Seek, S: Settle> Read for Behind<D, S> {
    fn read(&mut self, into: &mut [u8]) -> io::Result<usize> {
        let count = self.disk.read(into)?;
        self.position += count as u64;
        Ok(count)
    }
}

impl<D: Read + Write + Seek, S: Settle> Seek for Behind<D, S> {
    fn seek(&mut self, to: SeekFrom) -> io::Result<u64> {
        self.position = self.disk.seek(to)?;
        Ok(self.position)
    }
}

/// The system's own writing out of an open file or disk.
pub struct Kernel<'f>(pub &'f File);

#[cfg(target_os = "linux")]
impl Kernel<'_> {
    fn range(&self, (first, last): Range, flags: libc::c_uint) -> io::Result<()> {
        use std::os::fd::AsRawFd;
        let offset = libc::off64_t::try_from(first).map_err(io::Error::other)?;
        let length = libc::off64_t::try_from(last - first).map_err(io::Error::other)?;
        // SAFETY: the descriptor is the open file's own, and the call reads
        // only its plain arguments.
        if unsafe { libc::sync_file_range(self.0.as_raw_fd(), offset, length, flags) } == 0 {
            Ok(())
        } else {
            Err(io::Error::last_os_error())
        }
    }
}

#[cfg(target_os = "linux")]
impl Settle for Kernel<'_> {
    fn start(&mut self, range: Range) -> io::Result<()> {
        self.range(range, libc::SYNC_FILE_RANGE_WRITE)
    }

    fn wait(&mut self, range: Range) -> io::Result<()> {
        self.range(
            range,
            libc::SYNC_FILE_RANGE_WAIT_BEFORE
                | libc::SYNC_FILE_RANGE_WRITE
                | libc::SYNC_FILE_RANGE_WAIT_AFTER,
        )
    }
}

/// macOS's raw disks hold nothing back, and its image files are only ever
/// test sticks.
#[cfg(not(target_os = "linux"))]
impl Settle for Kernel<'_> {
    fn start(&mut self, _range: Range) -> io::Result<()> {
        Ok(())
    }

    fn wait(&mut self, _range: Range) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    /// What was started and waited for, in order.
    #[derive(Default)]
    struct Told(Vec<(&'static str, Range)>);

    impl Settle for &mut Told {
        fn start(&mut self, range: Range) -> io::Result<()> {
            self.0.push(("start", range));
            Ok(())
        }

        fn wait(&mut self, range: Range) -> io::Result<()> {
            self.0.push(("wait", range));
            Ok(())
        }
    }

    const MIB: u64 = 1 << 20;

    #[test]
    fn each_window_is_waited_for_once_the_next_is_handed_over() {
        let mut told = Told::default();
        let mut disk = Behind::new(Cursor::new(vec![]), &mut told);
        let mebibyte = vec![7u8; 1 << 20];
        for _ in 0..80 {
            disk.write_all(&mebibyte).unwrap();
        }
        disk.finish().unwrap();
        let written = disk.disk.get_ref().len();
        assert_eq!(written, 80 << 20);
        assert_eq!(
            told.0,
            [
                ("start", (0, 32 * MIB)),
                ("start", (32 * MIB, 64 * MIB)),
                ("wait", (0, 32 * MIB)),
                ("start", (64 * MIB, 80 * MIB)),
                ("wait", (32 * MIB, 64 * MIB)),
                ("wait", (64 * MIB, 80 * MIB)),
            ]
        );
    }

    #[test]
    fn a_window_reaches_from_its_first_write_to_its_last_wherever_they_land() {
        let mut told = Told::default();
        let mut disk = Behind::new(Cursor::new(vec![]), &mut told);
        disk.seek(SeekFrom::Start(100 * MIB)).unwrap();
        disk.write_all(&[1; 4096]).unwrap();
        disk.seek(SeekFrom::Start(MIB)).unwrap();
        disk.write_all(&[2; 4096]).unwrap();
        disk.finish().unwrap();
        assert_eq!(
            told.0,
            [
                ("start", (MIB, 100 * MIB + 4096)),
                ("wait", (MIB, 100 * MIB + 4096))
            ]
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn the_kernel_writes_out_a_file_as_it_is_told() {
        let directory = crate::Scratch::new("behind");
        std::fs::create_dir_all(&directory).unwrap();
        let path = directory.join("disk");
        let file = File::create(&path).unwrap();
        let mut disk = Behind::new(&file, Kernel(&file));
        let mebibyte = vec![9u8; 1 << 20];
        for _ in 0..70 {
            disk.write_all(&mebibyte).unwrap();
        }
        disk.finish().unwrap();
        assert_eq!(std::fs::metadata(&path).unwrap().len(), 70 << 20);
    }
}
