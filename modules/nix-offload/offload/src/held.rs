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
/// An output with no path, as a floating content-addressed one has, is
/// left out, since no store can say it holds it. A fixed one, a download
/// whose hash is known before it's fetched, is kept by the path [`fixed`]
/// reads, so one this machine holds can stand in for its fetch: on 9
/// November a lent builder was refused a Go zone's 25 modules by the
/// proxy this machine fetched them through, and built none of the zone.
/// `there` still has the builder fetch it first, since its own source
/// is far faster than this machine's uplink.
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
                    let path = output_path(shown, name, output)?;
                    Some((name.clone(), stored(&path)))
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

/// The paths `nix path-info --json` printed as valid that a builder whose
/// Nix doesn't trust this account takes, each with the size of its
/// archive: those signed, or content-addressed, which a store checks by
/// their content. It refuses the rest, `--no-check-sigs` or not, and
/// with them the whole copy they're in.
pub fn signed(info: &Value) -> BTreeMap<String, u64> {
    described(info)
        .into_iter()
        .filter(|(_, stored)| stored.signed)
        .map(|(path, stored)| (path, stored.size))
        .collect()
}

/// A path in this store as far as sending it goes: the size of its
/// archive, whether it's fixed, its content's hash known before it was
/// built, as a download's is, the derivation it was built from, and
/// whether a builder that trusts no account takes it, signed or
/// content-addressed.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Stored {
    pub size: u64,
    pub fixed: bool,
    pub deriver: Option<String>,
    pub signed: bool,
}

/// The valid paths `nix path-info --json` printed. It once printed a list
/// of objects that name their paths, and now prints an object of them by
/// path, an invalid one null, under `info` and named without the store's
/// folder in its second format. A fixed path's `ca` once read `fixed:` and a hash, and is now an
/// object naming its method, which a derivation's own text, the one kind
/// that isn't fixed, names `text`.
pub fn described(info: &Value) -> BTreeMap<String, Stored> {
    each(info)
        .into_iter()
        .map(|(path, info)| (path, stored_as(info)))
        .collect()
}

/// Whether every valid path `nix path-info --json` printed may come into a
/// store that checks signatures and trusts the keys `trusted`, each
/// `NAME:KEY` as `trusted-public-keys` names them: content-addressed, or
/// signed by a key of one of those names. It says so of none printed.
pub fn trusted_by(info: &Value, trusted: &[&str]) -> bool {
    let names: Vec<&str> = trusted
        .iter()
        .filter_map(|key| key.split_once(':').map(|(name, _)| name))
        .collect();
    let paths = each(info);
    !paths.is_empty()
        && paths.iter().all(|(_, info)| {
            info.get("ca").is_some_and(|ca| !ca.is_null())
                || info
                    .get("signatures")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                    .filter_map(|signature| signature.split_once(':'))
                    .any(|(name, _)| names.contains(&name))
        })
}

/// Each valid path `nix path-info --json` printed, with what it said of
/// it, in either of the formats [`described`] reads.
fn each(info: &Value) -> Vec<(String, &Value)> {
    if let Some(listed) = info.as_array() {
        return listed
            .iter()
            .filter(|info| info.get("valid").and_then(Value::as_bool) != Some(false))
            .filter_map(|info| Some((stored(info.get("path")?.as_str()?), info)))
            .collect();
    }
    let all = info.get("info").unwrap_or(info);
    all.as_object()
        .into_iter()
        .flatten()
        .filter(|(_, info)| info.is_object())
        .map(|(path, info)| (stored(path), info))
        .collect()
}

/// What `nix path-info --json` said of one path, as [`described`] keeps it.
fn stored_as(info: &Value) -> Stored {
    Stored {
        size: info.get("narSize").and_then(Value::as_u64).unwrap_or(0),
        fixed: match info.get("ca") {
            Some(Value::String(ca)) => ca.starts_with("fixed:"),
            Some(Value::Object(ca)) => ca.get("method").and_then(Value::as_str) != Some("text"),
            _ => false,
        },
        deriver: info.get("deriver").and_then(Value::as_str).map(stored),
        signed: info.get("ca").is_some_and(|ca| !ca.is_null())
            || info
                .get("signatures")
                .and_then(Value::as_array)
                .is_some_and(|signatures| !signatures.is_empty()),
    }
}

