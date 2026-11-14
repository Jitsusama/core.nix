//! Every view's page, held to a golden one drawn from `home-network.json`
//! a minute after it was read, in a zone five hours behind UTC, so a page
//! that changes is seen line by line. `EERO_GOLDEN=write cargo test`
//! writes the pages afresh, to be read before they're committed.

use std::path::Path;

use jiff::tz::{self, TimeZone};
use serde_json::Value;
use terminal_kit::Room;

use super::text::{Ink, Palette};
use super::{Clock, View, json, text};
use crate::model::{CloudState, HomeNetwork};

const MODEL: &str = include_str!("../../home-network.json");

fn model() -> HomeNetwork {
    serde_json::from_str(MODEL).expect("home-network.json reads")
}

fn clock(now: &str) -> Clock {
    Clock {
        now: now.parse().unwrap(),
        zone: TimeZone::fixed(tz::offset(-5)),
    }
}

/// A minute and a few seconds after the record was read.
fn soon() -> Clock {
    clock("2026-11-07T22:55:50Z")
}

/// Compares a page with its golden file, or writes the file when asked.
fn holds(name: &str, page: &str) {
    let file = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/views/golden")
        .join(name);
    if std::env::var("EERO_GOLDEN").as_deref() == Ok("write") {
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(&file, page).unwrap();
        return;
    }
    let golden = std::fs::read_to_string(&file).unwrap_or_else(|_| {
        panic!(
            "no golden page {}; write it with EERO_GOLDEN=write",
            file.display()
        )
    });
    if golden != page {
        let diff: Vec<String> = golden
            .lines()
            .zip(page.lines())
            .enumerate()
            .filter(|(_, (want, got))| want != got)
            .map(|(at, (want, got))| format!("line {}:\n  want {want}\n  got  {got}", at + 1))
            .collect();
        panic!(
            "{name} differs from its golden page ({} lines against {}):\n{}\n\nthe page:\n{page}",
            golden.lines().count(),
            page.lines().count(),
            diff.join("\n")
        );
    }
}

fn page(view: &View, model: &HomeNetwork) -> String {
    text(view, model, &soon(), &Ink::plain(), Room::Unbounded).unwrap()
}

/// The widths a terminal is opened at, from a split beside an editor to a
/// whole wide screen, as docs/commands.md sets every view at.
const WIDTHS: [usize; 4] = [60, 80, 100, 140];

/// Narrower than any of them, as a phone's terminal or a third of a
/// screen is, where a page still runs no line past the edge, though it's
/// held to no golden page there.
const NARROW: [usize; 2] = [40, 50];

/// A page set in a terminal each of the widths wide runs no line past its
/// edge, and is held to its golden page at that width, in a folder named
/// for it, where it's set otherwise than into a pipe; where it isn't,
/// there's no page there, so a page that fits as it is reads as one.
fn fits(name: &str, view: &View, model: &HomeNetwork) {
    let piped = page(view, model);
    let within = |width: usize| {
        let set = text(view, model, &soon(), &Ink::plain(), Room::Cells(width)).unwrap();
        for line in set.lines() {
            assert!(
                terminal_kit::layout::width(line) <= width,
                "{name} at {width} runs past the edge: {line}"
            );
        }
        set
    };
    for width in NARROW {
        within(width);
    }
    for width in WIDTHS {
        let set = within(width);
        let file = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src/views/golden")
            .join(width.to_string())
            .join(name);
        if set == piped {
            if std::env::var("EERO_GOLDEN").as_deref() == Ok("write") {
                _ = std::fs::remove_file(&file);
            }
            assert!(
                !file.exists(),
                "{name} at {width} is set as into a pipe, so {} should go",
                file.display()
            );
        } else {
            holds(&format!("{width}/{name}"), &set);
        }
    }
}

/// A view's JSON, read back.
fn slice(view: &View) -> Value {
    serde_json::from_str(&json(view, &model()).unwrap()).unwrap()
}

/// Each view's page from the record, and from the troubled one where it
/// marks what's wrong.
macro_rules! golden {
    ($($test:ident: $model:ident, $view:expr => $file:literal;)*) => {
        $(
            #[test]
            fn $test() {
                holds($file, &page(&$view, &$model()));
                fits($file, &$view, &$model());
            }
        )*
    };
}

