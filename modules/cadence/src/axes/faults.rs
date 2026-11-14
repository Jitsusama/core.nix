//! What the machine's logs said went wrong today, as health follows them
//! and groups them by thing in its day file: the faults the kernel logged
//! of his devices, the services' error lines, the stack traces printed,
//! and the security log's errors. Joel named each of these on 11 November
//! ("application crashes, stack traces printed, error messages in logs,
//! hardware devices faulting"), so each is a reading of its own, with the
//! thing that said it most beside it, so its owner knows where to start.
//! Every one is a stand-in, read from the logs rather than from what he
//! met, and each is held to none: a line that needn't be there is noise
//! he pays for too. A program's error lines aren't read here, since most
//! are its crashes, which quality already counts.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde::Deserialize;

use super::{Bar, Reading, counted, shown};

/// What health kept of one thing: its kind, its error lines, its stack
/// traces, and what it said, by signature.
#[derive(Debug, Default, Deserialize)]
struct Thing {
    kind: Option<String>,
    #[serde(default)]
    errors: u64,
    #[serde(default)]
    traces: u64,
    #[serde(default)]
    signatures: BTreeMap<String, Signature>,
}

#[derive(Debug, Default, Deserialize)]
struct Signature {
    #[serde(default)]
    count: u64,
}

#[derive(Debug, Default, Deserialize)]
struct Written {
    #[serde(default)]
    things: BTreeMap<String, Thing>,
}

/// The things health's day file names, or none when it wrote no day.
#[derive(Debug, Default)]
pub struct Faults {
    things: Option<BTreeMap<String, Thing>>,
}

/// Reads health's file for the day `now` falls in.
#[must_use]
pub fn gather(state_home: &Path, now: i64) -> Faults {
    Faults {
        things: fs::read_to_string(counted::health_file(state_home, now))
            .ok()
            .map(|json| read(&json)),
    }
}

fn read(json: &str) -> BTreeMap<String, Thing> {
    serde_json::from_str::<Written>(json)
        .map(|w| w.things)
        .unwrap_or_default()
}

/// The quality axis's three: devices, services and stack traces.
#[must_use]
pub fn quality(f: &Faults) -> Vec<Reading> {
    vec![
        errors(
            f,
            "devices",
            "{} faults the kernel logged of his devices today",
            "FEH1HE",
        )
        .from("health's day file, from the kernel's log"),
        errors(
            f,
            "services",
            "{} error lines from services today",
            "80ECO2",
        )
        .from("health's day file, from the services' journal"),
        traces(f),
    ]
}

/// The security log's errors, for the security axis.
#[must_use]
pub fn security(f: &Faults) -> Reading {
    errors(
        f,
        "security",
        "{} error lines in the security log today",
        "ISXEG8",
    )
    .from("health's day file, from sudo, polkit, logins and the audit log")
}

/// The error lines of one kind, with the thing that logged most of them.
fn errors(f: &Faults, kind: &str, says: &'static str, owner: &'static str) -> Reading {
    let of: Vec<(&String, &Thing)> = f
        .things
        .iter()
        .flatten()
        .filter(|(_, t)| t.kind.as_deref() == Some(kind))
        .collect();
    let total: u64 = of.iter().map(|(_, t)| t.errors).sum();
    let most = of
        .iter()
        .filter(|(_, t)| t.errors > 0)
        .max_by(|a, b| a.1.errors.cmp(&b.1.errors).then(b.0.cmp(a.0)))
        .map(|(name, t)| said(name, t, t.errors));
    Reading::new(says, f.things.as_ref().map(|_| figure(total)), "", owner)
        .bar(Bar::AtMost(0.0))
        .detail(most)
}

/// Every stack trace printed today, whatever printed it.
fn traces(f: &Faults) -> Reading {
    let total: u64 = f.things.iter().flatten().map(|(_, t)| t.traces).sum();
    let most = f
        .things
        .iter()
        .flatten()
        .filter(|(_, t)| t.traces > 0)
        .max_by(|a, b| a.1.traces.cmp(&b.1.traces).then(b.0.cmp(a.0)))
        .map(|(name, t)| said(name, t, t.traces));
    Reading::new(
        "{} stack traces printed today",
        f.things.as_ref().map(|_| figure(total)),
        "",
        "80ECO2",
    )
    .bar(Bar::AtMost(0.0))
    .detail(most)
    .from("health's day file, from every journal it follows")
}

