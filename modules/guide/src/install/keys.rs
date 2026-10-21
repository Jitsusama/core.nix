//! The ways into the disk: each YubiKey by name, one at a time, and the
//! recovery key, each proven to open it before the journey goes on.

use base64::Engine;
use sha2::{Digest, Sha256};

use crate::install::{INSTALL_KEY, RECOVERY_KEY, held, target};
use crate::observe::{fido2, luks};
use crate::tool::Path;
use crate::{
    Answer, Answers, Ask, Fault, Given, Machine, Place, Program, Secret, Standing, Step, Tool,
};

/// The relying party systemd enrols LUKS credentials under.
const RELYING_PARTY: &str = "io.systemd.cryptsetup";

/// Whether the key at `device` holds this credential, asked without a
/// touch or a PIN: a key answers a silent assertion only for a credential
/// it made.
pub fn holds(machine: &mut dyn Machine, device: &str, credential: &str) -> Result<bool, Fault> {
    let hash =
        base64::engine::general_purpose::STANDARD.encode(Sha256::digest(b"which key is this"));
    let input = Secret::new(format!("{hash}\n{RELYING_PARTY}\n{credential}\n"));
    let ran = machine.run_given(
        Program::Tool(Tool::Fido2Assert),
        &["-G", "-t", "up=false", device],
        Given {
            stdin: Some(&input),
            ..Given::default()
        },
    )?;
    match ran.status {
        0 => Ok(true),
        _ if String::from_utf8_lossy(&ran.stderr).contains("FIDO_ERR_NO_CREDENTIALS") => Ok(false),
        _ => Err(ran.refusal()),
    }
}

fn slug(name: &str) -> String {
    name.to_lowercase()
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|part| !part.is_empty() && *part != "the")
        .collect::<Vec<_>>()
        .join("-")
}

/// One YubiKey, by the name the person knows it by, enrolled on its own
/// with its PIN and a touch, then proven to open the disk.
pub struct EnrolYubikey {
    name: String,
    id: &'static str,
}

impl EnrolYubikey {
    pub fn new(name: &str) -> EnrolYubikey {
        // A journey has a step for each of a handful of keys, made once,
        // so its id lives as long as the guide does.
        let id = Box::leak(format!("enrol-{}", slug(name)).into_boxed_str());
        EnrolYubikey {
            name: name.into(),
            id,
        }
    }
}

impl Step for EnrolYubikey {
    fn id(&self) -> &'static str {
        self.id
    }

    fn place(&self) -> Place {
        Place::Stick
    }

    fn title(&self) -> &'static str {
        "Enrol a YubiKey"
    }

    fn observe(&self, machine: &mut dyn Machine) -> Result<Standing, Fault> {
        let target = target(machine)?;
        let header = luks::observe(machine, &target.volume())?;
        let enrolled = header.credentials().into_iter().any(|credential| {
            machine
                .journal()
                .knows(self.id, &format!("opened:{credential}"))
        });
        Ok(if enrolled {
            Standing::Done {
                evidence: format!("{} opens the disk", self.name),
            }
        } else {
            Standing::Needed {
                asks: vec![
                    Ask::Act {
                        id: "plugged",
                        what: format!(
                            "Plug in {}, and only {}. Touch it each time it blinks.",
                            self.name, self.name
                        ),
                    },
                    Ask::Secret {
                        id: "pin",
                        name: format!("{}'s PIN", self.name),
                    },
                ],
            }
        })
    }

    fn run(&self, machine: &mut dyn Machine, answers: &Answers) -> Result<(), Fault> {
        let Some(Answer::Secret(pin)) = answers.get("pin") else {
            return Err(Fault::Unanswered {
                step: self.id,
                ask: "pin",
            });
        };
        let target = target(machine)?;
        let volume = target.volume();
        let plugged = fido2::list(machine)?;
        let device = match plugged.as_slice() {
            [one] => one.path.clone(),
            [] => {
                return Err(Fault::Blocked {
                    step: self.id,
                    why: format!("no key is plugged in; plug in {}", self.name),
                });
            }
            many => {
                return Err(Fault::Blocked {
                    step: self.id,
                    why: format!(
                        "{} keys are plugged in; leave only {}",
                        many.len(),
                        self.name
                    ),
                });
            }
        };
        let before: Vec<String> = luks::observe(machine, &volume)?
            .credentials()
            .into_iter()
            .map(String::from)
            .collect();
        for credential in &before {
            if holds(machine, &device, credential)? {
                return Err(Fault::Blocked {
                    step: self.id,
                    why: format!("this key opens the disk already, so it isn't {}", self.name),
                });
            }
        }

        let enrolled = machine.run_given(
            Program::Tool(Tool::SystemdCryptenroll),
            &[
                &format!("--unlock-key-file={INSTALL_KEY}"),
                &format!("--fido2-device={device}"),
                "--fido2-with-client-pin=yes",
                "--fido2-with-user-presence=yes",
                &volume,
            ],
            Given {
                credentials: vec![("cryptenroll.fido2-pin", pin)],
                ..Given::default()
            },
        )?;
        if enrolled.status != 0 {
            return Err(enrolled.refusal());
        }
        let after = luks::observe(machine, &volume)?;
        let new: Vec<&str> = after
            .credentials()
            .into_iter()
            .filter(|credential| !before.iter().any(|old| old == credential))
            .collect();
        let [credential] = new.as_slice() else {
            return Err(Fault::Unproven {
                step: self.id,
                standing: format!("{} new FIDO2 credentials on the disk, not one", new.len()),
            });
        };

        // Proven by opening the disk with it, as the boot will.
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
        machine
            .journal()
            .record(self.id, &format!("opened:{credential}"))
    }
}

