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
//! Each take of theirs is also given to the speaker model, loaded and let
//! go with Parakeet, for an embedding of how it sounds; once the call is
//! written down whole the takes are grouped by voice (voices.rs), unless
//! the record already names their side. Only the embeddings are kept
//! until then, never the sound.
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
use dictation::voices::{self, Heard, Voices};

use crate::Settings;
use crate::book::{self, Record, Shared, TRANSCRIPT, write_whole};
use crate::filed;
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

/// A call being written, with each take of theirs that was embedded and
/// which of the lines it is.
#[derive(Default)]
struct Writing {
    lines: Vec<Line>,
    done: HashSet<Side>,
    heard: Vec<Heard>,
    heard_lines: Vec<usize>,
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
    let mut speakers: Option<Voices> = None;
    let mut calls: HashMap<String, Writing> = HashMap::new();
    let mut unsummed: Vec<String> = Vec::new();
    // The calls already told they couldn't be heard, so it's said once.
    let mut told: HashSet<String> = HashSet::new();
    compile_ahead(settings);
    loop {
        if calls.is_empty() && !unsummed.is_empty() {
            speakers = None;
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
                    speakers = None;
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
                    .or_insert_with(|| resumed(book, &key, me));
                let Some(parakeet) = loaded(settings, &mut speech, book, &key, &mut told) else {
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
                if side == Side::Theirs
                    && let Some(embedding) = embed(settings, &mut speakers, &samples)
                {
                    writing.heard_lines.push(writing.lines.len());
                    writing.heard.push(Heard {
                        embedding,
                        samples: samples.len(),
                    });
                }
                writing.lines.push(Line {
                    side,
                    start,
                    end: start + samples.len(),
                    text,
                    voice: None,
                });
                write(book, &key, &writing.lines, me, true);
            }
            Job::Done { key, side } => {
                let writing = calls
                    .entry(key.clone())
                    .or_insert_with(|| resumed(book, &key, me));
                writing.done.insert(side);
                if writing.done.len() == 2 {
                    tell_apart(book, &key, writing, me);
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

/// Compiles Parakeet and lets it go, when it's for the NPU and nothing is
/// compiled for this build, so no call's first take waits for it. Without
/// an NPU it compiles as a call's first take loads it, which is as quick.
fn compile_ahead(settings: &Settings) {
    if !settings.worth_compiling() {
        eprintln!("call-transcripts: Parakeet needs no compile ahead of a call");
        return;
    }
    match settings.load() {
        Ok(_) => eprintln!("call-transcripts: Parakeet compiled ahead of a call, and let go"),
        Err(why) => eprintln!("call-transcripts: Parakeet didn't compile ahead: {why}"),
    }
}

/// Parakeet, loaded first if it isn't; none when it won't load, which a
/// notice says once a call, `told` holding the calls already told.
fn loaded<'a>(
    settings: &Settings,
    speech: &'a mut Option<Parakeet>,
    book: &Shared,
    key: &str,
    told: &mut HashSet<String>,
) -> Option<&'a mut Parakeet> {
    if speech.is_none() {
        match settings.load() {
            Ok(parakeet) => *speech = Some(parakeet),
            Err(why) => {
                eprintln!("call-transcripts: Parakeet didn't load: {why}");
                if told.insert(key.to_owned()) {
                    let title = title_of(book, key);
                    let why = format!("The speech model didn't load: {why}");
                    notices::show(&notices::not_transcribed(&title, &why), |_| {});
                }
            }
        }
    }
    speech.as_mut()
}

/// How a take of theirs sounds, from the speaker model, loaded first if
/// it isn't; none when there's no model, or it fails, which is logged and
/// leaves the take under Others.
fn embed(settings: &Settings, speakers: &mut Option<Voices>, samples: &[f32]) -> Option<Vec<f32>> {
    let model = settings.voices.as_deref()?;
    if speakers.is_none() {
        match Voices::load(model) {
            Ok(loaded) => *speakers = Some(loaded),
            Err(why) => {
                eprintln!("call-transcripts: the speaker model didn't load: {why}");
                return None;
            }
        }
    }
    match speakers.as_mut()?.embed(samples) {
        Ok(embedding) => embedding,
        Err(why) => {
            eprintln!("call-transcripts: a take's voice wasn't heard: {why}");
            None
        }
    }
}

/// Gives each of their lines the voice its take was grouped into, unless
/// the record names their side for certain, as when one other person was
/// on the call; no more voices than the people known to be on it. A line
/// taken up again from before a restart or a rejoin has no embedding, so
/// once any take is embedded it's under Others: which of the new voices it
/// was can't be said.
fn tell_apart(book: &Shared, key: &str, writing: &mut Writing, me: &str) {
    if writing.heard.is_empty() {
        return;
    }
    let Some(record) = book.lock().record(key).cloned() else {
        return;
    };
    if record.them(me).is_some() {
        return;
    }
    let people = record.known.map(|_| record.people.len());
    for line in &mut writing.lines {
        line.voice = None;
    }
    let grouped = voices::group(&writing.heard, people);
    for (&index, voice) in writing.heard_lines.iter().zip(grouped) {
        writing.lines[index].voice = voice;
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
/// call rejoined, or recorded before the daemon restarted, holds, a line
/// Joel's only when it's under his name, `me`. Only its Transcript
/// section is read, so the summary's quotes are never lines said twice.
fn resumed(book: &Shared, key: &str, me: &str) -> Writing {
    let lines = book
        .lock()
        .record(key)
        .and_then(|record| record.transcript.clone())
        .and_then(|file| std::fs::read_to_string(file).ok())
        .map(|text| transcript::read(filed::transcript(&text), me))
        .unwrap_or_default();
    Writing {
        lines,
        ..Writing::default()
    }
}

/// Writes a call's file whole, its transcript under its summary, their
/// side under their name where the record knows it for certain, and
/// says in its record where it is, how it opens and whether it's still
/// being written. The summary is the record's, so writing the
/// transcript again keeps it; a call that's still on has none, since
/// a rejoin lets the last one go.
fn write(book: &Shared, key: &str, lines: &[Line], me: &str, transcribing: bool) {
    let mut book = book.lock();
    let Some(record) = book.record(key).cloned() else {
        return;
    };
    let file = record.folder.join(TRANSCRIPT);
    let mut opening = record.opening.clone();
    if !lines.is_empty() {
        let them = record.them(me);
        let note = record.note(me);
        let said = markdown(lines, me, them.as_deref(), note.as_deref());
        let written = filed::whole(&record.title, record.summary.as_deref(), &said);
        if let Err(why) = write_whole(&file, &written) {
            eprintln!("call-transcripts: writing {}: {why}", file.display());
            return;
        }
        opening = book::opening(&said);
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
