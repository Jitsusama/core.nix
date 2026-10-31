//! Follows calls as they come and go in PipeWire's graph, as
//! pause-for-calls does, and records each side of one while it's on.
//!
//! Each side is a pw-record of its own, made unlinked, and linked here by
//! hand: Joel's to the ports the call app records from, so it hears the
//! microphone the app chose, and the others' to the output ports of what
//! the call app plays, so a notice or music played by another program
//! isn't heard as theirs. The links are kept as the graph changes, the app
//! opening a stream or Joel choosing another microphone. Each recorder's
//! sound is cut into takes as it comes (takes.rs), which go to be heard
//! (hear.rs); nothing is written to disk but the transcript.
//!
//! A call rejoined, the same meeting started again, goes on in the same
//! record and transcript, its takes timed from the first start.
//!
//! As a call starts, a notice offers the line for its chat (notices.rs),
//! and its Stop Transcribing stops both recorders at once, keeping what's
//! written. A call stopped isn't transcribed again if it's rejoined, since
//! stopping is how Joel honours someone's asking him not to.

use std::collections::{HashMap, HashSet};
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::thread;
use std::time::{Duration, Instant};

use calls::{Call, Change};
use dictation::graph::{Graph, Link};
use dictation::mel::RATE;
use dictation::takes::Cutter;
use dictation::transcript::Side;
use serde_json::Value;

use crate::book::{Record, Shared, now};
use crate::hear::Job;
use crate::meeting::{self, Meeting};
use crate::notices;

/// How long a link asked for may take to show before it's asked again.
const LINKING: Duration = Duration::from_secs(2);

/// A gap in a recorder's sound longer than this is filled with silence,
/// so its takes keep the call's time while nothing is linked to it.
const GAP: usize = RATE / 2;

/// The call being recorded: its record's key, and its two recorders.
struct Recording {
    key: String,
    mine: Recorder,
    theirs: Recorder,
    /// Links asked for, and when, until they show.
    asked: HashMap<(u64, u64), Instant>,
}

/// What the loop following calls is given: the graph as it changes, or
/// Joel's Stop Transcribing for a call.
enum Heard {
    Graph(Value),
    Stop(String),
}

/// Follows calls until PipeWire goes, for systemd to start it again.
pub fn follow(book: &Arc<Shared>, calls: &Path, jobs: &Sender<Job>) -> Result<(), String> {
    let mut dump = Command::new("pw-dump")
        .args(["--monitor", "--no-colors"])
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|why| format!("pw-dump wouldn't start: {why}"))?;
    let out = dump.stdout.take().ok_or("pw-dump has no output")?;
    let (arrays, heard) = mpsc::channel();
    let stops = arrays.clone();
    thread::spawn(move || {
        let read = serde_json::Deserializer::from_reader(BufReader::new(out)).into_iter::<Value>();
        for array in read {
            let Ok(array) = array else { break };
            if arrays.send(Heard::Graph(array)).is_err() {
                break;
            }
        }
    });
    let runtime = std::env::var_os("XDG_RUNTIME_DIR").map_or_else(PathBuf::new, PathBuf::from);
    let mut graph = Graph::default();
    let mut call = Call::Off;
    let mut recording: Option<Recording> = None;
    // The call on, by its record's key, recorded or not.
    let mut on_call: Option<String> = None;
    // The calls Joel stopped, never transcribed again while this runs.
    let mut stopped: HashSet<String> = HashSet::new();
    let mut count = 0;
    loop {
        let wait = match (call.due(Instant::now()), &recording) {
            (Some(due), _) => due,
            (None, Some(_)) => LINKING,
            (None, None) => Duration::from_secs(3600),
        };
        match heard.recv_timeout(wait) {
            Ok(Heard::Graph(array)) => graph.take(&array),
            Ok(Heard::Stop(key)) => {
                if let Some(mut stopping) = recording.take_if(|now| now.key == key) {
                    stopping.stop();
                    eprintln!("call-transcripts: transcribing stopped, {key}");
                    stopped.insert(key);
                }
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }
        match call.step(graph.call().is_some(), Instant::now()) {
            Some(Change::Started) => {
                count += 1;
                let at = now();
                let on = meeting::on(&runtime, at);
                let key = Record::key_for(on.event.as_deref(), on.person.as_deref(), at);
                on_call = Some(key.clone());
                if stopped.contains(&key) {
                    eprintln!("call-transcripts: a call stopped is rejoined, {key}");
                    continue;
                }
                let title = on.title.clone();
                match start(book, calls, &key, on, at, jobs, count) {
                    Ok(started) => {
                        offer_line(&title, &key, &stops);
                        recording = Some(started);
                    }
                    Err(why) => {
                        eprintln!("call-transcripts: couldn't record the call: {why}");
                        let why = format!("It couldn't be recorded: {why}");
                        notices::show(&notices::not_transcribed(&title, &why), |_| {});
                    }
                }
            }
            Some(Change::Ended) => {
                if let Some(mut ended) = recording.take() {
                    ended.stop();
                }
                if let Some(key) = on_call.take() {
                    ended_at(book, &key);
                }
            }
            None => {}
        }
        if let (Some(recording), Some(app)) = (&mut recording, graph.call()) {
            link(&graph, recording, app);
        }
    }
    let _ = dump.wait();
    Err("pw-dump ended".to_owned())
}