/// The fixed outputs of the derivations `nix derivation show` printed, by
/// their paths, each with the derivation that gives it. Now one shows no
/// path among its outputs, so it's read from the derivation's
/// environment, where its builder is told it, or for one with structured
/// attributes under `outputs` in the JSON its environment carries; before,
/// it had one beside its hash.
///
/// A fixed output is matched to what the builder plans by this path, not
/// by the deriver this store recorded for it: one path is given by every
/// derivation that downloads the same bytes, and the one recorded is
/// whichever fetched it first. Matched by deriver, a model's 16.5 GB of
/// downloads, fetched here before curl moved, went up through the tunnel
/// for two hours on 5 November rather than being fetched by the builder.
pub fn fixed(shown: &Value) -> BTreeMap<String, String> {
    let all = shown.get("derivations").unwrap_or(shown);
    let mut fixed = BTreeMap::new();
    for (derivation, shown) in all.as_object().into_iter().flatten() {
        for (name, output) in shown
            .get("outputs")
            .and_then(Value::as_object)
            .into_iter()
            .flatten()
            .filter(|(_, output)| output.get("hash").is_some())
        {
            if let Some(path) = output_path(shown, name, output) {
                fixed.insert(stored(&path), stored(derivation));
            }
        }
    }
    fixed
}

/// The path of output `name`, shown as `output`, of the derivation
/// `shown`: its own, or for a fixed one, which now shows none, the one
/// its builder is told. A floating one is told a placeholder, not a
/// path, and has none.
fn output_path(shown: &Value, name: &str, output: &Value) -> Option<String> {
    let own = output
        .get("path")
        .and_then(Value::as_str)
        .map(str::to_owned);
    if output.get("hash").is_none() {
        return own;
    }
    own.or_else(|| {
        shown
            .pointer(&format!("/env/{name}"))
            .and_then(Value::as_str)
            .map(str::to_owned)
    })
    .or_else(|| structured_output(shown, name))
}

/// The path of output `name` of a derivation with structured attributes,
/// whose environment carries them as JSON in `__json` rather than a
/// variable for each output.
fn structured_output(shown: &Value, name: &str) -> Option<String> {
    let carried = shown.pointer("/env/__json").and_then(Value::as_str)?;
    let attributes: Value = serde_json::from_str(carried).ok()?;
    attributes
        .pointer(&format!("/outputs/{name}"))
        .and_then(Value::as_str)
        .map(str::to_owned)
}

/// Of what sending would take up with it, `closure`, the fixed derivations
/// the builder is to fetch for itself first: those whose outputs it would
/// otherwise build, as [`fixed`] reads them from its plan, since no cache
/// it asks holds them, whether they're sent in their own right or only
/// referenced by what is. Sent, the 14 GB of a model's downloads took a day
/// through a tunnel on 4 November, where their own source gives them in
/// minutes.
pub fn fetched(
    fixed: &BTreeMap<String, String>,
    closure: &BTreeMap<String, Stored>,
) -> Vec<String> {
    let fetched: BTreeSet<String> = closure
        .iter()
        .filter(|(_, stored)| stored.fixed)
        .filter_map(|(path, _)| fixed.get(path).cloned())
        .collect();
    fetched.into_iter().collect()
}

/// How much of `closure` the builder fetches for itself: the outputs of
/// `fetching`, among the planned `fixed` ones.
pub fn fetched_size(
    fixed: &BTreeMap<String, String>,
    closure: &BTreeMap<String, Stored>,
    fetching: &[String],
) -> u64 {
    closure
        .iter()
        .filter(|(path, _)| fixed.get(*path).is_some_and(|it| fetching.contains(it)))
        .map(|(_, stored)| stored.size)
        .sum()
}

