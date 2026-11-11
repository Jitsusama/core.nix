//! The measure's report: what each phase of the frame-time script took, against
//! one refresh interval.

use super::{Drawn, memory};
use crate::{App, Measure};

impl App {
    #[expect(
        clippy::cast_precision_loss,
        reason = "the view is measured in f64 pixels and points, and a band, a buffer or a zoom is a whole number within a screen's or a page's few hundred thousand"
    )]
    pub(crate) fn report(&self) {
        let Some(measure) = self.measure.as_ref() else {
            return;
        };
        // A compositor without a fixed refresh, as a headless one, gives
        // none, so the median interval stands in for it.
        let all: Vec<f64> = measure
            .drawn
            .windows(2)
            .filter_map(|pair| Some((pair[1].presented_ns? - pair[0].presented_ns?) as f64 / 1e6))
            .collect();
        let refresh_given = measure.refresh_ns > 0;
        let refresh_ms = if refresh_given {
            f64::from(measure.refresh_ns) / 1e6
        } else {
            percentile(&all, 0.5)
        };
        let mut phases = serde_json::Map::new();
        for name in ["scroll", "jump", "zoom", "resize"] {
            phases.insert(
                name.to_string(),
                Self::phase_report(measure, name, refresh_ms),
            );
        }
        let worst = phases
            .values()
            .map(|p| p["interval_ms_p99"].as_f64().unwrap_or(f64::MAX))
            .fold(0.0, f64::max);
        let report = serde_json::json!({
            "refresh_ms": refresh_ms,
            "refresh_given": refresh_given,
            "scale": self.scale,
            "size": [self.size.0, self.size.1],
            "set_ms": measure.set_ms,
            "set_parts": measure.set_parts.iter().map(|(name, ms, waited)| serde_json::json!({"part": name, "ms": (ms * 10.0).round() / 10.0, "waited_ms": (waited * 10.0).round() / 10.0})).collect::<Vec<_>>(),
            "configured_ms": measure.configured_ms,
            "rest_set_ms": measure.rest_ms,
            "first_frame_ms": measure.first_frame_ms,
            "band_drawn_ms_p50": percentile(&measure.drawn_ms, 0.50),
            "band_drawn_ms_p99": percentile(&measure.drawn_ms, 0.99),
            "discarded": measure.discarded,
            "jumps_missing": measure.missing,
            "jumps_held_ms": measure.held_ms.iter().map(|ms| (ms * 10.0).round() / 10.0).collect::<Vec<_>>(),
            "configures_while_resizing": measure.configures,
            "worst_p99_interval_ms": worst,
            "within_one_refresh": refresh_ms > 0.0 && worst <= refresh_ms * 1.05,
            "phases": phases,
            "memory_mb": memory(),
            "held_at_each_end": measure.held.iter().map(|(p, m, n)| serde_json::json!({"phase": p, "memory_mb": m, "bands": n})).collect::<Vec<_>>(),
        });
        println!("{report}");
    }

    /// One phase of the measure's script: its frames, when each reached
    /// the screen against the refresh, and what each frame's work took.
    #[expect(
        clippy::cast_precision_loss,
        reason = "nanoseconds between frames, which never pass a mantissa's 52 bits"
    )]
    fn phase_report(measure: &Measure, name: &str, refresh_ms: f64) -> serde_json::Value {
        let frames: Vec<(usize, &Drawn)> = measure
            .drawn
            .iter()
            .enumerate()
            .filter(|(_, d)| d.phase == name)
            .collect();
        let mut intervals = Vec::new();
        for (i, d) in &frames {
            if *i == 0 {
                continue;
            }
            if let (Some(now), Some(before)) = (d.presented_ns, measure.drawn[i - 1].presented_ns) {
                intervals.push((now.saturating_sub(before)) as f64 / 1e6);
            }
        }
        let work: Vec<f64> = frames.iter().map(|(_, d)| d.work_ms).collect();
        let drops = intervals
            .iter()
            .filter(|ms| **ms > 1.5 * refresh_ms)
            .count();
        let blank = frames.iter().filter(|(_, d)| d.blank).count();
        let stood_in = frames.iter().filter(|(_, d)| d.stood_in).count();
        let part = |n: usize| {
            let all: Vec<f64> = frames.iter().map(|(_, d)| d.parts_ms[n]).collect();
            percentile(&all, 1.0)
        };
        let worst_frame = frames
                .iter()
                .max_by(|a, b| a.1.work_ms.total_cmp(&b.1.work_ms))
                .map(|(_, d)| serde_json::json!({"t": d.t, "work_ms": d.work_ms, "upload_ms": d.parts_ms[0], "evict_ms": d.parts_ms[1], "swap_ms": d.parts_ms[2], "ask_ms": d.parts_ms[3], "script_ms": d.parts_ms[4], "draw_ms": d.parts_ms[5], "clear_ms": d.parts_ms[6], "waited_ms": d.waited_ms, "fresh": d.fresh}));
        let fresh_max = frames.iter().map(|(_, d)| d.fresh).max().unwrap_or(0);
        let slow_fresh = frames
            .iter()
            .filter(|(_, d)| d.parts_ms[5] > 4.0)
            .map(|(_, d)| d.fresh)
            .collect::<Vec<_>>();
        let waited: Vec<f64> = frames.iter().map(|(_, d)| d.waited_ms).collect();
        serde_json::json!({
            "frames": frames.len(),
            "presented": intervals.len(),
            "interval_ms_p50": percentile(&intervals, 0.50),
            "interval_ms_p99": percentile(&intervals, 0.99),
            "interval_ms_max": percentile(&intervals, 1.0),
            "dropped": drops,
            "work_ms_p50": percentile(&work, 0.50),
            "work_ms_p99": percentile(&work, 0.99),
            "work_ms_max": percentile(&work, 1.0),
            "upload_ms_max": part(0),
            "evict_ms_max": part(1),
            "swap_ms_max": part(2),
            "ask_ms_max": part(3),
            "script_ms_max": part(4),
            "draw_ms_max": part(5),
            "clear_ms_max": part(6),
            "fresh_max": fresh_max,
            "fresh_in_slow_draws": slow_fresh,
            "waited_ms_p99": percentile(&waited, 0.99),
            "waited_ms_max": percentile(&waited, 1.0),
            "worst_frame": worst_frame,
            "blank_frames": blank,
            "stand_in_frames": stood_in,
            "blank_at_s": frames.iter().filter(|(_, d)| d.blank).map(|(_, d)| (d.t * 1000.0).round() / 1000.0).collect::<Vec<_>>(),
        })
    }
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    reason = "the view is measured in f64 pixels and points, and a band, a buffer or a zoom is a whole number within a screen's or a page's few hundred thousand"
)]
fn percentile(values: &[f64], p: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let i = ((sorted.len() as f64 - 1.0) * p).round() as usize;
    (sorted[i] * 100.0).round() / 100.0
}

impl Measure {
    pub(crate) fn new(set_ms: f64, set_parts: Vec<(&'static str, f64, f64)>) -> Self {
        Measure {
            start: None,
            last: None,
            drawn: Vec::new(),
            refresh_ns: 0,
            discarded: 0,
            configures: 0,
            jumps: 0,
            drawn_ms: Vec::new(),
            set_ms,
            set_parts,
            configured_ms: None,
            rest_ms: None,
            first_frame_ms: None,
            held: Vec::new(),
            held_ms: Vec::new(),
            missing: Vec::new(),
        }
    }
}