/// Offers the line for the call's chat, quietly, as the call starts:
/// clicking the card copies it, and Stop Transcribing stops the call's
/// recorders.
fn offer_line(title: &str, key: &str, stops: &Sender<Heard>) {
    let (key, stops) = (key.to_owned(), stops.clone());
    notices::show(
        &notices::being_transcribed(title),
        move |chosen| match chosen {
            "stop" => {
                let _ = stops.send(Heard::Stop(key));
            }
            "copy" | "default" => notices::copy_line(),
            _ => {}
        },
    );
}

/// Starts recording a call, in a record of its own, or in the record of the
/// same meeting when it's rejoined.
fn start(
    book: &Shared,
    calls: &Path,
    key: &str,
    on: Meeting,
    at: u64,
    jobs: &Sender<Job>,
    count: u32,
) -> Result<Recording, String> {
    let key = key.to_owned();
    let mut record = book.lock().record(&key).cloned().unwrap_or_else(|| Record {
        key: key.clone(),
        folder: calls.join(meeting::folder(at, &on.title)),
        title: on.title,
        started: at,
        ended: None,
        event: on.event,
        person: on.person,
        transcript: None,
        summary: None,
        transcribing: true,
    });
    record.ended = None;
    record.transcribing = true;
    let since = usize::try_from((at - record.started) / 1000).unwrap_or(0) * RATE;
    book.lock().put(record.clone());
    let recorders = Recorder::start(
        &format!("call-transcripts-mine-{count}"),
        &key,
        Side::Mine,
        since,
        jobs,
    )
    .and_then(|mut mine| {
        match Recorder::start(
            &format!("call-transcripts-theirs-{count}"),
            &key,
            Side::Theirs,
            since,
            jobs,
        ) {
            Ok(theirs) => Ok((mine, theirs)),
            // Joel's side alone would be half a call, heard as all of it.
            Err(why) => {
                mine.stop();
                Err(why)
            }
        }
    });
    let (mine, theirs) = match recorders {
        Ok(both) => both,
        Err(why) => {
            book.lock().put(Record {
                transcribing: false,
                ..record
            });
            return Err(why);
        }
    };
    eprintln!("call-transcripts: a call started, {key}");
    Ok(Recording {
        key,
        mine,
        theirs,
        asked: HashMap::new(),
    })
}

impl Recording {
    /// Stops both recorders; what's heard after ends the transcript
    /// (hear.rs).
    fn stop(&mut self) {
        self.mine.stop();
        self.theirs.stop();
    }
}

/// Says in a call's record that it ended, now.
fn ended_at(book: &Shared, key: &str) {
    let mut book = book.lock();
    if let Some(record) = book.record(key).cloned() {
        book.put(Record {
            ended: Some(now()),
            ..record
        });
    }
    eprintln!("call-transcripts: the call ended, {key}");
}

/// Keeps each recorder linked to its side, and to nothing else.
fn link(graph: &Graph, recording: &mut Recording, app: u64) {
    let sides = [
        (graph.named(&recording.mine.name), graph.feeding(app)),
        (graph.named(&recording.theirs.name), graph.playing(app)),
    ];
    let links = graph.links();
    let mut wanted = HashSet::new();
    for (node, from) in sides {
        let Some(node) = node else { continue };
        let inputs = graph.ports(node, "input");
        wanted.extend(
            from.iter()
                .flat_map(|output| inputs.iter().map(|input| (*output, *input))),
        );
        for link in links.iter().filter(|link| inputs.contains(&link.input)) {
            if !wanted.contains(&(link.output, link.input)) {
                pw_link(&["-d", &link.id.to_string()]);
            }
        }
    }
    let have: HashSet<(u64, u64)> = links
        .iter()
        .map(|&Link { output, input, .. }| (output, input))
        .collect();
    recording
        .asked
        .retain(|pair, when| !have.contains(pair) && when.elapsed() < LINKING);
    for pair in wanted.difference(&have) {
        if !recording.asked.contains_key(pair) {
            pw_link(&[&pair.0.to_string(), &pair.1.to_string()]);
            recording.asked.insert(*pair, Instant::now());
        }
    }
}

/// What pw-link says when what it was given is no longer in the graph,
/// strerror's words for ENOENT.
const GONE: &str = "No such file or directory";

fn pw_link(args: &[&str]) {
    match Command::new("pw-link")
        .args(args)
        .stderr(Stdio::piped())
        .output()
    {
        Ok(done) if done.status.success() => {}
        // A port or link gone before pw-link reached it is the graph moving on, as a
        // player ends; the change that removed it is read next and asked again.
        Ok(done) if String::from_utf8_lossy(&done.stderr).contains(GONE) => {}
        Ok(done) => eprintln!(
            "call-transcripts: pw-link {}: {}",
            args.join(" "),
            String::from_utf8_lossy(&done.stderr).trim()
        ),
        Err(why) => eprintln!("call-transcripts: pw-link wouldn't start: {why}"),
    }
}

