//! Reviving a Mac from the stick, each step held against what its tools
//! print: the restore image found by its partition's name and mounted
//! only for reading, a network chosen and joined with its password on
//! standard input, the Mac found in DFU mode by irecovery, Apple asked to
//! sign before anything changes, and the revive an update that keeps the
//! Mac's disk, through the stick's own usbmuxd.

use guide::revive::{
    CheckSigning, ChooseNetwork, FindMac, FindRestore, JoinNetwork, Mac, RESTORE, Revive,
};
use guide::stand_in::StandIn;
use guide::{Answer, Answers, Ask, Fault, Machine, Program, Secret, Standing, Step, Tool};
use serde_json::json;

const LSBLK: &[&str] = &["--json", "--output-all", "--bytes"];
const DESCRIBE: &[&str] = &[
    "call",
    "/run/wifid/io.jitsusama.Wifi",
    "io.jitsusama.Wifi.Describe",
    "{}",
];
const JOIN: &[&str] = &[
    "call",
    "/run/wifid/io.jitsusama.Wifi",
    "io.jitsusama.Wifi.Join",
];
const FILE: &str = "/run/guide/restore/UniversalMac_27.2_26B123_Restore.ipsw";
const ECID: &str = "0x001a2b3c4d5e6f70";

/// What irecovery prints of an M2 laptop of Apple's in DFU mode, as its source
/// prints it (`tools/irecovery.c`, `print_device_info`).
const IN_DFU: &str = "CPID: 0x8112
CPRV: 0x11
BDID: 0x28
ECID: 0x001a2b3c4d5e6f70
CPFM: 0x03
SCEP: 0x01
IBFL: 0x3c
SRTG: [iBoot-10151.140.19]
SRNM: N/A
IMEI: N/A
NONC: N/A
SNON: N/A
PWND: N/A
MODE: DFU
PRODUCT: Mac14,2
MODEL: j413ap
NAME: MacBook Air (M2, 2022)
";

/// A stick as lsblk lists it: its boot partition, its system, and its
/// restore image, mounted where the guide reads it or not, when it
/// carries one.
fn stick(restore: Option<bool>) -> String {
    let mut partitions = vec![
        json!({"name": "sda1", "path": "/dev/sda1", "type": "part", "size": 268_435_456,
               "rm": true, "partn": 1, "partlabel": "esp", "fstype": "vfat"}),
        json!({"name": "sda3", "path": "/dev/sda3", "type": "part", "size": 2_362_232_012_u64,
               "rm": true, "partn": 3, "partlabel": "store", "fstype": "erofs"}),
    ];
    if let Some(mounted) = restore {
        partitions.push(json!({
            "name": "sda4", "path": "/dev/sda4", "type": "part", "size": 25_602_113_536_u64,
            "rm": true, "partn": 4, "partlabel": "UniversalMac_27.2_26B123", "fstype": "erofs",
            "mountpoints": [if mounted { Some(RESTORE) } else { None }],
        }));
    }
    json!({"blockdevices": [{
        "name": "sda", "path": "/dev/sda", "type": "disk", "size": 61_872_793_600_u64,
        "tran": "usb", "rm": true, "children": partitions,
    }]})
    .to_string()
}

/// What wifid describes: cards, saved networks and networks in reach.
fn wifi(online: bool, saved: &[&str]) -> String {
    json!({
        "cards": [{"name": "wlan0", "network": if online { Some("Cabin") } else { None },
                   "reach": if online { Some("online") } else { None }}],
        "saved": saved.iter().map(|name| json!({"name": name})).collect::<Vec<_>>(),
        "nearby": [
            {"name": "Cabin", "security": "personal", "strength": 4},
            {"name": "Office", "security": "enterprise", "strength": 3},
            {"name": "Café", "security": "open", "strength": 1},
        ],
        "radio": true,
        "wired": false,
        "looks": 2,
    })
    .to_string()
}

