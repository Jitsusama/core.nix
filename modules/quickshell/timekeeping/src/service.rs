//! io.jitsusama.Timekeeping, which the shell and varlinkctl ask over
//! varlink on the socket systemd holds for the reader.
//!
//! ```sh
//! varlinkctl call --more "$XDG_RUNTIME_DIR/io.jitsusama.Timekeeping" \
//!   io.jitsusama.Timekeeping.Watch '{}'
//! varlinkctl call "$XDG_RUNTIME_DIR/io.jitsusama.Timekeeping" \
//!   io.jitsusama.Timekeeping.Calendars \
//!   '{"day": "2026-11-05", "latitude": 51.4769, "longitude": -0.0005}'
//! ```

use std::env;
use std::io;
use std::net::SocketAddr;
use std::os::fd::{FromRawFd, OwnedFd};
use std::process;
use std::time::Duration;

use futures_util::{Stream, stream};
use serde::{Deserialize, Serialize};
use tokio::time::{self, Instant};
use zlink::introspect::CustomType;
use zlink::tokio::unix::Listener;
use zlink::{Reply, introspect, service};

use crate::calendar::{self, Calendar, Calendars, Eve, Evening, Figure, Reckoning};
use crate::chrony::{self, Chrony, Names, Unanswered};

/// How long apart a watch's readings are.
pub const EVERY: Duration = Duration::from_secs(1);

pub struct Timekeeping {
    /// chronyd's command port.
    pub chrony: SocketAddr,
}

#[derive(Debug, Clone, PartialEq, zlink::ReplyError, introspect::ReplyError)]
#[zlink(interface = "io.jitsusama.Timekeeping")]
pub enum Unkept {
    /// chronyd didn't answer, for this reason: it isn't running, or it
    /// refused.
    NotAnswering { reason: String },
    /// The day asked for names no day, or none the calendars reach.
    NotADay { day: String },
}

/// The leap second announced, or that the clock isn't synchronised.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, CustomType)]
#[serde(rename_all = "camelCase")]
#[zlink(rename_all = "camelCase")]
pub enum Leaping {
    Normal,
    Insert,
    Delete,
    Unsynchronised,
}

/// How the clock is kept, as `chronyc tracking` says it. Offsets, delays
/// and dispersions are in seconds, frequencies in parts per million, and
/// times in milliseconds since 1970.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, CustomType)]
#[serde(rename_all = "camelCase")]
#[zlink(rename_all = "camelCase")]
pub struct Clock {
    /// The server the clock is kept by, by its name, or a reference
    /// clock's letters; empty while it's kept by none.
    pub reference: String,
    pub address: String,
    pub stratum: i64,
    pub leap: Leaping,
    pub last_sync: Option<f64>,
    /// How far the clock is behind true time, being slewed away: positive
    /// while it's slow.
    pub correction: f64,
    /// The offset at the last update, positive when the clock was ahead.
    pub last_offset: f64,
    pub rms_offset: f64,
    /// How fast the clock runs without correction: positive while fast.
    pub frequency: f64,
    pub residual_frequency: f64,
    pub skew: f64,
    pub root_delay: f64,
    pub root_dispersion: f64,
    /// Seconds between the last two updates.
    pub update_interval: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, CustomType)]
#[serde(rename_all = "camelCase")]
#[zlink(rename_all = "camelCase")]
pub enum Kind {
    Server,
    Peer,
    Reference,
}

/// Where a server stood at the last selection.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, CustomType)]
#[serde(rename_all = "camelCase")]
#[zlink(rename_all = "camelCase")]
pub enum Standing {
    Chosen,
    Combined,
    NotCombined,
    TooVariable,
    Disagrees,
    Unusable,
}

