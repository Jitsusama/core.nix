//! Lends the machine a builder elsewhere while its machine queue has builds
//! a builder could take, and gives it back once it's idle.
//!
//! Usage: `nix-offload serve --helper PATH --prefix NAME --rate DOLLARS
//! --first-start HH:MM --last-start HH:MM --deadline HH:MM [--standing
//! PATH]`, or `nix-offload stand --standing PATH` to name standing
//! builders alone and lend none, or `nix-offload report --rate DOLLARS` for
//! today's line, or `nix-offload there [-L] [--keep-going] [--apply
//! FUNCTION] [--out DIR] FLAKE ATTR...` to build a flake on the builder, or
//! on a machine nearby for systems this one can't build, as src/there.rs
//! says. A build waiting for a builder says so in /run/nix-offload/wanted,
//! which counts as queued work.
//!
//! Every 20 seconds it reads the machine queue. The moment a build or a VM
//! check is queued or running, it asks the helper for a builder, names it
//! `NAME<seconds>`, and writes the Nix machines line the helper gives into
//! /run/nix-offload/machines, which Nix's build hook reads as each build
//! command starts (`builders = @` it) and the queue reads to count the
//! builder's room. The line makes big-parallel mandatory and offers nothing
//! a VM test needs, so big builds go to the builder first, and small ones
//! and VM tests, whose inputs built here are slow to copy out, stay; a
//! build sent whole to its store takes VM tests there instead. Standing builders that
//! answer go only into /run/nix-offload/machines-with-standing, beside the
//! lent one, for a build that names that file (`--builders @...`), never
//! every build. Once nothing has reached it for fifteen minutes, or the
//! deadline comes, it takes the line out first, so no build
//! is sent to a builder that's going, then gives the builder back. It
//! starts none outside the day's hours, so none is made overnight or to
//! live only minutes.
//!
//! It gives back only builders it asked for: each ask is journalled before
//! the helper runs, and a builder counts as gone only once the helper said
//! so. One not yet seen gone is asked to go again each tick, and no other
//! is lent beside it. Whatever else shares the prefix is someone else's,
//! and is only said.
//!
//! The helper says how, and this when, so this knows nothing of where
//! the builder lives:
//! - `helper up NAME SECONDS` makes it, to live at most SECONDS, the time
//!   left until the deadline, prints its machines line once Nix can use
//!   it, and stays running while it's there, holding the way to it. Before
//!   that line it may print `price DOLLARS`, the hour's price of what it
//!   made, since a fallback machine can cost more than `--rate`, and
//!   `store URI`, the builder's store as this account reaches it, written
//!   to /run/nix-offload/store, whose host, key and host key a build sent
//!   there as a command is reached by. It
//!   ends once the builder goes on its own, and gives the way up when told
//!   to end (SIGTERM, to its whole process group). The seconds are counted
//!   here, in the zone TZ names, so the helper needs no time zones of its
//!   own: a shell without zoneinfo reads a zone's 20:00 as UTC's.
//! - `helper hold NAME` holds the way to one already there, as `up` does
//!   once it's made, saying the same lines; it ends at once when there's
//!   none. A start takes back with it what a stop left, rather than give
//!   it back and make another, so a switch costs no wait for a builder and
//!   its store stays warm.
//! - `helper down NAME` gives it back, and succeeds once it's gone,
//!   whether or not it was there.
//! - `helper alive PREFIX` names, a line each, the builders there now.
//!
//! Each ask, start, stop and leftover is a line in the journal under XDG's
//! state folder, `nix-offload/journal.jsonl`, which `report` sums.
//!
//! Builders always there, such as other machines at home, are named in
//! the file `--standing` gives, and join the machines file while they
//! answer, so Nix never waits on one that's away.

mod changed;
mod decide;
mod journal;
mod machines;
mod queue;
mod there;

use std::io::{BufRead, BufReader};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitCode, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use std::{env, fs, thread};

use decide::{Hours, Tick};
use journal::{Event, Ran};
use machines::Machines;

/// How long a builder may take to be ready before it's given up on.
const READY: Duration = Duration::from_mins(15);
/// How often the queue is read.
const TICK: Duration = Duration::from_secs(20);
/// Where nix-daemon and the queue read the lent builder's line, for every
/// build.
const MACHINES: &str = "/run/nix-offload/machines";
/// Where a build that asks for standing builders too reads them, by
/// `--builders @` it: never every build, since they may be machines only
/// some work may reach.
const WITH_STANDING: &str = "/run/nix-offload/machines-with-standing";
/// Where a build sent to the lent builder as a command reads how to reach
/// it, as its store's URI, while one stands.
const STORE: &str = "/run/nix-offload/store";
/// The folder all of these are in, where the lender also says its pid in
/// `lender`, and a build waiting for a builder says it wants one in
/// `wanted`.
const DIR: &str = "/run/nix-offload";
/// Where the machine queue reads the builder being made, to hold the
/// builds it could take until Nix can see it.
const LENDING: &str = "/run/nix-offload/lending";

