//! The spare SSH key on each YubiKey, for when the TPM is wiped or the
//! laptop is gone: a resident key, so `ssh-keygen -K` brings it back on
//! any machine. A YubiKey already holding one, from another machine, has
//! it downloaded rather than made again, since a new one would leave
//! every place the old one was added holding a key nothing can use.
//!
//! The YubiKeys are the ones the machine's rescue bundle names, asked for
//! one at a time by those names. The installed machine can't tell one
//! from another by name, so the first time a key gives its spare, the
//! guide remembers which of the disk's credentials it holds under that
//! name, and a key that holds another name's is refused.
//!
//! ssh-keygen runs as root, in memory: at boot nobody is signed in, so
//! the account can't reach the key. The files go to the account after.

use serde::Deserialize;

use super::{facts, secret, volume};
use crate::install::keys::{holds, slug};
use crate::observe::{fido2, luks};
use crate::{Ask, Fault, Given, Machine, Place, Program, Secret, Standing, Step, Tool, rescue};

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

pub struct SpareKeys;

const ID: &str = "spare-keys";

/// The YubiKeys the machine's rescue bundle names, none when it has no
/// bundle or its bundle names none.
fn named(machine: &mut dyn Machine) -> Result<Vec<String>, Fault> {
    Ok(match machine.read(rescue::KEPT)? {
        Some(bytes) => rescue::Bundle::read(&bytes)?.yubikeys(),
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

/// What's still to do: the first spare missing, by its account and its
/// YubiKey's name, or why nothing can be done.
enum Next {
    Nothing { evidence: String },
    Make { spare: Spare, name: String },
    Refused(String),
}

fn next(machine: &mut dyn Machine) -> Result<Next, Fault> {
    let spares = facts(machine)?.spares;
    let names = named(machine)?;
    if spares.is_empty() {
        return Ok(Next::Nothing {
            evidence: "no account here keeps a spare SSH key on its YubiKeys".into(),
        });
    }
    if names.is_empty() {
        return Ok(Next::Nothing {
            evidence: "this machine's rescue bundle names no YubiKeys to keep a spare SSH key on"
                .into(),
        });
    }
    for spare in &spares {
        for name in &names {
            match kept(machine, &spare.file(name))? {
                Kept::Whole => {}
                Kept::Missing => {
                    return Ok(Next::Make {
                        spare: spare.clone(),
                        name: name.clone(),
                    });
                }
                Kept::Mismatched(why) => return Ok(Next::Refused(why)),
            }
        }
    }
    let accounts: Vec<&str> = spares.iter().map(|spare| spare.account.as_str()).collect();
    Ok(Next::Nothing {
        evidence: format!(
            "{} each hold {}'s spare SSH key",
            names.join(" and "),
            accounts.join(" and ")
        ),
    })
}

impl Step for SpareKeys {
    fn id(&self) -> &'static str {
        ID
    }

    fn place(&self) -> Place {
        Place::Installed
    }

    fn title(&self) -> &'static str {
        "Keep a spare SSH key on each YubiKey"
    }

    fn observe(&self, machine: &mut dyn Machine) -> Result<Standing, Fault> {
        Ok(match next(machine)? {
            Next::Nothing { evidence } => Standing::Done { evidence },
            Next::Refused(why) => Standing::Blocked {
                why,
                remedy: "Move both halves somewhere else, then look again, and the guide brings the spare back from the YubiKey.".into(),
            },
            Next::Make { name, .. } => Standing::Needed {
                asks: vec![
                    Ask::Act {
                        id: "plugged",
                        what: format!(
                            "Plug in {name}, and only {name}. Touch it if it blinks."
                        ),
                    },
                    Ask::Secret {
                        id: "pin",
                        name: format!("{name}'s PIN"),
                    },
                ],
            },
        })
    }

    fn run(&self, machine: &mut dyn Machine, answers: &crate::Answers) -> Result<(), Fault> {
        let (spare, name) = match next(machine)? {
            Next::Nothing { .. } => return Ok(()),
            Next::Refused(why) => return Err(Fault::Blocked { step: ID, why }),
            Next::Make { spare, name } => (spare, name),
        };
        let pin = secret(answers, "pin").ok_or(Fault::Unanswered {
            step: ID,
            ask: "pin",
        })?;
        let device = only_plugged(machine, &name)?;
        let credential = which(machine, &device, &name)?;

        machine.remove_tree(MAKING)?;
        let made = spare_from(machine, &device, pin, &spare, &name);
        let moved = made.and_then(|key| keep(machine, &key, &spare, &name));
        machine.remove_tree(MAKING)?;
        moved?;
        machine
            .journal()
            .record(ID, &format!("{}:{credential}", slug(&name)))
    }
}

/// The one YubiKey plugged in, or why it isn't there to use.
fn only_plugged(machine: &mut dyn Machine, name: &str) -> Result<String, Fault> {
    match fido2::plugged(machine)?.as_slice() {
        [one] => Ok(one.path.clone()),
        [] => Err(Fault::Blocked {
            step: ID,
            why: format!("no key is plugged in; plug in {name}"),
        }),
        many => Err(Fault::Blocked {
            step: ID,
            why: format!("{} keys are plugged in; leave only {name}", many.len()),
        }),
    }
}

/// Which of the disk's credentials the key at `device` holds, refused
/// when it holds none, so isn't one of the person's, or holds one the
/// guide has already met under another name.
fn which(machine: &mut dyn Machine, device: &str, name: &str) -> Result<String, Fault> {
    let facts = facts(machine)?;
    let header = luks::observe_as_root(machine, &volume(&facts))?;
    let credentials: Vec<String> = header.credentials().into_iter().map(String::from).collect();
    let mut held = None;
    for credential in credentials {
        if holds(machine, device, &credential)? {
            held = Some(credential);
            break;
        }
    }
    let Some(credential) = held else {
        return Err(Fault::Blocked {
            step: ID,
            why: format!("this key doesn't open the disk, so it isn't {name}"),
        });
    };
    for other in named(machine)? {
        if other != name
            && machine
                .journal()
                .knows(ID, &format!("{}:{credential}", slug(&other)))
        {
            return Err(Fault::Blocked {
                step: ID,
                why: format!("this key is {other}, not {name}; plug in {name}"),
            });
        }
    }
    Ok(credential)
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
