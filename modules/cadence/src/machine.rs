//! The machine's own figures for the day, from the file health writes
//! (modules/health): its boots and shutdowns, its errors by kind, what pi
//! and the shell held, the agents that died and the moments that went
//! worst, so a machine getting worse to use is judged with the rest of
//! the day, and the services whose restarts said no ready line, named.
//! Health judges nothing; the day does, here.

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::day::Measure;

const EVIDENCE: &str = "health's ~/.local/state/health/<date>.json, its figures";

/// A measure of health's, all read from the same file.
const fn measure(
    key: &'static str,
    axis: &'static str,
    says: &'static str,
    unit: &'static str,
    bar: Option<f64>,
    floor: f64,
) -> Measure {
    Measure {
        key,
        axis,
        evidence: EVIDENCE,
        says,
        unit,
        bar,
        floor,
    }
}

/// Every figure of health's the day reads. The bars are the specs
/// docs/design.md holds the machine to: a boot to a working launcher in
/// 30 s with the password left out, a shutdown in 10 s.
pub const MEASURES: &[Measure] = &[
    measure(
        "health.boot.seconds",
        "polish",
        "the slowest boot to a ready launcher, the password left out",
        " s",
        Some(30.0),
        2.0,
    ),
    measure(
        "health.shutdown.seconds",
        "polish",
        "the slowest shutdown",
        " s",
        Some(10.0),
        1.0,
    ),
    measure(
        "health.devices.errors",
        "quality",
        "errors from the machine's devices",
        "",
        None,
        5.0,
    ),
    measure(
        "health.programs.errors",
        "quality",
        "errors from programs",
        "",
        None,
        10.0,
    ),
    measure(
        "health.services.errors",
        "quality",
        "errors from services",
        "",
        None,
        10.0,
    ),
    measure(
        "health.security.errors",
        "quality",
        "errors from the machine's security",
        "",
        None,
        5.0,
    ),
    measure(
        "health.tools.errors",
        "time to done",
        "agents' tool calls failed or refused",
        "",
        None,
        20.0,
    ),
    measure(
        "health.words.errors",
        "polish",
        "lines where Joel sounded pained",
        "",
        None,
        2.0,
    ),
    measure(
        "health.feel.lost-minutes",
        "polish",
        "minutes lost in the day's worst moments",
        " min",
        None,
        2.0,
    ),
    // Each place's own, read by whoever owns it: pi, the launcher, Chrome.
    measure(
        "health.feel.*.lost-minutes",
        "polish",
        "minutes lost in the day's worst moments",
        " min",
        None,
        2.0,
    ),
    measure(
        "health.feel.stopped",
        "collaboration",
        "agents' turns he stopped in the day's worst moments",
        "",
        Some(0.0),
        1.0,
    ),
    measure(
        "health.feel.corrections",
        "collaboration",
        "agents he corrected in the day's worst moments",
        "",
        Some(0.0),
        1.0,
    ),
    measure(
        "health.memory.pi.most-mib",
        "resources",
        "the most pi held at once",
        " MiB",
        None,
        1024.0,
    ),
    measure(
        "health.memory.quickshell.most-mib",
        "resources",
        "the most the shell held at once",
        " MiB",
        None,
        50.0,
    ),
    measure(
        "health.agents.died",
        "quality",
        "agents that died without parking",
        "",
        None,
        2.0,
    ),
    // An ask for a secret is worth its interruption only while asks stay
    // rare and read, and asked once: secret-asks's own readings.
    measure(
        "health.asks.shown",
        "security",
        "asks for a secret Joel was shown",
        "",
        Some(3.0),
        1.0,
    ),
    measure(
        "health.asks.unread-yeses",
        "security",
        "asks said yes to sooner than they could be read",
        "",
        Some(0.0),
        1.0,
    ),
    measure(
        "health.asks.again",
        "security",
        "asks for an item and command already asked for that day",
        "",
        Some(0.0),
        1.0,
    ),
    measure(
        "health.asks.avoidable-percent",
        "security",
        "the share of asks a minted token would have spared",
        "%",
        Some(0.0),
        10.0,
    ),
    // Past 90% used, a tenth free, is where the disk guard's nearly-full
    // band begins unless a machine sets its own. Each disk by its mount,
    // so a full one is named where it is: on 13 November the day said
    // only that some disk was at 100%.
    measure(
        "health.disk.*.most-used-percent",
        "resources",
        "the most in use of the disk",
        "%",
        Some(90.0),
        5.0,
    ),
];

/// The part of health's day the day reads: the figures health works out
/// from what it tallied, so the two never count a thing two ways.
#[derive(Default, Deserialize)]
#[serde(default)]
struct Written {
    figures: BTreeMap<String, f64>,
}

/// The figures in health's day file, as `health.<figure>`, those of a
/// measure the day holds; a file missing or unread gives none, as any
/// unread source does.
#[must_use]
pub fn figures(json: &str) -> BTreeMap<String, f64> {
    let written: Written = serde_json::from_str(json).unwrap_or_default();
    written
        .figures
        .into_iter()
        .map(|(key, figure)| (format!("health.{key}"), figure))
        .filter(|(key, _)| MEASURES.iter().any(|measure| measure.holds(key)))
        .collect()
}

