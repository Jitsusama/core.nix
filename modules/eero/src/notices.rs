//! What the daemon tells Joel of the home network, as desktop notices:
//! each change worth his attention between the model it last compared and
//! the one it has now.
//!
//! - The internet going out, as eero says it or this machine's own look
//!   sees it from home, and coming back, with how long it was out.
//! - A node going offline and coming back online, by its location.
//! - A device eero first saw since its last answer joining, by its name
//!   and the node it joined; one reconnecting says nothing.
//! - A firmware update waiting, and the nodes' firmware changing.
//! - eero's own activity list, where an event says what none of these
//!   does.
//!
//! Only what changed while the daemon watched is told: the first model
//! after it starts, once eero answers, is what later ones are compared
//! with, and says nothing itself. Each change is told once, by a kind and
//! a subject the history records (history/), so a restart, or a figure
//! flapping back to one already told, never tells it again. Nothing
//! changing says nothing.

use std::collections::BTreeMap;

use crate::model::{BackhaulKind, CloudState, Device, Event, HomeNetwork, Node, OutageSeenBy};
use crate::time;
use crate::views::{counted, device_name, listed, node_name, span_on_desktop};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Urgency {
    Normal,
    /// What leaves the home network, or part of it, without a way out.
    Critical,
}

/// What a notice tells of, as the history records it was told.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    InternetOut,
    InternetBack,
    NodeOffline,
    NodeOnline,
    Joined,
    UpdateWaiting,
    Updated,
    Event,
}

impl Kind {
    /// Its name in the history, which never changes once written.
    pub fn name(self) -> &'static str {
        match self {
            Kind::InternetOut => "internet-out",
            Kind::InternetBack => "internet-back",
            Kind::NodeOffline => "node-offline",
            Kind::NodeOnline => "node-online",
            Kind::Joined => "joined",
            Kind::UpdateWaiting => "update-waiting",
            Kind::Updated => "updated",
            Kind::Event => "event",
        }
    }
}

/// One change told, and the subject each change it tells is known by.
pub type Told = (Kind, String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notice {
    /// What it's about, where a later notice says how that ended: the
    /// internet, a node or the firmware. The later one takes its place,
    /// so a notice that the internet is out never outlives its coming
    /// back.
    pub about: Option<String>,
    pub summary: String,
    pub body: String,
    pub urgency: Urgency,
    /// The changes it tells, recorded once it's raised.
    pub tells: Vec<Told>,
}

/// What the history says was told before.
pub trait Said {
    fn said(&self, kind: Kind, subject: &str) -> bool;
    /// When the internet went out, as the newest notice of it said, while
    /// no notice since has said it's back.
    fn out_since(&self) -> Option<String>;
}

/// What a model calls for: the notices to raise, and what to record as
/// told without one.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Next {
    pub notices: Vec<Notice>,
    /// An outage a notice told of that ended while nothing watched, as the
    /// first model after a start finds: recorded as over, so a later
    /// coming back is never put down to it.
    pub settled: Vec<Told>,
}

/// The model notices are worked out against, kept between reads.
#[derive(Debug, Default)]
pub struct Watch {
    /// Whether eero has answered since the daemon started, so the model's
    /// figures are its own and not the last run's.
    answered: bool,
    last: Option<HomeNetwork>,
}

impl Watch {
    /// What `model` calls for, `answered` saying whether eero answered the
    /// reads it was drawn from. Until eero first answers nothing is told,
    /// since the figures are the last run's, and the first model it
    /// answers only sets what later ones are compared with.
    pub fn next(&mut self, model: &HomeNetwork, answered: bool, said: &impl Said) -> Next {
        self.answered |= answered;
        if !self.answered {
            return Next::default();
        }
        let next = match &self.last {
            Some(last) => Next {
                notices: between(last, model, said),
                settled: Vec::new(),
            },
            None => Next {
                notices: Vec::new(),
                settled: match (way(model), said.out_since()) {
                    (Way::Up, Some(since)) => vec![(Kind::InternetBack, since)],
                    _ => Vec::new(),
                },
            },
        };
        self.last = Some(model.clone());
        next
    }
}

/// Every notice the change from `was` to `now` calls for, leaving out
/// what `said` says was told before.
pub fn between(was: &HomeNetwork, now: &HomeNetwork, said: &impl Said) -> Vec<Notice> {
    let mut notices: Vec<Notice> = internet(was, now, said).into_iter().collect();
    notices.extend(nodes(was, now));
    notices.extend(joined(was, now));
    notices.extend(update_waiting(was, now));
    notices.extend(updated(was, now, said));
    notices.extend(events(was, now));
    notices.retain(|notice| {
        !notice
            .tells
            .iter()
            .all(|(kind, subject)| said.said(*kind, subject))
    });
    notices
}

