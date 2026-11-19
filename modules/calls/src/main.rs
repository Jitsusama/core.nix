//! Says whether a call is on, for whatever mustn't cut one, such as an
//! update with nobody at it.
//!
//! ```text
//! a-call-is-on [--here] [RUNTIMES]
//! ```
//!
//! It's a question in the terminal kit's shape (`help()`): it exits 0
//! while a call is on and 1 while none is, saying nothing, and 69 when it
//! can't tell. On Linux it reads each account's PipeWire under RUNTIMES,
//! /run/user unless named (src/pipewire.rs); on a Mac it asks Core Audio,
//! which answers for every account (src/coreaudio.rs).
//!
//! On Linux a call on another of the person's machines is a call too, as
//! the mycelium nodes under RUNTIMES hear it from their peers
//! (src/nodes.rs, and src/elsewhere.rs for how a record is read). A node
//! that won't answer, a peer whose probe said nothing lately, a peer
//! declaring no call probe and a peer whose record's time can't be read or
//! trusted are each named on stderr, and none is a wait, since whatever
//! asks would wait on it for ever. `--here` asks about this machine alone,
//! as the node's own probe and the shaper must, or two machines would each
//! hold up the other's call. A Mac's nodes keep their sockets where root
//! can't find them, so a Mac hears only itself.

#[cfg(target_os = "macos")]
mod coreaudio;
#[cfg(target_os = "linux")]
mod pipewire;

use std::process::ExitCode;

use terminal_kit::{Exit, Failure, Help};

/// What `a-call-is-on --help` says: how it's asked, and what it answers.
fn help() -> Help {
    Help::new("a-call-is-on", "says whether a call is on, by its exit")
        .example(
            "a-call-is-on",
            "here or on another of the person's machines",
        )
        .example("a-call-is-on --here", "on this machine alone")
        .example("a-call-is-on /run/user", "under these runtime folders")
        .option("--here", "asks about this machine alone, not its peers")
        .argument("[runtimes]", "the accounts' runtime folders, /run/user")
        .exit(Exit::Done, "a call is on")
        .exit(Exit::Failed, "no call is on")
        .exit(
            Exit::Unavailable,
            "this machine's sound server couldn't be read",
        )
}

/// What was heard: whether a call is on, and the cautions on whose calls
/// went unheard.
struct Heard {
    on: Result<bool, String>,
    cautions: Vec<String>,
}

fn main() -> ExitCode {
    terminal_kit::ask(&help(), env!("CARGO_PKG_VERSION"), |context| {
        let (here, folders) = read(&context.args)?;
        let runtimes = match folders.as_slice() {
            [] => None,
            [runtimes] => Some(*runtimes),
            _ => {
                return Err(
                    Failure::new(Exit::Usage, "a-call-is-on takes one folder at most")
                        .try_this("a-call-is-on --help"),
                );
            }
        };
        let heard = heard(runtimes, here);
        // Under JSON, stderr holds JSON alone, so the cautions, words for a
        // person, are left out there.
        if !context.format().is_json() {
            for caution in &heard.cautions {
                let _ = context.tell("caution", "warning", caution);
            }
        }
        heard.on.map_err(|why| {
            Failure::new(Exit::Unavailable, "it couldn't tell whether a call is on").because(why)
        })
    })
}

/// Whether `args` ask about here alone, and the folders they name. A flag
/// is read wherever it stands, up to a `--`; terminal-kit leaves every
/// program's --verbose to it, to say more of a failure.
fn read(args: &[String]) -> Result<(bool, Vec<&str>), Failure> {
    let mut here = false;
    let mut folders = Vec::new();
    let mut args = args.iter().map(String::as_str);
    while let Some(arg) = args.next() {
        match arg {
            "--here" => here = true,
            "--verbose" | "-v" => {}
            "--" => folders.extend(args.by_ref()),
            // A mistyped flag read as a folder would find no account and
            // answer no call without a word.
            flag if flag.starts_with('-') => {
                return Err(Failure::new(
                    Exit::Usage,
                    format!("a-call-is-on knows no {flag}; its own is --here"),
                )
                .try_this("a-call-is-on --help"));
            }
            folder => folders.push(folder),
        }
    }
    Ok((here, folders))
}

/// Whether a call is on here, or, unless `here` alone is asked, on a peer:
/// a call on a peer is a call, asked first since it spares PipeWire's two
/// seconds; otherwise PipeWire here says, and only it can't tell. Whose
/// calls went unheard is said beside.
#[cfg(target_os = "linux")]
fn heard(runtimes: Option<&str>, here: bool) -> Heard {
    let runtimes = std::path::Path::new(runtimes.unwrap_or("/run/user"));
    if here {
        return Heard {
            on: pipewire::on(runtimes),
            cautions: Vec::new(),
        };
    }
    let peers = calls::elsewhere::hear(runtimes);
    let cautions = cautions(&peers);
    let on = if peers.calls.is_empty() {
        pipewire::on(runtimes)
    } else {
        Ok(true)
    };
    Heard { on, cautions }
}

/// What's said of each peer and node whose calls went unheard.
#[cfg(target_os = "linux")]
fn cautions(peers: &calls::elsewhere::Heard) -> Vec<String> {
    let unseen = peers
        .unseen
        .iter()
        .map(|machine| format!("{machine} declares no call probe, so a call there goes unseen"));
    let untold = peers.untold.iter().map(|machine| {
        format!("{machine}'s call probe said nothing lately, so a call there goes unseen")
    });
    let undated = peers.undated.iter().map(|machine| {
        format!(
            "{machine}'s record is dated ahead of this clock or not at all, \
             so a call there goes unseen"
        )
    });
    let unanswered = peers
        .unanswered
        .iter()
        .map(|why| format!("{why}, so its peers' calls go unheard"));
    let unnamed = (peers.unnamed > 0).then(|| {
        let past = match peers.unnamed {
            1 => "1 machine past them goes".to_owned(),
            many => format!("{many} machines past them go"),
        };
        format!(
            "the nodes here list more than {} machines, so {past} unnamed",
            calls::elsewhere::MACHINES_AT_MOST
        )
    });
    unseen
        .chain(untold)
        .chain(undated)
        .chain(unanswered)
        .chain(unnamed)
        .collect()
}

#[cfg(target_os = "macos")]
fn heard(_runtimes: Option<&str>, _here: bool) -> Heard {
    Heard {
        on: coreaudio::on(),
        cautions: Vec::new(),
    }
}
