use disk_guard::band::{Band, Thresholds};
use disk_guard::clock::{Awake, Tick};
use disk_guard::decide::{Decision, Fill, Situation, decide, spare};
use disk_guard::moments::{Power, Pressure, Stall};

const GIB: u64 = 1 << 30;
const HOUR: f64 = 3600.0;

fn thresholds() -> Thresholds {
    Thresholds {
        pressure: 150 * GIB,
        urgent: 80 * GIB,
        critical: 30 * GIB,
        target: 200 * GIB,
        margin: 10 * GIB,
    }
}

/// A fill observed every 5 s for `seconds`, consuming `rate` bytes a second.
fn filling(free: u64, rate: f64, seconds: u32) -> (Fill, u64) {
    let mut fill = Fill::default();
    let mut now = 0.0;
    let mut left = free as f64;
    fill.observe(now, left as u64);
    for _ in 0..seconds / 5 {
        now += 5.0;
        left -= rate * 5.0;
        fill.observe(now, left as u64);
    }
    (fill, left as u64)
}

fn situation(band: Band, free: u64, fill: &Fill) -> Situation<'_> {
    Situation {
        band,
        free,
        thresholds: thresholds(),
        fill,
        pass_seconds: 60.0,
        upkept: Some(1_000_000.0),
        now: 1_000_000.0 + HOUR,
        spare: true,
    }
}

#[test]
fn a_steady_fill_is_measured_as_its_rate() {
    let (fill, _) = filling(500 * GIB, 100e6, 600);
    let rate = fill.rate().unwrap();
    assert!((rate - 100e6).abs() < 5e6, "{rate}");
}

#[test]
fn freeing_space_forecasts_nothing() {
    let (fill, free) = filling(200 * GIB, -100e6, 600);
    assert_eq!(fill.until(free, 150 * GIB), None);
}

#[test]
fn a_fill_is_forecast_to_reach_a_threshold() {
    let (fill, free) = filling(200 * GIB, 1e9, 30);
    let until = fill.until(free, 150 * GIB).unwrap();
    let expected = (free - 150 * GIB) as f64 / 1e9;
    assert!(
        (until - expected).abs() < expected * 0.05,
        "{until} against {expected}"
    );
    assert_eq!(fill.until(140 * GIB, 150 * GIB), Some(0.0));
}

#[test]
fn a_band_is_answered_at_once_whatever_the_moment() {
    let fill = Fill::default();
    let mut now = situation(Band::Urgent, 70 * GIB, &fill);
    now.spare = false;
    assert_eq!(decide(&now), Decision::Reach(Band::Urgent));
}

#[test]
fn a_band_forecast_sooner_than_a_pass_takes_is_answered_ahead_of_it() {
    // About 112 s from pressure at 1 GB a second, against a 60 s pass and
    // its margin.
    let (fill, free) = filling(310 * GIB, 1e9, 60);
    assert_eq!(
        decide(&situation(Band::Healthy, free, &fill)),
        Decision::Reach(Band::Pressure)
    );
    // Hours away, nothing is due.
    let (slow, free) = filling(310 * GIB, 1e6, 600);
    assert_eq!(
        decide(&situation(Band::Healthy, free, &slow)),
        Decision::Rest
    );
    // Already in pressure and heading for urgent.
    let (fill, free) = filling(170 * GIB, 1e9, 60);
    let mut now = situation(Band::Pressure, free, &fill);
    now.free = 90 * GIB;
    assert_eq!(decide(&now), Decision::Reach(Band::Urgent));
}

#[test]
fn upkeep_comes_round_every_six_hours_when_the_machine_can_spare_it() {
    let fill = Fill::default();
    let mut now = situation(Band::Healthy, 500 * GIB, &fill);
    assert_eq!(decide(&now), Decision::Rest);
    now.now = now.upkept.unwrap() + 6.0 * HOUR;
    assert_eq!(decide(&now), Decision::Upkeep);
    now.spare = false;
    assert_eq!(decide(&now), Decision::Rest);
    now.spare = true;
    now.upkept = None;
    assert_eq!(decide(&now), Decision::Upkeep);
}

fn awake_for(seconds: f64) -> Awake {
    let mut awake = Awake::default();
    awake.observe(&Tick {
        wall: 1_000_000.0,
        monotonic: seconds,
        suspended: 0.0,
        boot: "b".into(),
    });
    awake
}

fn calm() -> Pressure {
    Pressure::default()
}

const MAINS: Power = Power {
    mains: true,
    battery: Some(20),
};

#[test]
fn routine_work_waits_for_power() {
    let awake = awake_for(HOUR);
    assert!(spare(MAINS, false, &awake, 1_000_000.0, calm()));
    let charged = Power {
        mains: false,
        battery: Some(51),
    };
    assert!(spare(charged, false, &awake, 1_000_000.0, calm()));
    let low = Power {
        mains: false,
        battery: Some(50),
    };
    assert!(!spare(low, false, &awake, 1_000_000.0, calm()));
    assert!(
        !spare(MAINS, true, &awake, 1_000_000.0, calm()),
        "power saver"
    );
    let desktop = Power {
        mains: true,
        battery: None,
    };
    assert!(spare(desktop, false, &awake, 1_000_000.0, calm()));
}

#[test]
fn routine_work_waits_ten_minutes_after_boot_or_resume() {
    assert!(!spare(MAINS, false, &awake_for(599.0), 1_000_000.0, calm()));
    assert!(spare(MAINS, false, &awake_for(600.0), 1_000_000.0, calm()));
    let mut resumed = awake_for(HOUR);
    resumed.observe(&Tick {
        wall: 1_000_000.0 + 8.0 * HOUR,
        monotonic: HOUR + 60.0,
        suspended: 8.0 * HOUR - 60.0,
        boot: "b".into(),
    });
    assert!(!spare(
        MAINS,
        false,
        &resumed,
        1_000_000.0 + 8.0 * HOUR,
        calm()
    ));
}

#[test]
fn routine_work_waits_while_tasks_stall_on_the_disk_or_memory() {
    let awake = awake_for(HOUR);
    let disk = Pressure {
        io: Stall {
            some: 15.0,
            full: 2.0,
        },
        ..calm()
    };
    assert!(!spare(MAINS, false, &awake, 1_000_000.0, disk));
    let memory = Pressure {
        memory: Stall {
            some: 15.0,
            full: 0.0,
        },
        ..calm()
    };
    assert!(!spare(MAINS, false, &awake, 1_000_000.0, memory));
    // Busy processors aren't a reason: routine work runs at idle priority.
    let cpu = Pressure {
        cpu: Stall {
            some: 90.0,
            full: 0.0,
        },
        ..calm()
    };
    assert!(spare(MAINS, false, &awake, 1_000_000.0, cpu));
}
