//! Replacing a recovered machine's lost YubiKey: the one the person still
//! has told apart from the disk's other credentials, then a new one
//! enrolled and proven before the lost ones' slots are wiped.

use super::{Found, the_machine};
use crate::observe::luks;
use crate::tool::Path;
use crate::{
    Answer, Answers, Ask, Fault, Given, Machine, Place, Program, Secret, Standing, Step, Tool,
};

/// The YubiKey the person still has, told apart from the disk's others by
/// a silent assertion, so the ones it isn't are the lost ones.
pub struct IdentifyKeptKey;

fn credentials(machine: &mut dyn Machine) -> Result<(Found, Vec<String>, luks::Header), Fault> {
    let found = the_machine(machine)?.map_err(|standing| Fault::Blocked {
        step: "replacing a YubiKey",
        why: standing.to_string(),
    })?;
    let header = luks::observe(machine, &found.volume())?;
    let credentials = header.credentials().into_iter().map(String::from).collect();
    Ok((found, credentials, header))
}

/// The one key plugged in, or why there isn't one.
fn plugged(machine: &mut dyn Machine, step: &'static str) -> Result<String, Fault> {
    let plugged = crate::observe::fido2::plugged(machine)?;
    match plugged.as_slice() {
        [one] => Ok(one.path.clone()),
        [] => Err(Fault::Blocked {
            step,
            why: "no key is plugged in".into(),
        }),
        many => Err(Fault::Blocked {
            step,
            why: format!("{} keys are plugged in; leave only one", many.len()),
        }),
    }
}

impl Step for IdentifyKeptKey {
    fn id(&self) -> &'static str {
        "identify-kept-key"
    }

    fn place(&self) -> Place {
        Place::Stick
    }

    fn title(&self) -> &'static str {
        "Find the YubiKey you still have"
    }

    fn observe(&self, machine: &mut dyn Machine) -> Result<Standing, Fault> {
        let (_, credentials, _) = credentials(machine)?;
        if credentials.is_empty() {
            return Ok(Standing::Blocked {
                why: "no YubiKey opens this disk".into(),
                remedy: "open a root shell inside it and enrol one with systemd-cryptenroll".into(),
            });
        }
        if credentials
            .iter()
            .any(|credential| machine.journal().knows(self.id(), credential))
        {
            return Ok(Standing::Done {
                evidence: "the YubiKey you still have is known".into(),
            });
        }
        Ok(Standing::Needed {
            asks: vec![Ask::Act {
                id: "kept",
                what: format!(
                    "Plug in the YubiKey you still have, and only it.{} Nothing is asked of it.",
                    crate::install::memory::port_words(machine)
                ),
            }],
        })
    }

    fn run(&self, machine: &mut dyn Machine, _answers: &Answers) -> Result<(), Fault> {
        let (_, credentials, _) = credentials(machine)?;
        let device = plugged(machine, self.id())?;
        for credential in &credentials {
            if crate::install::keys::holds(machine, &device, credential)? {
                return machine.journal().record(self.id(), credential);
            }
        }
        Err(Fault::Blocked {
            step: self.id(),
            why: "this key doesn't open the disk".into(),
        })
    }
}

/// A new YubiKey enrolled and proven, then the lost ones' slots wiped, so
/// the disk never has fewer ways in than when it started.
pub struct ReplaceLostKey;

