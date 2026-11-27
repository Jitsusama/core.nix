//! What wifid knows that the shell shows: each card, the network it's on and
//! how far that reaches, every saved network, and the networks in the air.
//! It's kept whole, so the
//! shell is sent all of it each time any of it changes and never has to
//! piece it together.

use std::collections::{BTreeMap, BTreeSet};
use std::net::Ipv4Addr;

use serde::{Deserialize, Serialize};
use tokio::time::{Duration, Instant};
use zlink::introspect::{CustomType, Type};

use crate::air::Heard;
use crate::check::Verdict;
use crate::detail::{self, Detail, Polled};
use crate::link::Link;
use crate::network::Security as Kind;
use crate::service::Refused;
use crate::store;

/// How many of a card's signal reads are kept, as many as a spark draws.
const SEEN: usize = 20;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {
    /// By link index, which stays the same while the card is there.
    cards: BTreeMap<i32, Card>,
    saved: Vec<Saved>,
    /// By the access point's path in wpa_supplicant, which names its card.
    heard: BTreeMap<String, Heard>,
    /// When a look last heard each of those, by the same path: what each
    /// says is as of then, and its age grows from it.
    heard_when: BTreeMap<String, Instant>,
    radio_off: bool,
    wired: bool,
    /// How many times each card, by name, has begun joining a network,
    /// and which of those its trouble came in, since wpa_supplicant can say
    /// how one attempt went after the next has been asked for.
    attempts: BTreeMap<String, u64>,
    troubled_in: BTreeMap<String, u64>,
    /// The cards, by name, short of a completed join since they were last
    /// joined. A card associates before its handshake, so it's at a new
    /// point while a move there may still fail.
    joining: BTreeSet<String>,
    /// The cards, by name, that have looked for networks since wifid was
    /// given them.
    looked: BTreeSet<String>,
    /// Whether each card, by name, is looking for networks now, and how
    /// many looks it has begun. A look takes the card off its channel, so
    /// a call's echoes lost meanwhile aren't its link fading (fading.rs),
    /// and one begun and ended between two reads still shows in the count.
    scanning: BTreeMap<String, Looking>,
    /// How many looks the cards have finished, as the shell is told.
    looks: u32,
    /// What each card's driver last read of its link, by the card's name,
    /// while the shell asks for a network in detail.
    polled: BTreeMap<String, Polled>,
    /// The signal each card's driver read at each of those polls, in dBm,
    /// the newest last, as many as a spark draws: so the signal's graph
    /// holds only what was read while someone looked.
    seen: BTreeMap<String, Vec<i32>>,
    /// The access point each card is joined to, by the card's name, as a
    /// path in wpa_supplicant.
    at: BTreeMap<String, String>,
    /// The gateway each card's default route goes through, by link index.
    gateways: BTreeMap<i32, Ipv4Addr>,
    /// The access point that last warned each card it drops it, by the
    /// card's name.
    warned_off: BTreeMap<String, Warning>,
}

/// An access point's warning that it drops a card, when it warned, and
/// until when the card's chooser isn't told of the point.
#[derive(Clone, Debug, PartialEq)]
struct Warning {
    point: String,
    /// When it warned this wifid, so that its heed is under way; none
    /// for one left to wpa_supplicant, which wifid doesn't move, or kept
    /// from before a restart, whose heed ended with it.
    at: Option<Instant>,
    until: Instant,
    /// Whether the card has since been on another point, so that being on
    /// this one again is a return rather than the stay the warning began
    /// in.
    left: bool,
}

/// A card on a network through an access point it hears, and the gateway
/// it reaches through it: what's echoed to measure the access point.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Joined {
    pub card: String,
    /// The access point's address, as 02:00:00:00:01:00.
    pub point: String,
    pub gateway: Ipv4Addr,
    /// The network's name, and whether it's saved as trusted.
    pub network: String,
    pub trusted: bool,
}

/// Every card, every saved network and the networks in the air, as the
/// shell is sent them.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Type)]
pub struct Described {
    pub cards: Vec<Card>,
    pub saved: Vec<Saved>,
    /// The strongest first.
    pub nearby: Vec<Nearby>,
    /// Whether Wi-Fi's radios are on.
    pub radio: bool,
    /// Whether a cable reaches beyond the machine.
    pub wired: bool,
    /// How many looks for networks the cards have finished, so whoever
    /// asked for one knows when it's done: what a look hears is described
    /// before it, and one that hears nothing changes nothing else.
    pub looks: u32,
}

/// Whether a card is looking for networks, and how many looks it's begun.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Looking {
    pub now: bool,
    pub begun: u32,
}

impl Looking {
    /// Whether the card looked at all between this read and that later one.
    pub fn looked_until(self, later: Looking) -> bool {
        self.now || later.now || self.begun != later.begun
    }
}

/// A Wi-Fi card.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, CustomType)]
#[serde(rename_all = "camelCase")]
#[zlink(rename_all = "camelCase")]
pub struct Card {
    /// Its name, as wlan0.
    pub name: String,
    /// The network it's on, once it has joined one.
    pub network: Option<String>,
    /// How far that network reaches, once it has given the card an address.
    pub reach: Option<Reach>,
    /// Where to sign in, while the network wants it.
    pub sign_in: Option<String>,
    /// What went wrong the last time it tried a network, until one gives it
    /// an address.
    pub trouble: Option<Trouble>,
}

/// What went wrong joining a network.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, CustomType)]
#[serde(rename_all = "camelCase")]
#[zlink(rename_all = "camelCase")]
pub enum Trouble {
    /// The password didn't work, which on a network joined before means it
    /// has changed.
    WrongPassword,
    /// The access point turned the card away.
    TurnedAway,
    /// The network's server couldn't prove it's the network's, so it was
    /// given no password.
    ServerUnproven,
}

/// How far a card has got joining a network.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Progress {
    /// It's finding the network and agreeing how to join it.
    Joining,
    /// It's on the network and waiting for an address.
    GettingAddress,
    /// It has an address, and wifid is finding out how far it reaches.
    Checking,
    /// It's on the network, which reaches so far.
    Reached(Reach),
    /// It couldn't join it.
    Troubled(Trouble),
    /// The card has gone.
    Gone,
}

/// How far a network reaches.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, CustomType)]
#[serde(rename_all = "camelCase")]
#[zlink(rename_all = "camelCase")]
pub enum Reach {
    /// wifid is finding out.
    Checking,
    /// Beyond itself.
    Online,
    /// Nowhere until someone signs in.
    SignIn,
    /// Nowhere, or nowhere sensible.
    Limited,
}

/// A saved network.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, CustomType)]
#[serde(rename_all = "camelCase")]
#[zlink(rename_all = "camelCase")]
pub struct Saved {
    pub name: String,
    pub security: Security,
    /// Whether Nix declares it, so only a rebuild changes or forgets it.
    pub declared: bool,
    /// Whether it's trusted to find the machine's services, and they it.
    pub trusted: bool,
    /// The highest is joined first.
    pub priority: i32,
    /// Whether it doesn't announce itself.
    pub hidden: bool,
}

/// A network in the air, as its strongest access point is heard.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, CustomType)]
#[serde(rename_all = "camelCase")]
#[zlink(rename_all = "camelCase")]
pub struct Nearby {
    pub name: String,
    pub security: Security,
    /// Bars of signal, from 1 to 4.
    pub strength: u8,
}

/// How a network is joined.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, CustomType)]
#[serde(rename_all = "camelCase")]
#[zlink(rename_all = "camelCase")]
pub enum Security {
    /// By anyone.
    Open,
    /// With a password.
    Personal,
    /// With an account, as at work.
    Enterprise,
}

