//! A card's link as the kernel's nl80211 says it: the station it's joined
//! to (signal at each antenna, the rates each way and how they're coded,
//! what was sent again, lost and missed, the access point's beacons) and
//! what the card's tuned to (its channel, width and transmit power).
//!
//! wpa_supplicant's `SignalPoll` reads only some of it, so the shell ran
//! `iw` three times every three seconds for the rest while it showed a
//! network (2026-11-10). wifid asks the kernel itself instead, over a
//! generic netlink socket anyone may read, only while someone watches a
//! network in detail, as it polls.
//!
//! The numbers are linux/nl80211.h's and linux/genetlink.h's, as of 7.2.
//!
//! Long because its cases, a third of it, stay beside the private parsers
//! they feed the kernel's own layout, as every module in wifid keeps them.

use std::io;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};

use tokio::io::Interest;
use tokio::io::unix::AsyncFd;

/// What a card's station says of its link. Each is none where the driver
/// doesn't say it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Station {
    /// In dBm: now, on average, and of the access point's beacons.
    pub signal: Option<i32>,
    pub average: Option<i32>,
    pub beacons: Option<i32>,
    /// The signal now at each antenna, in dBm.
    pub chains: Vec<i32>,
    pub receive: Rate,
    pub send: Rate,
    /// Frames sent again, and never sent, since the card joined.
    pub retries: Option<u32>,
    pub failed: Option<u32>,
    /// Beacons missed, and frames heard but dropped.
    pub missed: Option<u32>,
    pub dropped: Option<u64>,
    /// How long it's been joined, in seconds, and since a frame last came
    /// or went, in milliseconds.
    pub connected: Option<u32>,
    pub idle: Option<u32>,
    /// What the driver expects the link to carry, in kb/s.
    pub expected: Option<u32>,
    /// Whether its management frames are protected (MFP), and it sorts
    /// traffic by kind (WMM).
    pub protected: Option<bool>,
    pub wmm: Option<bool>,
    /// The access point's beacons: how often they come, in its time
    /// units, and one in how many says what it holds for a card asleep.
    pub interval: Option<u16>,
    pub dtim: Option<u8>,
    /// Whether frames open with the short preamble, and wait the short
    /// slot time.
    pub short_preamble: Option<bool>,
    pub short_slot: Option<bool>,
    /// How much has come and gone since the card joined.
    pub received: Traffic,
    pub sent: Traffic,
}

/// How much has gone one way.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Traffic {
    pub bytes: Option<u64>,
    pub packets: Option<u32>,
}

/// One way's rate, and how it was coded.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Rate {
    /// In kb/s.
    pub kbps: Option<u32>,
    /// 4 to 7, as the coding says it: EHT is Wi-Fi 7, HE 6, VHT 5 and HT 4.
    pub generation: Option<u8>,
    pub mcs: Option<u32>,
    pub streams: Option<u32>,
    /// In MHz.
    pub width: Option<u16>,
    /// The pause between symbols, in microseconds.
    pub guard: Option<f64>,
    /// Whether Wi-Fi 6's dual carrier is on.
    pub dcm: Option<bool>,
}

/// What a card's tuned to.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Tuning {
    /// In MHz.
    pub frequency: Option<u32>,
    pub width: Option<u16>,
    /// Whether it's a channel older than Wi-Fi 4, 20 MHz with no HT.
    pub legacy: bool,
    /// In dBm.
    pub power: Option<f64>,
}

// Netlink's message types and flags.
const NLMSG_ERROR: u16 = 2;
const NLMSG_DONE: u16 = 3;
const NLM_F_REQUEST: u16 = 1;
const NLM_F_ACK: u16 = 4;
const NLM_F_DUMP: u16 = 0x300;
/// An attribute's type bits; the two above say how it's laid out.
const NLA_TYPE: u16 = 0x3fff;

// Generic netlink's controller, which names each family's number.
const GENL_ID_CTRL: u16 = 16;
const CTRL_CMD_GETFAMILY: u8 = 3;
const CTRL_ATTR_FAMILY_ID: u16 = 1;
const CTRL_ATTR_FAMILY_NAME: u16 = 2;

const CMD_GET_INTERFACE: u8 = 5;
const CMD_GET_STATION: u8 = 17;

