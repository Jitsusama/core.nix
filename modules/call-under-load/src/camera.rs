//! Whether Joel's camera gives a picture before each of his meetings, so a
//! call that would open on none, as his Meet at 09:39 on 23 Nov did, is
//! told to him before he joins rather than seen by those he meets.
//!
//! Each minute its timer runs `call-under-load camera`. When a meeting of
//! his starts between 18 and 22 minutes from now, no call is on, no
//! meeting is on or less than 15 minutes off while it reads, and that
//! meeting wasn't checked yet, it takes the camera his programs take:
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
//!
//! Long because each way a check can go wrong under load, a relay mending
//! itself, a camera not there yet, systemd slow to answer, is a case its
//! tests replay beside the code that decides it.

use crate::gate::{Calendar, Call, Meeting};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant, SystemTime};

const MIN_MS: u64 = 60_000;
/// How long before a meeting, and through it, no camera is read, since
/// Joel may be setting up and a camera in use is one he sees and didn't
/// start, as at 09:51 on 24 Nov.
const UNREAD_MS: u64 = 15 * MIN_MS;
/// The longest a check takes from its first question to its last, rounded
/// up: up to three asks of a-call-is-on, systemd's budget each side of the
/// read and the read's own wait come to 165 s.
const READ_MS: u64 = 3 * MIN_MS;
/// The soonest a meeting may start for its check to begin: any later and
/// the check could still be reading inside the quarter hour.
const SOONEST_MS: u64 = UNREAD_MS + READ_MS;
/// The latest: 22 minutes out, so a fault worth a second look found at the
/// first run still has the minute it needs, with a minute's slack, before
/// the window closes.
const LEAD_MS: u64 = SOONEST_MS + 4 * MIN_MS;
/// How long a loopback is given to send its frames.
pub const WAIT: Duration = Duration::from_secs(120);
/// How long any other camera is given to answer.
const ANSWER: Duration = Duration::from_secs(10);
/// How many frames make a picture: more than one, so a camera that sends a
/// frame and stalls isn't taken for one that works.
const FRAMES: &str = "5";
/// How many frames are passed over first: a relay hands its splash to the
/// loopback until it hears a reader, then stops it before the camera's
/// setup, so only the first frames may be the splash. Three seconds of
/// them at 30 a second gives a loaded relay time to hear the reader,
/// while a camera whose first frame takes 40 s needs only a little over
/// a frame a second after it to give the rest inside the wait. A longer
/// skip trades a missed splash for a slow camera told falsely, and which
/// is likelier waits on the relay and camera measured under load.
const SPLASH: &str = "90";
/// How long systemd has to say which run the relay is on, each side of
/// the read: one ask may take all of it, as an answer slowed by load
/// needs, and one that fails at once is asked again a second later
/// until it's spent. The 15 s a side keep the whole check inside its
/// unit's four minutes.
const SHOWN: Duration = Duration::from_secs(15);
/// How long a failed ask waits before the next, so a bus that refused
/// one isn't asked again in the same moment.
const PAUSE: Duration = Duration::from_secs(1);
/// How long a meeting's check is remembered, so each is checked once.
const KEPT_MS: u64 = 24 * 60 * MIN_MS;

/// Why a camera gave no picture.
#[derive(Debug, PartialEq, Eq)]
pub enum Fault {
    /// Nothing came before its time was up.
    Silent,
    /// The program ended without one, saying how.
    Ended(String),
    /// The relay feeding the loopback started again while it was read, so
    /// what came may be its placeholder rather than the camera.
    Restarted,
    /// The machine's camera, or the file naming it, isn't there.
    Missing,
    /// The machine slept or stalled, so the read was cut short or never
    /// begun rather than run into the quarter hour before a meeting. It
    /// says nothing of the camera.
    Paused,
}

