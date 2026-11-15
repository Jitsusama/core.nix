//! An HP printer as a source: what its LEDM pages say, the XML pages its
//! own web page reads, asked of each printer eero lists
//! (contract/hp/ holds a real one's answers, redacted). The printer is
//! the device eero already lists by its hardware address, so it takes
//! the printer role there; what it's doing, its ink and the pages it has
//! printed are its printer block, and every page stays under `detail.hp`,
//! by the page's name.
//!
//! Only reads are asked: four fixed pages under `/DevMgmt/`, at the
//! address eero gives and nowhere an answer names. Printing, scanning,
//! changing a setting or acknowledging an alert waits on Joel's yes.

use std::collections::BTreeMap;

use serde_json::{Map, Value};

use crate::model::{Ink, Printer, Source, SourceState};
use crate::xml;

/// The source's name, and the key its answers sit under in `detail`.
pub const NAME: &str = "hp";

/// The pages asked, each only read: what it's doing and its alerts, its
/// cartridges, what it is, and what it has printed.
pub const PAGES: [&str; 4] = [
    "ProductStatusDyn",
    "ConsumableConfigDyn",
    "ProductConfigDyn",
    "ProductUsageDyn",
];

/// Each page's text, by its name.
pub type Answers = BTreeMap<String, String>;

/// Where a printer serves a page.
pub fn url(address: &str, page: &str) -> String {
    format!("http://{address}/DevMgmt/{page}.xml")
}

/// What a printer said, for the device eero lists by `mac`.
#[derive(Debug, Clone, PartialEq)]
pub struct Found {
    pub mac: String,
    pub model: Option<String>,
    pub firmware: Option<String>,
    pub serial: Option<String>,
    pub printer: Printer,
    pub detail: Value,
}

/// The printers eero lists, each by its hardware address and the address
/// eero reaches it at: a device whose maker is HP that eero takes for a
/// printer, so an HP laptop isn't asked.
pub fn listed(eero: &Value) -> Vec<(String, String)> {
    let Some(Value::Array(devices)) = eero.get("devices") else {
        return Vec::new();
    };
    devices
        .iter()
        .filter(|device| {
            let said = |key| device.get(key).and_then(Value::as_str);
            let hp = ["make", "manufacturer"].into_iter().any(|key| {
                said(key)
                    .is_some_and(|maker| maker.starts_with("HP") || maker.starts_with("Hewlett"))
            });
            hp && said("kind") == Some("printer")
        })
        .filter_map(|device| {
            let mac = device.get("mac")?.as_str()?;
            let address = device.get("address")?.as_str()?;
            Some((mac.to_owned(), address.to_owned()))
        })
        .collect()
}

/// Each printer as it answered, and the source, or none when eero lists
/// no printer. The source answers while any printer does.
pub fn read(
    said: Vec<(String, Result<Answers, String>)>,
    now: u64,
) -> (Option<Source>, Vec<Found>) {
    if said.is_empty() {
        return (None, Vec::new());
    }
    let mut found = Vec::new();
    let mut reason = None;
    for (mac, answers) in said {
        match answers {
            Ok(answers) => found.push(heard(mac, &answers)),
            Err(why) => reason = reason.or(Some(why)),
        }
    }
    let answering = !found.is_empty();
    let source = Source {
        name: NAME.into(),
        what: "the HP printers eero lists, read and never changed".into(),
        state: if answering {
            SourceState::Answering
        } else {
            SourceState::Unreachable
        },
        read_at: answering.then_some(now),
        since: (!answering).then_some(now),
        reason: if answering { None } else { reason },
    };
    (Some(source), found)
}

