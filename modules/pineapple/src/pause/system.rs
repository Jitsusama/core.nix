//! Running the machine's own programs for a stop, each to a limit, so a
//! machine that doesn't answer can't hold Joel's act up.

use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// Runs a program to its end: what it printed, or why it failed.
pub trait Runs {
    /// Runs `program` with `args`, answering its output when it exits 0.
    ///
    /// # Errors
    ///
    /// What it said, or why it couldn't run, when it didn't exit 0.
    fn run(&mut self, program: &Path, args: &[String]) -> Result<String, String>;
}

/// The machine's own programs, each given at most `limit` before it's
/// killed.
pub struct System {
    /// How long each program is given.
    pub limit: Duration,
}

/// How often a program running is looked at, to end it at its limit.
const LOOKED: Duration = Duration::from_millis(10);

impl Runs for System {
    fn run(&mut self, program: &Path, args: &[String]) -> Result<String, String> {
        let mut child = Command::new(program)
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| format!("{} can't run: {error}", program.to_string_lossy()))?;
        // Read as it runs, since a program printing more than a pipe holds
        // waits for its reader and would otherwise never end.
        let out = drain(child.stdout.take());
        let err = drain(child.stderr.take());
        let began = Instant::now();
        let status = loop {
            match child.try_wait() {
                Ok(Some(status)) => break status,
                Ok(None) if began.elapsed() >= self.limit => {
                    // Killed at its limit; it may have ended meanwhile,
                    // which is as good. Its readers are left to end with
                    // the pipes, which one of its own children may hold.
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(format!("no answer in {} s", self.limit.as_secs()));
                }
                Ok(None) => std::thread::sleep(LOOKED),
                Err(error) => return Err(error.to_string()),
            }
        };
        let out = String::from_utf8_lossy(&out.join().unwrap_or_default()).into_owned();
        if status.success() {
            return Ok(out);
        }
        let err = String::from_utf8_lossy(&err.join().unwrap_or_default()).into_owned();
        let said = if err.trim().is_empty() { out } else { err };
        Err(format!("{} ({status})", said.trim()))
    }
}

/// Reads all a pipe gives, on a thread of its own.
fn drain(pipe: Option<impl Read + Send + 'static>) -> JoinHandle<Vec<u8>> {
    std::thread::spawn(move || {
        let mut read = Vec::new();
        if let Some(mut pipe) = pipe {
            // What couldn't be read is said as nothing; the exit says
            // whether it ran.
            let _ = pipe.read_to_end(&mut read);
        }
        read
    })
}