golden! {
    status_holds_every_headline_figure: model, View::Status => "status.txt";
    status_marks_what_is_wrong: troubled, View::Status => "status-troubled.txt";
    devices_sit_under_their_nodes: model, View::Devices => "devices.txt";
    a_device_holds_every_figure: model, View::Device("unknown computer 1".into()) => "device.txt";
    a_device_eero_says_less_of_shows_a_dash:
        model, View::Device("02:00:00:00:00:01".into()) => "device-tablet.txt";
    a_device_over_ethernet_shows_its_port:
        troubled, View::Device("printer 1".into()) => "device-ethernet.txt";
    speeds_lists_every_test_with_a_spark: model, View::Speeds => "speeds.txt";
    nodes_lists_every_node: model, View::Nodes => "nodes.txt";
    nodes_marks_one_offline: troubled, View::Nodes => "nodes-troubled.txt";
    a_node_holds_its_radios_ports_and_devices:
        model, View::Node("Study".into()) => "node-study.txt";
    the_gateway_holds_its_linked_ports: model, View::Node("2003".into()) => "node-hall.txt";
    a_port_under_its_best_and_a_busy_radio_are_marked:
        troubled, View::Node("Hall".into()) => "node-troubled.txt";
    the_mesh_is_a_tree_from_the_gateway: model, View::Mesh => "mesh.txt";
    radio_sets_every_nodes_radios_side_by_side: model, View::Radio => "radio.txt";
    internet_holds_every_setting_in_sections: model, View::Internet => "internet.txt";
    internet_marks_the_way_out_down: troubled, View::Internet => "internet-troubled.txt";
    firmware_lists_each_nodes_own: model, View::Firmware => "firmware.txt";
    firmware_marks_an_update_waiting: troubled, View::Firmware => "firmware-troubled.txt";
    traffic_sets_every_hour_by_node: model, View::Traffic => "traffic.txt";
    profiles_counts_each_ones_devices: model, View::Profiles => "profiles.txt";
    a_profile_lists_its_devices_on_now_or_not:
        model, View::Profile("profile 1".into()) => "profile.txt";
    a_profile_sets_a_device_on_now_by_its_node:
        profiled, View::Profile("profile 3".into()) => "profile-on.txt";
    a_profile_names_each_filter_it_has_on:
        filtered, View::Profile("profile 2".into()) => "profile-filtered.txt";
    profiles_give_a_filter_its_column_while_one_has_it:
        filtered, View::Profiles => "profiles-filtered.txt";
    events_and_what_was_told_run_as_one_timeline: model, View::Events => "events.txt";
    a_nodes_devices_are_those_on_it_alone:
        model, View::NodeDevices("Study".into()) => "node-study-devices.txt";
    a_nodes_radios_are_its_own_alone:
        model, View::NodeRadios("Study".into()) => "node-study-radios.txt";
    a_profiles_devices_are_those_in_it_on_now_or_not:
        profiled, View::ProfileDevices("profile 3".into()) => "profile-devices.txt";
    sqm_says_whether_its_on_and_how_to_change_it: model, View::Sqm => "sqm.txt";
}

#[test]
fn what_a_node_or_profile_holds_keeps_the_wholes_schema() {
    // A program reading eero.devices/1 reads a node's or a profile's
    // devices the same way, with only theirs in it.
    let study = slice(&View::NodeDevices("Study".into()));
    assert_eq!(study["schema"], "eero.devices/1", "{study}");
    let whole = slice(&View::Devices);
    let on_study = |devices: &Value| {
        devices
            .as_array()
            .unwrap()
            .iter()
            .filter(|device| device["node"] == "2002")
            .count()
    };
    assert_eq!(
        study["devices"].as_array().unwrap().len(),
        on_study(&whole["devices"]),
        "the study's devices aren't those the whole has on it: {study}"
    );
    let radios = slice(&View::NodeRadios("Study".into()));
    assert_eq!(radios["schema"], "eero.radio/1", "{radios}");
    assert_eq!(radios["nodes"].as_array().unwrap().len(), 1, "{radios}");
    let profile = slice(&View::ProfileDevices("profile 1".into()));
    assert_eq!(profile["schema"], "eero.devices/1", "{profile}");
    // Its ten members are none on now nor off, so a program finds them
    // under unseen, by address and the profile's name for each.
    let unseen = profile["unseen"].as_array().unwrap();
    assert_eq!(unseen.len(), 10, "the profile's unseen members: {profile}");
    assert_eq!(unseen[0]["mac"], "02:00:00:00:00:2b", "{profile}");
    assert_eq!(unseen[0]["name"], "phone 3", "{profile}");
    assert!(
        slice(&View::Devices).get("unseen").is_none(),
        "every device's eero.devices/1 gained unseen"
    );
}

