//! An audit of every password the managers keep, run only when Joel asks:
//! which entries share a password, and which passwords Have I Been Pwned
//! has seen in a breach, worked out in the daemon's memory alone.
//!
//! Each password is hashed with SHA-1 as it's taken, and dropped there.
//! Entries whose hashes are the same share a password, each counting the
//! others. A breach is asked of Have I Been Pwned's range API by the first
//! five characters of the hash alone (k-anonymity: hundreds of hashes share
//! any five), padded so the answer's size says nothing either, and the
//! other thirty-five are matched here against the `SUFFIX:COUNT` lines it
//! answers, padding's lines counting 0. Nothing past the five characters
//! leaves the machine, and once the counts are found the hashes are
//! dropped too: what's kept of an entry is its [`Health`], two counts.

use std::collections::{BTreeSet, HashMap};
use std::fmt::Write;
use std::time::Duration;

use ring::digest::{SHA1_FOR_LEGACY_USE_ONLY, digest};
use serde_json::{Value, json};
use ureq::Agent;

/// Where Have I Been Pwned answers a range of hashes, each asked by its
/// five characters after it, unless `$PASSWORDS_BREACHES` says otherwise,
/// as the tests' stand-in does.
pub const BREACHES: &str = "https://api.pwnedpasswords.com/range/";

/// How many characters of a hash go to Have I Been Pwned: the range's.
pub const PREFIX: usize = 5;

/// How many ranges are asked at once: a few hundred passwords make a few
/// hundred ranges, each a short answer, so a handful at a time ends in
/// seconds without leaning on the service.
const AT_ONCE: usize = 8;

/// How long one range may take before the audit says Have I Been Pwned
/// couldn't be reached.
const PATIENCE: Duration = Duration::from_secs(15);

/// An entry's health as an audit found it: how many other entries share
/// its password, and how many times Have I Been Pwned has seen it in a
/// breach, or none when it couldn't be asked. Never a secret.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Health {
    pub reused: u64,
    pub breached: Option<u64>,
}

impl Health {
    /// As a record carries it: `{"reused", "breached"}`, breached left out
    /// where it couldn't be asked, so nothing is guessed.
    pub fn said(&self) -> Value {
        match self.breached {
            Some(breached) => json!({"reused": self.reused, "breached": breached}),
            None => json!({"reused": self.reused}),
        }
    }

    /// A health as [`Health::said`] says it, read back from what the daemon
    /// keeps of its last check, or none for anything else.
    pub fn heard(said: &Value) -> Option<Health> {
        Some(Health {
            reused: said.get("reused")?.as_u64()?,
            breached: said.get("breached").and_then(Value::as_u64),
        })
    }

    /// Whether it needs Joel's attention: shared, or seen in a breach.
    pub fn troubled(&self) -> bool {
        self.reused > 0 || self.breached.is_some_and(|breached| breached > 0)
    }

    /// Whether it was seen in a breach.
    pub fn breached(&self) -> bool {
        self.breached.is_some_and(|breached| breached > 0)
    }
}

/// What an audit found: each entry's health by its key, and why Have I
/// Been Pwned couldn't be asked, if it couldn't.
#[derive(Debug, Default)]
pub struct Audited {
    pub health: HashMap<String, Health>,
    pub unreached: Option<String>,
}

/// A password's SHA-1, in upper-case hexadecimal as Have I Been Pwned
/// writes its hashes.
pub fn hash(password: &str) -> String {
    digest(&SHA1_FOR_LEGACY_USE_ONLY, password.as_bytes())
        .as_ref()
        .iter()
        .fold(String::with_capacity(40), |mut hex, byte| {
            let _ = write!(hex, "{byte:02X}");
            hex
        })
}

/// How many times a range's answer says the hash ending in `suffix` was
/// seen: its `SUFFIX:COUNT` line's count, 0 for padding's lines, which
/// count 0, and for a suffix it doesn't list.
pub fn seen(range: &str, suffix: &str) -> u64 {
    range
        .lines()
        .filter_map(|line| line.trim().split_once(':'))
        .find(|(listed, _)| listed.eq_ignore_ascii_case(suffix))
        .and_then(|(_, count)| count.trim().parse().ok())
        .unwrap_or(0)
}

/// How many other entries share each one's hash, by its key.
pub fn shared(hashed: &[(String, String)]) -> HashMap<String, u64> {
    let mut sharing: HashMap<&str, u64> = HashMap::new();
    for (_, hash) in hashed {
        *sharing.entry(hash.as_str()).or_default() += 1;
    }
    hashed
        .iter()
        .map(|(key, hash)| (key.clone(), sharing[hash.as_str()] - 1))
        .collect()
}

