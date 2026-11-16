//! What net shows a person, and what it answers a machine, drawn from the
//! record alone (model.rs). Each view is a function of the record, the
//! time now, a pen and the room, so its tests set every width without a
//! terminal. A page shows each fact once, in layers: what every device can
//! say, then its role's own figures, then what each source says that
//! neither held, in the source's words (docs/glossary.md's Network words).
//! One device's page is device.rs, its sources' own words said.rs, what
//! the line carried carried.rs; the rest, and the answers, are here.

mod carried;
mod device;
mod said;
mod uplink;

use carried::carried;
pub use device::device;
pub use uplink::uplink;

use serde_json::{Map, Value, json};
use terminal_kit::layout::{self, Align, ITEMS, Room};
use terminal_kit::{Exit, Failure, Pen, flags, time};

use crate::model::{ConcernKind, Device, Link, Network, Role, Source, SourceState};

/// How long ago `then` was, for a person: `4 s ago`.
fn ago(now: u64, then: u64) -> String {
    format!("{} ago", time::span(now.saturating_sub(then)))
}

/// A role as a person says it.
fn role_words(role: Role) -> &'static str {
    match role {
        Role::Modem => "modem",
        Role::Router => "router",
        Role::AccessPoint => "access point",
        Role::Switch => "switch",
        Role::Receiver => "receiver",
        Role::Printer => "printer",
        Role::Client => "client",
    }
}

/// A count as net says one, its thousands set apart by commas so 1,900
/// pages don't read as a year; `--json` keeps the bare figure.
fn count(value: u64) -> String {
    let digits = value.to_string();
    let mut said = String::new();
    for (at, digit) in digits.chars().enumerate() {
        if at > 0 && (digits.len() - at).is_multiple_of(3) {
            said.push(',');
        }
        said.push(digit);
    }
    said
}

/// A count of a role as a person says it: `a modem`, `2 access points`.
fn counted_role(role: Role, many: usize) -> String {
    let word = role_words(role);
    match many {
        1 if word.starts_with(['a', 'e', 'i', 'o', 'u']) => format!("an {word}"),
        1 => format!("a {word}"),
        _ if word.ends_with("ch") => format!("{} {word}es", count(many as u64)),
        _ => format!("{} {word}s", count(many as u64)),
    }
}

fn roles_of(device: &Device) -> String {
    device
        .roles
        .iter()
        .map(|role| role_words(*role))
        .collect::<Vec<_>>()
        .join(", ")
}

/// The links a device is on, at either end.
fn touching<'a>(network: &'a Network, device: &'a Device) -> impl Iterator<Item = &'a Link> {
    network
        .links
        .iter()
        .filter(move |link| link.a.device == device.id || link.b.device == device.id)
}

/// The record's head: whose it is and how old.
fn head(network: &Network, now: u64, pen: &Pen) -> Vec<String> {
    vec![format!(
        "{}{}",
        pen.title("home network"),
        pen.role(
            "muted",
            &format!("{ITEMS}read {}", ago(now, network.collected_at))
        )
    )]
}

/// The sources' columns: each one's state, how old that news is (its
/// last reading where it answers, how long it hasn't where it doesn't),
/// and a note, what it reads or why it isn't answering, cut last.
const SOURCES: [(&str, Align); 4] = [
    ("source", Align::Left),
    ("state", Align::Left),
    ("age", Align::Right),
    ("note", Align::Left),
];

/// One source as a line of its table.
fn source_row(source: &Source, now: u64) -> Vec<String> {
    let age = |then: Option<u64>| {
        then.map(|then| time::span(now.saturating_sub(then)))
            .unwrap_or_default()
    };
    let reason = || source.reason.clone().unwrap_or_default();
    let (state, age, note) = match source.state {
        SourceState::Answering => ("answering", age(source.read_at), source.what.clone()),
        SourceState::Unreachable => ("unreachable", age(source.since), reason()),
        SourceState::SignedOut => ("signed out", age(source.since), reason()),
    };
    vec![source.name.clone(), state.to_owned(), age, note]
}

