//! The first two steps on the stick: the keyboard typing what's meant, and
//! the vault open.

use crate::install::{VAULT_KEY, vault_key};
use crate::observe::fido2;
use crate::vault::{self, VaultKey};
use crate::{Answer, Answers, Ask, Fault, Machine, Place, Secret, Standing, Step};

/// A word that comes out different on every common layout but Colemak's,
/// so what arrives shows which layout the keyboard types in.
pub const KEYBOARD_WORD: &str = "keyboard";

fn typed<'a>(answers: &'a Answers, id: &str) -> Option<&'a str> {
    match answers.get(id) {
        Some(Answer::Typed(text)) => Some(text),
        _ => None,
    }
}

/// Typing a word and seeing it arrive, before anything secret is typed on
/// a keyboard that might be in another layout.
pub struct CheckKeyboard;

impl Step for CheckKeyboard {
    fn id(&self) -> &'static str {
        "check-keyboard"
    }

    fn place(&self) -> Place {
        Place::Stick
    }

    fn title(&self) -> &'static str {
        "Check the keyboard types what you mean"
    }

    fn observe(&self, machine: &mut dyn Machine) -> Result<Standing, Fault> {
        Ok(if machine.journal().knows(self.id(), "typed") {
            Standing::Done {
                evidence: format!("\"{KEYBOARD_WORD}\" arrived as typed"),
            }
        } else {
            Standing::Needed {
                asks: vec![Ask::TypeBack {
                    id: "word",
                    expected: KEYBOARD_WORD.into(),
                    why: "to see the keyboard's layout before a secret depends on it".into(),
                }],
            }
        })
    }

    fn run(&self, machine: &mut dyn Machine, answers: &Answers) -> Result<(), Fault> {
        match typed(answers, "word") {
            Some(word) if word == KEYBOARD_WORD => machine.journal().record(self.id(), "typed"),
            Some(word) => Err(Fault::Blocked {
                step: self.id(),
                why: format!(
                    "\"{KEYBOARD_WORD}\" arrived as \"{word}\", so this keyboard isn't typing in the layout you know"
                ),
            }),
            None => Err(Fault::Unanswered {
                step: self.id(),
                ask: "word",
            }),
        }
    }
}

/// The vault opened by the recovery passphrase or by one of the machine's
/// YubiKeys, plugged in and touched, with its PIN, its key held in memory
/// for the steps after. One field takes either: what's typed is tried as
/// the passphrase first, which costs the key nothing, so typing the
/// passphrase with a key plugged in never spends one of its PIN's tries.
pub struct OpenVault;

/// The names, as a person says them: "the Nano", "the Nano or the 5Ci".
fn either(names: &[String]) -> String {
    match names {
        [] => String::new(),
        [one] => one.clone(),
        [rest @ .., last] => format!("{} or {last}", rest.join(", ")),
    }
}

/// The YubiKeys the vault says open it, the passphrase left out.
fn keys(machine: &mut dyn Machine) -> Result<Vec<String>, Fault> {
    let store = machine.vault()?;
    if !store.has(vault::KEY)? {
        return Ok(vec![]);
    }
    Ok(vault::manifest(store)?
        .opened_by
        .into_iter()
        .filter(|name| name != "the recovery passphrase")
        .collect())
}

/// The passphrase as the person knows it: where the host had them keep
/// it, under the name it gave.
fn passphrase(machine: &str) -> String {
    format!(
        "the stick's passphrase, the seven words in 1Password on your phone named \"{machine} stick passphrase\""
    )
}

fn sealed(machine: &mut dyn Machine, name: &str) -> Result<Vec<u8>, Fault> {
    let mut sealed = vec![];
    machine
        .vault()?
        .open(name)?
        .read_to_end(&mut sealed)
        .map_err(|why| Fault::unreadable("the vault's sealed key", why))?;
    Ok(sealed)
}

impl Step for OpenVault {
    fn id(&self) -> &'static str {
        "open-vault"
    }

    fn place(&self) -> Place {
        Place::Stick
    }

    fn title(&self) -> &'static str {
        "Open the stick's vault"
    }

    fn observe(&self, machine: &mut dyn Machine) -> Result<Standing, Fault> {
        // Its key held is the vault open, whether or not the stick is in:
        // it may be out for a YubiKey's port.
        if vault_key(machine)?.is_some() {
            return Ok(Standing::Done {
                evidence: "the stick's vault is open".into(),
            });
        }
        if !machine.vault()?.has(vault::KEY_PASSPHRASE)? {
            return Ok(Standing::Blocked {
                why: "this vault has no copy of its key a passphrase opens".into(),
                remedy: "write the stick again from a machine that has its recovery passphrase"
                    .into(),
            });
        }
        let keys = keys(machine)?;
        let name = passphrase(&vault::manifest(machine.vault()?)?.machine);
        if keys.is_empty() {
            return Ok(Standing::Needed {
                asks: vec![Ask::Secret {
                    id: "passphrase",
                    name,
                }],
            });
        }
        Ok(Standing::Needed {
            asks: vec![Ask::SecretOr {
                id: "passphrase",
                name,
                or: format!(
                    "if a port is free beside the stick, plug in {} there, type its PIN and press Return, then touch the key when it blinks",
                    either(&keys)
                ),
            }],
        })
    }

    fn run(&self, machine: &mut dyn Machine, answers: &Answers) -> Result<(), Fault> {
        let Some(Answer::Secret(typed)) = answers.get("passphrase") else {
            return Err(Fault::Unanswered {
                step: self.id(),
                ask: "passphrase",
            });
        };
        let sealed_to_passphrase = sealed(machine, vault::KEY_PASSPHRASE)?;
        // A host makes the passphrase as words, read as the words they say;
        // one chosen by hand, for a test, is taken as typed.
        let by_passphrase = match VaultKey::open_with_passphrase(&sealed_to_passphrase, typed) {
            Err(_) if crate::words::typed(typed.expose()) != typed.expose() => {
                VaultKey::open_with_passphrase(
                    &sealed_to_passphrase,
                    &Secret::new(crate::words::typed(typed.expose())),
                )
            }
            opened => opened,
        };
        let key = match by_passphrase {
            Ok(key) => key,
            Err(not_the_passphrase) => {
                let plugged = if keys(machine)?.is_empty() {
                    vec![]
                } else {
                    fido2::plugged(machine)?
                };
                match plugged.len() {
                    0 => return Err(not_the_passphrase),
                    1 => VaultKey::open_with_key(&sealed(machine, vault::KEY)?, typed)
                        .map_err(|fault| fido2::neither(fault, "the recovery passphrase"))?,
                    many => {
                        return Err(Fault::Blocked {
                            step: self.id(),
                            why: format!("{many} keys are plugged in; leave only one"),
                        });
                    }
                }
            }
        };
        machine.write(VAULT_KEY, key.to_secret().expose().as_bytes())?;
        // What it says to install is held too, so the steps that only read
        // it go on while the stick is out for a YubiKey's port, as is the
        // script that opens a stopped install's disk through a YubiKey.
        crate::install::hold_target(machine)?;
        crate::install::hold_mount_script(machine)?;
        crate::install::system::hold_carried_db(machine)
    }
}