const ATTR_IFINDEX: u16 = 3;
const ATTR_STA_INFO: u16 = 21;
const ATTR_WIPHY_FREQ: u16 = 38;
const ATTR_WIPHY_TX_POWER_LEVEL: u16 = 98;
const ATTR_CHANNEL_WIDTH: u16 = 159;

const STA_INFO_INACTIVE_TIME: u16 = 1;
const STA_INFO_RX_BYTES: u16 = 2;
const STA_INFO_TX_BYTES: u16 = 3;
const STA_INFO_SIGNAL: u16 = 7;
const STA_INFO_TX_BITRATE: u16 = 8;
const STA_INFO_RX_PACKETS: u16 = 9;
const STA_INFO_TX_PACKETS: u16 = 10;
const STA_INFO_TX_RETRIES: u16 = 11;
const STA_INFO_TX_FAILED: u16 = 12;
const STA_INFO_SIGNAL_AVG: u16 = 13;
const STA_INFO_RX_BITRATE: u16 = 14;
const STA_INFO_BSS_PARAM: u16 = 15;
const STA_INFO_CONNECTED_TIME: u16 = 16;
const STA_INFO_STA_FLAGS: u16 = 17;
const STA_INFO_BEACON_LOSS: u16 = 18;
const STA_INFO_RX_BYTES64: u16 = 23;
const STA_INFO_TX_BYTES64: u16 = 24;
const STA_INFO_CHAIN_SIGNAL: u16 = 25;
const STA_INFO_EXPECTED_THROUGHPUT: u16 = 27;
const STA_INFO_RX_DROP_MISC: u16 = 28;
const STA_INFO_BEACON_SIGNAL_AVG: u16 = 30;

const RATE_INFO_BITRATE: u16 = 1;
const RATE_INFO_MCS: u16 = 2;
const RATE_INFO_40_MHZ_WIDTH: u16 = 3;
const RATE_INFO_SHORT_GI: u16 = 4;
const RATE_INFO_BITRATE32: u16 = 5;
const RATE_INFO_VHT_MCS: u16 = 6;
const RATE_INFO_VHT_NSS: u16 = 7;
const RATE_INFO_80_MHZ_WIDTH: u16 = 8;
const RATE_INFO_80P80_MHZ_WIDTH: u16 = 9;
const RATE_INFO_160_MHZ_WIDTH: u16 = 10;
const RATE_INFO_HE_MCS: u16 = 13;
const RATE_INFO_HE_NSS: u16 = 14;
const RATE_INFO_HE_GI: u16 = 15;
const RATE_INFO_HE_DCM: u16 = 16;
const RATE_INFO_320_MHZ_WIDTH: u16 = 18;
const RATE_INFO_EHT_MCS: u16 = 19;
const RATE_INFO_EHT_NSS: u16 = 20;
const RATE_INFO_EHT_GI: u16 = 21;

const BSS_PARAM_SHORT_PREAMBLE: u16 = 2;
const BSS_PARAM_SHORT_SLOT_TIME: u16 = 3;
const BSS_PARAM_DTIM_PERIOD: u16 = 4;
const BSS_PARAM_BEACON_INTERVAL: u16 = 5;

const STA_FLAG_WME: u32 = 3;
const STA_FLAG_MFP: u32 = 4;

/// nl80211's channel widths, by their number, in MHz; 20 MHz with no HT
/// is the first.
const CHANNEL_WIDTHS: [(u32, u16); 7] = [
    (0, 20),
    (1, 20),
    (2, 40),
    (3, 80),
    (4, 160),
    (5, 160),
    (13, 320),
];
/// The guard intervals HE and EHT name by index, in microseconds, and the
/// two HT and VHT choose between.
const NEWER_GUARDS: [f64; 3] = [0.8, 1.6, 3.2];
const SHORT_GUARD: f64 = 0.4;
const LONG_GUARD: f64 = 0.8;
/// HT's index names each stream count by eight.
const HT_PER_STREAM: u32 = 8;
/// A rate's units, 100 kb/s.
const RATE_UNIT: u32 = 100;
/// Transmit power's units, hundredths of a dBm.
const POWER_UNIT: f64 = 100.0;

