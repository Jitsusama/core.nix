//! The system itself: Secure Boot keys made for the machine where its boot
//! loader signs with them, the system installed from the vault with no
//! network, and the account's password.

use std::io::Write;

use crate::install::{
    HELD, RECOVERY_KEY, ROOT, STICK_VAULT, SYSTEM_CACHE, held, target, vault, vault_key,
};
use crate::observe::disks;
use crate::observe::efi::{self, Mode, SecureBoot};
use crate::rescue::{self, Bundle, Keys};
use crate::stick;
use crate::tool::Path;
use crate::vault::Vault;
use crate::{
    Answer, Answers, Ask, Fault, Given, Machine, Place, Program, Secret, Standing, Step, Tool,
};

/// Where lanzaboote signs from on the installed machine (core's
/// lanzaboote module's pkiBundle), under the install's root.
const SBCTL: &str = "/mnt/var/lib/sbctl";
const CACHE: &str = "/mnt/var/cache/guide";
const PROFILE: &str = "/mnt/nix/var/nix/profiles/system";
const BOOT_LOADER: &str = "/mnt/boot/EFI/systemd/systemd-bootx64.efi";
/// What core's lanzaboote module links at a system's top to ready its
/// keys for systemd-boot to enrol, and where systemd-boot reads them.
const ENROLMENT: &str = "secure-boot-enrolment";
const ENROLLED_FROM: [&str; 3] = [
    "/mnt/boot/loader/keys/auto/PK.auth",
    "/mnt/boot/loader/keys/auto/KEK.auth",
    "/mnt/boot/loader/keys/auto/db.auth",
];

/// The program that readies the installed system's keys for systemd-boot
/// to enrol at its very first boot, when it has one and the firmware is
/// in setup mode, so would take them. Left to that boot, it readies them
/// once the disk is open and restarts, and the disk asks a second time.
fn enrolment(machine: &mut dyn Machine, system: &str) -> Result<Option<String>, Fault> {
    let program = format!("{system}/{ENROLMENT}");
    if machine.read_link(&format!("{ROOT}{program}"))?.is_none() {
        return Ok(None);
    }
    Ok(match SecureBoot::observe(machine)? {
        Some(SecureBoot {
            mode: Mode::Setup, ..
        }) => Some(program),
        _ => None,
    })
}

/// Whether the install puts back the keys the stick carries: only when
/// the firmware has a platform key and its db trusts them already. A
/// firmware without one would enrol whatever keys it's given, and the
/// stick's could come from a bundle anyone can seal, so the install makes
/// new ones instead.
fn puts_keys_back(machine: &mut dyn Machine) -> Result<bool, Fault> {
    let Some(db) = carried_db(machine)? else {
        return Ok(false);
    };
    Ok(match SecureBoot::observe(machine)? {
        Some(SecureBoot {
            mode: Mode::Setup | Mode::Audit,
            ..
        })
        | None => false,
        Some(_) => efi::db_certificates(machine)?.contains(&db),
    })
}

/// Whether the firmware will boot with the keys the install gives the
/// machine: new ones in setup mode, or the stick's own when its db already
/// trusts them. Said back as how, or why not as the standing to stop on.
fn firmware_takes_keys(machine: &mut dyn Machine) -> Result<Result<String, Standing>, Fault> {
    let carried = carried_db(machine)?;
    Ok(match (SecureBoot::observe(machine)?, carried) {
        (
            Some(SecureBoot {
                mode: mode @ (Mode::Setup | Mode::Audit),
                ..
            }),
            _,
        ) => Ok(format!(
            "the firmware is in {mode} mode, so it takes the keys made here on its first boot"
        )),
        (Some(found), Some(db)) if efi::db_certificates(machine)?.contains(&db) => Ok(format!(
            "the firmware is in {} mode and trusts the keys this stick carries",
            found.mode
        )),
        (Some(found), Some(_)) => Err(Standing::Blocked {
            why: format!(
                "the firmware is in {} mode and trusts other keys than the ones this stick carries",
                found.mode
            ),
            remedy: "write the stick again with this machine's keys, or clear the firmware's Secure Boot keys in its setup and restart from the stick".into(),
        }),
        (Some(found), None) => Err(Standing::Blocked {
            why: format!("the firmware's Secure Boot is in {} mode, so it won't take new keys", found.mode),
            remedy: "clear the firmware's Secure Boot keys in its setup, or write the stick again with this machine's keys, then restart from the stick".into(),
        }),
        (None, _) => Err(Standing::Blocked {
            why: "the firmware has no Secure Boot".into(),
            remedy: "boot the stick in UEFI mode".into(),
        }),
    })
}

