//! Lends the machine a builder elsewhere while its machine queue has builds
//! a builder could take, and gives it back once it's idle.
//!
//! Usage: `nix-offload serve --helper PATH --prefix NAME --rate DOLLARS
//! --first-start HH:MM --last-start HH:MM --deadline HH:MM [--longest HH:MM]
//! [--idle HH:MM] [--standing PATH] [--beside PATH] [--dir DIR] [--only-there]`,
//! or `nix-offload stand [--standing PATH] [--beside PATH]` to name standing builders alone and lend none, or
//! `nix-offload report --rate DOLLARS` for today's line, or `nix-offload
//! there [-L] [--keep-going] [--apply FUNCTION] [--out DIR]
//! [--print-out-paths] [--here] [--here-when-idle] [--outlive] [--bulk] [--first]
//! [--lent-only] (FLAKE ATTR... | FLAKE#ATTR... | DRV...)` to build a flake, or derivations, on the
//! builder, or on a machine nearby for systems this one can't build, as
//! src/there.rs says. A build waiting for a builder
//! says so in /run/nix-offload/wanted, which counts as queued work.
//!
//! `nix-offload relay --listen ADDRESS --to ADDRESS` passes connections on
//! to a builder's tunnel, dialling it again while it closes before the
//! builder speaks, as src/relay.rs says. It prints `listening` once it
//! listens, beside any other relay on that port, and told to leave
//! (SIGUSR1) it takes no new connection and ends once its last closes.
//!
//! `nix-offload ask --quest QUEST` on a machine with no way to the
//! builders, such as a Mac, has the machine that gates run `there
//! --changed` for its tree's commit through mycelium, which starts
//! `nix-offload answer QUEST COMMIT` there, as src/gate.rs says.
//! `nix-offload outside -- there WORD...` runs a call in the environment
//! the agent fence's exemption gives it, and grants nothing by itself, as
//! src/outside.rs says.
//!
//! `nix-offload progress HOST` says how far the builds on a lent builder
//! have got, as src/progress.rs says.
//!
//! A builder that lives `--longest` is deleted at its end whatever it's
//! building, and can't be given longer once it runs. So an hour before
//! that, while a build uses it or wants one, a successor is asked for;
//! once it stands, Nix is given its line alone, and the old builder's
//! helper is told to hand over (SIGUSR1), so every new connection goes to
//! the successor while the old one finishes what it has. The old one is
//! given back once its helper ends, which it does once nothing reaches it,
//! or once its time is up, and a run outliving its caller holds it until
//! then. Told to hand over now (SIGUSR2, as `systemctl --user kill -s USR2
//! --kill-whom=main` sends it), as when the builders' shapes changed, it
//! does the same at once, however long the builder has left.
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
//! every build. Once nothing has reached it for fifteen minutes, or as
//! long as `--idle` says, or the deadline comes, it takes the line out first, so no build
//! is sent to a builder that's going, then gives the builder back. It
//! starts none outside the day's hours, so none is made overnight or to
//! live only minutes. With `--longest`, each builder lives that long from
//! its asking instead of until the deadline, so one lent at any hour is
//! never ended mid-build at midnight; the deadline then only says when
//! the day's builders others left are named.
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
//!   to end (SIGTERM, to its whole process group). Told to hand over
//!   (SIGUSR1, to it alone), it takes no new connection to the builder,
//!   and ends once the last one closes. The seconds are counted
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
//! A lender's end ends the way to its builder, and every build reaching
//! it with that, so a switch that changes its unit reloads it (SIGHUP)
//! rather than restarting it. It then ends at once, leaving the builder
//! for the next start as a stop does, and exits 75, which its unit starts
//! the new one on. While a build reaches the builder, its helper, run
//! through `--outlasting` in a scope the unit's end leaves, is told to hand
//! over rather than end, so those builds finish beside the next start's
//! way. It waits only while a builder it handed to a successor finishes,
//! since nothing after it would give that one back.
//!
//! A second lender may run beside it for VM tests alone: `--dir` gives it
//! a folder of its own, such as /run/nix-offload-vm, whose files nothing
//! but `there` reads, and its lock and journal are named for that folder;
//! `--only-there` has it lend only while a `there` build waits on it. A
//! `there` build sends its VM tests to that lender's builder alone, so they
//! never share one with big builds: the attributes the patterns in XDG's
//! config folder, `nix-offload/vm-tests`, describe, and the derivations
//! whose features ask for kvm or nixos-test. Once that lender has run, a
//! VM test waits for its builder, through a switch's restart of it too,
//! and is never sent the lent way. A changed-checks run sent to the lent
//! builder (`there --changed`) names what changed there first meanwhile,
//! builds all but the VM tests there, and sends those the same way.
//! Another such lender, in /run/nix-offload-vm-more, lends a second VM
//! builder, which takes the VM tests past the first's room: as many as
//! `nix-offload/vm-builds` says it runs at once, else 4 (as both VM
//! lenders' helpers are told in `OFFLOAD_MAX_JOBS`), counting those runs
//! still going have sent it, each saying so in its folder's `sending`.
//! Lenders in /run/nix-offload-vm-3 to the last lane, which `last-vm-lane.txt`
//! names for nixos.nix and home.nix too, lend more, each taking what's
//! past the room of those before it. Past every builder's room, a run's
//! VM tests wait for room, in the order runs came, since Nix bounds each
//! run's build alone. A run's parts go at once, each to its own builder,
//! and each journals in `waited` how long it waited for room and for its
//! builder to stand.
//!
//! A VM lender given `--warm` has each builder it stands build those
//! installables for itself first, saying so in its folder's `warming`,
//! since a fresh builder's first VM tests build the desktop for an hour.
//! While a warm VM builder stands, a run's VM tests go only to warm ones,
//! and a cold one not on its way is asked for once their room runs out.
//!
//! Builders always there, such as other machines at home, are named in
//! the file `--standing` gives, and join the machines file while they
//! answer, so Nix never waits on one that's away. Those in the file
//! `--beside` gives may take any of this machine's work, so they join
//! /run/nix-offload/machines, for every build, beside the one lent, and
//! only while they answer. Both files are read again each time the
//! builders are named, so a switch that changes them needs no reload.
//!
//! Every lender proves its builder every minute with a tiny build no store
//! holds, as src/proof.rs says, and the main one the builders beside too,
//! unless root proves those (src/standing.rs), when it reads what root says.
//! A lent builder that fails two in a row is taken out of every file this
//! lender writes and given back as lost, which ends its helper and the
//! relay with it, so hung connections close and Nix sends their builds
//! elsewhere, and another is asked for at once. A builder beside that
//! fails two is left out until one passes. Each lender says how its
//! builders stand in its folder's `health.json`. While the main lender's
//! builder fails its proofs, or none is lent, its file for any work names
//! the second lent builder, else the warm VM builders, that passed theirs.

mod archived;
mod asking;
mod busy;
mod cargo;
mod changed;
mod cloud;
mod command;
mod decide;
mod ended;
mod finishing;
mod freeing;
mod gate;
mod held;
mod help;
mod journal;
mod keep;
mod kept;
mod lines;
mod machines;
mod outside;
mod passed;
mod progress;
mod proof;
mod queue;
mod relay;
mod replay;
mod running;
mod sleep;
mod standing;
#[cfg(test)]
mod testing;
mod there;
mod vet;

use std::collections::BTreeSet;
use std::io::{BufRead, BufReader, Read};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitCode, Stdio};

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use std::{env, fs, thread};
use terminal_kit::{Exit, Failure};

use cloud::Place;

use decide::{Hours, Restart, Tick};
use journal::{Event, Ran};
use machines::Machines;
use proof::{Health, Keeper, State};

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
/// Where a build that may go only to the builders beside, never the lent
/// one, reads those that answer, by `--builders @` it, so it builds here
/// when the file is empty.
const BESIDE: &str = "/run/nix-offload/machines-beside";
/// Where a build that may send any piece of public work away, such as an
/// update built with no local jobs, reads the one lent with nothing
/// mandatory, by `--builders @` it; empty while none is lent. Work only
/// some machines may take must ask for a feature the lent line lacks.
const ANYTHING: &str = "/run/nix-offload/machines-for-anything";
/// Where a build sent to the lent builder as a command reads how to reach
/// it, as its store's URI, while one stands.
const STORE: &str = "/run/nix-offload/store";
/// The folder all of these are in, where the lender also says its pid in
/// `lender`, and a build waiting for a builder says it wants one in
/// `wanted`.
const DIR: &str = "/run/nix-offload";
/// The folder of the lender whose builder takes only VM tests, when one
/// runs, as `serve --dir` it with `--only-there`.
const VM_DIR: &str = "/run/nix-offload-vm";
/// The folder of the lender whose builder takes what the first can't,
/// when one runs, as `serve --dir` it with `--only-there`.
const MORE_DIR: &str = "/run/nix-offload-more";

/// The second lender's folder, [`MORE_DIR`] unless `NIX_OFFLOAD_MORE_DIR`
/// names another, as tests do.
fn more_dir() -> PathBuf {
    env::var_os("NIX_OFFLOAD_MORE_DIR").map_or_else(|| PathBuf::from(MORE_DIR), PathBuf::from)
}

/// Where a `there` keeps what it sends while it runs: the runtime
/// folder, or the temporary one where there's none.
fn scratch() -> PathBuf {
    env::var_os("XDG_RUNTIME_DIR")
        .map_or_else(env::temp_dir, PathBuf::from)
        .join("nix-offload-there")
}
/// The folder of the lender whose builder takes the VM tests past the
/// first VM builder's room, when one runs, as `serve --dir` it with
/// `--only-there`.
const VM_MORE_DIR: &str = "/run/nix-offload-vm-more";
/// Where the machine queue reads the builder being made, to hold the
/// builds it could take until Nix can see it.
const LENDING: &str = "/run/nix-offload/lending";

/// The variable, with its value, that tells jobs-sorter (modules/slices) a
/// process waits on work done elsewhere.
const ELSEWHERE: (&str, &str) = ("WORK_RUNS_ELSEWHERE", "1");

/// What a program asked wrongly exits with, as sysexits.h numbers it.
const USAGE: i32 = 64;

/// Set when the service is told to end, so its builder is journalled for the
/// next start to give back.
static ENDING: AtomicBool = AtomicBool::new(false);

extern "C" fn ending(_: libc::c_int) {
    ENDING.store(true, Ordering::SeqCst);
}

/// Set when a switch has changed the service's unit, which reloads it
/// (SIGHUP) rather than restarting it, so it ends once nothing uses its
/// builder and systemd starts the new one.
static SUPERSEDED: AtomicBool = AtomicBool::new(false);

extern "C" fn superseded(_: libc::c_int) {
    SUPERSEDED.store(true, Ordering::SeqCst);
}

/// Set when the lender is told to hand its builder over now (SIGUSR2),
/// as when the builders' shapes changed: a successor in the first shape
/// stands, then the old builder finishes what it has, as at its end.
static TOLD_TO_HAND_OVER: AtomicBool = AtomicBool::new(false);

extern "C" fn told_to_hand_over_now(_: libc::c_int) {
    TOLD_TO_HAND_OVER.store(true, Ordering::SeqCst);
}

/// Hear a lender told to hand over now: `serve` alone, since every other
/// command would end on SIGUSR2.
fn hear_hand_overs() {
    // SAFETY: the handler only stores to an atomic.
    unsafe {
        let handler = told_to_hand_over_now as *const () as libc::sighandler_t;
        libc::signal(libc::SIGUSR2, handler);
    }
}

/// Set when a relay is told to leave (SIGUSR1), since its builder hands
/// over to a successor.
static LEAVING: AtomicBool = AtomicBool::new(false);

extern "C" fn leaving(_: libc::c_int) {
    LEAVING.store(true, Ordering::SeqCst);
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
    beside: Option<PathBuf>,
    journal: PathBuf,
    /// The folder its files are written in, `/run/nix-offload` unless
    /// `--dir` names another.
    dir: PathBuf,
    /// Whether it lends only while a `there` build waits for its builder,
    /// as `--only-there` says.
    only_there: bool,
    /// What each builder it stands builds for itself before taking VM
    /// tests, as `--warm` names them, space-separated.
    warm: Vec<String>,
    /// What each helper holding a builder runs through, as `--outlasting`
    /// names it, so it outlasts the service: one that starts it in a scope
    /// of its own lets a restart hand its builder over rather than cut the
    /// builds on its way.
    outlasting: Option<PathBuf>,
    /// The program told when no builder answers any work, and again once
    /// one does, as `--tell` names it.
    telling: Option<PathBuf>,
    /// The file naming what frees each builder beside once it's full, as
    /// `--freeing` names it.
    freeing: Option<PathBuf>,
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
    /// When the time it was asked to live runs out, in seconds since the
    /// epoch, so one that ran out while the machine slept isn't taken for
    /// lost.
    ends: u64,
    /// Where it is in the cloud, as its helper last said, which it says
    /// again while it holds it.
    place: Arc<Mutex<Place>>,
}

/// A builder that handed over to a successor, finishing what it has.
struct Leaving {
    builder: Lent,
    /// Whether a run outliving its caller built there as it handed over,
    /// which holds it until its time is up, since no ssh here reaches it.
    outliving: bool,
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
/// store, if it gave them, where it is in the cloud, and its machines
/// line.
#[derive(Debug, Default, PartialEq, Eq)]
struct Heard {
    rate: Option<u64>,
    store: Option<String>,
    place: Place,
    line: String,
}

/// Read what the helper says: `price DOLLARS`, `store URI`, `instance
/// NAME ZONE` and `cloud-token TOKEN` lines, in any order, then its
/// machines line.
fn heard_from(helper: &mut impl BufRead) -> Heard {
    let mut heard = Heard::default();
    for line in helper.lines() {
        let Ok(line) = line else { break };
        if let Some(price) = line.strip_prefix("price ") {
            heard.rate = journal::price(price.trim());
        } else if let Some(store) = line.strip_prefix("store ") {
            heard.store = Some(store.trim().to_owned());
        } else if !heard.place.heard(&line) {
            heard.line = line;
            break;
        }
    }
    heard
}

/// Where the builder is, as its helper last said.
fn placed(place: &Mutex<Place>) -> Place {
    place
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone()
}

/// Hear where the builder is from what the helper says after its line,
/// into `place`, until it ends: a fresh token while it holds the builder.
/// Read to its end, so a helper never writes to a pipe nobody reads.
fn heard_after(helper: impl BufRead, place: &Mutex<Place>) {
    for line in helper.lines() {
        let Ok(line) = line else { break };
        if let Ok(mut place) = place.lock() {
            place.heard(&line);
        }
    }
}

fn main() -> ExitCode {
    // First, before any thread starts: no ssh this starts may ask Joel.
    // SAFETY: nothing has started a thread yet, and no dependency starts
    // one before `main`.
    unsafe { asking::never_ask() };
    // Read lossily only to see whether help is asked: a word that isn't
    // text reaches the kit whole, which refuses it as asked wrongly (64)
    // where `env::args` would panic.
    let arguments: Vec<String> = env::args_os()
        .skip(1)
        .map(|argument| argument.to_string_lossy().into_owned())
        .collect();
    // Help, at any depth, is the kit's, and asks nothing else of it; every
    // other call reads its words below, as its units and gates call it.
    if help::asked(&arguments) {
        return help::answered();
    }
    // Every other call's words are read below, exactly as its units and
    // gates give them, so the kit takes none of them: a `-v` or `--json`
    // given to `there` is nix's. It ends through the kit, so its exits
    // are the table's (docs/commands.md). Past `--changed`, a `--help` is
    // changed-checks', as `help::asked` reads it.
    terminal_kit::main(
        &help::help().hands_words_on(&["--changed"]),
        env!("CARGO_PKG_VERSION"),
        |context| ran(&context.args),
    )
}

/// The call `arguments` make, run: one of the subcommands, or a lender.
fn ran(arguments: &[String]) -> Result<(), Failure> {
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
    match arguments.first().map(String::as_str) {
        Some("ask") => return gate::ask::ask(&arguments[1..]),
        Some("answer") => return gate::answer::answer(&arguments[1..], &state()),
        Some("outside") => return outside::outside(&arguments[1..]),
        Some("scoped") => return gate::fence::scoped(&arguments[1..]),
        Some("busy") => return busy::busy(&arguments[1..]),
        Some("cargo") => {
            let dir =
                env::var_os("NIX_OFFLOAD_DIR").map_or_else(|| PathBuf::from(DIR), PathBuf::from);
            return cargo::cargo(&arguments[1..], [dir.as_path(), more_dir().as_path()]);
        }
        _ => {}
    }
    if arguments.first().map(String::as_str) == Some("progress") {
        let [_, host] = arguments else {
            return Err(ended::asked_wrongly("progress takes one HOST", "progress"));
        };
        return progress(host);
    }
    if arguments.first().map(String::as_str) == Some("relay") {
        let (Some(listen), Some(to)) = (given("--listen"), given("--to")) else {
            return Err(ended::asked_wrongly(
                "relay needs --listen and --to",
                "relay",
            ));
        };
        return relaying(&listen, &to);
    }
    if arguments.first().map(String::as_str) == Some("lines") {
        return lining(given("--config"));
    }
    if arguments.first().map(String::as_str) == Some("prove-standing") {
        return proving_standing(given("--config"));
    }
    if arguments.first().map(String::as_str) == Some("keep") {
        return keep::keeping(given("--folder"));
    }
    match arguments
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        ["passed", "forget", commit] => return passed::forgetting(&state(), commit),
        ["passed", "import"] => return passed::importing(&state()),
        ["passed", "ran", commit] => return passed::saying_ran(&state(), commit),
        ["passed", ..] => {
            return Err(ended::asked_wrongly(
                "passed takes forget COMMIT, import or ran COMMIT",
                "passed",
            ));
        }
        _ => {}
    }
    if arguments.first().map(String::as_str) == Some("stand") {
        return standing_by(given("--standing"), given("--beside"));
    }
    serving(arguments, given)
}

/// A relay from `listen` to `to`, a lender's tunnel's stand-in, until its
/// helper tells it to leave or its callers are hung up on for too long.
fn relaying(listen: &str, to: &str) -> Result<(), Failure> {
    // Heard before it listens, so a helper's word to leave never ends it
    // with its connections.
    // SAFETY: the handler only stores to an atomic.
    unsafe {
        let handler = leaving as *const () as libc::sighandler_t;
        libc::signal(libc::SIGUSR1, handler);
    }
    // Every address the name gives, since ssh tries localhost's ::1
    // before 127.0.0.1, as the tunnel it stands in for listens on both.
    let listeners = relay::listening(listen);
    if listeners.is_empty() {
        return Err(Failure::new(
            Exit::Failed,
            format!("couldn't listen on {listen}"),
        ));
    }
    // Said, so a helper offers its builder only once its way stands.
    println!("listening");
    let _ = std::io::Write::flush(&mut std::io::stdout());
    // Less than the 30 s root's ssh gives a server to answer (nixos.nix),
    // so a caller hung up on hears it before ssh gives up on its own.
    // Root's build hook drops its only builder for a build on one failed
    // connection, and a tunnel opening under load took past 9 s three
    // times in an hour on 14 November. Three minutes of callers hung up
    // on is a tunnel that won't come back, where one connection's failing
    // to open is dialled again.
    let gave_up = relay::Relay {
        to: to.to_owned(),
        within: Duration::from_secs(25),
        pause: Duration::from_millis(250),
        gives_up: Duration::from_secs(180),
        moved: relay::Moved::named(),
    }
    .serve_all(listeners, &LEAVING);
    // serve_all said why as it gave up, so the kit adds nothing.
    if gave_up {
        return Err(Failure::new(
            Exit::Failed,
            format!("the relay to {to} gave up, as its callers were hung up on"),
        )
        .said_already());
    }
    Ok(())
}

/// A lender that stands by for its builder's standing, ending as a
/// superseded lender does once it hands over.
fn standing_by(standing: Option<String>, beside: Option<String>) -> Result<(), Failure> {
    if standing.is_none() && beside.is_none() {
        return Err(ended::asked_wrongly(
            "stand needs --standing or --beside",
            "stand",
        ));
    }
    ended::lent(stand(
        standing.map(PathBuf::from),
        beside.map(PathBuf::from),
    ))
}

/// `lines`: copies the lender's lines into root's, for as long as it runs,
/// keeping gcloud's state where systemd's `StateDirectory` says.
fn lining(config: Option<String>) -> Result<(), Failure> {
    let Some(config) = config else {
        return Err(ended::asked_wrongly("lines needs --config", "lines"));
    };
    let settings = match fs::read_to_string(&config)
        .map_err(|error| error.to_string())
        .and_then(|text| lines::settings(&text))
    {
        Ok(settings) => settings,
        Err(error) => {
            return Err(
                Failure::new(Exit::Config, format!("{config} can't be read")).because(error),
            );
        }
    };
    let state = env::var_os("STATE_DIRECTORY").map_or_else(
        || PathBuf::from("/var/lib/nix-offload-lines"),
        PathBuf::from,
    );
    lines::serve(&settings, state.join("gcloud"))
}

/// `prove-standing`: proves, as root, the standing builders lines' settings
/// name, saying how they stand in `proof::STANDING`, for as long as it runs.
fn proving_standing(config: Option<String>) -> Result<(), Failure> {
    let Some(config) = config else {
        return Err(ended::asked_wrongly(
            "prove-standing needs --config",
            "prove-standing",
        ));
    };
    match fs::read_to_string(&config)
        .map_err(|error| error.to_string())
        .and_then(|text| standing::standing(&text))
    {
        Ok(builders) => standing::serve(builders, "nix", Path::new(proof::STANDING)),
        Err(error) => {
            Err(Failure::new(Exit::Config, format!("{config} can't be read")).because(error))
        }
    }
}

/// Starts `there` again marked as work run elsewhere, and as sent first
/// when asked `--first`, unless it's so marked already; how it ended when
/// it couldn't. A `there` run only waits on a build done elsewhere, mostly,
/// so the machine's slices let its processes go after other work when
/// memory runs out; they read the mark from where each started, so this
/// starts again with it rather than only handing it on. `--first` is
/// carried the same way, so everything the run starts and sends is marked
/// sent first, and taken off what it's asked. `--lent-only` is carried the
/// same way, so a `there` the run starts goes nowhere else either. Each is
/// looked for before a `--`, past which the words are a command's.
fn marked(arguments: &[String]) -> Option<Failure> {
    let ours = arguments
        .iter()
        .position(|word| word == "--")
        .unwrap_or(arguments.len());
    let asked = |flag: &str| arguments[..ours].iter().any(|word| word == flag);
    let (asked_first, asked_lent) = (asked("--first"), asked("--lent-only"));
    if !asked_first
        && !asked_lent
        && env::var_os(ELSEWHERE.0).is_some_and(|value| value == ELSEWHERE.1)
    {
        return None;
    }
    let mut again = again();
    again
        .arg("there")
        .args(arguments.iter().enumerate().filter_map(|(at, word)| {
            (at >= ours || (word != "--first" && word != "--lent-only")).then_some(word)
        }))
        .env(ELSEWHERE.0, ELSEWHERE.1);
    if asked_first {
        again.env(there::FIRST.0, there::FIRST.1);
    }
    if asked_lent {
        again.env(there::LENT_ONLY.0, there::LENT_ONLY.1);
    }
    let again = again.exec();
    Some(Failure::new(Exit::Bug, "couldn't start there again marked").because(again.to_string()))
}

