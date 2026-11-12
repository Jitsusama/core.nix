//! Choose by Ear: an A/B Joel runs from the launcher to pick how his
//! microphone is cleaned. Six seconds of his voice, as calls hear it now,
//! are recorded and cleaned by every cleaner, offered back as A to D in an
//! order shuffled afresh, and revealed with what each cost once he picks
//! one. The record and acts are fixed in the quest's trial-protocol.md, in
//! the shell's daemon interface, version 1.
//!
//! What he said is held only here, in memory: never on disk, and the
//! service may reach no network. It's dropped on a pick, a discard, five
//! minutes without an act, or the daemon's exit.
//!
//! Each clip is scored too (score.rs), and its marks revealed with the
//! rest. A pick keeps a line of numbers and names beside the pick's file,
//! in `scored`: when, which microphone and output, which way was picked,
//! and every way's marks and cost, so the scorer can be checked against
//! his ear before it's trusted to choose. The sound itself is never kept.

use std::collections::BTreeMap;
use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Weak};
use std::time::{Duration, Instant};

use dictation::daemon::{self, Answers, Shared, VERSION, Watches, error, now};
// The A/B starts from the microphone as it is: its own node, found
// through its twin when the twin is the default.
pub use dictation::microphone::{Microphone, default_microphone, named};
use serde_json::{Value, json};

use crate::chosen::Chosen;
use crate::defaults::{default_microphone_is, default_output, dumped};
use crate::denoise::{Cleaner, RATE};
use crate::refusals::Refusal;
use crate::score::{self, Marks};
use crate::spot::Spot;
use crate::ways::{self, Way};

/// The record's key and kind.
const KEY: &str = "trial:now";
const KIND: &str = "trial";

/// How long a recording runs.
pub const SECONDS: u64 = 6;

/// How long clips are kept without an act.
const FORGOTTEN: Duration = Duration::from_mins(5);

/// How often a playing clip is looked at to see whether it's done.
const LISTENED: Duration = Duration::from_millis(50);

/// The letters the clips go by.
const LETTERS: [&str; 4] = ["A", "B", "C", "D"];

/// Where the A/B stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    Idle,
    Recording,
    Cleaning,
    Ready,
    Picked,
    Refused,
}

impl Stage {
    const fn name(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Recording => "recording",
            Self::Cleaning => "cleaning",
            Self::Ready => "ready",
            Self::Picked => "picked",
            Self::Refused => "refused",
        }
    }
}

/// The PipeWire programs the A/B runs, by name, so a test can stand in.
#[derive(Clone, Debug)]
pub struct Tools {
    pub dump: String,
    pub record: String,
    pub play: String,
    pub metadata: String,
}

impl Default for Tools {
    fn default() -> Self {
        Self {
            dump: "pw-dump".to_owned(),
            record: "pw-record".to_owned(),
            play: "pw-play".to_owned(),
            metadata: "pw-metadata".to_owned(),
        }
    }
}

/// One clip: what it was cleaned by, and its samples.
struct Clip {
    way: Way,
    samples: Arc<Vec<f32>>,
}

/// The A/B's state, shared between the clients and its own threads.
pub struct Trial {
    me: Weak<Shared<Self>>,
    models: PathBuf,
    /// The cleaner the microphones run, and where a pick is kept for them.
    running: Chosen,
    refusals: Vec<Refusal>,
    tools: Tools,
    watches: Watches,
    since: u64,
    heard: u64,
    stage: Stage,
    why: Option<String>,
    microphone: Option<Microphone>,
    /// The default microphone's node name as the last watch began, so the
    /// cleaner said running is its own before anything's recorded.
    default_microphone: Option<String>,
    /// Where the machine was as the last watch began or the recording
    /// started, which a pick is kept for.
    spot: Option<Spot>,
    /// What the clips play on: the default output, as Sound names it.
    output: Option<String>,
    until: Option<u64>,
    /// The clips by letter, while ready.
    clips: Vec<Clip>,
    /// The floor of the room they were recorded in, raw (room.rs).
    floor: Option<i32>,
    /// Each letter's way, kept past a pick for the reveal.
    ways: Vec<Way>,
    costs: BTreeMap<&'static str, f64>,
    /// Each way's marks by the scorer, once it's heard them.
    marks: BTreeMap<&'static str, Marks>,
    /// Whether this recording's pick and marks are kept yet.
    kept: bool,
    picked: Option<(usize, Way)>,
    playing: Option<(usize, Child)>,
    /// Counts each recording and discard, so a thread whose recording has
    /// been let go drops what it finishes.
    generation: u64,
    acted: Instant,
    /// Choose by Ear offered after a call (offer.rs), as it stands.
    offer: Value,
}

