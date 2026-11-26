//! machine SOCKET [ROOT] [--words FILE] [--numbers FILE]
//!
//! How this machine is doing, held ready for the shell and anything else
//! that asks, over SOCKET in the shell's daemon interface (docs/daemons.md):
//! the processor, the memory, each disk, this boot, the programs using
//! the machine most, its sensors, thermal zones and cooling, each power
//! supply, how it sleeps, its firmware, the screen's backlight and the
//! room's light, the brightness and warmth the backlight program
//! steers the built-in screen to, who has each camera open, Keep Awake's
//! locks and the power profile (kept.rs), and the keyboards connected,
//! caps lock, a board's layer and what key-counts has tallied of each
//! keyboard, each a record whose figures are
//! the kernel's own or its services'
//! (lib.rs says how they're named). The kernel's files are read under
//! ROOT, `/` unless a test says otherwise.
//!
//! Nothing is read while nobody watches, so an idle machine wakes it for
//! nothing. While someone does, what they watch is read once a second,
//! and once more a fifth of a second after a read whose rates aren't of
//! now (a first read's are since boot, and one after a gap since then),
//! so a figure stands for now as soon as it can. Each read tells every
//! watch the records it changed, all in one line. The programs are read
//! only while someone watches `using`, or a process's tree by its key,
//! since reading every process costs the most. What the system bus's
//! services say is asked each on a thread of its own, so a slow bus never
//! holds a read back, and only while its kind is watched: each disk's
//! health of `UDisks` once a minute, the battery's times of `UPower` every
//! five seconds, idling and what holds sleep off of logind every two, and
//! the firmware of fwupd once a minute and whenever it says it changed.
//! Its signals are listened for only while firmware is watched, but
//! logind's `PrepareForSleep` always, since a sleep happens while nothing
//! watches, and a signal wakes nothing until it comes. The backlight
//! program's files are heard as it writes them while their kinds are
//! watched, so a switch turned in the launcher reads turned within a
//! frame, not at the next second's read, and so are the cameras' holders,
//! so a program opening a camera is said as it does, and the keyboards'
//! watchers' files, so caps lock is lit on the screen as it is on the
//! board. Keep Awake's files and the
//! machine's chosen profile are heard as they're written, and
//! power-profiles-daemon as it says it changed, from the daemon's start,
//! since the shell watches them while the launcher's shut: neither is
//! read on a clock, so watching them wakes nothing until they change.
//! The network links and their addresses (links.rs) are dumped from the
//! kernel's route socket as their watch begins, and again only as the
//! kernel says one changed, so Network reads them without spawning ip.
//!
//! Heat's, Power's and Firmware's rows and headlines are made here too,
//! ready for the launcher (`heat_rows.rs`, `power_rows.rs`,
//! `firmware_rows.rs`), in the launcher's words
//! (`--words`, its `words.json`) and number settings (`--numbers`,
//! `{ group, point }`): a watch of kind `row` or `head` with `place`
//! `heat` has the sensors, zones, cooling and processor read as their
//! own watch would, one with `place` `power` the supplies, and one with
//! `place` `firmware` fwupd, and one with `place` `machine` the processor,
//! the memory and the boot (`machine_rows.rs`, `processor_rows.rs`),
//! and one with `place` `disk` the disks, whose rows are Machine's Disk
//! (`disk_rows.rs`), a place of its own since each read asks an NVMe
//! disk its temperature, and each read makes that place's rows again and tells only those that
//! changed. Machine's are made again as the programs using them are read
//! too, though watching its rows doesn't read them, since that reads
//! every process's files: the shell watches `using` while it's looked at.
//! The processor's own telemetry is asked only while Heat's rows are
//! watched, or `telemetry` is, as the shell watches it while a Processor
//! answer can be seen; Machine's rows are made again with each of its
//! seconds then, for what held the processor back and kept it from
//! resting (`telemetry_rows.rs`).
//! Handed no words, it makes no rows.
//!
//! It acts as well as answers, so the shell spawns and writes nothing of
//! its own: `charge-limit` moves a battery's charge limit, and the supply
//! is read again before the act is answered, so each watch is told the
//! new limit first. The firmware's acts (acts.rs) can wait minutes on
//! fwupd or a polkit prompt, so each goes on a thread of its own, one at
//! a time, and is answered once it's done, the connection answering
//! everything else meanwhile; fwupd is asked again as each ends.
//!
//! It speaks through daemon-serve, as every daemon does: each read that
//! moves a record hands it every record held, and it tells each watch
//! what moved. What a get, a list or a watch beginning names is read
//! first where it isn't fresh (`read_for`), and a watch beginning or
//! ending wakes what reads for it (`heard_watches`).
//!
//! Its parts are beside this file: what's watched and the rows made of
//! what's held (making.rs), the reads (reading.rs), what's heard as it's
//! written (hearing.rs), what the system bus's services say (bus.rs) and
//! the answers (answering.rs). The other files named here are the
//! library's, in src/.

