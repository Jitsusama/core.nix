//! Recovering a machine from a stick, its data left in place: the machine's
//! disk found by core's layout, opened with a YubiKey's PIN or the recovery
//! key, and mounted where a root shell inside it can repair anything. Each
//! repair the guide offers starts from here; the shell is the last resort,
//! and the first one built. Any stick does this, since it needs nothing
//! from the machine's own repository: the disk says what it is. Each
//! repair past the shell has a file of its own: boot.rs for how the
//! machine boots, copy.rs for its home folders and keys.rs for a lost
//! YubiKey.

pub mod boot;
pub mod copy;
pub mod keys;

use std::collections::BTreeSet;

use crate::install::disk::described;
use crate::install::{BOOT_JOURNAL, ROOT};
use crate::journal::{self, Fact};
use crate::observe::disks::{self, Disk, Kind};
use crate::tool::Path;
use crate::{
    Answer, Answers, Ask, Fault, Given, Machine, Place, Program, Secret, Standing, Step, Tool,
};

/// A machine core laid out: its disk, and what its volume is called.
pub struct Found {
    pub disk: Disk,
    pub name: String,
}

impl Found {
    pub fn volume(&self) -> String {
        format!("/dev/disk/by-partlabel/{}", self.name)
    }

    pub fn mapped(&self) -> String {
        format!("/dev/mapper/{}", self.name)
    }
}

/// Every internal disk holding core's layout: an encrypted partition named
/// for the machine's volume, beside its `<name>-boot`.
pub fn machines(machine: &mut dyn Machine) -> Result<Vec<Found>, Fault> {
    let mut found = vec![];
    for disk in disks::observe(machine)? {
        if disk.kind == Kind::Removable {
            continue;
        }
        let labels: Vec<&str> = disk
            .partitions
            .iter()
            .filter_map(|partition| partition.label.as_deref())
            .collect();
        let named = disk
            .partitions
            .iter()
            .filter(|partition| partition.filesystem.as_deref() == Some("crypto_LUKS"))
            .filter_map(|partition| partition.label.clone())
            .find(|label| labels.contains(&format!("{label}-boot").as_str()));
        if let Some(name) = named {
            found.push(Found { disk, name });
        }
    }
    Ok(found)
}

/// The one machine on this computer's disks, or why there isn't one.
pub fn the_machine(machine: &mut dyn Machine) -> Result<Result<Found, Standing>, Fault> {
    let mut found = machines(machine)?;
    Ok(match found.len() {
        1 => Ok(found.remove(0)),
        0 => Err(Standing::Blocked {
            why: "no disk here holds a machine core laid out".into(),
            remedy: "boot the stick on the machine to recover".into(),
        }),
        many => Err(Standing::Blocked {
            why: format!("{many} disks here hold a machine core laid out"),
            remedy: "leave only the machine's own disk in".into(),
        }),
    })
}

/// What a machine's own stick offers when this computer's disk already
/// holds a machine: to recover it, or to install over it. The install's
/// own confirmation still asks for the name typed back before any wipe.
pub const ALREADY_THERE: &[&str] = &["Recover it", "Install it again, wiping the disk"];

/// What it offers when the disk holds this machine's install, stopped part
/// way: to carry it on, or to start again.
pub const STOPPED: &[&str] = &["Carry it on", "Start again, wiping the disk"];

/// What it offers when this machine's install stopped before any YubiKey
/// was enrolled: only to start again, since nothing here opens that disk.
pub const STOPPED_EARLY: &[&str] = &["Start again, wiping the disk"];

/// Where the stick reads a disk's boot partition, before anything on the
/// disk is opened.
const BOOT: &str = "/run/guide/boot";

/// What a disk here holds that a machine's own stick asks about before it
/// installs.
#[derive(Debug, PartialEq, Eq)]
pub enum AlreadyThere {
    /// A machine, to recover or install over.
    Machine { question: String },
    /// This machine's install, stopped part way, on this disk, with what its
    /// journal's copy knew.
    Stopped {
        question: String,
        disk: String,
        facts: BTreeSet<Fact>,
    },
    /// This machine's install, stopped before any YubiKey was enrolled, so
    /// only the install key it held in memory opened the disk, and that
    /// went with the restart.
    StoppedEarly { question: String },
}

