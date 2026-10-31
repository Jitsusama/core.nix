//! Hears each take as it comes, on a thread of its own, and writes the
//! call's transcript again after each, so it's ready when the call ends.
//!
//! Whisper is loaded with a call's first take, at the NPU's background
//! priority, and let go once no call has needed it for a while, so it
//! holds no memory between calls. A take's sound is let go once it's
//! heard.
//!
//! A call written down whole is summed up once no call is being heard:
//! Whisper is let go first and the language model after, so the two are
//! never held at once, and a call that starts meanwhile waits its turn.
//! What can't be heard or summed up is said in a notice (notices.rs).

use std::collections::{HashMap, HashSet};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::time::Duration;

use dictation::speech::Speech;
use dictation::transcript::{self, Line, Side, markdown};

use crate::Settings;
use crate::book::{Record, SUMMARY, Shared, TRANSCRIPT, write_whole};
use crate::{notices, summary};

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
    /// Sum the call up again, Joel having asked once it couldn't be.
    SumUp { key: String },
}

/// A call being written.
#[derive(Default)]
struct Writing {
    lines: Vec<Line>,
    done: HashSet<Side>,
}

/// Hears every take given, until nothing more can be; `again` is where
/// a notice's Try Again gives its call back.
pub fn hear(
    settings: &Settings,
    book: &Shared,
    jobs: &Receiver<Job>,
    again: &Sender<Job>,
    me: &str,
) {
    let mut speech: Option<Speech> = None;
    let mut calls: HashMap<String, Writing> = HashMap::new();
    let mut unsummed: Vec<String> = Vec::new();
    // The calls already told they couldn't be heard, so it's said once.
    let mut told: HashSet<String> = HashSet::new();
    loop {
        if calls.is_empty() && !unsummed.is_empty() {
            if speech.take().is_some() {
                eprintln!("call-transcripts: Whisper let go");
            }
            sum_up(settings, book, &std::mem::take(&mut unsummed), again);
        }
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
                        Err(why) => {
                            eprintln!("call-transcripts: Whisper didn't load: {why}");
                            if told.insert(key.clone()) {
                                let title = title_of(book, &key);
                                let why = format!("Whisper didn't load: {why}");
                                notices::show(&notices::not_transcribed(&title, &why), |_| {});
                            }
                        }
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

/// A call's title, as its record has it.
fn title_of(book: &Shared, key: &str) -> String {
    book.lock()
        .record(key)
        .map(|record| record.title.clone())
        .unwrap_or_default()
}

/// Says a call couldn't be summed up, and why, offering Try Again when
/// the cause can pass and the transcript either way.
fn not_summed_up(record: &Record, why: &str, can_pass: bool, again: &Sender<Job>) {
    eprintln!("call-transcripts: {} wasn't summed up: {why}", record.key);
    let notice = notices::not_summed_up(&record.title, why, can_pass);
    let (key, again) = (record.key.clone(), again.clone());
    let transcript = record.transcript.clone();
    notices::show(&notice, move |chosen| match chosen {
        "again" => {
            let _ = again.send(Job::SumUp { key });
        }
        "transcript" => {
            if let Some(file) = transcript {
                notices::open(&file);
            }
        }
        _ => {}
    });
}

/// Sums up each call `keys` names that has a transcript, loading the
/// language model for them and letting it go after.
fn sum_up(settings: &Settings, book: &Shared, keys: &[String], again: &Sender<Job>) {
    let records: Vec<Record> = {
        let book = book.lock();
        keys.iter()
            .filter_map(|key| book.record(key).cloned())
            .filter(|record| record.transcript.is_some())
            .collect()
    };
    if records.is_empty() {
        return;
    }
    // A model that won't load, a summary that fails and a disk with no
    // room can each pass, as the NPU frees or room is made, so each offers
    // Try Again; a transcript that can't be read won't, so it doesn't.
    let mut model = match settings.load_summarizer() {
        Ok(model) => model,
        Err(why) => {
            eprintln!("call-transcripts: the summarizer didn't load: {why}");
            let why = format!("The summarizer didn't load: {why}");
            for record in &records {
                not_summed_up(record, &why, true, again);
            }
            return;
        }
    };
    for record in records {
        let read = record
            .transcript
            .as_ref()
            .map(std::fs::read_to_string)
            .transpose();
        let text = match read {
            Ok(Some(text)) => text,
            Ok(None) => continue,
            Err(why) => {
                let why = format!("Its transcript can't be read: {why}");
                not_summed_up(&record, &why, false, again);
                continue;
            }
        };
        let written = match summary::summarize(&mut model, &record.title, &text) {
            Ok(written) => written,
            Err(why) => {
                not_summed_up(&record, &why, true, again);
                continue;
            }
        };
        let file = record.folder.join(SUMMARY);
        if let Err(why) = write_whole(&file, &format!("{written}\n")) {
            let why = format!("{} couldn't be written: {why}", file.display());
            not_summed_up(&record, &why, true, again);
            continue;
        }
        eprintln!("call-transcripts: {} summed up", record.key);
        tell(&record.title, &written, &file);
        let mut book = book.lock();
        if let Some(now) = book.record(&record.key).cloned() {
            book.put(Record {
                summary: Some(written),
                ..now
            });
        }
    }
    eprintln!("call-transcripts: the summarizer let go");
}

/// Says a call's summary is ready, in a notice whose Read It opens it.
fn tell(title: &str, written: &str, file: &std::path::Path) {
    // The summary's first line, without a bullet's marker, since a
    // notice's line is no list.
    let first = written
        .lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or_default()
        .trim_start_matches(['-', '*', ' '])
        .to_owned();
    let file = file.to_owned();
    notices::show(&notices::summed_up(title, &first), move |chosen| {
        if chosen == "read" {
            notices::open(&file);
        }
    });
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
