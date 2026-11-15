//! `net devices NAME`: one device in layers. What every device can say,
//! its ports, radios and links, a role's own figures (a modem's channels
//! and log), then what each source says that none of those held, in the
//! source's own words.

use serde_json::Value;
use terminal_kit::layout::{self, Align, ITEMS, Room};
use terminal_kit::{Pen, time};

use super::{ago, roles_of, touching};
use crate::model::{Channel, Device, End, LinkState, LogEntry, Network};

/// `net devices NAME`: what every device can say, then its role's own
/// figures, then what each source says that those didn't.
pub fn device(network: &Network, device: &Device, now: u64, pen: &Pen, room: Room) -> String {
    let mut blocks = vec![
        vec![format!(
            "{}{}",
            pen.title(&device.name),
            pen.role(
                "muted",
                &format!("{ITEMS}{}", roles_of(device).replace(", ", ITEMS))
            )
        )],
        generic(device, now, pen, room),
        ports(device, pen, room),
        radios(device, pen, room),
        links(network, device, pen, room),
    ];
    if let Some(modem) = &device.modem {
        let facts = [
            ("docsis", modem.docsis.as_deref()),
            ("network access", modem.network_access.as_deref()),
            ("config file", modem.config_file.as_deref()),
            ("provider's address", modem.address.as_deref()),
        ];
        let mut block = vec![pen.label("modem")];
        block.extend(known_facts(&facts, pen, room));
        blocks.push(block);
        blocks.push(channels("down", &modem.downstream, "locked", pen, room));
        blocks.push(channels("up", &modem.upstream, "in use", pen, room));
        blocks.push(log(&modem.log, pen, room));
    }
    let shown = shown(device);
    for (source, said) in &device.detail {
        let mut fields = Vec::new();
        match said {
            Value::Object(said) => {
                for (key, value) in said {
                    if !SAID.contains(&key.as_str()) {
                        flatten(&key.replace('_', " "), value, &mut fields);
                    }
                }
            }
            said => flatten("", said, &mut fields),
        }
        fields.retain(|(_, value)| !shown.contains(value));
        if fields.is_empty() {
            continue;
        }
        let facts: Vec<(&str, Option<&str>)> = fields
            .iter()
            .map(|(label, value)| (label.as_str(), Some(value.as_str())))
            .collect();
        let mut block = vec![pen.label(&format!("from {source}"))];
        block.extend(layout::facts(&facts, "unknown", pen, room));
        blocks.push(block);
    }
    layout::joined(blocks)
}

/// Facts with a value, each beside its label; a figure a source doesn't
/// give is left out rather than said as unknown down a whole page.
fn known_facts(facts: &[(&str, Option<&str>)], pen: &Pen, room: Room) -> Vec<String> {
    let known: Vec<(&str, Option<&str>)> = facts
        .iter()
        .filter(|(_, value)| value.is_some())
        .copied()
        .collect();
    layout::facts(&known, "unknown", pen, room)
}

/// What every device can say, and where it came from.
fn generic(device: &Device, now: u64, pen: &Pen, room: Room) -> Vec<String> {
    let macs = device.macs.join(", ");
    let addresses = device.addresses.join(", ");
    let up = device.up_s.map(time::span);
    let read = match device.read_at {
        Some(read) => format!("from {}, {}", device.source, ago(now, read)),
        None => format!("never, from {}", device.source),
    };
    let facts = [
        ("vendor", device.vendor.as_deref()),
        ("model", device.model.as_deref()),
        ("hardware", device.hardware.as_deref()),
        ("firmware", device.firmware.as_deref()),
        ("serial", device.serial.as_deref()),
        ("mac", (!macs.is_empty()).then_some(macs.as_str())),
        (
            "address",
            (!addresses.is_empty()).then_some(addresses.as_str()),
        ),
        ("up", up.as_deref()),
        ("read", Some(read.as_str())),
    ];
    known_facts(&facts, pen, room)
}

fn figure<T: ToString>(value: Option<T>) -> String {
    value.map(|value| value.to_string()).unwrap_or_default()
}

const PORTS: [(&str, Align); 5] = [
    ("port", Align::Left),
    ("kind", Align::Left),
    ("link", Align::Left),
    ("speed mbit/s", Align::Right),
    ("duplex", Align::Left),
];

fn ports(device: &Device, pen: &Pen, room: Room) -> Vec<String> {
    if device.ports.is_empty() {
        return Vec::new();
    }
    let rows: Vec<Vec<String>> = device
        .ports
        .iter()
        .map(|port| {
            vec![
                port.name.clone(),
                to_words(&port.kind),
                port.link
                    .map(|link| match link {
                        LinkState::Up => "up",
                        LinkState::Down => "down",
                    })
                    .unwrap_or_default()
                    .to_owned(),
                figure(port.speed_mbps),
                port.duplex.clone().unwrap_or_default(),
            ]
        })
        .collect();
    layout::table_in(&PORTS, &rows, pen, room, &[4, 3])
}

