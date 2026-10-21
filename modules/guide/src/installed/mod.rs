//! Finishing a machine on the machine itself, once it's booted from its
//! own disk: Secure Boot checked to enforce with its own keys, then the
//! disk sealed to the TPM with a PIN and its slot tried first, so every
//! boot asks for that PIN, and a YubiKey or the recovery key opens it when
//! the TPM won't. The guide runs as the person; what needs root goes
//! through run0, which asks polkit.

use serde::Deserialize;

use crate::observe::efi::{Mode, SecureBoot};
use crate::observe::luks;
use crate::tool::Path;
use crate::{
    Answer, Answers, Ask, Fault, Given, Machine, Place, Program, Secret, Standing, Step, Tool,
};

/// What core's guide module says about the machine (../../nixos.nix).
pub const FACTS: &str = "/etc/guide/machine.json";
/// The measured-boot policy the TPM is sealed to, which core's lanzaboote
/// module has systemd-pcrlock keep for PCRs 0, 4 and 7.
pub const PCRLOCK: &str = "/var/lib/systemd/pcrlock.json";

#[derive(Debug, Deserialize)]
pub struct Facts {
    /// What core calls the disk's encrypted volume.
    pub disk_name: String,
}

pub fn facts(machine: &mut dyn Machine) -> Result<Facts, Fault> {
    let bytes = machine.read(FACTS)?.ok_or_else(|| Fault::Blocked {
        step: "reading the machine",
        why: format!("{FACTS} isn't here, so core's guide module isn't on this machine"),
    })?;
    serde_json::from_slice(&bytes).map_err(|why| Fault::unreadable(FACTS, why))
}

fn volume(facts: &Facts) -> String {
    format!("/dev/disk/by-partlabel/{}", facts.disk_name)
}

fn as_root<'a>() -> Given<'a> {
    Given {
        as_root: true,
        ..Given::default()
    }
}

/// Secure Boot enforcing with the machine's own keys, which the TPM's seal
/// then counts on: sealing before it does is what once left a disk sealed
/// to a policy it never boots with.
pub struct CheckSecureBoot;

impl Step for CheckSecureBoot {
    fn id(&self) -> &'static str {
        "check-secure-boot"
    }

    fn place(&self) -> Place {
        Place::Installed
    }

    fn title(&self) -> &'static str {
        "Check Secure Boot enforces"
    }

    fn observe(&self, machine: &mut dyn Machine) -> Result<Standing, Fault> {
        Ok(match SecureBoot::observe(machine)? {
            Some(SecureBoot {
                enforcing: true,
                mode: mode @ (Mode::User | Mode::Deployed),
            }) => Standing::Done {
                evidence: format!(
                    "Secure Boot enforces with this machine's own keys, in {mode} mode"
                ),
            },
            Some(SecureBoot {
                mode: Mode::Setup, ..
            }) => Standing::Blocked {
                why: "the firmware is still in setup mode, its keys not yet enrolled".into(),
                remedy: "restart: the first boot enrols them and restarts once more".into(),
            },
            Some(_) => Standing::Blocked {
                why: "Secure Boot isn't enforcing".into(),
                remedy: "turn Secure Boot on in the firmware's setup, then start the guide again"
                    .into(),
            },
            None => Standing::Blocked {
                why: "the firmware has no Secure Boot".into(),
                remedy: "boot in UEFI mode".into(),
            },
        })
    }

    fn run(&self, _machine: &mut dyn Machine, _answers: &Answers) -> Result<(), Fault> {
        // Nothing to do here: observing says whether it's done.
        Ok(())
    }
}

/// The disk sealed to the TPM with a PIN, under the measured-boot policy,
/// its slot preferred over the others, and proven to open with the PIN.
pub struct SealTpm;

fn secret<'a>(answers: &'a Answers, id: &str) -> Option<&'a Secret> {
    match answers.get(id) {
        Some(Answer::Secret(secret)) => Some(secret),
        _ => None,
    }
}

