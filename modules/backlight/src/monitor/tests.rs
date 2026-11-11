use std::fs;
use std::os::unix::fs::symlink;

use super::buses::{IN_CREATE, IN_DELETE, IN_Q_OVERFLOW};
use super::ddc::{
    ALWAYS_ON, HOT_PLUG, capabilities_request, get_vcp, hot_plug_to_set, identity, reading, set_vcp,
};
use super::*;

#[test]
fn asking_for_a_vcp_code_is_framed_and_summed() {
    assert_eq!(get_vcp(0x10), [0x51, 0x82, 0x01, 0x10, 0xAC]);
}

#[test]
fn setting_a_vcp_code_is_framed_and_summed() {
    let sent = set_vcp(0x10, 0x0140);
    assert_eq!(sent[..6], [0x51, 0x84, 0x03, 0x10, 0x01, 0x40]);
    let sum = [0x6E, 0x51, 0x84, 0x03, 0x10, 0x01, 0x40]
        .iter()
        .fold(0, |sum, byte| sum ^ byte);
    assert_eq!(sent[6], sum);
    assert_eq!(sent.len(), 7);
}

/// A monitor's answer, summed as one sends it.
fn answer(result: u8, vcp: u8, max: u16, now: u16) -> Vec<u8> {
    let [max_high, max_low] = max.to_be_bytes();
    let [now_high, now_low] = now.to_be_bytes();
    let mut reply = vec![
        0x6E, 0x88, 0x02, result, vcp, 0x00, max_high, max_low, now_high, now_low,
    ];
    reply.push(reply.iter().fold(0x50, |sum, byte| sum ^ byte));
    reply
}

#[test]
fn a_good_answer_gives_the_brightest_and_the_brightness_now() {
    assert_eq!(
        reading(&answer(0, 0x10, 100, 40), 0x10).unwrap(),
        Reading { max: 100, now: 40 }
    );
}

#[test]
fn an_answer_with_a_wrong_checksum_is_refused() {
    let mut reply = answer(0, 0x10, 100, 40);
    reply[10] ^= 1;
    assert!(
        reading(&reply, 0x10).is_err(),
        "an answer whose checksum is off by a bit was taken"
    );
}

#[test]
fn an_answer_for_another_vcp_code_is_refused() {
    assert!(
        reading(&answer(0, 0x12, 100, 40), 0x10).is_err(),
        "an answer for the contrast was taken as the brightness"
    );
}

#[test]
fn an_answer_that_says_no_such_code_is_refused() {
    assert!(
        reading(&answer(1, 0x10, 100, 40), 0x10).is_err(),
        "an answer saying the monitor has no such code was taken"
    );
}

#[test]
fn a_short_answer_is_refused() {
    assert!(
        reading(&[0x6E, 0x80, 0xBE], 0x10).is_err(),
        "a three-byte answer was taken"
    );
}

#[test]
fn the_panel_s_share_becomes_the_monitor_s_nearest_value() {
    assert_eq!(value_for(0.0, 100), 0);
    assert_eq!(value_for(1.0, 100), 100);
    assert_eq!(value_for(1.0 / 3.0, 100), 33);
    assert_eq!(value_for(2.0 / 3.0, 100), 67);
    assert_eq!(value_for(0.5, 101), 51);
    assert_eq!(value_for(2.0, 100), 100);
}

#[test]
fn a_z27k_g3_has_its_hot_plug_detection_held_and_other_monitors_do_not() {
    let z27k_g3 = [0x22, 0x0E, 0xD3, 0x36, 0, 0, 0, 0, 0x1C, 0x1F];
    let another_hp = [0x22, 0x0E, 0xD4, 0x36, 0, 0, 0, 0, 0x1C, 0x1F];
    let another_maker = [0x10, 0xAC, 0xD3, 0x36, 0, 0, 0, 0, 0x1C, 0x1F];
    assert!(
        holds_hot_plug(&z27k_g3),
        "a Z27k G3's hot-plug detection was left alone"
    );
    assert!(
        !holds_hot_plug(&another_hp),
        "another HP model's hot-plug detection was held"
    );
    assert!(
        !holds_hot_plug(&another_maker),
        "another maker's monitor with the same product code had its hot-plug detection held"
    );
}

#[test]
fn hot_plug_detection_at_low_power_is_set_to_always_on() {
    assert_eq!(hot_plug_to_set(0x0000), Some(0x0001));
    assert_eq!(hot_plug_to_set(0x0100), Some(0x0101));
}

#[test]
fn hot_plug_detection_always_on_is_left_alone() {
    assert_eq!(hot_plug_to_set(0x0001), None);
    assert_eq!(hot_plug_to_set(0x0101), None);
}

