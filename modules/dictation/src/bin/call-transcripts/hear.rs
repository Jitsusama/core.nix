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
//! Each take of theirs, once Parakeet has heard it, is also given to the
//! speaker model, loaded and let go with Parakeet and kept compiled where
//! it is, for an embedding of how it sounds; once the call is written
//! down whole the takes are grouped by voice (voices.rs), unless the
//! record already names their side, and each voice is named where who
//! was in the Meet as it spoke says so (presence.rs). Only the
//! embeddings are kept, never
//! the sound, and only in memory: the last few calls' stay once they end,
//! so a call rejoined is told apart whole.
//!
//! A call written down whole is given to be summed up (summarizer.rs)
//! once no call is being heard and Parakeet is let go, so the two models
//! are never held at once. What can't be heard is said in a notice
//! (notices.rs).

use std::collections::{HashMap, HashSet};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::time::Duration;

use dictation::mel::RATE;
use dictation::parakeet::Parakeet;
use dictation::transcript::{self, Line, Side, markdown_naming};
use dictation::voices::{self, Heard, Voices};

use crate::Settings;
use crate::book::{self, Record, Shared, TRANSCRIPT, write_whole};
use crate::filed;
use crate::presence::{self, Present, Spoke, Who};
use crate::{notices, said};

/// How long Parakeet is kept once the last call is written, so a call
/// rejoined soon after needn't load it again.
const UNUSED: Duration = Duration::from_mins(2);

/// How many calls that ended are kept as they were heard, embeddings and
/// all, so one rejoined is told apart whole: a call's are a few hundred
/// kilobytes an hour.
const KEPT_ENDED: usize = 4;

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
/// which of the lines it is, if Parakeet heard words in it: one it heard
/// none in still helps tell the others apart, as it does where the voices
/// check measures them (apart.rs). It's `whole` when its embeddings cover
/// the whole call, as they do for one heard from its start in this run,
/// and not for one taken up from its file after a restart. Its `guesses`
/// say who the voices left numbered may be, once it's told apart; they
/// aren't in its lines, so a restart loses them.
#[derive(Default)]
struct Writing {
    lines: Vec<Line>,
    done: HashSet<Side>,
    heard: Vec<Heard>,
    heard_lines: Vec<Option<usize>>,
    whole: bool,
    guesses: Option<String>,
}

impl Writing {
    /// Keeps a take's embedding, and which line it is, if it has one.
    fn heard(&mut self, embedding: Vec<f32>, samples: usize, line: Option<usize>) {
        self.heard.push(Heard {
            embedding,
            samples,
            worded: line.is_some(),
        });
        self.heard_lines.push(line);
    }
}

/// The speaker model, loaded with a call's first take of theirs, and
/// whether it wouldn't load, so it isn't read again for every take
/// until it's let go with Parakeet.
#[derive(Default)]
struct Speakers {
    model: Option<Voices>,
    refused: bool,
}

