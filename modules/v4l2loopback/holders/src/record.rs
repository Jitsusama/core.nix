//! How a USB camera streams, said in the watch's log every ten seconds
//! while it does, so a call that looked bad can be read back afterwards
//! rather than pieced together: who streams, the format and frame rate
//! they asked for, the frames the camera gave and the packets it sent in
//! error, and every control's value, exposure among them. It reads no
//! frames.
//!
//! The frames come from uvcvideo's own count for each streaming interface,
//! in debugfs, which it starts afresh with each stream and counts as the
//! camera sends them, whether or not the program takes them. So a camera
//! that lowers its frame rate to gather light in a dim room shows here as
//! fewer frames than were asked for, beside the exposure it chose. The
//! count is read at each of the watch's wakes, once a second while anyone
//! streams, and added up, so a program that stops and starts the stream
//! between two wakes, as a call does when it changes resolution, is still
//! counted whole rather than from its last start. A stream that a wake
//! sees stopped is said one last time, over what it gave since the last
//! line and ending `then stopped`, its time taken to that wake, up to a
//! second after; the next is counted from its own start, so no time
//! nobody streamed is counted as the camera's.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::count::{Gave, SHORTEST, Tally, counted, counts};
use crate::v4l2;

/// How often a streaming camera is said.
pub const EVERY: Duration = Duration::from_secs(10);

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

/// What the camera says it was asked for, without streaming from it.
#[derive(Default)]
pub struct Asked {
    /// Each left out when it isn't asked, as once the stream has stopped,
    /// and why it's unknown when it couldn't be read, so a call that stops
    /// reaching the camera shows in the line.
    pub format: Option<Result<v4l2::Format, String>>,
    pub rate: Option<Result<f64, String>>,
    pub controls: v4l2::Controls,
}

/// The line said of a streaming camera: who streams, each name quoted so
/// a program can't name itself into the figures, its streams by
/// interface, each named only when there are several, and what was asked
/// of it, or why that couldn't be read.
pub fn line(
    holders: &[String],
    asked: Result<&Asked, &str>,
    gave: &BTreeMap<u32, Result<Gave, String>>,
) -> String {
    // Writing to a String can't fail, so what write! returns is dropped.
    let mut said = named(holders);
    match asked {
        Ok(asked) => {
            let shown = matches!(asked.format, Some(Ok(_)));
            match &asked.format {
                Some(Ok(format)) => {
                    let _ = write!(
                        said,
                        "; {} {}x{}",
                        format.pixels, format.width, format.height
                    );
                }
                Some(Err(why)) => {
                    let _ = write!(said, "; format unknown: {why}");
                }
                None => {}
            }
            match &asked.rate {
                // The rate follows the format it was asked with, or stands
                // alone when that couldn't be said.
                Some(Ok(rate)) => {
                    let between = if shown { " at" } else { ";" };
                    let _ = write!(said, "{between} {rate:.1} fps asked");
                }
                Some(Err(why)) => {
                    let _ = write!(said, "; rate unknown: {why}");
                }
                None => {}
            }
        }
        Err(why) => {
            let _ = write!(said, "; can't ask the camera: {why}");
        }
    }
    if gave.is_empty() {
        said.push_str("; gave unknown: uvcvideo's count wasn't found");
    }
    for (interface, stream) in gave {
        said.push_str("; ");
        if gave.len() > 1 {
            let _ = write!(said, "interface {interface} ");
        }
        match stream {
            Ok(stream) => {
                let _ = write!(
                    said,
                    "gave {:.1} fps, {} packets in error",
                    stream.rate, stream.in_error
                );
                if stream.restarted {
                    said.push_str(", restarted");
                }
            }
            Err(why) => {
                let _ = write!(said, "gave unknown: {why}");
            }
        }
    }
    if let Ok(asked) = asked {
        let controls = &asked.controls;
        if !controls.values.is_empty() {
            said.push(';');
        }
        for control in &controls.values {
            let _ = write!(said, " {}=", called(&control.name));
            match control.reads {
                Ok(value) => {
                    let _ = write!(said, "{value}");
                }
                Err(why) => said.push_str(why),
            }
            // Quoted, as a holder is, since a mapping can name its items.
            if let Some(item) = &control.item {
                let _ = write!(said, " ({item:?})");
            }
        }
        if let Some(why) = &controls.cut_short {
            let _ = write!(said, "; controls cut short: {why}");
        }
    }
    said
}