fn standing(step: &dyn Step, stand_in: &mut StandIn) -> Standing {
    step.observe(stand_in).unwrap()
}

fn evidence(standing: Standing) -> String {
    match standing {
        Standing::Done { evidence } => evidence,
        other => panic!("not done: {other:?}"),
    }
}

#[test]
fn the_restore_image_is_found_by_its_partitions_name_and_mounted_only_for_reading() {
    let mut stand_in = StandIn::new();
    stand_in.prints(Tool::Lsblk, LSBLK, &stick(Some(false)));
    assert_eq!(
        standing(&FindRestore, &mut stand_in),
        Standing::Needed { asks: vec![] }
    );
    stand_in.prints(
        Tool::Mount,
        &["-o", "ro,X-mount.mkdir", "/dev/sda4", RESTORE],
        "",
    );
    FindRestore.run(&mut stand_in, &Answers::new()).unwrap();
    assert!(
        stand_in
            .calls
            .contains(&format!("mount -o ro,X-mount.mkdir /dev/sda4 {RESTORE}")),
        "{:#?}",
        stand_in.calls
    );

    let mut mounted = StandIn::new();
    mounted.prints(Tool::Lsblk, LSBLK, &stick(Some(true)));
    assert_eq!(
        evidence(standing(&FindRestore, &mut mounted)),
        "the stick carries macOS 27.2 (26B123)"
    );
}

#[test]
fn a_stick_carrying_no_restore_image_says_so() {
    let mut stand_in = StandIn::new();
    stand_in.prints(Tool::Lsblk, LSBLK, &stick(None));
    assert!(
        matches!(
            standing(&FindRestore, &mut stand_in),
            Standing::Blocked { .. }
        ),
        "a stick with no restore image went on"
    );
}

#[test]
fn a_laptop_already_online_asks_for_no_network() {
    let mut stand_in = StandIn::new();
    stand_in.prints(Tool::Varlinkctl, DESCRIBE, &wifi(true, &[]));
    let said = "wlan0 is on Cabin, which reaches the internet";
    assert_eq!(evidence(standing(&ChooseNetwork, &mut stand_in)), said);
    assert_eq!(evidence(standing(&JoinNetwork, &mut stand_in)), said);
}

#[test]
fn only_networks_needing_no_account_are_offered_strongest_first() {
    let mut stand_in = StandIn::new();
    stand_in.prints(Tool::Varlinkctl, DESCRIBE, &wifi(false, &[]));
    let Standing::Needed { asks } = standing(&ChooseNetwork, &mut stand_in) else {
        panic!("a laptop offline asked for no network");
    };
    let [Ask::Choose { choices, .. }] = asks.as_slice() else {
        panic!("{asks:?}");
    };
    assert_eq!(
        choices,
        &["Cabin, with a password, 4 bars", "Café, open, 1 bar"]
    );
}

fn chosen(at: usize) -> Answers {
    let mut answers = Answers::new();
    answers.give("network", Answer::Chosen(at));
    answers
}

#[test]
fn a_network_with_a_password_is_joined_with_it_on_standard_input() {
    let mut stand_in = StandIn::new();
    stand_in.prints(Tool::Varlinkctl, DESCRIBE, &wifi(false, &[]));
    ChooseNetwork.run(&mut stand_in, &chosen(0)).unwrap();
    assert_eq!(
        evidence(standing(&ChooseNetwork, &mut stand_in)),
        "it joins Cabin"
    );
    let Standing::Needed { asks } = standing(&JoinNetwork, &mut stand_in) else {
        panic!("joining asked for nothing");
    };
    assert_eq!(
        asks,
        vec![Ask::Secret {
            id: "password",
            name: "the password for Cabin".into()
        }]
    );
    stand_in.prints(Tool::Varlinkctl, JOIN, "{\n\t\"stage\" : \"online\"\n}\n");
    let mut answers = Answers::new();
    answers.give(
        "password",
        Answer::Secret(Secret::new("correct horse battery".into())),
    );
    JoinNetwork.run(&mut stand_in, &answers).unwrap();
    assert_eq!(
        stand_in.secrets,
        vec![(
            "stdin".to_string(),
            r#"{"name":"Cabin","password":"correct horse battery"}"#.to_string()
        )]
    );
    assert!(
        stand_in.calls.iter().all(|call| !call.contains("horse")),
        "{:#?}",
        stand_in.calls
    );
}

