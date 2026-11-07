//! When the feed reads next: at spend-watch's pace, at once when asked, and
//! further apart while the endpoint fails or asks it to wait.

use claude_plan_feed::{Outcome, Pace, Schedule};
use jiff::{SignedDuration, Timestamp};

const START: &str = "2026-11-06T15:00:00Z";

#[test]
fn the_first_read_is_at_once() {
    let schedule = Schedule::new(at(START));

    assert_eq!(schedule.due(), at(START));
}

#[test]
fn an_idle_account_is_read_every_ten_minutes() {
    let mut schedule = Schedule::new(at(START));

    schedule.read(&Outcome::Answered, at(START));

    assert_eq!(schedule.due(), later(600));
}

#[test]
fn an_active_account_is_read_every_minute() {
    let mut schedule = Schedule::new(at(START));
    schedule.pace(Pace::Active);

    schedule.read(&Outcome::Answered, at(START));

    assert_eq!(schedule.due(), later(60));
}

#[test]
fn becoming_active_brings_the_next_read_forward() {
    let mut schedule = Schedule::new(at(START));
    schedule.read(&Outcome::Answered, at(START));

    schedule.pace(Pace::Active);

    assert_eq!(schedule.due(), later(60));
}

#[test]
fn becoming_idle_puts_the_next_read_back() {
    let mut schedule = Schedule::new(at(START));
    schedule.pace(Pace::Active);
    schedule.read(&Outcome::Answered, at(START));

    schedule.pace(Pace::Idle);

    assert_eq!(schedule.due(), later(600));
}

#[test]
fn a_refresh_reads_at_once() {
    let mut schedule = Schedule::new(at(START));
    schedule.read(&Outcome::Answered, at(START));

    schedule.refresh(later(30));

    assert_eq!(schedule.due(), later(30));
}

#[test]
fn a_read_with_nothing_to_ask_with_comes_back_at_the_pace() {
    let mut schedule = Schedule::new(at(START));
    schedule.pace(Pace::Active);

    schedule.read(&Outcome::Unasked, at(START));

    assert_eq!(schedule.due(), later(60));
}

#[test]
fn failures_in_a_row_wait_twice_as_long_each_time_up_to_an_hour() {
    let mut schedule = Schedule::new(at(START));
    schedule.pace(Pace::Active);
    let mut waits = Vec::new();

    for _ in 0..8 {
        let now = schedule.due();
        schedule.read(&Outcome::Failed, now);
        waits.push(schedule.due().duration_since(now).as_secs());
    }

    assert_eq!(waits, [120, 240, 480, 960, 1920, 3600, 3600, 3600]);
}

#[test]
fn a_failure_s_wait_holds_when_the_pace_changes() {
    let mut schedule = Schedule::new(at(START));
    schedule.read(&Outcome::Failed, at(START));

    schedule.pace(Pace::Active);

    assert_eq!(schedule.due(), later(1200));
}

#[test]
fn a_good_read_ends_the_backing_off() {
    let mut schedule = Schedule::new(at(START));
    schedule.pace(Pace::Active);
    schedule.read(&Outcome::Failed, at(START));
    schedule.read(&Outcome::Failed, later(120));

    schedule.read(&Outcome::Answered, later(360));

    assert_eq!(schedule.due(), later(420));
}

#[test]
fn being_asked_to_wait_reads_no_sooner_than_asked() {
    let mut schedule = Schedule::new(at(START));
    schedule.pace(Pace::Active);

    schedule.read(&Outcome::Paused(later(900)), at(START));

    assert_eq!(schedule.due(), later(900));
}

#[test]
fn a_short_wait_still_backs_off() {
    let mut schedule = Schedule::new(at(START));
    schedule.pace(Pace::Active);

    schedule.read(&Outcome::Paused(later(5)), at(START));

    assert_eq!(schedule.due(), later(120));
}

#[test]
fn the_wait_until_a_read_is_never_less_than_none() {
    let schedule = Schedule::new(at(START));

    assert_eq!(schedule.wait(later(30)), std::time::Duration::ZERO);
}

fn at(instant: &str) -> Timestamp {
    instant
        .parse()
        .unwrap_or_else(|error| panic!("{instant}: {error}"))
}

fn later(seconds: i64) -> Timestamp {
    at(START) + SignedDuration::from_secs(seconds)
}
