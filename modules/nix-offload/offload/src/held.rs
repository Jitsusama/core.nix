//! Which outputs this machine holds that the lent builder would otherwise
//! build, worked out from what the builder says it would build, the
//! derivations as this machine's store shows them, and what this store
//! holds. `there` sends those up rather than have them built again: a
//! fresh builder compiled the running system's own kernel for over an
//! hour on 3 November, which this machine held and no cache did.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde_json::Value;

/// The derivations under the line of a dry run that says they will be
/// built, from as many such runs as `said` holds: those the builder can
/// neither find in its store nor fetch from a cache.
pub fn planned(said: &str) -> BTreeSet<String> {
    let mut building = false;
    let mut planned = BTreeSet::new();
    for line in said.lines() {
        if !line.starts_with(char::is_whitespace) {
            building = line.contains("will be built");
            continue;
        }
        let path = line.trim();
        // A store path's name is exact, so `.DRV` names no derivation.
        let derivation = Path::new(path)
            .extension()
            .is_some_and(|extension| extension == "drv");
        if building && path.starts_with("/nix/store/") && derivation {
            planned.insert(path.to_owned());
        }
    }
    planned
}

/// A derivation asked for, `path^outputs` or its path alone, by its path
/// with the outputs wanted of it, or none for all of them.
pub fn wanted(word: &str) -> (String, Option<BTreeSet<String>>) {
    match word.split_once('^') {
        None | Some((_, "*")) => (word.split('^').next().unwrap_or(word).to_owned(), None),
        Some((path, names)) => (
            path.to_owned(),
            Some(names.split(',').map(str::to_owned).collect()),
        ),
    }
}

/// A derivation as far as sending its outputs goes: each output's path
/// by its name, and which outputs of each derivation it's built from it
/// takes.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Derivation {
    pub outputs: BTreeMap<String, String>,
    pub inputs: BTreeMap<String, BTreeSet<String>>,
}

/// The derivations `nix derivation show` printed, by their paths. Its
/// format has changed: now they're under `derivations` and named without
/// the store's folder, with their inputs under `inputs.drvs`, where once
/// they stood alone with theirs under `inputDrvs`, each a list of outputs
/// or, later, an object holding one.
/// An output with no path, as a content-addressed one has, is left out,
/// since no store can say it holds it.
pub fn derivations(shown: &Value) -> BTreeMap<String, Derivation> {
    let all = shown.get("derivations").unwrap_or(shown);
    let Some(all) = all.as_object() else {
        return BTreeMap::new();
    };
    all.iter()
        .map(|(path, shown)| {
            let outputs = shown
                .get("outputs")
                .and_then(Value::as_object)
                .into_iter()
                .flatten()
                .filter_map(|(name, output)| {
                    let path = output.get("path")?.as_str()?;
                    Some((name.clone(), stored(path)))
                })
                .collect();
            let inputs = shown
                .pointer("/inputs/drvs")
                .or_else(|| shown.get("inputDrvs"))
                .and_then(Value::as_object)
                .into_iter()
                .flatten()
                .map(|(input, taken)| {
                    let taken = taken.get("outputs").unwrap_or(taken);
                    let names = taken
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(Value::as_str)
                        .map(str::to_owned)
                        .collect();
                    (stored(input), names)
                })
                .collect();
            (stored(path), Derivation { outputs, inputs })
        })
        .collect()
}

/// The paths `nix path-info --json` printed as valid, each with the size
/// of its archive. It once printed a list of objects that name their
/// paths, and now prints an object of them by path, an invalid one null,
/// under `info` and named without the store's folder in its second format.
pub fn held(info: &Value) -> BTreeMap<String, u64> {
    let size = |info: &Value| info.get("narSize").and_then(Value::as_u64).unwrap_or(0);
    if let Some(listed) = info.as_array() {
        return listed
            .iter()
            .filter(|info| info.get("valid").and_then(Value::as_bool) != Some(false))
            .filter_map(|info| Some((stored(info.get("path")?.as_str()?), size(info))))
            .collect();
    }
    let all = info.get("info").unwrap_or(info);
    all.as_object()
        .into_iter()
        .flatten()
        .filter(|(_, info)| info.is_object())
        .map(|(path, info)| (stored(path), size(info)))
        .collect()
}

/// A path in the store, whether it was named with the store's folder or
/// without.
fn stored(path: &str) -> String {
    if path.starts_with('/') {
        path.to_owned()
    } else {
        format!("/nix/store/{path}")
    }
}