/// How many devices there are, and what they do.
fn census(devices: &[Device]) -> String {
    let mut roles: Vec<Role> = devices
        .iter()
        .flat_map(|device| device.roles.clone())
        .collect();
    roles.sort_unstable();
    let mut counts: Vec<(Role, usize)> = Vec::new();
    for role in roles {
        match counts.last_mut() {
            Some((last, count)) if *last == role => *count += 1,
            _ => counts.push((role, 1)),
        }
    }
    let said: Vec<String> = counts
        .into_iter()
        .map(|(role, count)| counted_role(role, count))
        .collect();
    let said: Vec<&str> = said.iter().map(String::as_str).collect();
    let devices = match devices.len() {
        1 => "1 device".to_owned(),
        many => format!("{} devices", count(many as u64)),
    };
    if said.is_empty() {
        devices
    } else {
        format!("{devices}: {}", flags::and(&said))
    }
}

/// A concern's kind as its label says it.
fn kind_words(kind: ConcernKind) -> &'static str {
    match kind {
        ConcernKind::Busy => "busy",
        ConcernKind::Weak => "weak",
        ConcernKind::Slow => "slow",
        ConcernKind::Ceiling => "ceiling",
        // An adjective, as the others are, for a device filling the line.
        ConcernKind::Fills => "heavy",
        ConcernKind::Lost => "lost",
    }
}

/// What's worth a look, each concern in net's own words beside its kind,
/// then the internet as the narrowest point under a label of its own,
/// since that's the line's speed rather than a fault, a blank line between
/// its two ways so they don't run together in a narrow terminal. Neither
/// shows when there's nothing to say.
fn worth_a_look(network: &Network, pen: &Pen, room: Room) -> Vec<Vec<String>> {
    let (ceilings, faults): (Vec<_>, Vec<_>) = network
        .concerns
        .iter()
        .partition(|concern| concern.kind == ConcernKind::Ceiling);
    let mut blocks = Vec::new();
    if !faults.is_empty() {
        let facts: Vec<(&str, Option<&str>)> = faults
            .iter()
            .map(|concern| (kind_words(concern.kind), Some(concern.says.as_str())))
            .collect();
        let mut block = vec![pen.label("worth a look")];
        block.extend(layout::facts(&facts, "", pen, room));
        blocks.push(block);
    }
    let day = carried(network, pen, room);
    if !ceilings.is_empty() || !day.is_empty() {
        let mut block = vec![pen.label("the internet")];
        for (at, concern) in ceilings.into_iter().enumerate() {
            if at > 0 {
                block.push(String::new());
            }
            block.extend(
                layout::paragraph(&concern.says, 0, room)
                    .into_iter()
                    .map(|line| pen.role("text", &line)),
            );
        }
        if !day.is_empty() && block.len() > 1 {
            block.push(String::new());
        }
        block.extend(day);
        blocks.push(block);
    }
    blocks
}

/// `net`: whether each source answers, what the devices do, what's worth
/// a look, and the commands that go further.
pub fn home(network: &Network, now: u64, pen: &Pen, room: Room) -> String {
    let sources: Vec<Vec<String>> = network
        .sources
        .iter()
        .map(|source| source_row(source, now))
        .collect();
    let further = [
        (
            "net devices",
            "every device, what it does and where it's reached",
        ),
        (
            "net devices NAME",
            "one device in full, from each of its sources",
        ),
    ]
    .map(|(command, shows)| vec![command.to_owned(), shows.to_owned()]);
    let mut blocks = vec![
        head(network, now, pen),
        layout::table_in(&SOURCES, &sources, pen, room, &[2]),
        vec![pen.role("text", &layout::hung(&census(&network.devices), 0, room))],
    ];
    blocks.extend(worth_a_look(network, pen, room));
    blocks.push(layout::table_in(
        &[("further", Align::Left), ("shows", Align::Left)],
        &further,
        pen,
        room,
        &[],
    ));
    layout::joined(blocks)
}

/// The devices' columns, and the order they give way in, by place: the
/// model first, then how long each has been up, then its address. Its
/// name and roles always stay.
const DEVICES: [(&str, Align); 5] = [
    ("device", Align::Left),
    ("roles", Align::Left),
    ("address", Align::Left),
    ("up", Align::Right),
    ("model", Align::Left),
];
const DEVICES_GIVE_WAY: [usize; 3] = [4, 3, 2];