/// Why a server was chosen or set aside, as `chronyc selectdata` marks it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, CustomType)]
#[serde(rename_all = "camelCase")]
#[zlink(rename_all = "camelCase")]
pub enum Why {
    NoSelect,
    TooFewSamples,
    Unsynchronised,
    Stratum,
    TooDistant,
    TooVariable,
    WaitingForOthers,
    Stale,
    Orphan,
    Untrusted,
    Disagrees,
    WaitingForSources,
    NotPreferred,
    WaitingForSample,
    TooFarToCombine,
    Combined,
    Chosen,
    Other,
}

/// Why it was chosen or set aside, its score, and the interval it says
/// true time lies in, from the clock.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, CustomType)]
pub struct Chosen {
    pub why: Why,
    pub score: f64,
    pub low: f64,
    pub high: f64,
}

/// What a server's samples say together, as `chronyc sourcestats` says it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, CustomType)]
#[serde(rename_all = "camelCase")]
#[zlink(rename_all = "camelCase")]
pub struct Samples {
    /// How many samples it holds.
    pub count: i64,
    pub runs: i64,
    /// Seconds between the oldest sample and the newest.
    pub span: i64,
    pub deviation: f64,
    pub residual_frequency: f64,
    pub skew: f64,
    pub offset: f64,
    pub offset_error: f64,
}

/// A server's last exchange, as `chronyc ntpdata` says it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, CustomType)]
#[serde(rename_all = "camelCase")]
#[zlink(rename_all = "camelCase")]
pub struct Exchange {
    pub local_address: String,
    pub version: i64,
    pub stratum: i64,
    pub leap: Leaping,
    /// How finely its clock reads, in seconds.
    pub precision: f64,
    pub root_delay: f64,
    pub root_dispersion: f64,
    /// What it keeps its own time by.
    pub reference: String,
    pub reference_time: Option<f64>,
    pub offset: f64,
    pub delay: f64,
    pub dispersion: f64,
    pub response_time: f64,
    pub jitter_asymmetry: f64,
    /// How many of the ten tests its last packet passed.
    pub tests_passed: i64,
    pub interleaved: bool,
    pub authenticated: bool,
    pub sent: i64,
    pub received: i64,
    pub valid: i64,
    pub good: i64,
}

/// One server, as `chronyc sources` says it, with what else chronyd
/// answers of it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, CustomType)]
#[serde(rename_all = "camelCase")]
#[zlink(rename_all = "camelCase")]
pub struct Server {
    pub name: String,
    pub address: String,
    pub kind: Kind,
    pub standing: Standing,
    pub stratum: i64,
    /// Seconds between its polls.
    pub poll: f64,
    /// The last eight polls, a bit each, the latest lowest, set when it
    /// answered.
    pub reach: i64,
    /// Seconds since its last sample, none while it has none.
    pub last_sample: Option<i64>,
    /// Its last sample's offset, adjusted for the slewing since, and as
    /// measured, and the error it may hold: positive when the clock is
    /// ahead of it.
    pub offset: f64,
    pub measured: f64,
    pub error: f64,
    pub samples: Option<Samples>,
    pub chosen: Option<Chosen>,
    pub exchange: Option<Exchange>,
}

/// The hardware clock, as `chronyc rtcdata` says it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, CustomType)]
pub struct HardwareClock {
    /// Seconds it's ahead of true time.
    pub fast: f64,
    /// Parts per million it gains.
    pub gain: f64,
    pub samples: i64,
    pub runs: i64,
    pub span: i64,
    pub measured: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, CustomType)]
#[serde(rename_all = "camelCase")]
#[zlink(rename_all = "camelCase")]
pub struct Activity {
    pub online: i64,
    pub offline: i64,
    pub burst_online: i64,
    pub burst_offline: i64,
    pub unresolved: i64,
}

/// How the clock is kept now: the clock, every server the chosen first,
/// the hardware clock where chronyd tracks it, and how many servers are
/// online.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, CustomType)]
#[serde(rename_all = "camelCase")]
#[zlink(rename_all = "camelCase")]
pub struct Keeping {
    pub synchronised: bool,
    pub clock: Clock,
    pub servers: Vec<Server>,
    pub hardware_clock: Option<HardwareClock>,
    pub activity: Option<Activity>,
}

