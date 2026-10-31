//! The disk: confirmed by the machine's name typed back, laid out by the
//! machine's own disko script with a key held only in memory, and, last,
//! left with only the YubiKeys and the recovery key to open it. An install
//! a restart stopped part way is opened as it was left, by a YubiKey it
//! enrolled, rather than wiped.

use std::fmt::Write as _;

use crate::install::keys::slug;
use crate::install::{
    INSTALL_KEY, LAYOUT, LAYOUT_SCRIPT, MOUNT, ROOT, Target, boot_journal, target, vault,
};
use crate::observe::disks::{self, Disk, Kind};
use crate::observe::{fido2, luks};
use crate::tool::{self, Path};
use crate::{
    Answer, Answers, Ask, Fault, Given, Machine, Place, Program, Secret, Standing, Step, Tool,
};

fn blocked(why: String, remedy: String) -> Standing {
    Standing::Blocked { why, remedy }
}

/// What a person reads a disk by: its model, size and serial.
pub fn described(disk: &Disk) -> String {
    let mut said = format!(
        "{}, {} GB",
        disk.model.as_deref().unwrap_or("a disk with no model name"),
        disk.bytes / 1_000_000_000
    );
    if let Some(serial) = &disk.serial {
        // Writing to a String can't fail.
        _ = write!(said, ", serial {serial}");
    }
    said
}

/// The target's disk, or why it can't be the one installed onto: not
/// there, removable, or holding something mounted that the install
/// didn't mount.
pub fn found(machine: &mut dyn Machine, target: &Target) -> Result<Result<Disk, Standing>, Fault> {
    let Some(path) = machine.resolve(&target.disk)? else {
        return Ok(Err(blocked(
            format!(
                "the disk {} installs onto, {}, isn't in this machine",
                target.machine, target.disk
            ),
            format!(
                "boot this stick on {}, which it was written for",
                target.machine
            ),
        )));
    };
    let Some(disk) = disks::observe(machine)?
        .into_iter()
        .find(|disk| disk.path == path)
    else {
        return Ok(Err(blocked(
            format!("{path} isn't a whole disk"),
            "write the stick again with the machine's disk named by its id".into(),
        )));
    };
    if disk.kind == Kind::Removable {
        return Ok(Err(blocked(
            format!(
                "{} is removable, and a machine is never installed onto one",
                described(&disk)
            ),
            "write the stick again naming the machine's own disk".into(),
        )));
    }
    let elsewhere = disk
        .partitions
        .iter()
        .flat_map(|partition| &partition.mounted)
        .find(|mount| *mount != ROOT && !mount.starts_with(&format!("{ROOT}/")));
    if let Some(mount) = elsewhere {
        return Ok(Err(blocked(
            format!("{mount} on {} is in use", described(&disk)),
            "restart from the stick, so nothing on the disk is mounted".into(),
        )));
    }
    Ok(Ok(disk))
}

/// The disk shown by what it is, and the machine's name typed back, since
/// laying it out wipes everything on it.
pub struct ConfirmDisk;

impl Step for ConfirmDisk {
    fn id(&self) -> &'static str {
        "confirm-disk"
    }

    fn place(&self) -> Place {
        Place::Stick
    }

    fn title(&self) -> &'static str {
        "Confirm the disk to wipe"
    }

    fn observe(&self, machine: &mut dyn Machine) -> Result<Standing, Fault> {
        let target = target(machine)?;
        let disk = match found(machine, &target)? {
            Ok(disk) => disk,
            Err(standing) => return Ok(standing),
        };
        Ok(if machine.journal().knows(self.id(), &disk.path) {
            Standing::Done {
                evidence: format!("{} is {}'s, to wipe", described(&disk), target.machine),
            }
        } else {
            Standing::Needed {
                asks: vec![Ask::TypeBack {
                    id: "machine",
                    expected: target.machine.clone(),
                    why: format!("installing wipes {} and everything on it", described(&disk)),
                }],
            }
        })
    }

    fn run(&self, machine: &mut dyn Machine, answers: &Answers) -> Result<(), Fault> {
        let target = target(machine)?;
        let disk = found(machine, &target)?.map_err(|standing| Fault::Blocked {
            step: self.id(),
            why: standing.to_string(),
        })?;
        match answers.get("machine") {
            Some(Answer::Typed(typed)) if typed.trim() == target.machine => {
                machine.journal().record(self.id(), &disk.path)
            }
            Some(Answer::Typed(typed)) => Err(Fault::Blocked {
                step: self.id(),
                why: format!(
                    "\"{}\" isn't {}, so nothing was wiped",
                    typed.trim(),
                    target.machine
                ),
            }),
            _ => Err(Fault::Unanswered {
                step: self.id(),
                ask: "machine",
            }),
        }
    }
}