/// `net devices`: every device as one line of a table.
pub fn devices(network: &Network, now: u64, pen: &Pen, room: Room) -> String {
    let rows: Vec<Vec<String>> = network
        .devices
        .iter()
        .map(|device| {
            vec![
                device.name.clone(),
                roles_of(device),
                device.addresses.first().cloned().unwrap_or_default(),
                device.up_s.map(time::span).unwrap_or_default(),
                device.model.clone().unwrap_or_default(),
            ]
        })
        .collect();
    layout::joined(vec![
        head(network, now, pen),
        layout::table_in(&DEVICES, &rows, pen, room, &DEVICES_GIVE_WAY),
    ])
}

/// One of several devices sharing a name, told apart as a person would:
/// its first address and what it reaches the network through, where the
/// record says them, then its id only where no address of its own picks
/// it out.
fn told_apart(network: &Network, device: &Device) -> String {
    let through = touching(network, device).find_map(|link| {
        let other = if link.a.device == device.id {
            &link.b
        } else {
            &link.a
        };
        network.devices.iter().find(|each| each.id == other.device)
    });
    let place: Vec<String> = device
        .addresses
        .first()
        .cloned()
        .into_iter()
        .chain(through.map(|through| format!("on {}", through.name)))
        .collect();
    let id = (picking(network, device) == device.id).then(|| format!("id {}", device.id));
    match (place.as_slice(), id) {
        ([], id) => id.unwrap_or_default(),
        (place, None) => place.join(" "),
        (place, Some(id)) => format!("{}, {id}", place.join(" ")),
    }
}

/// The word that picks out one device of several: its first address, as
/// a person reads and types it, unless another device answers there too,
/// and then its id.
fn picking(network: &Network, device: &Device) -> String {
    device
        .addresses
        .first()
        .filter(|address| {
            network
                .devices
                .iter()
                .filter(|each| each.addresses.contains(address))
                .count()
                == 1
        })
        .unwrap_or(&device.id)
        .clone()
}

/// The device a word names: its id, any of its addresses, or its name in
/// any case. Two devices can share a name, as eero gives two phones the
/// same one, and then the word is refused with where each one is and its
/// id, offering the first one's address to choose by.
pub fn find<'a>(network: &'a Network, word: &str) -> Result<&'a Device, Failure> {
    if let Some(device) = network.devices.iter().find(|device| device.id == word) {
        return Ok(device);
    }
    if let Some(device) = network
        .devices
        .iter()
        .find(|device| device.addresses.iter().any(|address| address == word))
    {
        return Ok(device);
    }
    let sharing: Vec<&Device> = network
        .devices
        .iter()
        .filter(|device| device.name.eq_ignore_ascii_case(word))
        .collect();
    match sharing.as_slice() {
        [device] => return Ok(device),
        [first, ..] => {
            let each: Vec<String> = sharing
                .iter()
                .map(|device| told_apart(network, device))
                .collect();
            return Err(Failure::new(
                Exit::Usage,
                format!("{} devices are called {word}", count(sharing.len() as u64)),
            )
            .because(each.join(ITEMS))
            .try_this(format!("net devices {}", picking(network, first))));
        }
        [] => {}
    }
    let names: Vec<String> = network
        .devices
        .iter()
        .map(|device| device.name.to_lowercase())
        .collect();
    let names: Vec<&str> = names.iter().map(String::as_str).collect();
    let failure = Failure::new(Exit::NotFound, format!("no device is called {word}"));
    let near = flags::nearest(&word.to_lowercase(), &names).and_then(|near| {
        network
            .devices
            .iter()
            .find(|device| device.name.to_lowercase() == near)
    });
    Err(match near {
        Some(device) => failure.try_this(format!("net devices {}", quoted(&device.name))),
        None => failure.try_this("net devices"),
    })
}

/// A name as a shell takes it as one word.
fn quoted(name: &str) -> String {
    if name.contains(|c: char| c.is_whitespace() || "'\"$`\\".contains(c)) {
        format!("'{}'", name.replace('\'', r"'\''"))
    } else {
        name.to_owned()
    }
}