/// How the internet stands in a model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Way {
    Up,
    Out(SeenBy),
    /// Neither eero nor this machine can say: eero isn't answering, and
    /// this machine is away from home.
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeenBy {
    Eero,
    Probe,
}

/// How the internet stands: as eero says while it answers, since it sees
/// the gateway node itself, and otherwise as this machine's last look
/// found it from home.
pub fn way(model: &HomeNetwork) -> Way {
    let internet = &model.internet;
    if model.cloud.state == CloudState::Answering {
        if internet.isp_up == Some(false) {
            return Way::Out(SeenBy::Eero);
        }
        match internet.state.as_deref() {
            Some(state) if state.eq_ignore_ascii_case("connected") => return Way::Up,
            Some(state) if state.eq_ignore_ascii_case("disconnected") => {
                return Way::Out(SeenBy::Eero);
            }
            _ => {}
        }
    }
    match &internet.probe {
        Some(probe) if probe.home && probe.reachable => Way::Up,
        Some(probe) if probe.home => Way::Out(SeenBy::Probe),
        _ => Way::Unknown,
    }
}

/// The latest a model says anything of, eero's answer or this machine's
/// look, since while eero can't be reached only the look moves on.
fn seen(model: &HomeNetwork) -> Option<i64> {
    let look = model.internet.probe.as_ref().map(|probe| probe.at.as_str());
    [Some(model.read.as_str()), look]
        .into_iter()
        .flatten()
        .filter_map(time::to_unix)
        .max()
}

/// Seconds from one moment to a later one, as a length of time a notice
/// says, on the desktop.
fn lasted(from: i64, to: i64) -> String {
    span_on_desktop(u64::try_from(to - from).unwrap_or(0))
}

/// The internet going out or coming back. One already said to be out, and
/// seen out again after a while nothing could tell, is the same outage,
/// and one said to be out comes back from it.
fn internet(was: &HomeNetwork, now: &HomeNetwork, said: &impl Said) -> Option<Notice> {
    let since = said.out_since();
    let about = Some("internet".to_owned());
    let before = way(was);
    match way(now) {
        Way::Out(seen_by) => {
            let fresh = before == Way::Up || (before == Way::Unknown && since.is_none());
            if !fresh {
                return None;
            }
            let start = time::from_unix(seen(now)?);
            Some(Notice {
                about,
                summary: "The internet is out".into(),
                body: match seen_by {
                    SeenBy::Probe => "Nothing past the home network answers this machine.".into(),
                    SeenBy::Eero => eero_says(now),
                },
                urgency: Urgency::Critical,
                tells: vec![(Kind::InternetOut, start)],
            })
        }
        Way::Up => {
            let ended =
                matches!(before, Way::Out(_)) || (before == Way::Unknown && since.is_some());
            if !ended {
                return None;
            }
            let start = since.or_else(|| probe_start(was, now));
            let back = seen(now)?;
            let body = match start.as_deref().and_then(time::to_unix) {
                Some(from) => format!("It was out for {}.", lasted(from, back)),
                None => "How long it was out isn't known.".into(),
            };
            let subject = start.unwrap_or_else(|| time::from_unix(back));
            Some(Notice {
                about,
                summary: "The internet is back".into(),
                body,
                urgency: Urgency::Normal,
                tells: vec![(Kind::InternetBack, subject)],
            })
        }
        Way::Unknown => None,
    }
}

/// What eero says of an internet that's out.
fn eero_says(model: &HomeNetwork) -> String {
    match (model.internet.isp_up, model.internet.state.as_deref()) {
        (Some(false), _) => "eero says the internet provider is down.".into(),
        (_, Some(state)) => format!("eero says the internet is {state}."),
        (_, None) => "eero says the internet is out.".into(),
    }
}

/// When the outage this machine saw began, where it saw the one that
/// just ended: one that ended before `was` is an earlier outage.
fn probe_start(was: &HomeNetwork, now: &HomeNetwork) -> Option<String> {
    let after = seen(was)?;
    now.internet
        .history
        .outages
        .iter()
        .find(|outage| outage.seen_by == OutageSeenBy::Probe)
        .filter(|outage| {
            outage
                .end
                .as_deref()
                .and_then(time::to_unix)
                .is_some_and(|end| end >= after)
        })
        .and_then(|outage| outage.start.clone())
}

