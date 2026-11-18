//! What V4L2 says of a camera without streaming from it: the format and
//! frame interval it was last asked for, and its controls' values. A
//! second opener may ask all of these while another program streams, so
//! asking costs the stream nothing but a few control requests.
//!
//! The structures are the kernel's (`linux/videodev2.h`), laid out by hand
//! for the four calls asked, since nothing else here needs a binding.

use std::fs::File;
use std::io;
use std::os::fd::AsRawFd;
use std::os::raw::{c_int, c_ulong, c_void};

unsafe extern "C" {
    fn ioctl(fd: c_int, request: c_ulong, ...) -> c_int;
}

/// `_IOWR('V', number, size)`.
const fn readwrite(number: c_ulong, size: usize) -> c_ulong {
    (3 << 30) | ((size as c_ulong) << 16) | ((b'V' as c_ulong) << 8) | number
}

const CAPTURE: u32 = 1;
const NEXT: u32 = 0x8000_0000;
const DISABLED: u32 = 0x0001;
const WRITE_ONLY: u32 = 0x0040;

/// The kinds of control whose value one number says.
const INTEGER: u32 = 1;
const BOOLEAN: u32 = 2;
const MENU: u32 = 3;
const INTEGER_MENU: u32 = 9;

#[repr(C)]
struct Formatted {
    kind: u32,
    // The union's pointers align it to eight.
    pad: u32,
    width: u32,
    height: u32,
    pixels: [u8; 4],
    rest: [u8; 188],
}

#[repr(C)]
struct Streaming {
    kind: u32,
    capability: u32,
    mode: u32,
    numerator: u32,
    denominator: u32,
    rest: [u8; 184],
}

#[repr(C)]
struct Queried {
    id: u32,
    kind: u32,
    name: [u8; 32],
    minimum: i32,
    maximum: i32,
    step: i32,
    default: i32,
    flags: u32,
    reserved: [u32; 2],
}

#[repr(C)]
struct Control {
    id: u32,
    value: i32,
}

#[repr(C, packed)]
struct Menu {
    id: u32,
    index: u32,
    name: [u8; 32],
    reserved: u32,
}

const GET_FORMAT: c_ulong = readwrite(4, size_of::<Formatted>());
const GET_STREAMING: c_ulong = readwrite(21, size_of::<Streaming>());
const GET_CONTROL: c_ulong = readwrite(27, size_of::<Control>());
const QUERY_CONTROL: c_ulong = readwrite(36, size_of::<Queried>());
const QUERY_MENU: c_ulong = readwrite(37, size_of::<Menu>());

fn ask<T>(file: &File, request: c_ulong, argument: &mut T) -> io::Result<()> {
    // SAFETY: the argument is the structure the request names, laid out
    // as the kernel lays it out, and lives through the call.
    let done = unsafe {
        ioctl(
            file.as_raw_fd(),
            request,
            std::ptr::from_mut(argument).cast::<c_void>(),
        )
    };
    if done < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

/// A fixed field of text, up to its first NUL.
fn text(field: &[u8]) -> String {
    let end = field.iter().position(|b| *b == 0).unwrap_or(field.len());
    String::from_utf8_lossy(&field[..end]).into_owned()
}

/// The format a capture was last set to: its pixels' four letters, and
/// its width and height.
pub struct Format {
    pub pixels: String,
    pub width: u32,
    pub height: u32,
}

pub fn format(file: &File) -> io::Result<Format> {
    let mut asked = Formatted {
        kind: CAPTURE,
        pad: 0,
        width: 0,
        height: 0,
        pixels: [0; 4],
        rest: [0; 188],
    };
    ask(file, GET_FORMAT, &mut asked)?;
    Ok(Format {
        pixels: text(&asked.pixels),
        width: asked.width,
        height: asked.height,
    })
}

/// The frame rate a capture was last asked for, in frames a second.
pub fn asked_rate(file: &File) -> io::Result<f64> {
    let mut asked = Streaming {
        kind: CAPTURE,
        capability: 0,
        mode: 0,
        numerator: 0,
        denominator: 0,
        rest: [0; 184],
    };
    ask(file, GET_STREAMING, &mut asked)?;
    if asked.numerator == 0 {
        return Err(io::Error::other("no frame interval"));
    }
    Ok(f64::from(asked.denominator) / f64::from(asked.numerator))
}

/// A control's value, and for a menu, the name of the item it's at.
pub struct Value {
    pub name: String,
    pub value: i32,
    pub item: Option<String>,
}

/// Every control a number says the value of, in the driver's order. A
/// control that won't say is left out rather than failing the rest.
pub fn controls(file: &File) -> Vec<Value> {
    let mut values = Vec::new();
    let mut id = NEXT;
    loop {
        let mut queried = Queried {
            id,
            kind: 0,
            name: [0; 32],
            minimum: 0,
            maximum: 0,
            step: 0,
            default: 0,
            flags: 0,
            reserved: [0; 2],
        };
        if ask(file, QUERY_CONTROL, &mut queried).is_err() {
            return values;
        }
        id = queried.id | NEXT;
        let said = matches!(queried.kind, INTEGER | BOOLEAN | MENU | INTEGER_MENU);
        if !said || queried.flags & (DISABLED | WRITE_ONLY) != 0 {
            continue;
        }
        let mut control = Control {
            id: queried.id,
            value: 0,
        };
        if ask(file, GET_CONTROL, &mut control).is_err() {
            continue;
        }
        let item = (queried.kind == MENU).then(|| {
            let mut menu = Menu {
                id: queried.id,
                index: u32::try_from(control.value).unwrap_or(u32::MAX),
                name: [0; 32],
                reserved: 0,
            };
            ask(file, QUERY_MENU, &mut menu)
                .ok()
                .map(|()| text(&{ menu.name }))
        });
        values.push(Value {
            name: text(&queried.name),
            value: control.value,
            item: item.flatten(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_calls_are_the_kernels() {
        // As videodev2.h numbers them on a 64-bit machine.
        assert_eq!(GET_FORMAT, 0xc0d0_5604);
        assert_eq!(GET_STREAMING, 0xc0cc_5615);
        assert_eq!(GET_CONTROL, 0xc008_561b);
        assert_eq!(QUERY_CONTROL, 0xc044_5624);
        assert_eq!(QUERY_MENU, 0xc02c_5625);
    }

    #[test]
    fn a_field_of_text_ends_at_its_first_nul() {
        assert_eq!(text(b"MJPG"), "MJPG");
        assert_eq!(text(b"Exposure\0junk"), "Exposure");
    }
}
