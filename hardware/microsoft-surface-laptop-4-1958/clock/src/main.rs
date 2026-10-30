//! surface-clock: keeps the clock the Surface Laptop 4's firmware starts from
//! at the true time.
//!
//! At every power-on the firmware sets the CMOS clock from the Surface
//! Aggregator's own clock, less an offset of its own, so whatever the kernel
//! wrote to the CMOS clock is gone by the next start. `calibrate`, run at boot
//! before anything writes either clock, keeps the difference between the two.
//! `keep`, run while the system's time is synchronised and at shutdown, sets
//! the Aggregator's clock to the true time plus that difference, so the next
//! start's CMOS clock comes out right.
//!
//! Both talk to the Aggregator through its character device, with the
//! requests the Aggregator answers for its clock: 0x10 reads it and 0x0f sets
//! it, each as a little-endian 32-bit Unix time.

use std::fs::{self, File, OpenOptions};
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

const DEVICE: &str = "/dev/surface/aggregator";
const SSAM_CDEV_REQUEST: u64 = 0xC028_A501;
const HAS_RESPONSE: u16 = 1;
const TARGET_CATEGORY_SAM: u8 = 0x01;
const TARGET_ID_SAM: u8 = 0x01;
const COMMAND_SET_UNIX_TIME: u8 = 0x0f;
const COMMAND_GET_UNIX_TIME: u8 = 0x10;
/// Time zones run from 12 hours behind UTC to 14 ahead, so an offset past
/// that can't be the firmware's own and is refused.
const OFFSET_LIMIT: i64 = 14 * 3600;
/// What adjtimex answers while the kernel's time isn't synchronised.
const TIME_ERROR: i32 = 5;

/// A buffer of the character device's request, `struct ssam_cdev_request`.
#[repr(C, packed)]
struct Buffer {
    data: u64,
    length: u16,
    pad: [u8; 6],
}

#[repr(C, packed)]
struct Request {
    target_category: u8,
    target_id: u8,
    command_id: u8,
    instance_id: u8,
    flags: u16,
    status: i16,
    payload: Buffer,
    response: Buffer,
}

const _: () = assert!(std::mem::size_of::<Request>() == 40);

/// The kernel's `struct timex`, 208 bytes on x86-64. Only its first field,
/// the modes to change, matters, and it's left at none, so adjtimex only
/// reports.
#[repr(C, align(8))]
struct Timex([u8; 208]);

unsafe extern "C" {
    fn ioctl(fd: i32, request: u64, argument: *mut Request) -> i32;
    fn adjtimex(buffer: *mut Timex) -> i32;
}

/// Sends one request to the Aggregator's clock, with `payload` as its
/// argument, and answers how many bytes of `response` it filled.
fn send(device: &File, command: u8, payload: &[u8], response: &mut [u8]) -> Result<u16, String> {
    let length = |buffer: usize| {
        u16::try_from(buffer).map_err(|_| format!("a buffer of {buffer} bytes is past a request's"))
    };
    let mut request = Request {
        target_category: TARGET_CATEGORY_SAM,
        target_id: TARGET_ID_SAM,
        command_id: command,
        instance_id: 0,
        flags: if response.is_empty() { 0 } else { HAS_RESPONSE },
        status: 0,
        payload: Buffer {
            data: payload.as_ptr() as u64,
            length: length(payload.len())?,
            pad: [0; 6],
        },
        response: Buffer {
            data: response.as_mut_ptr() as u64,
            length: length(response.len())?,
            pad: [0; 6],
        },
    };
    // SAFETY: the request is the layout the driver reads, and both buffers
    // outlive the call, at the lengths given.
    let result = unsafe { ioctl(device.as_raw_fd(), SSAM_CDEV_REQUEST, &raw mut request) };
    let status = request.status;
    if result != 0 || status != 0 {
        let error = std::io::Error::last_os_error();
        return Err(format!(
            "the Aggregator refused request {command:#04x}: ioctl {result} ({error}), status {status}"
        ));
    }
    Ok(request.response.length)
}

fn read_controller(device: &File) -> Result<u32, String> {
    let mut time = [0u8; 4];
    match send(device, COMMAND_GET_UNIX_TIME, &[], &mut time)? {
        4 => Ok(u32::from_le_bytes(time)),
        length => Err(format!(
            "the Aggregator answered its time in {length} bytes, not 4"
        )),
    }
}

fn set_controller(device: &File, time: u32) -> Result<(), String> {
    send(device, COMMAND_SET_UNIX_TIME, &time.to_le_bytes(), &mut []).map(|_| ())
}

fn open_controller() -> Result<File, String> {
    OpenOptions::new()
        .read(true)
        .write(true)
        .open(DEVICE)
        .map_err(|error| format!("can't open {DEVICE}: {error}"))
}

/// The CMOS clock's time, found by its driver's name rather than taken as
/// rtc0.
fn read_cmos(rtc: &Path) -> Result<i64, String> {
    let entries =
        fs::read_dir(rtc).map_err(|error| format!("can't list {}: {error}", rtc.display()))?;
    for entry in entries.flatten() {
        let name = fs::read_to_string(entry.path().join("name")).unwrap_or_default();
        if name.starts_with("rtc_cmos") {
            let path = entry.path().join("since_epoch");
            let text = fs::read_to_string(&path)
                .map_err(|error| format!("can't read {}: {error}", path.display()))?;
            return text
                .trim()
                .parse()
                .map_err(|error| format!("{} holds {text:?}: {error}", path.display()));
        }
    }
    Err(format!("no CMOS clock under {}", rtc.display()))
}

