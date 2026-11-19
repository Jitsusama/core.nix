//! uvcvideo's own count of what a camera sent, for the record: where it
//! is in debugfs for each streaming interface, what it says, and the
//! frames and packets in error added up over a stretch between lines.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// The shortest stretch a rate is given for, since a few frames over a
/// moment would read as a stall.
pub const SHORTEST: Duration = Duration::from_millis(500);

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
/// node's interface in sysfs and its siblings, by interface number.
pub fn counted(sysfs: &Path, debugfs: &Path, node: &Path) -> BTreeMap<u32, PathBuf> {
    let video = sysfs
        .join("class/video4linux")
        .join(node.file_name().unwrap_or_default());
    let Ok(interface) = fs::canonicalize(video.join("device")) else {
        return BTreeMap::new();
    };
    let Some(device) = interface.parent() else {
        return BTreeMap::new();
    };
    let (Some(bus), Some(number)) = (attribute(device, "busnum"), attribute(device, "devnum"))
    else {
        return BTreeMap::new();
    };
    let Ok(interfaces) = fs::read_dir(device) else {
        return BTreeMap::new();
    };
    interfaces
        .flatten()
        .map(|entry| entry.path())
        .filter(|folder| {
            attribute(folder, "bInterfaceClass").as_deref() == Some("0e")
                && attribute(folder, "bInterfaceSubClass").as_deref() == Some("02")
        })
        .filter_map(|folder| {
            // sysfs gives the number in hex, and uvcvideo names it in decimal.
            let interface = attribute(&folder, "bInterfaceNumber")?;
            let interface = u32::from_str_radix(&interface, 16).ok()?;
            let stats = debugfs
                .join("usb/uvcvideo")
                .join(format!("{bus}-{number}-{interface}"))
                .join("stats");
            Some((interface, stats))
        })
        .collect()
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

/// One streaming interface's count, added up over a stretch.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Tally {
    /// The count last read, and when it was first read this stretch.
    pub last: Option<(Counts, Instant)>,
    frames: u64,
    /// uvcvideo counts a packet in error, or one too short for its header,
    /// rather than a frame.
    in_error: u64,
    restarted: bool,
    /// Why it couldn't be read, when it couldn't.
    pub unread: Option<String>,
}

impl Tally {
    /// Adds what was counted since the last read. A count lower than the
    /// last is a stream started afresh, whose count is all new. One that
    /// started afresh and passed the last read by the next wake, as a
    /// stream under a second old can, isn't told from one going on.
    pub fn add(&mut self, now: Counts, at: Instant) {
        self.unread = None;
        let Some((last, from)) = self.last else {
            self.last = Some((now, at));
            return;
        };
        let since =
            if now.frames < last.frames || now.errors < last.errors || now.invalid < last.invalid {
                self.restarted = true;
                now
            } else {
                Counts {
                    frames: now.frames - last.frames,
                    errors: now.errors - last.errors,
                    invalid: now.invalid - last.invalid,
                }
            };
        self.frames += since.frames;
        self.in_error += since.errors + since.invalid;
        self.last = Some((now, from));
    }

    /// What the stream gave up to now, and the tally started afresh from
    /// the count it was at. A count that couldn't be read now leaves the
    /// next stretch to start from its own first read, since the last one
    /// read is from an earlier wake.
    pub fn take(&mut self, at: Instant) -> Result<Gave, String> {
        let taken = std::mem::take(self);
        if let Some(why) = taken.unread {
            return Err(why);
        }
        self.last = taken.last.map(|(counts, _)| (counts, at));
        let Some((_, from)) = taken.last else {
            return Err("not counted yet".to_owned());
        };
        let stretch = at.duration_since(from);
        if stretch.is_zero() {
            return Err("not counted yet".to_owned());
        }
        // As a stretch begun by a read just before the line, after one
        // that failed.
        if stretch < SHORTEST {
            return Err("counted for under half a second".to_owned());
        }
        let seconds = stretch.as_secs_f64();
        #[expect(clippy::cast_precision_loss, reason = "a stretch's frames are few")]
        let rate = taken.frames as f64 / seconds;
        Ok(Gave {
            rate,
            in_error: taken.in_error,
            restarted: taken.restarted,
        })
    }
}

