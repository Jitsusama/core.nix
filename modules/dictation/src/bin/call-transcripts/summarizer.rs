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
use crate::book::{Record, SUMMARY, Shared, write_whole};
use crate::hear::Job;
use crate::{notices, said, summary};

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
        if let Some((record, text)) = readable(book, &key, again) {
            if let Some(program) = &summarizer.program {
                sum_up(
                    summarizer,
                    program,
                    book,
                    &record,
                    &text,
                    keys,
                    &mut queued,
                    again,
                );
            } else {
                written_down(&record);
            }
        }
    }
}

/// The call's record and transcript, when it's to be given to the model.
/// A transcript that can't be read can't pass, so it offers no Try
/// Again; a call with too little said is told so, with no model asked.
fn readable(book: &Shared, key: &str, again: &Sender<Job>) -> Option<(Record, String)> {
    let record = book.lock().record(key).cloned()?;
    let file = record.transcript.clone()?;
    match std::fs::read_to_string(file) {
        Ok(text) if said::too_little(&text) => {
            eprintln!("call-transcripts: too little was said on {key} to sum up");
            keep(book, &record, said::TOO_LITTLE, again);
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

/// Sums one call up, waiting as the manners say and trying again after
/// a stop or a failure, keys that come meanwhile being queued.
#[expect(
    clippy::too_many_arguments,
    reason = "each is a different thing the one summary needs"
)]
fn sum_up(
    summarizer: &Summarizer,
    program: &Path,
    book: &Shared,
    record: &Record,
    text: &str,
    keys: &Receiver<String>,
    queued: &mut Vec<String>,
    again: &Sender<Job>,
) {
    let mut failures = 0;
    let mut not_before = Instant::now();
    loop {
        // Waiting, on a call, the charger or a time, takes the keys that
        // come meanwhile, so none is lost.
        while Instant::now() < not_before || summarizer.stop().is_some() {
            match keys.recv_timeout(LOOK) {
                Ok(key) if !queued.contains(&key) && key != record.key => queued.push(key),
                Ok(_) | Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => return,
            }
        }
        eprintln!("call-transcripts: summing up {}", record.key);
        match summarizer.try_once(program, &record.title, them(record), text) {
            Tried::Summed(written) => {
                keep(book, record, &written, again);
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
                    not_summed_up(record, &why, true, again);
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
        let summed = summary::summarize(
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

/// Who the others on a call are, where the call knows: a Tuple call with
/// one person is titled by their name (meeting.rs); one with more, titled
/// "Ada and 2 more", and a meeting, whose title names no one for certain,
/// leave them as the transcript has them.
fn them(record: &Record) -> Option<&str> {
    record.person.as_ref()?;
    (!record.title.ends_with(" more")).then_some(record.title.as_str())
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

/// Writes `written` as the call's summary, says it's ready, and puts it
/// in the call's record. A disk with no room can pass, as room is made,
/// so failing to write offers Try Again.
fn keep(book: &Shared, record: &Record, written: &str, again: &Sender<Job>) {
    let file = record.folder.join(SUMMARY);
    if let Err(why) = write_whole(&file, &format!("{written}\n")) {
        let why = format!("{} couldn't be written: {why}", file.display());
        not_summed_up(record, &why, true, again);
        return;
    }
    eprintln!("call-transcripts: {} summed up", record.key);
    tell(&record.title, written, &file);
    let mut book = book.lock();
    if let Some(now) = book.record(&record.key).cloned() {
        book.put(Record {
            summary: Some(written.to_owned()),
            ..now
        });
    }
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
