//! The sound signal: whether PipeWire keeps every sound flowing while the
//! work behind Joel fills the machine.
//!
//! A node that misses its turn in PipeWire's graph (an xrun) is heard as a
//! click or a gap, and PipeWire counts each one in the node's errors, which
//! `pw-top` shows. Nothing playing, every sink is suspended and nothing can
//! miss a turn, so the signal plays silence of its own, at the latency a
//! call asks for, to keep the default sink's graph running. It reads every
//! node's errors before and after, and which nodes run halfway through,
//! and says which of those rose. Silence makes no
//! sound, so it can run while Joel sleeps; whatever else plays meanwhile,
//! Spotify or a call, is counted too.

use std::collections::{BTreeMap, BTreeSet};
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
    rows(top).map(|row| (row.name, row.errors)).collect()
}

/// The nodes running as `pw-top -b` printed: those carrying sound then.
pub fn running(top: &str) -> BTreeSet<String> {
    rows(top)
        .filter(|row| row.state == "R")
        .map(|row| row.name)
        .collect()
}

/// One node as `pw-top` shows it.
struct Row {
    state: String,
    name: String,
    errors: u64,
}

/// Every node line `pw-top -b` printed, in order; header lines and video
/// nodes are left out.
fn rows(top: &str) -> impl Iterator<Item = Row> + '_ {
    top.lines().filter_map(|line| {
        let fields: Vec<&str> = line.split_whitespace().collect();
        // State, id, quantum, rate, wait, busy, W/Q, B/Q, errors, then the
        // format, which may be blank or several words, and the name last.
        if fields.len() < 10 || fields[1].parse::<u32>().is_err() {
            return None;
        }
        let name = *fields.last()?;
        if NOT_SOUND.iter().any(|prefix| name.starts_with(prefix)) {
            return None;
        }
        Some(Row {
            state: fields[0].to_owned(),
            name: name.to_owned(),
            errors: fields[8].parse().ok()?,
        })
    })
}

/// The nodes in `carrying` whose errors rose from `before` to `after`, and
/// by how many. A node that appeared meanwhile counts from none. A node
/// that carried nothing can't be heard, whatever it counts: a microphone
/// nobody records from may count errors while it idles.
pub fn rose(
    before: &BTreeMap<String, u64>,
    after: &BTreeMap<String, u64>,
    carrying: &BTreeSet<String>,
) -> Vec<(String, u64)> {
    after
        .iter()
        .filter(|(name, _)| carrying.contains(*name))
        .filter_map(|(name, &now)| {
            let was = before.get(name).copied().unwrap_or(0);
            (now > was).then(|| (name.clone(), now - was))
        })
        .collect()
}

/// Plays silence for `length` and gives the nodes carrying sound whose
/// errors rose. Which nodes carry sound is read halfway through.
pub fn watch(length: Duration) -> Result<Vec<(String, u64)>, String> {
    let before = errors(&top()?);
    let player = std::thread::spawn(move || play_silence(length));
    std::thread::sleep(length / 2);
    let carrying = running(&top()?);
    player
        .join()
        .map_err(|_| "playing silence panicked".to_owned())??;
    let after = errors(&top()?);
    Ok(rose(&before, &after, &carrying))
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
R   73    256  48000   3.1us   1.0us  0.01  0.00    3    S32LE 2 48000 alsa_output.Speaker__sink
S   ID  QUANT   RATE    WAIT    BUSY   W/Q   B/Q  ERR FORMAT           NAME 
S   31      0      0    ---     ---   ---   ---     0                  Dummy-Driver
I   72    256  48000   5.4us   1.2us  0.00  0.00   92    S32LE 2 48000 alsa_input.Mic__source
S   41      0      0    ---     ---   ---   ---     0                   + shure_mv7_voice_input
S   46      0      0    ---     ---   ---   ---     9                  v4l2_input.video33
R   73    256  48000   3.1us   1.0us  0.01  0.00    4    S32LE 2 48000 alsa_output.Speaker__sink
";

    #[test]
    fn each_nodes_errors_are_read_from_its_last_iteration() {
        let errors = errors(TOP);
        assert_eq!(errors.get("alsa_input.Mic__source"), Some(&92));
        assert_eq!(errors.get("Dummy-Driver"), Some(&0));
        assert_eq!(errors.get("shure_mv7_voice_input"), Some(&0), "a follower");
        assert_eq!(errors.get("alsa_output.Speaker__sink"), Some(&4));
        assert_eq!(errors.len(), 4, "{errors:?}");
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
        let carrying = BTreeSet::from(["sink", "source", "stream"].map(str::to_owned));
        assert_eq!(
            rose(&before, &after, &carrying),
            [("source".to_owned(), 2), ("stream".to_owned(), 1)]
        );
    }

    #[test]
    fn only_running_nodes_carry_sound() {
        assert_eq!(
            running(TOP),
            BTreeSet::from(["alsa_output.Speaker__sink".to_owned()])
        );
    }

    #[test]
    fn a_node_that_carried_nothing_isnt_heard_whatever_it_counts() {
        let before = BTreeMap::from([("mic".to_owned(), 50), ("sink".to_owned(), 0)]);
        let after = BTreeMap::from([("mic".to_owned(), 98), ("sink".to_owned(), 1)]);
        let carrying = BTreeSet::from(["sink".to_owned()]);
        assert_eq!(rose(&before, &after, &carrying), [("sink".to_owned(), 1)]);
    }

    #[test]
    fn nothing_rose_when_nothing_changed() {
        let errors = errors(TOP);
        assert!(rose(&errors, &errors, &running(TOP)).is_empty());
    }
}
