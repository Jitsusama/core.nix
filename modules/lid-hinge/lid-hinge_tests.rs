use super::*;

fn at(seconds: u64) -> Instant {
    // Instants can't be made from a number, so they're laid from one start.
    thread_local!(static START: Instant = Instant::now());
    START.with(|start| *start + Duration::from_secs(seconds))
}

const LIT: [Option<f64>; 2] = [Some(8000.0), Some(8000.0)];

#[test]
fn a_lid_coming_down_shuts_at_its_switch() {
    let mut judge = Judge::new(false);
    let crossed: Vec<bool> = [99.0, 62.0, 20.0, 8.0, 0.0]
        .into_iter()
        .map(|degrees| judge.hinge(degrees))
        .collect();
    assert_eq!(crossed, [false, false, true, false, false]);
    assert_eq!(
        judge.switch(true, LIT.to_vec(), at(0)),
        Verdict::Shut("the hinge was nearly shut")
    );
}

#[test]
fn a_switch_shut_with_the_lid_wide_open_is_doubted() {
    let mut judge = Judge::new(false);
    judge.hinge(101.0);
    // The hub's zero for a shut switch may come just before it.
    judge.hinge(0.0);
    assert_eq!(judge.switch(true, LIT.to_vec(), at(0)), Verdict::Doubted);
    assert_eq!(judge.light(&LIT, at(1)), Verdict::Unchanged);
    assert_eq!(judge.switch(false, Vec::new(), at(124)), Verdict::Unchanged);
    assert!(!judge.doubting(), "the doubt outlasted the switch opening");
}

#[test]
fn a_doubted_close_is_believed_once_a_light_sensor_goes_dark() {
    let mut judge = Judge::new(false);
    judge.hinge(101.0);
    judge.switch(true, LIT.to_vec(), at(0));
    assert_eq!(
        judge.light(&[Some(0.0), Some(8000.0)], at(2)),
        Verdict::Shut("the light went dark")
    );
    assert_eq!(judge.switch(false, Vec::new(), at(9)), Verdict::Open);
}

#[test]
fn a_light_sensor_unlit_at_the_switch_tells_nothing() {
    let mut judge = Judge::new(false);
    judge.hinge(101.0);
    judge.switch(true, vec![Some(0.0), Some(8000.0)], at(0));
    assert_eq!(judge.light(&[Some(0.0), None], at(2)), Verdict::Unchanged);
}

#[test]
fn a_close_in_the_dark_is_believed() {
    let mut judge = Judge::new(false);
    judge.hinge(101.0);
    assert_eq!(
        judge.switch(true, vec![Some(0.0), None], at(0)),
        Verdict::Shut("no light sensor saw light to tell by")
    );
}

#[test]
fn a_doubt_ends_once_it_has_lasted_too_long() {
    let mut judge = Judge::new(false);
    judge.hinge(101.0);
    judge.switch(true, LIT.to_vec(), at(0));
    assert_eq!(judge.light(&LIT, at(599)), Verdict::Unchanged);
    assert_eq!(
        judge.light(&LIT, at(600)),
        Verdict::Shut("it stayed shut too long to doubt")
    );
}

#[test]
fn an_angle_from_before_the_last_opening_says_nothing() {
    let mut judge = Judge::new(false);
    judge.hinge(8.0);
    judge.switch(true, LIT.to_vec(), at(0));
    judge.switch(false, Vec::new(), at(5));
    assert_eq!(judge.switch(true, LIT.to_vec(), at(9)), Verdict::Doubted);
}

#[test]
fn a_lid_shut_as_it_starts_is_believed() {
    let mut judge = Judge::new(true);
    judge.hinge(0.0);
    assert_eq!(judge.switch(false, Vec::new(), at(0)), Verdict::Open);
}

#[test]
fn the_hinge_reads_in_degrees_from_its_buffer() {
    let field = Field::read("le:s16/32>>0").unwrap();
    let layout = Layout::of(vec![(0, field, true)], 0.017_453_293, 0.0).unwrap();
    let mut bytes = Vec::new();
    for raw in [108i32, -1, 8] {
        bytes.extend_from_slice(&raw.to_le_bytes());
    }
    bytes.extend_from_slice(&[1, 2]);
    let (angles, used) = layout.angles(&bytes);
    assert_eq!(angles.len(), 3);
    for (degrees, expected) in angles.iter().zip([108.0, -1.0, 8.0]) {
        assert!(
            (degrees - expected).abs() < 0.01,
            "{degrees} isn't {expected}"
        );
    }
    assert_eq!(used, 12);
}

#[test]
fn the_hinge_sits_after_the_channels_before_it() {
    let short = Field::read("le:s16/32>>0").unwrap();
    let stamp = Field::read("le:s64/64>>0").unwrap();
    let layout = Layout::of(
        vec![(3, stamp, false), (1, short, false), (0, short, true)],
        1.0,
        0.0,
    )
    .unwrap();
    assert_eq!((layout.at, layout.size), (0, 16));
    let layout = Layout::of(vec![(0, short, false), (1, short, true)], 1.0, 0.0).unwrap();
    assert_eq!((layout.at, layout.size), (4, 8));
}
