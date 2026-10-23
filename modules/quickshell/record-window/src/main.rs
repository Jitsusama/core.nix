//! record-window records one window into a video, for the shell's recorder
//! (Recorder.qml).
//!
//! wf-recorder records through wlr-screencopy, which takes only the screen or
//! a part of it, and niri says where a tiled window sits only to a
//! screencast. So this asks niri for the window's own screencast, over the
//! `ScreenCast` interface niri shares with GNOME's Mutter, as a call does
//! through the portal, and hands the PipeWire stream that comes of it to
//! GStreamer. That encodes it on the GPU through VA-API where Intel's or
//! AMD's driver runs it, as the screen's recordings are, and on the processor
//! elsewhere. The window is fitted into the size asked for, bordered, so the
//! video keeps one size as the window changes its own.
//!
//! While it records, it says so in `$XDG_RUNTIME_DIR/record-window.json`,
//! `{"pid", "session"}`: which of niri's casts is a recording rather than a
//! call's share, and which process to stop. SIGINT or SIGTERM finishes the
//! file, then ends the cast. Should the window close first, niri ends the
//! cast itself; the file is written in one-second fragments, so what was
//! recorded until then still plays.
//!
//! It exits 0 once the file is saved, 3 when it was stopped before anything
//! was recorded, 2 when it was asked wrongly, and otherwise 1, saying why on
//! standard error, with no file left behind.
//!
//! Usage: record-window WINDOW WIDTH HEIGHT FILE

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{ExitCode, Stdio};
use std::time::Duration;
use std::{env, fs, process};

use futures_util::StreamExt;
use tokio::io::AsyncReadExt;
use tokio::process::Command;
use tokio::signal::unix::{Signal, SignalKind, signal};
use tokio::time::{Instant, sleep_until};
use zbus::zvariant::{OwnedObjectPath, Value};
use zbus::{Connection, proxy};

#[proxy(
    interface = "org.gnome.Mutter.ScreenCast",
    default_service = "org.gnome.Mutter.ScreenCast",
    default_path = "/org/gnome/Mutter/ScreenCast"
)]
trait ScreenCast {
    fn create_session(&self, properties: HashMap<&str, Value<'_>>)
    -> zbus::Result<OwnedObjectPath>;
}

#[proxy(
    interface = "org.gnome.Mutter.ScreenCast.Session",
    default_service = "org.gnome.Mutter.ScreenCast"
)]
trait Session {
    fn record_window(&self, properties: HashMap<&str, Value<'_>>) -> zbus::Result<OwnedObjectPath>;
    fn start(&self) -> zbus::Result<()>;
    fn stop(&self) -> zbus::Result<()>;
    #[zbus(signal)]
    fn closed(&self) -> zbus::Result<()>;
}

#[proxy(
    interface = "org.gnome.Mutter.ScreenCast.Stream",
    default_service = "org.gnome.Mutter.ScreenCast"
)]
trait Stream {
    #[zbus(signal)]
    fn pipe_wire_stream_added(&self, node_id: u32) -> zbus::Result<()>;
}

/// How long GStreamer has to finish the file once it's told to.
const FINISHING: Duration = Duration::from_secs(10);

const USAGE: &str = "usage: record-window WINDOW WIDTH HEIGHT FILE";

/// What was asked for: niri's id for the window, the video's size in pixels
/// and where it goes.
#[derive(Debug, PartialEq, Eq)]
struct Asked {
    window: u64,
    width: u32,
    height: u32,
    file: PathBuf,
}

impl Asked {
    fn parse(args: &[String]) -> Result<Self, String> {
        let [window, width, height, file] = args else {
            return Err(USAGE.to_owned());
        };
        let number = |what: &str, given: &str| {
            given
                .parse::<u64>()
                .map_err(|_| format!("the {what} {given:?} isn't a number"))
        };
        Ok(Self {
            window: number("window", window)?,
            width: even(number("width", width)?),
            height: even(number("height", height)?),
            file: PathBuf::from(file),
        })
    }
}

/// The even size at or just below, and at least 2, since the encoders take
/// only even sizes.
fn even(size: u64) -> u32 {
    u32::try_from(size.max(2) & !1).unwrap_or(u32::MAX - 1)
}

/// Where the video is encoded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Encoder {
    Gpu,
    Processor,
}

impl Encoder {
    /// The GPU where the render node's driver is Intel's or AMD's, which
    /// encode through VA-API, as the screen's recordings choose.
    fn for_driver(driver: Option<&str>) -> Self {
        match driver {
            Some("i915" | "xe" | "amdgpu") => Self::Gpu,
            _ => Self::Processor,
        }
    }

