//! What wifid knows that the shell shows: each card, the network it's on and
//! how far that reaches, every saved network, and the networks in the air.
//! It's kept whole, so the
//! shell is sent all of it each time any of it changes and never has to
//! piece it together.

use std::collections::{BTreeMap, BTreeSet};
use std::net::Ipv4Addr;

use serde::{Deserialize, Serialize};
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
    radio_off: bool,
    wired: bool,
    /// How many times each card, by name, has begun joining a network,
    /// and which of those its trouble came in, since wpa_supplicant can say
    /// how one attempt went after the next has been asked for.
    attempts: BTreeMap<String, u64>,
    troubled_in: BTreeMap<String, u64>,
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
    /// no longer heard.
    pub fn heard(&mut self, path: &str, heard: Option<Heard>) {
        match heard {
            Some(heard) => self.heard.insert(path.into(), heard),
            None => self.heard.remove(path),
        };
    }

    /// Forgets every access point whose path starts so, as every one a card
    /// heard when it goes.
    pub fn unheard_under(&mut self, path: &str) {
        self.heard.retain(|heard, _| !heard.starts_with(path));
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
        let Some(path) = self.at.get(card) else {
            return Vec::new();
        };
        let (Some(on), Some(under)) = (self.heard.get(path), card_of(path)) else {
            return Vec::new();
        };
        self.heard
            .iter()
            .filter(|(other, heard)| card_of(other) == Some(under) && heard.name == on.name)
            .map(|(_, heard)| heard.clone())
            .collect()
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
        state.scanning("wlan0", false);
        let after = state.looking("wlan0");
        let later = state.looking("wlan0");

        assert!(before.looked_until(after), "a look came between");
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
