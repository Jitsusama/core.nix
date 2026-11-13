//! The service: listens when told, types what was said, and tells anyone
//! watching what it's doing.
//!
//! It takes its socket from systemd, or binds `$XDG_RUNTIME_DIR/dictation.sock`
//! when run by hand. A client writes one line:
//!
//! - `toggle`: start listening, or stop and type; the answer is the state
//!   it's in after, one line.
//! - `hold`: the same, from the chord as it's pressed. When it starts
//!   listening and the chord is then held at least 400 ms, letting it go
//!   stops and types; a tap's release is no stop, so a tap still toggles.
//!   dictation-chord, which may read the keys, says when the chord is let
//!   go; without it, `hold` is `toggle`.
//! - `watch`: the state now, then a line at each change, until the client
//!   goes: `idle`, `listening` or `transcribing`, where `listening` waits
//!   for the microphone's first sound. Between them come words
//!   for what the shell says: `still-writing` for a press while Whisper
//!   works, and, before an `idle` that typed nothing, `heard-nothing`
//!   with why: `muted`, the microphone was muted as listening began;
//!   `short`, under a third of a second; `silent`, under -40 dBFS
//!   throughout; or `unclear`, Whisper found no words.
//!
//! One thread holds the state and hears every event, so a press is always
//! answered at once. The microphone, through its noise-cancelled twin, is
//! read by pw-record into memory on a thread of its own, and Whisper runs on another, which loads the models
//! as listening starts and lets them go after ten minutes unused.
//!
//! A first load compiles the encoder for the NPU, which took 44 s on a
//! busy machine, and Joel's first words waited on it. So where there's an
//! NPU, the service loads the models once as it starts, at sign-in, which
//! fills OpenVINO's cache,
//! and lets them go again, since they hold 1.2 GiB: a load from the cache
//! takes about 1.5 s.
//!
//! Where a tidying model is given, a take that lands as prose, pi's prompt
//! included, is tidied before it's typed (tidy.rs), on a thread of its own
//! that holds the model as Whisper's holds Whisper: compiled as the service
//! starts, loaded as listening starts, let go after ten minutes unused. A
//! take waits on its tidy 2 s at most, and then is typed as heard, the
//! tidy dropped when it comes, since Joel asked for no delayed voice. A
//! terminal's take is never tidied, since a command is typed as said.

use std::io::{BufRead, BufReader, Read, Write};
use std::os::fd::FromRawFd;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

use dictation::mel::{RATE, SAMPLES};
use dictation::microphone;
use dictation::openvino::Priority;
use dictation::speech::Speech;
use dictation::tidy::Tidier;

use crate::kept;
use crate::mode::{self, Mode};

const UNUSED: Duration = Duration::from_mins(10);
// How long a take waits on its tidy before it's typed as heard.
const TIDY_CAP: Duration = Duration::from_secs(2);
// A chord let go sooner than this was a tap, which leaves it listening.
const HELD_MS: u128 = 400;
// Under -40 dBFS throughout is silence, which Whisper would otherwise hear
// words in.
const SILENT: f32 = 0.01;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum State {
    Idle,
    Listening,
    Transcribing,
}

impl State {
    fn word(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Listening => "listening",
            Self::Transcribing => "transcribing",
        }
    }
}

/// Why a recording typed nothing.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Nothing {
    Muted,
    Short,
    Silent,
    Unclear,
}

impl Nothing {
    fn word(self) -> &'static str {
        match self {
            Self::Muted => "heard-nothing muted",
            Self::Short => "heard-nothing short",
            Self::Silent => "heard-nothing silent",
            Self::Unclear => "heard-nothing unclear",
        }
    }
}