    fn here() -> Self {
        let driver = fs::read_link("/sys/class/drm/renderD128/device/driver").ok();
        Self::for_driver(
            driver
                .as_deref()
                .and_then(Path::file_name)
                .and_then(|name| name.to_str()),
        )
    }
}

/// gst-launch's arguments: niri's stream, fitted to the size and encoded,
/// into the file. niri hands its frames over as GPU buffers only, so even
/// the processor's encoder has them brought over by GL. It sends a frame
/// only when the window changes, each timed as it comes, and the last is
/// sent again at the end, so a window left still at the end doesn't cut the
/// video short. Each word is an argument of its own, which gst-launch
/// escapes, so the file's name may hold spaces.
fn pipeline(node: u32, asked: &Asked, encoder: Encoder) -> Vec<String> {
    let size = format!(
        "width={},height={},pixel-aspect-ratio=1/1",
        asked.width, asked.height
    );
    let fitted = match encoder {
        Encoder::Gpu => format!(
            "vapostproc add-borders=true ! video/x-raw(memory:VAMemory),format=NV12,{size} \
             ! vah264enc"
        ),
        Encoder::Processor => format!(
            "glupload ! glcolorconvert ! gldownload ! videoconvert ! videoscale add-borders=true \
             ! video/x-raw,format=I420,{size} ! x264enc speed-preset=veryfast"
        ),
    };
    let mut words: Vec<String> = format!(
        "-e pipewiresrc path={node} do-timestamp=true resend-last=true ! {fitted} ! h264parse \
         ! mp4mux fragment-duration=1000 ! filesink"
    )
    .split_whitespace()
    .map(String::from)
    .collect();
    words.push(format!("location={}", asked.file.display()));
    words
}

/// niri's number for a session, from the end of its path, which is the
/// `session_id` its casts are listed with.
fn session_number(path: &str) -> Option<u64> {
    path.rsplit_once("/u")?.1.parse().ok()
}

/// What GStreamer said went wrong, in its own words without the element
/// that said it: the first error it printed.
fn first_error(said: &str) -> String {
    let Some(line) = said.lines().find_map(|line| line.strip_prefix("ERROR: ")) else {
        return "it stopped without saying why".to_owned();
    };
    match line.strip_prefix("from element ") {
        Some(rest) => rest.split_once(": ").map_or(rest, |(_, why)| why),
        None => line,
    }
    .to_owned()
}

/// What the runtime file says while it records.
fn said(pid: u32, session: u64) -> String {
    format!("{{\"pid\":{pid},\"session\":{session}}}\n")
}

/// Says in the runtime directory that this records, until it's dropped. It's
/// written under another name and moved into place, so it's never read half
/// written, and taken back only while it's still this one's.
struct Saying {
    path: PathBuf,
    text: String,
}

impl Saying {
    fn new(session: u64) -> Result<Self, String> {
        let dir = env::var_os("XDG_RUNTIME_DIR")
            .map(PathBuf::from)
            .ok_or("XDG_RUNTIME_DIR isn't set")?;
        let path = dir.join("record-window.json");
        let part = dir.join(format!(".record-window-{}.json", process::id()));
        let text = said(process::id(), session);
        fs::write(&part, &text)
            .and_then(|()| fs::rename(&part, &path))
            .map_err(|error| format!("{} couldn't be written: {error}", path.display()))?;
        Ok(Self { path, text })
    }
}

impl Drop for Saying {
    fn drop(&mut self) {
        if fs::read_to_string(&self.path).is_ok_and(|now| now == self.text) {
            let _ = fs::remove_file(&self.path);
        }
    }
}

/// How a recording ended without going wrong.
#[derive(Debug, PartialEq, Eq)]
enum Ended {
    Saved,
    Stopped,
}

/// Waits to be told to stop, as Ctrl+C or systemd would tell it.
async fn told_to_stop(interrupt: &mut Signal, terminate: &mut Signal) {
    tokio::select! {
        _ = interrupt.recv() => {}
        _ = terminate.recv() => {}
    }
}

/// Tells gst-launch to finish its file, which its `-e` makes it do on
/// SIGINT.
fn finish(pid: Option<u32>) {
    if let Some(pid) = pid.and_then(|pid| i32::try_from(pid).ok()) {
        // SAFETY: kill only sends a signal; the pid is our own child's, which
        // hasn't been waited on, so it can't have been reused.
        unsafe {
            libc::kill(pid, libc::SIGINT);
        }
    }
}