mod answering;
mod bus;
mod hearing;
mod making;
mod reading;

use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::os::unix::net::UnixListener;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, Weak};
use std::time::{Duration, Instant};

use crate::answering::{health, own, read_for};
use crate::bus::{firmware_loop, health_loop, logind_loop, profile_loop, sleeps_loop, upower_loop};
use crate::hearing::{heard_loop, heard_until, kept_loop, light_loop, links_loop};
use crate::making::{hand_over, let_go};
use crate::reading::{Reader, shared, telemetry_loop, watch_loop};
use daemon_serve::serve::{Shared as Served, accept, listen};
use machine::interface::{DOMAIN, place_of};
use machine::words::Words;
use machine::{
    Root, camera, counts, disk_rows, firmware_rows, heat_rows, keyboards, power_rows, status,
    steered, telemetry,
};
use ready_rows::kept::Kept;
use serde_json::Value;
use terminal_kit::{Exit, Failure, Help, flags};

/// How often what's watched is read.
const EVERY: Duration = Duration::from_secs(1);
/// The kinds read as their files are written (`heard_loop`), never on
/// the second.
const STEERED: [&str; 2] = ["brightness", "warmth"];
const KEYBOARDS: [&str; 2] = ["keyboards", "capslock"];
/// The kinds read as their files are written, their bus says they
/// changed or their reader tells a second (`kept_loop`, `profile_loop`,
/// `telemetry_loop`).
const KEPT: [&str; 3] = ["kept", "profile", "telemetry"];
/// A place whose rows and headline are made here, and the kinds they're
/// made of, read together while they're watched.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Place {
    name: &'static str,
    sources: &'static [&'static str],
    /// Kinds whose reads make its rows again, though watching them
    /// doesn't read them.
    heard: &'static [&'static str],
}

impl Place {
    /// Its headline's key.
    fn head(self) -> String {
        format!("head:{}", self.name)
    }

    fn holds(self, key: &str) -> bool {
        place_of(key) == Some(self.name)
    }
}

const HEAT: Place = Place {
    name: "heat",
    sources: &["sensor", "zone", "cooling", "processor"],
    heard: &[],
};
const POWER: Place = Place {
    name: "power",
    sources: &["supply"],
    heard: &[],
};
const FIRMWARE: Place = Place {
    name: "firmware",
    sources: &["firmware"],
    heard: &[],
};
const MACHINE: Place = Place {
    name: "machine",
    sources: &["processor", "memory", "boot"],
    heard: &["using", "telemetry"],
};
const DISK: Place = Place {
    name: "disk",
    sources: &["disk"],
    heard: &[],
};
const PLACES: [Place; 5] = [HEAT, POWER, FIRMWARE, MACHINE, DISK];
/// Everything the threads share.
struct Daemon {
    held: BTreeMap<String, Value>,
    /// When each kind was last read.
    read_at: HashMap<String, Instant>,
    /// What serves the records, and holds the watches.
    served: Arc<Served>,
    /// Each disk's health as `UDisks` last said it.
    health: BTreeMap<String, Value>,
    /// How asking `UDisks` stands: "answering", or "refused" and why.
    udisks: Value,
    /// What `UPower` last worked out of each supply, by its name.
    upower: BTreeMap<String, Value>,
    /// What logind last said of idling and sleep.
    logind: Value,
    /// fwupd's records, as it last said them.
    firmware: Vec<Value>,
    /// The light sensors' records, as `light_loop` last read them.
    light: Vec<Value>,
    /// What power-profiles-daemon last said, as `profile_loop` heard it.
    profile: Value,
    /// The launcher's words, which rows are said in, or none to make none.
    words: Option<Words>,
    /// The reads Heat's sparks keep while its rows are watched.
    heat: heat_rows::Heat,
    /// The reads Power's sparks keep while its rows are watched.
    power: power_rows::Power,
    /// The reads the Disk's sparks keep while its rows are watched.
    disk: disk_rows::Disk,
    /// How Firmware's acts stand: the one going, so another waits its
    /// turn, and how each ended while its rows are watched.
    acting: firmware_rows::Acting,
    /// Each place's rows and headline as last told, by the place's name,
    /// so each row keeps its order and one coming or going sends itself
    /// alone, where ordering siblings by where they stood sent every one
    /// after it again (FEH1HE, 2026-11-10).
    kept: BTreeMap<&'static str, Kept>,
    /// What the processor's telemetry last told, while it's asked.
    telemetry: telemetry::Heard,
    /// The kinds whose source said they changed, to be asked again now.
    hurried: HashSet<String>,
    /// When the last sleep began and ended, as logind's signal said.
    slept: Arc<Mutex<Value>>,
    /// How long each of the last reads took, in microseconds.
    took: VecDeque<u64>,
    /// What each kind of those reads took, in the same order.
    kinds_took: VecDeque<BTreeMap<&'static str, u64>>,
}

