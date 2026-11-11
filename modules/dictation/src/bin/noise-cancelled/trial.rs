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

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Weak};
use std::time::{Duration, Instant};

use dictation::daemon::{self, Answers, Shared, VERSION, Watches, error, now};
use serde_json::{Value, json};

use crate::denoise::{Cleaner, FRAME, Model, RATE};

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

/// What a clip was cleaned by: one of the cleaners, or none.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Way {
    Raw,
    Cleaned(Cleaner),
}

impl Way {
    /// Every way, in the order they're cleaned, for the tests to offer.
    #[cfg(test)]
    pub const ALL: [Self; 4] = [
        Self::Raw,
        Self::Cleaned(Cleaner::DeepFilterNet),
        Self::Cleaned(Cleaner::DeepFilterNetGentle),
        Self::Cleaned(Cleaner::RNNoise),
    ];

    /// Its name, as the record says it.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Raw => "raw",
            Self::Cleaned(cleaner) => cleaner.name(),
        }
    }
}

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

/// A microphone as PipeWire names it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Microphone {
    /// Its node's name, which a recording targets.
    pub name: String,
    /// What it's called where Joel reads it.
    pub description: String,
}

impl Microphone {
    /// Its noise-cancelled twin's node name, as pipe.rs names it.
    pub fn twin(&self) -> String {
        format!("{}{}", crate::pipe::OURS, self.name)
    }
}

/// Something that, while it exits 0, means nothing may be recorded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Refusal {
    pub command: Vec<String>,
    pub why: String,
}

impl Refusal {
    /// Each refusal a file's JSON lists, as `[{"command": [...], "why": "..."}]`.
    pub fn listed(text: &str) -> Result<Vec<Self>, String> {
        let listed: Value =
            serde_json::from_str(text).map_err(|why| format!("reading the refusals: {why}"))?;
        let strings = |value: &Value| -> Option<Vec<String>> {
            value
                .as_array()?
                .iter()
                .map(|word| word.as_str().map(str::to_owned))
                .collect()
        };
        listed
            .as_array()
            .ok_or_else(|| "the refusals aren't a list".to_owned())?
            .iter()
            .map(|refusal| {
                let command = strings(&refusal["command"]);
                let why = refusal["why"].as_str().map(str::to_owned);
                command
                    .zip(why)
                    .map(|(command, why)| Self { command, why })
                    .ok_or_else(|| format!("a refusal needs a command and a why: {refusal}"))
            })
            .collect()
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
    running: Cleaner,
    refusals: Vec<Refusal>,
    tools: Tools,
    watches: Watches,
    since: u64,
    heard: u64,
    stage: Stage,
    why: Option<String>,
    microphone: Option<Microphone>,
    until: Option<u64>,
    /// The clips by letter, while ready.
    clips: Vec<Clip>,
    /// Each letter's way, kept past a pick for the reveal.
    ways: Vec<Way>,
    costs: BTreeMap<&'static str, f64>,
    picked: Option<(usize, Way)>,
    playing: Option<(usize, Child)>,
    /// Counts each recording and discard, so a thread whose recording has
    /// been let go drops what it finishes.
    generation: u64,
    acted: Instant,
}

impl Trial {
    /// The A/B, shared, ready for clients.
    pub fn shared(
        models: PathBuf,
        running: Cleaner,
        refusals: Vec<Refusal>,
        tools: Tools,
    ) -> Arc<Shared<Self>> {
        Arc::new_cyclic(|me| {
            Shared::new(Self {
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
                until: None,
                clips: Vec::new(),
                ways: Vec::new(),
                costs: BTreeMap::new(),
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
            "seconds": SECONDS,
            "until": self.until,
            "clips": clips,
            "playing": self.playing.as_ref().map(|(letter, _)| LETTERS[*letter]),
            "picked": picked,
            "revealed": revealed,
            "costs": self.picked.map(|_| &self.costs),
            "running": self.running.name(),
        })
    }

    /// Tells every watch the record changed.
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
        self.picked = None;
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
            "record" => self.start_recording(),
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
            other => Err(format!("no act {other:?}")),
        }
    }

