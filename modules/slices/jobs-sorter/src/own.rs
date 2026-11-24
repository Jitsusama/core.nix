//! A unit's own settings, read from `systemctl cat`'s text, leaving out the
//! runtime ones this sorter sets, so a give-back puts the unit's own back
//! rather than systemd's defaults. How a unit is ranked beside them is
//! `src/hosts.rs`'s to say; this changes only as systemd's files do.
//!
//! Those runtime settings are files, and systemd keeps them after the unit
//! stops: on 22 November 40 of the 124 folders pi's conversation units had
//! there were for units long gone. So once a unit no longer runs, the
//! sorter takes its own files of them back ([`left_behind`], [`forget`]),
//! from the folders it marked as it set them ([`mark`]).

use std::fs;
use std::io;
use std::path::Path;

/// The folder runtime properties set through systemd's manager land in,
/// which a unit's own settings leave out.
const RUNTIME_CONTROL: &str = "/systemd/user.control/";

/// The properties the sorter sets on a host's or a conversation's unit
/// (`crate::hosts::rank_call`), each kept by systemd as `50-<name>.conf`
/// in the unit's folder under the runtime control folder.
pub const SET: [&str; 5] = [
    "CPUWeight",
    "MemoryLow",
    "MemoryZSwapWriteback",
    "ManagedOOMPreference",
    "MemoryMin",
];

/// The file the sorter leaves in a unit's folder under the runtime
/// control folder once its settings land there, so a sweep takes back
/// only what it set: systemd writes the same files for a setting Joel
/// gives with `systemctl set-property --runtime`, and reads only those
/// ending `.conf`, so this one is the sorter's alone.
pub const MARK: &str = "jobs-sorter-set-these";

/// The units whose folders in `control`, the runtime control folder, the
/// sorter marked ([`mark`]) and start with one of `prefixes`: services
/// alone, never a template, whose settings every instance shares, and
/// never a folder reached through a link, which could point anywhere. A
/// control folder not there yet holds none.
pub fn left_behind(control: &Path, prefixes: &[&str]) -> io::Result<Vec<String>> {
    let entries = match fs::read_dir(control) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error),
    };
    let mut units: Vec<String> = entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            let unit = name.strip_suffix(".d")?.to_owned();
            // The entry's own type, which a link doesn't pass on.
            let folder = entry.file_type().is_ok_and(|kind| kind.is_dir());
            let ours = folder
                && prefixes.iter().any(|prefix| unit.starts_with(prefix))
                && unit.ends_with(".service")
                && !unit.ends_with("@.service")
                && entry.path().join(MARK).exists();
            ours.then_some(unit)
        })
        .collect();
    units.sort();
    Ok(units)
}

/// Marks `unit`'s folder in `control` as holding settings the sorter set,
/// once they've landed there; a folder not there, or reached through a
/// link, is left unmarked, so its settings are never taken back.
pub fn mark(control: &Path, unit: &str) -> io::Result<()> {
    let folder = control.join(format!("{unit}.d"));
    if !fs::symlink_metadata(&folder)?.is_dir() {
        return Ok(());
    }
    let mark = folder.join(MARK);
    if mark.exists() {
        return Ok(());
    }
    fs::write(
        mark,
        "The jobs sorter set this unit's 50-*.conf settings here, and takes them back once it \
         stops.\n",
    )
}

/// What `systemctl is-active` says of a unit, as far as taking back its
/// settings goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Active {
    /// Inactive: stopped, or not loaded at all, so its settings can go.
    Stopped,
    /// Failed: stopped, but kept loaded until it's reset, and systemd
    /// still lists its settings' files, so they stay.
    Failed,
    /// Anything else, starting, stopping or running, keeps them.
    Runs,
}

/// The words `systemctl` is given to say whether each of `units` runs,
/// one state a line in their order; `--` ends its options, so a unit's
/// name, read from a folder, is never taken for one.
pub fn is_active_call(units: &[String]) -> Vec<String> {
    ["--user", "is-active", "--"]
        .iter()
        .map(|word| (*word).to_owned())
        .chain(units.iter().cloned())
        .collect()
}

