//! The command line's views of the home network, each read from the record
//! the daemon keeps (`model.rs`) and set either as text for a person or as
//! the record's own slice in JSON, for a program.
//!
//! Every page is a pure function of the record, the time and the ink, so
//! each is held by a golden page drawn from `home-network.json`
//! (`golden/`). Each opens with what it's of and how old the reading is,
//! and says plainly when eero's cloud isn't answering or the reading has
//! grown old, since a figure that looks current and isn't is worse than
//! none.

mod clock;
mod devices;
mod events;
mod figures;
mod firmware;
mod fit;
mod graph;
mod home;
mod internet;
mod json;
mod marks;
mod mesh;
pub mod names;
mod nodes;
mod profiles;
mod radio;
mod speeds;
mod text;
pub mod theme;
mod traffic;

pub use clock::{Clock, changes_in, span_on_desktop};
pub use text::Ink;

/// A rate given in Mb/s as plain text, as every view sets one, for a line
/// outside the views: 76.8 Mb/s, 1.20 Gb/s.
pub fn rate(mbps: f64) -> String {
    figures::rate(mbps)
        .into_iter()
        .map(|run| run.text)
        .collect()
}

use std::borrow::Cow;

use terminal_kit::Room;

use crate::called;
use crate::model::{CloudState, Device, DeviceOff, HomeNetwork, Node, Profile};
use figures::joined;
use marks::marks;
use text::{Line, Page, Role, muted, plain, run, strong};

/// A reading older than this, while the record still says eero answers, is
/// one the daemon has stopped renewing.
const STALE_S: u64 = 15 * 60;

/// Every view, by the word that names it, and what it's of when it needs
/// one thing named.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum View {
    Status,
    Nodes,
    Node(String),
    /// The devices on now on one node.
    NodeDevices(String),
    /// One node's radios.
    NodeRadios(String),
    Mesh,
    Devices,
    Device(String),
    Internet,
    Speeds,
    Firmware,
    Radio,
    Traffic,
    Profiles,
    Profile(String),
    /// The devices in one profile, on now or not.
    ProfileDevices(String),
    /// Whether eero optimizes for conferencing and gaming.
    Sqm,
    Events,
}

/// The views' words, as a person types them and completion offers them,
/// with what each shows.
pub const VIEWS: [(&str, &str); 14] = [
    ("status", "the home network at a glance"),
    ("nodes", "every node: state, backhaul, devices, firmware"),
    ("node", "one node: its backhaul, radios, ports and devices"),
    ("mesh", "the nodes as a tree from the gateway"),
    ("devices", "every device on now, under its node"),
    ("device", "one device: every figure eero has for it"),
    ("internet", "the way out and every setting that shapes it"),
    ("speeds", "each speed test eero kept"),
    ("firmware", "what runs, what's next and when it updates"),
    ("radio", "every node's radios, band by band"),
    (
        "traffic",
        "bytes by hour, for the home network and each node",
    ),
    ("profiles", "each profile, what it blocks and its traffic"),
    ("profile", "one profile: every figure eero has for it"),
    ("events", "what happened, newest first"),
];

impl View {
    /// The view the words name, or why they name none.
    pub fn parse(words: &[&str]) -> Result<Self, String> {
        let one = |what: &str, by: &str, name: Option<&&str>, view: fn(String) -> Self| match name {
            Some(name) => Ok(view((*name).to_owned())),
            None => Err(format!("name the {what}: eero {what} <{by}>")),
        };
        let view = match words {
            ["status"] => Self::Status,
            ["nodes"] => Self::Nodes,
            ["node", name, "devices"] => Self::NodeDevices((*name).to_owned()),
            ["node", name, "radios"] => Self::NodeRadios((*name).to_owned()),
            ["node", rest @ ..] if rest.len() <= 1 => {
                one("node", "location|id", rest.first(), Self::Node)?
            }
            ["mesh"] => Self::Mesh,
            ["devices"] => Self::Devices,
            ["device", rest @ ..] if rest.len() <= 1 => {
                one("device", "name|mac", rest.first(), Self::Device)?
            }
            ["internet"] => Self::Internet,
            ["internet", "sqm"] => Self::Sqm,
            ["speeds"] => Self::Speeds,
            ["firmware"] => Self::Firmware,
            ["radio"] => Self::Radio,
            ["traffic"] => Self::Traffic,
            ["profiles"] => Self::Profiles,
            ["profile", name, "devices"] => Self::ProfileDevices((*name).to_owned()),
            ["profile", rest @ ..] if rest.len() <= 1 => {
                one("profile", "name|id", rest.first(), Self::Profile)?
            }
            ["events"] => Self::Events,
            _ => return Err(format!("no view is eero {}", words.join(" "))),
        };
        Ok(view)
    }
}