/// Whether the target's volume is laid out on its disk and open at the
/// install's root, which stays true until the install is done. Only the
/// journal tells a layout of this install's from an old system left open
/// there, by Recover or a carry, which must still be wiped; a start that
/// carries on after a restart learns the fact from the journal's copy.
fn laid_out(machine: &mut dyn Machine, target: &Target, disk: &Disk) -> Result<bool, Fault> {
    if !machine.journal().knows("lay-out-disk", &target.machine) {
        return Ok(false);
    }
    let Some(volume) = machine.resolve(&target.volume())? else {
        return Ok(false);
    };
    let on_disk = disk
        .partitions
        .iter()
        .any(|partition| partition.path == volume);
    let mounted = disk
        .partitions
        .iter()
        .any(|partition| partition.mounted.iter().any(|mount| mount == ROOT));
    Ok(on_disk
        && mounted
        && machine.resolve(&target.mapped())?.is_some()
        && machine.run(Tool::Cryptsetup, &["isLuks", &volume])?.status == 0)
}

/// A key of 32 random bytes, as hex, for the disk until the YubiKeys and
/// the recovery key are enrolled.
fn new_key() -> Result<String, Fault> {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).map_err(|why| Fault::unreadable("the system's randomness", why))?;
    Ok(bytes.iter().fold(String::new(), |mut hex, byte| {
        // Writing to a String can't fail.
        _ = write!(hex, "{byte:02x}");
        hex
    }))
}

/// The disk wiped and laid out by the machine's own disko script, its
/// encrypted volume made with a key held only in memory and mounted at
/// the install's root.
pub struct LayOutDisk;

impl Step for LayOutDisk {
    fn id(&self) -> &'static str {
        "lay-out-disk"
    }

    fn place(&self) -> Place {
        Place::Stick
    }

    fn title(&self) -> &'static str {
        "Wipe the disk and lay it out, encrypted"
    }

    fn observe(&self, machine: &mut dyn Machine) -> Result<Standing, Fault> {
        let target = target(machine)?;
        let disk = match found(machine, &target)? {
            Ok(disk) => disk,
            Err(standing) => return Ok(standing),
        };
        if laid_out(machine, &target, &disk)? {
            return Ok(Standing::Done {
                evidence: format!(
                    "{} holds {}'s volume, open at {ROOT}",
                    described(&disk),
                    target.machine
                ),
            });
        }
        if target.install_key != INSTALL_KEY {
            return Ok(blocked(
                format!(
                    "the layout of {} reads its first key from {}, not {INSTALL_KEY}",
                    target.machine, target.install_key
                ),
                format!("set jitsusama.disk.installKey to {INSTALL_KEY} and write the stick again"),
            ));
        }
        let have = tool::disk_packages();
        let missing: Vec<&str> = target
            .layout_packages
            .iter()
            .map(String::as_str)
            .filter(|package| !have.contains(package))
            .collect();
        if !missing.is_empty() {
            return Ok(blocked(
                format!(
                    "the layout of {} needs {}, which this stick hasn't got",
                    target.machine,
                    missing.join(", ")
                ),
                "write the stick again from a guide whose disk tools hold them".into(),
            ));
        }
        Ok(Standing::Needed { asks: vec![] })
    }

    fn run(&self, machine: &mut dyn Machine, _answers: &Answers) -> Result<(), Fault> {
        let target = target(machine)?;
        let disk = found(machine, &target)?.map_err(|standing| Fault::Blocked {
            step: self.id(),
            why: standing.to_string(),
        })?;
        if !machine.journal().knows("confirm-disk", &disk.path) {
            return Err(Fault::Blocked {
                step: self.id(),
                why: format!("{} hasn't been confirmed", described(&disk)),
            });
        }
        // An earlier start's volume, still open, goes first; neither being
        // there is no fault.
        machine.run(Tool::Umount, &["--recursive", ROOT])?;
        if machine.resolve(&target.mapped())?.is_some() {
            machine
                .run(Tool::Cryptsetup, &["close", &target.disk_name])?
                .ok()?;
        }

        let mut script = vec![];
        vault(machine)?.extract(LAYOUT_SCRIPT, &mut script)?;
        machine.write(LAYOUT, &script)?;
        machine.write(INSTALL_KEY, new_key()?.as_bytes())?;

        for partition in &disk.partitions {
            machine
                .run(Tool::Wipefs, &["--all", &partition.path])?
                .ok()?;
        }
        machine.run(Tool::Wipefs, &["--all", &disk.path])?.ok()?;
        machine
            .run(Tool::Sgdisk, &["--zap-all", &disk.path])?
            .ok()?;
        machine.run(Tool::Udevadm, &["settle"])?.ok()?;

        let ran = machine.run_given(
            Program::Tool(Tool::Bash),
            &[LAYOUT],
            Given {
                plain: vec![("PATH", Path::Disk.get()?)],
                ..Given::default()
            },
        )?;
        if ran.status != 0 {
            return Err(ran.refusal());
        }
        machine.run(Tool::Udevadm, &["settle"])?.ok()?;
        // Which machine the disk is laid out for, and from here on a copy
        // of the journal on its boot partition, so a start after a restart
        // can carry the install on.
        machine.journal().record(self.id(), &target.machine)?;
        machine
            .journal()
            .keep_a_copy(std::path::Path::new(&boot_journal()))
    }
}

