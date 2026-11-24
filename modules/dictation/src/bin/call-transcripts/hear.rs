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
//! it is, for an embedding of how each 3 s piece of it sounds
//! (`dictation::pieces`); once the call is written down whole the pieces
//! are grouped by voice (voices.rs), unless the record already names
//! their side, a line is split where its pieces' voice changes
//! (split.rs), and each voice is named where who was in the Meet as it
//! spoke says so (presence.rs). Only the embeddings and the words, with
//! when each was said, are kept, never the sound, and only in memory: the
//! last few calls' stay once they end, so a call rejoined is told apart
//! whole.
//!
//! A call written down whole is given to be summed up (summarizer.rs)
//! once no call is being heard and Parakeet is let go, so the two models
//! are never held at once. What can't be heard is said in a notice
//! (notices.rs).

use std::collections::{HashMap, HashSet};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::time::Duration;

use dictation::mel::RATE;
use dictation::parakeet::{Parakeet, Timings, Word};
use dictation::pieces;
use dictation::transcript::{self, Line, Side, markdown_unechoed};
use dictation::voices::{self, Heard, Voices};

use crate::Settings;
use crate::book::{self, Attendance, Record, Shared, TRANSCRIPT, write_whole};
use crate::filed;
use crate::presence::{self, Present, Spoke, Who};
use crate::split::{self, Piece};
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

/// A call being written, with each piece of a take of theirs that was
/// embedded and which of the lines it's of, if Parakeet heard words in
/// its take: one it heard none in still helps tell the others apart, as
/// it does where the voices check measures them (apart.rs). Each line of
/// theirs heard in this run keeps its words, timed from the call's start,
/// so it can be split where its voice changes. It's `whole` when its embeddings cover
/// the whole call, as they do for one heard from its start in this run,
/// and not for one taken up from its file after a restart. Its `guesses`
/// say who the voices left numbered may be, once it's told apart; they
/// aren't in its lines, so a restart loses them. Its first `read` lines
/// were read back from its file, their echoes already left out.
#[derive(Default)]
struct Writing {
    lines: Vec<Line>,
    read: usize,
    done: HashSet<Side>,
    heard: Vec<Heard>,
    pieces: Vec<Piece>,
    words: HashMap<usize, Vec<Word>>,
    takes: usize,
    whole: bool,
    guesses: Option<String>,
}

/// A piece of a take of theirs as the speaker model heard it: its
/// embedding, the sample of the call it starts at, and how many it holds.
type Embedded = (Vec<f32>, usize, usize);

