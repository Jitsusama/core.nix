//! meaning: what the launcher's things and what's typed into it mean, and
//! what pictures show, on the NPU, so the launcher can list what Joel
//! meant beside what he typed (the glossary's search by meaning), and the
//! screenshots that look like what he typed beside those named it. It
//! only adds: the shell lists what it answers below everything typing
//! found, never above.
//!
//! ```text
//! meaning serve SOCKET MODELS MEANINGS [SIGHT LOOKS FOLDER...]
//! meaning near MODELS WORDS [THING...]
//! meaning look SIGHT WORDS PICTURE...
//! ```
//!
//! `serve` answers the shell at SOCKET (known.rs), keeping each thing's
//! meaning in MEANINGS (store.rs), so a restart embeds only what's new.
//! Given SIGHT, SigLIP 2's folder (sight.rs), it looks at every picture in
//! each FOLDER too, as each comes, keeping what each looks like in LOOKS.
//! One worker holds the models and does one thing at a time, a query
//! before any thing or picture waiting, so a key waits behind at most one
//! of them; things and pictures wait while anything records from a
//! microphone (quiet.rs). Each model is loaded once there's something for
//! it to do, and all are let go once nothing has come for a while, so
//! they hold no memory at rest.
//!
//! `near` prints how near each THING, or each line read when none is
//! given, means WORDS, nearest first, with what each step took, for
//! measuring the model and setting the floor; `look` prints how near
//! each PICTURE is to WORDS, alike. MODELS is the folder
//! modules/meaning/models.nix makes for bge, SIGHT the one it makes for
//! SigLIP 2. `MEANING_DEVICE` names the device, the NPU unless it says
//! otherwise, falling back to the CPU for words but never for looking at
//! pictures, and `MEANING_CACHE` where
//! `OpenVINO` keeps what it compiles.

mod known;
mod quiet;
mod sight;
mod store;
mod text;

use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};

use dictation::daemon::{self, Shared};
use dictation::openvino::Core;
use dictation::picture::Picture;
use dictation::watch::{self, Change};

use crate::known::{Among, Job, Known};
use crate::quiet::Quiet;
use crate::sight::{Looks, Words};
use crate::store::Store;
use crate::text::{Texts, near};
use terminal_kit::{Exit, Failure, Help};

fn help() -> Help {
    Help::new("meaning", "finds the launcher's things by what they mean")
        .example("meaning near ~/m 'a note' notes mail", "how near each is")
        .example("meaning look ~/s 'a cat' a.png", "how near a.png is")
        .subcommand(
            Help::new("serve", "answers the shell, keeping each meaning")
                .argument("<socket>", "where the shell asks")
                .argument("<models>", "the words' model folder")
                .argument("<meanings>", "where each thing's meaning is kept")
                .argument("[sight]", "the pictures' model folder")
                .argument("[looks]", "where each picture's look is kept")
                .argument("[folder]...", "the folders of pictures to look at")
                .example("meaning serve S ~/m ~/k", "what its unit runs")
                .exit(Exit::Failed, "a model or the socket failed"),
        )
        .subcommand(
            Help::new("near", "how near each thing means the words")
                .argument("<models>", "the words' model folder")
                .argument("<words>", "what's typed")
                .argument("[thing]...", "what to measure; each line read if none")
                .example("meaning near ~/m 'a note' notes mail", "nearest first")
                .exit(Exit::Failed, "the model failed"),
        )
        .subcommand(
            Help::new("look", "how near each picture is to the words")
                .argument("<sight>", "the pictures' model folder")
                .argument("<words>", "what's typed")
                .argument("<picture>...", "the pictures to measure")
                .example("meaning look ~/s 'a cat' a.png", "nearest first")
                .exit(Exit::Failed, "the model failed or a picture can't be read"),
        )
        .exit(Exit::Failed, "a model, a picture or the socket failed")
}

/// How long the models are kept once nothing's been asked, so a launcher
/// opened again soon finds them loaded.
const UNUSED: Duration = Duration::from_mins(5);

/// How many things or pictures are done between saves while many wait,
/// so a stop loses little.
const SAVED_EVERY: usize = 64;

