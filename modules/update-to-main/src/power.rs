//! How the machine is powered, so an update isn't started on a battery
//! that may not last it: a build and a switch take minutes and a battery's
//! worth of work, and a machine that dies mid-switch boots what it had.

use std::fs;
use std::path::Path;
use std::process::Command;

/// The machine's power, as far as the update needs it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Power {
    /// On a charger, or with no battery at all.
    Plugged,
    /// On battery, at this charge.
    Battery(u8),
}

/// Reads Linux's power supplies: any mains or USB supply online means
/// plugged, and otherwise the lowest battery's charge counts.
pub fn linux(supplies: &Path) -> Power {
    let Ok(entries) = fs::read_dir(supplies) else {
        return Power::Plugged;
    };
    let mut lowest: Option<u8> = None;
    for entry in entries.flatten() {
        let path = entry.path();
        let read = |name: &str| {
            fs::read_to_string(path.join(name))
                .ok()
                .map(|s| s.trim().to_owned())
        };
        match read("type").as_deref() {
            Some("Battery") => {
                if let Some(charge) = read("capacity").and_then(|c| c.parse::<u8>().ok()) {
                    lowest = Some(lowest.map_or(charge, |l| l.min(charge)));
                }
            }
            Some(_) if read("online").as_deref() == Some("1") => return Power::Plugged,
            _ => {}
        }
    }
    lowest.map_or(Power::Plugged, Power::Battery)
}

/// Reads a Mac's power from `pmset -g batt`, whose first line names the
/// source and whose next gives the charge.
pub fn mac(pmset: &Path) -> Power {
    let Ok(out) = Command::new(pmset).args(["-g", "batt"]).output() else {
        return Power::Plugged;
    };
    parse_pmset(&String::from_utf8_lossy(&out.stdout))
}

pub fn parse_pmset(text: &str) -> Power {
    if !text.contains("'Battery Power'") {
        return Power::Plugged;
    }
    let charge = text
        .split(|c: char| c.is_whitespace() || c == ';')
        .find_map(|word| word.strip_suffix('%').and_then(|n| n.parse::<u8>().ok()));
    charge.map_or(Power::Plugged, Power::Battery)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_mac_on_its_charger_is_plugged() {
        let text = "Now drawing from 'AC Power'\n -InternalBattery-0 (id=1)\t80%; charging;\n";
        assert_eq!(parse_pmset(text), Power::Plugged);
    }

    #[test]
    fn a_mac_on_battery_says_its_charge() {
        let text = "Now drawing from 'Battery Power'\n -InternalBattery-0 (id=1)\t17%; discharging; 1:02 remaining present: true\n";
        assert_eq!(parse_pmset(text), Power::Battery(17));
    }

    #[test]
    fn linux_online_mains_is_plugged_whatever_the_battery() {
        let dir = tempfile::tempdir().unwrap();
        supply(
            dir.path(),
            "BAT0",
            &[("type", "Battery"), ("capacity", "5")],
        );
        supply(dir.path(), "AC", &[("type", "Mains"), ("online", "1")]);
        assert_eq!(linux(dir.path()), Power::Plugged);
    }

    #[test]
    fn linux_on_battery_says_the_lowest_charge() {
        let dir = tempfile::tempdir().unwrap();
        supply(
            dir.path(),
            "BAT0",
            &[("type", "Battery"), ("capacity", "40")],
        );
        supply(
            dir.path(),
            "BAT1",
            &[("type", "Battery"), ("capacity", "12")],
        );
        supply(dir.path(), "AC", &[("type", "Mains"), ("online", "0")]);
        assert_eq!(linux(dir.path()), Power::Battery(12));
    }

    #[test]
    fn linux_with_no_battery_is_plugged() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(linux(dir.path()), Power::Plugged);
    }

    fn supply(root: &Path, name: &str, files: &[(&str, &str)]) {
        let dir = root.join(name);
        fs::create_dir(&dir).unwrap();
        for (file, text) in files {
            fs::write(dir.join(file), format!("{text}\n")).unwrap();
        }
    }
}
