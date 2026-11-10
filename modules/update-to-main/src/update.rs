//! One update, from an ask to its ending: refuse on a low battery or while
//! the machine is busy with something an update mustn't cut, fetch
//! main, check it descends from what the machine last switched to and is
//! signed by a key that counts, or is what its host names, fetch and check
//! each input that follows its own main, build it, switch, check the
//! machine is whole, and go back to the system it left if anything after
//! the switch's start fails. Busy is asked again before the switch, since a
//! call may start while the build runs. A switch that would restart a
//! daemon the config holds, restart an account's unit it holds, or change a
//! path it holds such as the kernel, is refused unless an asker granted it. A
//! switch the follower asked for waits, once built, while someone is at the
//! machine. Each step is told as it starts.

use crate::config::{Busy, Config, Kind};
use crate::follows;
use crate::git::{self, Repo};
use crate::hosted;
use crate::line::Line;
use crate::power::{self, Power};
use crate::run::{self, Failure};
use crate::system;
use std::fs;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

/// How a refusal for a held daemon's restart or a held path's change
/// begins, so an asker that follows main can tell it from being busy and
/// stop until a person acts.
pub const NEEDS_HOLD: &str = "switching would change";

/// How long a busy command has to answer before it counts as busy.
const BUSY_WITHIN: Duration = Duration::from_secs(30);

/// How often a busy command is looked at while it runs.
const BUSY_EVERY: Duration = Duration::from_millis(100);

/// How often a built switch the follower asked for asks again whether
/// what it waits on is over, and the longest it waits before it ends with
/// nothing switched, what it built kept in the store for the next ask.
const WAITS_EVERY: Duration = Duration::from_secs(1);
const WAITS_AT_MOST: Duration = Duration::from_hours(1);

/// Runs one update, telling `tell` each step as it starts, and gives its
/// ending. `following` says whether only the follower asked for it, which
/// may turn false as it runs, once a person asks too.
///
/// A grant made before it starts is its own, and no grant outlives it,
/// however it ends: one left by an ask refused as busy, or by a build that
/// held nothing, would otherwise let a later switch restart a held daemon
/// after the work it was held for went on.
pub fn update(config: &Config, following: &dyn Fn() -> bool, tell: &mut dyn FnMut(&Line)) -> Line {
    let granted = take_grant(config);
    if granted {
        run::granted();
    }
    let ending = run(config, granted, following, tell);
    take_grant(config);
    if matches!(ending, Line::Done { .. } | Line::Unchanged { .. }) {
        let _ = fs::remove_file(config.behind_file());
    }
    ending
}

/// Marks the machine behind main from the first build of a main it
/// doesn't run, keeping the mark from an earlier one that didn't get it on.
fn mark_behind(config: &Config) {
    let behind = config.behind_file();
    if !behind.exists() {
        let _ = fs::write(behind, "");
    }
}

/// Takes away the grant for held changes, saying whether there was one.
fn take_grant(config: &Config) -> bool {
    config
        .holds
        .as_ref()
        .is_some_and(|holds| fs::remove_file(&holds.given).is_ok())
}

fn run(
    config: &Config,
    granted: bool,
    following: &dyn Fn() -> bool,
    tell: &mut dyn FnMut(&Line),
) -> Line {
    let last = last(config);
    if let Some(why) = too_low(config).or_else(|| busy(config)) {
        return Line::Refused { commit: last, why };
    }
    // The ask that brings the grant a built switch waited for switches to
    // that one, rather than fetch main again and build what merged since:
    // on 8 November the morning switch started over three times for newer
    // heads while Joel waited.
    if granted && let Some(waiting) = waited(config) {
        return switch_waiting(config, waiting, last, tell);
    }
    tell(&Line::Fetching);
    let main = Repo::main(config);
    let head = match &config.hosted {
        Some(hosted) => hosted::head(config, hosted, last.as_deref()),
        None => git::keyring(&main).and_then(|()| git::fetch(&main)),
    };
    let head = match head {
        Ok(head) => head,
        Err(failure) => return failed(last, failure),
    };
    let last_followed = follows::last(config);
    let followed = match follows::heads(config, &head, &last_followed) {
        Ok(Ok(followed)) => followed,
        Ok(Err(why)) => return Line::Refused { commit: last, why },
        Err(failure) => return failed(last, failure),
    };
    if last.as_deref() == Some(head.as_str()) && followed == last_followed {
        return Line::Unchanged {
            commit: head,
            followed,
        };
    }
    if let Some(last) = &last {
        let descends = match &config.hosted {
            Some(_) => hosted::descends(config, last, &head),
            None => git::descends(&main, last, &head),
        };
        match descends {
            Ok(true) => {}
            Ok(false) => {
                return Line::Refused {
                    commit: Some(last.clone()),
                    why: format!(
                        "main at {} doesn't descend from {}, which this machine last switched to",
                        git::short(&head),
                        git::short(last)
                    ),
                };
            }
            Err(failure) => return failed(Some(last.clone()), failure),
        }
    }
    // A hosted main is the host's word, asked over TLS; any other is only
    // as good as its signature.
    if config.hosted.is_none() {
        match git::signed(&main, &head) {
            Ok(Ok(())) => {}
            Ok(Err(why)) => return Line::Refused { commit: last, why },
            Err(failure) => return failed(last, failure),
        }
    }

    tell(&Line::Building {
        commit: head.clone(),
    });
    mark_behind(config);
    let flake = match &config.hosted {
        Some(hosted) => hosted::prepare(config, hosted, &head),
        None => Ok(git::flake(config, &head)),
    };
    let overrides = follows::overrides(config, &followed);
    let built = match flake.and_then(|flake| system::build(config, &flake, &overrides)) {
        Ok(built) => built,
        Err(failure) => return failed(last, failure),
    };
    // What was built stays in the store, so the next ask switches at once.
    if let Some(why) = busy(config) {
        return Line::Refused { commit: last, why };
    }
    if let Some(why) = held(config, &built, granted) {
        keep_waiting(config, &head, &followed, &built);
        return Line::Refused { commit: last, why };
    }
    // Heads that moved without moving the system, such as a merge of
    // another machine's files, switch nothing, and so restart nothing a
    // switch would.
    if system::current(config).is_some() && system::current(config) == fs::canonicalize(&built).ok()
    {
        if let Err(e) = remember(config, &head).and_then(|()| follows::remember(config, &followed))
        {
            return failed(Some(head), e);
        }
        return Line::Unchanged {
            commit: head,
            followed,
        };
    }
    if let Some(why) = quiet(config, following).or_else(|| busy(config)) {
        return Line::Refused { commit: last, why };
    }

    tell(&Line::Switching {
        commit: head.clone(),
    });
    // A stop is taken only before this line is told, so one that came
    // while the build ended is seen here, before anything switches.
    if run::stopping() {
        return failed(last, Failure::new("the update was stopped"));
    }
    switch(config, &built, last, head, followed)
}

