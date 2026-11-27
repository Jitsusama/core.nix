use super::*;

#[test]
fn an_idle_host_allows_the_idle_bound_and_the_slack() {
    assert_eq!(
        allowance(Duration::from_secs(2), 0.3),
        Duration::from_secs(2) + SLACK
    );
}

#[test]
fn a_busy_host_stretches_the_bound_by_its_pace() {
    assert_eq!(
        allowance(Duration::from_secs(2), 2.5),
        Duration::from_secs(5) + SLACK
    );
}

#[test]
fn an_oversubscribed_host_stretches_it_no_further_than_the_most_pace() {
    assert_eq!(
        allowance(Duration::from_secs(2), 40.0),
        Duration::from_secs(8) + SLACK
    );
}

#[test]
fn a_pace_never_falls_below_one_or_past_the_most() {
    // Each pace is one of the figures given, unchanged, so comparing their
    // bits is exact.
    assert_eq!(pace(0.0).to_bits(), 1.0_f64.to_bits(), "an idle host");
    assert_eq!(pace(1.5).to_bits(), 1.5_f64.to_bits(), "a busy one");
    assert_eq!(
        pace(MOST_PACE * 3.0).to_bits(),
        MOST_PACE.to_bits(),
        "an oversubscribed one"
    );
    assert_eq!(
        pace(f64::NAN).to_bits(),
        1.0_f64.to_bits(),
        "one that can't be read"
    );
}

#[test]
fn the_last_minutes_ready_tasks_come_first_in_the_load_average() {
    assert_eq!(
        ready_each_minute("23.74 31.78 40.35 12/6940 458458\n").map(f64::to_bits),
        Some(23.74_f64.to_bits())
    );
    assert_eq!(ready_each_minute(""), None, "nothing said");
    assert_eq!(ready_each_minute("busy 1.0"), None, "not a number");
}

#[test]
fn a_step_shorter_than_its_bound_is_within_it() {
    let timed = Timed::start();

    assert!(
        timed.within(Duration::from_secs(60)),
        "{}",
        timed.said(Duration::from_secs(60))
    );
}

#[test]
fn a_step_longer_than_any_pace_allows_is_not_within_it() {
    let timed = Timed {
        began: Instant::now()
            .checked_sub(Duration::from_secs(60))
            .expect("the host has run for a minute"),
        busy: 0.0,
    };

    assert!(
        !timed.within(Duration::from_millis(10)),
        "{}",
        timed.said(Duration::from_millis(10))
    );
}

#[test]
fn a_step_begun_on_a_busy_host_keeps_that_pace_though_the_host_quietens() {
    let timed = Timed {
        began: Instant::now(),
        busy: MOST_PACE,
    };

    assert!(
        timed.allowed(Duration::from_secs(1)) >= Duration::from_secs(4) + SLACK,
        "{}",
        timed.said(Duration::from_secs(1))
    );
}

#[test]
fn what_a_failure_says_names_both_bounds() {
    let timed = Timed {
        began: Instant::now(),
        busy: 0.0,
    };

    let said = timed.said(Duration::from_secs(2));

    assert!(said.contains("where 2s is allowed at idle"), "{said}");
    assert!(said.contains("on this host"), "{said}");
}
