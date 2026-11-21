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
//! Who's on a call is kept as everyone ever on it: those on it as it
//! starts or is rejoined, and, on a Tuple call or a followed Meet, those
//! it says are on it as it goes, since people come and go. Their side is
//! named only while that's one person, so someone joining late turns it
//! back to Others, every line before included.
//!
//! As a call starts, a notice offers the line for its chat (notices.rs),
//! and its Stop Transcribing stops both recorders at once, keeping what's
//! written. A call stopped isn't transcribed again if it's rejoined, since
//! stopping is how Joel honours someone's asking him not to.
//!
//! While a call is on, transcribed or not, the laptop is kept from going
//! idle, so idle sleep on battery never ends a call that only listens,
//! which a call app may not say is going on.

use std::collections::{HashMap, HashSet};
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
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

/// How often who's on a Tuple call or a Meet being recorded is asked
/// again.
const LOOK_AGAIN: Duration = Duration::from_secs(30);

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

/// logind's hold against going idle, kept while a call is on. The
/// screens stay on and unlocked under it, as a video call keeps them, and
/// a sleep Joel asks for still goes ahead, where a hold against sleep
/// would refuse his launcher's Suspend mid-call.
///
/// systemd-inhibit holds it for as long as the program it runs, cat, and
/// cat reads from this process, so the hold ends when it's dropped and
/// also when call-transcripts dies, rather than outliving it.
struct Awake(Child);

impl Awake {
    /// Holds the laptop awake for a call.
    fn hold() -> Option<Self> {
        let mut inhibit = Command::new("systemd-inhibit");
        inhibit.args([
            "--what=idle",
            "--mode=block",
            "--who=call-transcripts",
            "--why=A call is on",
            "cat",
        ]);
        Self::running(inhibit)
    }

    /// Runs what holds the laptop awake, reading this process's pipe.
    fn running(mut holding: Command) -> Option<Self> {
        match holding.stdin(Stdio::piped()).stdout(Stdio::null()).spawn() {
            Ok(child) => Some(Self(child)),
            Err(why) => {
                eprintln!("call-transcripts: couldn't keep the laptop awake for the call: {why}");
                None
            }
        }
    }
}

impl Drop for Awake {
    /// Closes the pipe, which ends cat and with it the hold.
    fn drop(&mut self) {
        drop(self.0.stdin.take());
        let _ = self.0.wait();
    }
}

/// What the loop following calls is given: the graph as it changes, or
/// Joel's Stop Transcribing for a call.
enum Heard {
    Graph(Value),
    Stop(String),
}