/// An install a restart stopped part way, its disk opened as it was left:
/// a YubiKey it enrolled gives the disk a new install key, the first one
/// having been only in the stick's memory, then the machine's own disko
/// script opens and mounts it, and the journal's copy goes on.
pub struct OpenStoppedInstall {
    /// The YubiKeys the target names, any of which may have been enrolled.
    pub yubikeys: Vec<String>,
}

impl OpenStoppedInstall {
    /// The names of the keys the stopped install proved open the disk, or
    /// every name when its journal says none.
    fn enrolled(&self, machine: &mut dyn Machine, credentials: &[String]) -> Vec<String> {
        let proven: Vec<String> = self
            .yubikeys
            .iter()
            .filter(|name| {
                let step = format!("enrol-{}", slug(name));
                credentials.iter().any(|credential| {
                    machine
                        .journal()
                        .knows(&step, &format!("opened:{credential}"))
                })
            })
            .cloned()
            .collect();
        if proven.is_empty() {
            self.yubikeys.clone()
        } else {
            proven
        }
    }
}

impl Step for OpenStoppedInstall {
    fn id(&self) -> &'static str {
        "open-stopped-install"
    }

    fn place(&self) -> Place {
        Place::Stick
    }

    fn title(&self) -> &'static str {
        "Open the install that stopped"
    }

    fn observe(&self, machine: &mut dyn Machine) -> Result<Standing, Fault> {
        let target = target(machine)?;
        let disk = match found(machine, &target)? {
            Ok(disk) => disk,
            Err(standing) => return Ok(standing),
        };
        if laid_out(machine, &target, &disk)? {
            return Ok(Standing::Done {
                evidence: format!(
                    "{} holds {}'s volume, open at {ROOT}",
                    described(&disk),
                    target.machine
                ),
            });
        }
        let credentials: Vec<String> = luks::observe(machine, &target.volume())?
            .credentials()
            .into_iter()
            .map(String::from)
            .collect();
        if credentials.is_empty() {
            return Ok(blocked(
                "no YubiKey was enrolled before the install stopped, so nothing here opens its disk"
                    .into(),
                "start the stick again and choose to install it again, which wipes the disk".into(),
            ));
        }
        let names = self.enrolled(machine, &credentials);
        let either = names.join(" or ");
        let port = crate::install::memory::port_words(machine);
        Ok(Standing::Needed {
            asks: vec![
                Ask::Act {
                    id: "plugged",
                    what: format!(
                        "Plug in {either}, which opened the disk before the install stopped, and only that key.{port} Touch it when it blinks."
                    ),
                },
                Ask::Secret {
                    id: "pin",
                    name: match names.as_slice() {
                        [one] => format!("{one}'s PIN"),
                        _ => "its PIN".into(),
                    },
                },
            ],
        })
    }

    fn run(&self, machine: &mut dyn Machine, answers: &Answers) -> Result<(), Fault> {
        let Some(Answer::Secret(pin)) = answers.get("pin") else {
            return Err(Fault::Unanswered {
                step: self.id(),
                ask: "pin",
            });
        };
        let target = target(machine)?;
        let volume = target.volume();
        let device = match fido2::plugged(machine)?.as_slice() {
            [one] => one.path.clone(),
            [] => {
                return Err(Fault::Blocked {
                    step: self.id(),
                    why: "no key is plugged in; plug in one that opened the disk".into(),
                });
            }
            many => {
                return Err(Fault::Blocked {
                    step: self.id(),
                    why: format!("{} keys are plugged in; leave only one", many.len()),
                });
            }
        };

        // The script that opens the disk, read first, so nothing on the
        // disk changes without it; held since the vault opened, so the
        // stick can be out for the key.
        let script = crate::install::mount_script(machine)?;

        // The new install key held before it's enrolled, as the first was,
        // and the slot the first one opened wiped, since nobody has it.
        let key = Secret::new(new_key()?);
        machine.write(INSTALL_KEY, key.expose().as_bytes())?;
        let enrolled = machine.run_given(
            Program::Tool(Tool::SystemdCryptenroll),
            &[
                &format!("--unlock-fido2-device={device}"),
                "--wipe-slot=password",
                "--password",
                &volume,
            ],
            Given {
                env: vec![("NEWPASSWORD", &key)],
                // systemd 261.3 reads the PIN that unlocks with a FIDO2
                // key as cryptsetup.fido2-pin, whatever name cryptenroll
                // hands it, so it goes under both, as the TPM's seal does.
                credentials: vec![
                    ("cryptsetup.fido2-pin", pin),
                    ("cryptenroll.fido2-pin", pin),
                ],
                ..Given::default()
            },
        )?;
        if enrolled.status != 0 {
            return Err(enrolled.refusal());
        }

        machine.write(MOUNT, &script)?;
        let ran = machine.run_given(
            Program::Tool(Tool::Bash),
            &[MOUNT],
            Given {
                plain: vec![("PATH", Path::Disk.get()?)],
                ..Given::default()
            },
        )?;
        if ran.status != 0 {
            return Err(ran.refusal());
        }
        machine.run(Tool::Udevadm, &["settle"])?.ok()?;
        machine
            .journal()
            .keep_a_copy(std::path::Path::new(&boot_journal()))
    }
}