/// The record with two devices on now in profile 3, since none of the
/// record's own profiles holds one.
fn profiled() -> HomeNetwork {
    let mut model = model();
    let on: Vec<String> = model.devices[..2].iter().map(|d| d.mac.clone()).collect();
    model.profiles[2].devices.extend(on);
    model
}

/// The record with profile 2 the default, kept to two schedules, and with
/// filters on of every sort a profile's page says, since none of the
/// record's own profiles has one on.
fn filtered() -> HomeNetwork {
    let mut model = model();
    let profile = &mut model.profiles[1];
    profile.default = Some(true);
    profile.schedules = Some(2);
    let filters = &mut profile.filters;
    filters.gaming = Some(true);
    filters.social = Some(true);
    filters.ad_block = Some(true);
    filters.blocked_sites = vec!["example.com".into()];
    filters.blocked_apps = vec!["tiktok".into()];
    filters.safe_search = Some(true);
    filters.allowed_sites = vec!["example.org".into(), "example.net".into()];
    model
}

/// The views a terminal's colours are tried on.
fn coloured() -> Vec<View> {
    vec![
        View::Status,
        View::Devices,
        View::Speeds,
        View::Node("Hall".into()),
        View::Internet,
        View::Firmware,
        View::Traffic,
    ]
}

/// The record with something wrong in each place a view marks it: a node
/// offline with its heartbeat missed, an update waiting, a port linked
/// under its best, a busy radio, the internet down, and a device over
/// Ethernet whose address is reserved, which the record's one reservation
/// isn't, with the guest subnet on but kept from the internet and open,
/// and the bands' security differing.
fn troubled() -> HomeNetwork {
    let mut model = model();
    let net = &mut model.internet;
    net.state = Some("disconnected".into());
    net.isp_up = Some(false);
    net.nodes.online = Some(2);
    net.nodes.offline = Some(1);
    net.firmware.has_update = Some(true);
    net.firmware.next = Some("v7.18.0-1".into());
    let den = &mut model.nodes[2];
    den.state = Some("OFFLINE".into());
    den.heartbeat_ok = Some(false);
    den.update_available = Some(true);
    let hall = &mut model.nodes[0];
    hall.ports[1].mbps = Some(100);
    hall.radios[2].busy_pct = Some(97);
    let printer = &mut model.devices[2];
    printer.ethernet = true;
    printer.port = Some(crate::model::DevicePort {
        name: Some("2".into()),
        mbps: Some(100),
    });
    let mac = printer.mac.clone();
    let addresses = &mut model.internet.addresses;
    addresses.reservations.push(crate::model::Reservation {
        mac,
        description: Some("office printer".into()),
        address: Some("10.0.0.30".into()),
    });
    let guest = &mut addresses.subnets[1];
    guest.on = Some(true);
    guest.internet = Some(false);
    guest.open = Some(true);
    model.internet.security[1].security = "wpa3".into();
    model
}

#[test]
fn an_unreachable_cloud_is_said_with_how_old_the_reading_is() {
    let mut model = model();
    model.cloud.state = CloudState::Unreachable;
    model.cloud.since = "2026-11-07T22:56:00Z".into();
    model.cloud.reason = Some("timed out".into());
    let page = text(
        &View::Status,
        &model,
        &clock("2026-11-07T23:10:00Z"),
        &Ink::plain(),
        Room::Unbounded,
    )
    .unwrap();
    let head: Vec<&str> = page.lines().take(3).collect();
    assert_eq!(
        head,
        [
            "home · read 15 m 17 s ago",
            "✕ eero's cloud unreachable since 2026-11-07 17:56 (14 m ago): timed out",
            "",
        ]
    );
}

