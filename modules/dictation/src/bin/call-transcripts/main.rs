//! call-transcripts: writes down what's said on Joel's calls, on this
//! machine, with Parakeet (dictation's parakeet.rs) on the NPU taking its
//! turn behind dictation and anything live, since nobody waits on a
//! transcript.
//!
//! Each side is heard apart, Joel's microphone and the call's own sound,
//! so who said what needs no guessing. Each side is cut at its pauses into
//! takes (takes.rs), each take is heard, and the two sides are laid out
//! in the order they spoke (transcript.rs). Parakeet, not dictation's
//! Whisper, hears them: it misheard a fifth fewer words of meetings, and
//! the one thing Whisper did better, writing the names in dictation's
//! words, a call couldn't use, since Whisper told them wrote them back
//! for a take with no speech in it. A take that's only what a speech
//! model writes for a sound is no turn (said.rs).
//!
//! Usage:
//!
//! ```text
//! call-transcripts serve SOCKET MODEL_DIR [SUMMARIZER] CALLS
//!     follow calls as they come and go, writing each one's transcript
//!     and then its summary into a folder of its own in CALLS, and say
//!     them over SOCKET in the shell's daemon interface (book.rs); the
//!     summary's model is asked through SUMMARIZER, a program that
//!     answers as `ask` does (summarizer.rs); with none, a call is only
//!     written down, and a notice says so
//! call-transcripts ask SUMMARIZER_DIR
//!     answer prompts a line each, as JSON, with the language model in
//!     SUMMARIZER_DIR, until the input ends (ask.rs)
//! call-transcripts transcribe MODEL_DIR MINE.wav THEIRS.wav
//!     print, as Markdown, the transcript of a call's two sides, each a
//!     16 kHz mono WAV starting at the call's start
//! call-transcripts hear MODEL_DIR CLIP.wav...
//!     print what's said in each 16 kHz mono WAV, heard whole as one take,
//!     a line each, which is how a model's mishearings are counted
//! call-transcripts score MODEL_DIR SET [CEILING]
//!     write down each meeting in SET as a call and print the share of
//!     its words each side got wrong, written down and heard alone
//!     (score.rs); fail when the set's, written down, is above CEILING
//! call-transcripts voices SPEAKER_MODEL SET [RIGHT HANDLED]
//!     tell the voices of each meeting's far end in SET apart, and print
//!     how much of what was named was named right, and how much of the
//!     speech was handled as it should be (apart.rs); fail when the set's
//!     are below RIGHT and HANDLED, percentages
//! call-transcripts names SPEAKER_MODEL SET [RIGHT NAMED]
//!     name the voices of each meeting's far end in SET from who was in
//!     as each spoke, its speakers made to join one at a time, and print
//!     how much of what was named was named right, and how much of the
//!     speech was given a name (naming.rs); fail when the set's are below
//!     RIGHT and NAMED, percentages
//! call-transcripts summarize SUMMARIZER_DIR TITLE TRANSCRIPT.md [NAME]
//!     print the summary of the call titled TITLE that a transcript
//!     writes down (summary.rs), the others on it called NAME if given,
//!     or that too little was said to sum up, with the model loaded here,
//!     minding none of serve's manners
//! ```
//!
//! How many calls there were is said to the realm's mycelium node as its
//! calls receptor, once a minute, with the calendar event of each call on
//! the declared account that a summary was written of, by its id alone
//! (receptor.rs).
//!
//! A call is what modules/calls counts as one: a program recording from a
//! microphone for two seconds, over once none has for five (follow.rs).
//! Its sound is kept only in memory until it's heard (hear.rs).
//!
//! It reads two of dictation's settings: `DICTATION_DEVICE`, and
//! `DICTATION_CACHE`, which must be a folder of its own, since a load
//! removes what another build compiled there. The
//! summarizer runs on `SUMMARIZER_DEVICE`, the GPU unless it's set, its
//! compiled model kept in `SUMMARIZER_CACHE`, a folder of its own for
//! the same reason. Its tokenizer needs OpenVINO's tokenizers library,
//! found by `OPENVINO_TOKENIZERS_PATH_GENAI`, as the service sets it.
//! Their side's voices are told apart with the speaker model at
//! `CALL_TRANSCRIPTS_VOICES` (voices.rs), and aren't when it's unset.

mod apart;
mod ask;
mod book;
mod filed;
mod follow;
mod hear;
mod meet;
mod meeting;
mod naming;
mod notices;
mod presence;
mod private;
mod receptor;
mod rings;
mod said;
mod score;
mod split;
mod summarizer;
mod summary;

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, mpsc};

