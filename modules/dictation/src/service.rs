//! The service: listens when told, types what was said, and tells anyone
//! watching what it's doing.
//!
//! It takes its socket from systemd, or binds `$XDG_RUNTIME_DIR/dictation.sock`
//! when run by hand. A client writes one line:
//!
//! - `toggle`: start listening, or stop and type; the answer is the state
//!   it's in after, one line.
//! - `watch`: the state now, then a line at each change, until the client
//!   goes: `idle`, `listening` or `transcribing`. Between them come words
//!   for what the shell says: `still-writing` for a press while Whisper
//!   works, and, before an `idle` that typed nothing, `heard-nothing`
//!   with why: `muted`, the microphone was muted as listening began;
//!   `short`, under a third of a second; `silent`, under -40 dBFS
//!   throughout; or `unclear`, Whisper found no words.
//!
//! One thread holds the state and hears every event, so a press is always
//! answered at once. The microphone is read by pw-record into memory on a
//! thread of its own, and Whisper runs on another, which loads the models
//! as listening starts and lets them go after ten minutes unused.
//!
//! A first load compiles the encoder for the NPU, which took 44 s on a
//! busy machine, and Joel's first words waited on it. So where there's an
//! NPU, the service loads the models once as it starts, at sign-in, which
//! fills OpenVINO's cache,
//! and lets them go again, since they hold 1.2 GiB: a load from the cache
//! takes about 1.5 s.

use std::io::{BufRead, BufReader, Read, Write};
use std::os::fd::FromRawFd;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

use crate::mel::{RATE, SAMPLES};
use crate::whisper::Whisper;

const UNUSED: Duration = Duration::from_mins(10);
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
    Toggle(UnixStream),
    Watch(UnixStream),
    Full(u64),
    Heard(Result<String, String>),
}

enum Job {
    Prime,
    Warm,
    Transcribe(Vec<f32>),
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

/// Runs the service with Whisper from `model`.
///
/// # Errors
///
/// When it can't take or bind its socket.
pub fn serve(model: &Path, device: String, cache: Option<PathBuf>) -> Result<(), String> {
    let listener = listener()?;
    let (events, heard) = mpsc::channel();
    let prime = device != "CPU" && accelerated();
    let jobs = worker(model.to_owned(), device, cache, events.clone());
    if prime {
        let _ = jobs.send(Job::Prime);
    }
    let coordinator = thread::spawn({
        let events = events.clone();
        move || coordinate(&heard, &events, &jobs)
    });
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
                "toggle" => Event::Toggle(stream),
                "watch" => Event::Watch(stream),
                _ => return,
            };
            let _ = events.send(event);
        });
    }
    drop(coordinator);
    Ok(())
}

