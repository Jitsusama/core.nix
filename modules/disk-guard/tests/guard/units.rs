use disk_guard::units::{Size, Span};
use std::time::Duration;

const HOUR: u64 = 3600;
const DAY: u64 = 24 * HOUR;

#[test]
fn a_span_reads_each_unit() {
    assert_eq!(Span::parse("20s"), Ok(Span(Duration::from_secs(20))));
    assert_eq!(Span::parse("5m"), Ok(Span(Duration::from_secs(300))));
    assert_eq!(Span::parse("6h"), Ok(Span(Duration::from_secs(6 * HOUR))));
    assert_eq!(Span::parse("30d"), Ok(Span(Duration::from_secs(30 * DAY))));
}

#[test]
fn a_span_adds_its_parts() {
    assert_eq!(
        Span::parse("1d1h"),
        Ok(Span(Duration::from_secs(DAY + HOUR)))
    );
}

#[test]
fn a_span_is_refused_without_a_unit_or_with_a_stray() {
    for text in ["", "5", "h", "5x", "1d 1h", "-1h", "1.5h"] {
        assert!(Span::parse(text).is_err(), "{text:?} was read");
    }
}

#[test]
fn a_size_reads_bytes_shares_and_binary_units() {
    assert_eq!(Size::parse("4096"), Ok(Size::Bytes(4096)));
    assert_eq!(Size::parse("15%"), Ok(Size::Percent(15.0)));
    assert_eq!(Size::parse("2.5%"), Ok(Size::Percent(2.5)));
    assert_eq!(Size::parse("256MiB"), Ok(Size::Bytes(256 << 20)));
    assert_eq!(Size::parse("24GiB"), Ok(Size::Bytes(24 << 30)));
    assert_eq!(Size::parse("1TiB"), Ok(Size::Bytes(1 << 40)));
}

#[test]
fn a_size_is_refused_past_the_whole_or_in_a_vague_unit() {
    for text in ["", "GiB", "24G", "24GB", "101%", "-1%", "1.5GiB", "x"] {
        assert!(Size::parse(text).is_err(), "{text:?} was read");
    }
}

#[test]
fn a_size_resolves_against_its_filesystem() {
    assert_eq!(Size::Bytes(10).of(1000), 10);
    assert_eq!(Size::Percent(15.0).of(1000), 150);
}
