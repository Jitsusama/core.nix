//! What a file shows of itself, for a drawer that shows it, asked for by
//! its key, `preview:<path>`: Capture's things taken and Neovim's files.
//! One `file-preview serve` is kept for every ask, started at the first
//! and asked a line at a time, so nothing starts for each drawer; its
//! program comes from `PROGRAMS_PREVIEW`. A file is asked again only once
//! it changed since it was last said. One that stops before answering is
//! started again for the next ask, but not within a minute of the last
//! that did, so one that can't start isn't started again and again, and
//! each record says whether it answered (`answering`), so a drawer says
//! it isn't rather than leaving its facts unsaid. One that says nothing
//! of a file within `ANSWERS_WITHIN` is stopped, as stuck on it, and that
//! file is said as not answering, unasked, until it changes, so one file
//! never holds up every read behind it (Y2YBTS, 2026-11-11).

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::time::{Duration, Instant, UNIX_EPOCH};

use serde_json::{Value, json};

/// How long after a file-preview stops unanswered none is started.
const RESTING: Duration = Duration::from_secs(60);

/// How long file-preview has to say what a file shows before it's taken
/// as stuck on it: a picture, a page or a frame is said within a second.
const ANSWERS_WITHIN: Duration = Duration::from_secs(3);

/// The path a key names, whole and on one line, or none.
#[must_use]
pub fn path_of(key: &str) -> Option<&str> {
    key.strip_prefix("preview:")
        .filter(|path| path.starts_with('/') && !path.contains('\n'))
}

/// When a file last changed, in milliseconds since 1970, as file-preview
/// says it (`changed`), or none for one that isn't there.
fn changed_ms(path: &str) -> Option<u64> {
    let changed = std::fs::metadata(Path::new(path)).ok()?.modified().ok()?;
    u64::try_from(changed.duration_since(UNIX_EPOCH).ok()?.as_millis()).ok()
}

/// Whether a record held says the file as it is now, so it needn't be
/// asked again.
#[must_use]
pub fn current(held: Option<&Value>, path: &str) -> bool {
    held.is_some_and(|record| {
        record["answering"] == json!(true)
            && record["shown"]["changed"].as_u64().is_some()
            && record["shown"]["changed"].as_u64() == changed_ms(path)
    })
}

/// What file-preview said of a file as its facts: null where it couldn't
/// say, or said anything but an object of facts.
fn facts(said: Option<&Value>) -> Value {
    said.filter(|said| said.is_object() && said.get("error").is_none())
        .cloned()
        .unwrap_or(Value::Null)
}

/// The one file-preview kept, while it runs.
struct Serving {
    child: Child,
    input: ChildStdin,
    /// Each line it says, read on a thread of its own, so an answer is
    /// waited on only so long.
    said: Receiver<String>,
}