/// Before the disk is touched: the firmware will take the keys the install
/// gives the machine, so a machine is never wiped to find out it can't be
/// booted again.
pub struct CheckFirmware;

impl Step for CheckFirmware {
    fn id(&self) -> &'static str {
        "check-firmware"
    }

    fn place(&self) -> Place {
        Place::Stick
    }

    fn title(&self) -> &'static str {
        "Check the firmware will boot what's installed"
    }

    fn observe(&self, machine: &mut dyn Machine) -> Result<Standing, Fault> {
        // Once the machine's keys are in place the install has passed this.
        if machine.read(&format!("{SBCTL}/keys/db/db.key"))?.is_some() {
            return Ok(Standing::Done {
                evidence: "the machine's own Secure Boot keys are in place".into(),
            });
        }
        Ok(match firmware_takes_keys(machine)? {
            Ok(evidence) => Standing::Done { evidence },
            Err(standing) => standing,
        })
    }

    fn run(&self, _machine: &mut dyn Machine, _answers: &Answers) -> Result<(), Fault> {
        // Nothing to do here: observing says whether it's ready.
        Ok(())
    }
}

/// The machine's Secure Boot keys, where the install's boot loader signs
/// with them: put back from the vault when the stick carries the ones the
/// firmware's db trusts already while it has a platform key, so a
/// reinstall needs no visit to the firmware, or else made here, which the
/// firmware takes only in setup mode. In setup mode the install readies
/// them for systemd-boot (`ReadyEnrolment`), which enrols them as the
/// machine first starts and restarts it, before any disk asks for a PIN.
pub struct SecureBootKeys;

/// Where the installed machine keeps its rescue bundle, under the
/// install's root.
fn bundle_path() -> String {
    format!("{ROOT}{}", rescue::KEPT)
}

/// The machine's rescue bundle, made as soon as it has its Secure Boot
/// keys and its recovery key, so no machine leaves its install without
/// one: the keys sealed to its YubiKeys and to the recovery key the
/// person keeps on their phone, kept on the machine and on the stick.
/// Anyone can read it, since it's sealed, so the guide reads it as the
/// person, without asking for root. A reinstall, which puts the keys
/// back, makes it again with the new recovery key.
pub struct SealRescueBundle;

impl Step for SealRescueBundle {
    fn id(&self) -> &'static str {
        "seal-rescue-bundle"
    }

    fn place(&self) -> Place {
        Place::Stick
    }

    fn title(&self) -> &'static str {
        "Seal its Secure Boot keys into a rescue bundle"
    }

    fn observe(&self, machine: &mut dyn Machine) -> Result<Standing, Fault> {
        let bundle = machine
            .read(&bundle_path())?
            .and_then(|bytes| Bundle::read(&bytes).ok());
        let certificate = machine.read(&format!("{SBCTL}/keys/db/db.pem"))?;
        if let (Some(bundle), Some(certificate)) = (bundle, certificate)
            && bundle.holds(&certificate)
            && bundle.passphrase() == rescue::RECOVERY_KEY
        {
            return Ok(Standing::Done {
                evidence: format!(
                    "{} holds its Secure Boot keys, opened by a YubiKey or the recovery key",
                    rescue::KEPT
                ),
            });
        }
        Ok(if machine.read(RECOVERY_KEY)?.is_some() {
            Standing::Needed { asks: vec![] }
        } else {
            Standing::Blocked {
                why: "the recovery key isn't held any more, so the bundle can't be sealed to it"
                    .into(),
                remedy: "restart from the stick, which makes another".into(),
            }
        })
    }

    fn run(&self, machine: &mut dyn Machine, _answers: &Answers) -> Result<(), Fault> {
        let recovery = held(machine, RECOVERY_KEY)?.ok_or(Fault::Blocked {
            step: self.id(),
            why: "the recovery key isn't held any more".into(),
        })?;
        let target = target(machine)?;
        let keys = Keys::read_in(machine, SBCTL)?;
        let bytes = Bundle::seal(&target, &keys, &recovery, rescue::RECOVERY_KEY, None)?;
        machine.publish(&bundle_path(), &bytes)
    }
}

