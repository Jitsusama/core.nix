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
//! machine. With a boot check, a system whose kernel, initrd or kernel
//! parameters changed becomes what the machine boots only once it passes.
//! Each step is told as it starts. Long because these steps are one ordered
//! sequence over shared state, and each going back must see the switch it
//! undoes, so splitting them would scatter one update across files.

use crate::busy;
use crate::config::{Config, Kind};
use crate::follows;
use crate::git::{self, Repo};
use crate::held;
use crate::hosted;
use crate::line::Line;
use crate::power::{self, Power};
use crate::run::{self, Failure};
use crate::system;
use std::fs;

/// How a refusal for a held daemon's restart or a held path's change
/// begins, so an asker that follows main can tell it from being busy and
/// stop until a person acts.
pub const NEEDS_HOLD: &str = "switching would change";

/// Runs one update, telling `tell` each step as it starts, and gives its
/// ending. `following` says whether only the follower asked for it, which
/// may turn false as it runs, once a person asks too.
///
/// A grant made before it starts is its own, and no grant outlives it,
/// however it ends: one left by an ask refused as busy, or by a build that
/// held nothing, would otherwise let a later switch restart a held daemon
/// after the work it was held for went on.
pub fn update(config: &Config, following: &dyn Fn() -> bool, tell: &mut dyn FnMut(&Line)) -> Line {
    update_at(config, following, &follows::Followed::new(), tell)
}

