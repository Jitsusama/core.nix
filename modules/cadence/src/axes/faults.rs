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
//! are its crashes, which quality already counts. Beside them, the
//! windows agents opened over his work, which the windows daemon warns of,
//! are read from his side, since he meets each one.

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
    /// When it was last said, in microseconds since the epoch.
    #[serde(default)]
    last: i64,
    /// Its first line, the first time, as it was said.
    #[serde(default)]
    example: String,
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
        unasked(f),
    ]
}

/// What the windows daemon says of a window an agent opened over his work
/// (modules/windows/src/unasked.rs), before the window and its opener.
const UNASKED: &str = "an agent opened a window over Joel's work: ";

/// Windows agents opened over his work, from the windows daemon's warning
/// alone, so they never fold into the services' errors, with the latest
/// said beside them. He meets each one, so it's his side, not a stand-in.
/// The bar's owner can't follow the opener from day to day, so it's
/// ZINAWZ's until the line names the opener's quest.
fn unasked(f: &Faults) -> Reading {
    let said: Vec<&Signature> = f
        .things
        .iter()
        .flatten()
        .filter(|(name, _)| name.as_str() == "windows")
        .flat_map(|(_, t)| &t.signatures)
        .filter(|(signature, _)| signature.starts_with(UNASKED))
        .map(|(_, s)| s)
        .collect();
    let total: u64 = said.iter().map(|s| s.count).sum();
    let latest = said.iter().max_by_key(|s| s.last).map(|s| {
        let what = s.example.strip_prefix(UNASKED).unwrap_or(&s.example);
        format!(
            "latest {}; ZINAWZ's until the opener's quest is named",
            cut(what)
        )
    });
    Reading::new(
        "{} windows agents opened over his work today",
        f.things.as_ref().map(|_| figure(total)),
        "",
        "ZINAWZ",
    )
    .his()
    .bar(Bar::AtMost(0.0))
    .detail(latest)
    .from("health's day file, from the windows daemon's journal")
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
    let mut words = format!("most {name} {}", shown(figure(n), ""));
    if let Some((signature, _)) = thing
        .signatures
        .iter()
        .filter(|(s, _)| !s.trim().is_empty())
        .max_by(|a, b| a.1.count.cmp(&b.1.count).then(b.0.cmp(a.0)))
    {
        words = format!("{words}: \"{}\"", cut(signature));
    }
    words
}

/// What was said, cut to a line.
fn cut(said: &str) -> String {
    const LONGEST: usize = 80;
    let kept: String = said.chars().take(LONGEST).collect();
    let more = if said.chars().count() > LONGEST {
        "..."
    } else {
        ""
    };
    format!("{kept}{more}")
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
        let [devices, services, traces, _] = quality(&f).try_into().unwrap();
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

    /// The windows daemon's day: two agents' windows, the later from a
    /// job, beside its own timings, none of them an error.
    fn opened_day() -> String {
        let host = format!(
            "{UNASKED}org.kde.heaptrack \"Heaptrack\", from pi-host-work-background.service \
             in /trees/ht"
        );
        let job = format!("{UNASKED}mpv \"clip\", from run-r1.scope in /tmp");
        serde_json::json!({"things": {
            "disk-guard": {"kind": "services", "errors": 14, "signatures": {
                "disk-guard: error: a pass freed nothing": {"count": 14}}},
            "windows": {"kind": "services", "errors": 0, "signatures": {
                "opened foot # ms after its process started": {"count": 30, "last": 9},
                host.clone(): {"count": 2, "last": 5, "example": host},
                job.replace("r1", "r#"): {"count": 1, "last": 7, "example": job}}}
        }})
        .to_string()
    }

    #[test]
    fn windows_agents_opened_are_his_and_said_with_the_latest() {
        let [_, services, _, opened] = quality(&faults(&opened_day())).try_into().unwrap();
        assert_eq!(opened.now, Some(3.0), "every agent's window wasn't counted");
        assert!(
            opened.his && opened.out(),
            "agents' windows weren't his side, or were in spec"
        );
        assert_eq!(
            opened.detail.as_deref(),
            Some(
                "latest mpv \"clip\", from run-r1.scope in /tmp; ZINAWZ's until the opener's \
                 quest is named"
            )
        );
        assert_eq!(
            opened.words(),
            "3 windows agents opened over his work today, at most 0"
        );
        let [_, alone, _, _] = quality(&faults(DAY)).try_into().unwrap();
        assert_eq!(
            (services.now, services.detail.as_deref()),
            (alone.now, alone.detail.as_deref()),
            "agents' windows moved the services' errors"
        );
    }

    #[test]
    fn a_day_with_no_agents_windows_reads_none() {
        let [_, _, _, opened] = quality(&faults(DAY)).try_into().unwrap();
        assert_eq!(opened.now, Some(0.0));
        assert_eq!(opened.detail, None);
        let [_, _, _, unread] = quality(&Faults::default()).try_into().unwrap();
        assert_eq!(
            unread.words(),
            "windows agents opened over his work today: no reading yet"
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
