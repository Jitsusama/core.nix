//! `DevTools` over a pipe rather than a port. A Chrome started with
//! `--remote-debugging-pipe` reads commands from its fd 3 and writes
//! answers to its fd 4, each a JSON message ended by a NUL. No port is
//! opened, so no other program on the machine can drive the browser, as
//! one listening on 127.0.0.1 could be driven by any program of Joel's.

use serde_json::{Value, json};
use std::fs::File;
use std::io::{self, Read, Write};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd};
use std::os::unix::process::CommandExt;
use std::process::{Child, Command};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::thread;
use std::time::{Duration, Instant};

unsafe extern "C" {
    fn pipe2(fds: *mut RawFd, flags: i32) -> i32;
    fn dup2(old: RawFd, new: RawFd) -> i32;
    fn fcntl(fd: RawFd, command: i32, argument: i32) -> i32;
}

/// `O_CLOEXEC`, so a pipe's ends reach only the Chrome they're meant for.
const CLOEXEC: i32 = 0o2_000_000;

/// `F_DUPFD_CLOEXEC`: a copy at the lowest free descriptor from the one asked.
const DUPFD_CLOEXEC: i32 = 1030;

/// The lowest descriptor a pipe's end is kept at. Above 4, the ends never
/// are Chrome's fds 3 and 4 already, where dup2 onto itself would leave
/// close-on-exec set, nor is one end overwritten by the other's copy.
const ABOVE_CHROMES: i32 = 10;

fn pipe() -> io::Result<(OwnedFd, OwnedFd)> {
    let mut fds = [0; 2];
    // SAFETY: `fds` has room for the two descriptors pipe2 writes.
    if unsafe { pipe2(fds.as_mut_ptr(), CLOEXEC) } != 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: pipe2 just opened both, and nothing else owns them.
    let (low_in, low_out) = unsafe { (OwnedFd::from_raw_fd(fds[0]), OwnedFd::from_raw_fd(fds[1])) };
    Ok((above_chromes(&low_in)?, above_chromes(&low_out)?))
}

/// A copy of `fd` at `ABOVE_CHROMES` or higher; the low one closes as it drops.
fn above_chromes(fd: &OwnedFd) -> io::Result<OwnedFd> {
    // SAFETY: fcntl only reads `fd`, which is open, and returns a new one.
    let copy = unsafe { fcntl(fd.as_raw_fd(), DUPFD_CLOEXEC, ABOVE_CHROMES) };
    if copy < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: fcntl just opened `copy`, and nothing else owns it.
    Ok(unsafe { OwnedFd::from_raw_fd(copy) })
}

/// A Chrome driven over its `DevTools` pipe, attached to its first page.
/// The Chrome ends as this drops.
pub struct Browser {
    child: Child,
    commands: File,
    answers: Receiver<Value>,
    next: u64,
    session: Option<String>,
}

impl Browser {
    /// Starts `command`, which names Chrome and its flags, with the pipe.
    pub fn start(mut command: Command) -> io::Result<Self> {
        let (theirs_in, ours_out) = pipe()?;
        let (ours_in, theirs_out) = pipe()?;
        let (read, write) = (theirs_in.as_raw_fd(), theirs_out.as_raw_fd());
        command.arg("--remote-debugging-pipe");
        // SAFETY: dup2 is async-signal-safe, and the closure allocates
        // nothing. dup2 clears close-on-exec on the copies it makes, which
        // differ from their sources since those are above 4, so Chrome
        // keeps fds 3 and 4 and nothing else of the pipes.
        unsafe {
            command.pre_exec(move || {
                if dup2(read, 3) < 0 || dup2(write, 4) < 0 {
                    return Err(io::Error::last_os_error());
                }
                Ok(())
            });
        }
        let child = command.spawn()?;
        drop((theirs_in, theirs_out));
        let (send, answers) = mpsc::channel();
        let mut reading = File::from(ours_in);
        // One thread reads the answers, so a call can wait with a timeout.
        thread::spawn(move || {
            let mut buffer = Vec::new();
            let mut chunk = vec![0u8; 65_536];
            while let Ok(n) = reading.read(&mut chunk) {
                if n == 0 {
                    break;
                }
                buffer.extend_from_slice(&chunk[..n]);
                while let Some(end) = buffer.iter().position(|b| *b == 0) {
                    let message: Vec<u8> = buffer.drain(..=end).collect();
                    let parsed = serde_json::from_slice(&message[..end]);
                    if parsed.map(|m| send.send(m)).is_ok_and(|sent| sent.is_err()) {
                        return;
                    }
                }
            }
        });
        Ok(Browser {
            child,
            commands: File::from(ours_out),
            answers,
            next: 0,
            session: None,
        })
    }
}