/// The names of net's own times, which an answer says in RFC 3339
/// (docs/commands.md, The Answer), where the record keeps seconds.
const TIMES: [&str; 4] = ["collected_at", "read_at", "since", "at"];

/// A value as an answer says it: each of net's times in RFC 3339. What a
/// source said, under `detail`, stays in its own words.
fn answered(value: Value) -> Value {
    match value {
        Value::Object(fields) => Value::Object(
            fields
                .into_iter()
                .map(|(key, value)| {
                    let value = match (&value, key.as_str()) {
                        (_, "detail") => value,
                        (Value::Number(seconds), key) if TIMES.contains(&key) => seconds
                            .as_i64()
                            .map_or(value.clone(), |seconds| json!(time::rfc3339(seconds))),
                        _ => answered(value),
                    };
                    (key, value)
                })
                .collect::<Map<_, _>>(),
        ),
        Value::Array(items) => Value::Array(items.into_iter().map(answered).collect()),
        other => other,
    }
}

/// `net --json`: the whole record, as `net.network/1`.
pub fn home_json(network: &Network) -> Value {
    answered(json!(network))
}

/// `net devices --json`, as `net.devices/1`.
pub fn devices_json(network: &Network) -> Value {
    answered(json!({
        "schema": "net.devices/1",
        "collected_at": network.collected_at,
        "devices": network.devices,
    }))
}

/// `net devices NAME --json`, as `net.device/1`: the device whole, each
/// source's detail kept as it said it, and the links it's on.
pub fn device_json(network: &Network, device: &Device) -> Value {
    let links: Vec<&Link> = touching(network, device).collect();
    answered(json!({
        "schema": "net.device/1",
        "collected_at": network.collected_at,
        "device": device,
        "links": links,
    }))
}

/// The record every view's tests read, and the widths they set; the
/// device page's tests read them too.
#[cfg(test)]
mod tests {
    use super::*;

    const RECORD: &str = include_str!("../../tests/network.json");

    pub(crate) fn network() -> Network {
        serde_json::from_str(RECORD).expect("the record reads")
    }

    /// Four seconds after the record was put together.
    pub(crate) fn now() -> u64 {
        network().collected_at + 4
    }

    pub(crate) const ROOMS: [Room; 4] = [
        Room::Cells(80),
        Room::Cells(100),
        Room::Cells(200),
        Room::Unbounded,
    ];

    #[test]
    fn home_says_how_each_source_answers_and_what_the_devices_do() {
        let network = network();
        for room in ROOMS {
            let text = home(&network, now(), &Pen::plain(), room);
            assert_eq!(layout::overrun(&text, room), None);
            assert!(text.starts_with("home network · read 4 s ago"), "{text}");
            // Read as words, since a narrow room wraps the line and the
            // longest source's name sets the column.
            let words = text.split_whitespace().collect::<Vec<_>>().join(" ");
            assert!(words.contains("modem answering"), "{text}");
            assert!(
                words.contains(
                    "23 devices: a modem, a router, 3 access points, a switch, a receiver, a \
                     printer and 16 clients"
                ),
                "{text}"
            );
            assert!(text.contains("net devices NAME"), "{text}");
        }
    }

    #[test]
    fn home_says_whats_worth_a_look_in_nets_words_and_the_internet_apart() {
        let network = network();
        for room in [Room::Cells(60), Room::Cells(100), Room::Unbounded] {
            let text = home(&network, now(), &Pen::plain(), room);
            assert_eq!(layout::overrun(&text, room), None);
            let words = text.split_whitespace().collect::<Vec<_>>().join(" ");
            let worth = words
                .find("WORTH A LOOK")
                .expect("the view says what's worth a look");
            let internet = words.find("THE INTERNET").expect("and the internet apart");
            assert!(worth < internet, "the faults come first: {text}");
            let mut last = worth;
            for said in [
                "lost The modem's downstream channel 16 has lost 878",
                "heavy unknown computer 1 is using 83% of the internet's speed up",
                "weak laptop computer 1 has a weak signal to Study",
            ] {
                let at = words
                    .find(said)
                    .unwrap_or_else(|| panic!("{said:?}: {text}"));
                assert!(
                    last < at && at < internet,
                    "{said:?} is a fault, worst first: {text}"
                );
                last = at;
            }
            assert!(
                text.contains("\n\nThe internet is the narrowest point up"),
                "each way's ceiling stands apart: {text}"
            );
            let up = words
                .find("The internet is the narrowest point up for every device")
                .expect("the line's ceiling is said");
            assert!(internet < up, "the ceiling is the internet's: {text}");
            let mut last = up;
            for said in [
                "What the line carried over the last day:",
                "down 167 GB, averaging 14.9 Mbit/s, 4% of its speed; 28.1 in its busiest hour",
                "up 72.7 GB, averaging 6.5 Mbit/s, 54% of its speed; 10.8 in its busiest hour",
            ] {
                let at = words
                    .find(said)
                    .unwrap_or_else(|| panic!("{said:?}: {text}"));
                assert!(last < at, "{said:?} follows the ceilings: {text}");
                last = at;
            }
        }
    }

