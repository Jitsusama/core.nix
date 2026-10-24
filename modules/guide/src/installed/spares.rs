//! The spare SSH key on each YubiKey, for when the TPM is wiped or the
//! laptop is gone: a resident key, so `ssh-keygen -K` brings it back on
//! any machine. A YubiKey already holding one, from another machine, has
//! it downloaded rather than made again, since a new one would leave
//! every place the old one was added holding a key nothing can use.
//!
//! The YubiKeys are the ones the stick enrolled, asked for one at a time
//! by name, and each known by the disk credential it enrolled, which the
//! stick wrote down (`install::keys::NAMED`): a key holding another
//! name's is refused, and so is one holding none.
//!
//! ssh-keygen runs as root, in memory: at boot nobody is signed in, so
//! the account can't reach the key. The files go to the account after.

use serde::Deserialize;

use super::{facts, secret};
use crate::install::keys::{NAMED, Named, holds, slug};
use crate::observe::fido2;
use crate::{Ask, Fault, Given, Machine, Place, Program, Secret, Standing, Step, Tool};

use super::keys::ASKPASS;

/// An account that keeps a spare on each YubiKey, as core's guide module
/// lists it.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct Spare {
    pub account: String,
    /// Where its SSH keys are, such as `/home/alice/.ssh`.
    pub folder: String,
    /// What the key's public half says, before the YubiKey's name.
    pub comment: String,
}

impl Spare {
    /// Where the spare from `name` is kept.
    fn file(&self, name: &str) -> String {
        format!("{}/yubikey-{}", self.folder, slug(name))
    }
}

/// Where ssh-keygen makes or downloads a key, in memory and root's alone.
const MAKING: &str = "/run/guide/spare";

/// What ssh-keygen names a downloaded key whose application is plain
/// `ssh:` and which has no user, by its kind, ed25519 first.
const DOWNLOADED: [&str; 2] = ["id_ed25519_sk_rk", "id_ecdsa_sk_rk"];

/// One YubiKey's spare, for the name the stick enrolled it under: a step
/// for each, as the stick enrols each, since each is plugged in and
/// unlocked with its own PIN.
pub struct SpareKey {
    name: String,
    id: &'static str,
}

impl SpareKey {
    pub fn new(name: &str) -> SpareKey {
        // A machine has a step for each of a handful of keys, made once,
        // so its id lives as long as the guide does.
        let id = Box::leak(format!("spare-{}", slug(name)).into_boxed_str());
        SpareKey {
            name: name.into(),
            id,
        }
    }
}

/// A step for each YubiKey the stick named here, and none on a machine
/// the guide didn't install.
pub fn steps(machine: &mut dyn Machine) -> Result<Vec<Box<dyn Step>>, Fault> {
    Ok(named(machine)?
        .iter()
        .map(|key| Box::new(SpareKey::new(&key.name)) as Box<dyn Step>)
        .collect())
}

/// The YubiKeys the stick enrolled, by name, none on a machine the
/// guide didn't install.
fn named(machine: &mut dyn Machine) -> Result<Vec<Named>, Fault> {
    Ok(match machine.read(NAMED)? {
        Some(bytes) => {
            serde_json::from_slice(&bytes).map_err(|why| Fault::unreadable(NAMED, why))?
        }
        None => vec![],
    })
}

/// A key's type and data, without its comment.
fn material(public: &str) -> String {
    public
        .split_whitespace()
        .take(2)
        .collect::<Vec<_>>()
        .join(" ")
}

enum Kept {
    Whole,
    Missing,
    Mismatched(String),
}

fn kept(machine: &mut dyn Machine, file: &str) -> Result<Kept, Fault> {
    let public = format!("{file}.pub");
    match (machine.read(file)?, machine.read(&public)?) {
        (None, None) => Ok(Kept::Missing),
        (Some(_), None) => Ok(Kept::Mismatched(format!(
            "{file} is here without its public half, {public}"
        ))),
        (None, Some(_)) => Ok(Kept::Mismatched(format!(
            "{public} is here without the key it's the public half of, {file}"
        ))),
        (Some(_), Some(bytes)) => {
            let read = machine.run(Tool::SshKeygen, &["-y", "-f", file])?.ok()?;
            let public = String::from_utf8_lossy(&bytes).into_owned();
            if material(&read) == material(&public) {
                Ok(Kept::Whole)
            } else {
                Ok(Kept::Mismatched(format!(
                    "{file}.pub isn't the public half of {file}"
                )))
            }
        }
    }
}

