//! The cable modem as a source: the status pages a Hitron modem serves at
//! the address every cable modem answers on, without signing in, each a
//! JSON list (contract/modem/ holds a real one's answers, redacted). The
//! modem is one device with the modem role; its channels and event log
//! are its modem block, and every page's answer stays under
//! `detail.modem`, page by page.
//!
//! A figure the pages don't give is `None`. They give a downstream
//! channel's modulation only as a code nothing here can read as a name,
//! and no page names the modem's maker or model, so those are kept in
//! `detail` as said and not named.

use std::collections::BTreeMap;

use serde_json::{Map, Value};

use crate::model::{
    Channel, Device, LinkState, LogEntry, Modem, Port, PortKind, Role, Source, SourceState,
};
use crate::time;

/// The modem's id as a device, and the source's name.
pub const ID: &str = "modem";

/// The address every cable modem answers on, from the home network.
pub const ADDRESS: &str = "192.168.100.1";

/// The pages read, each at `/data/<page>.asp`.
pub const PAGES: [&str; 9] = [
    "getSysInfo",
    "getCmDocsisWan",
    "getCMInit",
    "getLinkStatus",
    "dsinfo",
    "dsofdminfo",
    "usinfo",
    "usofdminfo",
    "status_log",
];

/// The form the modem's clock writes the time in, on its system page.
const CLOCK: &str = "%a %b %d, %Y, %H:%M:%S";

/// The form its event log writes each line's time in.
const LOG_CLOCK: &str = "%m/%d/%y %H:%M:%S";

/// Each page's answer, by its name; a page that didn't answer is missing.
pub type Pages = BTreeMap<String, Value>;

/// The modem as its pages say it, or why it isn't answering.
pub fn read(pages: Result<Pages, String>, now: u64) -> (Source, Option<Device>) {
    let what = format!("the cable modem's status pages at {ADDRESS}");
    let pages = pages.and_then(|pages| {
        if pages.contains_key("getSysInfo") {
            Ok(pages)
        } else {
            Err("its system page didn't answer".to_owned())
        }
    });
    match pages {
        Ok(pages) => (
            Source {
                name: ID.into(),
                what,
                state: SourceState::Answering,
                read_at: Some(now),
                since: None,
                reason: None,
            },
            Some(device(&pages, now)),
        ),
        Err(why) => (
            Source {
                name: ID.into(),
                what,
                state: SourceState::Unreachable,
                read_at: None,
                since: Some(now),
                reason: Some(why),
            },
            None,
        ),
    }
}

fn device(pages: &Pages, now: u64) -> Device {
    let system = first(pages, "getSysInfo");
    let link = first(pages, "getLinkStatus");
    let init = first(pages, "getCMInit");
    let cable_up = said(init, "trafficStatus").map(|traffic| traffic == "Enable");
    Device {
        id: ID.into(),
        name: "Modem".into(),
        roles: vec![Role::Modem],
        source: ID.into(),
        read_at: Some(now),
        vendor: None,
        model: None,
        hardware: said(system, "hwVersion").map(Into::into),
        firmware: said(system, "swVersion").map(Into::into),
        serial: said(system, "serialNumber").map(Into::into),
        macs: said(system, "rfMac")
            .map(str::to_lowercase)
            .into_iter()
            .collect(),
        addresses: vec![ADDRESS.into()],
        up_s: said(system, "systemUptime").and_then(uptime),
        ports: vec![
            Port {
                name: "cable".into(),
                kind: PortKind::Coax,
                link: cable_up.map(state),
                speed_mbps: None,
                duplex: None,
            },
            Port {
                name: "ethernet".into(),
                kind: PortKind::Ethernet,
                link: said(link, "LinkStatus").map(|link| state(link == "Up")),
                speed_mbps: said(link, "LinkSpeed")
                    .and_then(|speed| speed.trim_end_matches("Mbps").parse().ok()),
                duplex: said(link, "LinkDuplex").map(str::to_lowercase),
            },
        ],
        radios: Vec::new(),
        modem: Some(modem(pages, now)),
        detail: detail(pages),
    }
}