/// What the camera gave over a stretch: frames a second, the packets it
/// sent in error, and whether the stream started afresh within it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Gave {
    pub rate: f64,
    pub in_error: u64,
    pub restarted: bool,
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::camera::tests::Camera;

    const STATS: &str = "frames:  301\npackets: 9000\nempty:   12\nerrors:  2\ninvalid: 1\n\
                         pts: 301 early, 0 initial, 301 ok\n";

    /// A count of frames, packets in error and packets too short.
    pub fn at(frames: u64, errors: u64, invalid: u64) -> Counts {
        Counts {
            frames,
            errors,
            invalid,
        }
    }

    #[test]
    fn the_counts_are_read_from_uvcvideos_stats() {
        assert_eq!(counts(STATS), Some(at(301, 2, 1)));
        assert_eq!(counts("frames: 3\n"), None);
    }

    #[test]
    fn a_stream_is_found_by_its_bus_device_and_interface_in_decimal() {
        let camera = Camera::new("record-found", "MX Brio 705 for Business");
        let debugfs = camera.root.join("debug");
        // The streaming interface is 0a in sysfs.
        assert_eq!(
            counted(&camera.sysfs, &debugfs, &camera.node),
            BTreeMap::from([(10, debugfs.join("usb/uvcvideo/2-7-10/stats"))])
        );
    }

    #[test]
    fn what_the_camera_gave_is_its_frames_over_the_stretch() {
        let start = Instant::now();
        let mut tally = Tally::default();
        tally.add(at(100, 0, 0), start);
        tally.add(at(250, 1, 2), start + Duration::from_secs(10));
        assert_eq!(
            tally.take(start + Duration::from_secs(10)),
            Ok(Gave {
                rate: 15.0,
                in_error: 3,
                restarted: false
            })
        );
    }

    #[test]
    fn a_stream_started_afresh_within_a_stretch_is_counted_whole() {
        let start = Instant::now();
        let mut tally = Tally::default();
        tally.add(at(100, 0, 0), start);
        tally.add(at(200, 0, 0), start + Duration::from_secs(4));
        // It started again, and has counted 120 since.
        tally.add(at(120, 0, 0), start + Duration::from_secs(8));
        assert_eq!(
            tally.take(start + Duration::from_secs(8)),
            Ok(Gave {
                rate: 27.5,
                in_error: 0,
                restarted: true
            })
        );
        // The next stretch counts on from where this one ended.
        tally.add(at(420, 0, 0), start + Duration::from_secs(18));
        assert_eq!(
            tally.take(start + Duration::from_secs(18)),
            Ok(Gave {
                rate: 30.0,
                in_error: 0,
                restarted: false
            })
        );
    }

    #[test]
    fn a_count_unread_at_a_line_leaves_the_next_to_count_from_its_own_read() {
        let start = Instant::now();
        let mut tally = Tally::default();
        tally.add(at(0, 0, 0), start);
        tally.add(at(270, 0, 0), start + Duration::from_secs(9));
        tally.unread = Some("stats: busy".to_owned());
        let line = start + Duration::from_secs(10);
        assert_eq!(tally.take(line), Err("stats: busy".to_owned()));
        // Rebased to the read at nine seconds, the next would give 33.0 fps.
        tally.add(at(330, 0, 0), start + Duration::from_secs(11));
        tally.add(at(600, 0, 0), start + Duration::from_secs(20));
        assert_eq!(
            tally.take(start + Duration::from_secs(20)),
            Ok(Gave {
                rate: 30.0,
                in_error: 0,
                restarted: false
            })
        );
    }

    #[test]
    fn a_stretch_of_a_moment_gives_no_rate() {
        let start = Instant::now();
        let mut tally = Tally::default();
        tally.add(at(0, 0, 0), start);
        tally.add(at(2, 0, 0), start + Duration::from_millis(100));
        assert_eq!(
            tally.take(start + Duration::from_millis(100)),
            Err("counted for under half a second".to_owned())
        );
    }
}