fn main() -> ExitCode {
    terminal_kit::main(&help(), env!("CARGO_PKG_VERSION"), |context| {
        act(&context.args)
    })
}

fn act(args: &[String]) -> Result<(), Failure> {
    let done = match args.split_first() {
        Some((ask, [socket, models, meanings])) if ask == "serve" => serve(
            Path::new(socket),
            Path::new(models),
            Path::new(meanings),
            None,
        ),
        Some((ask, [socket, models, meanings, sight, looks, folders @ ..])) if ask == "serve" => {
            let folders: Vec<PathBuf> = folders.iter().map(PathBuf::from).collect();
            serve(
                Path::new(socket),
                Path::new(models),
                Path::new(meanings),
                Some((Path::new(sight), Path::new(looks), &folders)),
            )
        }
        Some((ask, [models, words, things @ ..])) if ask == "near" => {
            print_near(Path::new(models), words, things)
        }
        Some((ask, [sight, words, pictures @ ..])) if ask == "look" && !pictures.is_empty() => {
            print_look(Path::new(sight), words, pictures)
        }
        // The kit has refused a word no subcommand answers to, so only a
        // subcommand given too few or too many words is left.
        Some((first, _)) => {
            return Err(Failure::new(Exit::Usage, "those aren't the words it takes")
                .try_this(format!("meaning {first} --help")));
        }
        None => {
            return Err(
                Failure::new(Exit::Usage, "it needs a subcommand").try_this("meaning --help")
            );
        }
    };
    done.map_err(|why| Failure::new(Exit::Failed, why))
}

/// The device named for the models, the NPU unless it says otherwise.
fn wanted() -> String {
    std::env::var("MEANING_DEVICE").unwrap_or_else(|_| "NPU".to_owned())
}

/// What `load` makes on the device wanted, or, where it `falls_back`, on
/// the CPU when that one can't take it, as on a machine with no NPU, with
/// the device it's on.
fn on_device<T>(
    core: &Core,
    falls_back: bool,
    load: impl Fn(&Core, &str) -> Result<T, String>,
) -> Result<(T, String), String> {
    let device = wanted();
    match load(core, &device) {
        Err(why) if falls_back && device != "CPU" => {
            eprintln!("meaning: on the CPU, since the {device} wouldn't: {why}");
            Ok((load(core, "CPU")?, "CPU".to_owned()))
        }
        loaded => loaded.map(|made| (made, device)),
    }
}

/// `OpenVINO`, with the cache `MEANING_CACHE` names.
fn core() -> Result<Core, String> {
    let cache = std::env::var_os("MEANING_CACHE").map(PathBuf::from);
    Core::new(cache.as_deref())
}

/// Loads with `load`, saying in the log what it was and how long it took.
fn timed<T>(
    core: &Core,
    what: &str,
    load: impl Fn(&Core, &str) -> Result<T, String>,
) -> Result<(T, String), String> {
    timed_on(core, what, true, load)
}

/// Loads with `load` as `timed` does, on the CPU too only where it
/// `falls_back`.
fn timed_on<T>(
    core: &Core,
    what: &str,
    falls_back: bool,
    load: impl Fn(&Core, &str) -> Result<T, String>,
) -> Result<(T, String), String> {
    let began = Instant::now();
    let (made, device) = on_device(core, falls_back, load)?;
    eprintln!(
        "meaning: {what} loaded on the {device} in {} ms",
        began.elapsed().as_millis()
    );
    Ok((made, device))
}

/// The models the worker uses, each loaded once it's needed, and let go
/// together.
struct Models {
    texts_from: PathBuf,
    sight_from: Option<PathBuf>,
    texts: Option<Texts>,
    words: Option<Words>,
    looks: Option<Looks>,
    // Last, so it's let go after what it compiled.
    core: Option<Core>,
}

impl Models {
    fn new(texts_from: &Path, sight_from: Option<&Path>) -> Self {
        Self {
            texts_from: texts_from.to_owned(),
            sight_from: sight_from.map(Path::to_owned),
            texts: None,
            words: None,
            looks: None,
            core: None,
        }
    }

    fn loaded(&self) -> bool {
        self.core.is_some()
    }

    fn let_go(&mut self) {
        self.texts = None;
        self.words = None;
        self.looks = None;
        self.core = None;
    }

