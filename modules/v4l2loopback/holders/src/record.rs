//! How a USB camera streams, said in the watch's log every ten seconds
//! while it does, so a call that looked bad can be read back afterwards
//! rather than pieced together: who streams, the format and frame rate
//! they asked for, the frames the camera gave and any it sent broken, and
//! every control's value, exposure among them. It reads no frames.
//!
//! The frames come from uvcvideo's own count for each streaming interface,
//! in debugfs, which it starts afresh with each stream and counts as the
//! camera sends them, whether or not the program takes them. So a camera
//! that lowers its frame rate to gather light in a dim room shows here as
//! fewer frames than were asked for, beside the exposure it chose.

use std::fmt::Write;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::v4l2;

/// How often a streaming camera is said.
pub const EVERY: Duration = Duration::from_secs(10);

fn attribute(folder: &Path, name: &str) -> Option<String> {
    Some(
        fs::read_to_string(folder.join(name))
            .ok()?
            .trim()
            .to_owned(),
    )
}

/// uvcvideo's count for each of the camera's streaming interfaces, at
/// DEBUGFS/usb/uvcvideo/<bus>-<device>-<interface>/stats, found from the
/// node's interface in sysfs and its siblings.
pub fn counted(sysfs: &Path, debugfs: &Path, node: &Path) -> Vec<PathBuf> {
    let video = sysfs
        .join("class/video4linux")
        .join(node.file_name().unwrap_or_default());
    let Ok(interface) = fs::canonicalize(video.join("device")) else {
        return Vec::new();
    };
    let Some(device) = interface.parent() else {
        return Vec::new();
    };
    let (Some(bus), Some(number)) = (attribute(device, "busnum"), attribute(device, "devnum"))
    else {
        return Vec::new();
    };
    let Ok(interfaces) = fs::read_dir(device) else {
        return Vec::new();
    };
    let mut found: Vec<PathBuf> = interfaces
        .flatten()
        .map(|entry| entry.path())
        .filter(|folder| {
            attribute(folder, "bInterfaceClass").as_deref() == Some("0e")
                && attribute(folder, "bInterfaceSubClass").as_deref() == Some("02")
        })
        .filter_map(|folder| {
            let interface = attribute(&folder, "bInterfaceNumber")?;
            let interface = u32::from_str_radix(&interface, 16).ok()?;
            Some(
                debugfs
                    .join("usb/uvcvideo")
                    .join(format!("{bus}-{number}-{interface}"))
                    .join("stats"),
            )
        })
        .collect();
    found.sort();
    found
}

/// What uvcvideo has counted of a stream since it started.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Counts {
    pub frames: u64,
    pub errors: u64,
    pub invalid: u64,
}

/// The counts in uvcvideo's stats, which open with lines such as
/// `frames:  301`.
pub fn counts(stats: &str) -> Option<Counts> {
    let field = |name: &str| {
        stats.lines().find_map(|line| {
            let (key, value) = line.split_once(':')?;
            (key.trim() == name).then(|| value.trim().parse::<u64>().ok())?
        })
    };
    Some(Counts {
        frames: field("frames")?,
        errors: field("errors")?,
        invalid: field("invalid")?,
    })
}

/// A control's name as `v4l2-ctl` writes it: lower case, words joined by
/// underscores, so `Exposure Time, Absolute` is `exposure_time_absolute`.
pub fn called(name: &str) -> String {
    let mut called = String::new();
    for word in name
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
    {
        if !called.is_empty() {
            called.push('_');
        }
        called.push_str(&word.to_ascii_lowercase());
    }
    called
}

/// What the camera gave over a stretch: frames a second, and the frames it
/// sent broken or malformed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Gave {
    pub rate: f64,
    pub broken: u64,
}