/// What a press has to say beside the state: a word for the watchers, or,
/// for what can't happen, a notice.
#[derive(PartialEq, Eq, Debug)]
enum Said {
    Word(&'static str),
    Notice(&'static str, String),
}

const STILL_WRITING: &str = "still-writing";

enum Event {
    /// A press, and whether it came from the chord, which may be held.
    Press(UnixStream, bool),
    /// The chord let go, after this many ms held.
    Released(u128),
    Watch(UnixStream),
    /// The microphone's first sound for a listening, by its generation.
    Sounding(u64),
    Full(u64),
    Heard(Result<String, String>),
    /// The model loaded, in this many ms.
    Ready(u128),
    /// A take's tidy, by the take's number, or none where it may not be typed.
    Tidied(u64, Option<String>),
    /// A take's time to be tidied is up, by its number.
    Late(u64),
}

enum Job {
    Prime,
    Warm,
    Transcribe(Vec<f32>),
}

enum TidyJob {
    Prime,
    Warm,
    /// A take, by its number, to be tidied by when it's typed as heard.
    Tidy(String, u64, Instant),
}

/// The model that tidies prose before it's typed, and where it runs.
pub struct Tidying {
    pub model: PathBuf,
    pub device: String,
    pub cache: Option<PathBuf>,
}

struct Recording {
    child: Child,
    reader: thread::JoinHandle<()>,
    samples: Arc<Mutex<Vec<i16>>>,
    muted: bool,
}

/// The socket path a client and a hand-run service use.
#[must_use]
pub fn socket() -> PathBuf {
    std::env::var_os("XDG_RUNTIME_DIR")
        .map_or_else(|| PathBuf::from("/tmp"), PathBuf::from)
        .join("dictation.sock")
}

/// Runs the service with Whisper from `model`, and prose tidied as
/// `tidying` says, where it's given.
///
/// # Errors
///
/// When it can't take or bind its socket.
pub fn serve(
    model: &Path,
    device: String,
    cache: Option<PathBuf>,
    words: String,
    tidying: Option<Tidying>,
) -> Result<(), String> {
    let listener = listener()?;
    let (events, heard) = mpsc::channel();
    let prime = device != "CPU" && dictation::npu();
    let jobs = worker(model.to_owned(), device, cache, words, events.clone());
    if prime {
        let _ = jobs.send(Job::Prime);
    }
    // Compiling the tidier for the GPU takes seconds too, so it's done
    // ahead wherever there's one.
    let tidy = tidying.map(|tidying| tidier(tidying, events.clone()));
    if let Some(tidy) = &tidy {
        let _ = tidy.send(TidyJob::Prime);
    }
    thread::spawn(open_the_microphone);
    let coordinator = thread::spawn({
        let events = events.clone();
        move || coordinate(&heard, &events, &jobs, tidy.as_ref())
    });
    hear_the_chord(events.clone());
    for stream in listener.incoming() {
        let Ok(stream) = stream else { continue };
        let events = events.clone();
        thread::spawn(move || {
            let mut line = String::new();
            let mut reader = BufReader::new(&stream);
            if reader.read_line(&mut line).is_err() {
                return;
            }
            let event = match line.trim() {
                "toggle" => Event::Press(stream, false),
                "hold" => Event::Press(stream, true),
                "watch" => Event::Watch(stream),
                _ => return,
            };
            let _ = events.send(event);
        });
    }
    drop(coordinator);
    Ok(())
}

fn listener() -> Result<UnixListener, String> {
    let given = std::env::var("LISTEN_PID")
        .ok()
        .and_then(|pid| pid.parse::<u32>().ok())
        == Some(std::process::id())
        && std::env::var("LISTEN_FDS").as_deref() == Ok("1");
    if given {
        // SAFETY: systemd passed exactly one socket, as fd 3, to this
        // process, and nothing else owns it.
        return Ok(unsafe { UnixListener::from_raw_fd(3) });
    }
    let path = socket();
    let _ = std::fs::remove_file(&path);
    UnixListener::bind(&path).map_err(|e| format!("binding {}: {e}", path.display()))
}

/// Where dictation-chord says the chord was let go.
fn chord_socket() -> PathBuf {
    std::env::var_os("DICTATION_CHORD").map_or_else(
        || PathBuf::from("/run/dictation-chord/socket"),
        PathBuf::from,
    )
}

/// Hears dictation-chord on a thread of its own, and tries again every few
/// seconds while it's not there, as on a machine without it.
fn hear_the_chord(events: Sender<Event>) {
    thread::spawn(move || {
        loop {
            if let Ok(stream) = UnixStream::connect(chord_socket()) {
                for line in BufReader::new(stream).lines() {
                    let Ok(line) = line else { break };
                    let Some(ms) = line.strip_prefix("up ").and_then(|ms| ms.parse().ok()) else {
                        continue;
                    };
                    if events.send(Event::Released(ms)).is_err() {
                        return;
                    }
                }
            }
            thread::sleep(Duration::from_secs(5));
        }
    });
}

/// Whether letting go of the chord stops listening: only what a held
/// chord started, and only once it was held long enough not to be a tap.
fn let_go(state: State, held: Option<u64>, generation: u64, ms: u128) -> bool {
    state == State::Listening && held == Some(generation) && ms >= HELD_MS
}

/// The state the watchers are told. Listening starts when the microphone's
/// first sound arrives rather than at the press: an idle microphone took
/// 0.7 s to start, and a shell that said it was listening meanwhile had
/// its user speak into nothing.
fn shown(state: State, sounding: bool) -> State {
    if state == State::Listening && !sounding {
        State::Idle
    } else {
        state
    }
}

/// The words a take's tidy settles on, once, for the take that waits:
/// the tidy, or the take as heard when there's none, or when its time ran
/// out first. Whatever comes for a take already typed is dropped.
fn settle(waiting: &mut Option<(u64, String)>, take: u64, tidy: Option<String>) -> Option<String> {
    let (number, _) = waiting.as_ref()?;
    if *number != take {
        return None;
    }
    let (_, heard) = waiting.take()?;
    Some(tidy.unwrap_or(heard))
}

/// Says a take's time to be tidied is up once it is.
fn time_up(events: &Sender<Event>, take: u64) {
    let events = events.clone();
    thread::spawn(move || {
        thread::sleep(TIDY_CAP);
        let _ = events.send(Event::Late(take));
    });
}

/// Types a take as the window it lands in takes it, and keeps when, in
/// `file` and `last`, and tells the watchers, for the launcher's row.
fn write(text: &str, file: Option<&Path>, last: &mut Option<u64>, watchers: &mut Vec<UnixStream>) {
    match type_out(&mode::shaped(text, mode::focused())) {
        Ok(()) => {
            let at = kept::now();
            *last = Some(at);
            if let Some(Err(error)) = file.map(|f| kept::keep(f, at)) {
                eprintln!("dictation: {error}");
            }
            tell(&kept::last_take(at), watchers);
        }
        Err(error) => notice("Dictation can't type", &error),
    }
}

fn coordinate(
    heard: &Receiver<Event>,
    events: &Sender<Event>,
    jobs: &Sender<Job>,
    tidy: Option<&Sender<TidyJob>>,
) {
    let mut state = State::Idle;
    // The take waiting on its tidy, by its number, as heard.
    let mut waiting: Option<(u64, String)> = None;
    let mut takes = 0_u64;
    // Whether the current listening has heard the microphone yet.
    let mut sounding = false;
    let mut watchers: Vec<UnixStream> = Vec::new();
    let mut recording: Option<Recording> = None;
    let mut generation = 0_u64;
    // The listening a held chord started, by its generation.
    let mut held: Option<u64> = None;
    // What's said beside the state, for the launcher's row: when a take
    // last typed, kept across restarts, and how long the model last took
    // to load.
    let file = kept::here();
    let mut last = file.as_deref().and_then(kept::read);
    let mut ready: Option<u128> = None;
    for event in heard {
        let before = state;
        let told = shown(state, sounding);
        match event {
            Event::Watch(mut stream) => {
                let mut said = format!("{}\n", told.word());
                said.extend(ready.map(|ms| format!("{}\n", kept::ready(ms))));
                said.extend(last.map(|at| format!("{}\n", kept::last_take(at))));
                if stream.write_all(said.as_bytes()).is_ok() {
                    watchers.push(stream);
                }
            }
            Event::Press(mut stream, chord) => {
                let said;
                (state, said) = press(state, &mut recording, &mut generation, events, jobs);
                if before == State::Idle && state == State::Listening {
                    sounding = false;
                    if chord {
                        held = Some(generation);
                    }
                    if let Some(tidy) = tidy {
                        let _ = tidy.send(TidyJob::Warm);
                    }
                }
                if let Some(said) = said {
                    say(said, &mut watchers);
                }
                let _ = stream.write_all(format!("{}\n", state.word()).as_bytes());
            }
            Event::Released(ms) => {
                if let_go(state, held, generation, ms) {
                    generation += 1;
                    held = None;
                    state = stopped(stop(recording.take(), jobs), &mut watchers);
                }
            }
            Event::Sounding(at) => {
                if state == State::Listening && at == generation {
                    sounding = true;
                }
            }
            Event::Full(at) => {
                if state == State::Listening && at == generation {
                    generation += 1;
                    state = stopped(stop(recording.take(), jobs), &mut watchers);
                }
            }
            Event::Heard(result) => {
                state = State::Idle;
                match result {
                    Ok(text) if !text.is_empty() => match tidy {
                        // Still writing while it's tidied, so a press
                        // meanwhile is told so.
                        Some(tidy) if mode::focused() == Mode::Prose => {
                            state = State::Transcribing;
                            takes += 1;
                            waiting = Some((takes, text.clone()));
                            let _ =
                                tidy.send(TidyJob::Tidy(text, takes, Instant::now() + TIDY_CAP));
                            time_up(events, takes);
                        }
                        _ => write(&text, file.as_deref(), &mut last, &mut watchers),
                    },
                    Ok(_) => say(Said::Word(Nothing::Unclear.word()), &mut watchers),
                    Err(error) => notice("Dictation failed", &error),
                }
            }
            Event::Ready(ms) => {
                ready = Some(ms);
                tell(&kept::ready(ms), &mut watchers);
            }
            Event::Tidied(take, tidied) => {
                if let Some(text) = settle(&mut waiting, take, tidied) {
                    state = State::Idle;
                    write(&text, file.as_deref(), &mut last, &mut watchers);
                }
            }
            Event::Late(take) => {
                if let Some(text) = settle(&mut waiting, take, None) {
                    eprintln!("dictation: the tidy took over 2 s, so it's typed as heard");
                    state = State::Idle;
                    write(&text, file.as_deref(), &mut last, &mut watchers);
                }
            }
        }
        let now = shown(state, sounding);
        if now != told {
            tell(now.word(), &mut watchers);
        }
    }
}

/// What a press does in each state: start listening, stop and hand what
/// was heard to Whisper, or, while Whisper works, say so rather than
/// nothing, since it can't listen again until what it heard is typed. The
/// shell shows what the watchers' stream says, so a notice is only for
/// what can't happen.
fn press(
    state: State,
    recording: &mut Option<Recording>,
    generation: &mut u64,
    events: &Sender<Event>,
    jobs: &Sender<Job>,
) -> (State, Option<Said>) {
    match state {
        State::Idle => match listen(events, *generation) {
            Ok(started) => {
                *recording = Some(started);
                let _ = jobs.send(Job::Warm);
                (State::Listening, None)
            }
            Err(error) => (
                State::Idle,
                Some(Said::Notice("Dictation can't listen", error)),
            ),
        },
        State::Listening => {
            *generation += 1;
            match stop(recording.take(), jobs) {
                Ok(state) => (state, None),
                Err(nothing) => (State::Idle, Some(Said::Word(nothing.word()))),
            }
        }
        State::Transcribing => (State::Transcribing, Some(Said::Word(STILL_WRITING))),
    }
}

/// The state a stop leaves, after telling the watchers why it typed
/// nothing when it didn't.
fn stopped(stop: Result<State, Nothing>, watchers: &mut Vec<UnixStream>) -> State {
    stop.unwrap_or_else(|nothing| {
        say(Said::Word(nothing.word()), watchers);
        State::Idle
    })
}

/// Says a word to the watchers, or shows a notice. With nobody watching,
/// "still writing" is a notice too, so a press is never answered with
/// nothing.
fn say(said: Said, watchers: &mut Vec<UnixStream>) {
    match said {
        Said::Word(STILL_WRITING) if watchers.is_empty() => notice(
            "Dictation is still writing",
            "It types what you said as soon as it's done; press again then.",
        ),
        Said::Word(word) => tell(word, watchers),
        Said::Notice(summary, body) => notice(summary, &body),
    }
}

/// Tells each watcher a line, leaving out any that's gone.
fn tell(word: &str, watchers: &mut Vec<UnixStream>) {
    let line = format!("{word}\n");
    watchers.retain_mut(|watcher| watcher.write_all(line.as_bytes()).is_ok());
}

/// Whether the default microphone is muted, as `wpctl` says. When it
/// can't be asked, it's taken as not.
fn muted() -> bool {
    Command::new("wpctl")
        .args(["get-volume", "@DEFAULT_AUDIO_SOURCE@"])
        .stderr(Stdio::null())
        .output()
        .is_ok_and(|out| String::from_utf8_lossy(&out.stdout).contains("[MUTED]"))
}

/// Opens the default microphone for its first sound and lets it go, so a
/// source the sound server keeps running once opened is running before the
/// first press: starting the laptop's took 0.7 s, which cut off the start
/// of what was said. What it hears is thrown away.
fn open_the_microphone() {
    let Ok(mut child) = Command::new("pw-record")
        .args(target())
        .args(["--raw", "--rate", "16000", "--channels", "1", "-"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
    else {
        return;
    };
    if let Some(mut stdout) = child.stdout.take() {
        let _ = stdout.read(&mut [0_u8; 2]);
    }
    let _ = child.kill();
    let _ = child.wait();
}

/// What pw-record is pointed at: the default microphone's noise-cancelled
/// twin, so dictation hears what a call hears, cleaned as Joel chose by
/// ear, rather than the room. Asking PipeWire takes about 20 ms, inside
/// the press. Where there's no twin it's the microphone, and where
/// PipeWire can't be asked, nothing, which leaves it the sound server's.
fn target() -> Vec<String> {
    let dump = Command::new("pw-dump")
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()
        .and_then(|out| serde_json::from_slice(&out.stdout).ok());
    dump.as_ref()
        .and_then(microphone::to_record)
        .map(|node| vec!["--target".to_owned(), node])
        .unwrap_or_default()
}

/// Starts pw-record on the default microphone's twin, its samples read
/// into memory until it stops or Whisper's window is full.
fn listen(events: &Sender<Event>, generation: u64) -> Result<Recording, String> {
    let mut child = Command::new("pw-record")
        .args(target())
        .args([
            "--raw",
            "--rate",
            "16000",
            "--channels",
            "1",
            "--format",
            "s16",
            "-",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("starting pw-record: {e}"))?;
    let muted = muted();
    let mut stdout = child.stdout.take().ok_or("pw-record has no output")?;
    let samples = Arc::new(Mutex::new(Vec::with_capacity(SAMPLES)));
    let kept = Arc::clone(&samples);
    let events = events.clone();
    let reader = thread::spawn(move || {
        let mut buffer = [0_u8; 3200];
        let mut odd: Option<u8> = None;
        let mut sounding = false;
        while let Ok(read) = stdout.read(&mut buffer) {
            if read == 0 {
                break;
            }
            if !sounding {
                sounding = true;
                let _ = events.send(Event::Sounding(generation));
            }
            let mut bytes: Vec<u8> = odd.take().into_iter().collect();
            bytes.extend_from_slice(&buffer[..read]);
            if bytes.len() % 2 == 1 {
                odd = bytes.pop();
            }
            let mut held = kept.lock().unwrap_or_else(PoisonError::into_inner);
            let room = SAMPLES - held.len();
            held.extend(
                bytes
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .take(room)
                    .map(|&pair| i16::from_le_bytes(pair)),
            );
            if held.len() >= SAMPLES {
                let _ = events.send(Event::Full(generation));
                break;
            }
        }
    });
    Ok(Recording {
        child,
        reader,
        samples,
        muted,
    })
}

/// Why audio can't be transcribed, if it can't: a muted microphone gives
/// silence, and saying so is the more useful of the two.
fn judge(audio: &[f32], muted: bool) -> Option<Nothing> {
    let short = audio.len() < RATE / 3;
    let silent = audio.iter().all(|s| s.abs() < SILENT);
    match (short || silent, muted, short) {
        (false, _, _) => None,
        (true, true, _) => Some(Nothing::Muted),
        (true, false, true) => Some(Nothing::Short),
        (true, false, false) => Some(Nothing::Silent),
    }
}

/// Stops listening and hands what was heard to Whisper, unless it was
/// silence or less than a third of a second, which it says.
fn stop(recording: Option<Recording>, jobs: &Sender<Job>) -> Result<State, Nothing> {
    let Some(mut recording) = recording else {
        return Ok(State::Idle);
    };
    let _ = recording.child.kill();
    let _ = recording.child.wait();
    let _ = recording.reader.join();
    let samples = std::mem::take(
        &mut *recording
            .samples
            .lock()
            .unwrap_or_else(PoisonError::into_inner),
    );
    let audio: Vec<f32> = samples.iter().map(|&s| f32::from(s) / 32_768.0).collect();
    if let Some(nothing) = judge(&audio, recording.muted) {
        return Err(nothing);
    }
    if jobs.send(Job::Transcribe(audio)).is_err() {
        return Ok(State::Idle);
    }
    Ok(State::Transcribing)
}

/// The thread that holds Whisper: it loads and warms the models when
/// listening starts, so both are done while what's said is heard,
/// transcribes what it's given, and lets the models go once they've gone
/// unused for ten minutes.
fn worker(
    model: PathBuf,
    device: String,
    cache: Option<PathBuf>,
    words: String,
    events: Sender<Event>,
) -> Sender<Job> {
    let (jobs, work) = mpsc::channel::<Job>();
    thread::spawn(move || {
        let mut whisper: Option<Speech> = None;
        let load = |whisper: &mut Option<Speech>| -> Result<(), String> {
            if whisper.is_none() {
                let started = Instant::now();
                let mut loaded =
                    Speech::load(&model, &device, cache.as_deref(), &words, Priority::Waited)?;
                loaded.warm()?;
                let ms = started.elapsed().as_millis();
                eprintln!("dictation: loaded and warmed in {ms} ms");
                let _ = events.send(Event::Ready(ms));
                *whisper = Some(loaded);
            }
            Ok(())
        };
        while let Some(job) = next(&work, &mut whisper) {
            match job {
                Job::Prime => {
                    if whisper.is_none() {
                        let started = Instant::now();
                        match Speech::load(
                            &model,
                            &device,
                            cache.as_deref(),
                            &words,
                            Priority::Waited,
                        ) {
                            Ok(_) => {
                                let ms = started.elapsed().as_millis();
                                eprintln!("dictation: ready in {ms} ms");
                                let _ = events.send(Event::Ready(ms));
                            }
                            Err(error) => eprintln!("dictation: {error}"),
                        }
                    }
                }
                Job::Warm => {
                    if let Err(error) = load(&mut whisper) {
                        eprintln!("dictation: {error}");
                    }
                }
                Job::Transcribe(audio) => {
                    let result = load(&mut whisper).and_then(|()| {
                        let loaded = whisper.as_mut().ok_or("Whisper isn't loaded")?;
                        let (text, took) = loaded.transcribe(&audio)?;
                        eprintln!("dictation: {} ms heard; {took}", audio.len() * 1000 / RATE);
                        Ok(text)
                    });
                    if events.send(Event::Heard(result)).is_err() {
                        return;
                    }
                }
            }
        }
    });
    jobs
}

/// The next job a worker is given, letting what it holds go once it's
/// waited ten minutes for one, or none once nothing can give it one.
fn next<J, M>(work: &Receiver<J>, held: &mut Option<M>) -> Option<J> {
    match work.recv_timeout(UNUSED) {
        Ok(job) => Some(job),
        Err(RecvTimeoutError::Timeout) => {
            *held = None;
            work.recv().ok()
        }
        Err(RecvTimeoutError::Disconnected) => None,
    }
}

/// The thread that holds the tidying model as Whisper's holds Whisper,
/// and tidies each take it's given by its time, or says it can't. It
/// says how long each took, never what was said.
fn tidier(tidying: Tidying, events: Sender<Event>) -> Sender<TidyJob> {
    let (jobs, work) = mpsc::channel::<TidyJob>();
    thread::spawn(move || {
        let mut tidier: Option<Tidier> = None;
        let fresh = || {
            Tidier::load(
                &tidying.model,
                &tidying.device,
                tidying.cache.as_deref(),
                Priority::Waited,
            )
        };
        let load = |tidier: &mut Option<Tidier>| -> Result<(), String> {
            if tidier.is_none() {
                let started = Instant::now();
                let mut loaded = fresh()?;
                loaded.warm()?;
                eprintln!(
                    "dictation: the tidier loaded and warmed in {} ms",
                    started.elapsed().as_millis()
                );
                *tidier = Some(loaded);
            }
            Ok(())
        };
        while let Some(job) = next(&work, &mut tidier) {
            match job {
                TidyJob::Prime => {
                    if tidier.is_none() {
                        let started = Instant::now();
                        match fresh() {
                            Ok(_) => eprintln!(
                                "dictation: the tidier is ready in {} ms",
                                started.elapsed().as_millis()
                            ),
                            Err(error) => eprintln!("dictation: {error}"),
                        }
                    }
                }
                TidyJob::Warm => {
                    if let Err(error) = load(&mut tidier) {
                        eprintln!("dictation: {error}");
                    }
                }
                TidyJob::Tidy(take, number, by) => {
                    let started = Instant::now();
                    // A take whose time ran out while another was tidied
                    // is typed already.
                    let written = if started < by {
                        load(&mut tidier).and_then(|()| {
                            tidier
                                .as_mut()
                                .ok_or("the tidier isn't loaded")?
                                .tidy(&take, by)
                        })
                    } else {
                        Err("its time ran out before it was begun".to_owned())
                    };
                    let took = started.elapsed().as_millis();
                    match &written {
                        Ok(_) => eprintln!("dictation: tidied in {took} ms"),
                        Err(why) => eprintln!("dictation: typed as heard after {took} ms: {why}"),
                    }
                    if events.send(Event::Tidied(number, written.ok())).is_err() {
                        return;
                    }
                }
            }
        }
    });
    jobs
}

/// Types `text` into the focused window through the compositor's virtual
/// keyboard.
fn type_out(text: &str) -> Result<(), String> {
    let mut child = Command::new("wtype")
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .spawn()
        .map_err(|e| format!("starting wtype: {e}"))?;
    child
        .stdin
        .take()
        .ok_or("wtype has no input")?
        .write_all(text.as_bytes())
        .map_err(|e| format!("writing to wtype: {e}"))?;
    let status = child.wait().map_err(|e| format!("waiting on wtype: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("wtype exited {status}"))
    }
}

fn notice(summary: &str, body: &str) {
    let _ = Command::new("notify-send")
        .args(["--app-name=Dictation", "--expire-time=2000", summary, body])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .and_then(|mut child| child.wait());
}

/// Sends `line` to the service and copies what it answers to standard
/// output: one line for a toggle, every line for a watch.
///
/// # Errors
///
/// When the service can't be reached.
pub fn ask(line: &str) -> Result<(), String> {
    let path = socket();
    let mut stream =
        UnixStream::connect(&path).map_err(|e| format!("reaching {}: {e}", path.display()))?;
    stream
        .write_all(format!("{line}\n").as_bytes())
        .map_err(|e| format!("asking the service: {e}"))?;
    let mut out = std::io::stdout().lock();
    for answer in BufReader::new(stream).lines() {
        let answer = answer.map_err(|e| format!("reading the answer: {e}"))?;
        writeln!(out, "{answer}").map_err(|e| format!("writing the answer: {e}"))?;
        out.flush()
            .map_err(|e| format!("writing the answer: {e}"))?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "service_tests.rs"]
mod tests;
