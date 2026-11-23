//! cadence watches the delivery cycle from outside every session, so it
//! stays watched when the session keeping it fast dies, parks or loses its
//! model. Each `cadence tick`, run by a timer, reads the cycle's raw
//! records (core's merges, every switch update-to-main made,
//! the runs nix-offload sent, the net's round, the machine's pressure and
//! memory), writes the last hour's card to `card.txt` in its state folder,
//! and asks the watcher, by its mesh address, a question for each target
//! newly missed. A card overdue it sends the orchestrator above the
//! watcher itself. When the watcher leaves a question unanswered for
//! twenty minutes, or is gone for ten, it asks that orchestrator instead,
//! once. Given no watcher, it writes the card alone.
//!
//! Given `--restart`, each tick first revives, before anything that needs
//! the network: every orchestrator (each `--revive`, and any that
//! hears its own orchestration's group or that others name as theirs) and
//! every session of one whose pi died is started again on its own log, its window put back, and the
//! orchestrator told (revive.rs says which are left down).
//!
//! Given an orchestrator, each tick also wakes any session of an orchestrator's
//! idle twenty minutes with a next step, and tells the orchestrator of one
//! idle two hours, once for each idle spell (idle.rs), and
//! tells it and the unit's owner (by `--owner`, else the watcher) of any of
//! the account's units restarting in a loop, failed, or a daemon whose
//! socket reads signed out or doesn't answer, once for each spell
//! (units.rs). `cadence units [--at SECONDS]` says what that watch reads
//! and would tell, sending nothing; at a moment past, from the journal.
//! A machine gone down while it holds work is told Joel by a notice, and
//! the orchestrator by a finding (watch/down.rs; its journal and down.json).
//!
//! `cadence day`, run each evening by a timer of its own, reads the day
//! just ended against the days before it and sends the orchestrator its note
//! (daily.rs). Each finding goes in its ledger and to the owner its key's
//! prefix names, or to the watcher (findings.rs).
//!
//! `cadence axes`, typed by Joel, shows the ten axes he holds the machine
//! to, each against its bar with its owner, and `cadence axes <axis>`
//! steps into one (axes.rs). `cadence axes --served WORDS` prints them as
//! the launcher's Spec rows, for cadence-spec to serve (axes/spec.rs).
//!
//! `cadence review`, run each Friday evening by a timer, sends the week's
//! noise and blind spots to the addresses it's given (axes/review.rs).
//!
//! `cadence --help`, and each command's, say what each takes
//! (commands.rs).

mod asking;
mod axes;
mod builders;
mod card;
mod commands;
mod daily;
mod day;
mod elsewhere;
mod findings;
mod gate;
mod health;
mod holders;
mod hour;
mod idle;
mod judge;
mod machine;
mod mesh;
mod net;
mod owe;
mod passed;
mod phases;
mod records;
mod reverted;
mod revive;
mod speed;
mod start;
mod stats;
mod tracking;
mod units;
mod unrun;
mod watch;

use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fmt::Display;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::time::{SystemTime, UNIX_EPOCH};

use jiff::Timestamp;
use jiff::tz::TimeZone;
use terminal_kit::{Context, Exit, Failure};

use asking::{State, To};
use mesh::{Mesh, Message};
use net::Round;
use watch::{Landing, Snapshot, down};

/// The address cadence's questions come from.
const FROM: &str = "CADENCE";
/// Where a question of cadence's says its answer goes. No peer goes by
/// CADENCE, so the mesh tells whoever answers that it reaches no one, and
/// on 19 November 5FWIUS sent its answer to a wake to the cell that owns
/// cadence instead. The answer is still written, and cadence reads each
/// by its re from what the session sends, whoever it goes to: an alarm's
/// ends its watch, and a wake's counts as answered in collaboration.
const ANSWERS_GO: &str = "Send it to CADENCE: the mesh says that reaches no peer, but \
                          cadence reads it from what you send.";
/// Core's repository, which cadence keeps a bare clone of to read merges.
const CORE: &str = "https://github.com/Jitsusama/core.nix";
/// How far back the records are read, so the hour and the six hours a
/// merge may wait in are both covered.
const DAY: i64 = 86_400;