/// Whether the machine has an NPU, which shows as an accel device. Only its
/// compile is slow enough to do ahead; without one the encoder compiles for
/// the CPU as quickly as it loads, so loading at sign-in would only take
/// memory for nothing.
fn accelerated() -> bool {
    std::fs::read_dir("/dev/accel").is_ok_and(|mut entries| entries.next().is_some())
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

fn coordinate(heard: &Receiver<Event>, events: &Sender<Event>, jobs: &Sender<Job>) {
    let mut state = State::Idle;
    let mut watchers: Vec<UnixStream> = Vec::new();
    let mut recording: Option<Recording> = None;
    let mut generation = 0_u64;
    for event in heard {
        let before = state;
        match event {
            Event::Watch(mut stream) => {
                if stream
                    .write_all(format!("{}\n", state.word()).as_bytes())
                    .is_ok()
                {
                    watchers.push(stream);
                }
            }
            Event::Toggle(mut stream) => {
                let said;
                (state, said) = press(state, &mut recording, &mut generation, events, jobs);
                if let Some(said) = said {
                    say(said, &mut watchers);
                }
                let _ = stream.write_all(format!("{}\n", state.word()).as_bytes());
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
                    Ok(text) if !text.is_empty() => {
                        if let Err(error) = type_out(&text) {
                            notice("Dictation can't type", &error);
                        }
                    }
                    Ok(_) => say(Said::Word(Nothing::Unclear.word()), &mut watchers),
                    Err(error) => notice("Dictation failed", &error),
                }
            }
        }
        if state != before {
            tell(state.word(), &mut watchers);
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

/// Starts pw-record on the default microphone, its samples read into
/// memory until it stops or Whisper's window is full.
fn listen(events: &Sender<Event>, generation: u64) -> Result<Recording, String> {
    let mut child = Command::new("pw-record")
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
        while let Ok(read) = stdout.read(&mut buffer) {
            if read == 0 {
                break;
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

/// The thread that holds Whisper: it loads the models when listening
/// starts, transcribes what it's given, and lets the models go once
/// they've gone unused for ten minutes.
fn worker(
    model: PathBuf,
    device: String,
    cache: Option<PathBuf>,
    events: Sender<Event>,
) -> Sender<Job> {
    let (jobs, work) = mpsc::channel::<Job>();
    thread::spawn(move || {
        let mut whisper: Option<Whisper> = None;
        let load = |whisper: &mut Option<Whisper>| -> Result<(), String> {
            if whisper.is_none() {
                *whisper = Some(Whisper::load(&model, &device, cache.as_deref())?);
            }
            Ok(())
        };
        loop {
            let job = match work.recv_timeout(UNUSED) {
                Ok(job) => job,
                Err(RecvTimeoutError::Timeout) => {
                    whisper = None;
                    match work.recv() {
                        Ok(job) => job,
                        Err(_) => return,
                    }
                }
                Err(RecvTimeoutError::Disconnected) => return,
            };
            match job {
                Job::Prime => {
                    if whisper.is_none() {
                        let started = Instant::now();
                        match Whisper::load(&model, &device, cache.as_deref()) {
                            Ok(_) => eprintln!(
                                "dictation: ready in {} ms",
                                started.elapsed().as_millis()
                            ),
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
                        let (text, timings) = loaded.transcribe(&audio)?;
                        eprintln!(
                            "dictation: {} ms heard; {}",
                            audio.len() * 1000 / RATE,
                            timings.describe(&loaded.device)
                        );
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
mod tests {
    use super::*;

    #[test]
    fn each_state_has_its_word() {
        assert_eq!(
            [State::Idle, State::Listening, State::Transcribing].map(State::word),
            ["idle", "listening", "transcribing"]
        );
    }

    #[test]
    fn a_press_while_writing_says_so_and_waits() {
        let (events, _heard) = mpsc::channel();
        let (jobs, work) = mpsc::channel();
        let mut generation = 3;
        let (state, said) = press(
            State::Transcribing,
            &mut None,
            &mut generation,
            &events,
            &jobs,
        );
        assert_eq!(state, State::Transcribing);
        assert_eq!(said, Some(Said::Word("still-writing")));
        assert_eq!(generation, 3);
        assert!(work.try_recv().is_err());
    }

    #[test]
    fn a_press_while_listening_stops_quietly() {
        let (events, _heard) = mpsc::channel();
        let (jobs, _work) = mpsc::channel();
        let mut generation = 3;
        let (state, said) = press(State::Listening, &mut None, &mut generation, &events, &jobs);
        assert_eq!(state, State::Idle);
        assert!(said.is_none());
        assert_eq!(generation, 4);
    }

    /// A recording that has ended, holding `samples` of one level.
    fn recorded(level: i16, samples: usize, muted: bool) -> Recording {
        Recording {
            child: Command::new("true").spawn().expect("true runs"),
            reader: thread::spawn(|| {}),
            samples: Arc::new(Mutex::new(vec![level; samples])),
            muted,
        }
    }

    #[test]
    fn each_reason_for_nothing_has_its_word() {
        assert_eq!(
            [
                Nothing::Muted,
                Nothing::Short,
                Nothing::Silent,
                Nothing::Unclear
            ]
            .map(Nothing::word),
            [
                "heard-nothing muted",
                "heard-nothing short",
                "heard-nothing silent",
                "heard-nothing unclear"
            ]
        );
    }

    #[test]
    fn silence_isnt_transcribed_and_says_so() {
        let (jobs, work) = mpsc::channel();
        let silent = recorded(12, RATE, false);
        assert_eq!(stop(Some(silent), &jobs), Err(Nothing::Silent));
        assert!(work.try_recv().is_err());
    }

    #[test]
    fn speech_is_handed_to_whisper() {
        let (jobs, work) = mpsc::channel();
        let speech = recorded(8_000, RATE, false);
        assert_eq!(stop(Some(speech), &jobs), Ok(State::Transcribing));
        assert!(matches!(work.try_recv(), Ok(Job::Transcribe(audio)) if audio.len() == RATE));
    }

    #[test]
    fn a_moment_isnt_transcribed_and_says_so() {
        let (jobs, _work) = mpsc::channel();
        let brief = recorded(8_000, RATE / 4, false);
        assert_eq!(stop(Some(brief), &jobs), Err(Nothing::Short));
    }

    #[test]
    fn a_muted_microphone_is_named_over_silence_or_a_moment() {
        let (jobs, _work) = mpsc::channel();
        assert_eq!(
            stop(Some(recorded(0, RATE, true)), &jobs),
            Err(Nothing::Muted)
        );
        assert_eq!(
            stop(Some(recorded(0, RATE / 4, true)), &jobs),
            Err(Nothing::Muted)
        );
    }

    #[test]
    fn speech_from_a_microphone_read_as_muted_is_still_typed() {
        assert_eq!(judge(&vec![0.25; RATE], true), None);
    }

    #[test]
    fn a_stop_that_typed_nothing_says_why_before_idle() {
        let (mut ours, theirs) = UnixStream::pair().expect("a socket pair");
        let mut watchers = vec![theirs];
        assert_eq!(stopped(Err(Nothing::Short), &mut watchers), State::Idle);
        tell(State::Idle.word(), &mut watchers);
        drop(watchers);
        let mut heard = String::new();
        ours.read_to_string(&mut heard).expect("what was said");
        assert_eq!(heard, "heard-nothing short\nidle\n");
    }
}