/// One printer from its pages, each kept whole in its own words.
fn heard(mac: String, answers: &Answers) -> Found {
    let mut detail = Map::new();
    for (name, text) in answers {
        if let Some((_, value)) = xml::read(text) {
            detail.insert(name.clone(), value);
        }
    }
    let product = detail
        .get("ProductConfigDyn")
        .and_then(|page| page.get("ProductInformation"));
    let said = |field: &str| {
        product
            .and_then(|product| product.get(field))
            .and_then(text)
            .map(str::to_owned)
    };
    let firmware = product
        .and_then(|product| product.get("Version"))
        .and_then(|version| version.get("Revision"))
        .and_then(text)
        .map(str::to_owned);
    let state = listed_in(detail.get("ProductStatusDyn"), "Status")
        .into_iter()
        .filter_map(|status| status.get("StatusCategory").and_then(text))
        .map(doing)
        .collect();
    let ink = listed_in(detail.get("ConsumableConfigDyn"), "ConsumableInfo")
        .into_iter()
        .filter(|cartridge| {
            cartridge.get("ConsumableTypeEnum").and_then(text) == Some("inkCartridge")
        })
        .filter_map(|cartridge| {
            Some(Ink {
                colour: colour(cartridge.get("ConsumableLabelCode").and_then(text)?),
                left_percent: cartridge
                    .get("ConsumablePercentageLevelRemaining")
                    .and_then(text)
                    .and_then(|left| left.parse().ok()),
                state: cartridge
                    .get("ConsumableLifeState")
                    .and_then(|life| life.get("MeasuredQuantityState"))
                    .and_then(text)
                    .map(words),
            })
        })
        .collect();
    let pages = detail
        .get("ProductUsageDyn")
        .and_then(|page| page.get("PrinterSubunit"))
        .and_then(|printer| printer.get("TotalImpressions"))
        .and_then(text)
        .and_then(|pages| pages.parse().ok());
    Found {
        mac,
        model: said("MakeAndModel"),
        firmware,
        serial: said("SerialNumber"),
        printer: Printer { state, ink, pages },
        detail: Value::Object(detail),
    }
}

/// The elements named `name` in a page, one or many.
fn listed_in<'a>(page: Option<&'a Value>, name: &str) -> Vec<&'a Value> {
    match page.and_then(|page| page.get(name)) {
        Some(Value::Array(items)) => items.iter().collect(),
        Some(item) => vec![item],
        None => Vec::new(),
    }
}

/// An element's text, whether it's alone or beside the element's
/// attributes.
fn text(value: &Value) -> Option<&str> {
    match value {
        Value::String(text) => Some(text.as_str()),
        Value::Object(fields) => fields.get("text").and_then(Value::as_str),
        _ => None,
    }
    .filter(|text| !text.is_empty())
}

/// What a printer is doing, in plain words, from its status category; a
/// category not named here is its own word, split and lowered.
fn doing(category: &str) -> String {
    match category {
        "ready" => "ready".into(),
        "processing" => "printing".into(),
        "inPowerSave" => "saving power".into(),
        "trayEmpty" => "out of paper".into(),
        other => words(other),
    }
}

/// A cartridge's colour from its label's code: K for black, the letters
/// of cyan, magenta and yellow, or CMY for one cartridge of all three.
fn colour(code: &str) -> String {
    match code {
        "K" => "black".into(),
        "CMY" => "colour".into(),
        "C" => "cyan".into(),
        "M" => "magenta".into(),
        "Y" => "yellow".into(),
        other => other.to_lowercase(),
    }
}

/// An HP word, such as `veryLow`, as words: very low.
fn words(word: &str) -> String {
    let mut said = String::new();
    for character in word.chars() {
        if character.is_uppercase() && !said.is_empty() {
            said.push(' ');
        }
        said.extend(character.to_lowercase());
    }
    said
}

#[cfg(test)]
pub mod tests {
    use super::*;

    use std::fs;

    use serde_json::json;

    /// The contract's printer: its hardware address and address, as the
    /// eero record the collector's tests add it to gives them.
    pub const MAC: &str = "02:00:00:00:00:70";
    pub const ADDRESS: &str = "10.0.0.70";

    const NOW: u64 = 1_794_702_033;

    /// What the contract's printer answered, each page by its name.
    pub fn answers() -> Answers {
        PAGES
            .into_iter()
            .map(|page| {
                let file = format!("{}/contract/hp/{page}.xml", env!("CARGO_MANIFEST_DIR"));
                let text = fs::read_to_string(&file).expect("the contract has each page");
                (page.to_owned(), text)
            })
            .collect()
    }

    /// The printer as eero lists it.
    pub fn listing() -> Value {
        json!({
            "mac": MAC, "name": "Printer", "nickname": "Printer",
            "hostname": "hp000070", "kind": "printer", "make": "HP",
            "manufacturer": "Hewlett Packard", "address": ADDRESS,
            "addresses": [ADDRESS], "node": "2003", "ethernet": false,
            "band": "2.4 GHz", "channel": 6,
        })
    }

