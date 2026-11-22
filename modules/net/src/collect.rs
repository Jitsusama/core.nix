//! One record of the home network from what each source said: the modem,
//! the switch, then eero's devices and links, in that order, so a reader
//! always finds the same device in the same place, each receiver's,
//! printer's, Apple TV's and television's own figures on eero's device
//! for it; then
//! where it runs out of room (src/room.rs).

use serde_json::Value;

use crate::model::{self, Device, Kept, LinkState, Medium, Network, SCHEMA};
use crate::room;
use crate::sources::{airplay, eero, hp, modem, samsung, sony, switch};

/// What each source said in one round of reads.
pub struct Said {
    pub eero: Result<Value, String>,
    pub modem: Result<modem::Pages, String>,
    pub switch: Result<switch::Report, String>,
    /// What each receiver eero lists answered, by its hardware address.
    pub receivers: Vec<(String, Result<sony::Answers, String>)>,
    /// What each printer eero lists answered, by its hardware address.
    pub printers: Vec<(String, Result<hp::Answers, String>)>,
    /// What each Apple TV eero lists answered, by its hardware address.
    pub apple_tvs: Vec<(String, Result<airplay::Answer, String>)>,
    /// What each Samsung box eero lists that could be a TV answered, and
    /// its last answer as a TV.
    pub televisions: Vec<samsung::Asked>,
}

/// One minute's record, each source carried over from the minutes `kept`
/// before it. They're carried at the record's own time, the clock each
/// source stamps a new outage with, since an outage carries only from a
/// source whose since is that minute.
pub fn carried(said: Said, now: u64, kept: &mut Vec<Kept>) -> Network {
    let mut network = network(said, now);
    model::carry_over(&mut network.sources, kept, network.collected_at);
    network
}

pub fn network(said: Said, now: u64) -> Network {
    let mut eero = eero::read(said.eero, now);
    let (modem_source, modem) = modem::read(said.modem, now);
    let (switch_source, switch) = switch::read(said.switch, now);
    let (receiver_source, receivers) = sony::read(said.receivers, now);
    for found in receivers {
        hear_the_receiver(&mut eero.devices, found);
    }
    let (printer_source, printers) = hp::read(said.printers, now);
    for found in printers {
        hear_the_printer(&mut eero.devices, found);
    }
    let (apple_tv_source, apple_tvs) = airplay::read(said.apple_tvs, now);
    for found in apple_tvs {
        hear_the_apple_tv(&mut eero.devices, found);
    }
    let (television_source, televisions) = samsung::read(said.televisions, now);
    for found in televisions {
        hear_the_television(&mut eero.devices, found);
    }
    if let Some(switch) = &switch {
        // eero lists the switch as a wired client by the same hardware
        // address; the switch's own source says more, so it stands for
        // the box, and eero's cable to it now reaches it.
        eero.devices.retain(|device| device.id != switch.id);
        reach_the_switch(&mut eero.links, switch);
    }
    let mut network = Network {
        schema: SCHEMA.into(),
        collected_at: now,
        sources: [eero.source, modem_source, switch_source]
            .into_iter()
            .chain(receiver_source)
            .chain(printer_source)
            .chain(apple_tv_source)
            .chain(television_source)
            .collect(),
        devices: modem
            .into_iter()
            .chain(switch)
            .chain(eero.devices)
            .collect(),
        links: eero.links,
        internet: eero.internet,
        concerns: Vec::new(),
    };
    room::work_out(&mut network);
    network
}

/// Puts what a receiver said on eero's device for it: it's a receiver
/// rather than a client, its model is its own, and its answers sit beside
/// eero's. A receiver eero no longer lists is left out, since eero's
/// device is what places it on the network.
fn hear_the_receiver(devices: &mut [Device], found: sony::Found) {
    let id = format!("mac:{}", found.mac);
    let Some(device) = devices.iter_mut().find(|device| device.id == id) else {
        return;
    };
    device.roles = vec![crate::model::Role::Receiver];
    device.model = found.model.or(device.model.take());
    device.receiver = Some(found.receiver);
    device.detail.insert(sony::NAME.into(), found.detail);
}

/// Puts what a printer said on eero's device for it, as a receiver's is:
/// it's a printer rather than a client, its model, firmware and serial are
/// its own, and its pages sit beside eero's.
fn hear_the_printer(devices: &mut [Device], found: hp::Found) {
    let id = format!("mac:{}", found.mac);
    let Some(device) = devices.iter_mut().find(|device| device.id == id) else {
        return;
    };
    device.roles = vec![crate::model::Role::Printer];
    device.model = found.model.or(device.model.take());
    device.firmware = found.firmware.or(device.firmware.take());
    device.serial = found.serial.or(device.serial.take());
    device.printer = Some(found.printer);
    device.detail.insert(hp::NAME.into(), found.detail);
}

