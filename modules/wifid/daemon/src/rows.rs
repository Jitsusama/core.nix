//! Wi-Fi's rows in the launcher, made ready here so the shell only draws
//! them: Wi-Fi Off (or On), Sign In while a network wants it, and every
//! network in the air, the one connected first, then those saved, then the
//! rest by signal, each worded from the launcher's vocabulary
//! (launcher/words.json) as the shell hands it over, so wifid says no word
//! of its own. Each row is the ready-row form every daemon serves
//! (QEST-20261023-Y2YBTS attachments/speed/ready-rows.md), sent whole only
//! when something it draws changes (face.rs).

use std::cmp::Ordering;
use std::collections::BTreeMap;

use serde_json::{Value, json};

use crate::state::{Described, Reach, Saved, Security};

/// The launcher's place Wi-Fi's rows are in.
pub const PLACE: &str = "wifi";

/// The words Wi-Fi's rows are said in, from the launcher's vocabulary.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Words {
    /// Wi-Fi Off, as the radio's row reads while it's on, and its glyph.
    pub radio_on: Said,
    /// Wi-Fi On, while it's off.
    pub radio_off: Said,
    /// "Sign in to {network}", and its glyph.
    pub sign_in: Said,
    /// How a network stands beside its name, and how it's joined after it.
    pub connected: String,
    pub saved: String,
    /// "{n} of 4 bars".
    pub bars: String,
    pub open: String,
    pub password: String,
    pub account: String,
}

/// A row's name and glyph role.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Said {
    pub name: String,
    pub glyph: String,
}

impl Words {
    /// Wi-Fi's words from the whole vocabulary, or the first it lacks, so a
    /// vocabulary that lost one is said rather than shown as blanks.
    pub fn read(vocabulary: &Value) -> Result<Words, String> {
        let thing = |id: &str| {
            vocabulary["things"]
                .as_array()
                .and_then(|things| things.iter().find(|thing| thing["id"] == id))
                .ok_or_else(|| format!("the vocabulary has no {id}"))
        };
        let word = |within: &Value, id: &str, at: &[&str]| -> Result<String, String> {
            at.iter()
                .fold(within, |within, field| &within[field])
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| format!("the vocabulary's {id} has no {}", at.join(".")))
        };
        let radio = thing("wifi:radio")?;
        let sign_in = thing("wifi:sign-in")?;
        let network = thing("wifi:network")?;
        let said = |at: &str| word(network, "wifi:network", &["said", at]);
        Ok(Words {
            radio_on: Said {
                name: word(radio, "wifi:radio", &["name"])?,
                glyph: word(radio, "wifi:radio", &["glyph"])?,
            },
            radio_off: Said {
                name: word(radio, "wifi:radio", &["states", "off", "name"])?,
                glyph: word(radio, "wifi:radio", &["states", "off", "glyph"])?,
            },
            sign_in: Said {
                name: word(sign_in, "wifi:sign-in", &["name"])?,
                glyph: word(sign_in, "wifi:sign-in", &["glyph"])?,
            },
            connected: said("connected")?,
            saved: said("saved")?,
            bars: said("bars")?,
            open: said("open")?,
            password: said("password")?,
            account: said("account")?,
        })
    }
}

/// A network as its row is made from it: heard in the air, or the one a
/// card is on though it hides its name.
#[derive(Clone, Debug, PartialEq)]
struct Network<'a> {
    name: &'a str,
    security: Security,
    /// Bars, from 1 to 4, or 0 for one not heard.
    strength: u8,
    /// How it's saved here, if it is.
    kept: Option<&'a Saved>,
    connected: bool,
}

impl Network<'_> {
    fn known(&self) -> bool {
        self.kept.is_some()
    }
}