/// What a watch keeps between its answers: its conversation with chronyd,
/// the names it's heard, when it last read, and whether it's done.
type Watched = (Option<Chrony>, Names, Option<Instant>, bool);

fn unkept(why: &Unanswered) -> Unkept {
    Unkept::NotAnswering {
        reason: match why {
            Unanswered::Silent => "chronyd didn't answer".into(),
            Unanswered::Status(status) => format!("chronyd refused, with status {status}"),
            Unanswered::Unreachable(reason) => reason.clone(),
        },
    }
}

#[service(
    interface = "io.jitsusama.Timekeeping",
    types = [
        Activity, Calendar, Calendars, Chosen, Clock, Eve, Evening, Exchange, Figure, HardwareClock,
        Keeping, Kind, Leaping, Reckoning, Samples, Server, Standing, Why
    ],
    vendor = "jitsusama",
    product = "timekeeping",
    version = env!("CARGO_PKG_VERSION"),
    url = "https://github.com/Jitsusama/core.nix"
)]
impl Timekeeping {
    /// A day, as an ISO date, in the Chinese, Japanese, Hebrew and Islamic
    /// calendars, answered once: it's reckoned, so nothing is asked. Asked
    /// from a place, in degrees north and east, it says when the day ends
    /// there for the calendars whose days begin at sunset. The day is
    /// needed, but taken as optional: zlink hangs up on a call whose
    /// parameters it can't read, so `varlinkctl call ... '{}'` heard only
    /// "Connection reset by peer", and asked for no day it says `NotADay`.
    #[expect(
        clippy::unused_async,
        clippy::unused_async_trait_impl,
        reason = "zlink's service macro awaits every method it answers with, so each must be async"
    )]
    async fn calendars(
        &self,
        day: Option<String>,
        latitude: Option<f64>,
        longitude: Option<f64>,
    ) -> Result<Calendars, Unkept> {
        let day = day.unwrap_or_default();
        calendar::on(&day, latitude.zip(longitude)).ok_or(Unkept::NotADay { day })
    }

    /// How the clock is kept now. Called with more, it answers again each
    /// second until the call ends, asking chronyd only while someone is
    /// asking.
    #[zlink(more)]
    #[expect(
        clippy::unused_async,
        clippy::unused_async_trait_impl,
        reason = "zlink's service macro awaits every method it answers with, so each must be async"
    )]
    async fn watch(
        &self,
        more: bool,
    ) -> impl Stream<Item = Result<Reply<Keeping>, Unkept>> + Unpin {
        let at = self.chrony;
        let replies = stream::unfold(
            (None, Names::new(), None, false),
            move |(chrony, mut names, last, done): Watched| async move {
                if done {
                    return None;
                }
                if let Some(last) = last {
                    time::sleep_until(last + EVERY).await;
                }
                let now = Instant::now();
                let mut chrony = match chrony {
                    Some(chrony) => chrony,
                    None => match Chrony::open(at).await {
                        Ok(chrony) => chrony,
                        Err(why) => return Some((Err(unkept(&why)), (None, names, None, true))),
                    },
                };
                match chrony::read(&mut chrony, &mut names).await {
                    Ok(keeping) => {
                        let reply = Reply::new(Some(keeping)).set_continues(Some(more));
                        Some((Ok(reply), (Some(chrony), names, Some(now), !more)))
                    }
                    Err(why) => Some((Err(unkept(&why)), (None, names, None, true))),
                }
            },
        );
        Box::pin(replies)
    }
}