/// One side's recorder: a pw-record made unlinked, its node named so it's
/// found in the graph, its sound read on a thread of its own.
struct Recorder {
    name: String,
    child: Child,
}

impl Recorder {
    fn start(
        name: &str,
        key: &str,
        side: Side,
        since: usize,
        jobs: &Sender<Job>,
    ) -> Result<Self, String> {
        let mut child = Command::new("pw-record")
            .args([
                "--target",
                "0",
                "--raw",
                "--rate",
                "16000",
                "--channels",
                "1",
            ])
            .args(["--format", "f32", "-P"])
            .arg(format!(
                "{{ node.name = \"{name}\" node.description = \"Call transcript\" }}"
            ))
            .arg("-")
            .stdout(Stdio::piped())
            .spawn()
            .map_err(|why| format!("pw-record wouldn't start: {why}"))?;
        let out = child.stdout.take().ok_or("pw-record has no output")?;
        hold_more(&out);
        let (key, jobs) = (key.to_owned(), jobs.clone());
        thread::spawn(move || hear_side(out, &key, side, since, &jobs));
        Ok(Self {
            name: name.to_owned(),
            child,
        })
    }

    fn stop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// How much a recorder's pipe holds: 1 MiB, about 16 s of its sound, the
/// most an account may ask for.
const PIPE_BYTES: i32 = 1 << 20;

/// `fcntl`'s ask to resize a pipe, Linux's own.
const F_SETPIPE_SZ: i32 = 1031;

unsafe extern "C" {
    fn fcntl(fd: i32, cmd: i32, ...) -> i32;
}

/// Lets a recorder's pipe hold about 16 s rather than its default of
/// one, since this runs in background.slice, behind everything Joel uses:
/// on a loaded machine the reader can wait seconds for its turn, and a
/// full pipe would lose the call's sound.
fn hold_more(out: &std::process::ChildStdout) {
    use std::os::fd::AsRawFd;
    // SAFETY: the descriptor is the pipe's, open while `out` lives.
    if unsafe { fcntl(out.as_raw_fd(), F_SETPIPE_SZ, PIPE_BYTES) } < 0 {
        let why = std::io::Error::last_os_error();
        eprintln!("call-transcripts: the recorder's pipe stays small: {why}");
    }
}

/// Reads one side's sound until its recorder stops, giving each take it
/// cuts, timed from the call's start, and then saying the side is done.
/// The silence a side is owed before what was just read, in samples, so
/// its sound keeps the call's time: a recorder nothing is linked to isn't
/// run, so it gives nothing until it's linked again. Only a read that
/// emptied the pipe can follow such a gap. A full one is sound the reader
/// fell behind on, waiting its turn on a busy machine, and filling it
/// with silence would split a word in two.
fn silence_owed(due: usize, heard: usize, drained: bool) -> usize {
    if drained && due > heard + GAP {
        due - heard
    } else {
        0
    }
}

fn hear_side(out: impl Read, key: &str, side: Side, since: usize, jobs: &Sender<Job>) {
    let began = Instant::now();
    let mut cutter = Cutter::new();
    let mut heard = 0;
    let mut reader = BufReader::new(out);
    let mut bytes = vec![0; 4 * RATE / 10];
    // A read can end partway through a sample, whose rest comes next.
    let mut carried = Vec::with_capacity(4);
    let send = |take: dictation::takes::Take| {
        let _ = jobs.send(Job::Take {
            key: key.to_owned(),
            side,
            start: since + take.start,
            samples: take.samples,
        });
    };
    loop {
        let read = match reader.read(&mut bytes) {
            Ok(0) | Err(_) => break,
            Ok(read) => read,
        };
        let drained = read < bytes.len();
        carried.extend_from_slice(&bytes[..read]);
        let whole = carried.len() - carried.len() % 4;
        let due = usize::try_from(began.elapsed().as_millis()).unwrap_or(0) * RATE / 1000;
        let samples: Vec<f32> = carried
            .drain(..whole)
            .collect::<Vec<u8>>()
            .as_chunks::<4>()
            .0
            .iter()
            .map(|bytes| f32::from_le_bytes(*bytes))
            .collect();
        let gap = silence_owed(due, heard + samples.len(), drained);
        if gap > 0 {
            cutter.feed(&vec![0.0; gap]).into_iter().for_each(send);
            heard += gap;
        }
        cutter.feed(&samples).into_iter().for_each(send);
        heard += samples.len();
    }
    cutter.finish().into_iter().for_each(send);
    let _ = jobs.send(Job::Done {
        key: key.to_owned(),
        side,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_recorder_given_nothing_for_a_while_is_owed_that_silence() {
        assert_eq!(silence_owed(3 * RATE, RATE, true), 2 * RATE);
    }

    #[test]
    fn a_reader_behind_on_a_busy_machine_is_owed_no_silence() {
        assert_eq!(silence_owed(3 * RATE, RATE, false), 0);
    }

    #[test]
    fn the_clock_running_a_little_ahead_is_owed_no_silence() {
        assert_eq!(silence_owed(RATE + GAP, RATE, true), 0);
    }
}