fn modem(pages: &Pages, now: u64) -> Modem {
    let wan = first(pages, "getCmDocsisWan");
    let said = |key| wan.and_then(|wan| text(wan, key));
    let offset = first(pages, "getSysInfo")
        .and_then(|system| text(system, "systemTime"))
        .and_then(|clock| time::civil(clock, CLOCK))
        .and_then(|clock| time::clock_offset(clock, now));
    let log: Vec<LogEntry> = rows(pages, "status_log")
        .map(|line| log_entry(line, offset))
        .collect();
    Modem {
        docsis: log
            .iter()
            .find_map(|line| tag(&line.message, "CM-VER"))
            .map(Into::into),
        network_access: said("NetworkAccess").map(str::to_lowercase),
        config_file: said("Configname").map(Into::into),
        address: said("CmIpAddress").map(|address| {
            said("CmNetMask").and_then(prefix).map_or_else(
                || address.to_owned(),
                |prefix| format!("{address}/{prefix}"),
            )
        }),
        downstream: rows(pages, "dsinfo")
            .map(single_carrier_down)
            .chain(rows(pages, "dsofdminfo").map(ofdm_down))
            .collect(),
        upstream: rows(pages, "usinfo")
            .map(single_carrier_up)
            .chain(rows(pages, "usofdminfo").map(ofdma_up))
            .collect(),
        log: log
            .into_iter()
            .map(|line| LogEntry {
                message: clean(&line.message),
                ..line
            })
            .collect(),
    }
}

/// A downstream single-carrier channel, as `dsinfo` lists it: each one
/// the modem lists is one it's locked to.
fn single_carrier_down(row: &Value) -> Channel {
    Channel {
        channel: whole(row, "channelId"),
        kind: "sc-qam".into(),
        frequency_hz: whole(row, "frequency"),
        width_hz: None,
        modulation: None,
        power_dbmv: real(row, "signalStrength"),
        snr_db: real(row, "snr"),
        corrected: whole(row, "correcteds"),
        uncorrectable: whole(row, "uncorrect"),
        active: true,
    }
}

/// A downstream OFDM channel, as `dsofdminfo` lists it, locked once its
/// PLC is.
fn ofdm_down(row: &Value) -> Channel {
    Channel {
        channel: None,
        kind: "ofdm".into(),
        frequency_hz: whole(row, "Subcarr0freqFreq"),
        width_hz: None,
        modulation: None,
        power_dbmv: real(row, "plcpower"),
        snr_db: real(row, "SNR"),
        corrected: whole(row, "correcteds"),
        uncorrectable: whole(row, "uncorrect"),
        active: text(row, "plclock").map(str::trim) == Some("YES"),
    }
}

fn single_carrier_up(row: &Value) -> Channel {
    Channel {
        channel: whole(row, "channelId"),
        kind: "atdma".into(),
        frequency_hz: whole(row, "frequency"),
        width_hz: whole(row, "bandwidth"),
        modulation: text(row, "modtype").map(str::to_lowercase),
        power_dbmv: real(row, "signalStrength"),
        snr_db: None,
        corrected: None,
        uncorrectable: None,
        active: true,
    }
}

/// An upstream OFDMA channel, as `usofdminfo` lists it, with figures only
/// while it's in use.
fn ofdma_up(row: &Value) -> Channel {
    let active = text(row, "state").is_some_and(|state| state.trim() != "DISABLED");
    let while_active = |key| real(row, key).filter(|_| active);
    Channel {
        channel: whole(row, "uschindex"),
        kind: "ofdma".into(),
        frequency_hz: whole(row, "frequency").filter(|hz| *hz > 0),
        width_hz: while_active("channelBw").and_then(|mhz| hertz(mhz * 1e6)),
        modulation: None,
        power_dbmv: while_active("repPower"),
        snr_db: None,
        corrected: None,
        uncorrectable: None,
        active,
    }
}

fn log_entry(row: &Value, offset: Option<i64>) -> LogEntry {
    let clock = text(row, "time").unwrap_or_default();
    LogEntry {
        at: offset
            .and_then(|offset| Some(time::civil(clock, LOG_CLOCK)? - offset))
            .and_then(|at| u64::try_from(at).ok()),
        clock: clock.into(),
        priority: text(row, "priority").unwrap_or("unknown").into(),
        code: text(row, "type").map(Into::into),
        message: text(row, "event").unwrap_or_default().into(),
    }
}

