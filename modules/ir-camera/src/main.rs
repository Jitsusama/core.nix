//! ir-camera relays an IR camera's raw greyscale frames to a v4l2loopback
//! camera as 8-bit grey, the one format every program that opens a camera
//! knows. An IR sensor behind a camera pipeline like Intel's IPU sends
//! 10-bit samples in lines padded for its DMA, on a node that also offers
//! colour formats the sensor can't send, so a program asking that node for
//! its usual format gets one that fails as it starts. The loopback offers
//! only grey, at the sensor's size.
//!
//! The camera streams only while a program reads. The relay waits on the
//! loopback's event saying a reader started or stopped, and keeps the
//! camera open only while one reads. When the reader stops, the relay closes
//! it, which powers it down and turns its IR light off, and blanks the
//! loopback, so the next reader never sees the last one's face.
//!
//! ir-camera --source PATH --sink PATH --format Y10|Y12|Y16 --width PIXELS
//! --height PIXELS [--prepare PROGRAM]
//!
//! The sink is the loopback, and the prepare program runs before each
//! stream: for a camera behind a media pipeline, it wires the pipeline, and
//! it may make the source a link to the node the pipeline ends at.

mod grey;
mod v4l2;

use grey::{Raw, Shape};
use std::io;
use std::path::PathBuf;
use std::process::{Command, ExitCode};
use v4l2::{Capture, Device, PixFormat};

/// How many buffers the camera streams into.
const CAMERA_BUFFERS: u32 = 4;

/// v4l2loopback keeps at most this many frames, so writing this many
/// blank ones leaves none of the last stream's.
const LOOPBACK_FRAMES: usize = 32;

#[derive(Debug, PartialEq)]
struct Settings {
    source: PathBuf,
    sink: PathBuf,
    format: Raw,
    width: u32,
    height: u32,
    prepare: Option<PathBuf>,
}

fn settings(mut arguments: impl Iterator<Item = String>) -> Result<Settings, String> {
    let (mut source, mut sink, mut format, mut width, mut height, mut prepare) =
        (None, None, None, None, None, None);
    while let Some(flag) = arguments.next() {
        let value = arguments
            .next()
            .ok_or_else(|| format!("{flag} needs a value"))?;
        let size = |value: &str| {
            value
                .parse::<u32>()
                .ok()
                .filter(|size| *size > 0)
                .ok_or_else(|| format!("{flag} takes a number of pixels, not {value}"))
        };
        match flag.as_str() {
            "--source" => source = Some(PathBuf::from(value)),
            "--sink" => sink = Some(PathBuf::from(value)),
            "--format" => {
                format = Some(
                    Raw::named(&value)
                        .ok_or_else(|| format!("--format takes Y10, Y12 or Y16, not {value}"))?,
                )
            }
            "--width" => width = Some(size(&value)?),
            "--height" => height = Some(size(&value)?),
            "--prepare" => prepare = Some(PathBuf::from(value)),
            _ => return Err(format!("{flag} isn't a setting")),
        }
    }
    Ok(Settings {
        source: source.ok_or("--source is needed")?,
        sink: sink.ok_or("--sink is needed")?,
        format: format.ok_or("--format is needed")?,
        width: width.ok_or("--width is needed")?,
        height: height.ok_or("--height is needed")?,
        prepare,
    })
}

/// The loopback the relay writes grey frames to.
struct Sink {
    device: Device,
    shape: Shape,
    blank: Vec<u8>,
}

impl Sink {
    fn open(settings: &Settings) -> Result<Self, String> {
        let path = settings.sink.display();
        let device = Device::open(&settings.sink).map_err(|error| format!("{path}: {error}"))?;
        let wanted = PixFormat {
            width: settings.width,
            height: settings.height,
            pixelformat: v4l2::GREY,
            bytesperline: settings.width,
            sizeimage: settings.width * settings.height,
            ..PixFormat::default()
        };
        let format = device
            .set_format(v4l2::OUTPUT, wanted)
            .map_err(|error| format!("{path} takes no grey frames: {error}"))?;
        if (format.pixelformat, format.width, format.height)
            != (v4l2::GREY, wanted.width, wanted.height)
        {
            return Err(format!("{path} takes {format:?} rather than {wanted:?}"));
        }
        let shape = Shape {
            width: format.width as usize,
            height: format.height as usize,
            stride: (format.bytesperline.max(format.width)) as usize,
        };
        let length = (format.sizeimage as usize).max(shape.stride * shape.height);
        let mut sink = Self {
            device,
            shape,
            blank: vec![0; length],
        };
        // A first frame lets readers start, and the event says when they do.
        sink.blank().map_err(|error| format!("{path}: {error}"))?;
        sink.device
            .subscribe(v4l2::CLIENT_USAGE)
            .map_err(|error| format!("{path} says nothing of its readers: {error}"))?;
        Ok(sink)
    }

