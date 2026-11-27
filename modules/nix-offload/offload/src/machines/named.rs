//! Who the file for any work names: the one lent while it answers, else
//! a builder standing in, and what a program told of it hears as that
//! changes.

use super::text;
use crate::proof::{self, State};
use crate::there;
use std::fs;
use std::path::{Path, PathBuf};

/// Who the file for any work names, as [`for_anything`] decides.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Named {
    /// The one lent here, which answers or is yet to be proven.
    Lent,
    /// The second lent builder, standing in.
    Second,
    /// This many warm VM builders, standing in.
    Vm(usize),
    /// One lent here that doesn't answer, or none since one was left out,
    /// with nothing to stand in.
    Unanswered,
    /// The one lent, whose way up stalled as every other lender's did at
    /// once, so the stall is this machine's own and it stays named.
    Stalled,
    /// None is lent, and nothing stands in.
    Nothing,
}

/// The file for any work's text, and who it names: the one lent here, as
/// `lent` names it for any work, while it answers or is yet to be proven,
/// as its proofs' `state` says; else the `second` lent builder's lines
/// beside it while that answers, else the warm `vm` builders' that do.
/// The one lent is named first while it's lent, since one failing once
/// may yet answer. One whose store is full is named only while nothing
/// stands in: it collects at its own min-free as it builds, which beats
/// no builder, but any with room comes first.
pub fn for_anything(
    lent: Option<&str>,
    (state, shared_stall): (State, bool),
    second: &[String],
    vm: &[String],
) -> (String, Named) {
    let answers = lent.is_some() && matches!(state, State::None | State::Unproven | State::Passing);
    let full = lent.is_some() && state == State::Full;
    let (stand_ins, named): (&[String], Named) = if answers {
        (&[], Named::Lent)
    } else if !second.is_empty() {
        (second, Named::Second)
    } else if !vm.is_empty() {
        (vm, Named::Vm(vm.len()))
    } else if full {
        (&[], Named::Lent)
    } else if lent.is_some() && shared_stall {
        (&[], Named::Stalled)
    } else if lent.is_some() || state == State::Dropped {
        (&[], Named::Unanswered)
    } else {
        (&[], Named::Nothing)
    };
    let lent = lent.filter(|_| !full || stand_ins.is_empty());
    let lines = lent.into_iter().chain(stand_ins.iter().map(String::as_str));
    (text(lines, None), named)
}

/// Say who the file for any work names, once it changes from `was`.
pub(super) fn say_named(was: Named, named: Named) {
    let stood_in = matches!(
        was,
        Named::Second | Named::Vm(_) | Named::Unanswered | Named::Stalled
    );
    let said = match named {
        _ if named == was => return,
        Named::Lent if stood_in => {
            "the lent builder answers again, so it alone is named for any work".to_owned()
        }
        Named::Second => {
            "no lent builder answers here; the second lent builder stands in".to_owned()
        }
        Named::Vm(1) => "no lent builder answers; 1 VM builder stands in".to_owned(),
        Named::Vm(count) => format!("no lent builder answers; {count} VM builders stand in"),
        Named::Unanswered => "no builder answers at all".to_owned(),
        Named::Stalled => "every lender's way up stalled at once, so the stall is this \
                           machine's own, and the lent builder stays named for any work"
            .to_owned(),
        Named::Nothing if stood_in => "nothing stands in any more, and none is lent".to_owned(),
        Named::Lent | Named::Nothing => return,
    };
    eprintln!("nix-offload: {said}");
}

/// What a program told of the file for any work hears as it changes from
/// `was` to `named`: `none` and why, once no builder answers any work, and
/// `back` and who, once one does or none is wanted; nothing otherwise.
pub(super) fn told(was: Named, named: Named) -> Option<(&'static str, String)> {
    let back = |who: &str| Some(("back", format!("a builder answers again: {who}")));
    match (was, named) {
        (Named::Unanswered, Named::Unanswered) => None,
        (Named::Unanswered, Named::Stalled) => back(
            "every lender's way up stalled at once, so the stall is this machine's own, and \
             the lent one stays named",
        ),
        (_, Named::Unanswered) => Some((
            "none",
            format!(
                "no builder answers: the lent one failed {} proofs in a row and none stands \
                 in, so another is asked for",
                proof::FAILS
            ),
        )),
        (Named::Unanswered, Named::Lent) => back("a newly lent one takes any work"),
        (Named::Unanswered, Named::Second) => back("the second lent builder stands in"),
        (Named::Unanswered, Named::Vm(1)) => back("1 VM builder stands in"),
        (Named::Unanswered, Named::Vm(count)) => back(&format!("{count} VM builders stand in")),
        (Named::Unanswered, Named::Nothing) => {
            Some(("back", "none is lent, and no build waits on one".to_owned()))
        }
        _ => None,
    }
}

/// Run `program` with `word` and `sentence`, waiting for it, and say in the
/// journal when it couldn't run or failed.
pub(super) fn tell(program: &Path, word: &str, sentence: &str) {
    let ran = std::process::Command::new(program)
        .args([word, sentence])
        .stdin(std::process::Stdio::null())
        .status();
    match ran {
        Ok(status) if status.success() => {}
        Ok(status) => eprintln!("nix-offload: {} ended {status}", program.display()),
        Err(error) => eprintln!("nix-offload: running {}: {error}", program.display()),
    }
}

/// The lines for any work of the builders that may stand in, the `second`
/// lent builder's while it passed its last proof, and each warm VM
/// builder's of `vm` that did, as of `at`.
pub(super) fn stand_ins(second: &Path, vm: &[PathBuf], at: u64) -> (Vec<String>, Vec<String>) {
    let lines = |dir: &Path| -> Vec<String> {
        fs::read_to_string(dir.join("machines-for-anything"))
            .map(|text| {
                text.lines()
                    .map(str::trim)
                    .filter(|line| !line.is_empty())
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default()
    };
    let second = if proof::passes_in(second, at) {
        lines(second)
    } else {
        Vec::new()
    };
    // A lane kept for the runs naming it stands in for nothing else, or
    // any build would reach it while the lent one fails.
    let vm = vm
        .iter()
        .filter(|dir| proof::passes_in(dir, at) && there::warm(dir) && !there::named_only(dir))
        .flat_map(|dir| lines(dir))
        .collect();
    (second, vm)
}

/// Whether the lenders in `second` and `vm` stall as the one lent does:
/// none passes its proofs and one at least built its last but couldn't
/// carry its copy up, fresh at `at`. Then the way up from here is what
/// fails, which no other builder would cure.
pub(super) fn stalled_alike(second: &Path, vm: &[PathBuf], at: u64) -> bool {
    let lenders = || std::iter::once(second).chain(vm.iter().map(PathBuf::as_path));
    !lenders().any(|dir| proof::passes_in(dir, at))
        && lenders().any(|dir| proof::stalls_in(dir, at))
}

#[cfg(test)]
#[path = "named_tests.rs"]
mod tests;
