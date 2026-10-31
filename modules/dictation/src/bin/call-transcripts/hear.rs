//! Hears each take as it comes, on a thread of its own, and writes the
//! call's transcript again after each, so it's ready when the call ends.
//!
//! Whisper is loaded with a call's first take, at the NPU's background
//! priority, and let go once no call has needed it for a while, so it
//! holds no memory between calls. A take's sound is let go once it's
//! heard.

use std::collections::{HashMap, HashSet};
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::time::Duration;

use dictation::speech::Speech;
use dictation::transcript::{self, Line, Side, markdown};

use crate::Settings;
use crate::book::{Record, Shared, TRANSCRIPT, write_whole};

/// How long Whisper is kept once the last call is written, so a call
/// rejoined soon after needn't load it again.
const UNUSED: Duration = Duration::from_mins(2);

/// What a side of a call gives to be heard.
pub enum Job {
    /// A take, starting `start` samples into the call.
    Take {
        key: String,
        side: Side,
        start: usize,
        samples: Vec<f32>,
    },
    /// The side's recorder has stopped, and every take it cut was given.
    Done { key: String, side: Side },
}

/// A call being written.
#[derive(Default)]
struct Writing {
    lines: Vec<Line>,
    done: HashSet<Side>,
}

/// Hears every take given, until nothing more can be.
pub fn hear(settings: &Settings, book: &Shared, jobs: &Receiver<Job>, me: &str) {
    let mut speech: Option<Speech> = None;
    let mut calls: HashMap<String, Writing> = HashMap::new();
    loop {
        let job = if calls.is_empty() && speech.is_some() {
            match jobs.recv_timeout(UNUSED) {
                Ok(job) => job,
                Err(RecvTimeoutError::Timeout) => {
                    speech = None;
                    eprintln!("call-transcripts: Whisper let go");
                    continue;
                }
                Err(RecvTimeoutError::Disconnected) => return,
            }
        } else {
            match jobs.recv() {
                Ok(job) => job,
                Err(_) => return,
            }
        };
        match job {
            Job::Take {
                key,
                side,
                start,
                samples,
            } => {
                let writing = calls
                    .entry(key.clone())
                    .or_insert_with(|| resumed(book, &key));
                if speech.is_none() {
                    match settings.load() {
                        Ok(loaded) => speech = Some(loaded),
                        Err(why) => eprintln!("call-transcripts: Whisper didn't load: {why}"),
                    }
                }
                let Some(whisper) = &mut speech else { continue };
                let text = match whisper.transcribe(&samples) {
                    Ok((text, took)) => {
                        eprintln!("call-transcripts: {took}");
                        text.trim().to_owned()
                    }
                    Err(why) => {
                        eprintln!("call-transcripts: a take wasn't heard: {why}");
                        continue;
                    }
                };
                if text.is_empty() {
                    continue;
                }
                writing.lines.push(Line {
                    side,
                    start,
                    end: start + samples.len(),
                    text,
                });
                write(book, &key, &writing.lines, me, true);
            }
            Job::Done { key, side } => {
                let writing = calls
                    .entry(key.clone())
                    .or_insert_with(|| resumed(book, &key));
                writing.done.insert(side);
                if writing.done.len() == 2 {
                    write(book, &key, &writing.lines, me, false);
                    calls.remove(&key);
                }
            }
        }
    }
}

/// What a call already says, taken up again: what the transcript of a
/// call rejoined, or recorded before the daemon restarted, holds.
fn resumed(book: &Shared, key: &str) -> Writing {
    let lines = book
        .lock()
        .record(key)
        .and_then(|record| record.transcript.clone())
        .and_then(|file| std::fs::read_to_string(file).ok())
        .map(|text| transcript::read(&text))
        .unwrap_or_default();
    Writing {
        lines,
        done: HashSet::new(),
    }
}

/// Writes a call's transcript whole, and says in its record where it is
/// and whether it's still being written.
fn write(book: &Shared, key: &str, lines: &[Line], me: &str, transcribing: bool) {
    let mut book = book.lock();
    let Some(record) = book.record(key).cloned() else {
        return;
    };
    let file = record.folder.join(TRANSCRIPT);
    if !lines.is_empty()
        && let Err(why) = write_whole(&file, &markdown(lines, me))
    {
        eprintln!("call-transcripts: writing {}: {why}", file.display());
        return;
    }
    let transcript = file.exists().then_some(file);
    if record.transcript != transcript || record.transcribing != transcribing {
        book.put(Record {
            transcript,
            transcribing,
            ..record
        });
    }
}
