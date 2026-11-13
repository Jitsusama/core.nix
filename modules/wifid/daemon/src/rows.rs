//! Wi-Fi's rows in the launcher, made ready here so the shell only draws
//! them: Wi-Fi Off (or On), Sign In while a network wants it, and every
//! network in the air, the one connected first, then those saved, then the
//! rest by signal, each worded from the launcher's vocabulary
//! (launcher/words.json) as the shell hands it over, so wifid says no word
//! of its own. Each row is the ready-row form every daemon serves
//! (QEST-20261023-Y2YBTS attachments/speed/ready-rows.md), sent whole only
//! when something it draws changes (face.rs).

use std::cmp::Ordering;

use serde_json::{Value, json};

use crate::state::{Described, Reach, Saved, Security};
use crate::words::Words;

/// The launcher's place Wi-Fi's rows are in.
pub const PLACE: &str = "wifi";

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
/// `order`, which ready-rows' `Kept` gives it (face.rs). A machine with no
/// card has none.
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
            "key": key("sign-in"), "thing": "wifi:sign-in", "place": PLACE, "kind": "sign-in",
            "name": words.sign_in.name.replace("{network}", network),
            "glyph": words.sign_in.glyph, "hue": "machine", "depth": 0, "side": [],
            "held": {"network": network},
        }));
    }
    if described.radio {
        for network in networks(described) {
            rows.push(with_acts(&network, signing, words));
        }
    }
    let mut radio = radio_row(described.radio, words);
    radio["acts"] = json!([{
        "key": "wifi:radio",
        "ask": {"ask": "act", "act": "radio", "on": !described.radio},
        "becomes": radio_row(!described.radio, words),
    }]);
    rows.push(radio);
    rows
}

/// The radio's row, Wi-Fi Off while it's on.
fn radio_row(on: bool, words: &Words) -> Value {
    let radio = if on {
        &words.radio_on
    } else {
        &words.radio_off
    };
    json!({
        "key": key("radio"), "thing": "wifi:radio", "place": PLACE, "kind": "radio",
        "name": radio.name, "glyph": radio.glyph, "hue": "machine", "depth": 0, "side": [],
        "held": {"on": on},
    })
}

/// A network's row with its acts, each the ask that does it and the row
/// it becomes once done, in wifid's words, so the shell draws the change
/// at once and words nothing (launcher/Becoming.js). A join's fields ask
/// Joel to type what it needs on the way, each answer set in the ask where
/// its field says, and its stages are said in the foot as it goes
/// (ready-rows.md's fields and streams), so the shell builds no act's row
/// (Y2YBTS's 17a4dc7e, 1ace0a22; 2026-11-13).
fn with_acts(network: &Network, signing: Option<&str>, words: &Words) -> Value {
    let mut row = network_row(network, signing, words);
    let ask = |act: &str| json!({"ask": "act", "act": act, "network": network.name});
    let mut acts = Vec::new();
    // The sign-in window is a program of wifid's own, which the shell
    // opens (SignIn.qml), so signing in goes to it rather than asking.
    if network.connected && signing == Some(network.name) {
        acts.push(json!({"key": "verb:wifi-sign-in", "go": "wifid-sign-in"}));
    }
    if network.connected {
        let left = Network {
            connected: false,
            ..network.clone()
        };
        // A network left was one joined, so it's saved, and joining it again is how u takes
        // the leaving back.
        let rejoin = json!({"key": "verb:wifi-join", "ask": ask("join"), "streams": true});
        acts.push(json!({"key": "verb:wifi-leave", "ask": ask("leave"),
            "becomes": network_row(&left, None, words), "undo": rejoin}));
    }
    acts.extend(joins(network));
    // Nix keeps what it declares, so only a rebuild trusts or forgets it.
    if let Some(kept) = network.kept.filter(|kept| !kept.declared) {
        let mut trusting = ask("trust");
        trusting["trusted"] = json!(!kept.trusted);
        let mut trusted = row.clone();
        trusted["held"]["trusted"] = json!(!kept.trusted);
        let mut trust = json!({"key": "verb:wifi-trust", "ask": trusting, "becomes": trusted});
        // Trusted, its act reads as stopping, in its words' own state.
        if kept.trusted {
            trust["wordState"] = json!("trusted");
        }
        acts.push(trust);
        let forgotten = Network {
            kept: None,
            ..network.clone()
        };
        acts.push(json!({"key": "verb:wifi-forget", "ask": ask("forget"),
            "becomes": network_row(&forgotten, signing, words)}));
    }
    row["acts"] = json!(acts);
    row
}

