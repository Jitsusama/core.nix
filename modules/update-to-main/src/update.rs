//! One update, from an ask to its ending: refuse on a low battery, fetch
//! main, check it descends from what the machine last switched to and is
//! signed by a key that counts, or is what its host names, build it,
//! switch, check the machine is whole, and go back to the system it left if
//! anything after the switch's start fails. Each step is told as it starts.

use crate::config::{Config, Kind};
use crate::git;
use crate::hosted;
use crate::line::{Applied, Line};
use crate::power::{self, Power};
use crate::run::Failure;
use crate::system;
use std::fs;

/// Runs one update, telling `tell` each step as it starts, and gives its
/// ending.
pub fn update(config: &Config, tell: &mut dyn FnMut(&Line)) -> Line {
    let last = last(config);
    if let Some(why) = too_low(config) {
        return Line::Refused { commit: last, why };
    }
    tell(&Line::Fetching);
    let head = match &config.hosted {
        Some(hosted) => hosted::head(config, hosted),
        None => git::keyring(config).and_then(|()| git::fetch(config)),
    };
    let head = match head {
        Ok(head) => head,
        Err(failure) => return failed(last, failure),
    };
    if last.as_deref() == Some(head.as_str()) {
        return Line::Unchanged { commit: head };
    }
    if let Some(last) = &last {
        match git::descends(config, last, &head) {
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
        match git::signed(config, &head) {
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
    let built = match flake.and_then(|flake| system::build(config, &flake)) {
        Ok(built) => built,
        Err(failure) => return failed(last, failure),
    };

    tell(&Line::Switching {
        commit: head.clone(),
    });
    let previous = system::current(config);
    let switched = system::switch(config, &built).and_then(|switched| {
        if switched.applied == Applied::Now {
            system::healthy(config)?;
        }
        Ok(switched)
    });
    match switched {
        Ok(switched) => {
            if let Err(e) = remember(config, &head) {
                return failed(Some(head), e);
            }
            Line::Done {
                commit: head,
                applied: switched.applied,
                restart: switched.restart,
                restarted: switched.restarted,
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

fn failed(commit: Option<String>, failure: Failure) -> Line {
    Line::Failed {
        commit,
        why: failure.why,
        lines: failure.lines,
    }
}