/// What the copy of an install's journal on a disk's boot partition says,
/// when the partition holds one: read with the partition mounted read-only
/// and let go again, so nothing on the disk changes.
fn stopped_install(
    machine: &mut dyn Machine,
    found: &Found,
) -> Result<Option<BTreeSet<Fact>>, Fault> {
    let boot = format!("/dev/disk/by-partlabel/{}-boot", found.name);
    let mounted = machine.run(Tool::Mount, &["-o", "ro,X-mount.mkdir", &boot, BOOT])?;
    if mounted.status != 0 {
        return Ok(None);
    }
    let copy = format!("{BOOT}/{BOOT_JOURNAL}");
    let read = machine.read(&copy);
    machine.run(Tool::Umount, &[BOOT])?.ok()?;
    match read? {
        Some(bytes) => Ok(Some(journal::facts_in(
            &String::from_utf8_lossy(&bytes),
            &format!("the install journal on {}", described(&found.disk)),
        )?)),
        None => Ok(None),
    }
}

/// Whether a machine's own stick should ask before installing: when a disk
/// here already holds a machine or a stopped install, and the install
/// hasn't yet been confirmed onto it this boot, what to ask; otherwise
/// nothing, and the install goes on as it would. A stopped install is only
/// this machine's when its journal says the disk was laid out for it.
pub fn already_there(
    machine: &mut dyn Machine,
    installing: &str,
) -> Result<Option<AlreadyThere>, Fault> {
    let found = machines(machine)?;
    let confirmed = found
        .iter()
        .any(|found| machine.journal().knows("confirm-disk", &found.disk.path));
    let Some(found) = found.first().filter(|_| !confirmed) else {
        return Ok(None);
    };
    if let Some(facts) = stopped_install(machine, found)?
        && facts.contains(&("lay-out-disk".into(), installing.into()))
    {
        if !facts.iter().any(|(step, _)| step.starts_with("enrol-")) {
            return Ok(Some(AlreadyThere::StoppedEarly {
                question: format!(
                    "The install of {installing} onto {} stopped before a YubiKey was enrolled, so nothing can open what it wrote. Start again, wiping the disk?",
                    described(&found.disk)
                ),
            }));
        }
        return Ok(Some(AlreadyThere::Stopped {
            question: format!(
                "The install of {installing} onto {} stopped before it finished. Carry it on from where it stopped, or start again, wiping the disk?",
                described(&found.disk)
            ),
            disk: found.disk.path.clone(),
            facts,
        }));
    }
    Ok(Some(AlreadyThere::Machine {
        question: format!(
            "This computer's disk already holds a machine: {}. Recover it, or install {installing} on it again?",
            described(&found.disk)
        ),
    }))
}

/// The machine found, shown to the person before any secret is typed for
/// it, so they know which disk they're opening.
pub struct FindMachine;

impl Step for FindMachine {
    fn id(&self) -> &'static str {
        "find-machine"
    }

    fn place(&self) -> Place {
        Place::Stick
    }

    fn title(&self) -> &'static str {
        "Find the machine's disk"
    }

    fn observe(&self, machine: &mut dyn Machine) -> Result<Standing, Fault> {
        let found = match the_machine(machine)? {
            Ok(found) => found,
            Err(standing) => return Ok(standing),
        };
        let said = format!(
            "{} holds the machine, its volume named {}",
            described(&found.disk),
            found.name
        );
        Ok(if machine.journal().knows(self.id(), &found.disk.path) {
            Standing::Done { evidence: said }
        } else {
            Standing::Needed {
                asks: vec![Ask::Act {
                    id: "seen",
                    what: format!("{}.", capitalised(&said)),
                }],
            }
        })
    }

    fn run(&self, machine: &mut dyn Machine, _answers: &Answers) -> Result<(), Fault> {
        let found = the_machine(machine)?.map_err(|standing| Fault::Blocked {
            step: self.id(),
            why: standing.to_string(),
        })?;
        machine.journal().record(self.id(), &found.disk.path)
    }
}

/// The machine's disk opened, by the recovery key or the YubiKey plugged
/// in and its PIN, tried in that order so the recovery key typed with a
/// key plugged in never spends one of the key's PIN tries. The TPM isn't
/// tried: booted from a stick, the measurements it was sealed to differ.
pub struct OpenDisk;

