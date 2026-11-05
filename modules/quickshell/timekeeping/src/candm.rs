//! chrony's command protocol, version 6, as `candm.h` in chrony's source
//! lays it out, for the monitoring commands chronyd answers anyone on the
//! machine over its command port: each request and the reply it's answered
//! with, every number in network order.
//!
//! A request is padded to its reply's length, as chronyd requires of
//! version 6, so an answer is never larger than what asked for it.
//!
//! It's written here rather than taken from aws's chrony-candm crate,
//! which knows only the older reply to ntpdata (16) and not the one chrony
//! 4.9 gives (26, `RPY_NTP_DATA2`), so it couldn't read any server's last
//! exchange.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

/// The port chronyd answers commands on, on the loopback addresses alone.
pub const PORT: u16 = 323;

const VERSION: u8 = 6;
const REQUEST: u8 = 1;
const REPLY: u8 = 2;
/// Where a request's data starts, after version, type, two reserved bytes,
/// the command, the attempt, the sequence and two words of padding.
const REQUEST_HEADER: usize = 20;
/// Where a reply's data starts, after version, type, two reserved bytes,
/// the command, the reply's kind, the status, three words' padding, the
/// sequence and two words more.
const REPLY_HEADER: usize = 28;
const ADDRESS: usize = 20;

/// What chronyd is asked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    /// How many sources it has.
    Sources,
    /// One source's state, by its index.
    SourceData(u32),
    /// How the clock is kept: the reference, its offset and frequency.
    Tracking,
    /// The statistics of one source's samples, by its index.
    SourceStats(u32),
    /// How far out the hardware clock is and how fast it drifts.
    Rtc,
    /// How many sources are online, offline and unresolved.
    Activity,
    /// One NTP source's last exchange, by its address.
    NtpData(Address),
    /// The name one source was given, by its address.
    SourceName(Address),
    /// Why one source was chosen or set aside, by its index.
    SelectData(u32),
}

impl Command {
    /// The request's code, its data, the code of the reply it's answered
    /// with and that reply's data's length.
    fn layout(self) -> (u16, Vec<u8>, u16, usize) {
        match self {
            Self::Sources => (14, Vec::new(), 2, 4),
            Self::SourceData(index) => (15, index.to_be_bytes().to_vec(), 3, 48),
            Self::Tracking => (33, Vec::new(), 5, 80),
            Self::SourceStats(index) => (34, index.to_be_bytes().to_vec(), 6, 56),
            Self::Rtc => (35, Vec::new(), 7, 28),
            Self::Activity => (44, Vec::new(), 12, 20),
            Self::NtpData(address) => (57, address.to_bytes().to_vec(), 26, 144),
            Self::SourceName(address) => (65, address.to_bytes().to_vec(), 19, 256),
            Self::SelectData(index) => (69, index.to_be_bytes().to_vec(), 23, 48),
        }
    }
}

/// A source's address as chronyd keeps it: an IP address, or, before a
/// name has resolved, an id of chronyd's own.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Address {
    Unspecified,
    Ip(IpAddr),
    Id(u32),
}

impl Address {
    fn to_bytes(self) -> [u8; ADDRESS] {
        let mut bytes = [0; ADDRESS];
        let family: u16 = match self {
            Self::Unspecified => 0,
            Self::Ip(IpAddr::V4(ip)) => {
                bytes[..4].copy_from_slice(&ip.octets());
                1
            }
            Self::Ip(IpAddr::V6(ip)) => {
                bytes[..16].copy_from_slice(&ip.octets());
                2
            }
            Self::Id(id) => {
                bytes[..4].copy_from_slice(&id.to_be_bytes());
                3
            }
        };
        bytes[16..18].copy_from_slice(&family.to_be_bytes());
        bytes
    }
}

/// Why a reply couldn't be taken.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Refused {
    /// It isn't a reply to this request: another sequence, command or
    /// version, or too short. The next datagram may be.
    NotOurs,
    /// chronyd answered with this status rather than the data, as 13 for a
    /// hardware clock it doesn't track or 4 for a source it doesn't have.
    Status(u16),
}