/// Each attribute in `bytes`, by its type and what it holds. One that
/// runs past the end ends the walk, rather than reading beyond it.
fn attributes(bytes: &[u8]) -> impl Iterator<Item = (u16, &[u8])> {
    let mut rest = bytes;
    std::iter::from_fn(move || {
        let length = usize::from(u16::from_ne_bytes([*rest.first()?, *rest.get(1)?]));
        let kind = u16::from_ne_bytes([*rest.get(2)?, *rest.get(3)?]) & NLA_TYPE;
        let value = rest.get(4..length)?;
        rest = rest.get(aligned(length)..).unwrap_or_default();
        Some((kind, value))
    })
}

/// A length taken to the next four bytes, as netlink lays each out.
fn aligned(length: usize) -> usize {
    length.next_multiple_of(4)
}

/// The one attribute of `kind` in `bytes`.
fn find(bytes: &[u8], kind: u16) -> Option<&[u8]> {
    attributes(bytes)
        .find(|(each, _)| *each == kind)
        .map(|(_, value)| value)
}

fn byte(value: &[u8]) -> Option<u8> {
    value.first().copied()
}

fn short(value: &[u8]) -> Option<u16> {
    Some(u16::from_ne_bytes(value.get(..2)?.try_into().ok()?))
}

fn word(value: &[u8]) -> Option<u32> {
    Some(u32::from_ne_bytes(value.get(..4)?.try_into().ok()?))
}

fn long(value: &[u8]) -> Option<u64> {
    Some(u64::from_ne_bytes(value.get(..8)?.try_into().ok()?))
}

/// A signal, which nl80211 says as a signed byte of dBm.
fn dbm(value: &[u8]) -> Option<i32> {
    byte(value).map(|said| i32::from(i8::from_ne_bytes([said])))
}

/// A station, from what a `GET_STATION` answer holds after its headers.
pub fn station(attributes: &[u8]) -> Option<Station> {
    let info = find(attributes, ATTR_STA_INFO)?;
    let figure = |kind, read: fn(&[u8]) -> Option<u32>| find(info, kind).and_then(read);
    let flags = find(info, STA_INFO_STA_FLAGS)
        .and_then(|flags| Some((word(flags)?, word(flags.get(4..)?)?)));
    // A flag the mask doesn't name is one the driver didn't say.
    let flag = |bit: u32| {
        flags.and_then(|(mask, set)| (mask & 1 << bit != 0).then_some(set & 1 << bit != 0))
    };
    let bss = find(info, STA_INFO_BSS_PARAM).unwrap_or_default();
    let traffic = |wide, narrow, packets| Traffic {
        bytes: find(info, wide)
            .and_then(long)
            .or_else(|| find(info, narrow).and_then(word).map(u64::from)),
        packets: figure(packets, word),
    };
    Some(Station {
        signal: find(info, STA_INFO_SIGNAL).and_then(dbm),
        average: find(info, STA_INFO_SIGNAL_AVG).and_then(dbm),
        beacons: find(info, STA_INFO_BEACON_SIGNAL_AVG).and_then(dbm),
        chains: find(info, STA_INFO_CHAIN_SIGNAL)
            .map(|chains| {
                self::attributes(chains)
                    .filter_map(|(_, each)| dbm(each))
                    .collect()
            })
            .unwrap_or_default(),
        receive: find(info, STA_INFO_RX_BITRATE)
            .map(rate)
            .unwrap_or_default(),
        send: find(info, STA_INFO_TX_BITRATE)
            .map(rate)
            .unwrap_or_default(),
        retries: figure(STA_INFO_TX_RETRIES, word),
        failed: figure(STA_INFO_TX_FAILED, word),
        missed: figure(STA_INFO_BEACON_LOSS, word),
        dropped: find(info, STA_INFO_RX_DROP_MISC).and_then(long),
        connected: figure(STA_INFO_CONNECTED_TIME, word),
        idle: figure(STA_INFO_INACTIVE_TIME, word),
        expected: figure(STA_INFO_EXPECTED_THROUGHPUT, word),
        protected: flag(STA_FLAG_MFP),
        wmm: flag(STA_FLAG_WME),
        interval: find(bss, BSS_PARAM_BEACON_INTERVAL).and_then(short),
        dtim: find(bss, BSS_PARAM_DTIM_PERIOD).and_then(byte),
        short_preamble: (!bss.is_empty()).then(|| find(bss, BSS_PARAM_SHORT_PREAMBLE).is_some()),
        short_slot: (!bss.is_empty()).then(|| find(bss, BSS_PARAM_SHORT_SLOT_TIME).is_some()),
        received: traffic(STA_INFO_RX_BYTES64, STA_INFO_RX_BYTES, STA_INFO_RX_PACKETS),
        sent: traffic(STA_INFO_TX_BYTES64, STA_INFO_TX_BYTES, STA_INFO_TX_PACKETS),
    })
}

