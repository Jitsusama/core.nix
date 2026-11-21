//! The hour's figures in cadence's shape, `apps-under-load/hour.json`
//! in the state home, and a line for each run in `history.jsonl` beside
//! it. Cadence reads the first (modules/cadence/src/hour/measures/apps.rs);
//! the second is kept for a person looking back.

use crate::figures::Call;
use crate::history::Waits;
use serde::Serialize;
use serde_json::{Value, json};
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::Path;

/// Who answers for these figures, as cadence names owners.
const OWNER: &str = "5193FV";
/// Who answers for the launcher's.
const LAUNCHER: &str = "XY9H5C";

/// One figure, as cadence reads it.
#[derive(Debug, PartialEq, Serialize)]
#[expect(
    clippy::struct_field_names,
    reason = "cadence reads each figure's name as its measure key"
)]
pub struct Measure {
    pub measure: &'static str,
    pub now: Option<f64>,
    pub target: f64,
    pub lower_is_better: bool,
    pub owner: &'static str,
    pub behind: Vec<String>,
}

fn measure(
    measure: &'static str,
    now: Option<f64>,
    target: f64,
    lower_is_better: bool,
    behind: Vec<String>,
) -> Measure {
    Measure {
        measure,
        now: now.map(|x| (x * 10.0).round() / 10.0),
        target,
        lower_is_better,
        owner: OWNER,
        behind,
    }
}

/// What a wait's figure leaves out, said beside it.
fn left_out(app: &str, waits: &Waits) -> Vec<String> {
    let mut said = vec![format!(
        "Joel's {app}: each 10 s window's worst drawing-thread wait, from lag-watch, {} windows",
        waits.windows
    )];
    if waits.lowered > 0 {
        said.push(format!("hidden tabs left out in {} windows", waits.lowered));
    }
    if waits.unwatched > 0 {
        said.push(format!(
            "some of it unwatched in {} windows",
            waits.unwatched
        ));
    }
    said
}

/// The hour's figures: the call's, the apps' waits and the launcher's.
pub fn measures(
    call: &Call,
    stand_in: &str,
    chrome: &Waits,
    slack: &Waits,
    opens: &[f64],
) -> Vec<Measure> {
    let call_too = |first: &str| vec![first.to_string(), stand_in.to_string()];
    let mut launcher = measure(
        "launcher open ms p99",
        crate::figures::percentile(opens, 99),
        50.0,
        true,
        vec![format!(
            "{} openings in the last hour, chord or ask to on screen",
            opens.len()
        )],
    );
    launcher.owner = LAUNCHER;
    vec![
        measure(
            "call camera fps p10",
            call.camera,
            30.0,
            false,
            call_too("the camera's source"),
        ),
        measure(
            "call sent fps p10",
            call.sent,
            30.0,
            false,
            call_too("VP9 encoded in software"),
        ),
        measure(
            "call far end got fps p10",
            call.got,
            30.0,
            false,
            call_too("decoded"),
        ),
        measure(
            "call freezes per 10 min",
            call.freezes,
            0.0,
            true,
            call_too(&format!(
                "{} s frozen in the run",
                call.frozen_s.unwrap_or(0.0)
            )),
        ),
        measure(
            "Chrome click to next frame p99 ms",
            call.clicks_p99,
            50.0,
            true,
            vec!["a click every 5 s on the call's own tab, while the call runs".to_string()],
        ),
        measure(
            "Chrome CPU wait per wakeup p99 ms",
            chrome.worst_p99,
            2.0,
            true,
            left_out("Chrome", chrome),
        ),
        measure(
            "Slack CPU wait per wakeup p99 ms",
            slack.worst_p99,
            2.0,
            true,
            left_out("Slack", slack),
        ),
        launcher,
    ]
}

/// Writes the hour for cadence, whole or not at all, and adds the run's
/// line to the history beside it.
pub fn write(folder: &Path, at: u64, measures: &[Measure], run: &Value) -> io::Result<()> {
    fs::create_dir_all(folder)?;
    let hour = json!({"at": at, "measures": measures});
    let part = folder.join("hour.json.part");
    fs::write(&part, serde_json::to_vec_pretty(&hour)?)?;
    fs::rename(&part, folder.join("hour.json"))?;
    let figures: serde_json::Map<String, Value> = measures
        .iter()
        .map(|m| (m.measure.to_string(), json!(m.now)))
        .collect();
    let line = json!({"at": at, "run": run, "measures": figures});
    let mut history = OpenOptions::new()
        .create(true)
        .append(true)
        .open(folder.join("history.jsonl"))?;
    writeln!(history, "{line}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn call() -> Call {
        Call {
            camera: Some(30.0),
            sent: Some(29.96),
            got: Some(30.0),
            freezes: Some(0.0),
            frozen_s: Some(0.0),
            clicks_p50: Some(3.0),
            clicks_p99: Some(9.84),
            samples: 140,
        }
    }

    #[test]
    fn every_figure_says_which_way_is_better_and_who_owns_it() {
        let chrome = Waits {
            worst_p99: Some(19.66),
            windows: 360,
            lowered: 12,
            unwatched: 0,
        };
        let all = measures(
            &call(),
            "a fake camera",
            &chrome,
            &Waits::default(),
            &[20.0, 31.0],
        );
        assert_eq!(all.len(), 8);
        let named = |name: &str| all.iter().find(|m| m.measure == name).unwrap();
        assert_eq!(named("call sent fps p10").now, Some(30.0));
        assert!(!named("call sent fps p10").lower_is_better);
        assert_eq!(named("Chrome CPU wait per wakeup p99 ms").now, Some(19.7));
        assert!(named("Chrome CPU wait per wakeup p99 ms").behind[1].contains("12 windows"));
        assert_eq!(named("Slack CPU wait per wakeup p99 ms").now, None);
        assert_eq!(named("launcher open ms p99").owner, "XY9H5C");
        assert_eq!(named("launcher open ms p99").now, Some(31.0));
        assert_eq!(named("call freezes per 10 min").behind[1], "a fake camera");
    }

    #[test]
    fn the_hour_is_written_in_cadence_s_shape_and_the_run_kept_beside_it() {
        let root =
            std::env::temp_dir().join(format!("call-under-load-hour-{}", std::process::id()));
        let all = measures(
            &call(),
            "a fake camera",
            &Waits::default(),
            &Waits::default(),
            &[],
        );
        write(&root, 1_795_200_000, &all, &json!({"took_s": 190})).unwrap();
        write(&root, 1_795_203_600, &all, &json!({"took_s": 191})).unwrap();
        let hour: Value =
            serde_json::from_slice(&fs::read(root.join("hour.json")).unwrap()).unwrap();
        let history = fs::read_to_string(root.join("history.jsonl")).unwrap();
        let parts = fs::read_dir(&root).unwrap().count();
        fs::remove_dir_all(&root).unwrap();
        assert_eq!(hour["at"], 1_795_203_600);
        let first = &hour["measures"][0];
        for key in [
            "measure",
            "now",
            "target",
            "lower_is_better",
            "owner",
            "behind",
        ] {
            assert!(first.get(key).is_some(), "{key} missing from {first}");
        }
        assert_eq!(history.lines().count(), 2);
        assert_eq!(parts, 2, "only hour.json and history.jsonl are left");
    }
}
