//! The tests of chrony's command protocol: a request padded to its reply,
//! each figure of a tracking reply read, a refusal, and a source.

use super::*;

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() <= b.abs() * 1e-6 + 1e-12
}

#[test]
fn a_request_is_padded_to_its_replys_length() {
    let asked = request(Command::Tracking, 7);
    assert_eq!(asked.len(), 28 + 76);
    assert_eq!(&asked[..8], &[6, 1, 0, 0, 0, 33, 0, 0]);
    assert_eq!(&asked[8..12], &7_u32.to_be_bytes());
    let named = request(
        Command::SourceName(Address::Ip(
            "192.0.2.7"
                .parse()
                .unwrap_or_else(|error| panic!("{error}")),
        )),
        1,
    );
    assert_eq!(named.len(), 28 + 256);
    assert_eq!(&named[20..24], &[192, 0, 2, 7]);
    assert_eq!(&named[36..38], &1_u16.to_be_bytes());
}

#[test]
fn a_float_reads_as_chronyd_wrote_it() {
    for figure in [0.000_388, -0.012_5, 6.267, -19.5, 1.0, 0.0] {
        let word = u32::from_be_bytes(written::float(figure));
        assert!(
            near(float(word), figure),
            "{figure} read as {}",
            float(word)
        );
    }
}

#[test]
fn a_reply_to_another_request_isnt_taken() {
    let answer = written::reply(Command::Sources, 3, 0, &4_u32.to_be_bytes());
    assert_eq!(reply(Command::Sources, 4, &answer), Err(Refused::NotOurs));
    assert_eq!(reply(Command::Tracking, 3, &answer), Err(Refused::NotOurs));
    assert_eq!(reply(Command::Sources, 3, &answer).map(sources), Ok(4),);
}

/// A tracking reply as chronyd 4.9 wrote it, 104 bytes, to a request
/// numbered 7, keeping the clock by its own local reference at stratum 3.
const TRACKED: &str = "060200000021000500000000000000000000000700000000000000007f7f0101\
    000000000000000000000000000000000000000000030000000000006aebe015\
    3270844a00000000000000000000000000000000000000000000000000000000\
    0000000000000000";

#[test]
fn a_tracking_reply_as_chronyd_writes_it_is_taken() {
    let bytes: Vec<u8> = (0..TRACKED.len())
        .step_by(2)
        .map(|at| u8::from_str_radix(&TRACKED[at..at + 2], 16).unwrap_or_else(|e| panic!("{e}")))
        .collect();
    assert_eq!(bytes.len(), 104);
    let read = reply(Command::Tracking, 7, &bytes).map(tracking);
    assert_eq!(read.as_ref().map(|it| it.stratum), Ok(3));
    assert_eq!(
        read.map(|it| reference(it.reference)),
        Ok("7F7F0101".to_owned())
    );
}

#[test]
fn a_refusal_says_its_status() {
    let answer = written::reply(Command::Rtc, 9, 13, &[]);
    assert_eq!(reply(Command::Rtc, 9, &answer), Err(Refused::Status(13)));
}

#[test]
fn tracking_reads_each_figure() {
    let mut data = Vec::new();
    data.extend_from_slice(&0xC000_0207_u32.to_be_bytes());
    data.extend_from_slice(&written::address(Address::Ip(
        "192.0.2.7"
            .parse()
            .unwrap_or_else(|error| panic!("{error}")),
    )));
    data.extend_from_slice(&2_u16.to_be_bytes());
    data.extend_from_slice(&0_u16.to_be_bytes());
    data.extend_from_slice(&written::time(1_790_000_000));
    for figure in [
        0.000_012, -0.000_388, 0.000_2, 6.267, 0.001, 0.03, 0.015, 0.000_457, 64.2,
    ] {
        data.extend_from_slice(&written::float(figure));
    }
    let read = tracking(&data);
    assert_eq!(read.stratum, 2);
    assert_eq!(read.leap, Leap::Normal);
    let figures = [
        ("reference time", read.reference_time, 1_790_000_000.0),
        ("correction", read.correction, 0.000_012),
        ("last offset", read.last_offset, -0.000_388),
        ("frequency", read.frequency, 6.267),
        ("root dispersion", read.root_dispersion, 0.000_457),
        ("update interval", read.update_interval, 64.2),
    ];
    for (what, got, wanted) in figures {
        assert!(near(got, wanted), "the {what} read as {got}, not {wanted}");
    }
    assert_eq!(reference(read.reference), "C0000207");
}

#[test]
fn a_stratum_one_reference_reads_as_its_letters() {
    assert_eq!(reference(u32::from_be_bytes(*b"GPS\0")), "GPS");
    assert_eq!(reference(u32::from_be_bytes(*b"PPS1")), "PPS1");
}

#[test]
fn a_source_reads_its_state_and_offsets() {
    let mut data = Vec::new();
    data.extend_from_slice(&written::address(Address::Id(3)));
    data.extend_from_slice(&6_i16.to_be_bytes());
    data.extend_from_slice(&0_u16.to_be_bytes());
    data.extend_from_slice(&1_u16.to_be_bytes());
    data.extend_from_slice(&0_u16.to_be_bytes());
    data.extend_from_slice(&0_u16.to_be_bytes());
    data.extend_from_slice(&0_u16.to_be_bytes());
    data.extend_from_slice(&u32::MAX.to_be_bytes());
    let read = source(&data);
    assert_eq!(read.address, Address::Id(3));
    assert_eq!(read.state, State::Unusable);
    assert_eq!(read.since_sample, None);
    assert_eq!(read.poll, 6);
}

#[test]
fn a_name_ends_at_its_nul() {
    let mut data = b"0.pool.ntp.org".to_vec();
    data.resize(256, 0);
    assert_eq!(source_name(&data), "0.pool.ntp.org");
}