    fn core(&mut self) -> Result<&Core, String> {
        if self.core.is_none() {
            self.core = Some(core()?);
        }
        self.core
            .as_ref()
            .ok_or_else(|| "OpenVINO wasn't started".to_owned())
    }

    fn sight(&self) -> Result<PathBuf, String> {
        self.sight_from
            .clone()
            .ok_or_else(|| "no pictures are looked at here".to_owned())
    }

    /// bge, for the launcher's things and what's typed of them.
    fn texts(&mut self, known: &Shared<Known>) -> Result<&mut Texts, String> {
        if self.texts.is_none() {
            let folder = self.texts_from.clone();
            let (texts, device) = timed(self.core()?, "bge", |core, device| {
                Texts::load(core, &folder, device)
            })?;
            known.lock().device = device;
            self.texts = Some(texts);
        }
        self.texts
            .as_mut()
            .ok_or_else(|| "bge wasn't loaded".to_owned())
    }

    /// SigLIP's text tower, for what's typed of pictures.
    fn words(&mut self) -> Result<&mut Words, String> {
        if self.words.is_none() {
            let folder = self.sight()?;
            let (words, _) = timed(self.core()?, "SigLIP's words", |core, device| {
                Words::load(core, &folder, device)
            })?;
            self.words = Some(words);
        }
        self.words
            .as_mut()
            .ok_or_else(|| "SigLIP's words weren't loaded".to_owned())
    }

    /// SigLIP's vision tower, for the pictures.
    /// SigLIP's vision tower, only on the device wanted, never the CPU
    /// in its place: looking through every picture there would take the
    /// processor from Joel's work in the background, where a search's few
    /// words don't.
    fn looks(&mut self) -> Result<&mut Looks, String> {
        if self.looks.is_none() {
            let folder = self.sight()?;
            let (looks, _) = timed_on(self.core()?, "SigLIP's looks", false, |core, device| {
                Looks::load(core, &folder, device)
            })?;
            self.looks = Some(looks);
        }
        self.looks
            .as_mut()
            .ok_or_else(|| "SigLIP's looks weren't loaded".to_owned())
    }
}

/// Answers the shell, embedding what it's told and asked and looking at
/// the pictures in the folders given, until the socket can't be listened
/// on.
fn serve(
    socket: &Path,
    models: &Path,
    meanings: &Path,
    pictures: Option<(&Path, &Path, &[PathBuf])>,
) -> Result<(), String> {
    let (wake, woken) = mpsc::channel();
    let store = Store::read(meanings, &models.to_string_lossy());
    let looks = pictures.map(|(sight, looks, _)| Store::read(looks, &sight.to_string_lossy()));
    let known = Arc::new(Shared::new(Known::new(store, looks, wake)));
    if let Some((_, _, folders)) = pictures {
        hear(&known, folders);
    }
    let listener = daemon::listen(socket)?;
    {
        let known = Arc::clone(&known);
        std::thread::spawn(move || daemon::answer_all(&known, &listener));
    }
    {
        let held = known.lock();
        eprintln!(
            "meaning: answering, {} meanings kept, {} pictures to look at",
            held.store.len(),
            held.pictures_waiting()
        );
    }
    let mut models = Models::new(models, pictures.map(|(sight, _, _)| sight));
    work(&known, &woken, &mut models)
}

/// Hears pictures come into each folder and go, then takes those there
/// now, so none written between is missed. A folder that can't be made
/// or heard is said once and left out.
fn hear(known: &Arc<Shared<Known>>, folders: &[PathBuf]) {
    let (changes, heard) = mpsc::channel();
    for folder in folders {
        let heeded = std::fs::create_dir_all(folder)
            .map_err(|why| format!("making {}: {why}", folder.display()))
            .and_then(|()| watch::watch(folder, changes.clone()));
        if let Err(why) = heeded {
            eprintln!("meaning: no pictures from {}: {why}", folder.display());
        }
    }
    known.lock().look_through(folders);
    let known = Arc::clone(known);
    std::thread::spawn(move || {
        for change in heard {
            match change {
                Change::Came(path) => known.lock().came(&path),
                Change::Went(path) => known.lock().went(&path),
            }
        }
    });
}