fn is(node: &Node, state: &str) -> bool {
    node.state
        .as_deref()
        .is_some_and(|said| said.eq_ignore_ascii_case(state))
}

/// Each node going offline or coming back online. An offline spell is
/// known by the last heartbeat before it, which holds while it lasts.
fn nodes<'a>(was: &'a HomeNetwork, now: &'a HomeNetwork) -> impl Iterator<Item = Notice> + 'a {
    now.nodes.iter().filter_map(move |node| {
        let before = was.nodes.iter().find(|before| before.id == node.id)?;
        let name = node_name(node);
        let about = Some(format!("node {}", node.id));
        if is(before, "online") && is(node, "offline") {
            let heard = node
                .last_heartbeat
                .as_ref()
                .or(before.last_heartbeat.as_ref());
            Some(Notice {
                about,
                summary: format!("{name} is offline"),
                body: offline(before),
                urgency: Urgency::Critical,
                tells: vec![(
                    Kind::NodeOffline,
                    format!("{} {}", node.id, heard.unwrap_or(&now.read)),
                )],
            })
        } else if is(before, "offline") && is(node, "online") {
            let heard = before.last_heartbeat.as_deref();
            let spell = heard
                .and_then(time::to_unix)
                .zip(time::to_unix(&now.read))
                .map(|(from, to)| format!("It was offline for {}. ", lasted(from, to)))
                .unwrap_or_default();
            let devices = node
                .devices
                .all
                .map(|all| on_it(all, "is", "are"))
                .unwrap_or_default();
            Some(Notice {
                about,
                summary: format!("{name} is back online"),
                body: format!("{spell}{devices}").trim_end().to_owned(),
                urgency: Urgency::Normal,
                tells: vec![(
                    Kind::NodeOnline,
                    format!("{} {}", node.id, heard.unwrap_or(&was.read)),
                )],
            })
        } else {
            None
        }
    })
}

/// What a node that's gone offline held up: its devices, and where the
/// internet or its backhaul came from.
fn offline(node: &Node) -> String {
    let devices = node.devices.all.map(|all| on_it(all, "was", "were"));
    let backhaul = if node.gateway {
        Some("The internet comes in through it.".to_owned())
    } else {
        match (node.backhaul.kind, node.backhaul.upstream.as_deref()) {
            (BackhaulKind::WiFi, Some(upstream)) => {
                Some(format!("Its backhaul was Wi-Fi from {upstream}."))
            }
            (BackhaulKind::WiFi, None) => Some("Its backhaul was Wi-Fi.".into()),
            (BackhaulKind::Ethernet, _) => Some("Its backhaul was Ethernet.".into()),
        }
    };
    [devices, backhaul]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" ")
}

/// A count of devices on a node, as a sentence: "4 devices were on it."
fn on_it(count: u32, one: &str, more: &str) -> String {
    let verb = if count == 1 { one } else { more };
    let count = usize::try_from(count).unwrap_or(usize::MAX);
    format!("{} {verb} on it.", counted(count, "device"))
}

/// Each device eero first saw between its last answer and this one: one
/// it saw before, coming back, says nothing.
fn joined<'a>(was: &'a HomeNetwork, now: &'a HomeNetwork) -> impl Iterator<Item = Notice> + 'a {
    let window = time::to_unix(&was.read).zip(time::to_unix(&now.read));
    now.devices.iter().filter_map(move |device| {
        let (from, to) = window?;
        let first = device.first_seen.as_deref()?;
        let at = time::to_unix(first)?;
        let known = was
            .devices
            .iter()
            .any(|before| before.mac.eq_ignore_ascii_case(&device.mac));
        if known || at < from || at > to {
            return None;
        }
        Some(Notice {
            about: None,
            summary: format!("{} joined the home network", device_name(device)),
            body: format!("Never seen before, {}.", on(now, device)),
            urgency: Urgency::Normal,
            tells: vec![(Kind::Joined, format!("{} {first}", device.mac))],
        })
    })
}

/// Where a device is: its node, and over Ethernet or Wi-Fi at its band.
fn on(model: &HomeNetwork, device: &Device) -> String {
    let over = match (device.ethernet, device.band.as_deref()) {
        (true, _) => "over Ethernet".to_owned(),
        (false, Some(band)) => format!("over Wi-Fi at {band}"),
        (false, None) => "over Wi-Fi".to_owned(),
    };
    let node = device
        .node
        .as_deref()
        .and_then(|id| model.nodes.iter().find(|node| node.id == id));
    match node {
        Some(node) => format!("on {} {over}", node_name(node)),
        None => over,
    }
}