const RADIOS: [(&str, Align); 6] = [
    ("radio", Align::Left),
    ("band", Align::Left),
    ("channel", Align::Right),
    ("width mhz", Align::Right),
    ("busy %", Align::Right),
    ("power dbm", Align::Right),
];

fn radios(device: &Device, pen: &Pen, room: Room) -> Vec<String> {
    if device.radios.is_empty() {
        return Vec::new();
    }
    let rows: Vec<Vec<String>> = device
        .radios
        .iter()
        .map(|radio| {
            vec![
                radio.name.clone(),
                radio.band.clone().unwrap_or_default(),
                figure(radio.channel),
                figure(radio.width_mhz),
                figure(radio.busy_pct),
                figure(radio.power_dbm),
            ]
        })
        .collect();
    layout::table_in(&RADIOS, &rows, pen, room, &[5, 1, 3])
}

/// A serialized enum's words: `access-point` as `access point`.
fn to_words<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(|words| words.replace('-', " ")))
        .unwrap_or_default()
}

/// Where an end sits on its device: its port or its radio.
fn at(end: &End) -> Option<String> {
    end.port
        .as_ref()
        .map(|port| format!("port {port}"))
        .or_else(|| end.radio.clone())
}

/// The links' columns, and the order they give way in: where the link
/// sits on the other device, then on this one, then its channel. The
/// other device, the medium and the speed always stay.
const LINKS: [(&str, Align); 6] = [
    ("linked to", Align::Left),
    ("medium", Align::Left),
    ("channel", Align::Right),
    ("here", Align::Left),
    ("there", Align::Left),
    ("mbit/s", Align::Right),
];
const LINKS_GIVE_WAY: [usize; 3] = [4, 3, 2];

/// The devices this one links to, each a line of a table: the other
/// device, how, where on each, and at what speed.
fn links(network: &Network, device: &Device, pen: &Pen, room: Room) -> Vec<String> {
    let named = |id: &str| {
        network
            .devices
            .iter()
            .find(|other| other.id == id)
            .map_or_else(|| id.to_owned(), |other| other.name.clone())
    };
    let rows: Vec<Vec<String>> = touching(network, device)
        .map(|link| {
            let (here, there) = if link.a.device == device.id {
                (&link.a, &link.b)
            } else {
                (&link.b, &link.a)
            };
            vec![
                named(&there.device),
                to_words(&link.medium),
                figure(link.channel),
                at(here).unwrap_or_default(),
                at(there).unwrap_or_default(),
                figure(link.speed_mbps),
            ]
        })
        .collect();
    if rows.is_empty() {
        return Vec::new();
    }
    layout::table_in(&LINKS, &rows, pen, room, &LINKS_GIVE_WAY)
}

/// A cable line's channels one way, as a table headed by the way.
fn channels(way: &str, channels: &[Channel], active: &str, pen: &Pen, room: Room) -> Vec<String> {
    if channels.is_empty() {
        return Vec::new();
    }
    let columns = [
        (way, Align::Right),
        ("kind", Align::Left),
        ("mhz", Align::Right),
        ("width mhz", Align::Right),
        ("modulation", Align::Left),
        ("power dbmv", Align::Right),
        ("snr db", Align::Right),
        ("corrected", Align::Right),
        ("uncorrectable", Align::Right),
        (active, Align::Left),
    ];
    let megahertz = |hertz: Option<u64>| {
        #[expect(
            clippy::cast_precision_loss,
            reason = "a cable line's hertz are far under f64's whole numbers"
        )]
        hertz.map(|hertz| format!("{:.1}", hertz as f64 / 1e6))
    };
    let tenths = |value: Option<f64>| value.map(|value| format!("{value:.1}"));
    let rows: Vec<Vec<String>> = channels
        .iter()
        .map(|channel| {
            vec![
                figure(channel.channel),
                channel.kind.clone(),
                megahertz(channel.frequency_hz).unwrap_or_default(),
                megahertz(channel.width_hz).unwrap_or_default(),
                channel.modulation.clone().unwrap_or_default(),
                tenths(channel.power_dbmv).unwrap_or_default(),
                tenths(channel.snr_db).unwrap_or_default(),
                figure(channel.corrected),
                figure(channel.uncorrectable),
                if channel.active { "yes" } else { "no" }.to_owned(),
            ]
        })
        .collect();
    layout::table_in(&columns, &rows, pen, room, &[3, 7, 4, 2, 9])
}

/// The modem's event log, newest first, its message cut last.
fn log(log: &[LogEntry], pen: &Pen, room: Room) -> Vec<String> {
    if log.is_empty() {
        return Vec::new();
    }
    let columns = [
        ("logged", Align::Left),
        ("priority", Align::Left),
        ("message", Align::Left),
    ];
    let rows: Vec<Vec<String>> = log
        .iter()
        .map(|entry| {
            vec![
                entry.clock.clone(),
                entry.priority.clone(),
                entry.message.clone(),
            ]
        })
        .collect();
    layout::table_in(&columns, &rows, pen, room, &[])
}

