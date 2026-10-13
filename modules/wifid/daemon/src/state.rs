//! What wifid knows that the shell shows: each card, the network it's on and
//! how far that reaches, every saved network, and the networks in the air.
//! It's kept whole, so the
//! shell is sent all of it each time any of it changes and never has to
//! piece it together.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use zlink::introspect::{CustomType, Type};

use crate::air::Heard;
use crate::check::Verdict;
use crate::link::Link;
use crate::network::Security as Kind;
use crate::store;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {
    /// By link index, which stays the same while the card is there.
    cards: BTreeMap<i32, Card>,
    saved: Vec<Saved>,
    /// By the access point's path in wpa_supplicant, which names its card.
    heard: BTreeMap<String, Heard>,
}

/// Every card, every saved network and the networks in the air, as the
/// shell is sent them.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Type)]
pub struct Described {
    pub cards: Vec<Card>,
    pub saved: Vec<Saved>,
    /// The strongest first.
    pub nearby: Vec<Nearby>,
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
    }

    /// Forgets every card but these, which are all there are.
    pub fn keep_only(&mut self, indexes: &[i32]) {
        self.cards.retain(|index, _| indexes.contains(index));
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
                },
                declared: *declared,
                trusted: network.trusted,
                priority: network.priority,
                hidden: network.hidden,
            });
        }
    }

    /// Says what went wrong on the card of that name, or clears it.
    pub fn troubled(&mut self, card: &str, trouble: Option<Trouble>) {
        if let Some(card) = self.cards.values_mut().find(|shown| shown.name == card) {
            card.trouble = trouble;
        }
    }

    /// How far the card of that name has got joining the network.
    pub fn progress(&self, card: &str, network: &str) -> Progress {
        let Some(card) = self.cards.values().find(|shown| shown.name == card) else {
            return Progress::Gone;
        };
        let on = card.network.as_deref() == Some(network);
        match (on, card.reach, card.trouble) {
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

    pub fn described(&self) -> Described {
        Described {
            cards: self.cards.values().cloned().collect(),
            saved: self.saved.clone(),
            nearby: self.nearby(),
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
fn bars(signal: i16) -> u8 {
    match signal {
        -60.. => 4,
        -70.. => 3,
        -80.. => 2,
        _ => 1,
    }
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
        }
    }

    fn heard(name: &str, signal: i16) -> Option<Heard> {
        Some(Heard {
            name: name.into(),
            security: Security::Personal,
            signal,
        })
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
        let joining = state.progress("wlan2", "Roadside Inn");
        state.follow(&Link {
            routable: false,
            ..inn(2)
        });
        let addressing = state.progress("wlan2", "Roadside Inn");
        state.follow(&inn(2));
        state.checking(2);
        let checking = state.progress("wlan2", "Roadside Inn");
        state.judged(2, &Verdict::Online);
        let reached = state.progress("wlan2", "Roadside Inn");

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
    fn a_card_still_on_another_network_is_still_joining() {
        let mut state = State::default();
        state.follow(&inn(2));
        state.judged(2, &Verdict::Online);

        let progress = state.progress("wlan2", "Lantern");

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

        state.troubled("wlan2", Some(Trouble::WrongPassword));

        assert_eq!(card(&state).trouble, Some(Trouble::WrongPassword));
        assert_eq!(
            state.progress("wlan2", "Lantern"),
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

        state.troubled("wlan2", Some(Trouble::WrongPassword));

        assert_eq!(
            state.progress("wlan2", "Lantern"),
            Progress::Troubled(Trouble::WrongPassword)
        );
    }

    #[test]
    fn a_card_that_has_gone_joins_nothing() {
        let state = State::default();

        assert_eq!(state.progress("wlan2", "Lantern"), Progress::Gone);
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

        state.heard("/Interfaces/0/BSSs/1", heard("Lantern", -82));
        state.heard("/Interfaces/0/BSSs/2", heard("Lantern", -52));
        state.heard("/Interfaces/1/BSSs/3", heard("Lantern", -71));

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

        state.heard("/Interfaces/0/BSSs/1", heard("Faint", -90));
        state.heard("/Interfaces/0/BSSs/2", heard("Lantern", -66));
        state.heard("/Interfaces/0/BSSs/3", heard("Inn", -64));
        state.heard("/Interfaces/0/BSSs/4", heard("Far", -79));

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
        state.heard("/Interfaces/0/BSSs/1", heard("Lantern", -62));
        let before = state.described();

        state.heard("/Interfaces/0/BSSs/1", heard("Lantern", -68));

        assert_eq!(state.described(), before);
    }

    #[test]
    fn an_access_point_no_longer_heard_is_no_longer_shown() {
        let mut state = State::default();
        state.heard("/Interfaces/0/BSSs/1", heard("Lantern", -52));
        state.heard("/Interfaces/0/BSSs/2", heard("Lantern", -82));

        state.heard("/Interfaces/0/BSSs/1", None);

        assert_eq!(nearby(&state), [("Lantern".into(), 1)]);
    }

    #[test]
    fn a_card_going_takes_what_it_heard_with_it() {
        let mut state = State::default();
        state.heard("/Interfaces/1/BSSs/1", heard("Lantern", -52));
        state.heard("/Interfaces/10/BSSs/2", heard("Inn", -52));

        state.unheard_under("/Interfaces/1/");

        assert_eq!(nearby(&state), [("Inn".into(), 4)]);
    }
}
