//! `nix-offload prove-standing --config PATH`: proves, as root, each
//! standing builder the lines settings at PATH name, through the daemon's
//! build hook, and says how each stands in [`proof::STANDING`].
//!
//! A standing builder is reached only by root's ssh, so the one proof that
//! says it builds goes through the daemon's build hook with a line of its
//! own, which the daemon takes only from an account it trusts. Run by the
//! lender, that kept the lender's account trusted. Root runs it instead,
//! and the lender reads what it says, leaving a builder out as before
//! while its proofs fail or its store is under the floor. No key moves:
//! the account root's key logs in as on a standing builder is trusted
//! there, since the build hook sends derivations whole, so anything else
//! holding that key could put any path in the builder's store.
//!
//! What it writes, every [`PACE`], readable by anyone and written by root
//! alone:
//!
//! ```json
//! {"at": 1794391062, "builders": [{"uri": "ssh-ng://work-linux",
//!   "builder": "work-linux", "state": "passing", "lastSuccess": 1794391050,
//!   "lastFailure": null, "consecutiveFailures": 0, "freeBytes": 81604378624}]}
//! ```

use crate::proof::{self, Health, Keeper};
use serde_json::{Value, json};
use std::fs;
use std::path::Path;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// How often it looks for a proof that ended or is due: each builder is
/// still proven only every [`proof::EVERY`].
const PACE: Duration = Duration::from_secs(2);

/// A standing builder, by its line's fields the prover needs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Standing {
    pub uri: String,
    pub key: String,
    pub host_key: String,
    /// The systems it builds for, the first of which its proof is for.
    pub systems: Vec<String>,
}

impl Standing {
    /// The machines line its proofs go through: one build at a time, with
    /// no features asked of it.
    pub fn line(&self) -> String {
        let systems = if self.systems.is_empty() {
            "-".to_owned()
        } else {
            self.systems.join(",")
        };
        format!(
            "{} {systems} {} 1 1 - - {}",
            self.uri, self.key, self.host_key
        )
    }
}

/// The standing builders in lines' settings, `text`, each with the
/// systems it says, or this machine's own where it says none.
pub fn standing(text: &str) -> Result<Vec<Standing>, String> {
    let value: Value = serde_json::from_str(text).map_err(|error| error.to_string())?;
    let field = |item: &Value, name: &str| {
        item[name]
            .as_str()
            .map(str::to_owned)
            .ok_or(format!("a standing builder lacks {name}"))
    };
    value["standing"]
        .as_array()
        .ok_or("standing is missing")?
        .iter()
        .map(|item| {
            let systems = item["systems"].as_array().map_or_else(
                || vec![here()],
                |systems| {
                    systems
                        .iter()
                        .filter_map(Value::as_str)
                        .map(str::to_owned)
                        .collect()
                },
            );
            Ok(Standing {
                uri: field(item, "uri")?,
                key: field(item, "key")?,
                host_key: field(item, "hostKey")?,
                systems,
            })
        })
        .collect()
}

/// The system this machine builds for, as Nix names it.
fn here() -> String {
    format!("{}-{}", std::env::consts::ARCH, std::env::consts::OS)
}

fn seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs())
}

/// What it says of `kept` at `at`, as JSON.
pub fn said(kept: &[(Standing, Keeper)], at: u64) -> Value {
    let builders: Vec<Value> = kept
        .iter()
        .map(|(standing, keeper)| {
            let mut health = keeper.health.json();
            health["uri"] = standing.uri.clone().into();
            health
        })
        .collect();
    json!({ "at": at, "builders": builders })
}

/// Writes `said` to `path` whole, through a file beside it, so a lender
/// never reads half of it.
fn put(path: &Path, said: &Value) {
    let writing = path.with_extension("writing");
    let written =
        fs::write(&writing, format!("{said}\n")).and_then(|()| fs::rename(&writing, path));
    if let Err(error) = written {
        eprintln!("nix-offload: writing {}: {error}", path.display());
    }
}