/// Every value the page already shows, so a source's section leaves it
/// out: the device's name, its id's own part, and its generic fields.
fn shown(device: &Device) -> Vec<String> {
    let mut shown = vec![
        device.name.clone(),
        device.id.rsplit(':').next().unwrap_or_default().to_owned(),
    ];
    shown.extend(
        [
            &device.vendor,
            &device.model,
            &device.hardware,
            &device.firmware,
            &device.serial,
        ]
        .into_iter()
        .flatten()
        .cloned(),
    );
    shown.extend(device.macs.iter().cloned());
    shown.extend(device.addresses.iter().cloned());
    shown
}

/// A source's own fields that the page's tables and facts already say in
/// net's words, so its section leaves them out by name; the rest go by
/// value, through `shown`.
const SAID: [&str; 10] = [
    "id", "location", "model", "firmware", "serial", "mac", "address", "up_s", "ports", "radios",
];

/// The most items a list may hold and still be a fact on one line; a
/// longer one is a series, such as a history of readings, which only
/// `--json` gives whole.
const LIST_AT_MOST: usize = 4;

/// A source's fields as labels and words, a nested field labelled by the
/// path to it: `backhaul upstream`. What a fact can't say on a line, a
/// list of records or a series, and what the source left empty, are left
/// to `--json`.
fn flatten(path: &str, value: &Value, out: &mut Vec<(String, String)>) {
    let label = |key: &str| {
        let key = key.replace('_', " ");
        if path.is_empty() {
            key
        } else {
            format!("{path} {key}")
        }
    };
    match value {
        Value::Object(fields) => {
            for (key, value) in fields {
                flatten(&label(key), value, out);
            }
        }
        Value::Array(items) => {
            let records = items.iter().any(|item| item.is_object() || item.is_array());
            if !items.is_empty() && items.len() <= LIST_AT_MOST && !records {
                let words: Vec<String> = items.iter().map(scalar).collect();
                out.push((path.to_owned(), words.join(", ")));
            }
        }
        Value::Null => {}
        _ => out.push((path.to_owned(), scalar(value))),
    }
}

fn scalar(value: &Value) -> String {
    match value {
        Value::Bool(true) => "yes".to_owned(),
        Value::Bool(false) => "no".to_owned(),
        Value::String(words) => words.clone(),
        Value::Null => "none".to_owned(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::views::find;
    use crate::views::tests::{ROOMS, fits, network, now};

    #[test]
    fn a_device_shows_its_fields_then_its_source_says_the_rest() {
        let network = network();
        let study = find(&network, "Study").expect("the study's eero");
        for room in ROOMS {
            let text = device(&network, study, now(), &Pen::plain(), room);
            fits(&text, room);
            assert!(text.starts_with("Study · access point"), "{text}");
            assert!(text.contains("firmware"), "{text}");
            assert!(text.contains("FROM EERO"), "{text}");
            assert!(text.contains("backhaul upstream"), "{text}");
            assert!(text.contains("update available"), "{text}");
            // The source's id and location are the device's id and name,
            // already said.
            assert!(!text.contains("location"), "{text}");
            assert!(text.contains("Hall"), "the link to Hall is missing: {text}");
        }
    }

    #[test]
    fn a_source_leaves_its_records_and_series_to_json() {
        let network = network();
        let study = find(&network, "Study").expect("the study's eero");
        let text = device(&network, study, now(), &Pen::plain(), Room::Unbounded);
        let from = text.find("FROM EERO").expect("the eero's section");
        let section = &text[from..];
        for left in [
            "{",
            "usage download hours",
            "history at",
            "up s",
            "support ends",
        ] {
            assert!(
                !section.contains(left),
                "{left:?} is in the section: {section}"
            );
        }
        assert!(
            section.contains("ipv6"),
            "a short list is a fact: {section}"
        );
        assert!(section.contains("usage download total bytes"), "{section}");
    }

    #[test]
    fn each_link_is_a_line_of_its_table_under_its_labels() {
        let network = network();
        let study = find(&network, "Study").expect("the study's eero");
        let text = device(&network, study, now(), &Pen::plain(), Room::Cells(100));
        let lines: Vec<&str> = text.lines().collect();
        let head = lines
            .iter()
            .position(|line| line.starts_with("LINKED TO"))
            .expect("the links have their labels");
        let medium = lines[head].find("MEDIUM").expect("medium has a label");
        for line in &lines[head + 1..head + 11] {
            assert_eq!(
                line.find("wifi"),
                Some(medium),
                "{line:?} under {:?}",
                lines[head]
            );
        }
        assert!(
            lines.get(head + 11).is_none_or(|line| line.is_empty()),
            "the Study has 10 links: {text}"
        );
    }

    #[test]
    fn a_modem_shows_its_line_and_its_log() {
        let network = network();
        let modem = find(&network, "Modem").expect("the modem");
        for room in ROOMS {
            let text = device(&network, modem, now(), &Pen::plain(), room);
            fits(&text, room);
            assert!(text.contains("DOWN"), "{text}");
            assert!(text.contains("411.0"), "{text}");
            assert!(text.contains("REG-RSP-MP"), "{text}");
        }
    }
}
