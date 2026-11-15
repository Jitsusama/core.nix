//! A TP-Link Easy Smart switch as a source: the four status pages its web
//! server shows once signed in (contract/switch/ holds a real one's,
//! redacted). Each page writes its figures as JavaScript arrays, which
//! `parse` reads into the switch's status. The reader that signs in runs
//! apart from net-daemon, since the switch's password lets its holder
//! change the switch; it hands net-daemon only the status, as a report,
//! and `read` turns the report into one device with the switch role.
//!
//! A figure the pages don't give is `None`, and the status stays whole
//! under `detail.switch`, so nothing the pages said is lost.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::model::{Device, LinkState, Port, PortKind, Role, Source, SourceState};

/// The source's name.
pub const NAME: &str = "switch";

/// What a report says it is, so a reader never takes another file for one.
pub const SCHEMA: &str = "net.switch/1";

/// Where the switch's reader writes its report, readable by any account,
/// since it holds only what the status pages show.
pub const REPORT: &str = "/run/net-switch/report.json";

/// The pages read, each at `/<page>.htm` once signed in.
pub const PAGES: [&str; 4] = [
    "SystemInfoRpm",
    "PortSettingRpm",
    "PortStatisticsRpm",
    "CableDiagRpm",
];

/// Each page's text, by its name.
pub type Pages = BTreeMap<String, String>;

/// What the reader hands net-daemon: how the switch answered, and its
/// status when it did. Never its password or its sign-in.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Report {
    pub schema: String,
    pub read_at: u64,
    pub state: SourceState,
    pub reason: Option<String>,
    pub status: Option<Status>,
}

/// The switch as its pages say it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Status {
    pub description: Option<String>,
    pub mac: Option<String>,
    pub address: Option<String>,
    pub netmask: Option<String>,
    pub gateway: Option<String>,
    pub firmware: Option<String>,
    pub hardware: Option<String>,
    pub ports: Vec<SwitchPort>,
}

/// One port, in the switch's own terms.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SwitchPort {
    pub port: u8,
    pub enabled: Option<bool>,
    /// The speed it's set to, as the switch words it (`Auto`, `100MF`).
    pub speed_set: Option<String>,
    /// The speed it links at, as the switch words it (`Link Down`, `1000Full`).
    pub link: Option<String>,
    pub speed_mbps: Option<u32>,
    pub duplex: Option<String>,
    pub flow_control_set: Option<bool>,
    pub flow_control: Option<bool>,
    /// The link aggregation group it's in, if any.
    pub lag: Option<u8>,
    pub tx_good: Option<u64>,
    pub tx_bad: Option<u64>,
    pub rx_good: Option<u64>,
    pub rx_bad: Option<u64>,
    /// The last cable test, if one has run since the switch started.
    pub cable: Option<Cable>,
}

/// A cable test's answer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Cable {
    /// As the switch words it: `Normal`, `No Cable`, `Open`, `Short`,
    /// `Open & Short`, `Cross Cable` or `Others`.
    pub state: String,
    /// How far along the cable the test found its end or its fault.
    pub length_m: Option<u32>,
}