/// The request for `command`, numbered `sequence`.
pub fn request(command: Command, sequence: u32) -> Vec<u8> {
    let (code, data, _, reply) = command.layout();
    let mut bytes = vec![VERSION, REQUEST, 0, 0];
    bytes.extend_from_slice(&code.to_be_bytes());
    bytes.extend_from_slice(&0_u16.to_be_bytes());
    bytes.extend_from_slice(&sequence.to_be_bytes());
    bytes.extend_from_slice(&[0; 8]);
    bytes.extend_from_slice(&data);
    let length = (REQUEST_HEADER + data.len()).max(REPLY_HEADER + reply);
    bytes.resize(length, 0);
    bytes
}

/// The data of `bytes` as the reply to `command` numbered `sequence`.
pub fn reply(command: Command, sequence: u32, bytes: &[u8]) -> Result<&[u8], Refused> {
    let (code, _, kind, length) = command.layout();
    let word = |at: usize| u16::from_be_bytes([bytes[at], bytes[at + 1]]);
    if bytes.len() < REPLY_HEADER
        || bytes[0] != VERSION
        || bytes[1] != REPLY
        || word(4) != code
        || bytes[16..20] != sequence.to_be_bytes()
    {
        return Err(Refused::NotOurs);
    }
    let status = word(8);
    if status != 0 {
        return Err(Refused::Status(status));
    }
    if word(6) != kind || bytes.len() < REPLY_HEADER + length {
        return Err(Refused::NotOurs);
    }
    Ok(&bytes[REPLY_HEADER..REPLY_HEADER + length])
}

/// Reads a reply's data in order.
struct Fields<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Fields<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, at: 0 }
    }

    fn take<const N: usize>(&mut self) -> [u8; N] {
        let mut taken = [0; N];
        if let Some(bytes) = self.bytes.get(self.at..self.at + N) {
            taken.copy_from_slice(bytes);
        }
        self.at += N;
        taken
    }

    fn u8(&mut self) -> u8 {
        self.take::<1>()[0]
    }

    fn i8(&mut self) -> i8 {
        i8::from_be_bytes(self.take())
    }

    fn u16(&mut self) -> u16 {
        u16::from_be_bytes(self.take())
    }

    fn i16(&mut self) -> i16 {
        i16::from_be_bytes(self.take())
    }

    fn u32(&mut self) -> u32 {
        u32::from_be_bytes(self.take())
    }

    fn i32(&mut self) -> i32 {
        i32::from_be_bytes(self.take())
    }

    fn float(&mut self) -> f64 {
        float(self.u32())
    }

    /// A time as seconds since 1970, its high word unused where chronyd
    /// keeps 32-bit times.
    fn time(&mut self) -> f64 {
        let high = self.u32();
        let low = self.u32();
        let nanos = self.u32().min(999_999_999);
        let high = if high == 0x7fff_ffff { 0 } else { high };
        let seconds = (u64::from(high) << 32) | u64::from(low);
        #[expect(
            clippy::cast_precision_loss,
            reason = "a time to the microsecond past 2^52 seconds is millions of years off"
        )]
        let seconds = seconds as f64;
        seconds + f64::from(nanos) / 1e9
    }

    fn address(&mut self) -> Address {
        let bytes: [u8; ADDRESS] = self.take();
        let family = u16::from_be_bytes([bytes[16], bytes[17]]);
        let four = [bytes[0], bytes[1], bytes[2], bytes[3]];
        match family {
            1 => Address::Ip(IpAddr::V4(Ipv4Addr::from(four))),
            2 => {
                let mut sixteen = [0; 16];
                sixteen.copy_from_slice(&bytes[..16]);
                Address::Ip(IpAddr::V6(Ipv6Addr::from(sixteen)))
            }
            3 => Address::Id(u32::from_be_bytes(four)),
            _ => Address::Unspecified,
        }
    }
}