/// A view as text for a person, laid out in the room its stream has:
/// whole into a pipe, and fitted to a terminal's width.
pub fn text(
    view: &View,
    model: &HomeNetwork,
    clock: &Clock,
    ink: &Ink,
    room: Room,
) -> Result<String, String> {
    let page = match view {
        View::Status => home::page(model, clock),
        View::Nodes => nodes::all(model, clock),
        View::Node(name) => nodes::one(model, find_node(model, name)?, clock, ink),
        View::NodeDevices(name) => {
            let node = find_node(model, name)?;
            devices::all(&on_one_node(model, node), clock, Some(node_name(node)))
        }
        View::NodeRadios(name) => {
            let node = find_node(model, name)?;
            radio::page(&on_one_node(model, node), clock, ink, Some(node_name(node)))
        }
        View::Mesh => mesh::page(model, clock),
        View::Radio => radio::page(model, clock, ink, None),
        View::Devices => devices::all(model, clock, None),
        View::Device(name) => devices::one(model, find_device(model, name, Some(clock))?, clock),
        View::Speeds => speeds::page(model, clock, ink),
        View::Internet => internet::page(model, clock, ink),
        View::Firmware => firmware::page(model, clock),
        View::Traffic => traffic::page(model, clock, ink),
        View::Profiles => profiles::all(model, clock),
        View::Profile(name) => profiles::one(model, find_profile(model, name)?, clock, ink),
        View::ProfileDevices(name) => {
            let profile = find_profile(model, name)?;
            let within = in_one_profile(model, profile);
            let mut page = devices::all(&within, clock, Some(profile_name(profile)));
            devices::unseen(&mut page, &unseen(&within, profile));
            page
        }
        View::Sqm => internet::sqm(model, clock),
        View::Events => events::page(model, clock),
    };
    Ok(ink.page(page, room))
}

/// A view as JSON: the record's own slice of it, under the record's own
/// names, with when it was read and whether eero was answering.
pub fn json(view: &View, model: &HomeNetwork) -> Result<String, String> {
    match view {
        View::Status => home::json(model),
        View::Nodes => nodes::all_json(model),
        View::Node(name) => nodes::one_json(model, find_node(model, name)?),
        View::NodeDevices(name) => devices::all_json(&on_one_node(model, find_node(model, name)?)),
        View::NodeRadios(name) => radio::json(&on_one_node(model, find_node(model, name)?)),
        View::Mesh => mesh::json(model),
        View::Radio => radio::json(model),
        View::Devices => devices::all_json(model),
        View::Device(name) => devices::one_json(model, find_device(model, name, None)?),
        View::Speeds => speeds::json(model),
        View::Internet => internet::json(model),
        View::Firmware => firmware::json(model),
        View::Traffic => traffic::json(model),
        View::Profiles => profiles::all_json(model),
        View::Profile(name) => profiles::one_json(model, find_profile(model, name)?),
        View::ProfileDevices(name) => {
            let profile = find_profile(model, name)?;
            let within = in_one_profile(model, profile);
            devices::in_profile_json(&within, &unseen(&within, profile))
        }
        View::Sqm => internet::sqm_json(model),
        View::Events => events::json(model),
    }
}