/// What the camera gave between two counts of a stream. A count lower than
/// the last is a stream started afresh, which says nothing yet.
pub fn gave(before: Counts, now: Counts, over: Duration) -> Option<Gave> {
    let frames = now.frames.checked_sub(before.frames)?;
    let errors = now.errors.checked_sub(before.errors)?;
    let invalid = now.invalid.checked_sub(before.invalid)?;
    let seconds = over.as_secs_f64();
    if seconds <= 0.0 {
        return None;
    }
    #[expect(clippy::cast_precision_loss, reason = "a stretch's frames are few")]
    let rate = frames as f64 / seconds;
    Some(Gave {
        rate,
        broken: errors + invalid,
    })
}

/// The line said of a streaming camera.
pub fn line(
    holders: &[String],
    format: Option<&v4l2::Format>,
    asked: Option<f64>,
    gave: &[Option<Gave>],
    controls: &[v4l2::Value],
) -> String {
    // Writing to a String can't fail, so what write! returns is dropped.
    let mut said = format!("streams to {}", holders.join(", "));
    if let Some(format) = format {
        let _ = write!(
            said,
            "; {} {}x{}",
            format.pixels, format.width, format.height
        );
    }
    if let Some(asked) = asked {
        let _ = write!(said, " at {asked:.1} fps asked");
    }
    for stream in gave {
        match stream {
            Some(stream) => {
                let _ = write!(
                    said,
                    "; gave {:.1} fps, {} broken",
                    stream.rate, stream.broken
                );
            }
            None => said.push_str("; gave unknown"),
        }
    }
    if !controls.is_empty() {
        said.push(';');
        for control in controls {
            let _ = write!(said, " {}={}", called(&control.name), control.reads);
            if let Some(item) = &control.item {
                let _ = write!(said, " ({item})");
            }
        }
    }
    said
}

/// When a camera was last said, and uvcvideo's counts then.
pub struct Record {
    sysfs: PathBuf,
    debugfs: PathBuf,
    last: Option<(Instant, Vec<Option<Counts>>)>,
}

impl Record {
    pub fn new(sysfs: &Path, debugfs: &Path) -> Self {
        Self {
            sysfs: sysfs.to_owned(),
            debugfs: debugfs.to_owned(),
            last: None,
        }
    }

    /// While the camera streams, the line to say once ten seconds have
    /// passed since the last; the first of a stream counts from there. A
    /// camera not streaming starts the count afresh.
    pub fn now(&mut self, node: &Path, holders: &[String]) -> Option<String> {
        self.at(Instant::now(), node, holders)
    }