    #[test]
    fn a_network_with_nothing_worth_a_look_says_nothing_of_it() {
        let mut network = network();
        network.concerns.clear();
        let text = home(&network, now(), &Pen::plain(), Room::Unbounded);
        assert!(!text.contains("WORTH A LOOK"), "{text}");
        assert!(
            text.contains("THE INTERNET\nWhat the line carried"),
            "the internet says only its day: {text}"
        );
        if let Some(internet) = network.internet.as_mut() {
            internet.carried = None;
        }
        let text = home(&network, now(), &Pen::plain(), Room::Unbounded);
        assert!(!text.contains("THE INTERNET"), "{text}");
    }

    #[test]
    fn a_source_not_answering_says_for_how_long() {
        let mut network = network();
        let modem = &mut network.sources[1];
        assert_eq!(modem.name, "modem", "the record's second source moved");
        modem.state = SourceState::SignedOut;
        modem.since = Some(network.collected_at - 125);
        modem.reason = Some("no password released for it yet".to_owned());
        let text = home(&network, now(), &Pen::plain(), Room::Unbounded);
        let line = text
            .lines()
            .find(|line| line.starts_with("modem"))
            .expect("the modem's source has a line");
        assert!(line.contains("signed out"), "{line:?}");
        assert!(line.contains("no password released"), "{line:?}");
        assert!(line.contains("2 m 9 s"), "{line:?}");
    }

    #[test]
    fn every_device_is_a_line_of_its_table_under_its_labels() {
        let network = network();
        for room in ROOMS {
            let text = devices(&network, now(), &Pen::plain(), room);
            assert_eq!(layout::overrun(&text, room), None);
            let lines: Vec<&str> = text.lines().collect();
            let head = lines
                .iter()
                .position(|line| line.starts_with("DEVICE"))
                .expect("the table has its labels");
            let roles = lines[head].find("ROLES").expect("roles has a label");
            for (line, device) in lines[head + 1..].iter().zip(&network.devices) {
                assert!(line.starts_with(&device.name), "{line:?}");
                assert_eq!(
                    // Past the name, which may hold a role's word, as a
                    // device named for its role does.
                    line[device.name.len()..]
                        .find(&roles_of(device))
                        .map(|at| at + device.name.len()),
                    Some(roles),
                    "{device:?}'s roles aren't under their label: {line:?}"
                );
            }
        }
    }

    #[test]
    fn a_narrow_terminal_drops_the_model_before_the_address() {
        let text = devices(&network(), now(), &Pen::plain(), Room::Cells(50));
        assert_eq!(layout::overrun(&text, Room::Cells(50)), None);
        assert!(!text.contains("MODEL"), "{text}");
        assert!(text.contains("ROLES"), "{text}");
    }

    #[test]
    fn a_device_is_found_by_its_name_in_any_case_or_its_id() {
        let network = network();
        assert_eq!(
            find(&network, "study").map(|d| &d.id),
            Ok(&"eero:2002".to_owned())
        );
        assert_eq!(
            find(&network, "modem").map(|d| &d.id),
            Ok(&"modem".to_owned())
        );
        assert_eq!(
            find(&network, "eero:2003").map(|d| &d.name),
            Ok(&"Hall".to_owned())
        );
    }