/// chronyd's 32-bit float: a 7-bit signed exponent over a 25-bit signed
/// coefficient, `coefficient * 2^(exponent - 25)`.
pub fn float(word: u32) -> f64 {
    let exponent = i32::try_from(word >> 25).unwrap_or(0);
    let exponent = if exponent >= 64 {
        exponent - 128
    } else {
        exponent
    } - 25;
    let coefficient = i32::try_from(word & 0x01ff_ffff).unwrap_or(0);
    let coefficient = if coefficient >= 1 << 24 {
        coefficient - (1 << 25)
    } else {
        coefficient
    };
    f64::from(coefficient) * 2_f64.powi(exponent)
}

/// A reference id as chronyd shows it: its printable letters, as "GPS" of
/// a stratum-one server's, or, for an address's hash, its hex.
pub fn reference(id: u32) -> String {
    let letters: String = id
        .to_be_bytes()
        .iter()
        .filter(|byte| byte.is_ascii_graphic())
        .map(|&byte| char::from(byte))
        .collect();
    if letters.len() == id.to_be_bytes().iter().filter(|&&byte| byte != 0).count()
        && !letters.is_empty()
    {
        letters
    } else {
        format!("{id:08X}")
    }
}

/// The leap second announced.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Leap {
    Normal,
    Insert,
    Delete,
    Unsynchronised,
}

impl Leap {
    const fn of(status: u16) -> Self {
        match status {
            0 => Self::Normal,
            1 => Self::Insert,
            2 => Self::Delete,
            _ => Self::Unsynchronised,
        }
    }
}

/// How the clock is kept, as `chronyc tracking` says it.
#[derive(Clone, Debug, PartialEq)]
pub struct Tracking {
    pub reference: u32,
    pub address: Address,
    pub stratum: u16,
    pub leap: Leap,
    /// When the clock was last set by the reference, in seconds since 1970,
    /// zero before it ever was.
    pub reference_time: f64,
    /// How far the clock is behind true time, in seconds, which chronyd is
    /// slewing away: positive while it's slow.
    pub correction: f64,
    /// The offset at the last update, positive when the clock was ahead.
    pub last_offset: f64,
    pub rms_offset: f64,
    /// How fast the clock runs without correction, in parts per million:
    /// positive while it's fast.
    pub frequency: f64,
    pub residual_frequency: f64,
    pub skew: f64,
    pub root_delay: f64,
    pub root_dispersion: f64,
    pub update_interval: f64,
}

pub fn tracking(data: &[u8]) -> Tracking {
    let mut fields = Fields::new(data);
    Tracking {
        reference: fields.u32(),
        address: fields.address(),
        stratum: fields.u16(),
        leap: Leap::of(fields.u16()),
        reference_time: fields.time(),
        correction: fields.float(),
        last_offset: fields.float(),
        rms_offset: fields.float(),
        frequency: fields.float(),
        residual_frequency: fields.float(),
        skew: fields.float(),
        root_delay: fields.float(),
        root_dispersion: fields.float(),
        update_interval: fields.float(),
    }
}

pub fn sources(data: &[u8]) -> u32 {
    Fields::new(data).u32()
}

/// What kind of source it is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Server,
    Peer,
    Reference,
}

/// Where a source stood at the last selection, as `chronyc sources` marks
/// it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    /// `*`, the one the clock is kept by.
    Chosen,
    /// `+`, combined with the chosen one.
    Combined,
    /// `-`, selectable but not combined.
    NotCombined,
    /// `x`, disagreeing with the others.
    Falseticker,
    /// `~`, too variable.
    Jittery,
    /// `?`, unusable: unreachable or not yet sampled enough.
    Unusable,
}