    fn blank(&mut self) -> io::Result<()> {
        for _ in 0..LOOPBACK_FRAMES {
            self.device.write_frame(&self.blank)?;
        }
        Ok(())
    }

    /// Whether a reader is streaming, as the latest of the events waiting
    /// says, if any are.
    fn reading(&self) -> io::Result<Option<bool>> {
        let mut reading = None;
        while let Some((kind, count)) = self.device.event()? {
            if kind == v4l2::CLIENT_USAGE {
                reading = Some(count > 0);
            }
        }
        Ok(reading)
    }
}

/// Which side of the relay an error came from.
enum Failed {
    Camera(io::Error),
    Sink(io::Error),
}

/// The camera, while a reader reads it.
struct Stream {
    capture: Capture,
    shape: Shape,
    grey: Vec<u8>,
    // A frame that's short is likely the next one too, so it's said once.
    said_short: bool,
}

impl Stream {
    fn start(settings: &Settings, sink: &Sink) -> Option<Self> {
        if let Some(prepare) = &settings.prepare {
            let program = prepare.display();
            match Command::new(prepare).status() {
                Ok(status) if status.success() => {}
                Ok(status) => {
                    eprintln!("{program} failed ({status}), so the camera stays off.");
                    return None;
                }
                Err(error) => {
                    eprintln!("{program} wouldn't run ({error}), so the camera stays off.");
                    return None;
                }
            }
        }
        let wanted = PixFormat {
            width: settings.width,
            height: settings.height,
            pixelformat: settings.format.fourcc(),
            ..PixFormat::default()
        };
        match Capture::start(&settings.source, wanted, CAMERA_BUFFERS) {
            Ok(capture) => {
                eprintln!("A reader started, so the camera is on.");
                let shape = Shape {
                    width: capture.format.width as usize,
                    height: capture.format.height as usize,
                    stride: (capture.format.bytesperline.max(capture.format.width * 2)) as usize,
                };
                let grey = sink.blank.clone();
                Some(Self {
                    capture,
                    shape,
                    grey,
                    said_short: false,
                })
            }
            Err(error) => {
                eprintln!(
                    "The camera at {} won't stream: {error}.",
                    settings.source.display()
                );
                None
            }
        }
    }

    /// Passes the camera's next frame on, if it has one.
    fn forward(&mut self, format: Raw, sink: &mut Sink) -> Result<(), Failed> {
        let mut converted = Ok(());
        let (shape, grey) = (self.shape, &mut self.grey);
        let had = self
            .capture
            .next(|raw| {
                converted = grey::convert(format, raw, shape, grey, sink.shape);
            })
            .map_err(Failed::Camera)?;
        match converted {
            Ok(()) if had => sink.device.write_frame(&self.grey).map_err(Failed::Sink),
            Ok(()) => Ok(()),
            Err(short) => {
                if !self.said_short {
                    eprintln!("The camera sent {short}, so its frames are dropped.");
                    self.said_short = true;
                }
                Ok(())
            }
        }
    }
}

/// Waits until one of `fds` has what it asked for.
fn wait(fds: &mut [libc::pollfd]) -> io::Result<()> {
    loop {
        // SAFETY: fds is a slice of pollfd, this long.
        let result = unsafe { libc::poll(fds.as_mut_ptr(), fds.len() as libc::nfds_t, -1) };
        if result >= 0 {
            return Ok(());
        }
        let error = io::Error::last_os_error();
        if error.kind() != io::ErrorKind::Interrupted {
            return Err(error);
        }
    }
}

