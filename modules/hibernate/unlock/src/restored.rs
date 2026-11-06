//! How long a restore from hibernation kept the screen waiting, said once
//! the machine is back.
//!
//! The initrd that reads the image back is gone once it has: the image's
//! kernel takes its memory, its log included, so nothing the starting
//! kernel knew survives the jump but what the firmware keeps. Just before
//! the image is read, `hibernate-unlock restoring` runs in the initrd and
//! keeps the processor's time stamp counter in an EFI variable. The counter
//! starts at zero as the firmware starts and counts on through the jump, so
//! once the machine is back the hook reads it again and says how long
//! reading the image and bringing the machine back took after the disk
//! opened, and how long the whole start took from the power button. Once
//! the firmware has handed over, it sets only variables it keeps across
//! starts, and refuses a volatile one, so the hook removes the variable
//! once it has read it: one there was kept by this start's initrd.

use std::fs;
use std::io;
use std::os::fd::AsRawFd;
use std::path::Path;
use std::thread;
use std::time::{Duration, Instant};

/// Where the count is kept, in a namespace of its own.
pub const VARIABLE: &str =
    "/sys/firmware/efi/efivars/HibernateRestoring-dd4c2ee2-8abd-413f-8f63-48837b8a3f23";

/// The variable's attributes, which efivarfs takes as its first four
/// bytes: non-volatile, with boot services and runtime access, the only
/// kind the firmware sets once the kernel runs.
const ATTRIBUTES: [u8; 4] = [0x07, 0, 0, 0];

/// `FS_IOC_GETFLAGS` and `FS_IOC_SETFLAGS` on x86-64, and the flag efivarfs
/// sets on every variable it doesn't name as safe to remove, which keeps an
/// unlink from deleting it.
const GET_FLAGS: libc::c_ulong = 0x8008_6601;
const SET_FLAGS: libc::c_ulong = 0x4008_6602;
const IMMUTABLE: libc::c_int = 0x10;

/// How long the counter's rate is measured over: long enough that the
/// clock's reading takes a vanishing share of it.
const SAMPLED: Duration = Duration::from_millis(20);

/// The processor's time stamp counter, or none where there isn't one to
/// read.
#[cfg_attr(
    target_arch = "x86_64",
    expect(
        clippy::unnecessary_wraps,
        reason = "other processors have no counter to read"
    )
)]
pub fn counted() -> Option<u64> {
    #[cfg(target_arch = "x86_64")]
    {
        // SAFETY: every x86-64 processor has rdtsc, and reading it changes
        // nothing.
        Some(unsafe { std::arch::x86_64::_rdtsc() })
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        None
    }
}

/// Keeps the count now in `variable`, in one write, as efivarfs takes it.
pub fn keep(variable: &Path, count: u64) -> io::Result<()> {
    let mut written = ATTRIBUTES.to_vec();
    written.extend_from_slice(count.to_string().as_bytes());
    fs::write(variable, written)
}

/// The count kept in `variable`, or none when there isn't one.
pub fn kept(variable: &Path) -> Option<u64> {
    let read = fs::read(variable).ok()?;
    let text = std::str::from_utf8(read.get(ATTRIBUTES.len()..)?).ok()?;
    text.trim().parse().ok()
}

/// Removes `variable`, clearing efivarfs's immutable flag first. A file
/// elsewhere has no such flag to clear, so failing to clear it is no
/// failure; failing to remove it is.
pub fn forget(variable: &Path) -> io::Result<()> {
    if let Ok(file) = fs::File::open(variable) {
        let fd = file.as_raw_fd();
        let mut flags: libc::c_int = 0;
        // SAFETY: each call passes an open descriptor and a pointer to an
        // int, which is what both requests read or write.
        unsafe {
            if libc::ioctl(fd, GET_FLAGS, &raw mut flags) == 0 && flags & IMMUTABLE != 0 {
                flags &= !IMMUTABLE;
                libc::ioctl(fd, SET_FLAGS, &raw const flags);
            }
        }
    }
    fs::remove_file(variable)
}

/// How many counts the counter makes in a second, measured against the
/// monotonic clock.
pub fn rate() -> Option<f64> {
    let (began, from) = (Instant::now(), counted()?);
    thread::sleep(SAMPLED);
    let (ended, to) = (counted()?, began.elapsed());
    let counts = ended.checked_sub(from)?;
    // A count past 2^52 loses its last bits as a float, far below a
    // second's worth.
    #[expect(clippy::cast_precision_loss, reason = "a rate needs no exact count")]
    Some(counts as f64 / to.as_secs_f64())
}

/// What the hook says of a restore, from the count kept before the image
/// was read, the count now and the counter's rate; none when the kept count
/// is from after now, which no restore of this start could be.
pub fn said(then: u64, now: u64, rate: f64) -> Option<String> {
    let since = now.checked_sub(then)?;
    if rate <= 0.0 {
        return None;
    }
    #[expect(
        clippy::cast_precision_loss,
        reason = "tenths of a second need no exact count"
    )]
    let (after, before) = (since as f64 / rate, then as f64 / rate);
    Some(format!(
        "the image was read and the machine back {:.1} s after the disk opened, {:.1} s after \
         the power button",
        after,
        before + after
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// A folder of its own for a test.
    fn folder(name: &str) -> PathBuf {
        let folder = std::env::temp_dir().join(format!("restored-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&folder);
        fs::create_dir_all(&folder).unwrap();
        folder
    }

    #[test]
    fn a_kept_count_is_read_back_after_its_attributes() {
        let variable = folder("kept").join("HibernateRestoring");
        keep(&variable, 123_456_789).unwrap();
        let written = fs::read(&variable).unwrap();
        assert_eq!(written[..4], ATTRIBUTES);
        assert_eq!(kept(&variable), Some(123_456_789));
    }

    #[test]
    fn a_forgotten_count_is_gone() {
        let variable = folder("forget").join("HibernateRestoring");
        keep(&variable, 7).unwrap();
        forget(&variable).unwrap();
        assert_eq!(kept(&variable), None);
    }

    #[test]
    fn no_variable_to_forget_is_a_failure() {
        assert!(forget(&folder("none-to-forget").join("HibernateRestoring")).is_err());
    }

    #[test]
    fn no_variable_keeps_no_count() {
        assert_eq!(kept(&folder("none").join("HibernateRestoring")), None);
    }

    #[test]
    fn a_variable_too_short_for_its_attributes_keeps_no_count() {
        let variable = folder("short").join("HibernateRestoring");
        fs::write(&variable, [0x06, 0]).unwrap();
        assert_eq!(kept(&variable), None);
    }

    #[test]
    fn a_restore_is_said_in_seconds_after_the_disk_and_the_power_button() {
        // Opened 12 s into the start, and back 4.5 s later, at 2 GHz.
        let said = said(24_000_000_000, 33_000_000_000, 2e9).unwrap();
        assert_eq!(
            said,
            "the image was read and the machine back 4.5 s after the disk opened, 16.5 s after \
             the power button"
        );
    }

    #[test]
    fn a_count_kept_after_now_says_nothing() {
        assert_eq!(said(33_000_000_000, 24_000_000_000, 2e9), None);
    }

    #[test]
    fn no_rate_says_nothing() {
        assert_eq!(said(1, 2, 0.0), None);
    }

    #[cfg(target_arch = "x86_64")]
    #[test]
    fn the_counter_counts_on_at_a_rate() {
        let rate = rate().unwrap();
        assert!(rate > 1e6, "the counter counted {rate} a second");
    }
}
