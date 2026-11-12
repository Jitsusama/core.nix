//! Hears each take as it comes, on a thread of its own, and writes the
//! call's transcript again after each, so it's ready when the call ends.
//!
//! Parakeet is loaded with a call's first take, at the NPU's background
//! priority, and let go once no call has needed it for a while, so it
//! holds no memory between calls. A take's sound is let go once it's
//! heard.
//!
//! A new OpenVINO or driver means compiling Parakeet again, about a
//! minute on the NPU, which a call's first take used to wait for: on 12
//! November a call's transcript started 69 s late. So when the service
//! starts with nothing compiled for this build, which a switch or a boot
//! does and a call never does, it's compiled then and let go.
//!
//! A call written down whole is given to be summed up (summarizer.rs)
//! once no call is being heard and Parakeet is let go, so the two models
//! are never held at once. What can't be heard is said in a notice
//! (notices.rs).

use std::collections::{HashMap, HashSet};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::time::Duration;

use dictation::parakeet::Parakeet;
use dictation::transcript::{self, Line, Side, markdown};

use crate::Settings;
use crate::book::{self, Record, Shared, TRANSCRIPT, write_whole};
use crate::{notices, said};

/// How long Parakeet is kept once the last call is written, so a call
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
    /// Sum the call up again, Joel having asked once it couldn't be.
    SumUp { key: String },
}

/// A call being written.
#[derive(Default)]
struct Writing {
    lines: Vec<Line>,
    done: HashSet<Side>,
}

/// Hears every take given, until nothing more can be; each call written
/// down whole, or given back by a notice's Try Again, goes to `sums`.
pub fn hear(
    settings: &Settings,
    book: &Shared,
    jobs: &Receiver<Job>,
    sums: &Sender<String>,
    me: &str,
) {
    let mut speech: Option<Parakeet> = None;
    let mut calls: HashMap<String, Writing> = HashMap::new();
    let mut unsummed: Vec<String> = Vec::new();
    // The calls already told they couldn't be heard, so it's said once.
    let mut told: HashSet<String> = HashSet::new();
    compile_ahead(settings);
    loop {
        if calls.is_empty() && !unsummed.is_empty() {
            if speech.take().is_some() {
                eprintln!("call-transcripts: Parakeet let go");
            }
            for key in unsummed.drain(..) {
                let _ = sums.send(key);
            }
        }
        let job = if calls.is_empty() && speech.is_some() {
            match jobs.recv_timeout(UNUSED) {
                Ok(job) => job,
                Err(RecvTimeoutError::Timeout) => {
                    speech = None;
                    eprintln!("call-transcripts: Parakeet let go");
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
                        Err(why) => {
                            eprintln!("call-transcripts: Parakeet didn't load: {why}");
                            if told.insert(key.clone()) {
                                let title = title_of(book, &key);
                                let why = format!("The speech model didn't load: {why}");
                                notices::show(&notices::not_transcribed(&title, &why), |_| {});
                            }
                        }
                    }
                }
                let Some(parakeet) = &mut speech else {
                    continue;
                };
                let text = match parakeet.transcribe(&samples) {
                    Ok((text, took)) => {
                        eprintln!("call-transcripts: {}", took.describe(&parakeet.device));
                        text.trim().to_owned()
                    }
                    Err(why) => {
                        eprintln!("call-transcripts: a take wasn't heard: {why}");
                        continue;
                    }
                };
                let Some(text) = said::spoken(&text) else {
                    continue;
                };
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
                    told.remove(&key);
                    unsummed.push(key);
                }
            }
            Job::SumUp { key } => {
                if !unsummed.contains(&key) {
                    unsummed.push(key);
                }
            }
        }
    }
}

/// Compiles Parakeet and lets it go, when nothing is compiled for this
/// build, so no call's first take waits for it.
fn compile_ahead(settings: &Settings) {
    if settings.compiled() {
        return;
    }
    match settings.load() {
        Ok(_) => eprintln!("call-transcripts: Parakeet compiled ahead of a call, and let go"),
        Err(why) => eprintln!("call-transcripts: Parakeet didn't compile ahead: {why}"),
    }
}

/// A call's title, as its record has it.
fn title_of(book: &Shared, key: &str) -> String {
    book.lock()
        .record(key)
        .map(|record| record.title.clone())
        .unwrap_or_default()
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

/// Writes a call's transcript whole, and says in its record where it is,
/// how it opens and whether it's still being written.
fn write(book: &Shared, key: &str, lines: &[Line], me: &str, transcribing: bool) {
    let mut book = book.lock();
    let Some(record) = book.record(key).cloned() else {
        return;
    };
    let file = record.folder.join(TRANSCRIPT);
    let mut opening = record.opening.clone();
    if !lines.is_empty() {
        let written = markdown(lines, me);
        if let Err(why) = write_whole(&file, &written) {
            eprintln!("call-transcripts: writing {}: {why}", file.display());
            return;
        }
        opening = book::opening(&written);
    }
    let transcript = file.exists().then_some(file);
    if record.transcript != transcript
        || record.transcribing != transcribing
        || record.opening != opening
    {
        book.put(Record {
            transcript,
            opening,
            transcribing,
            ..record
        });
    }
}