/// What a check comes to.
#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    /// The camera gave what it was asked for.
    Fine,
    /// Joel is told, and the meeting kept as checked once he is.
    Tell(Fault),
    /// Read again next minute: a relay mending itself, a camera not there
    /// yet or one that couldn't be opened while its relay starts may be
    /// fine by then, and the window has a minute to spare. Only a camera
    /// silent for the whole wait is told at once.
    Again(Fault),
    /// The read ended inside the quarter hour before the meeting or after
    /// it began, or was cut short by a pause with no minute left to look
    /// again, which only a suspend or a stall brings about. It's kept as
    /// checked and only journalled: the fault may be the pause's, and a
    /// notice now would be late or false.
    Late(Fault),
}

/// Whether the next minute's check could fall outside the window, so a
/// fault seen now must be told now. The next run is given a minute's
/// slack past its own, since under load it can start late.
pub fn last_chance(start: u64, now: u64) -> bool {
    start < now + SOONEST_MS + 2 * MIN_MS
}

/// What a check of the meeting starting at `start`, ended at `now`, comes
/// to.
pub fn settled(gave: Result<(), Fault>, start: u64, now: u64) -> Outcome {
    match gave {
        Ok(()) => Outcome::Fine,
        Err(fault) if start < now + UNREAD_MS => Outcome::Late(fault),
        Err(fault @ (Fault::Restarted | Fault::Missing | Fault::Ended(_) | Fault::Paused))
            if !last_chance(start, now) =>
        {
            Outcome::Again(fault)
        }
        // A pause says nothing of the camera, so it's never told as dead.
        Err(Fault::Paused) => Outcome::Late(Fault::Paused),
        Err(fault) => Outcome::Tell(fault),
    }
}

impl std::fmt::Display for Fault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Fault::Silent => f.write_str("nothing came"),
            Fault::Ended(how) => f.write_str(how),
            Fault::Restarted => f.write_str("its relay started again while it was read"),
            Fault::Missing => f.write_str("it isn't there"),
            Fault::Paused => f.write_str("the machine slept or stalled, so it wasn't read in time"),
        }
    }
}

/// The system unit that feeds a machine's camera to its loopback, as the
/// XPS 14's does (hardware/dell-xps-14-da14260/camera/camera.nix). It
/// restarts itself when a reader has had no frame for 20 s, so a restart
/// while the loopback is read means the camera gave nothing.
const RELAY: &str = "camera.service";

/// The relay's run, by systemd's id for it, empty on a machine with no
/// such unit or none running, or nothing when systemd couldn't be asked,
/// as under the load that stalls a camera, in the time it's given.
pub fn relay_run() -> Option<String> {
    let args = ["show", "--value", "--property=InvocationID", RELAY].map(String::from);
    answered("systemctl", &args, SHOWN, PAUSE)
}

/// What `program` printed once one of its runs answered inside `budget`:
/// a run may take what's left of it, and one that failed is tried again
/// after `pause` while any is left.
fn answered(program: &str, args: &[String], budget: Duration, pause: Duration) -> Option<String> {
    let until = Instant::now() + budget;
    loop {
        let left = until.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return None;
        }
        if let Some(answer) = said(program, args, left) {
            return Some(answer);
        }
        thread::sleep(pause.min(until.saturating_duration_since(Instant::now())));
    }
}

/// What `program` printed, trimmed, once it exits 0 inside `deadline`, or
/// nothing when it couldn't run, failed, or was killed at the deadline.
fn said(program: &str, args: &[String], deadline: Duration) -> Option<String> {
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let until = Instant::now() + deadline;
    loop {
        match child.try_wait() {
            Ok(Some(status)) if status.success() => break,
            Ok(None) if Instant::now() < until => thread::sleep(Duration::from_millis(50)),
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
            Ok(Some(_)) | Err(_) => return None,
        }
    }
    let mut out = String::new();
    child.stdout.take()?.read_to_string(&mut out).ok()?;
    Some(out.trim().to_owned())
}