/// The variable, with its value, that tells jobs-sorter (modules/slices) a
/// process waits on work done elsewhere.
const ELSEWHERE: (&str, &str) = ("WORK_RUNS_ELSEWHERE", "1");

/// What a program asked wrongly exits with, as sysexits.h numbers it.
const USAGE: i32 = 64;

const USED: &str = "Usage: nix-offload serve --helper PATH --prefix NAME --rate DOLLARS \
                    --first-start HH:MM --last-start HH:MM --deadline HH:MM \
                    [--standing PATH] | nix-offload stand --standing PATH \
                    | nix-offload report --rate DOLLARS \
                    | nix-offload there [-L] [--keep-going] [--apply FUNCTION] [--out DIR] \
                    FLAKE ATTR... | nix-offload there --changed [CHANGED-CHECKS ARGUMENT...]";

/// Set when the service is told to end, so its builder is journalled for the
/// next start to give back.
static ENDING: AtomicBool = AtomicBool::new(false);

extern "C" fn ending(_: libc::c_int) {
    ENDING.store(true, Ordering::SeqCst);
}

/// How often a rest looks to see whether it's been told to end.
const HEARD_WITHIN: Duration = Duration::from_millis(100);

/// Wait `length`, or less once told to end. A signal can only set a flag
/// safely, and `thread::sleep` sleeps on through one, so the flag is read
/// in short steps: a stop that waited out a whole tick would hold every
/// other unit's start at a switch that long.
fn rest(length: Duration) {
    let began = Instant::now();
    while !ENDING.load(Ordering::SeqCst) {
        let left = length.saturating_sub(began.elapsed());
        if left.is_zero() {
            return;
        }
        thread::sleep(left.min(HEARD_WITHIN));
    }
}

/// What `serve` was given.
struct Settings {
    helper: PathBuf,
    prefix: String,
    hours: Hours,
    queue: PathBuf,
    standing: Option<PathBuf>,
    journal: PathBuf,
}

/// A builder lent to the machine.
struct Lent {
    name: String,
    helper: Child,
    line: String,
    store: Option<String>,
    host: String,
    rate: Option<u64>,
    since: u64,
    used: u64,
}

impl Lent {
    fn lending(&self) -> machines::Lending<'_> {
        machines::Lending {
            line: &self.line,
            store: self.store.as_deref(),
        }
    }
}

/// What the helper said once its builder was made: its price and its
/// store, if it gave them, and its machines line.
#[derive(Debug, Default, PartialEq, Eq)]
struct Heard {
    rate: Option<u64>,
    store: Option<String>,
    line: String,
}

/// Read what the helper says: `price DOLLARS` and `store URI` lines, in
/// any order, then its machines line.
fn heard_from(helper: impl BufRead) -> Heard {
    let mut heard = Heard::default();
    for line in helper.lines() {
        let Ok(line) = line else { break };
        if let Some(price) = line.strip_prefix("price ") {
            heard.rate = journal::price(price.trim());
        } else if let Some(store) = line.strip_prefix("store ") {
            heard.store = Some(store.trim().to_owned());
        } else {
            heard.line = line;
            break;
        }
    }
    heard
}

fn main() -> ExitCode {
    let arguments: Vec<String> = env::args().skip(1).collect();
    let given = |flag: &str| {
        arguments
            .iter()
            .position(|argument| argument == flag)
            .and_then(|at| arguments.get(at + 1))
            .cloned()
    };
    if arguments.first().map(String::as_str) == Some("there") {
        return there(&arguments[1..]);
    }
    if arguments.first().map(String::as_str) == Some("stand") {
        let Some(standing) = given("--standing") else {
            eprintln!("nix-offload: stand needs --standing\n{USED}");
            return ExitCode::from(64);
        };
        stand(&read_standing(Path::new(&standing)));
        return ExitCode::SUCCESS;
    }
    serving(&arguments, given)
}