use dictation::openvino::Priority;
use dictation::parakeet::Parakeet;
use dictation::takes::{Cutter, Take};
use dictation::transcript::{Line, Side, markdown};
use dictation::wav;
use terminal_kit::{Exit, Failure, Help};

use crate::ask::SHORT_OF_MEMORY;

fn help() -> Help {
    Help::new("call-transcripts", "writes down what's said on calls")
        .example("call-transcripts serve S ~/m ~/s ~/c", "what its unit runs")
        .example("call-transcripts hear ~/m a.wav", "what a.wav says")
        .subcommand(
            Help::new(
                "serve",
                "follows calls, writing each down and summing it up",
            )
            .argument("<socket>", "where the shell asks")
            .argument("<model-dir>", "the speech model's folder")
            .argument("[summarizer]", "the program answering as ask does")
            .argument("<calls>", "the folder each call is kept in")
            .example("call-transcripts serve S ~/m ~/s ~/c", "what its unit runs")
            .exit(Exit::Failed, "the model or the socket failed"),
        )
        .subcommand(
            Help::new("ask", "answers prompts a line each, as JSON")
                .argument("<summarizer-dir>", "the language model's folder")
                .example("call-transcripts ask ~/s", "what the summarizer runs")
                .exit(Exit::Failed, "the model failed, or a prompt isn't JSON")
                .exit(SHORT_OF_MEMORY, "too little memory to load it yet"),
        )
        .subcommand(
            Help::new("transcribe", "a call's two sides, as Markdown")
                .argument("<model-dir>", "the speech model's folder")
                .argument("<mine>", "Joel's side, a 16 kHz mono WAV")
                .argument("<theirs>", "the call's side, alike")
                .example(
                    "call-transcripts transcribe ~/m a.wav b.wav",
                    "who said what",
                )
                .exit(Exit::Failed, "the model failed or a file can't be read"),
        )
        .subcommand(
            Help::new("hear", "what each WAV says, heard whole, a line each")
                .argument("<model-dir>", "the speech model's folder")
                .argument("<clip>...", "16 kHz mono WAVs")
                .example("call-transcripts hear ~/m a.wav", "what a.wav says")
                .exit(Exit::Failed, "the model failed or a file can't be read"),
        )
        .subcommand(
            Help::new("score", "how many words of meetings it gets wrong")
                .argument("<model-dir>", "the speech model's folder")
                .argument("<set>", "the meetings' folder, as meetings.nix lays it out")
                .argument("[ceiling]", "the most percent wrong that passes")
                .example("call-transcripts score ~/m ~/set 19", "fails above 19%")
                .exit(
                    Exit::Failed,
                    "a file can't be read, or it's above the ceiling",
                ),
        )
        .subcommand(
            Help::new("voices", "how well it tells meetings' voices apart")
                .argument("<speaker-model>", "the speaker model's ONNX file")
                .argument("<set>", "the meetings' folder, as meetings.nix lays it out")
                .argument("[right]", "the least percent named right that passes")
                .argument(
                    "[handled]",
                    "the least percent of speech handled that passes",
                )
                .example(
                    "call-transcripts voices ~/t.onnx ~/set 90 70",
                    "fails below either",
                )
                .exit(Exit::Failed, "a file can't be read, or it's below a floor"),
        )
        .subcommand(
            Help::new(
                "names",
                "how well it names meetings' voices from who was in",
            )
            .argument("<speaker-model>", "the speaker model's ONNX file")
            .argument("<set>", "the meetings' folder, as meetings.nix lays it out")
            .argument("[right]", "the least percent named right that passes")
            .argument("[named]", "the least percent of speech named that passes")
            .example(
                "call-transcripts names ~/t.onnx ~/set 95 40",
                "fails below either",
            )
            .exit(Exit::Failed, "a file can't be read, or it's below a floor"),
        )
        .subcommand(
            Help::new("summarize", "the summary a transcript makes")
                .argument("<summarizer-dir>", "the language model's folder")
                .argument("<title>", "the call's title")
                .argument("<transcript>", "its transcript, as Markdown")
                .argument("[name]", "who the others on it are")
                .example("call-transcripts summarize ~/s Ada t.md", "its summary")
                .exit(Exit::Failed, "the model failed or a file can't be read"),
        )
        .exit(Exit::Failed, "a model, a file or the socket failed")
        .exit(
            SHORT_OF_MEMORY,
            "too little memory to load the summarizer yet",
        )
}

fn main() -> ExitCode {
    terminal_kit::main(&help(), env!("CARGO_PKG_VERSION"), |context| {
        act(&context.args)
    })
}