/// What a read came to, given the relay's run before and after it: frames
/// from a relay that started again in between aren't the camera's, and a
/// camera that couldn't be opened then was only between runs. A run that
/// couldn't be asked may hide a restart, whose new run's splash would pass
/// for a picture, so it counts as one: read again while a minute is
/// spare, and told rather than passed at the last chance.
pub fn judged(
    gave: Result<(), Fault>,
    before: Option<&str>,
    after: Option<&str>,
) -> Result<(), Fault> {
    match gave {
        Ok(()) | Err(Fault::Ended(_)) if before.is_none() || before != after => {
            Err(Fault::Restarted)
        }
        gave => gave,
    }
}

/// Whether a check begun at `now` could hold a camera inside the quarter
/// hour before any meeting or during one, as one between two meetings
/// close together would, so no check begins.
pub fn near(calendar: &Calendar, now: u64) -> Option<&Meeting> {
    calendar.meetings.iter().find(|meeting| {
        meeting.start.saturating_sub(UNREAD_MS) < now + READ_MS && now < meeting.end
    })
}

/// The latest time a read begun after `now` may end: the quarter hour
/// before the soonest meeting not yet over. `near` judged the check when it
/// began, but a suspend or stall since can bring the read itself closer.
pub fn read_by(calendar: &Calendar, now: u64) -> u64 {
    calendar
        .meetings
        .iter()
        .filter(|meeting| now < meeting.end)
        .map(|meeting| meeting.start.saturating_sub(UNREAD_MS))
        .min()
        .unwrap_or(u64::MAX)
}