struct Shared {
    daemon: Mutex<Daemon>,
    reader: Mutex<Reader>,
    woken: Condvar,
    /// `PipeWire`'s registry, listed before the reader's lock is taken, so
    /// a `PipeWire` slow to answer holds up no other read.
    pipewire: Arc<camera::PipeWire>,
}

/// What `machine --help` says, as its unit runs it.
fn help() -> Help {
    Help::new(
        "machine",
        "holds how this machine is doing ready for the shell",
    )
    .example(
        "machine SOCKET --words FILE --numbers FILE",
        "as its unit runs it, reading the kernel under /",
    )
    .example(
        "machine SOCKET ROOT",
        "reading the kernel's files under ROOT, as a test does",
    )
    .option(
        "--words FILE",
        "the launcher's words, without which it makes no rows",
    )
    .option("--numbers FILE", "how numbers group and point")
    .exit(
        Exit::Usage,
        "it was given no socket, or a flag or word it doesn't take",
    )
    .exit(Exit::Failed, "it couldn't listen on its socket")
}

/// The flags its unit may hand it, each with a file.
const FLAGS: [&str; 2] = ["--words", "--numbers"];

/// Where it serves, what it reads under, and the words its rows say.
struct Args {
    socket: PathBuf,
    root: Root,
    words: Option<String>,
    numbers: Option<String>,
}

/// What it was started with.
///
/// # Errors
///
/// Asked wrongly, 64, when it's given no socket.
fn args(given: &[String]) -> Result<Args, Failure> {
    let wrongly = |what: String| Failure::new(Exit::Usage, what).try_this("machine --help");
    let mut given = given.iter();
    let mut named = Vec::new();
    let (mut words, mut numbers) = (None, None);
    while let Some(arg) = given.next() {
        let held = match arg.as_str() {
            "--words" => &mut words,
            "--numbers" => &mut numbers,
            flag if flag.starts_with('-') => {
                return Err(flags::unknown("option", flag, &FLAGS).try_this("machine --help"));
            }
            word => {
                named.push(word.to_owned());
                continue;
            }
        };
        let value = given
            .next()
            .ok_or_else(|| wrongly(format!("{arg} needs a file")))?;
        *held = Some(value.clone());
    }
    let mut named = named.into_iter();
    let socket = named
        .next()
        .ok_or_else(|| wrongly("machine needs the socket it serves on".to_owned()))?;
    let root = named.next().unwrap_or_else(|| "/".to_owned());
    if let Some(extra) = named.next() {
        return Err(wrongly(format!("machine takes no word like {extra}")));
    }
    Ok(Args {
        socket: PathBuf::from(socket),
        root: Root::new(root),
        words,
        numbers,
    })
}

fn main() -> ExitCode {
    terminal_kit::main(&help(), env!("CARGO_PKG_VERSION"), |context| {
        serve(args(&context.args)?)
    })
}

