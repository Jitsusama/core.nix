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
//! system's manager, which alone tells it the unit's `LogLevelMax`. Its
//! help, its failures and its exits take the terminal kit's shape
//! (`help()`).

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
use terminal_kit::{Exit, Failure, Help};
use tokio::time::{Instant, sleep_until};
use zbus::Connection;

use config::{Config, Unit};
use revive::{Action, Outcome, Units, Why};
use settle::Settling;

/// What `network-revive --help` says.
fn help() -> Help {
    Help::new(
        "network-revive",
        "gives units the network let down another go",
    )
    .example("network-revive units.json", "as its unit runs it")
    .argument("CONFIG", "the units to look at, as home.nix writes them")
    .exit(Exit::Failed, "the buses went away, or couldn't be reached")
    .exit(Exit::Config, "its configuration couldn't be read")
}

fn main() -> ExitCode {
    terminal_kit::main(&help(), env!("CARGO_PKG_VERSION"), |context| {
        let config = Config::read(&configuration(&context.args)?).map_err(|error| {
            Failure::new(Exit::Config, "network-revive can't read its configuration").because(error)
        })?;
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|error| failed(&error))?
            .block_on(follow(&config))
            .map_err(|error| failed(&*error))
    })
}

/// The one word it's started with, its configuration's path.
///
/// # Errors
///
/// Asked wrongly, 64, given no path, more than one, or a flag.
fn configuration(args: &[String]) -> Result<PathBuf, Failure> {
    let wrongly = |what: String| Failure::new(Exit::Usage, what).try_this("network-revive --help");
    if let Some(flag) = args.iter().find(|arg| arg.starts_with('-')) {
        return Err(wrongly(format!("{flag} isn't a flag it takes")));
    }
    match args {
        [path] => Ok(PathBuf::from(path)),
        [] => Err(wrongly("network-revive takes its configuration".into())),
        [_, extra, ..] => Err(wrongly(format!(
            "network-revive takes one configuration, and {extra} is another"
        ))),
    }
}

fn failed(error: &dyn Error) -> Failure {
    Failure::new(Exit::Failed, "network-revive stopped following the network")
        .because(error.to_string())
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

#[cfg(test)]
mod tests {
    use super::{configuration, help};
    use std::path::PathBuf;
    use terminal_kit::{Environment, Exit};

    /// How the kit ends network-revive started with `words`, its body
    /// reading only its configuration's path.
    fn started(words: &[&str]) -> (Exit, String) {
        let ending = terminal_kit::run(
            &help(),
            "0.1.0",
            &Environment::of([false; 3], &[]),
            words.iter().map(|word| (*word).to_owned()).collect(),
            |context| configuration(&context.args).map(drop),
        );
        (ending.exit, ending.stderr)
    }

    #[test]
    fn its_units_word_is_read_as_its_configuration() {
        let unit = ["/nix/store/x-network-revive.json"];
        let read = configuration(&unit.map(String::from)).unwrap();
        assert_eq!(
            read,
            PathBuf::from(unit[0]),
            "its unit's one word wasn't read as the configuration"
        );
        assert_eq!(
            started(&unit).0,
            Exit::Done,
            "the kit refused its unit's words"
        );
    }

    #[test]
    fn a_start_asked_wrongly_ends_64_naming_its_help() {
        for words in [
            &[][..],
            &["a.json", "b.json"],
            &["--debug", "a.json"],
            &["a.json", "-v"],
        ] {
            let (exit, said) = started(words);
            assert_eq!(
                exit,
                Exit::Usage,
                "{words:?} wasn't refused as asked wrongly: {said}"
            );
            assert!(
                said.contains("network-revive --help"),
                "{words:?}'s refusal didn't name its help: {said}"
            );
        }
    }
}