struct Args {
    watcher: Option<String>,
    orchestrator: Option<String>,
    restart: Option<String>,
    orchestrators: Vec<String>,
    skip: Vec<String>,
    owners: BTreeMap<String, String>,
    /// The realms whose other machines' updates are read.
    realms: Vec<String>,
}

fn main() -> ExitCode {
    terminal_kit::main(&commands::help(), env!("CARGO_PKG_VERSION"), asked)
}

/// Runs the command its first word names; the kit has refused any other.
fn asked(context: &Context) -> Result<(), Failure> {
    let Some((command, rest)) = context.args.split_first() else {
        return Err(Failure::new(Exit::Usage, "cadence takes a command")
            .because("it does only what one asks")
            .try_this("cadence --help"));
    };
    let rest = rest.iter().cloned();
    match command.as_str() {
        "tick" => run(&ticking(rest)?).map_err(failed),
        "day" => day(rest),
        "units" => units_now(rest),
        "axes" => axes::run(rest, &context.out),
        "review" => axes::review::run(rest),
        "owe" => owe::run(rest, now_ms() / 1000),
        "hour" => hour::run(rest),
        _ => Err(Failure::new(Exit::Usage, "cadence takes its command first")
            .because(format!("{command} came before it"))
            .try_this("cadence --help")),
    }
}

/// A command asked wrongly, pointing at its own help.
fn wrongly(command: &str, what: impl Into<String>) -> Failure {
    Failure::new(Exit::Usage, what).try_this(format!("cadence {command} --help"))
}

/// A command that couldn't do what it was asked, saying why.
fn failed(err: impl Display) -> Failure {
    Failure::new(Exit::Failed, err.to_string())
}

/// Reads what `cadence tick` was asked.
fn ticking(mut args: impl Iterator<Item = String>) -> Result<Args, Failure> {
    let mut parsed = Args {
        watcher: None,
        orchestrator: None,
        restart: None,
        orchestrators: Vec::new(),
        skip: Vec::new(),
        owners: BTreeMap::new(),
        realms: Vec::new(),
    };
    while let Some(flag) = args.next() {
        let value = args.next();
        match (flag.as_str(), value) {
            ("--watcher", Some(v)) => parsed.watcher = Some(v),
            ("--orchestrator", Some(v)) => parsed.orchestrator = Some(v),
            ("--restart", Some(v)) => parsed.restart = Some(v),
            ("--revive", Some(v)) => parsed.orchestrators.push(v),
            ("--skip", Some(v)) => parsed.skip.push(v),
            ("--realm", Some(v)) => parsed.realms.push(v),
            ("--owner", Some(v)) if v.contains('=') => {
                let (prefix, address) = v.split_once('=').unwrap_or_default();
                parsed.owners.insert(prefix.to_owned(), address.to_owned());
            }
            _ => {
                return Err(wrongly(
                    "tick",
                    format!("{flag} isn't a flag it takes, or has no value"),
                ));
            }
        }
    }
    Ok(parsed)
}

/// Says what the unit watch reads and would tell now, or at `--at`
/// from the journal alone, sending nothing and keeping nothing.
fn units_now(mut args: impl Iterator<Item = String>) -> Result<(), Failure> {
    let at = match (args.next().as_deref(), args.next()) {
        (None, _) => None,
        (Some("--at"), Some(v)) if v.parse::<i64>().is_ok() => v.parse().ok(),
        _ => return Err(wrongly("units", "--at takes a number of seconds")),
    };
    let now = now_ms() / 1000;
    let runtime = env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from);
    let read = units::read(at.unwrap_or(now), at.is_none(), runtime.as_deref());
    let tells = units::due(
        &read,
        &mut units::Told::default(),
        at.unwrap_or(now),
        units::last_error,
        clock,
    );
    println!(
        "{} restarts in fifteen minutes, {} units failed, {} daemons asked",
        read.restarts.len(),
        read.failed.len(),
        read.daemons.len()
    );
    for tell in &tells {
        println!("{}", tell.text);
    }
    Ok(())
}

