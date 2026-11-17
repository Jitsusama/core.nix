//! When a proof may load the link: Joel away, as swayidle tells it, no
//! call on, no audio playing, and nothing the machine names busy.

use std::fs;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::{Mutex, PoisonError};

/// What the gate's idle notice waits for, in seconds.
const IDLE_S: u32 = 120;
/// Where the commands the proof waits on are named, as a JSON list of
/// commands, each a list of its program and arguments: the machine's
/// `jitsusama.shaper.proofWaitsWhile`, which the wrapper sets.
pub const WAITS_ENV: &str = "SHAPER_PROOF_WAITS_WHILE";

/// Reads the commands the proof waits on from `named`, the machine's;
/// none named waits on none.
pub fn waits_while(named: Option<&str>) -> Result<Vec<Vec<String>>, String> {
    let commands: Vec<Vec<String>> = match named.filter(|n| !n.trim().is_empty()) {
        Some(n) => serde_json::from_str(n).map_err(|e| format!("{WAITS_ENV}: {e}"))?,
        None => Vec::new(),
    };
    if commands.iter().any(Vec::is_empty) {
        return Err(format!("{WAITS_ENV}: a command with no program"));
    }
    Ok(commands)
}

/// Whether any of `commands` says the machine is busy: each exits 1
/// while it isn't, and anything else, a program that won't run
/// included, counts as busy, since it couldn't tell.
fn busy(commands: &[Vec<String>]) -> bool {
    commands.iter().any(|command| {
        !Command::new(&command[0])
            .args(&command[1..])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|s| s.code() == Some(1))
    })
}

/// Joel's being away and no call on, held as swayidle tells it.
pub struct Gate {
    mark: PathBuf,
    idle: Mutex<Child>,
    /// What else the load waits on, as `jitsusama.shaper.proofWaitsWhile`
    /// names it.
    waits: Vec<Vec<String>>,
}

impl Gate {
    pub fn new(waits: Vec<Vec<String>>) -> Option<Gate> {
        let runtime = std::env::var_os("XDG_RUNTIME_DIR")?;
        let mark = PathBuf::from(runtime).join(format!("shaper-proof-idle-{}", std::process::id()));
        let _ = fs::remove_file(&mark);
        let shown = mark.display().to_string();
        let idle = Command::new("swayidle")
            .args(["timeout", &IDLE_S.to_string(), &format!("touch {shown}")])
            .args(["resume", &format!("rm -f {shown}")])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .spawn()
            .ok()?;
        Some(Gate {
            mark,
            idle: Mutex::new(idle),
            waits,
        })
    }

    /// Ends swayidle and its mark, once the proof needs them no more: a
    /// stopped proof doesn't wait on the watch still holding the gate, so
    /// dropping it alone would leave swayidle running. Asked again, it
    /// does nothing more.
    pub fn end(&self) {
        let mut idle = self.idle.lock().unwrap_or_else(PoisonError::into_inner);
        let _ = idle.kill();
        let _ = idle.wait();
        let _ = fs::remove_file(&self.mark);
    }

    /// Whether the link may carry the load now. A call detector that
    /// can't tell is taken for a call, and so is audio playing, and the
    /// commands waited on are asked last, being the slowest.
    pub fn open(&self) -> bool {
        self.mark.exists()
            && Command::new("a-call-is-on")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .is_ok_and(|s| s.code() == Some(1))
            && !audio_plays()
            && !busy(&self.waits)
    }
}

/// Whether audio plays, as `pw-dump` shows PipeWire's graph. Joel may
/// watch a stream without touching the keys, so idle alone doesn't say
/// he's away. A graph that can't be read is taken for audio playing.
fn audio_plays() -> bool {
    Command::new("pw-dump")
        .stderr(Stdio::null())
        .output()
        .ok()
        .filter(|out| out.status.success())
        .and_then(|out| serde_json::from_slice(&out.stdout).ok())
        .is_none_or(|graph| plays(&graph))
}

/// Whether any sink runs in `graph`: a sink runs while a stream plays to
/// it, and is idle or suspended otherwise.
fn plays(graph: &serde_json::Value) -> bool {
    graph.as_array().is_some_and(|objects| {
        objects.iter().any(|object| {
            object["type"] == "PipeWire:Interface:Node"
                && object["info"]["props"]["media.class"] == "Audio/Sink"
                && object["info"]["state"] == "running"
        })
    })
}

impl Drop for Gate {
    fn drop(&mut self) {
        self.end();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn node(class: &str, state: &str) -> serde_json::Value {
        json!({
            "type": "PipeWire:Interface:Node",
            "info": { "state": state, "props": { "media.class": class } },
        })
    }

    #[test]
    fn a_running_sink_is_audio_playing() {
        let quiet = json!([
            node("Audio/Sink", "suspended"),
            node("Audio/Sink", "idle"),
            node("Audio/Source", "running"),
            { "type": "PipeWire:Interface:Client", "info": { "state": "running" } },
        ]);
        assert!(
            !plays(&quiet),
            "took a graph with no sink running for audio"
        );
        let playing = json!([
            node("Audio/Sink", "suspended"),
            node("Audio/Sink", "running")
        ]);
        assert!(plays(&playing), "missed a sink running");
    }

    #[test]
    fn a_command_waited_on_that_says_busy_holds_the_load() {
        let exits = |code: u8| vec!["sh".to_string(), "-c".into(), format!("exit {code}")];
        assert!(busy(&[exits(1), exits(0)]), "loaded through a meeting");
        assert!(
            busy(&[exits(2)]),
            "loaded through a calendar it couldn't read"
        );
        assert!(
            busy(&[vec!["no-such-program-here".into()]]),
            "loaded where a command waited on wouldn't run"
        );
        assert!(!busy(&[exits(1), exits(1)]), "held with nothing busy");
        assert!(!busy(&[]), "held with nothing to wait on");
    }

    #[test]
    fn the_commands_waited_on_are_read() {
        let named = r#"[["a-meeting-is-near", "/calls", "/off"]]"#;
        assert_eq!(
            waits_while(Some(named)).expect("a command"),
            vec![vec!["a-meeting-is-near", "/calls", "/off"]]
        );
        assert_eq!(waits_while(None).expect("none"), Vec::<Vec<String>>::new());
        assert!(
            waits_while(Some("[[]]")).is_err(),
            "took a command with no program"
        );
        assert!(
            waits_while(Some("a-meeting-is-near")).is_err(),
            "took what isn't JSON"
        );
    }
}