/// Puts what an Apple TV said on eero's device for it: it stays a client,
/// since a television is a box with a screen of its own and an Apple TV
/// has none, but its model and build are its own, and its answer sits
/// beside eero's.
fn hear_the_apple_tv(devices: &mut [Device], found: airplay::Found) {
    let id = format!("mac:{}", found.mac);
    let Some(device) = devices.iter_mut().find(|device| device.id == id) else {
        return;
    };
    device.model = found.model.or(device.model.take());
    device.firmware = found.firmware.or(device.firmware.take());
    device.detail.insert(airplay::NAME.into(), found.detail);
}

/// Puts what a TV said on eero's device for it: it's a television rather
/// than a client, its model and whether it's on are its own, and its
/// answer, when it gave one this minute, sits beside eero's. The name it
/// was given on its own screen stands for eero's, which is only what it
/// calls itself taking an address, unless Joel nicknamed it in eero.
fn hear_the_television(devices: &mut [Device], found: samsung::Found) {
    let id = format!("mac:{}", found.mac);
    let Some(device) = devices.iter_mut().find(|device| device.id == id) else {
        return;
    };
    let nicknamed = device
        .detail
        .get(eero::NAME)
        .and_then(|said| said.get("nickname"))
        .is_some_and(Value::is_string);
    if let (false, Some(name)) = (nicknamed, found.name) {
        device.name = name;
    }
    device.roles = vec![crate::model::Role::Television];
    device.model = found.model.or(device.model.take());
    device.television = Some(found.television);
    if let Some(detail) = found.detail {
        device.detail.insert(samsung::NAME.into(), detail);
    }
}