#[test]
fn what_a_z27k_g3_says_for_its_hot_plug_detection_is_read() {
    assert_eq!(
        reading(&answer(0, HOT_PLUG, 1, 0), HOT_PLUG).unwrap(),
        Reading { max: 1, now: 0 }
    );
}

#[test]
fn setting_hot_plug_detection_to_always_on_is_framed_as_the_menu_sets_it() {
    assert_eq!(
        set_vcp(HOT_PLUG, ALWAYS_ON)[..6],
        [0x51, 0x84, 0x03, 0xE7, 0x00, 0x01]
    );
}

#[test]
fn only_the_latest_level_asked_for_is_taken() {
    let mut wanted = Wanted::default();
    assert_eq!(wanted.take(), None);
    for level in [0.1, 0.2, 0.3] {
        wanted.want(level);
    }
    assert_eq!(wanted.take(), Some(0.3));
    assert_eq!(wanted.take(), None);
}

#[test]
fn a_gone_monitor_s_thread_is_asked_for_nothing_more() {
    let shared = Shared {
        wanted: Mutex::new(Wanted {
            level: Some(0.5),
            identity: None,
            gone: true,
        }),
        woken: Condvar::new(),
    };
    assert_eq!(shared.next(true), Next::Gone);
}

#[test]
fn a_level_waiting_comes_before_other_work_and_other_work_before_waiting() {
    let shared = Shared {
        wanted: Mutex::new(Wanted {
            level: Some(0.5),
            identity: None,
            gone: false,
        }),
        woken: Condvar::new(),
    };
    assert_eq!(shared.next(true), Next::Level(0.5));
    assert_eq!(shared.next(true), Next::Idle);
}

#[test]
fn a_thread_learns_its_monitor_once_after_a_level_and_before_other_work() {
    let identity = [0x22, 0x0E, 0xD3, 0x36, 1, 0, 0, 0, 0x1C, 0x1F];
    let mut monitor = Monitor {
        shared: Arc::new(Shared {
            wanted: Mutex::new(Wanted::default()),
            woken: Condvar::new(),
        }),
        identified: false,
    };
    monitor.identify(None);
    monitor.want(0.5);
    monitor.identify(Some(identity));
    let shared = Arc::clone(&monitor.shared);
    assert_eq!(shared.next(true), Next::Level(0.5));
    assert_eq!(shared.next(true), Next::Identified(identity));
    assert_eq!(shared.next(true), Next::Idle);
    // One that knows its monitor isn't told again.
    monitor.identify(Some(identity));
    assert_eq!(shared.next(true), Next::Idle);
}

#[test]
fn asking_for_capabilities_is_framed_and_summed() {
    let sent = capabilities_request(0x0020);
    assert_eq!(sent[..5], [0x51, 0x83, 0xF3, 0x00, 0x20]);
    let sum = [0x6E, 0x51, 0x83, 0xF3, 0x00, 0x20]
        .iter()
        .fold(0, |sum, byte| sum ^ byte);
    assert_eq!(sent[5], sum);
}

#[test]
fn a_firmware_level_with_no_brightest_is_still_read() {
    assert_eq!(
        reading(&answer(0, FIRMWARE, 0, 0x0100), FIRMWARE).unwrap(),
        Reading {
            max: 0,
            now: 0x0100
        }
    );
    assert!(
        reading(&answer(0, BRIGHTNESS, 0, 0), BRIGHTNESS).is_err(),
        "a brightness whose brightest is 0 was taken"
    );
}

#[test]
fn a_monitor_s_brightness_and_hot_plug_detection_are_named() {
    assert_eq!(share(40, 100), "40%");
    assert_eq!(share(1, 3), "33%");
    assert_eq!(share(0, 0), "0%");
    assert_eq!(hot_plug_named(0x0001), "always on");
    assert_eq!(hot_plug_named(0x0100), "low power");
}

/// A stand-in sysfs and /dev, gone when the test is.
struct Tree(PathBuf);

impl Tree {
    fn new(test: &str) -> Self {
        let root = std::env::temp_dir().join(format!("backlight-{}-{test}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        for dir in ["class", "dev", "devices"] {
            fs::create_dir_all(root.join(dir)).unwrap();
        }
        Tree(root)
    }

    /// A bus of this name, whose device sits at `device` under devices.
    fn add(&self, bus: &str, name: &str, device: &str) {
        let device = self.0.join("devices").join(device);
        fs::create_dir_all(&device).unwrap();
        let dir = self.0.join("class").join(bus);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("name"), format!("{name}\n")).unwrap();
        symlink(&device, dir.join("device")).unwrap();
        fs::write(self.0.join("dev").join(bus), "").unwrap();
    }

