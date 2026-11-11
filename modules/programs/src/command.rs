//! A source's command, run without a shell on the thread that asks, and
//! stopped past [`WAIT`] so a hung one never holds its source forever.

use std::io::Read;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

/// The longest a source's command may run before it's stopped.
pub const WAIT: Duration = Duration::from_secs(10);

/// Runs a command without a shell, answering what it printed, or why it
/// couldn't: it isn't there, it failed, or it ran past [`WAIT`].
///
/// # Errors
///
/// Says why when the command names nothing, couldn't start, couldn't be
/// read, ran past [`WAIT`] or exited other than with success.
pub fn run(argv: &[String]) -> Result<String, String> {
    let (program, arguments) = argv.split_first().ok_or("no command")?;
    let mut child = Command::new(program)
        .args(arguments)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|why| format!("{program} couldn't start: {why}"))?;
    let mut out = child.stdout.take().ok_or("no output")?;
    let (said, heard) = mpsc::channel();
    std::thread::spawn(move || {
        let mut text = String::new();
        let read = out.read_to_string(&mut text).map(|_| text);
        let _ = said.send(read);
    });
    let printed = if let Ok(read) = heard.recv_timeout(WAIT) {
        read.map_err(|why| format!("{program}'s output couldn't be read: {why}"))
    } else {
        let _ = child.kill();
        let _ = child.wait();
        return Err(format!("{program} ran past {}s", WAIT.as_secs()));
    };
    let status = child.wait().map_err(|why| why.to_string())?;
    if !status.success() {
        return Err(format!("{program} exited with {status}"));
    }
    printed
}
