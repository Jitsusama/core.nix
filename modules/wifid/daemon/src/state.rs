//! What wifid knows that the shell shows: each card, the network it's on and
//! how far that reaches, and every saved network. It's kept whole, so the
//! shell is sent all of it each time any of it changes and never has to
//! piece it together.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use zlink::introspect::{CustomType, Type};

use crate::check::Verdict;
use crate::link::Link;
use crate::network::Security as Kind;
use crate::store;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct State {
    /// By link index, which stays the same while the card is there.
    cards: BTreeMap<i32, Card>,
    saved: Vec<Saved>,
}

/// Every card and every saved network, as the shell is sent them.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Type)]
pub struct Described {
    pub cards: Vec<Card>,
    pub saved: Vec<Saved>,
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

/// How a network is joined.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, CustomType)]
#[serde(rename_all = "camelCase")]
#[zlink(rename_all = "camelCase")]
pub enum Security {
    /// By anyone.
    Open,
    /// With a password.
    Personal,
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

    pub fn described(&self) -> Described {
        Described {
            cards: self.cards.values().cloned().collect(),
            saved: self.saved.clone(),
        }
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
            }
        );
        assert_eq!(judged.reach, Some(Reach::SignIn));
        assert_eq!(judged.sign_in.as_deref(), Some("http://front-desk.inn/"));
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
}