/// A built system whose switch waits for a hold to be given.
#[derive(serde::Serialize, serde::Deserialize)]
struct Waiting {
    commit: String,
    followed: follows::Followed,
    system: std::path::PathBuf,
}

fn waiting_file(config: &Config) -> std::path::PathBuf {
    config.state.join("waiting.json")
}

/// Keeps the system built at `head` whose switch waits for a hold, for the
/// ask with the grant. Not keeping it costs only a fetch and a build.
fn keep_waiting(
    config: &Config,
    head: &str,
    followed: &follows::Followed,
    built: &std::path::Path,
) {
    let waiting = Waiting {
        commit: head.to_owned(),
        followed: followed.clone(),
        system: built.to_path_buf(),
    };
    if let Ok(text) = serde_json::to_string(&waiting) {
        let _ = fs::write(waiting_file(config), text);
    }
}

/// Switches to the system that waited for the hold now granted.
fn switch_waiting(
    config: &Config,
    waiting: Waiting,
    last: Option<String>,
    tell: &mut dyn FnMut(&Line),
) -> Line {
    let _ = fs::remove_file(waiting_file(config));
    tell(&Line::Building {
        commit: waiting.commit.clone(),
    });
    if let Some(why) = busy(config) {
        return Line::Refused { commit: last, why };
    }
    tell(&Line::Switching {
        commit: waiting.commit.clone(),
    });
    if run::stopping() {
        return failed(last, Failure::new("the update was stopped"));
    }
    switch(
        config,
        &waiting.system,
        last,
        waiting.commit,
        waiting.followed,
    )
}

/// The system waiting for a hold, while it's still in the store and isn't
/// what the machine runs.
fn waited(config: &Config) -> Option<Waiting> {
    let text = fs::read_to_string(waiting_file(config)).ok()?;
    let waiting: Waiting = serde_json::from_str(&text).ok()?;
    let system = fs::canonicalize(&waiting.system).ok()?;
    (system::current(config) != Some(system)).then_some(waiting)
}

/// Switches to `built`, main's `head` with its inputs at `followed`, and
/// goes back to what the machine ran if the switch or the health checks
/// fail.
fn switch(
    config: &Config,
    built: &std::path::Path,
    last: Option<String>,
    head: String,
    followed: follows::Followed,
) -> Line {
    let previous = system::current(config);
    // Whatever waited for a hold is older than what switches now.
    let _ = fs::remove_file(waiting_file(config));
    let switched = system::switch(config, built)
        .and_then(|switched| system::healthy(config).map(|()| switched));
    match switched {
        Ok(switched) => {
            if let Err(e) =
                remember(config, &head).and_then(|()| follows::remember(config, &followed))
            {
                return failed(Some(head), e);
            }
            Line::Done {
                commit: head,
                applied: switched.applied,
                restart: switched.restart,
                restarted: switched.restarted,
                followed,
            }
        }
        Err(mut failure) => {
            match previous {
                Some(previous) => match system::go_back(config, &previous) {
                    Ok(()) => failure
                        .why
                        .push_str("; the machine went back to what it ran"),
                    Err(back) => {
                        failure.why.push_str("; going back failed too: ");
                        failure.why.push_str(&back.why);
                        failure.lines.extend(back.lines);
                    }
                },
                None => failure.why.push_str("; there was no system to go back to"),
            }
            failed(last, failure)
        }
    }
}