impl Writing {
    /// Keeps the embedding of each piece of a take of theirs embedded,
    /// with the sample of the call it starts at and how many it holds,
    /// and which line the take is, if it has one; a piece is worded when
    /// the line's `words` that splitting would give it (`split::runs_of`)
    /// say more than hesitation, as a run must to be split into a line of
    /// its own. The voices check has one worded when the reference has any
    /// speech in it, so a piece holding only the end of a word is worded
    /// there and not here.
    fn heard(&mut self, line: Option<usize>, words: &[Word], embedded: Vec<Embedded>) {
        let starts: Vec<usize> = embedded.iter().map(|&(_, start, _)| start).collect();
        let runs = split::runs_of(&starts, words);
        for (piece, (embedding, start, samples)) in embedded.into_iter().enumerate() {
            let in_it: Vec<&str> = words
                .iter()
                .zip(&runs)
                .filter(|&(_, &run)| run == piece)
                .map(|(word, _)| word.text.as_str())
                .collect();
            self.heard.push(Heard {
                embedding,
                samples,
                worded: line.is_some() && said::spoken(&in_it.join(" ")).is_some(),
            });
            self.pieces.push(Piece {
                take: self.takes,
                start,
                line,
            });
        }
        self.takes += 1;
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
                let (text, words) =
                    heard_or_left_out(transcribe_take(parakeet, side, start, &samples));
                // Embedded once Parakeet has heard it, so a call's first
                // line never waits on the speaker model's loading.
                if side == Side::Theirs {
                    let embedded = embed_theirs(settings, &mut speakers, start, &samples);
                    let line = text.is_some().then_some(writing.lines.len());
                    writing.heard(line, &words, embedded);
                }
                let Some(text) = text else {
                    continue;
                };
                if side == Side::Theirs {
                    writing.words.insert(writing.lines.len(), words);
                }
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

/// What hears a take: Parakeet, or a script in the cases, so what's
/// kept of a take is pinned without a model.
pub trait Hears {
    /// The words said in a take, each timed from its start.
    fn words(&mut self, samples: &[f32]) -> dictation::openvino::Result<(Vec<Word>, Timings)>;
    /// What's said in a take.
    fn text(&mut self, samples: &[f32]) -> dictation::openvino::Result<(String, Timings)>;
    /// Where it runs, for the journal.
    fn device(&self) -> &str;
}

impl Hears for Parakeet {
    fn words(&mut self, samples: &[f32]) -> dictation::openvino::Result<(Vec<Word>, Timings)> {
        self.transcribe_words(samples)
    }

    fn text(&mut self, samples: &[f32]) -> dictation::openvino::Result<(String, Timings)> {
        self.transcribe(samples)
    }

    fn device(&self) -> &str {
        &self.device
    }
}

/// What's heard in a take of `side`'s, `start` samples into the call, as
/// the call keeps it: its text tidied, none when it holds no words, or
/// why it isn't heard, which the daemon logs and the `transcribe`
/// subcommand stops on. Only their words are kept, spelling their text
/// a space between each, as `transcribe` would, and timed from the
/// call's start, which is where its pieces are, so each word finds the
/// piece it was said in.
pub fn transcribe_take(
    hears: &mut impl Hears,
    side: Side,
    start: usize,
    samples: &[f32],
) -> dictation::openvino::Result<(Option<String>, Vec<Word>)> {
    let transcribed = if side == Side::Theirs {
        hears.words(samples).map(|(words, took)| {
            let spelled: Vec<&str> = words.iter().map(|word| word.text.as_str()).collect();
            (spelled.join(" "), from_call_start(start, words), took)
        })
    } else {
        hears
            .text(samples)
            .map(|(text, took)| (text, Vec::new(), took))
    };
    let (text, words, took) = transcribed?;
    eprintln!("call-transcripts: {}", took.describe(hears.device()));
    Ok((said::spoken(text.trim()), words))
}

/// What the daemon goes on with from a take `heard`: what was heard, or,
/// when it wasn't, no text and no words, logged once. One take not heard
/// mustn't end the call: it's left out, a take of theirs still helps tell
/// the voices apart, and the next is heard as ever.
fn heard_or_left_out(
    heard: dictation::openvino::Result<(Option<String>, Vec<Word>)>,
) -> (Option<String>, Vec<Word>) {
    heard.expect("a take wasn't heard")
}

/// A take's `words`, timed from its start, timed from the call's instead,
/// the take starting `start` samples into it.
fn from_call_start(start: usize, words: Vec<Word>) -> Vec<Word> {
    words
        .into_iter()
        .map(|word| Word {
            at: start + word.at,
            ..word
        })
        .collect()
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

/// A take of theirs embedded in pieces by the speaker model, as
/// `embed_take` says, its failures logged.
fn embed_theirs(
    settings: &Settings,
    speakers: &mut Speakers,
    start: usize,
    samples: &[f32],
) -> Vec<Embedded> {
    let (embedded, failed) = embed_take(start, samples, |piece| embed(settings, speakers, piece));
    if let Some(failed) = failed {
        eprintln!("call-transcripts: {failed}");
    }
    embedded
}

/// Each piece of a take of theirs that starts `start` samples into the
/// call, as `embed` hears it, with the sample of the call it starts at
/// and how many it holds. A piece `embed` fails on is left out, and the
/// failures are told in one line a take, so a model failing on every
/// piece doesn't bury the journal. The line gives the first failure's
/// reason, since it's most often the cause and later ones repeat it.
fn embed_take(
    start: usize,
    samples: &[f32],
    mut embed: impl FnMut(&[f32]) -> Result<Option<Vec<f32>>, String>,
) -> (Vec<Embedded>, Option<String>) {
    let cut = pieces::pieces(samples.len());
    let mut embedded = Vec::new();
    let mut failed = (0, None);
    for &(from, to) in &cut {
        let piece = &samples[from..to];
        match embed(piece) {
            Ok(Some(embedding)) => embedded.push((embedding, start + from, piece.len())),
            Ok(None) => {}
            Err(why) => {
                failed.0 += 1;
                failed.1.get_or_insert(why);
            }
        }
    }
    let of = cut.len();
    let failed = match failed {
        (over, Some(why)) => Some(format!("{over} of {of} pieces' voice not heard: {why}")),
        _ => None,
    };
    (embedded, failed)
}

/// How a piece of a take of theirs sounds, from the speaker model, loaded
/// first if it isn't; none when there's no model or it won't load, which
/// is logged once, or the piece is too short, and why when it fails.
fn embed(
    settings: &Settings,
    speakers: &mut Speakers,
    samples: &[f32],
) -> Result<Option<Vec<f32>>, String> {
    let Some(model) = settings.voices.as_deref() else {
        return Ok(None);
    };
    let Some(voices) = speakers.get(|| Voices::load(model, settings.cache.as_deref())) else {
        return Ok(None);
    };
    voices.embed(samples)
}

/// Gives each of their lines the voice its take was grouped into, unless
/// the record names their side for certain, as when one other person was
/// on the call; no more voices than the people known to be on it. Once
/// they're grouped anew, each voice is named where who was in the Meet
/// says so.
fn tell_apart(book: &Shared, key: &str, writing: &mut Writing, me: &str) {
    let (record, seen) = {
        let book = book.lock();
        let Some(record) = book.record(key).cloned() else {
            return;
        };
        (record, book.attendance(key))
    };
    if let TheirSide::Unnamed { people } = their_side(&record, me)
        && give_voices(writing, people)
    {
        writing.guesses = give_names(&mut writing.lines, &record, &seen);
        // Counts alone, never names, so the log holds nothing of the call.
        let voices = |named: bool| {
            let theirs = writing.lines.iter().filter(|line| line.voice.is_some());
            let chosen = theirs.filter(|line| line.name.is_some() || !named);
            chosen
                .filter_map(|line| line.voice)
                .collect::<HashSet<_>>()
                .len()
        };
        eprintln!(
            "call-transcripts: {} of {} voices named from {} seen in the Meet, {key}",
            voices(true),
            voices(false),
            seen.present.len(),
        );
    }
}

/// Gives each line of theirs whose voice is someone for certain that
/// person's name, by who the `record` has on the call and when Meet was
/// `seen` to have each in; what can be said of the voices left numbered
/// is given back, for the transcript's note.
fn give_names(lines: &mut [Line], record: &Record, seen: &Attendance) -> Option<String> {
    // A call whose Meet was never seen, a Tuple call or a Meet in a
    // browser's tab, says nothing of who was in when.
    if seen.present.is_empty() {
        return None;
    }
    // Someone on the call Meet gave no time for counts as in throughout,
    // so a voice of theirs is never put under another's name.
    let mut present = seen.present.clone();
    for name in &record.people {
        if !present.iter().any(|person| &person.name == name) {
            let name = name.clone();
            present.push(Present {
                name,
                joined: 0,
                left: None,
            });
        }
    }
    let at = |sample: usize| {
        let since = u64::try_from(sample * 1000 / RATE).unwrap_or(u64::MAX);
        record.started.saturating_add(since)
    };
    // What was said after the Meet was last looked at is left out, since
    // someone may have joined since.
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
        .filter(|take| take.to <= seen.looked)
        .collect();
    let voices = lines.iter().filter_map(|line| line.voice).max()? + 1;
    let who = presence::who(&present, &spoke, voices);
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

/// Gives each line of theirs the voice its pieces were grouped into, at
/// most `people` of them, a piece grouped into none taking the nearest
/// voice of its take's pieces, and splits a line where its pieces' voice
/// changes (split.rs). A call heard whole in this run, rejoins and all, is
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
    if writing.heard.len() > voices::MOST_TAKES {
        // Past the cap the shortest pieces go ungrouped and take their
        // take's nearest voice; a take with none left stays under Others,
        // so a transcript whose end does says why here.
        let over = writing.heard.len() - voices::MOST_TAKES;
        let of = writing.heard.len();
        eprintln!("call-transcripts: {over} of {of} pieces left out of telling voices apart");
    }
    let grouped = voices::group(&writing.heard, people);
    let takes: Vec<usize> = writing.pieces.iter().map(|piece| piece.take).collect();
    let voiced = pieces::within_takes(&takes, &grouped);
    let mut call = split::Call {
        lines: &mut writing.lines,
        pieces: &mut writing.pieces,
        words: &mut writing.words,
    };
    let split::Splits {
        of,
        split: cut,
        into,
    } = split::by_voice(&mut call, &voiced);
    if of > 0 {
        eprintln!("call-transcripts: {cut} of {of} takes of theirs split by voice, into {into}");
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
        read: lines.len(),
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
    if !writing.lines.is_empty() {
        let them = record.them(me);
        let note = record.note(me);
        let said = transcript_of(writing, me, them.as_deref(), note.as_deref());
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

/// The transcript `writing` holds, under their side's name `them` where
/// it's known and the voices' `note`. Joel's echoes are left out against
/// each take of theirs whole, never its parts, so a take split by voice
/// neither brings back an echo of it nor takes away his answer to it.
/// Lines read back from the file were told when first written, and are
/// kept, since a split take's parts read back can't be joined again.
fn transcript_of(writing: &Writing, me: &str, them: Option<&str>, note: Option<&str>) -> String {
    let heard = split::unechoed(&writing.lines, &writing.pieces, writing.read);
    let guesses = writing.guesses.as_deref();
    markdown_unechoed(&heard, me, them, note, guesses)
}

#[cfg(test)]
#[path = "hear_tests.rs"]
mod tests;