/// Each of the `asked` units' states from `systemctl is-active`'s
/// `stdout`; none when it doesn't say one for each, as when it can't
/// reach the user manager and says nothing.
pub fn states(stdout: &str, asked: usize) -> Option<Vec<Active>> {
    let states: Vec<Active> = stdout
        .lines()
        .map(|state| match state.trim() {
            "inactive" => Active::Stopped,
            "failed" => Active::Failed,
            _ => Active::Runs,
        })
        .collect();
    (states.len() == asked).then_some(states)
}

/// Takes the settings the sorter sets off `unit` in `control`, and its
/// mark and folder once nothing else is left in it, saying how many it
/// took, and the first error, if any: each file is tried whatever
/// another's removal did. Another's settings stay, and the folder with
/// them. A folder reached through a link is left whole.
pub fn forget(control: &Path, unit: &str) -> (usize, Option<io::Error>) {
    let folder = control.join(format!("{unit}.d"));
    if !fs::symlink_metadata(&folder).is_ok_and(|folder| folder.is_dir()) {
        return (0, None);
    }
    let mut taken = 0;
    let mut failed = None;
    for set in SET {
        match fs::remove_file(folder.join(format!("50-{set}.conf"))) {
            Ok(()) => taken += 1,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => {
                failed.get_or_insert(error);
            }
        }
    }
    // The mark stays while a setting does, so the next sorter's first look tries again.
    if failed.is_none() {
        let _ = fs::remove_file(folder.join(MARK));
    }
    // Another's setting keeps the folder, which then can't be removed, and that's as it should be.
    let _ = fs::remove_dir(&folder);
    (taken, failed)
}

/// A unit's own settings, as far as ranking it goes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Own {
    pub floor: u64,
    pub hard: u64,
    pub writeback: bool,
    pub preference: String,
}

impl Default for Own {
    /// systemd's defaults, which a unit that says nothing has.
    fn default() -> Self {
        Own {
            floor: 0,
            hard: 0,
            writeback: true,
            preference: "none".to_owned(),
        }
    }
}

/// The template an instance's own settings come from, `<name>@.service`
/// for `<name>@<instance>.service`, so its instances' are read once;
/// none for a unit that isn't an instance.
pub fn template(unit: &str) -> Option<String> {
    let (name, _) = unit.strip_suffix(".service")?.split_once('@')?;
    Some(format!("{name}@.service"))
}

/// A unit's own settings from `systemctl cat`'s text, its files each
/// under a `# <path>` line, leaving out the runtime ones set through the
/// manager. A later file's setting wins, and an empty one is systemd's
/// default again. A percentage is of `total`, the machine's memory.
pub fn own(text: &str, total: u64) -> Own {
    let mut own = Own::default();
    let mut counted = true;
    for line in text.lines() {
        let line = line.trim();
        if let Some(path) = line.strip_prefix("# /") {
            counted = !format!("/{path}").contains(RUNTIME_CONTROL);
            continue;
        }
        if !counted || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let value = value.trim();
        match key.trim() {
            "MemoryLow" => {
                own.floor = if value.is_empty() {
                    0
                } else {
                    size(value, total).unwrap_or(own.floor)
                };
            }
            "MemoryMin" => {
                own.hard = if value.is_empty() {
                    0
                } else {
                    size(value, total).unwrap_or(own.hard)
                };
            }
            "MemoryZSwapWriteback" => {
                own.writeback = value.is_empty() || !matches!(value, "no" | "false" | "0" | "off");
            }
            "ManagedOOMPreference" => {
                own.preference = if value.is_empty() {
                    "none".to_owned()
                } else {
                    value.to_owned()
                };
            }
            _ => {}
        }
    }
    own
}

/// A size as systemd reads one: bytes, with K, M, G or T for powers of
/// 1024, `infinity`, or a percentage of `total`.
pub fn size(value: &str, total: u64) -> Option<u64> {
    if value == "infinity" {
        return Some(u64::MAX);
    }
    if let Some(percent) = value.strip_suffix('%') {
        let percent: u128 = percent.trim().parse().ok()?;
        return u64::try_from(u128::from(total) * percent / 100).ok();
    }
    let (digits, shift) = match value.chars().last()? {
        'K' => (&value[..value.len() - 1], 10),
        'M' => (&value[..value.len() - 1], 20),
        'G' => (&value[..value.len() - 1], 30),
        'T' => (&value[..value.len() - 1], 40),
        _ => (value, 0),
    };
    let number: u64 = digits.trim().parse().ok()?;
    number.checked_mul(1 << shift)
}

