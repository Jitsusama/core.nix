//! Whether Joel's camera gives a picture before each of his meetings, so a
//! call that would open on none, as his Meet at 09:39 on 23 Nov did, is
//! told to him before he joins rather than seen by those he meets.
//!
//! Each minute its timer runs `call-under-load camera`. When a meeting of
//! his starts between three and six minutes from now, no call is on and
//! that meeting wasn't checked yet, it takes the camera his programs take:
//! one plugged in that he can open, as Joel chose for the camera program,
//! and otherwise the machine's own. Only a loopback, which hands its frames
//! to every reader at once, is read: for five frames, each sent to
//! /dev/null, so nothing it sees is kept, and given two minutes, since the
//! built-in's first frame can take 40 s under memory pressure. Any other
//! camera is only asked what it sends and never read, since a USB camera
//! gives its frames to one program at a time and Meet must always get it.
//! Frames arriving is what's checked, not what's in them. When the camera
//! gives nothing, a notice from Camera says so, with the meeting and how
//! soon it starts.

use crate::gate::{Calendar, Call, Meeting};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

const MIN_MS: u64 = 60_000;
/// The latest a meeting may start for its check to begin: six minutes out,
/// so two minutes of waiting end with four left to choose another camera.
const LEAD_MS: u64 = 6 * MIN_MS;
/// The soonest: one starting sooner has no time left for a check to help,
/// and Meet may be opening the camera already.
const SOONEST_MS: u64 = 3 * MIN_MS;
/// How long a loopback is given to send its frames.
pub const WAIT: Duration = Duration::from_secs(120);
/// How long any other camera is given to answer.
const ANSWER: Duration = Duration::from_secs(10);
/// How many frames make a picture: more than one, so a camera that sends a
/// frame and stalls isn't taken for one that works.
const FRAMES: &str = "5";
/// How long a meeting's check is remembered, so each is checked once.
const KEPT_MS: u64 = 24 * 60 * MIN_MS;

/// Why a camera gave no picture.
#[derive(Debug, PartialEq, Eq)]
pub enum Fault {
    /// Nothing came before its time was up.
    Silent,
    /// The program ended without one, saying how.
    Ended(String),
}

impl std::fmt::Display for Fault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Fault::Silent => f.write_str("nothing came"),
            Fault::Ended(how) => f.write_str(how),
        }
    }
}

/// The soonest meeting due its check at `now`, of those not in `checked`.
pub fn due<'a>(calendar: &'a Calendar, now: u64, checked: &[u64]) -> Option<&'a Meeting> {
    calendar
        .meetings
        .iter()
        .filter(|meeting| {
            meeting.start >= now + SOONEST_MS
                && meeting.start <= now + LEAD_MS
                && !checked.contains(&meeting.start)
        })
        .min_by_key(|meeting| meeting.start)
}

/// Whether the camera may be checked: only when `a-call-is-on` said no
/// call is on, since a call has its camera and an answer it couldn't give
/// may be one.
pub fn clear(call: &Call) -> bool {
    *call == Call::Off
}

/// The meetings' starts already checked, from `file`, those of the last day.
pub fn checked(file: &Path, now: u64) -> Vec<u64> {
    let starts: Vec<u64> = fs::read_to_string(file)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default();
    starts
        .into_iter()
        .filter(|start| start + KEPT_MS > now)
        .collect()
}

/// Adds `start` to the starts checked in `file`, whole or not at all.
pub fn remember(file: &Path, mut starts: Vec<u64>, start: u64) -> std::io::Result<()> {
    starts.push(start);
    if let Some(folder) = file.parent() {
        fs::create_dir_all(folder)?;
    }
    let part = file.with_extension("json.part");
    fs::write(&part, serde_json::to_string(&starts)?)?;
    fs::rename(&part, file)
}