/// Reads the day just ended and tells whom it was asked to.
fn day(mut args: impl Iterator<Item = String>) -> Result<(), Failure> {
    let mut told = daily::Told::default();
    while let Some(flag) = args.next() {
        match (flag.as_str(), args.next()) {
            ("--orchestrator", Some(v)) => told.orchestrator = Some(v),
            ("--watcher", Some(v)) => told.watcher = Some(v),
            ("--owner", Some(v)) if v.contains('=') => {
                let (prefix, address) = v.split_once('=').unwrap_or_default();
                told.owners.insert(prefix.to_owned(), address.to_owned());
            }
            _ => {
                return Err(wrongly(
                    "day",
                    format!("{flag} isn't a flag it takes, or has no value"),
                ));
            }
        }
    }
    folders()
        .and_then(|(_, state_home, folder)| {
            daily::run(&folder, &state_home, &told, now_ms() / 1000)
        })
        .map_err(failed)
}

/// The account's home, its state folder, and cadence's own in it.
fn folders() -> io::Result<(PathBuf, PathBuf, PathBuf)> {
    let home = PathBuf::from(env::var_os("HOME").ok_or_else(|| io::Error::other("no HOME"))?);
    let state_home =
        env::var_os("XDG_STATE_HOME").map_or_else(|| home.join(".local/state"), PathBuf::from);
    let folder = state_home.join("cadence");
    fs::create_dir_all(&folder)?;
    Ok((home, state_home, folder))
}

fn run(args: &Args) -> io::Result<()> {
    let (home, state_home, folder) = folders()?;
    let mesh = Mesh {
        root: state_home.join("pi/agentic-harness.pi"),
    };
    let now = now_ms() / 1000;
    let data_home =
        env::var_os("XDG_DATA_HOME").map_or_else(|| home.join(".local/share"), PathBuf::from);
    let quests = data_home.join("pi/agentic-harness.pi/quest-workflow/quests");
    if let (Some(restart), Some(orchestrator)) = (&args.restart, &args.orchestrator) {
        let reviving = start::Reviving {
            orchestrator: orchestrator.clone(),
            watcher: args.watcher.clone(),
            orchestrators: args.orchestrators.clone(),
            skip: args.skip.clone(),
            restart: restart.clone(),
            quests: quests.clone(),
        };
        let send = |to: &str, intent: &str, text: String| mesh.send(&message(to, intent, text));
        // Reviving doesn't wait on the card: a fetch that fails mustn't
        // leave a died orchestrator down.
        match start::revive(&reviving, &mesh, &folder, now, &send) {
            Ok(said) => said.iter().for_each(|line| println!("{line}")),
            Err(err) => eprintln!("cadence: reviving: {err}"),
        }
    }
    if args.orchestrator.is_some() {
        // Like reviving, this doesn't wait on the card, and a line that
        // can't be sent mustn't keep the card from being written.
        // pi's own folder, where it keeps each session's log.
        let agent = env::var_os("PI_CODING_AGENT_DIR")
            .map_or_else(|| home.join(".pi/agent"), PathBuf::from);
        if let Err(err) = idle::tell_idle(&mesh, &agent.join("sessions"), &quests, &folder, now) {
            eprintln!("cadence: telling of idle sessions: {err}");
        }
    }
    if let Some(orchestrator) = &args.orchestrator
        && let Err(err) = tell_units(&mesh, &folder, orchestrator, args, now)
    {
        eprintln!("cadence: telling of broken units: {err}");
    }
    // Read once a tick for each use, within a deadline.
    let realms = &args.realms;
    let machines = down::machines(realms);
    if let Some(to) = &args.orchestrator
        && let Err(err) = down::run(&mesh, &folder, to, &args.owners, realms, &machines, now)
    {
        eprintln!("cadence: telling of machines down: {err}");
    }
    if let Some(watcher) = &args.watcher
        && let Err(err) = tell_reverted(&mesh, &state_home.join("net"), &folder, watcher, now)
    {
        eprintln!("cadence: telling of reverts: {err}");
    }
    let mut snapshot = gather(&folder, &state_home, &mesh, args, &machines, now)?;
    let card = card::card(&snapshot, clock);
    write_whole(&folder.join("card.txt"), &card)?;

    let (Some(watcher), Some(orchestrator)) = (&args.watcher, &args.orchestrator) else {
        return Ok(());
    };
    let path = folder.join("state.json");
    let mut state: State = fs::read(&path)
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default();
    for by in state.askers().map(str::to_owned).collect::<BTreeSet<_>>() {
        let answers = mesh.sent_by(&by).into_iter().filter_map(|m| m.re).collect();
        snapshot.answered_by.insert(by, answers);
    }
    for question in asking::tick(&snapshot, &mut state) {
        let (to, by) = question.to.addressed(&snapshot, watcher, orchestrator);
        let text = match question.to {
            To::Watcher | To::Switcher | To::Holder => asked_to_answer(&question.text),
            To::Orchestrator => question.text.clone(),
        };
        let message = message(to, question.to.intent(), text);
        mesh.send(&message)?;
        if let Some(key) = &question.key {
            state.asked_as(key, &message.id, now, &question.text, by);
        }
    }
    if asking::card_due(&snapshot, &state) {
        let text = format!(
            "{card}Sent by cadence, as the session keeping the cycle fast sent none this hour; \
             tokens, sessions and the top bottleneck come with its next."
        );
        mesh.send(&message(orchestrator, "fyi", text))?;
        state.carded = Some(now);
    }
    write_whole(&path, &serde_json::to_string_pretty(&state)?)
}