/// The record as one node holds it: that node alone, and the devices on
/// it now and last on it, so a view of the whole reads as one of the node
/// and its JSON keeps the whole's schema.
fn on_one_node(model: &HomeNetwork, node: &Node) -> HomeNetwork {
    let on = |id: Option<&str>| id == Some(node.id.as_str());
    HomeNetwork {
        nodes: vec![node.clone()],
        devices: model
            .devices
            .iter()
            .filter(|device| on(device.node.as_deref()))
            .cloned()
            .collect(),
        devices_off: model
            .devices_off
            .iter()
            .filter(|device| on(device.node.as_deref()))
            .cloned()
            .collect(),
        ..model.clone()
    }
}

/// The record as one profile holds it: the devices in it, on now or not,
/// under each node as the whole has them.
fn in_one_profile(model: &HomeNetwork, profile: &Profile) -> HomeNetwork {
    let held = |mac: &str| {
        profile
            .devices
            .iter()
            .any(|member| member.eq_ignore_ascii_case(mac))
    };
    HomeNetwork {
        devices: model
            .devices
            .iter()
            .filter(|device| held(&device.mac))
            .cloned()
            .collect(),
        devices_off: model
            .devices_off
            .iter()
            .filter(|device| held(&device.mac))
            .cloned()
            .collect(),
        ..model.clone()
    }
}

/// The devices a profile holds that the record keeps neither as on now
/// nor as off, since eero hasn't seen them lately, each by the name the
/// profile gives it, so its devices page lists every one it holds.
fn unseen<'a>(within: &HomeNetwork, profile: &'a Profile) -> Vec<devices::Unseen<'a>> {
    let kept = |mac: &String| {
        within
            .devices
            .iter()
            .map(|device| &device.mac)
            .chain(within.devices_off.iter().map(|device| &device.mac))
            .any(|known| known.eq_ignore_ascii_case(mac))
    };
    profile
        .devices
        .iter()
        .filter(|mac| !kept(mac))
        .map(|mac| devices::Unseen {
            mac,
            name: profile
                .members
                .iter()
                .find(|member| member.mac.eq_ignore_ascii_case(mac))
                .and_then(|member| member.name.as_deref()),
        })
        .collect()
}

/// A page's title naming what it's within, as `Home · Study · 3 devices
/// on now`, or the whole's where it's within nothing.
fn titled_within(model: &HomeNetwork, within: Option<&str>, about: String) -> Line {
    match within {
        Some(within) => {
            let mut line = titled(model, Some(within.to_owned()));
            line.push(muted(format!(" {} ", marks().separator)));
            line.push(plain(about));
            line
        }
        None => titled(model, Some(about)),
    }
}

/// The head every page opens with: what it's of, how old the reading is,
/// and, when eero isn't answering or the reading has grown old, a line
/// saying so and what to do.
fn head(page: &mut Page, model: &HomeNetwork, clock: &Clock, title: Line) {
    let mut line = title;
    let age = clock::parse(&model.read).map(|read| clock.age(read));
    let separator = format!(" {} ", marks().separator);
    line.push(muted(separator.clone()));
    line.push(muted(match clock::parse(&model.read) {
        Some(read) => format!("read {}", clock.ago(read)),
        None => format!("read {}", model.read),
    }));
    page.line(line);
    let since = clock::parse(&model.cloud.since).map_or_else(
        || model.cloud.since.clone(),
        |since| format!("{} ({})", clock.minute(since), clock.ago(since)),
    );
    let reason = model
        .cloud
        .reason
        .as_deref()
        .map(|reason| format!(": {reason}"))
        .unwrap_or_default();
    match model.cloud.state {
        CloudState::Unreachable => page.line(vec![
            run(&marks().failed, Role::Alert),
            plain(format!(" eero's cloud unreachable since {since}{reason}")),
        ]),
        CloudState::SignedOut => page.line(vec![
            run(&marks().on, Role::Warning),
            plain(format!(
                " signed out of eero since {since}{reason}; sign in with "
            )),
            strong("eero sign-in <email or phone>"),
        ]),
        CloudState::Answering => {
            if let Some(age) = age.filter(|age| *age > STALE_S) {
                page.line(vec![
                    run(&marks().on, Role::Warning),
                    plain(format!(
                        " nothing new for {}; eero-daemon may have stopped: ",
                        clock::span(age)
                    )),
                    strong("systemctl --user start eero-daemon"),
                ]);
            }
        }
    }
    page.blank();
}