/// One way's rate, from a station's nested rate attributes: the newest
/// coding it says, EHT's first, and HT's index, which says its streams.
fn rate(info: &[u8]) -> Rate {
    let figure = |kind| find(info, kind).and_then(byte).map(u32::from);
    let has = |kind| find(info, kind).is_some();
    let kbps = find(info, RATE_INFO_BITRATE32)
        .and_then(word)
        .or_else(|| find(info, RATE_INFO_BITRATE).and_then(short).map(u32::from))
        .map(|units| units.saturating_mul(RATE_UNIT));
    let width = [
        (RATE_INFO_320_MHZ_WIDTH, 320),
        (RATE_INFO_160_MHZ_WIDTH, 160),
        (RATE_INFO_80P80_MHZ_WIDTH, 160),
        (RATE_INFO_80_MHZ_WIDTH, 80),
        (RATE_INFO_40_MHZ_WIDTH, 40),
    ]
    .into_iter()
    .find(|(kind, _)| has(*kind))
    .map_or(20, |(_, width)| width);
    let newer = |index: Option<u32>| {
        index.and_then(|index| NEWER_GUARDS.get(usize::try_from(index).ok()?).copied())
    };
    let older = if has(RATE_INFO_SHORT_GI) {
        SHORT_GUARD
    } else {
        LONG_GUARD
    };
    let (generation, mcs, streams, guard, dcm) = if let Some(mcs) = figure(RATE_INFO_EHT_MCS) {
        (
            7,
            Some(mcs),
            figure(RATE_INFO_EHT_NSS),
            newer(figure(RATE_INFO_EHT_GI)),
            None,
        )
    } else if let Some(mcs) = figure(RATE_INFO_HE_MCS) {
        let dcm = figure(RATE_INFO_HE_DCM).map(|on| on != 0);
        (
            6,
            Some(mcs),
            figure(RATE_INFO_HE_NSS),
            newer(figure(RATE_INFO_HE_GI)),
            dcm,
        )
    } else if let Some(mcs) = figure(RATE_INFO_VHT_MCS) {
        (5, Some(mcs), figure(RATE_INFO_VHT_NSS), Some(older), None)
    } else if let Some(index) = figure(RATE_INFO_MCS) {
        let streams = Some(index / HT_PER_STREAM + 1);
        (4, Some(index % HT_PER_STREAM), streams, Some(older), None)
    } else {
        return Rate {
            kbps,
            width: kbps.map(|_| width),
            ..Rate::default()
        };
    };
    Rate {
        kbps,
        generation: Some(generation),
        mcs,
        streams,
        width: Some(width),
        guard,
        dcm,
    }
}

/// What a card's tuned to, from what a `GET_INTERFACE` answer holds after
/// its headers.
pub fn tuning(attributes: &[u8]) -> Tuning {
    let width = find(attributes, ATTR_CHANNEL_WIDTH).and_then(word);
    Tuning {
        frequency: find(attributes, ATTR_WIPHY_FREQ).and_then(word),
        width: width.and_then(|width| {
            CHANNEL_WIDTHS
                .iter()
                .find(|(each, _)| *each == width)
                .map(|(_, mhz)| *mhz)
        }),
        legacy: width == Some(0),
        power: find(attributes, ATTR_WIPHY_TX_POWER_LEVEL)
            .and_then(word)
            .map(|level| f64::from(level) / POWER_UNIT),
    }
}

/// A request: netlink's header, generic netlink's, and its attributes.
fn request(family: u16, flags: u16, sequence: u32, command: u8, attributes: &[u8]) -> Vec<u8> {
    let length = 16 + 4 + attributes.len();
    let mut message = Vec::with_capacity(length);
    message.extend(u32::try_from(length).unwrap_or(u32::MAX).to_ne_bytes());
    message.extend(family.to_ne_bytes());
    message.extend((NLM_F_REQUEST | NLM_F_ACK | flags).to_ne_bytes());
    message.extend(sequence.to_ne_bytes());
    message.extend(0u32.to_ne_bytes());
    message.extend([command, 1, 0, 0]);
    message.extend(attributes);
    message
}