/// Who switches the machine: the owner of `cadence day`'s `switch.`
/// findings, as `--owner` names it, is asked of the tick's alarms only a
/// switch answers too.
fn switcher(args: &Args) -> Option<String> {
    findings::owner_of("switch.", &args.owners)
}

/// Tells each merge's owner that the net reverted it, or that its Mac
/// checks went uncompared (unrun.rs), remembering in `reverted.json` which were told.
fn tell_reverted(
    mesh: &Mesh,
    net: &Path,
    folder: &Path,
    watcher: &str,
    now: i64,
) -> io::Result<()> {
    let path = folder.join("reverted.json");
    let mut told: reverted::Told = fs::read(&path)
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default();
    let findings = net.join("findings");
    let reverts = reverted::reverts(&findings, now - 3600);
    let unrun = unrun::unrun(&findings, now - 3600);
    let landings =
        fs::read_to_string(mesh.root.join("mesh-workflow/landings.json")).unwrap_or_default();
    let mut failed = None;
    let mut tells = reverted::due(&reverts, &landings, &mut told, now);
    tells.extend(unrun::due(&unrun, &landings, &mut told, now));
    for tell in tells {
        // The owner is told as a finding, which it acts on in its next
        // turn; the watcher is asked, since only it can find the owner.
        let sent = match &tell.to {
            Some(owner) => mesh.send(&message(owner, "finding", tell.text)),
            None => mesh.send(&message(watcher, "question", tell.text)),
        };
        if let Err(err) = sent {
            // Not told, so the next tick tells it again. The keys don't
            // overlap, since an unrun merge's starts "mac:".
            told.reverts.remove(&tell.key);
            told.unrun.remove(&tell.key);
            failed = Some(err);
        }
    }
    write_whole(&path, &serde_json::to_string_pretty(&told)?)?;
    failed.map_or(Ok(()), Err)
}