/// Who streams, each name quoted.
fn named(holders: &[String]) -> String {
    let mut said = "streams to ".to_owned();
    for (index, holder) in holders.iter().enumerate() {
        let between = if index == 0 { "" } else { ", " };
        let _ = write!(said, "{between}{holder:?}");
    }
    said
}

/// A camera's stream as the record follows it, from when anyone streams
/// until a wake sees nobody does.
#[derive(Default)]
struct Stream {
    /// When its stretch since the last line began.
    since: Option<Instant>,
    /// The last wake that saw it streaming.
    seen: Option<Instant>,
    counted: BTreeMap<u32, PathBuf>,
    tallies: BTreeMap<u32, Tally>,
    /// Who streamed when it was last seen, for the line said once it stops.
    holders: Vec<String>,
    /// The node, held while the stream is, so asking it every ten seconds
    /// doesn't open it each time and wake the watch's own inotify.
    file: Option<File>,
}

impl Stream {
    /// Adds what each interface counted since the last wake.
    fn count(&mut self, at: Instant) {
        for (interface, stats) in &self.counted {
            let tally = self.tallies.entry(*interface).or_default();
            match fs::read_to_string(stats) {
                Ok(text) => match counts(&text) {
                    Some(counts) => tally.add(counts, at),
                    None => tally.unread = Some(format!("{} didn't read", stats.display())),
                },
                Err(error) => tally.unread = Some(format!("{}: {error}", stats.display())),
            }
        }
    }

    /// What each interface gave since the last line. Of several, one that
    /// gave nothing is left out while another gave frames, since a camera's
    /// other stream, unused, would read as a stall on every line.
    fn take(&mut self, at: Instant) -> BTreeMap<u32, Result<Gave, String>> {
        let mut gave: BTreeMap<_, _> = self
            .tallies
            .iter_mut()
            .map(|(interface, tally)| (*interface, tally.take(at)))
            .collect();
        let idle = |stream: &Result<Gave, String>| {
            stream
                .as_ref()
                .is_ok_and(|gave| gave.rate == 0.0 && gave.in_error == 0)
        };
        if gave.len() > 1 && !gave.values().all(idle) {
            gave.retain(|_, stream| !idle(stream));
        }
        gave
    }
}

/// What the record follows of a camera's stream.
pub struct Record {
    sysfs: PathBuf,
    debugfs: PathBuf,
    stream: Stream,
    /// The controls that didn't answer, kept for the stream, so each line
    /// doesn't wait on them again; the next stream asks them afresh, since
    /// a camera may time out only as a stream is set up.
    unanswered: BTreeSet<u32>,
}

impl Record {
    pub fn new(sysfs: &Path, debugfs: &Path) -> Self {
        Self {
            sysfs: sysfs.to_owned(),
            debugfs: debugfs.to_owned(),
            stream: Stream::default(),
            unanswered: BTreeSet::new(),
        }
    }

    /// While the camera streams, counts what it gave and, once ten seconds
    /// have passed since the last line, the line to say; the first of a
    /// stream counts from its first wake. Once it stops, the last line
    /// says so, the node is let go of, and the next stream starts afresh.
    pub fn now(&mut self, node: &Path, holders: &[String]) -> Option<String> {
        self.at(Instant::now(), node, holders)
    }

    /// The last line of a stream the watch ends during, saying what then
    /// happened, such as the camera going.
    pub fn ended(&mut self, then: &str) -> Option<String> {
        self.stop(then)
    }

