//! Sums each call up once it's written down, on a thread of its own, with
//! the summarizer's model in a process of its own (ask.rs) that's started
//! for one call's summary and ended after it, so nothing stays loaded
//! between calls.
//!
//! The model takes most of the laptop's memory and its GPU for minutes,
//! so it minds its manners. It runs only on mains power, waiting for the
//! charger otherwise; never while a call is on, stopping when one starts
//! and trying again once it's over; and holds a delay on sleep while it
//! runs, stopping when the laptop is about to sleep and trying again a
//! while after it wakes. The model's process waits itself for the memory
//! it needs (ask.rs), and one that fails, as when the GPU runs out, is
//! tried again a few times. A summary stopped or failed writes nothing,
//! since a summary of part of a call reads as the whole.
//!
//! A call with too little said isn't given to the model, and with no
//! summarizer, a call written down whole is said so in a notice that
//! opens its transcript. What can't be summed up is said in a notice
//! (notices.rs).

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::time::{Duration, Instant};

use serde_json::{Value, json};

use crate::ask::SHORT_OF_MEMORY;
use crate::book::{Record, SUMMARY, Shared, TRANSCRIPT, write_whole};
use crate::hear::Job;
use crate::{filed, notices, said, summary};

/// How often what the summarizer waits on is looked at again.
const LOOK: Duration = Duration::from_secs(5);

/// How long a stop for sleep waits before trying again. The time a
/// laptop is asleep isn't counted, so this runs out a while after it
/// wakes, and the summary is begun again then.
const AFTER_SLEEP: Duration = Duration::from_mins(1);

/// How long a summary that failed, or found too little memory free,
/// waits before it's tried again.
const AFTER_FAILING: Duration = Duration::from_mins(5);

/// How many times a summary that failed is tried before it's given up
/// on, and Joel told.
const TRIES: u32 = 3;

/// What the summarizer thread is given and shares.
pub struct Summarizer {
    /// The program that answers prompts, if there's one.
    pub program: Option<PathBuf>,
    /// Whether a call is on, as follow.rs says.
    pub calling: Arc<AtomicBool>,
    /// Where power supplies are, `/sys/class/power_supply`.
    pub power: PathBuf,
    /// What Joel is called in a transcript, so the others are never
    /// called the same.
    pub me: String,
}

/// Why a summary was stopped before it was done.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Stop {
    Call,
    Battery,
    Sleep,
}

/// What came of trying to sum a call up once.
enum Tried {
    Summed(String),
    Stopped(Stop),
    /// The model's process found too little memory free.
    Short,
    Failed(String),
}

/// Sums up every call whose key comes, until no more can.
pub fn run(summarizer: &Summarizer, book: &Shared, keys: &Receiver<String>, again: &Sender<Job>) {
    let mut queued: Vec<String> = Vec::new();
    loop {
        if queued.is_empty() {
            match keys.recv() {
                Ok(key) => queued.push(key),
                Err(_) => return,
            }
        }
        while let Ok(key) = keys.try_recv() {
            if !queued.contains(&key) {
                queued.push(key);
            }
        }
        let key = queued.remove(0);
        if let Some(program) = &summarizer.program {
            sum_up(summarizer, program, book, &key, keys, &mut queued, again);
        } else if let Some((record, _)) = readable(book, &key, again) {
            written_down(&record);
        }
    }
}

