//! Recovering a machine from a stick, its data left in place: the machine's
//! disk found by core's layout, opened with a YubiKey's PIN or the recovery
//! key, and mounted where a root shell inside it can repair anything. Each
//! repair the guide offers starts from here; the shell is the last resort,
//! and the first one built. Any stick does this, since it needs nothing
//! from the machine's own repository: the disk says what it is.

use crate::install::ROOT;
use crate::install::disk::described;
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

/// The machine's disk opened, by the YubiKey plugged in and its PIN or,
/// with none plugged in, the recovery key. The TPM isn't tried: booted
/// from a stick, the measurements it was sealed to differ.
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
            asks: vec![
                Ask::Act {
                    id: "key",
                    what: "Plug in a YubiKey that opens the disk, and touch it when it blinks. To use the recovery key instead, leave them out.".into(),
                },
                Ask::Secret {
                    id: "secret",
                    name: "the YubiKey's PIN, or with none plugged in, the recovery key".into(),
                },
            ],
        })
    }

    fn run(&self, machine: &mut dyn Machine, answers: &Answers) -> Result<(), Fault> {
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
        let opened = match crate::observe::fido2::list(machine)?.len() {
            0 => machine.run_given(
                Program::Tool(Tool::Cryptsetup),
                &["open", "--key-file=-", &volume, &found.name],
                Given {
                    stdin: Some(typed),
                    ..Given::default()
                },
            )?,
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

/// Everything the machine mounts, mounted under /mnt, its ESP too.
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
                    evidence: format!("the machine's system is mounted at {ROOT}"),
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
        let mounted: Vec<String> = found
            .disk
            .partitions
            .iter()
            .flat_map(|partition| partition.mounted.clone())
            .collect();
        for (subvolume, at) in SUBVOLUMES {
            let at = format!("{ROOT}{at}");
            if !mounted.contains(&at) {
                let options = format!("subvol={subvolume},{BTRFS},X-mount.mkdir");
                machine
                    .run(Tool::Mount, &["-o", &options, &mapped, &at])?
                    .ok()?;
            }
        }
        if !mounted.contains(&format!("{ROOT}/boot")) {
            let esp = format!("/dev/disk/by-partlabel/{}-boot", found.name);
            machine
                .run(
                    Tool::Mount,
                    &[
                        "-o",
                        "umask=0077,X-mount.mkdir",
                        &esp,
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

/// Every step of recovering, in order.
pub fn steps() -> Vec<Box<dyn Step>> {
    vec![
        Box::new(crate::install::vault::CheckKeyboard),
        Box::new(FindMachine),
        Box::new(OpenDisk),
        Box::new(MountSystem),
    ]
}