/// A recovery key made for the disk and held in memory, to be shown.
pub struct MakeRecoveryKey;

impl Step for MakeRecoveryKey {
    fn id(&self) -> &'static str {
        "make-recovery-key"
    }

    fn place(&self) -> Place {
        Place::Stick
    }

    fn title(&self) -> &'static str {
        "Make a recovery key"
    }

    fn observe(&self, machine: &mut dyn Machine) -> Result<Standing, Fault> {
        let target = target(machine)?;
        let header = luks::observe(machine, &target.volume())?;
        let kept = machine.journal().knows("keep-recovery-key", "opened");
        Ok(
            if header.has(&luks::Kind::Recovery) && (kept || machine.read(RECOVERY_KEY)?.is_some())
            {
                Standing::Done {
                    evidence: "the disk has a recovery key".into(),
                }
            } else {
                Standing::Needed { asks: vec![] }
            },
        )
    }

    fn run(&self, machine: &mut dyn Machine, _answers: &Answers) -> Result<(), Fault> {
        let target = target(machine)?;
        let volume = target.volume();
        let unlock = format!("--unlock-key-file={INSTALL_KEY}");
        let mut args = vec![unlock.as_str()];
        // One made before whose words are lost goes, so no key nobody has
        // stays on the disk.
        if luks::observe(machine, &volume)?.has(&luks::Kind::Recovery) {
            args.push("--wipe-slot=recovery");
        }
        args.extend(["--recovery-key", volume.as_str()]);
        let made = machine.run_given(
            Program::Tool(Tool::SystemdCryptenroll),
            &args,
            Given {
                prints_secret: true,
                ..Given::default()
            },
        )?;
        if made.status != 0 {
            return Err(made.refusal());
        }
        let printed = zeroize::Zeroizing::new(made.stdout);
        let key = String::from_utf8_lossy(&printed)
            .split_whitespace()
            .last()
            .map(String::from)
            .ok_or_else(|| {
                Fault::unreadable(
                    "what systemd-cryptenroll printed",
                    "there's no recovery key in it",
                )
            })?;
        machine.write(RECOVERY_KEY, key.as_bytes())
    }
}

/// The recovery key shown, typed back so it's known to be written down,
/// and proven to open the disk.
pub struct KeepRecoveryKey;

impl Step for KeepRecoveryKey {
    fn id(&self) -> &'static str {
        "keep-recovery-key"
    }

    fn place(&self) -> Place {
        Place::Stick
    }

    fn title(&self) -> &'static str {
        "Keep the recovery key"
    }

    fn observe(&self, machine: &mut dyn Machine) -> Result<Standing, Fault> {
        if machine.journal().knows(self.id(), "opened") {
            return Ok(Standing::Done {
                evidence: "the recovery key opens the disk".into(),
            });
        }
        Ok(match held(machine, RECOVERY_KEY)? {
            Some(key) => Standing::Needed {
                asks: vec![Ask::TypeBack {
                    id: "key",
                    expected: key.expose().into(),
                    why: "keep it in 1Password: it opens the disk when no YubiKey will".into(),
                }],
            },
            None => Standing::Blocked {
                why: "the recovery key isn't held any more".into(),
                remedy: "restart from the stick, which makes another".into(),
            },
        })
    }

    fn run(&self, machine: &mut dyn Machine, answers: &Answers) -> Result<(), Fault> {
        let key = held(machine, RECOVERY_KEY)?.ok_or(Fault::Blocked {
            step: self.id(),
            why: "the recovery key isn't held any more".into(),
        })?;
        match answers.get("key") {
            Some(Answer::Typed(typed)) if typed.trim() == key.expose() => {}
            Some(Answer::Typed(_)) => {
                return Err(Fault::Blocked {
                    step: self.id(),
                    why: "what was typed isn't the recovery key".into(),
                });
            }
            _ => {
                return Err(Fault::Unanswered {
                    step: self.id(),
                    ask: "key",
                });
            }
        }
        let target = target(machine)?;
        let opened = machine.run_given(
            Program::Tool(Tool::Cryptsetup),
            &[
                "open",
                "--test-passphrase",
                "--key-file=-",
                &target.volume(),
            ],
            Given {
                stdin: Some(&key),
                ..Given::default()
            },
        )?;
        if opened.status != 0 {
            return Err(opened.refusal());
        }
        machine.journal().record(self.id(), "opened")?;
        machine.remove(RECOVERY_KEY)
    }
}
