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
//! that won't answer, or a peer whose probe couldn't tell, can't tell; a
//! peer declaring no call probe is named on stderr as unseen, and is no
//! wait. `--here` asks about this machine alone, as the node's own probe
//! and the shaper must, or two machines would each hold up the other's
//! call. A Mac's nodes keep their sockets where root can't find them, so a
//! Mac hears only itself.

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
            "a sound server or a node couldn't be read",
        )
}

/// What was heard: whether a call is on, and the peers whose calls go
/// unseen.
struct Heard {
    on: Result<bool, String>,
    unseen: Vec<String>,
}

fn main() -> ExitCode {
    terminal_kit::ask(&help(), env!("CARGO_PKG_VERSION"), |context| {
        let (here, runtimes) = match context.args.as_slice() {
            [flag, rest @ ..] if flag == "--here" => (true, rest),
            rest => (false, rest),
        };
        let runtimes = match runtimes {
            [] => None,
            [runtimes] => Some(runtimes.as_str()),
            _ => {
                return Err(
                    Failure::new(Exit::Usage, "a-call-is-on takes one folder at most")
                        .try_this("a-call-is-on --help"),
                );
            }
        };
        let heard = heard(runtimes, here);
        for machine in &heard.unseen {
            let _ = context.tell(
                "caution",
                "warning",
                &format!("{machine} declares no call probe, so a call there goes unseen"),
            );
        }
        heard.on.map_err(|why| {
            Failure::new(Exit::Unavailable, "it couldn't tell whether a call is on").because(why)
        })
    })
}

/// Whether a call is on here, or, unless `here` alone is asked, on a peer:
/// a call anywhere is a call, and anything that couldn't tell, with no
/// call heard, can't tell. The peers are asked first, since a call heard
/// there spares PipeWire's two seconds.
#[cfg(target_os = "linux")]
fn heard(runtimes: Option<&str>, here: bool) -> Heard {
    let runtimes = std::path::Path::new(runtimes.unwrap_or("/run/user"));
    if here {
        return Heard {
            on: pipewire::on(runtimes),
            unseen: Vec::new(),
        };
    }
    let (peers, unseen) = peers(runtimes);
    let on = match peers {
        Ok(true) => Ok(true),
        peers => match (pipewire::on(runtimes), peers) {
            (Ok(true), _) => Ok(true),
            (Err(why), _) | (_, Err(why)) => Err(why),
            (Ok(false), Ok(_)) => Ok(false),
        },
    };
    Heard { on, unseen }
}

/// Whether the nodes under `runtimes` hear a call on a peer, and the peers
/// declaring no call probe.
#[cfg(target_os = "linux")]
fn peers(runtimes: &std::path::Path) -> (Result<bool, String>, Vec<String>) {
    let heard = match calls::elsewhere::hear(runtimes) {
        Ok(heard) => heard,
        Err(why) => return (Err(why), Vec::new()),
    };
    let on = if !heard.calls.is_empty() {
        Ok(true)
    } else if heard.untold.is_empty() {
        Ok(false)
    } else {
        Err(format!(
            "{} declares a call probe but said no yes or no lately",
            heard.untold.join(", ")
        ))
    };
    (on, heard.unseen)
}

#[cfg(target_os = "macos")]
fn heard(_runtimes: Option<&str>, _here: bool) -> Heard {
    Heard {
        on: coreaudio::on(),
        unseen: Vec::new(),
    }
}