/// Which camera Joel's programs take, with root standing for `/`: a USB
/// camera he can open whose port says it's removable, as one on his desk
/// is, and otherwise the machine's `device`, or the device a file of that
/// name names, as one made at boot is. One he can't open is left out, as a
/// camera another program shares out is.
pub fn which(root: &Path, device: Option<&str>) -> Option<PathBuf> {
    let by_id = root.join("dev/v4l/by-id");
    let mut nodes: Vec<PathBuf> = fs::read_dir(&by_id)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|node| {
            node.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("usb-") && name.ends_with("-video-index0"))
        })
        .collect();
    nodes.sort();
    let plugged = nodes.into_iter().find(|node| {
        let Some(video) = fs::read_link(node)
            .ok()
            .and_then(|to| to.file_name().map(ToOwned::to_owned))
        else {
            return false;
        };
        let port = root
            .join("sys/class/video4linux")
            .join(video)
            .join("device/../removable");
        // Opening a camera's node starts nothing; only reading it would.
        fs::read_to_string(port).is_ok_and(|said| said.trim() == "removable")
            && fs::File::open(node).is_ok()
    });
    if plugged.is_some() {
        return plugged;
    }
    let device = device?;
    // Only a plain file is read: reading a camera's own node would read
    // its frames.
    let named = root.join(device.trim_start_matches('/'));
    if fs::metadata(&named).is_ok_and(|meta| meta.is_file()) {
        let text = fs::read_to_string(&named).ok()?;
        return Some(PathBuf::from(text.trim()));
    }
    Some(PathBuf::from(device))
}

/// Whether `camera`, with root standing for `/`, is a loopback: a device
/// no hardware stands behind, which hands each frame to every reader.
pub fn loopback(root: &Path, camera: &Path) -> bool {
    let node = root.join(camera.strip_prefix("/").unwrap_or(camera));
    let Some(video) = fs::canonicalize(node)
        .ok()
        .and_then(|node| node.file_name().map(ToOwned::to_owned))
    else {
        return false;
    };
    let (Ok(root), Ok(class)) = (
        fs::canonicalize(root),
        fs::canonicalize(root.join("sys/class/video4linux").join(video)),
    ) else {
        return false;
    };
    class
        .strip_prefix(root)
        .is_ok_and(|class| class.starts_with("sys/devices/virtual"))
}

/// What `v4l2-ctl` is asked of `camera`: a loopback's five frames, each to
/// /dev/null, and any other camera only what it sends.
pub fn asked(camera: &Path, read: bool) -> Vec<String> {
    let device = camera.to_string_lossy().into_owned();
    let words: &[&str] = if read {
        &[
            "--stream-mmap",
            "--stream-count",
            FRAMES,
            "--stream-to",
            "/dev/null",
        ]
    } else {
        &["--get-fmt-video"]
    };
    ["--device", device.as_str()]
        .iter()
        .chain(words)
        .map(|&word| word.to_owned())
        .collect()
}

/// Text with the characters a notice's body reads as markup written out,
/// so a meeting's title, which anyone inviting him sets, is only words.
fn plain(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// What a camera that gave no picture is told as: a notice's summary and
/// its body, saying which meeting, how soon, and what the camera did, in
/// Joel's words rather than the program's, which the journal keeps.
pub fn told(meeting: &Meeting, now: u64, read: bool, fault: &Fault) -> (String, String) {
    let minutes = meeting.start.saturating_sub(now).div_ceil(MIN_MS);
    let camera = match (read, fault) {
        (true, Fault::Silent) => "The camera gave no picture in 2 minutes",
        (true, Fault::Ended(_)) => "The camera couldn't be opened",
        (false, _) => "The camera didn't answer",
    };
    (
        "Your camera gives no picture".to_string(),
        format!(
            "{camera}, and {} starts in {minutes} min. Choose another camera in the call, \
             or join with video off.",
            plain(&meeting.title)
        ),
    )
}

/// Runs `program` with `args`, done when it exits 0 within `deadline`.
pub fn within(program: &str, args: &[String], deadline: Duration) -> Result<(), Fault> {
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| Fault::Ended(format!("{program} couldn't run: {error}")))?;
    let until = Instant::now() + deadline;
    loop {
        match child.try_wait() {
            Ok(Some(status)) if status.success() => return Ok(()),
            Ok(Some(status)) => return Err(Fault::Ended(format!("{program} ended {status}"))),
            Ok(None) if Instant::now() < until => thread::sleep(Duration::from_millis(100)),
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(Fault::Silent);
            }
            Err(error) => return Err(Fault::Ended(error.to_string())),
        }
    }
}