/// Gives eero's cable to the switch the switch's end: when one port alone
/// is linked, the cable must come in on it, since the switch reaches
/// nothing else; with more, the pages don't say which, so it's left unsaid.
fn reach_the_switch(links: &mut [crate::model::Link], switch: &Device) {
    let mut linked = switch
        .ports
        .iter()
        .filter(|port| port.link == Some(LinkState::Up));
    let (Some(port), None) = (linked.next(), linked.next()) else {
        return;
    };
    for link in links
        .iter_mut()
        .filter(|link| link.medium == Medium::Ethernet)
    {
        if link.a.device == switch.id && link.a.port.is_none() {
            link.a.port = Some(port.name.clone());
            link.speed_mbps = link.speed_mbps.or(port.speed_mbps);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::fs;

    use crate::model::{Role, SourceState};
    use crate::sources::switch;

    /// When the golden record was collected: five seconds after the
    /// contract's modem read its clock, so its log is placed in UTC.
    const NOW: u64 = 1_794_702_033;

    /// The contract: eero's golden record with the stereo in it, a real
    /// modem's and switch's pages, and a real receiver's answers.
    fn contract() -> Said {
        let mut eero: Value = serde_json::from_str(include_str!("../../eero/home-network.json"))
            .expect("eero's golden record reads");
        if let Some(Value::Array(devices)) = eero.get_mut("devices") {
            devices.push(sony::tests::listing());
            devices.push(hp::tests::listing());
            devices.push(airplay::tests::listing());
            devices.push(samsung::tests::listing());
        }
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
        let switch = switch::PAGES
            .iter()
            .map(|page| {
                let file = format!("{}/contract/switch/{page}.htm", env!("CARGO_MANIFEST_DIR"));
                (
                    page.to_string(),
                    fs::read_to_string(&file).expect("the contract has each page"),
                )
            })
            .collect();
        let status = switch::parse(&switch).expect("the switch's contract reads");
        Said {
            eero: Ok(eero),
            modem: Ok(pages),
            switch: Ok(switch::Report {
                schema: switch::SCHEMA.into(),
                read_at: NOW - 3,
                state: SourceState::Answering,
                reason: None,
                status: Some(status),
            }),
            receivers: vec![(sony::tests::MAC.into(), Ok(sony::tests::answers()))],
            printers: vec![(hp::tests::MAC.into(), Ok(hp::tests::answers()))],
            apple_tvs: vec![(airplay::tests::MAC.into(), Ok(airplay::tests::answer()))],
            televisions: vec![samsung::tests::asked()],
        }
    }

    /// The contract, with eero seeing the switch as a wired client on the
    /// gateway's second port, as it does once the switch takes a lease.
    fn with_the_switch_behind_the_gateway() -> Said {
        let mut said = contract();
        if let Ok(Value::Object(eero)) = &mut said.eero
            && let Some(Value::Array(devices)) = eero.get_mut("devices")
        {
            devices.push(serde_json::json!({
                "mac": "02:00:00:00:00:30", "name": "closet", "node": "2003",
                "ethernet": true, "port": "2", "address": "10.0.0.30",
            }));
        }
        said
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

    /// Each ceiling names its way, as a device filling the line does, so a
    /// reader such as the page marks the right figure without reading
    /// net's sentence for it.
    #[test]
    fn each_ceiling_names_its_way() {
        let network = network(contract(), NOW);
        let ways: Vec<Option<&str>> = network
            .concerns
            .iter()
            .filter(|concern| concern.kind == crate::model::ConcernKind::Ceiling)
            .map(|concern| concern.on.as_deref())
            .collect();
        assert_eq!(
            ways,
            [Some("down"), Some("up")],
            "a ceiling without its way"
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
    fn the_switch_is_one_device_though_eero_lists_it_too() {
        let network = network(with_the_switch_behind_the_gateway(), NOW);
        let named: Vec<_> = network
            .devices
            .iter()
            .filter(|device| device.id == "mac:02:00:00:00:00:30")
            .collect();
        assert_eq!(named.len(), 1, "the switch is listed twice");
        assert_eq!(named[0].roles, [Role::Switch], "it isn't a switch");
        assert_eq!(named[0].name, "Closet", "the switch's own name was lost");
    }

    #[test]
    fn the_gateways_cable_reaches_the_switchs_one_linked_port() {
        let network = network(with_the_switch_behind_the_gateway(), NOW);
        let cable = network
            .links
            .iter()
            .find(|link| link.a.device == "mac:02:00:00:00:00:30")
            .expect("the cable to the switch is there");
        assert_eq!(
            cable.a.port.as_deref(),
            Some("8"),
            "the switch's end has no port"
        );
        assert_eq!(
            cable.b.port.as_deref(),
            Some("2"),
            "the gateway's end lost its port"
        );
        assert_eq!(
            cable.speed_mbps,
            Some(1000),
            "the cable's speed isn't the port's"
        );
    }

    #[test]
    fn the_stereo_is_eeros_device_for_it_heard_as_a_receiver() {
        let network = network(contract(), NOW);
        let named: Vec<_> = network
            .devices
            .iter()
            .filter(|device| device.id == "mac:02:00:00:00:00:60")
            .collect();
        let [stereo] = named.as_slice() else {
            panic!("the stereo should be one device: {named:?}");
        };
        assert_eq!(stereo.roles, [Role::Receiver], "it isn't a receiver");
        assert_eq!(stereo.name, "Stereo", "eero's name for it was lost");
        assert_eq!(
            stereo
                .receiver
                .as_ref()
                .and_then(|receiver| receiver.volume),
            Some(27),
            "its volume wasn't heard"
        );
        assert!(
            stereo.detail.contains_key("eero") && stereo.detail.contains_key("sony"),
            "a source's words were lost: {:?}",
            stereo.detail.keys().collect::<Vec<_>>()
        );
        assert!(
            network
                .sources
                .iter()
                .any(|source| source.name == sony::NAME && source.state == SourceState::Answering),
            "the receivers aren't said as a source"
        );
        assert!(
            network
                .links
                .iter()
                .any(|link| link.a.device == stereo.id || link.b.device == stereo.id),
            "eero's link to it was lost"
        );
    }

    #[test]
    fn the_printer_is_eeros_device_for_it_heard_as_a_printer() {
        let network = network(contract(), NOW);
        let named: Vec<_> = network
            .devices
            .iter()
            .filter(|device| device.id == "mac:02:00:00:00:00:70")
            .collect();
        let [printer] = named.as_slice() else {
            panic!("the printer should be one device: {named:?}");
        };
        assert_eq!(printer.roles, [Role::Printer], "it isn't a printer");
        assert_eq!(printer.name, "Printer", "eero's name for it was lost");
        assert_eq!(
            printer.serial.as_deref(),
            Some("SERIAL00000070"),
            "its serial wasn't heard"
        );
        assert_eq!(
            printer.printer.as_ref().map(|said| said.ink.len()),
            Some(2),
            "its ink wasn't heard"
        );
        assert!(
            printer.detail.contains_key("eero") && printer.detail.contains_key("hp"),
            "a source's words were lost: {:?}",
            printer.detail.keys().collect::<Vec<_>>()
        );
        assert!(
            network
                .sources
                .iter()
                .any(|source| source.name == hp::NAME && source.state == SourceState::Answering),
            "the printers aren't said as a source"
        );
    }

    #[test]
    fn the_apple_tv_is_eeros_device_for_it_heard_with_its_model() {
        let network = network(contract(), NOW);
        let named: Vec<_> = network
            .devices
            .iter()
            .filter(|device| device.id == "mac:02:00:00:00:00:80")
            .collect();
        let [apple_tv] = named.as_slice() else {
            panic!("the Apple TV should be one device: {named:?}");
        };
        assert_eq!(apple_tv.roles, [Role::Client], "it isn't a client");
        assert_eq!(apple_tv.name, "Apple TV", "eero's name for it was lost");
        assert_eq!(
            apple_tv.model.as_deref(),
            Some("Apple TV 4K (1st generation)"),
            "its model wasn't heard"
        );
        assert_eq!(
            apple_tv.firmware.as_deref(),
            Some("23L781"),
            "its build wasn't heard"
        );
        assert!(
            apple_tv.detail.contains_key("eero") && apple_tv.detail.contains_key("airplay"),
            "a source's words were lost: {:?}",
            apple_tv.detail.keys().collect::<Vec<_>>()
        );
        assert!(
            network.sources.iter().any(
                |source| source.name == airplay::NAME && source.state == SourceState::Answering
            ),
            "the Apple TVs aren't said as a source"
        );
    }

    /// The contract's TV, collected from `said`.
    fn the_tv(said: Said) -> (Device, Network) {
        let network = network(said, NOW);
        let id = format!("mac:{}", samsung::tests::MAC);
        let named: Vec<_> = network
            .devices
            .iter()
            .filter(|device| device.id == id)
            .collect();
        let [tv] = named.as_slice() else {
            panic!("the TV should be one device: {named:?}");
        };
        ((*tv).clone(), network)
    }

    #[test]
    fn the_tv_is_eeros_device_for_it_heard_as_a_television_by_its_own_name() {
        let (tv, network) = the_tv(contract());
        assert_eq!(tv.roles, [Role::Television], "it isn't a television");
        assert_eq!(
            tv.name, "Living Room",
            "its own name didn't stand for eero's"
        );
        assert_eq!(
            tv.model.as_deref(),
            Some("QN55Q70CAFXZC"),
            "its model wasn't heard"
        );
        assert_eq!(
            tv.television.and_then(|said| said.power).as_deref(),
            Some("on"),
            "its power wasn't heard"
        );
        assert!(
            tv.detail.contains_key("eero") && tv.detail.contains_key("samsung"),
            "a source's words were lost: {:?}",
            tv.detail.keys().collect::<Vec<_>>()
        );
        assert!(
            network.sources.iter().any(
                |source| source.name == samsung::NAME && source.state == SourceState::Answering
            ),
            "the Samsung TVs aren't said as a source"
        );
    }

    #[test]
    fn a_tv_joel_nicknamed_in_eero_keeps_his_name() {
        let mut said = contract();
        if let Ok(Value::Object(eero)) = &mut said.eero
            && let Some(Value::Array(devices)) = eero.get_mut("devices")
        {
            for device in devices.iter_mut() {
                if device["mac"] == samsung::tests::MAC {
                    device["nickname"] = Value::from("Big Screen");
                }
            }
        }
        let (tv, _) = the_tv(said);
        assert_eq!(
            tv.name, "Big Screen",
            "Joel's nickname gave way to the TV's name"
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

    #[test]
    fn an_outage_collected_twice_keeps_its_first_start() {
        // Each minute is kept at the record's own time, the one its sources
        // stamp a new outage with, so the next minute's outage takes the
        // first one's start.
        let mut kept = Vec::new();
        let mut down = |now| {
            let said = Said {
                modem: Err("timed out".into()),
                ..contract()
            };
            carried(said, now, &mut kept)
        };
        down(NOW - 60);
        let second = down(NOW);
        let modem = second
            .sources
            .iter()
            .find(|source| source.name == "modem")
            .expect("the record names the modem's source");
        assert_eq!(
            modem.since,
            Some(NOW - 60),
            "the modem's outage began again at the minute it was collected"
        );
        assert!(
            kept.iter().all(|(at, _)| *at == second.collected_at),
            "a minute was kept at another time than its record's"
        );
    }
}