/// Each service whose restarts said no ready line, with how often it
/// restarted, from health's day file: a kind of the day's by name, so
/// each is said once to its owner as it's new, who adds the line that
/// times its restarts to what it serves.
#[must_use]
pub fn unstamped(json: &str) -> BTreeMap<String, u32> {
    #[derive(Default, Deserialize)]
    #[serde(default)]
    struct Written {
        unstamped: BTreeMap<String, u32>,
    }
    serde_json::from_str::<Written>(json)
        .unwrap_or_default()
        .unstamped
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::day::Day;
    use crate::judge::judge;

    #[test]
    fn healths_figures_are_kept_by_their_names() {
        let figures = figures(
            r#"{"date": "2026-11-12", "things": {},
                "figures": {"boot.seconds": 27.0, "agents.died": 4, "unheard-of": 3,
                "asks.unread-yeses": 1, "disk./.most-used-percent": 71.5}}"#,
        );
        assert_eq!(
            figures,
            BTreeMap::from([
                ("health.agents.died".to_owned(), 4.0),
                ("health.asks.unread-yeses".to_owned(), 1.0),
                ("health.boot.seconds".to_owned(), 27.0),
                ("health.disk./.most-used-percent".to_owned(), 71.5),
            ]),
            "a figure of health's was lost, or one no measure holds was kept"
        );
    }

    #[test]
    fn asks_for_a_secret_are_on_the_security_axis() {
        let off: Vec<&str> = MEASURES
            .iter()
            .filter(|m| m.key.starts_with("health.asks.") && m.axis != "security")
            .map(|m| m.key)
            .collect();
        assert!(
            off.is_empty(),
            "asks were judged on another axis than security's readings of them: {off:?}"
        );
    }

    #[test]
    fn a_service_with_no_ready_line_is_named_to_its_owner_once_new() {
        let day = |named: &str| Day {
            named: BTreeMap::from([("unstamped".to_owned(), unstamped(named))]),
            ..Day::default()
        };
        let today = day(r#"{"unstamped": {"google.service": 2, "sound.service": 1}}"#);
        let before = [
            day(r#"{"unstamped": {"sound.service": 1}}"#),
            day(r#"{"unstamped": {}}"#),
            day(r#"{"unstamped": {"sound.service": 3}}"#),
        ];
        let named: Vec<(String, String)> = judge(&today, &before)
            .worse
            .into_iter()
            .map(|finding| (finding.key, finding.words))
            .collect();
        assert_eq!(
            named,
            [(
                "unstamped:google.service".to_owned(),
                "restarts with no ready line from google.service, new, 2 times".to_owned()
            )],
            "a service new to saying no ready line wasn't named, or one named before was again"
        );
    }

    #[test]
    fn a_day_health_didnt_write_gives_no_figures() {
        assert!(
            figures("").is_empty() && figures(r#"{"date": "2026-11-12"}"#).is_empty(),
            "a missing file or one without figures gave figures"
        );
    }

    #[test]
    fn a_boot_past_its_bar_is_a_finding_on_the_first_day() {
        let day = Day {
            figures: figures(r#"{"figures": {"boot.seconds": 41.5}}"#),
            ..Day::default()
        };
        let judged = judge(&day, &[]);
        assert!(
            judged
                .worse
                .iter()
                .any(|finding| finding.key == "health.boot.seconds" && finding.bar == Some(30.0)),
            "a 41.5 s boot wasn't held against its 30 s bar: {:?}",
            judged.worse
        );
    }

    #[test]
    fn a_full_disk_is_named_by_its_mount() {
        let day = Day {
            figures: figures(
                r#"{"figures": {"disk.most-used-percent": 100.0,
                    "disk./var/cache/builds.most-used-percent": 100.0,
                    "disk./.most-used-percent": 40.0}}"#,
            ),
            ..Day::default()
        };
        let words: Vec<String> = judge(&day, &[])
            .worse
            .into_iter()
            .map(|finding| finding.words)
            .collect();
        assert_eq!(
            words,
            ["the most in use of the disk at /var/cache/builds was 100%, past its bar of 90%"],
            "a disk past its bar wasn't named by its mount, or was said twice"
        );
    }

    #[test]
    fn each_places_lost_minutes_and_his_stops_and_corrections_are_kept() {
        let figures = figures(
            r#"{"figures": {"feel.lost-minutes": 16.0, "feel.pi.lost-minutes": 15.0,
                "feel.launcher.lost-minutes": 1.0, "feel.stopped": 2.0,
                "feel.corrections": 1.0}}"#,
        );
        assert_eq!(
            figures.keys().map(String::as_str).collect::<Vec<_>>(),
            [
                "health.feel.corrections",
                "health.feel.launcher.lost-minutes",
                "health.feel.lost-minutes",
                "health.feel.pi.lost-minutes",
                "health.feel.stopped",
            ],
            "a place's lost minutes, his stops or his corrections weren't kept"
        );
    }
}