/// A firmware update coming to wait, known by the firmware it brings.
fn update_waiting(was: &HomeNetwork, now: &HomeNetwork) -> Option<Notice> {
    let firmware = &now.internet.firmware;
    if firmware.has_update != Some(true) || was.internet.firmware.has_update == Some(true) {
        return None;
    }
    let body = match (&firmware.next, firmware.update_hour) {
        (Some(next), Some(hour)) => format!("eero updates the nodes to {next} at {hour:02}:00."),
        (Some(next), None) => format!("eero updates the nodes to {next}."),
        (None, Some(hour)) => format!("eero updates the nodes at {hour:02}:00."),
        (None, None) => "eero has new firmware for the nodes.".into(),
    };
    Some(Notice {
        about: Some("firmware".into()),
        summary: "A firmware update is waiting".into(),
        body,
        urgency: Urgency::Normal,
        tells: vec![(
            Kind::UpdateWaiting,
            firmware.next.clone().unwrap_or_default(),
        )],
    })
}

/// The nodes' firmware changing, one notice for each firmware they
/// changed to, since an update reaches every node in the same few minutes.
/// A node already said to have changed to it is left out of the notice.
fn updated(was: &HomeNetwork, now: &HomeNetwork, said: &impl Said) -> Vec<Notice> {
    let mut by_firmware: BTreeMap<&str, Vec<(&Node, &str)>> = BTreeMap::new();
    for node in &now.nodes {
        let before = was.nodes.iter().find(|before| before.id == node.id);
        let from = before.and_then(|before| before.firmware.as_deref());
        let (Some(from), Some(to)) = (from, node.firmware.as_deref()) else {
            continue;
        };
        if from != to && !said.said(Kind::Updated, &format!("{} {to}", node.id)) {
            by_firmware.entry(to).or_default().push((node, from));
        }
    }
    by_firmware
        .into_iter()
        .map(|(to, changed)| {
            let names: Vec<&str> = changed.iter().map(|(node, _)| node_name(node)).collect();
            let mut froms: Vec<&str> = changed.iter().map(|(_, from)| *from).collect();
            froms.sort_unstable();
            froms.dedup();
            let who = if changed.len() > 1 && changed.len() == now.nodes.len() {
                "The nodes".to_owned()
            } else {
                listed(&names)
            };
            Notice {
                about: Some("firmware".into()),
                summary: format!("{who} updated to {to}"),
                // A whole sentence, as every body is, since the drawer and
                // `eero events` read it after the summary.
                body: format!(
                    "{} on {}.",
                    if changed.len() > 1 {
                        "They were"
                    } else {
                        "It was"
                    },
                    listed(&froms)
                ),
                urgency: Urgency::Normal,
                tells: changed
                    .iter()
                    .map(|(node, _)| (Kind::Updated, format!("{} {to}", node.id)))
                    .collect(),
            }
        })
        .collect()
}

/// The most events one read tells, so a burst eero gives after a while
/// unreachable can't bury the notices.
const EVENTS_TOLD: usize = 3;

/// What eero's activity list says of a device or a node coming, going or
/// updating, which the changes above tell from eero's own figures: a
/// device's event names it connecting to or disconnecting from a node,
/// and a node's or the network's, words such as these.
const TOLD_OTHERWISE: [&str; 8] = [
    "offline",
    "online",
    "connected",
    "joined",
    "internet",
    "update",
    "firmware",
    "reboot",
];

/// The events eero added to its activity list since the last model, where
/// they say what nothing else here does. An activity list first seen sets
/// what's new and says nothing itself.
fn events<'a>(was: &'a HomeNetwork, now: &'a HomeNetwork) -> impl Iterator<Item = Notice> + 'a {
    let newest = was
        .events
        .iter()
        .filter_map(|event| time::to_unix(&event.at))
        .max();
    now.events
        .iter()
        .filter(move |event| {
            newest.is_some_and(|newest| time::to_unix(&event.at).is_some_and(|at| at >= newest))
                && !was.events.contains(event)
                && says_more(event)
        })
        .take(EVENTS_TOLD)
        .map(|event| Notice {
            about: None,
            summary: event.text.clone(),
            body: format!("eero's activity list, under {}.", event.kind),
            urgency: Urgency::Normal,
            tells: vec![(Kind::Event, format!("{} {}", event.at, event.text))],
        })
}

fn says_more(event: &Event) -> bool {
    let text = event.text.to_lowercase();
    !event.kind.eq_ignore_ascii_case("device")
        && !TOLD_OTHERWISE.iter().any(|word| text.contains(*word))
}

#[cfg(test)]
#[path = "notices_tests.rs"]
mod tests;