/// Tells the orchestrator, and each unit's owner or else the watcher, of
/// the units broken, remembering in `units.json` which spells were told.
fn tell_units(
    mesh: &Mesh,
    folder: &Path,
    orchestrator: &str,
    args: &Args,
    now: i64,
) -> io::Result<()> {
    let path = folder.join("units.json");
    let mut told: units::Told = fs::read(&path)
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default();
    let runtime = env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from);
    let mut read = units::read(now, true, runtime.as_deref());
    let mut failed = None;
    let known = mesh.known();
    units::leave_runs(&mut read.failed, |unit| {
        holders::owner(unit, &args.owners, &known).is_some()
    });
    for tell in units::due(&read, &mut told, now, units::last_error, clock) {
        let owner =
            holders::owner(&tell.unit, &args.owners, &known).or_else(|| args.watcher.clone());
        let mut to = vec![orchestrator.to_owned()];
        to.extend(owner.filter(|o| !o.eq_ignore_ascii_case(orchestrator)));
        // A finding: each acts on it in its next turn, and nothing waits
        // on an answer.
        for address in &to {
            if let Err(err) = mesh.send(&message(address, "finding", tell.text.clone())) {
                failed = Some(err);
            }
        }
        if failed.is_some() {
            // Not told, so the next tick tells it again.
            if let Some(spell) = told.spells.get_mut(&tell.key) {
                spell.told = false;
            }
        } else {
            println!("{} (to {})", tell.text, to.join(", "));
        }
    }
    write_whole(&path, &serde_json::to_string_pretty(&told)?)?;
    failed.map_or(Ok(()), Err)
}

/// Core's history, from a bare clone of its own kept beside the card and
/// fetched whole each tick, so it never lags a checkout's own fetching.
struct Core {
    dir: PathBuf,
}

impl Core {
    fn fetched(folder: &Path) -> io::Result<Self> {
        let dir = folder.join("core.git");
        if !dir.exists() {
            command(
                "git",
                &["clone", "--bare", "--quiet", CORE, &dir.to_string_lossy()],
            )?;
        }
        let core = Self { dir };
        fetch_or_keep(|| {
            core.git(&[
                "fetch",
                "--quiet",
                "origin",
                "+refs/heads/main:refs/heads/main",
            ])
        });
        Ok(core)
    }

    fn git(&self, args: &[&str]) -> io::Result<String> {
        let mut all = vec!["-C", self.dir.to_str().unwrap_or_default()];
        all.extend_from_slice(args);
        command("git", &all)
    }

    fn number(&self, args: &[&str]) -> Option<i64> {
        self.git(args).ok()?.trim().parse().ok()
    }

    /// Each merge to main since `since`, with the first look of the
    /// follower's that built it.
    fn landings(&self, since: i64, follower: &records::Follower) -> io::Result<Vec<Landing>> {
        let log = self.git(&[
            "log",
            "--first-parent",
            "--merges",
            &format!("--since=@{since}"),
            "--format=%H %ct %P %s",
            "main",
        ])?;
        let mut landings = Vec::new();
        for (mut merge, main, branch) in records::parse_merges(&log) {
            merge.last_commit = self.number(&["show", "-s", "--format=%ct", &branch]);
            merge.files = self
                .git(&["diff", "--name-only", &main, &merge.sha])?
                .lines()
                .map(str::to_owned)
                .collect();
            let on = follower
                .looks
                .iter()
                .find(|look| {
                    look.at >= merge.at
                        && self
                            .git(&["merge-base", "--is-ancestor", &merge.sha, &look.core])
                            .is_ok()
                })
                .cloned();
            landings.push(Landing { merge, on });
        }
        Ok(landings)
    }

    /// The net's round, read from its state, against main; none when it
    /// tests nothing and has seen main's head.
    fn round(&self, net_state: &str) -> Option<Round> {
        let (head, testing) = records::net_round(net_state)?;
        let merged = self.number(&["show", "-s", "--format=%ct", &head])?;
        let behind = self.number(&[
            "rev-list",
            "--count",
            "--first-parent",
            &format!("{head}..main"),
        ])?;
        if !testing && behind == 0 {
            return None;
        }
        Some(Round {
            head,
            merged,
            behind: usize::try_from(behind).ok()?,
            testing,
        })
    }
}

fn journal(args: &[&str]) -> String {
    let mut all = vec!["-o", "short-unix", "--no-pager"];
    all.extend_from_slice(args);
    command("journalctl", &all).unwrap_or_default()
}