/// Reads and serves until it's stopped.
///
/// # Errors
///
/// 1 when it can't listen on its socket.
fn serve(args: Args) -> Result<(), Failure> {
    let Args {
        socket,
        root,
        words,
        numbers,
    } = args;
    let words = worded(words.as_deref(), numbers.as_deref());
    let listener = listen(&socket).map_err(|why| {
        Failure::new(
            Exit::Failed,
            format!("machine can't listen on {}", socket.display()),
        )
        .because(why.to_string())
    })?;
    let (heard, watches) = mpsc::channel();
    let shared = started(&root, words, heard);
    {
        let shared = Arc::clone(&shared);
        std::thread::spawn(move || heard_watches(&shared, &watches));
    }
    {
        let shared = Arc::clone(&shared);
        let path = root.path(telemetry::PATH);
        std::thread::spawn(move || telemetry_loop(&shared, &path));
    }
    {
        let shared = Arc::clone(&shared);
        std::thread::spawn(move || watch_loop(&shared));
    }
    {
        let shared = Arc::clone(&shared);
        std::thread::spawn(move || health_loop(&shared));
    }
    {
        let shared = Arc::clone(&shared);
        std::thread::spawn(move || upower_loop(&shared));
    }
    {
        let shared = Arc::clone(&shared);
        std::thread::spawn(move || logind_loop(&shared));
    }
    {
        let slept = Arc::clone(&lock(&shared.daemon).slept);
        std::thread::spawn(move || sleeps_loop(&slept));
    }
    {
        let shared = Arc::clone(&shared);
        std::thread::spawn(move || firmware_loop(&shared));
    }
    if let Some(files) = steered::Files::of_session() {
        let shared = Arc::clone(&shared);
        let folders = files
            .folders()
            .iter()
            .map(|folder| root.path(folder))
            .collect::<Vec<_>>();
        std::thread::spawn(move || {
            heard_loop(&shared, &STEERED, &folders, |name| files.names(name));
        });
    }
    {
        let shared = Arc::clone(&shared);
        let folders = vec![root.path(camera::FOLDER)];
        // Read again on a clock too while `PipeWire` holds a camera, since
        // who reads it there changes with no file written.
        let due = |shared: &Shared| shared.pipewire.due();
        std::thread::spawn(move || {
            heard_until(&shared, &["camera"], &folders, camera::names, due);
        });
    }
    {
        let shared = Arc::clone(&shared);
        let root = root.clone();
        std::thread::spawn(move || kept_loop(&shared, &root));
    }
    {
        let shared = Arc::clone(&shared);
        std::thread::spawn(move || profile_loop(&shared));
    }
    hear_keyboards_and_watchers(&shared, &root);
    {
        let shared = Arc::clone(&shared);
        std::thread::spawn(move || links_loop(&shared));
    }
    {
        let shared = Arc::clone(&shared);
        std::thread::spawn(move || light_loop(&shared, &root));
    }
    answered(&shared, listener);
}

/// Every record held handed over, so how the sources stand is said from
/// the first hello, then each connection to `listener` answered, for
/// good, while the threads read.
fn answered(shared: &Shared, listener: UnixListener) -> ! {
    let served = {
        let daemon = lock(&shared.daemon);
        hand_over(&daemon);
        Arc::clone(&daemon.served)
    };
    accept(served, listener);
    loop {
        std::thread::park();
    }
}

/// What every thread shares, reading under `root`, saying rows in
/// `words` and telling `heard` as each watch begins or ends.
fn started(root: &Root, words: Option<Words>, heard: Sender<()>) -> Arc<Shared> {
    Arc::new_cyclic(|this| shared(root, words, serving(this, heard)))
}

/// What serves `shared`'s records: its acts answered each on a thread of
/// its own, since a firmware act waits on fwupd; what an ask names read
/// first; and health saying how long reads took. A watch beginning or
/// ending is told to `heard`, since it's told under the lock that holds
/// the watches, which the daemon's lock is never taken under.
fn serving(shared: &Weak<Shared>, heard: Sender<()>) -> Arc<Served> {
    let this = Weak::clone(shared);
    let answer = move |said: &Value| {
        let shared = this.upgrade()?;
        own(&shared, said)
    };
    let served = Served::new(DOMAIN, Box::new(answer));
    served.slow(&["act"]);
    let this = Weak::clone(shared);
    served.fresh(Box::new(move |wanted| {
        if let Some(shared) = this.upgrade() {
            read_for(&shared, wanted);
        }
    }));
    let this = Weak::clone(shared);
    served.health_too(Box::new(move || {
        this.upgrade().map_or(Value::Null, |shared| health(&shared))
    }));
    served.on_watches(Box::new(move || {
        let _ = heard.send(());
    }));
    served
}

/// As each watch begins or ends: lets go what no watch holds now, and
/// wakes every loop to look at what's watched again. The daemon's lock is
/// taken before the loops are woken, so one that just found nothing
/// watched is already waiting.
fn heard_watches(shared: &Shared, watches: &Receiver<()>) {
    for () in watches {
        let mut daemon = lock(&shared.daemon);
        let_go(&mut daemon);
        drop(daemon);
        shared.woken.notify_all();
    }
}

