//! Asking warp-cli, WARP's own client, which asks WARP's service over its
//! socket. Each question is one run of `warp-cli --json`, given a few
//! seconds, since a service that's stuck would otherwise hold the answer
//! up for good; what it said, and whether it said it as an error, comes
//! back as it was, for `read` to make sense of. Listening is one run held
//! open, `warp-cli --json --listen status`, which writes a line each time
//! WARP's status changes, for as long as it runs.

use std::io::{BufRead, BufReader, ErrorKind, Read};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

/// What warp-cli said to one question.
#[derive(Debug, PartialEq)]
pub struct Said {
    pub ok: bool,
    pub text: String,
}

/// Why warp-cli couldn't be asked at all.
#[derive(Debug, PartialEq)]
pub enum Unasked {
    /// There's no warp-cli where it was looked for.
    Missing(String),
    /// It started but didn't finish in time, or couldn't be run.
    Stuck(String),
}

/// WARP's status as warp-cli tells each change of it, a line of JSON
/// each, for as long as it's held; `stop` ends it, and its lines with it.
pub struct Listening {
    pub lines: Box<dyn BufRead + Send>,
    pub stop: Box<dyn FnOnce() + Send>,
}

pub trait Ask: Sync {
    fn ask(&self, question: &[&str]) -> Result<Said, Unasked>;

    /// WARP's status as it changes, or nothing where it can't be
    /// listened to.
    fn listen(&self) -> Option<Listening> {
        None
    }
}

/// The real warp-cli: `WARP_CLI` when it's set, as the shell sets it to
/// the one the system runs, else the one on the PATH.
pub struct WarpCli {
    pub program: PathBuf,
    pub patience: Duration,
}

impl WarpCli {
    pub fn from_env() -> Self {
        Self {
            program: std::env::var_os("WARP_CLI").map_or_else(|| "warp-cli".into(), PathBuf::from),
            patience: Duration::from_secs(5),
        }
    }
}

impl Ask for WarpCli {
    fn ask(&self, question: &[&str]) -> Result<Said, Unasked> {
        let shown = self.program.display().to_string();
        let mut child = Command::new(&self.program)
            .arg("--json")
            .args(question)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| match e.kind() {
                ErrorKind::NotFound => Unasked::Missing(shown.clone()),
                _ => Unasked::Stuck(format!("{shown} couldn't be run: {e}")),
            })?;
        // Read as it writes, so a long answer never fills the pipe and
        // stalls it until it's given up on.
        let mut out = child.stdout.take().expect("stdout is piped");
        let reader = thread::spawn(move || {
            let mut text = String::new();
            out.read_to_string(&mut text).map(|_| text)
        });
        let deadline = Instant::now() + self.patience;
        let status = loop {
            match child.try_wait() {
                Ok(Some(status)) => break status,
                Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(5)),
                Ok(None) => {
                    // Already ended or not, it's given up on either way.
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(Unasked::Stuck(format!(
                        "warp-cli {} didn't answer within {} s",
                        question.join(" "),
                        self.patience.as_secs()
                    )));
                }
                Err(e) => {
                    return Err(Unasked::Stuck(format!(
                        "{shown} couldn't be waited on: {e}"
                    )));
                }
            }
        };
        let text = reader
            .join()
            .map_err(|_| Unasked::Stuck(format!("{shown}'s answer couldn't be read")))?
            .map_err(|e| Unasked::Stuck(format!("{shown}'s answer couldn't be read: {e}")))?;
        Ok(Said {
            ok: status.success(),
            text,
        })
    }

    fn listen(&self) -> Option<Listening> {
        let mut child = Command::new(&self.program)
            .args(["--json", "--listen", "status"])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .ok()?;
        let out = child.stdout.take().expect("stdout is piped");
        Some(Listening {
            lines: Box::new(BufReader::new(out)),
            stop: Box::new(move || {
                // Already ended or not, it's let go of either way.
                let _ = child.kill();
                let _ = child.wait();
            }),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_warp_cli_that_isnt_there_is_missing_not_a_failure_to_run() {
        let cli = WarpCli {
            program: "/nonexistent/warp-cli".into(),
            patience: Duration::from_secs(1),
        };
        assert_eq!(
            cli.ask(&["status"]),
            Err(Unasked::Missing("/nonexistent/warp-cli".into()))
        );
    }
}
