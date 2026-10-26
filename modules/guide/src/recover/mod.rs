//! Recovering a machine from a stick, its data left in place: the machine's
//! disk found by core's layout, opened with a YubiKey's PIN or the recovery
//! key, and mounted where a root shell inside it can repair anything. Each
//! repair the guide offers starts from here; the shell is the last resort,
//! and the first one built. Any stick does this, since it needs nothing
//! from the machine's own repository: the disk says what it is.

use std::collections::BTreeSet;

use crate::install::disk::described;
use crate::install::{BOOT_JOURNAL, ROOT};
use crate::journal::{self, Fact};
use crate::observe::disks::{self, Disk, Kind};
use crate::observe::luks;
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
    "Choose which generation it boots next",
    "Replace a lost YubiKey",
    "Install its boot loader again",
    "Restart",
];

/// The boot loader the machine installs, which a damaged boot partition lacks.
const BOOT_LOADER: &str = "/mnt/boot/EFI/systemd/systemd-bootx64.efi";

/// The boot loader and the generations' boot files written to the boot partition
/// again by the installed system itself, as it writes them whenever it
/// switches, so lanzaboote signs them with the machine's own keys on its
/// disk. For a boot partition that was wiped, damaged or filled.
pub struct InstallBootLoader;

impl Step for InstallBootLoader {
    fn id(&self) -> &'static str {
        "install-boot-loader"
    }

    fn place(&self) -> Place {
        Place::Stick
    }

    fn title(&self) -> &'static str {
        "Install its boot loader again"
    }

    fn observe(&self, machine: &mut dyn Machine) -> Result<Standing, Fault> {
        Ok(
            if machine.journal().knows(self.id(), "installed")
                && machine.read(BOOT_LOADER)?.is_some()
            {
                Standing::Done {
                    evidence: "its boot loader is installed again".into(),
                }
            } else {
                Standing::Needed { asks: vec![] }
            },
        )
    }

    fn run(&self, machine: &mut dyn Machine, _answers: &Answers) -> Result<(), Fault> {
        let installed = machine.run_given(
            Program::Tool(Tool::NixosEnter),
            &[
                "--root",
                crate::install::ROOT,
                "-c",
                "NIXOS_INSTALL_BOOTLOADER=1 /nix/var/nix/profiles/system/bin/switch-to-configuration boot",
            ],
            Given {
                plain: vec![("PATH", Path::Install.get()?)],
                ..Given::default()
            },
        )?;
        if installed.status != 0 {
            return Err(installed.refusal());
        }
        machine.journal().record(self.id(), "installed")
    }
}

/// The YubiKey the person still has, told apart from the disk's others by
/// a silent assertion, so the ones it isn't are the lost ones.
pub struct IdentifyKeptKey;

fn credentials(machine: &mut dyn Machine) -> Result<(Found, Vec<String>, luks::Header), Fault> {
    let found = the_machine(machine)?.map_err(|standing| Fault::Blocked {
        step: "replacing a YubiKey",
        why: standing.to_string(),
    })?;
    let header = luks::observe(machine, &found.volume())?;
    let credentials = header.credentials().into_iter().map(String::from).collect();
    Ok((found, credentials, header))
}

/// The one key plugged in, or why there isn't one.
fn plugged(machine: &mut dyn Machine, step: &'static str) -> Result<String, Fault> {
    let plugged = crate::observe::fido2::plugged(machine)?;
    match plugged.as_slice() {
        [one] => Ok(one.path.clone()),
        [] => Err(Fault::Blocked {
            step,
            why: "no key is plugged in".into(),
        }),
        many => Err(Fault::Blocked {
            step,
            why: format!("{} keys are plugged in; leave only one", many.len()),
        }),
    }
}

impl Step for IdentifyKeptKey {
    fn id(&self) -> &'static str {
        "identify-kept-key"
    }

    fn place(&self) -> Place {
        Place::Stick
    }

    fn title(&self) -> &'static str {
        "Find the YubiKey you still have"
    }

    fn observe(&self, machine: &mut dyn Machine) -> Result<Standing, Fault> {
        let (_, credentials, _) = credentials(machine)?;
        if credentials.is_empty() {
            return Ok(Standing::Blocked {
                why: "no YubiKey opens this disk".into(),
                remedy: "open a root shell inside it and enrol one with systemd-cryptenroll".into(),
            });
        }
        if credentials
            .iter()
            .any(|credential| machine.journal().knows(self.id(), credential))
        {
            return Ok(Standing::Done {
                evidence: "the YubiKey you still have is known".into(),
            });
        }
        Ok(Standing::Needed {
            asks: vec![Ask::Act {
                id: "kept",
                what: format!(
                    "Plug in the YubiKey you still have, and only it.{} Nothing is asked of it.",
                    crate::install::memory::port_words(machine)
                ),
            }],
        })
    }

    fn run(&self, machine: &mut dyn Machine, _answers: &Answers) -> Result<(), Fault> {
        let (_, credentials, _) = credentials(machine)?;
        let device = plugged(machine, self.id())?;
        for credential in &credentials {
            if crate::install::keys::holds(machine, &device, credential)? {
                return machine.journal().record(self.id(), credential);
            }
        }
        Err(Fault::Blocked {
            step: self.id(),
            why: "this key doesn't open the disk".into(),
        })
    }
}