impl Trial {
    /// The A/B, shared, ready for clients.
    pub fn shared(
        models: PathBuf,
        running: Chosen,
        refusals: Vec<Refusal>,
        tools: Tools,
    ) -> Arc<Shared<Self>> {
        let offer = crate::offer::standing(&running);
        Arc::new_cyclic(|me| {
            Shared::new(Self {
                offer,
                me: me.clone(),
                models,
                running,
                refusals,
                tools,
                watches: Watches::default(),
                since: now(),
                heard: now(),
                stage: Stage::Idle,
                why: None,
                microphone: None,
                default_microphone: None,
                spot: None,
                output: None,
                until: None,
                clips: Vec::new(),
                floor: None,
                ways: Vec::new(),
                costs: BTreeMap::new(),
                marks: BTreeMap::new(),
                kept: false,
                picked: None,
                playing: None,
                generation: 0,
                acted: Instant::now(),
            })
        })
    }

    /// The record, as every client is told it.
    pub fn record(&self) -> Value {
        let shown = matches!(self.stage, Stage::Ready | Stage::Picked);
        let clips: Vec<&str> = if shown {
            LETTERS[..self.ways.len()].to_vec()
        } else {
            Vec::new()
        };
        let picked = self
            .picked
            .map(|(letter, way)| json!({"clip": LETTERS[letter], "cleaner": way.name()}));
        let revealed = self.picked.map(|_| {
            let each: serde_json::Map<String, Value> = self
                .ways
                .iter()
                .zip(LETTERS)
                .map(|(way, letter)| (letter.to_owned(), json!(way.name())))
                .collect();
            Value::Object(each)
        });
        json!({
            "key": KEY,
            "kind": KIND,
            "heard": self.heard,
            "stage": self.stage.name(),
            "why": self.why,
            "microphone": self.microphone.as_ref().map(|microphone| &microphone.description),
            "spot": self.spot.as_ref().map(|spot| &spot.name),
            "spot_named": self.spot.as_ref().is_some_and(|spot| self.running.named(&spot.key)),
            "output": self.output,
            "seconds": SECONDS,
            "until": self.until,
            "clips": clips,
            "playing": self.playing.as_ref().map(|(letter, _)| LETTERS[*letter]),
            "picked": picked,
            "revealed": revealed,
            "costs": self.picked.map(|_| &self.costs),
            "marks": self.picked.and_then(|_| self.marked()),
            "running": self.runs().name(),
            "offer": self.offer,
        })
    }

    /// The cleaner the microphone Sound shows runs: the one recorded from,
    /// else the default as the last watch began, at the spot read then.
    fn runs(&self) -> Cleaner {
        let microphone = self
            .microphone
            .as_ref()
            .map(|microphone| microphone.name.as_str());
        self.running.cleaner(
            microphone.or(self.default_microphone.as_deref()),
            self.spot.as_ref(),
        )
    }

    /// Tells every watch the record changed.
    /// Reads the offer again, as it's come or gone, telling it if it moved.
    pub fn offer_moved(&mut self) {
        let offer = crate::offer::standing(&self.running);
        if offer != self.offer {
            self.offer = offer;
            self.changed();
        }
    }