/// What the worker has done since it last had nothing to do, said once
/// it has, so the journal (and the checks) see each pass end.
#[derive(Default)]
struct Pass {
    things: usize,
    pictures: usize,
    done: usize,
}

impl Pass {
    /// Counts one more done, saying whether it's time to save.
    fn one(&mut self) -> bool {
        self.done += 1;
        self.done.is_multiple_of(SAVED_EVERY)
    }
}

/// The worker: one job at a time, each from what's shared, waiting for a
/// wake when there's none.
fn work(known: &Shared<Known>, woken: &Receiver<()>, models: &mut Models) -> Result<(), String> {
    let ended = || "the socket's listener ended".to_owned();
    let mut quiet = Quiet::default();
    let mut pass = Pass::default();
    loop {
        let job = known.lock().next();
        match job {
            Some(Job::Query(query)) => {
                let began = Instant::now();
                let meant = match query.among {
                    Among::Things => models
                        .texts(known)
                        .and_then(|texts| texts.embed(&query.words, true)),
                    Among::Pictures => models.words().and_then(|words| words.embed(&query.words)),
                };
                match meant {
                    Ok(meaning) => {
                        let took = began.elapsed().as_millis();
                        let mut held = known.lock();
                        held.last_ms = Some(took);
                        held.answer_query(&query, &meaning);
                    }
                    Err(why) => {
                        eprintln!("meaning: {:?} wasn't embedded: {why}", query.words);
                        let line = daemon::error(&query.id, "failed", &why);
                        let _ = query.told.send(line.to_string());
                    }
                }
            }
            Some(job @ (Job::Embed(_) | Job::Look(..))) => {
                let recording = quiet.recording();
                known.lock().held_back = recording;
                if recording {
                    // A query still wakes it; the rest waits for quiet.
                    match woken.recv_timeout(quiet::KEPT) {
                        Ok(()) | Err(RecvTimeoutError::Timeout) => continue,
                        Err(RecvTimeoutError::Disconnected) => return Err(ended()),
                    }
                }
                match job {
                    Job::Embed(text) => embed(known, models, &text, &mut pass)?,
                    Job::Look(path, key) => look(known, models, &path, &key, &mut pass),
                    Job::Query(_) => {}
                }
            }
            None => {
                let (kept, looked) = {
                    let mut held = known.lock();
                    held.store.save();
                    let looked = held.looks.as_mut().map_or(0, |looks| {
                        looks.save();
                        looks.len()
                    });
                    (held.store.len(), looked)
                };
                if pass.things > 0 {
                    eprintln!("meaning: {} things meant, {kept} kept", pass.things);
                }
                if pass.pictures > 0 {
                    eprintln!(
                        "meaning: {} pictures looked at, {looked} kept",
                        pass.pictures
                    );
                }
                pass = Pass::default();
                if models.loaded() {
                    match woken.recv_timeout(UNUSED) {
                        Ok(()) => {}
                        Err(RecvTimeoutError::Timeout) => {
                            models.let_go();
                            eprintln!("meaning: the models let go");
                        }
                        Err(RecvTimeoutError::Disconnected) => return Err(ended()),
                    }
                } else {
                    woken.recv().map_err(|_| ended())?;
                }
            }
        }
    }
}

/// Embeds a thing's text and keeps it.
fn embed(
    known: &Shared<Known>,
    models: &mut Models,
    text: &str,
    pass: &mut Pass,
) -> Result<(), String> {
    match models.texts(known)?.embed(text, false) {
        Ok(meaning) => {
            let mut held = known.lock();
            held.store.put(text, meaning);
            pass.things += 1;
            if pass.one() {
                held.store.save();
            }
        }
        Err(why) => {
            eprintln!("meaning: {text:?} wasn't embedded: {why}");
            // Kept as meaning nothing, so it isn't tried on every pass; a
            // new model or build tries again.
            known.lock().store.put(text, vec![0.0; text::WIDTH]);
        }
    }
    Ok(())
}