/// Whether a read begun at `now` ends by `by` even if it takes its whole
/// wait.
pub fn in_time(now: u64, by: u64) -> bool {
    now.saturating_add(WAIT.as_secs() * 1000) <= by
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
    // its frames. Nothing there yet, as while the relay restarts and its
    // runtime folder is gone, is no camera to check this minute.
    let named = root.join(device.trim_start_matches('/'));
    let meta = fs::metadata(&named).ok()?;
    if meta.is_file() {
        let text = fs::read_to_string(&named).ok()?;
        let text = text.trim();
        return (!text.is_empty()).then(|| PathBuf::from(text));
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

/// What `v4l2-ctl` is asked of `camera`: a loopback's five frames past its
/// splash, each to /dev/null, and any other camera only what it sends.
pub fn asked(camera: &Path, read: bool) -> Vec<String> {
    let device = camera.to_string_lossy().into_owned();
    let words: &[&str] = if read {
        &[
            "--stream-mmap",
            "--stream-skip",
            SPLASH,
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

/// The longest a meeting's title is told, so a long one never pushes out
/// what Joel should do.
const TITLE: usize = 60;

/// Text with the characters a notice's body reads as markup written out,
/// so a meeting's title, which anyone inviting him sets, is only words,
/// and cut to `TITLE` characters.
fn plain(text: &str) -> String {
    let text: String = if text.chars().count() > TITLE {
        text.chars().take(TITLE - 3).chain("...".chars()).collect()
    } else {
        text.to_owned()
    };
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
        (true, Fault::Restarted) => "The camera gave no picture and had to start again",
        (_, Fault::Missing) => "The camera isn't there",
        (true, Fault::Paused) => "The camera couldn't be read in time",
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

/// The last line `program` said on its errors, for the journal, at most
/// 200 characters.
fn last_said(child: &mut std::process::Child) -> String {
    use std::io::Read as _;
    let mut said = String::new();
    if let Some(mut errors) = child.stderr.take() {
        let _ = errors.read_to_string(&mut said);
    }
    let line = said.lines().rev().find(|line| !line.trim().is_empty());
    line.unwrap_or_default().trim().chars().take(200).collect()
}

/// Runs `program` with `args`, done when it exits 0 within `deadline`.
pub fn within(program: &str, args: &[String], deadline: Duration) -> Result<(), Fault> {
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| Fault::Ended(format!("{program} couldn't run: {error}")))?;
    // The wall clock's deadline too, since the monotonic one stands still
    // across a suspend and a read woken by it would light the camera when
    // the meeting is near.
    let (until, wall) = (Instant::now() + deadline, SystemTime::now() + deadline);
    loop {
        match child.try_wait() {
            Ok(Some(status)) if status.success() => return Ok(()),
            Ok(Some(status)) => {
                let said = last_said(&mut child);
                return Err(Fault::Ended(format!("{program} ended {status}: {said}")));
            }
            Ok(None) if Instant::now() < until && SystemTime::now() < wall => {
                thread::sleep(Duration::from_millis(100));
            }
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                // Cut by the wall clock alone means the machine slept or
                // stalled, not that the program gave nothing in its time.
                return Err(if Instant::now() < until {
                    Fault::Paused
                } else {
                    Fault::Silent
                });
            }
            Err(error) => return Err(Fault::Ended(error.to_string())),
        }
    }
}

/// Whether `camera` gives a picture: read when it's a loopback, and
/// otherwise only asked whether it answers, which a camera another
/// program holds still does. A read that couldn't end by `by`, the epoch
/// milliseconds `read_by` gives, isn't begun.
pub fn gives(camera: &Path, read: bool, by: u64) -> Result<(), Fault> {
    if !read {
        return within("v4l2-ctl", &asked(camera, false), ANSWER);
    }
    let before = relay_run();
    // Judged last before the read, since the asks before it can be slow
    // and a suspend among them would light the camera near the meeting.
    if !in_time(epoch_ms(), by) {
        return Err(Fault::Paused);
    }
    let gave = within("v4l2-ctl", &asked(camera, true), WAIT);
    // A camera silent the whole wait is told whichever run it was on, so
    // the relay isn't asked again and the notice isn't held up.
    if gave == Err(Fault::Silent) {
        return gave;
    }
    judged(gave, before.as_deref(), relay_run().as_deref())
}

/// The wall clock's time, in milliseconds since the epoch.
fn epoch_ms() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |since| {
            u64::try_from(since.as_millis()).unwrap_or(u64::MAX)
        })
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
    fn the_soonest_meeting_is_checked_once_between_18_and_22_minutes_before() {
        let now = 100 * MIN_MS;
        let calendar = at(&[
            now + 23 * MIN_MS,
            now + 19 * MIN_MS + 30_000,
            now + 18 * MIN_MS - 1,
            now + 19 * MIN_MS,
        ]);
        let start = |m: Option<&Meeting>| m.map(|m| m.start);
        assert_eq!(
            start(due(&calendar, now, &[])),
            Some(now + 19 * MIN_MS),
            "the soonest meeting in the window wasn't the one due"
        );
        assert_eq!(
            start(due(&calendar, now, &[now + 19 * MIN_MS])),
            Some(now + 19 * MIN_MS + 30_000),
            "a meeting already checked was checked again"
        );
        assert_eq!(
            start(due(
                &calendar,
                now + MIN_MS,
                &[now + 19 * MIN_MS, now + 19 * MIN_MS + 30_000]
            )),
            Some(now + 23 * MIN_MS),
            "a meeting 22 minutes out wasn't due"
        );
        assert_eq!(
            start(due(&at(&[now + 18 * MIN_MS]), now, &[])),
            Some(now + 18 * MIN_MS),
            "a meeting 18 minutes out wasn't due"
        );
        assert_eq!(
            start(due(&at(&[now + 18 * MIN_MS - 1]), now, &[])),
            None,
            "a meeting under 18 minutes out was checked, its read running into the quarter hour"
        );
        assert_eq!(
            start(due(&at(&[now + 22 * MIN_MS + 1]), now, &[])),
            None,
            "a meeting over 22 minutes out was checked"
        );
    }

    #[test]
    fn a_check_s_longest_run_fits_between_its_window_and_the_quarter_hour() {
        let longest =
            crate::gate::ASKING * u32::try_from(crate::call::UNSURE).unwrap() + 2 * SHOWN + WAIT;
        assert!(
            u128::from(READ_MS) >= longest.as_millis(),
            "a check can take {longest:?}, past the {READ_MS} ms the window keeps for it"
        );
        assert_eq!(
            SOONEST_MS,
            UNREAD_MS + READ_MS,
            "the window's soonest start lets a read run into the quarter hour"
        );
    }

    #[test]
    fn no_check_reads_a_camera_during_a_meeting_or_its_quarter_hour_before() {
        let now = 100 * MIN_MS;
        let start = |m: Option<&Meeting>| m.map(|m| m.start);
        // The one due: its quarter hour begins after the read has ended.
        let due_alone = at(&[now + 18 * MIN_MS]);
        assert_eq!(
            start(near(&due_alone, now)),
            None,
            "the meeting due its check kept its own check from running"
        );
        // Another meeting on now, ending ten minutes before the one due.
        let on = at(&[now - 20 * MIN_MS, now + 20 * MIN_MS]);
        assert_eq!(
            start(near(&on, now)),
            Some(now - 20 * MIN_MS),
            "a camera was read during a meeting"
        );
        // Another starting in 17 minutes, whose quarter hour the read's
        // three minutes would reach.
        let soon = at(&[now + 17 * MIN_MS + 59_999, now + 19 * MIN_MS]);
        assert_eq!(
            start(near(&soon, now)),
            Some(now + 17 * MIN_MS + 59_999),
            "a read ran into another meeting's quarter hour before it"
        );
        let ended = at(&[now - 30 * MIN_MS]);
        assert_eq!(
            start(near(&ended, now)),
            None,
            "a meeting already over kept the camera from its check"
        );
    }

    #[test]
    fn a_read_paused_until_near_a_meeting_is_never_begun() {
        let now = 100 * MIN_MS;
        let two = at(&[now - 30 * MIN_MS, now + 25 * MIN_MS, now + 18 * MIN_MS]);
        let by = read_by(&two, now);
        assert_eq!(
            by,
            now + 3 * MIN_MS,
            "the read's end wasn't the quarter hour before the soonest meeting not yet over"
        );
        assert!(
            in_time(now, by),
            "a read begun with its whole wait to spare was refused"
        );
        // As after a suspend during the asks before it: a minute and a
        // second later, the read's wait would reach the quarter hour.
        assert!(
            !in_time(now + MIN_MS + 1_000, by),
            "a read whose wait reached the quarter hour was begun"
        );
        assert_eq!(
            read_by(&at(&[]), now),
            u64::MAX,
            "no meeting at all kept a read from beginning"
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
        // One he can't open is there but refused, as a camera another
        // program shares out is.
        let node = root.join(format!("dev/video{n}"));
        fs::write(&node, "").unwrap();
        if !opens {
            use std::os::unix::fs::PermissionsExt as _;
            fs::set_permissions(&node, fs::Permissions::from_mode(0o000)).unwrap();
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
        let gone = which(&root.join("nowhere"), Some("/run/camera/device"));
        let alone = root.join("alone");
        fs::create_dir_all(alone.join("run/camera")).unwrap();
        fs::write(alone.join("run/camera/device"), "\n").unwrap();
        let empty = which(&alone, Some("/run/camera/device"));
        // A folder stands in for a camera's node: there, and not a file, so
        // never read as one.
        fs::create_dir_all(root.join("nowhere/dev/video0")).unwrap();
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
            gone, None,
            "a file naming the camera that isn't there yet was taken for it"
        );
        assert_eq!(
            empty, None,
            "an empty file naming the camera was taken for one"
        );
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
            "--device /dev/video32 --stream-mmap --stream-skip 90 --stream-count 5 \
             --stream-to /dev/null",
            "a loopback's frames weren't all sent to /dev/null"
        );
        assert_eq!(
            asked(Path::new("/dev/video0"), false),
            ["--device", "/dev/video0", "--get-fmt-video"],
            "a camera other than a loopback wasn't only asked what it sends"
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
        let again = told(&meeting, 6 * MIN_MS, true, &Fault::Restarted).1;
        assert!(
            again.starts_with("The camera gave no picture and had to start again,"),
            "a relay that started again wasn't told: {again}"
        );
        let long = Meeting {
            title: "x".repeat(200),
            ..meeting
        };
        let cut = told(&long, 6 * MIN_MS, true, &Fault::Silent).1;
        assert!(
            cut.contains(&format!("{}... starts", "x".repeat(57))),
            "a long title wasn't cut short: {cut}"
        );
    }

    #[test]
    fn a_fault_is_told_once_the_window_has_no_minute_left_or_at_once_if_it_s_final() {
        let start = 100 * MIN_MS;
        // A check begun at the window's first run and done in 10 s, and one
        // ending where the next run, a minute's slack given, would be late.
        let early = start - LEAD_MS + 10_000;
        let last = start - 20 * MIN_MS + 1;
        assert_eq!(
            settled(Ok(()), start, early),
            Outcome::Fine,
            "frames weren't taken"
        );
        assert_eq!(
            settled(Err(Fault::Restarted), start, early),
            Outcome::Again(Fault::Restarted),
            "a relay mending itself was told with minutes left to read it again"
        );
        assert_eq!(
            settled(Err(Fault::Missing), start, early),
            Outcome::Again(Fault::Missing),
            "a camera not there yet was told with minutes left to look again"
        );
        assert_eq!(
            settled(Err(Fault::Missing), start, last),
            Outcome::Tell(Fault::Missing),
            "a camera missing for the whole window was never told"
        );
        assert_eq!(
            settled(Err(Fault::Restarted), start, last),
            Outcome::Tell(Fault::Restarted),
            "a relay restarting at the last chance was never told"
        );
        assert_eq!(
            settled(Err(Fault::Ended("v4l2-ctl ended 1".into())), start, early),
            Outcome::Again(Fault::Ended("v4l2-ctl ended 1".into())),
            "a camera that couldn't be opened, as while its relay starts, wasn't tried again"
        );
        assert_eq!(
            settled(Err(Fault::Ended("v4l2-ctl ended 1".into())), start, last),
            Outcome::Tell(Fault::Ended("v4l2-ctl ended 1".into())),
            "a camera that couldn't be opened all window was never told"
        );
        assert_eq!(
            settled(Err(Fault::Silent), start, early),
            Outcome::Tell(Fault::Silent),
            "two minutes of nothing wasn't told at once"
        );
        assert_eq!(
            settled(Err(Fault::Silent), start, start),
            Outcome::Late(Fault::Silent),
            "a meeting already begun was told as starting"
        );
        assert_eq!(
            settled(Err(Fault::Silent), start, start - UNREAD_MS + 1),
            Outcome::Late(Fault::Silent),
            "a read that ended inside the quarter hour, as across a suspend, was told"
        );
        assert_eq!(
            settled(Err(Fault::Paused), start, early),
            Outcome::Again(Fault::Paused),
            "a read cut short by a pause wasn't tried again"
        );
        assert_eq!(
            settled(Err(Fault::Paused), start, last),
            Outcome::Late(Fault::Paused),
            "a read cut short by a pause was told to Joel as a dead camera"
        );
        assert!(
            !last_chance(start, early) && last_chance(start, last),
            "a check whose next run had no minute's slack wasn't told now"
        );
    }

    #[test]
    fn frames_from_a_relay_that_started_again_are_no_picture() {
        assert_eq!(
            judged(Ok(()), Some("a"), Some("a")),
            Ok(()),
            "frames from the same relay weren't taken"
        );
        assert_eq!(
            judged(Ok(()), Some("a"), Some("b")),
            Err(Fault::Restarted),
            "frames from a relay that started again were taken for the camera's"
        );
        assert_eq!(
            judged(Ok(()), Some("a"), None),
            Err(Fault::Restarted),
            "a relay that couldn't be asked after the read was taken as unchanged"
        );
        assert_eq!(
            judged(Ok(()), None, Some("a")),
            Err(Fault::Restarted),
            "a relay that couldn't be asked before the read was taken as unchanged"
        );
        assert_eq!(
            judged(Ok(()), None, None),
            Err(Fault::Restarted),
            "a relay that couldn't be asked at all was taken as unchanged"
        );
        assert_eq!(
            judged(Ok(()), Some(""), Some("a")),
            Err(Fault::Restarted),
            "a relay that started during the read was taken as unchanged"
        );
        assert_eq!(
            judged(Ok(()), Some(""), Some("")),
            Ok(()),
            "a machine with no relay unit had its frames refused"
        );
        assert_eq!(
            judged(
                Err(Fault::Ended("v4l2-ctl ended 1".into())),
                Some("a"),
                Some("b")
            ),
            Err(Fault::Restarted),
            "a camera between its relay's runs was told as one that can't be opened"
        );
        assert_eq!(
            judged(
                Err(Fault::Ended("v4l2-ctl ended 1".into())),
                Some("a"),
                Some("a")
            ),
            Err(Fault::Ended("v4l2-ctl ended 1".into())),
            "a camera that can't be opened was told as a restart"
        );
        assert_eq!(
            judged(Err(Fault::Silent), Some("a"), Some("b")),
            Err(Fault::Silent),
            "a camera that gave nothing was told as something else"
        );
    }

    #[test]
    fn a_program_is_given_its_time_and_no_more() {
        let none: [String; 0] = [];
        assert_eq!(
            within("true", &none, Duration::from_secs(10)),
            Ok(()),
            "a program that exited 0 wasn't taken as done"
        );
        let failed = within(
            "ls",
            &["/a-camera-that-isn-t-there".to_string()],
            Duration::from_secs(10),
        );
        assert!(
            matches!(&failed, Err(Fault::Ended(how)) if how.contains("a-camera-that-isn-t-there")),
            "a program that failed wasn't said to have ended, with what it said: {failed:?}"
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

    #[test]
    fn systemd_s_answer_is_taken_only_when_it_comes_in_time() {
        let words = |said: &[&str]| said.iter().map(|&word| word.to_owned()).collect::<Vec<_>>();
        assert_eq!(
            said("echo", &words(&[" run-a "]), Duration::from_secs(10)),
            Some("run-a".to_string()),
            "an answer in time wasn't taken"
        );
        assert_eq!(
            said("true", &[], Duration::from_secs(10)),
            Some(String::new()),
            "an empty answer, as for no such unit, was taken for a failed ask"
        );
        assert_eq!(
            said("false", &[], Duration::from_secs(10)),
            None,
            "a failed ask was taken for an answer"
        );
        let started = Instant::now();
        assert_eq!(
            said("sleep", &words(&["5"]), Duration::from_millis(300)),
            None,
            "an ask past its time was taken for an answer"
        );
        assert!(
            started.elapsed() < Duration::from_secs(3),
            "an ask past its time was waited on to its end"
        );
    }

    #[test]
    fn systemd_is_asked_again_after_a_refusal_and_a_slow_answer_gets_the_time_left() {
        let words = |said: &[&str]| said.iter().map(|&word| word.to_owned()).collect::<Vec<_>>();
        let asked = stand_in("asked");
        let _ = fs::remove_file(&asked);
        // Refuses its first ask and answers the next, as a bus that turned
        // one call away does.
        let script = format!(
            "test -e {0} && echo run-a || {{ touch {0}; exit 1; }}",
            asked.display()
        );
        assert_eq!(
            answered(
                "bash",
                &words(&["-c", &script]),
                Duration::from_secs(10),
                Duration::from_millis(100)
            ),
            Some("run-a".to_string()),
            "an ask refused once wasn't asked again"
        );
        // Answers after 1.5 s, past what a short fresh ask would wait.
        assert_eq!(
            answered(
                "bash",
                &words(&["-c", "sleep 1.5; echo run-b"]),
                Duration::from_secs(10),
                Duration::from_millis(100)
            ),
            Some("run-b".to_string()),
            "a slow answer wasn't given the time left"
        );
        let started = Instant::now();
        assert_eq!(
            answered(
                "false",
                &[],
                Duration::from_millis(800),
                Duration::from_millis(100)
            ),
            None,
            "an ask refused every time was taken for an answer"
        );
        assert!(
            started.elapsed() < Duration::from_secs(3),
            "asks went on past their time"
        );
        let _ = fs::remove_file(&asked);
    }
}