/// `there`: builds a flake's attributes where src/there.rs says, each part
/// journalled, exiting with the first status that failed.
fn there(arguments: &[String]) -> ExitCode {
    // A `there` run only waits on a build done elsewhere, mostly, so the
    // machine's slices let its processes go after other work when memory
    // runs out; they read the mark from where each started, so this starts
    // again with it rather than only handing it on.
    if env::var_os(ELSEWHERE.0).is_none_or(|value| value != ELSEWHERE.1) {
        let again = Command::new("/proc/self/exe")
            .arg("there")
            .args(arguments)
            .env(ELSEWHERE.0, ELSEWHERE.1)
            .exec();
        eprintln!("nix-offload: couldn't start there again marked: {again}");
        return ExitCode::from(71);
    }
    // What this `there` takes, a word each, so a caller can tell it from
    // an older one, which refuses the flag with 64.
    if arguments == ["--takes"] {
        println!("{}", there::TAKES.join("\n"));
        return ExitCode::SUCCESS;
    }
    if let Some(("--changed", rest)) = arguments
        .split_first()
        .map(|(first, rest)| (first.as_str(), rest))
    {
        return changed_there(rest);
    }
    let Some(asked) = there::asked(arguments) else {
        eprintln!("nix-offload: there takes a flake and its attributes\n{USED}");
        return ExitCode::from(64);
    };
    // A test names its own folder, as changed-checks lets one.
    let dir = env::var_os("NIX_OFFLOAD_DIR").map_or_else(|| PathBuf::from(DIR), PathBuf::from);
    let store = fs::read_to_string(dir.join("store"))
        .ok()
        .filter(|store| !store.trim().is_empty());
    let scratch = env::var_os("XDG_RUNTIME_DIR")
        .map_or_else(env::temp_dir, PathBuf::from)
        .join("nix-offload-there");
    let nearby = fs::read_to_string(config().join("nix-offload/nearby"))
        .map(|text| there::nearby(&text))
        .unwrap_or_default();
    let (near, left) = there::shared(&asked, &nearby, machines::answers);
    let journal = state().join("nix-offload/there.jsonl");
    let mut status = 0;
    let mut timed = |part: &there::Asked, build: &dyn Fn() -> there::Ran| {
        let began = now();
        let ran = build();
        let ended = now();
        there::note(&journal, ended, ended.saturating_sub(began), part, &ran);
        // The first that failed says how the whole did.
        if status == 0 {
            status = ran.status;
        }
    };
    for (machine, part) in &near {
        timed(part, &|| {
            there::nearby_run(machine, part, &scratch, ["ssh", "tar", "nix"])
        });
    }
    if let Some(part) = &left {
        // Asked for and waited on, rather than built here, while a lender
        // runs: a builder stands about 70 s after it's asked for.
        let store = store.or_else(|| there::waited(&dir, there::WAITING));
        // Waited for from when the builder was first lost.
        let lost = std::cell::Cell::new(None);
        let again = || {
            let since = lost.get().unwrap_or_else(std::time::Instant::now);
            lost.set(Some(since));
            there::stood_again(&dir, there::WAITING, since)
        };
        timed(part, &|| {
            there::run(part, store.as_deref(), &again, &scratch, ["ssh", "nix"])
        });
    }
    ExitCode::from(u8::try_from(status).unwrap_or(1))
}

/// `there --changed`: changed-checks run for the tree here on the lent
/// builder, else an evaluator with its builds sent on, else a builder
/// waited for (`changed.rs` says why). It exits with the run's status, or
/// `NOT_TAKEN` when nowhere else took it.
fn changed_there(arguments: &[String]) -> ExitCode {
    let dir = env::var_os("NIX_OFFLOAD_DIR").map_or_else(|| PathBuf::from(DIR), PathBuf::from);
    let scratch = env::var_os("XDG_RUNTIME_DIR")
        .map_or_else(env::temp_dir, PathBuf::from)
        .join("nix-offload-there");
    let not_taken = |why: &str| {
        eprintln!("nix-offload: {why}, so changed-checks runs here");
        ExitCode::from(changed::NOT_TAKEN)
    };
    let Ok(folder) = env::current_dir() else {
        return not_taken("this folder can't be read");
    };
    let source = match there::source(&folder, &scratch) {
        Ok(source) => source,
        Err(why) => return not_taken(&why),
    };
    let bundle = match &source {
        there::Source::Bundled { bundle, .. } => Some(bundle.clone()),
        there::Source::Pushed { .. } => None,
    };
    let code = changed_somewhere(arguments, &dir, &scratch, &folder, &source);
    if let Some(bundle) = bundle {
        let _ = fs::remove_file(bundle);
    }
    code.unwrap_or_else(|| not_taken("no builder or evaluator answered"))
}

/// Where `there --changed` runs, in its order, or none when nowhere did.
fn changed_somewhere(
    arguments: &[String],
    dir: &Path,
    scratch: &Path,
    folder: &Path,
    source: &there::Source,
) -> Option<ExitCode> {
    let exited = |status: i32| ExitCode::from(u8::try_from(status).unwrap_or(1));
    let on_builder = |store: Option<String>| {
        let lent = store.and_then(|store| there::builder(store.lines().next()?.trim()))?;
        let sending = changed::sending(source.clone(), arguments);
        match there::on(&lent, &sending, None, scratch, "ssh") {
            Ok(ran) if ran.status != there::UNREACHED => Some(exited(ran.status)),
            _ => {
                eprintln!("nix-offload: ssh couldn't reach {}", lent.host);
                None
            }
        }
    };
    let store = fs::read_to_string(dir.join("store"))
        .ok()
        .filter(|store| !store.trim().is_empty());
    if let Some(code) = on_builder(store) {
        return Some(code);
    }
    let wanted = changed::wanted(arguments);
    // An evaluator only names; what it names is built as any `there` build
    // is, since a machine that evaluates well may build slowly.
    let mut evaluating: Vec<String> = arguments
        .iter()
        .filter(|word| *word != "--build" && *word != "--name-only")
        .cloned()
        .collect();
    evaluating.push("--name-only".to_owned());
    // A changed-checks from before --name-only refuses it, as usage, and
    // without --build it only names anyway.
    let older = &evaluating[..evaluating.len() - 1];
    let evaluators = fs::read_to_string(config().join("nix-offload/evaluators"))
        .map(|text| changed::evaluators(&text))
        .unwrap_or_default();
    let github = match source {
        there::Source::Pushed { github, .. } | there::Source::Bundled { github, .. } => github,
    };
    for evaluator in evaluators
        .iter()
        .filter(|evaluator| evaluator.takes(github) && machines::answers(&evaluator.address))
    {
        eprintln!("nix-offload: evaluating on {}", evaluator.host);
        let ask = |words: &[String]| {
            let sending = changed::sending(source.clone(), words);
            changed::evaluated("ssh", &evaluator.host, &sending, scratch)
        };
        let asked = match ask(&evaluating) {
            Ok((USAGE, _)) => ask(older),
            asked => asked,
        };
        let (status, printed) = match asked {
            Ok((status, _)) if status == there::UNREACHED => continue,
            Ok(evaluated) => evaluated,
            Err(why) => {
                eprintln!("nix-offload: {why}");
                continue;
            }
        };
        let targets = changed::to_build(&printed, wanted.vm);
        if status != 0 || !wanted.build || targets.is_empty() {
            return Some(exited(status));
        }
        let Some(system) = &wanted.system else {
            eprintln!("nix-offload: changed-checks names its system when it builds elsewhere");
            return Some(ExitCode::from(64));
        };
        let built = changed::building(folder, system, &targets).status();
        return Some(built.map_or(ExitCode::FAILURE, |built| exited(built.code().unwrap_or(1))));
    }
    // Asked for and waited on, as any `there` waits, while a lender runs.
    on_builder(there::waited(dir, there::WAITING))
}