/// How a network not connected is joined: as it's saved, or, met for the
/// first time, asking its password, or its account over PEAP, on the way;
/// then, unless Nix keeps them, with a new password (one saved behind an
/// account), or an account over PEAP or TTLS, each but the one the first
/// join already asks.
fn joins(network: &Network) -> Vec<Value> {
    let met = !network.known();
    let typed = !network.kept.is_some_and(|kept| kept.declared);
    let password = || vec![field("password", "password")];
    let account = || {
        vec![
            field("identity", "account.identity"),
            field("server", "account.serverDomain"),
            field("password", "password"),
        ]
    };
    let mut acts = Vec::new();
    let mut asked = "";
    if !network.connected {
        let (fields, method) = match network.security {
            Security::Personal if met && typed => (password(), None),
            Security::Enterprise if met && typed => (account(), Some("peap")),
            _ => (Vec::new(), None),
        };
        asked = match method {
            Some(method) => method,
            None if fields.is_empty() => "",
            None => "password",
        };
        acts.push(join(network, "verb:wifi-join", method, fields));
    }
    if !typed {
        return acts;
    }
    let newly = match network.security {
        Security::Open => false,
        Security::Personal => asked != "password",
        Security::Enterprise => !met,
    };
    if newly {
        acts.push(join(network, "verb:wifi-password", None, password()));
    }
    if network.security == Security::Enterprise {
        if asked != "peap" {
            acts.push(join(
                network,
                "verb:wifi-account-peap",
                Some("peap"),
                account(),
            ));
        }
        acts.push(join(
            network,
            "verb:wifi-account-ttls",
            Some("ttls"),
            account(),
        ));
    }
    acts
}

/// A join, the account's method set where it's over one, its fields asked
/// first where it has any; its stages are said as it goes (acts.rs).
fn join(network: &Network, key: &str, method: Option<&str>, fields: Vec<Value>) -> Value {
    let mut ask = json!({"ask": "act", "act": "join", "network": network.name});
    if let Some(method) = method {
        ask["account"] = json!({ "method": method });
    }
    let mut act = json!({"key": key, "ask": ask, "streams": true});
    if !fields.is_empty() {
        act["fields"] = Value::Array(fields);
    }
    act
}

/// A field a join asks Joel to type into, its prompt the words key
/// `wifi:asks:<asks>` (launcher/words/wifi.json), its answer set at `into`
/// in the ask; a password typed unseen.
fn field(asks: &str, into: &str) -> Value {
    let mut field = json!({"ask": format!("wifi:asks:{asks}"), "into": into});
    if asks == "password" {
        field["secret"] = json!(true);
    }
    field
}