/// The commit the machine last switched to, when it has.
pub fn last(config: &Config) -> Option<String> {
    fs::read_to_string(config.last_file())
        .ok()
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty())
}

fn remember(config: &Config, commit: &str) -> Result<(), Failure> {
    let file = config.last_file();
    let new = file.with_extension("new");
    fs::write(&new, format!("{commit}\n"))
        .and_then(|()| fs::rename(&new, &file))
        .map_err(|e| Failure::new(format!("{}: {e}", file.display())))
}

fn too_low(config: &Config) -> Option<String> {
    let power = match (config.kind, &config.tools.pmset) {
        (Kind::Darwin, Some(pmset)) => power::mac(pmset),
        _ => power::linux(&config.power_supplies),
    };
    match power {
        Power::Battery(charge) if charge < config.battery_floor => Some(format!(
            "it's on battery at {charge}%, below {}%",
            config.battery_floor
        )),
        _ => None,
    }
}

/// Why switching to `built` waits for a hold, when it would restart a held
/// daemon or change a held path and the update wasn't `granted` it.
fn held(config: &Config, built: &std::path::Path, granted: bool) -> Option<String> {
    let holds = config.holds.as_ref()?;
    let current = system::current(config)?;
    let units = holds
        .units
        .iter()
        .filter(|u| system::daemon_changed(config.kind, &current, built, u));
    let paths = holds
        .paths
        .iter()
        .filter(|p| system::path_changed(&current, built, p));
    let accounts = holds.account_units.iter().flat_map(|(account, units)| {
        units
            .iter()
            .filter(|u| system::account_unit_restarts(&current, built, account, u))
            .map(move |u| format!("{account}'s {u}"))
    });
    let changes: Vec<String> = units.chain(paths).cloned().chain(accounts).collect();
    if changes.is_empty() || granted {
        return None;
    }
    Some(format!(
        "{NEEDS_HOLD} {}, which waits for a hold: once the machine's work is held, make {} \
         and ask again",
        changes.join(", "),
        holds.given.display()
    ))
}

/// What the machine is busy with, when any of its busy commands says so.
/// Those a follower waits on refuse nothing.
fn busy(config: &Config) -> Option<String> {
    config
        .refuse_while
        .iter()
        .filter(|busy| !busy.follower_waits)
        .find_map(is_busy)
}

/// Waits, for a switch only the follower asked for, while any command a
/// follower waits on says someone is at the machine, each for as long as
/// it may hold the switch, and gives why it switches nothing once it has
/// waited an hour. On 9 November a launcher stopped taking keys after five
/// switches the follower asked for in half an hour, each restarting the
/// desktop's shell under Joel's hands; and that evening no switch came for
/// two hours while he worked, with 26 merges waiting, so someone at the
/// keys holds it for 45 minutes and an open launcher the whole hour. A
/// person asking meanwhile, or a stop, ends the wait at once.
fn quiet(config: &Config, following: &dyn Fn() -> bool) -> Option<String> {
    let began = Instant::now();
    let mut said = None;
    while following() && !run::stopping() {
        let waited = began.elapsed();
        let why = config
            .refuse_while
            .iter()
            .filter(|busy| {
                busy.follower_waits
                    && busy
                        .follower_waits_at_most
                        .is_none_or(|most| waited < Duration::from_secs(most))
            })
            .find_map(is_busy)?;
        if waited >= WAITS_AT_MOST {
            return Some(format!(
                "{why} for an hour, so what was built waits in the store for the next ask"
            ));
        }
        if said.as_ref() != Some(&why) {
            eprintln!("the switch waits while {why}");
            said = Some(why);
        }
        thread::sleep(WAITS_EVERY);
    }
    None
}

fn is_busy(busy: &Busy) -> Option<String> {
    let (program, args) = busy.command.split_first()?;
    let unsure = |why: String| Some(format!("it can't tell whether {}: {why}", busy.why));
    let mut child = match Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(child) => child,
        Err(e) => return unsure(format!("{program}: {e}")),
    };
    let deadline = Instant::now() + BUSY_WITHIN;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                return match status.code() {
                    Some(0) => Some(busy.why.clone()),
                    Some(1) => None,
                    _ => unsure(format!("`{}` ended {status}", busy.command.join(" "))),
                };
            }
            Ok(None) if Instant::now() < deadline => thread::sleep(BUSY_EVERY),
            Ok(None) => {
                // It's past answering; what it was doing no longer matters.
                let _ = child.kill();
                let _ = child.wait();
                return unsure(format!(
                    "`{}` didn't answer within {} s",
                    busy.command.join(" "),
                    BUSY_WITHIN.as_secs()
                ));
            }
            Err(e) => return unsure(e.to_string()),
        }
    }
}

fn failed(commit: Option<String>, failure: Failure) -> Line {
    Line::Failed {
        commit,
        why: failure.why,
        lines: failure.lines,
    }
}