/// A node's name, its location, or its id where eero gives none.
pub fn node_name(node: &Node) -> &str {
    node.location.as_deref().unwrap_or(&node.id)
}

/// The location of the node with an id.
fn location<'a>(model: &'a HomeNetwork, id: Option<&'a str>) -> &'a str {
    let Some(id) = id else { return "no node" };
    model
        .nodes
        .iter()
        .find(|node| node.id == id)
        .map_or(id, node_name)
}

/// The devices on the node `id` names, this machine first and the rest as
/// eero orders them, so this machine is found where it hangs.
fn on_node<'a>(model: &'a HomeNetwork, id: Option<&str>) -> Vec<&'a Device> {
    let mut on: Vec<&Device> = model
        .devices
        .iter()
        .filter(|device| device.node.as_deref() == id)
        .collect();
    on.sort_by_key(|device| !device.this_machine);
    on
}

/// What a device on now is called (`called`), its maker said as the
/// command line says one.
pub fn device_name(device: &Device) -> Cow<'_, str> {
    called::device(device, called::MADE)
}

/// What a device not on now is called, as `device_name` says one on now.
pub fn off_name(device: &DeviceOff) -> Cow<'_, str> {
    called::off(device, called::MADE)
}

/// What the device with a hardware address is called, on now or not; none
/// for one eero doesn't list.
pub fn listed_name<'a>(model: &'a HomeNetwork, mac: &str) -> Option<Cow<'a, str>> {
    let on = model
        .devices
        .iter()
        .find(|device| device.mac.eq_ignore_ascii_case(mac));
    let off = || {
        model
            .devices_off
            .iter()
            .find(|device| device.mac.eq_ignore_ascii_case(mac))
    };
    on.map(device_name).or_else(|| off().map(off_name))
}

/// Words listing names: Hall, Study and Den.
pub fn listed(names: &[&str]) -> String {
    match names {
        [] => "none".into(),
        [one] => (*one).into(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}

fn find_node<'a>(model: &'a HomeNetwork, name: &str) -> Result<&'a Node, String> {
    model
        .nodes
        .iter()
        .find(|node| node.id == name || node_name(node).eq_ignore_ascii_case(name))
        .ok_or_else(|| {
            let names: Vec<&str> = model.nodes.iter().map(node_name).collect();
            format!("no node is called {name}; the nodes are {}", listed(&names))
        })
}

/// A profile's name, or its id where eero gives none.
fn profile_name(profile: &Profile) -> &str {
    profile.name.as_deref().unwrap_or(&profile.id)
}

fn find_profile<'a>(model: &'a HomeNetwork, name: &str) -> Result<&'a Profile, String> {
    model
        .profiles
        .iter()
        .find(|profile| profile.id == name || profile_name(profile).eq_ignore_ascii_case(name))
        .ok_or_else(|| {
            let names: Vec<&str> = model.profiles.iter().map(profile_name).collect();
            format!(
                "no profile is called {name}; the profiles are {}",
                listed(&names)
            )
        })
}