    #[test]
    fn a_device_is_found_by_any_of_its_addresses() {
        let network = network();
        assert_eq!(
            find(&network, "10.0.0.88").map(|d| &d.name),
            Ok(&"Study".to_owned())
        );
        assert_eq!(
            find(&network, "fe80::3").map(|d| &d.id),
            Ok(&"mac:02:00:00:00:00:05".to_owned()),
            "a second address doesn't find its device"
        );
    }

    #[test]
    fn a_name_two_devices_share_is_refused_with_where_each_is() {
        let network = network();
        let failure = find(&network, "phone 1").expect_err("two phones are called phone 1");
        assert_eq!(failure.exit, Exit::Usage, "{failure:?}");
        assert_eq!(
            failure.why.as_deref(),
            Some(&*format!("10.0.0.40 on Study{ITEMS}10.0.0.23 on Hall")),
            "each isn't told by where it is alone: {failure:?}"
        );
        assert_eq!(
            failure.then.as_deref(),
            Some("net devices 10.0.0.40"),
            "the try-line isn't an address a person types: {failure:?}"
        );
        assert_eq!(
            find(&network, "mac:02:00:00:00:00:05").map(|d| &d.name),
            Ok(&"phone 1".to_owned())
        );
    }

    #[test]
    fn devices_sharing_an_address_too_are_told_apart_by_their_ids() {
        let mut network = network();
        let first = network
            .devices
            .iter()
            .position(|device| device.id == "mac:02:00:00:00:00:03")
            .expect("the first phone");
        let second = network
            .devices
            .iter()
            .position(|device| device.id == "mac:02:00:00:00:00:05")
            .expect("the second phone");
        network.devices[second].addresses = network.devices[first].addresses.clone();
        let failure = find(&network, "phone 1").expect_err("two phones are called phone 1");
        assert_eq!(
            failure.why.as_deref(),
            Some(&*format!(
                "10.0.0.40 on Study, id mac:02:00:00:00:00:03{ITEMS}\
                 10.0.0.40 on Hall, id mac:02:00:00:00:00:05"
            )),
            "a shared address doesn't add each one's id: {failure:?}"
        );
        assert_eq!(
            failure.then.as_deref(),
            Some("net devices mac:02:00:00:00:00:03"),
            "{failure:?}"
        );
    }

    #[test]
    fn an_unknown_device_is_refused_with_the_nearest_name() {
        let network = network();
        let failure = find(&network, "Stdy").expect_err("Stdy isn't a device");
        assert_eq!(failure.exit, Exit::NotFound, "{failure:?}");
        assert_eq!(
            failure.then.as_deref(),
            Some("net devices Study"),
            "{failure:?}"
        );
        let failure = find(&network, "printr 1").expect_err("printr 1 isn't a device");
        assert_eq!(
            failure.then.as_deref(),
            Some("net devices 'printer 1'"),
            "{failure:?}"
        );
        let failure = find(&network, "garage").expect_err("garage isn't a device");
        assert_eq!(failure.then.as_deref(), Some("net devices"), "{failure:?}");
    }

    #[test]
    fn a_device_answers_whole_with_its_source_detail_as_said() {
        let network = network();
        let study = find(&network, "Study").expect("the study's eero");
        let answer = device_json(&network, study);
        assert_eq!(answer["schema"], "net.device/1", "{answer}");
        assert_eq!(
            answer["device"]["detail"]["eero"], study.detail["eero"],
            "the eero's detail wasn't kept whole"
        );
        assert_eq!(
            answer["collected_at"],
            time::rfc3339(1_794_702_033),
            "{answer}"
        );
        assert_eq!(
            answer["links"].as_array().map(Vec::len),
            Some(10),
            "{answer}"
        );
    }

    #[test]
    fn every_time_an_answer_gives_is_rfc_3339() {
        let answer = home_json(&network());
        assert_eq!(answer["schema"], crate::model::SCHEMA, "{answer}");
        assert!(answer["sources"][0]["read_at"].is_string(), "{answer}");
        let modem = &answer["devices"][0]["modem"]["log"][0];
        assert!(
            modem["at"].as_str().is_some_and(|at| at.ends_with('Z')),
            "{modem}"
        );
        let devices = devices_json(&network());
        assert!(devices["collected_at"].is_string(), "{devices}");
    }
}