    fn at(&mut self, at: Instant, node: &Path, holders: &[String]) -> Option<String> {
        if holders.is_empty() {
            self.last = None;
            return None;
        }
        if let Some((then, _)) = &self.last
            && at.duration_since(*then) < EVERY
        {
            return None;
        }
        let counted: Vec<Option<Counts>> = counted(&self.sysfs, &self.debugfs, node)
            .iter()
            .map(|stats| counts(&fs::read_to_string(stats).ok()?))
            .collect();
        let (then, before) = self.last.replace((at, counted.clone()))?;
        let over = at.duration_since(then);
        let gave: Vec<Option<Gave>> = counted
            .iter()
            .enumerate()
            .map(|(index, now)| gave((*before.get(index)?)?, (*now)?, over))
            .collect();
        let file = File::open(node).ok();
        let format = file.as_ref().and_then(|file| v4l2::format(file).ok());
        let asked = file.as_ref().and_then(|file| v4l2::asked_rate(file).ok());
        let controls = file.as_ref().map(v4l2::controls).unwrap_or_default();
        Some(line(holders, format.as_ref(), asked, &gave, &controls))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camera::tests::Camera;

    const STATS: &str = "frames:  301\npackets: 9000\nempty:   12\nerrors:  2\ninvalid: 1\n\
                         pts: 301 early, 0 initial, 301 ok\n";

    #[test]
    fn the_counts_are_read_from_uvcvideos_stats() {
        assert_eq!(
            counts(STATS),
            Some(Counts {
                frames: 301,
                errors: 2,
                invalid: 1
            })
        );
        assert_eq!(counts("frames: 3\n"), None);
    }

    #[test]
    fn a_stream_is_found_by_its_bus_device_and_interface() {
        let camera = Camera::new("record-found", "MX Brio 705 for Business");
        let debugfs = camera.root.join("debug");
        assert_eq!(
            counted(&camera.sysfs, &debugfs, &camera.node),
            [debugfs.join("usb/uvcvideo/2-7-1/stats")]
        );
    }

    #[test]
    fn what_the_camera_gave_is_its_frames_over_the_stretch() {
        let at = |frames, errors, invalid| Counts {
            frames,
            errors,
            invalid,
        };
        assert_eq!(
            gave(at(100, 0, 0), at(250, 1, 2), Duration::from_secs(10)),
            Some(Gave {
                rate: 15.0,
                broken: 3
            })
        );
        assert_eq!(
            gave(at(300, 0, 0), at(20, 0, 0), Duration::from_secs(10)),
            None
        );
    }

    #[test]
    fn a_control_is_called_as_v4l2_ctl_calls_it() {
        assert_eq!(called("Exposure Time, Absolute"), "exposure_time_absolute");
        assert_eq!(
            called("Exposure, Dynamic Framerate"),
            "exposure_dynamic_framerate"
        );
        assert_eq!(
            called("White Balance Temperature"),
            "white_balance_temperature"
        );
    }

    #[test]
    fn the_line_says_who_streams_how_and_what_the_camera_gave() {
        let format = v4l2::Format {
            pixels: "MJPG".to_owned(),
            width: 1920,
            height: 1080,
        };
        let controls = [
            v4l2::Value {
                name: "Auto Exposure".to_owned(),
                reads: 3,
                item: Some("Aperture Priority Mode".to_owned()),
            },
            v4l2::Value {
                name: "Exposure, Dynamic Framerate".to_owned(),
                reads: 1,
                item: None,
            },
        ];
        let gave = [Some(Gave {
            rate: 14.96,
            broken: 0,
        })];
        assert_eq!(
            line(
                &["chrome".to_owned()],
                Some(&format),
                Some(30.0),
                &gave,
                &controls
            ),
            "streams to chrome; MJPG 1920x1080 at 30.0 fps asked; gave 15.0 fps, 0 broken; \
             auto_exposure=3 (Aperture Priority Mode) exposure_dynamic_framerate=1"
        );
    }

    #[test]
    fn a_camera_nobody_streams_from_says_nothing() {
        let camera = Camera::new("record-idle", "MX Brio 705 for Business");
        let mut record = Record::new(&camera.sysfs, &camera.root.join("debug"));
        assert_eq!(record.now(&camera.node, &[]), None);
    }

    #[test]
    fn a_stream_is_said_every_ten_seconds_once_it_has_been_counted() {
        let camera = Camera::new("record-every", "MX Brio 705 for Business");
        let debugfs = camera.root.join("debug");
        let stats = debugfs.join("usb/uvcvideo/2-7-1");
        fs::create_dir_all(&stats).unwrap();
        let count = |frames: u64, errors: u64| {
            let text = format!("frames:  {frames}\nerrors:  {errors}\ninvalid: 0\n");
            fs::write(stats.join("stats"), text).unwrap();
        };
        let mut record = Record::new(&camera.sysfs, &debugfs);
        let holders = ["chrome".to_owned()];
        let start = Instant::now();
        count(10, 0);
        assert_eq!(record.at(start, &camera.node, &holders), None);
        count(160, 0);
        let soon = start + Duration::from_secs(5);
        assert_eq!(record.at(soon, &camera.node, &holders), None);
        count(160, 2);
        let later = start + EVERY;
        // The stand-in node answers no V4L2 call, so only the count is said.
        assert_eq!(
            record.at(later, &camera.node, &holders).as_deref(),
            Some("streams to chrome; gave 15.0 fps, 2 broken")
        );
        assert_eq!(record.at(later, &camera.node, &[]), None);
        assert_eq!(record.at(later + EVERY, &camera.node, &holders), None);
    }
}