    #[test]
    fn an_hp_eero_takes_for_a_printer_is_listed_and_no_other() {
        let eero = json!({"devices": [
            listing(),
            {"mac": "02:00:00:00:00:71", "make": "HP", "kind": "computer", "address": "10.0.0.71"},
            {"mac": "02:00:00:00:00:72", "make": "Brother", "kind": "printer",
             "address": "10.0.0.72"},
            {"mac": "02:00:00:00:00:73", "manufacturer": "Hewlett-Packard Company",
             "kind": "printer", "address": "10.0.0.73"},
        ]});
        assert_eq!(
            listed(&eero),
            [
                (MAC.to_owned(), ADDRESS.to_owned()),
                ("02:00:00:00:00:73".to_owned(), "10.0.0.73".to_owned()),
            ],
            "the wrong devices were taken for printers"
        );
    }

    #[test]
    fn each_page_is_asked_at_the_printer_eero_gives() {
        assert_eq!(
            url(ADDRESS, "ProductStatusDyn"),
            "http://10.0.0.70/DevMgmt/ProductStatusDyn.xml",
            "a page is asked somewhere else"
        );
    }

    #[test]
    fn a_printer_says_its_model_ink_and_what_its_doing() {
        let (source, found) = read(vec![(MAC.into(), Ok(answers()))], NOW);
        let source = source.expect("a printer makes a source");
        assert_eq!(source.state, SourceState::Answering, "it isn't answering");
        let [found] = found.as_slice() else {
            panic!("one printer should be found: {found:?}");
        };
        assert_eq!(found.mac, MAC, "it's found for another device");
        assert_eq!(
            found.model.as_deref(),
            Some("OfficeJet 3830 All-in-One Printer series"),
            "its model wasn't read"
        );
        assert_eq!(
            found.firmware.as_deref(),
            Some("SPP5FN2223AR"),
            "its firmware wasn't read"
        );
        assert_eq!(
            found.serial.as_deref(),
            Some("SERIAL00000070"),
            "its serial wasn't read"
        );
        assert_eq!(
            found.printer,
            Printer {
                state: vec!["out of paper".into(), "saving power".into()],
                ink: vec![
                    Ink {
                        colour: "colour".into(),
                        left_percent: Some(60),
                        state: Some("ok".into()),
                    },
                    Ink {
                        colour: "black".into(),
                        left_percent: Some(50),
                        state: Some("ok".into()),
                    },
                ],
                pages: Some(1900),
            },
            "its printer block is wrong"
        );
        assert_eq!(
            found.detail["ProductStatusDyn"]["AlertTable"]["ModificationNumber"],
            json!("7"),
            "its status page wasn't kept in its own words"
        );
        assert_eq!(
            found.detail["ProductConfigDyn"]["ProductSettings"]["PowerSaveTimeout"],
            json!("5minutes"),
            "its settings weren't kept in their own words"
        );
    }

    #[test]
    fn each_status_reads_in_plain_words() {
        for (category, words) in [
            ("ready", "ready"),
            ("processing", "printing"),
            ("inPowerSave", "saving power"),
            ("trayEmpty", "out of paper"),
            ("jamInPrinter", "jam in printer"),
        ] {
            assert_eq!(doing(category), words, "{category} reads wrong");
        }
    }

    #[test]
    fn each_cartridge_is_named_by_its_colour() {
        for (code, said) in [
            ("K", "black"),
            ("CMY", "colour"),
            ("C", "cyan"),
            ("M", "magenta"),
            ("Y", "yellow"),
        ] {
            assert_eq!(colour(code), said, "{code} reads wrong");
        }
    }

    #[test]
    fn a_printer_that_doesnt_answer_is_said_and_nothing_made_up() {
        let (source, found) = read(vec![(MAC.into(), Err("timed out".into()))], NOW);
        let source = source.expect("a listed printer makes a source");
        assert_eq!(
            source.state,
            SourceState::Unreachable,
            "it read as answering"
        );
        assert_eq!(
            source.reason.as_deref(),
            Some("timed out"),
            "the reason was lost"
        );
        assert!(found.is_empty(), "a printer was made up: {found:?}");
    }

    #[test]
    fn no_printer_listed_is_no_source() {
        let (source, found) = read(Vec::new(), NOW);
        assert!(source.is_none(), "a source with nothing to read was said");
        assert!(found.is_empty(), "a printer was made up");
    }
}
