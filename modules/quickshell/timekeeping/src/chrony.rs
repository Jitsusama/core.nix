//! Asking chronyd over its command port, and what its answers say together:
//! how the clock is kept, and every server, the chosen one first.

use std::collections::HashMap;
use std::io;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::time::Duration;

use tokio::net::UdpSocket;
use tokio::time::{self, Instant};

use crate::candm::{self, Address, Command, Leap, Mode, Refused, Selection, State};
use crate::service::{
    Activity, Chosen, Clock, Exchange, HardwareClock, Keeping, Kind, Leaping, Samples, Server,
    Standing, Why,
};

/// How long a reply may take before the request is sent again. chronyd on
/// the same machine answers in well under a millisecond.
const WAIT: Duration = Duration::from_millis(300);
const ATTEMPTS: usize = 3;

/// The most names kept between readings: far more servers than any
/// machine is given, so it's only ever cleared by a pool's names turning
/// over for good.
const NAMES: usize = 64;

/// Why chronyd's answer didn't come.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Unanswered {
    /// Nothing answered in three tries.
    Silent,
    /// chronyd answered with a status other than success.
    Status(u16),
    /// The port couldn't be reached, as when chronyd isn't running.
    Unreachable(String),
}

impl Unanswered {
    fn of(error: &io::Error) -> Self {
        Self::Unreachable(error.to_string())
    }
}

/// A conversation with chronyd.
pub struct Chrony {
    socket: UdpSocket,
    sequence: u32,
}

impl Chrony {
    /// A socket of its own on the loopback address, for chronyd's port at
    /// `at`.
    pub async fn open(at: SocketAddr) -> Result<Self, Unanswered> {
        let here: IpAddr = if at.is_ipv4() {
            Ipv4Addr::LOCALHOST.into()
        } else {
            Ipv6Addr::LOCALHOST.into()
        };
        let socket = UdpSocket::bind((here, 0))
            .await
            .map_err(|error| Unanswered::of(&error))?;
        socket
            .connect(at)
            .await
            .map_err(|error| Unanswered::of(&error))?;
        Ok(Self {
            socket,
            sequence: 0,
        })
    }

    /// The data of chronyd's answer to `command`, asked again where none
    /// comes, and a datagram answering anything else passed over.
    pub async fn ask(&mut self, command: Command) -> Result<Vec<u8>, Unanswered> {
        self.sequence = self.sequence.wrapping_add(1);
        let sequence = self.sequence;
        let request = candm::request(command, sequence);
        let mut buffer = [0; 1024];
        for _ in 0..ATTEMPTS {
            self.socket
                .send(&request)
                .await
                .map_err(|error| Unanswered::of(&error))?;
            let deadline = Instant::now() + WAIT;
            while let Ok(received) = time::timeout_at(deadline, self.socket.recv(&mut buffer)).await
            {
                let length = received.map_err(|error| Unanswered::of(&error))?;
                match candm::reply(command, sequence, &buffer[..length]) {
                    Ok(data) => return Ok(data.to_vec()),
                    Err(Refused::Status(status)) => return Err(Unanswered::Status(status)),
                    Err(Refused::NotOurs) => {}
                }
            }
        }
        Err(Unanswered::Silent)
    }

    /// An answer chronyd may refuse, as one for a hardware clock it doesn't
    /// track: none when it does.
    async fn may_ask(&mut self, command: Command) -> Result<Option<Vec<u8>>, Unanswered> {
        match self.ask(command).await {
            Ok(data) => Ok(Some(data)),
            Err(Unanswered::Status(_)) => Ok(None),
            Err(error) => Err(error),
        }
    }
}

/// The names chronyd gave its servers, by address, kept between readings
/// since a server's name doesn't change while it's asked.
pub type Names = HashMap<Address, String>;

/// How the clock is kept now, as chronyd tells it.
pub async fn read(chrony: &mut Chrony, names: &mut Names) -> Result<Keeping, Unanswered> {
    let tracking = candm::tracking(&chrony.ask(Command::Tracking).await?);
    let count = candm::sources(&chrony.ask(Command::Sources).await?);
    let mut servers = Vec::new();
    for index in 0..count {
        // A source gone since it was counted is passed over.
        let Some(source) = chrony.may_ask(Command::SourceData(index)).await? else {
            continue;
        };
        let source = candm::source(&source);
        let name = name(chrony, names, &source).await?;
        let samples = chrony
            .may_ask(Command::SourceStats(index))
            .await?
            .map(|data| candm::source_stats(&data));
        let selection = chrony
            .may_ask(Command::SelectData(index))
            .await?
            .map(|data| candm::select_data(&data));
        let exchange = match (source.mode, source.address) {
            (Mode::Reference, _) | (_, Address::Id(_) | Address::Unspecified) => None,
            (_, Address::Ip(_)) => chrony
                .may_ask(Command::NtpData(source.address))
                .await?
                .map(|data| candm::ntp_data(&data)),
        };
        servers.push(server(&source, name, samples, selection, exchange));
    }
    servers.sort_by_key(|server| rank(server.standing));
    let hardware_clock = chrony
        .may_ask(Command::Rtc)
        .await?
        .map(|data| hardware_clock(&candm::rtc(&data)));
    let activity = chrony
        .may_ask(Command::Activity)
        .await?
        .map(|data| activity(&candm::activity(&data)));
    Ok(Keeping {
        synchronised: tracking.leap != Leap::Unsynchronised,
        clock: clock(&tracking, &servers),
        servers,
        hardware_clock,
        activity,
    })
}