impl Speakers {
    /// The model, loaded with `load` first when it isn't and hasn't
    /// refused, a refusal logged once.
    fn get(&mut self, load: impl FnOnce() -> Result<Voices, String>) -> Option<&mut Voices> {
        if self.model.is_none() && !self.refused {
            match load() {
                Ok(loaded) => self.model = Some(loaded),
                Err(why) => {
                    eprintln!("call-transcripts: the speaker model didn't load: {why}");
                    self.refused = true;
                }
            }
        }
        self.model.as_mut()
    }
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
    let mut speakers = Speakers::default();
    let mut calls: HashMap<String, Writing> = HashMap::new();
    let mut ended: Vec<(String, Writing)> = Vec::new();
    let mut unsummed: Vec<String> = Vec::new();
    // The calls already told they couldn't be heard, so it's said once.
    let mut told: HashSet<String> = HashSet::new();
    compile_ahead(settings);
    loop {
        if calls.is_empty() && !unsummed.is_empty() {
            speakers = Speakers::default();
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
                    speakers = Speakers::default();
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
                    .or_insert_with(|| taken_up(&mut ended, book, &key, me));
                let Some(parakeet) = loaded(settings, &mut speech, book, &key, &mut told) else {
                    continue;
                };
                let text = match parakeet.transcribe(&samples) {
                    Ok((text, took)) => {
                        eprintln!("call-transcripts: {}", took.describe(&parakeet.device));
                        said::spoken(text.trim())
                    }
                    Err(why) => {
                        eprintln!("call-transcripts: a take wasn't heard: {why}");
                        None
                    }
                };
                // Embedded once Parakeet has heard it, so a call's first
                // line never waits on the speaker model's loading.
                if side == Side::Theirs
                    && let Some(embedding) = embed(settings, &mut speakers, &samples)
                {
                    let line = text.is_some().then_some(writing.lines.len());
                    writing.heard(embedding, samples.len(), line);
                }
                let Some(text) = text else {
                    continue;
                };
                writing.lines.push(Line {
                    side,
                    start,
                    end: start + samples.len(),
                    text,
                    voice: None,
                    name: None,
                });
                write(book, &key, writing, me, true);
            }
            Job::Done { key, side } => {
                let writing = calls
                    .entry(key.clone())
                    .or_insert_with(|| taken_up(&mut ended, book, &key, me));
                writing.done.insert(side);
                if writing.done.len() == 2 {
                    tell_apart(book, &key, writing, me);
                    write(book, &key, writing, me, false);
                    if let Some(writing) = calls.remove(&key) {
                        keep(&mut ended, &key, writing);
                    }
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
/// it isn't; none when there's no model, it won't load, which is logged
/// once, or it fails, which is logged, and the take is under Others.
fn embed(settings: &Settings, speakers: &mut Speakers, samples: &[f32]) -> Option<Vec<f32>> {
    let model = settings.voices.as_deref()?;
    let voices = speakers.get(|| Voices::load(model, settings.cache.as_deref()))?;
    match voices.embed(samples) {
        Ok(embedding) => embedding,
        Err(why) => {
            eprintln!("call-transcripts: a take's voice wasn't heard: {why}");
            None
        }
    }
}

/// Gives each of their lines the voice its take was grouped into, unless
/// the record names their side for certain, as when one other person was
/// on the call; no more voices than the people known to be on it. Once
/// they're grouped anew, each voice is named where who was in the Meet
/// says so.
fn tell_apart(book: &Shared, key: &str, writing: &mut Writing, me: &str) {
    let (record, present) = {
        let book = book.lock();
        let Some(record) = book.record(key).cloned() else {
            return;
        };
        (record, book.presence(key))
    };
    if let TheirSide::Unnamed { people } = their_side(&record, me)
        && give_voices(writing, people)
    {
        writing.guesses = give_names(&mut writing.lines, record.started, &present);
    }
}

/// Gives each line of theirs whose voice was `present` for certain that
/// person's name, for a call that `started` then, in milliseconds since
/// the epoch, as Meet says when each was in; what can be said of the
/// voices left numbered is given back, for the transcript's note.
fn give_names(lines: &mut [Line], started: u64, present: &[Present]) -> Option<String> {
    if present.is_empty() {
        return None;
    }
    let at = |sample: usize| started + u64::try_from(sample * 1000 / RATE).unwrap_or(u64::MAX);
    let spoke: Vec<Spoke> = lines
        .iter()
        .filter(|line| line.side == Side::Theirs)
        .filter_map(|line| {
            Some(Spoke {
                voice: line.voice?,
                from: at(line.start),
                to: at(line.end),
            })
        })
        .collect();
    let voices = spoke.iter().map(|take| take.voice + 1).max()?;
    let who = presence::who(present, &spoke, voices);
    for line in lines.iter_mut() {
        if let Some(Who::Is(name)) = line.voice.map(|voice| &who[voice]) {
            line.name = Some(name.clone());
        }
    }
    let heard: Vec<bool> = (0..voices)
        .map(|voice| spoke.iter().any(|take| take.voice == voice))
        .collect();
    presence::guesses(&who, &heard)
}

/// What a call's record says of their side, for telling its voices apart.
#[derive(Debug, PartialEq)]
enum TheirSide {
    /// Named for certain, as when one other person was on the call, so
    /// its voices aren't told apart.
    Named,
    /// Not named, so its voices are told apart: at most as many as the
    /// others on the call, where the record knows how many.
    Unnamed { people: Option<usize> },
}

/// What `record` says of their side, Joel, `me`, never among the others.
fn their_side(record: &Record, me: &str) -> TheirSide {
    if record.them(me).is_some() {
        TheirSide::Named
    } else {
        TheirSide::Unnamed {
            people: record.known.map(|_| record.people.len()),
        }
    }
}

/// Keeps a call that ended as it was heard, embeddings and all, so one
/// rejoined is told apart whole; only the last `KEPT_ENDED` are kept.
fn keep(ended: &mut Vec<(String, Writing)>, key: &str, mut writing: Writing) {
    writing.done.clear();
    ended.push((key.to_owned(), writing));
    if ended.len() > KEPT_ENDED {
        ended.remove(0);
    }
}

/// Gives each line of theirs the voice its take was grouped into, at most
/// `people` of them. A call heard whole in this run, rejoins and all, is
/// grouped whole again. Embeddings are kept only in memory, so a call
/// taken up again after a restart has none for what it said before: when
/// that part was told apart its voices stay, and this part's lines stay
/// under Others, since which of those voices each is can't be said, and
/// numbering them anew would call one person two; a voice named then
/// keeps its name. Whether they were grouped anew is given back.
fn give_voices(writing: &mut Writing, people: Option<usize>) -> bool {
    writing.guesses = None;
    if writing.whole {
        for line in &mut writing.lines {
            line.voice = None;
            line.name = None;
        }
    } else if writing
        .lines
        .iter()
        .any(|line| line.voice.is_some() || line.name.is_some())
    {
        return false;
    }
    let grouped = voices::group(&writing.heard, people);
    for (line, voice) in writing.heard_lines.iter().zip(grouped) {
        if let Some(&index) = line.as_ref() {
            writing.lines[index].voice = voice;
        }
    }
    true
}

/// A call's title, as its record has it.
fn title_of(book: &Shared, key: &str) -> String {
    book.lock()
        .record(key)
        .map(|record| record.title.clone())
        .unwrap_or_default()
}

/// A call taken up again: as this run held it when it ended, embeddings
/// and all, so a rejoin is told apart whole; else as its file has it.
fn taken_up(ended: &mut Vec<(String, Writing)>, book: &Shared, key: &str, me: &str) -> Writing {
    match ended.iter().position(|(kept, _)| kept == key) {
        Some(at) => ended.remove(at).1,
        None => resumed(book, key, me),
    }
}

/// What a call already says, taken up again: what the transcript of a
/// call recorded before the daemon restarted holds, or nothing for a call
/// just begun, which is then heard whole. A line is Joel's only when it's
/// under his name, `me`. Only its Transcript section is read, so the
/// summary's quotes are never lines said twice. A line under a name keeps
/// it only while the record doesn't name their side, since otherwise
/// that name is their side's, not a voice's.
fn resumed(book: &Shared, key: &str, me: &str) -> Writing {
    let Some(record) = book.lock().record(key).cloned() else {
        return Writing {
            whole: true,
            ..Writing::default()
        };
    };
    let mut lines: Vec<Line> = record
        .transcript
        .as_ref()
        .and_then(|file| std::fs::read_to_string(file).ok())
        .map(|text| transcript::read(filed::transcript(&text), me))
        .unwrap_or_default();
    if their_side(&record, me) == TheirSide::Named {
        for line in &mut lines {
            line.name = None;
        }
    }
    Writing {
        whole: lines.is_empty(),
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
fn write(book: &Shared, key: &str, writing: &Writing, me: &str, transcribing: bool) {
    let mut book = book.lock();
    let Some(record) = book.record(key).cloned() else {
        return;
    };
    let file = record.folder.join(TRANSCRIPT);
    let mut opening = record.opening.clone();
    let lines = &writing.lines;
    if !lines.is_empty() {
        let them = record.them(me);
        let note = record.note(me);
        let guesses = writing.guesses.as_deref();
        let said = markdown_naming(lines, me, them.as_deref(), note.as_deref(), guesses);
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

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::meeting::Known;

    const SECOND: usize = 16_000;

    fn line(side: Side, at: usize, voice: Option<usize>) -> Line {
        Line {
            side,
            start: at * SECOND,
            end: (at + 5) * SECOND,
            text: "Said.".to_owned(),
            voice,
            name: None,
        }
    }

    const ADA: [f32; 2] = [1.0, 0.0];
    const BO: [f32; 2] = [0.0, 1.0];

    /// Hears each take into `writing`, a line of theirs for each said.
    fn hear_takes(writing: &mut Writing, takes: &[([f32; 2], bool)]) {
        for &(voice, said) in takes {
            let at = writing.lines.len();
            let line_at = said.then_some(at);
            writing.heard(voice.to_vec(), 5 * SECOND, line_at);
            if said {
                writing.lines.push(line(Side::Theirs, 20 + 5 * at, None));
            }
        }
    }

    /// A call heard in one sitting: Ada, Bo, Joel, Ada again, a take of
    /// Bo's Parakeet heard no words in, and Bo again.
    fn sitting(earlier: Vec<Line>, whole: bool) -> Writing {
        let mut writing = Writing {
            lines: earlier,
            whole,
            ..Writing::default()
        };
        hear_takes(&mut writing, &[(ADA, true), (BO, true)]);
        writing.lines.push(line(Side::Mine, 10, None));
        hear_takes(&mut writing, &[(ADA, true), (BO, false), (BO, true)]);
        writing
    }

    fn voices(writing: &Writing) -> Vec<Option<usize>> {
        writing.lines.iter().map(|line| line.voice).collect()
    }

    #[test]
    fn each_line_heard_is_given_its_takes_voice() {
        let mut writing = sitting(Vec::new(), true);
        give_voices(&mut writing, None);
        assert_eq!(
            voices(&writing),
            [Some(0), Some(1), None, Some(0), Some(1)],
            "Ada's and Bo's lines weren't given their own voices, or Joel's was given one"
        );
    }

    /// A take whose embedding is `degrees` round from the first axis.
    fn at(degrees: f32) -> [f32; 2] {
        let radians = degrees.to_radians();
        [radians.cos(), radians.sin()]
    }

    #[test]
    fn a_take_with_no_words_helps_tell_the_voices_apart() {
        // Ada's two lines are 0.62 apart, too far to be one voice alone,
        // but a take of hers between them with no words is 0.17 from
        // each, and joined to one it brings the other within 0.6.
        let (first, between, second, bo) = (at(0.0), at(33.8), at(67.67), at(180.0));
        let mut helped = Writing::default();
        hear_takes(
            &mut helped,
            &[(first, true), (between, false), (second, true), (bo, true)],
        );
        give_voices(&mut helped, None);
        let mut unhelped = Writing::default();
        hear_takes(&mut unhelped, &[(first, true), (second, true), (bo, true)]);
        give_voices(&mut unhelped, None);
        assert_eq!(
            voices(&helped),
            [Some(0), Some(0), Some(1)],
            "Ada's lines weren't one voice with her wordless take between them"
        );
        assert_eq!(
            voices(&unhelped),
            [Some(0), Some(1), Some(2)],
            "Ada's lines were one voice without the wordless take, so it proved nothing"
        );
    }

    #[test]
    fn a_call_rejoined_in_this_run_is_told_apart_whole() {
        let mut writing = sitting(Vec::new(), true);
        give_voices(&mut writing, None);
        hear_takes(&mut writing, &[(BO, true), (ADA, true)]);
        give_voices(&mut writing, None);
        assert_eq!(
            voices(&writing),
            [Some(0), Some(1), None, Some(0), Some(1), Some(1), Some(0)],
            "the lines after a rejoin weren't given the voices the call already had"
        );
    }

    #[test]
    fn a_call_taken_up_again_keeps_the_voices_it_had_and_its_new_lines_stay_others() {
        let earlier = vec![
            line(Side::Theirs, 0, Some(0)),
            line(Side::Theirs, 5, Some(1)),
        ];
        let mut writing = sitting(earlier, false);
        give_voices(&mut writing, None);
        let voices: Vec<Option<usize>> = writing.lines.iter().map(|line| line.voice).collect();
        assert_eq!(
            &voices[..2],
            [Some(0), Some(1)],
            "the voices told apart before the rejoin were wiped"
        );
        assert!(
            voices[2..].iter().all(Option::is_none),
            "lines after a restart were numbered anew beside the old voices: {voices:?}"
        );
    }

    const STARTED: u64 = 1_800_000_000_000;

    fn present(name: &str, joined_s: u64) -> Present {
        Present {
            name: name.to_owned(),
            joined: STARTED + joined_s * 1000,
            left: None,
        }
    }

    fn names(lines: &[Line]) -> Vec<Option<&str>> {
        lines.iter().map(|line| line.name.as_deref()).collect()
    }

    #[test]
    fn a_voice_heard_only_while_one_other_was_in_takes_that_name_and_the_rest_are_guessed() {
        let mut lines = vec![
            line(Side::Theirs, 10, Some(0)),
            line(Side::Mine, 30, None),
            line(Side::Theirs, 60, Some(0)),
            line(Side::Theirs, 600, Some(1)),
            line(Side::Theirs, 620, None),
        ];
        let present = [present("Ada", 0), present("Bo", 300)];
        let guesses = give_names(&mut lines, STARTED, &present);
        assert_eq!(
            names(&lines),
            [Some("Ada"), None, Some("Ada"), None, None],
            "Ada's voice wasn't named for her, or Joel's line or Bo's or Others' was named"
        );
        assert_eq!(
            guesses.as_deref(),
            Some("Voice 2 is likely Bo."),
            "the voice heard while both were in wasn't said only likely Bo"
        );
    }

    #[test]
    fn with_nobody_seen_in_the_meet_no_voice_is_named_or_guessed() {
        let mut lines = vec![line(Side::Theirs, 10, Some(0))];
        assert_eq!(give_names(&mut lines, STARTED, &[]), None);
        assert_eq!(names(&lines), [None], "a voice was named with nobody seen");
    }

    #[test]
    fn a_call_taken_up_again_keeps_the_names_it_had() {
        let named = Line {
            name: Some("Ada".to_owned()),
            ..line(Side::Theirs, 0, None)
        };
        let mut writing = sitting(vec![named], false);
        assert!(
            !give_voices(&mut writing, None),
            "a call taken up again was grouped anew beside the voices it had"
        );
        assert_eq!(
            writing.lines[0].name.as_deref(),
            Some("Ada"),
            "the name given before a restart was lost"
        );
    }

    fn record(people: &[&str], known: Option<Known>) -> Record {
        Record {
            key: "call:e1".to_owned(),
            title: "Planning".to_owned(),
            started: 0,
            ended: None,
            event: Some("e1".to_owned()),
            person: None,
            account: None,
            people: people.iter().map(|&someone| someone.to_owned()).collect(),
            known,
            folder: PathBuf::new(),
            transcript: None,
            opening: None,
            summary: None,
            transcribing: false,
        }
    }

    #[test]
    fn voices_are_told_apart_only_where_their_side_is_unnamed_and_no_more_than_were_on() {
        assert_eq!(
            their_side(&record(&["Ada"], Some(Known::Invitation)), "Joel"),
            TheirSide::Named,
            "voices were told apart on a call whose one other person is named"
        );
        assert_eq!(
            their_side(&record(&["Ada", "Bo"], Some(Known::Tuple)), "Joel"),
            TheirSide::Unnamed { people: Some(2) },
            "with two others known, the voices weren't held to two"
        );
        assert_eq!(
            their_side(&record(&[], None), "Joel"),
            TheirSide::Unnamed { people: None },
            "with nobody known, the voices were held to a count"
        );
    }

    #[test]
    fn a_speaker_model_that_wont_load_is_tried_once_until_let_go() {
        let mut speakers = Speakers::default();
        let mut tries = 0;
        for _ in 0..3 {
            let model = speakers.get(|| {
                tries += 1;
                Err("no model".to_owned())
            });
            assert!(model.is_none(), "a model that won't load was given");
        }
        assert_eq!(
            tries, 1,
            "a model that won't load was read again for each take"
        );
        speakers = Speakers::default();
        let _ = speakers.get(|| {
            tries += 1;
            Err("no model".to_owned())
        });
        assert_eq!(tries, 2, "a model let go wasn't tried again");
    }
}