impl Step for OpenDisk {
    fn id(&self) -> &'static str {
        "open-disk"
    }

    fn place(&self) -> Place {
        Place::Stick
    }

    fn title(&self) -> &'static str {
        "Open the machine's disk"
    }

    fn observe(&self, machine: &mut dyn Machine) -> Result<Standing, Fault> {
        let found = match the_machine(machine)? {
            Ok(found) => found,
            Err(standing) => return Ok(standing),
        };
        if machine.resolve(&found.mapped())?.is_some() {
            return Ok(Standing::Done {
                evidence: format!("the disk is open as {}", found.mapped()),
            });
        }
        Ok(Standing::Needed {
            asks: vec![Ask::SecretOr {
                id: "secret",
                name: "the disk's recovery key, the seven words in 1Password on your phone".into(),
                or: "plug in a YubiKey that opens the disk, type its PIN and press Return, then touch the key when it blinks".into(),
            }],
        })
    }

    fn run(&self, machine: &mut dyn Machine, answers: &Answers) -> Result<(), Fault> {
        // cryptsetup's status when no slot takes what it was given
        // (lib/libcryptsetup.h, EPERM): anything else is the disk refusing.
        const NO_SLOT_TAKES_IT: i32 = 2;
        let Some(Answer::Secret(typed)) = answers.get("secret") else {
            return Err(Fault::Unanswered {
                step: self.id(),
                ask: "secret",
            });
        };
        let found = the_machine(machine)?.map_err(|standing| Fault::Blocked {
            step: self.id(),
            why: standing.to_string(),
        })?;
        let volume = found.volume();
        let line = Secret::new(format!("{}\n", typed.expose()));
        // The recovery key, read as the words it says, so a doubled space
        // or a capital doesn't count against it.
        let recovery = Secret::new(crate::words::typed(typed.expose()));
        let by_recovery_key = machine.run_given(
            Program::Tool(Tool::Cryptsetup),
            &["open", "--key-file=-", &volume, &found.name],
            Given {
                stdin: Some(&recovery),
                ..Given::default()
            },
        )?;
        if by_recovery_key.status != NO_SLOT_TAKES_IT {
            return match by_recovery_key.status {
                0 => Ok(()),
                _ => Err(by_recovery_key.refusal()),
            };
        }
        let opened = match crate::observe::fido2::plugged(machine)?.len() {
            0 => return Err(by_recovery_key.refusal()),
            1 => machine.run_given(
                Program::Tool(Tool::Cryptsetup),
                &[
                    "open",
                    "--token-only",
                    "--token-type",
                    "systemd-fido2",
                    &volume,
                    &found.name,
                ],
                Given {
                    plain: vec![("LD_LIBRARY_PATH", Path::Tokens.get()?)],
                    stdin: Some(&line),
                    ..Given::default()
                },
            )?,
            many => {
                return Err(Fault::Blocked {
                    step: self.id(),
                    why: format!("{many} keys are plugged in; leave only one"),
                });
            }
        };
        if opened.status == 0 {
            Ok(())
        } else {
            Err(opened.refusal())
        }
    }
}

/// Where each of core's subvolumes mounts under the root, as its layout
/// mounts them (../../disko/nixos.nix).
const SUBVOLUMES: &[(&str, &str)] = &[
    ("@", ""),
    ("@nix", "/nix"),
    ("@home", "/home"),
    ("@log", "/var/log"),
];
const BTRFS: &str = "compress=zstd:1,noatime,discard=async";

/// What a filesystem with errors mounts with: read-only, and every rescue
/// btrfs has for reading past damage, so nothing on it changes before
/// what matters is copied off.
const DAMAGED: &str = "ro,rescue=all";

/// What the check found, kept until the stick restarts: `clean`, or what
/// `btrfs check` said of the errors.
pub const CHECKED: &str = "/run/guide/checked";
const CLEAN: &str = "clean";

/// Whether the check found the machine's filesystem damaged.
fn damaged(machine: &mut dyn Machine) -> Result<bool, Fault> {
    Ok(machine
        .read(CHECKED)?
        .is_some_and(|kept| kept != CLEAN.as_bytes()))
}

/// The machine's filesystem checked before anything mounts it, changing
/// nothing, so one with errors is mounted only to read. Mounted to write,
/// a damaged filesystem can lose more of what's on it.
pub struct CheckSystem;

