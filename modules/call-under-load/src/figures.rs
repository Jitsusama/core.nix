//! The call's figures, worked out from the samples each end of the call
//! took once a second, as `page.html` takes them.

use serde::Serialize;
use serde_json::Value;

/// The first samples of a call, left out while the encoder ramps up.
pub const SKIP: usize = 10;
/// The frames a second the call is offered at, and so its target.
pub const FPS: f64 = 30.0;
/// How many samples a rate is counted over: five seconds.
const SPAN: usize = 5;

/// The nearest rank, so a short list's p99 is its worst.
pub fn percentile(values: &[f64], p: usize) -> Option<f64> {
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let rank = (sorted.len() * p)
        .div_ceil(100)
        .clamp(1, sorted.len().max(1));
    sorted.get(rank - 1).copied()
}

/// Frames a second over each five seconds of samples, from a running
/// count: `per` gives, from a sample, the layers an encoded frame is
/// counted in as of it. A window whose two ends count in different layers
/// is left out, since how its frames split between them can't be told.
///
/// Chrome's own `framesPerSecond` counts whole frames in the last second,
/// so a steady 30 reads 29 in about half the seconds; a count over five
/// seconds reads it as 30 and still shows a real dip, as a 0.4 s freeze
/// takes a window to about 27.6. The stats are sampled a few ms either
/// side of the frame clock, so a steady 30 read 29.8 in a window that
/// closed just before its 150th frame, and 30.2 in the next. So each
/// window is timed in whole frame slots and forgiven the one frame its
/// edges can cost, up to the rate itself: a dip of two frames or more, a
/// 67 ms stall, still shows.
pub fn rates(rows: &[Value], side: &str, key: &str, per: fn(&Value) -> f64) -> Vec<f64> {
    rows.iter()
        .zip(rows.iter().skip(SPAN))
        .filter_map(|(a, b)| {
            let (from, to) = (a["t"].as_f64()?, b["t"].as_f64()?);
            let frames = (b[side][key].as_f64()? - a[side][key].as_f64()?) / per(b);
            if to <= from || per(a).total_cmp(&per(b)).is_ne() {
                return None;
            }
            let slots = ((to - from) / 1000.0 * FPS).round().max(1.0);
            Some((frames + 1.0).min(slots) / (slots / FPS))
        })
        .collect()
}

/// A frame counted once, as a camera's and a decoder's are.
pub fn whole(_: &Value) -> f64 {
    1.0
}

/// How many spatial layers each encoded frame is counted in as of a
/// sample, from its scalability mode (`L3T3_KEY` is three). It's read for
/// each window, since Chrome sheds layers when the CPU can't keep up, and
/// a frame sent in fewer is still a frame sent.
pub fn layers(row: &Value) -> f64 {
    row["out"]["scalabilityMode"]
        .as_str()
        .and_then(|mode| mode.strip_prefix('L')?.chars().next()?.to_digit(10))
        .map_or(1.0, f64::from)
}

/// What the call did, from both ends' samples and the clicks' times.
#[derive(Debug, PartialEq, Serialize)]
pub struct Call {
    /// The camera's frames a second, its p10.
    pub camera: Option<f64>,
    /// What the near end encoded and sent, its p10.
    pub sent: Option<f64>,
    /// What the far end decoded, its p10.
    pub got: Option<f64>,
    /// The far end's freezes, per ten minutes.
    pub freezes: Option<f64>,
    /// How long the far end was frozen, in seconds, over the call.
    pub frozen_s: Option<f64>,
    /// A click on the call's own tab to its next frame, in ms.
    pub clicks_p50: Option<f64>,
    pub clicks_p99: Option<f64>,
    /// How many of the near end's samples were counted.
    pub samples: usize,
}

