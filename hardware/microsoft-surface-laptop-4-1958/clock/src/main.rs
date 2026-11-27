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
//!
//! Its units alone run it, in the terminal kit's shape, as
//! `surface-clock --help` says (`help()`).

use std::fs::{self, File, OpenOptions};
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use terminal_kit::{Exit, Failure, Help};

const DEVICE: &str = "/dev/surface/aggregator";
const SSAM_CDEV_REQUEST: u64 = 0xC028_A501;
const HAS_RESPONSE: u16 = 1;
const TARGET_CATEGORY_SAM: u8 = 0x01;
const TARGET_ID_SAM: u8 = 0x01;
const COMMAND_SET_UNIX_TIME: u8 = 0x0f;
const COMMAND_GET_UNIX_TIME: u8 = 0x10;
/// Time zones run from 12 hours behind UTC to 14 ahead. Which sign the
/// firmware's offset takes depends on which of its two clocks keeps local
/// time, so either is taken up to the widest zone's 14 hours, and an offset
/// past that can't be the firmware's own and is refused.
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

const _: () = assert!(
    std::mem::size_of::<Request>() == 40,
    "a request must be the 40 bytes its ioctl number says"
);

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

/// Whether `offset` is one a time zone could give, the one check of both
/// the offset measured and the offset kept, so the two can't drift apart.
fn a_zones(offset: i64) -> bool {
    offset.abs() <= OFFSET_LIMIT
}

/// The firmware's offset, checked to be one a time zone could give.
fn offset_from(controller: i64, cmos: i64) -> Result<i64, String> {
    let offset = controller - cmos;
    if !a_zones(offset) {
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
    if !a_zones(offset) {
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

/// The offset kept by the last `calibrate`, or none when no boot has
/// measured one yet.
fn kept_offset(path: &Path) -> Result<Option<i64>, String> {
    match fs::read_to_string(path) {
        Ok(text) => parse_offset(&text).map(Some),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(format!("can't read {}: {error}", path.display())),
    }
}

fn calibrate(state: &Path) -> Result<String, String> {
    // Once the time is synchronised the kernel writes it to the CMOS clock,
    // so the two clocks' difference is no longer the firmware's. That is so
    // whenever this starts after boot, as on a switch to a system that has it.
    if synchronised() {
        return Ok(
            "the system's time is synchronised, so the CMOS clock may follow it; the offset kept stays"
                .into(),
        );
    }
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
    let Some(offset) = kept_offset(&state.join("offset"))? else {
        return Ok(
            "no boot has measured the firmware's offset yet, so the Aggregator's clock is left alone"
                .into(),
        );
    };
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

const NO_STATE: &str = "there's no STATE_DIRECTORY to keep the offset in";

/// What `surface-clock --help` says: the two steps its units run.
fn help() -> Help {
    let step = |name: &str, about: &str, example: &str, failed: &str| {
        Help::new(name, about)
            .example(&format!("surface-clock {name}"), example)
            .exit(Exit::Failed, failed)
            .exit(Exit::Config, NO_STATE)
    };
    Help::new("surface-clock", "keeps the clock the firmware starts from")
        .example(
            "surface-clock calibrate",
            "at boot, before the clocks are set",
        )
        .example("surface-clock keep", "hourly and at shutdown, once in sync")
        .subcommand(step(
            "calibrate",
            "keeps the firmware's offset between its clocks",
            "as its unit runs it at boot",
            "a clock couldn't be read or the offset kept",
        ))
        .subcommand(step(
            "keep",
            "sets the Aggregator's clock to the true time",
            "as its unit runs it, hourly and at shutdown",
            "the Aggregator's clock couldn't be set",
        ))
        .exit(Exit::Failed, "a clock couldn't be read or set")
        .exit(Exit::Config, NO_STATE)
}

fn main() -> ExitCode {
    terminal_kit::main(&help(), env!("CARGO_PKG_VERSION"), |context| {
        let [step] = context.args.as_slice() else {
            return Err(Failure::new(Exit::Usage, "surface-clock takes one step")
                .because("calibrate or keep")
                .try_this("surface-clock --help"));
        };
        let state = std::env::var_os("STATE_DIRECTORY")
            .map(PathBuf::from)
            .ok_or_else(|| {
                Failure::new(Exit::Config, NO_STATE).because("its systemd unit sets it")
            })?;
        let said = if step == "calibrate" {
            calibrate(&state)
        } else {
            keep(&state)
        };
        let said = said.map_err(|error| {
            Failure::new(Exit::Failed, format!("surface-clock couldn't {step}")).because(error)
        })?;
        println!("{said}");
        Ok(())
    })
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
        offset_from(1_747_475_245, 1_793_366_371)
            .expect_err("a measured offset of over a year was taken");
        parse_offset("-50401\n").expect_err("a kept offset past 14 hours behind was read");
        parse_offset("50401\n").expect_err("a kept offset past 14 hours ahead was read");
    }

    #[test]
    fn a_measured_offset_is_taken_to_14_hours_either_way_and_no_further() {
        let cmos = 1_747_446_445;
        assert_eq!(
            offset_from(cmos + 50_400, cmos),
            Ok(50_400),
            "an Aggregator 14 hours ahead of the CMOS clock should be taken"
        );
        assert_eq!(
            offset_from(cmos - 50_400, cmos),
            Ok(-50_400),
            "an Aggregator 14 hours behind the CMOS clock should be taken"
        );
        offset_from(cmos + 50_401, cmos)
            .expect_err("an Aggregator past 14 hours ahead of the CMOS clock was taken");
        offset_from(cmos - 50_401, cmos)
            .expect_err("an Aggregator past 14 hours behind the CMOS clock was taken");
    }

    #[test]
    fn a_kept_offset_of_exactly_14_hours_is_read() {
        assert_eq!(
            parse_offset("50400\n"),
            Ok(50_400),
            "14 hours ahead should be read"
        );
        assert_eq!(
            parse_offset("-50400\n"),
            Ok(-50_400),
            "14 hours behind should be read too, the offset's sign following which clock is local"
        );
    }

    #[test]
    fn a_kept_offset_is_read_back() {
        assert_eq!(parse_offset("28800\n"), Ok(28_800));
        parse_offset("eight hours").expect_err("words were read as an offset");
    }

    #[test]
    fn no_kept_offset_is_none_and_a_kept_one_is_read() {
        let state =
            std::env::temp_dir().join(format!("surface-clock-offset-{}", std::process::id()));
        fs::create_dir_all(&state).unwrap();
        let path = state.join("offset");
        let before = kept_offset(&path);
        fs::write(&path, "28800\n").unwrap();
        let after = kept_offset(&path);
        fs::remove_dir_all(&state).unwrap();
        assert_eq!(before, Ok(None));
        assert_eq!(after, Ok(Some(28_800)));
    }

    #[test]
    fn the_aggregator_is_set_ahead_by_the_offset() {
        assert_eq!(controller_time(1_793_366_371, 28_800), Ok(1_793_395_171));
    }

    #[test]
    fn a_time_past_the_aggregator_s_clock_is_refused() {
        controller_time(i64::from(u32::MAX), 1)
            .expect_err("a time past the 32-bit clock's last second was set");
        controller_time(0, -1).expect_err("a time before the clock's first second was set");
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
