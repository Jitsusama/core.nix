//! A meeting told as the daemon tells it reads back as the same meeting,
//! so `google-rows`, which makes the VM tests' stand-in's rows from what
//! it's told (src/bin/google-rows.rs), makes the rows the daemon would.

use serde_json::json;

use super::tests::{MONDAY, planned};
use crate::calendar::{Dial, Every, Meeting, Repeats, Weekday};

#[test]
fn a_meeting_as_the_daemon_tells_it_reads_back_the_same() {
    let mut meeting = planned();
    meeting.series = Some("plan".into());
    meeting.kind = Some(crate::link::Kind::Meet);
    meeting.dial = Some(Dial {
        number: "+1 555 0100".into(),
        pin: Some("1234".into()),
    });
    meeting.repeats = Some(Repeats {
        every: Every::Week,
        interval: 2,
        days: vec![Weekday { day: 1, nth: None }],
        month_days: vec![-1],
        times: Some(4),
        until: None,
    });
    meeting.done = Some(crate::done::Done {
        how: crate::done::Went::Ended,
        at: MONDAY,
    });
    let mut told = serde_json::to_value(&meeting).expect("a meeting is told as JSON");
    told["guestsCounted"] = json!(super::guests_counted(&meeting));
    let read: Meeting = serde_json::from_value(told).expect("a told meeting reads back");
    assert_eq!(
        read, meeting,
        "google-rows would make rows of another meeting"
    );
}