/// One source, as `chronyc sources` says it.
#[derive(Clone, Debug, PartialEq)]
pub struct Source {
    pub address: Address,
    /// How often it's asked, as a power of two seconds.
    pub poll: i16,
    pub stratum: u16,
    pub state: State,
    pub mode: Mode,
    /// The last eight polls, a bit each, the latest lowest, set when it
    /// answered.
    pub reach: u16,
    /// Seconds since its last sample, or none while it has none.
    pub since_sample: Option<u32>,
    /// Its last sample's offset, adjusted for what the clock has been
    /// slewed since, and as measured, and the error it may hold, in
    /// seconds. Positive offsets are the clock ahead of the source.
    pub offset: f64,
    pub measured: f64,
    pub error: f64,
}

pub fn source(data: &[u8]) -> Source {
    let mut fields = Fields::new(data);
    let address = fields.address();
    let poll = fields.i16();
    let stratum = fields.u16();
    let state = match fields.u16() {
        0 => State::Chosen,
        2 => State::Falseticker,
        3 => State::Jittery,
        4 => State::Combined,
        5 => State::NotCombined,
        _ => State::Unusable,
    };
    let mode = match fields.u16() {
        1 => Mode::Peer,
        2 => Mode::Reference,
        _ => Mode::Server,
    };
    let _flags = fields.u16();
    let reach = fields.u16();
    let since = fields.u32();
    // chronyc's report of the measured offset comes first, then the
    // adjusted one, then the error.
    let measured = fields.float();
    let offset = fields.float();
    let error = fields.float();
    Source {
        address,
        poll,
        stratum,
        state,
        mode,
        reach,
        since_sample: (since != u32::MAX).then_some(since),
        offset,
        measured,
        error,
    }
}

/// What a source's samples say together, as `chronyc sourcestats` says it.
#[derive(Clone, Debug, PartialEq)]
pub struct SourceStats {
    pub samples: u32,
    pub runs: u32,
    pub span: u32,
    pub deviation: f64,
    pub residual_frequency: f64,
    pub skew: f64,
    pub offset: f64,
    pub offset_error: f64,
}

pub fn source_stats(data: &[u8]) -> SourceStats {
    let mut fields = Fields::new(data);
    let _reference = fields.u32();
    let _address = fields.address();
    SourceStats {
        samples: fields.u32(),
        runs: fields.u32(),
        span: fields.u32(),
        deviation: fields.float(),
        residual_frequency: fields.float(),
        skew: fields.float(),
        offset: fields.float(),
        offset_error: fields.float(),
    }
}

/// The hardware clock, as `chronyc rtcdata` says it.
#[derive(Clone, Debug, PartialEq)]
pub struct Rtc {
    pub reference_time: f64,
    pub samples: u16,
    pub runs: u16,
    pub span: u32,
    /// How far ahead of true time it is, in seconds.
    pub fast: f64,
    /// How fast it gains, in parts per million.
    pub gain: f64,
}

pub fn rtc(data: &[u8]) -> Rtc {
    let mut fields = Fields::new(data);
    Rtc {
        reference_time: fields.time(),
        samples: fields.u16(),
        runs: fields.u16(),
        span: fields.u32(),
        fast: fields.float(),
        gain: fields.float(),
    }
}

/// How many sources are online and offline, as `chronyc activity` says.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Activity {
    pub online: i32,
    pub offline: i32,
    pub burst_online: i32,
    pub burst_offline: i32,
    pub unresolved: i32,
}

pub fn activity(data: &[u8]) -> Activity {
    let mut fields = Fields::new(data);
    Activity {
        online: fields.i32(),
        offline: fields.i32(),
        burst_online: fields.i32(),
        burst_offline: fields.i32(),
        unresolved: fields.i32(),
    }
}

/// An NTP source's last exchange, as `chronyc ntpdata` says it.
#[derive(Clone, Debug, PartialEq)]
pub struct NtpData {
    pub local: Address,
    pub port: u16,
    pub leap: u8,
    pub version: u8,
    pub mode: u8,
    pub stratum: u8,
    pub poll: i8,
    /// How finely the server's clock reads, as a power of two seconds.
    pub precision: i8,
    pub root_delay: f64,
    pub root_dispersion: f64,
    pub reference: u32,
    pub reference_time: f64,
    pub offset: f64,
    pub delay: f64,
    pub dispersion: f64,
    pub response_time: f64,
    pub jitter_asymmetry: f64,
    /// Which of the ten tests its last packet passed, a bit each.
    pub tests: u16,
    pub interleaved: bool,
    pub authenticated: bool,
    pub sent: u32,
    pub received: u32,
    pub valid: u32,
    pub good: u32,
}

