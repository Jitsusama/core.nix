use std::io::Cursor;

use pipewire::spa::pod::serialize::PodSerializer;
use pipewire::spa::pod::{Object, Property, PropertyFlags, Value};
use pipewire::spa::sys;
use pipewire::spa::utils::Fraction;
use serde_json::json;

use super::{Timings, profiled};

/// A profile as the profiler says one: an object for each driver's cycle.
fn profile(cycles: Vec<Vec<(u32, Value)>>) -> Vec<u8> {
    let objects = cycles
        .into_iter()
        .map(|said| {
            Value::Object(Object {
                type_: sys::SPA_TYPE_OBJECT_Profiler,
                id: 0,
                properties: said
                    .into_iter()
                    .map(|(key, value)| Property {
                        key,
                        flags: PropertyFlags::empty(),
                        value,
                    })
                    .collect(),
            })
        })
        .collect();
    PodSerializer::serialize(Cursor::new(Vec::new()), &Value::Struct(objects))
        .map(|(written, _)| written.into_inner())
        .expect("a profile is written")
}

fn info(xruns: i32) -> (u32, Value) {
    let load = Value::Float(0.1);
    let said = vec![
        Value::Long(7),
        load.clone(),
        load.clone(),
        load,
        Value::Int(xruns),
    ];
    (sys::SPA_PROFILER_info, Value::Struct(said))
}

/// A clock of `duration` frames at `rate` a second.
fn clock(duration: i64, rate: u32) -> (u32, Value) {
    let said = vec![
        Value::Int(0),
        Value::Int(79),
        Value::String("alsa".to_owned()),
        Value::Long(0),
        Value::Fraction(Fraction {
            num: 1,
            denom: rate,
        }),
        Value::Long(0),
        Value::Long(duration),
        Value::Long(0),
        Value::Double(1.0),
        Value::Long(0),
    ];
    (sys::SPA_PROFILER_clock, Value::Struct(said))
}

/// A node's block: when it was signalled last, woke and finished, in
/// nanoseconds, and its xruns where it says them.
fn block(key: u32, id: i32, times: [i64; 3], xruns: Option<i32>) -> (u32, Value) {
    let [prev, awake, finish] = times;
    let mut said = vec![
        Value::Int(id),
        Value::String(format!("node {id}")),
        Value::Long(prev),
        Value::Long(prev + 10),
        Value::Long(awake),
        Value::Long(finish),
        Value::Int(3),
        Value::Fraction(Fraction {
            num: 256,
            denom: 48_000,
        }),
    ];
    said.extend(xruns.map(Value::Int));
    (key, Value::Struct(said))
}

fn driver(id: i32, times: [i64; 3], xruns: Option<i32>) -> (u32, Value) {
    block(sys::SPA_PROFILER_driverBlock, id, times, xruns)
}

fn follower(id: i32, times: [i64; 3], xruns: Option<i32>) -> (u32, Value) {
    block(sys::SPA_PROFILER_followerBlock, id, times, xruns)
}

fn near(said: Option<f64>, meant: f64) -> bool {
    said.is_some_and(|said| (said - meant).abs() < 1e-6)
}

#[test]
fn a_cycle_says_each_node_in_it_on_its_drivers_clock() {
    // 1024 frames at 48 kHz is 21.333 ms; the driver was busy 2.1333 ms
    // of it and its follower a tenth as long.
    let heard = profiled(&profile(vec![vec![
        info(1),
        clock(1024, 48_000),
        driver(79, [0, 100, 2_133_433], None),
        follower(93, [0, 200, 213_533], Some(2)),
    ]]));
    assert_eq!(heard.len(), 2, "{heard:?}");
    let (drives, follows) = (&heard[0], &heard[1]);
    assert_eq!(
        (
            drives.id,
            drives.quantum,
            drives.rate,
            drives.drives,
            drives.errors
        ),
        (79, 1024, 48_000, true, 1),
        "the driver, its xruns the cycle's: {drives:?}"
    );
    assert!(near(drives.busy, 0.1), "busy a tenth of it: {drives:?}");
    assert_eq!(
        (
            follows.id,
            follows.quantum,
            follows.rate,
            follows.drives,
            follows.errors
        ),
        (93, 1024, 48_000, false, 2),
        "the follower on its driver's cycle, with its own xruns: {follows:?}"
    );
    assert!(near(follows.busy, 0.01), "busy a hundredth: {follows:?}");
}

#[test]
fn a_node_woken_and_not_finished_ran_past_its_cycle() {
    let heard = profiled(&profile(vec![vec![
        clock(1024, 48_000),
        driver(79, [0, 100, 50], None),
        follower(93, [500, 100, 50], None),
    ]]));
    assert_eq!(heard.len(), 2, "{heard:?}");
    assert!(
        heard[0].unfinished && heard[0].busy.is_none(),
        "woke after it was signalled, and never finished: {:?}",
        heard[0]
    );
    assert!(
        !heard[1].unfinished && heard[1].busy.is_none(),
        "never woke since, so it says nothing of its load: {:?}",
        heard[1]
    );
}

#[test]
fn a_block_with_no_clock_said_has_no_cycle() {
    let heard = profiled(&profile(vec![vec![driver(79, [0, 100, 200], None)]]));
    assert_eq!(heard, Vec::new(), "no cycle to say it in");
    assert_eq!(
        profiled(b"not a pod"),
        Vec::new(),
        "nor anything that isn't one"
    );
}

#[test]
fn a_record_says_the_figures_as_heard() {
    let heard = profiled(&profile(vec![vec![
        clock(1000, 1000),
        driver(79, [0, 100, 100_000_100], Some(0)),
    ]]));
    assert_eq!(
        heard[0].record(),
        json!({
            "key": "timing:79", "kind": "timing", "id": 79, "quantum": 1000, "rate": 1000,
            "ms": 1000.0, "busy": 0.1, "unfinished": false, "errors": 0, "drives": true,
        }),
        "its cycle's length in milliseconds among them"
    );
}

#[test]
fn a_frame_stands_each_second_and_a_node_not_in_it_goes() {
    let both = profile(vec![vec![
        clock(1024, 48_000),
        driver(79, [0, 100, 200], None),
        follower(93, [0, 100, 200], None),
    ]]);
    let alone = profile(vec![vec![
        clock(1024, 48_000),
        driver(79, [0, 100, 200], None),
    ]]);
    let mut timings = Timings::default();
    assert!(!timings.hear(&both, 5_000), "the first gathers");
    assert!(
        !timings.hear(&both, 5_500),
        "as does the rest of its second"
    );
    let none: Vec<serde_json::Value> = Vec::new();
    assert_eq!(timings.records(), none, "nothing stands yet");
    assert!(timings.hear(&alone, 6_000), "a second on, it stands");
    let ids = |timings: &Timings| -> Vec<u64> {
        let records = timings.records();
        records
            .iter()
            .filter_map(|record| record["id"].as_u64())
            .collect()
    };
    assert_eq!(ids(&timings), vec![79, 93], "both ran in it");
    assert!(timings.hear(&alone, 7_000), "and the next");
    assert_eq!(
        ids(&timings),
        vec![79],
        "the follower ran in none of it, so it goes"
    );
    timings.forget();
    assert_eq!(timings.records(), none, "forgotten, none stands");
}
