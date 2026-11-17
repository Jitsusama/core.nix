use super::rate;

#[test]
fn a_rate_keeps_three_figures_in_the_unit_that_reads_it_so() {
    let cases = [
        (369.86, "370 Mb/s"),
        (11.9, "11.9 Mb/s"),
        (9.9, "9.90 Mb/s"),
        (1200.0, "1.20 Gb/s"),
        (0.145, "145 kb/s"),
        (0.000_098_2, "98.2 b/s"),
        (7.42, "7.42 Mb/s"),
    ];
    for (mbps, says) in cases {
        assert_eq!(rate(mbps), says, "{mbps} Mb/s");
    }
}

#[test]
fn a_rate_that_rounds_up_to_a_thousand_takes_the_next_unit() {
    assert_eq!(rate(999.96), "1.00 Gb/s");
    assert_eq!(rate(0.999_96), "1.00 Mb/s");
}

#[test]
fn no_rate_reads_zero() {
    for none in [0.0, -1.0, f64::NAN] {
        assert_eq!(rate(none), "0 b/s", "{none}");
    }
}