    fn changed(&mut self) {
        self.heard = now();
        let record = self.record();
        self.watches.tell(KEY, &record);
    }

    /// Stops what's playing, if anything is.
    fn hush(&mut self) {
        if let Some((_, mut child)) = self.playing.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }

    /// Lets go of every clip and goes back to idle.
    fn forget(&mut self) {
        self.hush();
        self.generation += 1;
        self.stage = Stage::Idle;
        self.why = None;
        self.until = None;
        self.clips.clear();
        self.ways.clear();
        self.costs.clear();
        self.marks.clear();
        self.kept = false;
        self.picked = None;
    }

    /// Every way's marks, once there are any.
    fn marked(&self) -> Option<Value> {
        if self.marks.is_empty() {
            return None;
        }
        let each: serde_json::Map<String, Value> = self
            .marks
            .iter()
            .map(|(way, marks)| ((*way).to_owned(), marks.said()))
            .collect();
        Some(Value::Object(each))
    }

    /// Keeps the pick and every way's marks beside the pick's file, once
    /// both are in, and only once a recording.
    fn keep(&mut self) {
        let Some((_, way)) = self.picked else { return };
        if self.kept || self.marks.is_empty() {
            return;
        }
        let Some(file) = self.running.beside("scored") else {
            return;
        };
        self.kept = true;
        let line = json!({
            "at": now(),
            "microphone": self.microphone.as_ref().map(|microphone| &microphone.description),
            "spot": self.spot.as_ref().map(|spot| &spot.name),
            "output": self.output,
            "picked": way.name(),
            "marks": self.marked(),
            "costs": &self.costs,
        });
        // Its folder made here too, since a raw pick writes no pick file.
        let appended = file
            .parent()
            .map_or(Ok(()), std::fs::create_dir_all)
            .and_then(|()| OpenOptions::new().create(true).append(true).open(&file))
            .and_then(|mut kept| writeln!(kept, "{line}"));
        if let Err(why) = appended {
            eprintln!(
                "noise-cancelled: keeping the marks in {}: {why}",
                file.display()
            );
        }
    }

    /// Refuses to record, with why.
    fn refuse(&mut self, why: &str) {
        self.forget();
        self.stage = Stage::Refused;
        self.why = Some(why.to_owned());
    }

    /// The letter an act names, among the clips there are.
    fn letter(&self, said: &Value) -> Result<usize, String> {
        let clip = said["clip"].as_str().unwrap_or_default();
        LETTERS[..self.clips.len()]
            .iter()
            .position(|letter| *letter == clip)
            .ok_or_else(|| format!("there's no clip {clip:?} to hear"))
    }

    /// Does an act, saying why not when it can't be done.
    fn act(&mut self, said: &Value) -> Result<(), String> {
        self.acted = Instant::now();
        match said["act"].as_str().unwrap_or_default() {
            "record" => self.start_recording(said["microphone"].as_str()),
            "play" => {
                let letter = self.letter(said)?;
                self.play(letter)
            }
            "stop" => {
                self.hush();
                Ok(())
            }
            "pick" => {
                let letter = self.letter(said)?;
                self.pick(letter)
            }
            "discard" => {
                self.forget();
                Ok(())
            }
            "name" => self.name_spot(said["name"].as_str().unwrap_or_default()),
            "offered" => {
                crate::offer::taken(&self.running)?;
                self.offer_moved();
                Ok(())
            }
            other => Err(format!("no act {other:?}")),
        }
    }

    /// Names the spot the trial is at in Joel's words.
    fn name_spot(&mut self, name: &str) -> Result<(), String> {
        let spot = self.spot.clone().unwrap_or_else(|| self.running.spot());
        self.spot = Some(self.running.name(&spot, name)?);
        Ok(())
    }

