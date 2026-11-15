//! Asking each part's owner for its answer: its command run with a clock,
//! all at once, so the slowest owner bounds the wait rather than their sum.
//! An owner not on this machine, one that fails, and one that runs past
//! the clock each leave an answer saying why, never a hang.

use std::io::Read;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::Value;
use system::parts::Answer;

/// How long an owner may take. Each answers from what it holds in well
/// under a second; `repos` reads every checkout's git files, about two
/// seconds over a thousand trees, so this leaves it room on a busy machine.
const CLOCK: Duration = Duration::from_secs(10);
/// How often a running owner is looked at.
const LOOK: Duration = Duration::from_millis(20);

/// Each command's answer, in the order asked.
pub fn all(commands: &[&[&str]]) -> Vec<Answer> {
    let asked: Vec<_> = commands
        .iter()
        .map(|words| {
            let words: Vec<String> = words.iter().map(|word| (*word).to_owned()).collect();
            thread::spawn(move || one(&words))
        })
        .collect();
    asked
        .into_iter()
        .map(|asked| {
            asked
                .join()
                .unwrap_or_else(|_| Err("its reader stopped on a bug".to_owned()))
        })
        .collect()
}

fn one(words: &[String]) -> Answer {
    let said = words.join(" ");
    let mut child = Command::new(&words[0])
        .args(&words[1..])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| match error.kind() {
            std::io::ErrorKind::NotFound => format!("{} isn't on this machine", words[0]),
            _ => format!("{said} couldn't start: {error}"),
        })?;
    // Read as it writes, so a long answer never fills the pipe and stalls
    // the owner while it's waited on.
    let mut out = child.stdout.take().ok_or("its answer couldn't be read")?;
    let reading = thread::spawn(move || {
        let mut bytes = Vec::new();
        out.read_to_end(&mut bytes).map(|_| bytes)
    });
    let started = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().map_err(|error| error.to_string())? {
            break status;
        }
        if started.elapsed() > CLOCK {
            // Its answer would come too late to be of use; it's ended so
            // nothing of it outlives the command.
            let _ = child.kill();
            let _ = child.wait();
            return Err(format!("{said} gave no answer in {} s", CLOCK.as_secs()));
        }
        thread::sleep(LOOK);
    };
    let bytes = reading
        .join()
        .map_err(|_| "its answer's reader stopped on a bug".to_owned())?
        .map_err(|error| error.to_string())?;
    if !status.success() {
        return Err(match status.code() {
            Some(code) => format!("{said} ended with {code}"),
            None => format!("{said} was stopped"),
        });
    }
    serde_json::from_slice::<Value>(&bytes).map_err(|_| format!("{said} said something not JSON"))
}