/// One attribute, laid out with its padding.
fn attribute(kind: u16, value: &[u8]) -> Vec<u8> {
    let length = 4 + value.len();
    let mut said = Vec::with_capacity(aligned(length));
    said.extend(u16::try_from(length).unwrap_or(u16::MAX).to_ne_bytes());
    said.extend(kind.to_ne_bytes());
    said.extend(value);
    said.resize(aligned(length), 0);
    said
}

/// What an answer says: the attributes of each message of it, or the
/// kernel's refusal. An answer ends at netlink's done, or its
/// acknowledgement.
#[derive(Debug, PartialEq)]
enum Answered {
    Messages(Vec<Vec<u8>>),
    More(Vec<Vec<u8>>),
    Refused(i32),
}

/// The messages in one datagram of an answer to `sequence`: generic
/// netlink's attributes of each, with whether more are to come.
fn answered(datagram: &[u8], sequence: u32) -> Answered {
    let mut messages = Vec::new();
    let mut rest = datagram;
    while rest.len() >= 16 {
        let length = word(rest).map_or(0, |length| usize::try_from(length).unwrap_or(0));
        let (Some(message), Some(kind)) = (rest.get(..length), short(&rest[4..])) else {
            break;
        };
        if length < 16 {
            break;
        }
        let ours = word(&rest[8..]) == Some(sequence);
        if ours && kind == NLMSG_DONE {
            return Answered::Messages(messages);
        }
        if ours && kind == NLMSG_ERROR {
            let error = message.get(16..).and_then(word).map_or(0, u32::cast_signed);
            return if error == 0 {
                Answered::Messages(messages)
            } else {
                Answered::Refused(-error)
            };
        }
        if ours && let Some(attributes) = message.get(20..) {
            messages.push(attributes.to_vec());
        }
        rest = rest.get(aligned(length)..).unwrap_or_default();
    }
    Answered::More(messages)
}

/// A generic netlink socket with nl80211's family number, found once.
pub struct Nl80211 {
    socket: AsyncFd<OwnedFd>,
    family: u16,
    sequence: u32,
}

const AF_NETLINK: libc::c_int = 16;
const NETLINK_GENERIC: libc::c_int = 16;
/// Room for one datagram of an answer; the kernel fits a dump to it.
const DATAGRAM: usize = 32 * 1024;

impl Nl80211 {
    pub async fn open() -> io::Result<Nl80211> {
        // SAFETY: it takes only numbers.
        let fd = unsafe {
            libc::socket(
                AF_NETLINK,
                libc::SOCK_RAW | libc::SOCK_NONBLOCK | libc::SOCK_CLOEXEC,
                NETLINK_GENERIC,
            )
        };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: the descriptor was just made, and nothing else holds it.
        let owned = unsafe { OwnedFd::from_raw_fd(fd) };
        let mut nl80211 = Nl80211 {
            socket: AsyncFd::with_interest(owned, Interest::READABLE)?,
            family: GENL_ID_CTRL,
            sequence: 0,
        };
        let name = attribute(CTRL_ATTR_FAMILY_NAME, b"nl80211\0");
        let family = nl80211
            .ask(CTRL_CMD_GETFAMILY, 0, &name)
            .await?
            .iter()
            .find_map(|said| find(said, CTRL_ATTR_FAMILY_ID).and_then(short))
            .ok_or_else(|| io::Error::other("the kernel names no nl80211"))?;
        nl80211.family = family;
        Ok(nl80211)
    }

    /// The station the card named is joined to, if it's on one.
    pub async fn station(&mut self, card: &str) -> io::Result<Option<Station>> {
        let index = attribute(ATTR_IFINDEX, &index_of(card)?.to_ne_bytes());
        let said = self.ask(CMD_GET_STATION, NLM_F_DUMP, &index).await?;
        Ok(said.iter().find_map(|each| station(each)))
    }

    /// What the card named is tuned to.
    pub async fn tuning(&mut self, card: &str) -> io::Result<Tuning> {
        let index = attribute(ATTR_IFINDEX, &index_of(card)?.to_ne_bytes());
        let said = self.ask(CMD_GET_INTERFACE, 0, &index).await?;
        Ok(said.first().map(|each| tuning(each)).unwrap_or_default())
    }