#[test]
fn an_open_or_saved_network_asks_for_no_password() {
    for (at, saved) in [(1, &[][..]), (0, &["Cabin"][..])] {
        let mut stand_in = StandIn::new();
        stand_in.prints(Tool::Varlinkctl, DESCRIBE, &wifi(false, saved));
        ChooseNetwork.run(&mut stand_in, &chosen(at)).unwrap();
        assert_eq!(
            standing(&JoinNetwork, &mut stand_in),
            Standing::Needed { asks: vec![] }
        );
    }
}

#[test]
fn a_wrong_password_is_said_in_words() {
    let mut stand_in = StandIn::new();
    stand_in.prints(Tool::Varlinkctl, DESCRIBE, &wifi(false, &[]));
    ChooseNetwork.run(&mut stand_in, &chosen(0)).unwrap();
    stand_in.answers(
        Program::Tool(Tool::Varlinkctl),
        JOIN,
        1,
        "",
        "Method call io.jitsusama.Wifi.Join() failed: io.jitsusama.Wifi.WrongPassword\n",
    );
    let mut answers = Answers::new();
    answers.give("password", Answer::Secret(Secret::new("wrong".into())));
    let refused = JoinNetwork.run(&mut stand_in, &answers);
    assert!(
        matches!(&refused, Err(Fault::Blocked { why, .. }) if why == "that password didn't work for Cabin"),
        "{refused:?}"
    );
}

#[test]
fn irecovery_describes_the_mac_waiting_in_dfu_mode() {
    let mac = Mac::parse(IN_DFU).unwrap();
    assert_eq!(
        (mac.ecid.as_str(), mac.product.as_str(), mac.waiting()),
        (ECID, "Mac14,2", true)
    );
    assert_eq!(
        mac.described(),
        "a MacBook Air (M2, 2022), Mac14,2, waits in DFU mode"
    );
}

/// A stick with its restore image mounted, online, and an M2 Air in DFU
/// mode found.
fn found() -> StandIn {
    let mut stand_in = StandIn::new();
    stand_in
        .prints(Tool::Lsblk, LSBLK, &stick(Some(true)))
        .prints(Tool::Irecovery, &["-q"], IN_DFU);
    FindMac.run(&mut stand_in, &Answers::new()).unwrap();
    stand_in
}

#[test]
fn a_mac_not_yet_in_dfu_mode_is_told_how_to_get_there() {
    let mut stand_in = StandIn::new();
    stand_in.refuses(
        Tool::Irecovery,
        &["-q"],
        255,
        "ERROR: Unable to connect to device",
    );
    let Standing::Needed { asks } = standing(&FindMac, &mut stand_in) else {
        panic!("a Mac not plugged in was found");
    };
    let [Ask::Act { what, .. }] = asks.as_slice() else {
        panic!("{asks:?}");
    };
    assert!(
        what.contains("left Control, left Option, right Shift and the power button"),
        "{what}"
    );
    assert!(
        matches!(
            FindMac.run(&mut stand_in, &Answers::new()),
            Err(Fault::Blocked { .. })
        ),
        "no Mac was found, and the step went on"
    );

    let mut found = found();
    assert_eq!(
        evidence(standing(&FindMac, &mut found)),
        "a MacBook Air (M2, 2022), Mac14,2, waits in DFU mode"
    );
}