/// What one of the switch's speed codes says: its word on the settings
/// page, its word on the statistics page, and what it means as a speed
/// and a duplex, when it's a speed at all.
#[derive(Clone, Copy)]
struct Speed {
    set: &'static str,
    linked: &'static str,
    means: Option<(u32, &'static str)>,
}

const fn speed_code(
    set: &'static str,
    linked: &'static str,
    means: Option<(u32, &'static str)>,
) -> Speed {
    Speed { set, linked, means }
}

/// The switch's speed codes, by their number.
const SPEEDS: [Speed; 7] = [
    speed_code("Link Down", "Link Down", None),
    speed_code("Auto", "Auto", None),
    speed_code("10MH", "10Half", Some((10, "half"))),
    speed_code("10MF", "10Full", Some((10, "full"))),
    speed_code("100MH", "100Half", Some((100, "half"))),
    speed_code("100MF", "100Full", Some((100, "full"))),
    speed_code("1000MF", "1000Full", Some((1000, "full"))),
];

/// What a cable test's code means, as the cable page words it.
fn cable_state(code: i64) -> &'static str {
    match code {
        0 => "No Cable",
        1 => "Normal",
        2 => "Open",
        3 => "Short",
        4 => "Open & Short",
        5 => "Cross Cable",
        _ => "Others",
    }
}

/// The switch's status from its pages, or why they don't read as one. A
/// page that comes back as the sign-in form, as it does once the switch's
/// session has ended, holds none of its arrays and is refused.
pub fn parse(pages: &Pages) -> Result<Status, String> {
    let page = |name: &str| {
        pages
            .get(name)
            .map(String::as_str)
            .ok_or_else(|| format!("its {name} page is missing"))
    };
    let system = page("SystemInfoRpm")?;
    let setting = page("PortSettingRpm")?;
    let statistics = page("PortStatisticsRpm")?;
    let cables = page("CableDiagRpm")?;
    let refused = |name: &str| format!("its {name} page holds no status, as if signed out");
    let first = |key| array(system, key).and_then(|said| said.into_iter().next());
    if first("macStr").is_none() {
        return Err(refused("SystemInfoRpm"));
    }
    let state = array(setting, "state").ok_or_else(|| refused("PortSettingRpm"))?;
    let link_status =
        array(statistics, "link_status").ok_or_else(|| refused("PortStatisticsRpm"))?;
    let count = value(setting, "max_port_num")
        .and_then(|count| count.parse::<u8>().ok())
        .unwrap_or(8);
    let numbers = |page, key| {
        array(page, key)
            .unwrap_or_default()
            .into_iter()
            .map(|said| said.parse::<i64>().ok())
            .collect::<Vec<_>>()
    };
    let (state, link_status) = (whole(&state), whole(&link_status));
    let (speed_set, trunk) = (numbers(setting, "spd_cfg"), numbers(setting, "trunk_info"));
    let (flow_set, flow) = (numbers(setting, "fc_cfg"), numbers(setting, "fc_act"));
    let packets = numbers(statistics, "pkts");
    let (cable_state_codes, cable_lengths) = (
        numbers(cables, "cablestate"),
        numbers(cables, "cablelength"),
    );
    let at = |list: &[Option<i64>], index: usize| list.get(index).copied().flatten();
    let ports = (0..usize::from(count))
        .map(|index| {
            let linked = at(&link_status, index).and_then(speed);
            let packet = |offset| {
                at(&packets, 4 * index + offset).and_then(|count| u64::try_from(count).ok())
            };
            SwitchPort {
                port: u8::try_from(index + 1).unwrap_or(u8::MAX),
                enabled: at(&state, index).map(|state| state == 1),
                speed_set: at(&speed_set, index)
                    .and_then(speed)
                    .map(|speed| speed.set.into()),
                link: linked.map(|speed| speed.linked.into()),
                speed_mbps: linked.and_then(|speed| speed.means).map(|(mbps, _)| mbps),
                duplex: linked
                    .and_then(|speed| speed.means)
                    .map(|(_, duplex)| duplex.into()),
                flow_control_set: at(&flow_set, index).map(|on| on == 1),
                flow_control: at(&flow, index).map(|on| on == 1),
                lag: at(&trunk, index)
                    .filter(|group| *group > 0)
                    .and_then(|group| u8::try_from(group).ok()),
                tx_good: packet(0),
                tx_bad: packet(1),
                rx_good: packet(2),
                rx_bad: packet(3),
                cable: at(&cable_state_codes, index)
                    .filter(|code| *code >= 0)
                    .map(|code| Cable {
                        state: cable_state(code).into(),
                        length_m: at(&cable_lengths, index)
                            .and_then(|length| u32::try_from(length).ok()),
                    }),
            }
        })
        .collect();
    Ok(Status {
        description: first("descriStr"),
        mac: first("macStr"),
        address: first("ipStr"),
        netmask: first("netmaskStr"),
        gateway: first("gatewayStr"),
        firmware: first("firmwareStr"),
        hardware: first("hardwareStr"),
        ports,
    })
}

/// The page's words for a speed code, and what it means.
fn speed(code: i64) -> Option<Speed> {
    usize::try_from(code)
        .ok()
        .and_then(|code| SPEEDS.get(code).copied())
}

fn whole(said: &[String]) -> Vec<Option<i64>> {
    said.iter().map(|said| said.parse().ok()).collect()
}

/// The items of the array a page's script names `key`, as in `state:[1,0]`
/// or `var cablestate=[1,0]`, each unquoted: the pages write their
/// figures only so.
fn array(page: &str, key: &str) -> Option<Vec<String>> {
    let start = named(page, key, '[')?;
    let rest = &page[start..];
    let end = rest.find(']')?;
    Some(
        rest[..end]
            .split(',')
            .map(|item| item.trim().trim_matches('"').to_owned())
            .filter(|item| !item.is_empty())
            .collect(),
    )
}

/// The bare value a page's script gives `key`, as in `var max_port_num = 8`.
fn value(page: &str, key: &str) -> Option<String> {
    let mut at = 0;
    while let Some(found) = page[at..].find(key) {
        let start = at + found;
        at = start + key.len();
        if !standalone(page, start) {
            continue;
        }
        let rest = page[at..].trim_start();
        let Some(rest) = rest.strip_prefix('=').or_else(|| rest.strip_prefix(':')) else {
            continue;
        };
        let said: String = rest
            .trim_start()
            .chars()
            .take_while(char::is_ascii_alphanumeric)
            .collect();
        return (!said.is_empty()).then_some(said);
    }
    None
}

/// Where what `key` holds begins, just past its opening mark, when the
/// page's script gives `key` one: `key:<open>` or `key=<open>`.
fn named(page: &str, key: &str, open: char) -> Option<usize> {
    let mut at = 0;
    while let Some(found) = page[at..].find(key) {
        let start = at + found;
        at = start + key.len();
        if !standalone(page, start) {
            continue;
        }
        let rest = page[at..].trim_start();
        let Some(rest) = rest.strip_prefix(':').or_else(|| rest.strip_prefix('=')) else {
            continue;
        };
        let rest = rest.trim_start();
        if rest.starts_with(open) {
            return Some(page.len() - rest.len() + open.len_utf8());
        }
    }
    None
}

/// Whether the name found at `start` is a whole name, not the end of a
/// longer one, as `state` is in `cablestate`.
fn standalone(page: &str, start: usize) -> bool {
    page[..start]
        .chars()
        .next_back()
        .is_none_or(|before| !(before.is_ascii_alphanumeric() || before == '_'))
}

/// The switch as its reader reported it, or why it isn't there.
pub fn read(report: Result<Report, String>, now: u64) -> (Source, Option<Device>) {
    let what = "the switch's status pages, read by its own reader".to_owned();
    let report = report.and_then(|report| {
        if report.schema == SCHEMA {
            Ok(report)
        } else {
            Err(format!(
                "its reader's report is {}, not {SCHEMA}",
                report.schema
            ))
        }
    });
    match report {
        Ok(Report {
            read_at,
            state: SourceState::Answering,
            status: Some(status),
            ..
        }) => (
            Source {
                name: NAME.into(),
                what,
                state: SourceState::Answering,
                read_at: Some(read_at),
                since: None,
                reason: None,
            },
            Some(device(&status, read_at)),
        ),
        Ok(report) => (
            Source {
                name: NAME.into(),
                what,
                state: if report.state == SourceState::Answering {
                    SourceState::Unreachable
                } else {
                    report.state
                },
                read_at: None,
                since: Some(report.read_at),
                reason: report
                    .reason
                    .or_else(|| Some("its reader gave no status".into())),
            },
            None,
        ),
        Err(why) => (
            Source {
                name: NAME.into(),
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

fn device(status: &Status, read_at: u64) -> Device {
    let mac = status.mac.as_deref().map(str::to_lowercase);
    // The hardware is written as the model, then its version: `TL-SG108E 3.0`.
    let (model, hardware) = match status.hardware.as_deref().map(|said| said.split_once(' ')) {
        Some(Some((model, version))) => (Some(model.to_owned()), Some(version.to_owned())),
        _ => (status.hardware.clone(), None),
    };
    let ports = status
        .ports
        .iter()
        .map(|port| Port {
            name: port.port.to_string(),
            kind: PortKind::Ethernet,
            link: port.link.as_deref().map(|link| {
                if link == "Link Down" {
                    LinkState::Down
                } else {
                    LinkState::Up
                }
            }),
            speed_mbps: port.speed_mbps,
            duplex: port.duplex.clone(),
        })
        .collect();
    let mut detail = Map::new();
    detail.insert(
        NAME.into(),
        serde_json::to_value(status).unwrap_or(Value::Null),
    );
    Device {
        id: mac
            .as_ref()
            .map_or_else(|| NAME.to_owned(), |mac| format!("mac:{mac}")),
        name: status
            .description
            .clone()
            .unwrap_or_else(|| "Switch".into()),
        roles: vec![Role::Switch],
        source: NAME.into(),
        read_at: Some(read_at),
        // The source reads only TP-Link's Easy Smart switches.
        vendor: Some("TP-Link".into()),
        model,
        hardware,
        firmware: status.firmware.clone(),
        serial: None,
        macs: mac.into_iter().collect(),
        addresses: status.address.clone().into_iter().collect(),
        up_s: None,
        ports,
        radios: Vec::new(),
        modem: None,
        path: None,
        detail,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::fs;

    fn contract() -> Pages {
        PAGES
            .iter()
            .map(|page| {
                let file = format!("{}/contract/switch/{page}.htm", env!("CARGO_MANIFEST_DIR"));
                (
                    (*page).to_string(),
                    fs::read_to_string(&file).expect("the contract has each page"),
                )
            })
            .collect()
    }

    fn answering(status: Status) -> Report {
        Report {
            schema: SCHEMA.into(),
            read_at: 1_794_700_000,
            state: SourceState::Answering,
            reason: None,
            status: Some(status),
        }
    }

    #[test]
    fn its_system_page_names_it() {
        let status = parse(&contract()).expect("the contract reads");
        assert_eq!(status.description.as_deref(), Some("Closet"));
        assert_eq!(status.mac.as_deref(), Some("02:00:00:00:00:30"));
        assert_eq!(status.address.as_deref(), Some("10.0.0.30"));
        assert_eq!(status.hardware.as_deref(), Some("TL-SG108E 3.0"));
        assert_eq!(
            status.firmware.as_deref(),
            Some("1.0.0 Build 20170717 Rel.42042(Beta)")
        );
    }

    #[test]
    fn it_has_as_many_ports_as_it_says() {
        let status = parse(&contract()).expect("the contract reads");
        let ports: Vec<u8> = status.ports.iter().map(|port| port.port).collect();
        assert_eq!(ports, (1..=8).collect::<Vec<u8>>());
    }

    #[test]
    fn a_linked_port_says_its_speed_and_its_packets() {
        let status = parse(&contract()).expect("the contract reads");
        let eight = &status.ports[7];
        assert_eq!(eight.link.as_deref(), Some("1000Full"));
        assert_eq!(eight.speed_mbps, Some(1000));
        assert_eq!(eight.duplex.as_deref(), Some("full"));
        assert_eq!(eight.speed_set.as_deref(), Some("Auto"));
        assert_eq!(eight.enabled, Some(true));
        assert_eq!(eight.flow_control, Some(false));
        assert_eq!(eight.lag, None);
        assert_eq!(
            (eight.tx_good, eight.tx_bad, eight.rx_good, eight.rx_bad),
            (Some(6_150_636), Some(0), Some(9_617_421), Some(0))
        );
    }

    #[test]
    fn a_port_with_no_link_says_so_and_keeps_its_old_packets() {
        let status = parse(&contract()).expect("the contract reads");
        let two = &status.ports[1];
        assert_eq!(two.link.as_deref(), Some("Link Down"));
        assert_eq!((two.speed_mbps, two.duplex.as_deref()), (None, None));
        assert_eq!(two.tx_good, Some(8_066_519));
    }

    #[test]
    fn an_untested_cable_says_nothing_and_a_tested_one_its_fault() {
        let status = parse(&contract()).expect("the contract reads");
        assert!(
            status.ports.iter().all(|port| port.cable.is_none()),
            "no cable test had run, yet a port says one: {:?}",
            status.ports
        );
        let mut pages = contract();
        pages.insert(
            "CableDiagRpm".into(),
            "var cablestate=[1,0,2,-1,-1,-1,-1,-1];\nvar cablelength=[12,-1,7,-1,-1,-1,-1,-1];"
                .into(),
        );
        let status = parse(&pages).expect("it reads");
        let cable = |port: usize| status.ports[port].cable.clone();
        assert_eq!(
            cable(0),
            Some(Cable {
                state: "Normal".into(),
                length_m: Some(12)
            })
        );
        assert_eq!(
            cable(1),
            Some(Cable {
                state: "No Cable".into(),
                length_m: None
            })
        );
        assert_eq!(cable(2).map(|cable| cable.state), Some("Open".into()));
        assert_eq!(cable(3), None);
    }

    #[test]
    fn a_sign_in_page_in_place_of_a_status_page_is_refused() {
        let mut pages = contract();
        pages.insert(
            "PortSettingRpm".into(),
            "<html><form action=logon.cgi></form></html>".into(),
        );
        assert!(parse(&pages).is_err(), "a sign-in page read as status");
    }

    #[test]
    fn the_report_reads_into_one_switch() {
        let status = parse(&contract()).expect("the contract reads");
        let (source, device) = read(Ok(answering(status)), 1_794_700_005);
        assert_eq!(source.state, SourceState::Answering);
        let device = device.expect("a switch");
        assert_eq!(device.id, "mac:02:00:00:00:00:30");
        assert_eq!(device.name, "Closet");
        assert_eq!(device.roles, vec![Role::Switch]);
        assert_eq!(device.vendor.as_deref(), Some("TP-Link"));
        assert_eq!(device.model.as_deref(), Some("TL-SG108E"));
        assert_eq!(device.hardware.as_deref(), Some("3.0"));
        assert_eq!(device.addresses, vec!["10.0.0.30".to_string()]);
        let eight = &device.ports[7];
        assert_eq!(
            (eight.name.as_str(), eight.link, eight.speed_mbps),
            ("8", Some(LinkState::Up), Some(1000))
        );
        assert_eq!(device.ports[1].link, Some(LinkState::Down));
        assert!(device.detail.get(NAME).is_some(), "the status stays whole");
    }

    #[test]
    fn a_signed_out_report_says_why_and_gives_no_switch() {
        let report = Report {
            schema: SCHEMA.into(),
            read_at: 1_794_700_000,
            state: SourceState::SignedOut,
            reason: Some("no password is set for it".into()),
            status: None,
        };
        let (source, device) = read(Ok(report), 1_794_700_005);
        assert_eq!(source.state, SourceState::SignedOut);
        assert_eq!(source.reason.as_deref(), Some("no password is set for it"));
        assert!(
            device.is_none(),
            "a signed-out switch gave a device: {device:?}"
        );
    }

    #[test]
    fn no_report_reads_as_unreachable() {
        let (source, device) = read(Err("its reader's report isn't there".into()), 10);
        assert_eq!(source.state, SourceState::Unreachable);
        assert_eq!(source.since, Some(10));
        assert!(device.is_none(), "no report gave a device: {device:?}");
    }
}