/// A `KEY=value` tag the modem adds to a log line.
fn tag<'a>(message: &'a str, key: &str) -> Option<&'a str> {
    message
        .split(';')
        .find_map(|part| part.strip_prefix(key)?.strip_prefix('='))
}

/// A log line's words without the tags every line repeats, which stay in
/// `detail`, and with its runs of spaces closed.
fn clean(message: &str) -> String {
    let words = message.split(";CM-MAC=").next().unwrap_or(message);
    words.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// `226h:33m:39s` as seconds.
fn uptime(said: &str) -> Option<u64> {
    let mut parts = said.split(':');
    let mut part = |unit| parts.next()?.trim().strip_suffix(unit)?.parse::<u64>().ok();
    Some(part('h')? * 3600 + part('m')? * 60 + part('s')?)
}

/// A dotted netmask's prefix length.
fn prefix(mask: &str) -> Option<u32> {
    let mask: std::net::Ipv4Addr = mask.parse().ok()?;
    let bits = u32::from(mask);
    (bits.leading_ones() + bits.trailing_zeros() == 32).then_some(bits.leading_ones())
}

fn state(up: bool) -> LinkState {
    if up { LinkState::Up } else { LinkState::Down }
}

/// Every page's answer, as said, under the source's name.
fn detail(pages: &Pages) -> Map<String, Value> {
    let said: Map<String, Value> = pages
        .iter()
        .map(|(page, answer)| (page.clone(), answer.clone()))
        .collect();
    let mut detail = Map::new();
    detail.insert(ID.into(), Value::Object(said));
    detail
}

/// A figure from a page's first row, when the page answered.
fn said<'a>(page: Option<&'a Value>, key: &str) -> Option<&'a str> {
    page.and_then(|page| text(page, key))
}

fn rows<'a>(pages: &'a Pages, page: &str) -> impl Iterator<Item = &'a Value> {
    pages
        .get(page)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
}

fn first<'a>(pages: &'a Pages, page: &str) -> Option<&'a Value> {
    rows(pages, page).next()
}

fn text<'a>(row: &'a Value, key: &str) -> Option<&'a str> {
    row.get(key).and_then(Value::as_str)
}

/// A whole number the modem writes as text, as `" 455600000"`.
fn whole<T: std::str::FromStr>(row: &Value, key: &str) -> Option<T> {
    text(row, key)?.trim().parse().ok()
}

fn real(row: &Value, key: &str) -> Option<f64> {
    whole::<f64>(row, key).filter(|figure| figure.is_finite())
}

fn hertz(hz: f64) -> Option<u64> {
    format!("{hz:.0}").parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uptime_reads_as_seconds() {
        assert_eq!(uptime("226h:33m:39s"), Some(815_619), "the uptime misread");
    }

    #[test]
    fn a_netmask_reads_as_its_prefix() {
        assert_eq!(prefix("255.255.240.0"), Some(20), "the netmask misread");
        assert_eq!(
            prefix("255.0.255.0"),
            None,
            "a broken netmask read as a prefix"
        );
    }

    #[test]
    fn a_log_line_loses_only_its_repeated_tags() {
        let said =
            "CM-STATUS message sent.  Event Type Code: 5;CM-MAC=02:00:00:00:10:01;CM-VER=3.1;";
        assert_eq!(
            clean(said),
            "CM-STATUS message sent. Event Type Code: 5",
            "the words changed"
        );
        assert_eq!(
            tag(said, "CM-VER"),
            Some("3.1"),
            "the DOCSIS version wasn't found"
        );
    }

    #[test]
    fn a_modem_that_didnt_answer_says_why() {
        let (source, device) = read(Err("timed out".into()), 100);
        assert_eq!(
            source.state,
            SourceState::Unreachable,
            "an unanswered modem read as answering"
        );
        assert_eq!(
            source.reason.as_deref(),
            Some("timed out"),
            "the reason was lost"
        );
        assert!(device.is_none(), "a device was made from nothing");
    }
}
