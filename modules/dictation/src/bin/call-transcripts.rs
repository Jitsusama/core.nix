//! call-transcripts: writes down what's said on a call, on this machine,
//! with Whisper on the NPU taking its turn behind dictation and anything
//! live, since nobody waits on a transcript.
//!
//! Each side is heard apart, Joel's microphone and the call's own sound,
//! so who said what needs no guessing. Each side is cut at its pauses into
//! takes (takes.rs), each take is heard with Joel's words, and the two
//! sides are laid out in the order they spoke.
//!
//! Usage:
//!
//! ```text
//! call-transcripts transcribe MODEL_DIR MINE.wav THEIRS.wav
//!     print, as Markdown, the transcript of a call's two sides, each a
//!     16 kHz mono WAV starting at the call's start
//! ```
//!
//! It reads the same three settings dictation does: `DICTATION_DEVICE`,
//! `DICTATION_CACHE`, which must be a folder of its own, since a load
//! removes what another build compiled there, and `DICTATION_WORDS`.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use dictation::mel::RATE;
use dictation::openvino::Priority;
use dictation::speech::Speech;
use dictation::takes::{Cutter, Take};
use dictation::wav;

const USAGE: &str = "usage: call-transcripts transcribe MODEL_DIR MINE.wav THEIRS.wav";

/// Who spoke a line: Joel, whose microphone it came from, or the others on
/// the call, whose sound it came from.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Side {
    Mine,
    Theirs,
}

/// A take heard: who said it, when in the call, and what.
#[derive(Debug, PartialEq)]
struct Line {
    side: Side,
    start: usize,
    text: String,
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.iter().map(String::as_str).collect::<Vec<_>>()[..] {
        ["transcribe", model, mine, theirs] => {
            transcribe(Path::new(model), Path::new(mine), Path::new(theirs))
        }
        _ => Err(format!("{USAGE} (got {args:?})")),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("call-transcripts: {error}");
            ExitCode::FAILURE
        }
    }
}

fn transcribe(model: &Path, mine: &Path, theirs: &Path) -> Result<(), String> {
    let words = match std::env::var_os("DICTATION_WORDS") {
        Some(file) => std::fs::read_to_string(&file)
            .map_err(|e| format!("reading {}: {e}", Path::new(&file).display()))?,
        None => String::new(),
    };
    let device = std::env::var("DICTATION_DEVICE").unwrap_or_else(|_| "NPU".to_owned());
    let cache = std::env::var_os("DICTATION_CACHE").map(PathBuf::from);
    let mut speech = Speech::load(
        model,
        &device,
        cache.as_deref(),
        &words,
        Priority::Background,
    )?;
    let mut lines = Vec::new();
    for (side, file) in [(Side::Mine, mine), (Side::Theirs, theirs)] {
        let bytes = std::fs::read(file).map_err(|e| format!("reading {}: {e}", file.display()))?;
        for take in takes(&wav::samples(&bytes)?) {
            let (text, took) = speech.transcribe(&take.samples)?;
            eprintln!("call-transcripts: {took}");
            let text = text.trim();
            if !text.is_empty() {
                lines.push(Line {
                    side,
                    start: take.start,
                    text: text.to_owned(),
                });
            }
        }
    }
    print!("{}", markdown(lines, &own_name()));
    Ok(())
}

/// A side's sound cut into takes.
fn takes(sound: &[f32]) -> Vec<Take> {
    let mut cutter = Cutter::new();
    let mut takes = cutter.feed(sound);
    takes.extend(cutter.finish());
    takes
}

/// The account's own name, its first word as the system knows it, for
/// Joel's lines; or "Me" where it has none.
fn own_name() -> String {
    let user = std::env::var("USER").unwrap_or_default();
    std::fs::read_to_string("/etc/passwd")
        .ok()
        .and_then(|passwd| {
            passwd.lines().find_map(|entry| {
                let fields: Vec<&str> = entry.split(':').collect();
                (fields.first() == Some(&user.as_str()))
                    .then(|| fields.get(4)?.split([' ', ',']).next().map(str::to_owned))
                    .flatten()
            })
        })
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "Me".to_owned())
}

/// The transcript: lines in the order they were said, a speaker's lines in
/// a row joined into one paragraph under the time it began.
fn markdown(mut lines: Vec<Line>, me: &str) -> String {
    lines.sort_by_key(|line| line.start);
    let mut paragraphs = vec!["# Transcript".to_owned()];
    let mut last = None;
    for line in lines {
        match paragraphs.last_mut() {
            Some(paragraph) if last == Some(line.side) => {
                paragraph.push(' ');
                paragraph.push_str(&line.text);
            }
            _ => {
                let who = match line.side {
                    Side::Mine => me,
                    Side::Theirs => "Others",
                };
                let mut paragraph = String::new();
                let _ = write!(paragraph, "**{who}**, {}: {}", clock(line.start), line.text);
                paragraphs.push(paragraph);
            }
        }
        last = Some(line.side);
    }
    paragraphs.join("\n\n") + "\n"
}

/// Where in the call a sample falls, as minutes and seconds, with hours
/// once there are any.
fn clock(sample: usize) -> String {
    let seconds = sample / RATE;
    let (hours, minutes, seconds) = (seconds / 3600, seconds / 60 % 60, seconds % 60);
    if hours > 0 {
        format!("{hours}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes}:{seconds:02}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(side: Side, second: usize, text: &str) -> Line {
        Line {
            side,
            start: second * RATE,
            text: text.to_owned(),
        }
    }

    #[test]
    fn the_sides_are_laid_out_in_the_order_they_spoke_and_a_run_is_one_paragraph() {
        let lines = vec![
            line(Side::Theirs, 4, "Thursday works."),
            line(Side::Mine, 0, "Can we move it?"),
            line(Side::Mine, 9, "Great."),
            line(Side::Mine, 11, "Thanks."),
        ];
        let want = [
            "# Transcript",
            "",
            "**Joel**, 0:00: Can we move it?",
            "",
            "**Others**, 0:04: Thursday works.",
            "",
            "**Joel**, 0:09: Great. Thanks.",
            "",
        ];
        assert_eq!(markdown(lines, "Joel"), want.join("\n"));
    }

    #[test]
    fn the_clock_gains_hours_only_once_there_are_some() {
        assert_eq!(clock(65 * RATE), "1:05");
        assert_eq!(clock(3_725 * RATE), "1:02:05");
    }
}
