//! The system itself: Secure Boot keys made for the machine where its boot
//! loader signs with them, the system installed from the vault with no
//! network, and the account's password.

use crate::install::{HELD, ROOT, SYSTEM_CACHE, target, vault, vault_key};
use crate::observe::efi::{self, Mode, SecureBoot};
use crate::tool::Path;
use crate::{
    Answer, Answers, Ask, Fault, Given, Machine, Place, Program, Secret, Standing, Step, Tool,
};

/// Where lanzaboote signs from on the installed machine (core's
/// lanzaboote module's pkiBundle), under the install's root.
const SBCTL: &str = "/mnt/var/lib/sbctl";
const CACHE: &str = "/mnt/var/cache/guide";
const PROFILE: &str = "/mnt/nix/var/nix/profiles/system";
const BOOT_LOADER: &str = "/mnt/boot/EFI/systemd/systemd-bootx64.efi";

/// The machine's Secure Boot keys, where the install's boot loader signs
/// with them: put back from the vault when the stick carries the ones the
/// firmware already trusts, so a reinstall needs no visit to the firmware,
/// or else made here, which the firmware takes only in setup mode. The
/// first boot enrols them in setup mode, and restarts once either way.
pub struct SecureBootKeys;

/// What the vault keeps them as: sbctl's own directory.
const SBCTL_IN_VAULT: &str = "sbctl";

/// The vault's db certificate as DER, when the stick carries the keys.
fn carried_db(machine: &mut dyn Machine) -> Result<Option<Vec<u8>>, Fault> {
    let mut vault = vault(machine)?;
    let pem = format!("{SBCTL_IN_VAULT}/keys/db/db.pem");
    if !vault.files().iter().any(|file| file.path == pem) {
        return Ok(None);
    }
    let mut text = vec![];
    vault.extract(&pem, &mut text)?;
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
        let carried = carried_db(machine)?;
        Ok(match (SecureBoot::observe(machine)?, carried) {
            (Some(SecureBoot { mode: Mode::Setup, .. }), _) => Standing::Needed { asks: vec![] },
            (Some(_), Some(db)) if efi::db_certificates(machine)?.contains(&db) => Standing::Needed { asks: vec![] },
            (Some(found), Some(_)) => Standing::Blocked {
                why: format!(
                    "the firmware is in {} mode and trusts other keys than the ones this stick carries",
                    found.mode
                ),
                remedy: "write the stick again with this machine's keys, or clear the firmware's Secure Boot keys in its setup and restart from the stick".into(),
            },
            (Some(found), None) => Standing::Blocked {
                why: format!("the firmware's Secure Boot is in {} mode, so it won't take new keys", found.mode),
                remedy: "clear the firmware's Secure Boot keys in its setup, as the firmware card shows, then restart from the stick".into(),
            },
            (None, _) => Standing::Blocked {
                why: "the firmware has no Secure Boot".into(),
                remedy: "boot the stick in UEFI mode".into(),
            },
        })
    }

    fn run(&self, machine: &mut dyn Machine, _answers: &Answers) -> Result<(), Fault> {
        if carried_db(machine)?.is_some() {
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
        machine.unpack(key, SYSTEM_CACHE, CACHE)?;
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
