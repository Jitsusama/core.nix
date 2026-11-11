//! The repairs to how a recovered machine boots: its boot loader written
//! again by its own installed system, and the generation it boots next
//! chosen from those on its boot partition.

use crate::tool::Path;
use crate::{Answer, Answers, Ask, Fault, Given, Machine, Place, Program, Standing, Step, Tool};

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