/// A source's name: the one chronyd was given, once asked, its address
/// where it has none, and a reference clock's own letters.
async fn name(
    chrony: &mut Chrony,
    names: &mut Names,
    source: &candm::Source,
) -> Result<String, Unanswered> {
    if source.mode == Mode::Reference {
        return Ok(match source.address {
            Address::Ip(IpAddr::V4(ip)) => candm::reference(u32::from(ip)),
            _ => String::new(),
        });
    }
    if let Some(name) = names.get(&source.address) {
        return Ok(name.clone());
    }
    let given = chrony
        .may_ask(Command::SourceName(source.address))
        .await?
        .map(|data| candm::source_name(&data))
        .filter(|name| !name.is_empty());
    let name = given.unwrap_or_else(|| address(source.address));
    if names.len() >= NAMES {
        names.clear();
    }
    names.insert(source.address, name.clone());
    Ok(name)
}

fn address(address: Address) -> String {
    match address {
        Address::Ip(ip) => ip.to_string(),
        Address::Id(_) | Address::Unspecified => String::new(),
    }
}

/// Where a standing sorts: the chosen first, then those that agree, then
/// those set aside, and those out of reach last.
const fn rank(standing: Standing) -> u8 {
    match standing {
        Standing::Chosen => 0,
        Standing::Combined => 1,
        Standing::NotCombined => 2,
        Standing::TooVariable => 3,
        Standing::Disagrees => 4,
        Standing::Unusable => 5,
    }
}

/// Seconds as milliseconds since 1970, or none for chronyd's zero, a time
/// that never was.
fn when(seconds: f64) -> Option<f64> {
    (seconds > 0.0).then_some(seconds * 1000.0)
}

const fn leaping(leap: Leap) -> Leaping {
    match leap {
        Leap::Normal => Leaping::Normal,
        Leap::Insert => Leaping::Insert,
        Leap::Delete => Leaping::Delete,
        Leap::Unsynchronised => Leaping::Unsynchronised,
    }
}

fn clock(tracking: &candm::Tracking, servers: &[Server]) -> Clock {
    let by = address(tracking.address);
    let server = servers
        .iter()
        .find(|server| !by.is_empty() && server.address == by)
        .map(|server| server.name.clone());
    let reference = if tracking.reference == 0 {
        String::new()
    } else {
        server.unwrap_or_else(|| candm::reference(tracking.reference))
    };
    Clock {
        reference,
        address: by,
        stratum: tracking.stratum.into(),
        leap: leaping(tracking.leap),
        last_sync: when(tracking.reference_time),
        correction: tracking.correction,
        last_offset: tracking.last_offset,
        rms_offset: tracking.rms_offset,
        frequency: tracking.frequency,
        residual_frequency: tracking.residual_frequency,
        skew: tracking.skew,
        root_delay: tracking.root_delay,
        root_dispersion: tracking.root_dispersion,
        update_interval: tracking.update_interval,
    }
}

const fn why(selection: Selection) -> Why {
    match selection {
        Selection::NoSelect => Why::NoSelect,
        Selection::TooFewSamples => Why::TooFewSamples,
        Selection::Unsynchronised => Why::Unsynchronised,
        Selection::Stratum => Why::Stratum,
        Selection::TooDistant => Why::TooDistant,
        Selection::TooVariable => Why::TooVariable,
        Selection::WaitingForOthers => Why::WaitingForOthers,
        Selection::Stale => Why::Stale,
        Selection::Orphan => Why::Orphan,
        Selection::Untrusted => Why::Untrusted,
        Selection::Disagrees => Why::Disagrees,
        Selection::WaitingForSources => Why::WaitingForSources,
        Selection::NotPreferred => Why::NotPreferred,
        Selection::WaitingForSample => Why::WaitingForSample,
        Selection::TooFarToCombine => Why::TooFarToCombine,
        Selection::Combined => Why::Combined,
        Selection::Chosen => Why::Chosen,
        Selection::Other => Why::Other,
    }
}