/// Runs one update as `update` does, with each followed input `at` names
/// at that commit rather than its main's head (src/follows.rs says what
/// such a commit must be).
pub fn update_at(
    config: &Config,
    following: &dyn Fn() -> bool,
    at: &follows::Followed,
    tell: &mut dyn FnMut(&Line),
) -> Line {
    let granted = take_grant(config);
    if granted {
        run::granted();
    }
    let ending = run(config, granted, following, at, tell);
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
    at: &follows::Followed,
    tell: &mut dyn FnMut(&Line),
) -> Line {
    let last = last(config);
    if let Some(why) = paused(config, following)
        .or_else(|| too_low(config))
        .or_else(|| busy::now(config))
    {
        return Line::Refused { commit: last, why };
    }
    // The ask that brings the grant a built switch waited for switches to
    // that one, rather than fetch main again and build what merged since:
    // on 8 November the morning switch started over three times for newer
    // heads while Joel waited.
    // One built with its inputs elsewhere than an ask names isn't what
    // that ask was for. A plain ask names none, so it takes whatever
    // waits, even one built for an ask at a named commit: switching what
    // was built beats building newer, as for any waiting switch.
    if granted
        && let Some(waiting) = waited(config)
        && at
            .iter()
            .all(|(name, commit)| waiting.followed.get(name) == Some(commit))
        && held::stops(config, &waiting.commit, &waiting.followed).is_none()
    {
        return switch_waiting(config, waiting, last, tell);
    }
    // Switching what waited for a grant builds nothing, so a check that
    // refuses the ask alone is asked only past it: the load it reads may
    // be the waiting build's own. It's asked before the fetch, so a refused
    // ask costs a loaded machine nothing; the follower doesn't ask again
    // soon for heads the machine runs (`follow`).
    if let Some(why) = busy::at_the_ask(config, following) {
        return Line::Refused { commit: last, why };
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
    let followed = match follows::heads(config, &head, &last_followed, &waited_asks(config), at) {
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
    if let Some(refused) = untrusted(config, &main, last.as_deref(), &head) {
        return refused;
    }
    if let Some(why) = held::stops(config, &head, &followed) {
        return Line::Refused { commit: last, why };
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
    let flake = match flake {
        Ok(flake) => flake,
        Err(failure) => return failed(last, failure),
    };
    let built = match system::build(config, &flake, &overrides) {
        Ok(built) => built,
        Err(failure) => return failed(last, failure),
    };
    // What was built stays in the store, so the next ask switches at once.
    if let Some(why) = busy::now(config) {
        return Line::Refused { commit: last, why };
    }
    if let Some(why) = held(config, &built, granted) {
        keep_waiting(config, &head, &followed, &built);
        return Line::Refused { commit: last, why };
    }
    // Heads that moved without moving the system, such as a merge of
    // another machine's files, switch nothing, and so restart nothing a
    // switch would.
    if runs_wholly(config, &built) {
        if let Err(e) = ran_wholly(config, &head, &followed) {
            return failed(Some(head), e);
        }
        return Line::Unchanged {
            commit: head,
            followed,
        };
    }
    // A pause or hold asked while it built or waited, up to an hour, is
    // seen here, before anything switches.
    if let Some(why) = busy::quiet(config, following)
        .or_else(|| busy::now(config))
        .or_else(|| paused(config, following))
        .or_else(|| held::stops(config, &head, &followed))
    {
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
    let built = system::Built {
        system: &built,
        flake: &flake,
        overrides: &overrides,
    };
    switch(config, &built, last, head, followed)
}

/// Why an update the follower alone asked for mustn't run, if updates
/// are paused. A person's ask runs through a pause, so a switch granted by
/// hand needn't resume and pause again around its ask, which would let the
/// follower in if the asker died between them.
fn paused(config: &Config, following: &dyn Fn() -> bool) -> Option<String> {
    held::paused(config).filter(|_| following())
}

/// Each asked input a build waiting for a hold was built at. An ask at an
/// input that moves only when asked is one whose move most often waits for
/// a hold, and its commit is remembered only once it switches: without
/// this, the next update builds that input at its pin, and its switch, or
/// a newer wait, spends the ask. It's kept, never a floor, since it never
/// ran, so a later ask may still step behind it. One a hold keeps off is
/// let go, so the hold stops that ask rather than every update of main's.
fn waited_asks(config: &Config) -> follows::Followed {
    waited(config)
        .map(|waiting| {
            waiting
                .followed
                .into_iter()
                .filter(|(name, commit)| {
                    config.follows.get(name).is_some_and(|f| f.asked)
                        && !held::keeps_off(config, name, commit)
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Remembers main at `head` and its inputs at `followed` as what runs,
/// when it builds what the machine already runs. A build that waited for
/// a hold is let go then: what ran since is a later ask's, so the next
/// update mustn't build the wait again.
fn ran_wholly(config: &Config, head: &str, followed: &follows::Followed) -> Result<(), Failure> {
    remember(config, head)?;
    follows::remember(config, followed)?;
    let _ = fs::remove_file(waiting_file(config));
    Ok(())
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
    if let Some(why) = busy::now(config) {
        return Line::Refused { commit: last, why };
    }
    // Only a boot check reads the flake, so it's worked out only for a
    // system that waits on one, before the switch is told.
    let flake = if system::waits_on_its_boot_check(config, &waiting.system) {
        match waiting_flake(config, &waiting.commit) {
            Ok(flake) => flake,
            Err(failure) => return failed(last, failure),
        }
    } else {
        String::new()
    };
    tell(&Line::Switching {
        commit: waiting.commit.clone(),
    });
    if run::stopping() {
        return failed(last, Failure::new("the update was stopped"));
    }
    let overrides = follows::overrides(config, &waiting.followed);
    let built = system::Built {
        system: &waiting.system,
        flake: &flake,
        overrides: &overrides,
    };
    switch(config, &built, last, waiting.commit, waiting.followed)
}

/// The flake a system that waited was built from, for its boot check: root's
/// copy at its commit, or on a hosted machine its folder unpacked again, since
/// every unpack wipes the one before and a later ask may have unpacked
/// another. One whose folder can't be had again fails the grant, saying why,
/// rather than have the check judge a system it can't rebuild.
fn waiting_flake(config: &Config, commit: &str) -> Result<String, Failure> {
    match &config.hosted {
        None => Ok(git::flake(config, commit)),
        Some(hosted) => hosted::prepare(config, hosted, commit).map_err(|failure| Failure {
            why: format!(
                "unpacking main at {commit} again for its boot check: {}",
                failure.why
            ),
            ..failure
        }),
    }
}

/// Why main at `head` mustn't be built, if it mustn't: it doesn't descend
/// from `last`, which the machine last switched to, or it isn't signed.
fn untrusted(config: &Config, main: &Repo<'_>, last: Option<&str>, head: &str) -> Option<Line> {
    if let Some(last) = last {
        let descends = match &config.hosted {
            Some(_) => hosted::descends(config, last, head),
            None => git::descends(main, last, head),
        };
        match descends {
            Ok(true) => {}
            Ok(false) => {
                return Some(Line::Refused {
                    commit: Some(last.to_owned()),
                    why: format!(
                        "main at {} doesn't descend from {}, which this machine last switched to",
                        git::short(head),
                        git::short(last)
                    ),
                });
            }
            Err(failure) => return Some(failed(Some(last.to_owned()), failure)),
        }
    }
    // A hosted main is the host's word, asked over TLS; any other is only
    // as good as its signature.
    if config.hosted.is_none() {
        match git::signed(main, head) {
            Ok(Ok(())) => {}
            Ok(Err(why)) => {
                return Some(Line::Refused {
                    commit: last.map(str::to_owned),
                    why,
                });
            }
            Err(failure) => return Some(failed(last.map(str::to_owned), failure)),
        }
    }
    None
}

/// The system waiting for a hold, while it's still in the store and isn't
/// what the machine runs.
fn waited(config: &Config) -> Option<Waiting> {
    let text = fs::read_to_string(waiting_file(config)).ok()?;
    let waiting: Waiting = serde_json::from_str(&text).ok()?;
    fs::canonicalize(&waiting.system).ok()?;
    (!system::runs(config, &waiting.system)).then_some(waiting)
}

/// Switches to `built`, main's `head` with its inputs at `followed`, and
/// goes back to what the machine ran if the switch or the health checks
/// fail.
fn switch(
    config: &Config,
    built: &system::Built,
    last: Option<String>,
    head: String,
    followed: follows::Followed,
) -> Line {
    // After a switch its daemon left, what runs was never checked, so a
    // switch now goes back, if it must, to what ran before that one
    // began, whatever main builds since.
    let previous = match begun(config) {
        Some(Begun {
            system: left,
            previous: Some(previous),
            failed: Some(why),
        }) if fs::canonicalize(&left).ok() == fs::canonicalize(built.system).ok() => {
            // Going back from this very system didn't finish, cut off as
            // a Mac's is when going back reloads its daemon too, or
            // failed: it's made again, not the broken switch.
            let mut failure = going_back(config, built.system, Failure::new(why), Some(previous));
            failure
                .why
                .push_str(", making again a going back that didn't finish");
            return failed(last, failure);
        }
        Some(left) => {
            eprintln!(
                "the switch to {} was left unended, so this one goes back, if it must, to what \
                 ran before it",
                left.system.display()
            );
            left.previous
        }
        None => {
            if begun_file(config).exists() {
                eprintln!(
                    "{} won't read, so this switch goes back, if it must, to what runs",
                    begun_file(config).display()
                );
            }
            system::went_from(config)
        }
    };
    // What runs may be what was built, unchecked, as when the profile was
    // left behind it; going back there would keep a broken system, so a
    // switch goes back to what the profile names instead.
    let other = |system: &std::path::PathBuf| {
        fs::canonicalize(system).ok() != fs::canonicalize(built.system).ok()
    };
    let previous = previous
        .filter(other)
        .or_else(|| system::profiled(config).filter(other));
    let begun = Begun {
        system: built.system.to_path_buf(),
        previous: previous.clone(),
        failed: None,
    };
    if let Err(failure) = begin(config, &begun) {
        return failed(last, failure);
    }
    switch_from(config, built, last, head, followed, previous)
}

/// Whether the machine runs `built` as a switch to it leaves it: running
/// it, the profile naming it, and no switch left unended. Short of any,
/// the switch is made again.
fn runs_wholly(config: &Config, built: &std::path::Path) -> bool {
    let system = fs::canonicalize(built).ok();
    system.is_some()
        && system::runs(config, built)
        && system::profiled(config) == system
        && !begun_file(config).exists()
}

/// A switch begun and not yet ended: what it switches to, for whoever
/// reads the file, and what ran before it. One left behind is a daemon's
/// that stopped mid-switch, as a Mac's does when activation reloads it;
/// its activation runs on and may finish the switch alone, so the next
/// update makes it again, with its health checks, rather than finding
/// the system running and done. It's ended once the switch is checked,
/// or once going back from it worked.
#[derive(serde::Serialize, serde::Deserialize)]
struct Begun {
    system: std::path::PathBuf,
    previous: Option<std::path::PathBuf>,
    /// Why the switch failed, once going back from it began, so going
    /// back cut off is finished rather than the switch made again.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    failed: Option<String>,
}

fn begun_file(config: &Config) -> std::path::PathBuf {
    config.state.join("switching.json")
}

/// The switch a daemon began and never ended, if one did.
fn begun(config: &Config) -> Option<Begun> {
    serde_json::from_str(&fs::read_to_string(begun_file(config)).ok()?).ok()
}

/// Marks `begun` as begun, written whole or not at all.
fn begin(config: &Config, begun: &Begun) -> Result<(), Failure> {
    let file = begun_file(config);
    let writing = file.with_extension("json.new");
    let said = |e: std::io::Error| Failure::new(format!("{}: {e}", file.display()));
    let text = serde_json::to_string(begun).map_err(|e| Failure::new(e.to_string()))?;
    fs::write(&writing, text).map_err(said)?;
    fs::rename(&writing, &file).map_err(said)
}

fn end_switch(config: &Config) {
    let _ = fs::remove_file(begun_file(config));
}

/// Switches to `built` as `switch` says, going back to `previous`.
fn switch_from(
    config: &Config,
    built: &system::Built,
    last: Option<String>,
    head: String,
    followed: follows::Followed,
    previous: Option<std::path::PathBuf>,
) -> Line {
    // Whatever waited for a hold is older than what switches now.
    let _ = fs::remove_file(waiting_file(config));
    let switched = system::switch(config, built.system)
        .and_then(|switched| system::healthy(config).map(|()| switched));
    match switched {
        Ok(mut switched) => {
            // Ended before the head is remembered, so a daemon that dies
            // between the two leaves the head to build again, not a
            // checked switch marked unended.
            end_switch(config);
            if let Err(e) =
                remember(config, &head).and_then(|()| follows::remember(config, &followed))
            {
                return failed(Some(head), e);
            }
            // A failed boot check leaves the system running, since it's
            // whole, but booting what it booted before, which no restart
            // changes.
            let mut boots = None;
            if switched.unbooted
                && let Err(failure) = system::boot_once_checked(config, built, &head)
            {
                switched.restart = None;
                boots = Some(format!(
                    "{}, so the machine boots what it booted before: {}",
                    failure.why,
                    failure.lines.join("; ")
                ));
            }
            Line::Done {
                commit: head,
                applied: switched.applied,
                restart: switched.restart,
                restarted: switched.restarted,
                boots,
                followed,
            }
        }
        Err(failure) => failed(last, going_back(config, built.system, failure, previous)),
    }
}

/// Goes back from `system`, whose switch failed as `failure` says, to
/// `previous`, saying how that went. Going back is marked begun first,
/// so one its daemon is cut off in, as a Mac's going back can reload it
/// as surely as going forward, is finished by the next update.
fn going_back(
    config: &Config,
    system: &std::path::Path,
    mut failure: Failure,
    previous: Option<std::path::PathBuf>,
) -> Failure {
    let Some(previous) = previous else {
        failure.why.push_str("; there was no system to go back to");
        return failure;
    };
    let begun = Begun {
        system: system.to_path_buf(),
        previous: Some(previous.clone()),
        failed: Some(failure.why.clone()),
    };
    if let Err(unmarked) = begin(config, &begun) {
        eprintln!("{}", unmarked.why);
    }
    match system::go_back(config, &previous) {
        Ok(()) => {
            end_switch(config);
            failure
                .why
                .push_str("; the machine went back to what it ran");
        }
        Err(back) => {
            failure.why.push_str("; going back failed too: ");
            failure.why.push_str(&back.why);
            failure.lines.extend(back.lines);
        }
    }
    failure
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

fn failed(commit: Option<String>, failure: Failure) -> Line {
    Line::Failed {
        commit,
        why: failure.why,
        lines: failure.lines,
    }
}