/// Every row, in the order the launcher lists them, each without its
/// `order`, which `Orders` gives it. A machine with no card has none.
pub fn rows(described: &Described, words: &Words) -> Vec<Value> {
    if described.cards.is_empty() {
        return Vec::new();
    }
    let mut rows = Vec::new();
    let signing = described
        .cards
        .iter()
        .find(|card| card.reach == Some(Reach::SignIn))
        .and_then(|card| card.network.as_deref());
    if let Some(network) = signing {
        rows.push(json!({
            "key": key("sign-in"), "row": "wifi:sign-in", "place": PLACE, "kind": "sign-in",
            "name": words.sign_in.name.replace("{network}", network),
            "glyph": words.sign_in.glyph, "hue": "machine", "depth": 0, "side": [],
            "held": {"network": network},
        }));
    }
    if described.radio {
        for network in networks(described) {
            rows.push(network_row(&network, signing, words));
        }
    }
    let radio = if described.radio {
        &words.radio_on
    } else {
        &words.radio_off
    };
    rows.push(json!({
        "key": key("radio"), "row": "wifi:radio", "place": PLACE, "kind": "radio",
        "name": radio.name, "glyph": radio.glyph, "hue": "machine", "depth": 0, "side": [],
        "held": {"on": described.radio},
    }));
    rows
}

fn key(id: &str) -> String {
    format!("row:{PLACE}:{id}")
}

/// Every network in the air, and the one each card is on even when it
/// hides its name, the one connected first, then those saved, then by
/// signal, then A to Z, as launcher/Order.js's live order has it.
fn networks(described: &Described) -> Vec<Network<'_>> {
    let mut heard: Vec<Network> = described
        .nearby
        .iter()
        .map(|network| made(described, &network.name, network.security, network.strength))
        .collect();
    for name in described
        .cards
        .iter()
        .filter_map(|card| card.network.as_deref())
    {
        if !heard.iter().any(|network| network.name == name) {
            let security = saved(described, name).map_or(Security::Open, |kept| kept.security);
            heard.push(made(described, name, security, 0));
        }
    }
    let stands = |network: &Network| {
        if network.connected {
            0
        } else if network.known() {
            1
        } else {
            2
        }
    };
    heard.sort_by(|one, other| {
        stands(one)
            .cmp(&stands(other))
            .then(other.strength.cmp(&one.strength))
            .then_with(|| by_name(one.name, other.name))
    });
    heard
}

fn saved<'a>(described: &'a Described, name: &str) -> Option<&'a Saved> {
    described.saved.iter().find(|saved| saved.name == name)
}

fn made<'a>(
    described: &'a Described,
    name: &'a str,
    security: Security,
    strength: u8,
) -> Network<'a> {
    Network {
        name,
        security,
        strength,
        kept: saved(described, name),
        connected: described
            .cards
            .iter()
            .any(|card| card.network.as_deref() == Some(name)),
    }
}

/// A to Z as names are read: runs of digits by their value, so Tab 9 comes
/// before Tab 10, and the rest without regard to case.
fn by_name(one: &str, other: &str) -> Ordering {
    let runs = |name: &str| -> Vec<String> {
        let mut runs: Vec<String> = Vec::new();
        for letter in name.to_lowercase().chars() {
            match runs.last_mut() {
                Some(run) if run.chars().all(|c| c.is_ascii_digit()) == letter.is_ascii_digit() => {
                    run.push(letter);
                }
                _ => runs.push(letter.to_string()),
            }
        }
        runs
    };
    let (a, b) = (runs(one), runs(other));
    for (a, b) in a.iter().zip(&b) {
        let numbers = a.starts_with(|c: char| c.is_ascii_digit())
            && b.starts_with(|c: char| c.is_ascii_digit());
        let said = if numbers {
            let value = |run: &str| run.trim_start_matches('0').to_owned();
            let (a, b) = (value(a), value(b));
            a.len().cmp(&b.len()).then_with(|| a.cmp(&b))
        } else {
            a.cmp(b)
        };
        if said != Ordering::Equal {
            return said;
        }
    }
    a.len().cmp(&b.len())
}