/// The disk's first key taken off it, so only the YubiKeys and the
/// recovery key open it, and forgotten.
pub struct WipeInstallKey {
    /// How many YubiKeys were enrolled, each of which must be there first.
    pub yubikeys: usize,
}

impl Step for WipeInstallKey {
    fn id(&self) -> &'static str {
        "wipe-install-key"
    }

    fn place(&self) -> Place {
        Place::Stick
    }

    fn title(&self) -> &'static str {
        "Take the install's key off the disk"
    }

    fn observe(&self, machine: &mut dyn Machine) -> Result<Standing, Fault> {
        let target = target(machine)?;
        let header = luks::observe(machine, &target.volume())?;
        let keys = header.credentials().len();
        let recovery = header.has(&luks::Kind::Recovery);
        if keys < self.yubikeys || !recovery {
            return Ok(blocked(
                format!(
                    "only {keys} YubiKeys{} open the disk",
                    if recovery {
                        " and the recovery key"
                    } else {
                        ""
                    }
                ),
                "enrol each YubiKey and the recovery key first".into(),
            ));
        }
        Ok(
            if header.bare().is_empty()
                && machine.read(INSTALL_KEY)?.is_none()
                && machine.journal().copy().is_none()
            {
                Standing::Done {
                    evidence: format!("only {keys} YubiKeys and the recovery key open the disk"),
                }
            } else {
                Standing::Needed { asks: vec![] }
            },
        )
    }

    fn run(&self, machine: &mut dyn Machine, _answers: &Answers) -> Result<(), Fault> {
        let target = target(machine)?;
        if !luks::observe(machine, &target.volume())?.bare().is_empty() {
            machine
                .run(
                    Tool::SystemdCryptenroll,
                    &[
                        &format!("--unlock-key-file={INSTALL_KEY}"),
                        "--wipe-slot=password",
                        &target.volume(),
                    ],
                )?
                .ok()?;
        }
        // With the key off, nothing is left to carry on, so the journal's
        // copy goes before the key is forgotten: a start that finds the
        // copy finds an install whose key may still be on the disk.
        machine.journal().drop_copy()?;
        machine.remove(INSTALL_KEY)
    }
}