fn server(
    source: &candm::Source,
    name: String,
    samples: Option<candm::SourceStats>,
    selection: Option<candm::SelectData>,
    exchange: Option<candm::NtpData>,
) -> Server {
    Server {
        name,
        address: address(source.address),
        kind: match source.mode {
            Mode::Server => Kind::Server,
            Mode::Peer => Kind::Peer,
            Mode::Reference => Kind::Reference,
        },
        standing: match source.state {
            State::Chosen => Standing::Chosen,
            State::Combined => Standing::Combined,
            State::NotCombined => Standing::NotCombined,
            State::Jittery => Standing::TooVariable,
            State::Falseticker => Standing::Disagrees,
            State::Unusable => Standing::Unusable,
        },
        stratum: source.stratum.into(),
        poll: 2_f64.powi(source.poll.into()),
        reach: source.reach.into(),
        last_sample: source.since_sample.map(i64::from),
        offset: source.offset,
        measured: source.measured,
        error: source.error,
        samples: samples.map(|stats| Samples {
            count: stats.samples.into(),
            runs: stats.runs.into(),
            span: stats.span.into(),
            deviation: stats.deviation,
            residual_frequency: stats.residual_frequency,
            skew: stats.skew,
            offset: stats.offset,
            offset_error: stats.offset_error,
        }),
        chosen: selection.map(|selection| Chosen {
            why: why(selection.selection),
            score: selection.score,
            low: selection.low,
            high: selection.high,
        }),
        exchange: exchange.map(|data| Exchange {
            local_address: address(data.local),
            version: data.version.into(),
            stratum: data.stratum.into(),
            leap: leaping(match data.leap {
                0 => Leap::Normal,
                1 => Leap::Insert,
                2 => Leap::Delete,
                _ => Leap::Unsynchronised,
            }),
            precision: 2_f64.powi(data.precision.into()),
            root_delay: data.root_delay,
            root_dispersion: data.root_dispersion,
            reference: candm::reference(data.reference),
            reference_time: when(data.reference_time),
            offset: data.offset,
            delay: data.delay,
            dispersion: data.dispersion,
            response_time: data.response_time,
            jitter_asymmetry: data.jitter_asymmetry,
            tests_passed: data.tests.count_ones().into(),
            interleaved: data.interleaved,
            authenticated: data.authenticated,
            sent: data.sent.into(),
            received: data.received.into(),
            valid: data.valid.into(),
            good: data.good.into(),
        }),
    }
}

fn hardware_clock(rtc: &candm::Rtc) -> HardwareClock {
    HardwareClock {
        fast: rtc.fast,
        gain: rtc.gain,
        samples: rtc.samples.into(),
        runs: rtc.runs.into(),
        span: rtc.span.into(),
        measured: when(rtc.reference_time),
    }
}

fn activity(activity: &candm::Activity) -> Activity {
    Activity {
        online: activity.online.into(),
        offline: activity.offline.into(),
        burst_online: activity.burst_online.into(),
        burst_offline: activity.burst_offline.into(),
        unresolved: activity.unresolved.into(),
    }
}

/// A chronyd of the tests' own on the loopback address, answering what's
/// asked from replies written as chronyd writes them.
#[cfg(test)]
pub mod stand_in {
    use std::net::SocketAddr;

    use tokio::net::UdpSocket;

    use crate::candm::{Address, Command, written};