fn act(args: &[String]) -> Result<(), Failure> {
    let result = match args.iter().map(String::as_str).collect::<Vec<_>>()[..] {
        ["serve", socket, model, summarizer, calls] => serve(
            Path::new(socket),
            PathBuf::from(model),
            Some(PathBuf::from(summarizer)),
            Path::new(calls),
        ),
        ["serve", socket, model, calls] => serve(
            Path::new(socket),
            PathBuf::from(model),
            None,
            Path::new(calls),
        ),
        ["ask", summarizer] => return ask::serve(Path::new(summarizer)),
        ["summarize", summarizer, title, transcript] => {
            summarize(Path::new(summarizer), title, None, Path::new(transcript))
        }
        ["summarize", summarizer, title, transcript, them] => summarize(
            Path::new(summarizer),
            title,
            Some(them),
            Path::new(transcript),
        ),
        ["transcribe", model, mine, theirs] => {
            transcribe(Path::new(model), Path::new(mine), Path::new(theirs))
        }
        ["hear", model, ref clips @ ..] if !clips.is_empty() => hear_clips(Path::new(model), clips),
        ["score", model, set] => score::score(Path::new(model), Path::new(set), None),
        ["score", model, set, ceiling] => match score::ceiling(ceiling) {
            Some(most) => score::score(Path::new(model), Path::new(set), Some(most)),
            None => {
                return Err(Failure::new(
                    Exit::Usage,
                    "the ceiling is a percentage below 100, to one place at most",
                )
                .try_this(format!("call-transcripts score {model} {set} 19")));
            }
        },
        ["voices", model, set] => apart::apart(Path::new(model), Path::new(set), None),
        ["voices", model, set, right, handled] => {
            match (score::ceiling(right), score::ceiling(handled)) {
                (Some(right), Some(handled)) => {
                    apart::apart(Path::new(model), Path::new(set), Some((right, handled)))
                }
                _ => {
                    return Err(Failure::new(
                        Exit::Usage,
                        "each floor is a percentage below 100, to one place at most",
                    )
                    .try_this(format!("call-transcripts voices {model} {set} 90 70")));
                }
            }
        }
        ["names", model, set] => naming::names(Path::new(model), Path::new(set), None),
        ["names", model, set, right, named] => {
            match (score::ceiling(right), score::ceiling(named)) {
                (Some(right), Some(named)) => {
                    naming::names(Path::new(model), Path::new(set), Some((right, named)))
                }
                _ => {
                    return Err(Failure::new(
                        Exit::Usage,
                        "each floor is a percentage below 100, to one place at most",
                    )
                    .try_this(format!("call-transcripts names {model} {set} 95 40")));
                }
            }
        }
        // The kit has refused a word no subcommand answers to, so only a
        // subcommand given too few or too many words is left.
        [first, ..] => {
            return Err(Failure::new(Exit::Usage, "those aren't the words it takes")
                .try_this(format!("call-transcripts {first} --help")));
        }
        [] => {
            return Err(Failure::new(Exit::Usage, "it needs a subcommand")
                .try_this("call-transcripts --help"));
        }
    };
    result.map_err(|error| Failure::new(Exit::Failed, error))
}

/// Where Parakeet runs, as dictation's Whisper does, and the speaker
/// model their side's voices are told apart with, where there's one.
pub struct Settings {
    pub model: PathBuf,
    pub device: String,
    pub cache: Option<PathBuf>,
    pub voices: Option<PathBuf>,
}

impl Settings {
    fn read(model: PathBuf) -> Self {
        Self {
            model,
            device: std::env::var("DICTATION_DEVICE").unwrap_or_else(|_| "NPU".to_owned()),
            cache: std::env::var_os("DICTATION_CACHE").map(PathBuf::from),
            voices: std::env::var_os("CALL_TRANSCRIPTS_VOICES").map(PathBuf::from),
        }
    }

    /// Whether Parakeet is worth compiling before a call: on the NPU, when
    /// nothing is compiled for this build, so a call's first take waits
    /// seconds rather than a minute.
    pub fn worth_compiling(&self) -> bool {
        Parakeet::worth_compiling(
            &self.model,
            &self.device,
            self.cache.as_deref(),
            dictation::npu(),
        )
    }

    /// Parakeet, at the NPU's background priority.
    pub fn load(&self) -> Result<Parakeet, String> {
        Parakeet::load(
            &self.model,
            &self.device,
            self.cache.as_deref(),
            Priority::Background,
        )
    }
}