/// `there`: builds a flake's attributes where src/there.rs says, each part
/// journalled, exiting with the first status that failed.
/// A run of derivations one of which no lent builder builds for, refused
/// with 64 and what would build it.
fn foreign(asked: &there::Asked, dir: &Path) -> Option<Failure> {
    let (path, system, systems) = there::foreign_derivation(asked, dir)?;
    Some(
        ended::usage(format!(
            "no lent builder takes {path}, which builds for {system}"
        ))
        .because(format!(
            "The lent builders build for {} alone; nix build --builders naming a builder \
                 for {system} builds it.",
            systems.join(", ")
        )),
    )
}

/// Builds each machine nearby's part of `near`, journalled, saying the
/// first status that failed. What a machine that can't fetch the flake,
/// or that ssh can't reach, takes goes with the rest in `left`, to the
/// builders, rather than failing there, when a lent builder in `dir`
/// builds for it; the rest waits, ending the run unsent. A Mac with no
/// room is sent nothing, as one ssh can't reach ([`held_nearby`]).
fn built_nearby(
    near: &[(&there::Nearby, there::Asked)],
    mut left: Option<there::Asked>,
    dir: &Path,
    scratch: &Path,
    journal: &Path,
) -> (i32, Option<there::Asked>) {
    let mut status = 0;
    for (machine, part) in near {
        let began = now();
        let held = held_nearby(machine, scratch);
        let ran = (!held && there::fetches(machine, part, scratch, "ssh"))
            .then(|| there::nearby_run(machine, part, scratch, ["ssh", "tar", "nix"]))
            .flatten();
        let Some(ran) = ran else {
            // ssh failing there may be the machine going, which what it
            // said within the last minute can't know, so it's asked afresh.
            if !held {
                machines::forget_reached(&machine.host);
            }
            let (goes, stays) = there::lent_builds(part, dir);
            if !stays.is_empty() {
                eprintln!(
                    "nix-offload: no lent builder builds {} of what {} nearby was to take, so it \
                     waits for {}",
                    stays.len(),
                    machine.host,
                    machine.host
                );
                if status == 0 {
                    status = there::UNSENT;
                }
            }
            if let Some(goes) = goes {
                left = Some(there::rejoined(left.take(), &goes));
            }
            continue;
        };
        let ended = now();
        there::note(
            journal,
            ended,
            ended.saturating_sub(began),
            None,
            part,
            &ran,
        );
        if ran.status != 0 {
            eprintln!(
                "nix-offload: {}'s part, {}, ended {}",
                machine.host,
                part.attributes().join(" "),
                ran.status
            );
        }
        // The first that failed says how the whole did.
        if status == 0 {
            status = ran.status;
        }
    }
    (status, left)
}

/// Whether `machine`, a Mac, has no room for more, saying the load it
/// goes or waits at, by its load alone (`changed::mac::room`): a person's
/// own part, which its turn there bounds beside another, never waits on
/// another's build as a run for its system does (`held_for_a_mac`). A run
/// whose caller asked already isn't asked again, nor is a machine that
/// builds for no Mac.
fn held_nearby(machine: &there::Nearby, scratch: &Path) -> bool {
    use changed::mac;
    let asked_already = env::var_os(ROOM_ASKED.0).is_some_and(|value| value == ROOM_ASKED.1);
    if asked_already || !machine.systems.iter().any(|system| system == mac::SYSTEM) {
        return false;
    }
    match mac::room(&machine.host, there::load_of(machine, scratch, "ssh")) {
        Ok(room) => {
            eprintln!("nix-offload: {room}, so what it was to take goes to it");
            false
        }
        Err(why) => {
            eprintln!("nix-offload: {why}, so it's sent none of what it was to take");
            true
        }
    }
}

/// Refuses `--here` and `--here-when-idle` under `--lent-only`, which
/// builds on a lent builder alone.
fn here_refused(asked: &there::Asked) -> Option<Failure> {
    (there::lent_only() && asked.builds_here())
        .then(|| ended::usage("--lent-only builds on a lent builder alone, so it takes no --here"))
}

/// The person's machines nearby, none under `--lent-only`: a machine
/// nearby is one of the person's, which builds as them.
fn nearby() -> Vec<there::Nearby> {
    if there::lent_only() {
        return Vec::new();
    }
    nearby_listed()
}

/// This same program, to run again (`program`).
pub fn again() -> Command {
    Command::new(program(Path::new("/proc/self/exe")))
}

/// This same program's path: `proc_exe` where it is, as on Linux, which
/// names the very file that runs even once a switch replaced it; else, as
/// on a Mac, which has no `/proc/self/exe` and where on 16 November a
/// `there` ended at once, unable to start itself again, the file it was
/// started from, past any link a switch would repoint. Should neither be
/// known, the one named `nix-offload` first in `PATH`.
fn program(proc_exe: &Path) -> PathBuf {
    if proc_exe.exists() {
        return proc_exe.to_owned();
    }
    env::current_exe().map_or_else(
        |_| PathBuf::from("nix-offload"),
        |path| fs::canonicalize(&path).unwrap_or(path),
    )
}

/// The person's machines nearby, as listed, `--lent-only` or not.
fn nearby_listed() -> Vec<there::Nearby> {
    fs::read_to_string(config().join("nix-offload/nearby"))
        .map(|text| there::nearby(&text))
        .unwrap_or_default()
}

fn there(arguments: &[String]) -> Result<(), Failure> {
    if let Some(failure) = marked(arguments) {
        return Err(failure);
    }
    // What this `there` takes, a word each, so a caller can tell it from
    // an older one, which refuses the flag with 64.
    if arguments == ["--takes"] {
        println!("{}", there::TAKES.join("\n"));
        return Ok(());
    }
    let (waiting, arguments) = waiting_for(arguments);
    if let [first, rest @ ..] = arguments
        && first == "--changed"
    {
        return changed_there(rest, waiting, true);
    }
    if let [first, rest @ ..] = arguments
        && first == "--run"
    {
        return run_there(rest, waiting);
    }
    let Some(asked) = there::asked(arguments) else {
        return Err(ended::asked_wrongly(
            "there takes a flake and its attributes, or derivations",
            "there",
        ));
    };
    let scratch = scratch();
    if let Some(refused) = here_refused(&asked) {
        return Err(refused);
    }
    // Pinned before anything waits, so what's built and kept as passed is
    // the commit it was asked at, however far its branch moves meanwhile.
    let asked = asked.pinned(&scratch);
    let journal = state().join("nix-offload/there.jsonl");
    let kept = Kept::of(&asked);
    let Some(asked) = kept.unpassed(asked, &journal) else {
        return Ok(());
    };
    if let Some(status) = built_idle(&asked, &journal) {
        return built(status);
    }
    // A test names its own folder, as changed-checks lets one.
    let dir = env::var_os("NIX_OFFLOAD_DIR").map_or_else(|| PathBuf::from(DIR), PathBuf::from);
    if let Some(refused) = foreign(&asked, &dir) {
        return Err(refused);
    }
    let vm_dir =
        env::var_os("NIX_OFFLOAD_VM_DIR").map_or_else(|| PathBuf::from(VM_DIR), PathBuf::from);
    let nearby = nearby();
    let (near, left) = there::shared(&asked, &nearby, |machine| {
        machines::nearby_answers(&machine.address, &machine.host)
    });
    let (left, owed) = there::owed(left, &nearby, &this_system());
    if !owed.is_empty() {
        eprintln!(
            "nix-offload: no machine nearby that builds for {} answers, and a lent builder \
             can't, so it's left unbuilt, owed for when one does",
            owed.join(", ")
        );
    }
    let (mut status, left) = built_nearby(&near, left, &dir, &scratch, &journal);
    let (vm_dirs, vm_parts, left) = vm_apart_from(left, &vm_dir);
    // The rest goes to a second builder while the first is full, when a
    // lender runs to make one.
    let more_dir = more_dir();
    let first_dir = dir;
    let left_dir = match &left {
        Some(_) if there::lends(&more_dir) => more_than(&first_dir, &more_dir, &scratch),
        _ => first_dir.clone(),
    };
    let mut parts: Vec<(&PathBuf, Option<there::Asked>)> = vm_dirs.iter().zip(vm_parts).collect();
    parts.push((&left_dir, left));
    // A VM builder past the first that doesn't come leaves its tests to
    // the first; the first and second lent builders stand in for each other.
    let fallbacks: Vec<(&PathBuf, &PathBuf)> = std::iter::once((&more_dir, &first_dir))
        .chain(there::lends(&more_dir).then_some((&first_dir, &more_dir)))
        .chain(vm_dirs.iter().skip(1).map(|dir| (dir, &vm_dirs[0])))
        .collect();
    let vm_dirs: Vec<&PathBuf> = vm_dirs.iter().collect();
    let ran = at_once(&parts, &vm_dirs, vm_room(), &fallbacks, &scratch, waiting);
    for (part, began, ended, waited, ran) in &ran {
        there::note(
            &journal,
            *ended,
            ended.saturating_sub(*began),
            Some(*waited),
            part,
            ran,
        );
        // The first that failed says how the whole did.
        if status == 0 {
            status = ran.status;
        }
    }
    // What's owed hasn't passed, so the run says it's unbuilt, as a
    // changed run with no builder does, and isn't kept.
    if status == 0 && !owed.is_empty() {
        status = i32::from(changed::UNBUILT);
    }
    // Which of a build that failed passed isn't known, so only a whole
    // that passed is kept.
    if status == 0 {
        kept.keep(&asked);
    }
    built(status)
}

/// How a `there` build ends: done, or with the first status that failed,
/// nix's own or one of `there`'s, which its callers read by number.
fn built(status: i32) -> Result<(), Failure> {
    ended::status(status, || {
        "what was asked didn't build, as the lines above say".to_owned()
    })
}

/// How a `there` run waits for a builder, and the arguments left: a run
/// worth only a builder that already stands says so first, with
/// `--never-ask`, so it never has the lender make one for it.
fn waiting_for(arguments: &[String]) -> (there::Waiting, &[String]) {
    match arguments.split_first() {
        Some((first, rest)) if first == "--never-ask" => (
            there::Waiting {
                asks: false,
                ..there::WAITING
            },
            rest,
        ),
        _ => (there::WAITING, arguments),
    }
}

/// `left` with its VM tests taken apart, split between the VM builders,
/// whose folders it says with a part for each, beside what's left. VM
/// tests go to a builder of their own, so they never share one with big
/// builds: beside a kernel build on 2 November they ran past their
/// timeouts. Only where a lender for them has ever run here, in
/// `vm_dir`; with none, they go the lent way.
fn vm_apart_from(
    left: Option<there::Asked>,
    vm_dir: &Path,
) -> (
    Vec<PathBuf>,
    Vec<Option<there::Asked>>,
    Option<there::Asked>,
) {
    let patterns = vm_patterns();
    let (vm, left) = match left {
        Some(part) if vm_apart(&patterns, vm_dir) => {
            there::vm_tests(&part, &patterns, &there::featured(&part, "nix"))
        }
        left => (None, left),
    };
    let vm_dirs = vm_dirs(vm_dir);
    let vm_parts = overflowed(vm, &vm_dirs);
    (vm_dirs, vm_parts, left)
}

/// How far the builds on the lent builder ssh calls `host` have got, as
/// src/progress.rs says, asked of the first lender whose store names it.
fn progress(host: &str) -> Result<(), Failure> {
    let vm_dir =
        env::var_os("NIX_OFFLOAD_VM_DIR").map_or_else(|| PathBuf::from(VM_DIR), PathBuf::from);
    let dir = env::var_os("NIX_OFFLOAD_DIR").map_or_else(|| PathBuf::from(DIR), PathBuf::from);
    let lent = [dir, more_dir()]
        .into_iter()
        .chain(vm_dirs(&vm_dir))
        .filter_map(|dir| standing(&dir))
        .filter_map(|store| there::builder(store.lines().next()?.trim()))
        .find(|lent| lent.host == host);
    let Some(lent) = lent else {
        return Err(Failure::new(
            Exit::TryAgain,
            format!("no lender lends a builder called {host} that root vouches for"),
        ));
    };
    let scratch = env::var_os("XDG_RUNTIME_DIR")
        .map_or_else(env::temp_dir, PathBuf::from)
        .join("nix-offload-progress");
    let answer = there::reach(&lent, &scratch, "ssh").and_then(|(mut command, known, said)| {
        command
            .args(["-o", "ConnectTimeout=10"])
            .arg(&lent.host)
            .arg(progress::ASKED);
        let answer = command.output().map_err(|error| error.to_string());
        for file in [&known, &said] {
            let _ = fs::remove_file(file);
        }
        answer
    });
    let read = answer
        .ok()
        .filter(|answer| answer.status.success())
        .and_then(|answer| progress::read(&String::from_utf8_lossy(&answer.stdout)));
    let Some(figures) = read else {
        return Err(Failure::new(
            Exit::Unavailable,
            format!("ssh couldn't reach {host} for its progress"),
        ));
    };
    println!("{}", progress::line(host, figures, now()));
    Ok(())
}

/// The folders of every VM lender, the first's `vm_dir` first, then the
/// second's and those past it.
fn vm_dirs(vm_dir: &Path) -> Vec<PathBuf> {
    let vm_more_dir = env::var_os("NIX_OFFLOAD_VM_MORE_DIR")
        .map_or_else(|| PathBuf::from(VM_MORE_DIR), PathBuf::from);
    [vm_dir.to_path_buf(), vm_more_dir]
        .into_iter()
        .chain(vm_extra_dirs())
        .collect()
}

/// The number of the last VM lender's folder, which nixos.nix makes and
/// home.nix lends to as well, all three reading this one file so they
/// can't drift apart.
const LAST_VM_LANE: &str = include_str!("../last-vm-lane.txt");

/// The folders of the VM lenders past the second, /run/nix-offload-vm-3
/// to the one `last-vm-lane.txt` names, as many as home.nix's vmTests.extra
/// may lend from; `NIX_OFFLOAD_VM_EXTRA_DIRS` names others, a path list.
fn vm_extra_dirs() -> Vec<PathBuf> {
    env::var_os("NIX_OFFLOAD_VM_EXTRA_DIRS")
        .map_or_else(own_vm_extra_dirs, |dirs| env::split_paths(&dirs).collect())
}

/// The machine's own VM lender folders past the second, from 3 to the
/// last lane. A number that doesn't parse gives none, which the test of
/// these folders fails on, rather than a machine finding out.
fn own_vm_extra_dirs() -> Vec<PathBuf> {
    let last: u32 = LAST_VM_LANE.trim().parse().unwrap_or(2);
    (3..=last)
        .map(|number| PathBuf::from(format!("{VM_DIR}-{number}")))
        .collect()
}

/// The VM tests `vm` split between the VM builders of `vm_dirs`, a part
/// for each, those past the first's room going on to the next while a
/// lender runs in its folder to make one, counting what other runs still
/// going have sent each and wait to send it. With no lender past the
/// first, all go to the first. Those waiting count, since a run sent
/// where others wait goes only after them: on 4 November ten runs waited
/// 51 minutes on the first builder, with 2 of its 4 claimed, while the
/// second sat idle. A builder past the first that stands while root
/// doesn't vouch for it takes none, since a run split there would wait
/// on root while the first had room. One kept for the runs that name it
/// takes none past the first, as it's the first only for those runs.
fn overflowed(vm: Option<there::Asked>, vm_dirs: &[PathBuf]) -> Vec<Option<there::Asked>> {
    let mut parts: Vec<Option<there::Asked>> = vm_dirs.iter().map(|_| None).collect();
    let Some(part) = vm else {
        return parts;
    };
    let spills_onto =
        |index: &usize| there::lends(&vm_dirs[*index]) && !there::named_only(&vm_dirs[*index]);
    let lending: Vec<usize> = (0..vm_dirs.len())
        .filter(|index| *index == 0 || (spills_onto(index) && !there::unvouched(&vm_dirs[*index])))
        .collect();
    if lending.len() == 1 {
        parts[0] = Some(part);
        return parts;
    }
    let builds = vm_room();
    let busy_in =
        |index: &usize| there::claimed(&vm_dirs[*index]) + there::waiting(&vm_dirs[*index]);
    // Those left out as root vets them still come, so none more is asked
    // for in their place.
    let vetting = (1..vm_dirs.len())
        .filter(|index| spills_onto(index) && there::unvouched(&vm_dirs[*index]))
        .count();
    let lending = warm_of(&part, lending, vm_dirs, builds, vetting, busy_in);
    let busy: Vec<usize> = lending.iter().map(busy_in).collect();
    let split = there::overflow(&part, &busy, builds);
    let onward: usize = split.iter().skip(1).flatten().map(there::count).sum();
    if onward > 0 {
        let to = if lending.len() == 2 {
            "a second VM builder"
        } else {
            "the other VM builders"
        };
        eprintln!(
            "nix-offload: the VM builder has {} VM tests sent or waiting from runs here and \
             runs {builds} at once, so {onward} go to {to}",
            busy[0],
        );
    }
    for (index, piece) in lending.into_iter().zip(split) {
        parts[index] = piece;
    }
    parts
}

/// Of the VM builders `lending`, by index into `vm_dirs`, those `part`'s
/// VM tests may go to: the warm ones while any stands, else all of them,
/// since a cold one's first tests build for an hour what a warm one holds.
/// Where the warm ones' room runs out, cold ones that aren't on their way
/// are asked for, to warm for the runs after, less the `vetting` ones
/// standing as root vets them, which are on their way too.
fn warm_of(
    part: &there::Asked,
    lending: Vec<usize>,
    vm_dirs: &[PathBuf],
    builds: usize,
    vetting: usize,
    busy_in: impl Fn(&usize) -> usize,
) -> Vec<usize> {
    let (warm, cold): (Vec<usize>, Vec<usize>) = lending
        .iter()
        .partition(|index| there::warm(&vm_dirs[**index]));
    if warm.is_empty() || cold.is_empty() {
        return lending;
    }
    let busy: Vec<usize> = warm.iter().map(&busy_in).collect();
    let (coming, unasked): (Vec<usize>, Vec<usize>) = cold
        .iter()
        .partition(|index| there::coming(&vm_dirs[**index]));
    let asking = there::to_warm(there::count(part), &busy, builds, coming.len() + vetting)
        .min(unasked.len());
    for index in &unasked[..asking] {
        there::want(&vm_dirs[*index]);
    }
    if coming.is_empty() && asking == 0 {
        return warm;
    }
    let cold_said = match coming.len() {
        0 => String::new(),
        1 => "1 VM builder is still standing or warming, so ".to_owned(),
        count => format!("{count} VM builders are still standing or warming, so "),
    };
    let warm_said = match warm.len() {
        1 => "the warm one".to_owned(),
        count => format!("the {count} warm ones"),
    };
    let asked_said = match asking {
        0 => String::new(),
        1 => ", and 1 more is asked for, to warm for the runs after".to_owned(),
        asking => format!(", and {asking} more are asked for, to warm for the runs after"),
    };
    eprintln!("nix-offload: {cold_said}VM tests go to {warm_said}{asked_said}");
    warm
}

/// Sends each of `parts` to its builder at once, on a thread of its own
/// with scratch of its own, since each goes to a builder of its own: one
/// after another, the second VM builder's tests waited out the first's.
/// Those sent to one of `vm_dirs` go once it has room for them, as many
/// as `room`, and are claimed there while they go. Says how each ran,
/// with when it began and ended and the seconds it waited for room and
/// for its builder to stand, in the order given. Each waits for its
/// builder as `waiting` says.
fn at_once<'a>(
    parts: &'a [(&PathBuf, Option<there::Asked>)],
    vm_dirs: &[&PathBuf],
    room: usize,
    fallbacks: &[(&PathBuf, &PathBuf)],
    scratch: &Path,
    waiting: there::Waiting,
) -> Vec<(&'a there::Asked, u64, u64, u64, there::Ran)> {
    thread::scope(|scope| {
        let sending: Vec<_> = parts
            .iter()
            .enumerate()
            .filter_map(|(index, (dir, part))| Some((index, *dir, part.as_ref()?)))
            .map(|(index, dir, part)| {
                let scratch = scratch.join(index.to_string());
                let vm = vm_dirs.contains(&dir);
                // The other VM builders, while a lender runs to make each,
                // which a VM part waiting for room goes to once one has some,
                // all but those kept for the runs that name them.
                let others: Vec<&Path> = vm_dirs
                    .iter()
                    .filter(|other| {
                        **other != dir && there::lends(other) && !there::named_only(other)
                    })
                    .map(|other| other.as_path())
                    .collect();
                scope.spawn(move || {
                    let began = now();
                    let (dir, _claim, first) = if vm {
                        let rank = there::rank(part);
                        there::admitted(dir, &others, there::count(part), room, rank, waiting)
                    } else {
                        (dir.as_path(), None, waiting)
                    };
                    let roomed = now().saturating_sub(began);
                    let waits = [first, waiting];
                    let (ran, stood) = sent(part, dir, vm, fallbacks, &scratch, waits);
                    (part, began, now(), roomed + stood, ran)
                })
            })
            .collect();
        sending
            .into_iter()
            .map(|sending| sending.join().expect("a part's thread panicked"))
            .collect()
    })
}

