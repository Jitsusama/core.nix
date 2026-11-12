use super::*;

fn at(seconds: u64) -> Instant {
    // Instants can't be made from a number, so they're laid from one start.
    thread_local!(static START: Instant = Instant::now());
    START.with(|start| *start + Duration::from_secs(seconds))
}

const LIT: [Option<f64>; 2] = [Some(8000.0), Some(8000.0)];

/// A lid long open at 101°, as the hinge last read it.
fn open_wide() -> Judge {
    let mut judge = Judge::new(false);
    judge.hinge(101.0, at(0));
    judge
}

#[test]
fn a_lid_coming_down_shuts_at_its_switch() {
    let mut judge = Judge::new(false);
    let crossed: Vec<bool> = [99.0, 62.0, 20.0, 8.0, 0.0]
        .into_iter()
        .map(|degrees| judge.hinge(degrees, at(0)))
        .collect();
    assert_eq!(crossed, [false, false, true, false, false]);
    assert_eq!(
        judge.switch(true, LIT.to_vec(), at(0)),
        Verdict::Shut("the hinge was nearly shut")
    );
}

#[test]
fn a_lid_held_nearly_shut_a_while_still_shuts_at_its_switch() {
    let mut judge = Judge::new(false);
    judge.hinge(70.0, at(0));
    judge.hinge(40.0, at(1));
    judge.hinge(41.0, at(2));
    assert_eq!(
        judge.switch(true, LIT.to_vec(), at(23)),
        Verdict::Shut("the hinge was nearly shut")
    );
}

#[test]
fn a_nearly_shut_reading_that_comes_alone_is_doubted() {
    // 12 Nov, 00:07:22: the hinge had read the lid open for hours, then
    // read 28° in the same second as the switch said shut, with Joel
    // typing on the laptop.
    let mut judge = open_wide();
    assert!(
        !judge.hinge(28.0, at(11_740)),
        "a reading alone was taken as the lid coming down"
    );
    assert_eq!(
        judge.switch(true, LIT.to_vec(), at(11_740)),
        Verdict::Doubted
    );
}

#[test]
fn a_switch_shut_with_the_lid_wide_open_is_doubted() {
    let mut judge = open_wide();
    // The hub's zero for a shut switch may come just before it.
    judge.hinge(0.0, at(0));
    assert_eq!(judge.switch(true, LIT.to_vec(), at(0)), Verdict::Doubted);
    assert_eq!(judge.light(&LIT, at(1)), Verdict::Unchanged);
    assert_eq!(judge.switch(false, Vec::new(), at(124)), Verdict::Unchanged);
    assert!(!judge.doubting(), "the doubt outlasted the switch opening");
}

#[test]
fn a_doubted_close_is_believed_once_a_light_sensor_goes_dark_with_no_hands_on_it() {
    // The sensor hub stops reading light once the switch says shut, so a
    // dark reading counts only once nobody has used the laptop a while.
    let mut judge = open_wide();
    judge.switch(true, LIT.to_vec(), at(0));
    assert_eq!(
        judge.light(&[Some(0.0), Some(8000.0)], at(2)),
        Verdict::Unchanged
    );
    assert_eq!(
        judge.light(&[Some(0.0), Some(8000.0)], at(30)),
        Verdict::Shut("the light went dark")
    );
    assert_eq!(judge.switch(false, Vec::new(), at(9)), Verdict::Open);
}

#[test]
fn a_light_sensor_unlit_at_the_switch_tells_nothing() {
    let mut judge = open_wide();
    judge.switch(true, vec![Some(0.0), Some(8000.0)], at(0));
    assert_eq!(judge.light(&[Some(0.0), None], at(30)), Verdict::Unchanged);
}

#[test]
fn a_close_in_the_dark_is_believed() {
    let mut judge = open_wide();
    assert_eq!(
        judge.switch(true, vec![Some(0.0), None], at(0)),
        Verdict::Doubted
    );
    assert_eq!(
        judge.light(&[Some(0.0), None], at(30)),
        Verdict::Shut("no light sensor saw light to tell by")
    );
}

#[test]
fn a_doubt_ends_once_it_has_lasted_too_long() {
    let mut judge = open_wide();
    judge.switch(true, LIT.to_vec(), at(0));
    assert_eq!(judge.light(&LIT, at(599)), Verdict::Unchanged);
    assert_eq!(
        judge.light(&LIT, at(600)),
        Verdict::Shut("it stayed shut too long to doubt")
    );
}

#[test]
fn a_doubted_lid_typed_on_stays_open() {
    // The sensor hub reads no light once the switch says shut, so on 12 Nov
    // only the keys showed the lid was open, and the doubt's ten minutes
    // would have shut it under Joel's hands.
    let mut judge = open_wide();
    judge.switch(true, LIT.to_vec(), at(0));
    judge.keys(64_917, at(1));
    judge.keys(65_210, at(590));
    let unlit = [Some(0.0), Some(0.0)];
    assert_eq!(judge.light(&unlit, at(600)), Verdict::Unchanged);
    assert_eq!(judge.light(&LIT, at(900)), Verdict::Unchanged);
    judge.keys(65_210, at(1000));
    assert_eq!(
        judge.light(&LIT, at(1190)),
        Verdict::Shut("it stayed shut too long to doubt")
    );
}

#[test]
fn a_shut_lid_typed_on_opens_again() {
    let mut judge = Judge::new(false);
    judge.hinge(70.0, at(0));
    judge.hinge(20.0, at(1));
    judge.keys(100, at(1));
    assert_eq!(
        judge.switch(true, LIT.to_vec(), at(1)),
        Verdict::Shut("the hinge was nearly shut")
    );
    // A palm lifting off the touchpad as the lid comes down.
    assert_eq!(judge.keys(104, at(2)), Verdict::Unchanged);
    assert_eq!(judge.keys(104, at(9)), Verdict::Unchanged);
    assert_eq!(judge.keys(108, at(12)), Verdict::Used);
    assert!(judge.doubting(), "a lid opened by its keys wasn't doubted");
    assert_eq!(judge.light(&[Some(0.0), None], at(60)), Verdict::Unchanged);
    assert_eq!(
        judge.light(&[Some(0.0), None], at(612)),
        Verdict::Shut("it stayed shut too long to doubt")
    );
}

#[test]
fn the_laptops_presses_are_read_from_its_interrupts() {
    let interrupts = "           CPU0       CPU1       CPU2\n  \
        0:         12          0          0  IR-IO-APIC    2-edge      timer\n  \
        1:      60000       4917          0  IR-IO-APIC    1-edge      i8042\n \
        12:        125          0          0  IR-IO-APIC   12-edge      i8042\n";
    let hands = ["i8042".to_owned(), "VEN_2C2F:00".to_owned()];
    assert_eq!(presses(interrupts, &hands), Some(65_042));
    assert_eq!(presses("           CPU0\n  0:  12  timer\n", &hands), None);
}

#[test]
fn an_angle_from_before_the_last_opening_says_nothing() {
    let mut judge = Judge::new(false);
    judge.hinge(70.0, at(0));
    judge.hinge(8.0, at(0));
    judge.switch(true, LIT.to_vec(), at(0));
    judge.switch(false, Vec::new(), at(5));
    assert_eq!(judge.switch(true, LIT.to_vec(), at(9)), Verdict::Doubted);
}

#[test]
fn a_lid_shut_as_it_starts_is_believed() {
    let mut judge = Judge::new(true);
    judge.hinge(0.0, at(0));
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