fn network_row(network: &Network, signing: Option<&str>, words: &Words) -> Value {
    let (note, locked, password, account) = match network.security {
        Security::Open => (&words.open, false, false, false),
        Security::Personal => (&words.password, true, true, false),
        Security::Enterprise => (&words.account, true, true, true),
    };
    let standing = if network.connected {
        Some(&words.connected)
    } else if network.known() {
        Some(&words.saved)
    } else {
        None
    };
    let bars =
        (network.strength > 0).then(|| words.bars.replace("{n}", &network.strength.to_string()));
    let side: Vec<Value> = standing
        .cloned()
        .into_iter()
        .chain(bars)
        .map(|said| json!({"said": said}))
        .collect();
    let glyph = format!(
        "{}{}",
        if locked { "lockedSignal" } else { "signal" },
        network.strength.min(4)
    );
    json!({
        // Keyed apart from the radio's and the sign-in's, so a network
        // named radio is a row of its own.
        "key": key(&format!("network:{}", network.name)),
        "row": format!("wifi:{}", network.name), "place": PLACE,
        "kind": "network", "name": network.name, "note": note, "glyph": glyph, "hue": "machine",
        "depth": 0, "side": side,
        "held": {
            "network": network.name, "security": security(network.security),
            "strength": network.strength, "known": network.known(),
            "declared": network.kept.is_some_and(|kept| kept.declared),
            "trusted": network.kept.is_some_and(|kept| kept.trusted),
            "connected": network.connected, "locked": locked, "password": password,
            "account": account, "signing": network.connected && signing == Some(network.name),
        },
    })
}

fn security(security: Security) -> &'static str {
    match security {
        Security::Open => "open",
        Security::Personal => "personal",
        Security::Enterprise => "enterprise",
    }
}

/// How far apart rows are placed when they're placed afresh, so one row
/// moving later finds room between two others.
pub const SPACING: i64 = 1024;

/// Each row's place, kept from one making to the next, so a row moving
/// changes its own `order` alone: the longest run of rows still in their
/// order keeps theirs, and each other row is placed midway between its
/// neighbours. Only when no room is left between two is every row placed
/// afresh, said in the same change.
#[derive(Clone, Debug, Default)]
pub struct Orders {
    last: BTreeMap<String, i64>,
}

impl Orders {
    /// The rows with their `order`, by the keys they were listed in.
    pub fn place(&mut self, mut rows: Vec<Value>) -> Vec<Value> {
        let keys: Vec<String> = rows
            .iter()
            .map(|row| row["key"].as_str().unwrap_or("").to_owned())
            .collect();
        let orders = self.orders(&keys);
        for (row, order) in rows.iter_mut().zip(&orders) {
            row["order"] = json!(order);
        }
        self.last = keys.into_iter().zip(orders).collect();
        rows
    }

    fn orders(&self, keys: &[String]) -> Vec<i64> {
        let before: Vec<Option<i64>> = keys.iter().map(|key| self.last.get(key).copied()).collect();
        let kept = longest_run(&before);
        let mut orders: Vec<Option<i64>> = before
            .iter()
            .enumerate()
            .map(|(at, order)| if kept.contains(&at) { *order } else { None })
            .collect();
        let mut at = 0;
        while at < orders.len() {
            if orders[at].is_some() {
                at += 1;
                continue;
            }
            let end = (at..orders.len())
                .find(|&end| orders[end].is_some())
                .unwrap_or(orders.len());
            let count = i64::try_from(end - at).unwrap_or(i64::MAX);
            let low = if at == 0 { None } else { orders[at - 1] };
            let high = orders.get(end).copied().flatten();
            let (low, high) = match (low, high) {
                (Some(low), Some(high)) => (low, high),
                (Some(low), None) => (low, low + SPACING * (count + 1)),
                (None, Some(high)) => (high - SPACING * (count + 1), high),
                (None, None) => (0, SPACING * (count + 1)),
            };
            let step = (high - low) / (count + 1);
            if step < 1 {
                return (1..=keys.len())
                    .map(|place| SPACING * i64::try_from(place).unwrap_or(i64::MAX))
                    .collect();
            }
            for (placed, slot) in orders[at..end].iter_mut().enumerate() {
                *slot = Some(low + step * (i64::try_from(placed).unwrap_or(0) + 1));
            }
            at = end;
        }
        orders.into_iter().map(|order| order.unwrap_or(0)).collect()
    }
}

