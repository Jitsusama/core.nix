//! Each repository's figures for the day, from the file code-health
//! writes (modules/code-health), so a repository getting messier is judged
//! with the rest of the day. Core names no repository built on it: the
//! file says them, and a measure with `*` for the repository holds each.

use std::collections::BTreeMap;

use crate::day::{Measure, measures};

/// Where each repository's figures are read from.
pub const HEALTH: &str = "code-health's ~/.local/state/code-health/<date>.json and its -named.json";

/// Each repository's measures, `*` standing for the repository.
pub const MEASURES: &[Measure] = &[
    Measure {
        key: "code.*.long-files",
        axis: "organization",
        evidence: HEALTH,
        says: "long files that don't say why",
        unit: "",
        bar: None,
        floor: 1.0,
    },
    Measure {
        key: "code.*.crowded-folders",
        axis: "organization",
        evidence: HEALTH,
        says: "crowded folders that don't say why",
        unit: "",
        bar: None,
        floor: 1.0,
    },
    Measure {
        key: "code.*.suppressions",
        axis: "quality",
        evidence: HEALTH,
        says: "lint suppressions",
        unit: "",
        bar: None,
        floor: 1.0,
    },
    Measure {
        key: "code.*.duplicated-lines",
        axis: "quality",
        evidence: HEALTH,
        says: "lines repeated from elsewhere",
        unit: "",
        bar: None,
        floor: 20.0,
    },
    Measure {
        key: "code.*.quarantined-tests",
        axis: "quality",
        evidence: HEALTH,
        says: "tests in quarantine",
        unit: "",
        bar: None,
        floor: 1.0,
    },
    Measure {
        key: "code.*.daemons-waiting",
        axis: "organization",
        evidence: HEALTH,
        says: "points of docs/daemons.md daemons still break",
        unit: "",
        bar: None,
        floor: 1.0,
    },
    Measure {
        key: "code.*.edit-to-merge",
        axis: "time to done",
        evidence: HEALTH,
        says: "median time from a change's first edit to its merge",
        unit: " min",
        bar: None,
        floor: 5.0,
    },
    // A file a tenth of the day's changes touched is one each queues on.
    Measure {
        key: "code.*.shared-files",
        axis: "organization",
        evidence: HEALTH,
        says: "files a tenth of the day's changes touched",
        unit: "",
        bar: None,
        floor: 1.0,
    },
];

/// The figures in code-health's `{repo: {measure: figure}}`, as
/// `code.<repo>.<measure>`, those of a measure the day holds; a file
/// missing or unread gives none, as any unread source does.
#[must_use]
pub fn figures(json: &str) -> BTreeMap<String, f64> {
    let repos: BTreeMap<String, BTreeMap<String, f64>> =
        serde_json::from_str(json).unwrap_or_default();
    let mut figures = BTreeMap::new();
    for (repo, measures) in repos {
        for (measure, figure) in measures {
            figures.insert(format!("code.{repo}.{measure}"), figure);
        }
    }
    figures.retain(|key, _| {
        let one = BTreeMap::from([(key.clone(), 0.0)]);
        measures().any(|measure| !measure.read(&one).is_empty())
    });
    figures
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::day::Day;
    use crate::judge::judge;

    #[test]
    fn each_repositorys_figures_are_kept_by_its_name() {
        let figures = figures(
            r#"{"core.nix": {"long-files": 99, "unheard-of": 3},
                "harness": {"suppressions": 12, "shared-files": 2}}"#,
        );
        assert_eq!(
            figures,
            BTreeMap::from([
                ("code.core.nix.long-files".to_owned(), 99.0),
                ("code.harness.shared-files".to_owned(), 2.0),
                ("code.harness.suppressions".to_owned(), 12.0),
            ]),
            "a repository's figure was lost, or one no measure holds was kept"
        );
        assert!(
            super::figures("not json").is_empty(),
            "a file that isn't code-health's gave figures"
        );
    }

    #[test]
    fn a_repository_is_held_against_its_own_days_by_any_rise() {
        let day = |long: f64, other: f64| Day {
            figures: figures(&format!(
                r#"{{"core.nix": {{"long-files": {long}}}, "harness": {{"long-files": {other}}}}}"#
            )),
            ..Day::default()
        };
        let before: Vec<Day> = (0..3).map(|_| day(99.0, 5.0)).collect();
        let judged = judge(&day(101.0, 5.0), &before);
        let words: Vec<&str> = judged.worse.iter().map(|f| f.words.as_str()).collect();
        assert_eq!(
            words,
            ["core.nix's long files that don't say why went from 99 to 101"],
            "a repository's rise wasn't found, or another's level was"
        );
        assert_eq!(
            judged.read, 2,
            "each repository's figure wasn't read as one of its own: {judged:?}"
        );
    }
}
