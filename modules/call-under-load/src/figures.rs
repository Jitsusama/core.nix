//! The call's figures, worked out from the samples each end of the call
//! took once a second, as `page.html` takes them.
//!
//! Long because the picture's and the sound's figures are read from the
//! same samples by the same helpers, and their tests sit beside them.

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
    /// When each freeze had ended, in whole seconds since the epoch: the
    /// time of the far sample whose count of freezes rose, once for each
    /// freeze it rose by, so a freeze can be laid beside lag-watch's
    /// ten-second rows to say what the machine was doing. Cut down to the
    /// second, as lag-watch's own row ends are, so a freeze falls in the
    /// row that held it. A stream that restarted still times its freezes
    /// here, though their count and length can't be told then.
    pub frozen_at: Vec<f64>,
    /// A click on the call's own tab to its next frame, in ms.
    pub clicks_p50: Option<f64>,
    pub clicks_p99: Option<f64>,
    /// Whether the far end played the near end's sound over the call, a
    /// figure of its own, so a call that heard nothing reads as a fault
    /// rather than as sound figures gone missing.
    pub heard: bool,
    /// How the far end heard the near end's sound: the share of what it
    /// played that it had to make up for late or lost packets, in %.
    pub concealed: Option<f64>,
    /// How unevenly the sound's packets came, p99 over the call, in ms.
    pub jitter_p99: Option<f64>,
    /// The sound's packets lost over the call, a count, so even one shows.
    pub lost: Option<f64>,
    /// Why the sound went unheard, or why some of its figures are missing,
    /// so each says why rather than vanishing.
    pub sound_why: Option<String>,
    /// How many of the near end's samples were counted.
    pub samples: usize,
}

/// The change in a running count from the first sample to the last.
fn grew(first: &Value, last: &Value, key: &str) -> Option<f64> {
    Some(last["aud"][key].as_f64()? - first["aud"][key].as_f64()?)
}

/// The far end's counts that only grow while one receiver runs, so a fall
/// between two samples means its counts were started again. Packets lost
/// isn't among them: it falls when a duplicate makes up for a loss.
const GROWING: [&str; 3] = [
    "totalSamplesReceived",
    "concealedSamples",
    "packetsReceived",
];

/// A share in %, rounded up to the tenth the hour files, so a share just
/// past its target never reads as on it; none when there was nothing to
/// share. The nudge down keeps a whole tenth's float error from rounding
/// it up a tenth more, and adding 0 makes a share of none 0 rather than -0.
fn share(part: f64, whole: f64) -> Option<f64> {
    (whole > 0.0).then(|| ((part / whole * 1000.0 - 1e-9).ceil() + 0.0) / 10.0)
}

/// The far end's sound, as the hour counts it.
#[derive(Debug, Default, PartialEq)]
struct Sound {
    heard: bool,
    concealed: Option<f64>,
    jitter_p99: Option<f64>,
    lost: Option<f64>,
    why: Option<String>,
}