fn gather(
    folder: &Path,
    state_home: &Path,
    mesh: &Mesh,
    args: &Args,
    machines: &BTreeMap<String, String>,
    now: i64,
) -> io::Result<Snapshot> {
    let core = Core::fetched(folder)?;
    let since = format!("--since=@{}", now - DAY);
    let mut follower = records::parse_follower(&journal(&[
        "--user",
        "-u",
        "update-to-main-follows",
        &since,
    ]));
    let service = journal(&["-u", "update-to-main.service", &since]);
    follower.saw(records::parse_looks(&service));
    // A switch a reboot cut short writes no done, yet its core runs.
    let booted = fs::read_link("/run/booted-system").unwrap_or_default();
    follower.looks.extend(
        records::boot_time(&fs::read_to_string("/proc/stat").unwrap_or_default())
            .and_then(|at| records::boot_look(&service, &booted.to_string_lossy(), at)),
    );
    let landings = core.landings(now - DAY, &follower)?;
    let net = state_home.join("net");
    let round = core.round(&fs::read_to_string(net.join("state.json")).unwrap_or_default());
    let held = fs::read_to_string(net.join("held.json")).unwrap_or_default();
    let budgets = watch::budgets::gather(
        std::env::var_os("XDG_RUNTIME_DIR")
            .as_deref()
            .map(Path::new),
        now,
    );
    let (available_gib, swap_gib) =
        records::memory(&fs::read_to_string("/proc/meminfo").unwrap_or_default());
    let kernel = journal(&[
        "-k",
        &format!("--since=@{}", now - 7 * DAY),
        "--grep=oom-kill:",
    ]);
    let watcher = args.watcher.as_deref().and_then(|a| mesh.presence(a));
    let sent = args
        .watcher
        .as_deref()
        .map(|a| mesh.sent_by(a))
        .unwrap_or_default();
    let orchestrator = args.orchestrator.as_deref().map(|s| format!("@{s}"));
    Ok(Snapshot {
        now,
        landings,
        follower,
        failing_elsewhere: elsewhere::failing_in(machines),
        runs: records::parse_runs(
            &fs::read_to_string(state_home.join("nix-offload/there.jsonl")).unwrap_or_default(),
            now - DAY,
        ),
        cpu_avg300: records::avg300(&fs::read_to_string("/proc/pressure/cpu").unwrap_or_default()),
        joel_cpu_avg300: records::account(
            &fs::read_to_string("/proc/self/cgroup").unwrap_or_default(),
        )
        .and_then(|account| {
            ["app.slice", "session.slice"]
                .iter()
                .filter_map(|slice| {
                    let path = format!("{account}/{slice}/cpu.pressure");
                    records::avg300(&fs::read_to_string(path).ok()?)
                })
                .reduce(f64::max)
        }),
        available_gib,
        swap_gib,
        last_oom_kill: records::last_machine_oom_kill(&kernel),
        watched: args.watcher.is_some(),
        round,
        net_hold: records::net_hold(&fs::read_to_string(net.join("hold.json")).unwrap_or_default()),
        net_held: records::net_held(&held),
        net_held_since: watch::held::read(&held),
        budgets_unwatched: budgets.as_ref().err().cloned(),
        asleep: watch::asleep::now(),
        budgets: budgets.unwrap_or_default(),
        net_hung: crate::net::hung(&net.join("findings"), now - 3600),
        watcher_alive: mesh::alive(watcher.as_ref(), |pid| {
            Path::new(&format!("/proc/{pid}")).exists()
        }),
        watcher_session: watcher.map(|w| w.session),
        last_card: sent
            .iter()
            .filter(|m| {
                orchestrator.as_ref().is_some_and(|s| m.to.contains(s))
                    && m.text.starts_with("Card ")
            })
            .filter_map(|m| mesh::stamped_at(&m.id))
            .max()
            .map(|ms| ms / 1000),
        answered: sent.into_iter().filter_map(|m| m.re).collect(),
        switcher: switcher(args),
        answered_by: BTreeMap::new(),
    })
}

/// An alarm as the watcher or the switcher is asked it. An answer ends
/// cadence's watch, so it's asked for once handled; a switch outlasting
/// the twenty minutes says why it isn't yet.
fn asked_to_answer(alarm: &str) -> String {
    format!(
        "{alarm} Answer with re set to this question once handled, or to say why it can't be \
         yet: cadence looks for that every five minutes. {ANSWERS_GO}"
    )
}

