//! `net devices NAME`: one device in layers. What every device can say,
//! its ports, radios and links, a role's own figures (a modem's channels
//! and log, a receiver's volume and what it's doing, a printer's ink),
//! then what each source says that none of those held, in the source's own words.

use terminal_kit::layout::{self, Align, ITEMS, Room};
use terminal_kit::{Pen, time};

use super::{ago, count, roles_of, said, touching};
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
    if let Some(receiver) = &device.receiver {
        let volume = receiver.volume.map(|volume| match receiver.loudest {
            Some(loudest) => format!("{volume} of {loudest}"),
            None => volume.to_string(),
        });
        let facts = [
            ("volume", volume.as_deref()),
            (
                "muted",
                receiver.muted.map(|muted| if muted { "yes" } else { "no" }),
            ),
            ("doing", receiver.state.as_deref()),
            ("playing from", receiver.medium.as_deref()),
            ("track", receiver.track.as_deref()),
        ];
        let mut block = vec![pen.label("receiver")];
        block.extend(known_facts(&facts, pen, room));
        blocks.push(block);
    }
    if let Some(printer) = &device.printer {
        let doing = (!printer.state.is_empty()).then(|| printer.state.join(", "));
        let pages = printer.pages.map(count);
        // Each cartridge is a fact of its own, its state said only when it
        // isn't fine, so a page with nothing wrong reads as its figures.
        let ink: Vec<(String, Option<String>)> = printer
            .ink
            .iter()
            .map(|ink| {
                let left = ink.left_percent.map(|left| format!("{left}%"));
                let state = ink.state.as_deref().filter(|state| *state != "ok");
                let said = match (left, state) {
                    (Some(left), Some(state)) => Some(format!("{left}, {state}")),
                    (left, state) => left.or(state.map(str::to_owned)),
                };
                (format!("{} ink", ink.colour), said)
            })
            .collect();
        let mut facts = vec![("doing", doing.as_deref())];
        facts.extend(
            ink.iter()
                .map(|(label, said)| (label.as_str(), said.as_deref())),
        );
        facts.push(("pages printed", pages.as_deref()));
        let mut block = vec![pen.label("printer")];
        block.extend(known_facts(&facts, pen, room));
        blocks.push(block);
    }
    blocks.extend(said::sections(device, pen, room));
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
    ("speed Mb/s", Align::Right),
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
    telling(&PORTS, &rows, pen, room, &[4, 3])
}

/// A table of only the columns that say something: one goes when every
/// row leaves it empty, as down ports leave their speed. The first
/// column names each row, so it stays.
fn telling(
    columns: &[(&str, Align)],
    rows: &[Vec<String>],
    pen: &Pen,
    room: Room,
    gives_way: &[usize],
) -> Vec<String> {
    let says = |place: usize| place == 0 || rows.iter().any(|row| !row[place].is_empty());
    let kept: Vec<usize> = (0..columns.len()).filter(|&place| says(place)).collect();
    let columns: Vec<(&str, Align)> = kept.iter().map(|&place| columns[place]).collect();
    let rows: Vec<Vec<String>> = rows
        .iter()
        .map(|row| kept.iter().map(|&place| row[place].clone()).collect())
        .collect();
    let gives_way: Vec<usize> = gives_way
        .iter()
        .filter_map(|place| kept.iter().position(|kept| kept == place))
        .collect();
    layout::table_in(&columns, &rows, pen, room, &gives_way)
}

const RADIOS: [(&str, Align); 6] = [
    ("radio", Align::Left),
    ("band", Align::Left),
    ("channel", Align::Right),
    ("width MHz", Align::Right),
    ("busy %", Align::Right),
    ("power dBm", Align::Right),
];