#[test]
fn a_reading_grown_old_says_how_old_and_how_to_start_the_daemon() {
    let page = text(
        &View::Status,
        &model(),
        &clock("2026-11-08T01:00:00Z"),
        &Ink::plain(),
        Room::Unbounded,
    )
    .unwrap();
    assert_eq!(
        page.lines().nth(1),
        Some(
            "● nothing new for 2 h 5 m; eero-daemon may have stopped: \
             systemctl --user start eero-daemon"
        )
    );
}

#[test]
fn signed_out_says_how_to_sign_in() {
    let mut model = model();
    model.cloud.state = CloudState::SignedOut;
    let page = page(&View::Status, &model);
    let said = page.lines().nth(1).unwrap();
    assert!(
        said.ends_with("sign in with eero sign-in <email or phone>"),
        "{page}"
    );
}

#[test]
fn an_unknown_node_names_the_nodes_there_are() {
    let refused = text(
        &View::Node("Attic".into()),
        &model(),
        &soon(),
        &Ink::plain(),
        Room::Unbounded,
    );
    assert_eq!(
        refused.unwrap_err(),
        "no node is called Attic; the nodes are Hall, Study and Den"
    );
}

#[test]
fn an_unknown_profile_names_the_profiles_there_are() {
    let refused = text(
        &View::Profile("kids".into()),
        &model(),
        &soon(),
        &Ink::plain(),
        Room::Unbounded,
    );
    assert_eq!(
        refused.unwrap_err(),
        "no profile is called kids; the profiles are profile 1, profile 2 and profile 3"
    );
}

#[test]
fn a_name_two_devices_share_is_refused_with_each_ones_address() {
    let refused = text(
        &View::Device("phone 1".into()),
        &model(),
        &soon(),
        &Ink::plain(),
        Room::Unbounded,
    );
    assert_eq!(
        refused.unwrap_err(),
        "2 devices are called phone 1: 02:00:00:00:00:03 on Study and 02:00:00:00:00:05 on Hall; \
         name one by its hardware address"
    );
}

#[test]
fn a_device_not_on_now_says_where_and_when_it_last_was() {
    let refused = text(
        &View::Device("phone 2".into()),
        &model(),
        &soon(),
        &Ink::plain(),
        Room::Unbounded,
    );
    assert_eq!(
        refused.unwrap_err(),
        "phone 2 isn't on now; it was last on Den, 2026-11-06 18:58, 22 h 56 m ago"
    );
}

#[test]
fn as_json_a_device_not_on_now_says_eeros_own_time() {
    let refused = json(&View::Device("phone 2".into()), &model());
    assert_eq!(
        refused.unwrap_err(),
        "phone 2 isn't on now; it was last on Den, 2026-11-06T23:58:58.114Z"
    );
}

#[test]
fn on_a_terminal_every_line_keeps_its_text_and_only_gains_colour() {
    let palette = Palette {
        text: None,
        strong: Some("1".into()),
        muted: Some("37".into()),
        track: Some("2".into()),
        alert: Some("31".into()),
        warning: Some("93".into()),
    };
    let ink = Ink {
        palette: Some(palette),
    };
    for view in coloured() {
        let coloured = text(&view, &model(), &soon(), &ink, Room::Unbounded).unwrap();
        let plain = page(&view, &model());
        // A graph's empty cells are drawn whole on a terminal, as its track.
        let stripped = strip(&coloured).replace('⣿', "⠀");
        assert_eq!(stripped, plain.replace('⣿', "⠀"), "{view:?}");
        assert!(
            coloured.contains("\x1b[1m"),
            "{view:?} draws nothing strong"
        );
    }
}