/// The call's record and transcript, when it's to be given to the model.
/// A transcript that can't be read can't pass, so it offers no Try
/// Again; a call with too little said is told so, with no model asked;
/// and one whose file holds a summary already is summed up, since a call
/// can be asked about twice as it's rejoined, and a rejoin with more
/// said writes it again with none.
fn readable(book: &Shared, key: &str, again: &Sender<Job>) -> Option<(Record, String)> {
    let record = book.lock().record(key).cloned()?;
    let file = record.transcript.clone()?;
    let read = std::fs::read_to_string(file);
    if read
        .as_ref()
        .is_ok_and(|file| filed::summary(file).is_some())
    {
        eprintln!("call-transcripts: {key} is summed up already");
        return None;
    }
    match read.map(|text| filed::transcript(&text).to_owned()) {
        Ok(text) if said::too_little(&text) => {
            eprintln!("call-transcripts: too little was said on {key} to sum up");
            keep(book, &record, said::TOO_LITTLE, &text, again);
            None
        }
        Ok(text) => Some((record, text)),
        Err(why) => {
            let why = format!("Its transcript can't be read: {why}");
            not_summed_up(&record, &why, false, again);
            None
        }
    }
}

/// Sums the call `key` up, waiting as the manners say and trying again
/// after a stop or a failure, keys that come meanwhile being queued. It
/// waits too while the call is written, rejoined, and reads the
/// transcript again for each try, so a try is of the whole call as it
/// stands.
fn sum_up(
    summarizer: &Summarizer,
    program: &Path,
    book: &Shared,
    call: &str,
    keys: &Receiver<String>,
    queued: &mut Vec<String>,
    again: &Sender<Job>,
) {
    let mut failures = 0;
    let mut not_before = Instant::now();
    loop {
        // Waiting, on a call, the charger or a time, takes the keys that
        // come meanwhile, so none is lost.
        while Instant::now() < not_before
            || summarizer.stop().is_some()
            || book.lock().record(call).is_some_and(|now| now.transcribing)
        {
            match keys.recv_timeout(LOOK) {
                Ok(key) if !queued.contains(&key) && key != call => queued.push(key),
                Ok(_) | Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => return,
            }
        }
        let Some((record, text)) = readable(book, call, again) else {
            return;
        };
        eprintln!("call-transcripts: summing up {call}");
        let them = record.them(&summarizer.me);
        match summarizer.try_once(program, &record.title, them.as_deref(), &text) {
            Tried::Summed(written) => {
                keep(book, &record, &written, &text, again);
                return;
            }
            Tried::Stopped(stop) => {
                eprintln!("call-transcripts: summing up stopped, {stop:?}");
                if stop == Stop::Sleep {
                    not_before = Instant::now() + AFTER_SLEEP;
                }
            }
            Tried::Short => not_before = Instant::now() + AFTER_FAILING,
            Tried::Failed(why) => {
                failures += 1;
                eprintln!("call-transcripts: summing up failed ({failures}): {why}");
                if failures >= TRIES {
                    not_summed_up(&record, &why, true, again);
                    return;
                }
                not_before = Instant::now() + AFTER_FAILING;
            }
        }
    }
}

impl Summarizer {
    /// Why the summarizer mustn't run now, if it mustn't.
    fn stop(&self) -> Option<Stop> {
        if self.calling.load(Ordering::SeqCst) {
            Some(Stop::Call)
        } else if !on_mains(&self.power) {
            Some(Stop::Battery)
        } else {
            None
        }
    }

    /// Sums the call up once with a model of its own, holding off sleep
    /// while it does.
    fn try_once(&self, program: &Path, title: &str, them: Option<&str>, text: &str) -> Tried {
        let sleep = match Sleep::hold() {
            Ok(sleep) => sleep,
            Err(why) => return Tried::Failed(why),
        };
        let mut model = match Model::start(program) {
            Ok(model) => model,
            Err(why) => return Tried::Failed(why),
        };
        let mut stopped = None;
        let summed = summary::summarize_as(
            &mut |prompt, tokens| {
                model.ask(prompt, tokens, &mut || {
                    let stop = self
                        .stop()
                        .or_else(|| sleep.coming().then_some(Stop::Sleep));
                    stopped = stopped.or(stop);
                    stop.is_some()
                })
            },
            title,
            Some(&self.me),
            them,
            text,
        );
        if let Some(stop) = stopped {
            // Killed at once, mid-answer, so the GPU is free before the
            // call needs it or the laptop sleeps.
            drop(model);
            return Tried::Stopped(stop);
        }
        match (summed, model.end()) {
            (Ok(written), _) => Tried::Summed(written),
            (Err(_), Some(code)) if code == i32::from(SHORT_OF_MEMORY.code()) => Tried::Short,
            (Err(why), _) => Tried::Failed(why),
        }
    }
}