    fn at(&mut self, at: Instant, node: &Path, holders: &[String]) -> Option<String> {
        if holders.is_empty() {
            return self.stop("stopped");
        }
        let stream = &mut self.stream;
        holders.clone_into(&mut stream.holders);
        let since = *stream.since.get_or_insert_with(|| {
            stream.counted = counted(&self.sysfs, &self.debugfs, node);
            self.unanswered.clear();
            at
        });
        stream.count(at);
        stream.seen = Some(at);
        if at.duration_since(since) < EVERY {
            return None;
        }
        stream.since = Some(at);
        let gave = stream.take(at);
        let file = match &mut stream.file {
            Some(file) => file,
            file @ None => match File::open(node) {
                Ok(opened) => file.insert(opened),
                Err(error) => return Some(line(holders, Err(&error.to_string()), &gave)),
            },
        };
        let asked = Asked {
            format: Some(v4l2::format(file).map_err(|error| error.to_string())),
            rate: Some(v4l2::asked_rate(file).map_err(|error| error.to_string())),
            controls: v4l2::controls(file, &mut self.unanswered),
        };
        Some(line(holders, Ok(&asked), &gave))
    }

    /// Says a stream followed has ended, with what it gave since its last
    /// line up to the last wake that saw it, so none of the time after it
    /// stopped is in the rate. A stretch under half a second is left out,
    /// since the watch's own open of the node wakes it a tenth of a second
    /// after a line, and a few frames over that would read as a stall.
    /// It's said even when nothing was counted, so a stream's end is told
    /// from the watch's.
    fn stop(&mut self, then: &str) -> Option<String> {
        let mut stopped = std::mem::take(&mut self.stream);
        let seen = stopped.seen?;
        stopped.tallies.retain(|_, tally| {
            tally.unread.is_some()
                || tally
                    .last
                    .is_some_and(|(_, from)| seen.saturating_duration_since(from) >= SHORTEST)
        });
        let gave = stopped.take(seen);
        let said = if gave.is_empty() && !stopped.counted.is_empty() {
            named(&stopped.holders)
        } else {
            line(&stopped.holders, Ok(&Asked::default()), &gave)
        };
        Some(format!("{said}; then {then}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camera::tests::Camera;
    use crate::count::tests::at;

    #[test]
    fn a_rate_whose_format_is_unknown_stands_alone() {
        let asked = Asked {
            format: Some(Err("Device or resource busy (os error 16)".to_owned())),
            rate: Some(Ok(30.0)),
            controls: v4l2::Controls::default(),
        };
        assert_eq!(
            line(&["chrome".to_owned()], Ok(&asked), &BTreeMap::new()),
            "streams to \"chrome\"; format unknown: Device or resource busy (os error 16); \
             30.0 fps asked; gave unknown: uvcvideo's count wasn't found"
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
        let asked = Asked {
            format: Some(Ok(v4l2::Format {
                pixels: "MJPG".to_owned(),
                width: 1920,
                height: 1080,
            })),
            rate: Some(Ok(30.0)),
            controls: v4l2::Controls {
                values: vec![
                    v4l2::Value {
                        name: "Auto Exposure".to_owned(),
                        reads: Ok(3),
                        item: Some("Aperture Priority Mode".to_owned()),
                    },
                    v4l2::Value {
                        name: "Exposure, Dynamic Framerate".to_owned(),
                        reads: Ok(1),
                        item: None,
                    },
                    v4l2::Value {
                        name: "Exposure Time, Absolute".to_owned(),
                        reads: Err("unanswered"),
                        item: None,
                    },
                ],
                cut_short: Some("Broken pipe (os error 32)".to_owned()),
            },
        };
        let gave = BTreeMap::from([(
            1,
            Ok(Gave {
                rate: 14.96,
                in_error: 0,
                restarted: false,
            }),
        )]);
        assert_eq!(
            line(&["chrome".to_owned()], Ok(&asked), &gave),
            "streams to \"chrome\"; MJPG 1920x1080 at 30.0 fps asked; \
             gave 15.0 fps, 0 packets in error; \
             auto_exposure=3 (\"Aperture Priority Mode\") exposure_dynamic_framerate=1 \
             exposure_time_absolute=unanswered; controls cut short: Broken pipe (os error 32)"
        );
    }

    #[test]
    fn a_camera_of_several_streams_says_each_by_its_interface_and_why_one_is_unknown() {
        let gave = BTreeMap::from([
            (
                1,
                Ok(Gave {
                    rate: 30.0,
                    in_error: 4,
                    restarted: true,
                }),
            ),
            (3, Err("Permission denied (os error 13)".to_owned())),
        ]);
        assert_eq!(
            line(&["chrome".to_owned()], Err("No such device"), &gave),
            "streams to \"chrome\"; can't ask the camera: No such device; \
             interface 1 gave 30.0 fps, 4 packets in error, restarted; \
             interface 3 gave unknown: Permission denied (os error 13)"
        );
        assert_eq!(
            line(
                &["chrome".to_owned()],
                Ok(&Asked::default()),
                &BTreeMap::new()
            ),
            "streams to \"chrome\"; gave unknown: uvcvideo's count wasn't found"
        );
    }

    #[test]
    fn a_holder_cant_name_itself_into_the_figures() {
        let holders = ["x; gave 30.0 fps".to_owned(), "zoom\u{202e}".to_owned()];
        assert_eq!(
            line(&holders, Ok(&Asked::default()), &BTreeMap::new()),
            "streams to \"x; gave 30.0 fps\", \"zoom\\u{202e}\"; \
             gave unknown: uvcvideo's count wasn't found"
        );
    }

    /// A stand-in camera, uvcvideo's count of its stream, and a record.
    fn streaming(test: &str) -> (Camera, PathBuf, Record) {
        let camera = Camera::new(test, "MX Brio 705 for Business");
        let debugfs = camera.root.join("debug");
        let stats = debugfs.join("usb/uvcvideo/2-7-10");
        fs::create_dir_all(&stats).unwrap();
        let record = Record::new(&camera.sysfs, &debugfs);
        (camera, stats.join("stats"), record)
    }

    /// What a node that answers no V4L2 call is said to have been asked.
    const UNASKED: &str = "; format unknown: Inappropriate ioctl for device (os error 25); \
                           rate unknown: Inappropriate ioctl for device (os error 25)";

    #[test]
    fn a_stream_with_no_count_found_is_said_to_end_all_the_same() {
        let (camera, _, mut record) = streaming("record-uncounted-end");
        // A node sysfs doesn't know has no streaming interface to count.
        let unknown = camera.root.join("dev/video9");
        let holders = ["chrome".to_owned()];
        let start = Instant::now();
        assert_eq!(record.at(start, &unknown, &holders), None);
        assert_eq!(
            record
                .at(start + Duration::from_secs(2), &unknown, &[])
                .as_deref(),
            Some(
                "streams to \"chrome\"; gave unknown: uvcvideo's count wasn't found; then stopped"
            )
        );
    }

    fn count(stats: &Path, frames: u64, errors: u64) {
        let text = format!("frames:  {frames}\nerrors:  {errors}\ninvalid: 0\n");
        fs::write(stats, text).unwrap();
    }

    #[test]
    fn a_stream_is_said_every_ten_seconds_once_it_has_been_counted() {
        let (camera, stats, mut record) = streaming("record-every");
        let holders = ["chrome".to_owned()];
        let start = Instant::now();
        count(&stats, 10, 0);
        assert_eq!(record.at(start, &camera.node, &holders), None);
        count(&stats, 160, 0);
        let soon = start + Duration::from_secs(5);
        assert_eq!(record.at(soon, &camera.node, &holders), None);
        count(&stats, 160, 2);
        let later = start + Duration::from_secs(12);
        // The stand-in node is a folder, which answers no V4L2 call, and
        // the line says so.
        assert_eq!(
            record.at(later, &camera.node, &holders),
            Some(format!(
                "streams to \"chrome\"{UNASKED}; gave 12.5 fps, 2 packets in error"
            ))
        );
        assert!(record.stream.file.is_some(), "the node isn't held");
        count(&stats, 190, 2);
        let last = later + Duration::from_secs(1);
        assert_eq!(record.at(last, &camera.node, &holders), None);
        // Nobody streams: what it gave up to the last wake that saw it is
        // said, none of the second since, and the node is let go of.
        let stopped = last + Duration::from_secs(1);
        assert_eq!(
            record.at(stopped, &camera.node, &[]).as_deref(),
            Some("streams to \"chrome\"; gave 30.0 fps, 0 packets in error; then stopped")
        );
        assert!(record.stream.file.is_none(), "the node is still held");
        assert_eq!(record.at(stopped, &camera.node, &[]), None);
        // An hour on, with no wake between, the next stream counts from its
        // own start rather than the last one's: carried on from 190, its
        // 300 would read 11.0 fps.
        count(&stats, 0, 0);
        let again = stopped + Duration::from_secs(3600);
        assert_eq!(record.at(again, &camera.node, &holders), None);
        count(&stats, 150, 0);
        let soon = again + Duration::from_secs(5);
        assert_eq!(record.at(soon, &camera.node, &holders), None);
        count(&stats, 300, 0);
        assert_eq!(
            record.at(again + EVERY, &camera.node, &holders),
            Some(format!(
                "streams to \"chrome\"{UNASKED}; gave 30.0 fps, 0 packets in error"
            ))
        );
    }

    #[test]
    fn a_stop_a_moment_after_a_line_gives_no_rate_for_the_moment() {
        let (camera, stats, mut record) = streaming("record-moment");
        let holders = ["chrome".to_owned()];
        let start = Instant::now();
        count(&stats, 10, 0);
        assert_eq!(record.at(start, &camera.node, &holders), None);
        count(&stats, 310, 0);
        let line = start + EVERY;
        assert!(record.at(line, &camera.node, &holders).is_some(), "no line");
        // The watch's own open wakes it a tenth of a second on, and two
        // frames there would read 20.0 fps.
        count(&stats, 312, 0);
        let woken = line + Duration::from_millis(100);
        assert_eq!(record.at(woken, &camera.node, &holders), None);
        assert_eq!(
            record
                .at(woken + Duration::from_secs(1), &camera.node, &[])
                .as_deref(),
            Some("streams to \"chrome\"; then stopped")
        );
    }

    #[test]
    fn a_new_stream_asks_again_what_the_last_found_unanswered() {
        let (camera, stats, mut record) = streaming("record-asks-again");
        record.unanswered.insert(0x0098_0900);
        count(&stats, 10, 0);
        let holders = ["chrome".to_owned()];
        let start = Instant::now();
        assert_eq!(record.at(start, &camera.node, &holders), None);
        assert_eq!(record.unanswered.len(), 0);
        // Within the stream, one found unanswered stays so at each wake.
        record.unanswered.insert(0x0098_0900);
        count(&stats, 40, 0);
        let wake = start + Duration::from_secs(1);
        assert_eq!(record.at(wake, &camera.node, &holders), None);
        assert!(
            record.at(start + EVERY, &camera.node, &holders).is_some(),
            "no line"
        );
        assert_eq!(record.unanswered.len(), 1);
    }

    #[test]
    fn a_second_stream_left_unused_isnt_said_to_stall() {
        let start = Instant::now();
        let later = start + EVERY;
        let mut stream = Stream::default();
        for (interface, frames) in [(1, 300), (3, 0)] {
            let tally = stream.tallies.entry(interface).or_default();
            tally.add(at(0, 0, 0), start);
            tally.add(at(frames, 0, 0), later);
        }
        let gave = stream.take(later);
        assert_eq!(gave.keys().collect::<Vec<_>>(), [&1]);
        // Both giving nothing is said of both.
        let gave = stream.take(later + EVERY);
        assert_eq!(gave.keys().collect::<Vec<_>>(), [&1, &3]);
    }

    #[test]
    fn a_stopped_stream_is_said_and_the_next_counts_from_its_own_start() {
        let (camera, stats, mut record) = streaming("record-stopped");
        let holders = ["chrome".to_owned()];
        let start = Instant::now();
        count(&stats, 0, 0);
        assert_eq!(record.at(start, &camera.node, &holders), None);
        // A stream seen stopped before it was counted says only that.
        assert_eq!(
            record.at(start, &camera.node, &[]).as_deref(),
            Some("streams to \"chrome\"; then stopped")
        );
        assert_eq!(record.at(start, &camera.node, &holders), None);
        count(&stats, 120, 0);
        let last = start + Duration::from_secs(4);
        assert_eq!(record.at(last, &camera.node, &holders), None);
        let stopped = last + Duration::from_secs(1);
        assert_eq!(
            record.at(stopped, &camera.node, &[]).as_deref(),
            Some("streams to \"chrome\"; gave 30.0 fps, 0 packets in error; then stopped")
        );
        // It starts again at a new size, and uvcvideo counts afresh, past
        // where the last stream's count was; none of the pause is counted.
        let again = stopped + Duration::from_secs(1);
        count(&stats, 10, 0);
        assert_eq!(record.at(again, &camera.node, &holders), None);
        count(&stats, 310, 0);
        assert_eq!(
            record.at(again + EVERY, &camera.node, &holders),
            Some(format!(
                "streams to \"chrome\"{UNASKED}; gave 30.0 fps, 0 packets in error"
            ))
        );
    }

    #[test]
    fn a_count_that_cant_be_read_says_where_it_was_looked_for() {
        let (camera, stats, mut record) = streaming("record-unread");
        let holders = ["chrome".to_owned()];
        let start = Instant::now();
        assert_eq!(record.at(start, &camera.node, &holders), None);
        let said = record.at(start + EVERY, &camera.node, &holders).unwrap();
        let missing = format!("gave unknown: {}: No such file", stats.display());
        assert!(said.contains(&missing), "{said}");
        // Once it can be read, the next stretch counts from there.
        count(&stats, 0, 0);
        let later = start + EVERY + Duration::from_secs(1);
        assert_eq!(record.at(later, &camera.node, &holders), None);
        count(&stats, 300, 0);
        let said = record.at(later + EVERY, &camera.node, &holders).unwrap();
        assert!(said.contains("gave 30.0 fps"), "{said}");
    }

    #[test]
    fn a_stream_is_said_to_end_though_its_count_cant_be_read() {
        let (camera, stats, mut record) = streaming("record-unread-end");
        let holders = ["chrome".to_owned()];
        let start = Instant::now();
        assert_eq!(record.at(start, &camera.node, &holders), None);
        let soon = start + Duration::from_secs(3);
        assert_eq!(record.at(soon, &camera.node, &holders), None);
        let said = record.at(soon, &camera.node, &[]).unwrap();
        let missing = format!("gave unknown: {}: No such file", stats.display());
        assert!(said.contains(&missing), "{said}");
        assert!(said.ends_with("; then stopped"), "{said}");
    }

    #[test]
    fn a_stream_the_watch_ends_during_says_what_it_gave_and_what_happened() {
        let (camera, stats, mut record) = streaming("record-ended");
        let holders = ["chrome".to_owned()];
        let start = Instant::now();
        assert_eq!(record.ended("the camera went"), None);
        count(&stats, 0, 0);
        assert_eq!(record.at(start, &camera.node, &holders), None);
        count(&stats, 60, 0);
        let last = start + Duration::from_secs(2);
        assert_eq!(record.at(last, &camera.node, &holders), None);
        assert_eq!(
            record.ended("the camera went").as_deref(),
            Some("streams to \"chrome\"; gave 30.0 fps, 0 packets in error; then the camera went")
        );
        assert_eq!(record.ended("the camera went"), None);
    }
}