/// What the vault keeps them as: sbctl's own directory.
const SBCTL_IN_VAULT: &str = "sbctl";

/// The vault's db certificate, as it was held when the vault opened: what
/// the vault holds, or nothing when it holds none.
const HELD_DB: &str = "/run/guide/carried-db.pem";

/// The vault's db certificate as PEM, when the stick carries the keys.
fn carried_pem(machine: &mut dyn Machine) -> Result<Option<Vec<u8>>, Fault> {
    let mut vault = vault(machine)?;
    let pem = format!("{SBCTL_IN_VAULT}/keys/db/db.pem");
    if !vault.files().iter().any(|file| file.path == pem) {
        return Ok(None);
    }
    let mut text = vec![];
    vault.extract(&pem, &mut text)?;
    Ok(Some(text))
}

/// Holds the vault's db certificate, or that it has none, so the firmware
/// is checked against it while the stick is out.
pub fn hold_carried_db(machine: &mut dyn Machine) -> Result<(), Fault> {
    let pem = carried_pem(machine)?.unwrap_or_default();
    machine.write(HELD_DB, &pem)
}

/// The vault's db certificate as DER, when the stick carries the keys:
/// from the vault, or, while the stick is out, as it was held.
fn carried_db(machine: &mut dyn Machine) -> Result<Option<Vec<u8>>, Fault> {
    let text = match carried_pem(machine) {
        Err(Fault::StickOut) => {
            let held = machine.read(HELD_DB)?.ok_or(Fault::StickOut)?;
            (!held.is_empty()).then_some(held)
        }
        text => text?,
    };
    let Some(text) = text else {
        return Ok(None);
    };
    let (_, der) = rsa::pkcs8::der::pem::decode_vec(&text)
        .map_err(|why| Fault::unreadable("the vault's db certificate", why))?;
    Ok(Some(der))
}

impl Step for SecureBootKeys {
    fn id(&self) -> &'static str {
        "secure-boot-keys"
    }

    fn place(&self) -> Place {
        Place::Stick
    }

    fn title(&self) -> &'static str {
        "Give the machine its Secure Boot keys"
    }

    fn observe(&self, machine: &mut dyn Machine) -> Result<Standing, Fault> {
        let there = machine.read(&format!("{SBCTL}/keys/db/db.key"))?.is_some()
            && machine.read(&format!("{SBCTL}/GUID"))?.is_some();
        if there {
            return Ok(Standing::Done {
                evidence: "the machine's own Secure Boot keys are in place".into(),
            });
        }
        Ok(match firmware_takes_keys(machine)? {
            Ok(_) => Standing::Needed { asks: vec![] },
            Err(standing) => standing,
        })
    }

    fn run(&self, machine: &mut dyn Machine, _answers: &Answers) -> Result<(), Fault> {
        if puts_keys_back(machine)? {
            let key = vault_key(machine)?.ok_or(Fault::Blocked {
                step: self.id(),
                why: "the vault isn't open".into(),
            })?;
            machine.unpack(key, SBCTL_IN_VAULT, SBCTL)?;
            return Ok(());
        }
        machine
            .run(
                Tool::Sbctl,
                &[
                    // Landlock would keep sbctl inside the stick's own
                    // /var/lib/sbctl, and these go to the new system's.
                    "--disable-landlock",
                    "create-keys",
                    "--export",
                    &format!("{SBCTL}/keys"),
                    "--database-path",
                    &format!("{SBCTL}/GUID"),
                ],
            )?
            .ok()
            .map(drop)
    }
}

