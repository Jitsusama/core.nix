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
//! Given an orchestrator, each tick also tells an orchestrator of any session of
//! its that has sat idle two hours, once for each idle spell (idle.rs).
//!
//! `cadence day`, run each evening by a timer of its own, reads the day
//! just ended against the days before it and sends the orchestrator its note
//! (daily.rs). Each finding goes in its ledger and to the owner its key's
//! prefix names, or to the watcher (findings.rs).
//!
//! `cadence axes`, typed by Joel, shows the nine axes he holds the machine
//! to, each against its bar with its owner, and `cadence axes <axis>`
//! steps into one (axes.rs).
//!
//!     cadence tick [--watcher ADDRESS --orchestrator ADDRESS]
//!         [--restart WORDS [--revive ADDRESS]... [--skip ADDRESS]...]
//!     cadence day [--orchestrator ADDRESS [--watcher ADDRESS] [--owner PREFIX=ADDRESS]...]
//!     cadence axes [AXIS]

mod axes;
mod builders;
mod card;
mod daily;
mod day;
mod findings;
mod gate;
mod health;
mod idle;
mod judge;
mod machine;
mod mesh;
mod net;
mod owe;
mod passed;
mod records;
mod revive;
mod speed;
mod start;
mod stats;
mod watch;

use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::time::{SystemTime, UNIX_EPOCH};

use jiff::Timestamp;
use jiff::tz::TimeZone;

use mesh::{Mesh, Message};
use net::Round;
use watch::{Landing, Snapshot, State, To};

/// The address cadence's questions come from.
const FROM: &str = "CADENCE";
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
}

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        Some("tick") => {}
        Some("day") => return day(args),
        Some("axes") => return axes::run(args),
        Some("owe") => {
            return match folders() {
                Ok((_, _, folder)) => owe::run(args, &folder, now_ms() / 1000),
                Err(err) => {
                    eprintln!("cadence: {err}");
                    ExitCode::FAILURE
                }
            };
        }
        _ => {
            eprintln!(
                "usage: cadence tick [--watcher ADDRESS --orchestrator ADDRESS] [--restart WORDS \
                 [--revive ADDRESS]... [--skip ADDRESS]...]\n       \
                 {DAY_USAGE}\n       \
                 cadence axes [AXIS]\n       {}",
                owe::USAGE
            );
            return ExitCode::from(2);
        }
    }
    let mut parsed = Args {
        watcher: None,
        orchestrator: None,
        restart: None,
        orchestrators: Vec::new(),
        skip: Vec::new(),
    };
    while let Some(flag) = args.next() {
        let value = args.next();
        match (flag.as_str(), value) {
            ("--watcher", Some(v)) => parsed.watcher = Some(v),
            ("--orchestrator", Some(v)) => parsed.orchestrator = Some(v),
            ("--restart", Some(v)) => parsed.restart = Some(v),
            ("--revive", Some(v)) => parsed.orchestrators.push(v),
            ("--skip", Some(v)) => parsed.skip.push(v),
            _ => {
                eprintln!("cadence: {flag} isn't a flag it takes, or has no value");
                return ExitCode::from(2);
            }
        }
    }
    match run(&parsed) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("cadence: {err}");
            ExitCode::FAILURE
        }
    }
}

const DAY_USAGE: &str =
    "cadence day [--orchestrator ADDRESS [--watcher ADDRESS] [--owner PREFIX=ADDRESS]...]";

fn day(mut args: impl Iterator<Item = String>) -> ExitCode {
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
                eprintln!("usage: {DAY_USAGE}");
                return ExitCode::from(2);
            }
        }
    }
    let ran = folders().and_then(|(_, state_home, folder)| {
        daily::run(&folder, &state_home, &told, now_ms() / 1000)
    });
    match ran {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("cadence: {err}");
            ExitCode::FAILURE
        }
    }
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
    if let (Some(restart), Some(orchestrator)) = (&args.restart, &args.orchestrator) {
        let data_home =
            env::var_os("XDG_DATA_HOME").map_or_else(|| home.join(".local/share"), PathBuf::from);
        let reviving = start::Reviving {
            orchestrator: orchestrator.clone(),
            watcher: args.watcher.clone(),
            orchestrators: args.orchestrators.clone(),
            skip: args.skip.clone(),
            restart: restart.clone(),
            quests: data_home.join("pi/agentic-harness.pi/quest-workflow/quests"),
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
        if let Err(err) = tell_idle(&mesh, &agent.join("sessions"), &folder, now) {
            eprintln!("cadence: telling of idle sessions: {err}");
        }
    }
    let snapshot = gather(&folder, &state_home, &mesh, args, now)?;
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
    for question in watch::tick(&snapshot, &mut state) {
        let to = match question.to {
            To::Watcher => watcher,
            To::Orchestrator => orchestrator,
        };
        let text = match question.to {
            To::Watcher => format!(
                "{} Answer with re set to this question once handled.",
                question.text
            ),
            To::Orchestrator => question.text.clone(),
        };
        let message = message(to, question.to.intent(), text);
        mesh.send(&message)?;
        if let Some(key) = &question.key {
            state.asked_as(key, &message.id, now, &question.text);
        }
    }
    if watch::card_due(&snapshot, &state) {
        let text = format!(
            "{card}Sent by cadence, as the session keeping the cycle fast sent none this hour; \
             tokens, sessions and the top bottleneck come with its next."
        );
        mesh.send(&message(orchestrator, "fyi", text))?;
        state.carded = Some(now);
    }
    write_whole(&path, &serde_json::to_string_pretty(&state)?)
}

/// Tells each orchestrator of its sessions that have sat idle two hours,
/// remembering in `idle.json` which spells were told.
fn tell_idle(mesh: &Mesh, logs: &Path, folder: &Path, now: i64) -> io::Result<()> {
    let path = folder.join("idle.json");
    let mut told: idle::Told = fs::read(&path)
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default();
    let boot = fs::read_to_string("/proc/sys/kernel/random/boot_id")
        .ok()
        .map(|id| id.trim().to_owned());
    let logged = mesh::logs_written(logs);
    let sessions: Vec<idle::Quiet> = mesh
        .all()
        .iter()
        .filter_map(|p| {
            idle::Quiet::from(
                p,
                logged.get(&p.session).copied(),
                boot.as_deref(),
                mesh.is_parked(&p.session),
                |p| p.runs(start::is_pi, start::is_host),
            )
        })
        .collect();
    let tells = idle::due(&sessions, &mut told, now, clock);
    let mut failed = None;
    for tell in &tells {
        if failed.is_some() {
            told.spells.remove(&tell.session);
            continue;
        }
        // Told as the orchestrator is: nothing waits on its answer.
        match mesh.send(&message(
            &tell.to,
            To::Orchestrator.intent(),
            tell.text.clone(),
        )) {
            Ok(()) => println!("{}", tell.text),
            Err(err) => {
                // Not told, so the next tick tells it again.
                told.spells.remove(&tell.session);
                failed = Some(err);
            }
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
    follower.saw(records::parse_looks(&journal(&[
        "-u",
        "update-to-main.service",
        &since,
    ])));
    let landings = core.landings(now - DAY, &follower)?;
    let net = state_home.join("net");
    let round = core.round(&fs::read_to_string(net.join("state.json")).unwrap_or_default());
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
    })
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
    let staged = path.with_extension("new");
    fs::write(&staged, text)?;
    fs::rename(staged, path)
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::*;

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
