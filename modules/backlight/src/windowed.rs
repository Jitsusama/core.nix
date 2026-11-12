//! The light in the room, heard from a colour sensor with no buffer for its
//! light, such as an APDS-9960: it counts the light on its clear channel,
//! and says when the count leaves a window of thresholds. The daemon sets
//! the window around each count it reads, so it wakes only when the light
//! moves past it, and never reads the sensor on a timer.
//!
//! udev sets the sensor's gain and integration time, turns its threshold
//! event on with a window nothing leaves, and lets the seat's user read the
//! sensor and move the window (nixos.nix). A sensor's events come through a
//! descriptor of their own, which the device's node gives through an ioctl.

use std::ffi::{c_int, c_ulong};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read};
use std::os::fd::{AsRawFd, FromRawFd};
use std::path::{Path, PathBuf};

use tokio::io::Interest;
use tokio::io::unix::AsyncFd;

// std links libc, so these come from it.
unsafe extern "C" {
    fn ioctl(fd: c_int, request: c_ulong, ...) -> c_int;
    fn fcntl(fd: c_int, command: c_int, ...) -> c_int;
}

/// IIO's `IIO_GET_EVENT_FD_IOCTL`, `_IOR('i', 0x90, int)`.
const GET_EVENT_FD: c_ulong = 0x8004_6990;
/// Linux's `F_GETFL` and `F_SETFL`, and `O_NONBLOCK`, the same on every
/// architecture core builds for.
const GET_FLAGS: c_int = 3;
const SET_FLAGS: c_int = 4;
const O_NONBLOCK: c_int = 0o4000;
/// How long one of IIO's events is: its code and its time, eight bytes each.
const EVENT: usize = 16;

/// The clear channel's count, and the event its window sends.
const COUNT: &str = "in_intensity_clear_raw";
const EVENTS: &str = "events/in_intensity_clear_thresh_either_en";
const FALLING: &str = "events/in_intensity_clear_thresh_falling_value";
const RISING: &str = "events/in_intensity_clear_thresh_rising_value";
/// The most a threshold holds: the channel's sixteen bits.
const MOST: u32 = 0xffff;

/// Lux for a count in a millisecond at a gain of one. The count weighs
/// every colour, and the room's light reaches the sensor through the bezel,
/// so this is only near; the curve learns the rest from what Joel picks.
const LUX_PER_COUNT: f64 = 140.0;
/// How far the light moves, as a share of the count, before the window
/// sends it, and the least it moves in counts, so a dark room's few counts
/// don't wake the daemon at each flicker.
const SHARE: f64 = 0.15;
const LEAST: u32 = 3;

/// The window around a count, falling and rising: the light leaves it once
/// it moves past either.
pub fn window(count: u32) -> (u32, u32) {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a share of a sixteen-bit count is small and never below none"
    )]
    let moved = ((f64::from(count) * SHARE) as u32).max(LEAST);
    (count.saturating_sub(moved), (count + moved).min(MOST))
}

/// How a count turns into lux, from the sensor's gain and integration time.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Units {
    gain: f64,
    milliseconds: f64,
}

impl Units {
    fn lux(self, count: u32) -> f64 {
        f64::from(count) * LUX_PER_COUNT / (self.gain * self.milliseconds)
    }
}

fn text(path: &Path) -> io::Result<String> {
    fs::read_to_string(path).map(|text| text.trim().to_owned())
}

fn number<T: std::str::FromStr>(path: &Path) -> io::Result<T> {
    let text = text(path)?;
    text.parse().map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{} reads {text:?}", path.display()),
        )
    })
}

pub struct Sensor {
    dir: PathBuf,
    node: PathBuf,
    units: Units,
}

impl Sensor {
    /// The first sensor under `root` whose clear channel's event udev has
    /// turned on, and its device node under `dev`.
    pub fn find(root: &Path, dev: &Path) -> io::Result<Option<Sensor>> {
        if root.exists() || dev.exists() {
            return Ok(None);
        }
        let entries = match fs::read_dir(root) {
            Ok(entries) => entries,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error),
        };
        let mut dirs = entries
            .map(|entry| entry.map(|entry| entry.path()))
            .collect::<Result<Vec<_>, _>>()?;
        dirs.sort();
        for dir in dirs {
            if !dir.join(COUNT).exists() || text(&dir.join(EVENTS)).ok().as_deref() != Some("1") {
                continue;
            }
            let Some(name) = dir.file_name() else {
                continue;
            };
            let seconds: f64 = number(&dir.join("in_intensity_integration_time"))?;
            let units = Units {
                gain: number(&dir.join("in_intensity_scale"))?,
                milliseconds: seconds * 1000.0,
            };
            if units.gain <= 0.0 || units.milliseconds <= 0.0 {
                continue;
            }
            return Ok(Some(Sensor {
                node: dev.join(name),
                dir,
                units,
            }));
        }
        Ok(None)
    }

    /// The sensor's name, to say which is heard.
    pub fn name(&self) -> String {
        self.dir
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default()
    }

    /// Opens the sensor's events, sets the window around the count now,
    /// and gives the light now.
    pub fn open(self) -> io::Result<(Window, f64)> {
        let node = OpenOptions::new().read(true).open(&self.node)?;
        let mut events: c_int = -1;
        // SAFETY: the ioctl writes one int, the events' descriptor, where
        // it's pointed, and the node stays open through the call.
        if unsafe { ioctl(node.as_raw_fd(), GET_EVENT_FD, &raw mut events) } < 0 || events < 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: the ioctl gave a new descriptor that nothing else owns.
        let events = unsafe { File::from_raw_fd(events) };
        // SAFETY: fcntl only reads and sets the open descriptor's flags.
        let flags = unsafe { fcntl(events.as_raw_fd(), GET_FLAGS) };
        if flags < 0 || unsafe { fcntl(events.as_raw_fd(), SET_FLAGS, flags | O_NONBLOCK) } < 0 {
            return Err(io::Error::last_os_error());
        }
        let lux = moved(&self.dir, self.units)?;
        let window = Window {
            events: AsyncFd::with_interest(events, Interest::READABLE)?,
            dir: self.dir,
            units: self.units,
        };
        Ok((window, lux))
    }
}