/// The firmware's offset, checked to be one a time zone could give.
fn offset_from(controller: i64, cmos: i64) -> Result<i64, String> {
    let offset = controller - cmos;
    if offset.abs() > OFFSET_LIMIT {
        return Err(format!(
            "the Aggregator's clock is {offset} s from the CMOS clock, past any time zone's offset"
        ));
    }
    Ok(offset)
}

fn parse_offset(text: &str) -> Result<i64, String> {
    let offset: i64 = text
        .trim()
        .parse()
        .map_err(|error| format!("the offset kept, {text:?}: {error}"))?;
    if offset.abs() > OFFSET_LIMIT {
        return Err(format!(
            "the offset kept, {offset} s, is past any time zone's offset"
        ));
    }
    Ok(offset)
}

/// What the Aggregator's clock should read for the firmware to set the CMOS
/// clock to `now`.
fn controller_time(now: i64, offset: i64) -> Result<u32, String> {
    u32::try_from(now + offset)
        .map_err(|_| format!("{now} plus {offset} s doesn't fit the Aggregator's 32-bit clock"))
}

fn synchronised() -> bool {
    let mut timex = Timex([0; 208]);
    // SAFETY: the buffer is the size of the kernel's struct timex, and with
    // no modes set the call only fills it in.
    let state = unsafe { adjtimex(&raw mut timex) };
    state >= 0 && state != TIME_ERROR
}

/// The system's time in seconds, refused when it is one no clock should be
/// set to.
fn now() -> Result<i64, String> {
    let since = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "the system's time is before 1970".to_string())?;
    i64::try_from(since.as_secs()).map_err(|_| "the system's time is past 64 bits".to_string())
}

fn calibrate(state: &Path) -> Result<String, String> {
    let device = open_controller()?;
    let controller = read_controller(&device)?;
    let cmos = read_cmos(Path::new("/sys/class/rtc"))?;
    let offset = offset_from(i64::from(controller), cmos)?;
    let path = state.join("offset");
    fs::write(&path, format!("{offset}\n"))
        .map_err(|error| format!("can't write {}: {error}", path.display()))?;
    Ok(format!(
        "the firmware set the CMOS clock {offset} s from the Aggregator's, which reads {controller}"
    ))
}

fn keep(state: &Path) -> Result<String, String> {
    if !synchronised() {
        return Ok(
            "the system's time isn't synchronised, so the Aggregator's clock is left alone".into(),
        );
    }
    let path = state.join("offset");
    let text = fs::read_to_string(&path)
        .map_err(|error| format!("can't read {}: {error}", path.display()))?;
    let offset = parse_offset(&text)?;
    let device = open_controller()?;
    let before = read_controller(&device)?;
    let target = controller_time(now()?, offset)?;
    set_controller(&device, target)?;
    let after = read_controller(&device)?;
    if after.abs_diff(target) > 2 {
        return Err(format!(
            "the Aggregator's clock reads {after} after being set to {target}"
        ));
    }
    Ok(format!(
        "set the Aggregator's clock from {before} to {target}, the time {offset:+} s; it reads {after}"
    ))
}

fn main() -> ExitCode {
    let state = std::env::var_os("STATE_DIRECTORY").map(PathBuf::from);
    let result = match (std::env::args().nth(1).as_deref(), state) {
        (Some("calibrate"), Some(state)) => calibrate(&state),
        (Some("keep"), Some(state)) => keep(&state),
        (_, None) => Err("STATE_DIRECTORY isn't set; run it from its systemd unit".into()),
        _ => Err("usage: surface-clock calibrate|keep".into()),
    };
    match result {
        Ok(said) => {
            println!("{said}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_offset_measured_on_the_laptop_is_taken() {
        assert_eq!(offset_from(1_747_475_245, 1_747_446_445), Ok(28_800));
    }

    #[test]
    fn an_offset_past_any_time_zone_is_refused() {
        assert!(offset_from(1_747_475_245, 1_793_366_371).is_err());
        assert!(parse_offset("-50401\n").is_err());
    }

    #[test]
    fn a_kept_offset_is_read_back() {
        assert_eq!(parse_offset("28800\n"), Ok(28_800));
        assert!(parse_offset("eight hours").is_err());
    }

    #[test]
    fn the_aggregator_is_set_ahead_by_the_offset() {
        assert_eq!(controller_time(1_793_366_371, 28_800), Ok(1_793_395_171));
    }

    #[test]
    fn a_time_past_the_aggregator_s_clock_is_refused() {
        assert!(controller_time(i64::from(u32::MAX), 1).is_err());
        assert!(controller_time(0, -1).is_err());
    }

    #[test]
    fn the_cmos_clock_is_found_by_its_driver() {
        let rtc = std::env::temp_dir().join(format!("surface-clock-{}", std::process::id()));
        for (device, name, time) in [
            ("rtc0", "acpi-tad", "1"),
            ("rtc1", "rtc_cmos rtc_cmos", "1793366371"),
        ] {
            fs::create_dir_all(rtc.join(device)).unwrap();
            fs::write(rtc.join(device).join("name"), format!("{name}\n")).unwrap();
            fs::write(rtc.join(device).join("since_epoch"), format!("{time}\n")).unwrap();
        }
        let found = read_cmos(&rtc);
        fs::remove_dir_all(&rtc).unwrap();
        assert_eq!(found, Ok(1_793_366_371));
    }
}
