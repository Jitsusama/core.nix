//! system: how the whole system stands, the machines of the realm, the
//! mycelium, the home network and the code, each from its owner's own
//! command (the crate's lib.rs says why).
//!
//! ```text
//! system          a line for each part and each machine, and where to go deeper
//! system --json   the whole record, as system.view/1
//! system machine NAME   one machine whole, and what can be done with it
//! system serve    the record held ready on its socket (serve.rs)
//! ```
//!
//! It takes the shape every command of ours does (docs/commands.md),
//! through the terminal kit.

mod ask;
mod said;
mod serve;
mod views;

use std::io;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use system::homes::Homes;
use system::kept::{self, Kept, Notes};
use system::parts;
use system::record::System;
use terminal_kit::{Context, Exit, Failure, Help};

/// The kernel's count of boot error records, once this process has read
/// it, so `system serve` asks the journal for it once (power.rs).
static FOUND: system::power::Found = system::power::Found::new();

fn main() -> ExitCode {
    terminal_kit::main(&help(), env!("CARGO_PKG_VERSION"), run)
}

fn help() -> Help {
    Help::new(
        "system",
        "how the whole system stands: the machines, the mycelium, the network and the code",
    )
    .example(
        "system",
        "a line for each part and each machine, and where each goes deeper",
    )
    .example("system --json", "the whole record, as system.view/1")
    .example(
        "system machine NAME",
        "one machine whole: its cells, network and code, and what can be done with it",
    )
    .example(
        "system serve --socket SOCKET",
        "the record held ready there for the views, read again every 15 s",
    )
    .exit(Exit::Usage, "the words are unknown")
    .exit(Exit::NotFound, "no part lists the machine named")
    .exit(
        Exit::Unavailable,
        "which machines there are couldn't be read",
    )
}

fn run(context: &Context) -> Result<(), Failure> {
    if let [serve, rest @ ..] = context.args.as_slice()
        && serve == "serve"
    {
        return serve::run(rest);
    }
    let written = match context.args.as_slice() {
        [] => {
            let (system, _) = read();
            if context.format().is_json() {
                context.answer(&system.json())
            } else {
                context.say(&views::system(&system, &context.out, context.room))
            }
        }
        [machine, name] if machine == "machine" => {
            let (system, _) = read();
            let missing = || match system.machines_unread() {
                Some(why) => Failure::new(
                    Exit::Unavailable,
                    "system couldn't read which machines there are",
                )
                .because(why)
                .try_this("mycelium machines"),
                None => Failure::new(
                    Exit::NotFound,
                    format!("no part lists a machine named {name}"),
                )
                .try_this("system"),
            };
            if context.format().is_json() {
                context.answer(&system.machine(name).ok_or_else(missing)?)
            } else {
                let drawn = views::machine(&system, name, &context.out, context.room, now())
                    .ok_or_else(missing)?;
                context.say(&drawn)
            }
        }
        _ => {
            return Err(Failure::new(Exit::Usage, "system doesn't know those words")
                .try_this("system --help"));
        }
    };
    written.or_else(|error| match error.kind() {
        // A reader that stopped early, as `head` does, ends the answer.
        io::ErrorKind::BrokenPipe => Ok(()),
        _ => Err(
            Failure::new(Exit::Failed, "system couldn't write its answer")
                .because(error.to_string()),
        ),
    })
}

/// The system as each owner says it now, and which owners it was read
/// from kept answers of.
pub(crate) fn read() -> (System, Notes) {
    let since_boot = std::fs::read_to_string("/proc/uptime")
        .ok()
        .and_then(|uptime| system::power::since_boot(&uptime));
    let mut asked: Vec<&[&str]> = vec![
        &["mycelium", "machines"],
        &["mycelium", "health"],
        &["net", "--json"],
        &["repos", "--json"],
    ];
    let journal_asked = system::power::worth_asking(&FOUND, since_boot);
    if journal_asked {
        asked.push(system::power::ASK);
    }
    let mut answers = ask::all(&asked);
    let journal = journal_asked.then(|| answers.pop()).flatten();
    // Each owner's answer stands on its last good one for a read missed,
    // as under load one may be; with no state home, none is kept. Each
    // realm keeps its own, as mycelium answers for the realm asked.
    let kept = state_home().map(|state| {
        let realm = std::env::var("MYCELIUM_REALM").unwrap_or_default();
        Kept::in_folder(state.join("system/kept").join(kept::realm_folder(&realm)))
    });
    let at = now();
    let mut used = asked
        .iter()
        .zip(answers)
        .map(|(words, answer)| match &kept {
            Some(kept) => kept.used(&words.join(" "), answer, at, clock),
            None => kept::Used { answer, note: None },
        });
    let mut answered = || {
        used.next()
            .unwrap_or_else(|| unreachable!("one answer is given for each command asked"))
    };
    let (machines, health, net, repos) = (answered(), answered(), answered(), answered());
    let notes = Notes {
        machines: machines.note,
        health: health.note,
        net: net.note,
        repos: repos.note,
    };
    let (mut machines, health, net, repos) =
        (machines.answer, health.answer, net.answer, repos.answer);
    let journal = system::power::remembered(&FOUND, journal);
    note_a_firmware_error(&mut machines, &journal, since_boot);
    let mut system = System {
        machine: terminal_kit::environment::machine(),
        read_at: terminal_kit::time::rfc3339(now()),
        parts: vec![
            parts::machines(
                &machines,
                &health,
                net.as_ref().ok(),
                repos.as_ref().ok(),
                backup(),
            ),
            parts::mycelium(&health),
            parts::network(&net),
            parts::code(&repos),
        ],
    };
    kept::mark(&mut system, &notes);
    if let Some(sites) = sites() {
        Homes::read(&sites).place(&mut system);
    }
    (system, notes)
}

/// Puts an error the firmware kept before this boot into this machine's
/// record, from the journal's answer (power.rs says why it's read here).
fn note_a_firmware_error(
    machines: &mut parts::Answer,
    journal: &parts::Answer,
    since_boot: Option<u64>,
) {
    if let Some(ago) = since_boot.and_then(|since| system::power::kept_before_boot(journal, since))
    {
        system::power::note(machines, ago);
    }
}

/// Where the local sites are kept on this machine (modules/local-sites),
/// under the data home.
fn sites() -> Option<PathBuf> {
    let data = std::env::var_os("XDG_DATA_HOME")
        .filter(|data| !data.is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share"))
        })?;
    Some(data.join("jitsusama/sites"))
}

/// When this machine's quest backup last backed up, as the backup writes it
/// under its state home (epoch seconds); none where this machine keeps no
/// backup.
fn backup() -> Option<parts::Backup> {
    let state = state_home()?;
    parts::backup(
        std::fs::read_to_string(state.join("quest-backup/landed")),
        now(),
    )
}

/// The state home, where a command keeps what outlives one run.
fn state_home() -> Option<PathBuf> {
    std::env::var_os("XDG_STATE_HOME")
        .filter(|state| !state.is_empty())
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/state")))
}

/// A moment as a time of day where the machine is, as `11:54`.
fn clock(at: i64) -> String {
    terminal_kit::local::at(at).map_or_else(
        || "an unknown time".to_owned(),
        |local| format!("{:02}:{:02}", local.hour, local.minute),
    )
}

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| {
            i64::try_from(since.as_secs()).unwrap_or(i64::MAX)
        })
}