impl State {
    /// Takes in what networkd says of a card's link. A card that has left
    /// its network, or lost its way beyond it, no longer has a reach.
    pub fn follow(&mut self, link: &Link) {
        let card = self.cards.entry(link.index).or_insert_with(|| Card {
            name: link.name.clone(),
            network: None,
            reach: None,
            sign_in: None,
            trouble: None,
        });
        if card.network != link.ssid || !link.routable {
            (card.reach, card.sign_in) = (None, None);
        }
        card.name.clone_from(&link.name);
        card.network.clone_from(&link.ssid);
        match link.gateway {
            Some(gateway) => self.gateways.insert(link.index, gateway),
            None => self.gateways.remove(&link.index),
        };
    }

    /// Forgets every card but these, which are all there are.
    pub fn keep_only(&mut self, indexes: &[i32]) {
        let gone = self
            .cards
            .iter()
            .filter(|(index, _)| !indexes.contains(index));
        let gone: Vec<String> = gone.map(|(_, card)| card.name.clone()).collect();
        self.cards.retain(|index, _| indexes.contains(index));
        self.gateways.retain(|index, _| indexes.contains(index));
        let names: Vec<&String> = self.cards.values().map(|card| &card.name).collect();
        self.looked.retain(|card| names.contains(&card));
        self.scanning.retain(|card, _| names.contains(&card));
        // Only a card that goes takes its access point, since wifid hands
        // cards over, and asks where each is, before it follows any.
        self.at.retain(|card, _| !gone.contains(card));
    }

    /// Says wifid is finding out how far a card's network reaches, unless
    /// it already knew: a network checked again keeps its reach meanwhile.
    pub fn checking(&mut self, index: i32) {
        if let Some(card) = self.cards.get_mut(&index) {
            card.reach.get_or_insert(Reach::Checking);
            // Only an address says a join got through: networkd names the
            // network as the card associates, before a wrong password
            // is found out.
            card.trouble = None;
        }
    }

    /// Takes in what the check found of a card's network.
    pub fn judged(&mut self, index: i32, verdict: &Verdict) {
        let Some(card) = self.cards.get_mut(&index) else {
            return;
        };
        (card.reach, card.sign_in) = match verdict {
            Verdict::Online => (Some(Reach::Online), None),
            Verdict::SignIn { page } => (Some(Reach::SignIn), Some(page.clone())),
            Verdict::Limited => (Some(Reach::Limited), None),
        };
    }

    /// Takes in the saved networks, in the order the store gives them. Of
    /// two of one name, the first is the one joined, so it's the one kept.
    pub fn saved(&mut self, saved: &[store::Saved]) {
        self.saved.clear();
        for store::Saved {
            network, declared, ..
        } in saved
        {
            if self.saved.iter().any(|shown| shown.name == network.name) {
                continue;
            }
            self.saved.push(Saved {
                name: network.name.clone(),
                security: match network.security {
                    Kind::Open => Security::Open,
                    Kind::Personal => Security::Personal,
                    Kind::Enterprise => Security::Enterprise,
                },
                declared: *declared,
                trusted: network.trusted,
                priority: network.priority,
                hidden: network.hidden,
            });
        }
    }

    /// Takes in whether Wi-Fi's radios are on.
    pub fn radio(&mut self, on: bool) {
        self.radio_off = !on;
        // What a card heard before is no guide to what it'll hear now.
        if !on {
            self.looked.clear();
        }
    }

    /// Takes in whether a cable reaches beyond the machine.
    pub fn wired(&mut self, wired: bool) {
        self.wired = wired;
    }

    /// Says what went wrong on the card of that name, or clears it.
    pub fn troubled(&mut self, card: &str, trouble: Option<Trouble>) {
        let attempt = self.attempts(card);
        let name = card;
        if let Some(card) = self.cards.values_mut().find(|shown| shown.name == card) {
            // wpa_supplicant says the join failed after saying the server
            // couldn't prove itself, and the first is the one that says why.
            let unproven = card.trouble == Some(Trouble::ServerUnproven);
            if unproven && trouble == Some(Trouble::WrongPassword) {
                return;
            }
            card.trouble = trouble;
            match trouble {
                Some(_) => self.troubled_in.insert(name.to_string(), attempt),
                None => self.troubled_in.remove(name),
            };
        }
    }

    /// Takes in that the card of that name has begun joining a network.
    pub fn attempting(&mut self, card: &str) {
        *self.attempts.entry(card.to_string()).or_default() += 1;
        self.joining(card);
    }

    /// Takes in that the card of that name is short of a completed join,
    /// begun or not as wifid saw it: one first seen partway, or found
    /// partway as wifid starts, hasn't left its point until it lands.
    pub fn joining(&mut self, card: &str) {
        self.joining.insert(card.to_owned());
    }

    /// Takes in that the card of that name has finished joining, at the
    /// point it's at: only then has it left one point for another.
    pub fn landed(&mut self, card: &str) {
        self.joining.remove(card);
        self.settle_warnings();
    }

    /// Takes in that the card of that name has finished joining, at the
    /// point the step that says so names beside it, or none. Its point is
    /// taken in first, as wpa_supplicant sends both in one signal when
    /// they come within 5 ms: settled against the point it was joining
    /// before, a card landing elsewhere could count as back where it was
    /// warned, and forget the warning.
    pub fn landed_at(&mut self, card: &str, point: Option<String>) {
        self.at(card, point);
        self.landed(card);
    }

    /// How many times the card of that name has begun joining a network.
    pub fn attempts(&self, card: &str) -> u64 {
        self.attempts.get(card).copied().unwrap_or_default()
    }

    /// How far the card of that name has got joining the network.
    pub fn progress(&self, card: &str, network: &str, since: u64) -> Progress {
        // Only what went wrong in an attempt begun since the join began is
        // the join's.
        let own = self
            .troubled_in
            .get(card)
            .is_some_and(|attempt| *attempt > since);
        let Some(card) = self.cards.values().find(|shown| shown.name == card) else {
            return Progress::Gone;
        };
        let trouble = card.trouble.filter(|_| own);
        let on = card.network.as_deref() == Some(network);
        match (on, card.reach, trouble) {
            (true, Some(Reach::Checking), _) => Progress::Checking,
            (true, Some(reach), _) => Progress::Reached(reach),
            (_, _, Some(trouble)) => Progress::Troubled(trouble),
            (true, None, None) => Progress::GettingAddress,
            (false, _, None) => Progress::Joining,
        }
    }

    /// Takes in what wpa_supplicant says of an access point, or that it's
    /// no longer heard, and when a look last heard it, from its age now.
    pub fn heard(&mut self, path: &str, heard: Option<Heard>) {
        if let Some(heard) = heard {
            let now = Instant::now();
            let ago = Duration::from_secs(heard.age.into());
            let when = now.checked_sub(ago).unwrap_or(now);
            self.heard_when.insert(path.into(), when);
            self.heard.insert(path.into(), heard);
        } else {
            self.heard_when.remove(path);
            self.heard.remove(path);
        }
        self.settle_warnings();
    }

    /// Forgets every access point whose path starts so, as every one a card
    /// heard when it goes.
    pub fn unheard_under(&mut self, path: &str) {
        self.heard.retain(|heard, _| !heard.starts_with(path));
        self.heard_when.retain(|heard, _| !heard.starts_with(path));
    }

    /// How many looks have ended since wifid started, heard or failed, so
    /// whoever asked for one knows when it's done.
    pub fn looks(&self) -> u32 {
        self.looks
    }

    /// Takes in that the card of that name has looked for networks.
    pub fn looked(&mut self, card: &str) {
        self.looked.insert(card.to_string());
        self.look_ended();
    }

    /// Takes in whether the card is looking for networks now, as
    /// wpa_supplicant says.
    pub fn scanning(&mut self, card: &str, scanning: bool) {
        let looking = self.scanning.entry(card.to_owned()).or_default();
        if scanning && !looking.now {
            looking.begun = looking.begun.wrapping_add(1);
        }
        looking.now = scanning;
    }