/// Builds `part` on the builder `dir`'s lender lends, asked for and waited
/// on rather than built here while a lender runs: a builder stands about
/// 70 s after it's asked for. One asked for beside another, as `fallbacks`
/// pairs them, that doesn't come leaves the build to the other. VM tests,
/// as `vm` says these are, wait through their lender's restart. It waits
/// for its builder as `first` says, what's left once it held for root,
/// and for one lost mid-run as `waiting` says. Says how it ran, and the
/// seconds it waited for its builder to stand.
fn sent(
    part: &there::Asked,
    dir: &Path,
    vm: bool,
    fallbacks: &[(&PathBuf, &PathBuf)],
    scratch: &Path,
    [first, waiting]: [there::Waiting; 2],
) -> (there::Ran, u64) {
    let asked = std::time::Instant::now();
    let (dir, store) = match standing(dir).or_else(|| there::waited(dir, first)) {
        None => match fallbacks.iter().find(|(beside, _)| beside.as_path() == dir) {
            Some((_, other)) => (other.as_path(), standing(other)),
            None => (dir, None),
        },
        store => (dir, store),
    };
    // A VM test waits through its lender's restart, as a switch does it,
    // rather than going anywhere else: sent the lent way meanwhile, good
    // ones timed out beside kernel builds.
    let store = match store {
        None if vm && !there::lends(dir) => {
            there::stood_again(dir, first, std::time::Instant::now())
        }
        store => store,
    };
    // Said apart from the lent builder's absence, which the line after it
    // reads as.
    if vm && store.is_none() && there::unvouched(dir) {
        eprintln!(
            "nix-offload: the VM builder in {} stands, but root never vouched for it, so its \
             VM tests weren't sent, nor the lent way",
            dir.display()
        );
    } else if vm && store.is_none() {
        eprintln!(
            "nix-offload: the VM lender in {} lent no VM builder, so its VM tests weren't \
             sent, nor the lent way",
            dir.display()
        );
    }
    let stood = asked.elapsed().as_secs();
    // A builder lost mid-run goes to the one beside it while that one
    // stands, rather than waiting for its own: a tunnel that died at 20:51
    // on 7 November left builds waiting 8 minutes for the next builder.
    // VM tests wait for their own lender, as above. Within the outage's
    // `longest` only, so two lost builders never hand a run back and
    // forth for good.
    let beside = fallbacks
        .iter()
        .find(|(from, _)| from.as_path() == dir)
        .map(|(_, to)| to.as_path())
        .filter(|_| !vm);
    let again = |since: std::time::Instant| {
        beside
            .filter(|_| since.elapsed() < waiting.longest)
            .and_then(standing)
            .or_else(|| there::stood_again(dir, waiting, since))
    };
    // Every lent builder signs with the same keys, which the first
    // lender's folder names, VM builders' included.
    let first = env::var_os("NIX_OFFLOAD_DIR").map_or_else(|| PathBuf::from(DIR), PathBuf::from);
    let ran = there::run(
        part,
        store.as_deref(),
        &again,
        waiting,
        scratch,
        ["ssh", "nix"],
        there::signatures(&first),
    );
    (ran, stood)
}

/// What a `there` keeps when it passes, where it may: a check that passed
/// at a commit passes there again, wherever it went, and no store a
/// builder lent since may hold it (`passed.rs`).
struct Kept {
    passed: passed::Passed,
    /// Where checks asked for are kept, when they may be.
    checks: Option<passed::At>,
    /// Where subtests asked for are kept, and what's run, when they may be.
    subtests: Option<(passed::At, String)>,
}

impl Kept {
    fn of(asked: &there::Asked) -> Self {
        Self {
            passed: passed::Passed::here(&state()),
            checks: asked.kept_at(),
            subtests: asked.subtests_at(),
        }
    }

    /// What's asked, without what passed already, each saying so and
    /// journalled; none when nothing's left to build.
    fn unpassed(&self, asked: there::Asked, journal: &Path) -> Option<there::Asked> {
        if let (Some((at, run)), Some(out)) = (&self.subtests, asked.out())
            && let Some(before) = self.passed.subtests(at, run)
        {
            let same = fs::canonicalize(&before).ok() == fs::canonicalize(out).ok();
            if same || there::copied(&before, out) == 0 {
                eprintln!(
                    "nix-offload: these subtests passed at {} already, so they aren't run again; \
                     their pictures are copied from {}, and `nix-offload passed forget {}` \
                     has them run again",
                    at.short(),
                    before.display(),
                    at.commit
                );
                there::note_already(journal, now(), &asked, asked.attributes());
                return None;
            }
        }
        let Some(at) = &self.checks else {
            return Some(asked);
        };
        let before = self.passed.checks(at);
        let already: Vec<String> = asked
            .attributes()
            .iter()
            .filter(|attribute| before.contains(attribute))
            .cloned()
            .collect();
        if already.is_empty() {
            return Some(asked);
        }
        for check in &already {
            eprintln!(
                "nix-offload: {check} passed at {} already, so it isn't built again",
                at.short()
            );
        }
        eprintln!(
            "nix-offload: `nix-offload passed forget {}` has it built again",
            at.commit
        );
        there::note_already(journal, now(), &asked, &already);
        asked.without(&already)
    }

    /// Keep that what was asked passed.
    fn keep(&self, asked: &there::Asked) {
        if let Some(at) = &self.checks {
            self.passed.keep_checks(at, asked.attributes());
        }
        if let (Some((at, run)), Some(out)) = (&self.subtests, asked.out()) {
            // Kept whole, since the next may run in another folder.
            let out = fs::canonicalize(out).unwrap_or_else(|_| out.to_owned());
            self.passed.keep_subtests(at, run, &out);
        }
    }
}

/// The lender's folder a build goes to: `more`, the second's, while the
/// first's builder is full or doesn't answer, as `there::to_second`
/// decides, else `first`. A build waits for the first while none stands,
/// as it always has, but not while one stands that root doesn't vouch
/// for yet and the second's does: it would wait on root with the second
/// idle.
fn more_than(first: &Path, more: &Path, scratch: &Path) -> PathBuf {
    let Some(stands) = standing(first) else {
        if there::unvouched(first) && standing(more).is_some() {
            eprintln!(
                "nix-offload: root doesn't vouch for the lent builder yet, and the second's it \
                 does, so this goes to the second"
            );
            return more.to_owned();
        }
        return first.to_owned();
    };
    let busy = |store: String| there::pressure(&store, scratch, "ssh");
    let waits = busy(stands);
    // A second that stands but root doesn't yet vouch for is no idle
    // second to ask for: a build sent there waits on root, so it counts
    // as one standing whose load can't be read, which keeps builds here.
    let second = || match standing(more) {
        Some(store) => there::Second::Stands(busy(store)),
        None if there::unvouched(more) => there::Second::Stands(None),
        None => there::Second::Unlent,
    };
    match waits {
        // The second is read only once the first is full.
        Some(waits) if waits >= there::FULL && there::to_second(Some(waits), &second()) => {
            eprintln!(
                "nix-offload: the lent builder's tasks wait {waits:.0}% of the time for its CPUs, \
                 so this goes to a second"
            );
            more.to_owned()
        }
        None if there::to_second(None, &second()) => {
            eprintln!("nix-offload: the lent builder didn't answer, so this goes to a second");
            more.to_owned()
        }
        _ => first.to_owned(),
    }
}

/// How many VM tests each VM builder runs at once, as XDG's config folder
/// says in `nix-offload/vm-builds`, else as many as its helper is told by
/// default.
fn vm_room() -> usize {
    fs::read_to_string(config().join("nix-offload/vm-builds"))
        .ok()
        .and_then(|text| text.trim().parse::<usize>().ok())
        .filter(|builds| *builds > 0)
        .unwrap_or(there::VM_ROOM)
}

/// `asked` built here and journalled, when it asks to be while this
/// machine is idle and is, by `NIX_OFFLOAD_CPU_PRESSURE`'s file, else
/// /proc/pressure/cpu, with how it exits; none otherwise.
fn built_idle(asked: &there::Asked, journal: &Path) -> Option<i32> {
    let pressure = env::var_os("NIX_OFFLOAD_CPU_PRESSURE")
        .map_or_else(|| PathBuf::from("/proc/pressure/cpu"), PathBuf::from);
    let pressure = || fs::read_to_string(&pressure).unwrap_or_default();
    let began = now();
    let ran = there::here_when_idle(asked, &pressure, &vm_patterns(), "nix")?;
    let ended = now();
    there::note(
        journal,
        ended,
        ended.saturating_sub(began),
        None,
        asked,
        &ran,
    );
    Some(ran.status)
}

/// The patterns of VM tests' attributes, from XDG's config folder; none
/// where the machine names no builder for them.
fn vm_patterns() -> Vec<String> {
    fs::read_to_string(config().join("nix-offload/vm-tests"))
        .map(|text| there::patterns(&text))
        .unwrap_or_default()
}

/// Whether VM tests go to a builder of their own: while the machine names
/// some and a lender for them has run since it started, as its `lender`
/// file says, whether or not it runs this moment, since a switch restarts
/// it.
fn vm_apart(patterns: &[String], vm_dir: &Path) -> bool {
    !patterns.is_empty() && vm_dir.join("lender").exists()
}

/// The system this machine builds for, as Nix names it.
fn this_system() -> String {
    let os = match env::consts::OS {
        "macos" => "darwin",
        os => os,
    };
    format!("{}-{os}", env::consts::ARCH)
}

/// The store a lender's folder names while its builder stands and root
/// vouches for it.
fn standing(dir: &Path) -> Option<String> {
    there::vouched_store(dir)
}

/// Says why a passing suite run in a warm target isn't kept as passed,
/// which `keep_ran` sees to.
fn say_a_warm_pass_isnt_kept(status: Option<i32>, words: &[String]) {
    if status == Some(0) && cargo::runs_warm(words) {
        eprintln!(
            "nix-offload: a suite in a warm target keeps no pass, since an earlier run \
             there could have written what it read"
        );
    }
}

/// `there --run`: a command run on a lent builder, in a checkout of the
/// commit here, ending with its status (`command.rs` says why). With no
/// builder standing it waits for one as `waiting` says, and with none it
/// exits `NOT_TAKEN`, having run nothing, here or anywhere. A run that
/// ends 0 is kept as passed at the commit, with its words and copies
/// (`passed.rs`), which `nix-offload passed ran COMMIT` says, unless it
/// went as files cut to its cones (`archived.rs`) or ran a suite in a
/// warm target (`cargo::runs_warm`).
fn run_there(arguments: &[String], waiting: there::Waiting) -> Result<(), Failure> {
    let asked = command::asked(arguments).map_err(|why| ended::asked_wrongly(why, "there"))?;
    let not_run = |why: &str| ended::backs_off(taken_nowhere(), format!("{why}, so nothing ran"));
    let dir = env::var_os("NIX_OFFLOAD_DIR").map_or_else(|| PathBuf::from(DIR), PathBuf::from);
    let more = more_dir();
    let lenders = [dir.as_path(), more.as_path()];
    // Before packing, which for a tree off GitHub gzips its whole cone
    // here, only to be thrown away when nothing could take it.
    if !lenders
        .iter()
        .any(|lender| standing(lender).is_some() || there::lends(lender))
    {
        return Err(not_run("no builder is lent"));
    }
    let scratch = scratch();
    let Ok(folder) = env::current_dir() else {
        return Err(not_run("this folder can't be read"));
    };
    let (at, checkout, bundle) = match there::for_run(&folder, &scratch) {
        Ok(sending) => sending.sent(),
        Err(why) => return Err(not_run(&why)),
    };
    // One name wherever it goes, since a run sent on to the other builder
    // starts afresh there and never shares this one's files.
    let run = command::run_name();
    let sending = there::Sending {
        command: command::command(&checkout, &asked, &run),
        bundle,
    };
    let cuts = there::Cuts::default();
    let started = std::time::Instant::now();
    let mut took = None;
    let mut on = |store: &str| {
        let status = command::on(
            store,
            &sending,
            &asked,
            &run,
            &scratch,
            ["ssh", "nix"],
            &cuts,
        );
        if status.is_some() {
            took = store
                .lines()
                .next()
                .and_then(|line| there::builder(line.trim()))
                .map(|lent| lent.host);
        }
        status
    };
    let mut status = changed::in_turn(lenders, std::process::id())
        .iter()
        .filter_map(|lender| standing(lender))
        .find_map(|store| on(&store));
    if status.is_none()
        && let Some(lender) = lenders.into_iter().find(|lender| there::lends(lender))
    {
        eprintln!(
            "nix-offload: no lent builder answered, so this waits for one to stand, for up \
             to {} minutes",
            waiting.longest.as_secs().div_ceil(60)
        );
        there::want(lender);
        let since = std::time::Instant::now();
        while status.is_none()
            && let Some(store) = there::stood_again(lender, waiting, since)
        {
            status = on(&store);
        }
    }
    if let Some(bundle) = &sending.bundle {
        archived::gone(bundle);
    }
    command::note(
        &state().join("nix-offload/runs.jsonl"),
        &command::Noted {
            at: now(),
            seconds: started.elapsed().as_secs(),
            folder: &folder,
            asked: &asked,
            on: took.as_deref(),
            status,
        },
    );
    // What ran at the commit sent, kept by nix-offload alone, so a gate
    // (the harness's host tests) can take a pass from a fenced session,
    // which can ask for a run but can't reach the keeper.
    if let Some(at) = &at {
        say_a_warm_pass_isnt_kept(status, &asked.words);
        passed::Passed::here(&state()).keep_ran(at, &asked.words, &asked.copies, status);
    }
    match status {
        Some(status) => ended::status(status, || {
            format!("{} failed on a lent builder", asked.words.join(" "))
        }),
        None => Err(not_run("no lent builder took it")),
    }
}

/// How a changed-checks run that went to no builder ends, for `why`: its
/// caller runs it here, unless it's kept to the lent builders, which
/// leaves nothing to run here.
fn unsent(why: &str) -> Failure {
    if there::lent_only() {
        return ended::backs_off(
            left_unbuilt(),
            format!("{why}, and --lent-only runs nothing here"),
        );
    }
    ended::backs_off(
        taken_nowhere(),
        format!("{why}, so changed-checks runs here"),
    )
}

/// How a changed-checks run ends that no builder or evaluator answered.
/// While a lender runs (`lending`), this machine is one that sends its
/// work away, and an evaluation here holds about 5 GB beside the
/// person's work: on 7 November a tunnel that died had every run
/// evaluate here for 8 minutes. So none runs here unasked.
fn none_answered(lending: bool) -> Failure {
    if lending {
        return ended::backs_off(
            left_unbuilt(),
            "no builder or evaluator answered, and a lender runs, so nothing was evaluated \
             here; changed-checks --here runs it on this machine",
        );
    }
    unsent("no builder or evaluator answered")
}

/// The table's exit for [`changed::NOT_TAKEN`], which a run nothing else
/// took ends with, so its caller runs it here.
fn taken_nowhere() -> Exit {
    Exit::of(changed::NOT_TAKEN).unwrap_or(Exit::TryAgain)
}

/// The table's exit for [`changed::UNBUILT`], which a run ends with that
/// left its checks unbuilt.
fn left_unbuilt() -> Exit {
    Exit::of(changed::UNBUILT).unwrap_or(Exit::Unavailable)
}

/// A run said as going and holding its work alone, both let go when this
/// is dropped (`running.rs`).
struct Going {
    _said: Option<running::Running>,
    _alone: Option<running::Held>,
}

/// What a run found of a twin doing its work in its tree.
enum Twinned {
    /// None, so it goes, said as going.
    Not(Going),
    /// One that has ended, so it looks again at what passed.
    Ended,
    /// One still going, which a run that `waits` for nothing leaves be.
    Busy,
}

/// This run, said as going from `started`, so a merge sharing its files
/// waits for it, and holding alone the work that the tree at `folder`'s
/// commit `at` and `arguments` name; or, once it waited for a twin doing
/// that work to end, or where it `waits` for nothing and one goes, that.
fn going_alone(
    at: Option<&passed::At>,
    folder: &Path,
    arguments: &[String],
    started: u64,
    waits: bool,
) -> Twinned {
    let Some(at) = at else {
        return Twinned::Not(Going {
            _said: None,
            _alone: None,
        });
    };
    let twin = running::Twin::new(&at.commit, folder, arguments);
    let me = std::process::id();
    let alone = match running::alone(&state(), me, &twin, now, waits) {
        running::Alone::Held(held) => Some(held),
        running::Alone::Waited => return Twinned::Ended,
        running::Alone::Busy => return Twinned::Busy,
        running::Alone::Unheld => None,
    };
    Twinned::Not(Going {
        _said: running::Running::start(&state(), me, &twin, started),
        _alone: alone,
    })
}

/// The checks a pass at `at` already told of for `system`, by their names
/// under it, so a run there need not build them again.
fn told(passed: &passed::Passed, at: Option<&passed::At>, system: Option<&str>) -> Vec<String> {
    let (Some(at), Some(system)) = (at, system) else {
        return Vec::new();
    };
    let under = format!("checks.{system}.");
    passed
        .checks(at)
        .iter()
        .filter_map(|check| check.strip_prefix(&under).map(str::to_owned))
        .collect()
}

/// `there --changed`: changed-checks run for the tree here on the lent
/// builder, else a warm VM builder, else an evaluator with its builds
/// sent on, else a builder waited for as `waiting` says (`changed.rs` says why). It exits with the
/// run's status, or `NOT_TAKEN` when nowhere else took it. With `replays`, a
/// change whose base no lent builder keeps evaluated runs replayed onto a
/// main one does (`replay.rs`), and again on its own base should that fail,
/// so a replay never fails a run that would have passed.
fn changed_there(
    arguments: &[String],
    waiting: there::Waiting,
    replays: bool,
) -> Result<(), Failure> {
    let dir = env::var_os("NIX_OFFLOAD_DIR").map_or_else(|| PathBuf::from(DIR), PathBuf::from);
    let scratch = scratch();
    let own = changed::run_by(there::lent_only()).map_err(|why| unsent(&why))?;
    let Ok(folder) = env::current_dir() else {
        return Err(unsent("this folder can't be read"));
    };
    let given = arguments;
    // A run naming its system is compared for that system alone.
    let named = changed::wanted(given).system.is_some();
    let arguments: &[String] = &with_system(arguments);
    let more_dir = more_dir();
    let source = there::source(&folder, &scratch, None, there::Sends::Changes)
        .map_err(|why| unsent(&why))?;
    let (source, replayed) = if replays {
        replay_kept(arguments, &folder, &scratch, source, [&dir, &more_dir])
    } else {
        (source, None)
    };
    let bundle = match &source {
        there::Source::Bundled { bundle, .. } => Some(bundle.clone()),
        there::Source::Pushed { .. } => None,
    };
    let forget = || {
        if let Some(bundle) = &bundle {
            let _ = fs::remove_file(bundle);
        }
    };
    // The same run at a commit where it passed passes again, so it's said
    // again rather than evaluated and built anew (`passed.rs`).
    let passed = passed::Passed::here(&state());
    let at = there::at_of(&source);
    if passed_already(&passed, at.as_ref(), arguments) {
        forget();
        return Ok(());
    }
    if let Some(held) = held_for_a_mac(arguments) {
        forget();
        return Err(held);
    }
    let wanted = changed::wanted(arguments);
    // The same run already going in this tree ends in what this one would
    // find, so this one waits for it and looks again at what passed; one
    // given `--never-ask` wants an answer at once, so it ends as one no
    // builder took, which its caller already backs off from.
    let started = now();
    let going = match going_alone(at.as_ref(), &folder, arguments, started, waiting.asks) {
        Twinned::Not(going) => going,
        Twinned::Ended => {
            forget();
            return changed_there(given, waiting, replays);
        }
        // Said as it was found busy (running.rs), so the kit adds nothing.
        Twinned::Busy => {
            forget();
            return Err(Failure::new(
                taken_nowhere(),
                "the same run goes in this tree already, and --never-ask waits for none",
            )
            .said_already());
        }
    };
    let told = told(&passed, at.as_ref(), wanted.system.as_deref());
    let vm_dir =
        env::var_os("NIX_OFFLOAD_VM_DIR").map_or_else(|| PathBuf::from(VM_DIR), PathBuf::from);
    // Decided only while the VM lender ran, a run sent as a switch
    // restarted it ran whole on the lent builder, VM tests and all.
    let vm_apart = vm_apart(&vm_patterns(), &vm_dir);
    let sent = Sent {
        source: &source,
        told: &told,
        folder: &folder,
        scratch: &scratch,
        waiting,
        moves: false,
        own,
    };
    let finished = sent.for_its_system(arguments, [&dir, &more_dir], &vm_dirs(&vm_dir), vm_apart);
    forget();
    let Some(finished) = finished else {
        return Err(none_answered(there::lends(&dir) || there::lends(&more_dir)));
    };
    if let Err(code) = kept_after_a_mac(
        &passed,
        at.as_ref(),
        arguments,
        (!named).then_some(folder.as_path()),
        &finished,
        started,
        waiting,
    ) {
        return ended::status(code, || {
            "a Mac's checks didn't pass, so this run's pass isn't kept".to_owned()
        });
    }
    if runs_again(replayed.as_deref(), finished.status) {
        // The replay ended, so it goes unsaid and lets its work go first.
        drop(going);
        return changed_there(given, waiting, false);
    }
    ended::status(finished.status, || {
        "the changed checks didn't pass, as the lines above say".to_owned()
    })
}

/// `arguments`, naming this machine's system when they name none: given
/// none, the far side would name its own, and the VM tests sent apart need
/// one. It's named before the run is looked for, so a run kept is found
/// either way.
fn with_system(arguments: &[String]) -> Vec<String> {
    let mut arguments = arguments.to_vec();
    if changed::wanted(&arguments).system.is_none() {
        arguments.extend(["--system".to_owned(), this_system()]);
    }
    arguments
}

/// Set on a gate's own Mac comparison, whose run asked the Mac for room
/// already, so it isn't asked twice.
const ROOM_ASKED: (&str, &str) = ("NIX_OFFLOAD_MAC_ROOM_ASKED", "1");

/// How a changed-checks run with `arguments` ends when it waits rather
/// than go to a busy Mac nearby, as [`changed::mac::room_alone`] decides from
/// the first that answers, or none when it goes. It ends as one nothing
/// built, which its callers try again later and never run here, where it
/// would build nothing.
fn held_for_a_mac(arguments: &[String]) -> Option<Failure> {
    use changed::mac;
    let wanted = changed::wanted(arguments);
    let asked_already = env::var_os(ROOM_ASKED.0).is_some_and(|value| value == ROOM_ASKED.1);
    // `--lent-only` keeps the person's Mac out of it altogether, and the
    // keeper's `--keep-base` run only evaluates, on a lent builder.
    let keeps = arguments.iter().any(|word| word == "--keep-base");
    if asked_already
        || keeps
        || there::lent_only()
        || !mac::asks_for_room(wanted.system.as_deref(), &this_system(), wanted.build)
    {
        return None;
    }
    let mac = nearby_listed()
        .into_iter()
        .filter(|machine| machine.systems.iter().any(|system| system == mac::SYSTEM))
        .find(|machine| machines::nearby_answers(&machine.address, &machine.host))?;
    // A run that goes says the load it went at, so a busy Mac that was
    // sent one can be told from the gate that let it through.
    match mac::room_alone(&mac.host, there::load_of(&mac, &scratch(), "ssh")) {
        Ok(room) => {
            eprintln!("nix-offload: {room}, so a run for its system goes");
            None
        }
        Err(why) => Some(ended::backs_off(
            left_unbuilt(),
            format!("{why}, so a run for its system waits, ended as one nothing built"),
        )),
    }
}

