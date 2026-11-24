//! Whether Joel's camera gives a picture before each of his meetings, so a
//! call that would open on none, as his Meet at 09:39 on 23 Nov did, is
//! told to him before he joins rather than seen by those he meets.
//!
//! Each minute its timer runs `call-under-load camera`. When a meeting of
//! his starts between three and six minutes from now, no call is on and
//! that meeting wasn't checked yet, it takes the camera his programs take:
//! one plugged in while there is one, as Joel chose for the camera program,
//! and otherwise the machine's own. The machine's is opened and read for
//! five frames, each sent to /dev/null, so nothing it sees is kept; it's
//! given two minutes, since the built-in's first frame can take 40 s under
//! memory pressure. A camera plugged in is only asked whether it answers
//! and never read, since a USB camera gives its frames to one program at a
//! time and Meet must always get it. When the camera gives nothing, a
//! notice from Camera says so, with the meeting and how soon it starts.

use crate::gate::{Calendar, Meeting};
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
/// How long the machine's camera is given to send its frames.
pub const WAIT: Duration = Duration::from_secs(120);
/// How long a camera plugged in is given to answer.
const ANSWER: Duration = Duration::from_secs(10);
/// How many frames make a picture: more than one, so a camera that sends a
/// frame and stalls isn't taken for one that works.
const FRAMES: &str = "5";
/// How long a meeting's check is remembered, so each is checked once.
const KEPT_MS: u64 = 24 * 60 * MIN_MS;

/// The next meeting due its check at `now`, of those not in `checked`, by
/// their starts.
pub fn due<'a>(calendar: &'a Calendar, now: u64, checked: &[u64]) -> Option<&'a Meeting> {
    calendar.meetings.iter().find(|meeting| {
        meeting.start >= now + SOONEST_MS
            && meeting.start <= now + LEAD_MS
            && !checked.contains(&meeting.start)
    })
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
/// camera whose port says it's removable, as one on his desk is, and
/// otherwise the machine's `device`, or the device a file of that name
/// names, as one made at boot is. True when it's one plugged in.
pub fn which(root: &Path, device: Option<&str>) -> Option<(PathBuf, bool)> {
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
        fs::read_to_string(port).is_ok_and(|said| said.trim() == "removable")
    });
    if let Some(node) = plugged {
        return Some((node, true));
    }
    let device = device?;
    // Only a plain file is read: reading a camera's own node would read
    // its frames.
    let named = root.join(device.trim_start_matches('/'));
    if fs::metadata(&named).is_ok_and(|meta| meta.is_file()) {
        let text = fs::read_to_string(&named).ok()?;
        return Some((PathBuf::from(text.trim()), false));
    }
    Some((PathBuf::from(device), false))
}

/// What a camera that gave no picture is told as: a notice's summary and
/// its body, saying which meeting, how soon, and what the camera did.
pub fn told(meeting: &Meeting, now: u64, plugged: bool, why: &str) -> (String, String) {
    let minutes = meeting.start.saturating_sub(now).div_ceil(MIN_MS);
    let camera = if plugged {
        "The camera plugged in didn't answer"
    } else {
        "The camera gave no picture in 2 minutes"
    };
    (
        "Your camera gives no picture".to_string(),
        format!(
            "{camera} ({why}), and {} starts in {minutes} min. Choose another camera \
             in the call, or join with video off.",
            meeting.title
        ),
    )
}

/// Runs `program` with `args`, done when it exits 0 within `deadline`, and
/// otherwise why not.
pub fn within(program: &str, args: &[&str], deadline: Duration) -> Result<(), String> {
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| format!("{program} couldn't run: {error}"))?;
    let until = Instant::now() + deadline;
    loop {
        match child.try_wait() {
            Ok(Some(status)) if status.success() => return Ok(()),
            Ok(Some(status)) => return Err(format!("it ended {status}")),
            Ok(None) if Instant::now() < until => thread::sleep(Duration::from_millis(100)),
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err("nothing came".to_string());
            }
            Err(error) => return Err(error.to_string()),
        }
    }
}