pub fn ntp_data(data: &[u8]) -> NtpData {
    let mut fields = Fields::new(data);
    let _remote = fields.address();
    let local = fields.address();
    let port = fields.u16();
    let leap = fields.u8();
    let version = fields.u8();
    let mode = fields.u8();
    let stratum = fields.u8();
    let poll = fields.i8();
    let precision = fields.i8();
    let root_delay = fields.float();
    let root_dispersion = fields.float();
    let reference = fields.u32();
    let reference_time = fields.time();
    let offset = fields.float();
    let delay = fields.float();
    let dispersion = fields.float();
    let response_time = fields.float();
    let jitter_asymmetry = fields.float();
    let flags = fields.u16();
    let _transmit_stamps = fields.u8();
    let _receive_stamps = fields.u8();
    NtpData {
        local,
        port,
        leap,
        version,
        mode,
        stratum,
        poll,
        precision,
        root_delay,
        root_dispersion,
        reference,
        reference_time,
        offset,
        delay,
        dispersion,
        response_time,
        jitter_asymmetry,
        tests: flags & 0x3ff,
        interleaved: flags & 0x4000 != 0,
        authenticated: flags & 0x8000 != 0,
        sent: fields.u32(),
        received: fields.u32(),
        valid: fields.u32(),
        good: fields.u32(),
    }
}

/// The name a source was given, as `chronyc sourcename` says it.
pub fn source_name(data: &[u8]) -> String {
    let end = data
        .iter()
        .position(|&byte| byte == 0)
        .unwrap_or(data.len());
    String::from_utf8_lossy(&data[..end]).into_owned()
}

/// Why a source was chosen or set aside, as `chronyc selectdata` marks it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Selection {
    /// `N`, told never to be chosen.
    NoSelect,
    /// `M`, too few samples yet.
    TooFewSamples,
    /// `s`, not synchronised itself.
    Unsynchronised,
    /// `r`, a stratum outside what's allowed.
    Stratum,
    /// `d`, too far from its own reference.
    TooDistant,
    /// `~`, too variable.
    TooVariable,
    /// `w`, waiting on others to have enough samples.
    WaitingForOthers,
    /// `S`, its samples are old, or it's been unreachable too long.
    Stale,
    /// `O`, an orphan's stratum.
    Orphan,
    /// `T`, not agreeing with the trusted sources.
    Untrusted,
    /// `x`, disagreeing with the others.
    Disagrees,
    /// `W`, waiting for enough sources.
    WaitingForSources,
    /// `P`, another is preferred.
    NotPreferred,
    /// `U`, waiting for a new sample after a change of source.
    WaitingForSample,
    /// `D`, too far to combine with the others.
    TooFarToCombine,
    /// `+`, combined with the chosen one.
    Combined,
    /// `*`, the one the clock is kept by.
    Chosen,
    /// A mark this reader doesn't know.
    Other,
}

/// Why a source was chosen or set aside, and how it scored.
#[derive(Clone, Debug, PartialEq)]
pub struct SelectData {
    pub selection: Selection,
    pub last_sample: u32,
    pub score: f64,
    /// The interval it said true time lies in, from the clock, in seconds.
    pub low: f64,
    pub high: f64,
}