/// How much of `closure` goes up: what the builder would build of it, as
/// `planned`, or as `fixed` for a download, less what it fetched for
/// itself. The rest it holds, or fetches from a cache.
pub fn sent_up(
    planned: &BTreeSet<String>,
    fixed: &BTreeMap<String, String>,
    closure: &BTreeMap<String, Stored>,
    fetched: &[String],
) -> u64 {
    closure
        .iter()
        .filter(|(path, stored)| match fixed.get(*path) {
            Some(derivation) => !fetched.contains(derivation),
            None => {
                !stored.fixed
                    && stored
                        .deriver
                        .as_ref()
                        .is_some_and(|deriver| planned.contains(deriver))
            }
        })
        .map(|(_, stored)| stored.size)
        .sum()
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
    fn a_fixed_output_is_one_to_send_by_the_path_its_builder_is_told() {
        // Now one shows no path among its outputs, only in its builder's
        // environment, or with structured attributes in the JSON it
        // carries; before, it had one beside its hash. A floating one is
        // told a placeholder, never a path, so it's left out.
        let now = json!({"version": 4, "derivations": {
            "a.drv": {"outputs": {"out": {"hash": "sha256-AAAA", "method": "flat"}},
                      "env": {"out": "/nix/store/a"}, "inputs": {"drvs": {}}},
            "c.drv": {"outputs": {"out": {"hash": "sha256-CCCC", "method": "flat"}},
                      "env": {"__json": "{\"outputs\":{\"out\":\"/nix/store/c\"}}"},
                      "inputs": {"drvs": {}}},
            "b.drv": {"outputs": {"out": {"method": "nar", "hashAlgo": "sha256"}},
                      "env": {"out": "/1rz4g4znpzjwh1xymhjpm42vipw92pr73vdgl6xs1hycac8k"},
                      "inputs": {"drvs": {}}}}});
        let before = json!({"/nix/store/a.drv": {
            "outputs": {"out": {"path": "/nix/store/a", "hashAlgo": "sha256", "hash": "00"}},
            "inputDrvs": {}}});
        let a = BTreeMap::from([("out".to_owned(), "/nix/store/a".to_owned())]);
        for shown in [now.clone(), before] {
            assert_eq!(
                derivations(&shown)["/nix/store/a.drv"].outputs,
                a,
                "{shown}"
            );
        }
        assert_eq!(
            derivations(&now)["/nix/store/b.drv"].outputs,
            BTreeMap::new()
        );
        assert_eq!(
            derivations(&now)["/nix/store/c.drv"].outputs,
            BTreeMap::from([("out".to_owned(), "/nix/store/c".to_owned())])
        );
    }

    #[test]
    fn a_fixed_output_is_known_by_its_path_in_either_format() {
        // Now one shows its path only in its environment, or with
        // structured attributes in the JSON its environment carries;
        // before, it had one beside its hash. One that isn't fixed isn't
        // among them.
        let now = json!({"version": 4, "derivations": {
            "a.drv": {"outputs": {"out": {"hash": "sha256-AAAA", "method": "flat"}},
                      "env": {"out": "/nix/store/a"}, "inputs": {"drvs": {}}},
            "c.drv": {"outputs": {"out": {"hash": "sha256-CCCC", "method": "flat"}},
                      "env": {"__json": "{\"outputs\":{\"out\":\"/nix/store/c\"}}"},
                      "inputs": {"drvs": {}}},
            "b.drv": {"outputs": {"out": {"path": "b"}}, "env": {"out": "/nix/store/b"},
                      "inputs": {"drvs": {}}}}});
        let before = json!({"/nix/store/a.drv": {
            "outputs": {"out": {"path": "/nix/store/a", "hashAlgo": "sha256", "hash": "00"}},
            "inputDrvs": {}}});
        let a = ("/nix/store/a".to_owned(), "/nix/store/a.drv".to_owned());
        let c = ("/nix/store/c".to_owned(), "/nix/store/c.drv".to_owned());
        assert_eq!(fixed(&now), BTreeMap::from([a.clone(), c]));
        assert_eq!(fixed(&before), BTreeMap::from([a]));
    }

    #[test]
    fn what_the_builder_lacks_of_a_fixed_path_it_fetches_itself() {
        let info = json!({
            "/nix/store/model": {"narSize": 5, "ca": null, "deriver": "/nix/store/model.drv"},
            "/nix/store/weights": {"narSize": 14_000, "ca": "fixed:sha256:00",
                                   "deriver": "/nix/store/weights-before.drv"},
            "/nix/store/libc": {"narSize": 300, "ca": null, "deriver": "/nix/store/libc.drv"},
            "/nix/store/patch": {"narSize": 2, "ca": "fixed:r:sha256:00", "deriver": null}});
        let closure = described(&info);
        assert!(closure["/nix/store/weights"].fixed && !closure["/nix/store/model"].fixed);
        // The builder would build the model and its weights, and has libc.
        // This store recorded the weights as another derivation's, which
        // downloads the same bytes, so they're known by their path.
        let planned = set(&["/nix/store/model.drv", "/nix/store/weights.drv"]);
        let planned_fixed = BTreeMap::from([(
            "/nix/store/weights".to_owned(),
            "/nix/store/weights.drv".to_owned(),
        )]);
        let fetching = fetched(&planned_fixed, &closure);
        assert_eq!(fetching, ["/nix/store/weights.drv"]);
        assert_eq!(fetched_size(&planned_fixed, &closure, &fetching), 14_000);
        assert_eq!(sent_up(&planned, &planned_fixed, &closure, &fetching), 5);
        // A fetch that failed leaves the weights to go up too.
        assert_eq!(sent_up(&planned, &planned_fixed, &closure, &[]), 14_005);
        // Weights the builder already holds aren't planned, nor sent.
        assert_eq!(
            fetched(&BTreeMap::new(), &closure),
            [] as [std::string::String; 0]
        );
        assert_eq!(sent_up(&planned, &BTreeMap::new(), &closure, &[]), 5);
        // In the second format, a fixed path names its method; a
        // derivation's text isn't fixed.
        let info = json!({"version": 2, "info": {
            "weights": {"narSize": 1, "ca": {"method": "flat", "hash": "sha256-AA"},
                        "deriver": "weights.drv"},
            "a.drv": {"narSize": 1, "ca": {"method": "text", "hash": "sha256-AA"}}}});
        let closure = described(&info);
        assert!(closure["/nix/store/weights"].fixed && !closure["/nix/store/a.drv"].fixed);
        assert_eq!(
            closure["/nix/store/weights"].deriver.as_deref(),
            Some("/nix/store/weights.drv")
        );
    }

    #[test]
    fn what_this_store_holds_is_read_in_each_format_nix_has_given_it() {
        let wanted = BTreeMap::from([("/nix/store/a".to_owned(), 24)]);
        let formats = [
            json!({"version": 2, "info": {"a": {"narSize": 24, "signatures": ["k:A"]},
                                          "b": null}}),
            json!({"/nix/store/a": {"narSize": 24, "signatures": ["k:A"]},
                   "/nix/store/b": null}),
            json!([{"path": "/nix/store/a", "narSize": 24, "signatures": ["k:A"]},
                   {"path": "/nix/store/b", "valid": false}]),
        ];
        for info in formats {
            assert_eq!(signed(&info), wanted, "{info}");
        }
    }

    #[test]
    fn only_what_is_signed_or_content_addressed_goes_to_a_builder_that_trusts_no_account() {
        // Built here before this machine signed what it built, a path
        // has no signature, and a builder trusting no account refuses it.
        let info = json!({
            "/nix/store/built-before": {"narSize": 1, "ca": null, "signatures": []},
            "/nix/store/built-since": {"narSize": 2, "ca": null,
                                       "signatures": ["here-1:AAAA"]},
            "/nix/store/download": {"narSize": 3, "ca": "fixed:sha256:00"},
            "/nix/store/gone": null});
        assert_eq!(
            signed(&info),
            BTreeMap::from([
                ("/nix/store/built-since".to_owned(), 2),
                ("/nix/store/download".to_owned(), 3),
            ])
        );
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
        assert_eq!(
            instead(
                &planned,
                &derivations,
                &asked,
                &holding(&["/nix/store/kernel"])
            ),
            [] as [String; 0]
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
        assert_eq!(
            instead(&planned, &derivations, &asked, &held),
            [] as [std::string::String; 0]
        );
    }
}