    /// Records `microphone`, by its node name, or the default without one.
    fn start_recording(&mut self, microphone: Option<&str>) -> Result<(), String> {
        if matches!(self.stage, Stage::Recording | Stage::Cleaning) {
            return Err("it's already recording".to_owned());
        }
        if let Some(refusal) = self.refusals.iter().find(|refusal| refusal.holds()) {
            let why = refusal.why.clone();
            self.refuse(&why);
            return Err(why);
        }
        let dump = dumped(&self.tools.dump);
        let microphone = dump.as_ref().map_err(Clone::clone).and_then(|dump| {
            match microphone {
                Some(name) => named(dump, name),
                None => default_microphone(dump),
            }
            .ok_or_else(|| "there's no microphone to record".to_owned())
        });
        let microphone = match microphone {
            Ok(microphone) => microphone,
            Err(why) => {
                self.refuse(&why);
                return Err(why);
            }
        };
        self.forget();
        let generation = self.generation;
        let recorder = Command::new(&self.tools.record)
            .args(["--target", &microphone.name, "--rate", &RATE.to_string()])
            .args(["--channels", "1", "--format", "f32", "--raw", "-"])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|why| format!("recording: {why}"))?;
        self.stage = Stage::Recording;
        self.until = Some(now() + SECONDS * 1000);
        self.microphone = Some(microphone);
        self.spot = Some(self.running.spot());
        self.output = dump.ok().as_ref().and_then(default_output);
        let me = self.me.clone();
        let models = self.models.clone();
        std::thread::spawn(move || recorded(&me, generation, recorder, &models));
        Ok(())
    }

    fn play(&mut self, letter: usize) -> Result<(), String> {
        self.hush();
        // Asked again as each clip plays, since Joel may have moved to his
        // headset since he recorded.
        if let Ok(dump) = dumped(&self.tools.dump) {
            self.output = default_output(&dump);
        }
        let mut player = Command::new(&self.tools.play)
            .args(["--rate", &RATE.to_string(), "--channels", "1"])
            .args(["--format", "f32", "--raw", "-"])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|why| format!("playing: {why}"))?;
        if let Some(mut given) = player.stdin.take() {
            let samples = Arc::clone(&self.clips[letter].samples);
            std::thread::spawn(move || {
                let bytes: Vec<u8> = samples
                    .iter()
                    .flat_map(|sample| sample.to_le_bytes())
                    .collect();
                let _ = given.write_all(&bytes);
            });
        }
        self.playing = Some((letter, player));
        let me = self.me.clone();
        std::thread::spawn(move || listened(&me));
        Ok(())
    }

    fn pick(&mut self, letter: usize) -> Result<(), String> {
        if self.stage != Stage::Ready {
            return Err("there's nothing to pick from".to_owned());
        }
        self.hush();
        let way = self.clips[letter].way;
        if let Some(microphone) = self.microphone.as_ref() {
            let spot = self.spot.clone().unwrap_or_else(|| self.running.spot());
            if let Way::Cleaned(cleaner) = way {
                // Kept first, so the microphone calls hear is never the twin
                // cleaning some other way than the one picked.
                self.running.choose(&microphone.name, &spot, cleaner)?;
                default_microphone_is(&self.tools.metadata, &microphone.twin())?;
            } else {
                // Calls hear the microphone itself, though a twin was chosen
                // before, and Choose by Ear isn't offered for it there again.
                default_microphone_is(&self.tools.metadata, &microphone.name)?;
                crate::offer::settled(&self.running, &microphone.name, &spot.key)?;
            }
            // Either way, a call far louder or quieter than this room offers
            // Choose by Ear again.
            if let Some(floor) = self.floor {
                crate::room::kept(&self.running, &microphone.name, &spot.key, floor)?;
            }
        }
        eprintln!(
            "noise-cancelled: {} picked by ear, over {}",
            way.name(),
            self.ways
                .iter()
                .filter(|other| **other != way)
                .map(|other| other.name())
                .collect::<Vec<_>>()
                .join(", ")
        );
        self.clips.clear();
        self.picked = Some((letter, way));
        self.stage = Stage::Picked;
        self.keep();
        Ok(())
    }

    /// Takes the scorer's marks for the recording `generation` counted.
    /// Marks for a recording since let go are dropped.
    pub fn scored(&mut self, generation: u64, marks: BTreeMap<&'static str, Marks>) {
        if generation != self.generation || marks.is_empty() {
            return;
        }
        self.marks = marks;
        self.keep();
        self.changed();
    }

    /// Lets go of clips nobody has acted on for a while.
    pub fn forgotten(&mut self) {
        if self.stage == Stage::Ready && self.acted.elapsed() >= FORGOTTEN {
            self.forget();
            self.changed();
        }
    }
}