/// One range of hashes from Have I Been Pwned, at `breaches` followed by
/// the five characters, padded: the only thing an audit sends anywhere.
/// What fails says so without the address, which holds the five.
pub fn range(agent: &Agent, breaches: &str, prefix: &str) -> Result<String, String> {
    let mut answer = agent
        .get(format!("{breaches}{prefix}"))
        .header("Add-Padding", "true")
        .call()
        .map_err(|error| format!("Have I Been Pwned couldn't be reached: {error}"))?;
    let status = answer.status().as_u16();
    if status != 200 {
        return Err(format!("Have I Been Pwned answered {status}"));
    }
    answer
        .body_mut()
        .read_to_string()
        .map_err(|error| format!("Have I Been Pwned's answer couldn't be read: {error}"))
}

/// Every range these prefixes name, a few at a time, each by its prefix.
fn ranges(breaches: &str, prefixes: &[&str]) -> HashMap<String, Result<String, String>> {
    let agent: Agent = Agent::config_builder()
        .timeout_global(Some(PATIENCE))
        .http_status_as_error(false)
        .build()
        .into();
    let share = prefixes.len().div_ceil(AT_ONCE).max(1);
    std::thread::scope(|scope| {
        let asking: Vec<_> = prefixes
            .chunks(share)
            .map(|some| {
                let agent = agent.clone();
                scope.spawn(move || {
                    some.iter()
                        .map(|prefix| ((*prefix).to_owned(), range(&agent, breaches, prefix)))
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        asking
            .into_iter()
            .flat_map(|thread| thread.join().unwrap_or_default())
            .collect()
    })
}

/// Audits these passwords, each by its entry's key, asking Have I Been
/// Pwned at `breaches`. Each password is hashed and dropped as it's taken,
/// and the hashes as this returns, so only the counts outlive it.
pub fn audit(passwords: Vec<(String, String)>, breaches: &str) -> Audited {
    // Each password goes out of reach as its closure ends, hashed.
    let hashed: Vec<(String, String)> = passwords
        .into_iter()
        .map(|(key, password)| (key, hash(&password)))
        .collect();
    let reused = shared(&hashed);
    let prefixes: Vec<&str> = hashed
        .iter()
        .map(|(_, hash)| &hash[..PREFIX])
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let ranges = ranges(breaches, &prefixes);
    let mut audited = Audited::default();
    for (key, hash) in &hashed {
        let times = match ranges.get(&hash[..PREFIX]) {
            Some(Ok(range)) => Some(seen(range, &hash[PREFIX..])),
            Some(Err(why)) => {
                audited.unreached.get_or_insert_with(|| why.clone());
                None
            }
            None => None,
        };
        let health = Health {
            reused: reused.get(key).copied().unwrap_or(0),
            breached: times,
        };
        audited.health.insert(key.clone(), health);
    }
    audited
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_hash_is_sha1_in_upper_case() {
        assert_eq!(hash("abc"), "A9993E364706816ABA3E25717850C26C9CD0D89D");
        assert_eq!(hash("password"), "5BAA61E4C9B93F3F0682250B6CF8331B7EE68FD8");
    }

    #[test]
    fn a_range_is_matched_by_suffix_and_its_padding_counts_nothing() {
        let range = "0018A45C4D1DEF81644B54AB7F969B88D65:1\r\n\
                     1E4C9B93F3F0682250B6CF8331B7EE68FD8:9659365\r\n\
                     FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF:0\r\n";
        assert_eq!(
            seen(range, "1E4C9B93F3F0682250B6CF8331B7EE68FD8"),
            9_659_365
        );
        assert_eq!(
            seen(range, "1e4c9b93f3f0682250b6cf8331b7ee68fd8"),
            9_659_365
        );
        assert_eq!(seen(range, "FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF"), 0);
        assert_eq!(seen(range, "0000000000000000000000000000000000A"), 0);
        assert_eq!(seen("", "anything"), 0);
    }

    #[test]
    fn entries_sharing_a_hash_each_count_the_others() {
        let hashed = [
            ("a".to_owned(), "H1".to_owned()),
            ("b".to_owned(), "H1".to_owned()),
            ("c".to_owned(), "H2".to_owned()),
            ("d".to_owned(), "H1".to_owned()),
        ];
        let shared = shared(&hashed);
        assert_eq!(shared["a"], 2);
        assert_eq!(shared["d"], 2);
        assert_eq!(shared["c"], 0);
    }

    #[test]
    fn health_says_only_what_was_found() {
        let unasked = Health {
            reused: 1,
            breached: None,
        };
        assert_eq!(unasked.said(), json!({"reused": 1}));
        assert!(unasked.troubled());
        let well = Health {
            reused: 0,
            breached: Some(0),
        };
        assert_eq!(well.said(), json!({"reused": 0, "breached": 0}));
        assert!(!well.troubled());
    }

    #[test]
    fn health_is_read_back_as_it_was_said() {
        for health in [
            Health {
                reused: 2,
                breached: Some(1204),
            },
            Health {
                reused: 1,
                breached: None,
            },
        ] {
            assert_eq!(Health::heard(&health.said()), Some(health));
        }
        assert_eq!(Health::heard(&json!({"breached": 3})), None);
        assert_eq!(Health::heard(&json!("well")), None);
    }
}
