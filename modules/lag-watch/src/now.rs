//! The latest window, as `lag-watch` and `lag-watch now` answer it: for
//! Joel, how hard the work behind him was working and how long ago, what
//! he waited on or that nothing waited, and how quickly niri and sound
//! were served; for a script, the window itself as the status file holds
//! it.

use crate::report::figure;
use crate::window::Window;
use serde_json::{Value, json};
use terminal_kit::layout;
use terminal_kit::{Pen, Room};

/// A duration as its largest two units, as the desktop writes one.
pub fn duration(seconds: u64) -> String {
    match seconds {
        s if s < 60 => format!("{s} s"),
        s if s < 3600 => format!("{} m {} s", s / 60, s % 60),
        s if s < 86_400 => format!("{} h {} m", s / 3600, s % 3600 / 60),
        s => format!("{} d {} h", s / 86_400, s % 86_400 / 3600),
    }
}

/// The window as Joel reads it, `age` seconds after it ended.
pub fn text(window: &Window, age: u64, pen: &Pen, room: Room) -> String {
    let band = format!("{:?} work behind", window.band);
    let head = format!(
        "{}  {}",
        pen.title(&band),
        pen.role("muted", &format!("{} ago", duration(age)))
    );
    let waited: Vec<String> = if window.trouble.is_empty() {
        vec![format!(
            "{}{}",
            pen.marked("done", "success"),
            pen.role("text", "nothing waited")
        )]
    } else {
        window
            .trouble
            .iter()
            .map(|line| {
                format!(
                    "{}{}",
                    pen.marked("caution", "warning"),
                    pen.role("text", &layout::hung(line, layout::MARK, room))
                )
            })
            .collect()
    };
    let wait = |thread: &str| window.waits.get(thread).map(|wait| wait.mean_us);
    let niri = format!(
        "{}, {} at p99",
        figure(window.niri.p50_ms, "ms"),
        figure(window.niri.p99_ms, "ms")
    );
    let turn = |us: Option<f64>| us.map(|us| format!("{} a turn", figure(us, "µs")));
    let niri_waits = turn(wait("niri"));
    let sound_waits = turn(wait("pipewire"));
    let calls: Vec<(String, String)> = window
        .call
        .iter()
        .map(|(program, worst)| {
            (
                format!("{program} in a call"),
                format!(
                    "{} waits {} a turn",
                    worst.thread,
                    figure(worst.wait_us, "µs")
                ),
            )
        })
        .collect();
    let mut facts: Vec<(&str, Option<&str>)> = vec![
        ("niri answers", Some(niri.as_str())),
        ("niri waits", niri_waits.as_deref()),
        ("sound waits", sound_waits.as_deref()),
    ];
    facts.extend(
        calls
            .iter()
            .map(|(label, value)| (label.as_str(), Some(value.as_str()))),
    );
    layout::joined(vec![
        vec![head],
        waited,
        layout::facts(&facts, "not watched", pen, room),
    ])
}

/// The window for a script, as the status file holds it, and how long ago
/// it ended.
pub fn json(window: &Window, age: u64) -> Value {
    json!({
        "schema": "lag-watch.now/1",
        "age_seconds": age,
        "window": window,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::window::{Answers, Band, Wait, Worst};
    use std::collections::BTreeMap;

    fn window(trouble: &[&str]) -> Window {
        let mut waits = BTreeMap::new();
        waits.insert(
            "niri".to_string(),
            Wait {
                mean_us: 14.0,
                worst_us: 90.0,
            },
        );
        Window {
            ended: 0,
            seconds: 10.0,
            system: "gen".into(),
            band: Band::Quiet,
            stalls: BTreeMap::new(),
            memory: BTreeMap::new(),
            waits,
            call: BTreeMap::new(),
            niri: Answers {
                p50_ms: 1.2,
                p99_ms: 3.4,
                ..Answers::default()
            },
            trouble: trouble.iter().map(ToString::to_string).collect(),
            worst: String::new(),
            who: String::new(),
            since: None,
        }
    }

    #[test]
    fn a_quiet_window_says_nothing_waited_and_how_niri_was_served() {
        let text = text(&window(&[]), 4, &Pen::plain(), Room::Unbounded);
        assert_eq!(
            text,
            "Quiet work behind  4 s ago\n\
             \n✓ nothing waited\n\
             \nniri answers  1.20 ms, 3.40 ms at p99\n\
             niri waits    14.0 µs a turn\n\
             sound waits   not watched\n"
        );
    }

    #[test]
    fn each_wait_is_a_line_of_its_own() {
        let text = text(
            &window(&["niri took 60 ms to answer", "the desktop stalled on memory"]),
            75,
            &Pen::plain(),
            Room::Unbounded,
        );
        assert!(
            text.starts_with("Quiet work behind  1 m 15 s ago\n"),
            "{text}"
        );
        assert!(text.contains("\n! niri took 60 ms to answer\n"), "{text}");
        assert!(
            text.contains("\n! the desktop stalled on memory\n"),
            "{text}"
        );
        assert!(!text.contains("nothing waited"), "{text}");
    }

    #[test]
    fn a_call_names_its_program_and_its_worst_thread() {
        let mut window = window(&[]);
        window.call.insert(
            "tuple".to_string(),
            Worst {
                thread: "AudioEncoder".to_string(),
                wait_us: 30.0,
            },
        );
        let text = text(&window, 0, &Pen::plain(), Room::Unbounded);
        assert!(
            text.contains("tuple in a call  AudioEncoder waits 30.0 µs a turn"),
            "{text}"
        );
    }

    #[test]
    fn durations_read_in_their_largest_two_units() {
        assert_eq!(duration(4), "4 s");
        assert_eq!(duration(134), "2 m 14 s");
        assert_eq!(duration(7380), "2 h 3 m");
        assert_eq!(duration(90_000), "1 d 1 h");
    }

    #[test]
    fn a_script_reads_the_window_named_by_its_schema() {
        let answer = json(&window(&[]), 4);
        assert_eq!(answer["schema"], "lag-watch.now/1");
        assert_eq!(answer["age_seconds"], 4);
        assert_eq!(answer["window"]["band"], "quiet");
    }
}