// Its whole group is stopped, so a pdftoppm or ffmpeg it waits on, stuck
// on the same file, goes with it rather than running on.
impl Drop for Serving {
    fn drop(&mut self) {
        if let Ok(group) = i32::try_from(self.child.id()) {
            // SAFETY: kill takes no pointers; the group is the child's own,
            // made as it started, and not yet waited on, so not reused.
            unsafe { libc::kill(-group, libc::SIGKILL) };
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// The file-preview kept for every ask.
#[derive(Default)]
pub struct Previewer {
    program: Option<String>,
    serving: Option<Serving>,
    /// When the last one stopped before answering anything.
    stopped: Option<Instant>,
    /// Each file one was stuck on, with when it had last changed then.
    stuck: HashMap<String, Option<u64>>,
}

impl Previewer {
    /// One that runs `program`, or none at all where none is named.
    #[must_use]
    pub fn new(program: Option<String>) -> Previewer {
        Previewer {
            program: program.filter(|program| !program.is_empty()),
            ..Previewer::default()
        }
    }

    /// The record a key names, asking file-preview, or none for a key that
    /// isn't a preview's. What it said is `shown`, or null where it
    /// couldn't say, as of a file that isn't there.
    pub fn record(&mut self, key: &str) -> Option<Value> {
        let path = path_of(key)?;
        let said = self.show(path);
        Some(json!({
            "key": key, "kind": "preview", "path": path,
            "answering": said.is_some(), "shown": facts(said.as_ref()),
        }))
    }

    /// What file-preview says of a path, starting one where none runs, or
    /// none where none answers.
    fn show(&mut self, path: &str) -> Option<Value> {
        if let Some(changed) = self.stuck.get(path) {
            if *changed == changed_ms(path) {
                return None;
            }
            self.stuck.remove(path);
        }
        if self.serving.is_none() {
            self.serving = self.start();
        }
        let serving = self.serving.as_mut()?;
        let asked = writeln!(serving.input, "show {path}").and_then(|()| serving.input.flush());
        let said = match asked {
            Ok(()) => serving.said.recv_timeout(ANSWERS_WITHIN),
            Err(_) => Err(RecvTimeoutError::Disconnected),
        };
        match said {
            Ok(line) => Some(serde_json::from_str(&line).unwrap_or(Value::Null)),
            Err(RecvTimeoutError::Timeout) => {
                self.serving = None;
                self.stuck.insert(path.to_owned(), changed_ms(path));
                eprintln!(
                    "programs: file-preview said nothing of {path} within {ANSWERS_WITHIN:?}, \
                     so it was stopped, and the file says it isn't answered until it changes"
                );
                None
            }
            Err(RecvTimeoutError::Disconnected) => {
                self.serving = None;
                self.stopped = Some(Instant::now());
                eprintln!("programs: file-preview stopped before answering; drawers say so");
                None
            }
        }
    }

    fn start(&self) -> Option<Serving> {
        let program = self.program.as_ref()?;
        if self.stopped.is_some_and(|at| at.elapsed() < RESTING) {
            return None;
        }
        let mut child = Command::new(program)
            .arg("serve")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .process_group(0)
            .spawn()
            .inspect_err(|error| eprintln!("programs: file-preview didn't start: {error}"))
            .ok()?;
        let input = child.stdin.take()?;
        let output = BufReader::new(child.stdout.take()?);
        let (tell, said) = mpsc::channel();
        // It ends as the output does, once the child is stopped.
        std::thread::spawn(move || {
            for line in output.lines() {
                let Ok(line) = line else { return };
                if tell.send(line).is_err() {
                    return;
                }
            }
        });
        Some(Serving { child, input, said })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_whole_path_on_one_line_is_a_preview() {
        assert_eq!(path_of("preview:/a.png"), Some("/a.png"), "a whole path");
        assert_eq!(path_of("preview:a.png"), None, "a path from nowhere");
        assert_eq!(path_of("preview:/a\n.png"), None, "a path of two lines");
        assert_eq!(path_of("folder:/a.png"), None, "another kind");
    }

    #[test]
    fn what_it_said_is_its_facts_and_nothing_where_it_couldnt_say() {
        let said: Vec<Value> = [
            "{\"size\": 3}",
            "{\"error\": \"unreadable\"}",
            "null",
            "[1]",
            "7",
        ]
        .iter()
        .map(|line| facts(serde_json::from_str::<Value>(line).ok().as_ref()))
        .collect();
        assert_eq!(
            said,
            [
                json!({"size": 3}),
                Value::Null,
                Value::Null,
                Value::Null,
                Value::Null
            ],
            "its facts, and nothing else"
        );
        assert_eq!(facts(None), Value::Null, "nothing where none answered");
    }

    #[test]
    fn with_no_program_nothing_answers() {
        let record = Previewer::new(None).record("preview:/a.png").unwrap();
        assert_eq!(record["answering"], json!(false), "none answered: {record}");
        assert_eq!(record["shown"], Value::Null, "nothing was said: {record}");
    }

    #[test]
    fn a_record_held_is_current_only_while_the_file_is_unchanged() {
        let path = std::env::temp_dir().join(format!("preview-{}", std::process::id()));
        std::fs::write(&path, "x").unwrap();
        let path = path.display().to_string();
        let changed = changed_ms(&path).unwrap();
        let held = json!({"answering": true, "shown": {"changed": changed}});
        assert!(current(Some(&held), &path), "the same file was asked again");
        let older = json!({"answering": true, "shown": {"changed": changed - 1}});
        assert!(
            !current(Some(&older), &path),
            "a changed file wasn't asked again"
        );
        let unanswered = json!({"answering": false, "shown": null});
        assert!(
            !current(Some(&unanswered), &path),
            "an unanswered one wasn't asked again"
        );
        std::fs::remove_file(&path).unwrap();
    }
}