/// `serve`: lends builders while the queue has builds one could take.
fn serving(arguments: &[String], given: impl Fn(&str) -> Option<String>) -> ExitCode {
    let Some(rate) = given("--rate").as_deref().and_then(journal::price) else {
        eprintln!("nix-offload: --rate takes a price, such as 0.68\n{USED}");
        return ExitCode::from(64);
    };
    let journal = state().join("nix-offload/journal.jsonl");
    match arguments.first().map(String::as_str) {
        Some("report") => {
            let journal = fs::read_to_string(&journal).unwrap_or_default();
            println!("{}", journal::said(&journal::day(&journal, today(), rate)));
            ExitCode::SUCCESS
        }
        Some("serve") => {
            let seconds = |flag| given(flag).as_deref().and_then(second_of);
            let (Some(helper), Some(prefix), Some(first), Some(last), Some(deadline)) = (
                given("--helper"),
                given("--prefix"),
                seconds("--first-start"),
                seconds("--last-start"),
                seconds("--deadline"),
            ) else {
                eprintln!("nix-offload: serve needs each of its settings\n{USED}");
                return ExitCode::from(64);
            };
            if !(first <= last && last <= deadline) {
                eprintln!("nix-offload: the day's hours must run first, last, deadline\n{USED}");
                return ExitCode::from(64);
            }
            let runtime = env::var_os("XDG_RUNTIME_DIR").map_or_else(
                || PathBuf::from("/run/user").join(unsafe { libc::getuid() }.to_string()),
                PathBuf::from,
            );
            // Two would each give back what the other lent.
            let Ok(lock) = fs::File::create(runtime.join("nix-offload.lock")) else {
                eprintln!("nix-offload: can't make its lock in {}", runtime.display());
                return ExitCode::FAILURE;
            };
            if lock.try_lock().is_err() {
                eprintln!("nix-offload: another serve is running");
                return ExitCode::FAILURE;
            }
            serve(&Settings {
                helper: helper.into(),
                prefix,
                hours: Hours {
                    first,
                    last,
                    deadline,
                },
                queue: runtime.join("jobs/queue.json"),
                standing: given("--standing").map(PathBuf::from),
                journal,
            });
            ExitCode::SUCCESS
        }
        _ => {
            eprintln!("{USED}");
            ExitCode::from(64)
        }
    }
}

/// Hear a stop, so what's lent is given back first.
fn hear_stops() {
    // SAFETY: the handler only stores to an atomic.
    unsafe {
        let handler = ending as *const () as libc::sighandler_t;
        libc::signal(libc::SIGTERM, handler);
        libc::signal(libc::SIGINT, handler);
    }
}

fn read_standing(path: &Path) -> Vec<machines::Standing> {
    machines::standing(&fs::read_to_string(path).unwrap_or_else(|error| {
        eprintln!("nix-offload: reading {}: {error}", path.display());
        String::new()
    }))
}

/// Name the standing builders that answer, lending none, until told to end.
fn stand(standing: &[machines::Standing]) {
    hear_stops();
    let mut machines = Machines::new(
        MACHINES.into(),
        WITH_STANDING.into(),
        STORE.into(),
        LENDING.into(),
        standing.to_vec(),
    );
    while !ENDING.load(Ordering::SeqCst) {
        machines.show(None);
        rest(TICK);
    }
}