fn message(to: &str, intent: &str, text: String) -> Message {
    let ms = now_ms();
    let pid = std::process::id();
    // Apart from another process's ids by its pid and the moment's
    // nanoseconds, as the harness's random tail keeps them apart.
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.subsec_nanos());
    let tail = nanos ^ pid.rotate_left(16);
    Message {
        v: 1,
        id: format!("{ms}-{FROM}-{pid}-{tail:08x}"),
        at: Timestamp::from_millisecond(ms).map_or_else(|_| String::new(), |t| t.to_string()),
        from: FROM.to_owned(),
        to: vec![format!("@{to}")],
        intent: intent.to_owned(),
        text,
        pid,
        re: None,
    }
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|d| i64::try_from(d.as_millis()).ok())
        .unwrap_or_default()
}

fn clock(at: i64) -> String {
    Timestamp::from_second(at)
        .map(|t| t.to_zoned(TimeZone::system()).strftime("%H:%M").to_string())
        .unwrap_or_default()
}

fn command(program: &str, args: &[&str]) -> io::Result<String> {
    let out = Command::new(program).args(args).output()?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    } else {
        Err(io::Error::other(format!(
            "{program} {} failed: {}",
            args.first().unwrap_or(&""),
            String::from_utf8_lossy(&out.stderr).trim()
        )))
    }
}

/// Fetches with `fetch`, once more when it fails, and when both fail says
/// so and keeps the history the clone has, minutes behind: on 11 November
/// one GitHub error ("Error in the HTTP2 framing layer") failed a whole
/// tick, its card and questions with it. Says whether it fetched.
fn fetch_or_keep(fetch: impl Fn() -> io::Result<String>) -> bool {
    let Err(first) = fetch() else {
        return true;
    };
    eprintln!("cadence: fetching core again: {first}");
    match fetch() {
        Ok(_) => true,
        Err(again) => {
            eprintln!("cadence: core's history is from its last fetch: {again}");
            false
        }
    }
}

fn write_whole(path: &Path, text: &str) -> io::Result<()> {
    // Staged under this process's name, so an hour run by hand can't write
    // into the one the timer's renames, removed if it fails, as no later
    // hour shares it, and synced first, so a crash leaves a file whole: a
    // kept reading that won't parse holds the hour's findings unjudged.
    let staged = path.with_extension(format!("new.{}", std::process::id()));
    let written = fs::File::create(&staged).and_then(|mut file| {
        io::Write::write_all(&mut file, text.as_bytes())?;
        file.sync_all()
    });
    let done = written.and_then(|()| fs::rename(&staged, path));
    if done.is_err() {
        let _ = fs::remove_file(&staged);
    }
    done
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::*;

    #[test]
    fn an_alarm_says_its_answer_goes_to_cadence_though_no_peer_is_reached() {
        let asked = asked_to_answer("Cadence alarm: hot.");
        assert!(
            asked.starts_with("Cadence alarm: hot. Answer with re"),
            "{asked}"
        );
        assert!(
            asked.ends_with(
                "Send it to CADENCE: the mesh says that reaches no peer, but cadence reads it \
                 from what you send."
            ),
            "{asked}"
        );
    }

    #[test]
    fn a_fetch_that_fails_once_is_tried_again() {
        let tries = Cell::new(0);
        let fetched = fetch_or_keep(|| {
            tries.set(tries.get() + 1);
            if tries.get() == 1 {
                Err(io::Error::other("Error in the HTTP2 framing layer"))
            } else {
                Ok(String::new())
            }
        });
        assert!(fetched, "a fetch that failed once wasn't tried again");
        assert_eq!(tries.get(), 2, "the fetch wasn't tried exactly twice");
    }

    #[test]
    fn a_fetch_that_keeps_failing_leaves_the_tick_to_go_on() {
        let tries = Cell::new(0);
        let fetched = fetch_or_keep(|| {
            tries.set(tries.get() + 1);
            Err(io::Error::other("Error in the HTTP2 framing layer"))
        });
        assert!(
            !fetched,
            "a fetch that failed twice was said to have fetched"
        );
        assert_eq!(
            tries.get(),
            2,
            "a failing fetch wasn't given up after two tries"
        );
    }
}