impl Drop for Browser {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Browser {
    /// Sends `method` with `params` and waits up to `wait` for its answer.
    pub fn call(&mut self, method: &str, params: Value, wait: Duration) -> Result<Value, String> {
        self.next += 1;
        let mut message = json!({"id": self.next, "method": method});
        message["params"] = params;
        if let Some(session) = &self.session {
            message["sessionId"] = json!(session);
        }
        let mut bytes = serde_json::to_vec(&message).map_err(|e| e.to_string())?;
        bytes.push(0);
        self.commands
            .write_all(&bytes)
            .map_err(|e| format!("Chrome's DevTools pipe: {e}"))?;
        let until = Instant::now() + wait;
        loop {
            let left = until.saturating_duration_since(Instant::now());
            match self.answers.recv_timeout(left) {
                Ok(answer) if answer["id"] == self.next => {
                    return match answer.get("error") {
                        Some(error) => Err(format!("{method}: {error}")),
                        None => Ok(answer["result"].clone()),
                    };
                }
                Ok(_) => {}
                Err(RecvTimeoutError::Timeout) => return Err(format!("{method} had no answer")),
                Err(RecvTimeoutError::Disconnected) => {
                    return Err("Chrome closed its DevTools pipe".into());
                }
            }
        }
    }

    /// Attaches to the first page, so later calls go to it.
    pub fn attach(&mut self, wait: Duration) -> Result<(), String> {
        let until = Instant::now() + wait;
        // The first page comes a moment after the browser answers.
        let page = loop {
            let targets = self.call("Target.getTargets", json!({}), wait)?;
            let page = targets["targetInfos"]
                .as_array()
                .and_then(|all| all.iter().find(|t| t["type"] == "page"))
                .map(|t| t["targetId"].clone());
            match page {
                Some(page) => break page,
                None if Instant::now() < until => thread::sleep(Duration::from_millis(200)),
                None => return Err("Chrome opened no page".into()),
            }
        };
        let attached = self.call(
            "Target.attachToTarget",
            json!({"targetId": page, "flatten": true}),
            wait,
        )?;
        self.session = attached["sessionId"].as_str().map(String::from);
        Ok(())
    }

    /// Runs `expression` in the page, waiting for a promise it returns, and
    /// answers its value.
    pub fn eval(&mut self, expression: &str, wait: Duration) -> Result<Value, String> {
        let params = json!({"expression": expression, "awaitPromise": true, "returnByValue": true});
        let answer = self.call("Runtime.evaluate", params, wait)?;
        if let Some(thrown) = answer.get("exceptionDetails") {
            let said = thrown["exception"]["description"].as_str().unwrap_or("");
            return Err(format!("the page threw: {said}"));
        }
        Ok(answer["result"]["value"].clone())
    }

    /// A click at `x`, `y` in the page, pressed and let go.
    pub fn click(&mut self, x: f64, y: f64, wait: Duration) -> Result<(), String> {
        for kind in ["mousePressed", "mouseReleased"] {
            let params = json!({"type": kind, "x": x, "y": y, "button": "left", "clickCount": 1});
            self.call("Input.dispatchMouseEvent", params, wait)?;
        }
        Ok(())
    }
}
