//! One record of the home network from what each source said: eero's
//! devices and links, then the modem, in that order, so a reader always
//! finds the same device in the same place.

use serde_json::Value;

use crate::model::{Network, SCHEMA};
use crate::sources::{eero, modem};

/// What each source said in one round of reads.
pub struct Said {
    pub eero: Result<Value, String>,
    pub modem: Result<modem::Pages, String>,
}

pub fn network(said: Said, now: u64) -> Network {
    let eero = eero::read(said.eero, now);
    let (modem_source, modem) = modem::read(said.modem, now);
    Network {
        schema: SCHEMA.into(),
        collected_at: now,
        sources: vec![eero.source, modem_source],
        devices: modem.into_iter().chain(eero.devices).collect(),
        links: eero.links,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::fs;

    use crate::model::{Role, SourceState};

    /// When the golden record was collected: five seconds after the
    /// contract's modem read its clock, so its log is placed in UTC.
    const NOW: u64 = 1_794_702_033;

    /// The contract: eero's golden record, and a real modem's pages.
    fn contract() -> Said {
        let eero = serde_json::from_str(include_str!("../../eero/home-network.json"))
            .expect("eero's golden record reads");
        let pages = modem::PAGES
            .iter()
            .map(|page| {
                let file = format!("{}/contract/modem/{page}.json", env!("CARGO_MANIFEST_DIR"));
                let answer = fs::read_to_string(&file).expect("the contract has each page");
                (
                    page.to_string(),
                    serde_json::from_str(&answer).expect("each page is JSON"),
                )
            })
            .collect();
        Said {
            eero: Ok(eero),
            modem: Ok(pages),
        }
    }

    /// The record the views are drawn from in their tests is exactly what
    /// the contract collects into. `NET_BLESS=1 cargo test` writes it anew
    /// after a change that's meant. They're compared as values, since the
    /// repository's formatter lays the file out its own way.
    #[test]
    fn the_contract_collects_into_the_golden_record() {
        let collected = network(contract(), NOW);
        let record = serde_json::to_value(&collected).expect("it writes");
        let golden = format!("{}/tests/network.json", env!("CARGO_MANIFEST_DIR"));
        if std::env::var_os("NET_BLESS").is_some() {
            // Written from the model, not the value, so fields keep the
            // model's order rather than the alphabet's.
            let text = serde_json::to_string_pretty(&collected).expect("it writes") + "\n";
            fs::write(&golden, text).expect("the golden record is written");
        }
        let want: Value =
            serde_json::from_str(&fs::read_to_string(&golden).expect("the golden record is there"))
                .expect("the golden record is JSON");
        assert!(
            record == want,
            "the record isn't the golden one; NET_BLESS=1 if that's meant"
        );
    }

    #[test]
    fn the_modem_comes_first_with_every_channel() {
        let network = network(contract(), NOW);
        let first = &network.devices[0];
        assert_eq!(first.roles, [Role::Modem], "the modem isn't first");
        let modem = first.modem.as_ref().expect("the modem has its block");
        assert_eq!(
            modem.downstream.len(),
            16 + 2,
            "a downstream channel went missing"
        );
        assert_eq!(
            modem.upstream.len(),
            4 + 2,
            "an upstream channel went missing"
        );
        assert_eq!(
            modem.docsis.as_deref(),
            Some("3.1"),
            "the DOCSIS version wasn't read"
        );
    }

    #[test]
    fn the_modems_log_is_placed_by_its_clock() {
        let network = network(contract(), NOW);
        let log = &network.devices[0]
            .modem
            .as_ref()
            .expect("the modem has its block")
            .log;
        assert_eq!(
            log[0].clock, "11/12/26 12:40:45",
            "the first line's clock changed"
        );
        assert_eq!(
            log[0].at,
            Some(1_794_487_245),
            "the first line isn't placed in UTC"
        );
    }

    #[test]
    fn the_gateway_is_one_device_with_both_roles_and_its_cable_out() {
        let network = network(contract(), NOW);
        let gateway = network
            .devices
            .iter()
            .find(|device| device.name == "Hall")
            .expect("the gateway node is a device");
        assert_eq!(
            gateway.roles,
            [Role::Router, Role::AccessPoint],
            "its roles are wrong"
        );
        assert!(
            network
                .links
                .iter()
                .any(|link| link.a.device == modem::ID && link.b.device == gateway.id),
            "the modem's cable to the gateway is missing"
        );
    }

    #[test]
    fn a_source_that_didnt_answer_is_said_and_the_rest_kept() {
        let said = Said {
            modem: Err("timed out".into()),
            ..contract()
        };
        let network = network(said, NOW);
        assert_eq!(
            network.sources[1].state,
            SourceState::Unreachable,
            "the modem read as answering"
        );
        assert!(
            network.devices.iter().all(|device| device.id != modem::ID),
            "a modem was made up"
        );
        assert!(
            !network.devices.is_empty(),
            "eero's devices went with the modem"
        );
    }
}
