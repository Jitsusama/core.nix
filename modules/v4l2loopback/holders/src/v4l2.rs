//! What V4L2 says of a camera without streaming from it: the format and
//! frame interval it was last asked for, and its controls' values. A
//! second opener may ask all of these while another program streams, so
//! asking costs the stream nothing but a few control requests.
//!
//! The structures are the kernel's (`linux/videodev2.h`), laid out by hand
//! for the four calls asked, since nothing else here needs a binding.

use std::collections::BTreeSet;
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

// The errors that end a list of controls, as errno-base.h numbers them.
const EINVAL: i32 = 22;
const ENOTTY: i32 = 25;

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

/// Asks the driver one thing.
///
/// # Safety
///
/// `T` must be the structure `request` names, laid out as the kernel lays
/// it out, since the kernel reads and writes its size through the pointer.
unsafe fn ask<T>(file: &File, request: c_ulong, argument: &mut T) -> io::Result<()> {
    // SAFETY: the caller pairs the request with its structure, and the
    // argument lives through the call.
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

/// A fixed field of text, up to its first NUL, with anything that could
/// break the journal's line made plain, as a holder's name is.
fn text(field: &[u8]) -> String {
    let end = field.iter().position(|b| *b == 0).unwrap_or(field.len());
    crate::holders::text(&field[..end])
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
    // SAFETY: GET_FORMAT names a v4l2_format, which Formatted lays out.
    unsafe { ask(file, GET_FORMAT, &mut asked) }?;
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
    // SAFETY: GET_STREAMING names a v4l2_streamparm, as Streaming is.
    unsafe { ask(file, GET_STREAMING, &mut asked) }?;
    if asked.numerator == 0 {
        return Err(io::Error::other("no frame interval"));
    }
    Ok(f64::from(asked.denominator) / f64::from(asked.numerator))
}

/// A control, the value it reads or why it doesn't, and for a menu, the
/// name of the item it's at.
pub struct Value {
    pub name: String,
    pub reads: Result<i32, &'static str>,
    pub item: Option<String>,
}

/// A camera's controls, and why the list ends short when it does.
#[derive(Default)]
pub struct Controls {
    pub values: Vec<Value>,
    pub cut_short: Option<String>,
}

/// Every control a number says the value of, in the driver's order. A
/// control refused is said so rather than failing the rest. One
/// that timed out joins those unanswered, which aren't asked again:
/// uvcvideo asks the camera itself for a control that changes on its own,
/// such as exposure while it's automatic, and waits five seconds on one
/// that doesn't answer, so the list ends at the first to time out, each
/// line waiting on at most one while the watch can't see who streams, and
/// the next line goes on past it. Any other failure is asked again, since
/// a camera may refuse a control only while its stream starts. Asking
/// what the next control is asks the camera too: its range the first
/// time, and its master's value for one that follows another, as exposure
/// follows auto exposure. One that times out there is kept by the id it
/// was asked after, and the list ends there, said so, from then on.
pub fn controls(file: &File, unanswered: &mut BTreeSet<u32>) -> Controls {
    listed(file, unanswered)
}

/// The three calls `controls` makes of a camera, so a test can answer
/// them as a camera that stalls would.
trait Asks {
    fn query(&self, queried: &mut Queried) -> io::Result<()>;
    fn get(&self, control: &mut Control) -> io::Result<()>;
    fn menu(&self, menu: &mut Menu) -> io::Result<()>;
}

impl Asks for File {
    fn query(&self, queried: &mut Queried) -> io::Result<()> {
        // SAFETY: QUERY_CONTROL names a v4l2_queryctrl, as Queried is.
        unsafe { ask(self, QUERY_CONTROL, queried) }
    }

    fn get(&self, control: &mut Control) -> io::Result<()> {
        // SAFETY: GET_CONTROL names a v4l2_control, as Control is.
        unsafe { ask(self, GET_CONTROL, control) }
    }

    fn menu(&self, menu: &mut Menu) -> io::Result<()> {
        // SAFETY: QUERY_MENU names a v4l2_querymenu, as Menu is.
        unsafe { ask(self, QUERY_MENU, menu) }
    }
}

fn listed(camera: &impl Asks, unanswered: &mut BTreeSet<u32>) -> Controls {
    let mut values = Vec::new();
    let mut id = NEXT;
    loop {
        // An id asked after carries NEXT, which no control's own id does.
        if unanswered.contains(&id) {
            let cut_short = Some("the next control was unanswered".to_owned());
            return Controls { values, cut_short };
        }
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
        if let Err(error) = camera.query(&mut queried) {
            if unanswering(&error) {
                unanswered.insert(id);
            }
            let cut_short = (!ends_the_list(&error)).then(|| error.to_string());
            return Controls { values, cut_short };
        }
        id = queried.id | NEXT;
        let said = matches!(queried.kind, INTEGER | BOOLEAN | MENU | INTEGER_MENU);
        if !said || queried.flags & (DISABLED | WRITE_ONLY) != 0 {
            continue;
        }
        let name = text(&queried.name);
        if unanswered.contains(&queried.id) {
            values.push(Value {
                name,
                reads: Err("unanswered"),
                item: None,
            });
            continue;
        }
        let mut control = Control {
            id: queried.id,
            value: 0,
        };
        if let Err(error) = camera.get(&mut control) {
            let timed_out = unanswering(&error);
            values.push(Value {
                name,
                reads: Err(if timed_out { "unanswered" } else { "unread" }),
                item: None,
            });
            if timed_out {
                unanswered.insert(queried.id);
                let cut_short = Some("a control was unanswered".to_owned());
                return Controls { values, cut_short };
            }
            continue;
        }
        let item = (queried.kind == MENU).then(|| {
            let mut menu = Menu {
                id: queried.id,
                index: u32::try_from(control.value).unwrap_or(u32::MAX),
                name: [0; 32],
                reserved: 0,
            };
            camera.menu(&mut menu).ok().map(|()| text(&{ menu.name }))
        });
        values.push(Value {
            name,
            reads: Ok(control.value),
            item: item.flatten(),
        });
    }
}

/// Whether the camera didn't answer at all, which uvcvideo says after
/// waiting five seconds, rather than refusing for now.
fn unanswering(error: &io::Error) -> bool {
    error.kind() == io::ErrorKind::TimedOut
}

/// Whether a failed query is the list's end: EINVAL past the last control,
/// or ENOTTY from a node that isn't V4L2's and has none.
fn ends_the_list(error: &io::Error) -> bool {
    matches!(error.raw_os_error(), Some(EINVAL | ENOTTY))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::offset_of;

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
    fn the_fields_sit_where_the_kernel_reads_them() {
        // As videodev2.h lays them out on a 64-bit machine.
        assert_eq!(
            [
                offset_of!(Formatted, width),
                offset_of!(Formatted, height),
                offset_of!(Formatted, pixels),
            ],
            [8, 12, 16]
        );
        assert_eq!(
            [
                offset_of!(Streaming, numerator),
                offset_of!(Streaming, denominator),
            ],
            [12, 16]
        );
        assert_eq!(
            [
                offset_of!(Queried, kind),
                offset_of!(Queried, name),
                offset_of!(Queried, flags),
            ],
            [4, 8, 56]
        );
        assert_eq!(offset_of!(Control, value), 4);
        assert_eq!([offset_of!(Menu, index), offset_of!(Menu, name)], [4, 8]);
    }

    #[test]
    fn a_field_of_text_ends_at_its_first_nul_and_breaks_no_line() {
        assert_eq!(text(b"MJPG"), "MJPG");
        assert_eq!(text(b"Exposure\0junk"), "Exposure");
        assert_eq!(text(b"Auto\nMode"), "Auto?Mode");
    }

    #[test]
    fn only_a_control_that_timed_out_is_kept_as_unanswered() {
        const EBUSY: i32 = 16;
        const EPIPE: i32 = 32;
        let error = io::Error::from_raw_os_error;
        let kept: Vec<bool> = [ETIMEDOUT, EBUSY, EPIPE, EIO, EINVAL]
            .map(|code| unanswering(&error(code)))
            .into();
        assert_eq!(kept, [true, false, false, false, false]);
    }

    #[test]
    fn the_list_ends_with_einval_or_a_node_that_isnt_v4l2s_and_is_cut_short_else() {
        let error = io::Error::from_raw_os_error;
        let ends: Vec<bool> = [EINVAL, ENOTTY, 110, 5]
            .map(|code| ends_the_list(&error(code)))
            .into();
        assert_eq!(ends, [true, true, false, false]);
        // /dev/full is no V4L2 node: it has no controls, and isn't cut short.
        let full = File::open("/dev/full").unwrap();
        let controls = controls(&full, &mut BTreeSet::new());
        assert_eq!(controls.values.len(), 0);
        assert_eq!(controls.cut_short, None);
    }

    const ETIMEDOUT: i32 = 110;
    const EIO: i32 = 5;

    /// A control as the stand-in camera answers it: an errno where it
    /// fails to say what it is, or to read.
    struct Standing {
        id: u32,
        name: &'static str,
        kind: u32,
        queries: Result<(), i32>,
        reads: Result<i32, i32>,
    }

    fn standing(id: u32, name: &'static str, reads: Result<i32, i32>) -> Standing {
        Standing {
            id,
            name,
            kind: INTEGER,
            queries: Ok(()),
            reads,
        }
    }

    /// A camera that answers from a list, noting each control it's asked
    /// to read, so a test can count the waits.
    struct Camera {
        controls: Vec<Standing>,
        read: std::cell::RefCell<Vec<u32>>,
    }

    impl Camera {
        fn new(controls: Vec<Standing>) -> Self {
            Self {
                controls,
                read: std::cell::RefCell::default(),
            }
        }

        fn find(&self, id: u32) -> &Standing {
            self.controls
                .iter()
                .find(|c| c.id == id)
                .expect("asked of no control")
        }
    }

    impl Asks for Camera {
        fn query(&self, queried: &mut Queried) -> io::Result<()> {
            assert_ne!(
                queried.id & NEXT,
                0,
                "a control was queried but not after another"
            );
            let after = queried.id & !NEXT;
            let next = self
                .controls
                .iter()
                .find(|c| c.id > after)
                .ok_or_else(|| io::Error::from_raw_os_error(EINVAL))?;
            next.queries.map_err(io::Error::from_raw_os_error)?;
            queried.id = next.id;
            queried.kind = next.kind;
            queried.name[..next.name.len()].copy_from_slice(next.name.as_bytes());
            Ok(())
        }

        fn get(&self, control: &mut Control) -> io::Result<()> {
            self.read.borrow_mut().push(control.id);
            control.value = self
                .find(control.id)
                .reads
                .map_err(io::Error::from_raw_os_error)?;
            Ok(())
        }

        fn menu(&self, menu: &mut Menu) -> io::Result<()> {
            let item = b"Aperture Priority Mode";
            let mut name = [0; 32];
            name[..item.len()].copy_from_slice(item);
            menu.name = name;
            Ok(())
        }
    }

    fn said(controls: &Controls) -> Vec<(&str, Result<i32, &str>, Option<&str>)> {
        let said = controls.values.iter();
        said.map(|v| (v.name.as_str(), v.reads, v.item.as_deref()))
            .collect()
    }

    #[test]
    fn a_control_that_times_out_ends_the_line_and_is_passed_over_on_the_next() {
        let camera = Camera::new(vec![
            standing(1, "Brightness", Ok(128)),
            standing(2, "Exposure Time, Absolute", Err(ETIMEDOUT)),
            standing(3, "Gain", Ok(4)),
        ]);
        let mut unanswered = BTreeSet::new();
        let first = listed(&camera, &mut unanswered);
        assert_eq!(
            said(&first),
            [
                ("Brightness", Ok(128), None),
                ("Exposure Time, Absolute", Err("unanswered"), None),
            ]
        );
        assert_eq!(first.cut_short.as_deref(), Some("a control was unanswered"));
        let next = listed(&camera, &mut unanswered);
        assert_eq!(
            said(&next),
            [
                ("Brightness", Ok(128), None),
                ("Exposure Time, Absolute", Err("unanswered"), None),
                ("Gain", Ok(4), None),
            ]
        );
        assert_eq!(next.cut_short, None);
        // The one that timed out was waited on once, over both lines.
        assert_eq!(*camera.read.borrow(), [1, 2, 1, 3]);
    }

    #[test]
    fn a_control_refused_is_said_unread_and_asked_again() {
        let camera = Camera::new(vec![
            standing(1, "Focus, Absolute", Err(EIO)),
            Standing {
                kind: MENU,
                ..standing(2, "Auto Exposure", Ok(3))
            },
        ]);
        let mut unanswered = BTreeSet::new();
        let first = listed(&camera, &mut unanswered);
        let expected = [
            ("Focus, Absolute", Err("unread"), None),
            ("Auto Exposure", Ok(3), Some("Aperture Priority Mode")),
        ];
        assert_eq!(said(&first), expected);
        assert_eq!(first.cut_short, None);
        assert_eq!(said(&listed(&camera, &mut unanswered)), expected);
        assert_eq!(*camera.read.borrow(), [1, 2, 1, 2]);
    }

    #[test]
    fn a_query_that_times_out_ends_the_list_there_from_then_on() {
        // As exposure's query waits on auto exposure's value.
        let camera = Camera::new(vec![
            standing(1, "Brightness", Ok(128)),
            Standing {
                queries: Err(ETIMEDOUT),
                ..standing(2, "Exposure Time, Absolute", Ok(156))
            },
        ]);
        let mut unanswered = BTreeSet::new();
        let first = listed(&camera, &mut unanswered);
        assert_eq!(said(&first), [("Brightness", Ok(128), None)]);
        assert_eq!(
            first.cut_short.as_deref(),
            Some("Connection timed out (os error 110)")
        );
        let next = listed(&camera, &mut unanswered);
        assert_eq!(said(&next), [("Brightness", Ok(128), None)]);
        assert_eq!(
            next.cut_short.as_deref(),
            Some("the next control was unanswered")
        );
    }

    #[test]
    fn a_list_that_stalled_ends_where_it_stalled_from_then_on() {
        // /dev/full stands in: what's kept is asked before the node is.
        let full = File::open("/dev/full").unwrap();
        let mut unanswered = BTreeSet::from([NEXT]);
        let controls = controls(&full, &mut unanswered);
        assert_eq!(controls.values.len(), 0);
        assert_eq!(
            controls.cut_short.as_deref(),
            Some("the next control was unanswered")
        );
    }
}