/// The system copied from the vault's signed cache into the new store and
/// installed with its boot loader, with no network.
pub struct InstallSystem;

impl Step for InstallSystem {
    fn id(&self) -> &'static str {
        "install-system"
    }

    fn place(&self) -> Place {
        Place::Stick
    }

    fn title(&self) -> &'static str {
        "Install the system"
    }

    fn observe(&self, machine: &mut dyn Machine) -> Result<Standing, Fault> {
        let target = target(machine)?;
        let installed = machine.resolve_inside(ROOT, PROFILE)?.as_deref()
            == Some(target.system.as_str())
            && machine.read(BOOT_LOADER)?.is_some();
        Ok(if installed {
            Standing::Done {
                evidence: format!("{} is installed, with its boot loader", target.machine),
            }
        } else {
            Standing::Needed { asks: vec![] }
        })
    }

    fn run(&self, machine: &mut dyn Machine, _answers: &Answers) -> Result<(), Fault> {
        let target = target(machine)?;
        let key = vault_key(machine)?.ok_or(Fault::Blocked {
            step: self.id(),
            why: "the vault isn't open".into(),
        })?;
        // The cache goes onto the new disk, never into memory, since it's
        // the whole system.
        let cached = machine.unpack(key, SYSTEM_CACHE, CACHE)?;
        // Nix reads every file of the cache once as it copies the system,
        // so what it has read says how far along it is.
        machine
            .meter()
            .stage_by_reads("Copying it onto the disk", Some(cached));
        let from = format!("file://{CACHE}");
        let to = format!("local?root={ROOT}");
        // The cache carries no Nix signatures, since the vault already
        // authenticates every byte of it: age seals each object to the
        // vault's key, and its sealed index holds each file's SHA-256, so
        // only someone holding that key could change it, and a signing key
        // kept beside it would rest on the same key. Nix still checks each
        // NAR against the hash its narinfo names.
        let copied = machine.run_given(
            Program::Tool(Tool::Nix),
            &[
                "--extra-experimental-features",
                "nix-command",
                "copy",
                "--no-check-sigs",
                "--from",
                &from,
                "--to",
                &to,
                &target.system,
            ],
            Given {
                plain: vec![("HOME", HELD)],
                ..Given::default()
            },
        )?;
        if copied.status != 0 {
            return Err(copied.refusal());
        }
        machine.meter().stage("Installing its boot loader", None);
        let installed = machine.run_given(
            Program::Tool(Tool::NixosInstall),
            &[
                "--root",
                ROOT,
                "--system",
                &target.system,
                "--no-root-passwd",
                "--no-channel-copy",
            ],
            Given {
                plain: vec![("PATH", Path::Install.get()?), ("HOME", HELD)],
                ..Given::default()
            },
        )?;
        if installed.status != 0 {
            return Err(installed.refusal());
        }
        machine.remove_tree(CACHE)
    }
}

/// The installed system's Secure Boot keys readied on its disk for
/// systemd-boot to enrol as it first starts, before any disk is opened,
/// when the firmware is in setup mode and so would take them. A firmware
/// with keys of its own enrols nothing.
pub struct ReadyEnrolment;