impl Step for CheckSystem {
    fn id(&self) -> &'static str {
        "check-system"
    }

    fn place(&self) -> Place {
        Place::Stick
    }

    fn title(&self) -> &'static str {
        "Check the machine's filesystem"
    }

    fn observe(&self, machine: &mut dyn Machine) -> Result<Standing, Fault> {
        let found = match the_machine(machine)? {
            Ok(found) => found,
            Err(standing) => return Ok(standing),
        };
        if machine.read(CHECKED)?.is_some() {
            return Ok(Standing::Done {
                evidence: if damaged(machine)? {
                    format!(
                        "its filesystem has errors, so it's mounted only to read: copy off what matters before any repair. What the check said is in {CHECKED}"
                    )
                } else {
                    "its filesystem has no errors".into()
                },
            });
        }
        let mounted = found
            .disk
            .partitions
            .iter()
            .any(|partition| partition.mounted.contains(&ROOT.to_string()));
        Ok(if mounted {
            Standing::Done {
                evidence: format!(
                    "its filesystem was mounted at {ROOT} before it could be checked"
                ),
            }
        } else {
            Standing::Needed { asks: vec![] }
        })
    }

    fn run(&self, machine: &mut dyn Machine, _answers: &Answers) -> Result<(), Fault> {
        let found = the_machine(machine)?.map_err(|standing| Fault::Blocked {
            step: self.id(),
            why: standing.to_string(),
        })?;
        let checked = machine.run(Tool::Btrfs, &["check", "--readonly", &found.mapped()])?;
        let kept = if checked.status == 0 {
            CLEAN.as_bytes().to_vec()
        } else {
            [checked.stdout.as_slice(), checked.stderr.as_slice()].concat()
        };
        machine.write(CHECKED, &kept)
    }
}

/// Everything the machine mounts, mounted under /mnt, its boot partition too.
pub struct MountSystem;

impl Step for MountSystem {
    fn id(&self) -> &'static str {
        "mount-system"
    }

    fn place(&self) -> Place {
        Place::Stick
    }

    fn title(&self) -> &'static str {
        "Mount the machine's system"
    }

    fn observe(&self, machine: &mut dyn Machine) -> Result<Standing, Fault> {
        let found = match the_machine(machine)? {
            Ok(found) => found,
            Err(standing) => return Ok(standing),
        };
        let mounted: Vec<String> = found
            .disk
            .partitions
            .iter()
            .flat_map(|partition| partition.mounted.clone())
            .collect();
        let all = SUBVOLUMES
            .iter()
            .map(|(_, at)| format!("{ROOT}{at}"))
            .chain(std::iter::once(format!("{ROOT}/boot")))
            .all(|at| mounted.contains(&at));
        Ok(
            if all && machine.read(&format!("{ROOT}/etc/NIXOS"))?.is_some() {
                Standing::Done {
                    evidence: if damaged(machine)? {
                        format!("the machine's system is mounted at {ROOT}, only to read")
                    } else {
                        format!("the machine's system is mounted at {ROOT}")
                    },
                }
            } else {
                Standing::Needed { asks: vec![] }
            },
        )
    }

    fn run(&self, machine: &mut dyn Machine, _answers: &Answers) -> Result<(), Fault> {
        let found = the_machine(machine)?.map_err(|standing| Fault::Blocked {
            step: self.id(),
            why: standing.to_string(),
        })?;
        let mapped = found.mapped();
        let damaged = damaged(machine)?;
        let mounted: Vec<String> = found
            .disk
            .partitions
            .iter()
            .flat_map(|partition| partition.mounted.clone())
            .collect();
        for (subvolume, at) in SUBVOLUMES {
            let at = format!("{ROOT}{at}");
            if !mounted.contains(&at) {
                let mode = if damaged { DAMAGED } else { BTRFS };
                let options = format!("subvol={subvolume},{mode},X-mount.mkdir");
                machine
                    .run(Tool::Mount, &["-o", &options, &mapped, &at])?
                    .ok()?;
            }
        }
        if !mounted.contains(&format!("{ROOT}/boot")) {
            let boot_partition = format!("/dev/disk/by-partlabel/{}-boot", found.name);
            machine
                .run(
                    Tool::Mount,
                    &[
                        "-o",
                        "umask=0077,X-mount.mkdir",
                        &boot_partition,
                        &format!("{ROOT}/boot"),
                    ],
                )?
                .ok()?;
        }
        Ok(())
    }
}

fn capitalised(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// The repairs a person chooses from once the machine's disk is open,
/// with what each says, in the order they're offered.
pub const REPAIRS: &[&str] = &[
    "Open a root shell inside it",
    "Copy its home folders onto a drive",
    "Choose which generation it boots next",
    "Replace a lost YubiKey",
    "Install its boot loader again",
    "Restart",
];

/// Every step of recovering, in order.
pub fn steps() -> Vec<Box<dyn Step>> {
    vec![
        Box::new(crate::install::memory::RunFromMemory),
        Box::new(crate::install::vault::CheckKeyboard),
        Box::new(FindMachine),
        Box::new(OpenDisk),
        Box::new(CheckSystem),
        Box::new(MountSystem),
    ]
}