/// Lend a builder while the queue wants one, until told to end.
fn serve(settings: &Settings) {
    hear_stops();
    let standing = settings
        .standing
        .as_deref()
        .map_or_else(Vec::new, read_standing);
    let mut machines = Machines::new(
        MACHINES.into(),
        WITH_STANDING.into(),
        STORE.into(),
        LENDING.into(),
        standing,
    );
    machines.show(None);
    machines.asking(None);
    // Said so a build with no builder waits for one only while a lender
    // runs to make it.
    let lender = Path::new(DIR).join("lender");
    if let Err(error) = fs::write(&lender, format!("{}\n", std::process::id())) {
        eprintln!(
            "nix-offload: couldn't say I lend in {}: {error}",
            lender.display()
        );
    }
    let mut lent = adopt(settings, &mut machines, into_day());
    let mut not_before = 0;
    let mut said_on = None;
    while !ENDING.load(Ordering::SeqCst) {
        let tick = Tick {
            now: now(),
            into_day: into_day(),
            wants: fs::read_to_string(&settings.queue)
                .is_ok_and(|queue| queue::wants(&queue, now()))
                || there::wanted(Path::new(DIR)),
        };
        let unreturned = give_back_unreturned(settings, lent.as_ref());
        lent = match lent.take() {
            None => match decide::lend(settings.hours, tick, not_before, unreturned) {
                Some(seconds) => {
                    let lent = lend(settings, &mut machines, tick.now, seconds);
                    if lent.is_none() {
                        not_before = decide::next_after(Event::Failed, tick.wants, now());
                    }
                    lent
                }
                None => None,
            },
            Some(mut builder) => {
                if in_use(Path::new("/proc"), &builder.host) {
                    builder.used = tick.now;
                }
                let gone = builder.helper.try_wait().ok().flatten().is_some();
                match decide::give_back(settings.hours, tick, builder.used, gone) {
                    Some(event) => {
                        give_back(settings, &mut machines, builder, event);
                        not_before = decide::next_after(event, tick.wants, now());
                        None
                    }
                    None => Some(builder),
                }
            }
        };
        // Once a day, after the deadline, say what's there that isn't this
        // one's to give back.
        let day = local_day();
        if tick.into_day >= settings.hours.deadline && said_on != Some(day) {
            say_others(settings);
            said_on = Some(day);
        }
        machines.show(lent.as_ref().map(Lent::lending));
        rest(TICK);
    }
    if let Some(mut builder) = lent {
        machines.show(None);
        end(&mut builder.helper);
        leave(settings, &builder.name, builder.since, builder.rate);
    }
}

/// Leave a builder for the next start to give back, since a stop that
/// waited on its delete would hold every other unit's start at a switch.
/// Its own idle delete takes it if no start comes.
fn leave(settings: &Settings, name: &str, asked: u64, rate: Option<u64>) {
    let now = now();
    let ran = Ran {
        seconds: now - asked,
        rate,
    };
    let _ = journal::note(&settings.journal, now, Event::Ended, name, Some(ran));
    eprintln!("nix-offload: told to end, so {name} is left for the next start to give back");
}

/// Ask for a builder, naming it as being made until it stands or won't, so
/// the machine queue holds the builds it could take until Nix can see it.
fn lend(settings: &Settings, machines: &mut Machines, now: u64, left: u64) -> Option<Lent> {
    let name = format!("{}{now}", settings.prefix);
    machines.asking(Some(&name));
    let builder = ask(settings, machines, name, now, left);
    machines.asking(None);
    builder
}

/// Ask the helper for a builder, and give Nix its line once it's ready.
fn ask(
    settings: &Settings,
    machines: &mut Machines,
    name: String,
    now: u64,
    left: u64,
) -> Option<Lent> {
    eprintln!("nix-offload: asking for {name}, since the queue has builds a builder could take");
    if let Err(error) = journal::note(&settings.journal, now, Event::Asked, &name, None) {
        // Unjournalled, it couldn't be known as this one's to give back.
        eprintln!("nix-offload: journalling the ask: {error}; none is asked for");
        return None;
    }
    let (mut helper, said) = hear(settings, &["up", &name, &left.to_string()])?;
    let Heard { rate, store, line } = said;
    let line = line.trim();
    match host_of(line) {
        Some(host) if helper.try_wait().ok().flatten().is_none() => {
            machines.show(Some(machines::Lending {
                line,
                store: store.as_deref(),
            }));
            let ran = Ran {
                seconds: self::now() - now,
                rate,
            };
            let _ = journal::note(
                &settings.journal,
                self::now(),
                Event::Started,
                &name,
                Some(ran),
            );
            eprintln!("nix-offload: {name} is ready; Nix may send it builds");
            Some(Lent {
                name,
                helper,
                line: line.to_owned(),
                store,
                host,
                rate,
                since: now,
                used: self::now(),
            })
        }
        _ if ENDING.load(Ordering::SeqCst) => {
            end(&mut helper);
            leave(settings, &name, now, rate);
            None
        }
        _ => {
            eprintln!("nix-offload: {name} wasn't ready, so it's given back");
            end(&mut helper);
            if down(settings, &name) {
                let ran = Ran {
                    seconds: self::now() - now,
                    rate,
                };
                let _ = journal::note(
                    &settings.journal,
                    self::now(),
                    Event::Failed,
                    &name,
                    Some(ran),
                );
            }
            None
        }
    }
}

