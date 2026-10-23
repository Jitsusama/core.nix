//! The few V4L2 calls the relay makes, on structures laid out as
//! <linux/videodev2.h> lays them out on 64-bit Linux. The assertions under
//! each hold them to the sizes and offsets the kernel's headers give.

use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::mem::{offset_of, size_of};
use std::os::fd::{AsRawFd, RawFd};
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;
use std::ptr::NonNull;

pub const CAPTURE: u32 = 1;
pub const OUTPUT: u32 = 2;
const MEMORY_MMAP: u32 = 1;
const FIELD_NONE: u32 = 1;

pub const GREY: u32 = fourcc(*b"GREY");

/// v4l2loopback's event saying whether a reader is streaming.
pub const CLIENT_USAGE: u32 = 0x0800_0000 + 0x08e0_0000 + 1;
const SEND_INITIAL: u32 = 1;

const VIDIOC_S_FMT: libc::c_ulong = 0xc0d0_5605;
const VIDIOC_REQBUFS: libc::c_ulong = 0xc014_5608;
const VIDIOC_QUERYBUF: libc::c_ulong = 0xc058_5609;
const VIDIOC_QBUF: libc::c_ulong = 0xc058_560f;
const VIDIOC_DQBUF: libc::c_ulong = 0xc058_5611;
const VIDIOC_STREAMON: libc::c_ulong = 0x4004_5612;
const VIDIOC_STREAMOFF: libc::c_ulong = 0x4004_5613;
const VIDIOC_DQEVENT: libc::c_ulong = 0x8088_5659;
const VIDIOC_SUBSCRIBE_EVENT: libc::c_ulong = 0x4020_565a;