impl Step for ReadyEnrolment {
    fn id(&self) -> &'static str {
        "ready-enrolment"
    }

    fn place(&self) -> Place {
        Place::Stick
    }

    fn title(&self) -> &'static str {
        "Ready its Secure Boot keys for the firmware"
    }

    fn observe(&self, machine: &mut dyn Machine) -> Result<Standing, Fault> {
        let target = target(machine)?;
        if machine.resolve_inside(ROOT, PROFILE)?.as_deref() != Some(target.system.as_str()) {
            // Not installed yet, which comes first.
            return Ok(Standing::Needed { asks: vec![] });
        }
        if enrolment(machine, &target.system)?.is_none() {
            return Ok(Standing::Done {
                evidence: "the firmware isn't in setup mode, so it keeps the keys it has".into(),
            });
        }
        for file in ENROLLED_FROM {
            if machine.read(file)?.is_none() {
                return Ok(Standing::Needed { asks: vec![] });
            }
        }
        Ok(Standing::Done {
            evidence: "systemd-boot enrols its Secure Boot keys as it first starts, before the disk is opened".into(),
        })
    }

    fn run(&self, machine: &mut dyn Machine, _answers: &Answers) -> Result<(), Fault> {
        let target = target(machine)?;
        let Some(program) = enrolment(machine, &target.system)? else {
            return Ok(());
        };
        // Inside the installed system, as its first boot would: its own
        // sbctl and keys, and its ESP at /boot.
        let readied = machine.run_given(
            Program::Tool(Tool::NixosEnter),
            &["--root", ROOT, "-c", &program],
            Given {
                plain: vec![("PATH", Path::Install.get()?)],
                ..Given::default()
            },
        )?;
        if readied.status != 0 {
            return Err(readied.refusal());
        }
        Ok(())
    }
}

/// What the stick's image names its EFI system partition.
const ESP_NAME: &str = "esp";

/// The stick made the machine's own once its install has its keys: the
/// keys sealed into the stick's vault, where a reinstall takes them from,
/// the machine's rescue bundle beside them, where a host writing a stick
/// finds it, and the stick's boot program signed with the machine's db
/// key. Pulled out, it's the machine's repair stick: the firmware boots it
/// once it enforces the machine's keys, and reinstalling from it needs no
/// visit to the firmware and no other machine.
pub struct KeepOnStick;

impl Step for KeepOnStick {
    fn id(&self) -> &'static str {
        "keep-on-stick"
    }

    fn place(&self) -> Place {
        Place::Stick
    }

    fn title(&self) -> &'static str {
        "Make the stick this machine's own"
    }

    fn observe(&self, machine: &mut dyn Machine) -> Result<Standing, Fault> {
        let Some(certificate) = machine.read(&format!("{SBCTL}/keys/db/db.pem"))? else {
            return Ok(Standing::Blocked {
                why: "the machine has no Secure Boot keys yet".into(),
                remedy: "restart from the stick, which gives it them".into(),
            });
        };
        let target = target(machine)?;
        Ok(
            if machine
                .journal()
                .knows(self.id(), &rescue::fingerprint(&certificate))
            {
                Standing::Done {
                    evidence: format!(
                        "the stick carries {}'s keys and rescue bundle, and boots with its keys",
                        target.machine
                    ),
                }
            } else {
                Standing::Needed { asks: vec![] }
            },
        )
    }

    fn run(&self, machine: &mut dyn Machine, _answers: &Answers) -> Result<(), Fault> {
        let target = target(machine)?;
        let keys = Keys::read_in(machine, SBCTL)?;
        let bundle = machine.read(&bundle_path())?.ok_or(Fault::Blocked {
            step: self.id(),
            why: "the machine has no rescue bundle yet".into(),
        })?;
        let key = vault_key(machine)?.ok_or(Fault::Blocked {
            step: self.id(),
            why: "the vault isn't open".into(),
        })?;
        // Only this stick's own boot partition: the one named for it on the
        // disk its vault is on, never one on a disk beside it.
        let no_esp = || Fault::Blocked {
            step: "keep-on-stick",
            why: "the stick's vault is on a disk with no boot partition beside it".into(),
        };
        let vault = machine.resolve(STICK_VAULT)?.ok_or_else(no_esp)?;
        let esp = disks::observe(machine)?
            .into_iter()
            .find(|disk| {
                disk.partitions
                    .iter()
                    .any(|partition| partition.path == vault)
            })
            .and_then(|disk| {
                disk.partitions
                    .into_iter()
                    .find(|partition| partition.label.as_deref() == Some(ESP_NAME))
            })
            .ok_or_else(no_esp)?;
        let carried = rescue::carried_as(&target.machine);
        // Refused before the stick is touched, if the keys can't sign.
        let signer = keys.signer()?;
        machine.change_vault(&mut |store| {
            let mut vault = Vault::open(&mut *store, key.clone())?;
            vault.put_all(
                keys.files()
                    .map(|(name, bytes)| (format!("{SBCTL_IN_VAULT}/{name}"), bytes)),
            )?;
            drop(vault);
            let mut file = store.create(&carried)?;
            file.write_all(&bundle)
                .and_then(|()| file.flush())
                .map_err(|why| Fault::Unkept {
                    path: carried.clone(),
                    why: why.to_string(),
                })
        })?;
        // The stick mounts its own boot partition at /boot as it starts, and
        // nothing may read it there while it's changed underneath.
        for mounted in &esp.mounted {
            machine.run(Tool::Umount, &[mounted])?.ok()?;
        }
        stick::sign_in_place(&mut machine.open_partition(&esp.path)?, &signer)?;
        machine
            .journal()
            .record(self.id(), &rescue::fingerprint(keys.file("keys/db/db.pem")))
    }
}