/// Follows calls until PipeWire goes, for systemd to start it again,
/// saying in `calling` whether one is on, for the summarizer to wait on.
pub fn follow(
    book: &Arc<Shared>,
    calls: &Path,
    jobs: &Sender<Job>,
    calling: &AtomicBool,
    me: &str,
) -> Result<(), String> {
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
    // Held from a call's start to its end.
    let mut awake: Option<Awake> = None;
    // The calls Joel stopped, never transcribed again while this runs.
    let mut stopped: HashSet<String> = HashSet::new();
    // The notice saying the call is being transcribed, withdrawn once it
    // isn't, as the call ends or its transcribing stops.
    let mut offered: Option<notices::Shown> = None;
    let mut count = 0;
    let mut looked = Instant::now();
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
                    if let Some(said) = offered.take() {
                        said.withdraw();
                    }
                    eprintln!("call-transcripts: transcribing stopped, {key}");
                    stopped.insert(key);
                }
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }
        match call.step(graph.call().is_some(), Instant::now()) {
            Some(Change::Started) => {
                calling.store(true, Ordering::SeqCst);
                count += 1;
                let at = now();
                let on = meeting::on(&runtime, at, me);
                let key = Record::key_for(on.event.as_deref(), on.person.as_deref(), at);
                on_call = Some(key.clone());
                if let Some(held) = Awake::hold() {
                    awake.replace(held);
                }
                if stopped.contains(&key) {
                    eprintln!("call-transcripts: a call stopped is rejoined, {key}");
                    continue;
                }
                let title = on.title.clone();
                match start(book, calls, &key, on, at, jobs, count) {
                    Ok(started) => {
                        offered = Some(offer_line(&title, &key, &stops));
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
                drop(awake.take());
                calling.store(false, Ordering::SeqCst);
                if let Some(mut ended) = recording.take() {
                    ended.stop();
                }
                if let Some(said) = offered.take() {
                    said.withdraw();
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
        // Said from what's recorded here, after every way one starts or
        // stops, so the shell keeps its Meet followed only while it is.
        book.lock()
            .recording(recording.as_ref().map(|now| now.key.as_str()));
        look_again(book, &runtime, recording.as_ref(), &mut looked, me);
    }
    let _ = dump.wait();
    Err("pw-dump ended".to_owned())
}

/// Offers the line for the call's chat, quietly, as the call starts:
/// clicking the card copies it, and Stop Transcribing stops the call's
/// recorders. It's given back to be withdrawn once the call isn't being
/// transcribed.
fn offer_line(title: &str, key: &str, stops: &Sender<Heard>) -> notices::Shown {
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
    )
}

/// The account a call `kept` as its meeting's is kept as once that
/// meeting is joined on `now`: a meeting rejoined on another account than
/// it was first is no longer known to be either's, so it keeps none, and a
/// join whose account went unheard keeps the one it had.
fn rejoined(kept: Option<String>, now: Option<String>) -> Option<String> {
    match now {
        Some(now) if kept.as_ref() != Some(&now) => None,
        _ => kept,
    }
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
    let account = on.account;
    let seen = (on.people.clone(), on.known);
    let mut record = book.lock().record(&key).cloned().unwrap_or_else(|| Record {
        key: key.clone(),
        folder: calls.join(meeting::folder(at, &on.title)),
        title: on.title,
        started: at,
        ended: None,
        event: on.event,
        person: on.person,
        account: account.clone(),
        people: on.people,
        known: on.known,
        transcript: None,
        opening: None,
        summary: None,
        transcribing: true,
    });
    record.account = rejoined(record.account.take(), account);
    (record.people, record.known) =
        meeting::came((std::mem::take(&mut record.people), record.known), seen);
    let ended = record.ended.take();
    record.transcribing = true;
    // A summary of the call's first legs would read as the whole of it,
    // so a rejoined call says its summary is to come until it ends. One
    // that can't be heard keeps it, as its file still does.
    let summed = record.summary.take();
    let since = usize::try_from((at - record.started) / 1000).unwrap_or(0) * RATE;
    book.lock().put(record.clone());
    let mine = Recorder::start(
        &format!("call-transcripts-mine-{count}"),
        &key,
        Side::Mine,
        since,
        jobs,
    );
    let mut mine = match mine {
        Ok(mine) => mine,
        Err(why) => {
            unheard(
                book,
                Record {
                    summary: summed,
                    ended,
                    ..record
                },
                None::<fn()>,
                jobs,
            );
            return Err(why);
        }
    };
    let theirs = Recorder::start(
        &format!("call-transcripts-theirs-{count}"),
        &key,
        Side::Theirs,
        since,
        jobs,
    );
    let theirs = match theirs {
        Ok(theirs) => theirs,
        // Joel's side alone would be half a call, heard as all of it.
        Err(why) => {
            let record = Record {
                summary: summed,
                ended,
                ..record
            };
            unheard(book, record, Some(|| mine.stop()), jobs);
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

/// Puts a call whose recording couldn't start back as it was, `record`
/// holding its summary and end again, and leaves nothing waiting on it.
/// Joel's side, if it started, is stopped after, so what it heard is
/// written under the summary put back; its own end and the end said here
/// for their side, which never started, let hear finish the call, as it
/// sums nothing up while a call is half done. A call with no recorder
/// is asked to be summed up, as a rejoin that took its summary away
/// might have been the only way it would be again.
fn unheard(book: &Shared, record: Record, stop_mine: Option<impl FnOnce()>, jobs: &Sender<Job>) {
    let key = record.key.clone();
    book.lock().put(Record {
        transcribing: false,
        ..record
    });
    let _ = match stop_mine {
        Some(stop) => {
            stop();
            jobs.send(Job::Done {
                key,
                side: Side::Theirs,
            })
        }
        None => jobs.send(Job::SumUp { key }),
    };
}

impl Recording {
    /// Stops both recorders; what's heard after ends the transcript
    /// (hear.rs).
    fn stop(&mut self) {
        self.mine.stop();
        self.theirs.stop();
    }
}

/// Adds to a call's record whoever its Tuple call or Meet says is on it
/// now (meet.rs's `still_on`), since people join and leave as it goes,
/// Joel, `me`, left out of a Meet's, and when each was in. Asked every
/// `LOOK_AGAIN` since it was last `looked`, while a call is recorded.
fn look_again(
    book: &Shared,
    runtime: &Path,
    recording: Option<&Recording>,
    looked: &mut Instant,
    me: &str,
) {
    let Some(key) = recording
        .map(|recording| recording.key.as_str())
        .filter(|_| looked.elapsed() >= LOOK_AGAIN)
    else {
        return;
    };
    *looked = Instant::now();
    let Some((known, event)) = book
        .lock()
        .record(key)
        .map(|record| (record.known, record.event.clone()))
    else {
        return;
    };
    // Asked outside the book's lock, since a daemon may be slow.
    let Some(seen) = crate::meet::still_on(runtime, known, event.as_deref(), me) else {
        return;
    };
    if book.lock().saw(key, seen, now()) {
        eprintln!("call-transcripts: who's on it changed, {key}");
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
            .arg(props(name))
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

/// A recorder's node: named so it's found in the graph, and async, so the
/// call never waits on it. A follower PipeWire schedules in step is one
/// its driver waits for before ending a cycle, and a recorder in
/// background.slice with no realtime thread is late on a busy machine:
/// on Joel's side that holds back the microphone's cycle, on theirs the
/// speaker's, the voice he hears. An async node is never waited for, at a
/// cycle's latency the transcript doesn't notice, so a late recorder loses
/// only its own sound.
fn props(name: &str) -> String {
    format!("{{ node.name = \"{name}\" node.description = \"Call transcript\" node.async = true }}")
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
    fn a_meeting_rejoined_on_another_account_is_kept_as_no_one_s() {
        let ours = || Some("jsama@example.com".to_owned());
        assert_eq!(
            rejoined(ours(), ours()),
            ours(),
            "rejoined on its own account"
        );
        assert_eq!(
            rejoined(ours(), Some("skiff@example.com".to_owned())),
            None,
            "rejoined on another account, it stayed the first's"
        );
        assert_eq!(
            rejoined(ours(), None),
            ours(),
            "an unheard rejoin lost its account"
        );
        assert_eq!(
            rejoined(None, ours()),
            None,
            "a call with no account took one on rejoining"
        );
    }

    /// A call rejoined, as `start` leaves its record before recording.
    fn rejoining(calls: &Path) -> (Shared, Record) {
        let record = Record {
            key: "call:e1".to_owned(),
            title: "Planning".to_owned(),
            started: 10,
            ended: None,
            event: Some("e1".to_owned()),
            person: None,
            account: None,
            people: Vec::new(),
            known: None,
            folder: calls.join("call-e1"),
            transcript: None,
            opening: None,
            summary: None,
            transcribing: true,
        };
        let mut book = crate::book::Book::default();
        book.put(record.clone());
        (Shared::new(book), record)
    }

    #[test]
    fn a_rejoin_that_cant_record_is_put_back_and_finished_by_hear() {
        let calls = tempfile::tempdir().unwrap();
        let (book, record) = rejoining(calls.path());
        let (jobs, heard) = mpsc::channel();
        let stopped = std::cell::Cell::new(false);
        let back = Record {
            summary: Some("- kept".to_owned()),
            ended: Some(20),
            ..record
        };
        unheard(&book, back, Some(|| stopped.set(true)), &jobs);
        let held = book.lock().record("call:e1").cloned().unwrap();
        assert_eq!(
            (held.summary.as_deref(), held.ended, held.transcribing),
            (Some("- kept"), Some(20), false),
            "the record wasn't put back as it was"
        );
        assert!(stopped.get(), "Joel's side was left recording");
        assert!(
            matches!(
                heard.try_recv(),
                Ok(Job::Done { key, side: Side::Theirs }) if key == "call:e1"
            ),
            "hear wasn't told their side is done, so the call stays half done"
        );
    }

    #[test]
    fn a_call_with_no_recorder_is_asked_to_be_summed_up() {
        let calls = tempfile::tempdir().unwrap();
        let (book, record) = rejoining(calls.path());
        let (jobs, heard) = mpsc::channel();
        unheard(&book, record, None::<fn()>, &jobs);
        assert!(
            matches!(heard.try_recv(), Ok(Job::SumUp { key }) if key == "call:e1"),
            "a rejoin that took its summary away left it unsummed"
        );
        assert!(
            !book.lock().record("call:e1").unwrap().transcribing,
            "the call is said to be still written"
        );
    }

    #[test]
    fn letting_go_of_the_hold_ends_what_holds_it() {
        let awake = Awake::running(Command::new("cat")).expect("cat runs");
        let id = awake.0.id();
        drop(awake);
        assert!(
            !Path::new(&format!("/proc/{id}")).exists(),
            "cat still runs once the hold is let go"
        );
    }

    #[test]
    fn a_recorder_never_holds_up_the_calls_sound() {
        let props = props("call-transcripts-theirs-1");
        assert!(
            props.contains("node.async = true"),
            "a recorder late to its cycle would hold back the call's: {props}"
        );
        assert!(
            props.contains("node.name = \"call-transcripts-theirs-1\""),
            "the recorder isn't named for the graph to find: {props}"
        );
    }

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
