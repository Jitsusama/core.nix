//! Which check each failed derivation of a red builds, so its red is
//! judged as that check's.

use std::collections::{BTreeMap, BTreeSet};

use super::VM_RUN;

/// The check each of the `failed` derivations builds, of the checks
/// `built`: the one of its name, the VM test it runs, or the one `named`
/// says, asked only of names neither gives; none when one of them names
/// no check, since what broke can't then be told. Nor when a check the
/// riders `added`, which no failed derivation is named like, builds one
/// named like another check, or can't say what it builds: a flake of it
/// would then be judged as that other check, main's, and land.
pub(super) fn checks_of(
    failed: &[String],
    built: &[String],
    added: &BTreeSet<String>,
    named: &dyn Fn(&[String]) -> BTreeMap<String, String>,
) -> Option<Vec<String>> {
    let by_name = |name: &str| {
        let check = name.strip_prefix(VM_RUN).unwrap_or(name);
        built
            .iter()
            .any(|one| one == check)
            .then(|| check.to_owned())
    };
    let unplaced: Vec<String> = failed
        .iter()
        .filter(|name| by_name(name).is_none())
        .cloned()
        .collect();
    let others: Vec<String> = built
        .iter()
        .filter(|check| {
            !failed
                .iter()
                .any(|name| by_name(name).as_deref() == Some(check))
        })
        .cloned()
        .collect();
    let unsure: Vec<&String> = others
        .iter()
        .filter(|check| added.contains(*check))
        .collect();
    let named = if unplaced.is_empty() && unsure.is_empty() {
        BTreeMap::new()
    } else {
        named(&others)
    };
    let shared = unsure.iter().any(|check| {
        named
            .iter()
            .find(|(_, of)| of == check)
            .is_none_or(|(name, _)| failed.contains(name) && by_name(name).is_some())
    });
    if shared {
        return None;
    }
    failed
        .iter()
        .map(|name| by_name(name).or_else(|| named.get(name).cloned()))
        .collect()
}
