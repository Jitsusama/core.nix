//! The first two steps on the stick: the keyboard typing what's meant, and
//! the vault open.

use crate::install::{VAULT_KEY, vault_key};
use crate::observe::fido2;
use crate::vault::{self, Vault, VaultKey};
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
                        "Plug in {named}, and touch it when it blinks, or use the recovery passphrase instead."
                    ),
                },
                Ask::Secret {
                    id: "passphrase",
                    name: format!("the PIN of {named}, or the recovery passphrase"),
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
        machine.write(VAULT_KEY, key.to_secret().expose().as_bytes())
    }
}