/// How a Mac's comparison joined a changed-checks run that passed.
enum OnAMac {
    /// None was asked: no Mac is nearby, the run doesn't build for
    /// another system, or the change touches nothing a Mac's checks read.
    Unasked,
    /// It passed.
    Passed,
    /// It was skipped, for the reason given.
    Skipped(String),
    /// It failed, and the run ends with its status.
    Failed(i32),
}

/// Whether a changed-checks run with `arguments` passed at `at` already,
/// saying what it printed then, and so.
fn passed_already(passed: &passed::Passed, at: Option<&passed::At>, arguments: &[String]) -> bool {
    let Some(at) = at else {
        return false;
    };
    let Some(printed) = passed.run(at, arguments) else {
        return false;
    };
    print!("{printed}");
    eprintln!(
        "nix-offload: changed-checks {} passed at {} already, so nothing is evaluated \
         or built again; `nix-offload passed forget {}` has it run again",
        arguments.join(" "),
        at.short(),
        at.commit
    );
    true
}

/// Keeps that a changed-checks run with `arguments`, started at `started`
/// and ended as `finished`, passed at `at`, once a Mac's comparison joining
/// it has passed or is kept as skipped beside it (`changed/mac.rs`). A
/// Mac's failure is the run's, and nothing is kept. With no `folder`, as
/// for a run that named its system, no Mac is asked.
fn kept_after_a_mac(
    passed: &passed::Passed,
    at: Option<&passed::At>,
    arguments: &[String],
    folder: Option<&Path>,
    finished: &Finished,
    started: u64,
    waiting: there::Waiting,
) -> Result<(), i32> {
    if finished.status != 0 {
        return Ok(());
    }
    let mac = folder.map_or(OnAMac::Unasked, |folder| {
        on_a_mac(arguments, folder, waiting)
    });
    if let OnAMac::Failed(code) = mac {
        return Err(code);
    }
    if let (Some(at), Some(printed)) = (at, &finished.printed) {
        kept_passing(passed, at, arguments, printed, started);
        if let OnAMac::Skipped(because) = &mac {
            passed.keep_skipped(at, &changed::mac::for_a_mac(arguments), because);
        }
    }
    Ok(())
}

/// A Mac's comparison of the change in `folder`, joining a run with
/// `arguments` that passed, when a Mac nearby builds for its system and
/// the change touches a file its checks read (`changed/mac.rs`). It goes
/// as any run for a Mac's system goes, named on a lent builder and built
/// on the Mac, in a run of its own, ended past [`changed::mac::longest`].
fn on_a_mac(arguments: &[String], folder: &Path, waiting: there::Waiting) -> OnAMac {
    use changed::mac;
    let macs: Vec<there::Nearby> = nearby_listed()
        .into_iter()
        .filter(|machine| machine.systems.iter().any(|system| system == mac::SYSTEM))
        .collect();
    if macs.is_empty() || !mac::joins(arguments) || this_system() == mac::SYSTEM {
        return OnAMac::Unasked;
    }
    let Some(base) = changed::base_of(arguments, folder, "HEAD") else {
        return OnAMac::Unasked;
    };
    let touched = mac::touched(folder, &base);
    let kept = || {
        let dir = env::var_os("NIX_OFFLOAD_DIR").map_or_else(|| PathBuf::from(DIR), PathBuf::from);
        kept_listed([&dir, &more_dir()], &scratch())
            .iter()
            .any(|listed| !replay::kept_in(listed, mac::SYSTEM).is_empty())
    };
    let Some(read) = mac::read_among(folder, &touched, kept) else {
        return OnAMac::Unasked;
    };
    let skipped = |because: String| {
        eprintln!(
            "nix-offload: {read} is read by a Mac's checks, and {because}, so their \
             comparison is kept as skipped, for the net to run once it may"
        );
        OnAMac::Skipped(because)
    };
    if there::lent_only() {
        return skipped("--lent-only keeps the change off a Mac nearby".to_owned());
    }
    let answering =
        |machine: &&there::Nearby| machines::nearby_answers(&machine.address, &machine.host);
    let Some(mac) = macs.iter().find(answering) else {
        let names: Vec<&str> = macs.iter().map(|machine| machine.host.as_str()).collect();
        return skipped(format!("{} nearby doesn't answer", names.join(", ")));
    };
    // A Mac with no room, or building another run already, waits rather
    // than take more: its comparison is owed, and the net runs it once
    // the Mac has room.
    // One that says nothing a load reads from has none either, as a run
    // for a Mac's system takes it (`held_for_a_mac`).
    let room = match mac::room_alone(&mac.host, there::load_of(mac, &scratch(), "ssh")) {
        Ok(room) => room,
        Err(why) => return skipped(why),
    };
    eprintln!(
        "nix-offload: {read} is read by a Mac's checks, so they're compared there too: {room}"
    );
    let longest = mac::longest();
    let compared = compared_within(&mac::for_a_mac(arguments), waiting, longest);
    // A comparison that didn't pass, or never ended, may be the Mac going,
    // which what ssh said of it within the last minute can't know, so it's
    // asked afresh.
    if compared != Some(0) {
        machines::forget_reached(&mac.host);
    }
    match compared {
        Some(0) => OnAMac::Passed,
        None => skipped(format!(
            "{}'s comparison ran past {}",
            mac.host,
            mac::said(longest)
        )),
        // Ending so would tell this run's caller nothing took it, and to
        // run it all on this machine.
        Some(status)
            if status == i32::from(changed::NOT_TAKEN) || status == i32::from(changed::UNBUILT) =>
        {
            skipped(format!("no builder took {}'s comparison", mac.host))
        }
        // A Mac that went as it compared left its build to a builder that
        // can't build for it, so the failure says nothing of the change.
        Some(_) if !answering(&mac) => {
            skipped(format!("{} stopped answering as it compared", mac.host))
        }
        Some(status) => {
            eprintln!("nix-offload: a Mac's checks didn't pass, so this run's pass isn't kept");
            OnAMac::Failed(status)
        }
    }
}

/// How a changed-checks run with `arguments`, in a run of this program of
/// its own, ended, or none when it ran past `longest` and was ended.
fn compared_within(
    arguments: &[String],
    waiting: there::Waiting,
    longest: Duration,
) -> Option<i32> {
    let mut comparing = again();
    comparing.arg("there").env(ROOM_ASKED.0, ROOM_ASKED.1);
    if !waiting.asks {
        comparing.arg("--never-ask");
    }
    let spawned = comparing
        .arg("--changed")
        .args(arguments)
        .stdin(Stdio::null())
        // What this run names is its own run's, so the Mac's is only said.
        .stdout(Stdio::from(std::io::stderr()))
        .process_group(0)
        .spawn();
    let mut comparing = match spawned {
        Ok(comparing) => comparing,
        Err(error) => {
            eprintln!("nix-offload: couldn't start a Mac's comparison: {error}");
            return Some(1);
        }
    };
    let began = Instant::now();
    loop {
        match comparing.try_wait() {
            Ok(Some(ended)) => return Some(ended.code().unwrap_or(1)),
            Ok(None) if began.elapsed() < longest => {
                // Asked each second: a comparison takes minutes, so a
                // second more past its end is nothing.
                thread::sleep(Duration::from_secs(1));
            }
            _ => {
                let group = i32::try_from(comparing.id()).unwrap_or(i32::MAX);
                // SAFETY: killpg only signals the group the comparison leads.
                unsafe { libc::killpg(group, libc::SIGKILL) };
                let _ = comparing.wait();
                return None;
            }
        }
    }
}

/// Whether a run that ended with `status`, of the change `replayed` as
/// said, runs again on the change's own base: when it failed, other than
/// by finding the builders busy, since a replay mustn't fail a run that
/// would have passed. It says which, either way.
fn runs_again(replayed: Option<&str>, status: i32) -> bool {
    let Some(replayed) = replayed else {
        return false;
    };
    if status != 0 && status != there::BUSY {
        eprintln!("nix-offload: {replayed} failed, so the change runs again on its own base");
        return true;
    }
    eprintln!("nix-offload: what passed is {replayed}, and its pass is kept under that");
    false
}

/// What each lent builder of `lenders` that answers lists of the checks
/// it keeps evaluated (`replay::LISTING`), asked of them all at once.
fn kept_listed(lenders: [&Path; 2], scratch: &Path) -> Vec<String> {
    let lent: Vec<there::Builder> = lenders
        .into_iter()
        .filter_map(standing)
        .filter_map(|store| there::builder(store.lines().next()?.trim()))
        .collect();
    let listing = there::Sending {
        command: replay::LISTING.to_owned(),
        bundle: None,
    };
    thread::scope(|scope| {
        let asked: Vec<_> = lent
            .iter()
            .map(|builder| scope.spawn(|| there::printed_on(builder, &listing, scratch, "ssh")))
            .collect();
        asked
            .into_iter()
            .filter_map(|asked| match asked.join() {
                Ok(Ok((ran, listed))) if ran.status == 0 => Some(listed),
                _ => None,
            })
            .collect()
    })
}

/// `source`, or, when no lent builder keeps its base's checks evaluated,
/// the same change replayed onto the newest main one does (`replay.rs`),
/// with what it was replayed as. Anything that keeps it from being
/// replayed is said, and the run goes as it was.
fn replay_kept(
    arguments: &[String],
    folder: &Path,
    scratch: &Path,
    source: there::Source,
    lenders: [&Path; 2],
) -> (there::Source, Option<String>) {
    let (there::Source::Pushed { commit, .. } | there::Source::Bundled { commit, .. }) = &source;
    let (Some(base), Some(system)) = (
        changed::base_of(arguments, folder, commit),
        changed::wanted(arguments).system,
    ) else {
        return (source, None);
    };
    let listed = kept_listed(lenders, scratch);
    // With no lent builder answering, the run finds its way as it would have.
    if listed.is_empty() {
        return (source, None);
    }
    let kept: std::collections::HashSet<String> = listed
        .iter()
        .flat_map(|listed| replay::kept_in(listed, &system))
        .collect();
    let short = |commit: &str| commit.get(..12).unwrap_or(commit).to_owned();
    let not_kept = format!("no lent builder keeps {}'s checks evaluated", short(&base));
    let theirs = changed::theirs(arguments);
    let replayed = match replay::replayed(folder, &base, commit, theirs, &kept, scratch) {
        Ok(None) => return (source, None),
        Ok(Some(replayed)) => replayed,
        Err(why) => {
            eprintln!(
                "nix-offload: {not_kept}, and the change can't go onto a main one does: {why}"
            );
            return (source, None);
        }
    };
    match there::source(
        folder,
        scratch,
        Some(&replayed.commit),
        there::Sends::Changes,
    ) {
        Ok(sent) => {
            let said = format!(
                "{}'s change replayed as {} onto {}",
                short(commit),
                short(&replayed.commit),
                short(&replayed.onto)
            );
            // A bundle of the replay took the original's place, as each
            // run bundles into one file of its own.
            eprintln!("nix-offload: {not_kept}, so this checks {said}, which one keeps");
            (sent, Some(said))
        }
        Err(why) => {
            eprintln!("nix-offload: {not_kept}, and the change replayed couldn't be sent: {why}");
            // Its bundle may have gone over the original's, so that's made
            // again, and the run goes as it would have.
            let commit = commit.clone();
            let again = there::source(folder, scratch, Some(&commit), there::Sends::Changes);
            (again.unwrap_or(source), None)
        }
    }
}

/// Keeps that the changed-checks run with `arguments`, started at
/// `started`, passed at `at`, having printed `printed`, and, where it
/// built, that each check it named passed there. What it named is the
/// far side's to say; the commit and the run are what this one sent.
fn kept_passing(
    passed: &passed::Passed,
    at: &passed::At,
    arguments: &[String],
    printed: &str,
    started: u64,
) {
    passed.keep_run(at, arguments, printed, started);
    let wanted = changed::wanted(arguments);
    if let (true, Some(system)) = (wanted.build, &wanted.system) {
        let built: Vec<String> = changed::to_build(printed, wanted.vm)
            .iter()
            .map(|name| format!("checks.{system}.{name}"))
            .collect();
        passed.keep_checks(at, &built);
    }
}

/// How a `there --changed` run ended: its status, and what changed-checks
/// printed, where it was read.
struct Finished {
    status: i32,
    printed: Option<String>,
}

impl Finished {
    fn of(status: i32, printed: Option<String>) -> Self {
        Self { status, printed }
    }
}

/// What a `there --changed` run sends wherever it goes: the tree's
/// source, the checks `told` passed at its commit already, the tree's
/// folder here, the scratch folder its files go in meanwhile, and how it
/// waits for a builder when none stands.
#[derive(Clone, Copy)]
struct Sent<'a> {
    source: &'a there::Source,
    told: &'a [String],
    folder: &'a Path,
    scratch: &'a Path,
    waiting: there::Waiting,
    /// Whether it may go to the other lent builder, rather than wait.
    moves: bool,
    /// The changed-checks it runs by store path, sent up first, rather
    /// than the commit's own.
    own: Option<&'a str>,
}

impl Sent<'_> {
    /// The command changed-checks is sent as with `arguments`.
    fn sending(&self, arguments: &[String]) -> there::Sending {
        let mut sending = changed::sending(self.source.clone(), arguments, self.told, self.own);
        if self.moves {
            sending.command.insert_str(0, there::MOVES);
        }
        sending
    }

    /// The run on the lent builder a store names, or none when there's no
    /// store or ssh couldn't reach it. With `vm_apart`, VM tests go to a
    /// builder of their own. A store `on_vm`, a warm VM builder's, takes
    /// no turn of the lent builders' (`changed/turns.rs`).
    fn lent_to(
        &self,
        arguments: &[String],
        vm_apart: bool,
        store: Option<String>,
        on_vm: bool,
    ) -> Option<Finished> {
        let wanted = changed::wanted(arguments);
        let lent = store.and_then(|store| there::builder(store.lines().next()?.trim()))?;
        // A turn at sending it there, held until the run ends
        // (`changed/turns.rs`); a run that may move is sent on from a full
        // builder to the next. They're kept in the account's own folder,
        // as the uplink's are, so no other account's can hold them; with
        // none, which is said, the run goes on without one.
        let turn = there::turns_folder()
            .map(|turns| changed::turns::sending(&turns, &lent.host, self.moves, !on_vm));
        let _turn = match turn {
            Some(changed::turns::Turn::Full) => return Some(Finished::of(there::BUSY, None)),
            Some(changed::turns::Turn::Had(held)) => held,
            None => Vec::new(),
        };
        // Sent as `--run`'s `--copy` paths are, as held outputs go, since
        // the builder trusts no account.
        if let Some(own) = self.own {
            let package = [changed::package_of(own)];
            if let Err(why) = there::paths_up(
                &package,
                &lent,
                "nix",
                there::send_within(0),
                &there::Cuts::default(),
            ) {
                eprintln!("nix-offload: {why}");
                return None;
            }
        }
        // What changed-checks keeps of the run there comes home after it
        // (`changed/runs.rs`).
        let before = changed::runs::length_on(&lent, self.scratch);
        let finished = if let (true, true, Some(system)) = (vm_apart, wanted.vm, &wanted.system) {
            let apart = Apart {
                lent: &lent,
                sent: self,
                system,
            };
            apart.changed(arguments)
        } else {
            // What it names is read as it's passed on, so a run that
            // passed is kept with it.
            let sending = self.sending(arguments);
            match there::teed_on(&lent, &sending, self.scratch, "ssh") {
                Ok((ran, printed)) if ran.status != there::UNREACHED => {
                    Some(Finished::of(ran.status, Some(printed)))
                }
                _ => {
                    eprintln!("nix-offload: ssh couldn't reach {}", lent.host);
                    None
                }
            }
        };
        // A run turned away never reached changed-checks.
        if finished
            .as_ref()
            .is_some_and(|finished| finished.status != there::BUSY)
        {
            let home = state().join("nix-offload/changed-runs.jsonl");
            changed::runs::bring(&lent, self.scratch, before, &home);
        }
        finished
    }

    /// The two lent builders' stores, `lent`, with the one that keeps the
    /// run's base evaluated first ([`changed::kept_first`]), both asked at
    /// once, since each ssh took about 1.6 s on 12 November. They stay as
    /// they came unless both stand and the base and system are known.
    fn kept_first(&self, arguments: &[String], lent: [Option<String>; 2]) -> [Option<String>; 2] {
        let (there::Source::Pushed { commit, .. } | there::Source::Bundled { commit, .. }) =
            self.source;
        let base = changed::base_of(arguments, self.folder, commit);
        let asking = changed::wanted(arguments)
            .system
            .zip(base.as_deref())
            .filter(|_| lent.iter().all(Option::is_some))
            .and_then(|(system, base)| changed::keeps_base(base, &system));
        let Some(asking) = asking else {
            return lent;
        };
        let keeps = |store: &Option<String>| {
            let Some(builder) = store
                .as_deref()
                .and_then(|store| there::builder(store.lines().next()?.trim()))
            else {
                return false;
            };
            let sending = there::Sending {
                command: asking.clone(),
                bundle: None,
            };
            there::printed_on(&builder, &sending, self.scratch, "ssh")
                .is_ok_and(|(ran, _)| ran.status == 0)
        };
        let kept = thread::scope(|scope| {
            let first = scope.spawn(|| keeps(&lent[0]));
            let second = keeps(&lent[1]);
            [first.join().unwrap_or(false), second]
        });
        if !kept[0] && kept[1] {
            let short = base
                .as_deref()
                .and_then(|base| base.get(..12))
                .unwrap_or_default();
            eprintln!(
                "nix-offload: the other lent builder keeps {short}'s checks evaluated, so this \
                 run asks it first"
            );
        }
        changed::kept_first(lent, kept)
    }

    /// The run on a lent builder, the one that keeps its base evaluated
    /// first, else the two taken in turn, then on a warm VM builder of
    /// `vm`'s, or none when none stood or ssh reached none.
    /// While more than one stands, each is asked to turn it away rather
    /// than make it wait, so it goes to the next; when all do, it waits at
    /// the first it asked. On 10 November an evaluation took 4.7 minutes
    /// with three or fewer on a lent builder and 7.5 with seven or more,
    /// while warm VM builders, whose gate keeps the same turns, stood idle.
    fn lent_somewhere(
        &self,
        arguments: &[String],
        lenders: [&Path; 2],
        vm: &[PathBuf],
        vm_apart: bool,
    ) -> Option<Finished> {
        let lent = changed::in_turn(lenders, std::process::id()).map(standing);
        let lent_builders = lent.len();
        let stores: Vec<Option<String>> = self
            .kept_first(arguments, lent)
            .into_iter()
            .chain(
                // A lane kept for the runs naming it takes no whole run.
                vm.iter()
                    .filter(|builder| there::warm(builder) && !there::named_only(builder))
                    .map(|builder| standing(builder)),
            )
            .collect();
        let moving = Self {
            moves: stores.iter().filter(|store| store.is_some()).count() > 1,
            ..*self
        };
        // The two lent builders come first, then the warm VM builders.
        let on_vm = |index: usize| index >= lent_builders;
        let mut busy = None;
        for (index, store) in stores.iter().enumerate() {
            match moving.lent_to(arguments, vm_apart, store.clone(), on_vm(index)) {
                Some(finished) if moving.moves && finished.status == there::BUSY => {
                    busy = busy.or_else(|| Some((index, store.clone()?)));
                }
                Some(finished) => {
                    self.share_kept(arguments, store.as_deref(), &stores);
                    return Some(finished);
                }
                None => {}
            }
        }
        let (index, store) = busy?;
        let finished = self.lent_to(arguments, vm_apart, Some(store.clone()), on_vm(index));
        if finished.is_some() {
            self.share_kept(arguments, Some(&store), &stores);
        }
        finished
    }

    /// Copy what the run's base keeps evaluated on `ran`, the lent builder
    /// it ran on, to the other lent builder of the first two `stores`, so
    /// a run sent there next evaluates its own tree alone
    /// ([`changed::sharing_kept`]): its checks, then each other record
    /// kept, which is no use without them. A copy that fails costs the
    /// other an evaluation and nothing more, so it's said and the run goes
    /// on.
    fn share_kept(&self, arguments: &[String], ran: Option<&str>, stores: &[Option<String>]) {
        let lent = |store: Option<&str>| {
            store.and_then(|store| there::builder(store.lines().next()?.trim()))
        };
        let other = match stores {
            [first, second, ..] if first.as_deref() == ran => second.as_deref(),
            [first, second, ..] if second.as_deref() == ran => first.as_deref(),
            _ => return,
        };
        let (Some(from), Some(to)) = (lent(ran), lent(other)) else {
            return;
        };
        let (there::Source::Pushed { commit, .. } | there::Source::Bundled { commit, .. }) =
            self.source;
        let (Some(base), Some(system)) = (
            changed::base_of(arguments, self.folder, commit),
            changed::wanted(arguments).system,
        ) else {
            return;
        };
        let [checks, rest @ ..] = changed::KEPT_AS;
        if !self.copy_kept(&from, &to, &base, &system, checks) {
            return;
        }
        for record in rest {
            self.copy_kept(&from, &to, &base, &system, record);
        }
    }

    /// Copy one `record` of what `base` keeps for `system` from `from` to
    /// `to`, saying so when it fails, and whether `from` kept it to copy.
    fn copy_kept(
        &self,
        from: &there::Builder,
        to: &there::Builder,
        base: &str,
        system: &str,
        record: &str,
    ) -> bool {
        let Some((read, take)) = changed::sharing_kept(base, system, record) else {
            return false;
        };
        let short = base.get(..12).unwrap_or(base);
        let reading = there::Sending {
            command: read,
            bundle: None,
        };
        // A run that ended before its base was evaluated keeps nothing.
        let kept = match there::printed_on(from, &reading, self.scratch, "ssh") {
            Ok((ran, kept)) if ran.status == 0 && !kept.is_empty() => kept,
            _ => return false,
        };
        let copy = self
            .scratch
            .join(format!("kept-{}-{short}.{record}", std::process::id()));
        if let Err(error) = fs::write(&copy, kept) {
            eprintln!("nix-offload: couldn't hold {short}'s kept {record} to copy: {error}");
            return true;
        }
        let taking = there::Sending {
            command: take,
            bundle: Some(copy.clone()),
        };
        match there::printed_on(to, &taking, self.scratch, "ssh") {
            Ok((ran, _)) if ran.status == 0 => {}
            Ok((ran, _)) => eprintln!(
                "nix-offload: {} didn't take {short}'s kept {record}: it ended {}",
                to.host, ran.status
            ),
            Err(why) => eprintln!("nix-offload: couldn't copy {short}'s kept {record}: {why}"),
        }
        let _ = fs::remove_file(&copy);
        true
    }

    /// Where `there --changed` runs, in its order, or none when nowhere
    /// did: a lent builder, else a warm VM builder of `vm`'s. With
    /// `vm_apart`, VM tests go to a builder of their own, as any `there`
    /// sends them.
    fn somewhere(
        &self,
        arguments: &[String],
        [dir, more]: [&Path; 2],
        vm: &[PathBuf],
        vm_apart: bool,
    ) -> Option<Finished> {
        let Self {
            source, scratch, ..
        } = *self;
        let wanted = changed::wanted(arguments);
        let on_builder = |store: Option<String>| self.lent_to(arguments, vm_apart, store, false);
        // The second lent builder stands in for the first, so one whose
        // tunnel died never leaves the run to an evaluator or this machine
        // while the other stands. Runs take the two in turn, and each asks
        // the second's lender for its builder while it lives, so a run's
        // evaluation never waits on the first for a turn while the second
        // could have given it one.
        if there::lends(more) {
            there::want(more);
        }
        // A warm VM builder that stands takes the run while no lent
        // builder answers, rather than its waiting minutes for one: from
        // 01:00 to 08:05 on 8 November the lent builders ran out of
        // memory and went silent, and every run waited and ended 69, while
        // the VM builders took every VM test sent them by name.
        if let Some(finished) = self.lent_somewhere(arguments, [dir, more], vm, vm_apart) {
            return Some(finished);
        }
        // While a lender runs, a run its builders didn't answer waits for
        // one to stand again, for up to `waiting.longest`, rather than going
        // to an evaluator: on 7 November the evaluator, 8 cores, held 56
        // evaluations from runs whose lent builder's tunnel had dropped,
        // and printed nothing for half an hour.
        if let Some(lender) = [dir, more].into_iter().find(|lender| there::lends(lender)) {
            eprintln!(
                "nix-offload: no lent builder answered, so this waits for one to stand again, \
                 for up to {} minutes",
                self.waiting.longest.as_secs().div_ceil(60)
            );
            let since = std::time::Instant::now();
            while let Some(store) = there::stood_again(lender, self.waiting, since) {
                let again = on_builder(Some(store)).or_else(|| on_builder(standing(more)));
                if again.is_some() {
                    return again;
                }
            }
            return on_builder(standing(more));
        }
        // An evaluator only names; what it names is built as any `there`
        // build is, since a machine that evaluates well may build slowly.
        let mut evaluating: Vec<String> = arguments
            .iter()
            .filter(|word| *word != "--build" && *word != "--name-only")
            .cloned()
            .collect();
        evaluating.push("--name-only".to_owned());
        // A changed-checks from before --name-only refuses it, as usage,
        // and without --build it only names anyway.
        let older = &evaluating[..evaluating.len() - 1];
        // An evaluator is one of the person's machines, evaluating as them.
        let evaluators = fs::read_to_string(config().join("nix-offload/evaluators"))
            .ok()
            .filter(|_| !there::lent_only())
            .map(|text| changed::evaluators(&text))
            .unwrap_or_default();
        let github = match source {
            there::Source::Pushed { github, .. } | there::Source::Bundled { github, .. } => github,
        };
        for evaluator in evaluators.iter().filter(|evaluator| {
            evaluator.takes(github) && machines::nearby_answers(&evaluator.address, &evaluator.host)
        }) {
            eprintln!("nix-offload: evaluating on {}", evaluator.host);
            let ask = |words: &[String]| {
                changed::evaluated(
                    "ssh",
                    &evaluator.host,
                    &self.sending(words),
                    scratch,
                    changed::evaluating_longest(),
                )
            };
            // Held only while it evaluates; what it names builds elsewhere.
            let turn = changed::turn(scratch, &evaluator.host);
            let asked = match ask(&evaluating) {
                Ok((USAGE, _)) => ask(older),
                asked => asked,
            };
            drop(turn);
            let (status, printed) = match asked {
                Ok((status, _)) if status == there::UNREACHED => continue,
                Ok(evaluated) => evaluated,
                Err(why) => {
                    eprintln!("nix-offload: {why}");
                    continue;
                }
            };
            return Some(self.built_named(&wanted, status, printed));
        }
        // Asked for and waited on, as any `there` waits, while a lender
        // runs, then the second once more, which may have stood meanwhile.
        on_builder(there::waited(dir, self.waiting)).or_else(|| on_builder(standing(more)))
    }

    /// `somewhere`, but a run that builds for a system no lent builder
    /// builds for and a machine nearby does only names what changed
    /// there, and what it names builds as any `there` build does, on that
    /// machine. The lent builders build for Intel Linux alone, so a
    /// changed run for a Mac's system sent whole to one could only fail,
    /// while a Mac nearby, which builds for it, stood idle.
    fn for_its_system(
        &self,
        arguments: &[String],
        lenders: [&Path; 2],
        vm: &[PathBuf],
        vm_apart: bool,
    ) -> Option<Finished> {
        let wanted = changed::wanted(arguments);
        let Some(system) = wanted
            .system
            .as_deref()
            .filter(|system| wanted.build && built_only_nearby(system, lenders[0]))
        else {
            return self.somewhere(arguments, lenders, vm, vm_apart);
        };
        // A keep evaluates its base's checks whole there, so it says so
        // rather than that only a change is named.
        let there = if arguments.iter().any(|word| word == "--keep-base") {
            "keeps its checks evaluated on the lent builder"
        } else {
            "only names what changed"
        };
        eprintln!(
            "nix-offload: no lent builder builds for {system} and a machine nearby does, so \
             changed-checks {there}, and what it names builds nearby"
        );
        let mut naming: Vec<String> = arguments
            .iter()
            .filter(|word| *word != "--build")
            .cloned()
            .collect();
        naming.push("--name-only".to_owned());
        let named = self.somewhere(&naming, lenders, vm, false)?;
        let printed = named.printed.unwrap_or_default();
        Some(self.built_named(&wanted, named.status, printed))
    }

    /// What a run that only named, ending `status` with `printed`, named,
    /// built as `wanted` asks, as any `there` build is.
    fn built_named(&self, wanted: &changed::Wanted, status: i32, printed: String) -> Finished {
        let folder = self.folder;
        let targets = changed::to_build(&printed, wanted.vm);
        if status != 0 || !wanted.build || targets.is_empty() {
            return Finished::of(status, Some(printed));
        }
        let Some(system) = &wanted.system else {
            eprintln!("nix-offload: changed-checks names its system when it builds elsewhere");
            return Finished::of(USAGE, None);
        };
        // What main holds in quarantine is built alone after the rest,
        // as changed-checks builds it, so its failure blocks nothing
        // but in a subtest its entry doesn't list.
        let (blocking, held) = changed::apart(targets, &changed::held_on_main(folder));
        let status = if blocking.is_empty() {
            0
        } else {
            built_evaluated(folder, system, &blocking)
        };
        let alone = changed::built_alone(&held, &mut |names| built_heard(folder, system, names));
        let status = if status == 0 { alone } else { status };
        Finished::of(status, Some(printed))
    }
}

