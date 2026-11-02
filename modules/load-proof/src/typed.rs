//! Typed: how long Joel's own keys in the launcher take to reach the
//! screen while the machine is loaded.
//!
//! Nothing can type as Joel does without a virtual keyboard, which his
//! account isn't given, so this signal types nothing. At the end of its
//! length it asks the launcher for what it keeps of the last keys (`qs ipc
//! call launcher framed`): each one's press, when its handler took it and
//! when it reached a frame, in milliseconds of the monotonic clock. It
//! keeps the keys the handler took since the signal started, and times
//! each from its press to its frame, or from its handler when no press
//! reached the launcher. A step Joel typed nothing in counts none.

use std::process::{Command, Stdio};
use std::time::Duration;

use rustix::time::{ClockId, clock_gettime};

/// Waits `length`, then gives the time from press to frame of every key
/// the launcher took meanwhile.
///
/// # Errors
///
/// When the launcher can't be asked, or answers with something it doesn't
/// keep.
pub fn watch(length: Duration) -> Result<Vec<Duration>, String> {
    let since = now_ms();
    std::thread::sleep(length);
    let out = Command::new("qs")
        .args(["ipc", "call", "launcher", "framed"])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .map_err(|e| format!("asking the launcher: {e}"))?;
    if !out.status.success() {
        return Err(format!("the launcher didn't answer: {}", out.status));
    }
    delays(&String::from_utf8_lossy(&out.stdout), since)
}

/// The monotonic clock now, in milliseconds, as the launcher counts.
fn now_ms() -> f64 {
    let now = clock_gettime(ClockId::Monotonic);
    let seconds = u64::try_from(now.tv_sec).unwrap_or(0);
    let nanos = u32::try_from(now.tv_nsec).unwrap_or(0);
    Duration::new(seconds, nanos).as_secs_f64() * 1e3
}

/// The time from press to frame of each key in the launcher's answer
/// `said` that its handler took at or after `since_ms`.
///
/// # Errors
///
/// When the answer lacks one of its lists, or a list holds something
/// other than a number or null.
pub fn delays(said: &str, since_ms: f64) -> Result<Vec<Duration>, String> {
    let pressed = list(said, "pressed")?;
    let heard = list(said, "heard")?;
    let frames = list(said, "frames")?;
    let keys = heard
        .iter()
        .zip(&frames)
        .enumerate()
        .filter_map(|(at, (heard, frame))| {
            let (heard, frame) = ((*heard)?, (*frame)?);
            let from = pressed.get(at).copied().flatten().unwrap_or(heard);
            (heard >= since_ms).then(|| Duration::from_secs_f64((frame - from).max(0.0) / 1e3))
        })
        .collect();
    Ok(keys)
}

/// The list named `name` in the answer, each figure or null.
fn list(said: &str, name: &str) -> Result<Vec<Option<f64>>, String> {
    let key = format!("\"{name}\":[");
    let start = said
        .find(&key)
        .ok_or_else(|| format!("the launcher gave no {name}"))?
        + key.len();
    let end = said[start..]
        .find(']')
        .ok_or_else(|| format!("the launcher's {name} never ended"))?
        + start;
    said[start..end]
        .split(',')
        .map(str::trim)
        .filter(|word| !word.is_empty())
        .map(|word| match word {
            "null" => Ok(None),
            word => word
                .parse()
                .map(Some)
                .map_err(|e| format!("the launcher's {name} held {word}: {e}")),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAID: &str = concat!(
        r#"{"timed":4,"clock":"monotonic","pressed":[100.0,null,1000.5,1030.0],"#,
        r#""heard":[101.0,210.0,1001.0,1031.0],"frames":[105.0,240.0,1020.5,1131.0]}"#,
    );

    fn ms(times: &[Duration]) -> Vec<f64> {
        times
            .iter()
            .map(|time| (time.as_secs_f64() * 1e4).round() / 10.0)
            .collect()
    }

    #[test]
    fn it_times_each_key_from_its_press_to_its_frame() {
        assert_eq!(ms(&delays(SAID, 0.0).unwrap()), [5.0, 30.0, 20.0, 101.0]);
    }

    #[test]
    fn it_keeps_only_the_keys_taken_since_it_started() {
        assert_eq!(ms(&delays(SAID, 1000.0).unwrap()), [20.0, 101.0]);
    }

    #[test]
    fn a_key_whose_press_went_missing_is_timed_from_its_handler() {
        assert_eq!(ms(&delays(SAID, 200.0).unwrap())[..1], [30.0]);
    }

    #[test]
    fn a_step_with_no_keys_counts_none() {
        assert!(delays(SAID, 2000.0).unwrap().is_empty());
    }

    #[test]
    fn an_answer_without_its_frames_is_refused() {
        let said = r#"{"pressed":[1.0],"heard":[2.0]}"#;
        assert_eq!(delays(said, 0.0), Err("the launcher gave no frames".into()));
    }
}