#[cfg(test)]
mod tests {
    use super::*;

    const GIB: u64 = 1 << 30;

    #[test]
    fn a_units_own_settings_leave_out_the_runtime_ones() {
        let text = "\
# /home/j/.config/systemd/user/pi-host-work-attended.service -> /nix/store/x
[Service]
ManagedOOMPreference=avoid
MemoryLow=512M

# /run/user/1000/systemd/user.control/pi-host-work-attended.service.d/50-MemoryLow.conf
[Service]
MemoryLow=3221225472
ManagedOOMPreference=none
MemoryMin=536870912
";
        assert_eq!(
            own(text, 64 * GIB),
            Own {
                floor: 512 << 20,
                hard: 0,
                writeback: true,
                preference: "avoid".to_owned(),
            },
            "a floor this sorter set at runtime isn't the unit's own"
        );
    }

    #[test]
    fn a_later_file_wins_and_an_empty_setting_is_the_default_again() {
        let text = "\
# /etc/systemd/user/pi-host-a.service
[Service]
MemoryLow=1G
MemoryZSwapWriteback=no
# /etc/systemd/user/pi-host-a.service.d/10-more.conf
[Service]
MemoryLow=
MemoryLow=10%
";
        let own = own(text, 100 * GIB);
        assert_eq!(
            own.floor,
            10 * GIB,
            "the last floor said is the one that holds"
        );
        assert!(!own.writeback, "writeback turned off stays off");
        assert_eq!(
            super::own("[Service]\nMemoryMin=64M\n", GIB).hard,
            64 << 20,
            "a unit's own hard floor is read, to be given back"
        );
        assert_eq!(
            super::own("[Service]\nMemoryLow=\n", GIB).floor,
            0,
            "an empty floor is systemd's default"
        );
    }

    #[test]
    fn sizes_read_as_systemd_reads_them() {
        assert_eq!(size("512M", 0), Some(512 << 20), "M is mebibytes");
        assert_eq!(size("1G", 0), Some(GIB), "G is gibibytes");
        assert_eq!(size("4096", 0), Some(4096), "no suffix is bytes");
        assert_eq!(size("infinity", 0), Some(u64::MAX), "infinity is no limit");
        assert_eq!(
            size("25%", 8 * GIB),
            Some(2 * GIB),
            "a percentage is of memory"
        );
        assert_eq!(size("lots", 0), None, "words aren't a size");
    }