/// The launcher's words, from its words.json at `words` and its number
/// settings at `numbers`, or none, said in the journal, where it was
/// handed none or they can't be read.
fn worded(words: Option<&str>, numbers: Option<&str>) -> Option<Words> {
    let read = |path: &str| {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|text| serde_json::from_str::<Value>(&text).ok())
    };
    let Some(vocabulary) = words.and_then(read) else {
        eprintln!("<5>handed no words it can read, so it makes no rows");
        return None;
    };
    let numbers = numbers.and_then(read).unwrap_or(Value::Null);
    let mark = |name: &str| numbers[name].as_str().unwrap_or("").to_owned();
    Some(Words::new(&vocabulary, &mark("group"), &mark("point")))
}

/// Hears the keyboards' lights, the layer and the key counts, and the
/// watchers' status, each as its files are written.
fn hear_keyboards_and_watchers(shared: &Arc<Shared>, root: &Root) {
    {
        let shared = Arc::clone(shared);
        let folders = vec![root.path(keyboards::LIGHTS)];
        std::thread::spawn(move || heard_loop(&shared, &KEYBOARDS, &folders, keyboards::names));
    }
    {
        let shared = Arc::clone(shared);
        let folders = vec![root.path(counts::FOLDER)];
        std::thread::spawn(move || heard_loop(&shared, &["counts"], &folders, counts::names));
    }
    if let Some(folder) = keyboards::layer_file()
        .as_deref()
        .and_then(|file| std::path::Path::new(file).parent())
        .map(|folder| root.path(&folder.to_string_lossy()))
    {
        let shared = Arc::clone(shared);
        let folders = vec![folder];
        std::thread::spawn(move || {
            heard_loop(&shared, &["layer"], &folders, keyboards::names_the_layer);
        });
    }
    let shared = Arc::clone(shared);
    let folders = lock(&shared.reader)
        .watchers
        .iter()
        .map(|watcher| root.path(&watcher.folder))
        .collect::<Vec<_>>();
    std::thread::spawn(move || {
        let due = |shared: &Shared| {
            let reader = lock(&shared.reader);
            reader.statuses.due(&reader.watchers)
        };
        heard_until(&shared, &["status"], &folders, status::names, due);
    });
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use std::io::{BufRead, BufReader, Lines, Write};
    use std::os::unix::net::{UnixListener, UnixStream};

    use super::*;
    use crate::reading::read;
    use serde_json::json;

    /// The daemon's threads' shared state alone, reading under `root`,
    /// with no loop reading for it.
    fn alone(root: &Root, words: Option<Words>) -> Arc<Shared> {
        started(root, words, mpsc::channel().0)
    }

    /// A connection to what serves `shared`, on a socket of its own, and
    /// its lines as they're said.
    fn connected(shared: &Shared, name: &str) -> (UnixStream, Lines<BufReader<UnixStream>>) {
        let folder = std::env::temp_dir().join(format!("machine-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).unwrap();
        let socket = folder.join("machine.sock");
        let listener = UnixListener::bind(&socket).unwrap();
        accept(Arc::clone(&lock(&shared.daemon).served), listener);
        let stream = UnixStream::connect(&socket).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(10)))
            .unwrap();
        let lines = BufReader::new(stream.try_clone().unwrap()).lines();
        (stream, lines)
    }

    // A read that comes too soon after the last to measure the processor,
    // as a watch's first read and the loop it wakes do, keeps its record
    // held as it was, rather than letting it go: Processor's row went
    // for a second as a cold ask ended reading (the nested-niri check,
    // 2026-11-11).
    #[test]
    fn a_processor_read_too_soon_keeps_its_record() {
        let folder = std::env::temp_dir().join(format!("machine-soon-{}", std::process::id()));
        std::fs::create_dir_all(folder.join("proc")).unwrap();
        std::fs::write(
            folder.join("proc/stat"),
            "cpu  100 0 100 800 0 0 0 0 0 0\ncpu0 100 0 100 800 0 0 0 0 0 0\n",
        )
        .unwrap();
        let shared = alone(&Root::new(&folder), None);
        let kinds = HashSet::from(["processor".to_owned()]);
        read(&shared, &kinds, &[]);
        let first = lock(&shared.daemon).held.get("processor:0").cloned();
        read(&shared, &kinds, &[]);
        let again = lock(&shared.daemon).held.get("processor:0").cloned();
        std::fs::remove_dir_all(&folder).unwrap();
        let first = first.expect("the first read holds the processor");
        assert_eq!(first["busy_ratio"], json!(0.2), "{first}");
        assert_eq!(again, Some(first), "the second read let the record go");
    }

    // Firmware's rows say an act is going while it goes, and how it
    // ended once it has, before the act is answered: here an install on a
    // machine with no fwupdmgr, which is refused at once.
    #[test]
    fn firmwares_rows_say_an_act_going_then_how_it_ended() {
        // The launcher's own words (tests/words.json), which the words
        // check holds to words.json.
        let vocabulary: Value = serde_json::from_str(include_str!("../../../tests/words.json"))
            .expect("the launcher's words parse");
        let words = Words::new(&vocabulary, ",", ".");
        let root = Root::new(std::env::temp_dir());
        let shared = alone(&root, Some(words));
        lock(&shared.daemon).firmware = vec![json!({ "key": "firmware:b10",
            "kind": "firmware", "id": "b10", "name": "System Firmware",
            "version": "1.12.0", "update": { "version": "1.13.0" } })];
        let (mut stream, mut lines) = connected(&shared, "firmware");
        writeln!(
            stream,
            r#"{{"id":"f","ask":"watch","kind":["row"],"place":"firmware"}}"#
        )
        .unwrap();
        let first = lines.next().expect("the watch is answered").unwrap();
        let first: Value = serde_json::from_str(&first).unwrap();
        let records = first["records"].as_array().into_iter().flatten();
        let keys: Vec<&Value> = records.map(|record| &record["key"]).collect();
        assert!(
            keys.contains(&&json!("row:firmware:b10:install")),
            "Install is offered: {first}"
        );
        let said = json!({ "id": 1, "ask": "act", "act": "firmware-install",
            "device": "b10", "patience_s": 60 });
        writeln!(stream, "{said}").unwrap();
        let mut notes = Vec::new();
        loop {
            let line = lines.next().expect("the act is answered").unwrap();
            let line: Value = serde_json::from_str(&line).unwrap();
            if line["id"] == 1 {
                assert_eq!(line["error"]["code"], "bad-act", "{line}");
                break;
            }
            let changed = line["changed"].as_array().into_iter().flatten();
            notes.extend(
                changed
                    .filter(|record| record["key"] == "row:firmware:b10:install")
                    .map(|record| record["row"]["note"].clone()),
            );
        }
        assert_eq!(
            notes,
            [
                "installing",
                "didn't install \u{b7} no fwupdmgr on this machine"
            ],
            "going, then how it ended, before the answer"
        );
        assert!(
            lock(&shared.daemon).acting.going.is_none(),
            "nothing is going"
        );
    }

    fn given(line: &str) -> Vec<String> {
        line.split_whitespace().map(str::to_owned).collect()
    }

    /// What `args` refuses `line` with, as its exit's code and its words.
    fn refused(line: &str) -> (u8, String) {
        let failure = args(&given(line)).err().expect("it's refused");
        let code = failure.code();
        let words = format!("{} {}", failure.what, failure.why.unwrap_or_default());
        (code, words)
    }

    #[test]
    fn a_flag_it_doesnt_know_is_refused_as_asked_wrongly_naming_the_nearest() {
        let (code, words) = refused("--word words.json machine.sock");
        assert_eq!(code, 64, "a mistyped flag: {words}");
        assert!(words.contains("--word"), "names the flag: {words}");
        assert!(words.contains("--words"), "names the nearest: {words}");
    }

    #[test]
    fn a_flag_without_its_value_is_refused_as_asked_wrongly() {
        let (code, words) = refused("machine.sock --numbers");
        assert_eq!(code, 64, "a flag with no value: {words}");
        assert!(words.contains("--numbers"), "names the flag: {words}");
    }

    #[test]
    fn a_third_word_is_refused_as_asked_wrongly() {
        let (code, words) = refused("machine.sock / extra");
        assert_eq!(code, 64, "a word past the root: {words}");
        assert!(words.contains("extra"), "names the word: {words}");
    }

    #[test]
    fn its_units_line_is_taken_as_it_was() {
        let taken = args(&given("machine.sock --words w.json --numbers n.json")).unwrap();
        assert_eq!(taken.socket, PathBuf::from("machine.sock"));
        assert_eq!(
            (taken.words.as_deref(), taken.numbers.as_deref()),
            (Some("w.json"), Some("n.json"))
        );
    }
}