/// Whether no builder of `dir`'s lender builds for `system` and a machine
/// nearby does. Never while what the lent builders build for isn't known.
fn built_only_nearby(system: &str, dir: &Path) -> bool {
    let lent = there::lent_systems(dir);
    !lent.is_empty()
        && !lent.iter().any(|lent| lent == system)
        && nearby()
            .iter()
            .any(|machine| machine.systems.iter().any(|built| built == system))
}

/// A changed-checks run on the lent builder whose VM tests build apart. Run
/// whole there, they shared it with big builds: nine at a load of 100 on
/// its 32 cores on 2 November, with a builder for VM tests standing apart.
struct Apart<'a> {
    lent: &'a there::Builder,
    sent: &'a Sent<'a>,
    system: &'a str,
}

impl Apart<'_> {
    /// The builder names what changed, then builds all but the VM tests,
    /// evaluating nothing again, since changed-checks keeps a commit's
    /// derivations; meanwhile the VM tests it named go as any `there`
    /// build goes, to their own builder. None when ssh couldn't reach it.
    fn changed(&self, arguments: &[String]) -> Option<Finished> {
        let mut naming: Vec<String> = arguments
            .iter()
            .filter(|word| *word != "--build")
            .cloned()
            .collect();
        naming.push("--name-only".to_owned());
        let sending = self.sent.sending(&naming);
        let scratch = self.sent.scratch;
        let (named, printed) = match there::printed_on(self.lent, &sending, scratch, "ssh") {
            Ok((named, printed)) if named.status != there::UNREACHED => (named, printed),
            _ => {
                eprintln!("nix-offload: ssh couldn't reach {}", self.lent.host);
                return None;
            }
        };
        match named.status {
            0 => {}
            // A commit's changed-checks from before --name-only refuses it,
            // so it can't say which are VM tests: it builds all but those
            // there, and fails for want of them, rather than build them
            // beside big builds.
            USAGE => {
                let mut rest = arguments.to_vec();
                rest.push("--no-vm".to_owned());
                let built = self.run(&rest);
                if built != 0 {
                    return Some(Finished::of(built, None));
                }
                eprintln!(
                    "nix-offload: this commit's changed-checks can't name its checks alone, so \
                     its VM tests weren't built; build them with nix-offload there"
                );
                return Some(Finished::of(i32::from(changed::UNBUILT), None));
            }
            status => {
                print!("{printed}");
                return Some(Finished::of(status, Some(printed)));
            }
        }
        let vm: Vec<String> = changed::to_build(&printed, true)
            .into_iter()
            .filter(|name| name.starts_with("vm-"))
            .collect();
        // The VM tests main holds in quarantine go alone once the rest are
        // done, as changed-checks builds them, so their failure blocks
        // nothing but in a subtest their entry doesn't list; the lent
        // builder's own run splits those it builds.
        let (vm, held) = changed::apart(vm, &changed::held_on_main(self.sent.folder));
        let testing = if vm.is_empty() {
            None
        } else {
            match changed::building(self.sent.folder, self.system, &vm).spawn() {
                Ok(testing) => Some(testing),
                Err(error) => {
                    eprintln!("nix-offload: couldn't send the VM tests on: {error}");
                    return Some(Finished::of(1, None));
                }
            }
        };
        let mut rest = arguments.to_vec();
        rest.push("--no-vm".to_owned());
        let built = self.run(&rest);
        let vm_tested = testing.map(|testing| tested(testing, vm.len()));
        let alone = changed::built_alone(&held, &mut |names| {
            let building =
                changed::hearing(&mut changed::building(self.sent.folder, self.system, names));
            building.map_or_else(
                |error| {
                    eprintln!("nix-offload: couldn't send {} on: {error}", names.join(" "));
                    (1, BTreeSet::new())
                },
                |(testing, failed)| (tested(testing, names.len()), failed),
            )
        });
        // The first that failed says how the whole did.
        let status = [built, vm_tested.unwrap_or(0), alone]
            .into_iter()
            .find(|status| *status != 0)
            .unwrap_or(0);
        Some(Finished::of(status, Some(printed)))
    }

    /// changed-checks run on the lent builder with `arguments`, its status.
    fn run(&self, arguments: &[String]) -> i32 {
        let sending = self.sent.sending(arguments);
        let bring = there::Bring::Nothing;
        match there::on(self.lent, &sending, bring, self.sent.scratch, "ssh") {
            Ok(ran) => ran.status,
            Err(why) => {
                eprintln!("nix-offload: {why}");
                there::UNREACHED
            }
        }
    }
}

/// How building `names`, which an evaluator named, from the flake in
/// `folder` did.
fn built_evaluated(folder: &Path, system: &str, names: &[String]) -> i32 {
    let built = changed::building(folder, system, names).status();
    built.map_or(1, |built| evaluated_ended(built.code(), names.len()))
}

/// How building `names` did, as `built_evaluated` says, with the subtests
/// its log says failed.
fn built_heard(folder: &Path, system: &str, names: &[String]) -> changed::Heard {
    let heard = changed::hearing(&mut changed::building(folder, system, names));
    heard.map_or_else(
        |_| (1, BTreeSet::new()),
        |(mut building, failed)| {
            let code = building.wait().ok().and_then(|built| built.code());
            (evaluated_ended(code, names.len()), failed)
        },
    )
}

/// What a build of `named` checks an evaluator named ended with, said as
/// the run's own status.
fn evaluated_ended(code: Option<i32>, named: usize) -> i32 {
    match code {
        // Never `NOT_TAKEN`, which would have changed-checks do it all
        // here, the build it was just refused included.
        Some(there::UNSENT) => {
            eprintln!(
                "nix-offload: {named} checks were named, but no builder took their build, \
                 so nothing was built"
            );
            i32::from(changed::UNBUILT)
        }
        code => code.unwrap_or(1),
    }
}

/// How the `there` sending `named` VM tests on did, once it ends: never
/// `NOT_TAKEN`, which would have changed-checks run it all here.
fn tested(mut testing: Child, named: usize) -> i32 {
    match testing.wait().ok().and_then(|status| status.code()) {
        Some(there::UNSENT) => {
            eprintln!(
                "nix-offload: {named} VM tests were named, but no builder took their build, \
                 so they weren't built"
            );
            i32::from(changed::UNBUILT)
        }
        code => code.unwrap_or(1),
    }
}

/// `serve`: lends builders while the queue has builds one could take.
fn serving(arguments: &[String], given: impl Fn(&str) -> Option<String>) -> Result<(), Failure> {
    let word = arguments.first().map(String::as_str);
    if !matches!(word, Some("report" | "serve")) {
        return Err(unknown(word));
    }
    let Some(rate) = given("--rate").as_deref().and_then(journal::price) else {
        return Err(ended::asked_wrongly(
            "--rate takes a price, such as 0.68",
            word.unwrap_or("serve"),
        ));
    };
    // A lender's lock and journal are named for its folder, so a second
    // lender, for VM tests, never gives back what the first lent.
    let dir = given("--dir").map_or_else(|| PathBuf::from(DIR), PathBuf::from);
    let name = dir.file_name().map_or_else(
        || "nix-offload".into(),
        |name| name.to_string_lossy().into_owned(),
    );
    let journal = state().join(&name).join("journal.jsonl");
    match arguments.first().map(String::as_str) {
        Some("report") => {
            let journal = fs::read_to_string(&journal).unwrap_or_default();
            println!("{}", journal::said(&journal::day(&journal, today(), rate)));
            Ok(())
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
                return Err(ended::asked_wrongly(
                    "serve needs each of its settings",
                    "serve",
                ));
            };
            let longest = span(given("--longest"), "--longest")?;
            let idle = span(given("--idle"), "--idle")?.map_or(decide::IDLE, u64::from);
            if !(first <= last && last <= deadline) {
                return Err(ended::asked_wrongly(
                    "the day's hours must run first, last, deadline",
                    "serve",
                ));
            }
            let (runtime, _lock) = locked(&name)?;
            let superseded = serve(&Settings {
                helper: helper.into(),
                prefix,
                hours: Hours {
                    first,
                    last,
                    deadline,
                    longest,
                    idle,
                },
                queue: runtime.join("jobs/queue.json"),
                standing: given("--standing").map(PathBuf::from),
                beside: given("--beside").map(PathBuf::from),
                journal,
                dir,
                only_there: arguments.iter().any(|argument| argument == "--only-there"),
                warm: given("--warm")
                    .map(|warm| warm.split_whitespace().map(str::to_owned).collect())
                    .unwrap_or_default(),
                outlasting: given("--outlasting").map(PathBuf::from),
                telling: given("--tell").map(PathBuf::from),
                freeing: given("--freeing").map(PathBuf::from),
            });
            ended::lent(superseded)
        }
        _ => Err(unknown(word)),
    }
}

/// The span `text` gives in HH:MM, for `flag`, which takes one more than
/// none or is left out.
fn span(text: Option<String>, flag: &str) -> Result<Option<u32>, Failure> {
    match text.map(|text| second_of(&text)) {
        None => Ok(None),
        Some(Some(span)) if span > 0 => Ok(Some(span)),
        Some(_) => Err(ended::asked_wrongly(
            format!("{flag} takes HH:MM, more than none"),
            "serve",
        )),
    }
}

/// The runtime folder, and the lock a lender named `name` holds in it
/// while it serves: two would each give back what the other lent.
fn locked(name: &str) -> Result<(PathBuf, fs::File), Failure> {
    let runtime = env::var_os("XDG_RUNTIME_DIR").map_or_else(
        || PathBuf::from("/run/user").join(unsafe { libc::getuid() }.to_string()),
        PathBuf::from,
    );
    let Ok(lock) = fs::File::create(runtime.join(format!("{name}.lock"))) else {
        return Err(Failure::new(
            Exit::Failed,
            format!("can't make its lock in {}", runtime.display()),
        ));
    };
    if lock.try_lock().is_err() {
        return Err(Failure::new(Exit::Failed, "another serve is running"));
    }
    Ok((runtime, lock))
}

/// The failure for a call whose first word is none of nix-offload's
/// subcommands, naming the nearest where one is near.
fn unknown(word: Option<&str>) -> Failure {
    let help = help::help();
    let known: Vec<&str> = help.subs.iter().map(|sub| sub.name.as_str()).collect();
    match word {
        Some(word) => terminal_kit::flags::unknown("subcommand", word, &known),
        None => ended::usage("nix-offload needs a subcommand").try_this("nix-offload --help"),
    }
}

/// Hear a stop, so what's lent is given back first.
fn hear_stops() {
    // SAFETY: the handler only stores to an atomic.
    unsafe {
        let handler = ending as *const () as libc::sighandler_t;
        libc::signal(libc::SIGTERM, handler);
        libc::signal(libc::SIGINT, handler);
        let handler = superseded as *const () as libc::sighandler_t;
        libc::signal(libc::SIGHUP, handler);
    }
}

fn read_standing(path: &Path) -> Vec<machines::Standing> {
    machines::standing(&fs::read_to_string(path).unwrap_or_else(|error| {
        eprintln!("nix-offload: reading {}: {error}", path.display());
        String::new()
    }))
}

/// Name the standing builders, and those beside, that answer, lending
/// none, until told to end, or at once once superseded, since it holds
/// nothing. It says whether it ended superseded.
fn stand(standing: Option<PathBuf>, beside: Option<PathBuf>) -> bool {
    hear_stops();
    let read = |path: &Option<PathBuf>| path.as_deref().map_or_else(Vec::new, read_standing);
    let mut machines = Machines::new(
        MACHINES.into(),
        WITH_STANDING.into(),
        STORE.into(),
        LENDING.into(),
        read(&standing),
    )
    .beside(BESIDE.into(), read(&beside))
    .going_on_from(
        proof::earlier(&Path::new(DIR).join(proof::HEALTH), "beside")
            .iter()
            .filter_map(Health::from_json)
            .collect(),
    )
    .for_anything(ANYTHING.into())
    .read_from(standing, beside);
    while !ENDING.load(Ordering::SeqCst) {
        if SUPERSEDED.load(Ordering::SeqCst) {
            return true;
        }
        machines.show(None);
        proof::write(
            Path::new(DIR),
            None,
            machines.beside_health().as_deref(),
            now(),
        );
        rest(TICK);
    }
    false
}

/// The files a lender names its builders in. Each is in the lender's own
/// folder, so a second lender, for VM tests, writes none that Nix or the
/// queue reads.
fn machines_of(settings: &Settings) -> Machines {
    let standing = settings
        .standing
        .as_deref()
        .map_or_else(Vec::new, read_standing);
    let dir = &settings.dir;
    let machines = Machines::new(
        dir.join("machines"),
        dir.join("machines-with-standing"),
        dir.join("store"),
        dir.join("lending"),
        standing,
    )
    .beside(
        dir.join("machines-beside"),
        settings
            .beside
            .as_deref()
            .map_or_else(Vec::new, read_standing),
    )
    .for_anything(dir.join("machines-for-anything"))
    .read_from(settings.standing.clone(), settings.beside.clone())
    .freeing_from(settings.freeing.clone())
    // Every lender says where its builder is, since another's builder
    // stands in for the first's in the lines root reads, and each line
    // is vouched for by its own lender's folder.
    .in_cloud(cloud::Cloud::in_dir(
        dir,
        Path::new(cloud::TOKENS),
        Path::new(cloud::ACCOUNTS),
    ));
    // Only the lender for every build has others stand in for it: the
    // rest lend only to `there`, which reads no file for any work.
    if settings.only_there {
        return machines;
    }
    let vm_dir =
        env::var_os("NIX_OFFLOAD_VM_DIR").map_or_else(|| PathBuf::from(VM_DIR), PathBuf::from);
    machines
        .stand_ins_from(more_dir(), vm_dirs(&vm_dir))
        .telling(settings.telling.clone())
}

/// Prove `builder` with a tiny build when one is due, over `ssh` as
/// `there` reaches it, and then a copy up as `nix` sends one, keeping its
/// proofs in `keeper`, which starts afresh for a builder it didn't keep:
/// whether its last `proof::FAILS` failed. One whose helper named no store
/// fails, since only the daemon could reach it, and the daemon takes a
/// builder line only from an account it trusts.
fn kept(
    settings: &Settings,
    keeper: &mut Option<Keeper>,
    builder: &Lent,
    at: Instant,
    [ssh, nix]: [&str; 2],
) -> bool {
    if keeper
        .as_ref()
        .is_none_or(|keeper| keeper.health.builder != builder.name)
    {
        *keeper = Some(Keeper::of(&builder.name));
    }
    let Some(keeper) = keeper.as_mut() else {
        return false;
    };
    let store = builder.store.clone();
    let (scratch, ssh, nix) = (
        settings.dir.join("proving.d"),
        ssh.to_owned(),
        nix.to_owned(),
    );
    let (name, moving) = (builder.name.clone(), moving_to(settings, &builder.name));
    let was = keeper.health.state();
    let heard = keeper.keep(at, now(), move || {
        store.map_or_else(proof::Proven::default, |store| {
            let built = proof::over_ssh(&store, &scratch, &ssh, proof::WITHIN);
            proof::carrying(built, || {
                proof::carries(&store, &scratch, &nix, proof::CARRYING)
                    || proof::busy(&name, &moving)
            })
        })
    });
    let name = &builder.name;
    let state = keeper.health.state();
    match heard {
        Some(false) if keeper.health.stalled && !keeper.health.dropped() => eprintln!(
            "nix-offload: {name} built its proof but couldn't carry a copy up; another in a row \
             and no new build goes there until the way to it carries one"
        ),
        Some(false) if !keeper.health.dropped() => {
            eprintln!("nix-offload: {name} failed a proof; another in a row gives it back");
        }
        Some(true) if state == State::Full && was != State::Full => eprintln!(
            "nix-offload: {name} has {} GB free, under the {} GB floor, so builders with room \
             stand in for any work",
            keeper.health.free.unwrap_or_default() / 1_000_000_000,
            proof::FLOOR / 1_000_000_000
        ),
        Some(true) if was == State::Full && state == State::Passing => {
            eprintln!("nix-offload: {name} has room again");
        }
        Some(true) if was == State::Failing => {
            eprintln!("nix-offload: {name} passed its proof again");
        }
        _ => {}
    }
    keeper.health.dropped()
}

