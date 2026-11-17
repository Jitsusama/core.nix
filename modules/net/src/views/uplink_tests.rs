use super::rate;

#[test]
fn the_uplink_says_its_kilobits_as_a_rate() {
    // The report holds kbit/s, so a slip in the scale would read a
    // thousand times off.
    assert_eq!(rate(1200.0), "1.20 Mb/s");
    assert_eq!(rate(640.0), "640 kb/s");
    assert_eq!(rate(10_800.0), "10.8 Mb/s");
}
