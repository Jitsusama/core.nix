//! The spot's tests, over a machine's files laid out in a folder: home
//! by its node, away by its network, and each screen and dock plugged in
//! telling one spot from another.

use std::fs;
use std::time::Duration;

use super::*;

/// An empty machine, named for the test, whose files the test writes.
fn machine(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("spot-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("the machine's folder is made");
    root
}

/// Writes `said` to `path` under `root`, its folders made.
fn wrote(root: &Path, path: &str, said: &[u8]) {
    let file = root.join(path);
    fs::create_dir_all(file.parent().expect("a file has a folder")).expect("its folder is made");
    fs::write(file, said).expect("the file is written");
}

/// The eero daemon's file, with this machine on `node`.
fn at_home(root: &Path, node: &str) {
    let network = serde_json::json!({
        "nodes": [
            {"id": "6366035", "location": "Basement"},
            {"id": "6366013", "location": "Main Floor"},
        ],
        "devices": [
            {"mac": "02:00:00:00:00:02", "this_machine": false, "node": "6366035"},
            {"mac": "02:00:00:00:00:01", "this_machine": true, "node": node},
        ],
    });
    wrote(
        root,
        "run/user/eero/home-network.json",
        network.to_string().as_bytes(),
    );
}

/// A routable link joined by `network`'s file.
fn joined(root: &Path, link: u32, network: &str) {
    let state = format!(
        "ADMIN_STATE=configured\nOPER_STATE=routable\nNETWORK_FILE=/run/systemd/network/{network}.network\n"
    );
    wrote(
        root,
        &format!("run/systemd/netif/links/{link}"),
        state.as_bytes(),
    );
}

/// An EDID whose bytes 8 to 17 end in `serial`, naming itself `name`.
fn edid(serial: u8, name: &str) -> Vec<u8> {
    let mut edid = vec![0u8; 128];
    edid[..8].copy_from_slice(&[0, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0]);
    edid[8..18].copy_from_slice(&[0x10, 0xac, 0x41, 0xa1, 0, 0, 0, serial, 0x20, 0x22]);
    edid[54..59].copy_from_slice(&[0, 0, 0, 0xfc, 0]);
    let mut text = name.as_bytes().to_vec();
    text.push(b'\n');
    text.resize(13, b' ');
    edid[59..72].copy_from_slice(&text);
    edid
}

/// A screen on `connector`, plugged in or not.
fn screen(root: &Path, connector: &str, status: &str, edid: &[u8]) {
    wrote(
        root,
        &format!("sys/class/drm/{connector}/status"),
        status.as_bytes(),
    );
    wrote(root, &format!("sys/class/drm/{connector}/edid"), edid);
}

#[test]
fn home_is_told_by_the_node_its_on() {
    let root = machine("node");
    at_home(&root, "6366013");
    let main_floor = Where::at(&root).spot();
    assert_eq!(main_floor.key, "home=6366013");
    assert_eq!(main_floor.name, "Main Floor");
    at_home(&root, "6366035");
    assert_eq!(
        Where::at(&root).spot().name,
        "Basement",
        "moving to another node is another spot"
    );
}

#[test]
fn an_old_read_of_the_home_network_says_nothing() {
    let root = machine("stale");
    at_home(&root, "6366013");
    joined(&root, 3, "50-wifi-Roadside Inn");
    let file = fs::File::options()
        .write(true)
        .open(root.join("run/user/eero/home-network.json"))
        .expect("the eero file opens");
    file.set_modified(SystemTime::now() - FRESH - Duration::from_secs(60))
        .expect("its time is set back");
    let spot = Where::at(&root).spot();
    assert_eq!(
        spot.key, "network=50-wifi-Roadside%20Inn",
        "a space in a Wi-Fi's name would split its line of the picks' file"
    );
    assert_eq!(spot.name, "away, on Roadside Inn");
}

#[test]
fn away_is_told_by_the_network_joined() {
    let root = machine("away");
    joined(&root, 3, "50-wifi-Lantern");
    wrote(&root, "run/systemd/netif/links/1", b"OPER_STATE=carrier\n");
    wrote(
        &root,
        "run/systemd/netif/links/18",
        b"OPER_STATE=routable\n",
    );
    let spot = Where::at(&root).spot();
    assert_eq!(spot.key, "network=50-wifi-Lantern");
    assert_eq!(spot.name, "away, on Lantern");
    assert_eq!(
        Where::at(&machine("nothing")).spot(),
        Spot {
            key: "network=none".to_owned(),
            name: "away".to_owned(),
        },
        "a machine with nothing known is somewhere still"
    );
}

#[test]
fn each_screen_plugged_in_is_another_spot() {
    let root = machine("screens");
    at_home(&root, "6366013");
    screen(&root, "card0-eDP-1", "connected", &edid(1, "Built In"));
    screen(&root, "card0-DP-2", "disconnected", &edid(2, "U2723QE"));
    assert_eq!(
        Where::at(&root).spot().key,
        "home=6366013",
        "the laptop's own screen and one unplugged say nothing"
    );
    screen(&root, "card0-DP-2", "connected", &edid(2, "U2723QE"));
    let desk = Where::at(&root).spot();
    assert_eq!(desk.key, "home=6366013+screen=10ac41a1000000022022");
    assert_eq!(desk.name, "Main Floor, with the U2723QE");
    screen(&root, "card0-DP-2", "connected", &edid(3, "U2723QE"));
    assert_ne!(
        Where::at(&root).spot().key,
        desk.key,
        "another screen of the same model is another desk"
    );
}

#[test]
fn a_dock_is_another_spot() {
    let root = machine("dock");
    at_home(&root, "6366013");
    wrote(&root, "sys/bus/thunderbolt/devices/0-0/vendor", b"0x8087\n");
    wrote(&root, "sys/bus/thunderbolt/devices/0-0/device", b"0x1\n");
    wrote(&root, "sys/bus/thunderbolt/devices/domain0/uevent", b"");
    assert_eq!(
        Where::at(&root).spot().key,
        "home=6366013",
        "the machine's own controller is no dock"
    );
    wrote(&root, "sys/bus/thunderbolt/devices/0-1/vendor", b"0xd4\n");
    wrote(&root, "sys/bus/thunderbolt/devices/0-1/device", b"0xb070\n");
    let docked = Where::at(&root).spot();
    assert_eq!(docked.key, "home=6366013+dock=0xd4.0xb070");
    assert_eq!(docked.name, "Main Floor, docked");
}

#[test]
fn a_screen_naming_nothing_is_a_screen() {
    let mut unnamed = edid(4, "");
    unnamed[54..72].fill(0);
    assert_eq!(
        edid_said(&unnamed).map(|(_, name)| name),
        Some("screen".to_owned())
    );
    assert_eq!(edid_said(&[0; 16]), None, "half an EDID is none");
}
