//! last-minute keeps the last minute of the screen and its sound in memory,
//! for the shell's recorder (Recorder.qml), and saves it when asked, so
//! something worth keeping can be kept after it's happened.
//!
//! wf-recorder records the screen through wlr-screencopy as the shell's own
//! recordings do, and its sound from the default sink, never the
//! microphone; but here it writes an MPEG transport stream into this
//! program's pipe rather than a file. What's kept is held as the stream's
//! own packets, from the keyframe at or before the span ago (kept.rs). It's
//! never written anywhere until it's saved, and it's gone with this program.
//!
//! Standard input takes a file to save it to, one a line. Each is answered
//! on standard output, `saved FILE` with the file it went to, numbered where
//! that one was taken, or `failed FILE: why`. ffmpeg copies the stream into
//! an MP4 under a hidden name, linked to its own once it's whole, so what
//! watches the folder never sees half a file.
//!
//! It ends once its input does, and wf-recorder with it, however this ends.
//! It exits 1 when wf-recorder couldn't start or stopped, saying why on
//! standard error, and 2 when it was asked wrongly.
//!
//! Usage: last-minute SECONDS -- WF-RECORDER-OPTIONS...

mod kept;

use std::io::{self, BufRead, ErrorKind, Read, Write};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitCode, Stdio};
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant};
use std::{env, fs, process, thread};

use kept::{Kept, PACKET, SYNC};

fn main() -> ExitCode {
    let mut arguments = env::args().skip(1);
    let span = arguments
        .next()
        .and_then(|seconds| seconds.parse::<u64>().ok());
    let (Some(span), Some("--")) = (span.filter(|&span| span > 0), arguments.next().as_deref())
    else {
        eprintln!("usage: last-minute SECONDS -- WF-RECORDER-OPTIONS...");
        return ExitCode::from(2);
    };
    let given: Vec<String> = arguments.collect();
    let mut recorder = match record(&given) {
        Ok(recorder) => recorder,
        Err(why) => {
            eprintln!("last-minute: wf-recorder could not start: {why}");
            return ExitCode::FAILURE;
        }
    };
    let Some(stream) = recorder.stdout.take() else {
        eprintln!("last-minute: wf-recorder gave no stream");
        return ExitCode::FAILURE;
    };
    // Which process records, since the compositor lists its screencopy
    // among what records the screen, and this one records for nobody
    // until it's saved.
    {
        let mut out = io::stdout().lock();
        let _ = writeln!(out, "keeping {}", recorder.id());
        let _ = out.flush();
    }
    let kept: &'static Mutex<Kept> =
        Box::leak(Box::new(Mutex::new(Kept::new(Duration::from_secs(span)))));
    thread::spawn(move || {
        keep(stream, kept);
        eprintln!("last-minute: wf-recorder stopped");
        process::exit(1);
    });
    for asked in io::stdin().lock().lines() {
        let Ok(asked) = asked else {
            break;
        };
        if !asked.is_empty() {
            answer(&asked, kept);
        }
    }
    // Asked for nothing more, so nothing is kept for anyone.
    let _ = recorder.kill();
    let _ = recorder.wait();
    ExitCode::SUCCESS
}

/// wf-recorder writing the stream to its standard output, given what the
/// recorder chose: the screen, the encoder and the sound. A keyframe at
/// least every 120 frames keeps a chunk to a few seconds.
fn record(given: &[String]) -> io::Result<Child> {
    let mut command = Command::new("wf-recorder");
    command
        .args([
            "-y",
            "--muxer",
            "mpegts",
            "--file",
            "pipe:1",
            "--codec-param",
            "g=120",
        ])
        .args(given)
        .stdin(Stdio::null())
        .stdout(Stdio::piped());
    // SAFETY: prctl is async-signal-safe, and nothing else runs between
    // the fork and the exec. It ends wf-recorder with this program however
    // this one ends, so nothing records the screen for nobody.
    unsafe {
        command.pre_exec(|| {
            if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM) == -1 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        });
    }
    command.spawn()
}

