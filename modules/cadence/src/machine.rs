//! The machine's own figures for the day, from the file health writes
//! (modules/health): its boots and shutdowns, its errors by kind, what pi
//! and the shell held, the agents that died and the moments that went
//! worst, so a machine getting worse to use is judged with the rest of
//! the day. Health judges nothing; the day does, here.

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
    measure(
        "health.memory.pi.most-gib",
        "resources",
        "the most pi held at once",
        " GiB",
        None,
        1.0,
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
        .filter(|(key, _)| MEASURES.iter().any(|measure| measure.key == key))
        .collect()
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
                "figures": {"boot.seconds": 27.0, "agents.died": 4, "unheard-of": 3}}"#,
        );
        assert_eq!(
            figures,
            BTreeMap::from([
                ("health.agents.died".to_owned(), 4.0),
                ("health.boot.seconds".to_owned(), 27.0),
            ]),
            "a figure of health's was lost, or one no measure holds was kept"
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
}
