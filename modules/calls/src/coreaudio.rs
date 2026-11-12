//! Whether a call is on, on a Mac: CoreAudio keeps an object for each
//! process that uses sound, saying whether it records from a microphone
//! right now, and coreaudiod answers for every account, so root asks once.
//! A process recording counts once it still records two seconds later, as
//! src/lib.rs counts a call.

use calls::{MacRecording, STARTS_AFTER};
use std::ffi::{CStr, c_char, c_void};
use std::thread;

/// An object of CoreAudio's, such as a process using sound.
type Object = u32;

/// Which of an object's properties is asked for, as CoreAudio's
/// `AudioObjectPropertyAddress` is laid out.
#[repr(C)]
struct Address {
    selector: u32,
    scope: u32,
    element: u32,
}

/// CoreAudio's own object, which lists the processes.
const SYSTEM: Object = 1;
/// The scope and element every property here is asked in.
const GLOBAL: u32 = u32::from_be_bytes(*b"glob");
const MAIN: u32 = 0;
/// The system's processes using sound, and each one's id, bundle id and
/// whether it records from a microphone, as AudioHardware.h names them.
const PROCESSES: u32 = u32::from_be_bytes(*b"prs#");
const PID: u32 = u32::from_be_bytes(*b"ppid");
const BUNDLE: u32 = u32::from_be_bytes(*b"pbid");
const RECORDING: u32 = u32::from_be_bytes(*b"piri");
/// CoreFoundation's `kCFStringEncodingUTF8`.
const UTF8: u32 = 0x0800_0100;

#[link(name = "CoreAudio", kind = "framework")]
unsafe extern "C" {
    fn AudioObjectGetPropertyDataSize(
        object: Object,
        address: *const Address,
        qualifier_size: u32,
        qualifier: *const c_void,
        size: *mut u32,
    ) -> i32;
    fn AudioObjectGetPropertyData(
        object: Object,
        address: *const Address,
        qualifier_size: u32,
        qualifier: *const c_void,
        size: *mut u32,
        data: *mut c_void,
    ) -> i32;
}

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFStringGetCString(
        string: *const c_void,
        buffer: *mut c_char,
        size: isize,
        encoding: u32,
    ) -> u8;
    fn CFRelease(object: *const c_void);
}

/// Whether a process that calls records from a microphone, and still does
/// two seconds later.
pub fn on() -> Result<bool, String> {
    let first = recording()?;
    if !first.any() {
        return Ok(false);
    }
    thread::sleep(STARTS_AFTER);
    Ok(first.goes_on_in(&recording()?))
}

/// The processes recording from a microphone now.
fn recording() -> Result<MacRecording, String> {
    let mut recording = MacRecording::default();
    for process in processes()? {
        // A process gone since the list was read answers no longer, and
        // records no longer.
        let Ok(records) = value::<u32>(process, RECORDING) else {
            continue;
        };
        if records != 0 {
            let pid = value::<i32>(process, PID)?;
            recording.hears(pid, &bundle(process).unwrap_or_default());
        }
    }
    Ok(recording)
}

fn address(selector: u32) -> Address {
    Address {
        selector,
        scope: GLOBAL,
        element: MAIN,
    }
}

/// The processes CoreAudio keeps an object for.
fn processes() -> Result<Vec<Object>, String> {
    let address = address(PROCESSES);
    let mut size = 0;
    // SAFETY: the address and the size are valid for the call, and no
    // qualifier is passed.
    let status = unsafe {
        AudioObjectGetPropertyDataSize(
            SYSTEM,
            &raw const address,
            0,
            std::ptr::null(),
            &raw mut size,
        )
    };
    if status != 0 {
        return Err(format!(
            "CoreAudio wouldn't say how many processes use sound ({status})"
        ));
    }
    let mut objects = vec![0 as Object; size as usize / size_of::<Object>()];
    let mut size = u32::try_from(objects.len() * size_of::<Object>()).unwrap_or(0);
    // SAFETY: the buffer holds `size` bytes of objects, which is what
    // CoreAudio writes at most, and it says in `size` how many it wrote.
    let status = unsafe {
        AudioObjectGetPropertyData(
            SYSTEM,
            &raw const address,
            0,
            std::ptr::null(),
            &raw mut size,
            objects.as_mut_ptr().cast(),
        )
    };
    if status != 0 {
        return Err(format!(
            "CoreAudio wouldn't list the processes using sound ({status})"
        ));
    }
    objects.truncate(size as usize / size_of::<Object>());
    Ok(objects)
}

/// A number-sized property of an object, a process's id or whether it
/// records.
fn value<T: Default + Copy>(object: Object, selector: u32) -> Result<T, String> {
    let address = address(selector);
    let mut value = T::default();
    let mut size = u32::try_from(size_of::<T>()).unwrap_or(0);
    // SAFETY: `value` holds `size` bytes, the size of what each selector
    // asked here answers with.
    let status = unsafe {
        AudioObjectGetPropertyData(
            object,
            &raw const address,
            0,
            std::ptr::null(),
            &raw mut size,
            (&raw mut value).cast(),
        )
    };
    if status != 0 {
        return Err(format!(
            "CoreAudio wouldn't say a process's {selector:#x} ({status})"
        ));
    }
    Ok(value)
}

/// A process's bundle id, none when it has none.
fn bundle(process: Object) -> Option<String> {
    let string = value::<usize>(process, BUNDLE).ok()? as *const c_void;
    if string.is_null() {
        return None;
    }
    let mut buffer = [0 as c_char; 256];
    // SAFETY: `string` is the CFString CoreAudio made for the caller, and
    // the buffer holds the 256 bytes it's said to.
    let copied = unsafe {
        let copied = CFStringGetCString(string, buffer.as_mut_ptr(), 256, UTF8);
        CFRelease(string);
        copied
    };
    if copied == 0 {
        return None;
    }
    // SAFETY: CFStringGetCString ended what it copied with a nul.
    let named = unsafe { CStr::from_ptr(buffer.as_ptr()) };
    Some(named.to_string_lossy().into_owned())
}