/// The light now, with the sensor's window moved around its count. The
/// falling threshold is set first when the light fell and the rising first
/// when it rose, so the two never cross on the way.
fn moved(dir: &Path, units: Units) -> io::Result<f64> {
    let count: u32 = number(&dir.join(COUNT))?;
    let (falling, rising) = window(count);
    let was: u32 = number(&dir.join(FALLING))?;
    let set = |path: &str, value: u32| fs::write(dir.join(path), value.to_string());
    if falling < was {
        set(FALLING, falling)?;
        set(RISING, rising)?;
    } else {
        set(RISING, rising)?;
        set(FALLING, falling)?;
    }
    Ok(units.lux(count))
}

pub struct Window {
    events: AsyncFd<File>,
    dir: PathBuf,
    units: Units,
}

impl Window {
    /// Waits for the light to leave the window, and says what it is now.
    pub async fn changed(&mut self) -> io::Result<f64> {
        let mut said = [0; EVENT * 8];
        loop {
            let mut ready = self.events.readable_mut().await?;
            match ready.try_io(|events| events.get_mut().read(&mut said)) {
                Ok(Ok(0)) => {
                    return Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "the light sensor went away",
                    ));
                }
                Ok(Ok(_)) => return moved(&self.dir, self.units),
                Ok(Err(error)) => return Err(error),
                Err(_) => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A folder of IIO devices, gone when the test is.
    struct Devices(PathBuf);

    impl Devices {
        fn new(test: &str) -> Self {
            let root = std::env::temp_dir().join(format!("windowed-{}-{test}", std::process::id()));
            let _ = fs::remove_dir_all(&root);
            fs::create_dir_all(&root).unwrap();
            Devices(root)
        }

        /// A colour sensor whose event is on or off, as udev left it.
        fn add(&self, name: &str, event: &str) -> PathBuf {
            let dir = self.0.join(name);
            fs::create_dir_all(dir.join("events")).unwrap();
            for (file, text) in [
                (COUNT, "120"),
                (EVENTS, event),
                (FALLING, "0"),
                (RISING, "65535"),
                ("in_intensity_scale", "4"),
                ("in_intensity_integration_time", "0.100000"),
            ] {
                fs::write(dir.join(file), format!("{text}\n")).unwrap();
            }
            dir
        }
    }

    impl Drop for Devices {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn a_colour_sensor_whose_event_udev_turned_on_is_found() {
        let devices = Devices::new("found");
        devices.add("iio:device0", "0");
        devices.add("iio:device1", "1");
        let sensor = Sensor::find(&devices.0, Path::new("/dev"))
            .unwrap()
            .expect("a colour sensor udev set up wasn't found");
        assert_eq!(sensor.name(), "iio:device1");
        assert_eq!(sensor.node, Path::new("/dev/iio:device1"));
        assert_eq!(
            sensor.units,
            Units {
                gain: 4.0,
                milliseconds: 100.0
            }
        );
    }

    #[test]
    fn a_colour_sensor_udev_left_alone_is_not_found() {
        let devices = Devices::new("alone");
        devices.add("iio:device0", "0");
        assert!(
            Sensor::find(&devices.0, Path::new("/dev"))
                .unwrap()
                .is_none(),
            "a sensor whose event is off would never wake the daemon"
        );
        assert!(
            Sensor::find(&devices.0.join("gone"), Path::new("/dev"))
                .unwrap()
                .is_none(),
            "no devices should be none found"
        );
    }

    #[test]
    fn a_count_turns_into_lux_by_its_gain_and_time() {
        let units = Units {
            gain: 4.0,
            milliseconds: 100.0,
        };
        assert!(
            (units.lux(400) - 140.0).abs() < 1e-9,
            "400 counts in 100 ms at a gain of 4 gave {} lux",
            units.lux(400)
        );
        assert!(
            units.lux(0).abs() < f64::EPSILON,
            "no counts should be no lux"
        );
    }

    #[test]
    fn the_window_is_a_share_of_the_count_or_the_least_either_side() {
        assert_eq!(window(1000), (850, 1150), "a lit room's window");
        assert_eq!(window(4), (1, 7), "a dark room's window, the least wide");
        assert_eq!(window(0), (0, 3), "no light, which can only rise");
        assert_eq!(
            window(MOST),
            (55_706, MOST),
            "the brightest, which can only fall"
        );
    }

    #[test]
    fn reading_moves_the_window_around_the_count() {
        let devices = Devices::new("moves");
        let dir = devices.add("iio:device0", "1");
        let units = Units {
            gain: 4.0,
            milliseconds: 100.0,
        };
        let lux = moved(&dir, units).unwrap();
        assert!((lux - 42.0).abs() < 1e-9, "120 counts gave {lux} lux");
        let falling = text(&dir.join(FALLING)).unwrap();
        let rising = text(&dir.join(RISING)).unwrap();
        assert_eq!((falling.as_str(), rising.as_str()), ("102", "138"));
    }
}
