//! `nix-offload cargo`: an agent's cargo, as a dev shell hands it over
//! (`modules/nix-offload/cargo.sh`), sent to a lent builder when it's a
//! whole suite of a committed tree, and otherwise run here.
//!
//! At 04:15 on 16 November this machine's processors were 70% stalled over
//! five minutes, from agents' cargo tests and compiles in jobs, while the
//! lent builders sat idle: whole suites going to `there --run` held only by
//! each agent's word. So a whole `cargo test`, `clippy` or `bench` goes
//! there by itself while a lent builder stands and the tree is committed,
//! as `there --run -- nix develop -c cargo ...`, the copy's own dev shell
//! giving the cargo it runs. A run asked offline is sent online, held to
//! its lockfile, since a lent builder holds no crates until it fetches them.
//!
//! What a run there can't stand for stays here: a build or a check, whose
//! target is wanted here; a named test, which is quick; a run whose output a
//! program reads (`--message-format`, as an editor's checks are); and one
//! whose words a shell would read differently there, since `--run` sends
//! them as words. A heavy one waits its turn in night-shift's, saying why,
//! as does a suite with no lent builder standing, with changes not
//! committed, or that no builder took. `NIX_OFFLOAD_CARGO=here` runs it
//! here at once, as it was asked.

use std::env;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Command, ExitCode, Stdio};

use crate::changed::NOT_TAKEN;

/// Subcommands whose whole run goes to a lent builder: what they say is
/// their output, so nothing they make is wanted here.
const SENT: [&str; 4] = ["test", "t", "clippy", "bench"];

/// Subcommands heavy enough to wait their turn, whose target is wanted
/// here, so they never go.
const HEAVY: [&str; 5] = ["build", "b", "check", "c", "doc"];

/// cargo's options that take the next word as their value.
const VALUED: [&str; 20] = [
    "-C",
    "--config",
    "-Z",
    "--color",
    "-p",
    "--package",
    "--exclude",
    "-F",
    "--features",
    "--target",
    "--target-dir",
    "--manifest-path",
    "-j",
    "--jobs",
    "--profile",
    "--bin",
    "--example",
    "--test",
    "--bench",
    "--message-format",
];

/// A test harness's options, after `--`, that take the next word, and
/// clippy's lint levels, which do too.
const HARNESS_VALUED: [&str; 12] = [
    "--test-threads",
    "--skip",
    "--format",
    "--color",
    "-Z",
    "--logfile",
    "--shuffle-seed",
    "-D",
    "-W",
    "-A",
    "-F",
    "--cap-lints",
];

/// Where an agent's cargo runs.
#[derive(Debug, PartialEq, Eq)]
pub enum Way {
    /// Here at once, as it was asked: light, or read by a program.
    As,
    /// Here, in its turn, saying why.
    Turn(&'static str),
    /// On a lent builder, if one stands and the tree is committed.
    There,
}

/// Where cargo given `words` runs, by the words alone.
pub fn way(words: &[String]) -> Way {
    let Some((at, subcommand)) = subcommand(words) else {
        return Way::As;
    };
    let after = &words[at + 1..];
    if words
        .iter()
        .any(|word| word == "--message-format" || word.starts_with("--message-format="))
    {
        return Way::As;
    }
    if HEAVY.contains(&subcommand) {
        return Way::Turn("what it builds is wanted here");
    }
    if !SENT.contains(&subcommand) {
        return Way::As;
    }
    if subcommand != "clippy" && named(after) {
        return Way::As;
    }
    if !words.iter().all(|word| plain(word)) {
        return Way::Turn("its words would read differently in a lent builder's shell");
    }
    Way::There
}

/// The subcommand `words` ask for, and where it is, past cargo's own
/// options and a `+toolchain`.
fn subcommand(words: &[String]) -> Option<(usize, &str)> {
    let mut at = 0;
    while let Some(word) = words.get(at) {
        if VALUED.contains(&word.as_str()) {
            at += 2;
        } else if word.starts_with('-') || word.starts_with('+') {
            at += 1;
        } else {
            return Some((at, word.as_str()));
        }
    }
    None
}

/// Whether a test or bench run's words, past its subcommand, name the
/// tests it runs: a filter before `--`, or one after it.
fn named(after: &[String]) -> bool {
    let (own, harness) = match after.iter().position(|word| word == "--") {
        Some(at) => (&after[..at], &after[at + 1..]),
        None => (after, &[][..]),
    };
    loose(own, &VALUED) || loose(harness, &HARNESS_VALUED)
}

/// Whether any of `words` is neither an option nor the value of one of
/// `valued`.
fn loose(words: &[String], valued: &[&str]) -> bool {
    let mut words = words.iter();
    while let Some(word) = words.next() {
        if valued.contains(&word.as_str()) {
            words.next();
        } else if !word.starts_with('-') {
            return true;
        }
    }
    false
}

/// Whether `word` reads the same as one word in any shell.
fn plain(word: &str) -> bool {
    !word.is_empty()
        && word
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "_./:=@+,%-".contains(c))
}