/// The device on now that a name or hardware address names. One not on now
/// is refused with where and when it last was: by the clock when there is
/// one, as every page says a time, and as eero's own time when not.
fn find_device<'a>(
    model: &'a HomeNetwork,
    name: &str,
    clock: Option<&Clock>,
) -> Result<&'a Device, String> {
    if let Some(device) = model
        .devices
        .iter()
        .find(|d| d.mac.eq_ignore_ascii_case(name))
    {
        return Ok(device);
    }
    let called =
        |device: &&Device| answers_to(name, [&device.name, &device.nickname, &device.hostname]);
    let matches: Vec<&Device> = model.devices.iter().filter(called).collect();
    match matches.as_slice() {
        [one] => Ok(one),
        [] => Err(off_now(model, name, clock)
            .unwrap_or_else(|| format!("no device on now is called {name}"))),
        many => {
            let each: Vec<String> = many
                .iter()
                .map(|d| format!("{} on {}", d.mac, location(model, d.node.as_deref())))
                .collect();
            let each: Vec<&str> = each.iter().map(String::as_str).collect();
            Err(format!(
                "{} devices are called {name}: {}; name one by its hardware address",
                many.len(),
                listed(&each)
            ))
        }
    }
}

/// Why a device eero knows isn't shown: it isn't on now, and where and
/// when it last was.
fn off_now(model: &HomeNetwork, name: &str, clock: Option<&Clock>) -> Option<String> {
    let off = off_device(model, name)?;
    let mut last = Vec::new();
    if let Some(node) = off.node.as_deref() {
        last.push(location(model, Some(node)).to_owned());
    }
    if let Some(at) = off.last_active.as_deref().and_then(clock::parse) {
        last.push(match clock {
            Some(clock) => format!("{}, {}", clock.minute(at), clock.ago(at)),
            None => at.to_string(),
        });
    }
    Some(if last.is_empty() {
        format!("{name} isn't on now")
    } else {
        format!("{name} isn't on now; it was last on {}", last.join(", "))
    })
}

/// Whether any of a device's names is the one asked for, in any case.
fn answers_to(name: &str, names: [&Option<String>; 3]) -> bool {
    names
        .into_iter()
        .flatten()
        .any(|said| said.eq_ignore_ascii_case(name))
}

/// The device not on now that a name or hardware address finds, the first
/// eero lists where several answer to it.
fn off_device<'a>(model: &'a HomeNetwork, name: &str) -> Option<&'a DeviceOff> {
    model.devices_off.iter().find(|device| {
        device.mac.eq_ignore_ascii_case(name)
            || answers_to(name, [&device.name, &device.nickname, &device.hostname])
    })
}

/// Whether a device on now answers to a name, so `eero device` shows it
/// rather than one that's off.
fn on_now_answers_to(model: &HomeNetwork, name: &str) -> bool {
    model.devices.iter().any(|device| {
        device.mac.eq_ignore_ascii_case(name)
            || answers_to(name, [&device.name, &device.nickname, &device.hostname])
    })
}

/// A count and its noun, the noun made plural past one: 1 device, 15 devices.
pub fn counted(count: usize, noun: &str) -> String {
    if count == 1 {
        format!("{count} {noun}")
    } else {
        format!("{count} {noun}s")
    }
}

/// The title of a page about the home network as a whole.
fn titled(model: &HomeNetwork, about: Option<String>) -> Line {
    let mut line = vec![strong(&model.name)];
    if let Some(about) = about {
        line.push(muted(format!(" {} ", marks().separator)));
        line.push(plain(about));
    }
    line
}

/// A line of facts joined by the separator.
fn facts(parts: Vec<Line>) -> Line {
    joined(parts.into_iter().filter(|part| !part.is_empty()).collect())
}

/// A figure after the muted word saying what it is.
fn labelled(label: &str, value: Line) -> Line {
    let mut line = vec![muted(format!("{label} "))];
    line.extend(value);
    line
}

/// One of eero's times, to the minute in the machine's zone, and how long
/// ago it was.
fn when(stamp: Option<&str>, clock: &Clock) -> Line {
    stamp
        .and_then(clock::parse)
        .map_or_else(figures::unknown, |at| {
            vec![
                plain(clock.minute(at)),
                muted(format!(", {}", clock.ago(at))),
            ]
        })
}

#[cfg(test)]
mod golden;