/// Whether `camera` gives a picture: read when it's a loopback, and
/// otherwise only asked.
pub fn gives(camera: &Path, read: bool) -> Result<(), Fault> {
    let deadline = if read { WAIT } else { ANSWER };
    within("v4l2-ctl", &asked(camera, read), deadline)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;

    fn at(starts: &[u64]) -> Calendar {
        Calendar {
            read: 0,
            meetings: starts
                .iter()
                .map(|&start| Meeting {
                    title: format!("meeting {start}"),
                    start,
                    end: start + 30 * MIN_MS,
                })
                .collect(),
        }
    }

    fn stand_in(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("camera-{name}-{}", std::process::id()))
    }

    #[test]
    fn the_soonest_meeting_is_checked_once_between_three_and_six_minutes_before() {
        let now = 100 * MIN_MS;
        let calendar = at(&[
            now + 7 * MIN_MS,
            now + 5 * MIN_MS,
            now + 3 * MIN_MS - 1,
            now + 4 * MIN_MS,
        ]);
        let start = |m: Option<&Meeting>| m.map(|m| m.start);
        assert_eq!(
            start(due(&calendar, now, &[])),
            Some(now + 4 * MIN_MS),
            "the soonest meeting in the window wasn't the one due"
        );
        assert_eq!(
            start(due(&calendar, now, &[now + 4 * MIN_MS])),
            Some(now + 5 * MIN_MS),
            "a meeting already checked was checked again"
        );
        assert_eq!(
            start(due(
                &calendar,
                now + MIN_MS,
                &[now + 4 * MIN_MS, now + 5 * MIN_MS]
            )),
            Some(now + 7 * MIN_MS),
            "a meeting six minutes out wasn't due"
        );
        assert_eq!(
            start(due(&at(&[now + 3 * MIN_MS]), now, &[])),
            Some(now + 3 * MIN_MS),
            "a meeting three minutes out wasn't due"
        );
        assert_eq!(
            start(due(&at(&[now + 3 * MIN_MS - 1]), now, &[])),
            None,
            "a meeting under three minutes out was checked"
        );
    }

    #[test]
    fn the_camera_waits_unless_a_call_is_surely_off() {
        assert!(
            clear(&Call::Off),
            "no call on kept the camera from its check"
        );
        assert!(!clear(&Call::On), "a call on had its camera checked");
        assert!(
            !clear(&Call::Unknown("no answer".into())),
            "an answer that couldn't tell was taken for no call"
        );
    }

    #[test]
    fn a_check_is_remembered_for_a_day() {
        let root = stand_in("kept");
        let file = root.join("camera-checked.json");
        let now = KEPT_MS + 10;
        assert!(
            checked(&file, now).is_empty(),
            "no file read as checks made"
        );
        remember(&file, vec![5], now + MIN_MS).unwrap();
        let kept = checked(&file, now);
        fs::remove_dir_all(&root).unwrap();
        assert_eq!(
            kept,
            [now + MIN_MS],
            "a check wasn't remembered, or one over a day old was kept"
        );
    }

    /// A stand-in USB camera numbered `n` under `root`, its port saying
    /// `port`, with a node that can be opened when `opens`.
    fn usb(root: &Path, n: u32, port: &str, opens: bool) {
        let sys = root.join(format!("sys/devices/usb{n}/port"));
        fs::create_dir_all(sys.join("interface")).unwrap();
        fs::write(sys.join("removable"), port).unwrap();
        let class = root.join(format!("sys/class/video4linux/video{n}"));
        fs::create_dir_all(&class).unwrap();
        symlink(sys.join("interface"), class.join("device")).unwrap();
        let by_id = root.join("dev/v4l/by-id");
        fs::create_dir_all(&by_id).unwrap();
        if opens {
            fs::write(root.join(format!("dev/video{n}")), "").unwrap();
        }
        symlink(
            format!("../../video{n}"),
            by_id.join(format!("usb-Cam_{n}-video-index0")),
        )
        .unwrap();
    }

    #[test]
    fn the_camera_is_one_plugged_in_he_can_open_or_else_the_machine_s() {
        let root = stand_in("which");
        fs::create_dir_all(root.join("run/camera")).unwrap();
        fs::write(root.join("run/camera/device"), "/dev/video32\n").unwrap();
        let machine = which(&root, Some("/run/camera/device"));
        usb(&root, 1, "fixed\n", true);
        let fixed = which(&root, Some("/run/camera/device"));
        usb(&root, 2, "removable\n", false);
        let shut = which(&root, Some("/run/camera/device"));
        usb(&root, 3, "removable\n", true);
        let plugged = which(&root, Some("/run/camera/device"));
        let none = which(&root.join("nowhere"), None);
        let named = which(&root.join("nowhere"), Some("/dev/video0"));
        fs::remove_dir_all(&root).unwrap();
        assert_eq!(
            machine,
            Some(PathBuf::from("/dev/video32")),
            "the machine's camera wasn't read from the file naming it"
        );
        assert_eq!(
            fixed, machine,
            "a built-in USB camera was taken for one plugged in"
        );
        assert_eq!(shut, machine, "a camera he can't open was taken for his");
        assert_eq!(
            plugged,
            Some(root.join("dev/v4l/by-id/usb-Cam_3-video-index0")),
            "a camera plugged in wasn't taken over the machine's"
        );
        assert_eq!(none, None, "a machine with no camera named one");
        assert_eq!(
            named,
            Some(PathBuf::from("/dev/video0")),
            "a device that isn't a file naming one wasn't taken as it is"
        );
    }

    #[test]
    fn only_a_loopback_is_read_and_any_other_camera_only_asked() {
        let root = stand_in("loopback");
        for (n, behind) in [(32, "virtual"), (0, "pci0000:00/0000:00:14.0/usb3")] {
            let device = root.join(format!("sys/devices/{behind}/video4linux/video{n}"));
            fs::create_dir_all(&device).unwrap();
            fs::create_dir_all(root.join("sys/class/video4linux")).unwrap();
            symlink(
                &device,
                root.join(format!("sys/class/video4linux/video{n}")),
            )
            .unwrap();
            fs::create_dir_all(root.join("dev")).unwrap();
            fs::write(root.join(format!("dev/video{n}")), "").unwrap();
        }
        let virtual_one = loopback(&root, Path::new("/dev/video32"));
        let usb_one = loopback(&root, Path::new("/dev/video0"));
        let gone = loopback(&root, Path::new("/dev/video9"));
        fs::remove_dir_all(&root).unwrap();
        assert!(virtual_one, "a loopback wasn't read");
        assert!(!usb_one, "a camera with hardware behind it was read");
        assert!(!gone, "a camera that isn't there was read");
        let read = asked(Path::new("/dev/video32"), true);
        assert_eq!(
            read.join(" "),
            "--device /dev/video32 --stream-mmap --stream-count 5 --stream-to /dev/null",
            "a loopback's frames weren't all sent to /dev/null"
        );
        let only = asked(Path::new("/dev/video0"), false);
        assert!(
            !only.iter().any(|word| word.starts_with("--stream")),
            "a camera other than a loopback was read: {only:?}"
        );
    }

    #[test]
    fn a_camera_with_no_picture_is_told_plainly_with_its_meeting_and_how_soon() {
        let meeting = Meeting {
            title: "Joel:Steve <b>&</b>".to_string(),
            start: 10 * MIN_MS,
            end: 40 * MIN_MS,
        };
        assert_eq!(
            told(&meeting, 5 * MIN_MS - 1, true, &Fault::Silent),
            (
                "Your camera gives no picture".to_string(),
                "The camera gave no picture in 2 minutes, and Joel:Steve \
                 &lt;b&gt;&amp;&lt;/b&gt; starts in 6 min. Choose another camera in the \
                 call, or join with video off."
                    .to_string()
            ),
            "a camera giving nothing wasn't told plainly, or a title's markup was kept"
        );
        let opening = told(&meeting, 6 * MIN_MS, true, &Fault::Ended("exit 1".into())).1;
        assert!(
            opening.starts_with("The camera couldn't be opened,"),
            "a camera that failed at once was said to have waited: {opening}"
        );
        let asked = told(&meeting, 6 * MIN_MS, false, &Fault::Silent).1;
        assert!(
            asked.starts_with("The camera didn't answer,"),
            "a camera only asked was said to have been read: {asked}"
        );
    }

    #[test]
    fn a_program_is_given_its_time_and_no_more() {
        let none: [String; 0] = [];
        assert_eq!(
            within("true", &none, Duration::from_secs(5)),
            Ok(()),
            "a program that exited 0 wasn't taken as done"
        );
        assert!(
            matches!(
                within("false", &none, Duration::from_secs(5)),
                Err(Fault::Ended(_))
            ),
            "a program that failed wasn't said to have ended"
        );
        let started = Instant::now();
        assert_eq!(
            within("sleep", &["5".to_string()], Duration::from_millis(300)),
            Err(Fault::Silent),
            "a program past its time wasn't stopped"
        );
        assert!(
            started.elapsed() < Duration::from_secs(3),
            "a program past its time was waited on to its end"
        );
    }
}