/// What's known at this tick.
fn ticked(settings: &Settings, sleep: &mut sleep::Sleep) -> Tick {
    let asked = there::wanted(&settings.dir);
    Tick {
        now: now(),
        into_day: into_day(),
        wants: decide::wants(
            settings.only_there,
            || fs::read_to_string(&settings.queue).is_ok_and(|queue| queue::wants(&queue, now())),
            asked,
        ),
        asked,
        woke: sleep.woke(now()),
    }
}

/// Give back `builder` as `event` says: as one unanswered when its
/// proofs in `keeper` say it failed them, keeping them for its folder to
/// say so until another stands, else forgetting them.
fn given_back(
    settings: &Settings,
    machines: &mut Machines,
    builder: Lent,
    event: Event,
    keeper: &mut Option<Keeper>,
) {
    let unanswered = keeper
        .as_ref()
        .is_some_and(|keeper| keeper.health.builder == builder.name && keeper.health.dropped());
    if unanswered {
        give_back_unanswered(settings, machines, builder);
    } else {
        *keeper = None;
        give_back(settings, machines, builder, event);
    }
}

/// Name the one `lent`, if any, in every file a build reads, and say how
/// it stands by the proofs `keeper` holds in `dir`'s health.json, beside
/// the builders beside. Only its own proofs say how the one lent stands:
/// a successor that just took over is yet to be proven.
fn named(dir: &Path, machines: &mut Machines, lent: Option<&Lent>, keeper: Option<&Keeper>) {
    let health = keeper.map(|keeper| &keeper.health);
    let state = match (lent, health) {
        (Some(builder), Some(health)) if health.builder == builder.name => health.state(),
        (Some(_), _) => State::Unproven,
        (None, Some(health)) => health.state(),
        (None, None) => State::None,
    };
    machines.proven(lent.map(|builder| builder.line.as_str()), state);
    machines.way_stalled(matches!((lent, health), (Some(builder), Some(health))
            if health.builder == builder.name && health.stalled));
    machines.placing(lent.map(|builder| placed(&builder.place)));
    machines.show(lent.map(Lent::lending));
    proof::write(dir, health, machines.beside_health().as_deref(), now());
}

/// What a lender has said of the builder it keeps though it fails its
/// proofs, so it says each once while that lasts.
#[derive(Default)]
struct SaidKept {
    /// The builder this is said of, so the next one starts afresh.
    builder: String,
    reached: bool,
    stalled: bool,
    busy: bool,
    /// How many times in a row the builder hasn't said whether it answers.
    silent: u32,
}

impl SaidKept {
    /// What's said of `builder` from now: a builder lent after one given
    /// back doesn't take its count of missed answers, or it could go at
    /// its first miss.
    fn of(&mut self, builder: &str) {
        if self.builder != builder {
            *self = Self {
                builder: builder.to_owned(),
                ..Self::default()
            };
        }
    }
}

/// Whether `builder`, which a build `reached` this tick or not, is given
/// back as lost by its proofs in `keeper`, proven at `at` with `ssh` and
/// `nix`, saying once in `said` why one failing them is kept instead.
fn unanswered(
    settings: &Settings,
    keeper: &mut Option<Keeper>,
    builder: &Lent,
    reached: bool,
    at: Instant,
    said: &mut SaidKept,
    [ssh, nix]: [&str; 2],
) -> bool {
    said.of(&builder.name);
    let dropped = kept(settings, keeper, builder, at, [ssh, nix]);
    let failures = keeper.as_ref().map_or(0, |keeper| keeper.health.failures);
    // One that builds but can't take a copy has a way to it that's full,
    // not gone, and is kept as one builds reach is: given back, it would
    // be lent again over the same way, cold, and a VM builder takes 17
    // minutes to warm.
    let stalled = keeper.as_ref().is_some_and(|keeper| keeper.health.stalled);
    let silent_before = said.silent;
    // Whether it was asked if it answers, as proven_lost decided, so what
    // is said can't drift from the decision.
    let mut asked = false;
    let unanswered = decide::proven_lost(
        dropped,
        failures,
        reached || stalled,
        &mut said.silent,
        || {
            asked = true;
            answers(settings, builder, ssh)
        },
    );
    if unanswered {
        let reaching = if reached || stalled {
            "though builds reach it"
        } else {
            "and no build is seen to reach it"
        };
        eprintln!(
            "nix-offload: {} didn't say whether it answers {} times in a row, so it's lost \
             {reaching}",
            builder.name, said.silent
        );
    }
    if !unanswered && said.silent > silent_before && builder.store.is_none() {
        eprintln!(
            "nix-offload: {} has no store of its own to ask whether it answers, {} of {} in a \
             row, so it keeps what it builds for now",
            builder.name,
            said.silent,
            decide::SILENT_WHILE_REACHED
        );
    } else if !unanswered && said.silent > silent_before {
        eprintln!(
            "nix-offload: {} didn't say within {} s whether it answers, {} of {} in a row, so \
             it keeps what it builds for now",
            builder.name,
            proof::ANSWERING.as_secs(),
            said.silent,
            decide::SILENT_WHILE_REACHED
        );
    }
    if dropped && !unanswered && stalled && !std::mem::replace(&mut said.stalled, true) {
        eprintln!(
            "nix-offload: {} builds but couldn't carry a copy up {failures} proofs in a row, \
             so no new build goes there, but it's kept until the way to it carries one, or \
             once it has failed {} in a row, until it stops answering",
            builder.name,
            decide::FAILS_WHILE_REACHED
        );
    }
    said.stalled &= dropped && stalled;
    if dropped && !unanswered && !stalled && reached && !std::mem::replace(&mut said.reached, true)
    {
        eprintln!(
            "nix-offload: {} failed {failures} proofs in a row, so no new build goes there, but \
             builds still reach it, so it's kept until they end, or once it has failed {} in a \
             row, until it stops answering",
            builder.name,
            decide::FAILS_WHILE_REACHED
        );
    }
    said.reached &= dropped;
    let busy = dropped && !unanswered && said.silent == 0 && asked;
    if busy && !std::mem::replace(&mut said.busy, true) {
        eprintln!(
            "nix-offload: {} still answers, so it's busy rather than lost, and keeps what it \
             builds",
            builder.name
        );
    }
    said.busy &= dropped;
    unanswered
}

/// Whether `builder` answers at all, its daemon saying which store it is
/// over `ssh`, for one failing its proofs while builds still reach it. One
/// reached only through root's daemon has no store of its own to ask, so
/// it's taken as not answering, as before this was asked.
fn answers(settings: &Settings, builder: &Lent, ssh: &str) -> bool {
    let Some(store) = &builder.store else {
        return false;
    };
    proof::answers_over_ssh(
        store,
        &settings.dir.join("answering.d"),
        ssh,
        proof::ANSWERING,
    )
}

/// Give back `builder`, which failed its last proofs, as lost: no build
/// is sent there from now, since every file this lender writes leaves it
/// out, and ending its helper ends the relay, closing every connection
/// hung on it, so Nix sends those builds elsewhere. Another is asked for
/// at once, as after any builder lost.
fn give_back_unanswered(settings: &Settings, machines: &mut Machines, builder: Lent) {
    eprintln!(
        "nix-offload: {} failed its last proofs, so no build goes there, its relay ends and \
         it's given back",
        builder.name
    );
    machines.proven(None, State::Dropped);
    give_back(settings, machines, builder, Event::Lost);
}

/// Whether a lender whose unit changed ends now, and how, as
/// `decide::restarts` says, saying the first time it's put off why it
/// waits.
fn ends_superseded(reached: bool, finishing: bool, said: &mut bool) -> Option<Restart> {
    let superseded = SUPERSEDED.load(Ordering::SeqCst);
    let how = decide::restarts(superseded, reached, finishing);
    match how {
        Some(Restart::Ending) => {
            eprintln!(
                "nix-offload: its unit changed and nothing uses its builder, so it starts again"
            );
        }
        Some(Restart::HandingOver) => eprintln!(
            "nix-offload: its unit changed, so it starts again, and builds go on reaching its \
             builder while the next start takes it back"
        ),
        None if superseded && !*said => {
            eprintln!(
                "nix-offload: its unit changed, so it starts again once the builder it handed \
                 over finishes"
            );
            *said = true;
        }
        None => {}
    }
    how
}

/// Lend a builder while the queue wants one, until told to end, or until
/// superseded once nothing uses or wants its builder, saying whether it
/// ended superseded.
fn serve(settings: &Settings) -> bool {
    hear_stops();
    hear_hand_overs();
    let dir = &settings.dir;
    let mut machines = machines_of(settings);
    // Said so a build with no builder waits for one only while a lender
    // runs to make it.
    let lender = dir.join("lender");
    if let Err(error) = fs::write(&lender, format!("{}\n", std::process::id())) {
        eprintln!(
            "nix-offload: couldn't say I lend in {}: {error}",
            lender.display()
        );
    }
    // And held locked while it runs, since a fenced run has a pid
    // namespace of its own, where the pid names nothing. Shared, as the
    // lender it supersedes holds it too until it ends.
    let _lends = fs::File::open(&lender).and_then(|file| file.lock_shared().map(|()| file));
    // Runs once said they wanted a builder in the lender's folder, which
    // a fence gives its commands only to read; they say it beside it now
    // (there::wants_of).
    let _ = fs::remove_dir_all(dir.join("wanted"));
    let mut lent = start(settings, &mut machines, into_day());
    let mut leaving: Vec<Leaving> = Vec::new();
    let mut hold = decide::Hold::default();
    let mut hand_over_after = 0;
    let mut said_on = None;
    let mut sleep = sleep::Sleep::default();
    let mut said_superseded = false;
    let mut restarts = false;
    let mut handing_over = false;
    let mut said_kept = SaidKept::default();
    let mut outliving_as_it_restarts: Option<bool> = None;
    // The builders left to finish that this start has said it keeps.
    let mut said_finishing = BTreeSet::new();
    // The lent builder's proofs, kept past its give-back when it failed
    // them, so its folder says so until another stands.
    let mut keeper: Option<Keeper> = None;
    while !ENDING.load(Ordering::SeqCst) {
        let tick = ticked(settings, &mut sleep);
        let unreturned =
            give_back_unreturned(settings, lent.as_ref(), &leaving, &mut said_finishing);
        let mut reached = false;
        lent = match lent.take() {
            None => decide::lend(settings.hours, tick, hold, unreturned).and_then(|seconds| {
                // Told to hand over with nothing lent, there's nothing to hand over.
                TOLD_TO_HAND_OVER.store(false, Ordering::SeqCst);
                let lent = lend(settings, &mut machines, tick.now, seconds);
                if lent.is_none() {
                    hold = after_failing(tick);
                }
                lent
            }),
            Some(mut builder) => {
                reached = reached_now(settings, tick, &builder, &mut outliving_as_it_restarts);
                if reached {
                    builder.used = tick.now;
                }
                let gone = builder.helper.try_wait().ok().flatten().is_some();
                let unanswered = !gone
                    && unanswered(
                        settings,
                        &mut keeper,
                        &builder,
                        reached,
                        Instant::now(),
                        &mut said_kept,
                        ["ssh", "nix"],
                    );
                let (used, ends) = (builder.used, builder.ends);
                match decide::give_back_proven(settings.hours, tick, used, ends, gone, unanswered) {
                    Some(event) => {
                        given_back(settings, &mut machines, builder, event, &mut keeper);
                        hold = decide::next_after(event, tick.wants, used, ends, tick.woke, now());
                        None
                    }
                    None => Some(handed_over(
                        settings,
                        &mut machines,
                        builder,
                        tick,
                        reached,
                        &mut hand_over_after,
                        &mut leaving,
                    )),
                }
            }
        };
        leaving = let_go(settings, leaving, now());
        // A leaving builder's helper runs while a build reaches it, and
        // nothing after a restart would give that builder back.
        let finishing = leaving
            .iter_mut()
            .any(|left| runs(&mut left.builder.helper));
        // Once a day, after the deadline, say what's there that isn't this
        // one's to give back.
        let day = local_day();
        if tick.into_day >= settings.hours.deadline && said_on != Some(day) {
            say_others(settings);
            said_on = Some(day);
        }
        named(dir, &mut machines, lent.as_ref(), keeper.as_ref());
        if let Some(how) = ends_superseded(reached, finishing, &mut said_superseded) {
            restarts = true;
            handing_over = how == Restart::HandingOver;
            break;
        }
        rest(TICK);
    }
    if let Some(builder) = lent {
        let outliving = outliving_as_it_restarts.unwrap_or(false);
        left_as_it_ends(settings, &mut machines, builder, handing_over, outliving);
    }
    left_leaving(settings, leaving, handing_over);
    restarts
}

/// Whether a build reaches `builder` this `tick`. Once its unit changed,
/// a run outliving its caller there counts too, asked once into
/// `as_it_restarts`, so the builder is handed over and held for the next
/// start: with no ssh reaching it, nothing else says it's in use.
fn reached_now(
    settings: &Settings,
    tick: Tick,
    builder: &Lent,
    as_it_restarts: &mut Option<bool>,
) -> bool {
    let reached = in_use(Path::new("/proc"), &builder.host) || outlives_on(settings, tick, builder);
    // Asked even when reached, since its note says so once its ssh ends.
    let restarts = SUPERSEDED.load(Ordering::SeqCst);
    let outlives = restarts && *as_it_restarts.get_or_insert_with(|| outliving(builder));
    reached || outlives
}

/// Leave each builder that handed over to a successor for the next start.
/// As a restart `hands_over`, each keeps its way, finishing what it has,
/// and is noted so the next start gives it back only once that's done;
/// otherwise its helper ends with the service.
fn left_leaving(settings: &Settings, leaving: Vec<Leaving>, hands_over: bool) {
    for Leaving {
        mut builder,
        outliving,
    } in leaving
    {
        if hands_over {
            finishes(settings, &builder, outliving, false);
        } else {
            end(&mut builder.helper);
        }
        leave(settings, &builder.name, builder.since, builder.rate);
    }
}

/// Note that `builder`'s helper finishes the builds it carries, whether a
/// run `outliving` its caller builds there, and whether its way is left
/// `accepting` new connections, for the next start.
fn finishes(settings: &Settings, builder: &Lent, outliving: bool, accepting: bool) {
    let left = finishing::Finishing {
        helper: Some(builder.helper.id()),
        outliving,
        ends: builder.ends,
        accepting,
    };
    if let Err(error) = finishing::note(&settings.dir, &builder.name, &left) {
        eprintln!(
            "nix-offload: couldn't note that {} finishes its builds: {error}, so the next \
             start may give it back under them",
            builder.name
        );
    }
}

/// Whether a run outliving its caller builds on `builder` now, asked while
/// its way stands, since none reaches it once it's left.
fn outliving(builder: &Lent) -> bool {
    builder.store.as_deref().is_some_and(|store| {
        let scratch = env::var_os("XDG_RUNTIME_DIR")
            .map_or_else(env::temp_dir, PathBuf::from)
            .join("nix-offload-serve");
        there::outlives_on(store, &scratch, "ssh")
    })
}

/// The builder lent from this tick on: a successor once `builder`, given
/// whether a build `reached` it this `tick`, hands over, as its time is
/// nearly up or it's told to, or `builder` itself. The one handed over
/// joins `leaving`.
fn handed_over(
    settings: &Settings,
    machines: &mut Machines,
    builder: Lent,
    tick: Tick,
    reached: bool,
    hand_over_after: &mut u64,
    leaving: &mut Vec<Leaving>,
) -> Lent {
    let (name, left) = (&builder.name, builder.ends.saturating_sub(tick.now) / 60);
    let seconds = if TOLD_TO_HAND_OVER.swap(false, Ordering::SeqCst) {
        let seconds = decide::hands_over_told(settings.hours, tick, builder.ends, *hand_over_after);
        match seconds {
            Some(_) => eprintln!("nix-offload: {name} is told to hand over, {left} minutes early"),
            None => eprintln!(
                "nix-offload: {name} is told to hand over, but can't: it lives to the deadline, \
                 its time is up, or a successor failed to stand within the hour"
            ),
        }
        seconds
    } else {
        let seconds = decide::hands_over(
            settings.hours,
            tick,
            builder.ends,
            reached,
            *hand_over_after,
        );
        if seconds.is_some() {
            eprintln!(
                "nix-offload: {name} ends within {left} minutes and a build uses or wants it"
            );
        }
        seconds
    };
    let Some(seconds) = seconds else {
        return builder;
    };
    let (kept, outgoing) = hand_over(settings, machines, builder, tick.now, seconds);
    match outgoing {
        Some(outgoing) => leaving.push(outgoing),
        None => *hand_over_after = after_failing(tick).until,
    }
    kept
}

/// When to ask again for a builder that failed to stand.
fn after_failing(tick: Tick) -> decide::Hold {
    decide::next_after(Event::Failed, tick.wants, 0, u64::MAX, false, now())
}

/// Whether a helper still runs.
fn runs(helper: &mut Child) -> bool {
    helper.try_wait().ok().flatten().is_none()
}

/// Ask for a successor to `builder`, to live `seconds`. Once it stands, Nix is given its
/// line alone, and the old builder's helper is told to hand over, taking
/// no new connection. It gives the builder lent from then on, and the one
/// leaving, if any: without a successor, the old builder is kept as it was.
fn hand_over(
    settings: &Settings,
    machines: &mut Machines,
    builder: Lent,
    now: u64,
    seconds: u64,
) -> (Lent, Option<Leaving>) {
    let name = format!("{}{now}", settings.prefix);
    eprintln!(
        "nix-offload: so {name} is asked for to take over from {}",
        builder.name
    );
    let outliving = outliving(&builder);
    let Some(successor) = ask(settings, machines, name, now, seconds) else {
        eprintln!(
            "nix-offload: no successor stood, so {} keeps on",
            builder.name
        );
        return (builder, None);
    };
    told_to_hand_over(&builder.helper);
    eprintln!(
        "nix-offload: {} took over; {} finishes what it has",
        successor.name, builder.name
    );
    (successor, Some(Leaving { builder, outliving }))
}

/// Tell a helper its builder hands over: the helper alone, since the
/// tunnel it holds would end on the signal.
fn told_to_hand_over(helper: &Child) {
    if let Ok(pid) = libc::pid_t::try_from(helper.id()) {
        // SAFETY: a signal to our own child, which traps it.
        unsafe { libc::kill(pid, libc::SIGUSR1) };
    }
}

/// Give back each builder that handed over once `decide::leaves` says so,
/// keeping the rest.
fn let_go(settings: &Settings, leaving: Vec<Leaving>, now: u64) -> Vec<Leaving> {
    let mut kept = Vec::new();
    for mut left in leaving {
        let gone = !runs(&mut left.builder.helper);
        if decide::leaves(now, left.builder.ends, gone, left.outliving) {
            returned(settings, left.builder, Event::Stopped);
        } else {
            kept.push(left);
        }
    }
    kept
}

/// Leave the builder lent as the service ends for the next start to take
/// back. Its helper ends with it and its line goes, unless it
/// `hands_over`: then the helper, run through `--outlasting` so the unit's
/// end leaves it, and its line stay, taking new connections until the
/// next start holds the builder, as `start` says, and the builder is noted
/// as finishing, with whether a run `outliving` its caller builds there,
/// so a start that can't take it back leaves it until they end.
fn left_as_it_ends(
    settings: &Settings,
    machines: &mut Machines,
    mut builder: Lent,
    hands_over: bool,
    outliving: bool,
) {
    if hands_over {
        finishes(settings, &builder, outliving, true);
    } else {
        machines.show(None);
        end(&mut builder.helper);
    }
    leave(settings, &builder.name, builder.since, builder.rate);
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
    eprintln!("nix-offload: ending, so {name} is left for the next start to take back");
}

