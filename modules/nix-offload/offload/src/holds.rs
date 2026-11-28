//! Machines their owners hold: a file `held/<machine>` under nix-offload's
//! state folder says that no work goes to the machine it names until its
//! owner takes the file away. Its first line says why, and an empty file
//! holds it for no reason given. A machine is named as nix-offload knows
//! it: a machine nearby by its host or address as the nearby list gives
//! them, a builder standing or beside by its address or its machines
//! line's host, and a lent builder by its ssh alias or its full name.
//!
//! On 27 November at 21:19 a Mac's owner held it, and a `there --changed`
//! run without `--lent-only` sent it 32 checks anyway, as a machine
//! nearby; the Mac fell to 24% free memory, swapping, until its owner
//! killed the build. Nothing let an owner say "give this machine nothing",
//! so every caller had to remember `--lent-only`. A hold is read each time
//! machines are chosen, so it takes hold at once and needs no reload.

use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

/// The folder of holds under nix-offload's state folder.
const FOLDER: &str = "nix-offload/held";

/// The folder holds are read from: under XDG's state folder, beside
/// `runs.jsonl` and `journal.jsonl`, with the same override they take.
pub fn folder() -> PathBuf {
    crate::state().join(FOLDER)
}

/// A machine its owner holds, and why, where they said.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hold {
    pub machine: String,
    pub reason: Option<String>,
}

impl fmt::Display for Hold {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.reason {
            Some(reason) => write!(formatter, "{} is held ({reason})", self.machine),
            None => write!(formatter, "{} is held (no reason given)", self.machine),
        }
    }
}

/// Every hold in a folder of them, by the machine each names.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Holds {
    reasons: BTreeMap<String, Option<String>>,
}

impl Holds {
    /// The holds in `folder`: none when it isn't there, since no owner
    /// has held anything, and an error when it's there but can't be read.
    /// A file in it that can't be read still holds its machine, for no
    /// reason given, since its being there is the owner's word.
    pub fn read(folder: &Path) -> io::Result<Self> {
        let entries = match fs::read_dir(folder) {
            Ok(entries) => entries,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(error) => return Err(error),
        };
        let mut reasons = BTreeMap::new();
        for entry in entries {
            let entry = entry?;
            let Some(machine) = entry.file_name().to_str().map(str::to_owned) else {
                continue;
            };
            // A file being written, or one an editor left, names no machine.
            if machine.starts_with('.') {
                continue;
            }
            // A hold whose reason can't be read still holds its machine,
            // with no reason said, so an unreadable file holds rather than frees.
            let reason = fs::read_to_string(entry.path()).ok().and_then(|text| {
                let first = text.lines().next().unwrap_or_default().trim();
                (!first.is_empty()).then(|| first.to_owned())
            });
            reasons.insert(machine, reason);
        }
        Ok(Self { reasons })
    }

    /// The hold on a machine known by any of `names`, if its owner holds
    /// it, named as the hold names it.
    pub fn of<'a>(&self, names: impl IntoIterator<Item = &'a str>) -> Option<Hold> {
        names.into_iter().find_map(|name| {
            self.reasons.get(name).map(|reason| Hold {
                machine: name.to_owned(),
                reason: reason.clone(),
            })
        })
    }

    /// `machines` parted into those that may be given work and those
    /// their owners hold, each with its hold, as `names` says each is
    /// known.
    pub fn parted<T, N>(&self, machines: Vec<T>, names: N) -> (Vec<T>, Vec<(T, Hold)>)
    where
        N: Fn(&T) -> Vec<String>,
    {
        let mut free = Vec::new();
        let mut kept = Vec::new();
        for machine in machines {
            match self.of(names(&machine).iter().map(String::as_str)) {
                Some(why) => kept.push((machine, why)),
                None => free.push(machine),
            }
        }
        (free, kept)
    }
}

/// The holds in `folder` now. A folder that can't be read holds nothing,
/// since a hold must never stop work by accident, and says so once a
/// process, however often it's read, as a lender reads it every tick.
pub fn now(folder: &Path) -> Holds {
    static SAID: AtomicBool = AtomicBool::new(false);
    read_saying(folder, &SAID)
}

/// [`now`], saying an unreadable folder only while `said` is unset.
fn read_saying(folder: &Path, said: &AtomicBool) -> Holds {
    Holds::read(folder).unwrap_or_else(|error| {
        if !said.swap(true, Ordering::Relaxed) {
            eprintln!(
                "nix-offload: {} can't be read ({error}), so no machine is held",
                folder.display()
            );
        }
        Holds::default()
    })
}