/// A format's four-character code.
pub const fn fourcc(code: [u8; 4]) -> u32 {
    u32::from_le_bytes(code)
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PixFormat {
    pub width: u32,
    pub height: u32,
    pub pixelformat: u32,
    pub field: u32,
    pub bytesperline: u32,
    pub sizeimage: u32,
    pub colorspace: u32,
    pub private: u32,
    pub flags: u32,
    pub ycbcr_enc: u32,
    pub quantization: u32,
    pub xfer_func: u32,
}
const _: () = assert!(size_of::<PixFormat>() == 48);

// The kernel's union of formats, of which only the single plane one is used.
// The union also holds structures with pointers, so it's aligned to eight.
#[repr(C, align(8))]
struct Formats {
    pix: PixFormat,
    rest: [u8; 152],
}

#[repr(C)]
struct Format {
    kind: u32,
    formats: Formats,
}
const _: () = assert!(size_of::<Format>() == 208 && offset_of!(Format, formats) == 8);

#[repr(C)]
struct RequestBuffers {
    count: u32,
    kind: u32,
    memory: u32,
    capabilities: u32,
    flags: u8,
    reserved: [u8; 3],
}
const _: () = assert!(size_of::<RequestBuffers>() == 20);

#[repr(C)]
#[derive(Clone, Copy)]
struct Timecode {
    kind: u32,
    flags: u32,
    frames: u8,
    seconds: u8,
    minutes: u8,
    hours: u8,
    userbits: [u8; 4],
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Buffer {
    index: u32,
    kind: u32,
    bytesused: u32,
    flags: u32,
    field: u32,
    timestamp: libc::timeval,
    timecode: Timecode,
    sequence: u32,
    memory: u32,
    // For memory mapped buffers, the offset to map, in the low half.
    offset: u64,
    length: u32,
    reserved2: u32,
    request_fd: i32,
}
const _: () = assert!(
    size_of::<Buffer>() == 88
        && offset_of!(Buffer, timestamp) == 24
        && offset_of!(Buffer, timecode) == 40
        && offset_of!(Buffer, sequence) == 56
        && offset_of!(Buffer, memory) == 60
        && offset_of!(Buffer, offset) == 64
        && offset_of!(Buffer, length) == 72
        && offset_of!(Buffer, request_fd) == 80
);

#[repr(C)]
struct EventSubscription {
    kind: u32,
    id: u32,
    flags: u32,
    reserved: [u32; 5],
}
const _: () = assert!(size_of::<EventSubscription>() == 32);

#[repr(C)]
struct Event {
    kind: u32,
    data: [u64; 8],
    pending: u32,
    sequence: u32,
    timestamp: libc::timespec,
    id: u32,
    reserved: [u32; 8],
}
const _: () = assert!(
    size_of::<Event>() == 136
        && offset_of!(Event, data) == 8
        && offset_of!(Event, pending) == 72
        && offset_of!(Event, timestamp) == 80
        && offset_of!(Event, id) == 96
);

/// An all-zero value of one of the structures above, each of which the
/// kernel reads zero as unset in.
fn zeroed<T>() -> T {
    // SAFETY: every structure here is plain integers, for which zero is valid.
    unsafe { std::mem::zeroed() }
}

/// A V4L2 device, opened so that waiting is left to poll.
pub struct Device {
    file: File,
}

impl Device {
    pub fn open(path: &Path) -> io::Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(libc::O_NONBLOCK | libc::O_CLOEXEC)
            .open(path)?;
        Ok(Self { file })
    }

    fn ioctl<T>(&self, request: libc::c_ulong, argument: &mut T) -> io::Result<()> {
        loop {
            // SAFETY: each request is given the structure the kernel defines
            // for it, which the assertions above hold to the kernel's layout.
            let result = unsafe { libc::ioctl(self.fd(), request, std::ptr::from_mut(argument)) };
            if result != -1 {
                return Ok(());
            }
            let error = io::Error::last_os_error();
            if error.kind() != io::ErrorKind::Interrupted {
                return Err(error);
            }
        }
    }

    /// Asks for a format, and says what the driver chose instead.
    pub fn set_format(&self, kind: u32, wanted: PixFormat) -> io::Result<PixFormat> {
        let mut format = Format { kind, ..zeroed() };
        format.formats.pix = PixFormat {
            field: FIELD_NONE,
            ..wanted
        };
        self.ioctl(VIDIOC_S_FMT, &mut format)?;
        Ok(format.formats.pix)
    }

    /// Asks for memory mapped buffers, and says how many the driver gave.
    fn request_buffers(&self, kind: u32, count: u32) -> io::Result<u32> {
        let mut request = RequestBuffers {
            count,
            kind,
            memory: MEMORY_MMAP,
            ..zeroed()
        };
        self.ioctl(VIDIOC_REQBUFS, &mut request)?;
        Ok(request.count)
    }

    fn buffer(&self, kind: u32, index: u32) -> io::Result<Buffer> {
        let mut buffer = Buffer {
            index,
            kind,
            memory: MEMORY_MMAP,
            ..zeroed()
        };
        self.ioctl(VIDIOC_QUERYBUF, &mut buffer)?;
        Ok(buffer)
    }

    fn queue(&self, kind: u32, index: u32) -> io::Result<()> {
        let mut buffer = Buffer {
            index,
            kind,
            memory: MEMORY_MMAP,
            ..zeroed()
        };
        self.ioctl(VIDIOC_QBUF, &mut buffer)
    }

    /// The next filled buffer, if one is.
    fn dequeue(&self, kind: u32) -> io::Result<Option<Buffer>> {
        let mut buffer = Buffer {
            kind,
            memory: MEMORY_MMAP,
            ..zeroed()
        };
        match self.ioctl(VIDIOC_DQBUF, &mut buffer) {
            Ok(()) => Ok(Some(buffer)),
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => Ok(None),
            Err(error) => Err(error),
        }
    }

    fn stream(&self, kind: u32, on: bool) -> io::Result<()> {
        // A buffer type is a small number, so it reads the same signed.
        let mut kind: libc::c_int = kind.cast_signed();
        self.ioctl(
            if on {
                VIDIOC_STREAMON
            } else {
                VIDIOC_STREAMOFF
            },
            &mut kind,
        )
    }

    /// Asks for an event, sent once at once as well as each time it happens.
    pub fn subscribe(&self, kind: u32) -> io::Result<()> {
        let mut subscription = EventSubscription {
            kind,
            flags: SEND_INITIAL,
            ..zeroed()
        };
        self.ioctl(VIDIOC_SUBSCRIBE_EVENT, &mut subscription)
    }

    /// The next event's kind and its data's first word, if one is waiting.
    pub fn event(&self) -> io::Result<Option<(u32, u32)>> {
        let mut event: Event = zeroed();
        match self.ioctl(VIDIOC_DQEVENT, &mut event) {
            Ok(()) => Ok(Some((event.kind, low_half(event.data[0])))),
            // The kernel says there's none this way, waiting or not.
            Err(error) if error.raw_os_error() == Some(libc::ENOENT) => Ok(None),
            Err(error) => Err(error),
        }
    }

    /// Writes a whole frame, as a loopback's producer does.
    pub fn write_frame(&mut self, frame: &[u8]) -> io::Result<()> {
        let written = self.file.write(frame)?;
        if written == frame.len() {
            Ok(())
        } else {
            Err(io::Error::new(
                io::ErrorKind::WriteZero,
                format!(
                    "the device took {written} of a frame's {} bytes",
                    frame.len()
                ),
            ))
        }
    }

    pub fn fd(&self) -> RawFd {
        self.file.as_raw_fd()
    }
}

/// A buffer of a device's, mapped into memory.
struct Mapping {
    start: NonNull<u8>,
    length: usize,
}

/// The low 32 bits of a 64-bit field, where the kernel puts a 32-bit word in
/// a union wider than it.
fn low_half(field: u64) -> u32 {
    let [a, b, c, d, ..] = field.to_le_bytes();
    u32::from_le_bytes([a, b, c, d])
}

impl Mapping {
    fn new(device: &Device, buffer: &Buffer) -> io::Result<Self> {
        let length = buffer.length as usize;
        // SAFETY: the kernel checks the offset and length it gave against
        // the buffer, and the mapping is shared, so nothing here owns it.
        let start = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                length,
                libc::PROT_READ,
                libc::MAP_SHARED,
                device.fd(),
                libc::off_t::from(low_half(buffer.offset)),
            )
        };
        if start == libc::MAP_FAILED {
            return Err(io::Error::last_os_error());
        }
        let start = NonNull::new(start.cast()).ok_or_else(|| io::Error::other("mapped at zero"))?;
        Ok(Self { start, length })
    }

    fn bytes(&self) -> &[u8] {
        // SAFETY: the mapping is this long, and lives as long as self.
        unsafe { std::slice::from_raw_parts(self.start.as_ptr(), self.length) }
    }
}