fn serve(
    socket: &Path,
    model: PathBuf,
    summarizer: Option<PathBuf>,
    calls: &Path,
) -> Result<(), String> {
    let settings = Settings::read(model);
    private::keep_private(calls);
    let book = Arc::new(book::Shared::new(book::Book::read(calls)));
    let listener = dictation::daemon::listen(socket)?;
    let calling = Arc::new(AtomicBool::new(false));
    let (jobs, heard) = mpsc::channel();
    let (sums, summing) = mpsc::channel();
    {
        let book = Arc::clone(&book);
        std::thread::spawn(move || hear::hear(&settings, &book, &heard, &sums, &own_name()));
    }
    {
        let summarizer = summarizer::Summarizer {
            program: summarizer,
            calling: Arc::clone(&calling),
            power: PathBuf::from("/sys/class/power_supply"),
            me: own_name(),
        };
        let (book, again) = (Arc::clone(&book), jobs.clone());
        std::thread::spawn(move || summarizer::run(&summarizer, &book, &summing, &again));
    }
    {
        let book = Arc::clone(&book);
        std::thread::spawn(move || dictation::daemon::answer_all(&book, &listener));
    }
    if let Some(receptor) = receptor::Receptor::from_env() {
        let book = Arc::clone(&book);
        std::thread::spawn(move || receptor::keep(&receptor, &book));
    }
    // A Meet names him by his whole name, so his first alone could be
    // someone else's; with none, a Meet's people are never told from his.
    let whole = whole_name().unwrap_or_default();
    if whole.is_empty() {
        eprintln!("call-transcripts: the account has no name, so no Meet names their side");
    }
    follow::follow(&book, calls, &jobs, &calling, &whole)
}

fn summarize(
    summarizer: &Path,
    title: &str,
    them: Option<&str>,
    transcript: &Path,
) -> Result<(), String> {
    let file = std::fs::read_to_string(transcript)
        .map_err(|e| format!("reading {}: {e}", transcript.display()))?;
    // A call's one file is summed up from its transcript alone, never
    // from a summary already over it.
    let text = filed::transcript(&file);
    if said::too_little(text) {
        println!("{}", said::TOO_LITTLE);
        return Ok(());
    }
    let mut model = ask::load(summarizer).map_err(|unloaded| match unloaded {
        ask::Unloaded::Short(why) | ask::Unloaded::Failed(why) => why,
    })?;
    // Joel's name, as the daemon wrote his lines under it, so a voice
    // named from the Meet that opens the call is never taken for him.
    let written = summary::summarize_as(
        &mut |prompt, tokens| {
            let answer = model.answer(prompt, tokens)?;
            // What each pass answered, to check a summary by.
            eprintln!("call-transcripts: asked, answered {answer:?}");
            Ok(answer)
        },
        title,
        Some(&own_name()),
        them,
        text,
    )?;
    println!("{written}");
    Ok(())
}

fn transcribe(model: &Path, mine: &Path, theirs: &Path) -> Result<(), String> {
    let mut speech = Settings::read(model.to_owned()).load()?;
    let mut lines = Vec::new();
    for (side, file) in [(Side::Mine, mine), (Side::Theirs, theirs)] {
        let bytes = std::fs::read(file).map_err(|e| format!("reading {}: {e}", file.display()))?;
        for take in takes(&wav::samples(&bytes)?) {
            let (text, took) = speech.transcribe(&take.samples)?;
            eprintln!("call-transcripts: {}", took.describe(&speech.device));
            if let Some(text) = said::spoken(&text) {
                lines.push(Line {
                    side,
                    start: take.start,
                    end: take.start + take.samples.len(),
                    text,
                    voice: None,
                    name: None,
                });
            }
        }
    }
    print!("{}", markdown(&lines, &own_name(), None, None));
    Ok(())
}

fn hear_clips(model: &Path, clips: &[&str]) -> Result<(), String> {
    let mut speech = Settings::read(model.to_owned()).load()?;
    for clip in clips {
        let bytes = std::fs::read(clip).map_err(|e| format!("reading {clip}: {e}"))?;
        let (text, took) = speech.transcribe(&wav::samples(&bytes)?)?;
        eprintln!(
            "call-transcripts: {clip}: {}",
            took.describe(&speech.device)
        );
        println!("{}", text.replace('\n', " "));
    }
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
    whole_name()
        .and_then(|name| name.split_whitespace().next().map(str::to_owned))
        .unwrap_or_else(|| "Me".to_owned())
}

/// The account's whole name as the system knows it, the part of its
/// entry's comment before any comma; or none where it has none.
fn whole_name() -> Option<String> {
    let user = std::env::var("USER").unwrap_or_default();
    std::fs::read_to_string("/etc/passwd")
        .ok()
        .and_then(|passwd| {
            passwd.lines().find_map(|entry| {
                let fields: Vec<&str> = entry.split(':').collect();
                (fields.first() == Some(&user.as_str()))
                    .then(|| {
                        fields
                            .get(4)?
                            .split(',')
                            .next()
                            .map(|name| name.trim().to_owned())
                    })
                    .flatten()
            })
        })
        .filter(|name| !name.is_empty())
}
