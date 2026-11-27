//! What the train asks of lent builders to judge a red: a build of some
//! checks at a commit, and which check each derivation name builds. Kept
//! apart from the judging, which changes with what a red may excuse, while
//! this changes only with how nix-offload is asked.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::process::Command;

use serde_json::Value;

use super::super::gated::words_of;
use super::plainly_named;
use crate::state::short;

/// What the train asks of lent builders to judge a red: the real ones are
/// `Lenders`, and a test's record what each build asks for.
pub(in crate::train) trait Lent {
    /// Build `checks` of `flake`, at `commit`, from `checkout`, their
    /// words in `log`: the exit code and the words.
    fn build(
        &mut self,
        checkout: &Path,
        flake: &str,
        log: &Path,
        checks: &[&str],
        commit: &str,
    ) -> (Option<i32>, String);

    /// The check each derivation name of `checks` builds at `flake`.
    fn named(&self, checkout: &Path, flake: &str, checks: &[String]) -> BTreeMap<String, String>;
}

/// The lent builders, through nix-offload.
pub(in crate::train) struct Lenders;

impl Lent for Lenders {
    fn build(
        &mut self,
        checkout: &Path,
        flake: &str,
        log: &Path,
        checks: &[&str],
        commit: &str,
    ) -> (Option<i32>, String) {
        build(checkout, flake, log, checks, commit)
    }

    fn named(&self, checkout: &Path, flake: &str, checks: &[String]) -> BTreeMap<String, String> {
        named_at(checkout, flake, checks)
    }
}

/// Build `checks` of `flake`, at `commit`, from the train's `checkout`, on
/// a lent builder only, their words in `log`: its exit code and its words.
fn build(
    checkout: &Path,
    flake: &str,
    log: &Path,
    checks: &[&str],
    commit: &str,
) -> (Option<i32>, String) {
    let Ok(words) = fs::File::create(log) else {
        return (None, String::new());
    };
    let Ok(errors) = words.try_clone() else {
        return (None, String::new());
    };
    let named = match checks {
        [one] => (*one).to_owned(),
        many => format!("its {} checks", many.len()),
    };
    println!(
        "merge-net: the train builds {named} again at {}, its words in {}",
        short(commit),
        log.display()
    );
    let ran = Command::new("nix-offload")
        .args(["there", "--lent-only", "-L", flake])
        .args(
            checks
                .iter()
                .map(|check| format!("checks.{}.{check}", crate::SYSTEM)),
        )
        .current_dir(checkout)
        .env_remove("SSH_AUTH_SOCK")
        .stdout(words)
        .stderr(errors)
        .status();
    let code = ran.ok().and_then(|status| status.code());
    (code, words_of(log))
}

/// The check each derivation name of `checks` builds at `flake`, read on
/// a lent builder; nothing where it can't be read, or a check isn't
/// plainly named.
fn named_at(checkout: &Path, flake: &str, checks: &[String]) -> BTreeMap<String, String> {
    if !checks.iter().all(|check| plainly_named(check)) {
        return BTreeMap::new();
    }
    let names: Vec<String> = checks
        .iter()
        .map(|check| Value::from(check.as_str()).to_string())
        .collect();
    let apply = format!(
        "checks: builtins.listToAttrs (map (check: \
         {{ name = checks.${{check}}.name; value = check; }}) [ {} ])",
        names.join(" ")
    );
    let attribute = format!("{flake}#checks.{}", crate::SYSTEM);
    Command::new("nix-offload")
        .args(crate::world::eval_args(
            &attribute,
            &["--json", "--apply", &apply],
        ))
        .current_dir(checkout)
        .output()
        .ok()
        .filter(|ran| ran.status.success())
        .and_then(|ran| serde_json::from_slice::<BTreeMap<String, String>>(&ran.stdout).ok())
        .unwrap_or_default()
}