async fn record(asked: &Asked) -> Result<Ended, String> {
    let unanswered = |error: zbus::Error| format!("niri's screencast isn't answering: {error}");
    // Heard from the start, so a stop asked for at any point is kept.
    let mut interrupt = signal(SignalKind::interrupt()).map_err(|error| error.to_string())?;
    let mut terminate = signal(SignalKind::terminate()).map_err(|error| error.to_string())?;

    let connection = Connection::session()
        .await
        .map_err(|error| format!("the session bus isn't answering: {error}"))?;
    let screencast = ScreenCastProxy::new(&connection)
        .await
        .map_err(unanswered)?;
    let path = screencast
        .create_session(HashMap::new())
        .await
        .map_err(unanswered)?;
    let session = SessionProxy::builder(&connection)
        .path(path.clone())
        .map_err(unanswered)?
        .build()
        .await
        .map_err(unanswered)?;
    // Followed before it starts, so niri can't end it unheard.
    let mut closed = session.receive_closed().await.map_err(unanswered)?;
    let stream = session
        .record_window(HashMap::from([
            ("window-id", Value::from(asked.window)),
            // The pointer drawn into the picture, as somebody watching sees it.
            ("cursor-mode", Value::from(1_u32)),
            ("is-recording", Value::from(true)),
        ]))
        .await
        .map_err(unanswered)?;
    let stream = StreamProxy::builder(&connection)
        .path(stream)
        .map_err(unanswered)?
        .build()
        .await
        .map_err(unanswered)?;
    let mut added = stream
        .receive_pipe_wire_stream_added()
        .await
        .map_err(unanswered)?;
    let number = session_number(path.as_str()).ok_or("niri named its session oddly")?;
    let _saying = Saying::new(number)?;
    session.start().await.map_err(unanswered)?;

    let node = tokio::select! {
        signal = added.next() => match signal {
            Some(signal) => signal.args().map_err(unanswered)?.node_id,
            None => return Err("niri went away".to_owned()),
        },
        _ = closed.next() => return Err("niri couldn't share the window".to_owned()),
        () = told_to_stop(&mut interrupt, &mut terminate) => {
            let _ = session.stop().await;
            return Ok(Ended::Stopped);
        }
    };

    let mut encoding = Command::new("gst-launch-1.0")
        .args(pipeline(node, asked, Encoder::here()))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(|error| format!("GStreamer couldn't start: {error}"))?;
    let mut said = encoding.stderr.take().ok_or("GStreamer said nothing")?;
    let heard = tokio::spawn(async move {
        let mut all = Vec::new();
        let _ = said.read_to_end(&mut all).await;
        String::from_utf8_lossy(&all).into_owned()
    });

    let pid = encoding.id();
    let mut cast_ended = false;
    let mut killed = false;
    let mut due: Option<Instant> = None;
    let status = loop {
        tokio::select! {
            status = encoding.wait() => break status.map_err(|error| error.to_string())?,
            _ = closed.next(), if !cast_ended => cast_ended = true,
            () = told_to_stop(&mut interrupt, &mut terminate) => {
                finish(pid);
                due.get_or_insert_with(|| Instant::now() + FINISHING);
            }
            () = sleep_until(due.unwrap_or_else(Instant::now)), if due.is_some() => {
                let _ = encoding.start_kill();
                killed = true;
                due = None;
            }
        }
    };
    if !cast_ended {
        let _ = session.stop().await;
    }

    let kept = fs::metadata(&asked.file).is_ok_and(|file| file.len() > 0);
    // A file cut short, by the window closing or by a finish too slow to
    // wait for, keeps the fragments written before.
    if status.success() || (kept && (cast_ended || killed)) {
        return Ok(Ended::Saved);
    }
    let _ = fs::remove_file(&asked.file);
    Err(format!(
        "GStreamer stopped: {}",
        first_error(&heard.await.unwrap_or_default())
    ))
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let asked = match Asked::parse(&args) {
        Ok(asked) => asked,
        Err(why) => {
            eprintln!("{why}");
            return ExitCode::from(2);
        }
    };
    match record(&asked).await {
        Ok(Ended::Saved) => ExitCode::SUCCESS,
        Ok(Ended::Stopped) => ExitCode::from(3),
        Err(why) => {
            eprintln!("{why}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn asked(file: &str) -> Asked {
        Asked {
            window: 7,
            width: 1400,
            height: 1748,
            file: PathBuf::from(file),
        }
    }

    fn words(text: &str) -> Vec<String> {
        text.split_whitespace().map(String::from).collect()
    }

    #[test]
    fn takes_a_window_its_size_and_a_file() {
        let args = words("7 1400 1748 /home/joel/Videos/Recordings/a.mp4");
        assert_eq!(
            Asked::parse(&args),
            Ok(asked("/home/joel/Videos/Recordings/a.mp4"))
        );
    }

    #[test]
    fn makes_an_odd_size_even() {
        let asked = Asked::parse(&words("7 1401 3 a.mp4")).unwrap();
        assert_eq!((asked.width, asked.height), (1400, 2));
        assert_eq!(even(0), 2);
        assert_eq!(even(1), 2);
        assert_eq!(even(u64::MAX), u32::MAX - 1);
    }

    #[test]
    fn says_how_its_asked_when_asked_wrongly() {
        assert_eq!(Asked::parse(&words("7 1400 1748")), Err(USAGE.to_owned()));
        assert_eq!(
            Asked::parse(&words("7 1400 1748 a.mp4 b.mp4")),
            Err(USAGE.to_owned())
        );
        assert_eq!(
            Asked::parse(&words("seven 1400 1748 a.mp4")),
            Err("the window \"seven\" isn't a number".to_owned())
        );
        assert_eq!(
            Asked::parse(&words("7 wide 1748 a.mp4")),
            Err("the width \"wide\" isn't a number".to_owned())
        );
    }

    #[test]
    fn encodes_on_the_gpu_only_where_va_api_does() {
        assert_eq!(Encoder::for_driver(Some("xe")), Encoder::Gpu);
        assert_eq!(Encoder::for_driver(Some("i915")), Encoder::Gpu);
        assert_eq!(Encoder::for_driver(Some("amdgpu")), Encoder::Gpu);
        assert_eq!(Encoder::for_driver(Some("virtio-gpu")), Encoder::Processor);
        assert_eq!(Encoder::for_driver(None), Encoder::Processor);
    }

    #[test]
    fn fits_and_encodes_on_the_gpu() {
        let mut expected = words(
            "-e pipewiresrc path=58 do-timestamp=true resend-last=true ! vapostproc add-borders=true \
             ! video/x-raw(memory:VAMemory),format=NV12,width=1400,height=1748,pixel-aspect-ratio=1/1 \
             ! vah264enc ! h264parse ! mp4mux fragment-duration=1000 ! filesink",
        );
        expected.push("location=/r/2026-10-17 18-50-00.mp4".to_owned());
        assert_eq!(
            pipeline(58, &asked("/r/2026-10-17 18-50-00.mp4"), Encoder::Gpu),
            expected
        );
    }

    #[test]
    fn fits_and_encodes_on_the_processor() {
        let mut expected = words(
            "-e pipewiresrc path=58 do-timestamp=true resend-last=true ! glupload ! glcolorconvert \
             ! gldownload \
             ! videoconvert ! videoscale add-borders=true \
             ! video/x-raw,format=I420,width=1400,height=1748,pixel-aspect-ratio=1/1 \
             ! x264enc speed-preset=veryfast ! h264parse ! mp4mux fragment-duration=1000 \
             ! filesink",
        );
        expected.push("location=/r/a.mp4".to_owned());
        assert_eq!(
            pipeline(58, &asked("/r/a.mp4"), Encoder::Processor),
            expected
        );
    }

    #[test]
    fn reads_the_session_from_its_path() {
        assert_eq!(
            session_number("/org/gnome/Mutter/ScreenCast/Session/u12"),
            Some(12)
        );
        assert_eq!(session_number("/org/gnome/Mutter/ScreenCast/Session"), None);
        assert_eq!(
            session_number("/org/gnome/Mutter/ScreenCast/Session/ux"),
            None
        );
    }

    #[test]
    fn says_gstreamers_first_error_without_its_element() {
        let said = "Setting pipeline to PAUSED ...\n\
            ERROR: from element /GstPipeline:pipeline0/GstPipeWireSrc:pipewiresrc0: \
            stream error: target not found\n\
            Additional debug info:\n\
            ERROR: pipeline doesn't want to preroll.\n";
        assert_eq!(first_error(said), "stream error: target not found");
        assert_eq!(
            first_error("ERROR: pipeline doesn't want to preroll.\n"),
            "pipeline doesn't want to preroll."
        );
        assert_eq!(first_error(""), "it stopped without saying why");
    }

    #[test]
    fn says_its_process_and_session() {
        assert_eq!(said(4242, 7), "{\"pid\":4242,\"session\":7}\n");
    }
}