/// Ask for a builder, naming it as being made until it stands or won't, so
/// the machine queue holds the builds it could take until Nix can see it.
fn lend(settings: &Settings, machines: &mut Machines, now: u64, left: u64) -> Option<Lent> {
    let name = format!("{}{now}", settings.prefix);
    eprintln!("nix-offload: asking for {name}, since the queue has builds a builder could take");
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
    if let Err(error) = journal::note(&settings.journal, now, Event::Asked, &name, None) {
        // Unjournalled, it couldn't be known as this one's to give back.
        eprintln!("nix-offload: journalling the ask: {error}; none is asked for");
        return None;
    }
    let (mut helper, said, place) = hear(settings, &name, &["up", &name, &left.to_string()])?;
    let Heard {
        rate, store, line, ..
    } = said;
    let line = line.trim();
    match host_of(line) {
        Some(host) if helper.try_wait().ok().flatten().is_none() => {
            // Said warming before Nix and `there` can see it.
            if let Some(store) = &store {
                warm(settings, &name, store);
            }
            machines.placing(Some(placed(&place)));
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
                ends: now + left,
                place,
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

/// Has the builder `name`, just stood, build what `settings.warm` names
/// for itself, on a thread of its own, saying so in its folder's
/// `warming` until it's done, so `there` sends VM tests to a warm builder
/// while one stands. Its ssh holds the builder in use as it warms.
fn warm(settings: &Settings, name: &str, store: &str) {
    let Some(builder) = there::builder(store) else {
        return;
    };
    if settings.warm.is_empty() {
        return;
    }
    let warming = settings.dir.join(there::WARMING);
    let held = format!("{} {name}\n", std::process::id());
    if let Err(error) = fs::write(&warming, &held) {
        eprintln!("nix-offload: couldn't say {name} warms: {error}, so it takes VM tests cold");
        return;
    }
    let installables = settings.warm.clone();
    let scratch = settings.dir.join("warming.d");
    let name = name.to_owned();
    eprintln!("nix-offload: {name} builds what warms it before it takes VM tests");
    thread::spawn(move || {
        let began = Instant::now();
        let warmed = there::warmed(&builder, &installables, &scratch, "ssh");
        // Only its own: a builder stood after this one went has its own.
        if fs::read_to_string(&warming).is_ok_and(|now| now == held) {
            let _ = fs::remove_file(&warming);
        }
        let minutes = began.elapsed().as_secs() / 60;
        if warmed {
            eprintln!("nix-offload: {name} is warm after {minutes} minutes, so it takes VM tests");
        } else {
            eprintln!(
                "nix-offload: {name} couldn't build all that warms it in {minutes} minutes, so \
                 it takes VM tests as it is"
            );
        }
    });
}

/// Run the helper with `arguments`, and hear what it says until its
/// machines line, for as long as a builder may take to be ready, then
/// where its builder is for as long as it runs, into the place given.
/// The file the relay to builder `name` says what went up to it in.
fn moving_to(settings: &Settings, name: &str) -> PathBuf {
    settings.dir.join("moving.d").join(name)
}

/// The helper, run with `arguments` for builder `name`, and what it said
/// once its builder was made; none when it couldn't be run.
fn hear(
    settings: &Settings,
    name: &str,
    arguments: &[&str],
) -> Option<(Child, Heard, Arc<Mutex<Place>>)> {
    let mut helper = match &settings.outlasting {
        Some(outlasting) => {
            let mut through = Command::new(outlasting);
            through.arg(&settings.helper);
            through
        }
        None => Command::new(&settings.helper),
    };
    let mut helper = helper
        .args(arguments)
        // Its relay inherits it, so says there what goes up to its builder.
        .env(relay::MOVED_UP, moving_to(settings, name))
        .stdout(Stdio::piped())
        .process_group(0)
        .spawn()
        .map_err(|error| eprintln!("nix-offload: running the helper: {error}"))
        .ok()?;
    let (said, heard) = mpsc::channel();
    let out = helper.stdout.take()?;
    let place = Arc::new(Mutex::new(Place::default()));
    let placing = Arc::clone(&place);
    thread::spawn(move || {
        let mut out = BufReader::new(out);
        let heard = heard_from(&mut out);
        if let Ok(mut place) = placing.lock() {
            place.clone_from(&heard.place);
        }
        let _ = said.send(heard);
        heard_after(out, &placing);
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
    Some((helper, said, place))
}

/// Start lending, taking back what the last stop left. A restart that
/// handed its builder over left its line named and its way taking
/// connections, so builds go on reaching the builder while this takes it
/// back. On 13 November a switch restarted both lenders, and with the
/// line out for the 23 s each took, root's Nix built here. Only once this
/// start holds the builder, or names it nowhere, is the old way told to
/// take no new connection.
fn start(settings: &Settings, machines: &mut Machines, into_day: u32) -> Option<Lent> {
    let accepting = finishing::accepting(&settings.dir);
    if accepting.is_empty() {
        machines.show(None);
    }
    machines.asking(None);
    let lent = adopt(settings, machines, into_day);
    if lent.is_none() && !accepting.is_empty() {
        machines.show(None);
    }
    for name in &accepting {
        finishing::hand_over(&settings.dir, name);
    }
    lent
}

/// Take back the builder the last stop left, before the deadline, rather
/// than give it back and make another: a switch restarts this, and a new
/// builder costs a minute's wait and a cold store, where a VM test took
/// 201 s against 67 s on one already warm. It keeps the time it was asked
/// for, so it's priced whole, and the time the stop left it as its last
/// use, so its idle time runs on. One that can't be held is given back by
/// the first tick, as any left builder is. While it's taken back it's
/// named as coming, as `lend` names one being made, so a build waiting on
/// this lender waits for it rather than giving up: on 13 November one took
/// 96 s, and 22 runs ended unsent at 30 s.
fn adopt(settings: &Settings, machines: &mut Machines, into_day: u32) -> Option<Lent> {
    if decide::past_deadline(settings.hours, into_day) {
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
    machines.asking(Some(&name));
    let Some((mut helper, said, place)) = hear(settings, &name, &["hold", &name]) else {
        machines.asking(None);
        return None;
    };
    let Heard {
        rate, store, line, ..
    } = said;
    let line = line.trim();
    let Some(host) = host_of(line).filter(|_| helper.try_wait().ok().flatten().is_none()) else {
        if finishing::still(&settings.dir, &name, now()) {
            eprintln!(
                "nix-offload: {name} couldn't be taken back, but the builds the last stop left \
                 it still run there, so it's kept until they end"
            );
        } else {
            eprintln!("nix-offload: {name} couldn't be taken back, so it's given back");
        }
        machines.asking(None);
        end(&mut helper);
        return None;
    };
    machines.placing(Some(placed(&place)));
    machines.show(Some(machines::Lending {
        line,
        store: store.as_deref(),
    }));
    // Only once Nix can see it, so a build waiting never sees neither.
    machines.asking(None);
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
        ends: decide::ends(settings.hours, left.asked, now(), into_day),
        place,
    })
}

/// Take the builder from Nix, then give it back, noting how long it ran.
fn give_back(settings: &Settings, machines: &mut Machines, builder: Lent, event: Event) {
    machines.show(None);
    let warming = settings.dir.join(there::WARMING);
    if fs::read_to_string(&warming).is_ok_and(|held| held.trim_end().ends_with(&builder.name)) {
        let _ = fs::remove_file(&warming);
    }
    returned(settings, builder, event);
}

/// Give a builder Nix no longer names back, noting how long it ran.
fn returned(settings: &Settings, mut builder: Lent, event: Event) {
    end(&mut builder.helper);
    if down(settings, &builder.name) {
        finishing::forget(&settings.dir, &builder.name);
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
/// the one lent, those leaving, and those still finishing what the last
/// stop left them, and say how many are still not gone. One finishing is
/// said once, into `said`, and doesn't hold up lending another, since its
/// builds end on their own: on 10 November a start that couldn't take two
/// builders back gave both back under the builds they carried.
fn give_back_unreturned(
    settings: &Settings,
    lent: Option<&Lent>,
    leaving: &[Leaving],
    said: &mut BTreeSet<String>,
) -> usize {
    let journal = fs::read_to_string(&settings.journal).unwrap_or_default();
    let mut still = 0;
    for builder in journal::unreturned(&journal) {
        let name = &builder.name;
        let mut held = lent
            .into_iter()
            .chain(leaving.iter().map(|left| &left.builder));
        if held.any(|held| &held.name == name) {
            continue;
        }
        if finishing::still(&settings.dir, name, now()) {
            if said.insert(name.clone()) {
                eprintln!(
                    "nix-offload: {name} still finishes the builds the last stop left it, so \
                     it's given back once they end"
                );
            }
            continue;
        }
        eprintln!("nix-offload: {name} isn't yet seen gone, so it's given back");
        if down(settings, name) {
            let now = now();
            let (event, ran) = builder.given_back(now, settings.hours.idle);
            let _ = journal::note(&settings.journal, now, event, name, ran);
        } else {
            still += 1;
        }
    }
    still
}

/// How long a helper call has to end once a stop has told it to, before
/// it's killed.
const GRACE: Duration = Duration::from_secs(2);

/// Run the helper with `arguments` to its end and give what it printed,
/// unless a stop comes first. Told to stop, the service ends the call and
/// everything it started, and gives nothing. The call runs in a process
/// group of its own, since the unit's stop signals the service alone: on
/// 7 November a stop came while the helper deleted a builder, and its
/// cloud client, with the network going, ran on until systemd killed it all
/// 10 s later, holding the reboot. The builder goes either way, and one
/// still there is the next start's to take back.
fn call_helper(settings: &Settings, arguments: &[&str]) -> Option<Vec<u8>> {
    let mut call = Command::new(&settings.helper)
        .args(arguments)
        .stdout(Stdio::piped())
        .process_group(0)
        .spawn()
        .map_err(|error| eprintln!("nix-offload: running the helper: {error}"))
        .ok()?;
    let mut out = call.stdout.take()?;
    let said = thread::spawn(move || {
        let mut said = Vec::new();
        let _ = out.read_to_end(&mut said);
        said
    });
    loop {
        match call.try_wait() {
            Ok(Some(status)) if status.success() => return said.join().ok(),
            Ok(Some(_)) | Err(_) => return None,
            Ok(None) if ENDING.load(Ordering::SeqCst) => {
                eprintln!(
                    "nix-offload: told to stop, so the helper's {} ends",
                    arguments[0]
                );
                end_within(&mut call, GRACE);
                return None;
            }
            Ok(None) => thread::sleep(Duration::from_millis(50)),
        }
    }
}

/// Tell `call` and everything it started to end, and kill what's left
/// after `grace`.
fn end_within(call: &mut Child, grace: Duration) {
    let Ok(pid) = libc::pid_t::try_from(call.id()) else {
        return;
    };
    // SAFETY: signals to our own child's process group, which it leads.
    unsafe { libc::kill(-pid, libc::SIGTERM) };
    let told = Instant::now();
    while told.elapsed() < grace {
        if call.try_wait().is_ok_and(|status| status.is_some()) {
            return;
        }
        thread::sleep(Duration::from_millis(50));
    }
    // SAFETY: as above.
    unsafe { libc::kill(-pid, libc::SIGKILL) };
    let _ = call.wait();
}

/// Say any builder under the prefix that isn't this one's, leaving it be.
fn say_others(settings: &Settings) {
    let Some(alive) = call_helper(settings, &["alive", &settings.prefix]) else {
        eprintln!("nix-offload: couldn't ask the helper what's there");
        return;
    };
    let journal = fs::read_to_string(&settings.journal).unwrap_or_default();
    let ours = journal::unreturned(&journal);
    for name in String::from_utf8_lossy(&alive).lines().map(str::trim) {
        if !name.is_empty() && !ours.iter().any(|known| known.name == name) {
            eprintln!("nix-offload: {name} is there after the deadline, and isn't this one's");
        }
    }
}

/// Ask the helper to give a builder back: whether it's gone.
fn down(settings: &Settings, name: &str) -> bool {
    let gone = call_helper(settings, &["down", name]).is_some();
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

/// Whether a run asked to outlive its caller still builds on `builder`,
/// which holds it lent though no ssh here reaches it, saying so. It's
/// asked only when `decide::asks_outliving` says so.
fn outlives_on(settings: &Settings, tick: Tick, builder: &Lent) -> bool {
    if !decide::asks_outliving(settings.hours, tick, builder.used, builder.ends) {
        return false;
    }
    // Where ssh keeps what it writes asking.
    let scratch = env::var_os("XDG_RUNTIME_DIR")
        .map_or_else(env::temp_dir, PathBuf::from)
        .join("nix-offload-serve");
    let outlives = builder
        .store
        .as_deref()
        .is_some_and(|store| there::outlives_on(store, &scratch, "ssh"));
    if outlives {
        eprintln!(
            "nix-offload: {} builds a run that outlives its caller, so it stays lent",
            builder.name
        );
    }
    outlives
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
    use crate::testing::executable;
    use std::sync::{PoisonError, RwLock, RwLockReadGuard};

    /// Held to stop this process, which every test hearing a helper waits
    /// out: a stop it heard would cut the helper's word short, and failed
    /// a test taking a builder back on 4 November.
    static STOPS: RwLock<()> = RwLock::new(());

    /// Held while a test hears a helper, so no stop is told meanwhile.
    fn unstopped() -> RwLockReadGuard<'static, ()> {
        STOPS.read().unwrap_or_else(PoisonError::into_inner)
    }

    #[test]
    fn the_own_vm_lanes_run_from_3_to_the_last_one_the_file_names() {
        let last: u32 = LAST_VM_LANE
            .trim()
            .parse()
            .expect("last-vm-lane.txt isn't a number");
        let dirs = own_vm_extra_dirs();
        assert_eq!(
            dirs.len(),
            usize::try_from(last - 2).unwrap(),
            "a lane went missing"
        );
        assert_eq!(dirs.first(), Some(&PathBuf::from(format!("{VM_DIR}-3"))));
        assert_eq!(
            dirs.last(),
            Some(&PathBuf::from(format!("{VM_DIR}-{last}")))
        );
        // As 22 November's 28 lanes past the second need, and the folders
        // and lenders nixos.nix and home.nix make from the same file.
        assert!(last >= 30, "fewer lanes than the machine lends: {last}");
    }

    #[test]
    fn this_program_runs_again_by_procs_name_for_it_or_by_its_own_file() {
        let here = tempfile::tempdir().unwrap();
        let proc_exe = here.path().join("exe");
        fs::write(&proc_exe, "").unwrap();
        assert_eq!(
            program(&proc_exe),
            proc_exe,
            "a system naming the running file wasn't taken at its word"
        );
        // As on a Mac, which names none.
        let named = program(&here.path().join("none"));
        assert_eq!(
            Some(named.clone()),
            env::current_exe()
                .ok()
                .and_then(|path| fs::canonicalize(path).ok()),
            "with no /proc this program runs again as {named:?}, not its own file"
        );
        assert!(named.is_file(), "{named:?} isn't a file to run");
    }

    #[test]
    fn never_ask_given_first_waits_for_no_builder_and_leaves_the_rest() {
        let words = |text: &str| text.split(' ').map(str::to_owned).collect::<Vec<_>>();
        let given = words("--never-ask --changed --keep-base");
        let (waiting, rest) = waiting_for(&given);
        assert!(!waiting.asks);
        assert_eq!(rest, words("--changed --keep-base"));
        let given = words("--changed --never-ask");
        let (waiting, rest) = waiting_for(&given);
        assert!(waiting.asks, "only first, as usage says");
        assert_eq!(rest, given);
    }

    #[test]
    fn a_stop_ends_a_rest_at_once() {
        let _alone = STOPS.write().unwrap_or_else(PoisonError::into_inner);
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
        ENDING.store(false, Ordering::SeqCst);
    }

    /// A helper whose give-back ignores a stop, as a cloud client waiting
    /// on a network that's gone can.
    const DELETES_ON: &str = r#"#!/bin/sh
trap '' TERM
[ "$1" = down ] && sleep 60
exit 0
"#;

    #[test]
    fn a_stop_ends_a_give_back_in_flight() {
        let _alone = STOPS.write().unwrap_or_else(PoisonError::into_inner);
        let folder = tempfile::tempdir().unwrap();
        let (mut settings, _) = holding(folder.path(), "");
        settings.helper = folder.path().join("deletes-on");
        executable(&settings.helper, DELETES_ON);
        hear_stops();
        thread::spawn(|| {
            thread::sleep(Duration::from_millis(300));
            // SAFETY: a signal to this process, whose handler only stores
            // to an atomic.
            unsafe { libc::kill(libc::getpid(), libc::SIGTERM) };
        });
        let began = Instant::now();
        let gone = down(&settings, "lend-5");
        ENDING.store(false, Ordering::SeqCst);
        assert!(!gone, "a give-back cut short isn't one that happened");
        assert!(
            began.elapsed() < GRACE + Duration::from_secs(1),
            "a stop took {:?} to end a give-back",
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
            heard_from(
                &mut format!(
                    "price 0.89\ninstance lend-5 us-central1-a\nstore {store}\n\
                     cloud-token ya29.x\n{line}\nlater\n"
                )
                .as_bytes()
            ),
            Heard {
                rate: Some(890),
                store: Some(store.to_owned()),
                place: Place {
                    instance: Some("lend-5 us-central1-a".to_owned()),
                    token: Some("ya29.x".to_owned()),
                },
                line: line.to_owned(),
            }
        );
        assert_eq!(
            heard_from(&mut format!("{line}\n").as_bytes()),
            Heard {
                line: line.to_owned(),
                ..Heard::default()
            }
        );
        assert_eq!(
            heard_from(&mut "price 0.89\n".as_bytes()),
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
        let helper = folder.join("helper");
        executable(&helper, HOLDS);
        fs::write(folder.join("journal.jsonl"), journal).unwrap();
        let settings = Settings {
            helper,
            prefix: "lend-".to_owned(),
            hours: Hours {
                first: 0,
                last: 80_000,
                deadline: 86_000,
                longest: None,
                idle: decide::IDLE,
            },
            queue: folder.join("queue.json"),
            standing: None,
            beside: None,
            journal: folder.join("journal.jsonl"),
            dir: folder.to_path_buf(),
            only_there: false,
            warm: Vec::new(),
            outlasting: None,
            telling: None,
            freeing: None,
        };
        (settings, machines_in(folder))
    }

    /// The files a lender in `folder` names its builders in.
    fn machines_in(folder: &Path) -> Machines {
        let file = |name: &str| folder.join(name);
        Machines::new(
            file("machines"),
            file("with-standing"),
            file("store"),
            file("lending"),
            Vec::new(),
        )
    }

    #[test]
    fn a_start_takes_back_what_the_last_stop_left() {
        let _heard = unstopped();
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

    /// A helper that holds `lend-5` as [`HOLDS`] does, first keeping what
    /// `lending` said while it was asked.
    const HOLDS_SEEING: &str = r#"#!/bin/sh
here=$(dirname "$0")
if [ "$1 $2" = "hold lend-5" ]; then
  cat "$here/lending" > "$here/seen"
  echo "store ssh-ng://far?ssh-key=/k"
  echo "ssh-ng://far x86_64-linux /k 8 1 big-parallel - -"
  exec sleep 60
fi
exit 1
"#;

    #[test]
    fn a_start_names_the_builder_it_takes_back_while_it_takes_it() {
        // On 13 November a switch restarted both lenders, and taking back
        // what the stop left took 96 s. A build waiting on a lender gives
        // up once `lending` has named nothing for 30 s, so 22 runs ended
        // unsent though a builder came.
        let _heard = unstopped();
        let folder = tempfile::tempdir().unwrap();
        let (settings, mut machines) = holding(folder.path(), LEFT);
        executable(&settings.helper, HOLDS_SEEING);
        let mut lent = adopt(&settings, &mut machines, 3600).expect("taken back");
        let read = |name: &str| fs::read_to_string(folder.path().join(name)).unwrap_or_default();
        assert_eq!(
            read("seen"),
            "lend-5\n",
            "a build waiting while it was taken back heard nothing was coming"
        );
        assert_eq!(read("lending"), "", "it was still named once it stood");
        end(&mut lent.helper);
    }

    /// A helper that holds `lend-5`, saying where it is, and a fresh
    /// token after its line.
    const PLACES: &str = r#"#!/bin/sh
echo "instance lend-5 us-central1-a"
echo "cloud-token first"
echo "ssh-ng://far x86_64-linux /k 8 1 big-parallel - -"
echo "cloud-token fresh"
exec sleep 60
"#;

    #[test]
    fn the_lender_says_where_its_builder_is_and_takes_each_fresh_token() {
        let _heard = unstopped();
        let folder = tempfile::tempdir().unwrap();
        let journal = [
            r#"{"at":5,"event":"asked","name":"lend-5"}"#,
            r#"{"at":9,"event":"ended","name":"lend-5","seconds":4}"#,
        ]
        .join("\n")
            + "\n";
        let (settings, machines) = holding(folder.path(), &journal);
        executable(&settings.helper, PLACES);
        let dir = folder.path();
        let tokens = dir.join("tokens");
        fs::create_dir(&tokens).unwrap();
        let mut machines =
            machines.in_cloud(cloud::Cloud::in_dir(dir, &tokens, &dir.join("no-groups")));
        let mut lent = adopt(&settings, &mut machines, 3600).expect("taken back");
        let read = |name: &str| fs::read_to_string(dir.join(name)).unwrap_or_default();
        let lender = dir.file_name().unwrap().to_str().unwrap();
        let token = || fs::read_to_string(tokens.join(lender)).unwrap_or_default();
        assert_eq!(
            read("instance"),
            "lend-5 us-central1-a\n",
            "where it is, before Nix is told of it"
        );
        let asked = Instant::now();
        while placed(&lent.place).token.as_deref() != Some("fresh")
            && asked.elapsed() < Duration::from_secs(5)
        {
            thread::sleep(Duration::from_millis(20));
        }
        named(dir, &mut machines, Some(&lent), None);
        assert_eq!(token(), "fresh\n", "the token it said last");
        named(dir, &mut machines, None, None);
        assert_eq!(
            (read("instance"), token()),
            (String::new(), String::new()),
            "nothing lent, nothing placed"
        );
        end(&mut lent.helper);
    }

    /// A helper whose `hold lend-5` hands over when told to, ending as
    /// its relay would once its last connection closed, and whose `up`
    /// stands `next`, or nothing for `lend-9`.
    const HANDS_OVER: &str = r#"#!/bin/sh
case "$1 $2" in
"hold lend-5")
  trap 'exit 0' USR1
  echo "ssh-ng://far x86_64-linux /k 8 1 big-parallel - -"
  sleep 60 &
  wait
  exit 3
  ;;
"up lend-9") exit 1 ;;
up*)
  echo "store ssh-ng://next?ssh-key=/k"
  echo "ssh-ng://next x86_64-linux /k 8 1 big-parallel - -"
  exec sleep 60
  ;;
esac
exit 1
"#;

    /// How a helper ended, waiting at most 5 s; none while it runs on.
    fn ended(helper: &mut Child) -> Option<std::process::ExitStatus> {
        let ending = Instant::now() + Duration::from_secs(5);
        while Instant::now() < ending {
            if let Some(status) = helper.try_wait().unwrap() {
                return Some(status);
            }
            thread::sleep(Duration::from_millis(20));
        }
        None
    }

    #[test]
    fn a_successor_takes_nixs_line_and_the_old_helper_is_told_to_hand_over() {
        let _heard = unstopped();
        let folder = tempfile::tempdir().unwrap();
        let journal = r#"{"at":5,"event":"asked","name":"lend-5"}
{"at":9,"event":"ended","name":"lend-5","seconds":4}
"#;
        let (settings, mut machines) = holding(folder.path(), journal);
        executable(&settings.helper, HANDS_OVER);
        let old = adopt(&settings, &mut machines, 3600).expect("taken back");
        let (mut successor, left) = hand_over(&settings, &mut machines, old, 20, 12 * 3600);
        let mut left = left.expect("no successor stood");
        assert_eq!(
            (successor.name.as_str(), successor.host.as_str()),
            ("lend-20", "next")
        );
        assert_eq!(successor.ends, 20 + 12 * 3600);
        let line = fs::read_to_string(folder.path().join("machines")).unwrap();
        assert!(line.starts_with("ssh-ng://next "), "{line}");
        assert!(!line.contains("ssh-ng://far "), "{line}");
        let status = ended(&mut left.builder.helper).expect("the old helper ran on");
        assert_eq!(status.code(), Some(0), "it wasn't told to hand over");
        assert!(!left.outliving);
        end(&mut successor.helper);
    }

    /// A helper whose `hold lend-5` keeps in `seen` what `machines` said
    /// as it was asked, and in `at-hand-over` what it said once told to
    /// hand over, refusing while `refuse` is beside it.
    const HANDS_OVER_SEEING: &str = r#"#!/bin/sh
here=$(dirname "$0")
[ "$1 $2" = "hold lend-5" ] || exit 1
[ -e "$here/refuse" ] && exit 1
cat "$here/machines" > "$here/seen" 2>/dev/null
trap 'cat "$here/machines" > "$here/at-hand-over" 2>/dev/null; exit 0' USR1
echo "ssh-ng://far x86_64-linux /k 8 1 big-parallel - -"
sleep 60 &
wait
exit 3
"#;

    /// How the helper `pid`, our own child, ended, once it has.
    fn reaped(pid: u32) -> i32 {
        let pid = libc::pid_t::try_from(pid).unwrap();
        let mut status = 0;
        // SAFETY: waiting on our own child, which nothing else reaps.
        let reaped = unsafe { libc::waitpid(pid, &raw mut status, 0) };
        assert_eq!(reaped, pid, "it wasn't ours to reap");
        status
    }

    // Lending, and so handing a builder over, is Linux's alone, as are the
    // two after this.
    #[cfg(target_os = "linux")]
    #[test]
    fn a_builder_handed_over_as_the_service_ends_is_named_until_the_next_start_holds_it() {
        // On 13 November a switch restarted both lenders while builds ran
        // on their builders. Each stop took its line out and closed its way
        // to new connections, and taking the builder back took 23 s, so for
        // that long root's Nix named no lent builder and built here.
        let _heard = unstopped();
        let folder = tempfile::tempdir().unwrap();
        let (settings, mut machines) = holding(folder.path(), LEFT);
        executable(&settings.helper, HANDS_OVER_SEEING);
        let old = adopt(&settings, &mut machines, 3600).expect("taken back");
        let pid = old.helper.id();
        left_as_it_ends(&settings, &mut machines, old, true, false);
        let read = |name: &str| fs::read_to_string(folder.path().join(name)).unwrap_or_default();
        assert!(
            read("machines").starts_with("ssh-ng://far "),
            "its line went with the stop: {:?}",
            read("machines")
        );
        assert_eq!(
            read("at-hand-over"),
            "",
            "the stop told its way to hand over"
        );
        let mut next = machines_in(folder.path());
        let mut lent = start(&settings, &mut next, 3600).expect("taken back");
        assert!(
            read("seen").starts_with("ssh-ng://far "),
            "it was named nowhere while it was taken back: {:?}",
            read("seen")
        );
        let status = reaped(pid);
        assert!(
            libc::WIFEXITED(status) && libc::WEXITSTATUS(status) == 0,
            "the old way wasn't told to hand over once the start held it: {status}"
        );
        assert!(
            read("at-hand-over").starts_with("ssh-ng://far "),
            "it was named nowhere as its old way handed over"
        );
        assert!(
            read("machines").starts_with("ssh-ng://far "),
            "it was named nowhere once taken back"
        );
        end(&mut lent.helper);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn a_start_that_cant_take_back_what_was_handed_over_names_it_nowhere_first() {
        let _heard = unstopped();
        let folder = tempfile::tempdir().unwrap();
        let (settings, mut machines) = holding(folder.path(), LEFT);
        executable(&settings.helper, HANDS_OVER_SEEING);
        let old = adopt(&settings, &mut machines, 3600).expect("taken back");
        let pid = old.helper.id();
        left_as_it_ends(&settings, &mut machines, old, true, false);
        fs::write(folder.path().join("refuse"), "").unwrap();
        let mut next = machines_in(folder.path());
        assert!(
            start(&settings, &mut next, 3600).is_none(),
            "a refused one was taken back"
        );
        let status = reaped(pid);
        assert!(
            libc::WIFEXITED(status) && libc::WEXITSTATUS(status) == 0,
            "the old way wasn't told to hand over: {status}"
        );
        let read = |name: &str| fs::read_to_string(folder.path().join(name)).unwrap_or_default();
        assert_eq!(
            read("at-hand-over"),
            "",
            "it was still named as its old way took no new connection"
        );
        assert_eq!(read("machines"), "", "one not taken back was still named");
    }

    #[test]
    fn a_builder_handed_over_as_the_service_ends_keeps_the_builds_on_its_way() {
        let _heard = unstopped();
        let folder = tempfile::tempdir().unwrap();
        let journal = r#"{"at":5,"event":"asked","name":"lend-5"}
{"at":9,"event":"ended","name":"lend-5","seconds":4}
"#;
        let (mut settings, mut machines) = holding(folder.path(), journal);
        executable(&settings.helper, HANDS_OVER);
        let outlasting = folder.path().join("outlasting");
        executable(
            &outlasting,
            "#!/bin/sh\necho \"$@\" > \"$(dirname \"$0\")/through\"\nexec \"$@\"\n",
        );
        settings.outlasting = Some(outlasting);
        let old = adopt(&settings, &mut machines, 3600).expect("taken back");
        let through = fs::read_to_string(folder.path().join("through")).unwrap();
        assert!(through.ends_with("hold lend-5\n"), "{through}");
        let pid = old.helper.id();
        left_as_it_ends(&settings, &mut machines, old, true, false);
        // Told to end, the stand-in would die of the signal.
        assert!(
            finishing::read(folder.path(), "lend-5").is_some_and(|left| left.accepting),
            "its way wasn't left taking connections"
        );
        // SAFETY: our own child, ended here, as the next start would tell it.
        unsafe { libc::kill(libc::pid_t::try_from(pid).unwrap(), libc::SIGUSR1) };
        let status = reaped(pid);
        assert!(
            libc::WIFEXITED(status) && libc::WEXITSTATUS(status) == 0,
            "its way ended with the service: {status}"
        );
        let journal = fs::read_to_string(&settings.journal).unwrap();
        assert!(
            journal
                .lines()
                .last()
                .unwrap()
                .contains(r#""event":"ended","name":"lend-5""#),
            "{journal}"
        );
    }

    #[test]
    fn a_builder_left_as_the_service_ends_ends_its_helper() {
        let _heard = unstopped();
        let folder = tempfile::tempdir().unwrap();
        let journal = r#"{"at":5,"event":"asked","name":"lend-5"}
{"at":9,"event":"ended","name":"lend-5","seconds":4}
"#;
        let (settings, mut machines) = holding(folder.path(), journal);
        executable(&settings.helper, HANDS_OVER);
        let old = adopt(&settings, &mut machines, 3600).expect("taken back");
        let pid = libc::pid_t::try_from(old.helper.id()).unwrap();
        left_as_it_ends(&settings, &mut machines, old, false, false);
        // SAFETY: a probe of a pid we started, reaped by `end`.
        assert_ne!(unsafe { libc::kill(pid, 0) }, 0, "its helper runs on");
    }

    #[test]
    fn with_no_successor_the_old_builder_keeps_on_as_it_was() {
        let _heard = unstopped();
        let folder = tempfile::tempdir().unwrap();
        let journal = r#"{"at":5,"event":"asked","name":"lend-5"}
{"at":9,"event":"ended","name":"lend-5","seconds":4}
"#;
        let (settings, mut machines) = holding(folder.path(), journal);
        executable(&settings.helper, HANDS_OVER);
        let old = adopt(&settings, &mut machines, 3600).expect("taken back");
        let (mut old, left) = hand_over(&settings, &mut machines, old, 9, 12 * 3600);
        assert!(left.is_none(), "a successor stood");
        assert_eq!(old.name, "lend-5");
        assert!(runs(&mut old.helper), "the old helper ended");
        let line = fs::read_to_string(folder.path().join("machines")).unwrap();
        assert!(line.starts_with("ssh-ng://far "), "{line}");
        end(&mut old.helper);
    }

    /// A helper that holds `lend-5` behind a relay, saying its pid in
    /// `relay` beside it, and says each give-back in `given-back`.
    const RELAYS: &str = r#"#!/bin/sh
here=$(dirname "$0")
case "$1 $2" in
"hold lend-5")
  echo "store ssh-ng://far?ssh-key=/k"
  echo "ssh-ng://far x86_64-linux /k 8 1 big-parallel - -"
  sleep 60 &
  echo $! > "$here/relay"
  wait
  ;;
"down lend-5") echo lend-5 >> "$here/given-back"; exit 0 ;;
esac
exit 1
"#;

    /// A `nix` whose copies up all land, for proofs whose ssh builds.
    const CARRIES: &str = r#"#!/bin/sh
case "$*" in
"store add"*) echo /nix/store/aaaa-nix-offload-carried ;;
esac
exit 0
"#;

    /// The `nix` beside `ssh` that its proofs copy up with: one whose
    /// copies land, unless the test wrote its own.
    fn nix_beside(ssh: &str) -> String {
        let nix = Path::new(ssh).with_file_name("nix");
        if !nix.exists() {
            executable(&nix, CARRIES);
        }
        nix.display().to_string()
    }

    /// Whether `keeper` says the builder is left out once the proof due at
    /// `at` has ended, waiting at most 5 s.
    fn proven_at(
        settings: &Settings,
        keeper: &mut Option<Keeper>,
        builder: &Lent,
        at: Instant,
        ssh: &str,
    ) -> bool {
        let nix = nix_beside(ssh);
        let heard = |keeper: &Option<Keeper>| {
            keeper.as_ref().map(|keeper| {
                let health = &keeper.health;
                (health.failures, health.last_success, health.last_failure)
            })
        };
        let before = heard(keeper);
        let ending = Instant::now() + Duration::from_secs(5);
        loop {
            let dropped = kept(settings, keeper, builder, at, [ssh, &nix]);
            let now = heard(keeper);
            if now != before && now != Some((0, None, None)) {
                return dropped;
            }
            assert!(Instant::now() < ending, "no proof ended");
            thread::sleep(Duration::from_millis(10));
        }
    }

    // Only a lender proves its builders, and only Linux runs one.
    #[cfg(target_os = "linux")]
    #[test]
    fn a_lent_builder_failing_two_proofs_in_a_row_is_named_nowhere_and_given_back() {
        let _heard = unstopped();
        let folder = tempfile::tempdir().unwrap();
        let journal = r#"{"at":5,"event":"asked","name":"lend-5"}
{"at":9,"event":"ended","name":"lend-5","seconds":4}
"#;
        let (settings, machines) = holding(folder.path(), journal);
        executable(&settings.helper, RELAYS);
        let mut machines = machines.for_anything(folder.path().join("machines-for-anything"));
        let builder = adopt(&settings, &mut machines, 3600).expect("taken back");
        let read = |name: &str| fs::read_to_string(folder.path().join(name)).unwrap_or_default();
        assert!(read("machines").starts_with("ssh-ng://far "));
        assert!(read("machines-for-anything").starts_with("ssh-ng://far "));
        let ending = Instant::now() + Duration::from_secs(5);
        while read("relay").is_empty() && Instant::now() < ending {
            thread::sleep(Duration::from_millis(10));
        }
        let relay: i32 = read("relay").trim().parse().expect("the relay's pid");
        // ssh as a builder out of memory answers it: never.
        let ssh = folder.path().join("ssh");
        executable(&ssh, "#!/bin/sh\nexit 255\n");
        let ssh = ssh.display().to_string();
        let mut keeper = None;
        let began = Instant::now();
        assert!(
            !proven_at(&settings, &mut keeper, &builder, began, &ssh),
            "one failure is a blip"
        );
        assert!(proven_at(
            &settings,
            &mut keeper,
            &builder,
            began + proof::EVERY,
            &ssh
        ));
        give_back_unanswered(&settings, &mut machines, builder);
        assert_eq!(read("machines"), "", "no build goes there");
        assert_eq!(read("with-standing"), "");
        assert_eq!(read("machines-for-anything"), "");
        assert_eq!(read("store"), "");
        assert!(proof::ends(relay), "its relay runs on");
        assert_eq!(read("given-back"), "lend-5\n");
        let journal = fs::read_to_string(&settings.journal).unwrap();
        assert!(
            journal
                .lines()
                .last()
                .unwrap()
                .contains("\"event\":\"lost\""),
            "{journal}"
        );
        assert_eq!(
            journal::unreturned(&journal),
            Vec::<journal::Unreturned>::new()
        );
        // Another is asked for at once, as after any builder lost.
        let hold = decide::next_after(Event::Lost, true, 0, u64::MAX, false, 100);
        assert_eq!(hold.until, 100);
        let health = keeper.as_ref().map(|keeper| &keeper.health);
        proof::write(folder.path(), health, None, 100);
        let said: serde_json::Value =
            serde_json::from_str(&read(proof::HEALTH)).expect("its health");
        assert_eq!(said["builder"], "lend-5");
        assert_eq!(said["state"], "dropped");
        assert_eq!(said["consecutiveFailures"], 2);
    }

    // Only a lender proves its builders, and only Linux runs one.
    #[cfg(target_os = "linux")]
    #[test]
    fn a_lent_builder_that_answers_its_proofs_stays() {
        let _heard = unstopped();
        let folder = tempfile::tempdir().unwrap();
        let journal = r#"{"at":5,"event":"asked","name":"lend-5"}
{"at":9,"event":"ended","name":"lend-5","seconds":4}
"#;
        let (settings, mut machines) = holding(folder.path(), journal);
        executable(&settings.helper, RELAYS);
        let mut builder = adopt(&settings, &mut machines, 3600).expect("taken back");
        let ssh = folder.path().join("ssh");
        executable(
            &ssh,
            "#!/bin/sh\n[ -f \"$(dirname \"$0\")/answers\" ] || exit 255\n",
        );
        let ssh = ssh.display().to_string();
        let mut keeper = None;
        let began = Instant::now();
        assert!(!proven_at(&settings, &mut keeper, &builder, began, &ssh));
        fs::write(folder.path().join("answers"), "").unwrap();
        assert!(!proven_at(
            &settings,
            &mut keeper,
            &builder,
            began + proof::EVERY,
            &ssh
        ));
        let health = &keeper.as_ref().expect("kept").health;
        assert_eq!(
            health.state(),
            State::Passing,
            "a failure between passes is forgotten"
        );
        assert!(runs(&mut builder.helper));
        end(&mut builder.helper);
    }

    // On 13 November both lent builders built their proofs while the
    // uplink beside them was too full to take a copy. One that builds but
    // can't carry is named nowhere, but kept warm rather than given back
    // to be lent cold over the same way.
    // Only a lender proves its builders, and only Linux runs one.
    #[cfg(target_os = "linux")]
    #[test]
    fn a_lent_builder_that_builds_but_cant_carry_a_copy_is_named_nowhere_but_kept() {
        let _heard = unstopped();
        let folder = tempfile::tempdir().unwrap();
        let journal = r#"{"at":5,"event":"asked","name":"lend-5"}
{"at":9,"event":"ended","name":"lend-5","seconds":4}
"#;
        let (settings, mut machines) = holding(folder.path(), journal);
        executable(&settings.helper, RELAYS);
        let mut builder = adopt(&settings, &mut machines, 3600).expect("taken back");
        let ssh = folder.path().join("ssh");
        executable(&ssh, "#!/bin/sh\nexit 0\n");
        let nix = folder.path().join("nix");
        executable(
            &nix,
            "#!/bin/sh\ncase \"$*\" in\n\"store add\"*) echo /nix/store/aaaa-nix-offload-carried ;;\n\
             *) exit 1 ;;\nesac\n",
        );
        let (ssh, nix) = (ssh.display().to_string(), nix.display().to_string());
        let mut keeper = None;
        let began = Instant::now();
        assert!(
            !proven_at(&settings, &mut keeper, &builder, began, &ssh),
            "one failure is a blip"
        );
        assert!(
            proven_at(&settings, &mut keeper, &builder, began + proof::EVERY, &ssh),
            "two in a row name it nowhere"
        );
        let health = &keeper.as_ref().expect("kept").health;
        assert!(health.stalled, "it built, so it's stalled: {health:?}");
        assert_eq!(health.state(), State::Dropped);
        let mut said = SaidKept::default();
        assert!(
            !unanswered(
                &settings,
                &mut keeper,
                &builder,
                false,
                began + proof::EVERY,
                &mut said,
                [&ssh, &nix],
            ),
            "no build reaches it, but it isn't lost"
        );
        assert!(
            said.stalled && !said.reached,
            "it says it's kept as one that can't carry"
        );
        assert!(runs(&mut builder.helper), "its helper still runs");
        end(&mut builder.helper);
    }

    #[test]
    fn a_builder_lent_anew_doesnt_take_the_last_ones_missed_answers() {
        let mut said = SaidKept::default();
        said.of("lend-5");
        said.silent = decide::SILENT_WHILE_REACHED;
        said.busy = true;
        said.of("lend-5");
        assert_eq!(
            said.silent,
            decide::SILENT_WHILE_REACHED,
            "the same builder keeps it"
        );
        said.of("lend-6");
        assert_eq!(said.silent, 0, "the next builder starts at none");
        assert!(!said.busy, "nor is it said to be busy");
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

    /// A helper that won't take `lend-5` back, as one refusing builders
    /// made before a change to how they're reached does, and says each
    /// give-back in `given-back`.
    const REFUSES: &str = r#"#!/bin/sh
here=$(dirname "$0")
case "$1 $2" in
"hold lend-5") echo "lend-5 was made before hook, so it's given back" >&2; exit 1 ;;
"down lend-5") echo lend-5 >> "$here/given-back"; exit 0 ;;
esac
exit 1
"#;

    const LEFT: &str = r#"{"at":5,"event":"asked","name":"lend-5"}
{"at":9,"event":"ended","name":"lend-5","seconds":4}
"#;

    #[cfg(target_os = "linux")]
    #[test]
    fn a_builder_a_start_cant_take_back_is_kept_while_its_way_carries_builds() {
        let _heard = unstopped();
        let folder = tempfile::tempdir().unwrap();
        let (settings, mut machines) = holding(folder.path(), LEFT);
        executable(&settings.helper, REFUSES);
        // The way the last stop handed over, still carrying a build.
        let mut way = Command::new("sh")
            .args(["-c", "sleep 60; :", "hold", "lend-5"])
            .spawn()
            .unwrap();
        let left = finishing::Finishing {
            helper: Some(way.id()),
            outliving: false,
            ends: now() + 3600,
            accepting: false,
        };
        finishing::note(folder.path(), "lend-5", &left).unwrap();
        assert!(
            adopt(&settings, &mut machines, 3600).is_none(),
            "a builder the helper refused was taken back"
        );
        let read = |name: &str| fs::read_to_string(folder.path().join(name)).unwrap_or_default();
        assert_eq!(
            read("lending"),
            "",
            "one it couldn't take back was still named as coming"
        );
        let mut said = BTreeSet::new();
        assert_eq!(
            give_back_unreturned(&settings, None, &[], &mut said),
            0,
            "one finishing its builds held up lending another"
        );
        assert_eq!(read("given-back"), "", "given back with a build on its way");
        way.kill().unwrap();
        way.wait().unwrap();
        assert_eq!(give_back_unreturned(&settings, None, &[], &mut said), 0);
        assert_eq!(read("given-back"), "lend-5\n", "kept once its builds ended");
        assert_eq!(
            finishing::read(folder.path(), "lend-5"),
            None,
            "its note outlived it"
        );
    }

    #[test]
    fn a_builder_left_holding_a_run_outliving_its_caller_is_kept_until_its_time_is_up() {
        let _heard = unstopped();
        let folder = tempfile::tempdir().unwrap();
        let (settings, _) = holding(folder.path(), LEFT);
        executable(&settings.helper, REFUSES);
        let read = || fs::read_to_string(folder.path().join("given-back")).unwrap_or_default();
        let left = |ends| finishing::Finishing {
            helper: None,
            outliving: true,
            ends,
            accepting: false,
        };
        let mut said = BTreeSet::new();
        finishing::note(folder.path(), "lend-5", &left(now() + 3600)).unwrap();
        give_back_unreturned(&settings, None, &[], &mut said);
        assert_eq!(
            read(),
            "",
            "given back with a run outliving its caller there"
        );
        finishing::note(folder.path(), "lend-5", &left(now() - 1)).unwrap();
        give_back_unreturned(&settings, None, &[], &mut said);
        assert_eq!(read(), "lend-5\n", "kept past its time");
    }

    #[test]
    fn a_builder_handed_over_as_the_service_ends_is_noted_for_the_next_start() {
        let _heard = unstopped();
        let folder = tempfile::tempdir().unwrap();
        let (settings, mut machines) = holding(folder.path(), LEFT);
        executable(&settings.helper, HANDS_OVER);
        let old = adopt(&settings, &mut machines, 3600).expect("taken back");
        let (pid, ends) = (old.helper.id(), old.ends);
        left_as_it_ends(&settings, &mut machines, old, true, true);
        assert_eq!(
            finishing::read(folder.path(), "lend-5"),
            Some(finishing::Finishing {
                helper: Some(pid),
                outliving: true,
                ends,
                accepting: true,
            })
        );
        let old = adopt(&settings, &mut machines, 3600).expect("taken back");
        finishing::forget(folder.path(), "lend-5");
        left_as_it_ends(&settings, &mut machines, old, false, false);
        assert_eq!(
            finishing::read(folder.path(), "lend-5"),
            None,
            "noted with nothing left to finish"
        );
    }

    #[test]
    fn builders_leaving_as_a_restart_hands_over_keep_their_ways() {
        let _heard = unstopped();
        let folder = tempfile::tempdir().unwrap();
        let (settings, mut machines) = holding(folder.path(), LEFT);
        executable(&settings.helper, HANDS_OVER);
        let old = adopt(&settings, &mut machines, 3600).expect("taken back");
        let pid = old.helper.id();
        let leaving = vec![Leaving {
            builder: old,
            outliving: false,
        }];
        left_leaving(&settings, leaving, true);
        let noted = finishing::read(folder.path(), "lend-5").expect("not noted");
        assert_eq!(noted.helper, Some(pid));
        assert!(
            !noted.accepting,
            "one already handed to a successor takes connections"
        );
        let pid = libc::pid_t::try_from(pid).unwrap();
        // SAFETY: a probe of a pid we started and haven't reaped.
        assert_eq!(unsafe { libc::kill(pid, 0) }, 0, "its way was ended");
        // SAFETY: our own child, ended and reaped here.
        unsafe { libc::kill(pid, libc::SIGKILL) };
        let mut status = 0;
        // SAFETY: waiting on our own child, which nothing else reaps.
        unsafe { libc::waitpid(pid, &raw mut status, 0) };
    }
}
