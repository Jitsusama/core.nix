//! Sets how strongly a haptic touchpad clicks, as Windows' "haptic feedback
//! intensity" slider does: a percentage written to the field a Precision
//! Touchpad's report descriptor names Intensity on the Haptics usage page.
//! The field is found in the descriptor rather than known by its report, so
//! any pad that has one is set and every other device is passed over.
//!
//! Usage: touchpad-click-strength <percent> [hidraw node name...]
//!
//! With no names it looks at every hidraw node. The report is read first and
//! written only when the field holds something else, so running it again,
//! after every resume, changes nothing and says nothing. The field's
//! neighbours in its report are written back as they were read.
//!
//! It needs only the standard library, which already links the C library
//! the one call comes from.

use std::ffi::{c_int, c_ulong, c_void};
use std::fs::{self, OpenOptions};
use std::os::fd::AsRawFd;
use std::process::ExitCode;

const HAPTICS: u16 = 0x0e;
const INTENSITY: u16 = 0x23;

unsafe extern "C" {
    fn ioctl(fd: c_int, request: c_ulong, ...) -> c_int;
}

// The kernel's _IOC for hidraw's feature reports, read and write both, since
// the first byte goes in naming the report and the rest comes back.
const fn feature(number: c_ulong, size: usize) -> c_ulong {
    (3 << 30) | ((size as c_ulong) << 16) | ((b'H' as c_ulong) << 8) | number
}
const fn set_feature(size: usize) -> c_ulong {
    feature(0x06, size)
}
const fn get_feature(size: usize) -> c_ulong {
    feature(0x07, size)
}

/// Where the intensity sits: its report, its first bit after the report's
/// number, how many bits it takes, the range the descriptor gives it, and
/// how long the whole report is with its number.
#[derive(Debug, PartialEq)]
struct Field {
    report: u8,
    offset: usize,
    size: usize,
    minimum: i64,
    maximum: i64,
    length: usize,
}

#[derive(Clone, Copy, Default)]
struct Globals {
    page: u16,
    minimum: i64,
    maximum: i64,
    unsigned_maximum: u32,
    size: usize,
    count: usize,
    report: u8,
}

/// A HID item's low two bytes, which is all of it a 16-bit field holds.
fn low_u16(value: u32) -> u16 {
    let [low, high, ..] = value.to_le_bytes();
    u16::from_le_bytes([low, high])
}

/// A HID item's low byte, which is all of it a report id holds.
fn low_u8(value: u32) -> u8 {
    value.to_le_bytes()[0]
}

/// The intensity field a descriptor gives a feature report, if it gives one
/// in a numbered report, which every Precision Touchpad's reports are.
fn intensity(descriptor: &[u8]) -> Option<Field> {
    let mut globals = Globals::default();
    let mut stack = Vec::new();
    // A usage is on the page it names, or the page in force at the main
    // item it describes.
    let mut usages: Vec<(Option<u16>, u16)> = Vec::new();
    let mut bits = [0usize; 256];
    let mut found = None;
    let mut at = 0;
    while at < descriptor.len() {
        let prefix = descriptor[at];
        if prefix == 0xfe {
            at += 3 + *descriptor.get(at + 1)? as usize;
            continue;
        }
        let size = [0, 1, 2, 4][(prefix & 3) as usize];
        let data = descriptor.get(at + 1..at + 1 + size)?;
        at += 1 + size;
        let unsigned = data
            .iter()
            .rev()
            .fold(0u32, |value, byte| value << 8 | u32::from(*byte));
        let signed = match size {
            1 => i64::from(data[0].cast_signed()),
            2 => i64::from(i16::from_le_bytes([data[0], data[1]])),
            4 => i64::from(i32::from_le_bytes([data[0], data[1], data[2], data[3]])),
            _ => 0,
        };
        match prefix & 0xfc {
            0x04 => globals.page = low_u16(unsigned),
            0x14 => globals.minimum = signed,
            0x24 => {
                globals.maximum = signed;
                globals.unsigned_maximum = unsigned;
            }
            0x74 => globals.size = unsigned as usize,
            0x84 => globals.report = low_u8(unsigned),
            0x94 => globals.count = unsigned as usize,
            0xa4 => stack.push(globals),
            0xb4 => globals = stack.pop()?,
            0x08 if size == 4 => usages.push((Some(low_u16(unsigned >> 16)), low_u16(unsigned))),
            0x08 => usages.push((None, low_u16(unsigned))),
            0xb0 => {
                let report = globals.report as usize;
                let index = usages.iter().position(|&(page, usage)| {
                    (page.unwrap_or(globals.page), usage) == (HAPTICS, INTENSITY)
                });
                if let (None, Some(index), 1..) = (&found, index, globals.report) {
                    // A logical maximum written with its top bit set is read
                    // unsigned when the minimum isn't below zero.
                    let maximum = if globals.maximum < globals.minimum {
                        i64::from(globals.unsigned_maximum)
                    } else {
                        globals.maximum
                    };
                    found = Some(Field {
                        report: globals.report,
                        offset: bits[report]
                            + index.min(globals.count.saturating_sub(1)) * globals.size,
                        size: globals.size,
                        minimum: globals.minimum,
                        maximum,
                        length: 0,
                    });
                }
                bits[report] += globals.size * globals.count;
                usages.clear();
            }
            0x80 | 0x90 | 0xa0 | 0xc0 => usages.clear(),
            _ => {}
        }
    }
    let mut field = found?;
    field.length = 1 + bits[field.report as usize].div_ceil(8);
    (1..=32).contains(&field.size).then_some(field)
}