/// Proves each of `builders` once due, with `nix`, and writes how they
/// stand to `path`, once.
pub fn prove_once(kept: &mut [(Standing, Keeper)], nix: &str, path: &Path, now: Instant) {
    for (standing, keeper) in kept.iter_mut() {
        let (line, nix) = (standing.line(), nix.to_owned());
        let was = keeper.health.state();
        keeper.keep(now, seconds(), move || {
            proof::through_daemon(&line, &nix, proof::WITHIN)
        });
        let is = keeper.health.state();
        if was != is {
            eprintln!(
                "nix-offload: {} is {} now",
                keeper.health.builder,
                is.name()
            );
        }
    }
    put(path, &said(kept, seconds()));
}

/// Proves `builders` for as long as it runs, writing to `path`.
pub fn serve(builders: Vec<Standing>, nix: &str, path: &Path) -> ! {
    let mut kept = going_on(builders, &proof::earlier(path, "builders"));
    loop {
        prove_once(&mut kept, nix, path, Instant::now());
        std::thread::sleep(PACE);
    }
}

/// A keeper for each of `builders`, going on from how `earlier`, what
/// the prover before this one last wrote, said it stood: the prover starts
/// again with each switch that changes its settings, and one starting
/// afresh would say a full builder is unproven, so lenders would name it.
fn going_on(builders: Vec<Standing>, earlier: &[Value]) -> Vec<(Standing, Keeper)> {
    builders
        .into_iter()
        .map(|standing| {
            let health = earlier
                .iter()
                .find(|said| said["uri"] == standing.uri.as_str())
                .and_then(Health::from_json)
                .unwrap_or_else(|| Health::of(&proof::named(&standing.uri)));
            (standing, Keeper::going_on(health))
        })
        .collect()
}

/// A standing builder's health from a prover's, for tests elsewhere.
#[cfg(test)]
pub fn health_of(said: &Value, uri: &str) -> Option<Health> {
    said["builders"]
        .as_array()?
        .iter()
        .find(|builder| builder["uri"] == uri)
        .and_then(Health::from_json)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SETTINGS: &str = r#"{"standing": [
        {"uri": "ssh-ng://work-linux", "key": "/etc/nix/offload_ed25519",
         "hostKey": "SG9zdA==", "systems": ["x86_64-linux", "aarch64-linux"]},
        {"uri": "ssh-ng://beside", "key": "/etc/nix/offload_ed25519", "hostKey": "T3RoZXI="}
    ]}"#;

    #[test]
    fn each_standing_builder_is_read_with_its_systems_or_this_machines() {
        let standing = standing(SETTINGS).expect("settings that read");
        assert_eq!(standing.len(), 2, "both builders: {standing:?}");
        assert_eq!(
            standing[0].line(),
            "ssh-ng://work-linux x86_64-linux,aarch64-linux /etc/nix/offload_ed25519 1 1 - - \
             SG9zdA==",
            "the first's line, both its systems first"
        );
        assert_eq!(
            standing[1].systems,
            [here()],
            "one naming none builds for this one's"
        );
    }

    #[test]
    fn settings_without_standing_builders_are_refused() {
        assert!(standing("{}").is_err(), "no standing list");
        assert!(
            standing(r#"{"standing": [{"uri": "ssh-ng://x"}]}"#).is_err(),
            "an entry without a key or host key"
        );
    }

    #[test]
    fn what_it_says_reads_back_by_uri() {
        let standing = standing(SETTINGS).expect("settings that read");
        let mut keeper = Keeper::of("work-linux");
        keeper.health.heard(
            proof::Proven {
                passed: true,
                free: Some(6_000_000_000),
                ..proof::Proven::default()
            },
            100,
        );
        let kept = vec![(standing[0].clone(), keeper)];
        let said = said(&kept, 120);
        assert_eq!(said["at"], 120, "when it was written: {said}");
        let health = health_of(&said, "ssh-ng://work-linux").expect("the builder");
        assert_eq!(
            health.state(),
            proof::State::Full,
            "under the floor: {said}"
        );
        assert_eq!(health.free, Some(6_000_000_000), "its free bytes: {said}");
        assert!(
            health_of(&said, "ssh-ng://beside").is_none(),
            "one it never kept"
        );
        let started = going_on(standing, said["builders"].as_array().expect("builders"));
        assert_eq!(
            started[0].1.health.state(),
            proof::State::Full,
            "a prover started again goes on from the full one: {said}"
        );
        assert_eq!(
            started[1].1.health.state(),
            proof::State::Unproven,
            "and one it never kept is unproven"
        );
    }
}