/// Whether the laptop runs from the wall: a supply in `power` says it's
/// online, or none says either way, as on a machine with no battery.
pub fn on_mains(power: &Path) -> bool {
    let Ok(supplies) = std::fs::read_dir(power) else {
        return true;
    };
    let online: Vec<String> = supplies
        .filter_map(|supply| std::fs::read_to_string(supply.ok()?.path().join("online")).ok())
        .collect();
    online.is_empty() || online.iter().any(|said| said.trim() == "1")
}

/// The summarizer's model, in its own process.
struct Model {
    child: Child,
    input: Option<ChildStdin>,
    answers: Receiver<String>,
}

impl Model {
    fn start(program: &Path) -> Result<Self, String> {
        let mut child = Command::new(program)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .map_err(|why| format!("{} wouldn't start: {why}", program.display()))?;
        let input = child.stdin.take();
        let output = child.stdout.take().ok_or("the summarizer has no output")?;
        let (said, answers) = mpsc::channel();
        std::thread::spawn(move || {
            for line in BufReader::new(output).lines() {
                let Ok(line) = line else { break };
                if said.send(line).is_err() {
                    break;
                }
            }
        });
        Ok(Self {
            child,
            input,
            answers,
        })
    }

    /// The model's answer to `prompt`, at most `tokens` long, given up on
    /// as soon as `stop` says to.
    fn ask(
        &mut self,
        prompt: &str,
        tokens: usize,
        stop: &mut dyn FnMut() -> bool,
    ) -> Result<String, String> {
        if stop() {
            return Err("stopped".to_owned());
        }
        let input = self.input.as_mut().ok_or("the summarizer has ended")?;
        writeln!(input, "{}", json!({ "prompt": prompt, "tokens": tokens }))
            .and_then(|()| input.flush())
            .map_err(|why| format!("the summarizer ended: {why}"))?;
        loop {
            match self.answers.recv_timeout(Duration::from_secs(1)) {
                Ok(line) => {
                    let answer: Value = serde_json::from_str(&line)
                        .map_err(|why| format!("the summarizer's answer isn't JSON: {why}"))?;
                    return Ok(answer["answer"].as_str().unwrap_or_default().to_owned());
                }
                Err(RecvTimeoutError::Timeout) if stop() => return Err("stopped".to_owned()),
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => {
                    return Err("the summarizer ended before it answered".to_owned());
                }
            }
        }
    }

    /// Lets the model go, told it's asked no more, which it hears once
    /// it's done answering; gives how its process ended.
    fn end(mut self) -> Option<i32> {
        drop(self.input.take());
        self.child.wait().ok()?.code()
    }
}