/// What's still to do on one YubiKey: the spare missing from it, by the
/// account keeping it, or why nothing can be done.
enum Next {
    Nothing { evidence: String },
    Make { spare: Spare, key: Named },
    Refused(String),
}

fn next(machine: &mut dyn Machine, name: &str) -> Result<Next, Fault> {
    let spares = facts(machine)?.spares;
    if spares.is_empty() {
        return Ok(Next::Nothing {
            evidence: "no account here keeps a spare SSH key on its YubiKeys".into(),
        });
    }
    let Some(key) = named(machine)?.into_iter().find(|key| key.name == name) else {
        return Ok(Next::Nothing {
            evidence: format!("the stick didn't name {name} among this machine's YubiKeys"),
        });
    };
    for spare in &spares {
        match kept(machine, &spare.file(name))? {
            Kept::Whole => {}
            Kept::Missing => {
                return Ok(Next::Make {
                    spare: spare.clone(),
                    key,
                });
            }
            Kept::Mismatched(why) => return Ok(Next::Refused(why)),
        }
    }
    let accounts: Vec<&str> = spares.iter().map(|spare| spare.account.as_str()).collect();
    Ok(Next::Nothing {
        evidence: format!("{name} holds {}'s spare SSH key", accounts.join(" and ")),
    })
}

impl Step for SpareKey {
    fn id(&self) -> &'static str {
        self.id
    }

    fn place(&self) -> Place {
        Place::Installed
    }

    fn title(&self) -> &'static str {
        "Keep a spare SSH key on a YubiKey"
    }

    fn observe(&self, machine: &mut dyn Machine) -> Result<Standing, Fault> {
        Ok(match next(machine, &self.name)? {
            Next::Nothing { evidence } => Standing::Done { evidence },
            Next::Refused(why) => Standing::Blocked {
                why,
                remedy: "Move both halves somewhere else, then look again, and the guide brings the spare back from the YubiKey.".into(),
            },
            Next::Make { key, .. } => Standing::Needed {
                asks: vec![
                    Ask::Act {
                        id: "plugged",
                        what: format!(
                            "Plug in {}, and only {}. Touch it if it blinks.",
                            key.name, key.name
                        ),
                    },
                    Ask::Secret {
                        id: "pin",
                        name: format!("{}'s PIN", key.name),
                    },
                ],
            },
        })
    }

    fn run(&self, machine: &mut dyn Machine, answers: &crate::Answers) -> Result<(), Fault> {
        let step = self.id;
        let (spare, key) = match next(machine, &self.name)? {
            Next::Nothing { .. } => return Ok(()),
            Next::Refused(why) => return Err(Fault::Blocked { step, why }),
            Next::Make { spare, key } => (spare, key),
        };
        let name = key.name.clone();
        let pin = secret(answers, "pin").ok_or(Fault::Unanswered { step, ask: "pin" })?;
        let device = only_plugged(machine, step, &name)?;
        is_the_one(machine, step, &device, &key)?;

        machine.remove_tree(MAKING)?;
        let made = spare_from(machine, &device, pin, &spare, &name);
        let moved = made.and_then(|file| keep(machine, &file, &spare, &name));
        machine.remove_tree(MAKING)?;
        moved
    }
}

/// The one YubiKey plugged in, or why it isn't there to use.
fn only_plugged(
    machine: &mut dyn Machine,
    step: &'static str,
    name: &str,
) -> Result<String, Fault> {
    match fido2::plugged(machine)?.as_slice() {
        [one] => Ok(one.path.clone()),
        [] => Err(Fault::Blocked {
            step,
            why: format!("no key is plugged in; plug in {name}"),
        }),
        many => Err(Fault::Blocked {
            step,
            why: format!("{} keys are plugged in; leave only {name}", many.len()),
        }),
    }
}