fn strip(text: &str) -> String {
    let mut out = String::new();
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            for c in chars.by_ref() {
                if c == 'm' {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

#[test]
fn a_slice_carries_when_it_was_read_and_whether_eero_answered() {
    let status = slice(&View::Status);
    assert_eq!(status["read"], "2026-11-07T22:54:43Z");
    assert_eq!(status["cloud"]["state"], "answering");
    assert_eq!(status["name"], "home");
    assert_eq!(status["internet"]["speed_test"]["down_mbps"], 369.86);
    assert_eq!(status["devices"].as_array().unwrap().len(), 15);
}

#[test]
fn each_slice_names_its_view_as_its_schema() {
    let views = [
        (View::Status, "status"),
        (View::Nodes, "nodes"),
        (View::Node("Study".into()), "node"),
        (View::Mesh, "mesh"),
        (View::Devices, "devices"),
        (View::Device("tablet 1".into()), "device"),
        (View::Internet, "internet"),
        (View::Speeds, "speeds"),
        (View::Firmware, "firmware"),
        (View::Radio, "radio"),
        (View::Traffic, "traffic"),
        (View::Profiles, "profiles"),
        (View::Profile("3002".into()), "profile"),
        (View::Events, "events"),
        (View::Sqm, "internet-sqm"),
    ];
    for (view, word) in views {
        assert_eq!(
            slice(&view)["schema"],
            format!("eero.{word}/1"),
            "{view:?} names another schema"
        );
    }
}

#[test]
fn the_devices_slice_holds_each_device_whole() {
    let model = model();
    let devices = slice(&View::Devices);
    assert_eq!(
        devices["devices"],
        serde_json::to_value(&model.devices).unwrap()
    );
    let device = slice(&View::Device("tablet 1".into()));
    assert_eq!(
        device["device"],
        serde_json::to_value(&model.devices[3]).unwrap()
    );
}

#[test]
fn the_speeds_slice_holds_every_test() {
    let net = model().internet;
    let speeds = slice(&View::Speeds);
    assert_eq!(
        speeds["speed_test"],
        serde_json::to_value(&net.speed_test).unwrap()
    );
    assert_eq!(
        speeds["speed_tests"],
        serde_json::to_value(&net.speed_tests).unwrap()
    );
}

#[test]
fn the_internet_firmware_and_traffic_slices_hold_their_part() {
    let model = model();
    assert_eq!(
        slice(&View::Internet)["internet"],
        serde_json::to_value(&model.internet).unwrap()
    );
    let firmware = slice(&View::Firmware);
    assert_eq!(
        firmware["firmware"],
        serde_json::to_value(&model.internet.firmware).unwrap()
    );
    assert_eq!(
        firmware["nodes"][1]["firmware"],
        serde_json::to_value(&model.nodes[1].firmware).unwrap()
    );
    let traffic = slice(&View::Traffic);
    assert_eq!(
        traffic["usage"],
        serde_json::to_value(&model.internet.usage).unwrap()
    );
    assert_eq!(
        traffic["nodes"][0]["usage"],
        serde_json::to_value(&model.nodes[0].usage).unwrap()
    );
}

#[test]
fn the_profile_and_event_slices_hold_them_whole() {
    let model = model();
    assert_eq!(
        slice(&View::Profiles)["profiles"],
        serde_json::to_value(&model.profiles).unwrap()
    );
    let profile = slice(&View::Profile("3002".into()));
    assert_eq!(
        profile["profile"],
        serde_json::to_value(&model.profiles[1]).unwrap()
    );
    assert!(profile["devices"].is_array());
    let events = slice(&View::Events);
    assert_eq!(
        events["events"],
        serde_json::to_value(&model.events).unwrap()
    );
    assert_eq!(events["told"], serde_json::to_value(&model.told).unwrap());
}

#[test]
fn the_node_slices_hold_each_node_whole_and_the_mesh_and_radios_their_part() {
    let model = model();
    assert_eq!(
        slice(&View::Nodes)["nodes"],
        serde_json::to_value(&model.nodes).unwrap()
    );
    let study = slice(&View::Node("Study".into()));
    assert_eq!(
        study["node"],
        serde_json::to_value(&model.nodes[1]).unwrap()
    );
    assert_eq!(study["devices"].as_array().unwrap().len(), 9);
    let mesh = slice(&View::Mesh);
    assert_eq!(
        mesh["nodes"][1]["backhaul"],
        serde_json::to_value(&model.nodes[1].backhaul).unwrap()
    );
    let radio = slice(&View::Radio);
    assert_eq!(
        radio["nodes"][2]["radios"],
        serde_json::to_value(&model.nodes[2].radios).unwrap()
    );
}
