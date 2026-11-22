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

/// What stands in for Joel's voice, said beside the sound's figures. The
/// far end plays on this machine too, under the same load, so what it
/// made up counts its own playing as well as the near end's sending.
const SOUND: &str = "Chromium's fake microphone, a beep each second, sent as Opus; \
                     the far end plays it on this machine, into a fake output";

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

/// The hour's figures: the call's, the apps' waits and the launcher's,
/// whose note says when the shell's journal wasn't read whole, and why.
pub fn measures(
    call: &Call,
    stand_in: &str,
    chrome: &Waits,
    slack: &Waits,
    opens: &[f64],
    unread: Option<&str>,
) -> Vec<Measure> {
    let call_too = |first: &str| vec![first.to_string(), stand_in.to_string()];
    let call_sound = |first: &str| {
        let mut said = vec![first.to_string(), SOUND.to_string()];
        said.extend(call.unheard.iter().map(|why| format!("unheard: {why}")));
        said
    };
    let mut behind = vec![format!(
        "{} openings in the last hour, chord or ask to on screen",
        opens.len()
    )];
    if let Some(why) = unread {
        behind.push(format!("the shell's journal wasn't read whole: {why}"));
    }
    let mut launcher = measure(
        "launcher open ms p99",
        crate::figures::percentile(opens, 99),
        50.0,
        true,
        behind,
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
            call_too(&call.frozen_s.map_or_else(
                || "how long it froze is unread".to_string(),
                |s| format!("{s} s frozen in the run"),
            )),
        ),
        // Past 1% made up, a voice starts to sound clipped; past 30 ms of
        // jitter, a call's buffer has to grow or drop sound to keep up.
        measure(
            "call sound concealed %",
            call.concealed,
            1.0,
            true,
            call_sound("the share the far end made up for late or lost sound"),
        ),
        measure(
            "call sound jitter p99 ms",
            call.jitter_p99,
            30.0,
            true,
            call_sound("how unevenly the sound's packets reached the far end"),
        ),
        measure(
            "call sound packets lost %",
            call.lost,
            0.0,
            true,
            call_sound("lost between two programs on one machine, so only under load"),
        ),
        measure(
            "call click to next frame p99 ms",
            call.clicks_p99,
            50.0,
            true,
            vec![
                "a click every 5 s on the test call's own Chromium tab, not Joel's Chrome"
                    .to_string(),
            ],
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

/// Writes the hour for cadence, whole or not at all, with the run beside
/// its figures, and adds the run's line to the history beside it. Cadence
/// reads only the figures; the run says what lies behind them, the
/// launcher's openings during the test and just before it among them.
pub fn write(folder: &Path, at: u64, measures: &[Measure], run: &Value) -> io::Result<()> {
    fs::create_dir_all(folder)?;
    let hour = json!({"at": at, "measures": measures, "run": run});
    // Named for this run, so a run by hand beside the hour's can't take it.
    let part = folder.join(format!("hour.json.{}.part", std::process::id()));
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
            concealed: Some(0.42),
            jitter_p99: Some(24.0),
            lost: Some(0.25),
            unheard: None,
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
            Some("journalctl ended with code 1: no"),
        );
        assert_eq!(all.len(), 11);
        let named = |name: &str| all.iter().find(|m| m.measure == name).unwrap();
        for (name, now, target) in [
            ("call sound concealed %", 0.4, 1.0),
            ("call sound jitter p99 ms", 24.0, 30.0),
            ("call sound packets lost %", 0.3, 0.0),
        ] {
            assert_eq!(
                (named(name).now, named(name).target),
                (Some(now), target),
                "{name} reads the wrong figure or target"
            );
            assert_eq!(
                named(name).behind[1],
                SOUND,
                "{name} doesn't say what it heard"
            );
        }
        assert_eq!(named("call sent fps p10").now, Some(30.0));
        // A figure read the wrong way passes cadence at any value.
        for m in &all {
            let fps = m.measure.contains(" fps ");
            assert_eq!(
                m.lower_is_better,
                !fps,
                "{} reads as {} is better",
                m.measure,
                if m.lower_is_better { "lower" } else { "higher" }
            );
        }
        assert_eq!(named("Chrome CPU wait per wakeup p99 ms").now, Some(19.7));
        let behind = &named("Chrome CPU wait per wakeup p99 ms").behind;
        assert!(
            behind[1].contains("12 windows"),
            "Chrome's wait doesn't say how many windows it read: {behind:?}"
        );
        assert_eq!(named("Slack CPU wait per wakeup p99 ms").now, None);
        assert_eq!(named("launcher open ms p99").owner, "XY9H5C");
        assert_eq!(named("launcher open ms p99").now, Some(31.0));
        assert_eq!(
            named("launcher open ms p99").behind[1],
            "the shell's journal wasn't read whole: journalctl ended with code 1: no",
            "an unread journal is said beside the figure cadence reads"
        );
        assert_eq!(named("call freezes per 10 min").behind[1], "a fake camera");
    }

    #[test]
    fn a_journal_read_whole_adds_nothing_to_the_launcher_s_note() {
        let all = measures(
            &call(),
            "a fake camera",
            &Waits::default(),
            &Waits::default(),
            &[20.0, 31.0],
            None,
        );
        let launcher = all
            .iter()
            .find(|m| m.measure == "launcher open ms p99")
            .unwrap();
        assert_eq!(launcher.now, Some(31.0));
        assert_eq!(
            launcher.behind,
            ["2 openings in the last hour, chord or ask to on screen"]
        );
    }

    #[test]
    fn sound_that_went_unheard_says_why_beside_each_of_its_figures() {
        let unheard = Call {
            concealed: None,
            jitter_p99: None,
            lost: None,
            unheard: Some("the far end's playing was refused: NotAllowedError".to_string()),
            ..call()
        };
        let all = measures(
            &unheard,
            "a fake camera",
            &Waits::default(),
            &Waits::default(),
            &[],
            None,
        );
        for m in all.iter().filter(|m| m.measure.starts_with("call sound ")) {
            assert_eq!(
                m.behind.last().map(String::as_str),
                Some("unheard: the far end's playing was refused: NotAllowedError"),
                "{} doesn't say why it's missing",
                m.measure
            );
        }
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
            None,
        );
        write(&root, 1_795_200_000, &all, &json!({"took_s": 190})).unwrap();
        write(&root, 1_795_203_600, &all, &json!({"took_s": 191})).unwrap();
        let hour: Value =
            serde_json::from_slice(&fs::read(root.join("hour.json")).unwrap()).unwrap();
        let history = fs::read_to_string(root.join("history.jsonl")).unwrap();
        let parts = fs::read_dir(&root).unwrap().count();
        fs::remove_dir_all(&root).unwrap();
        assert_eq!(hour["at"], 1_795_203_600);
        assert_eq!(
            hour["run"],
            json!({"took_s": 191}),
            "the newest run beside the hour"
        );
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