impl Step for SealTpm {
    fn id(&self) -> &'static str {
        "seal-tpm"
    }

    fn place(&self) -> Place {
        Place::Installed
    }

    fn title(&self) -> &'static str {
        "Seal the disk to the TPM"
    }

    fn observe(&self, machine: &mut dyn Machine) -> Result<Standing, Fault> {
        let facts = facts(machine)?;
        let header = luks::observe_as_root(machine, &volume(&facts))?;
        let sealed: Vec<&luks::Token> = header
            .tokens
            .iter()
            .filter(|token| token.kind == luks::Kind::Tpm2)
            .collect();
        let proven = sealed.iter().any(|token| {
            token
                .keyslots
                .iter()
                .all(|slot| header.preferred.contains(slot))
                && token.keyslots.iter().all(|slot| {
                    machine
                        .journal()
                        .knows(self.id(), &format!("opened:{slot}"))
                })
        });
        if proven {
            return Ok(Standing::Done {
                evidence: "the TPM opens the disk with its PIN, tried before the others".into(),
            });
        }
        Ok(Standing::Needed {
            asks: vec![
                Ask::Secret {
                    id: "tpm-pin",
                    name: "a PIN for the TPM, which the disk asks for at every boot".into(),
                },
                Ask::Secret {
                    id: "again",
                    name: "the same PIN again".into(),
                },
                Ask::Act {
                    id: "key",
                    what: "Plug in a YubiKey, which opens the disk for the TPM's slot to be made. Touch it when it blinks.".into(),
                },
                Ask::Secret {
                    id: "key-pin",
                    name: "the YubiKey's PIN".into(),
                },
            ],
        })
    }

    fn run(&self, machine: &mut dyn Machine, answers: &Answers) -> Result<(), Fault> {
        let (Some(pin), Some(again), Some(key_pin)) = (
            secret(answers, "tpm-pin"),
            secret(answers, "again"),
            secret(answers, "key-pin"),
        ) else {
            return Err(Fault::Unanswered {
                step: self.id(),
                ask: "tpm-pin",
            });
        };
        if pin != again {
            return Err(Fault::Blocked {
                step: self.id(),
                why: "the two PINs weren't the same".into(),
            });
        }
        if pin.expose().chars().count() < 4 {
            return Err(Fault::Blocked {
                step: self.id(),
                why: "a TPM PIN has to be at least 4 characters".into(),
            });
        }
        let facts = facts(machine)?;
        let volume = volume(&facts);
        let before = luks::observe_as_root(machine, &volume)?;

        let pcrlock = format!("--tpm2-pcrlock={PCRLOCK}");
        let mut args = vec![
            "--unlock-fido2-device=auto",
            "--tpm2-device=auto",
            "--tpm2-with-pin=yes",
            pcrlock.as_str(),
        ];
        // A slot sealed before is replaced, so none is left that nothing
        // proved.
        if before.has(&luks::Kind::Tpm2) {
            args.push("--wipe-slot=tpm2");
        }
        args.push(&volume);
        let enrolled = machine.run_given(
            Program::Tool(Tool::SystemdCryptenroll),
            &args,
            Given {
                // systemd 261.3 asks for the PIN that unlocks with a FIDO2
                // key as cryptsetup.fido2-pin, whatever name cryptenroll
                // hands it (src/shared/cryptsetup-fido2.c ignores its
                // askpw_credential), so the PIN goes under both: the name
                // read today, and the one cryptenroll means.
                credentials: vec![
                    ("cryptsetup.fido2-pin", key_pin),
                    ("cryptenroll.fido2-pin", key_pin),
                    ("cryptenroll.new-tpm2-pin", pin),
                ],
                as_root: true,
                ..Given::default()
            },
        )?;
        if enrolled.status != 0 {
            return Err(enrolled.refusal());
        }

        let after = luks::observe_as_root(machine, &volume)?;
        let slots: Vec<u32> = after
            .tokens
            .iter()
            .filter(|token| token.kind == luks::Kind::Tpm2)
            .flat_map(|token| token.keyslots.iter().copied())
            .collect();
        let [slot] = slots.as_slice() else {
            return Err(Fault::Unproven {
                step: self.id(),
                standing: format!("{} TPM slots on the disk, not one", slots.len()),
            });
        };
        let slot = slot.to_string();
        machine
            .run_given(
                Program::Tool(Tool::Cryptsetup),
                &[
                    "config",
                    &volume,
                    "--key-slot",
                    &slot,
                    "--priority",
                    "prefer",
                ],
                as_root(),
            )?
            .ok()?;

        // Proven by opening the disk with the TPM and the PIN, as the boot
        // will, through cryptsetup's token plugin.
        let typed = Secret::new(format!("{}\n", pin.expose()));
        let opened = machine.run_given(
            Program::Tool(Tool::Cryptsetup),
            &[
                "open",
                "--test-passphrase",
                "--token-only",
                "--token-type",
                "systemd-tpm2",
                &volume,
            ],
            Given {
                plain: vec![("LD_LIBRARY_PATH", Path::Tokens.get()?)],
                stdin: Some(&typed),
                as_root: true,
                ..Given::default()
            },
        )?;
        if opened.status != 0 {
            return Err(opened.refusal());
        }
        machine
            .journal()
            .record(self.id(), &format!("opened:{slot}"))
    }
}

/// Every step on the installed machine, in order.
pub fn steps() -> Vec<Box<dyn Step>> {
    vec![Box::new(CheckSecureBoot), Box::new(SealTpm)]
}