impl Field {
    fn read(&self, report: &[u8]) -> i64 {
        let raw = (0..self.size).fold(0u64, |value, bit| {
            let at = self.offset + bit;
            value | u64::from(report[1 + at / 8] >> (at % 8) & 1) << bit
        });
        // A field is at most 32 bits, so its bits read as an i64 unchanged.
        let raw = raw.cast_signed();
        if self.minimum < 0 && raw >> (self.size - 1) & 1 == 1 {
            raw - (1i64 << self.size)
        } else {
            raw
        }
    }

    fn write(&self, report: &mut [u8], value: i64) {
        for bit in 0..self.size {
            let at = self.offset + bit;
            let byte = &mut report[1 + at / 8];
            *byte = *byte & !(1 << (at % 8)) | u8::from((value >> bit) & 1 == 1) << (at % 8);
        }
    }

    /// The value a percentage is, rounded, within the field's range.
    fn scaled(&self, percent: u8) -> i64 {
        let span = self.maximum - self.minimum;
        self.minimum + (span * i64::from(percent.min(100)) + 50) / 100
    }

    fn percent(&self, value: i64) -> i64 {
        let span = (self.maximum - self.minimum).max(1);
        ((value - self.minimum) * 100 + span / 2) / span
    }

    /// Sets the field in a report read from the pad, and says what it held,
    /// or nothing when it already holds the percentage.
    fn adjust(&self, report: &mut [u8], percent: u8) -> Option<i64> {
        let was = self.read(report);
        let wanted = self.scaled(percent);
        (was != wanted).then(|| {
            self.write(report, wanted);
            was
        })
    }
}

fn exchange(fd: c_int, request: c_ulong, report: &mut [u8]) -> Result<(), String> {
    let answer = unsafe { ioctl(fd, request, report.as_mut_ptr().cast::<c_void>()) };
    if answer < 0 {
        return Err(std::io::Error::last_os_error().to_string());
    }
    Ok(())
}

// Reads the field's report, checking the pad answered with that report, as
// some answer one feature request with another.
fn fetch(fd: c_int, field: &Field) -> Result<Vec<u8>, String> {
    let mut report = vec![0u8; field.length];
    report[0] = field.report;
    exchange(fd, get_feature(field.length), &mut report)?;
    if report[0] != field.report {
        return Err(format!(
            "answered report {} for {}",
            report[0], field.report
        ));
    }
    Ok(report)
}

fn set(name: &str, percent: u8) -> Result<(), String> {
    let path = format!("/sys/class/hidraw/{name}/device/report_descriptor");
    let Some(field) = fs::read(&path).ok().as_deref().and_then(intensity) else {
        return Ok(());
    };
    let device = OpenOptions::new()
        .read(true)
        .write(true)
        .open(format!("/dev/{name}"))
        .map_err(|error| error.to_string())?;
    let fd = device.as_raw_fd();
    let mut report = fetch(fd, &field)?;
    let Some(was) = field.adjust(&mut report, percent) else {
        return Ok(());
    };
    exchange(fd, set_feature(field.length), &mut report)?;
    let now = field.read(&fetch(fd, &field)?);
    if now != field.scaled(percent) {
        return Err(format!(
            "kept {}% after being set to {percent}%",
            field.percent(now)
        ));
    }
    println!("{name} clicks at {percent}%, from {}%", field.percent(was));
    Ok(())
}