    /// What the stand-in serves: the chosen server at 192.0.2.7 named
    /// time.harbour.test, a falseticker at 192.0.2.9 with no name of its
    /// own, and no hardware clock.
    fn answer(code: u16, asked: &[u8]) -> (Command, u16, Vec<u8>) {
        let index = u32::from_be_bytes([asked[20], asked[21], asked[22], asked[23]]);
        let chosen = Address::Ip([192, 0, 2, 7].into());
        let wrong = Address::Ip([192, 0, 2, 9].into());
        let source = |address, state: u16, offset: f64| {
            let mut data = written::address(address).to_vec();
            data.extend_from_slice(&6_i16.to_be_bytes());
            data.extend_from_slice(&2_u16.to_be_bytes());
            data.extend_from_slice(&state.to_be_bytes());
            data.extend_from_slice(&0_u16.to_be_bytes());
            data.extend_from_slice(&0_u16.to_be_bytes());
            data.extend_from_slice(&0o377_u16.to_be_bytes());
            data.extend_from_slice(&12_u32.to_be_bytes());
            data.extend_from_slice(&written::float(offset));
            data.extend_from_slice(&written::float(offset));
            data.extend_from_slice(&written::float(0.000_02));
            data
        };
        match code {
            33 => {
                let mut data = 0xC000_0207_u32.to_be_bytes().to_vec();
                data.extend_from_slice(&written::address(chosen));
                data.extend_from_slice(&3_u16.to_be_bytes());
                data.extend_from_slice(&0_u16.to_be_bytes());
                data.extend_from_slice(&written::time(1_790_000_000));
                for figure in [
                    0.000_01, -0.000_4, 0.000_2, 6.267, 0.001, 0.03, 0.015, 0.000_5, 64.0,
                ] {
                    data.extend_from_slice(&written::float(figure));
                }
                (Command::Tracking, 0, data)
            }
            14 => (Command::Sources, 0, 2_u32.to_be_bytes().to_vec()),
            // The falseticker is chronyd's first, so the chosen one is
            // put first by the reader, not by chance.
            15 if index == 0 => (Command::SourceData(0), 0, source(wrong, 2, 0.25)),
            15 => (Command::SourceData(1), 0, source(chosen, 0, -0.000_4)),
            65 => {
                // Named only the chosen one; the other was given by address.
                let mut name = if asked[20..24] == [192, 0, 2, 7] {
                    b"time.harbour.test".to_vec()
                } else {
                    Vec::new()
                };
                name.resize(256, 0);
                (Command::SourceName(chosen), 0, name)
            }
            35 => (Command::Rtc, 13, Vec::new()),
            44 => (
                Command::Activity,
                0,
                [2_i32, 0, 0, 0, 0]
                    .iter()
                    .flat_map(|n| n.to_be_bytes())
                    .collect(),
            ),
            // Statistics, selection and exchanges it refuses, as a chronyd
            // that doesn't open them does.
            34 => (Command::SourceStats(index), 2, Vec::new()),
            69 => (Command::SelectData(index), 2, Vec::new()),
            _ => (Command::NtpData(chosen), 2, Vec::new()),
        }
    }

    /// Starts the stand-in, saying where it listens.
    pub async fn start() -> SocketAddr {
        let socket = UdpSocket::bind("127.0.0.1:0")
            .await
            .unwrap_or_else(|error| panic!("no socket for the stand-in: {error}"));
        let at = socket
            .local_addr()
            .unwrap_or_else(|error| panic!("{error}"));
        tokio::spawn(async move {
            let mut buffer = [0; 1024];
            while let Ok((length, from)) = socket.recv_from(&mut buffer).await {
                let asked = &buffer[..length];
                let code = u16::from_be_bytes([asked[4], asked[5]]);
                let sequence = u32::from_be_bytes([asked[8], asked[9], asked[10], asked[11]]);
                let (command, status, data) = answer(code, asked);
                let mut reply = written::reply(command, sequence, status, &data);
                // A refusal names the command asked, whichever it was.
                reply[4..6].copy_from_slice(&code.to_be_bytes());
                let _ = socket.send_to(&reply, from).await;
            }
        });
        at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn every_server_is_read_the_chosen_first() {
        let at = stand_in::start().await;
        let mut chrony = Chrony::open(at)
            .await
            .unwrap_or_else(|error| panic!("{error:?}"));
        let mut names = Names::new();
        let keeping = read(&mut chrony, &mut names)
            .await
            .unwrap_or_else(|error| panic!("{error:?}"));
        assert!(keeping.synchronised);
        let servers: Vec<(&str, Standing)> = keeping
            .servers
            .iter()
            .map(|server| (server.name.as_str(), server.standing))
            .collect();
        assert_eq!(
            servers,
            [
                ("time.harbour.test", Standing::Chosen),
                ("192.0.2.9", Standing::Disagrees),
            ]
        );
        // The clock names its reference by the server it's kept by.
        assert_eq!(keeping.clock.reference, "time.harbour.test");
        assert_eq!(keeping.clock.last_sync, Some(1_790_000_000_000.0));
        assert!((keeping.clock.frequency - 6.267).abs() < 1e-5);
        assert!((keeping.servers[0].poll - 64.0).abs() < f64::EPSILON);
        // What chronyd refuses is left out, and the rest still comes.
        assert_eq!(keeping.hardware_clock, None);
        assert_eq!(keeping.servers[0].samples, None);
        assert_eq!(keeping.activity.map(|activity| activity.online), Some(2));
        // Names are kept for the next reading.
        assert_eq!(names.len(), 2);
    }

    #[tokio::test]
    async fn with_nothing_on_the_port_it_says_so() {
        // A port bound and let go, so nothing listens on it.
        let at = UdpSocket::bind("127.0.0.1:0")
            .await
            .and_then(|socket| socket.local_addr())
            .unwrap_or_else(|error| panic!("{error}"));
        let mut chrony = Chrony::open(at)
            .await
            .unwrap_or_else(|error| panic!("{error:?}"));
        let answer = read(&mut chrony, &mut Names::new()).await;
        assert!(
            matches!(answer, Err(Unanswered::Unreachable(_) | Unanswered::Silent)),
            "{answer:?}"
        );
    }
}