/// The socket systemd holds for the reader, if it passed one, as the first
/// of the files it hands a service.
pub fn listener() -> io::Result<Option<Listener>> {
    let ours = env::var("LISTEN_PID").ok().and_then(|pid| pid.parse().ok()) == Some(process::id());
    if !ours || env::var("LISTEN_FDS").as_deref() != Ok("1") {
        return Ok(None);
    }
    // SAFETY: systemd handed this process the socket as file 3, its first,
    // and nothing else in the reader takes it.
    let socket = unsafe { OwnedFd::from_raw_fd(3) };
    let listener =
        Listener::try_from(socket).map_err(|error| io::Error::other(error.to_string()))?;
    Ok(Some(listener))
}

#[cfg(test)]
mod tests {
    use futures_util::StreamExt;

    use super::*;
    use crate::chrony::stand_in;

    #[tokio::test]
    async fn asked_once_it_answers_once() {
        let timekeeping = Timekeeping {
            chrony: stand_in::start().await,
        };
        let mut replies = timekeeping.watch(false).await;
        let Some(Ok(reply)) = replies.next().await else {
            panic!("no reading");
        };
        let keeping = reply.into_parameters();
        assert!(keeping.is_some_and(|keeping| keeping.servers.len() == 2));
        assert!(replies.next().await.is_none());
    }

    #[tokio::test]
    async fn watched_it_answers_again_a_second_later() {
        let timekeeping = Timekeeping {
            chrony: stand_in::start().await,
        };
        let mut replies = timekeeping.watch(true).await;
        let started = Instant::now();
        assert!(matches!(replies.next().await, Some(Ok(_))));
        assert!(matches!(replies.next().await, Some(Ok(_))));
        assert!(started.elapsed() >= EVERY);
    }

    #[tokio::test]
    async fn with_chronyd_gone_it_says_so_and_ends() {
        let gone = tokio::net::UdpSocket::bind("127.0.0.1:0")
            .await
            .and_then(|socket| socket.local_addr())
            .unwrap_or_else(|error| panic!("{error}"));
        let timekeeping = Timekeeping { chrony: gone };
        let mut replies = timekeeping.watch(true).await;
        assert!(matches!(
            replies.next().await,
            Some(Err(Unkept::NotAnswering { .. }))
        ));
        assert!(replies.next().await.is_none());
    }

    /// Everything said on a connection until its first reply ends, or
    /// until it's hung up.
    async fn said_to(stream: &tokio::net::UnixStream) -> io::Result<String> {
        let mut said = Vec::new();
        while !said.ends_with(&[0]) {
            stream.readable().await?;
            let mut read = [0; 4096];
            match stream.try_read(&mut read) {
                Ok(0) => break,
                Ok(count) => said.extend_from_slice(&read[..count]),
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {}
                Err(error) => return Err(error),
            }
        }
        Ok(String::from_utf8_lossy(&said).into_owned())
    }

    #[tokio::test]
    async fn asked_for_no_day_it_says_so_rather_than_hanging_up() {
        let path = env::temp_dir().join(format!("timekeeping-{}.socket", process::id()));
        let _gone = std::fs::remove_file(&path);
        let listener = zlink::tokio::unix::bind(&path).unwrap_or_else(|error| panic!("{error}"));
        let timekeeping = Timekeeping {
            chrony: SocketAddr::from(([127, 0, 0, 1], 0)),
        };
        let served = zlink::Server::new(listener, timekeeping).run();
        let asked = async {
            let stream = tokio::net::UnixStream::connect(&path).await?;
            let call = b"{\"method\":\"io.jitsusama.Timekeeping.Calendars\",\"parameters\":{}}\0";
            stream.writable().await?;
            assert_eq!(stream.try_write(call)?, call.len());
            said_to(&stream).await
        };
        let said = tokio::select! {
            ended = served => panic!("the reader stopped serving: {ended:?}"),
            said = asked => said.unwrap_or_else(|error| panic!("{error}")),
        };
        let _gone = std::fs::remove_file(&path);
        assert!(
            said.contains("\"io.jitsusama.Timekeeping.NotADay\""),
            "the reader said {said:?}"
        );
    }
}