    /// Sends a command and reads its whole answer.
    async fn ask(
        &mut self,
        command: u8,
        flags: u16,
        attributes: &[u8],
    ) -> io::Result<Vec<Vec<u8>>> {
        self.sequence = self.sequence.wrapping_add(1);
        let sequence = self.sequence;
        let message = request(self.family, flags, sequence, command, attributes);
        // SAFETY: the message outlives the call, and the length is its own.
        let sent = unsafe {
            libc::send(
                self.socket.as_raw_fd(),
                message.as_ptr().cast(),
                message.len(),
                0,
            )
        };
        if sent < 0 {
            return Err(io::Error::last_os_error());
        }
        let mut messages = Vec::new();
        let mut datagram = vec![0u8; DATAGRAM];
        loop {
            let mut ready = self.socket.readable().await?;
            let read = ready.try_io(|socket| {
                // SAFETY: the buffer is DATAGRAM long and outlives the call.
                let read = unsafe {
                    libc::recv(
                        socket.as_raw_fd(),
                        datagram.as_mut_ptr().cast(),
                        DATAGRAM,
                        0,
                    )
                };
                usize::try_from(read).map_err(|_| io::Error::last_os_error())
            });
            let Ok(read) = read else {
                continue;
            };
            match answered(&datagram[..read?], sequence) {
                Answered::Messages(more) => {
                    messages.extend(more);
                    return Ok(messages);
                }
                Answered::More(more) => messages.extend(more),
                Answered::Refused(error) => return Err(io::Error::from_raw_os_error(error)),
            }
        }
    }
}