fn main() -> ExitCode {
    let mut arguments = std::env::args().skip(1);
    let Some(percent) = arguments
        .next()
        .and_then(|it| it.parse::<u8>().ok())
        .filter(|it| *it <= 100)
    else {
        eprintln!("Usage: touchpad-click-strength <percent, 0 to 100> [hidraw node name...]");
        return ExitCode::from(64);
    };
    let mut names: Vec<String> = arguments.collect();
    if names.is_empty() {
        names = fs::read_dir("/sys/class/hidraw")
            .map(|entries| {
                entries
                    .filter_map(|entry| entry.ok()?.file_name().into_string().ok())
                    .collect()
            })
            .unwrap_or_default();
    }
    let mut failed = false;
    for name in names {
        if let Err(error) = set(&name, percent) {
            eprintln!("{name}: the click strength wasn't set: {error}");
            failed = true;
        }
    }
    if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // The XPS 14's 2C2F:0034 pad's haptics collection and the input mode
    // report after it, as its descriptor gives them.
    const PAD: &[u8] = &[
        0x05, 0x0e, 0x09, 0x01, 0xa1, 0x02, 0x09, 0x23, 0x85, 0x0b, 0x25, 0x64, 0x95, 0x01, 0xb1,
        0x02, 0xc0, 0xc0, 0x05, 0x0d, 0x09, 0x0e, 0xa1, 0x01, 0x85, 0x04, 0x09, 0x22, 0xa1, 0x02,
        0x09, 0x52, 0x25, 0x0a, 0xb1, 0x02, 0xc0,
    ];

    // Globals a descriptor sets before the haptics collection.
    fn before(rest: &[u8]) -> Vec<u8> {
        [&[0x15, 0x00, 0x75, 0x08][..], rest].concat()
    }

    #[test]
    fn finds_the_pads_intensity() {
        let field = intensity(&before(PAD)).unwrap();
        assert_eq!(
            field,
            Field {
                report: 11,
                offset: 0,
                size: 8,
                minimum: 0,
                maximum: 100,
                length: 2
            }
        );
    }

    #[test]
    fn passes_over_a_device_without_haptics() {
        assert_eq!(intensity(&before(&PAD[18..])), None);
        assert_eq!(intensity(&[]), None);
    }

    #[test]
    fn passes_over_an_intensity_that_is_not_a_feature() {
        let mut input = before(PAD);
        input[4 + 14] = 0x81;
        assert_eq!(intensity(&input), None);
    }

    #[test]
    fn takes_a_usage_naming_its_own_page() {
        let mut descriptor = before(&[0x05, 0x0d, 0x0b, 0x23, 0x00, 0x0e, 0x00]);
        descriptor.extend_from_slice(&[0x85, 0x03, 0x25, 0x64, 0x95, 0x01, 0xb1, 0x02]);
        assert_eq!(intensity(&descriptor).unwrap().report, 3);
    }

    #[test]
    fn finds_it_after_the_fields_before_it_and_past_a_long_item() {
        let descriptor = before(&[
            0xfe, 0x02, 0x10, 0xaa, 0xbb, // a long item, skipped whole
            0x05, 0x0e, 0x85, 0x05, 0x75, 0x04, 0x95, 0x03, 0x09, 0x20, 0xb1, 0x02, // 12 bits
            0xa4, 0x75, 0x01, 0x95, 0x01, 0xb1, 0x03,
            0xb4, // 1 padding bit, pushed and popped
            0x09, 0x23, 0x25, 0x0f, 0x95, 0x01, 0xb1, 0x02, // the intensity, 4 bits
            0x09, 0x21, 0x75, 0x08, 0xb1, 0x02, // 8 bits after it
        ]);
        let field = intensity(&descriptor).unwrap();
        assert_eq!((field.report, field.offset, field.size), (5, 13, 4));
        assert_eq!((field.maximum, field.length), (15, 1 + 25usize.div_ceil(8)));
    }

    #[test]
    fn reads_a_maximum_with_its_top_bit_set_unsigned() {
        let mut descriptor = before(PAD);
        descriptor[4 + 10..4 + 12].copy_from_slice(&[0x25, 0xff]);
        assert_eq!(intensity(&descriptor).unwrap().maximum, 255);
    }

    #[test]
    fn writes_the_field_and_keeps_its_neighbours() {
        let field = Field {
            report: 5,
            offset: 13,
            size: 4,
            minimum: 0,
            maximum: 15,
            length: 5,
        };
        let mut report = [5, 0xff, 0xff, 0xff, 0xff];
        field.write(&mut report, 0b0101);
        assert_eq!(report, [5, 0xff, 0b1011_1111, 0b1111_1110, 0xff]);
        assert_eq!(field.read(&report), 0b0101);
    }

    #[test]
    fn scales_a_percentage_to_the_range() {
        let field = Field {
            report: 1,
            offset: 0,
            size: 8,
            minimum: 0,
            maximum: 255,
            length: 2,
        };
        assert_eq!(
            (field.scaled(0), field.scaled(30), field.scaled(100)),
            (0, 77, 255)
        );
        assert_eq!(field.percent(77), 30);
    }

    #[test]
    fn sets_only_what_differs() {
        let field = intensity(&before(PAD)).unwrap();
        let mut report = [11, 50];
        assert_eq!(field.adjust(&mut report, 30), Some(50));
        assert_eq!(report, [11, 30]);
        assert_eq!(field.adjust(&mut report, 30), None);
        assert_eq!(report, [11, 30]);
    }
}
