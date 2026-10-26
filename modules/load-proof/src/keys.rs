//! Keys: whether a key typed in a terminal comes back as quickly as on
//! an idle machine.
//!
//! The probe plays both ends of a terminal. As kitty does, it holds a
//! pty's master side and writes a key to it; a second process, standing
//! in for the shell, holds the other side in raw mode and writes each key
//! back, as zsh's line editor echoes what Joel types. The time from the
//! write to the key coming back covers what load can slow between a
//! terminal and its shell: the kernel's pty worker passing the byte on,
//! the shell waking for it and writing it back, and the terminal waking
//! for the echo. Drawing the echo is left to the frames signal.
//!
//! Keys go at a quick typist's pace, one every 30 ms, so a slow echo
//! shows as one slow key rather than holding up the rest.

use std::ffi::OsStr;
use std::fs::File;
use std::io::{Read, Write};
use std::os::fd::{AsFd, OwnedFd};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use rustix::pty::{OpenptFlags, grantpt, openpt, ptsname, unlockpt};
use rustix::termios::{OptionalActions, tcgetattr, tcsetattr};

/// How often a key is typed.
pub const EVERY: Duration = Duration::from_millis(30);

/// The subcommand the shell's stand-in runs as, given its side's path.
pub const ECHO: &str = "keys-echo";

/// Types a key every `EVERY` for `length` into a pty whose other side a
/// stand-in shell, this program run as `ECHO`, writes back, and gives the
/// time each took to return.
///
/// # Errors
///
/// When the pty can't be opened, the stand-in can't start, or the stand-in
/// stops answering.
pub fn watch(length: Duration) -> Result<Vec<Duration>, String> {
    let (master, path) = open().map_err(|e| format!("opening a pty: {e}"))?;
    let me = std::env::current_exe().map_err(|e| format!("finding myself: {e}"))?;
    let shell = Shell(
        Command::new(me)
            .arg(ECHO)
            .arg(&path)
            .stdin(Stdio::null())
            .spawn()
            .map_err(|e| format!("starting the shell's stand-in: {e}"))?,
    );
    let mut terminal = File::from(master);
    // The first key waits for the stand-in to start and is not counted.
    round_trip(&mut terminal).map_err(|e| format!("waiting for the stand-in: {e}"))?;
    let echoes = typed(&mut terminal, length).map_err(|e| format!("typing: {e}"))?;
    // Hanging up ends the stand-in; the guard reaps it.
    drop(terminal);
    drop(shell);
    Ok(echoes)
}

/// The stand-in shell, stopped if the probe ends without hanging up.
struct Shell(Child);

impl Drop for Shell {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// A new pty's master side, and the path of its other side.
fn open() -> std::io::Result<(OwnedFd, std::ffi::OsString)> {
    let master = openpt(OpenptFlags::RDWR | OpenptFlags::NOCTTY)?;
    grantpt(&master)?;
    unlockpt(&master)?;
    let name = ptsname(&master, Vec::new())?;
    let path = std::os::unix::ffi::OsStringExt::from_vec(name.into_bytes());
    Ok((master, path))
}

/// Types a key every `EVERY` for `length`, giving how long each took to
/// come back. A key typed while one is still out waits for it, as a
/// terminal's next key would queue behind it.
fn typed(terminal: &mut File, length: Duration) -> std::io::Result<Vec<Duration>> {
    let start = Instant::now();
    let mut next = start;
    let mut echoes = Vec::new();
    while start.elapsed() < length {
        echoes.push(round_trip(terminal)?);
        next += EVERY;
        if let Some(wait) = next.checked_duration_since(Instant::now()) {
            std::thread::sleep(wait);
        }
    }
    Ok(echoes)
}

/// Writes one key to the terminal's side and waits for it to come back.
fn round_trip(terminal: &mut File) -> std::io::Result<Duration> {
    let sent = Instant::now();
    terminal.write_all(b"k")?;
    let mut back = [0u8; 1];
    terminal.read_exact(&mut back)?;
    Ok(sent.elapsed())
}

/// The shell's stand-in: opens the pty's other side at `path`, puts it in
/// raw mode as a line editor does, and writes back every byte until the
/// terminal's side hangs up.
///
/// # Errors
///
/// When the side can't be opened or put in raw mode. A hang-up ends it
/// without one.
pub fn echo(path: &OsStr) -> Result<(), String> {
    let side = File::options()
        .read(true)
        .write(true)
        .open(path)
        .map_err(|e| format!("opening {}: {e}", path.to_string_lossy()))?;
    raw(&side).map_err(|e| format!("raw mode: {e}"))?;
    echo_on(side);
    Ok(())
}

/// Puts a pty's side in raw mode: no echo of its own, no line editing.
fn raw(side: &impl AsFd) -> std::io::Result<()> {
    let mut termios = tcgetattr(side)?;
    termios.make_raw();
    tcsetattr(side, OptionalActions::Now, &termios)?;
    Ok(())
}

/// Writes back every byte read from `side` until it hangs up.
fn echo_on(mut side: File) {
    let mut byte = [0u8; 1];
    while side.read_exact(&mut byte).is_ok() {
        if side.write_all(&byte).is_err() {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_key_typed_comes_back_through_a_pty_in_raw_mode() {
        let (master, path) = open().expect("a pty");
        let side = File::options()
            .read(true)
            .write(true)
            .open(&path)
            .expect("its other side");
        raw(&side).expect("raw mode");
        let shell = std::thread::spawn(move || echo_on(side));
        let mut terminal = File::from(master);
        let echoes = typed(&mut terminal, EVERY * 5).expect("typing");
        assert!(echoes.len() >= 5, "{echoes:?}");
        assert!(
            echoes.iter().all(|echo| *echo < Duration::from_secs(1)),
            "{echoes:?}"
        );
        drop(terminal);
        shell.join().expect("the stand-in ends at the hang-up");
    }
}