impl Step for ReplaceLostKey {
    fn id(&self) -> &'static str {
        "replace-lost-key"
    }

    fn place(&self) -> Place {
        Place::Stick
    }

    fn title(&self) -> &'static str {
        "Replace the lost YubiKey"
    }

    fn observe(&self, machine: &mut dyn Machine) -> Result<Standing, Fault> {
        let (_, credentials, _) = credentials(machine)?;
        let kept = credentials
            .iter()
            .find(|credential| machine.journal().knows("identify-kept-key", credential))
            .cloned();
        let added = credentials
            .iter()
            .find(|credential| machine.journal().knows(self.id(), credential))
            .cloned();
        if let (Some(kept), Some(added)) = (&kept, &added)
            && credentials
                .iter()
                .all(|credential| credential == kept || credential == added)
        {
            return Ok(Standing::Done {
                evidence: "the new YubiKey opens the disk, and the lost one no longer does".into(),
            });
        }
        if kept.is_none() {
            return Ok(Standing::Blocked {
                why: "the YubiKey you still have isn't known yet".into(),
                remedy: "plug it in and choose this repair again".into(),
            });
        }
        Ok(Standing::Needed {
            asks: vec![
                Ask::Act {
                    id: "new",
                    what: "Unplug it, and plug in the new YubiKey. Touch it each time it blinks."
                        .into(),
                },
                Ask::Secret {
                    id: "pin",
                    name: "the new YubiKey's PIN".into(),
                },
                Ask::Secret {
                    id: "recovery",
                    name: "the recovery key".into(),
                },
            ],
        })
    }

    fn run(&self, machine: &mut dyn Machine, answers: &Answers) -> Result<(), Fault> {
        let (Some(Answer::Secret(pin)), Some(Answer::Secret(typed))) =
            (answers.get("pin"), answers.get("recovery"))
        else {
            return Err(Fault::Unanswered {
                step: self.id(),
                ask: "pin",
            });
        };
        let recovery = &Secret::new(crate::words::typed(typed.expose()));
        let (found, before, header) = credentials(machine)?;
        let volume = found.volume();
        let kept = before
            .iter()
            .find(|credential| machine.journal().knows("identify-kept-key", credential))
            .cloned()
            .ok_or(Fault::Blocked {
                step: self.id(),
                why: "the YubiKey you still have isn't known".into(),
            })?;
        let device = plugged(machine, self.id())?;
        for credential in &before {
            if crate::install::keys::holds(machine, &device, credential)? {
                return Err(Fault::Blocked {
                    step: self.id(),
                    why: "this key opens the disk already, so it isn't a new one".into(),
                });
            }
        }

        // The new key first, proven, and only then the lost ones away.
        let enrolled = machine.run_given(
            Program::Tool(Tool::SystemdCryptenroll),
            &[
                &format!("--fido2-device={device}"),
                "--fido2-with-client-pin=yes",
                "--fido2-with-user-presence=yes",
                &volume,
            ],
            Given {
                env: vec![("PASSWORD", recovery)],
                credentials: vec![("cryptenroll.fido2-pin", pin)],
                ..Given::default()
            },
        )?;
        if enrolled.status != 0 {
            return Err(enrolled.refusal());
        }
        let after = luks::observe(machine, &volume)?;
        let new: Vec<String> = after
            .credentials()
            .into_iter()
            .filter(|credential| !before.iter().any(|old| old == credential))
            .map(String::from)
            .collect();
        let [added] = new.as_slice() else {
            return Err(Fault::Unproven {
                step: self.id(),
                standing: format!("{} new FIDO2 credentials on the disk, not one", new.len()),
            });
        };
        let typed = Secret::new(format!("{}\n", pin.expose()));
        let opened = machine.run_given(
            Program::Tool(Tool::Cryptsetup),
            &[
                "open",
                "--test-passphrase",
                "--token-only",
                "--token-type",
                "systemd-fido2",
                &volume,
            ],
            Given {
                plain: vec![("LD_LIBRARY_PATH", Path::Tokens.get()?)],
                stdin: Some(&typed),
                ..Given::default()
            },
        )?;
        if opened.status != 0 {
            return Err(opened.refusal());
        }
        machine.journal().record(self.id(), added)?;

        let lost: Vec<String> = header
            .tokens
            .iter()
            .filter(|token| matches!(&token.kind, luks::Kind::Fido2 { credential } if *credential != kept))
            .flat_map(|token| token.keyslots.iter().map(u32::to_string))
            .collect();
        if !lost.is_empty() {
            let wipe = format!("--wipe-slot={}", lost.join(","));
            let wiped = machine.run_given(
                Program::Tool(Tool::SystemdCryptenroll),
                &[&wipe, &volume],
                Given {
                    env: vec![("PASSWORD", recovery)],
                    ..Given::default()
                },
            )?;
            if wiped.status != 0 {
                return Err(wiped.refusal());
            }
        }
        Ok(())
    }
}
