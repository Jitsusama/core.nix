use super::*;
use std::os::unix::fs::symlink;

/// A folder of the test's own, gone when it is.
struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn scratch(name: &str) -> Scratch {
    let root = std::env::temp_dir().join(format!("pebble-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    Scratch(root)
}

/// The media keys' collection and, after it, the control one, as the
/// speakers' firmware 1.31 describes them; 1.10 had a firmware update's
/// on 0xFF99 in its place.
const DESCRIBED: [u8; 20] = [
    0x05, 0x0C, 0x09, 0x01, 0xA1, 0x01, 0xC0, 0x06, 0x01, 0xFF, 0x09, 0x01, 0xA1, 0x01, 0x85, 0x03,
    0x0A, 0x3D, 0xFF, 0xC0,
];
const OLD: [u8; 10] = [0x05, 0x0C, 0x09, 0x01, 0xC0, 0x06, 0x99, 0xFF, 0x09, 0x01];

fn hidraw(root: &Path, name: &str, id: &str, descriptor: &[u8]) {
    let device = root.join("class/hidraw").join(name).join("device");
    fs::create_dir_all(&device).unwrap();
    fs::write(
        device.join("uevent"),
        format!("DRIVER=hid-generic\nHID_ID={id}\n"),
    )
    .unwrap();
    fs::write(device.join("report_descriptor"), descriptor).unwrap();
}

#[test]
fn the_speakers_control_node_is_theirs_with_the_control_collection() {
    let root = scratch("control");
    hidraw(&root.0, "hidraw0", "0003:0000046D:0000C900", &DESCRIBED);
    hidraw(&root.0, "hidraw4", "0003:0000041E:0000329A", &DESCRIBED);
    assert_eq!(
        control(&root.0, Path::new("/dev")),
        Some(PathBuf::from("/dev/hidraw4"))
    );
}

#[test]
fn speakers_on_firmware_without_the_control_collection_have_none() {
    let root = scratch("old");
    hidraw(&root.0, "hidraw1", "0003:0000041E:0000329A", &OLD);
    assert_eq!(control(&root.0, Path::new("/dev")), None);
    assert_eq!(control(&root.0.join("nothing"), Path::new("/dev")), None);
}

fn sound_card(root: &Path, number: u32, vendor: &str, product: &str) {
    let usb = root.join(format!("devices/usb{number}"));
    let interface = usb.join("interface");
    fs::create_dir_all(&interface).unwrap();
    fs::write(usb.join("idVendor"), format!("{vendor}\n")).unwrap();
    fs::write(usb.join("idProduct"), format!("{product}\n")).unwrap();
    let card = root.join(format!("class/sound/card{number}"));
    fs::create_dir_all(&card).unwrap();
    symlink(&interface, card.join("device")).unwrap();
}

#[test]
fn the_speakers_sound_card_is_found_by_its_usb_device() {
    let root = scratch("card");
    sound_card(&root.0, 0, "8086", "0000");
    sound_card(&root.0, 2, "14ed", "1012");
    sound_card(&root.0, 3, "041e", "329a");
    fs::create_dir_all(root.0.join("class/sound/controlC3")).unwrap();
    assert_eq!(card(&root.0), Some(3));
    assert_eq!(card(&root.0.join("nothing")), None);
}

#[test]
fn a_closed_playback_device_plays_nothing_and_an_open_one_does() {
    assert!(!open("closed\n"), "a closed device counted as playing");
    assert!(!open(""), "an empty status counted as playing");
    assert!(
        open("state: RUNNING\nowner_pid   : 2114\n"),
        "a running device counted as silent"
    );
    assert!(
        open("state: PREPARED\n"),
        "a prepared device counted as silent"
    );
}

#[test]
fn only_a_lock_s_file_saying_locked_is_locked() {
    assert!(locked("locked\n"), "\"locked\" read as unlocked");
    assert!(!locked("unlocked\n"), "\"unlocked\" read as locked");
    assert!(!locked(""), "an empty file read as locked");
}

#[test]
fn the_lighting_is_on_while_sound_plays_and_for_a_while_after() {
    let idle = IDLE;
    assert!(
        wanted(true, false, Duration::ZERO, idle),
        "sound playing left the lighting off"
    );
    assert!(
        wanted(true, true, Duration::ZERO, idle),
        "music while locked"
    );
    assert!(
        wanted(false, false, Duration::from_secs(899), idle),
        "the lighting went out before the idle time passed"
    );
    assert!(
        !wanted(false, false, IDLE, idle),
        "the lighting stayed on once the idle time passed"
    );
    assert!(
        !wanted(false, true, Duration::from_secs(1), idle),
        "the lighting stayed on while locked and quiet"
    );
}

#[test]
fn a_command_is_framed_as_the_speakers_read_it() {
    let report = frame(LIGHTING, &[SWITCH, 0]);
    assert_eq!(report[..7], [0x03, 0x6A, 0x3A, 0x02, 0x00, 0x25, 0x00]);
    assert!(
        report[7..].iter().all(|byte| *byte == 0),
        "the report's padding held {:?}",
        &report[7..]
    );
    assert_eq!(report.len(), 64);
}

/// A report as the speakers send it, padded as hidraw gives it.
fn from_speakers(command: u8, payload: &[u8]) -> Vec<u8> {
    let mut report = frame(command, payload).to_vec();
    report.truncate(REPORT);
    report
}

#[test]
fn what_the_speakers_say_of_their_lighting_is_understood() {
    // As they answered on 2026-10-23: the switch taken, then its state.
    assert_eq!(
        said(&from_speakers(0x02, &[0x3A, 0x00, 0x25])),
        Said::Acknowledged {
            operation: SWITCH,
            taken: true
        }
    );
    assert_eq!(
        said(&from_speakers(0x02, &[0x3A, 0x83, 0x25])),
        Said::Acknowledged {
            operation: SWITCH,
            taken: false
        }
    );
    assert_eq!(said(&from_speakers(0x3A, &[0x26, 0x00])), Said::Lit(false));
    assert_eq!(said(&from_speakers(0x3A, &[0x26, 0x01])), Said::Lit(true));
    assert_eq!(
        said(&from_speakers(0x3A, &[0x2C, 0x01, 0x03, 0x35, 0x05])),
        Said::Other
    );
}

#[test]
fn what_isn_t_a_whole_report_of_theirs_says_nothing() {
    assert_eq!(said(&[0x01, 0x40]), Said::Other);
    assert_eq!(said(&[0x03, 0x6A, 0x3A, 0x09, 0x00, 0x26]), Said::Other);
    assert_eq!(said(&[]), Said::Other);
}

fn lights() -> Lights {
    Lights::new(PathBuf::from("/nonexistent"), IDLE)
}

#[test]
fn the_speakers_saying_what_they_were_told_is_no_hand_on_them() {
    let mut lights = lights();
    lights.lit = Some(false);
    lights.told = Some(false);
    lights.hear(&[Said::Lit(false)]);
    assert!(
        !lights.by_hand,
        "the echo of what it was told counted as a hand on them"
    );
    assert_eq!(lights.told, None);
}

#[test]
fn the_speakers_echoing_a_switch_as_it_s_made_is_no_hand_on_them() {
    let mut lights = lights();
    lights.told = Some(true);
    lights.lit = Some(true);
    lights.hear(&[Said::Other, Said::Lit(true)]);
    assert!(
        !lights.by_hand,
        "the echo of a switch as it was made counted as a hand on them"
    );
    assert_eq!(lights.told, None);
}

#[test]
fn lighting_switched_off_at_the_speakers_stays_off_until_switched_on() {
    let mut lights = lights();
    lights.lit = Some(true);
    lights.playing = true;
    lights.hear(&[Said::Lit(false)]);
    assert!(
        lights.by_hand,
        "switching off at the speakers didn't count as by hand"
    );
    assert!(!lights.wanted(), "even with sound playing");
    lights.hear(&[Said::Lit(true)]);
    assert!(
        !lights.by_hand,
        "switching back on at the speakers still counted as by hand"
    );
    assert!(
        lights.wanted(),
        "switching back on at the speakers left the lighting unwanted"
    );
}

#[test]
fn lighting_switched_on_at_the_speakers_counts_as_someone_using_them() {
    let mut lights = lights();
    lights.lit = Some(false);
    lights.active = Instant::now()
        .checked_sub(Duration::from_secs(3600))
        .unwrap();
    assert!(
        !lights.wanted(),
        "an hour of quiet left the lighting wanted"
    );
    lights.hear(&[Said::Lit(true)]);
    assert!(
        lights.wanted(),
        "switching on at the speakers left the lighting unwanted"
    );
}

#[test]
fn what_the_lighting_does_is_said_only_while_there_are_speakers() {
    let mut lights = lights();
    lights.lit = Some(false);
    assert_eq!(lights.doing(), None);
    lights.speakers = Some(Speakers {
        file: File::open("/dev/null").unwrap(),
    });
    assert_eq!(lights.doing(), Some("off"));
    lights.by_hand = true;
    assert_eq!(lights.doing(), Some("off by hand"));
    lights.lit = Some(true);
    assert_eq!(lights.doing(), Some("on"));
    lights.lit = None;
    assert_eq!(lights.doing(), None, "not yet known");
}

#[test]
fn it_waits_to_go_out_only_while_lit_unlocked_and_quiet() {
    let mut lights = lights();
    lights.lit = Some(true);
    assert!(
        lights.until_quiet().is_some_and(|left| left <= lights.idle),
        "lit and quiet, it waited {:?} to go out",
        lights.until_quiet()
    );
    lights.playing = true;
    assert_eq!(lights.until_quiet(), None);
    lights.playing = false;
    lights.locked = true;
    assert_eq!(lights.until_quiet(), None);
    lights.locked = false;
    lights.lit = Some(false);
    assert_eq!(lights.until_quiet(), None);
}