/// The places of the longest run of orders still increasing, leaving out
/// rows with none.
fn longest_run(orders: &[Option<i64>]) -> Vec<usize> {
    let placed: Vec<(usize, i64)> = orders
        .iter()
        .enumerate()
        .filter_map(|(at, order)| order.map(|order| (at, order)))
        .collect();
    // The longest run ending at each, and the one before it there.
    let mut length = vec![1usize; placed.len()];
    let mut previous = vec![None; placed.len()];
    for one in 0..placed.len() {
        for before in 0..one {
            if placed[before].1 < placed[one].1 && length[before] + 1 > length[one] {
                length[one] = length[before] + 1;
                previous[one] = Some(before);
            }
        }
    }
    let mut run = Vec::new();
    let mut at = (0..placed.len()).max_by_key(|&at| length[at]);
    while let Some(one) = at {
        run.push(placed[one].0);
        at = previous[one];
    }
    run.reverse();
    run
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{Card, Nearby};

    /// Wi-Fi's part of the launcher's vocabulary, as launcher/words.json
    /// says it.
    fn vocabulary() -> Value {
        json!({"things": [
            {"id": "wifi:radio", "name": "Wi-Fi Off", "glyph": "offline",
                "states": {"off": {"name": "Wi-Fi On", "glyph": "wireless"}}},
            {"id": "wifi:sign-in", "name": "Sign in to {network}", "glyph": "signIn"},
            {"id": "wifi:network", "said": {"connected": "connected", "saved": "saved",
                "bars": "{n} of 4 bars", "open": "open", "password": "a password",
                "account": "an account"}},
        ]})
    }

    fn words() -> Words {
        Words::read(&vocabulary()).expect("the vocabulary has Wi-Fi's words")
    }

    /// The launcher's own vocabulary, which the check
    /// wifid-reads-the-launchers-words hands the tests, so a word renamed
    /// there fails a check rather than blanking a row. wifid's own build
    /// isn't handed it, or each change to the launcher's words would start
    /// wifid again.
    #[test]
    fn the_launchers_vocabulary_has_wifis_words() {
        let Ok(path) = std::env::var("WIFID_TEST_WORDS") else {
            return;
        };
        let text = std::fs::read_to_string(path).expect("the vocabulary reads");
        let launchers: Value = serde_json::from_str(&text).expect("the vocabulary is JSON");
        assert_eq!(Words::read(&launchers), Ok(words()));
    }

    #[test]
    fn a_vocabulary_without_a_word_says_which() {
        let said = Words::read(&json!({"things": [{"id": "wifi:radio"}]}));
        assert_eq!(said, Err("the vocabulary has no wifi:sign-in".into()));
    }

    fn nearby(name: &str, security: Security, strength: u8) -> Nearby {
        Nearby {
            name: name.into(),
            security,
            strength,
        }
    }

    fn saved(name: &str) -> Saved {
        Saved {
            name: name.into(),
            security: Security::Personal,
            declared: false,
            trusted: true,
            priority: 0,
            hidden: false,
        }
    }

    fn card(network: Option<&str>, reach: Option<Reach>) -> Card {
        Card {
            name: "wlan0".into(),
            network: network.map(Into::into),
            reach,
            sign_in: None,
            trouble: None,
        }
    }

    fn described() -> Described {
        Described {
            cards: vec![card(Some("Lantern"), Some(Reach::Online))],
            saved: vec![saved("Lantern"), saved("Harbour")],
            nearby: vec![
                nearby("Cafe 10", Security::Open, 4),
                nearby("Cafe 9", Security::Open, 4),
                nearby("Harbour", Security::Personal, 2),
                nearby("Lantern", Security::Personal, 3),
                nearby("Campus", Security::Enterprise, 1),
            ],
            radio: true,
            wired: false,
            looks: 1,
        }
    }

    fn names(rows: &[Value]) -> Vec<&str> {
        rows.iter().filter_map(|row| row["name"].as_str()).collect()
    }

    #[test]
    fn the_one_connected_comes_first_then_those_saved_then_by_signal_and_name() {
        let rows = rows(&described(), &words());
        assert_eq!(
            names(&rows),
            [
                "Lantern",
                "Harbour",
                "Cafe 9",
                "Cafe 10",
                "Campus",
                "Wi-Fi Off"
            ]
        );
    }

    #[test]
    fn a_network_says_how_it_stands_its_bars_and_how_its_joined() {
        let rows = rows(&described(), &words());
        assert_eq!(rows[0]["key"], "row:wifi:network:Lantern");
        assert_eq!(rows[0]["row"], "wifi:Lantern");
        assert_eq!(rows[0]["note"], "a password");
        assert_eq!(rows[0]["glyph"], "lockedSignal3");
        assert_eq!(
            rows[0]["side"],
            json!([{"said": "connected"}, {"said": "3 of 4 bars"}])
        );
        assert_eq!(rows[0]["held"]["connected"], true);
        assert_eq!(
            rows[1]["side"],
            json!([{"said": "saved"}, {"said": "2 of 4 bars"}])
        );
        assert_eq!(rows[2]["glyph"], "signal4");
        assert_eq!(rows[2]["note"], "open");
        assert_eq!(rows[4]["note"], "an account");
        assert_eq!(rows[4]["held"]["account"], true);
    }

    #[test]
    fn the_network_joined_is_a_row_even_when_it_hides_its_name() {
        let mut hidden = described();
        hidden.nearby.retain(|network| network.name != "Lantern");
        let rows = rows(&hidden, &words());
        assert_eq!(rows[0]["name"], "Lantern");
        assert_eq!(rows[0]["glyph"], "lockedSignal0");
        assert_eq!(rows[0]["side"], json!([{"said": "connected"}]));
    }

    #[test]
    fn a_network_wanting_a_sign_in_puts_its_row_first() {
        let mut wanting = described();
        wanting.cards = vec![card(Some("Lantern"), Some(Reach::SignIn))];
        let rows = rows(&wanting, &words());
        assert_eq!(rows[0]["key"], "row:wifi:sign-in");
        assert_eq!(rows[0]["name"], "Sign in to Lantern");
        assert_eq!(rows[1]["held"]["signing"], true);
    }

    #[test]
    fn with_the_radio_off_only_turning_it_on_is_left() {
        let mut off = described();
        off.radio = false;
        let rows = rows(&off, &words());
        assert_eq!(names(&rows), ["Wi-Fi On"]);
        assert_eq!(rows[0]["held"]["on"], false);
    }

    #[test]
    fn a_machine_without_a_card_has_no_rows() {
        let mut none = described();
        none.cards.clear();
        assert_eq!(rows(&none, &words()), Vec::<Value>::new());
    }

    fn placed(orders: &mut Orders, keys: &[&str]) -> Vec<i64> {
        let rows = keys.iter().map(|key| json!({"key": key})).collect();
        orders
            .place(rows)
            .iter()
            .map(|row| row["order"].as_i64().expect("an order"))
            .collect()
    }

    #[test]
    fn rows_are_placed_apart_and_one_moving_changes_its_own_order_alone() {
        let mut orders = Orders::default();
        assert_eq!(
            placed(&mut orders, &["a", "b", "c", "d"]),
            [1024, 2048, 3072, 4096]
        );
        assert_eq!(
            placed(&mut orders, &["a", "d", "b", "c"]),
            [1024, 1536, 2048, 3072]
        );
        assert_eq!(
            placed(&mut orders, &["e", "a", "d", "b", "c", "f"]),
            [0, 1024, 1536, 2048, 3072, 4096]
        );
    }

    #[test]
    fn with_no_room_left_every_row_is_placed_afresh() {
        let mut orders = Orders::default();
        placed(&mut orders, &["a", "b"]);
        let mut squeezed = vec!["a"];
        let names: Vec<String> = (0..1100).map(|n| format!("n{n}")).collect();
        squeezed.extend(names.iter().map(String::as_str));
        squeezed.push("b");
        let said = placed(&mut orders, &squeezed);
        let spaced: Vec<i64> = (1..=1102).map(|n| n * SPACING).collect();
        assert_eq!(said, spaced);
    }
}