/// Whether the key at `device` is `key`: it holds the credential that
/// key enrolled, and when it doesn't, which it is instead, if it's one
/// of the person's at all.
fn is_the_one(
    machine: &mut dyn Machine,
    step: &'static str,
    device: &str,
    key: &Named,
) -> Result<(), Fault> {
    if holds(machine, device, &key.credential)? {
        return Ok(());
    }
    let name = &key.name;
    for other in named(machine)? {
        if other.name != *name && holds(machine, device, &other.credential)? {
            return Err(Fault::Blocked {
                step,
                why: format!("this key is {}, not {name}; plug in {name}", other.name),
            });
        }
    }
    Err(Fault::Blocked {
        step,
        why: format!("this key doesn't open the disk, so it isn't {name}"),
    })
}

/// The spare off the key at `device`, downloaded when the key holds one
/// and made on it when not, left in `MAKING`; the file it's in.
fn spare_from(
    machine: &mut dyn Machine,
    device: &str,
    pin: &Secret,
    spare: &Spare,
    name: &str,
) -> Result<String, Fault> {
    let program = |tool: Tool| tool.program().ok_or(Fault::Unbuilt { tool: tool.name() });
    let keygen = program(Tool::SshKeygen)?;
    // ssh-keygen asks for the PIN only on a terminal, unless told to ask
    // the guide, which answers with the PIN it's given.
    let askpass = std::env::current_exe()
        .map_err(|why| Fault::Unstarted {
            program: Tool::SshKeygen.name().into(),
            why: format!("the guide couldn't find itself to answer for the PIN: {why}"),
        })?
        .to_string_lossy()
        .into_owned();
    let given = || Given {
        plain: vec![("SSH_ASKPASS", &askpass), ("SSH_ASKPASS_REQUIRE", "force")],
        env: vec![(ASKPASS, pin)],
        ..Given::default()
    };
    machine
        .run(Tool::Mkdir, &["-p", "-m", "700", MAKING])?
        .ok()?;
    let option = format!("device={device}");
    // Its file's passphrase is empty, as -N says, or the guide would
    // answer that with the PIN too.
    machine
        .run_given(
            Program::Tool(Tool::Env),
            &["-C", MAKING, keygen, "-K", "-N", "", "-O", &option],
            given(),
        )?
        .ok()?;
    let comment = format!("{} {}", spare.comment, slug(name));
    for file in DOWNLOADED {
        let downloaded = format!("{MAKING}/{file}");
        if machine.read(&downloaded)?.is_some() {
            machine
                .run(
                    Tool::SshKeygen,
                    &["-c", "-C", &comment, "-P", "", "-f", &downloaded],
                )?
                .ok()?;
            return Ok(downloaded);
        }
    }
    let made = format!("{MAKING}/made");
    machine
        .run_given(
            Program::Tool(Tool::SshKeygen),
            &[
                "-t",
                "ed25519-sk",
                "-O",
                "resident",
                "-O",
                "verify-required",
                "-O",
                &option,
                "-N",
                "",
                "-C",
                &comment,
                "-f",
                &made,
            ],
            given(),
        )?
        .ok()?;
    Ok(made)
}

/// The spare in `key` put where the account keeps it, as its own, with
/// its folder made as the account if it isn't there.
fn keep(machine: &mut dyn Machine, key: &str, spare: &Spare, name: &str) -> Result<(), Fault> {
    let mkdir = Tool::Mkdir.program().ok_or(Fault::Unbuilt {
        tool: Tool::Mkdir.name(),
    })?;
    machine
        .run(
            Tool::Runuser,
            &[
                "-u",
                &spare.account,
                "--",
                mkdir,
                "-p",
                "-m",
                "700",
                &spare.folder,
            ],
        )?
        .ok()?;
    let file = spare.file(name);
    let public = format!("{file}.pub");
    let owner = format!("{}:", spare.account);
    machine
        .run(Tool::Chown, &[&owner, key, &format!("{key}.pub")])?
        .ok()?;
    // Never over a file that's there, which the guide only takes for
    // missing when neither half is.
    machine
        .run(Tool::Mv, &["-n", &format!("{key}.pub"), &public])?
        .ok()?;
    machine.run(Tool::Mv, &["-n", key, &file])?.ok()?;
    Ok(())
}