/// The far end's sound, from its samples' running counts: whether it
/// played any, concealed as a share of what it played, its jitter as a
/// p99 and its packets lost as a count. A far end that played nothing
/// gives no figure but that it didn't, since its jitter and losses alone
/// would read as sound heard well. Counts started again partway can't
/// give a share, and a count never reported gives nothing; either way the
/// figures left say why.
fn heard(far: &[Value]) -> Sound {
    let unheard = |why: String| Sound {
        why: Some(why),
        ..Sound::default()
    };
    if far.is_empty() {
        return unheard(format!(
            "the far end sent no samples past the first {SKIP}, which settle the call"
        ));
    }
    match far.iter().rev().find_map(|row| row["played"].as_str()) {
        None => return unheard("the far end never began playing what it heard".to_string()),
        Some("playing") => {}
        Some(said) => return unheard(format!("the far end's playing was {said}")),
    }
    let sound: Vec<&Value> = far
        .iter()
        .filter(|row| row["aud"]["totalSamplesReceived"].is_number())
        .collect();
    let (Some(first), Some(end)) = (sound.first(), sound.last()) else {
        return unheard("the far end counted no sound".to_string());
    };
    if sound.len() < 2 {
        return unheard("the far end counted sound only once, too few to tell".to_string());
    }
    // Each step in `key` between the samples that carry it, so a fall
    // across a sample that left it out is still a fall.
    let steps = |key: &str| -> Vec<(f64, f64)> {
        let counts: Vec<f64> = sound
            .iter()
            .filter_map(|row| row["aud"][key].as_f64())
            .collect();
        counts.windows(2).map(|pair| (pair[0], pair[1])).collect()
    };
    let rose = |key: &str| steps(key).iter().any(|(a, b)| b > a);
    let fell = |key: &str| steps(key).iter().any(|(a, b)| b < a);
    if !rose("totalSamplesReceived") {
        return unheard("the far end played no sound over the call".to_string());
    }
    let jitter: Vec<f64> = sound
        .iter()
        .filter_map(|row| row["aud"]["jitter"].as_f64())
        .map(|s| s * 1000.0)
        .collect();
    let jitter_p99 = percentile(&jitter, 99);
    let unreported = |missing: &[&str]| format!("the far end didn't report {}", missing.join(", "));
    if GROWING.iter().any(|key| fell(key)) {
        let mut why = "the far end's counts started again partway, so what it made up and lost \
                       can't be told"
            .to_string();
        if jitter_p99.is_none() {
            why = format!("{why}; {}", unreported(&["jitter"]));
        }
        return Sound {
            heard: true,
            jitter_p99,
            why: Some(why),
            ..Sound::default()
        };
    }
    // Every sample the far end played counts, its quiet between beeps
    // too, since that quiet stands in for Joel talking.
    let concealed = grew(first, end, "concealedSamples")
        .zip(grew(first, end, "totalSamplesReceived"))
        .and_then(|(made_up, played)| share(made_up, played));
    // A fall in packets lost is a duplicate making up for a loss, so none.
    let lost = grew(first, end, "packetsLost")
        .filter(|_| grew(first, end, "packetsReceived").is_some())
        .map(|lost| lost.max(0.0));
    let missing: Vec<&str> = [
        (concealed.is_none(), "concealedSamples"),
        (jitter_p99.is_none(), "jitter"),
        (lost.is_none(), "packetsLost or packetsReceived"),
    ]
    .into_iter()
    .filter_map(|(gone, key)| gone.then_some(key))
    .collect();
    Sound {
        heard: true,
        concealed,
        jitter_p99,
        lost,
        why: (!missing.is_empty()).then(|| unreported(&missing)),
    }
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
    // Chrome counts a freeze once its next frame comes, so the sample
    // whose count rose is the one by which it had ended, and a count that
    // rose by two between samples ended two.
    let frozen_at = frozen
        .windows(2)
        .flat_map(|pair| {
            let risen = pair[1]["in"]["freezeCount"]
                .as_u64()
                .zip(pair[0]["in"]["freezeCount"].as_u64())
                .map_or(0, |(b, a)| b.saturating_sub(a));
            let at = pair[1]["t"].as_f64().map(|ms| (ms / 1000.0).floor());
            std::iter::repeat_n(at, usize::try_from(risen).unwrap_or(0)).flatten()
        })
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
        // A count that fell between any two samples came from a stream that
        // restarted, so it says nothing of the call's freezes rather than
        // reading as fewer. A restart can start again from 0 and climb past
        // the first sample's count, so first and last alone can't show it.
        let fell = |key: &str| {
            frozen.windows(2).any(|pair| {
                pair[1]["in"][key]
                    .as_f64()
                    .zip(pair[0]["in"][key].as_f64())
                    .is_some_and(|(b, a)| b < a)
            })
        };
        let change = |key: &str| {
            last["in"][key]
                .as_f64()
                .zip(first["in"][key].as_f64())
                .filter(|_| !fell(key))
        };
        // Rounded up to the tenth, its worse side, as cadence shows it.
        freezes = change("freezeCount")
            .map(|(b, a)| (((b - a) / tenths * 10.0 - 1e-9).ceil() + 0.0) / 10.0)
            .filter(|f| f.is_finite());
        frozen_s = change("totalFreezesDuration").map(|(b, a)| ((b - a) * 100.0).round() / 100.0);
    }
    let sound = heard(far);
    Call {
        camera: percentile(&camera, 10),
        sent: percentile(&sent, 10),
        got: percentile(&got, 10),
        freezes,
        frozen_s,
        frozen_at,
        clicks_p50: percentile(clicks, 50),
        clicks_p99: percentile(clicks, 99),
        heard: sound.heard,
        concealed: sound.concealed,
        jitter_p99: sound.jitter_p99,
        lost: sound.lost,
        sound_why: sound.why,
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
        assert_eq!(
            figures.frozen_at,
            vec![40.0],
            "the freeze wasn't timed by the sample that counted it"
        );
        assert_eq!(figures.clicks_p99, Some(9.0));
        // Sampled late, every 1.2 s as a loaded timer runs, the same freeze
        // over the same 70.8 s reads by the clock, not the sample count.
        for (t, row) in far.iter_mut().enumerate() {
            row["t"] = json!(t * 1200);
        }
        assert_eq!(call(&mine, &far, &[]).freezes, Some(8.5));
        // Every 1.1 s, the freeze over 64.9 s is 9.245 in ten minutes,
        // which rounds up to its worse side, never down to 9.2.
        for (t, row) in far.iter_mut().enumerate() {
            row["t"] = json!(t * 1100);
        }
        assert_eq!(
            call(&mine, &far, &[]).freezes,
            Some(9.3),
            "freezes rounded down"
        );
        // Two freezes, the second a sample later, are each timed, in
        // whole seconds of the samples' own clock.
        far[41]["in"]["freezeCount"] = json!(2);
        for row in far.iter_mut().skip(42) {
            row["in"]["freezeCount"] = json!(2);
        }
        assert_eq!(
            call(&mine, &far, &[]).frozen_at,
            vec![44.0, 45.0],
            "each freeze wasn't timed"
        );
        // Two freezes ended between one sample and the next are both
        // timed, and a time is cut down to its second, as lag-watch's are,
        // never rounded up into the next row.
        far[41]["in"]["freezeCount"] = json!(3);
        far[41]["t"] = json!(45_600);
        for row in far.iter_mut().skip(42) {
            row["in"]["freezeCount"] = json!(3);
        }
        assert_eq!(
            call(&mine, &far, &[]).frozen_at,
            vec![44.0, 45.0, 45.0],
            "two freezes in one sample weren't each timed, or a time was rounded up"
        );
        far[41]["t"] = json!(41 * 1100);
        for row in far.iter_mut().skip(41) {
            row["in"]["freezeCount"] = json!(1);
        }
        // Samples with no time between them give no rate at all.
        let mut still = far.clone();
        for row in &mut still {
            row["t"] = json!(0);
        }
        let still = call(&mine, &still, &[]);
        assert_eq!(
            (still.freezes, still.frozen_s),
            (None, Some(0.4)),
            "a call with no time read as a freezes rate"
        );
        // A stream that restarted mid-call, its counts back at 0 and then
        // climbing past the first sample's, is unknown, not fewer freezes.
        for (t, row) in far.iter_mut().enumerate() {
            let frozen = t >= 40 && !(50..60).contains(&t);
            row["in"]["freezeCount"] = json!(u32::from(frozen));
            row["in"]["totalFreezesDuration"] = json!(if frozen { 0.4 } else { 0.0 });
        }
        let restarted = call(&mine, &far, &[]);
        assert_eq!(
            (restarted.freezes, restarted.frozen_s),
            (None, None),
            "a restarted stream's counts read as a figure"
        );
    }

    #[test]
    fn the_far_end_s_sound_is_counted_from_its_running_counts_once_the_call_settles() {
        // 48,000 samples a second played and 50 packets heard, with 480
        // made up and one packet lost each second once the call settles;
        // its first seconds, left out, made up everything and lost three
        // packets a second, with one jitter spike that mustn't count. The
        // settled call's worst jitter comes midway, not at its end.
        let far: Vec<Value> = (0..30_u32)
            .map(|t| {
                let settled = f64::from(t.saturating_sub(10));
                let early = f64::from(t.min(10));
                playing(
                    t,
                    &json!({
                        "totalSamplesReceived": 48_000.0 * f64::from(t),
                        "concealedSamples": 48_000.0 * early + 480.0 * settled,
                        "packetsReceived": 50.0 * f64::from(t) - 3.0 * early - settled,
                        "packetsLost": 3.0 * early + settled,
                        "jitter": match t { 5 => 0.090, 20 => 0.040, _ => 0.010 },
                    }),
                )
            })
            .collect();
        let figures = call(&[], &far, &[]);
        assert!(figures.heard, "a call that played sound reads as heard");
        assert_eq!(figures.concealed, Some(1.0), "480 in 48,000 is 1%");
        assert_eq!(
            figures.lost,
            Some(19.0),
            "one lost in each of the 19 seconds between the settled ends"
        );
        assert_eq!(
            figures.jitter_p99,
            Some(40.0),
            "the settled seconds' worst is the p99, wherever it comes"
        );
        assert_eq!(figures.sound_why, None);
    }

    /// One second of the far end's samples, playing what it hears.
    fn playing(t: u32, aud: &Value) -> Value {
        json!({"t": t * 1000, "played": "playing", "aud": aud})
    }

    #[test]
    fn a_call_that_heard_nothing_has_no_sound_figures_and_says_why() {
        let sound = |far: &[Value]| {
            let f = call(&[], far, &[]);
            (
                (f.heard, f.concealed, f.jitter_p99, f.lost),
                f.sound_why.unwrap_or_default(),
            )
        };
        let stat = json!({"totalSamplesReceived": 0.0, "packetsReceived": 50.0, "jitter": 0.01});
        // Packets still arrive and are counted when playing is refused.
        let arriving = |t: u32| {
            json!({"t": t * 1000, "played": "refused: NotAllowedError", "aud": {
                "totalSamplesReceived": 48_000.0 * f64::from(t),
                "concealedSamples": 0.0,
                "packetsReceived": 50.0 * f64::from(t),
                "packetsLost": 0.0,
                "jitter": 0.01,
            }})
        };
        let counted = |t: u32| {
            json!({
                "totalSamplesReceived": 48_000.0 * f64::from(t),
                "concealedSamples": 0.0,
                "packetsReceived": 50.0 * f64::from(t),
                "packetsLost": 0.0,
                "jitter": 0.01,
            })
        };
        // Playing until `at`, then what it said after, its counts growing.
        let stopped = |at: u32, said: &'static str| -> Vec<Value> {
            (0..30_u32)
                .map(|t| {
                    let played = if t < at { "playing" } else { said };
                    json!({"t": t * 1000, "played": played, "aud": counted(t)})
                })
                .collect()
        };
        let cases = [
            (Vec::new(), "sent no samples past the first 10"),
            // A far end that sent samples, but all within the settling ones.
            (
                (0..5).map(|t| playing(t, &counted(t))).collect(),
                "sent no samples past the first 10",
            ),
            (stopped(15, "stopped: pause"), "playing was stopped: pause"),
            (
                stopped(20, "failed: AbortError: gone"),
                "playing was failed: AbortError: gone",
            ),
            (
                (0..30)
                    .map(|t| json!({"t": t * 1000, "aud": stat}))
                    .collect::<Vec<_>>(),
                "never began playing",
            ),
            ((0..30).map(arriving).collect(), "refused: NotAllowedError"),
            (
                (0..30).map(|t| playing(t, &json!({}))).collect(),
                "counted no sound",
            ),
            (
                (0..30).map(|t| playing(t, &stat)).collect(),
                "played no sound",
            ),
            // One settled sample can't show sound growing, so isn't heard.
            (
                (0..11)
                    .map(|t| playing(t, &json!({"totalSamplesReceived": 1.0})))
                    .collect(),
                "counted sound only once",
            ),
        ];
        for (far, why) in cases {
            let (figures, said) = sound(&far);
            assert_eq!(
                figures,
                (false, None, None, None),
                "jitter or loss read with nothing heard, for {why:?}"
            );
            assert!(said.contains(why), "{said:?} doesn't say {why:?}");
        }
    }

    #[test]
    fn a_figure_the_far_end_never_reported_says_so() {
        // The far end's counts with `gone` left out of every sample.
        let without = |gone: &[&str]| -> Vec<Value> {
            (0..30_u32)
                .map(|t| {
                    let mut aud = json!({
                        "totalSamplesReceived": 48_000.0 * f64::from(t),
                        "concealedSamples": 0.0,
                        "packetsReceived": 50.0 * f64::from(t),
                        "packetsLost": 0.0,
                        "jitter": 0.01,
                    });
                    if let Some(aud) = aud.as_object_mut() {
                        for key in gone {
                            aud.remove(*key);
                        }
                    }
                    playing(t, &aud)
                })
                .collect()
        };
        let cases = [
            (vec!["jitter"], (Some(0.0), None, Some(0.0)), "jitter"),
            (
                vec!["concealedSamples"],
                (None, Some(10.0), Some(0.0)),
                "concealedSamples",
            ),
            (
                vec!["packetsLost"],
                (Some(0.0), Some(10.0), None),
                "packetsLost or packetsReceived",
            ),
            (
                vec!["packetsReceived"],
                (Some(0.0), Some(10.0), None),
                "packetsLost or packetsReceived",
            ),
            (
                vec!["concealedSamples", "jitter"],
                (None, None, Some(0.0)),
                "concealedSamples, jitter",
            ),
        ];
        for (gone, left, said) in cases {
            let figures = call(&[], &without(&gone), &[]);
            assert_eq!(
                (figures.concealed, figures.jitter_p99, figures.lost),
                left,
                "the figures left without {gone:?}"
            );
            assert!(figures.heard, "sound played without {gone:?} is heard");
            assert_eq!(
                figures.sound_why,
                Some(format!("the far end didn't report {said}")),
                "what's said without {gone:?}"
            );
        }
    }

    #[test]
    fn a_fall_across_a_sample_without_the_count_is_still_a_restart() {
        // concealedSamples falls from 100 to 5 across a sample that left it
        // out, which neighbouring pairs alone would never compare.
        let far: Vec<Value> = (0..30_u32)
            .map(|t| {
                let mut aud = json!({
                    "totalSamplesReceived": 48_000.0 * f64::from(t),
                    "concealedSamples": if t < 20 { 100.0 } else { 5.0 },
                    "packetsReceived": 50.0 * f64::from(t),
                    "packetsLost": 0.0,
                    "jitter": 0.01,
                });
                if let Some(aud) = aud.as_object_mut().filter(|_| t == 19) {
                    aud.remove("concealedSamples");
                }
                playing(t, &aud)
            })
            .collect();
        let figures = call(&[], &far, &[]);
        assert_eq!(
            figures.concealed, None,
            "a fall across the gap gave a share"
        );
        assert!(
            figures
                .sound_why
                .as_deref()
                .is_some_and(|why| why.contains("started again partway")),
            "the restart isn't said: {:?}",
            figures.sound_why
        );
    }

    #[test]
    fn a_share_is_rounded_up_to_the_tenth_the_hour_files() {
        assert_eq!(share(104.0, 10_000.0), Some(1.1), "1.04% is past 1%");
        assert_eq!(share(100.0, 10_000.0), Some(1.0), "1% is on 1%");
        // No share under a whole has shown the float error in three million
        // tries, but 403 over 100 multiplies out to 4030.0000000000005, so
        // the guard against it is pinned there.
        assert_eq!(
            share(403.0, 100.0),
            Some(403.0),
            "a whole tenth's float error rounded it up"
        );
        let none = share(0.0, 48_000.0).unwrap_or(f64::NAN);
        assert!(
            none == 0.0 && none.is_sign_positive(),
            "no share is 0, not -0: {none}"
        );
        assert_eq!(share(1.0, 0.0), None, "nothing played, nothing shared");
    }

    #[test]
    fn a_count_started_again_partway_reads_as_unknown_never_as_better() {
        // At second `at` the far end's made-up and received counts start
        // again, the samples it played too when `all`, as a remade
        // receiver's would. Early enough, they grow back past where the
        // settled call began, which comparing its ends alone would miss.
        let restarted = |at: u32, all: bool| -> Vec<Value> {
            (0..30_u32)
                .map(|t| {
                    let since = f64::from(if t < at { t } else { t - at });
                    let played = if all { since } else { f64::from(t) };
                    playing(
                        t,
                        &json!({
                            "totalSamplesReceived": 48_000.0 * played,
                            "concealedSamples": 4_800.0 * since,
                            "packetsReceived": 49.0 * since,
                            "packetsLost": since,
                            "jitter": 0.01,
                        }),
                    )
                })
                .collect()
        };
        // One count started again alone, the others growing on.
        let alone = |key: &str| -> Vec<Value> {
            (0..30_u32)
                .map(|t| {
                    let mut aud = json!({
                        "totalSamplesReceived": 48_000.0 * f64::from(t),
                        "concealedSamples": 4_800.0 * f64::from(t),
                        "packetsReceived": 49.0 * f64::from(t),
                        "packetsLost": f64::from(t),
                        "jitter": 0.01,
                    });
                    if t >= 20 {
                        aud[key] = json!(10.0 * f64::from(t - 20));
                    }
                    playing(t, &aud)
                })
                .collect()
        };
        let mut cases: Vec<(String, Vec<Value>)> = [(20, false), (12, false), (20, true)]
            .into_iter()
            .map(|(at, all)| (format!("at {at} (all: {all})"), restarted(at, all)))
            .collect();
        cases.extend(
            GROWING
                .into_iter()
                .map(|key| (format!("{key} alone"), alone(key))),
        );
        for (case, far) in cases {
            let figures = call(&[], &far, &[]);
            assert_eq!(
                (figures.heard, figures.concealed, figures.lost),
                (true, None, None),
                "counts started again {case} gave a share"
            );
            assert_eq!(figures.jitter_p99, Some(10.0), "jitter isn't a count");
            assert!(
                figures
                    .sound_why
                    .as_deref()
                    .is_some_and(|why| why.contains("started again partway")),
                "a restart {case} isn't said: {:?}",
                figures.sound_why
            );
        }
    }

    #[test]
    fn a_restart_without_jitter_says_jitter_went_unreported_too() {
        let far: Vec<Value> = (0..30_u32)
            .map(|t| {
                let since = f64::from(if t < 20 { t } else { t - 20 });
                playing(
                    t,
                    &json!({
                        "totalSamplesReceived": 48_000.0 * f64::from(t),
                        "concealedSamples": 4_800.0 * since,
                        "packetsReceived": 49.0 * since,
                        "packetsLost": since,
                    }),
                )
            })
            .collect();
        let figures = call(&[], &far, &[]);
        assert_eq!(
            (figures.heard, figures.jitter_p99),
            (true, None),
            "a restart without jitter gave a jitter"
        );
        assert!(
            figures.sound_why.as_deref().is_some_and(|why| {
                why.contains("started again partway") && why.contains("didn't report jitter")
            }),
            "the unreported jitter isn't said beside the restart: {:?}",
            figures.sound_why
        );
    }

    #[test]
    fn a_duplicate_making_up_for_a_loss_counts_as_none_lost() {
        // packetsLost falls by one when a late duplicate arrives.
        let far: Vec<Value> = (0..30_u32)
            .map(|t| {
                playing(
                    t,
                    &json!({
                        "totalSamplesReceived": 48_000.0 * f64::from(t),
                        "concealedSamples": 0.0,
                        "packetsReceived": 50.0 * f64::from(t),
                        "packetsLost": if t < 15 { 1.0 } else { 0.0 },
                    }),
                )
            })
            .collect();
        assert_eq!(
            call(&[], &far, &[]).lost,
            Some(0.0),
            "never below none lost"
        );
    }

    #[test]
    fn a_call_with_too_few_samples_has_no_figures() {
        let figures = call(&[at(0, 0.0)], &[], &[]);
        assert_eq!(figures.samples, 0);
        assert_eq!(
            (figures.camera, figures.freezes, figures.clicks_p99),
            (None, None, None)
        );
        assert!(!figures.heard, "a call with no far end isn't heard");
        assert!(
            figures
                .sound_why
                .as_deref()
                .is_some_and(|why| why.starts_with("the far end sent no samples")),
            "{:?}",
            figures.sound_why
        );
    }
}