fn relay(settings: &Settings) -> Result<(), String> {
    let mut sink = Sink::open(settings)?;
    let mut stream: Option<Stream> = None;
    loop {
        let mut fds = [
            libc::pollfd {
                fd: sink.device.fd(),
                events: libc::POLLPRI,
                revents: 0,
            },
            libc::pollfd {
                fd: stream.as_ref().map_or(-1, |stream| stream.capture.fd()),
                events: libc::POLLIN,
                revents: 0,
            },
        ];
        wait(&mut fds).map_err(|error| format!("waiting failed: {error}"))?;
        let sink_gone = |error: io::Error| format!("{}: {error}", settings.sink.display());
        if fds[0].revents & (libc::POLLERR | libc::POLLHUP | libc::POLLNVAL) != 0 {
            return Err(format!("{} is gone", settings.sink.display()));
        }
        if fds[0].revents & libc::POLLPRI != 0 {
            match (sink.reading().map_err(sink_gone)?, stream.is_some()) {
                (Some(true), false) => stream = Stream::start(settings, &sink),
                (Some(false), true) => {
                    stream = None;
                    sink.blank().map_err(sink_gone)?;
                    eprintln!("The reader stopped, so the camera is off.");
                }
                _ => {}
            }
        }
        let Some(camera) = stream.as_mut() else {
            continue;
        };
        let revents = fds[1].revents;
        let failed = if revents & libc::POLLIN != 0 {
            match camera.forward(settings.format, &mut sink) {
                Ok(()) => None,
                Err(Failed::Camera(error)) => Some(error.to_string()),
                Err(Failed::Sink(error)) => return Err(sink_gone(error)),
            }
        } else if revents & (libc::POLLERR | libc::POLLHUP | libc::POLLNVAL) != 0 {
            Some("it stopped streaming".to_string())
        } else {
            None
        };
        if let Some(reason) = failed {
            // Until the next reader: retrying at once could loop as fast as
            // the camera fails.
            eprintln!("The camera failed ({reason}), so it's off until the next reader.");
            stream = None;
            sink.blank().map_err(sink_gone)?;
        }
    }
}

fn main() -> ExitCode {
    let result = settings(std::env::args().skip(1)).and_then(|settings| relay(&settings));
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("ir-camera: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(arguments: &[&str]) -> Result<Settings, String> {
        settings(arguments.iter().map(|argument| argument.to_string()))
    }

    const ALL: [&str; 10] = [
        "--source",
        "/run/ir/source",
        "--sink",
        "/dev/video9",
        "--format",
        "Y10",
        "--width",
        "648",
        "--height",
        "368",
    ];

    #[test]
    fn reads_its_settings() {
        let settings = parse(&[&ALL[..], &["--prepare", "/bin/wire"]].concat()).unwrap();
        assert_eq!(
            settings,
            Settings {
                source: "/run/ir/source".into(),
                sink: "/dev/video9".into(),
                format: Raw::Y10,
                width: 648,
                height: 368,
                prepare: Some("/bin/wire".into()),
            }
        );
        assert_eq!(parse(&ALL).unwrap().prepare, None);
    }

    #[test]
    fn needs_each_setting_but_prepare() {
        for missing in ["--source", "--sink", "--format", "--width", "--height"] {
            let index = ALL.iter().position(|flag| *flag == missing).unwrap();
            let without = [&ALL[..index], &ALL[index + 2..]].concat();
            assert_eq!(parse(&without), Err(format!("{missing} is needed")));
        }
    }

    #[test]
    fn refuses_what_it_cannot_use() {
        let with = |flag: &str, value: &str| {
            let mut arguments = ALL.to_vec();
            let index = arguments.iter().position(|each| *each == flag).unwrap();
            arguments[index + 1] = value;
            parse(&arguments)
        };
        assert_eq!(
            with("--format", "GREY"),
            Err("--format takes Y10, Y12 or Y16, not GREY".into())
        );
        assert_eq!(
            with("--width", "0"),
            Err("--width takes a number of pixels, not 0".into())
        );
        assert_eq!(
            with("--height", "tall"),
            Err("--height takes a number of pixels, not tall".into())
        );
        assert_eq!(
            parse(&["--colour", "red"]),
            Err("--colour isn't a setting".into())
        );
        assert_eq!(parse(&["--source"]), Err("--source needs a value".into()));
    }
}