/// The names a machines line's host is known by: the host as written,
/// and without the user it logs in as, so `builder@home` is held as
/// `home` too.
pub fn host_names(host: &str) -> Vec<String> {
    let mut names = vec![host.to_owned()];
    if let Some((_, bare)) = host.rsplit_once('@') {
        names.push(bare.to_owned());
    }
    names
}

/// The names an address is known by: as written, and without its port,
/// so `192.168.4.46:22` is held as `192.168.4.46` too.
pub fn address_names(address: &str) -> Vec<String> {
    let mut names = vec![address.to_owned()];
    if let Some((bare, port)) = address.rsplit_once(':')
        && port.parse::<u16>().is_ok()
    {
        names.push(bare.trim_matches(['[', ']']).to_owned());
    }
    names
}

#[cfg(test)]
mod tests {
    use super::*;

    fn holding(files: &[(&str, &str)]) -> tempfile::TempDir {
        let folder = tempfile::tempdir().expect("a folder");
        for (machine, text) in files {
            fs::write(folder.path().join(machine), text).expect("a hold");
        }
        folder
    }

    #[test]
    fn a_hold_says_its_first_line_as_why() {
        let folder = holding(&[("mac", "builds my app tonight\nsecond line\n")]);
        let holds = Holds::read(folder.path()).expect("the holds");
        let hold = holds.of(["mac.local", "mac"]).expect("the mac is held");
        assert_eq!(
            hold.to_string(),
            "mac is held (builds my app tonight)",
            "only the first line is the reason"
        );
    }

    #[test]
    fn an_empty_hold_holds_for_no_reason_given() {
        let folder = holding(&[("mac", "")]);
        let holds = Holds::read(folder.path()).expect("the holds");
        assert_eq!(
            holds.of(["mac"]).map(|hold| hold.to_string()).as_deref(),
            Some("mac is held (no reason given)")
        );
    }

    #[test]
    fn a_machine_no_file_names_is_free() {
        let folder = holding(&[("mac", "away")]);
        let holds = Holds::read(folder.path()).expect("the holds");
        assert_eq!(holds.of(["linux", "linux.local"]), None);
    }

    #[test]
    fn no_folder_holds_nothing_and_is_no_fault() {
        let folder = tempfile::tempdir().expect("a folder");
        let holds = Holds::read(&folder.path().join("held")).expect("no holds, said as none");
        assert_eq!(holds, Holds::default());
    }

    #[test]
    fn a_file_starting_with_a_dot_names_no_machine() {
        let folder = holding(&[(".mac.swp", "")]);
        let holds = Holds::read(folder.path()).expect("the holds");
        assert_eq!(holds.of([".mac.swp", "mac"]), None);
    }

    #[test]
    fn an_unreadable_folder_holds_nothing_and_says_so_once() {
        // A file where the folder should be can't be read as one, as a
        // folder whose mode refuses this account can't, and root reads
        // through any mode, which would leave the test proving nothing.
        let parent = tempfile::tempdir().expect("a folder");
        let folder = parent.path().join("held");
        fs::write(&folder, "not a folder").expect("a file in its place");
        assert!(Holds::read(&folder).is_err(), "it's read as unreadable");
        let said = AtomicBool::new(false);
        assert_eq!(read_saying(&folder, &said), Holds::default());
        assert!(said.load(Ordering::Relaxed), "it said so");
        assert_eq!(
            read_saying(&folder, &said),
            Holds::default(),
            "it holds nothing again"
        );
    }

    #[test]
    fn machines_are_parted_by_their_holds() {
        let folder = holding(&[("mac", "mine tonight")]);
        let holds = Holds::read(folder.path()).expect("the holds");
        let names = |machine: &&str| vec![(*machine).to_owned()];
        let (free, held) = holds.parted(vec!["linux", "mac"], names);
        assert_eq!(free, vec!["linux"]);
        assert_eq!(held.len(), 1);
        assert_eq!(held[0].1.to_string(), "mac is held (mine tonight)");
    }

    #[test]
    fn a_host_is_known_without_its_user_too() {
        assert_eq!(host_names("builder@home"), vec!["builder@home", "home"]);
        assert_eq!(host_names("mac"), vec!["mac"]);
    }

    #[test]
    fn an_address_is_known_without_its_port_too() {
        assert_eq!(
            address_names("192.168.4.46:22"),
            vec!["192.168.4.46:22", "192.168.4.46"]
        );
        assert_eq!(
            address_names("[fd00::2]:22"),
            vec!["[fd00::2]:22", "fd00::2"]
        );
        assert_eq!(address_names("mac.local"), vec!["mac.local"]);
    }
}
