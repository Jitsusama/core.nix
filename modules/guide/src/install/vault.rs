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

/// The vault opened with its recovery passphrase, its key held in memory
/// for the steps after. Opening it with a YubiKey comes with the age
/// plugin's prompts.
pub struct OpenVault;

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
        let store = machine.vault()?;
        if !store.has(vault::KEY_PASSPHRASE)? {
            return Ok(Standing::Blocked {
                why: "this vault has no copy of its key a passphrase opens".into(),
                remedy: "write the stick again from a machine that has its recovery passphrase"
                    .into(),
            });
        }
        Ok(Standing::Needed {
            asks: vec![Ask::Secret {
                id: "passphrase",
                name: "the vault's recovery passphrase".into(),
            }],
        })
    }

    fn run(&self, machine: &mut dyn Machine, answers: &Answers) -> Result<(), Fault> {
        let Some(Answer::Secret(passphrase)) = answers.get("passphrase") else {
            return Err(Fault::Unanswered {
                step: self.id(),
                ask: "passphrase",
            });
        };
        let mut sealed = vec![];
        machine
            .vault()?
            .open(vault::KEY_PASSPHRASE)?
            .read_to_end(&mut sealed)
            .map_err(|why| Fault::unreadable("the vault's sealed key", why))?;
        let key = VaultKey::open_with_passphrase(&sealed, passphrase)?;
        machine.write(VAULT_KEY, key.to_secret().expose().as_bytes())
    }
}