/// Whether `camera` gives a picture: the machine's read for its frames,
/// each to /dev/null, and one plugged in only asked what it sends.
pub fn gives(camera: &Path, plugged: bool) -> Result<(), String> {
    let device = camera.to_string_lossy().into_owned();
    let device = device.as_str();
    if plugged {
        within("v4l2-ctl", &["--device", device, "--get-fmt-video"], ANSWER)
    } else {
        within(
            "v4l2-ctl",
            &[
                "--device",
                device,
                "--stream-mmap",
                "--stream-count",
                FRAMES,
                "--stream-to",
                "/dev/null",
            ],
            WAIT,
        )
    }
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

    #[test]
    fn a_meeting_is_checked_once_between_three_and_six_minutes_before() {
        let now = 100 * MIN_MS;
        let calendar = at(&[now + 2 * MIN_MS, now + 7 * MIN_MS, now + 5 * MIN_MS]);
        let start = |m: Option<&Meeting>| m.map(|m| m.start);
        assert_eq!(
            start(due(&calendar, now, &[])),
            Some(now + 5 * MIN_MS),
            "a meeting five minutes out wasn't due, or one too soon or too far was"
        );
        assert_eq!(
            start(due(&calendar, now, &[now + 5 * MIN_MS])),
            None,
            "a meeting already checked was checked again"
        );
        assert_eq!(
            start(due(&calendar, now + MIN_MS, &[])),
            Some(now + 7 * MIN_MS),
            "a meeting six minutes out wasn't due"
        );
    }

    #[test]
    fn a_check_is_remembered_for_a_day() {
        let root = std::env::temp_dir().join(format!("camera-kept-{}", std::process::id()));
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

    #[test]
    fn the_camera_is_one_plugged_in_or_else_the_machine_s() {
        let root = std::env::temp_dir().join(format!("camera-which-{}", std::process::id()));
        let video = |n: u32, port: &str| {
            let sys = root.join(format!("sys/devices/usb{n}/port"));
            fs::create_dir_all(sys.join("interface")).unwrap();
            fs::write(sys.join("removable"), port).unwrap();
            let class = root.join("sys/class/video4linux");
            fs::create_dir_all(&class).unwrap();
            fs::create_dir_all(class.join(format!("video{n}"))).unwrap();
            symlink(
                sys.join("interface"),
                class.join(format!("video{n}/device")),
            )
            .unwrap();
            let by_id = root.join("dev/v4l/by-id");
            fs::create_dir_all(&by_id).unwrap();
            symlink(
                format!("../../video{n}"),
                by_id.join(format!("usb-Cam_{n}-video-index0")),
            )
            .unwrap();
        };
        fs::create_dir_all(root.join("run/camera")).unwrap();
        fs::write(root.join("run/camera/device"), "/dev/video32\n").unwrap();
        let machine = which(&root, Some("/run/camera/device"));
        video(1, "fixed\n");
        let fixed = which(&root, Some("/run/camera/device"));
        video(2, "removable\n");
        let plugged = which(&root, Some("/run/camera/device"));
        let none = which(&root.join("nowhere"), None);
        fs::remove_dir_all(&root).unwrap();
        assert_eq!(
            machine,
            Some((PathBuf::from("/dev/video32"), false)),
            "the machine's camera wasn't read from the file naming it"
        );
        assert_eq!(
            fixed, machine,
            "a built-in USB camera was taken for one plugged in"
        );
        assert_eq!(
            plugged,
            Some((root.join("dev/v4l/by-id/usb-Cam_2-video-index0"), true)),
            "a camera plugged in wasn't taken over the machine's"
        );
        assert_eq!(none, None, "a machine with no camera named one");
    }

    #[test]
    fn a_camera_with_no_picture_is_told_with_its_meeting_and_how_soon() {
        let meeting = Meeting {
            title: "Joel:Steve".to_string(),
            start: 10 * MIN_MS,
            end: 40 * MIN_MS,
        };
        assert_eq!(
            told(&meeting, 5 * MIN_MS - 1, false, "nothing came"),
            (
                "Your camera gives no picture".to_string(),
                "The camera gave no picture in 2 minutes (nothing came), and Joel:Steve \
                 starts in 6 min. Choose another camera in the call, or join with video off."
                    .to_string()
            ),
            "a camera giving nothing wasn't told plainly"
        );
        assert!(
            told(&meeting, 6 * MIN_MS, true, "it ended exit status: 1")
                .1
                .starts_with("The camera plugged in didn't answer"),
            "a camera plugged in was said to have been read"
        );
    }

    #[test]
    fn a_program_is_given_its_time_and_no_more() {
        assert_eq!(
            within("true", &[], Duration::from_secs(5)),
            Ok(()),
            "a program that exited 0 wasn't taken as done"
        );
        assert!(
            within("false", &[], Duration::from_secs(5)).is_err(),
            "a program that failed was taken as done"
        );
        let started = Instant::now();
        assert_eq!(
            within("sleep", &["5"], Duration::from_millis(300)),
            Err("nothing came".to_string()),
            "a program past its time wasn't stopped"
        );
        assert!(
            started.elapsed() < Duration::from_secs(3),
            "a program past its time was waited on to its end"
        );
    }
}
