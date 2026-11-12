//! A lid that says it's shut while Joel works on the laptop under it: a
//! fault no journal line tells of, since every program believes the lid.
//! On 12 November optimus's lid read shut for over half an hour with its
//! screen on and its keys typed on, and nothing said so.

use std::fs;
use std::path::Path;

/// What one look at the lid, the laptop's screens and its keys saw.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Looked {
    /// Whether any lid reads shut.
    pub shut: bool,
    /// Whether a built-in screen is on: enabled, and its power on.
    pub screen_on: bool,
    /// The laptop's own keyboard's and touchpad's interrupts so far.
    pub keys: u64,
}

/// What the lid says.
const LIDS: &str = "/proc/acpi/button/lid";

/// Where each screen's connector is, a built-in one named eDP.
const SCREENS: &str = "/sys/class/drm";

/// Look at the lid, the built-in screens and the laptop's own keys.
#[must_use]
pub fn look() -> Looked {
    Looked {
        shut: lid_shut(Path::new(LIDS)),
        screen_on: screen_on(Path::new(SCREENS)),
        keys: fs::read_to_string("/proc/interrupts").map_or(0, |said| keys(&said)),
    }
}

/// Whether a lid under `lids` reads closed.
fn lid_shut(lids: &Path) -> bool {
    fs::read_dir(lids)
        .into_iter()
        .flatten()
        .flatten()
        .any(|lid| {
            fs::read_to_string(lid.path().join("state")).is_ok_and(|state| state.contains("closed"))
        })
}

/// Whether a built-in screen's connector under `screens` is enabled with
/// its power on.
fn screen_on(screens: &Path) -> bool {
    fs::read_dir(screens)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|connector| connector.file_name().to_string_lossy().contains("-eDP-"))
        .any(|connector| {
            let said =
                |name: &str| fs::read_to_string(connector.path().join(name)).unwrap_or_default();
            said("enabled").trim() == "enabled" && said("dpms").trim() == "On"
        })
}

/// The interrupts the laptop's own keyboard and touchpad have raised, from
/// `/proc/interrupts`, where the i8042 controller has their lines.
fn keys(interrupts: &str) -> u64 {
    interrupts
        .lines()
        .filter(|line| line.contains("i8042"))
        .flat_map(|line| {
            line.split_whitespace()
                .skip(1)
                .map_while(|count| count.parse::<u64>().ok())
        })
        .sum()
}

/// Whether the lid is wrong: it reads shut while the built-in screen is on
/// and the laptop's own keys were used since the look before.
#[must_use]
pub fn wrong(now: &Looked, before: Option<&Looked>) -> bool {
    now.shut && now.screen_on && before.is_some_and(|before| now.keys > before.keys)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_lid_shut_over_a_screen_in_use_is_wrong() {
        let used = Looked {
            shut: true,
            screen_on: true,
            keys: 10,
        };
        let before = Looked { keys: 9, ..used };
        assert!(wrong(&used, Some(&before)), "shut, on and typed on");
        assert!(!wrong(&used, None), "not on the first look");
        assert!(!wrong(&used, Some(&used)), "not while the keys rest");
        let dark = Looked {
            screen_on: false,
            ..used
        };
        assert!(
            !wrong(&dark, Some(&before)),
            "a shut lid over a dark screen is right"
        );
    }

    #[test]
    fn the_files_are_read_as_the_kernel_writes_them() {
        let root = tempfile::tempdir().expect("a folder");
        let lid = root.path().join("lid/LID0");
        fs::create_dir_all(&lid).expect("a lid");
        fs::write(lid.join("state"), "state:      closed\n").expect("its state");
        assert!(lid_shut(&root.path().join("lid")), "closed reads shut");
        let screen = root.path().join("drm/card0-eDP-1");
        fs::create_dir_all(&screen).expect("a screen");
        fs::write(screen.join("enabled"), "enabled\n").expect("enabled");
        fs::write(screen.join("dpms"), "On\n").expect("its power");
        let other = root.path().join("drm/card0-DP-2");
        fs::create_dir_all(&other).expect("another screen");
        assert!(
            screen_on(&root.path().join("drm")),
            "the built-in screen is on"
        );
        let interrupts = "           CPU0       CPU1\n\
                            1:          0      64917   IR-IO-APIC    1-edge      i8042\n\
                            8:          0          0   IR-IO-APIC    8-edge      rtc0\n\
                           12:        125          0   IR-IO-APIC   12-edge      i8042\n";
        assert_eq!(
            keys(interrupts),
            65_042,
            "both i8042 lines, every processor"
        );
    }
}