    fn start_recording(&mut self) -> Result<(), String> {
        if matches!(self.stage, Stage::Recording | Stage::Cleaning) {
            return Err("it's already recording".to_owned());
        }
        if let Some(refusal) = self.refusals.iter().find(|refusal| refused(refusal)) {
            let why = refusal.why.clone();
            self.refuse(&why);
            return Err(why);
        }
        let microphone = dumped(&self.tools.dump).and_then(|dump| {
            default_microphone(&dump).ok_or_else(|| "there's no microphone to record".to_owned())
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
        let me = self.me.clone();
        let models = self.models.clone();
        std::thread::spawn(move || recorded(&me, generation, recorder, &models));
        Ok(())
    }

    fn play(&mut self, letter: usize) -> Result<(), String> {
        self.hush();
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
        let twin = self.microphone.as_ref().map(Microphone::twin);
        if let (Way::Cleaned(_), Some(twin)) = (way, twin) {
            default_microphone_is(&self.tools.metadata, &twin)?;
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
        Ok(())
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

/// Whether a refusal holds now: its command ran and exited 0.
fn refused(refusal: &Refusal) -> bool {
    let Some((program, arguments)) = refusal.command.split_first() else {
        return false;
    };
    Command::new(program)
        .args(arguments)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

/// What PipeWire holds now, as pw-dump says it.
fn dumped(dump: &str) -> Result<Value, String> {
    let out = Command::new(dump)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .map_err(|why| format!("asking PipeWire: {why}"))?;
    serde_json::from_slice(&out.stdout).map_err(|why| format!("reading PipeWire: {why}"))
}

/// The default microphone in a dump: the one calls hear, its own node
/// when the default is its noise-cancelled twin, so the A/B starts from
/// the microphone as it is.
pub fn default_microphone(dump: &Value) -> Option<Microphone> {
    let objects = dump.as_array()?;
    let default = objects
        .iter()
        .filter(|object| object["props"]["metadata.name"] == "default")
        .filter_map(|object| object["metadata"].as_array())
        .flatten()
        .find(|entry| entry["key"] == "default.audio.source")?;
    let named = default["value"]["name"].as_str()?;
    let name = named.strip_prefix(crate::pipe::OURS).unwrap_or(named);
    objects.iter().find_map(|object| {
        let props = &object["info"]["props"];
        (props["node.name"] == name && props["media.class"] == "Audio/Source").then(|| Microphone {
            name: name.to_owned(),
            description: props["node.description"]
                .as_str()
                .unwrap_or(name)
                .to_owned(),
        })
    })
}

/// Makes a node the default microphone, as choosing it in Sound does:
/// `WirePlumber`'s own configured default, which it keeps.
fn default_microphone_is(metadata: &str, name: &str) -> Result<(), String> {
    let value = json!({"name": name}).to_string();
    let status = Command::new(metadata)
        .args([
            "-n",
            "default",
            "0",
            "default.configured.audio.source",
            &value,
        ])
        .arg("Spa:String:JSON")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|why| format!("choosing the microphone: {why}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "choosing the microphone: pw-metadata said {status}"
        ))
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
    let cleaned = clean_every_way(models, &samples);
    let mut trial = trial.lock();
    if trial.generation != generation {
        return;
    }
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
}

/// The recording, every way, with each way's share of one core while it
/// cleaned: the processor time this thread spent, over the clip's length.
fn clean_every_way(models: &Path, samples: &[f32]) -> Result<Vec<(Way, Vec<f32>, f64)>, String> {
    #[expect(
        clippy::cast_precision_loss,
        reason = "a clip is far under 2^52 samples"
    )]
    let length = samples.len() as f64 / f64::from(RATE);
    let mut clips = vec![(Way::Raw, samples.to_vec(), 0.0)];
    for cleaner in Cleaner::ALL {
        // Loaded outside the time it's charged, since the microphones load
        // it once and keep it while a call goes on.
        let mut denoiser = Model::load(models, cleaner)?.denoiser()?;
        let started = spent();
        let mut cleaned = Vec::with_capacity(samples.len());
        for chunk in samples.chunks(FRAME) {
            let mut frame = [0.0; FRAME];
            frame[..chunk.len()].copy_from_slice(chunk);
            cleaned.extend(denoiser.clean(&frame)?);
        }
        cleaned.truncate(samples.len());
        let cost = spent().saturating_sub(started).as_secs_f64() / length;
        clips.push((Way::Cleaned(cleaner), cleaned, cost));
    }
    Ok(clips)
}

/// The processor time this thread has spent, from the kernel's
/// scheduler statistics, which count it to the nanosecond.
fn spent() -> Duration {
    std::fs::read_to_string("/proc/thread-self/schedstat")
        .ok()
        .and_then(|stat| stat.split_whitespace().next()?.parse().ok())
        .map_or(Duration::ZERO, Duration::from_nanos)
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
pub fn serve(
    socket: &Path,
    models: &Path,
    running: Cleaner,
    refusals: &Path,
) -> Result<(), String> {
    let refusals = std::fs::read_to_string(refusals)
        .map_err(|why| format!("reading {}: {why}", refusals.display()))
        .and_then(|text| Refusal::listed(&text))?;
    let trial = Trial::shared(models.to_path_buf(), running, refusals, Tools::default());
    let listener = match crate::handed() {
        Some(listener) => listener,
        None => daemon::listen(socket)?,
    };
    {
        let trial = Arc::clone(&trial);
        std::thread::spawn(move || daemon::answer_all(&trial, &listener));
    }
    eprintln!("noise-cancelled: choosing by ear, {running} running");
    loop {
        std::thread::sleep(Duration::from_secs(10));
        trial.lock().forgotten();
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;

    use super::*;

    fn trial(refusals: Vec<Refusal>) -> Arc<Shared<Trial>> {
        let tools = Tools {
            dump: "false".to_owned(),
            record: "false".to_owned(),
            play: "true".to_owned(),
            metadata: "true".to_owned(),
        };
        Trial::shared(PathBuf::new(), Cleaner::DeepFilterNet, refusals, tools)
    }

    fn ask(trial: &Arc<Shared<Trial>>, line: &str) -> Value {
        let (told, _) = mpsc::channel();
        trial.lock().answer(line, 0, &told)
    }

    /// A trial with its clips recorded and cleaned, as the recording
    /// thread leaves it.
    fn ready(trial: &Arc<Shared<Trial>>) {
        let mut held = trial.lock();
        held.stage = Stage::Ready;
        held.microphone = Some(Microphone {
            name: "mv7".to_owned(),
            description: "Shure MV7".to_owned(),
        });
        held.clips = [Way::ALL[2], Way::ALL[0], Way::ALL[3], Way::ALL[1]]
            .into_iter()
            .map(|way| Clip {
                way,
                samples: Arc::new(vec![0.0; 10]),
            })
            .collect();
        held.ways = held.clips.iter().map(|clip| clip.way).collect();
        held.costs = Way::ALL.iter().map(|way| (way.name(), 0.1)).collect();
    }

    #[test]
    fn it_says_the_cleaner_running_and_nothing_else_while_idle() {
        let said = ask(
            &trial(Vec::new()),
            r#"{"id":1,"ask":"watch","kind":"trial"}"#,
        );
        let record = &said["records"][0];
        assert_eq!(record["key"], KEY);
        assert_eq!(record["stage"], "idle");
        assert_eq!(record["running"], "deepfilternet");
        assert_eq!(record["seconds"], SECONDS);
        assert_eq!(record["clips"], json!([]));
        assert_eq!(record["revealed"], Value::Null);
    }

    #[test]
    fn a_refusal_that_holds_says_why_and_records_nothing() {
        let refusals = vec![
            Refusal {
                command: vec!["false".to_owned()],
                why: "a meeting is near".to_owned(),
            },
            Refusal {
                command: vec!["true".to_owned()],
                why: "a call is on".to_owned(),
            },
        ];
        let trial = trial(refusals);
        let said = ask(&trial, r#"{"id":2,"ask":"act","act":"record"}"#);
        assert_eq!(said["error"]["code"], "refused");
        assert_eq!(said["error"]["message"], "a call is on");
        let record = trial.lock().record();
        assert_eq!(record["stage"], "refused");
        assert_eq!(record["why"], "a call is on");
    }

    #[test]
    fn refusals_are_read_as_nix_writes_them() {
        let listed = Refusal::listed(r#"[{"command": ["a-call-is-on"], "why": "a call is on"}]"#);
        assert_eq!(
            listed,
            Ok(vec![Refusal {
                command: vec!["a-call-is-on".to_owned()],
                why: "a call is on".to_owned(),
            }])
        );
        assert!(Refusal::listed(r#"[{"why": "no command"}]"#).is_err());
    }

    #[test]
    fn it_wont_record_twice_at_once() {
        let trial = trial(Vec::new());
        trial.lock().stage = Stage::Recording;
        let said = ask(&trial, r#"{"id":3,"ask":"act","act":"record"}"#);
        assert_eq!(said["error"]["message"], "it's already recording");
    }

    #[test]
    fn the_clips_are_offered_blind() {
        let trial = trial(Vec::new());
        ready(&trial);
        let record = trial.lock().record();
        assert_eq!(record["clips"], json!(["A", "B", "C", "D"]));
        assert_eq!(record["revealed"], Value::Null);
        assert_eq!(record["costs"], Value::Null);
        assert_eq!(record["microphone"], "Shure MV7");
    }

    #[test]
    fn a_pick_reveals_every_clip_and_lets_them_go() {
        let trial = trial(Vec::new());
        ready(&trial);
        let said = ask(&trial, r#"{"id":4,"ask":"act","act":"pick","clip":"D"}"#);
        assert_eq!(said["taken"], true, "{said}");
        let record = trial.lock().record();
        assert_eq!(record["stage"], "picked");
        assert_eq!(
            record["picked"],
            json!({"clip": "D", "cleaner": "deepfilternet"})
        );
        assert_eq!(
            record["revealed"],
            json!({"A": "deepfilternet-gentle", "B": "raw", "C": "rnnoise", "D": "deepfilternet"})
        );
        assert_eq!(record["costs"]["rnnoise"], 0.1);
        assert!(trial.lock().clips.is_empty());
    }

    #[test]
    fn a_clip_nobody_has_cant_be_played_or_picked() {
        let trial = trial(Vec::new());
        let said = ask(&trial, r#"{"id":5,"ask":"act","act":"play","clip":"A"}"#);
        assert_eq!(said["error"]["message"], r#"there's no clip "A" to hear"#);
        ready(&trial);
        let said = ask(&trial, r#"{"id":6,"ask":"act","act":"pick","clip":"E"}"#);
        assert_eq!(said["error"]["code"], "refused");
    }

    #[test]
    fn playing_says_which_clip_and_a_discard_forgets_them() {
        let trial = trial(Vec::new());
        ready(&trial);
        let said = ask(&trial, r#"{"id":7,"ask":"act","act":"play","clip":"B"}"#);
        assert_eq!(said["taken"], true, "{said}");
        assert_eq!(trial.lock().record()["playing"], "B");
        ask(&trial, r#"{"id":8,"ask":"act","act":"discard"}"#);
        let record = trial.lock().record();
        assert_eq!(record["stage"], "idle");
        assert_eq!(record["playing"], Value::Null);
        assert!(trial.lock().clips.is_empty());
    }

    #[test]
    fn every_watch_hears_each_act() {
        let trial = trial(Vec::new());
        let (told, heard) = mpsc::channel();
        trial
            .lock()
            .answer(r#"{"id":9,"ask":"watch","kind":"trial"}"#, 1, &told);
        ready(&trial);
        ask(&trial, r#"{"id":10,"ask":"act","act":"discard"}"#);
        let line: Value = serde_json::from_str(&heard.recv().unwrap()).unwrap();
        assert_eq!(line["id"], 9);
        assert_eq!(line["changed"][0]["stage"], "idle");
    }

    #[test]
    fn the_default_microphone_is_found_through_its_twin() {
        let dump = json!([
            {"type": "PipeWire:Interface:Metadata", "props": {"metadata.name": "default"},
             "metadata": [{"subject": 0, "key": "default.audio.source",
                           "value": {"name": "noise-cancelled.mv7"}}]},
            {"info": {"props": {"node.name": "noise-cancelled.mv7",
                                "node.description": "Shure MV7, Noise Cancelled",
                                "media.class": "Audio/Source"}}},
            {"info": {"props": {"node.name": "mv7", "node.description": "Shure MV7",
                                "media.class": "Audio/Source"}}},
        ]);
        let microphone = default_microphone(&dump).expect("a microphone");
        assert_eq!(microphone.name, "mv7");
        assert_eq!(microphone.description, "Shure MV7");
        assert_eq!(microphone.twin(), "noise-cancelled.mv7");
    }

    #[test]
    fn every_shuffle_offers_each_way_once() {
        let mut seen = std::collections::HashSet::new();
        for seed in 0..200 {
            let mut order = shuffled(seed * 7919);
            seen.insert(order);
            order.sort_unstable();
            assert_eq!(order, [0, 1, 2, 3], "seed {seed}");
        }
        assert!(seen.len() > 12, "only {} orders came", seen.len());
    }
}
