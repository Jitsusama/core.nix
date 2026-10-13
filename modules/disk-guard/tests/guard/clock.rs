use disk_guard::clock::{Awake, Tick};

fn tick(wall: f64, monotonic: f64, suspended: f64, boot: &str) -> Tick {
    Tick {
        wall,
        monotonic,
        suspended,
        boot: boot.to_string(),
    }
}

#[test]
fn with_no_history_wall_time_counts() {
    assert_eq!(Awake::default().since(0.0, 100.0), 100.0);
}

#[test]
fn the_first_tick_counts_back_to_the_boot() {
    let mut awake = Awake::default();
    awake.observe(&tick(1000.0, 10.0, 0.0, "a"));
    assert_eq!(awake.since(995.0, 1000.0), 5.0);
    // Before the boot, with nothing recorded, wall time stands in.
    assert_eq!(awake.since(900.0, 1000.0), 100.0);
}

#[test]
fn ticks_while_awake_extend_the_span() {
    let mut awake = Awake::default();
    awake.observe(&tick(1000.0, 10.0, 0.0, "a"));
    awake.observe(&tick(1005.0, 15.0, 0.0, "a"));
    assert_eq!(awake.since(995.0, 1005.0), 10.0);
}

#[test]
fn a_suspend_adds_nothing() {
    let mut awake = Awake::default();
    awake.observe(&tick(1000.0, 10.0, 0.0, "a"));
    awake.observe(&tick(4600.0, 15.0, 3595.0, "a"));
    assert_eq!(awake.since(995.0, 4600.0), 10.0);
}

#[test]
fn a_night_off_adds_nothing() {
    let mut awake = Awake::default();
    awake.observe(&tick(1000.0, 10.0, 0.0, "a"));
    awake.observe(&tick(90000.0, 60.0, 0.0, "b"));
    assert_eq!(awake.since(995.0, 90000.0), 65.0);
}

#[test]
fn a_restart_within_a_boot_bridges_the_gap() {
    let mut awake = Awake::default();
    awake.observe(&tick(1000.0, 10.0, 0.0, "a"));
    let saved = serde_json::to_string(&awake).unwrap();
    let mut restarted: Awake = serde_json::from_str(&saved).unwrap();
    restarted.observe(&tick(1030.0, 40.0, 0.0, "a"));
    assert_eq!(restarted.since(995.0, 1030.0), 35.0);
}

#[test]
fn a_wall_clock_set_back_starts_afresh() {
    let mut awake = Awake::default();
    awake.observe(&tick(1000.0, 10.0, 0.0, "a"));
    awake.observe(&tick(500.0, 15.0, 0.0, "a"));
    assert_eq!(awake.since(495.0, 500.0), 5.0);
}

#[test]
fn spans_older_than_the_horizon_are_dropped() {
    let mut awake = Awake::default();
    awake.observe(&tick(1000.0, 10.0, 0.0, "a"));
    awake.observe(&tick(200.0 * 86400.0, 10.0, 0.0, "b"));
    awake.forget_before(100.0 * 86400.0);
    assert_eq!(awake.spans().len(), 1);
}