    #[test]
    fn only_a_gone_units_settings_the_sorter_marked_are_taken_back_and_anothers_left() {
        let control = std::env::temp_dir().join(format!("own-control-{}", std::process::id()));
        let _ = fs::remove_dir_all(&control);
        // The five files systemd writes for the settings the sorter sets, spelled as it does.
        let ours = [
            "50-CPUWeight.conf",
            "50-MemoryLow.conf",
            "50-MemoryZSwapWriteback.conf",
            "50-ManagedOOMPreference.conf",
            "50-MemoryMin.conf",
        ];
        let folder = |unit: &str, files: &[&str], marked: bool| {
            let dir = control.join(format!("{unit}.d"));
            fs::create_dir_all(&dir).unwrap();
            for file in files {
                fs::write(dir.join(file), "[Service]\n").unwrap();
            }
            if marked {
                mark(&control, unit).unwrap();
            }
        };
        folder("pi-conversation-w@gone.service", &ours, true);
        folder("pi-host-work-gone.service", &ours, true);
        folder(
            "pi-conversation-w@shared.service",
            &[ours[1], "50-TasksMax.conf"],
            true,
        );
        folder("pi-conversation-w@reverted.service", &[], true);
        folder("pi-conversation-w@by-hand.service", &ours, false);
        folder("pi-conversation-w@.service", &ours, true);
        folder("pi-conversation-x.scope", &ours, true);
        folder("pi-host-work.slice", &ours, true);
        folder("kitty-1-2.scope", &ours, true);
        let elsewhere = control.join("elsewhere");
        folder("elsewhere/pi-host-work-linked.service", &ours, true);
        std::os::unix::fs::symlink(
            elsewhere.join("pi-host-work-linked.service.d"),
            control.join("pi-host-work-linked.service.d"),
        )
        .unwrap();
        let found = left_behind(&control, &["pi-conversation-", "pi-host-"]);
        let taken = [
            "pi-conversation-w@gone.service",
            "pi-conversation-w@reverted.service",
            "pi-conversation-w@shared.service",
            "pi-host-work-gone.service",
        ]
        .map(|unit| forget(&control, unit).0);
        let linked = forget(&control, "pi-host-work-linked.service").0;
        let there = |path: &str| control.join(path).exists();
        let left = [
            "pi-conversation-w@gone.service.d",
            "pi-conversation-w@reverted.service.d",
            "pi-conversation-w@shared.service.d/50-TasksMax.conf",
            "pi-conversation-w@shared.service.d/50-MemoryLow.conf",
            "elsewhere/pi-host-work-linked.service.d/50-MemoryMin.conf",
        ]
        .map(there);
        let _ = fs::remove_dir_all(&control);
        assert_eq!(
            found.unwrap(),
            [
                "pi-conversation-w@gone.service",
                "pi-conversation-w@reverted.service",
                "pi-conversation-w@shared.service",
                "pi-host-work-gone.service",
            ],
            "only services of the prefixes the sorter marked, no template, no other kind of \
             unit, none set by hand and none through a link"
        );
        assert_eq!(
            (taken, linked),
            ([5, 0, 1, 5], 0),
            "every one of the sorter's settings is taken, and none through a link"
        );
        assert_eq!(
            left,
            [false, false, true, false, true],
            "a folder left empty goes, its mark with it, while another's setting stays, and \
             what a link points to"
        );
        assert!(
            left_behind(&control, &["pi-host-"]).unwrap().is_empty()
                && forget(&control, "pi-host-work-never.service").0 == 0,
            "a control folder not there yet holds nothing to take back"
        );
    }

    #[test]
    fn a_failed_removal_still_takes_the_rest_and_says_why() {
        let control = std::env::temp_dir().join(format!("own-forget-{}", std::process::id()));
        let folder = control.join("pi-host-x.service.d");
        fs::create_dir_all(folder.join("50-CPUWeight.conf")).unwrap();
        fs::write(folder.join("50-MemoryMin.conf"), "[Service]\n").unwrap();
        let (taken, error) = forget(&control, "pi-host-x.service");
        let _ = fs::remove_dir_all(&control);
        assert_eq!(
            (taken, error.is_some()),
            (1, true),
            "a file that can't be removed leaves the others taken, and is said"
        );
    }

    #[test]
    fn only_inactive_counts_as_stopped_and_an_answer_short_of_one_a_unit_is_none() {
        use Active::{Failed, Runs, Stopped};
        assert_eq!(
            states("inactive\n", 1),
            Some(vec![Stopped]),
            "inactive is stopped"
        );
        assert_eq!(
            states("failed\n", 1),
            Some(vec![Failed]),
            "failed is kept apart, as systemd keeps it loaded"
        );
        for running in [
            "active\n",
            "activating\n",
            "deactivating\n",
            "reloading\n",
            "\n",
        ] {
            assert_eq!(
                states(running, 1),
                Some(vec![Runs]),
                "{running:?} runs, or may, so it keeps its settings"
            );
        }
        assert_eq!(
            states("inactive\nactive\nfailed\n", 3),
            Some(vec![Stopped, Runs, Failed]),
            "one state a line, in the order asked"
        );
        assert_eq!(
            states("", 2),
            None,
            "no answer, as with no bus, says nothing"
        );
        assert_eq!(
            states("inactive\n", 2),
            None,
            "nor does one short of a unit, which can't be lined up"
        );
    }

    #[test]
    fn is_active_takes_no_unit_for_an_option() {
        assert_eq!(
            is_active_call(&["-x.service".to_owned(), "y.service".to_owned()]),
            ["--user", "is-active", "--", "-x.service", "y.service"],
            "the units follow --, in the order given"
        );
    }

    #[test]
    fn an_instances_own_settings_are_its_templates() {
        assert_eq!(
            template("pi-conversation-work-background@0193-ab.service").as_deref(),
            Some("pi-conversation-work-background@.service"),
            "a conversation's unit is read as its template"
        );
        assert_eq!(
            template("pi-host-work-attended.service"),
            None,
            "a host is its own"
        );
    }
}
