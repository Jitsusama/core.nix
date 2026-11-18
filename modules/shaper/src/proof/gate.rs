//! When a proof may load the link: Joel away, as swayidle tells it, no
//! call on, no audio playing, and nothing the machine names busy.

use super::load::end_group;
use super::stop::STOPPED;
use std::fs;
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::Ordering;
use std::sync::{Mutex, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

/// What the gate's idle notice waits for, in seconds.
const IDLE_S: u32 = 120;
/// Where the commands the proof waits on are named, as a JSON list of
/// commands, each a list of its program and arguments: the machine's
/// `jitsusama.shaper.proofWaitsWhile`, which the wrapper sets.
pub const WAITS_ENV: &str = "SHAPER_PROOF_WAITS_WHILE";

/// How long a command the gate asks may take.
const ASKED_FOR: Duration = Duration::from_secs(10);

/// The code `command` exits with, in a group of its own so the
/// terminal's Ctrl-C or Ctrl-Z reaches only the proof; None if it won't
/// run, or if it's still going `within` from now or at a stop, when the
/// whole of its group is ended, so a hung one can't hold the gate.
fn exits(mut command: Command, within: Duration, halted: impl Fn() -> bool) -> Option<i32> {
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .process_group(0)
        .spawn()
        .ok()?;
    let by = Instant::now() + within;
    loop {
        if let Ok(Some(status)) = child.try_wait() {
            return status.code();
        }
        if halted() || Instant::now() >= by {
            end_group(&child);
            let _ = child.wait();
            return None;
        }
        thread::sleep(Duration::from_millis(50));
    }
}

/// Whether a signal has asked the proof to stop.
fn signalled() -> bool {
    STOPPED.load(Ordering::Relaxed)
}

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
/// while it isn't, and anything else, a program that won't run or one
/// cut off included, counts as busy, since it couldn't tell. One that
/// hangs is said, since the proof's 75 would otherwise blame a meeting.
fn busy(commands: &[Vec<String>], halted: impl Fn() -> bool) -> bool {
    commands.iter().any(|command| {
        let mut asked = Command::new(&command[0]);
        asked.args(&command[1..]);
        let asked_at = Instant::now();
        let code = exits(asked, ASKED_FOR, &halted);
        if code.is_none() && asked_at.elapsed() >= ASKED_FOR {
            super::stop::tell(&format!(
                "{} didn't answer within {} s, so it counts as busy",
                command[0],
                ASKED_FOR.as_secs()
            ));
        }
        code != Some(1)
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
        // Only one proof runs at a time, so any other proof's mark is
        // one a proof killed outright left.
        if let Ok(marks) = fs::read_dir(mark.parent()?) {
            for left in marks.flatten() {
                if left
                    .file_name()
                    .to_string_lossy()
                    .starts_with("shaper-proof-idle-")
                {
                    let _ = fs::remove_file(left.path());
                }
            }
        }
        // swayidle hands each command to a shell, so the path is quoted.
        let shown = format!("'{}'", mark.display().to_string().replace('\'', r"'\''"));
        let mut idle = Command::new("swayidle");
        idle.args(["timeout", &IDLE_S.to_string(), &format!("touch {shown}")])
            .args(["resume", &format!("rm -f {shown}")])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            // In a group of its own, so a Ctrl-C or a closed terminal
            // reaches the proof alone, which then ends it, rather than
            // ending it first and reading as a swayidle lost.
            .process_group(0);
        // SAFETY: prctl(2) and getppid(2) are async-signal-safe, and an
        // error from a raw errno allocates nothing, so fit to run between
        // fork and exec. It ends swayidle once the thread
        // that started it, the proof's main one, ends, however the proof
        // ends; a proof gone before the prctl is caught by its parent
        // being another, and swayidle never starts.
        let parent = libc::pid_t::try_from(std::process::id()).ok()?;
        unsafe {
            idle.pre_exec(move || {
                libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM);
                if libc::getppid() == parent {
                    Ok(())
                } else {
                    Err(std::io::Error::from_raw_os_error(libc::ESRCH))
                }
            });
        }
        let idle = idle.spawn().ok()?;
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
    /// commands waited on are asked last, being the slowest. A swayidle
    /// gone closes it too, since its mark would then never be taken back
    /// when Joel returns.
    pub fn open(&self) -> bool {
        self.mark.exists()
            && !self.lost()
            && exits(Command::new("a-call-is-on"), ASKED_FOR, signalled) == Some(1)
            && !audio_plays()
            && !busy(&self.waits, signalled)
    }

    /// Why the gate is shut, for Joel: a lost swayidle said as itself, not
    /// as his coming back.
    pub fn why_shut(&self) -> &'static str {
        if self.lost() {
            "swayidle went away, so nothing hears whether Joel is back"
        } else {
            "Joel is back, or a call, audio or a meeting started"
        }
    }

    /// Whether swayidle has gone, so nothing hears Joel come back and the
    /// gate can never open again.
    pub fn lost(&self) -> bool {
        gone(&mut self.idle.lock().unwrap_or_else(PoisonError::into_inner))
    }
}

/// Whether `child` has ended, or can't be asked.
fn gone(child: &mut Child) -> bool {
    !matches!(child.try_wait(), Ok(None))
}

/// Whether audio plays, as `pw-dump` shows PipeWire's graph. Joel may
/// watch a stream without touching the keys, so idle alone doesn't say
/// he's away. A graph that can't be read is taken for audio playing.
fn audio_plays() -> bool {
    // Cut off as any command the gate asks is, leaving a graph cut short,
    // which can't be read.
    Command::new("pw-dump")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .process_group(0)
        .spawn()
        .ok()
        .map(|dump| super::probes::finish(dump, Instant::now() + ASKED_FOR, signalled))
        .and_then(|graph| serde_json::from_str(&graph).ok())
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
    fn a_swayidle_ended_is_gone() {
        let mut ended = Command::new("true").spawn().expect("true runs");
        let _ = ended.wait();
        assert!(gone(&mut ended), "an ended swayidle read as running");
        let mut running = Command::new("sleep").arg("5").spawn().expect("sleep runs");
        assert!(!gone(&mut running), "a running swayidle read as gone");
        let _ = running.kill();
        let _ = running.wait();
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
        // Not the process-wide stop, which the signal tests set.
        let busy = |commands: &[Vec<String>]| busy(commands, || false);
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
    fn a_command_asked_is_cut_off_at_a_stop_or_when_it_hangs() {
        // Its background sleep says its pid, so its group is known.
        let pids = std::env::temp_dir().join(format!("shaper-gate-{}", std::process::id()));
        let hangs = || {
            let mut c = Command::new("sh");
            c.args(["-c", "sleep 30 & echo $! >\"$1\"; sleep 30", "sh"])
                .arg(&pids);
            c
        };
        let asked = Instant::now();
        assert_eq!(exits(hangs(), ASKED_FOR, || true), None, "a stop waited on");
        // A second for the hung one, so a busy machine's shell has time to
        // say its pid before it's cut off.
        assert_eq!(exits(hangs(), Duration::from_secs(1), || false), None);
        assert!(
            asked.elapsed() < Duration::from_secs(3),
            "a hung command held the gate {:?}",
            asked.elapsed()
        );
        let child: u32 = fs::read_to_string(&pids)
            .ok()
            .and_then(|p| p.trim().parse().ok())
            .expect("the background sleep's pid");
        let _ = fs::remove_file(&pids);
        let dies = Instant::now();
        while fs::read_to_string(format!("/proc/{child}/stat")).is_ok_and(|s| {
            s.rsplit_once(") ")
                .is_some_and(|(_, r)| !r.starts_with('Z'))
        }) && dies.elapsed() < Duration::from_secs(1)
        {
            thread::sleep(Duration::from_millis(20));
        }
        assert!(
            dies.elapsed() < Duration::from_secs(1),
            "a cut-off command's own child ran on"
        );
        let mut answers = Command::new("sh");
        answers.args(["-c", "exit 1"]);
        assert_eq!(exits(answers, ASKED_FOR, || false), Some(1));
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