/// The words a lent builder's cargo is given for `words`, and whether the
/// run was asked offline: by `--offline`, `--frozen`, `net.offline=true`,
/// or `CARGO_NET_OFFLINE` here, `offline_here`, which doesn't go there. Such a
/// run goes `--locked` instead, since a lent builder has no crates but
/// what it fetches, and the lockfile holds every version it fetches to
/// what the run would have read here. A test harness's words, past `--`,
/// are its own and go as they are.
pub fn for_a_lender(words: &[String], offline_here: Option<&str>) -> (Vec<String>, bool) {
    let own = words
        .iter()
        .position(|word| word == "--")
        .unwrap_or(words.len());
    let (cargos, harness) = words.split_at(own);
    let mut sent = Vec::new();
    let mut offline = offline_here == Some("true");
    let mut locked = false;
    let mut cargos = cargos.iter();
    while let Some(word) = cargos.next() {
        match word.as_str() {
            "--offline" | "--frozen" | "--config=net.offline=true" => offline = true,
            "--config"
                if cargos
                    .as_slice()
                    .first()
                    .is_some_and(|v| v == "net.offline=true") =>
            {
                cargos.next();
                offline = true;
            }
            "--locked" => {
                if !locked {
                    sent.push(word.clone());
                }
                locked = true;
            }
            _ => sent.push(word.clone()),
        }
    }
    if offline && !locked {
        sent.push("--locked".to_owned());
    }
    sent.extend(harness.iter().cloned());
    (sent, offline)
}

/// What becomes of a suite that would go there: there, or here in its
/// turn and why, by whether a lent builder `stands` and the tree is
/// `committed`.
pub fn going(stands: bool, committed: bool) -> Way {
    if !stands {
        Way::Turn("no lent builder stands")
    } else if !committed {
        Way::Turn("the tree has changes not committed, and a lent builder runs only what is")
    } else {
        Way::There
    }
}

/// `nix-offload cargo --cargo PATH -- WORDS...`: cargo at PATH given
/// WORDS, run where [`way`] says, `lenders` being the lent builders'
/// folders. It ends with cargo's own code, wherever it ran.
pub fn cargo(arguments: &[String], lenders: [&Path; 2]) -> ExitCode {
    let Some((real, words)) = asked(arguments) else {
        eprintln!("nix-offload: cargo takes --cargo PATH -- WORDS...");
        return ExitCode::from(64);
    };
    let here = env::var_os("NIX_OFFLOAD_CARGO").is_some_and(|value| value == "here");
    let way = if here { Way::As } else { way(words) };
    let way = match way {
        Way::There => going(
            lenders
                .iter()
                .any(|lender| crate::standing(lender).is_some()),
            committed(),
        ),
        other => other,
    };
    let what = subcommand(words).map_or("", |(_, subcommand)| subcommand);
    match way {
        Way::As => exited(&Command::new(real).args(words).exec()),
        Way::Turn(why) => turn(real, words, what, why),
        Way::There => {
            eprintln!(
                "nix-offload: cargo {what} of the whole tree goes to a lent builder, as \
                 there --run sends it; NIX_OFFLOAD_CARGO=here runs it here"
            );
            let offline_here = env::var("CARGO_NET_OFFLINE").ok();
            let (sent, offline) = for_a_lender(words, offline_here.as_deref());
            if offline {
                eprintln!(
                    "nix-offload: asked offline, it goes online there, held to the lockfile, \
                     since a lent builder has no crates until it fetches them"
                );
            }
            let status = Command::new(env::current_exe().unwrap_or_else(|_| "nix-offload".into()))
                .args(["there", "--run", "--", "nix", "develop", "-c", "cargo"])
                .args(&sent)
                .status();
            match status.ok().and_then(|status| status.code()) {
                Some(code) if code == i32::from(NOT_TAKEN) => {
                    turn(real, words, what, "no lent builder took it")
                }
                Some(code) => ExitCode::from(u8::try_from(code).unwrap_or(1)),
                None => ExitCode::from(1),
            }
        }
    }
}

/// The cargo and its words `arguments` give.
fn asked(arguments: &[String]) -> Option<(&str, &[String])> {
    match arguments {
        [flag, real, dashes, words @ ..] if flag == "--cargo" && dashes == "--" => {
            Some((real.as_str(), words))
        }
        _ => None,
    }
}

/// cargo run here in night-shift's turn, said why, or at once where
/// there's no night-shift, as on a machine that has none.
fn turn(real: &str, words: &[String], what: &str, why: &str) -> ExitCode {
    eprintln!("nix-offload: cargo {what} runs here in its turn, since {why}");
    let night_shift = Command::new("night-shift")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success());
    if night_shift {
        exited(
            &Command::new("night-shift")
                .args(["turn", "cargo", "--", real])
                .args(words)
                .exec(),
        )
    } else {
        exited(&Command::new(real).args(words).exec())
    }
}

/// What a failed exec ends with, said.
fn exited(error: &std::io::Error) -> ExitCode {
    eprintln!("nix-offload: cargo couldn't start: {error}");
    ExitCode::from(69)
}

/// Whether the tree here holds nothing git would call a change, untracked
/// files included, since a lent builder's copy has none of them.
fn committed() -> bool {
    Command::new("git")
        .args(["status", "--porcelain", "--untracked-files=normal"])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .is_ok_and(|out| out.status.success() && out.stdout.is_empty())
}

#[cfg(test)]
mod tests;