/// Whether the account has a password it can sign in with, in a shadow
/// file's text.
fn has_password(shadow: &str, account: &str) -> bool {
    shadow
        .lines()
        .filter_map(|line| line.split_once(':'))
        .find(|(name, _)| *name == account)
        .and_then(|(_, rest)| rest.split(':').next())
        .is_some_and(|hash| !hash.is_empty() && !hash.starts_with('!') && !hash.starts_with('*'))
}

/// The password the person signs in with, typed twice.
pub struct SetPassword;

impl Step for SetPassword {
    fn id(&self) -> &'static str {
        "set-password"
    }

    fn place(&self) -> Place {
        Place::Stick
    }

    fn title(&self) -> &'static str {
        "Choose the account's password"
    }

    fn observe(&self, machine: &mut dyn Machine) -> Result<Standing, Fault> {
        let target = target(machine)?;
        let shadow = machine.read("/mnt/etc/shadow")?.unwrap_or_default();
        Ok(
            if has_password(&String::from_utf8_lossy(&shadow), &target.account) {
                Standing::Done {
                    evidence: format!("{} has a password", target.account),
                }
            } else {
                Standing::Needed {
                    asks: vec![
                        Ask::Secret {
                            id: "password",
                            name: format!("a password for {}", target.account),
                        },
                        Ask::Secret {
                            id: "again",
                            name: "the same password again".into(),
                        },
                    ],
                }
            },
        )
    }

    fn run(&self, machine: &mut dyn Machine, answers: &Answers) -> Result<(), Fault> {
        let (Some(Answer::Secret(password)), Some(Answer::Secret(again))) =
            (answers.get("password"), answers.get("again"))
        else {
            return Err(Fault::Unanswered {
                step: self.id(),
                ask: "password",
            });
        };
        if password != again {
            return Err(Fault::Blocked {
                step: self.id(),
                why: "the two passwords weren't the same".into(),
            });
        }
        let target = target(machine)?;
        let line = Secret::new(format!("{}:{}\n", target.account, password.expose()));
        // By the new system's own path, as nixos-install calls passwd,
        // since nixos-enter keeps this PATH, which has only the install's
        // tools.
        let set = machine.run_given(
            Program::Tool(Tool::NixosEnter),
            &[
                "--root",
                ROOT,
                "-c",
                "/nix/var/nix/profiles/system/sw/bin/chpasswd",
            ],
            Given {
                plain: vec![("PATH", Path::Install.get()?)],
                stdin: Some(&line),
                ..Given::default()
            },
        )?;
        if set.status == 0 {
            Ok(())
        } else {
            Err(set.refusal())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_real_hash_is_a_password() {
        let shadow = "root:!:1::::::\nalice:$y$j9T$abc$def:1::::::\nbob:!$y$j9T$abc:1::::::\ncarol::1::::::\n";
        assert!(has_password(shadow, "alice"));
        for locked in ["root", "bob", "carol", "dave"] {
            assert!(!has_password(shadow, locked), "{locked}");
        }
    }
}