fn radios(device: &Device, pen: &Pen, room: Room) -> Vec<String> {
    if device.radios.is_empty() {
        return Vec::new();
    }
    let rows: Vec<Vec<String>> = device
        .radios
        .iter()
        .map(|radio| {
            // A radio named by its band says it once.
            let band = radio.band.clone().filter(|band| *band != radio.name);
            vec![
                radio.name.clone(),
                band.unwrap_or_default(),
                figure(radio.channel),
                figure(radio.width_mhz),
                figure(radio.busy_pct),
                figure(radio.power_dbm),
            ]
        })
        .collect();
    telling(&RADIOS, &rows, pen, room, &[5, 1, 3])
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
/// other device, the medium, the speed and the signal always stay.
const LINKS: [(&str, Align); 7] = [
    ("linked to", Align::Left),
    ("medium", Align::Left),
    ("channel", Align::Right),
    ("here", Align::Left),
    ("there", Align::Left),
    ("mbit/s", Align::Right),
    ("signal dBm", Align::Right),
];
const LINKS_GIVE_WAY: [usize; 3] = [4, 3, 2];

/// The devices this one links to, each a line of a table: the other
/// device, how, where on each, at what speed and how strong a signal.
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
                figure(link.signal_dbm),
            ]
        })
        .collect();
    if rows.is_empty() {
        return Vec::new();
    }
    telling(&LINKS, &rows, pen, room, &LINKS_GIVE_WAY)
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
        ("width MHz", Align::Right),
        ("modulation", Align::Left),
        ("power dBmV", Align::Right),
        ("snr dB", Align::Right),
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
    // Each kind together, in the order the modem first lists it, then by
    // the number it gives each, the ones it numbers none last; the modem
    // lists them in the order they locked.
    let kind_at = |kind: &str| channels.iter().position(|each| each.kind == kind);
    let mut channels: Vec<&Channel> = channels.iter().collect();
    channels.sort_by_key(|channel| {
        (
            kind_at(&channel.kind),
            channel.channel.is_none(),
            channel.channel,
        )
    });
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
                channel.corrected.map(count).unwrap_or_default(),
                channel.uncorrectable.map(count).unwrap_or_default(),
                if channel.active { "yes" } else { "no" }.to_owned(),
            ]
        })
        .collect();
    telling(&columns, &rows, pen, room, &[3, 7, 4, 2, 9])
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::views::find;
    use crate::views::tests::{ROOMS, network, now};

    #[test]
    fn a_device_shows_its_fields_then_its_source_says_the_rest() {
        let network = network();
        let study = find(&network, "Study").expect("the study's eero");
        for room in ROOMS {
            let text = device(&network, study, now(), &Pen::plain(), room);
            assert_eq!(layout::overrun(&text, room), None);
            assert!(text.starts_with("Study · access point"), "{text}");
            assert!(text.contains("firmware"), "{text}");
            assert!(text.contains("FROM EERO"), "{text}");
            assert!(
                text.contains("\nbackhaul\n") && text.contains("  upstream"),
                "its backhaul's upstream isn't under its backhaul: {text}"
            );
            assert!(text.contains("update available"), "{text}");
            // The source's id and location are the device's id and name,
            // already said.
            assert!(!text.contains("location"), "{text}");
            assert!(text.contains("Hall"), "the link to Hall is missing: {text}");
        }
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
    fn a_column_saying_nothing_new_in_any_row_is_left_out() {
        let network = network();
        let study = find(&network, "Study").expect("the study's eero");
        let text = device(&network, study, now(), &Pen::plain(), Room::Cells(200));
        let head = |label: &str| {
            text.lines()
                .find(|line| line.starts_with(label))
                .unwrap_or_else(|| panic!("no table opens with {label}: {text}"))
        };
        // Both ports are down, so neither says a speed or a duplex.
        assert_eq!(
            head("PORT").split_whitespace().collect::<Vec<_>>(),
            ["PORT", "KIND", "LINK"],
            "{text}"
        );
        // Each radio is named by its band.
        assert!(!head("RADIO").contains("BAND"), "{text}");
    }

    #[test]
    fn a_unit_in_a_label_keeps_its_own_case() {
        let network = network();
        let head = |name: &str, opening: &str| {
            let found = find(&network, name).expect("the fixture holds it");
            let text = device(&network, found, now(), &Pen::plain(), Room::Cells(200));
            text.lines()
                .find(|line| line.starts_with(opening))
                .unwrap_or_else(|| panic!("no table opens with {opening}: {text}"))
                .to_owned()
        };
        let down = head("Modem", "DOWN");
        for label in ["POWER dBmV", "SNR dB"] {
            assert!(down.contains(label), "{label} isn't in {down:?}");
        }
        let links = head("Study", "LINKED TO");
        assert!(links.contains("SIGNAL dBm"), "{links:?}");
    }

    #[test]
    fn columns_that_happen_to_agree_both_stay() {
        let columns = [
            ("channel", Align::Left),
            ("corrected", Align::Right),
            ("uncorrectable", Align::Right),
        ];
        let rows = [vec!["1".to_owned(), "0".to_owned(), "0".to_owned()]];
        let table = telling(&columns, &rows, &Pen::plain(), Room::Cells(80), &[]);
        assert!(
            table[0].ends_with("UNCORRECTABLE"),
            "a column equal to its neighbour went: {table:?}"
        );
    }

    #[test]
    fn a_modems_channels_go_by_kind_then_number() {
        let network = network();
        let modem = find(&network, "Modem").expect("the modem");
        let text = device(&network, modem, now(), &Pen::plain(), Room::Cells(200));
        let numbers: Vec<&str> = text
            .lines()
            .skip_while(|line| !line.starts_with("DOWN"))
            .skip(1)
            .take_while(|line| !line.is_empty())
            .map(|line| line.split_whitespace().next().unwrap_or_default())
            .collect();
        let expected: Vec<String> = (1..=16).map(|n| n.to_string()).collect();
        assert_eq!(numbers[..16], expected, "{text}");
        assert_eq!(numbers[16..], ["ofdm", "ofdm"], "{text}");
        let up: Vec<&str> = text
            .lines()
            .skip_while(|line| !line.starts_with("UP"))
            .skip(1)
            .take_while(|line| !line.is_empty())
            .map(|line| line.split_whitespace().nth(1).unwrap_or_default())
            .collect();
        assert_eq!(
            up,
            ["atdma", "atdma", "atdma", "atdma", "ofdma", "ofdma"],
            "{text}"
        );
    }

    #[test]
    fn a_link_says_its_signal_where_its_source_measured_one() {
        let network = network();
        let study = find(&network, "Study").expect("the study's eero");
        let text = device(&network, study, now(), &Pen::plain(), Room::Cells(200));
        let phone = text
            .lines()
            .find(|line| line.starts_with("phone 1 "))
            .expect("the study links to a phone");
        assert!(
            phone.trim_end().ends_with("-69"),
            "the phone's signal isn't last on its line: {phone}"
        );
    }

    #[test]
    fn a_modem_shows_its_line_and_its_log() {
        let network = network();
        let modem = find(&network, "Modem").expect("the modem");
        for room in ROOMS {
            let text = device(&network, modem, now(), &Pen::plain(), room);
            assert_eq!(layout::overrun(&text, room), None);
            assert!(text.contains("DOWN"), "{text}");
            assert!(text.contains("411.0"), "{text}");
            assert!(text.contains("REG-RSP-MP"), "{text}");
        }
        let text = device(&network, modem, now(), &Pen::plain(), Room::Unbounded);
        assert!(
            text.contains("1,488,060,867"),
            "its corrected codewords aren't counted with commas: {text}"
        );
    }

    #[test]
    fn a_count_sets_its_thousands_apart() {
        for (value, said) in [
            (0_u64, "0"),
            (999, "999"),
            (1_900, "1,900"),
            (40_000, "40,000"),
            (1_488_060_867, "1,488,060,867"),
        ] {
            assert_eq!(count(value), said, "{value} reads wrong");
        }
    }

    #[test]
    fn a_receiver_shows_its_volume_and_what_its_doing() {
        let network = network();
        let stereo = find(&network, "Stereo").expect("the stereo");
        for room in ROOMS {
            let text = device(&network, stereo, now(), &Pen::plain(), room);
            assert_eq!(layout::overrun(&text, room), None);
            assert!(text.contains("RECEIVER"), "its block has no label: {text}");
            assert!(
                text.contains("27 of 100"),
                "its volume isn't shown on its scale: {text}"
            );
            assert!(
                text.contains("nothing to play"),
                "what it's doing isn't shown: {text}"
            );
            assert!(
                !text.contains("playing from"),
                "it plays from nothing, yet says so: {text}"
            );
            assert!(
                text.contains("NO_MEDIA_PRESENT"),
                "its transport's own word isn't in Sony's section: {text}"
            );
            assert!(text.contains("STR-DN1050"), "its model isn't shown: {text}");
        }
    }

    #[test]
    fn a_printer_shows_its_ink_and_what_its_doing() {
        let network = network();
        let printer = find(&network, "Printer").expect("the printer");
        for room in ROOMS {
            let text = device(&network, printer, now(), &Pen::plain(), room);
            assert_eq!(layout::overrun(&text, room), None);
            let words = text.split_whitespace().collect::<Vec<_>>().join(" ");
            assert!(text.contains("PRINTER"), "its block has no label: {text}");
            assert!(
                words.contains("doing out of paper, saving power"),
                "what it's doing isn't shown: {text}"
            );
            assert!(
                words.contains("black ink 50%"),
                "its black ink isn't shown: {text}"
            );
            assert!(
                words.contains("colour ink 60%"),
                "its colour ink isn't shown: {text}"
            );
            assert!(
                words.contains("pages printed 1,900"),
                "what it has printed isn't shown: {text}"
            );
            assert!(
                text.contains("SERIAL00000070") && text.contains("SPP5FN2223AR"),
                "its serial and firmware aren't shown: {text}"
            );
            assert!(
                words.contains("trayEmpty, inPowerSave"),
                "HP's own status codes aren't in its section: {text}"
            );
            assert!(
                !text.contains("PowerSaveTimeout") && !text.contains("TotalImpressions"),
                "its settings and counters bury its section, rather than staying in --json: \
                 {text}"
            );
        }
    }
}
