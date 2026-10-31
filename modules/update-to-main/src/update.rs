//! One update, from an ask to its ending: refuse on a low battery or while
//! the machine is busy with something an update mustn't cut, fetch
//! main, check it descends from what the machine last switched to and is
//! signed by a key that counts, or is what its host names, fetch and check
//! each input that follows its own main, build it, switch, check the
//! machine is whole, and go back to the system it left if anything after
//! the switch's start fails. Busy is asked again before the switch, since a
//! call may start while the build runs. Each step is told as it starts.

use crate::config::{Busy, Config, Kind};
use crate::follows;
use crate::git::{self, Repo};
use crate::hosted;
use crate::line::{Applied, Line};
use crate::power::{self, Power};
use crate::run::Failure;
use crate::system;
use std::fs;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

/// How long a busy command has to answer before it counts as busy.
const BUSY_WITHIN: Duration = Duration::from_secs(30);

/// How often a busy command is looked at while it runs.
const BUSY_EVERY: Duration = Duration::from_millis(100);

/// Runs one update, telling `tell` each step as it starts, and gives its
/// ending.
pub fn update(config: &Config, tell: &mut dyn FnMut(&Line)) -> Line {
    let last = last(config);
    if let Some(why) = too_low(config).or_else(|| busy(config)) {
        return Line::Refused { commit: last, why };
    }
    tell(&Line::Fetching);
    let main = Repo::main(config);
    let head = match &config.hosted {
        Some(hosted) => hosted::head(config, hosted),
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
        match git::descends(&main, last, &head) {
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

    tell(&Line::Switching {
        commit: head.clone(),
    });
    switch(config, &built, last, head, followed)
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
    let switched = system::switch(config, built).and_then(|switched| {
        if switched.applied == Applied::Now {
            system::healthy(config)?;
        }
        Ok(switched)
    });
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

/// What the machine is busy with, when any of its busy commands says so.
fn busy(config: &Config) -> Option<String> {
    config.refuse_while.iter().find_map(is_busy)
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
