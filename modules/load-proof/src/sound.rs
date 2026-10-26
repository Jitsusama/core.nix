//! The sound signal: whether PipeWire keeps every sound flowing while the
//! work behind Joel fills the machine.
//!
//! A node that misses its turn in PipeWire's graph (an xrun) is heard as a
//! click or a gap, and PipeWire counts each one in the node's errors, which
//! `pw-top` shows. Nothing playing, every sink is suspended and nothing can
//! miss a turn, so the signal plays silence of its own, at the latency a
//! call asks for, to keep the default sink's graph running. It reads every
//! node's errors before and after, and says which rose. Silence makes no
//! sound, so it can run while Joel sleeps; whatever else plays meanwhile,
//! Spotify or a call, is counted too.

use std::collections::BTreeMap;
use std::io::Write;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// Where PipeWire's tools are: the build names them, and otherwise they're
/// looked for on the path.
const TOOLS: Option<&str> = option_env!("PIPEWIRE_BIN");

/// A call's latency: 256 frames at 48 kHz, about 5 ms, which is what
/// PipeWire's graph runs at while a call is on.
const LATENCY: &str = "256/48000";
const RATE: usize = 48_000;
/// Two channels of 16-bit samples.
const FRAME: usize = 4;

/// Nodes whose errors aren't sound: cameras, which PipeWire also carries.
const NOT_SOUND: [&str; 2] = ["v4l2_", "libcamera"];

/// Each node's errors, by name, from what `pw-top -b` prints. A node shown
/// in several iterations takes its last; header lines and video nodes are
/// left out.
pub fn errors(top: &str) -> BTreeMap<String, u64> {
    let mut errors = BTreeMap::new();
    for line in top.lines() {
        let fields: Vec<&str> = line.split_whitespace().collect();
        // State, id, quantum, rate, wait, busy, W/Q, B/Q, errors, then the
        // format, which may be blank or several words, and the name last.
        let (Some(id), Some(count), Some(name)) = (fields.get(1), fields.get(8), fields.last())
        else {
            continue;
        };
        if fields.len() < 10 || id.parse::<u32>().is_err() {
            continue;
        }
        let Ok(count) = count.parse() else {
            continue;
        };
        if NOT_SOUND.iter().any(|prefix| name.starts_with(prefix)) {
            continue;
        }
        errors.insert((*name).to_owned(), count);
    }
    errors
}

/// The nodes whose errors rose from `before` to `after`, and by how many.
/// A node that appeared meanwhile counts from none.
pub fn rose(before: &BTreeMap<String, u64>, after: &BTreeMap<String, u64>) -> Vec<(String, u64)> {
    after
        .iter()
        .filter_map(|(name, &now)| {
            let was = before.get(name).copied().unwrap_or(0);
            (now > was).then(|| (name.clone(), now - was))
        })
        .collect()
}

/// Plays silence for `length` and gives the nodes whose errors rose.
pub fn watch(length: Duration) -> Result<Vec<(String, u64)>, String> {
    let before = errors(&top()?);
    play_silence(length)?;
    let after = errors(&top()?);
    Ok(rose(&before, &after))
}

fn tool(name: &str) -> String {
    TOOLS.map_or_else(|| name.to_owned(), |bin| format!("{bin}/{name}"))
}

/// What `pw-top` shows of the graph. Its first iteration can come before
/// every node has reported, so two are read and the last is kept.
fn top() -> Result<String, String> {
    let out = Command::new(tool("pw-top"))
        .args(["-b", "-n", "2"])
        .stderr(Stdio::null())
        .output()
        .map_err(|e| format!("running pw-top: {e}"))?;
    if !out.status.success() {
        return Err(format!("pw-top failed: {}", out.status));
    }
    String::from_utf8(out.stdout).map_err(|e| format!("reading pw-top: {e}"))
}

/// Plays `length` of silence to the default sink. `pw-cat` takes it as
/// quickly as it plays it, so writing it paces itself.
fn play_silence(length: Duration) -> Result<(), String> {
    let mut player = Command::new(tool("pw-cat"))
        .args([
            "--playback",
            "--raw",
            "--format",
            "s16",
            "--rate",
            "48000",
            "--channels",
            "2",
            "--latency",
            LATENCY,
            "--media-role",
            "Communication",
            "-",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("running pw-cat: {e}"))?;
    let mut input = player.stdin.take().ok_or("pw-cat took no input")?;
    // A tenth of a second at a time, so the end comes on time.
    let silence = vec![0u8; RATE / 10 * FRAME];
    let end = Instant::now() + length;
    let wrote = loop {
        if Instant::now() >= end {
            break Ok(());
        }
        if let Err(e) = input.write_all(&silence) {
            break Err(format!("playing silence: {e}"));
        }
    };
    drop(input);
    let _ = player.kill();
    let _ = player.wait();
    wrote
}

#[cfg(test)]
mod tests {
    use super::*;

    // Two iterations of `pw-top -b`, as a laptop printed them, cut short.
    const TOP: &str = "\
S   ID  QUANT   RATE    WAIT    BUSY   W/Q   B/Q  ERR FORMAT           NAME 
S   31      0      0    ---     ---   ---   ---     0                  Dummy-Driver
I   72    256  48000   5.4us   1.2us  0.00  0.00   90    S32LE 2 48000 alsa_input.Mic__source
S   41      0      0    ---     ---   ---   ---     0                   + shure_mv7_voice_input
S   46      0      0    ---     ---   ---   ---     7                  v4l2_input.video33
S   ID  QUANT   RATE    WAIT    BUSY   W/Q   B/Q  ERR FORMAT           NAME 
S   31      0      0    ---     ---   ---   ---     0                  Dummy-Driver
I   72    256  48000   5.4us   1.2us  0.00  0.00   92    S32LE 2 48000 alsa_input.Mic__source
S   41      0      0    ---     ---   ---   ---     0                   + shure_mv7_voice_input
S   46      0      0    ---     ---   ---   ---     9                  v4l2_input.video33
";

    #[test]
    fn each_nodes_errors_are_read_from_its_last_iteration() {
        let errors = errors(TOP);
        assert_eq!(errors.get("alsa_input.Mic__source"), Some(&92));
        assert_eq!(errors.get("Dummy-Driver"), Some(&0));
        assert_eq!(errors.get("shure_mv7_voice_input"), Some(&0), "a follower");
        assert_eq!(errors.len(), 3, "{errors:?}");
    }

    #[test]
    fn a_camera_is_not_sound() {
        assert!(!errors(TOP).contains_key("v4l2_input.video33"));
    }

    #[test]
    fn only_nodes_whose_errors_rose_are_named() {
        let before = BTreeMap::from([("sink".to_owned(), 3), ("source".to_owned(), 5)]);
        let after = BTreeMap::from([
            ("sink".to_owned(), 3),
            ("source".to_owned(), 7),
            ("stream".to_owned(), 1),
        ]);
        assert_eq!(
            rose(&before, &after),
            [("source".to_owned(), 2), ("stream".to_owned(), 1)]
        );
    }

    #[test]
    fn nothing_rose_when_nothing_changed() {
        let errors = errors(TOP);
        assert!(rose(&errors, &errors).is_empty());
    }
}