    /// Whether the card is looking for networks now, and how many looks
    /// it has begun.
    pub fn looking(&self, card: &str) -> Looking {
        self.scanning.get(card).copied().unwrap_or_default()
    }

    /// Takes in that a card's look ended, heard or failed: one that failed
    /// hasn't looked, so settles nothing, but whoever asked for it is done
    /// waiting.
    pub fn look_ended(&mut self) {
        self.looks = self.looks.wrapping_add(1);
    }

    /// Whether the machine is as online as it's going to get for now: a
    /// card or cable reaches beyond the machine, Wi-Fi is off, or every
    /// card has looked and heard no saved network it hasn't failed to join.
    pub fn settled(&self) -> bool {
        let reaching = self.cards.values().any(|card| card.reach.is_some());
        if self.wired || self.radio_off || reaching {
            return true;
        }
        // A card can appear a moment after wifid starts, as its driver
        // loads, so having none settles nothing.
        let cards = self.cards.values();
        let looked =
            !self.cards.is_empty() && cards.clone().all(|card| self.looked.contains(&card.name));
        let saved = |heard: &Heard| self.saved.iter().any(|saved| saved.name == heard.name);
        let joinable = self.heard.values().any(saved);
        let failed = cards.into_iter().all(|card| card.trouble.is_some());
        looked && (!joinable || failed)
    }

    /// Takes in what a card's driver read of its link, or that nobody asks
    /// any more.
    pub fn polled(&mut self, card: &str, polled: Option<Polled>) {
        let Some(polled) = polled else {
            self.polled.remove(card);
            self.seen.remove(card);
            return;
        };
        if let Some(signal) = polled.signal {
            let seen = self.seen.entry(card.into()).or_default();
            seen.push(signal);
            let over = seen.len().saturating_sub(SEEN);
            seen.drain(..over);
        }
        self.polled.insert(card.into(), polled);
    }

    /// The signal each card's read since someone looked, newest last.
    pub fn seen(&self) -> &BTreeMap<String, Vec<i32>> {
        &self.seen
    }

    /// Takes in the access point a card is joined to, or that it's joined
    /// to none.
    pub fn at(&mut self, card: &str, point: Option<String>) {
        match point {
            Some(point) => self.at.insert(card.into(), point),
            None => self.at.remove(card),
        };
        self.settle_warnings();
    }

    /// Forgets the warning of a point a card has left and come back to,
    /// as being held there again shows the warning is over: kept, a later
    /// trial from it couldn't go back to it until its long keep-off ran
    /// out. A card between points, on none or still joining one, hasn't
    /// left, since a move that fails, before or after it associates, puts
    /// it back where it was warned.
    fn settle_warnings(&mut self) {
        let points: BTreeMap<String, Option<String>> = self
            .warned_off
            .keys()
            .filter(|card| !self.joining.contains(*card))
            .map(|card| (card.clone(), self.point_of(card)))
            .collect();
        self.warned_off.retain(|card, warning| {
            let Some(Some(on)) = points.get(card) else {
                return true;
            };
            let back = on.eq_ignore_ascii_case(&warning.point);
            warning.left |= !back;
            !(back && warning.left)
        });
    }

    /// Lets go of every card's reading, as nobody asks any more.
    pub fn unpolled(&mut self) {
        self.polled.clear();
        self.seen.clear();
    }

    /// The card to move to an access point, by the access point's address:
    /// the one that hears it and is on its network, since a card roams only
    /// within the network it's on.
    pub fn roamer(&self, bssid: &str) -> Result<String, Refused> {
        let mut heard = self
            .heard
            .iter()
            .filter(|(_, heard)| heard.bssid.eq_ignore_ascii_case(bssid))
            .peekable();
        if heard.peek().is_none() {
            return Err(Refused::NotNearby);
        }
        heard
            .find_map(|(path, point)| {
                let by = card_of(path)?;
                let on = |card: &&Card| card.network.as_deref() == Some(point.name.as_str());
                let at = |card: &&Card| self.at.get(&card.name).map(String::as_str);
                let listening = |card: &&Card| at(card).and_then(card_of) == Some(by);
                let card = self
                    .cards
                    .values()
                    .find(|card| on(card) && listening(card))?;
                Some(card.name.clone())
            })
            .ok_or(Refused::NotJoined)
    }

    /// The access points of the network a card is on that it hears, the
    /// one it's on among them, for choosing between them (choose.rs).
    pub fn around(&self, card: &str) -> Vec<Heard> {
        self.around_by_path(card)
            .map(|(_, heard)| heard.clone())
            .collect()
    }

