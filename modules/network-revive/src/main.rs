//! network-revive gives an account's units another go when the network
//! changes, for programs that can't start without it, or start wrong.
//!
//! systemd's user manager knows nothing of the network: it can't follow the
//! system's network-online.target, so a unit that looks a name up as it
//! starts fails on a boot whose network comes late, and stays failed once
//! its restarts run out. networkd says on D-Bus whenever one of its links
//! changes, and resolved whenever the servers it asks do (network.rs), and
//! this follows both, as wifid follows networkd. A burst of changes, as
//! joining a network makes, settles first (settle.rs). Then, if what the
//! machine can reach is different (reach.rs), each unit it was given is
//! looked at (revive.rs), all at once, so one whose check is slow keeps no
//! other waiting. It goes on hearing while they're looked at, and should
//! the network change again meanwhile, the looks start over from what it
//! is now, though anything already asked of systemd is carried through.
//!
//! It says in the journal what it does: each unit it starts, each it
//! reloads, and each it leaves down because the unit's check still fails.
//! What it found each time it looked it says at debug level, and only when
//! its configuration asks: journald drops a unit's debug lines only for the
//! system's manager, which alone tells it the unit's `LogLevelMax`.

mod config;
mod network;
mod reach;
mod revive;
mod settle;
mod systemd;

use std::error::Error;
use std::future::{self, Future};
use std::path::PathBuf;
use std::pin::Pin;
use std::process::ExitCode;

use futures_util::StreamExt;
use futures_util::stream::FuturesUnordered;
use tokio::time::{Instant, sleep_until};
use zbus::Connection;

use config::{Config, Unit};
use revive::{Action, Outcome, Units, Why};
use settle::Settling;

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    let Some(path) = std::env::args_os().nth(1).map(PathBuf::from) else {
        eprintln!("<3>usage: network-revive CONFIG");
        return ExitCode::from(2);
    };
    let config = match Config::read(&path) {
        Ok(config) => config,
        Err(error) => {
            eprintln!("<3>{error}");
            return ExitCode::from(2);
        }
    };
    match follow(&config).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("<3>{error}");
            ExitCode::FAILURE
        }
    }
}

async fn follow(config: &Config) -> Result<(), Box<dyn Error>> {
    let system = Connection::system().await?;
    let units = systemd::Units::new(&Connection::session().await?).await?;
    // Followed before networkd is first asked, so no change in between is
    // missed.
    let mut changes = network::changes(&system).await?;
    let mut reach = network::reach(&system).await;
    if reach.is_none() {
        eprintln!("<4>networkd isn't answering; waiting for it");
    }
    let mut looking: Pin<Box<dyn Future<Output = ()> + '_>> =
        Box::pin(look(&units, &config.units, Why::Started));
    let mut settling = Settling::default();
    loop {
        let due = settling.due();
        tokio::select! {
            change = changes.next() => match change {
                Some(()) => settling.heard(Instant::now()),
                None => return Err("the system bus went away".into()),
            },
            () = sleep_until(due.unwrap_or_else(Instant::now)), if due.is_some() => {
                settling.looked();
                let now = network::reach(&system).await;
                // Nothing is known while networkd is away, so nothing has
                // changed until it's back and says otherwise.
                let debug = |line: &str| {
                    if config.debug {
                        println!("<7>{line}");
                    }
                };
                match now {
                    None => debug("networkd isn't answering"),
                    Some(_) if now == reach => debug("the machine reaches what it did"),
                    Some(ref found) => {
                        debug(&format!("the machine reaches {found}"));
                        reach = now;
                        looking = Box::pin(look(&units, &config.units, Why::Changed));
                    }
                }
            }
            () = &mut looking => looking = Box::pin(future::pending()),
        }
    }
}

/// Looks at every unit, saying each outcome as it comes.
async fn look(units: &impl Units, wanted: &[Unit], why: Why) {
    let mut looks: FuturesUnordered<_> = wanted
        .iter()
        .map(|unit| async move { (unit, revive::revive(units, unit, why).await) })
        .collect();
    while let Some((unit, outcome)) = looks.next().await {
        say(&unit.name, why, &outcome);
    }
}

fn say(name: &str, why: Why, outcome: &Outcome) {
    let because = match why {
        Why::Changed => ", as the network changed",
        Why::Started => "",
    };
    let after = match why {
        Why::Changed => "after the network changed",
        Why::Started => "since network-revive started",
    };
    match outcome {
        Outcome::Left | Outcome::Kept { .. } => {}
        Outcome::Started(was) if was.active == "failed" => {
            println!("<5>started {name} again, which had failed{because}");
        }
        Outcome::Started(_) => println!("<5>started {name} now, not at its next try{because}"),
        Outcome::Reloaded => println!("<6>reloaded {name}{because}"),
        Outcome::Refused { why } => {
            let tried = revive::AGAIN
                .iter()
                .sum::<tokio::time::Duration>()
                .as_secs();
            println!("<4>left {name} down: its check still failed {tried} s {after}: {why}");
        }
        Outcome::Failed { action, error } => {
            let verb = match action {
                Action::Start => "start",
                Action::Reload => "reload",
            };
            println!("<4>couldn't {verb} {name}: {error}");
        }
    }
}