impl Answers for Trial {
    fn answer(&mut self, line: &str, connection: u64, told: &Sender<String>) -> Value {
        let Ok(said) = serde_json::from_str::<Value>(line) else {
            return error(&Value::Null, "unknown-ask", "not a JSON line");
        };
        let id = said.get("id").cloned().unwrap_or(Value::Null);
        let asked = said["ask"].as_str().unwrap_or_default();
        let kind = said["kind"].as_str().unwrap_or(KIND);
        if matches!(asked, "list" | "watch" | "get") && kind != KIND {
            let why = format!("this daemon keeps only {KIND}, not {kind}");
            return error(&id, "unknown-kind", &why);
        }
        if asked == "watch" {
            // Sound watches as it opens, which is when Joel may have moved
            // to another microphone since the last.
            self.default_microphone = dumped(&self.tools.dump)
                .ok()
                .as_ref()
                .and_then(default_microphone)
                .map(|microphone| microphone.name);
            if self.stage == Stage::Idle {
                self.spot = Some(self.running.spot());
            }
        }
        let keys = daemon::keys(&said);
        let named = keys
            .as_ref()
            .is_none_or(|keys| keys.iter().any(|key| key == KEY));
        let held = || {
            if named {
                vec![self.record()]
            } else {
                Vec::new()
            }
        };
        match asked {
            "hello" => json!({
                "id": id, "version": VERSION, "domain": "noise cancelling",
                "state": "answering", "since": self.since,
            }),
            "get" | "list" => json!({"id": id, "records": held()}),
            "watch" => {
                let records = held();
                self.watches.watch(connection, &id, keys, told);
                json!({"id": id, "records": records})
            }
            "unwatch" => {
                self.watches.unwatch(connection, &id);
                json!({"id": id, "unwatched": true})
            }
            "act" => {
                let done = self.act(&said);
                self.changed();
                match done {
                    Ok(()) => json!({"id": id, "taken": true}),
                    Err(why) => error(&id, "refused", &why),
                }
            }
            "health" => json!({
                "id": id, "version": VERSION, "state": "answering",
                "stage": self.stage.name(), "watching": self.watches.len(),
            }),
            other => error(&id, "unknown-ask", &format!("no ask {other:?}")),
        }
    }

    fn gone(&mut self, connection: u64) {
        self.watches.gone(connection);
    }
}