    /// What `around` says, each by its path.
    fn around_by_path<'a>(&'a self, card: &str) -> impl Iterator<Item = (&'a str, &'a Heard)> {
        let on = self
            .at
            .get(card)
            .and_then(|path| Some((self.heard.get(path)?, card_of(path)?)));
        self.heard.iter().filter_map(move |(other, heard)| {
            let (on, under) = on?;
            (card_of(other) == Some(under) && heard.name == on.name)
                .then_some((other.as_str(), heard))
        })
    }

    /// What a card hears around it, as `around` says, for its chooser to
    /// choose from at `now`: less the point that warned it drops the card,
    /// until `warned_off` says, unless the card is on that point again.
    /// The chooser hears nothing of a warning itself, so the minute after
    /// a warned move it would otherwise send the card straight back to a
    /// point that carried better and drops it again.
    ///
    /// Each one's age is as of `now`, from when a look last heard it, as
    /// wpa_supplicant says an age only when a look hears the point again:
    /// a call fading minutes after a look would otherwise take that look's
    /// level for one read just now (fading.rs).
    pub fn around_to_choose(&self, card: &str, now: Instant) -> Vec<Heard> {
        let warned = self.warned_from(card, now);
        self.around_by_path(card)
            .filter(|(_, heard)| {
                warned.is_none_or(|point| !heard.bssid.eq_ignore_ascii_case(point))
            })
            .map(|(path, heard)| {
                let mut heard = heard.clone();
                if let Some(when) = self.heard_when.get(path) {
                    let ago = now.saturating_duration_since(*when).as_secs();
                    heard.age = u32::try_from(ago).unwrap_or(u32::MAX);
                }
                heard
            })
            .collect()
    }

    /// The access point a card keeps off at `now` as it warned it drops
    /// the card, until `warned_off` says, unless the card is on it again.
    /// A point merely unheard isn't kept off: a move there looks for it.
    pub fn warned_from(&self, card: &str, now: Instant) -> Option<&str> {
        let Warning { point, until, .. } = self.warned_off.get(card)?;
        let away = self
            .point_of(card)
            .is_none_or(|on| !on.eq_ignore_ascii_case(point));
        (now < *until && away).then_some(point.as_str())
    }

    /// Keeps the access point that warned a card it drops it from the
    /// card's chooser until then, in place of any it was warned by
    /// before. `at` is when it warned, where its heed moves the card
    /// (`Heed::moves`); none where it's left to wpa_supplicant.
    pub fn warned_off(&mut self, card: &str, point: &str, (at, until): (Option<Instant>, Instant)) {
        let warning = Warning {
            point: point.to_owned(),
            at,
            until,
            left: false,
        };
        self.warned_off.insert(card.to_owned(), warning);
        self.settle_warnings();
    }

    /// Keeps the access point that warned a card before a restart from
    /// the card's chooser until then, as `warned_off` does for one heard
    /// now, but with no heed under way: the wifid that heeded it is gone.
    pub fn kept_off(&mut self, card: &str, point: &str, until: Instant) {
        self.warned_off(card, point, (None, until));
    }

    /// When the access point a card is on warned it drops the card, if it
    /// did since wifid began, its heed moves the card, and that's still
    /// kept to at `now`: the warning's own heed is then moving the card
    /// off it, or looking first. One let go to wpa_supplicant, or kept
    /// from before a restart, has no heed of wifid's under way.
    pub fn warned_on(&self, card: &str, now: Instant) -> Option<Instant> {
        let Warning {
            point, at, until, ..
        } = self.warned_off.get(card)?;
        let on = self
            .point_of(card)
            .is_some_and(|on| on.eq_ignore_ascii_case(point));
        at.filter(|_| now < *until && on)
    }

    /// The address of the access point the card is at, where it hears it,
    /// whether or not it has a gateway yet.
    pub fn point_of(&self, card: &str) -> Option<String> {
        let path = self.at.get(card)?;
        Some(self.heard.get(path)?.bssid.clone())
    }

    /// How the network a card is on is joined, as it's saved: wpa_supplicant
    /// joins a network only as it's saved to be, whatever a point of it
    /// advertises, and anyone in range can advertise as the point.
    pub fn link_of(&self, card: &str) -> Option<Security> {
        let card = self.cards.values().find(|shown| shown.name == card)?;
        let network = card.network.as_deref()?;
        let saved = self.saved.iter().find(|saved| saved.name == network)?;
        Some(saved.security)
    }

    /// Each card on a network through an access point it hears, with a
    /// gateway to echo.
    pub fn joined(&self) -> Vec<Joined> {
        let joined = self.cards.iter().filter(|(_, card)| card.network.is_some());
        joined
            .filter_map(|(index, card)| {
                let path = self.at.get(&card.name)?;
                let network = card.network.clone()?;
                let saved = self.saved.iter().find(|saved| saved.name == network);
                Some(Joined {
                    card: card.name.clone(),
                    point: self.heard.get(path)?.bssid.clone(),
                    gateway: *self.gateways.get(index)?,
                    trusted: saved.is_some_and(|saved| saved.trusted),
                    network,
                })
            })
            .collect()
    }

    /// The saved networks' names.
    pub fn saved_names(&self) -> Vec<String> {
        self.saved.iter().map(|saved| saved.name.clone()).collect()
    }

    /// Whether a card of that name is there, joined or not.
    pub fn has_card(&self, name: &str) -> bool {
        self.cards.values().any(|card| card.name == name)
    }

    /// The cards on a network, by name, which are those worth polling.
    pub fn on_networks(&self) -> Vec<String> {
        let on = self.cards.values().filter(|card| card.network.is_some());
        on.map(|card| card.name.clone()).collect()
    }

    /// A network in the air in detail, with what each card on it read of
    /// its link last.
    pub fn detail(&self, name: &str) -> Detail {
        let heard = self
            .heard
            .iter()
            .map(|(path, heard)| (path.as_str(), heard));
        let on = self
            .cards
            .values()
            .filter(|card| card.network.as_deref() == Some(name));
        let at = on.clone().filter_map(|card| self.at.get(&card.name));
        let at = at.map(String::as_str);
        let polled = on.filter_map(|card| {
            let polled = self.polled.get(&card.name)?;
            Some((card.name.as_str(), polled))
        });
        detail::detail(name, heard, at, polled)
    }

    pub fn described(&self) -> Described {
        Described {
            cards: self.cards.values().cloned().collect(),
            saved: self.saved.clone(),
            nearby: self.nearby(),
            radio: !self.radio_off,
            wired: self.wired,
            looks: self.looks,
        }
    }

    fn nearby(&self) -> Vec<Nearby> {
        let mut strongest: BTreeMap<&str, &Heard> = BTreeMap::new();
        for heard in self.heard.values() {
            let kept = strongest.entry(&heard.name).or_insert(heard);
            if heard.signal > kept.signal {
                *kept = heard;
            }
        }
        let mut nearby: Vec<Nearby> = strongest
            .into_values()
            .map(|heard| Nearby {
                name: heard.name.clone(),
                security: heard.security,
                strength: bars(heard.signal),
            })
            .collect();
        // Already in order of name, which a stable sort keeps among equals.
        nearby.sort_by_key(|network| std::cmp::Reverse(network.strength));
        nearby
    }
}

/// The bars a signal is drawn with: one more for every 10 dB above -80.
pub fn bars(signal: i16) -> u8 {
    match signal {
        -60.. => 4,
        -70.. => 3,
        -80.. => 2,
        _ => 1,
    }
}