pub fn call(mine: &[Value], far: &[Value], clicks: &[f64]) -> Call {
    let mine = mine.get(SKIP..).unwrap_or_default();
    let far = far.get(SKIP..).unwrap_or_default();
    // The source's running frame count where Chrome gives one, and its
    // whole-second figure where it doesn't.
    let mut camera = rates(mine, "src", "frames", whole);
    if camera.is_empty() {
        camera = mine
            .iter()
            .filter_map(|row| row["src"]["framesPerSecond"].as_f64())
            .filter(|fps| *fps > 0.0)
            .collect();
    }
    let sent = rates(mine, "out", "framesEncoded", layers);
    let got = rates(far, "in", "framesDecoded", whole);
    let frozen: Vec<&Value> = far
        .iter()
        .filter(|row| row["in"]["freezeCount"].is_number())
        .collect();
    let (mut freezes, mut frozen_s) = (None, None);
    if let (Some(first), Some(last)) = (frozen.first(), frozen.last())
        && frozen.len() > 1
    {
        // Timed by the samples' own clock, since a loaded machine's
        // one-second timer runs late, and counting samples would read the
        // call as shorter than it was.
        let span_ms = last["t"]
            .as_f64()
            .zip(first["t"].as_f64())
            .map(|(b, a)| b - a);
        let tenths = span_ms.filter(|ms| *ms > 0.0).unwrap_or(f64::NAN) / 600_000.0;
        let change = |key: &str| last["in"][key].as_f64().zip(first["in"][key].as_f64());
        freezes = change("freezeCount").map(|(b, a)| ((b - a) / tenths * 10.0).round() / 10.0);
        frozen_s = change("totalFreezesDuration").map(|(b, a)| ((b - a) * 100.0).round() / 100.0);
    }
    Call {
        camera: percentile(&camera, 10),
        sent: percentile(&sent, 10),
        got: percentile(&got, 10),
        freezes,
        frozen_s,
        clicks_p50: percentile(clicks, 50),
        clicks_p99: percentile(clicks, 99),
        samples: mine.len(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// A count as a float: samples number in the hundreds.
    fn count(n: usize) -> f64 {
        f64::from(u32::try_from(n).unwrap_or(u32::MAX))
    }

    /// A sample at `t` s with `frames` counted so far, at each stage.
    fn at(t: u64, frames: f64) -> Value {
        json!({"t": t * 1000, "src": {"frames": frames},
               "out": {"framesEncoded": frames * 3.0, "scalabilityMode": "L3T3_KEY"},
               "in": {"framesDecoded": frames}})
    }

    #[test]
    fn a_percentile_is_the_nearest_rank() {
        let values = [5.0, 1.0, 4.0, 2.0, 3.0];
        assert_eq!(percentile(&values, 10), Some(1.0));
        assert_eq!(percentile(&values, 50), Some(3.0));
        assert_eq!(percentile(&values, 99), Some(5.0));
        assert_eq!(percentile(&[], 99), None);
    }

    #[test]
    fn a_steady_thirty_reads_thirty_whichever_side_of_a_frame_it_is_read() {
        // Read just before the 150th frame: 149 counted, forgiven one.
        let rows = [
            at(0, 0.0),
            at(1, 30.0),
            at(2, 60.0),
            at(3, 90.0),
            at(4, 120.0),
            at(5, 149.0),
        ];
        assert_eq!(rates(&rows, "src", "frames", whole), vec![30.0]);
        // Read just after it: 151, held to the slots there were.
        let rows = [
            at(0, 0.0),
            at(1, 0.0),
            at(2, 0.0),
            at(3, 0.0),
            at(4, 0.0),
            at(5, 151.0),
        ];
        assert_eq!(rates(&rows, "src", "frames", whole), vec![30.0]);
    }

    #[test]
    fn a_stall_of_two_frames_shows() {
        let rows = [
            at(0, 0.0),
            at(1, 0.0),
            at(2, 0.0),
            at(3, 0.0),
            at(4, 0.0),
            at(5, 148.0),
        ];
        assert_eq!(rates(&rows, "src", "frames", whole), vec![29.8]);
    }

    #[test]
    fn an_encoded_frame_is_counted_once_however_many_layers_it_has() {
        let rows: Vec<Value> = (0..6)
            .map(|t| at(t, 30.0 * count(usize::try_from(t).unwrap())))
            .collect();
        assert_eq!(rates(&rows, "out", "framesEncoded", layers), vec![30.0]);
        // Sent in two layers throughout, after shedding one under load,
        // every frame still counts once.
        let shed: Vec<Value> = rows
            .iter()
            .map(|row| {
                let mut row = row.clone();
                row["out"]["framesEncoded"] = json!(row["src"]["frames"].as_f64().unwrap() * 2.0);
                row["out"]["scalabilityMode"] = json!("L2T3_KEY");
                row
            })
            .collect();
        // A window that sheds a layer partway can't say how its frames
        // split, so it's left out rather than read as a dip or a surplus.
        let mut shedding = rows.clone();
        shedding[5] = shed[5].clone();
        assert_eq!(
            rates(&shedding, "out", "framesEncoded", layers),
            Vec::<f64>::new()
        );
        assert_eq!(rates(&shed, "out", "framesEncoded", layers), vec![30.0]);
        let one = layers(&json!({"out": {}}));
        assert!(
            (one - 1.0).abs() < f64::EPSILON,
            "a call with no layers named counts as one, read as {one}"
        );
    }

    #[test]
    fn a_call_leaves_out_its_first_seconds_and_counts_the_far_end_s_freezes() {
        let seconds = |n: u64| -> Vec<Value> {
            (0..n)
                .map(|t| at(t, 30.0 * count(usize::try_from(t).unwrap())))
                .collect()
        };
        let mine = seconds(70);
        let mut far = seconds(70);
        // A freeze of 0.4 s once the call settles, 60 samples counted: one
        // in a minute is ten in ten minutes.
        for (t, row) in far.iter_mut().enumerate() {
            let frozen = t >= 40;
            row["in"]["freezeCount"] = json!(u32::from(frozen));
            row["in"]["totalFreezesDuration"] = json!(if frozen { 0.4 } else { 0.0 });
        }
        let figures = call(&mine, &far, &[3.0, 9.0]);
        assert_eq!(figures.samples, 60);
        assert_eq!(figures.camera, Some(30.0));
        assert_eq!(figures.sent, Some(30.0));
        assert_eq!(figures.got, Some(30.0));
        assert_eq!(figures.freezes, Some(10.2));
        assert_eq!(figures.frozen_s, Some(0.4));
        assert_eq!(figures.clicks_p99, Some(9.0));
        // Sampled late, every 1.2 s as a loaded timer runs, the same freeze
        // over the same 70.8 s reads by the clock, not the sample count.
        for (t, row) in far.iter_mut().enumerate() {
            row["t"] = json!(t * 1200);
        }
        assert_eq!(call(&mine, &far, &[]).freezes, Some(8.5));
    }

    #[test]
    fn a_call_with_too_few_samples_has_no_figures() {
        let figures = call(&[at(0, 0.0)], &[], &[]);
        assert_eq!(figures.samples, 0);
        assert_eq!(
            (figures.camera, figures.freezes, figures.clicks_p99),
            (None, None, None)
        );
    }
}