impl Drop for Model {
    /// Ends the model's process at once, mid-answer or not; one that
    /// already ended is only reaped.
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// A delay on sleep held through logind while the summarizer runs, and
/// the word that sleep is coming, from logind's `PrepareForSleep`.
struct Sleep {
    /// systemd-inhibit, holding the delay until its input ends.
    held: Child,
    /// gdbus, saying each of logind's signals a line.
    heard: Child,
    coming: Arc<AtomicBool>,
}

impl Sleep {
    fn hold() -> Result<Self, String> {
        let held = Command::new("systemd-inhibit")
            .args([
                "--what=sleep",
                "--mode=delay",
                "--who=Calls",
                "--why=Summing up a call",
                "cat",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .spawn()
            .map_err(|why| format!("systemd-inhibit wouldn't start: {why}"))?;
        let mut heard = Command::new("gdbus")
            .args([
                "monitor",
                "--system",
                "--dest",
                "org.freedesktop.login1",
                "--object-path",
                "/org/freedesktop/login1",
            ])
            .stdout(Stdio::piped())
            .spawn()
            .map_err(|why| format!("gdbus wouldn't start: {why}"))?;
        let output = heard.stdout.take().ok_or("gdbus has no output")?;
        let coming = Arc::new(AtomicBool::new(false));
        let told = Arc::clone(&coming);
        std::thread::spawn(move || {
            for line in BufReader::new(output).lines() {
                let Ok(line) = line else { break };
                if sleeps(&line) {
                    told.store(true, Ordering::SeqCst);
                }
            }
        });
        Ok(Self {
            held,
            heard,
            coming,
        })
    }

    /// Whether logind has said the laptop is about to sleep.
    fn coming(&self) -> bool {
        self.coming.load(Ordering::SeqCst)
    }
}

impl Drop for Sleep {
    fn drop(&mut self) {
        // Its input ending ends cat, and systemd-inhibit lets the delay go.
        drop(self.held.stdin.take());
        let _ = self.held.wait();
        let _ = self.heard.kill();
        let _ = self.heard.wait();
    }
}

/// Whether a line of gdbus's is logind saying the laptop is about to
/// sleep, rather than that it's woken.
fn sleeps(line: &str) -> bool {
    line.contains("org.freedesktop.login1.Manager.PrepareForSleep (true")
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

/// Writes `written` as the call's summary, over its transcript in its
/// one file, says it's ready, and puts it in the call's record. The
/// transcript is read and the file written under the book's lock, as
/// the transcript writes it, so neither loses what the other wrote.
///
/// A summary is kept only while the transcript is still `summed`, the
/// one it was made from, since one of part of a call reads as the whole.
/// A call rejoined meanwhile is summed up again once it ends, or once
/// it's found it can't be recorded (follow.rs); one that went on, its
/// rejoin over before this summary was done, is asked about again now.
/// A summary.md from before goes, the file holding the summary now. A
/// disk with no room can pass, as room is made, so failing to write
/// offers Try Again.
fn keep(book: &Shared, record: &Record, written: &str, summed: &str, again: &Sender<Job>) {
    match filed_away(book, record, written, summed) {
        Filed::Rejoined => eprintln!(
            "call-transcripts: {} was rejoined while summed up",
            record.key
        ),
        Filed::WentOn => {
            eprintln!("call-transcripts: {} went on while summed up", record.key);
            let key = record.key.clone();
            let _ = again.send(Job::SumUp { key });
        }
        Filed::Failed { why, can_pass } => not_summed_up(record, &why, can_pass, again),
        Filed::Kept => {
            eprintln!("call-transcripts: {} summed up", record.key);
            tell(&record.title, written, &record.folder.join(TRANSCRIPT));
        }
    }
}

/// What became of a summary [`filed_away`] was given.
#[derive(Debug, PartialEq, Eq)]
enum Filed {
    /// The call is on again, rejoined, and is summed up once it ends.
    Rejoined,
    /// The call went on after the summary was made, and is summed up
    /// again.
    WentOn,
    /// The call's file couldn't be read or written, for the reason given;
    /// a disk with no room can pass, as room is made, but a file that
    /// can't be read can't, as [`readable`] says.
    Failed { why: String, can_pass: bool },
    /// The summary is in the call's file and its record.
    Kept,
}

/// Writes `written` into the call's one file and record, under the
/// book's lock, as [`keep`] says, and says what became of it, leaving
/// [`keep`] to tell Joel.
fn filed_away(book: &Shared, record: &Record, written: &str, summed: &str) -> Filed {
    let file = record.folder.join(TRANSCRIPT);
    let mut book = book.lock();
    let now = book.record(&record.key).cloned();
    // Rejoined and on, it's summed up once it ends, even before it's
    // said anything more.
    if now.as_ref().is_some_and(|now| now.transcribing) {
        return Filed::Rejoined;
    }
    let title = now.as_ref().map_or(&record.title, |now| &now.title);
    let text = match std::fs::read_to_string(&file) {
        Ok(text) => text,
        Err(why) => {
            let why = format!("{} couldn't be read: {why}", file.display());
            return Filed::Failed {
                why,
                can_pass: false,
            };
        }
    };
    let Some(whole) = filed::summed(&text, title, written, summed) else {
        return Filed::WentOn;
    };
    if let Err(why) = write_whole(&file, &whole) {
        let why = format!("{} couldn't be written: {why}", file.display());
        return Filed::Failed {
            why,
            can_pass: true,
        };
    }
    let before = record.folder.join(SUMMARY);
    if let Err(why) = std::fs::remove_file(&before)
        && why.kind() != std::io::ErrorKind::NotFound
    {
        eprintln!("call-transcripts: removing {}: {why}", before.display());
    }
    if let Some(now) = now {
        book.put(Record {
            summary: filed::summary(&whole),
            ..now
        });
    }
    Filed::Kept
}

/// Says a call is written down, with no summarizer to sum it up, in a
/// notice that opens its transcript.
fn written_down(record: &Record) {
    eprintln!(
        "call-transcripts: {} is written down, with no summary",
        record.key
    );
    let transcript = record.transcript.clone();
    notices::show(&notices::written_down(&record.title), move |chosen| {
        if let ("transcript" | "default", Some(file)) = (chosen, &transcript) {
            notices::open(file);
        }
    });
}

/// Says a call's summary is ready, in a notice whose Read It opens it.
fn tell(title: &str, written: &str, file: &Path) {
    // The summary's first quote, or its first line when it has none,
    // without a bullet's marker, since a notice's line is no list; a
    // heading only repeats the notice's own.
    let lines = || written.lines().map(str::trim).filter(|l| !l.is_empty());
    let first = lines()
        .find(|line| line.starts_with("- "))
        .or_else(|| lines().find(|line| !line.starts_with('#')))
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

#[cfg(test)]
mod tests {
    use super::*;

    fn supplies(online: &[(&str, &str)]) -> tempfile::TempDir {
        let power = tempfile::tempdir().unwrap();
        for (name, said) in online {
            let supply = power.path().join(name);
            std::fs::create_dir(&supply).unwrap();
            std::fs::write(supply.join("online"), said).unwrap();
        }
        power
    }

    const SAID: &str = "# Transcript\n\n**Joel**, 0:00: Can we move it?\n";
    const SUMMED: &str = "# Planning\n\n- Joel: \"move it\" [0:00]";

    /// A call that ended, in a folder of its own under `calls`, written
    /// down with its summary still to come and a summary.md left from
    /// before beside it, and the book holding it.
    fn ended(calls: &Path) -> (Shared, Record) {
        let record = Record {
            key: "call:e1".to_owned(),
            title: "Planning".to_owned(),
            started: 10,
            ended: Some(20),
            event: Some("e1".to_owned()),
            person: None,
            account: None,
            people: Vec::new(),
            known: None,
            folder: calls.join("call-e1"),
            transcript: None,
            opening: None,
            summary: None,
            transcribing: false,
        };
        let mut book = crate::book::Book::default();
        book.put(record.clone());
        let file = filed::whole("Planning", None, SAID);
        std::fs::write(record.folder.join(TRANSCRIPT), file).unwrap();
        std::fs::write(record.folder.join(SUMMARY), "# Planning\n\n- old\n").unwrap();
        (Shared::new(book), record)
    }

    fn file(record: &Record) -> String {
        std::fs::read_to_string(record.folder.join(TRANSCRIPT)).unwrap()
    }

    #[test]
    fn a_summary_of_the_whole_call_goes_in_its_file_and_record_and_summary_md_goes() {
        let calls = tempfile::tempdir().unwrap();
        let (book, record) = ended(calls.path());
        let summed = filed::transcript(&file(&record)).to_owned();
        assert_eq!(filed_away(&book, &record, SUMMED, &summed), Filed::Kept);
        assert_eq!(
            filed::summary(&file(&record)).as_deref(),
            Some("- Joel: \"move it\" [0:00]"),
            "the file holds the summary"
        );
        assert_eq!(
            filed::transcript(&file(&record)),
            summed,
            "over the same words"
        );
        assert!(
            !record.folder.join(SUMMARY).exists(),
            "summary.md from before is removed"
        );
        let held = book.lock().record(&record.key).unwrap().summary.clone();
        assert_eq!(
            held.as_deref(),
            Some("- Joel: \"move it\" [0:00]"),
            "the record holds it"
        );
        let (again, _asked) = mpsc::channel();
        assert!(
            readable(&book, &record.key, &again).is_none(),
            "a call summed up already, asked about twice, was given to the model again"
        );
    }

    #[test]
    fn a_summary_of_a_call_rejoined_or_gone_on_is_not_kept() {
        let calls = tempfile::tempdir().unwrap();
        let (book, record) = ended(calls.path());
        let before = file(&record);
        let summed = filed::transcript(&before).to_owned();
        let went_on = "## Transcript\n\n**Joel**, 0:00: Can we move it to Friday?\n";
        assert_eq!(
            filed_away(&book, &record, SUMMED, went_on),
            Filed::WentOn,
            "summed from words the call has since gone past"
        );
        assert_eq!(file(&record), before, "a call gone on keeps its file");
        let (again, asked) = mpsc::channel();
        keep(&book, &record, SUMMED, went_on, &again);
        assert!(
            matches!(asked.try_recv(), Ok(Job::SumUp { key }) if key == record.key),
            "a call gone on wasn't asked to be summed up again"
        );
        let rejoined = Record {
            transcribing: true,
            ..record.clone()
        };
        book.lock().put(rejoined);
        keep(&book, &record, SUMMED, &summed, &again);
        assert!(
            asked.try_recv().is_err(),
            "a rejoined call was asked about before its end"
        );
        assert_eq!(filed_away(&book, &record, SUMMED, &summed), Filed::Rejoined);
        assert_eq!(file(&record), before, "a rejoined call keeps its file");
        assert!(
            record.folder.join(SUMMARY).exists(),
            "summary.md stays until a summary takes its place"
        );
    }

    #[test]
    fn a_file_that_cant_be_read_is_said_to_be_unread() {
        let calls = tempfile::tempdir().unwrap();
        let (book, record) = ended(calls.path());
        std::fs::remove_file(record.folder.join(TRANSCRIPT)).unwrap();
        let Filed::Failed { why, can_pass } = filed_away(&book, &record, SUMMED, SAID) else {
            panic!("a missing file can't take a summary");
        };
        assert!(why.contains("couldn't be read"), "said as unread: {why}");
        assert!(!can_pass, "a file that can't be read offered Try Again");
    }

    #[test]
    fn a_charger_plugged_in_is_mains_power() {
        let power = supplies(&[("AC", "1\n"), ("ucsi-source-psy-1", "0\n")]);
        assert!(on_mains(power.path()));
    }

    #[test]
    fn every_charger_unplugged_is_battery() {
        let power = supplies(&[("AC", "0\n"), ("ucsi-source-psy-1", "0\n")]);
        assert!(!on_mains(power.path()));
    }

    #[test]
    fn a_machine_with_no_charger_runs_from_the_wall() {
        let power = supplies(&[]);
        std::fs::create_dir(power.path().join("BAT0")).unwrap();
        assert!(on_mains(power.path()));
        assert!(on_mains(&power.path().join("nowhere")));
    }

    #[test]
    fn logind_saying_sleep_is_coming_is_heard_and_waking_isnt() {
        assert!(sleeps(
            "/org/freedesktop/login1: org.freedesktop.login1.Manager.PrepareForSleep (true,)"
        ));
        assert!(!sleeps(
            "/org/freedesktop/login1: org.freedesktop.login1.Manager.PrepareForSleep (false,)"
        ));
    }
}