/// The outputs to send up in place of building their derivations: of
/// those `planned` to be built, each whose outputs still wanted this
/// machine holds every one of. `asked` are the derivations asked for,
/// each with the outputs wanted of it, or none for all of them.
///
/// What's wanted of a derivation is what those built there take of it,
/// so they're settled first: one whose outputs go up wants nothing of its
/// inputs, and a kernel whose headers only an out-of-tree module takes
/// goes up without them once that module does. One no derivation there
/// takes anything of isn't built at all. A derivation the store didn't
/// show stays as it was, built there with whatever it's built from.
pub fn instead(
    planned: &BTreeSet<String>,
    derivations: &BTreeMap<String, Derivation>,
    asked: &[(String, Option<BTreeSet<String>>)],
    held: &BTreeMap<String, u64>,
) -> Vec<String> {
    // How many of those planned take something of each, which are
    // settled before it is.
    let mut takers: BTreeMap<&str, usize> = planned.iter().map(|path| (path.as_str(), 0)).collect();
    for path in planned {
        for input in derivations
            .get(path)
            .into_iter()
            .flat_map(|it| it.inputs.keys())
        {
            if let Some(count) = takers.get_mut(input.as_str()) {
                *count += 1;
            }
        }
    }
    let mut wanted: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for (path, outputs) in asked {
        let (Some(path), Some(derivation)) = (planned.get(path), derivations.get(path)) else {
            continue;
        };
        let names = wanted.entry(path.as_str()).or_default();
        match outputs {
            Some(outputs) => names.extend(outputs.iter().map(String::as_str)),
            None => names.extend(derivation.outputs.keys().map(String::as_str)),
        }
    }
    let mut settling: Vec<&str> = takers
        .iter()
        .filter(|(_, count)| **count == 0)
        .map(|(path, _)| *path)
        .collect();
    let mut sending = Vec::new();
    while let Some(path) = settling.pop() {
        let Some(derivation) = derivations.get(path) else {
            continue;
        };
        let names = wanted.remove(path).unwrap_or_default();
        let outputs: Option<Vec<&String>> = names
            .iter()
            .map(|name| {
                derivation
                    .outputs
                    .get(*name)
                    .filter(|path| held.contains_key(*path))
            })
            .collect();
        match outputs {
            _ if names.is_empty() => {}
            Some(outputs) => sending.extend(outputs.into_iter().cloned()),
            None => {
                for (input, taken) in &derivation.inputs {
                    if let Some(input) = planned.get(input) {
                        let names = wanted.entry(input.as_str()).or_default();
                        names.extend(taken.iter().map(String::as_str));
                    }
                }
            }
        }
        for input in derivation.inputs.keys() {
            if let Some(count) = takers.get_mut(input.as_str()) {
                *count -= 1;
                if *count == 0 {
                    settling.push(input);
                }
            }
        }
    }
    sending.sort();
    sending.dedup();
    sending
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn set(names: &[&str]) -> BTreeSet<String> {
        names.iter().map(|name| (*name).to_owned()).collect()
    }

    fn derivation(outputs: &[&str], inputs: &[(&str, &[&str])]) -> Derivation {
        Derivation {
            outputs: outputs
                .iter()
                .map(|name| ((*name).to_owned(), format!("/nix/store/{name}")))
                .collect(),
            inputs: inputs
                .iter()
                .map(|(path, names)| ((*path).to_owned(), set(names)))
                .collect(),
        }
    }

    fn holding(paths: &[&str]) -> BTreeMap<String, u64> {
        paths.iter().map(|path| ((*path).to_owned(), 1)).collect()
    }

    #[test]
    fn a_dry_run_says_what_it_would_build_apart_from_what_it_would_fetch() {
        let said = "these 2 derivations will be built:\n  /nix/store/a.drv\n  /nix/store/b.drv\n\
                    this path will be fetched (1 MiB download, 2 MiB unpacked):\n  /nix/store/c\n\
                    this derivation will be built:\n  /nix/store/d.drv\n";
        assert_eq!(
            planned(said),
            set(&["/nix/store/a.drv", "/nix/store/b.drv", "/nix/store/d.drv"])
        );
        assert!(planned("error: something\n  /nix/store/e.drv\n").is_empty());
    }

    #[test]
    fn derivations_are_read_in_each_format_nix_has_shown_them() {
        let wanted = BTreeMap::from([(
            "/nix/store/a.drv".to_owned(),
            Derivation {
                outputs: BTreeMap::from([("out".to_owned(), "/nix/store/a".to_owned())]),
                inputs: BTreeMap::from([("/nix/store/b.drv".to_owned(), set(&["dev"]))]),
            },
        )]);
        let now = json!({"version": 4, "derivations": {"a.drv": {
            "outputs": {"out": {"path": "a"}, "ca": {"method": "nar"}},
            "inputs": {"drvs": {"b.drv": {"outputs": ["dev"], "dynamicOutputs": {}}}}}}});
        let before = json!({"/nix/store/a.drv": {
            "outputs": {"out": {"path": "/nix/store/a"}},
            "inputDrvs": {"/nix/store/b.drv": {"outputs": ["dev"]}}}});
        let first = json!({"/nix/store/a.drv": {
            "outputs": {"out": {"path": "/nix/store/a"}},
            "inputDrvs": {"/nix/store/b.drv": ["dev"]}}});
        for shown in [now, before, first] {
            assert_eq!(derivations(&shown), wanted, "{shown}");
        }
    }

    #[test]
    fn what_this_store_holds_is_read_in_each_format_nix_has_given_it() {
        let wanted = BTreeMap::from([("/nix/store/a".to_owned(), 24)]);
        let formats = [
            json!({"version": 2, "info": {"a": {"narSize": 24}, "b": null}}),
            json!({"/nix/store/a": {"narSize": 24}, "/nix/store/b": null}),
            json!([{"path": "/nix/store/a", "narSize": 24},
                   {"path": "/nix/store/b", "valid": false}]),
        ];
        for info in formats {
            assert_eq!(held(&info), wanted, "{info}");
        }
    }

    #[test]
    fn a_derivation_goes_up_when_this_store_holds_every_output_wanted_of_it() {
        // The system takes the kernel, and a module its headers, which
        // this store lacks, so the kernel goes up only with the module.
        let planned = set(&[
            "/nix/store/system.drv",
            "/nix/store/kernel.drv",
            "/nix/store/module.drv",
        ]);
        let derivations = BTreeMap::from([
            (
                "/nix/store/system.drv".to_owned(),
                derivation(
                    &["system"],
                    &[
                        ("/nix/store/kernel.drv", &["kernel"]),
                        ("/nix/store/module.drv", &["module"]),
                    ],
                ),
            ),
            (
                "/nix/store/kernel.drv".to_owned(),
                derivation(&["kernel", "headers"], &[]),
            ),
            (
                "/nix/store/module.drv".to_owned(),
                derivation(&["module"], &[("/nix/store/kernel.drv", &["headers"])]),
            ),
        ]);
        let asked = [("/nix/store/system.drv".to_owned(), None)];
        assert_eq!(
            instead(
                &planned,
                &derivations,
                &asked,
                &holding(&["/nix/store/kernel", "/nix/store/module"])
            ),
            ["/nix/store/kernel", "/nix/store/module"]
        );
        // Without the module, the kernel is built there for its headers.
        assert!(
            instead(
                &planned,
                &derivations,
                &asked,
                &holding(&["/nix/store/kernel"])
            )
            .is_empty()
        );
        // Held whole, the system goes up alone and nothing under it is wanted.
        assert_eq!(
            instead(
                &planned,
                &derivations,
                &asked,
                &holding(&["/nix/store/system", "/nix/store/kernel"])
            ),
            ["/nix/store/system"]
        );
    }

    #[test]
    fn a_derivation_asked_for_whole_wants_all_its_outputs() {
        assert_eq!(
            wanted("/nix/store/a.drv"),
            ("/nix/store/a.drv".to_owned(), None)
        );
        assert_eq!(
            wanted("/nix/store/a.drv^*"),
            ("/nix/store/a.drv".to_owned(), None)
        );
        assert_eq!(
            wanted("/nix/store/a.drv^out,doc"),
            ("/nix/store/a.drv".to_owned(), Some(set(&["out", "doc"])))
        );
    }

    #[test]
    fn only_the_outputs_asked_for_are_wanted_of_what_was_asked() {
        let planned = set(&["/nix/store/a.drv"]);
        let derivations = BTreeMap::from([(
            "/nix/store/a.drv".to_owned(),
            derivation(&["out", "doc"], &[]),
        )]);
        let held = holding(&["/nix/store/out"]);
        let asked = [("/nix/store/a.drv".to_owned(), Some(set(&["out"])))];
        assert_eq!(
            instead(&planned, &derivations, &asked, &held),
            ["/nix/store/out"]
        );
        let asked = [("/nix/store/a.drv".to_owned(), None)];
        assert!(instead(&planned, &derivations, &asked, &held).is_empty());
    }
}
