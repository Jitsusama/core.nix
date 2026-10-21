//! The first two steps on the stick: the keyboard typing what's meant, and
//! the vault open.

use crate::install::{VAULT_KEY, vault_key};
use crate::vault::{self, Vault, VaultKey};
use crate::{Answer, Answers, Ask, Fault, Machine, Place, Standing, Step};

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

/// The vault opened by one of the machine's YubiKeys, plugged in and
/// touched, with its PIN, or with none plugged in by the recovery
/// passphrase, its key held in memory for the steps after. One field takes
/// either, since which it is follows from whether a key is plugged in.
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
        if let Some(key) = vault_key(machine)? {
            let vault = Vault::open(machine.vault()?, key)?;
            return Ok(Standing::Done {
                evidence: format!("the vault is open, holding {} files", vault.files().len()),
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
        if keys.is_empty() {
            return Ok(Standing::Needed {
                asks: vec![Ask::Secret {
                    id: "passphrase",
                    name: "the vault's recovery passphrase".into(),
                }],
            });
        }
        let named = either(&keys);
        Ok(Standing::Needed {
            asks: vec![
                Ask::Act {
                    id: "key",
                    what: format!(
                        "Plug in {named}, and touch it when it blinks. To use the recovery passphrase instead, leave it out."
                    ),
                },
                Ask::Secret {
                    id: "passphrase",
                    name: format!(
                        "the PIN of {named}, or with none plugged in, the recovery passphrase"
                    ),
                },
            ],
        })
    }

    fn run(&self, machine: &mut dyn Machine, answers: &Answers) -> Result<(), Fault> {
        let Some(Answer::Secret(typed)) = answers.get("passphrase") else {
            return Err(Fault::Unanswered {
                step: self.id(),
                ask: "passphrase",
            });
        };
        let plugged = if keys(machine)?.is_empty() {
            vec![]
        } else {
            crate::observe::fido2::list(machine)?
        };
        let key = match plugged.len() {
            0 => VaultKey::open_with_passphrase(&sealed(machine, vault::KEY_PASSPHRASE)?, typed)?,
            1 => VaultKey::open_with_key(&sealed(machine, vault::KEY)?, typed)?,
            many => {
                return Err(Fault::Blocked {
                    step: self.id(),
                    why: format!("{many} keys are plugged in; leave only one"),
                });
            }
        };
        machine.write(VAULT_KEY, key.to_secret().expose().as_bytes())
    }
}