/// Run the helper with `arguments`, and hear what it says until its
/// machines line, for as long as a builder may take to be ready.
fn hear(settings: &Settings, arguments: &[&str]) -> Option<(Child, Heard)> {
    let mut helper = Command::new(&settings.helper)
        .args(arguments)
        .stdout(Stdio::piped())
        .process_group(0)
        .spawn()
        .map_err(|error| eprintln!("nix-offload: running the helper: {error}"))
        .ok()?;
    let (said, heard) = mpsc::channel();
    let out = helper.stdout.take()?;
    thread::spawn(move || {
        let _ = said.send(heard_from(BufReader::new(out)));
    });
    // In short waits, so a stop is heard while the builder is being made.
    let asked = Instant::now();
    let mut said = Heard::default();
    while asked.elapsed() < READY && !ENDING.load(Ordering::SeqCst) {
        match heard.recv_timeout(Duration::from_secs(1)) {
            Ok(heard) => {
                said = heard;
                break;
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
    Some((helper, said))
}

/// Take back the builder the last stop left, before the deadline, rather
/// than give it back and make another: a switch restarts this, and a new
/// builder costs a minute's wait and a cold store, where a VM test took
/// 201 s against 67 s on one already warm. It keeps the time it was asked
/// for, so it's priced whole, and the time the stop left it as its last
/// use, so its idle time runs on. One that can't be held is given back by
/// the first tick, as any left builder is.
fn adopt(settings: &Settings, machines: &mut Machines, into_day: u32) -> Option<Lent> {
    if into_day >= settings.hours.deadline {
        return None;
    }
    let journal = fs::read_to_string(&settings.journal).unwrap_or_default();
    let left = journal::unreturned(&journal)
        .into_iter()
        .rev()
        .find(|builder| builder.ended.is_some())?;
    let ended = left.ended?;
    let name = left.name;
    eprintln!("nix-offload: taking back {name}, which the last stop left");
    let (mut helper, said) = hear(settings, &["hold", &name])?;
    let Heard { rate, store, line } = said;
    let line = line.trim();
    let Some(host) = host_of(line).filter(|_| helper.try_wait().ok().flatten().is_none()) else {
        eprintln!("nix-offload: {name} couldn't be taken back, so it's given back");
        end(&mut helper);
        return None;
    };
    machines.show(Some(machines::Lending {
        line,
        store: store.as_deref(),
    }));
    let _ = journal::note(&settings.journal, now(), Event::Held, &name, None);
    eprintln!("nix-offload: {name} is taken back; Nix may send it builds");
    Some(Lent {
        name,
        helper,
        line: line.to_owned(),
        store,
        host,
        rate: rate.or(left.rate),
        since: left.asked,
        used: ended.at,
    })
}

/// Take the builder from Nix, then give it back, noting how long it ran.
fn give_back(settings: &Settings, machines: &mut Machines, mut builder: Lent, event: Event) {
    machines.show(None);
    end(&mut builder.helper);
    if down(settings, &builder.name) {
        let now = now();
        let ran = Ran {
            seconds: now - builder.since,
            rate: builder.rate,
        };
        let _ = journal::note(&settings.journal, now, event, &builder.name, Some(ran));
        eprintln!(
            "nix-offload: gave {} back after {} minutes",
            builder.name,
            ran.seconds / 60
        );
    }
}

/// Ask again for every builder this asked for and hasn't seen gone, but
/// the one lent, and say how many are still not gone.
fn give_back_unreturned(settings: &Settings, lent: Option<&Lent>) -> usize {
    let journal = fs::read_to_string(&settings.journal).unwrap_or_default();
    let mut still = 0;
    for builder in journal::unreturned(&journal) {
        let name = &builder.name;
        if lent.is_some_and(|lent| &lent.name == name) {
            continue;
        }
        eprintln!("nix-offload: {name} isn't yet seen gone, so it's given back");
        if down(settings, name) {
            let now = now();
            let (event, ran) = builder.given_back(now, decide::IDLE);
            let _ = journal::note(&settings.journal, now, event, name, ran);
        } else {
            still += 1;
        }
    }
    still
}

/// Say any builder under the prefix that isn't this one's, leaving it be.
fn say_others(settings: &Settings) {
    let Ok(alive) = Command::new(&settings.helper)
        .args(["alive", &settings.prefix])
        .output()
    else {
        eprintln!("nix-offload: couldn't ask the helper what's there");
        return;
    };
    let journal = fs::read_to_string(&settings.journal).unwrap_or_default();
    let ours = journal::unreturned(&journal);
    for name in String::from_utf8_lossy(&alive.stdout)
        .lines()
        .map(str::trim)
    {
        if !name.is_empty() && !ours.iter().any(|known| known.name == name) {
            eprintln!("nix-offload: {name} is there after the deadline, and isn't this one's");
        }
    }
}

/// Ask the helper to give a builder back: whether it's gone.
fn down(settings: &Settings, name: &str) -> bool {
    let gone = Command::new(&settings.helper)
        .args(["down", name])
        .status()
        .is_ok_and(|status| status.success());
    if !gone {
        eprintln!("nix-offload: the helper couldn't give {name} back; it's asked again next tick");
    }
    gone
}

/// Tell the helper and everything it started to end, and wait for it.
fn end(helper: &mut Child) {
    if let Ok(pid) = libc::pid_t::try_from(helper.id()) {
        // SAFETY: a signal to our own child's process group, which it leads.
        unsafe { libc::kill(-pid, libc::SIGTERM) };
    }
    let _ = helper.wait();
}

/// The ssh host a machines line names, which Nix's build hook reaches it
/// by: `ssh-ng://builder@host x86_64-linux ...` names `host`.
fn host_of(line: &str) -> Option<String> {
    let uri = line.split_whitespace().next()?;
    let (_, rest) = uri.split_once("://")?;
    let rest = rest.split(['?', '/']).next().unwrap_or(rest);
    let host = rest.rsplit_once('@').map_or(rest, |(_, host)| host);
    (!host.is_empty()).then(|| host.to_owned())
}

/// Whether an ssh reaching `host` runs: Nix's build hook holds one for
/// each build it sends there, naming it `host` or `user@host`.
fn in_use(proc: &Path, host: &str) -> bool {
    let Ok(entries) = fs::read_dir(proc) else {
        return false;
    };
    let with_user = format!("@{host}");
    entries.flatten().any(|entry| {
        let Ok(command) = fs::read(entry.path().join("cmdline")) else {
            return false;
        };
        let mut words = command.split(|byte| *byte == 0);
        let is_ssh = words
            .next()
            .is_some_and(|program| program.rsplit(|byte| *byte == b'/').next() == Some(b"ssh"));
        is_ssh && words.any(|word| word == host.as_bytes() || word.ends_with(with_user.as_bytes()))
    })
}

/// Seconds into the day of a time written `HH:MM`.
fn second_of(written: &str) -> Option<u32> {
    let (hours, minutes) = written.split_once(':')?;
    let (hours, minutes): (u32, u32) = (hours.parse().ok()?, minutes.parse().ok()?);
    (hours < 24 && minutes < 60).then_some(hours * 3600 + minutes * 60)
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs())
}

/// The local time now, in the zone TZ names.
fn local() -> libc::tm {
    let now = libc::time_t::try_from(now()).unwrap_or(0);
    // SAFETY: localtime_r writes only the tm it's given.
    unsafe {
        let mut tm = std::mem::zeroed();
        libc::localtime_r(&raw const now, &raw mut tm);
        tm
    }
}

fn into_day() -> u32 {
    let tm = local();
    u32::try_from(tm.tm_hour * 3600 + tm.tm_min * 60 + tm.tm_sec).unwrap_or(0)
}

fn local_day() -> i32 {
    let tm = local();
    tm.tm_year * 1000 + tm.tm_yday
}

/// The start of today, local time, in seconds since the epoch.
fn today() -> u64 {
    now().saturating_sub(u64::from(into_day()))
}

fn state() -> PathBuf {
    env::var_os("XDG_STATE_HOME").map_or_else(
        || PathBuf::from(env::var_os("HOME").unwrap_or_default()).join(".local/state"),
        PathBuf::from,
    )
}

/// XDG's config folder, where the machines nearby are named.
fn config() -> PathBuf {
    env::var_os("XDG_CONFIG_HOME").map_or_else(
        || PathBuf::from(env::var_os("HOME").unwrap_or_default()).join(".config"),
        PathBuf::from,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_stop_ends_a_rest_at_once() {
        hear_stops();
        thread::spawn(|| {
            thread::sleep(Duration::from_millis(200));
            // SAFETY: a signal to this process, whose handler only stores
            // to an atomic.
            unsafe { libc::kill(libc::getpid(), libc::SIGTERM) };
        });
        let began = Instant::now();
        rest(TICK);
        assert!(
            began.elapsed() < Duration::from_secs(1),
            "a stop took {:?} to end a rest",
            began.elapsed()
        );
    }

    #[test]
    fn a_lines_host_is_what_ssh_reaches() {
        let line = "ssh-ng://builder@offload x86_64-linux /etc/nix/key 32 1 kvm - AAAA";
        assert_eq!(host_of(line).as_deref(), Some("offload"));
        assert_eq!(
            host_of("ssh-ng://offload x86_64-linux").as_deref(),
            Some("offload")
        );
        assert_eq!(
            host_of("ssh-ng://offload?remote-store=x x86_64-linux").as_deref(),
            Some("offload")
        );
        assert_eq!(host_of("").as_deref(), None);
        assert_eq!(host_of("offload x86_64-linux").as_deref(), None);
    }

    #[test]
    fn a_builder_is_in_use_while_an_ssh_reaches_it() {
        let proc = tempfile::tempdir().expect("a folder");
        let process = |pid: &str, command: &[&str]| {
            let dir = proc.path().join(pid);
            fs::create_dir(&dir).expect("made");
            fs::write(dir.join("cmdline"), command.join("\0")).expect("written");
        };
        process("1", &["/run/current-system/sw/bin/nix-daemon", "--daemon"]);
        process("2", &["sshd", "offload"]);
        assert!(!in_use(proc.path(), "offload"));
        process(
            "3",
            &[
                "/nix/store/x-openssh/bin/ssh",
                "-x",
                "builder@offload",
                "nix-daemon",
                "--stdio",
            ],
        );
        assert!(in_use(proc.path(), "offload"));
        assert!(!in_use(proc.path(), "elsewhere"));
    }

    #[test]
    fn times_of_day_are_read_as_seconds() {
        assert_eq!(second_of("20:00"), Some(72_000));
        assert_eq!(second_of("19:30"), Some(70_200));
        assert_eq!(second_of("24:00"), None);
        assert_eq!(second_of("noon"), None);
    }

    #[test]
    fn the_helper_may_price_its_builder_and_name_its_store_before_its_line() {
        let line = "ssh-ng://nix-offload x86_64-linux /key 5 1 kvm - aG9zdA==";
        let store = "ssh-ng://nix-offload?ssh-key=/k";
        assert_eq!(
            heard_from(format!("price 0.89\nstore {store}\n{line}\nlater\n").as_bytes()),
            Heard {
                rate: Some(890),
                store: Some(store.to_owned()),
                line: line.to_owned(),
            }
        );
        assert_eq!(
            heard_from(format!("{line}\n").as_bytes()),
            Heard {
                line: line.to_owned(),
                ..Heard::default()
            }
        );
        assert_eq!(
            heard_from("price 0.89\n".as_bytes()),
            Heard {
                rate: Some(890),
                ..Heard::default()
            }
        );
    }

    /// A helper that holds `lend-5` when asked and has nothing else.
    const HOLDS: &str = r#"#!/bin/sh
if [ "$1 $2" = "hold lend-5" ]; then
  echo "price 0.68"
  echo "store ssh-ng://far?ssh-key=/k"
  echo "ssh-ng://far x86_64-linux /k 8 1 big-parallel - -"
  exec sleep 60
fi
exit 1
"#;

    fn holding(folder: &Path, journal: &str) -> (Settings, Machines) {
        use std::os::unix::fs::PermissionsExt;
        let helper = folder.join("helper");
        fs::write(&helper, HOLDS).unwrap();
        fs::set_permissions(&helper, fs::Permissions::from_mode(0o755)).unwrap();
        fs::write(folder.join("journal.jsonl"), journal).unwrap();
        let settings = Settings {
            helper,
            prefix: "lend-".to_owned(),
            hours: Hours {
                first: 0,
                last: 80_000,
                deadline: 86_000,
            },
            queue: folder.join("queue.json"),
            standing: None,
            journal: folder.join("journal.jsonl"),
        };
        let file = |name: &str| folder.join(name);
        let machines = Machines::new(
            file("machines"),
            file("with-standing"),
            file("store"),
            file("lending"),
            Vec::new(),
        );
        (settings, machines)
    }

    #[test]
    fn a_start_takes_back_what_the_last_stop_left() {
        let folder = tempfile::tempdir().unwrap();
        let journal = [
            r#"{"at":5,"event":"asked","name":"lend-5"}"#,
            r#"{"at":9,"event":"ended","name":"lend-5","seconds":4,"rate":890}"#,
        ]
        .join("\n")
            + "\n";
        let (settings, mut machines) = holding(folder.path(), &journal);
        let mut lent = adopt(&settings, &mut machines, 3600).expect("taken back");
        assert_eq!(
            (
                lent.name.as_str(),
                lent.host.as_str(),
                lent.since,
                lent.used
            ),
            ("lend-5", "far", 5, 9)
        );
        assert_eq!(lent.rate, Some(680));
        let machines_line = fs::read_to_string(folder.path().join("machines")).unwrap();
        assert!(
            machines_line.starts_with("ssh-ng://far "),
            "{machines_line}"
        );
        end(&mut lent.helper);
        let journal = fs::read_to_string(&settings.journal).unwrap();
        assert!(
            journal::unreturned(&journal)[0].ended.is_none(),
            "{journal}"
        );
    }

    #[test]
    fn nothing_is_taken_back_after_the_deadline_or_when_it_is_gone() {
        let folder = tempfile::tempdir().unwrap();
        let left = |name: &str| {
            format!(
                "{{\"at\":5,\"event\":\"asked\",\"name\":\"{name}\"}}\n\
                 {{\"at\":9,\"event\":\"ended\",\"name\":\"{name}\",\"seconds\":4}}\n"
            )
        };
        let (settings, mut machines) = holding(folder.path(), &left("lend-5"));
        assert!(adopt(&settings, &mut machines, 86_000).is_none());
        let (settings, mut machines) = holding(folder.path(), &left("lend-6"));
        assert!(adopt(&settings, &mut machines, 3600).is_none());
        let (settings, mut machines) = holding(folder.path(), "");
        assert!(adopt(&settings, &mut machines, 3600).is_none());
    }
}