/// A new YubiKey enrolled and proven, then the lost ones' slots wiped, so
/// the disk never has fewer ways in than when it started.
pub struct ReplaceLostKey;

impl Step for ReplaceLostKey {
    fn id(&self) -> &'static str {
        "replace-lost-key"
    }

    fn place(&self) -> Place {
        Place::Stick
    }

    fn title(&self) -> &'static str {
        "Replace the lost YubiKey"
    }

    fn observe(&self, machine: &mut dyn Machine) -> Result<Standing, Fault> {
        let (_, credentials, _) = credentials(machine)?;
        let kept = credentials
            .iter()
            .find(|credential| machine.journal().knows("identify-kept-key", credential))
            .cloned();
        let added = credentials
            .iter()
            .find(|credential| machine.journal().knows(self.id(), credential))
            .cloned();
        if let (Some(kept), Some(added)) = (&kept, &added)
            && credentials
                .iter()
                .all(|credential| credential == kept || credential == added)
        {
            return Ok(Standing::Done {
                evidence: "the new YubiKey opens the disk, and the lost one no longer does".into(),
            });
        }
        if kept.is_none() {
            return Ok(Standing::Blocked {
                why: "the YubiKey you still have isn't known yet".into(),
                remedy: "plug it in and choose this repair again".into(),
            });
        }
        Ok(Standing::Needed {
            asks: vec![
                Ask::Act {
                    id: "new",
                    what: "Unplug it, and plug in the new YubiKey. Touch it each time it blinks."
                        .into(),
                },
                Ask::Secret {
                    id: "pin",
                    name: "the new YubiKey's PIN".into(),
                },
                Ask::Secret {
                    id: "recovery",
                    name: "the recovery key".into(),
                },
            ],
        })
    }

    fn run(&self, machine: &mut dyn Machine, answers: &Answers) -> Result<(), Fault> {
        let (Some(Answer::Secret(pin)), Some(Answer::Secret(typed))) =
            (answers.get("pin"), answers.get("recovery"))
        else {
            return Err(Fault::Unanswered {
                step: self.id(),
                ask: "pin",
            });
        };
        let recovery = &Secret::new(crate::words::typed(typed.expose()));
        let (found, before, header) = credentials(machine)?;
        let volume = found.volume();
        let kept = before
            .iter()
            .find(|credential| machine.journal().knows("identify-kept-key", credential))
            .cloned()
            .ok_or(Fault::Blocked {
                step: self.id(),
                why: "the YubiKey you still have isn't known".into(),
            })?;
        let device = plugged(machine, self.id())?;
        for credential in &before {
            if crate::install::keys::holds(machine, &device, credential)? {
                return Err(Fault::Blocked {
                    step: self.id(),
                    why: "this key opens the disk already, so it isn't a new one".into(),
                });
            }
        }

        // The new key first, proven, and only then the lost ones away.
        let enrolled = machine.run_given(
            Program::Tool(Tool::SystemdCryptenroll),
            &[
                &format!("--fido2-device={device}"),
                "--fido2-with-client-pin=yes",
                "--fido2-with-user-presence=yes",
                &volume,
            ],
            Given {
                env: vec![("PASSWORD", recovery)],
                credentials: vec![("cryptenroll.fido2-pin", pin)],
                ..Given::default()
            },
        )?;
        if enrolled.status != 0 {
            return Err(enrolled.refusal());
        }
        let after = luks::observe(machine, &volume)?;
        let new: Vec<String> = after
            .credentials()
            .into_iter()
            .filter(|credential| !before.iter().any(|old| old == credential))
            .map(String::from)
            .collect();
        let [added] = new.as_slice() else {
            return Err(Fault::Unproven {
                step: self.id(),
                standing: format!("{} new FIDO2 credentials on the disk, not one", new.len()),
            });
        };
        let typed = Secret::new(format!("{}\n", pin.expose()));
        let opened = machine.run_given(
            Program::Tool(Tool::Cryptsetup),
            &[
                "open",
                "--test-passphrase",
                "--token-only",
                "--token-type",
                "systemd-fido2",
                &volume,
            ],
            Given {
                plain: vec![("LD_LIBRARY_PATH", Path::Tokens.get()?)],
                stdin: Some(&typed),
                ..Given::default()
            },
        )?;
        if opened.status != 0 {
            return Err(opened.refusal());
        }
        machine.journal().record(self.id(), added)?;

        let lost: Vec<String> = header
            .tokens
            .iter()
            .filter(|token| matches!(&token.kind, luks::Kind::Fido2 { credential } if *credential != kept))
            .flat_map(|token| token.keyslots.iter().map(u32::to_string))
            .collect();
        if !lost.is_empty() {
            let wipe = format!("--wipe-slot={}", lost.join(","));
            let wiped = machine.run_given(
                Program::Tool(Tool::SystemdCryptenroll),
                &[&wipe, &volume],
                Given {
                    env: vec![("PASSWORD", recovery)],
                    ..Given::default()
                },
            )?;
            if wiped.status != 0 {
                return Err(wiped.refusal());
            }
        }
        Ok(())
    }
}