pub fn select_data(data: &[u8]) -> SelectData {
    let mut fields = Fields::new(data);
    let _reference = fields.u32();
    let _address = fields.address();
    let selection = match fields.u8() {
        b'N' => Selection::NoSelect,
        b'M' => Selection::TooFewSamples,
        b's' => Selection::Unsynchronised,
        b'r' => Selection::Stratum,
        b'd' => Selection::TooDistant,
        b'~' => Selection::TooVariable,
        b'w' => Selection::WaitingForOthers,
        b'S' => Selection::Stale,
        b'O' => Selection::Orphan,
        b'T' => Selection::Untrusted,
        b'x' => Selection::Disagrees,
        b'W' => Selection::WaitingForSources,
        b'P' => Selection::NotPreferred,
        b'U' => Selection::WaitingForSample,
        b'D' => Selection::TooFarToCombine,
        b'+' => Selection::Combined,
        b'*' => Selection::Chosen,
        _ => Selection::Other,
    };
    let _authentication = fields.u8();
    let _leap = fields.u8();
    let _pad = fields.u8();
    let _configured = fields.u16();
    let _effective = fields.u16();
    SelectData {
        selection,
        last_sample: fields.u32(),
        score: fields.float(),
        low: fields.float(),
        high: fields.float(),
    }
}

/// Built replies, as chronyd writes them, for the tests here and the
/// reader's.
#[cfg(test)]
pub mod written {
    use super::{Address, Command, REPLY, REPLY_HEADER, VERSION};

    /// chronyd's float for `x`, as its `UTI_FloatHostToNetwork` writes it.
    pub fn float(x: f64) -> [u8; 4] {
        if x.abs() < 1e-100 {
            return [0; 4];
        }
        let negative = x < 0.0;
        let magnitude = x.abs();
        #[expect(
            clippy::cast_possible_truncation,
            reason = "the test's figures are far inside chronyd's range"
        )]
        let mut exponent = (magnitude.log2() + 1.0) as i32;
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "a positive figure scaled into 25 bits"
        )]
        let mut coefficient = (magnitude * 2_f64.powi(25 - exponent) + 0.5) as u32;
        let most = (1_u32 << 24) - 1 + u32::from(negative);
        while coefficient > most {
            coefficient >>= 1;
            exponent += 1;
        }
        let coefficient = if negative {
            coefficient.wrapping_neg() & 0x01ff_ffff
        } else {
            coefficient
        };
        let exponent = u32::from_ne_bytes(exponent.to_ne_bytes()) & 0x7f;
        ((exponent << 25) | coefficient).to_be_bytes()
    }

    pub fn address(address: Address) -> [u8; 20] {
        address.to_bytes()
    }

    /// A time in seconds since 1970, whole seconds.
    pub fn time(seconds: u32) -> [u8; 12] {
        let mut bytes = [0; 12];
        bytes[4..8].copy_from_slice(&seconds.to_be_bytes());
        bytes
    }

    /// chronyd's reply to `command` numbered `sequence`, its status and
    /// its data.
    pub fn reply(command: Command, sequence: u32, status: u16, data: &[u8]) -> Vec<u8> {
        let (code, _, kind, length) = command.layout();
        let mut bytes = vec![VERSION, REPLY, 0, 0];
        bytes.extend_from_slice(&code.to_be_bytes());
        bytes.extend_from_slice(&kind.to_be_bytes());
        bytes.extend_from_slice(&status.to_be_bytes());
        bytes.extend_from_slice(&[0; 6]);
        bytes.extend_from_slice(&sequence.to_be_bytes());
        bytes.extend_from_slice(&[0; 8]);
        bytes.extend_from_slice(data);
        bytes.resize(REPLY_HEADER + length, 0);
        bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn near(a: f64, b: f64) -> bool {
        (a - b).abs() <= b.abs() * 1e-6 + 1e-12
    }

    #[test]
    fn a_request_is_padded_to_its_replys_length() {
        let asked = request(Command::Tracking, 7);
        assert_eq!(asked.len(), 28 + 80);
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
        assert!(near(read.reference_time, 1_790_000_000.0));
        assert!(near(read.correction, 0.000_012));
        assert!(near(read.last_offset, -0.000_388));
        assert!(near(read.frequency, 6.267));
        assert!(near(read.root_dispersion, 0.000_457));
        assert!(near(read.update_interval, 64.2));
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
}