    /// What a connector under devices says is on it, as DRM's status.
    fn status(&self, connector: &str, status: &str) {
        let dir = self.0.join("devices").join(connector);
        fs::write(dir.join("status"), format!("{status}\n")).unwrap();
    }

    fn buses(&self) -> Vec<(String, Reach)> {
        buses(&self.0.join("class"), &self.0.join("dev")).unwrap()
    }

    fn names(&self) -> Vec<String> {
        self.buses().into_iter().map(|(name, _)| name).collect()
    }
}

impl Drop for Tree {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn a_bus_behind_an_mst_hub_reaches_a_monitor() {
    let tree = Tree::new("mst");
    tree.add("i2c-13", "DPMST", "i2c-13");
    assert_eq!(tree.buses(), [("i2c-13".into(), Reach::ThroughAHub)]);
}

#[test]
fn a_displayport_connector_s_bus_reaches_a_monitor_and_the_panel_s_does_not() {
    let tree = Tree::new("connectors");
    tree.add(
        "i2c-7",
        "AUX USBC1/DDI TC1/PHY TC1",
        "card1/card1-DP-2/i2c-7",
    );
    tree.add("i2c-3", "AUX A/DDI A/PHY A", "card1/card1-eDP-1/i2c-3");
    tree.add("i2c-5", "i915 gmbus dpb", "card1/card1-HDMI-A-1/i2c-5");
    assert_eq!(tree.buses(), [("i2c-7".into(), Reach::ByItsConnector)]);
}

#[test]
fn a_connector_with_nothing_on_it_is_passed_over() {
    let tree = Tree::new("disconnected");
    tree.add(
        "i2c-10",
        "AUX USBC1/DDI TC1/PHY TC1",
        "card0/card0-DP-1/i2c-10",
    );
    tree.add(
        "i2c-11",
        "AUX USBC2/DDI TC2/PHY TC2",
        "card0/card0-DP-2/i2c-11",
    );
    tree.status("card0/card0-DP-1", "disconnected");
    tree.status("card0/card0-DP-2", "connected");
    assert_eq!(tree.buses(), [("i2c-11".into(), Reach::ByItsConnector)]);
}

#[test]
fn the_machine_s_other_buses_are_left_alone() {
    let tree = Tree::new("others");
    tree.add(
        "i2c-0",
        "SMBus I801 adapter at efa0",
        "pci0000:00/0000:00:1f.4/i2c-0",
    );
    tree.add(
        "i2c-1",
        "Synopsys DesignWare I2C adapter",
        "pci0000:00/i2c-1",
    );
    assert_eq!(tree.buses(), [] as [(String, Reach); 0]);
}

#[test]
fn a_bus_whose_node_has_gone_is_gone() {
    let tree = Tree::new("gone");
    tree.add("i2c-13", "DPMST", "i2c-13");
    tree.add("i2c-14", "DPMST", "i2c-14");
    fs::remove_file(tree.0.join("dev/i2c-14")).unwrap();
    assert_eq!(tree.names(), ["i2c-13"]);
}

/// An EDID's first block for a monitor, summed as a monitor sums it.
fn edid(serial: u8) -> Vec<u8> {
    let mut block = vec![0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00];
    // HP's maker code, a model, the serial, the week and year made.
    block.extend_from_slice(&[0x22, 0xF0, 0x4E, 0x37, serial, 0, 0, 0, 12, 34]);
    block.resize(127, 0);
    block.push(0u8.wrapping_sub(block.iter().fold(0u8, |sum, byte| sum.wrapping_add(*byte))));
    block
}

#[test]
fn an_edid_names_its_monitor_by_maker_model_serial_and_making() {
    assert_eq!(
        identity(&edid(7)),
        Some([0x22, 0xF0, 0x4E, 0x37, 7, 0, 0, 0, 12, 34])
    );
}

#[test]
fn what_isn_t_a_whole_edid_names_no_monitor() {
    let mut summed_wrong = edid(7);
    summed_wrong[127] ^= 1;
    let mut no_header = edid(7);
    no_header[0] = 0xFF;
    assert_eq!(identity(&summed_wrong), None);
    assert_eq!(identity(&no_header), None);
    assert_eq!(identity(&edid(7)[..100]), None);
    assert_eq!(identity(&[0; 128]), None);
}

fn seen(name: &str, reach: Reach, serial: Option<u8>) -> Seen<'_> {
    Seen {
        name,
        reach,
        identity: serial.and_then(|serial| identity(&edid(serial))),
    }
}

#[test]
fn a_connector_reaching_a_hub_s_monitor_is_left_to_the_hub_s_bus() {
    let found = [
        seen("i2c-10", Reach::ByItsConnector, Some(1)),
        seen("i2c-13", Reach::ThroughAHub, Some(1)),
    ];
    assert_eq!(doubled(&found), HashMap::from([("i2c-10", "i2c-13")]));
}