pub fn key(id: &str) -> String {
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
        "thing": format!("wifi:{}", network.name), "place": PLACE,
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

/// Wi-Fi's part of the launcher's vocabulary, each entry copied whole from
/// launcher/words.json into tests/words.json, which
/// the-daemons-test-the-launchers-words holds to it, so no case passes on
/// words Joel never reads, as Machine's did on eleven (2026-11-11).
#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{Card, Nearby};
    use crate::words::launchers_words;

    fn words() -> Words {
        Words::read(&launchers_words()).expect("the vocabulary has Wi-Fi's words")
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
        assert_eq!(rows[0]["thing"], "wifi:Lantern");
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

    fn act<'a>(row: &'a Value, key: &str) -> &'a Value {
        row["acts"]
            .as_array()
            .and_then(|acts| acts.iter().find(|act| act["key"] == key))
            .unwrap_or(&Value::Null)
    }

    #[test]
    fn leaving_the_network_joined_becomes_its_row_as_one_saved() {
        let rows = rows(&described(), &words());
        let leave = act(&rows[0], "verb:wifi-leave");
        assert_eq!(
            leave["ask"],
            json!({"ask": "act", "act": "leave", "network": "Lantern"})
        );
        assert_eq!(
            leave["becomes"]["side"],
            json!([{"said": "saved"}, {"said": "3 of 4 bars"}])
        );
        assert_eq!(leave["becomes"]["held"]["connected"], false);
        assert_eq!(
            leave["undo"]["ask"],
            json!({"ask": "act", "act": "join", "network": "Lantern"}),
            "u joins it again"
        );
        assert_eq!(act(&rows[1], "verb:wifi-leave"), &Value::Null);
    }

    #[test]
    fn a_network_saved_here_is_trusted_and_forgotten_as_its_row_says() {
        let rows = rows(&described(), &words());
        let trust = act(&rows[1], "verb:wifi-trust");
        assert_eq!(trust["ask"]["trusted"], false);
        assert_eq!(trust["becomes"]["held"]["trusted"], false);
        let forget = act(&rows[1], "verb:wifi-forget");
        assert_eq!(forget["ask"]["act"], "forget");
        assert_eq!(forget["becomes"]["side"], json!([{"said": "2 of 4 bars"}]));
        assert_eq!(forget["becomes"]["held"]["known"], false);
        // One never met, or one Nix declares, has neither.
        assert_eq!(act(&rows[2], "verb:wifi-trust"), &Value::Null);
        assert_eq!(act(&rows[2], "verb:wifi-forget"), &Value::Null);
        let mut declared = described();
        declared.saved[1].declared = true;
        let rows = super::rows(&declared, &words());
        assert_eq!(act(&rows[1], "verb:wifi-forget"), &Value::Null);
    }

    /// The keys of the acts on the network's row by its name, in the order it lists them.
    fn acts_on<'a>(rows: &'a [Value], name: &str) -> Vec<&'a str> {
        let row = rows
            .iter()
            .find(|row| row["name"] == name)
            .unwrap_or_else(|| panic!("no row for {name}"));
        let acts = row["acts"].as_array().map_or(&[][..], Vec::as_slice);
        acts.iter().filter_map(|act| act["key"].as_str()).collect()
    }

    #[test]
    fn each_network_s_acts_come_in_the_order_its_row_lists_them() {
        let mut air = described();
        air.nearby.push(nearby("Attic", Security::Personal, 2));
        let rows = rows(&air, &words());
        assert_eq!(
            acts_on(&rows, "Lantern"),
            [
                "verb:wifi-leave",
                "verb:wifi-password",
                "verb:wifi-trust",
                "verb:wifi-forget"
            ]
        );
        assert_eq!(
            acts_on(&rows, "Harbour"),
            [
                "verb:wifi-join",
                "verb:wifi-password",
                "verb:wifi-trust",
                "verb:wifi-forget"
            ]
        );
        assert_eq!(acts_on(&rows, "Cafe 9"), ["verb:wifi-join"]);
        // Met for the first time, its join asks the password, so it isn't offered twice.
        assert_eq!(acts_on(&rows, "Attic"), ["verb:wifi-join"]);
        // Behind an account, its join asks it over PEAP, and TTLS is offered beside it.
        assert_eq!(
            acts_on(&rows, "Campus"),
            ["verb:wifi-join", "verb:wifi-account-ttls"]
        );
    }

    #[test]
    fn a_network_met_for_the_first_time_asks_its_password_on_the_way() {
        let mut air = described();
        air.nearby.push(nearby("Attic", Security::Personal, 2));
        let rows = rows(&air, &words());
        let attic = rows
            .iter()
            .find(|row| row["name"] == "Attic")
            .expect("Attic");
        assert_eq!(
            act(attic, "verb:wifi-join"),
            &json!({"key": "verb:wifi-join", "streams": true,
                "ask": {"ask": "act", "act": "join", "network": "Attic"},
                "fields": [{"ask": "wifi:asks:password", "into": "password", "secret": true}]})
        );
        // One saved joins as it's saved, asking nothing, and says its way as it goes.
        assert_eq!(
            act(&rows[1], "verb:wifi-join"),
            &json!({"key": "verb:wifi-join", "streams": true,
                "ask": {"ask": "act", "act": "join", "network": "Harbour"}})
        );
    }

    #[test]
    fn an_account_is_asked_its_name_its_server_s_domain_then_its_password() {
        let rows = rows(&described(), &words());
        let campus = &rows[4];
        for (key, method) in [
            ("verb:wifi-join", "peap"),
            ("verb:wifi-account-ttls", "ttls"),
        ] {
            let joining = act(campus, key);
            assert_eq!(
                joining["ask"]["account"],
                json!({"method": method}),
                "{key}"
            );
            assert_eq!(
                joining["fields"],
                json!([
                    {"ask": "wifi:asks:identity", "into": "account.identity"},
                    {"ask": "wifi:asks:server", "into": "account.serverDomain"},
                    {"ask": "wifi:asks:password", "into": "password", "secret": true}
                ]),
                "{key}"
            );
        }
    }

    #[test]
    fn a_network_nix_declares_is_joined_but_never_asks_a_password() {
        let mut declared = described();
        declared.saved[1].declared = true;
        let rows = rows(&declared, &words());
        assert_eq!(acts_on(&rows, "Harbour"), ["verb:wifi-join"]);
    }

    #[test]
    fn the_network_wanting_a_sign_in_signs_in_first() {
        let mut wanting = described();
        wanting.cards = vec![card(Some("Lantern"), Some(Reach::SignIn))];
        let rows = rows(&wanting, &words());
        assert_eq!(
            rows[1]["acts"][0],
            json!({"key": "verb:wifi-sign-in", "go": "wifid-sign-in"})
        );
        assert_eq!(rows[1]["acts"][1]["key"], "verb:wifi-leave");
    }

    #[test]
    fn a_trusted_network_s_act_reads_as_stopping_trusting_it() {
        let rows = rows(&described(), &words());
        assert_eq!(act(&rows[1], "verb:wifi-trust")["wordState"], "trusted");
        let mut doubted = described();
        doubted.saved[1].trusted = false;
        let rows = super::rows(&doubted, &words());
        assert_eq!(act(&rows[1], "verb:wifi-trust").get("wordState"), None);
    }

    #[test]
    fn a_join_s_stages_and_refusals_are_said_in_the_launcher_s_words() {
        let words = words();
        assert_eq!(
            words.staged("gettingAddress", "Lantern").as_deref(),
            Some("getting an address from Lantern\u{2026}")
        );
        assert_eq!(
            words.refused("WrongPassword"),
            Some("that password didn't work")
        );
        assert_eq!(words.staged("dancing", "Lantern"), None);
        assert_eq!(words.refused("Dancing"), None);
    }

    #[test]
    fn the_radio_s_act_turns_it_the_other_way() {
        let rows = rows(&described(), &words());
        let radio = rows.last().expect("a radio row");
        let turn = act(radio, "wifi:radio");
        assert_eq!(
            turn["ask"],
            json!({"ask": "act", "act": "radio", "on": false})
        );
        assert_eq!(turn["becomes"]["name"], "Wi-Fi On");
        assert_eq!(turn["becomes"]["held"]["on"], false);
    }

    #[test]
    fn a_machine_without_a_card_has_no_rows() {
        let mut none = described();
        none.cards.clear();
        assert_eq!(rows(&none, &words()), Vec::<Value>::new());
    }
}