/// The card an access point's path is under, since its path in
/// `wpa_supplicant` begins with its card's.
fn card_of(path: &str) -> Option<&str> {
    path.split_once("/BSSs/").map(|(card, _)| card)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::network::{Address, Network};

    fn inn(index: i32) -> Link {
        Link {
            index,
            name: format!("wlan{index}"),
            ssid: Some("Roadside Inn".into()),
            routable: true,
            dns: Vec::new(),
            portal: None,
            gateway: Some(Ipv4Addr::new(10, 43, 0, 1)),
        }
    }

    fn heard(name: &str, signal: i16) -> Heard {
        Heard {
            name: name.into(),
            security: Security::Personal,
            signal,
            bssid: "02:00:00:00:01:00".into(),
            frequency: 2437,
            age: 0,
            radio: crate::radio::Radio::default(),
        }
    }

    fn nearby(state: &State) -> Vec<(String, u8)> {
        let nearby = state.described().nearby.into_iter();
        nearby
            .map(|network| (network.name, network.strength))
            .collect()
    }

    fn card(state: &State) -> Card {
        state.described().cards.remove(0)
    }

    fn saved(name: &str, declared: bool, priority: i32) -> store::Saved {
        store::Saved {
            network: Network {
                name: name.into(),
                security: Kind::Personal,
                secret: None,
                priority,
                hidden: false,
                address: Address::Network,
                trusted: false,
                account: None,
            },
            file: PathBuf::from(format!("/{name}.network")),
            declared,
        }
    }

    #[test]
    fn a_card_is_shown_with_its_network_and_how_far_that_reaches() {
        let mut state = State::default();

        state.follow(&inn(2));
        state.checking(2);
        let checking = card(&state);
        state.judged(
            2,
            &Verdict::SignIn {
                page: "http://front-desk.inn/".into(),
            },
        );
        let judged = card(&state);

        assert_eq!(
            checking,
            Card {
                name: "wlan2".into(),
                network: Some("Roadside Inn".into()),
                reach: Some(Reach::Checking),
                sign_in: None,
                trouble: None,
            }
        );
        assert_eq!(judged.reach, Some(Reach::SignIn));
        assert_eq!(judged.sign_in.as_deref(), Some("http://front-desk.inn/"));
    }

    #[test]
    fn a_card_joining_a_network_goes_from_finding_it_to_how_far_it_reaches() {
        let mut state = State::default();
        state.follow(&Link {
            ssid: None,
            routable: false,
            ..inn(2)
        });
        let joining = state.progress("wlan2", "Roadside Inn", 0);
        state.follow(&Link {
            routable: false,
            ..inn(2)
        });
        let addressing = state.progress("wlan2", "Roadside Inn", 0);
        state.follow(&inn(2));
        state.checking(2);
        let checking = state.progress("wlan2", "Roadside Inn", 0);
        state.judged(2, &Verdict::Online);
        let reached = state.progress("wlan2", "Roadside Inn", 0);

        assert_eq!(
            [joining, addressing, checking, reached],
            [
                Progress::Joining,
                Progress::GettingAddress,
                Progress::Checking,
                Progress::Reached(Reach::Online),
            ]
        );
    }

    #[test]
    fn wifi_is_shown_on_until_its_radios_are_turned_off() {
        let mut state = State::default();
        let at_first = state.described().radio;

        state.radio(false);

        assert!(at_first);
        assert!(!state.described().radio);
    }

    #[test]
    fn a_cable_is_shown_while_it_reaches_beyond_the_machine() {
        let mut state = State::default();

        state.wired(true);

        assert!(state.described().wired);
    }

    #[test]
    fn a_card_still_on_another_network_is_still_joining() {
        let mut state = State::default();
        state.follow(&inn(2));
        state.judged(2, &Verdict::Online);

        let progress = state.progress("wlan2", "Lantern", 0);

        assert_eq!(progress, Progress::Joining);
    }

    #[test]
    fn a_card_that_couldn_t_join_says_why() {
        let mut state = State::default();
        state.follow(&Link {
            ssid: None,
            routable: false,
            ..inn(2)
        });
        state.attempting("wlan2");

        state.troubled("wlan2", Some(Trouble::WrongPassword));

        assert_eq!(card(&state).trouble, Some(Trouble::WrongPassword));
        assert_eq!(
            state.progress("wlan2", "Lantern", 0),
            Progress::Troubled(Trouble::WrongPassword)
        );
    }

    #[test]
    fn trouble_is_forgotten_once_a_network_gives_the_card_an_address() {
        let mut state = State::default();
        state.follow(&Link {
            ssid: None,
            routable: false,
            ..inn(2)
        });
        state.troubled("wlan2", Some(Trouble::TurnedAway));

        state.follow(&inn(2));
        let associated = card(&state).trouble;
        state.checking(2);

        assert_eq!(associated, Some(Trouble::TurnedAway));
        assert_eq!(card(&state).trouble, None);
    }

    #[test]
    fn a_wrong_password_found_out_on_the_network_is_still_trouble() {
        let mut state = State::default();
        state.follow(&Link {
            ssid: Some("Lantern".into()),
            routable: false,
            ..inn(2)
        });
        state.attempting("wlan2");

        state.troubled("wlan2", Some(Trouble::WrongPassword));

        assert_eq!(
            state.progress("wlan2", "Lantern", 0),
            Progress::Troubled(Trouble::WrongPassword)
        );
    }

    #[test]
    fn trouble_said_late_of_an_attempt_before_the_join_isn_t_the_join_s() {
        let mut state = State::default();
        state.follow(&Link {
            ssid: None,
            routable: false,
            ..inn(2)
        });
        state.attempting("wlan2");
        let since = state.attempts("wlan2");

        state.troubled("wlan2", Some(Trouble::WrongPassword));
        let late = state.progress("wlan2", "Lantern", since);
        state.attempting("wlan2");
        state.troubled("wlan2", Some(Trouble::WrongPassword));
        let own = state.progress("wlan2", "Lantern", since);

        assert_eq!(late, Progress::Joining);
        assert_eq!(own, Progress::Troubled(Trouble::WrongPassword));
    }

    #[test]
    fn a_server_that_couldn_t_prove_itself_isn_t_taken_for_a_wrong_password() {
        let mut state = State::default();
        state.follow(&Link {
            ssid: None,
            routable: false,
            ..inn(2)
        });

        state.troubled("wlan2", Some(Trouble::ServerUnproven));
        state.troubled("wlan2", Some(Trouble::WrongPassword));

        assert_eq!(card(&state).trouble, Some(Trouble::ServerUnproven));
    }

    #[test]
    fn a_card_that_has_gone_joins_nothing() {
        let state = State::default();

        assert_eq!(state.progress("wlan2", "Lantern", 0), Progress::Gone);
    }

    #[test]
    fn cards_are_shown_in_the_order_of_their_links() {
        let mut state = State::default();

        state.follow(&inn(5));
        state.follow(&inn(2));

        let names: Vec<String> = state
            .described()
            .cards
            .into_iter()
            .map(|card| card.name)
            .collect();
        assert_eq!(names, ["wlan2", "wlan5"]);
    }

    #[test]
    fn signing_in_leaves_no_page_to_sign_in_at() {
        let mut state = State::default();
        state.follow(&inn(2));
        state.judged(
            2,
            &Verdict::SignIn {
                page: "http://front-desk.inn/".into(),
            },
        );

        state.judged(2, &Verdict::Online);

        assert_eq!(
            (card(&state).reach, card(&state).sign_in),
            (Some(Reach::Online), None)
        );
    }

    #[test]
    fn a_network_checked_again_keeps_its_reach_meanwhile() {
        let mut state = State::default();
        state.follow(&inn(2));
        state.judged(2, &Verdict::Online);

        state.checking(2);
        state.follow(&Link {
            dns: vec!["10.0.0.1".parse().unwrap()],
            ..inn(2)
        });

        assert_eq!(card(&state).reach, Some(Reach::Online));
    }

    #[test]
    fn a_card_leaving_its_network_or_its_way_beyond_has_no_reach() {
        let mut moved = State::default();
        moved.follow(&inn(2));
        moved.judged(
            2,
            &Verdict::SignIn {
                page: "http://front-desk.inn/".into(),
            },
        );
        let mut cut_off = moved.clone();

        moved.follow(&Link {
            ssid: Some("Lantern".into()),
            ..inn(2)
        });
        cut_off.follow(&Link {
            routable: false,
            ..inn(2)
        });

        assert_eq!((card(&moved).reach, card(&moved).sign_in), (None, None));
        assert_eq!(card(&moved).network.as_deref(), Some("Lantern"));
        assert_eq!((card(&cut_off).reach, card(&cut_off).sign_in), (None, None));
    }

    #[test]
    fn only_the_cards_there_still_are_kept() {
        let mut state = State::default();
        state.follow(&inn(2));
        state.follow(&inn(3));

        state.keep_only(&[3]);
        state.judged(2, &Verdict::Online);

        let names: Vec<String> = state
            .described()
            .cards
            .into_iter()
            .map(|card| card.name)
            .collect();
        assert_eq!(names, ["wlan3"]);
    }

    #[test]
    fn saved_networks_are_shown_once_each_as_they_re_joined() {
        let mut state = State::default();

        state.saved(&[
            saved("Roadside Inn", true, 0),
            saved("Lantern", false, 10),
            saved("Roadside Inn", false, 5),
        ]);

        assert_eq!(
            state.described().saved,
            [
                Saved {
                    name: "Roadside Inn".into(),
                    security: Security::Personal,
                    declared: true,
                    trusted: false,
                    priority: 0,
                    hidden: false,
                },
                Saved {
                    name: "Lantern".into(),
                    security: Security::Personal,
                    declared: false,
                    trusted: false,
                    priority: 10,
                    hidden: false,
                },
            ]
        );
    }

    #[test]
    fn a_network_in_the_air_is_shown_once_by_its_strongest_access_point() {
        let mut state = State::default();

        state.heard("/Interfaces/0/BSSs/1", Some(heard("Lantern", -82)));
        state.heard("/Interfaces/0/BSSs/2", Some(heard("Lantern", -52)));
        state.heard("/Interfaces/1/BSSs/3", Some(heard("Lantern", -71)));

        assert_eq!(
            state.described().nearby,
            [Nearby {
                name: "Lantern".into(),
                security: Security::Personal,
                strength: 4,
            }]
        );
    }

    #[test]
    fn networks_in_the_air_are_shown_strongest_first_then_by_name() {
        let mut state = State::default();

        state.heard("/Interfaces/0/BSSs/1", Some(heard("Faint", -90)));
        state.heard("/Interfaces/0/BSSs/2", Some(heard("Lantern", -66)));
        state.heard("/Interfaces/0/BSSs/3", Some(heard("Inn", -64)));
        state.heard("/Interfaces/0/BSSs/4", Some(heard("Far", -79)));

        assert_eq!(
            nearby(&state),
            [
                ("Inn".into(), 3),
                ("Lantern".into(), 3),
                ("Far".into(), 2),
                ("Faint".into(), 1),
            ]
        );
    }

    #[test]
    fn signal_wandering_within_a_bar_changes_nothing_shown() {
        let mut state = State::default();
        state.heard("/Interfaces/0/BSSs/1", Some(heard("Lantern", -62)));
        let before = state.described();

        state.heard("/Interfaces/0/BSSs/1", Some(heard("Lantern", -68)));

        assert_eq!(state.described(), before);
    }

    #[test]
    fn an_access_point_no_longer_heard_is_no_longer_shown() {
        let mut state = State::default();
        state.heard("/Interfaces/0/BSSs/1", Some(heard("Lantern", -52)));
        state.heard("/Interfaces/0/BSSs/2", Some(heard("Lantern", -82)));

        state.heard("/Interfaces/0/BSSs/1", None);

        assert_eq!(nearby(&state), [("Lantern".into(), 1)]);
    }

    fn on_nothing(index: i32) -> Link {
        Link {
            ssid: None,
            routable: false,
            ..inn(index)
        }
    }

    #[test]
    fn a_card_reaching_beyond_its_network_is_settled() {
        let mut state = State::default();
        state.follow(&inn(2));

        state.checking(2);

        assert!(state.settled());
    }

    #[test]
    fn a_card_that_looked_and_heard_no_saved_network_is_settled() {
        let mut state = State::default();
        state.saved(&[saved("Lantern", false, 0)]);
        state.follow(&on_nothing(2));
        state.heard("/Interfaces/0/BSSs/1", Some(heard("Somebody Else's", -52)));

        state.looked("wlan2");

        assert!(state.settled());
    }

    #[test]
    fn each_look_a_card_finishes_is_described_even_hearing_nothing() {
        let mut state = State::default();
        state.follow(&on_nothing(2));
        let before = state.described();

        state.looked("wlan2");
        state.looked("wlan2");

        let after = state.described();
        assert_eq!((before.looks, after.looks), (0, 2));
        assert_eq!(after.nearby, before.nearby);
    }

    #[test]
    fn a_failed_look_ends_the_wait_but_settles_nothing() {
        let mut state = State::default();
        state.follow(&on_nothing(2));

        state.look_ended();

        assert_eq!(state.described().looks, 1);
        assert!(!state.settled());
    }

    #[test]
    fn a_look_begun_and_ended_between_two_reads_still_shows() {
        let mut state = State::default();
        let before = state.looking("wlan0");

        state.scanning("wlan0", true);
        state.scanning("wlan0", true);
        state.scanning("wlan0", false);
        let after = state.looking("wlan0");
        let later = state.looking("wlan0");

        assert!(before.looked_until(after), "a look came between");
        assert_eq!(after.begun, 1, "said looking twice, it's one look");
        assert!(!after.looked_until(later), "none came after");
    }

    #[test]
    fn wifi_turned_off_is_settled() {
        let mut state = State::default();
        state.follow(&on_nothing(2));

        state.radio(false);

        assert!(state.settled());
    }

    #[test]
    fn a_cable_reaching_beyond_the_machine_is_settled() {
        let mut state = State::default();

        state.wired(true);

        assert!(state.settled());
    }

    #[test]
    fn nothing_is_settled_before_a_card_appears() {
        let state = State::default();

        assert!(!state.settled());
    }

    #[test]
    fn a_card_yet_to_look_isn_t_settled() {
        let mut state = State::default();
        state.follow(&on_nothing(2));
        state.follow(&on_nothing(3));

        state.looked("wlan2");

        assert!(!state.settled());
    }

    #[test]
    fn a_look_before_wifi_was_turned_off_doesn_t_count_once_it_s_on() {
        let mut state = State::default();
        state.follow(&on_nothing(2));
        state.looked("wlan2");
        state.radio(false);

        state.radio(true);

        assert!(!state.settled());
    }

    #[test]
    fn a_card_that_went_takes_its_look_with_it() {
        let mut state = State::default();
        state.follow(&on_nothing(2));
        state.looked("wlan2");
        state.keep_only(&[]);

        state.follow(&on_nothing(2));

        assert!(!state.settled());
    }

    #[test]
    fn a_card_hearing_a_saved_network_isn_t_settled_until_it_s_on_it() {
        let mut state = State::default();
        state.saved(&[saved("Lantern", false, 0)]);
        state.follow(&on_nothing(2));
        state.heard("/Interfaces/0/BSSs/1", Some(heard("Lantern", -52)));

        state.looked("wlan2");

        assert!(!state.settled());
    }

    #[test]
    fn a_card_that_couldn_t_join_the_saved_network_it_heard_is_settled() {
        let mut state = State::default();
        state.saved(&[saved("Lantern", false, 0)]);
        state.follow(&on_nothing(2));
        state.heard("/Interfaces/0/BSSs/1", Some(heard("Lantern", -52)));
        state.looked("wlan2");
        state.attempting("wlan2");

        state.troubled("wlan2", Some(Trouble::WrongPassword));

        assert!(state.settled());
    }

    #[test]
    fn a_card_going_takes_what_it_heard_with_it() {
        let mut state = State::default();
        state.heard("/Interfaces/1/BSSs/1", Some(heard("Lantern", -52)));
        state.heard("/Interfaces/10/BSSs/2", Some(heard("Inn", -52)));

        state.unheard_under("/Interfaces/1/");

        assert_eq!(nearby(&state), [("Inn".into(), 4)]);
    }

    fn polled(point: &str) -> Polled {
        Polled {
            point: point.into(),
            signal: Some(-52),
            ..Polled::default()
        }
    }

    #[test]
    fn a_network_in_detail_has_what_the_card_on_it_read_of_its_link() {
        let mut state = State::default();
        state.follow(&inn(2));
        state.heard("/Interfaces/0/BSSs/1", Some(heard("Roadside Inn", -52)));

        state.polled("wlan2", Some(polled("/Interfaces/0/BSSs/1")));
        let detail = state.detail("Roadside Inn");

        assert_eq!(detail.joined[0].card, "wlan2");
        assert!(detail.points[0].joined);
    }

    #[test]
    fn a_network_in_detail_marks_the_access_point_a_card_is_on_before_its_link_is_read() {
        let mut state = State::default();
        state.follow(&inn(2));
        state.heard("/Interfaces/0/BSSs/1", Some(heard("Roadside Inn", -52)));

        state.at("wlan2", Some("/Interfaces/0/BSSs/1".into()));
        let detail = state.detail("Roadside Inn");

        assert!(detail.points[0].joined);
        assert_eq!(detail.joined, []);
    }

    #[test]
    fn an_access_point_a_card_has_left_is_no_longer_marked() {
        let mut state = State::default();
        state.follow(&inn(2));
        state.heard("/Interfaces/0/BSSs/1", Some(heard("Roadside Inn", -52)));
        state.at("wlan2", Some("/Interfaces/0/BSSs/1".into()));

        state.at("wlan2", None);

        assert!(!state.detail("Roadside Inn").points[0].joined);
    }

    #[test]
    fn a_card_s_access_point_taken_in_before_the_card_is_followed_is_kept() {
        let mut state = State::default();
        state.at("wlan2", Some("/Interfaces/0/BSSs/1".into()));

        state.keep_only(&[2]);
        state.follow(&inn(2));
        state.heard("/Interfaces/0/BSSs/1", Some(heard("Roadside Inn", -52)));

        assert!(state.detail("Roadside Inn").points[0].joined);
    }

    /// A card on Roadside Inn at one access point, hearing another of its
    /// at 02:00:00:00:02:00.
    fn at_the_inn() -> State {
        let mut state = State::default();
        state.follow(&inn(2));
        let mut other = heard("Roadside Inn", -60);
        other.bssid = "02:00:00:00:02:00".into();
        state.heard("/Interfaces/0/BSSs/1", Some(heard("Roadside Inn", -52)));
        state.heard("/Interfaces/0/BSSs/2", Some(other));
        state.at("wlan2", Some("/Interfaces/0/BSSs/1".into()));
        state
    }

    #[test]
    fn a_card_on_a_network_is_measured_through_its_access_point_and_gateway() {
        let state = at_the_inn();

        let joined = Joined {
            card: "wlan2".into(),
            point: "02:00:00:00:01:00".into(),
            gateway: Ipv4Addr::new(10, 43, 0, 1),
            network: "Roadside Inn".into(),
            trusted: false,
        };
        assert_eq!(
            state.joined(),
            [joined],
            "the access point it's on, not the other"
        );
    }

    #[test]
    fn a_card_on_a_trusted_network_is_joined_as_trusted() {
        let mut state = at_the_inn();
        let mut inn = saved("Roadside Inn", false, 0);
        inn.network.trusted = true;

        state.saved(&[inn, saved("Lantern", false, 0)]);

        let joined = state.joined();
        assert!(joined[0].trusted, "joined: {joined:?}");
        assert_eq!(state.saved_names(), ["Roadside Inn", "Lantern"]);
    }

    #[test]
    fn a_card_s_link_is_joined_as_its_network_is_saved_whatever_its_point_advertises() {
        let mut state = at_the_inn();
        let mut open = heard("Roadside Inn", -52);
        open.security = Security::Open;
        state.heard("/Interfaces/0/BSSs/1", Some(open));
        assert_eq!(
            state.link_of("wlan2"),
            None,
            "a network not saved was known"
        );

        state.saved(&[saved("Lantern", false, 0), saved("Roadside Inn", false, 0)]);
        assert_eq!(
            state.link_of("wlan2"),
            Some(Security::Personal),
            "a point advertising its network open was taken at its word"
        );
        assert_eq!(
            state.link_of("wlan9"),
            None,
            "a card on no network had a link"
        );
    }

    #[test]
    fn a_card_hears_around_it_only_its_own_network_s_access_points() {
        let mut state = at_the_inn();
        state.heard("/Interfaces/0/BSSs/3", Some(heard("Lantern", -40)));
        state.heard("/Interfaces/1/BSSs/1", Some(heard("Roadside Inn", -45)));

        let around: Vec<_> = state
            .around("wlan2")
            .into_iter()
            .map(|heard| heard.signal)
            .collect();

        assert_eq!(
            around,
            [-52, -60],
            "the inn's two, not Lantern nor another card's"
        );
        assert!(
            state.around("wlan9").is_empty(),
            "a card on no network hears none"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn a_card_s_chooser_hears_each_point_s_age_as_of_when_it_chooses() {
        let mut state = at_the_inn();
        let mut aged = heard("Roadside Inn", -60);
        aged.bssid = "02:00:00:00:02:00".into();
        aged.age = 3;
        state.heard("/Interfaces/0/BSSs/2", Some(aged));

        let later = Instant::now() + Duration::from_secs(40);
        let seconds_ago: Vec<u32> = state
            .around_to_choose("wlan2", later)
            .into_iter()
            .map(|heard| heard.age)
            .collect();

        assert_eq!(
            seconds_ago,
            [40, 43],
            "a point's age stood still from when it was taken in"
        );
    }

    #[test]
    fn a_card_s_chooser_hears_nothing_of_the_point_that_warned_it_while_that_holds() {
        let mut state = at_the_inn();
        let on = state.point_of("wlan2").expect("on a point");
        let signals = |around: Vec<Heard>| -> Vec<i16> {
            around.into_iter().map(|heard| heard.signal).collect()
        };
        let other = state
            .around("wlan2")
            .into_iter()
            .find(|heard| heard.bssid != on)
            .expect("another point of the inn");
        let now = Instant::now();
        state.warned_off(
            "wlan2",
            &other.bssid.to_uppercase(),
            (Some(now), now + Duration::from_secs(600)),
        );
        assert_eq!(
            state.warned_on("wlan2", now),
            None,
            "a card was said to be on the point that warned it, though it's on another"
        );

        assert_eq!(
            signals(state.around_to_choose("wlan2", now)),
            [-52],
            "the point that warned, however written, is left out"
        );
        assert_eq!(
            state.warned_from("wlan2", now),
            Some(&*other.bssid.to_uppercase()),
            "the point left out wasn't said to be kept off"
        );
        assert_eq!(
            state.warned_from("wlan2", now + Duration::from_secs(600)),
            None,
            "a point was kept off past its warning's time"
        );
        assert_eq!(
            signals(state.around_to_choose("wlan2", now + Duration::from_secs(600))),
            [-52, -60],
            "and heard again once the warning's time is up"
        );
        state.warned_off("wlan2", &on, (None, now + Duration::from_secs(600)));
        assert_eq!(
            state.warned_on("wlan2", now),
            None,
            "a card let go from the point that warned it was said to be moving off it"
        );
        state.warned_off("wlan2", &on, (Some(now), now + Duration::from_secs(600)));
        assert_eq!(
            state.warned_on("wlan2", now),
            Some(now),
            "a card on the point that warned it wasn't said to be, as of the warning"
        );
        assert_eq!(
            state.warned_on("wlan2", now + Duration::from_secs(600)),
            None,
            "a warning was said to hold past its time"
        );
        assert_eq!(
            signals(state.around_to_choose("wlan2", now)),
            [-52, -60],
            "a card back on the point that warned it hears it, and the earlier point again"
        );
        assert_eq!(
            state.warned_from("wlan2", now),
            None,
            "a card back on the point that warned it was kept off it"
        );
    }

    #[test]
    fn a_warning_kept_from_before_a_restart_keeps_its_point_off_with_no_heed_under_way() {
        let mut state = at_the_inn();
        let warned = state.point_of("wlan2").expect("on a point");
        let now = Instant::now();
        state.kept_off("wlan2", &warned, now + Duration::from_secs(600));
        assert_eq!(
            state.warned_on("wlan2", now),
            None,
            "a warning from before a restart was said to have its heed under way"
        );

        state.at("wlan2", Some("/Interfaces/0/BSSs/2".into()));

        assert_eq!(
            state.warned_from("wlan2", now),
            Some(&*warned),
            "a warning from before a restart didn't keep its point off"
        );
    }

    #[test]
    fn a_join_found_partway_as_wifid_starts_keeps_the_kept_warning_should_it_fail() {
        let mut state = at_the_inn();
        let warned = state.point_of("wlan2").expect("on a point");
        let now = Instant::now();
        state.kept_off("wlan2", &warned, now + Duration::from_secs(600));

        // Handed over mid-move: associated with the hall, its handshake to
        // come, which fails and puts it back where it was warned.
        state.joining("wlan2");
        state.at("wlan2", Some("/Interfaces/0/BSSs/2".into()));
        state.at("wlan2", Some("/Interfaces/0/BSSs/1".into()));
        state.landed("wlan2");
        state.attempting("wlan2");
        state.at("wlan2", Some("/Interfaces/0/BSSs/2".into()));

        assert_eq!(
            state.warned_from("wlan2", now),
            Some(&*warned),
            "a join found partway that failed back to the warned point forgot the warning"
        );
    }

    #[test]
    fn a_join_landing_elsewhere_beside_its_point_keeps_the_warning() {
        let mut state = at_the_inn();
        let warned = state.point_of("wlan2").expect("on a point");
        let now = Instant::now();
        state.warned_off("wlan2", &warned, (now, now + Duration::from_secs(3600)));
        // Left for the hall, then a join back to the point that warned,
        // which associates there and lands on the hall instead, the step
        // and the hall in one signal. The inn's own point is the first.
        let (inn, hall) = ("/Interfaces/0/BSSs/1", "/Interfaces/0/BSSs/2");
        state.attempting("wlan2");
        state.at("wlan2", Some(hall.into()));
        state.landed("wlan2");
        state.attempting("wlan2");
        state.at("wlan2", Some(inn.into()));
        state.landed_at("wlan2", Some(hall.into()));

        assert_eq!(
            state.warned_from("wlan2", now),
            Some(&*warned),
            "a join that landed on the hall settled as though back on the warned point"
        );
    }

    #[test]
    fn a_warning_is_forgotten_once_its_card_has_left_its_point_and_come_back() {
        let mut state = at_the_inn();
        let warned = state.point_of("wlan2").expect("on a point");
        let now = Instant::now();
        let lasts = now + Duration::from_secs(3600);
        state.warned_off("wlan2", &warned, (Some(now), lasts));
        state.at("wlan2", None);
        state.at("wlan2", Some("/Interfaces/0/BSSs/1".into()));
        state.at("wlan2", Some("/Interfaces/0/BSSs/2".into()));
        assert_eq!(
            state.warned_from("wlan2", now),
            Some(&*warned),
            "a move that failed back to the point that warned forgot its warning"
        );

        state.at("wlan2", Some("/Interfaces/0/BSSs/1".into()));
        state.at("wlan2", Some("/Interfaces/0/BSSs/2".into()));

        assert_eq!(
            state.warned_from("wlan2", now),
            None,
            "a card held on the point that warned it again was kept off it once it left"
        );
        assert_eq!(
            state.around_to_choose("wlan2", now).len(),
            2,
            "its chooser didn't hear the point it came back to and left"
        );
    }

    #[test]
    fn a_move_that_fails_once_it_associates_keeps_its_card_s_warning() {
        let mut state = at_the_inn();
        let warned = state.point_of("wlan2").expect("on a point");
        let now = Instant::now();
        state.warned_off(
            "wlan2",
            &warned,
            (Some(now), now + Duration::from_secs(3600)),
        );
        let moves = |state: &mut State, to: &str, lands: bool| {
            state.attempting("wlan2");
            state.at("wlan2", Some(to.into()));
            if lands {
                state.landed("wlan2");
            } else {
                state.at("wlan2", None);
            }
        };

        moves(&mut state, "/Interfaces/0/BSSs/2", false);
        moves(&mut state, "/Interfaces/0/BSSs/1", true);
        // As the card associates with the hall again, away from the point
        // that warned, that point is kept off only if the warning lasted.
        state.attempting("wlan2");
        state.at("wlan2", Some("/Interfaces/0/BSSs/2".into()));
        let failed = state.warned_from("wlan2", now).map(str::to_owned);
        state.landed("wlan2");
        moves(&mut state, "/Interfaces/0/BSSs/1", true);

        assert_eq!(
            failed.as_deref(),
            Some(&*warned),
            "a move associated with the hall, not yet joined, forgot the warning"
        );
        assert_eq!(
            state.warned_from("wlan2", now),
            None,
            "a card that joined the hall and came back was still kept off where it was warned"
        );
    }

    #[test]
    fn a_card_leaves_its_warned_point_for_one_heard_only_once_it_s_there() {
        let mut state = at_the_inn();
        let warned = state.point_of("wlan2").expect("on a point");
        let now = Instant::now();
        state.warned_off(
            "wlan2",
            &warned,
            (Some(now), now + Duration::from_secs(3600)),
        );
        let mut far = heard("Roadside Inn", -70);
        far.bssid = "02:00:00:00:03:00".into();

        state.at("wlan2", Some("/Interfaces/0/BSSs/3".into()));
        state.heard("/Interfaces/0/BSSs/3", Some(far));
        state.at("wlan2", Some("/Interfaces/0/BSSs/1".into()));

        assert_eq!(
            state.warned_from("wlan2", now),
            None,
            "a card on a point heard only after it got there never counted as leaving"
        );
    }

    #[test]
    fn a_card_is_there_off_its_network_until_it_goes() {
        let mut state = at_the_inn();
        state.follow(&Link {
            ssid: None,
            gateway: None,
            ..inn(2)
        });
        state.at("wlan2", None);
        let joined = state.joined();
        let between = state.has_card("wlan2");
        state.keep_only(&[]);

        assert!(joined.is_empty(), "a card off its network was joined");
        assert!(between, "a card between points, off its network, was gone");
        assert!(!state.has_card("wlan2"), "a card gone was still there");
    }

    #[test]
    fn a_card_without_a_gateway_or_a_heard_access_point_is_not_measured() {
        let mut unheard = at_the_inn();
        unheard.heard("/Interfaces/0/BSSs/1", None);
        let mut unrouted = at_the_inn();
        unrouted.follow(&Link {
            gateway: None,
            ..inn(2)
        });
        let mut gone = at_the_inn();
        gone.keep_only(&[]);

        assert_eq!(unheard.joined(), [], "its access point unheard");
        assert_eq!(unrouted.joined(), [], "its gateway gone");
        let joined = at_the_inn().joined();
        let Joined { card, point, .. } = joined.first().expect("the inn's card is joined");
        assert_eq!(
            unrouted.point_of(card).as_ref(),
            Some(point),
            "its access point still known, for a move to go back to"
        );
        assert_eq!(unheard.point_of(card), None, "none known where unheard");
        assert!(
            gone.gateways.is_empty(),
            "a card that goes takes its gateway"
        );
    }

    #[test]
    fn a_card_roams_to_an_access_point_of_its_network_it_hears() {
        let state = at_the_inn();

        assert_eq!(state.roamer("02:00:00:00:02:00"), Ok("wlan2".to_owned()));
    }

    #[test]
    fn no_card_roams_to_an_access_point_unheard_or_off_its_network() {
        let mut state = at_the_inn();
        let mut lantern = heard("Lantern", -50);
        lantern.bssid = "02:00:00:00:03:00".into();
        state.heard("/Interfaces/0/BSSs/3", Some(lantern));
        let mut elsewhere = heard("Roadside Inn", -50);
        elsewhere.bssid = "02:00:00:00:04:00".into();
        state.heard("/Interfaces/1/BSSs/4", Some(elsewhere));

        assert_eq!(
            state.roamer("02:00:00:00:09:00"),
            Err(Refused::NotNearby),
            "an access point nobody hears"
        );
        assert_eq!(
            state.roamer("02:00:00:00:03:00"),
            Err(Refused::NotJoined),
            "another network's access point"
        );
        assert_eq!(
            state.roamer("02:00:00:00:04:00"),
            Err(Refused::NotJoined),
            "one only another card hears"
        );
    }

    #[test]
    fn a_card_that_goes_takes_its_access_point_with_it() {
        let mut state = State::default();
        state.follow(&inn(2));
        state.at("wlan2", Some("/Interfaces/0/BSSs/1".into()));

        state.keep_only(&[]);

        assert_eq!(state, State::default());
    }

    #[test]
    fn a_card_s_reading_is_kept_out_of_a_network_it_isn_t_on() {
        let mut state = State::default();
        state.follow(&inn(2));
        state.heard("/Interfaces/0/BSSs/2", Some(heard("Lantern", -52)));

        state.polled("wlan2", Some(polled("/Interfaces/0/BSSs/1")));
        let detail = state.detail("Lantern");

        assert_eq!(detail.joined, []);
        assert_eq!(detail.points.len(), 1);
    }

    #[test]
    fn a_reading_nobody_asks_for_any_more_is_let_go() {
        let mut state = State::default();
        state.follow(&inn(2));
        state.polled("wlan2", Some(polled("/Interfaces/0/BSSs/1")));

        state.polled("wlan2", None);

        assert_eq!(state.detail("Roadside Inn").joined, []);
    }

    #[test]
    fn every_reading_is_let_go_once_nobody_asks() {
        let mut state = State::default();
        state.follow(&inn(2));
        state.polled("wlan2", Some(polled("/Interfaces/0/BSSs/1")));

        state.unpolled();

        let mut unasked = State::default();
        unasked.follow(&inn(2));
        assert_eq!(state, unasked);
    }

    #[test]
    fn the_cards_worth_polling_are_those_on_a_network() {
        let mut state = State::default();
        state.follow(&inn(2));
        state.follow(&on_nothing(3));

        assert_eq!(state.on_networks(), ["wlan2"]);
    }
}