fn restoring(own: &[&'static str]) -> Vec<&'static str> {
    let mut args = own.to_vec();
    args.extend([
        "-y",
        "-i",
        ECID,
        "-C",
        "/run/guide/revive",
        "--logfile=/run/guide/revive/restore.log",
        FILE,
    ]);
    args
}

#[test]
fn apple_is_asked_to_sign_before_anything_changes() {
    let mut stand_in = found();
    assert_eq!(
        standing(&CheckSigning, &mut stand_in),
        Standing::Needed { asks: vec![] }
    );
    stand_in
        .prints(Tool::Mkdir, &["-p", "/run/guide/revive"], "")
        .prints(Tool::Idevicerestore, &restoring(&["-t"]), "");
    CheckSigning.run(&mut stand_in, &Answers::new()).unwrap();
    assert_eq!(
        evidence(standing(&CheckSigning, &mut stand_in)),
        "Apple signs macOS 27.2 (26B123) for this Mac"
    );
}

#[test]
fn a_macos_apple_no_longer_signs_is_said_in_words() {
    let mut stand_in = found();
    stand_in
        .prints(Tool::Mkdir, &["-p", "/run/guide/revive"], "")
        .answers(
            Program::Tool(Tool::Idevicerestore),
            &restoring(&["-t"]),
            255,
            "ERROR: Unable to get SHSH blobs for this device\n",
            "",
        );
    let refused = CheckSigning.run(&mut stand_in, &Answers::new());
    assert!(
        matches!(&refused, Err(Fault::Blocked { why, .. }) if why.starts_with("Apple no longer signs macOS 27.2 (26B123)")),
        "{refused:?}"
    );
}

#[test]
fn the_revive_keeps_the_macs_disk_through_the_sticks_own_usbmuxd() {
    let mut stand_in = found();
    let Standing::Needed { asks } = standing(&Revive, &mut stand_in) else {
        panic!("the revive asked nothing first");
    };
    assert!(
        matches!(
            asks.as_slice(),
            [Ask::Confirm {
                verb: "revive it",
                ..
            }]
        ),
        "{asks:?}"
    );
    stand_in
        .prints(Tool::Mkdir, &["-p", "/run/guide/revive"], "")
        .prints(Tool::Systemctl, &["start", "guide-usbmuxd.service"], "")
        .prints(Tool::Idevicerestore, &restoring(&[]), "DONE\n")
        .prints(Tool::Systemctl, &["stop", "guide-usbmuxd.service"], "");
    let mut answers = Answers::new();
    answers.give("revive", Answer::Done);
    Revive.run(&mut stand_in, &answers).unwrap();
    let restored = stand_in
        .calls
        .iter()
        .find(|call| call.starts_with("idevicerestore"))
        .unwrap();
    assert!(!restored.contains(" -e"), "{restored}");
    assert!(restored.ends_with("{USBMUXD_SOCKET_ADDRESS}"), "{restored}");
    assert!(
        stand_in
            .calls
            .iter()
            .any(|call| call == "systemctl stop guide-usbmuxd.service"),
        "{:#?}",
        stand_in.calls
    );

    let fact = format!("{ECID} UniversalMac_27.2_26B123");
    assert!(stand_in.journal().knows("revive-mac", &fact));

    // Revived, the Mac starts up into macOS, and every step stays done.
    let mut revived = StandIn::new();
    revived
        .prints(Tool::Lsblk, LSBLK, &stick(Some(true)))
        .refuses(
            Tool::Irecovery,
            &["-q"],
            255,
            "ERROR: Unable to connect to device",
        );
    revived.journal().record("find-mac", ECID).unwrap();
    revived.journal().record("check-signing", &fact).unwrap();
    revived.journal().record("revive-mac", &fact).unwrap();
    assert_eq!(
        evidence(standing(&FindMac, &mut revived)),
        "the Mac was found in DFU mode"
    );
    assert!(matches!(
        standing(&CheckSigning, &mut revived),
        Standing::Done { .. }
    ));
    assert_eq!(
        evidence(standing(&Revive, &mut revived)),
        "macOS 27.2 (26B123) is back on the Mac, which starts up by itself"
    );
}