#[test]
fn monitors_behind_a_hub_are_each_followed_though_their_edids_match() {
    let found = [
        seen("i2c-10", Reach::ByItsConnector, Some(1)),
        seen("i2c-13", Reach::ThroughAHub, Some(1)),
        seen("i2c-14", Reach::ThroughAHub, Some(1)),
    ];
    assert_eq!(doubled(&found).len(), 1);
    assert!(
        doubled(&found[1..]).is_empty(),
        "one of two hub buses whose EDIDs match was left to the other"
    );
}

#[test]
fn a_connector_is_followed_unless_a_hub_s_bus_is_known_to_reach_its_monitor() {
    assert!(
        doubled(&[
            seen("i2c-7", Reach::ByItsConnector, Some(1)),
            seen("i2c-8", Reach::ByItsConnector, Some(1)),
            seen("i2c-13", Reach::ThroughAHub, Some(2)),
            seen("i2c-10", Reach::ByItsConnector, None),
            seen("i2c-14", Reach::ThroughAHub, None),
        ])
        .is_empty(),
        "a connector was left to a bus not known to reach its monitor"
    );
}

#[test]
fn no_class_of_buses_is_none_found() {
    let tree = Tree::new("no-class");
    assert_eq!(
        buses(&tree.0.join("missing"), &tree.0.join("dev")).unwrap(),
        [] as [(String, Reach); 0]
    );
}

/// An inotify event for a name, padded as the kernel pads it.
fn event(mask: u32, name: &str) -> Vec<u8> {
    let mut padded = name.as_bytes().to_vec();
    if !name.is_empty() {
        padded.resize((name.len() / 16 + 1) * 16, 0);
    }
    let mut event = Vec::new();
    event.extend_from_slice(&1i32.to_ne_bytes());
    event.extend_from_slice(&mask.to_ne_bytes());
    event.extend_from_slice(&0u32.to_ne_bytes());
    event.extend_from_slice(&u32::try_from(padded.len()).unwrap().to_ne_bytes());
    event.extend_from_slice(&padded);
    event
}

#[test]
fn a_bus_coming_is_heard_among_other_nodes() {
    let mut events = event(IN_CREATE, "tty7");
    events.extend(event(IN_CREATE, "i2c-13"));
    assert!(
        names_a_bus(&events),
        "an I2C bus created among other nodes went unheard"
    );
}

#[test]
fn other_nodes_coming_and_going_are_not_a_bus() {
    let mut events = event(IN_CREATE, "hidraw3");
    events.extend(event(IN_DELETE, "input/event9"));
    assert!(
        !names_a_bus(&events),
        "nodes that are no I2C bus were taken for one"
    );
}

#[test]
fn lost_events_might_have_named_a_bus() {
    assert!(
        names_a_bus(&event(IN_Q_OVERFLOW, "")),
        "lost events were taken to name no bus"
    );
}

/// A kernel uevent as it comes off the netlink socket: its action and
/// path, then each of its fields, each ended by a NUL.
fn uevent(fields: &[&str]) -> Vec<u8> {
    fields
        .iter()
        .flat_map(|field| [field.as_bytes(), b"\0"].concat())
        .collect()
}

#[test]
fn a_connector_s_cable_coming_or_going_is_heard() {
    let hotplug = uevent(&[
        "change@/devices/pci0000:00/0000:00:02.0/drm/card0",
        "ACTION=change",
        "DEVPATH=/devices/pci0000:00/0000:00:02.0/drm/card0",
        "SUBSYSTEM=drm",
        "HOTPLUG=1",
        "CONNECTOR=287",
        "DEVNAME=dri/card0",
        "SEQNUM=5123",
    ]);
    assert!(
        says_a_cable_moved(&hotplug),
        "a DRM connector's hot-plug uevent went unheard"
    );
}

#[test]
fn other_uevents_are_not_a_cable() {
    let lease = uevent(&[
        "change@/devices/pci0000:00/0000:00:02.0/drm/card0",
        "ACTION=change",
        "SUBSYSTEM=drm",
        "LEASE=1",
    ]);
    let usb = uevent(&[
        "add@/devices/pci0000:00/0000:00:14.0/usb3/3-1",
        "ACTION=add",
        "SUBSYSTEM=usb",
        "HOTPLUG=1",
    ]);
    assert!(
        !says_a_cable_moved(&lease),
        "a DRM lease uevent was taken for a cable"
    );
    assert!(
        !says_a_cable_moved(&usb),
        "a USB device's hot-plug uevent was taken for a cable"
    );
    assert!(
        !says_a_cable_moved(b""),
        "an empty uevent was taken for a cable"
    );
}