/// The thing that said most, how many, and what it said most often, cut
/// to a line, so the owner can tell it from the rest at a glance.
fn said(name: &str, thing: &Thing, n: u64) -> String {
    const LONGEST: usize = 80;
    let mut words = format!("most {name} {}", shown(figure(n), ""));
    if let Some((signature, _)) = thing
        .signatures
        .iter()
        .filter(|(s, _)| !s.trim().is_empty())
        .max_by(|a, b| a.1.count.cmp(&b.1.count).then(b.0.cmp(a.0)))
    {
        let cut: String = signature.chars().take(LONGEST).collect();
        let more = if signature.chars().count() > LONGEST {
            "..."
        } else {
            ""
        };
        words = format!("{words}: \"{cut}{more}\"");
    }
    words
}

#[expect(
    clippy::cast_precision_loss,
    reason = "a day's lines are far under 2^52"
)]
fn figure(n: u64) -> f64 {
    n as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    const DAY: &str = r#"{"things": {
        "iwlwifi": {"kind": "devices", "errors": 4, "traces": 0, "signatures": {
            "iwlwifi #: Tx queue alloc failed": {"count": 4}}},
        "lid": {"kind": "devices", "errors": 1, "traces": 0, "signatures": {
            "the lid reads shut while its keys are typed": {"count": 1}}},
        "Memory": {"kind": "devices", "errors": 0, "traces": 2, "signatures": {}},
        "disk-guard": {"kind": "services", "errors": 14, "traces": 0, "signatures": {
            "disk-guard: error: a pass freed nothing": {"count": 14},
            "disk-guard: stopping": {"count": 2}}},
        "quickshell": {"kind": "programs", "errors": 9, "traces": 5, "signatures": {
            "": {"count": 9}}}
    }}"#;

    fn faults(json: &str) -> Faults {
        Faults {
            things: Some(read(json)),
        }
    }

    #[test]
    fn each_kind_is_summed_with_the_thing_that_said_most() {
        let f = faults(DAY);
        let [devices, services, traces] = quality(&f).try_into().unwrap();
        assert_eq!(devices.now, Some(5.0));
        assert_eq!(
            devices.detail.as_deref(),
            Some("most iwlwifi 4: \"iwlwifi #: Tx queue alloc failed\"")
        );
        assert_eq!(services.now, Some(14.0));
        assert_eq!(
            services.detail.as_deref(),
            Some("most disk-guard 14: \"disk-guard: error: a pass freed nothing\"")
        );
        assert_eq!(traces.now, Some(7.0));
        // A thing whose only signature is blank is named without one.
        assert_eq!(traces.detail.as_deref(), Some("most quickshell 5"));
        assert!(
            devices.out() && !devices.his,
            "a device's fault was in spec, or read from his side"
        );
    }

    #[test]
    fn a_day_with_none_reads_none_and_no_day_reads_unread() {
        let quiet = security(&faults(r#"{"things": {}}"#));
        assert_eq!(quiet.now, Some(0.0));
        assert_eq!(quiet.detail, None);
        assert!(!quiet.out(), "a quiet security log was out");
        let unread = security(&Faults::default());
        assert_eq!(
            unread.words(),
            "error lines in the security log today: no reading yet"
        );
    }

    #[test]
    fn a_long_signature_is_cut_to_a_line() {
        let long = "x".repeat(120);
        let json = format!(
            r#"{{"things": {{"t": {{"kind": "security", "errors": 1,
            "signatures": {{"{long}": {{"count": 1}}}}}}}}}}"#
        );
        let detail = security(&faults(&json)).detail.unwrap();
        assert_eq!(detail, format!("most t 1: \"{}...\"", "x".repeat(80)));
    }
}