/// A card's interface index, by its name.
fn index_of(card: &str) -> io::Result<u32> {
    let name = std::ffi::CString::new(card).map_err(io::Error::other)?;
    // SAFETY: the name is a C string that outlives the call.
    match unsafe { libc::if_nametoindex(name.as_ptr()) } {
        0 => Err(io::Error::last_os_error()),
        index => Ok(index),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nested(kind: u16, inside: &[Vec<u8>]) -> Vec<u8> {
        attribute(kind, &inside.concat())
    }

    fn flag(kind: u16) -> Vec<u8> {
        attribute(kind, &[])
    }

    /// A station as iwlwifi says one on a Wi-Fi 6 network: two antennas,
    /// HE both ways at 80 MHz, and the beacons' figures.
    fn joined() -> Vec<u8> {
        let send = nested(
            STA_INFO_TX_BITRATE,
            &[
                attribute(RATE_INFO_BITRATE32, &12_009u32.to_ne_bytes()),
                flag(RATE_INFO_80_MHZ_WIDTH),
                attribute(RATE_INFO_HE_MCS, &[11]),
                attribute(RATE_INFO_HE_NSS, &[2]),
                attribute(RATE_INFO_HE_GI, &[0]),
                attribute(RATE_INFO_HE_DCM, &[0]),
            ],
        );
        let receive = nested(
            STA_INFO_RX_BITRATE,
            &[
                attribute(RATE_INFO_BITRATE32, &8_647u32.to_ne_bytes()),
                flag(RATE_INFO_80_MHZ_WIDTH),
                attribute(RATE_INFO_VHT_MCS, &[9]),
                attribute(RATE_INFO_VHT_NSS, &[2]),
                flag(RATE_INFO_SHORT_GI),
            ],
        );
        let mask = 1u32 << STA_FLAG_WME | 1 << STA_FLAG_MFP;
        let flags = [mask.to_ne_bytes(), (1u32 << STA_FLAG_WME).to_ne_bytes()].concat();
        nested(
            ATTR_STA_INFO,
            &[
                attribute(STA_INFO_INACTIVE_TIME, &12u32.to_ne_bytes()),
                attribute(STA_INFO_RX_BYTES64, &987_654_321u64.to_ne_bytes()),
                attribute(STA_INFO_TX_BYTES, &1_234u32.to_ne_bytes()),
                attribute(STA_INFO_RX_PACKETS, &4_321u32.to_ne_bytes()),
                attribute(STA_INFO_SIGNAL, &[(-52i8).to_ne_bytes()[0]]),
                attribute(STA_INFO_SIGNAL_AVG, &[(-54i8).to_ne_bytes()[0]]),
                attribute(STA_INFO_BEACON_SIGNAL_AVG, &[(-50i8).to_ne_bytes()[0]]),
                nested(
                    STA_INFO_CHAIN_SIGNAL,
                    &[
                        attribute(0, &[(-53i8).to_ne_bytes()[0]]),
                        attribute(1, &[(-58i8).to_ne_bytes()[0]]),
                    ],
                ),
                send,
                receive,
                attribute(STA_INFO_TX_RETRIES, &17u32.to_ne_bytes()),
                attribute(STA_INFO_TX_FAILED, &2u32.to_ne_bytes()),
                attribute(STA_INFO_BEACON_LOSS, &1u32.to_ne_bytes()),
                attribute(STA_INFO_RX_DROP_MISC, &40u64.to_ne_bytes()),
                attribute(STA_INFO_CONNECTED_TIME, &3_600u32.to_ne_bytes()),
                attribute(STA_INFO_EXPECTED_THROUGHPUT, &600_000u32.to_ne_bytes()),
                attribute(STA_INFO_STA_FLAGS, &flags),
                nested(
                    STA_INFO_BSS_PARAM,
                    &[
                        flag(BSS_PARAM_SHORT_SLOT_TIME),
                        attribute(BSS_PARAM_DTIM_PERIOD, &[3]),
                        attribute(BSS_PARAM_BEACON_INTERVAL, &100u16.to_ne_bytes()),
                    ],
                ),
            ],
        )
    }

    #[test]
    fn a_station_says_its_signal_at_each_antenna_and_its_beacons() {
        let station = station(&joined()).expect("a station");
        assert_eq!(
            (
                station.signal,
                station.average,
                station.beacons,
                station.chains
            ),
            (Some(-52), Some(-54), Some(-50), vec![-53, -58])
        );
        assert_eq!(
            (
                station.interval,
                station.dtim,
                station.short_slot,
                station.short_preamble
            ),
            (Some(100), Some(3), Some(true), Some(false))
        );
    }

    #[test]
    fn a_station_says_what_went_wrong_and_for_how_long_it_has_been_on() {
        let station = station(&joined()).expect("a station");
        assert_eq!(
            (
                station.retries,
                station.failed,
                station.missed,
                station.dropped
            ),
            (Some(17), Some(2), Some(1), Some(40))
        );
        assert_eq!(
            (station.connected, station.idle, station.expected),
            (Some(3_600), Some(12), Some(600_000))
        );
    }

    #[test]
    fn a_flag_the_mask_leaves_out_is_unsaid_and_one_it_names_is_said() {
        let station = station(&joined()).expect("a station");
        assert_eq!((station.wmm, station.protected), (Some(true), Some(false)));
        let bare = nested(ATTR_STA_INFO, &[attribute(STA_INFO_SIGNAL, &[0xc0])]);
        let bare = super::station(&bare).expect("a station");
        assert_eq!((bare.wmm, bare.short_slot), (None, None));
    }

    #[test]
    fn traffic_takes_the_wide_count_and_falls_back_to_the_narrow_one() {
        let station = station(&joined()).expect("a station");
        assert_eq!(
            (station.received, station.sent),
            (
                Traffic {
                    bytes: Some(987_654_321),
                    packets: Some(4_321)
                },
                Traffic {
                    bytes: Some(1_234),
                    packets: None
                }
            )
        );
    }

    #[test]
    fn each_way_says_its_newest_coding() {
        let station = station(&joined()).expect("a station");
        assert_eq!(
            station.send,
            Rate {
                kbps: Some(1_200_900),
                generation: Some(6),
                mcs: Some(11),
                streams: Some(2),
                width: Some(80),
                guard: Some(0.8),
                dcm: Some(false),
            }
        );
        assert_eq!(
            station.receive,
            Rate {
                kbps: Some(864_700),
                generation: Some(5),
                mcs: Some(9),
                streams: Some(2),
                width: Some(80),
                guard: Some(0.4),
                dcm: None,
            }
        );
    }

    #[test]
    fn wifi_7_and_wifi_4_and_older_rates_are_each_coded_as_theirs() {
        let eht = rate(
            &[
                attribute(RATE_INFO_BITRATE32, &28_820u32.to_ne_bytes()),
                flag(RATE_INFO_320_MHZ_WIDTH),
                attribute(RATE_INFO_EHT_MCS, &[13]),
                attribute(RATE_INFO_EHT_NSS, &[2]),
                attribute(RATE_INFO_EHT_GI, &[1]),
            ]
            .concat(),
        );
        assert_eq!(
            (eht.generation, eht.mcs, eht.width, eht.guard),
            (Some(7), Some(13), Some(320), Some(1.6))
        );
        let ht = rate(
            &[
                attribute(RATE_INFO_BITRATE, &1_300u16.to_ne_bytes()),
                attribute(RATE_INFO_MCS, &[15]),
            ]
            .concat(),
        );
        assert_eq!(
            (
                ht.kbps,
                ht.generation,
                ht.mcs,
                ht.streams,
                ht.width,
                ht.guard
            ),
            (
                Some(130_000),
                Some(4),
                Some(7),
                Some(2),
                Some(20),
                Some(0.8)
            )
        );
        let legacy = rate(&attribute(RATE_INFO_BITRATE, &60u16.to_ne_bytes()));
        assert_eq!(
            legacy,
            Rate {
                kbps: Some(6_000),
                width: Some(20),
                ..Rate::default()
            }
        );
    }

    #[test]
    fn an_answer_with_no_station_is_no_station() {
        assert_eq!(station(&attribute(ATTR_IFINDEX, &3u32.to_ne_bytes())), None);
    }

    #[test]
    fn a_tuning_says_its_frequency_width_and_power() {
        let said = [
            attribute(ATTR_IFINDEX, &3u32.to_ne_bytes()),
            attribute(ATTR_WIPHY_FREQ, &5_180u32.to_ne_bytes()),
            attribute(ATTR_CHANNEL_WIDTH, &3u32.to_ne_bytes()),
            attribute(ATTR_WIPHY_TX_POWER_LEVEL, &2_200u32.to_ne_bytes()),
        ]
        .concat();
        assert_eq!(
            tuning(&said),
            Tuning {
                frequency: Some(5_180),
                width: Some(80),
                legacy: false,
                power: Some(22.0),
            }
        );
        let older = tuning(&attribute(ATTR_CHANNEL_WIDTH, &0u32.to_ne_bytes()));
        assert_eq!((older.width, older.legacy), (Some(20), true));
    }

    #[test]
    fn an_attribute_running_past_its_end_ends_the_walk() {
        let mut said = attribute(ATTR_WIPHY_FREQ, &5_180u32.to_ne_bytes());
        said.extend([40, 0, 3, 0, 1]);
        assert_eq!(attributes(&said).count(), 1);
    }

    /// A message of an answer, as the kernel sends it.
    fn message(kind: u16, sequence: u32, body: &[u8]) -> Vec<u8> {
        let mut said = request(kind, 0, sequence, CMD_GET_STATION, body);
        said[6..8].copy_from_slice(&0u16.to_ne_bytes());
        said
    }

    fn error(sequence: u32, code: i32) -> Vec<u8> {
        let mut said = message(NLMSG_ERROR, sequence, &[]);
        said.truncate(16);
        said.extend(code.to_ne_bytes());
        said.extend([0; 16]);
        let length = u32::try_from(said.len()).expect("a short message");
        said[..4].copy_from_slice(&length.to_ne_bytes());
        said
    }

    #[test]
    fn a_dump_goes_on_until_done_and_ends_there() {
        let body = attribute(ATTR_IFINDEX, &3u32.to_ne_bytes());
        let first = message(30, 7, &body);
        assert_eq!(answered(&first, 7), Answered::More(vec![body.clone()]));
        let done = [message(30, 7, &body), message(NLMSG_DONE, 7, &[0; 4])].concat();
        assert_eq!(answered(&done, 7), Answered::Messages(vec![body]));
    }

    #[test]
    fn an_acknowledgement_ends_an_answer_and_a_refusal_says_why() {
        let body = attribute(ATTR_IFINDEX, &3u32.to_ne_bytes());
        let acked = [message(30, 7, &body), error(7, 0)].concat();
        assert_eq!(answered(&acked, 7), Answered::Messages(vec![body]));
        assert_eq!(answered(&error(7, -19), 7), Answered::Refused(19));
    }

    #[test]
    fn what_answers_another_ask_is_passed_over() {
        let body = attribute(ATTR_IFINDEX, &3u32.to_ne_bytes());
        let said = [message(30, 6, &body), error(6, 0)].concat();
        assert_eq!(answered(&said, 7), Answered::More(vec![]));
    }
}