impl Drop for Mapping {
    fn drop(&mut self) {
        // SAFETY: this maps exactly what new mapped, once.
        unsafe { libc::munmap(self.start.as_ptr().cast(), self.length) };
    }
}

/// A capture device streaming into memory mapped buffers. Dropping it stops
/// the stream and closes the device, which is what powers a camera down.
pub struct Capture {
    // Unmapped before the device closes, as fields drop in order.
    mappings: Vec<Mapping>,
    device: Device,
    pub format: PixFormat,
}

impl Capture {
    /// Starts streaming `wanted` from the device at `path`, refusing a
    /// device that would stream anything else.
    pub fn start(path: &Path, wanted: PixFormat, buffers: u32) -> io::Result<Self> {
        let device = Device::open(path)?;
        let format = device.set_format(CAPTURE, wanted)?;
        let fits = format.pixelformat == wanted.pixelformat
            && format.width == wanted.width
            && format.height == wanted.height;
        if !fits {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "it streams {}x{} {:?} rather than {}x{} {:?}",
                    format.width,
                    format.height,
                    format.pixelformat.to_le_bytes().map(char::from),
                    wanted.width,
                    wanted.height,
                    wanted.pixelformat.to_le_bytes().map(char::from),
                ),
            ));
        }
        let count = device.request_buffers(CAPTURE, buffers)?;
        let mut mappings = Vec::new();
        for index in 0..count {
            mappings.push(Mapping::new(&device, &device.buffer(CAPTURE, index)?)?);
            device.queue(CAPTURE, index)?;
        }
        device.stream(CAPTURE, true)?;
        Ok(Self {
            mappings,
            device,
            format,
        })
    }

    /// Hands the next filled frame to `read`, and gives its buffer back,
    /// saying whether there was one.
    pub fn next(&self, read: impl FnOnce(&[u8])) -> io::Result<bool> {
        let Some(buffer) = self.device.dequeue(CAPTURE)? else {
            return Ok(false);
        };
        let mapping = self
            .mappings
            .get(buffer.index as usize)
            .ok_or_else(|| io::Error::other(format!("no buffer {}", buffer.index)))?;
        let used = (buffer.bytesused as usize).min(mapping.length);
        read(&mapping.bytes()[..used]);
        self.device.queue(CAPTURE, buffer.index)?;
        Ok(true)
    }

    pub fn fd(&self) -> RawFd {
        self.device.fd()
    }
}

impl Drop for Capture {
    fn drop(&mut self) {
        let _ = self.device.stream(CAPTURE, false);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_formats_as_the_kernel_does() {
        assert_eq!(GREY, 0x5945_5247);
        assert_eq!(CLIENT_USAGE, 0x10e0_0001);
    }
}