/// Hears the recording out, cleans it every way, and offers the clips.
fn recorded(me: &Weak<Shared<Trial>>, generation: u64, mut recorder: Child, models: &Path) {
    let wanted = usize::try_from(SECONDS * u64::from(RATE) * 4).unwrap_or(0);
    let mut bytes = vec![0; wanted];
    let heard = recorder
        .stdout
        .take()
        .ok_or_else(|| "the recording gave nothing".to_owned())
        .and_then(|mut out| {
            out.read_exact(&mut bytes)
                .map_err(|why| format!("recording: {why}"))
        });
    let _ = recorder.kill();
    let _ = recorder.wait();
    let Some(trial) = me.upgrade() else { return };
    if let Err(why) = heard {
        let mut trial = trial.lock();
        if trial.generation == generation {
            eprintln!("noise-cancelled: {why}");
            trial.refuse("the microphone couldn't be recorded");
            trial.changed();
        }
        return;
    }
    {
        let mut trial = trial.lock();
        if trial.generation != generation {
            return;
        }
        trial.stage = Stage::Cleaning;
        trial.until = None;
        trial.changed();
    }
    let samples: Vec<f32> = bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|sample| f32::from_le_bytes(*sample))
        .collect();
    // Measured raw, since the clips are made as loud as each other after.
    let floor = crate::room::of(&samples);
    let cleaned = ways::every_way(models, &samples);
    let mut trial = trial.lock();
    if trial.generation != generation {
        return;
    }
    trial.floor = floor;
    match cleaned {
        Ok(clips) => {
            let order = shuffled(now());
            trial.costs = clips
                .iter()
                .map(|(way, _, cost)| (way.name(), *cost))
                .collect();
            let mut clips: Vec<Option<(Way, Vec<f32>)>> = clips
                .into_iter()
                .map(|(way, samples, _)| Some((way, samples)))
                .collect();
            trial.clips = order
                .iter()
                .filter_map(|index| clips[*index].take())
                .map(|(way, samples)| Clip {
                    way,
                    samples: Arc::new(samples),
                })
                .collect();
            trial.ways = trial.clips.iter().map(|clip| clip.way).collect();
            trial.stage = Stage::Ready;
            trial.acted = Instant::now();
        }
        Err(why) => {
            eprintln!("noise-cancelled: cleaning the recording: {why}");
            trial.refuse("the recording couldn't be cleaned");
        }
    }
    trial.changed();
    // Scored once the clips are offered, so he can listen while it works,
    // with the lock let go; the marks wait for his pick to be shown.
    let heard: Vec<(&'static str, Arc<Vec<f32>>)> = trial
        .clips
        .iter()
        .map(|clip| (clip.way.name(), Arc::clone(&clip.samples)))
        .collect();
    drop(trial);
    if heard.is_empty() {
        return;
    }
    let marks = score::every_way(models, &heard);
    if let Some(trial) = me.upgrade() {
        trial.lock().scored(generation, marks);
    }
}

/// Watches a clip play, and says so once it's done.
fn listened(me: &Weak<Shared<Trial>>) {
    loop {
        std::thread::sleep(LISTENED);
        let Some(trial) = me.upgrade() else { return };
        let mut trial = trial.lock();
        let done = match trial.playing.as_mut() {
            None => return,
            Some((_, child)) => !matches!(child.try_wait(), Ok(None)),
        };
        if done {
            trial.playing = None;
            trial.changed();
            return;
        }
    }
}

/// The four ways in an order of their own, from `seed`: a Fisher-Yates
/// shuffle by a small generator, enough that Joel can't know the order.
pub fn shuffled(seed: u64) -> [usize; 4] {
    let mut order = [0, 1, 2, 3];
    let mut state = seed | 1;
    for last in (1..order.len()).rev() {
        // xorshift64.
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        let bound = u64::try_from(last + 1).unwrap_or(1);
        let index = usize::try_from(state % bound).unwrap_or(0);
        order.swap(last, index);
    }
    order
}

/// Answers the launcher until systemd stops it.
pub fn serve(socket: &Path, models: &Path, running: Chosen, refusals: &Path) -> Result<(), String> {
    let refusals = Refusal::read(refusals)?;
    eprintln!(
        "noise-cancelled: choosing by ear, {} running",
        running.cleaner(None, None)
    );
    let offer = running.beside("offer");
    let trial = Trial::shared(models.to_path_buf(), running, refusals, Tools::default());
    crate::offer::told_to(&trial, offer.as_deref());
    let listener = crate::handed().map_or_else(|| daemon::listen(socket), Ok)?;
    {
        let trial = Arc::clone(&trial);
        std::thread::spawn(move || daemon::answer_all(&trial, &listener));
    }
    loop {
        std::thread::sleep(Duration::from_secs(10));
        trial.lock().forgotten();
    }
}

#[cfg(test)]
#[path = "trial_tests.rs"]
mod tests;