/// Looks at a picture and keeps what it looks like; one that can't be
/// read or looked at is kept as showing nothing.
fn look(known: &Shared<Known>, models: &mut Models, path: &Path, key: &str, pass: &mut Pass) {
    // Read before the tower's asked for, so a picture that can't be read
    // loads nothing.
    let seen = match Picture::read(path) {
        Ok(picture) => match models.looks() {
            Ok(looks) => looks.look(&picture),
            Err(why) => {
                // Not the picture's fault, so none is kept as showing
                // nothing; pictures aren't looked at, nor searched, until
                // the daemon starts again.
                eprintln!("meaning: pictures aren't looked at here: {why}");
                known.lock().looks = None;
                return;
            }
        },
        Err(why) => Err(why),
    };
    let mut held = known.lock();
    let Some(looks) = held.looks.as_mut() else {
        return;
    };
    match seen {
        Ok(look) => {
            looks.put(key, look);
            pass.pictures += 1;
            if pass.one() {
                looks.save();
            }
        }
        Err(why) => {
            eprintln!("meaning: {} wasn't looked at: {why}", path.display());
            // Kept as showing nothing, so it isn't tried again until it's
            // written over.
            looks.put(key, vec![0.0; text::WIDTH]);
        }
    }
}

/// The lines given, or those read when none are.
fn given_or_read(given: &[String]) -> Vec<String> {
    if given.is_empty() {
        std::io::stdin()
            .lock()
            .lines()
            .map_while(Result::ok)
            .filter(|line| !line.trim().is_empty())
            .collect()
    } else {
        given.to_vec()
    }
}

/// Milliseconds since `began`, to two places.
fn ms(began: Instant) -> f64 {
    began.elapsed().as_secs_f64() * 1000.0
}

/// How long each of `count` took, of `took` milliseconds in all.
fn each(took: f64, count: usize) -> f64 {
    took / f64::from(u32::try_from(count.max(1)).unwrap_or(u32::MAX))
}

/// Prints what's nearest first, each to three places.
fn print_nearest(mut found: Vec<(f32, String)>) {
    found.sort_by(|a, b| b.0.total_cmp(&a.0));
    for (near, what) in found {
        println!("{near:.3}\t{what}");
    }
}

/// Prints how near each thing means the words, nearest first, saying
/// what loading and each embedding took.
fn print_near(models: &Path, words: &str, things: &[String]) -> Result<(), String> {
    let things = given_or_read(things);
    let core = core()?;
    let (mut texts, _) = timed(&core, "bge", |core, device| {
        Texts::load(core, models, device)
    })?;
    let began = Instant::now();
    let asked = texts.embed(words, true)?;
    eprintln!("meaning: the words in {:.2} ms", ms(began));
    let began = Instant::now();
    let mut found = Vec::with_capacity(things.len());
    for thing in &things {
        found.push((near(&asked, &texts.embed(thing, false)?), thing.clone()));
    }
    let took = ms(began);
    eprintln!(
        "meaning: {} things in {took:.1} ms, {:.2} ms each",
        things.len(),
        each(took, things.len())
    );
    print_nearest(found);
    Ok(())
}

/// Prints how near each picture is to the words, nearest first, saying
/// what loading, reading and looking at each took.
fn print_look(sight: &Path, words: &str, pictures: &[String]) -> Result<(), String> {
    let core = core()?;
    let (mut said, _) = timed(&core, "SigLIP's words", |core, device| {
        Words::load(core, sight, device)
    })?;
    let (mut looks, _) = timed(&core, "SigLIP's looks", |core, device| {
        Looks::load(core, sight, device)
    })?;
    let began = Instant::now();
    let asked = said.embed(words)?;
    eprintln!("meaning: the words in {:.2} ms", ms(began));
    let (mut reading, mut looking) = (0.0, 0.0);
    let mut found = Vec::with_capacity(pictures.len());
    for path in pictures {
        let began = Instant::now();
        let picture = Picture::read(Path::new(path))?;
        reading += ms(began);
        let began = Instant::now();
        found.push((near(&asked, &looks.look(&picture)?), path.clone()));
        looking += ms(began);
    }
    eprintln!(
        "meaning: {} pictures, {:.1} ms each read, {:.1} ms each looked at",
        pictures.len(),
        each(reading, pictures.len()),
        each(looking, pictures.len())
    );
    print_nearest(found);
    Ok(())
}
