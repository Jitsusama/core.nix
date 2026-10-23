//! Keeps logind's lock against the lid's handling held while a monitor is
//! plugged in, so a laptop shut at a desk keeps running once going idle turns
//! the monitor off. logind counts a monitor only while its connector drives
//! it, but one plugged in reads connected with its screen off.
//!
//! Usage: lid-docked SYSTEMCTL UNIT. It looks at the graphics cards'
//! connectors once, and starts UNIT, which holds the lock, when a monitor is
//! plugged in and stops it when none is, through SYSTEMCTL without waiting.
//! Starting a unit that runs does nothing, so the lock never lapses while a
//! monitor stays. udev runs it whenever a card says a monitor came or went.
//! `LID_DOCKED_DRM` names the connectors' directory, for tests.

use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::Path;
use std::process::{Command, ExitCode};

/// The connector types logind counts as a monitor's, by the part of a
/// connector's name after its card, as logind-core.c lists them, and USB,
/// for a monitor on a USB display adapter. Anything else, the laptop's own
/// eDP screen among them, isn't one.
const EXTERNAL: [&str; 13] = [
    "VGA-",
    "DVI-I-",
    "DVI-D-",
    "DVI-A-",
    "Composite-",
    "SVIDEO-",
    "Component-",
    "DIN-",
    "DP-",
    "HDMI-A-",
    "HDMI-B-",
    "TV-",
    "USB-",
];

/// Whether a DRM device's name is an external connector's, such as
/// card0-DP-5, and not a card's, a render node's or the laptop's screen.
fn external(name: &str) -> bool {
    name.split_once('-').is_some_and(|(card, kind)| {
        card.starts_with("card") && EXTERNAL.iter().any(|prefix| kind.starts_with(prefix))
    })
}

/// The external connectors with something plugged in: any whose status
/// isn't "disconnected", as logind counts them, sorted.
fn plugged_in(drm: &Path) -> Vec<String> {
    let Ok(entries) = fs::read_dir(drm) else {
        return Vec::new();
    };
    let mut plugged: Vec<String> = entries
        .flatten()
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| external(name))
        .filter(|name| {
            fs::read_to_string(drm.join(name).join("status"))
                .map_or(true, |status| status.trim() != "disconnected")
        })
        .collect();
    plugged.sort();
    plugged
}

/// What to ask of the unit that holds the lock, given what's plugged in.
fn verb(plugged: &[String]) -> &'static str {
    if plugged.is_empty() { "stop" } else { "start" }
}

fn main() -> ExitCode {
    let mut args = env::args_os().skip(1);
    let (Some(systemctl), Some(unit), None) = (args.next(), args.next(), args.next()) else {
        eprintln!("usage: lid-docked SYSTEMCTL UNIT");
        return ExitCode::from(2);
    };
    let drm = env::var_os("LID_DOCKED_DRM").unwrap_or_else(|| OsString::from("/sys/class/drm"));
    let plugged = plugged_in(Path::new(&drm));
    let verb = verb(&plugged);
    match Command::new(&systemctl)
        .args(["--no-block", verb])
        .arg(&unit)
        .status()
    {
        Ok(status) if status.success() => ExitCode::SUCCESS,
        Ok(status) => {
            eprintln!("lid-docked: systemctl {verb} failed: {status}");
            ExitCode::FAILURE
        }
        Err(error) => {
            eprintln!(
                "lid-docked: can't run {}: {error}",
                Path::new(&systemctl).display()
            );
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::process;

    #[test]
    fn knows_a_monitors_connector() {
        assert!(external("card0-DP-5"));
        assert!(external("card1-HDMI-A-1"));
        assert!(external("card0-DVI-D-1"));
    }

    #[test]
    fn knows_what_isnt_a_monitors_connector() {
        assert!(!external("card0-eDP-1"));
        assert!(!external("card0-LVDS-1"));
        assert!(!external("card0-DSI-1"));
        assert!(!external("card0-Virtual-1"));
        assert!(!external("card0"));
        assert!(!external("renderD128"));
        assert!(!external("version"));
    }

    struct Drm(PathBuf);

    impl Drm {
        fn new(name: &str) -> Drm {
            let dir = env::temp_dir().join(format!("lid-docked-{name}-{}", process::id()));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(&dir).unwrap();
            Drm(dir)
        }

        fn connector(&self, name: &str, status: Option<&str>) -> &Drm {
            let dir = self.0.join(name);
            fs::create_dir_all(&dir).unwrap();
            if let Some(status) = status {
                fs::write(dir.join("status"), format!("{status}\n")).unwrap();
            }
            self
        }
    }

    impl Drop for Drm {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn counts_a_monitor_whose_screen_is_off() {
        let drm = Drm::new("off");
        drm.connector("card0-eDP-1", Some("connected"))
            .connector("card0-DP-1", Some("disconnected"))
            .connector("card0-DP-5", Some("connected"));
        fs::write(drm.0.join("card0-DP-5").join("enabled"), "disabled\n").unwrap();
        assert_eq!(plugged_in(&drm.0), ["card0-DP-5"]);
    }

    #[test]
    fn counts_every_status_but_disconnected() {
        let drm = Drm::new("status");
        drm.connector("card0-DP-1", Some("unknown"))
            .connector("card0-HDMI-A-1", None)
            .connector("card0-DP-2", Some("disconnected"));
        assert_eq!(plugged_in(&drm.0), ["card0-DP-1", "card0-HDMI-A-1"]);
    }

    #[test]
    fn finds_nothing_with_only_the_laptops_screen() {
        let drm = Drm::new("laptop");
        drm.connector("card0-eDP-1", Some("connected"))
            .connector("card0-DP-1", Some("disconnected"));
        assert!(plugged_in(&drm.0).is_empty());
    }

    #[test]
    fn finds_nothing_without_a_card() {
        assert!(plugged_in(Path::new("/nonexistent")).is_empty());
    }

    #[test]
    fn holds_the_lid_only_with_a_monitor() {
        assert_eq!(verb(&[]), "stop");
        assert_eq!(verb(&["card0-DP-5".to_owned()]), "start");
    }
}