/// Where the machine's boot partition is mounted.
const BOOT_PARTITION: &str = "/mnt/boot";

/// The generations on the machine's boot partition, newest first.
fn generations(machine: &mut dyn Machine) -> Result<Vec<crate::observe::entries::Entry>, Fault> {
    let mut found: Vec<_> = crate::observe::entries::list_at(machine, BOOT_PARTITION)?
        .into_iter()
        .filter(|entry| entry.generation.is_some())
        .collect();
    found.sort_by_key(|entry| std::cmp::Reverse(entry.generation));
    Ok(found)
}

/// The firmware variable systemd-boot reads its default from before
/// loader.conf, under the Boot Loader Interface's vendor GUID.
const ENTRY_DEFAULT: &str = "LoaderEntryDefault-4a67b082-0a4c-41cf-b6c7-440b29bb8c4f";

/// An entry's id as the variable holds it: UTF-16, with its terminator.
fn utf16(text: &str) -> Vec<u8> {
    text.encode_utf16()
        .chain(std::iter::once(0))
        .flat_map(u16::to_le_bytes)
        .collect()
}

/// The generation the machine boots next, chosen from those on its boot partition,
/// for a machine whose newest won't start. Nothing is removed: the
/// firmware's boot default is set, as `bootctl set-default` sets it but
/// from a stick, where bootctl refuses since systemd-boot didn't boot it,
/// and the menu still offers the rest.
pub struct BootGeneration;

impl Step for BootGeneration {
    fn id(&self) -> &'static str {
        "boot-generation"
    }

    fn place(&self) -> Place {
        Place::Stick
    }

    fn title(&self) -> &'static str {
        "Choose which generation boots next"
    }

    fn observe(&self, machine: &mut dyn Machine) -> Result<Standing, Fault> {
        let found = generations(machine)?;
        let set = machine
            .read(&format!("/sys/firmware/efi/efivars/{ENTRY_DEFAULT}"))?
            .filter(|stored| stored.len() > 4)
            .map(|stored| stored[4..].to_vec());
        if let Some(chosen) = found.iter().find(|entry| {
            set.as_deref() == Some(utf16(&entry.id).as_slice())
                && machine.journal().knows(self.id(), &entry.id)
        }) {
            return Ok(Standing::Done {
                evidence: format!(
                    "it boots generation {} next",
                    chosen.generation.unwrap_or_default()
                ),
            });
        }
        if found.is_empty() {
            return Ok(Standing::Blocked {
                why: "the machine's boot partition holds no generation to boot".into(),
                remedy: "open a root shell inside it and install its boot loader again".into(),
            });
        }
        Ok(Standing::Needed {
            asks: vec![Ask::Choose {
                id: "generation",
                question: "Which should it boot next? The newest is first.".into(),
                choices: found
                    .iter()
                    .map(|entry| {
                        let mut said = entry.described();
                        if entry.default {
                            said.push_str(", which it boots now");
                        }
                        said
                    })
                    .collect(),
            }],
        })
    }

    fn run(&self, machine: &mut dyn Machine, answers: &Answers) -> Result<(), Fault> {
        let Some(Answer::Chosen(at)) = answers.get("generation") else {
            return Err(Fault::Unanswered {
                step: self.id(),
                ask: "generation",
            });
        };
        let found = generations(machine)?;
        let entry = found.get(*at).ok_or(Fault::Blocked {
            step: self.id(),
            why: "that generation isn't on the boot partition any more".into(),
        })?;
        machine.set_efi_variable(ENTRY_DEFAULT, &utf16(&entry.id))?;
        machine.journal().record(self.id(), &entry.id)
    }
}

/// Every step of recovering, in order.
pub fn steps() -> Vec<Box<dyn Step>> {
    vec![
        Box::new(crate::install::memory::RunFromMemory),
        Box::new(crate::install::vault::CheckKeyboard),
        Box::new(FindMachine),
        Box::new(OpenDisk),
        Box::new(MountSystem),
    ]
}