/// Reads the stream until it ends, keeping each whole packet, and finding
/// the packets' edges again should a byte be lost.
fn keep(mut stream: impl Read, kept: &Mutex<Kept>) {
    let mut buffer = vec![0; PACKET * 512];
    let mut held = 0;
    loop {
        match stream.read(&mut buffer[held..]) {
            Ok(0) => return,
            Ok(read) => held += read,
            Err(why) if why.kind() == ErrorKind::Interrupted => continue,
            Err(_) => return,
        }
        let now = Instant::now();
        let mut at = 0;
        let mut kept = kept.lock().unwrap_or_else(PoisonError::into_inner);
        while held - at >= PACKET {
            if buffer[at] == SYNC {
                kept.take(&buffer[at..at + PACKET], now);
                at += PACKET;
            } else {
                at += 1;
            }
        }
        drop(kept);
        buffer.copy_within(at..held, 0);
        held -= at;
    }
}

/// Saves what's kept to the file asked for, and says how it went.
fn answer(asked: &str, kept: &Mutex<Kept>) {
    let stream = kept.lock().unwrap_or_else(PoisonError::into_inner).stream();
    let saved = stream
        .ok_or_else(|| "nothing is kept yet".to_owned())
        .and_then(|stream| save(&stream, Path::new(asked)));
    let mut out = io::stdout().lock();
    let _ = match saved {
        Ok(file) => writeln!(out, "saved {}", file.display()),
        Err(why) => writeln!(out, "failed {asked}: {why}"),
    };
    let _ = out.flush();
}

fn save(stream: &[u8], file: &Path) -> Result<PathBuf, String> {
    let folder = file
        .parent()
        .filter(|folder| !folder.as_os_str().is_empty())
        .ok_or("it names no folder")?;
    fs::create_dir_all(folder).map_err(|why| format!("{}: {why}", folder.display()))?;
    let part = folder.join(format!(".last-minute-{}.mp4", process::id()));
    let saved = remux(stream, &part).and_then(|()| place(&part, file));
    let _ = fs::remove_file(&part);
    saved
}

/// Copies the stream into an MP4, its index at the front so it plays at once.
fn remux(stream: &[u8], part: &Path) -> Result<(), String> {
    let mut ffmpeg = Command::new("ffmpeg")
        .args(["-loglevel", "error", "-y", "-f", "mpegts", "-i", "pipe:0"])
        .args(["-c", "copy", "-movflags", "+faststart", "-f", "mp4"])
        .arg(part)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|why| format!("ffmpeg could not start: {why}"))?;
    let mut input = ffmpeg.stdin.take().ok_or("ffmpeg took no input")?;
    // Written while ffmpeg's errors are read, so neither waits on the other.
    let output = thread::scope(|scope| {
        scope.spawn(move || input.write_all(stream));
        ffmpeg.wait_with_output()
    })
    .map_err(|why| format!("ffmpeg: {why}"))?;
    if output.status.success() {
        Ok(())
    } else {
        let said = String::from_utf8_lossy(&output.stderr);
        Err(format!("ffmpeg could not write it: {}", said.trim()))
    }
}

/// Links the whole file to the name asked for, or, where that's taken, to
/// the same name numbered, as two screenshots in one second are.
fn place(part: &Path, file: &Path) -> Result<PathBuf, String> {
    let folder = file.parent().unwrap_or(Path::new("."));
    let stem = file.file_stem().unwrap_or_default().to_string_lossy();
    for n in 1..1000 {
        let named = if n == 1 {
            file.to_path_buf()
        } else {
            folder.join(format!("{stem} {n}.mp4"))
        };
        match fs::hard_link(part, &named) {
            Ok(()) => return Ok(named),
            Err(why) if why.kind() == ErrorKind::AlreadyExists => {}
            Err(why) => return Err(format!("{}: {why}", named.display())),
        }
    }
    Err(format!("{} and its numbers are taken", file.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_taken_name_is_numbered_rather_than_written_over() {
        let folder = env::temp_dir().join(format!("last-minute-test-{}", process::id()));
        fs::create_dir_all(&folder).unwrap();
        let part = folder.join(".part.mp4");
        fs::write(&part, "minute").unwrap();
        let file = folder.join("2026-11-02 15-30-00.mp4");
        fs::write(&file, "earlier").unwrap();
        let placed = place(&part, &file).unwrap();
        assert_eq!(placed, folder.join("2026-11-02 15-30-00 2.mp4"));
        assert_eq!(fs::read_to_string(&file).unwrap(), "earlier");
        assert_eq!(fs::read_to_string(&placed).unwrap(), "minute");
        fs::remove_dir_all(&folder).unwrap();
    }
}
