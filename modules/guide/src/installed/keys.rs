//! The SSH keys an account keeps in the TPM, made on the machine it
//! installed, as the account, so no private key is ever on the disk:
//! ssh-tpm-agent's login key and the signing keys, each named by the
//! module that wants it (core's `jitsusama.tpmKeys`) and listed in the
//! machine's facts. One PIN, typed twice, goes to every key that has one.
//!
//! A key's file holds its private half sealed to the storage key its TPM's
//! seed makes, and clearing the TPM gives it another seed, so a key made
//! before can never load again. The guide loads each key to see, and makes
//! one the TPM refuses again, as it makes a missing one.

use serde::Deserialize;

use super::{as_root, facts, secret};
use crate::observe::keyfile::{self, KeyFile};
use crate::{Ask, Fault, Given, Machine, Place, Program, Secret, Standing, Step, Tool};

/// What the guide prints, and nothing else, when ssh-tpm-keygen runs it
/// as its `SSH_ASKPASS`: the PIN the keys are made with, set only in the
/// keygen's own environment, which no other account can read.
pub const ASKPASS: &str = "GUIDE_ASKPASS";

/// A key an account keeps in the TPM, as core's guide module lists it.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct TpmKey {
    /// Whose key it is.
    pub account: String,
    /// Where it's kept, without the `.tpm` and `.pub` ssh-tpm-keygen adds.
    pub file: String,
    /// What its public half says it is, such as `alice@machine signing`.
    pub comment: String,
    /// Whether it asks for the PIN before it's used.
    pub pin: bool,
}

impl TpmKey {
    fn private(&self) -> String {
        format!("{}.tpm", self.file)
    }

    fn public(&self) -> String {
        format!("{}.pub", self.file)
    }

    /// What a person calls it: the file's own name.
    fn name(&self) -> &str {
        self.file.rsplit('/').next().unwrap_or(&self.file)
    }
}

pub struct MakeKeys;

/// Where the guide loads a key to see the TPM still takes it: root's
/// alone, and gone once it's looked.
const LOADING: &str = "/run/guide/tpm-key";
const PUBLIC: &str = "/run/guide/tpm-key/public";
const PRIVATE: &str = "/run/guide/tpm-key/private";

/// The storage key go-tpm-keyfiles loads a key under when its parent is
/// the owner: the TCG's ECC template H-2 (its tpm.go's
/// `ECCSRK_H2_Template`), which the owner's seed makes the same every
/// time, until the TPM is cleared.
const STORAGE_KEY: &[&str] = &[
    "-T",
    "device:/dev/tpmrm0",
    "-C",
    "o",
    "-g",
    "sha256",
    "-G",
    "ecc256:aes128cfb",
    "-a",
    "fixedtpm|fixedparent|sensitivedataorigin|userwithauth|noda|restricted|decrypt",
    "-c",
    "/run/guide/tpm-key/parent.ctx",
];

/// Loads the key's halves under that storage key.
const LOAD: &[&str] = &[
    "-T",
    "device:/dev/tpmrm0",
    "-C",
    "/run/guide/tpm-key/parent.ctx",
    "-u",
    PUBLIC,
    "-r",
    PRIVATE,
    "-c",
    "/run/guide/tpm-key/key.ctx",
];

/// What `tpm2_load` says when the TPM finds a key's private half wasn't
/// sealed by the storage key it's loaded under: `TPM_RC_INTEGRITY` on
/// its first parameter, that half. Any other refusal says nothing of the
/// key, so it's a fault rather than a key to make again.
const NOT_ITS_SEED: &str = "Esys_Load(0x1DF)";

/// How a key stands on the disk.
enum Kept {
    /// Both halves, the public one read from the private one, which the
    /// TPM loads.
    Whole,
    /// Neither half, so it's to make.
    Missing,
    /// Both halves, but the TPM, cleared since, refuses the private one,
    /// so it's to make again.
    Lost,
    /// One half, or a public half that isn't the private one's: something
    /// nothing should overwrite.
    Mismatched(String),
}

fn kept(machine: &mut dyn Machine, key: &TpmKey, storage: &mut bool) -> Result<Kept, Fault> {
    let private = machine.read(&key.private())?;
    let public = machine.read(&key.public())?;
    match (private, public) {
        (None, None) => Ok(Kept::Missing),
        (Some(_), None) => Ok(Kept::Mismatched(format!(
            "{} is here without its public half, {}",
            key.private(),
            key.public()
        ))),
        (None, Some(_)) => Ok(Kept::Mismatched(format!(
            "{} is here without the key in the TPM it's the public half of, {}",
            key.public(),
            key.private()
        ))),
        (Some(private), Some(public)) => {
            let read = machine
                .run_given(
                    Program::Tool(Tool::SshTpmKeygen),
                    &["--print-pubkey", &key.private()],
                    Given::default(),
                )?
                .ok()?;
            if read.trim() != String::from_utf8_lossy(&public).trim() {
                Ok(Kept::Mismatched(format!(
                    "{} isn't the public half of {}",
                    key.public(),
                    key.private()
                )))
            } else if loads(machine, &private, storage)? {
                Ok(Kept::Whole)
            } else {
                Ok(Kept::Lost)
            }
        }
    }
}

/// Whether the TPM still loads a key, which one cleared since it was made
/// won't. A key under any parent but the owner's storage key, or not one
/// to load, is taken as it stands, since the guide can't tell. `storage`
/// says whether that storage key is made already, once for every key.
fn loads(machine: &mut dyn Machine, file: &[u8], storage: &mut bool) -> Result<bool, Fault> {
    let key = KeyFile::parse(file)?;
    if !key.loadable || key.parent != keyfile::OWNER {
        return Ok(true);
    }
    machine.write(PUBLIC, &key.public)?;
    machine.write(PRIVATE, &key.private)?;
    if !*storage {
        machine
            .run_given(
                Program::Tool(Tool::Tpm2Createprimary),
                STORAGE_KEY,
                as_root(),
            )?
            .ok()?;
        *storage = true;
    }
    let loaded = machine.run_given(Program::Tool(Tool::Tpm2Load), LOAD, as_root())?;
    if loaded.status == 0 {
        Ok(true)
    } else if String::from_utf8_lossy(&loaded.stderr).contains(NOT_ITS_SEED) {
        Ok(false)
    } else {
        Err(loaded.refusal())
    }
}

/// The accounts whose keys in `keys` have a PIN, as a person reads them.
fn whose(keys: &[&TpmKey]) -> String {
    let mut accounts: Vec<&str> = keys
        .iter()
        .filter(|key| key.pin)
        .map(|key| key.account.as_str())
        .collect();
    accounts.dedup();
    accounts.join(" and ")
}

/// A key to make, and whether a cleared TPM lost the one there.
struct ToMake {
    key: TpmKey,
    lost: bool,
}

/// The keys still to make, or why none can be.
fn to_make(machine: &mut dyn Machine) -> Result<Result<Vec<ToMake>, String>, Fault> {
    machine.remove_tree(LOADING)?;
    let made = to_make_looking(machine);
    machine.remove_tree(LOADING)?;
    made
}

fn to_make_looking(machine: &mut dyn Machine) -> Result<Result<Vec<ToMake>, String>, Fault> {
    let mut storage = false;
    let mut missing = vec![];
    for key in facts(machine)?.keys {
        match kept(machine, &key, &mut storage)? {
            Kept::Whole => {}
            Kept::Missing => missing.push(ToMake { key, lost: false }),
            Kept::Lost => missing.push(ToMake { key, lost: true }),
            Kept::Mismatched(why) => return Ok(Err(why)),
        }
    }
    Ok(Ok(missing))
}

impl Step for MakeKeys {
    fn id(&self) -> &'static str {
        "make-keys"
    }

    fn place(&self) -> Place {
        Place::Installed
    }

    fn title(&self) -> &'static str {
        "Make the SSH keys in the TPM"
    }

    fn observe(&self, machine: &mut dyn Machine) -> Result<Standing, Fault> {
        let keys = facts(machine)?.keys;
        let missing = match to_make(machine)? {
            Ok(missing) => missing,
            Err(why) => {
                return Ok(Standing::Blocked {
                    why,
                    remedy: "Move both halves somewhere else, then look again, and the guide makes the key again.".into(),
                });
            }
        };
        if missing.is_empty() {
            let names: Vec<&str> = keys.iter().map(TpmKey::name).collect();
            return Ok(Standing::Done {
                evidence: if names.is_empty() {
                    "no account here keeps an SSH key in the TPM".into()
                } else {
                    format!("the TPM holds the SSH keys {}", names.join(", "))
                },
            });
        }
        let lost = missing.iter().any(|making| making.lost);
        let missing: Vec<&TpmKey> = missing.iter().map(|making| &making.key).collect();
        if !missing.iter().any(|key| key.pin) {
            return Ok(Standing::Needed { asks: vec![] });
        }
        let name = if lost {
            format!(
                "a PIN for {}'s new SSH keys, made again since the TPM was cleared and can't open the old ones; they ask for it once after each sign-in",
                whose(&missing)
            )
        } else {
            format!(
                "a PIN for {}'s SSH keys, which they ask for once after each sign-in",
                whose(&missing)
            )
        };
        Ok(Standing::Needed {
            asks: vec![
                Ask::Secret {
                    id: "key-pin",
                    name,
                },
                Ask::Secret {
                    id: "again",
                    name: "the same PIN again".into(),
                },
            ],
        })
    }

    fn run(&self, machine: &mut dyn Machine, answers: &crate::Answers) -> Result<(), Fault> {
        let missing = match to_make(machine)? {
            Ok(missing) => missing,
            Err(why) => {
                return Err(Fault::Blocked {
                    step: self.id(),
                    why,
                });
            }
        };
        let none = Secret::new(String::new());
        let pin = if missing.iter().any(|making| making.key.pin) {
            let (Some(pin), Some(again)) = (secret(answers, "key-pin"), secret(answers, "again"))
            else {
                return Err(Fault::Unanswered {
                    step: self.id(),
                    ask: "key-pin",
                });
            };
            refused(pin, again).map_or(Ok(pin), |why| {
                Err(Fault::Blocked {
                    step: self.id(),
                    why: why.into(),
                })
            })?
        } else {
            &none
        };
        let keygen = Tool::SshTpmKeygen.program().ok_or(Fault::Unbuilt {
            tool: Tool::SshTpmKeygen.name(),
        })?;
        let mkdir = Tool::Mkdir.program().ok_or(Fault::Unbuilt {
            tool: Tool::Mkdir.name(),
        })?;
        // ssh-tpm-keygen asks for the PIN only on a terminal, and through
        // pipes makes the key with none, so it asks the guide instead.
        let askpass = std::env::current_exe()
            .map_err(|why| Fault::Unstarted {
                program: Tool::SshTpmKeygen.name().into(),
                why: format!("the guide couldn't find itself to answer for the PIN: {why}"),
            })?
            .to_string_lossy()
            .into_owned();
        for ToMake { key, lost } in &missing {
            // Nothing can ever load a key the TPM's old seed sealed, and
            // ssh-tpm-keygen asks before making one over it.
            if *lost {
                machine.remove(&key.private())?;
                machine.remove(&key.public())?;
            }
            let folder = key.file.rsplit_once('/').map_or(".", |(folder, _)| folder);
            machine
                .run_given(
                    Program::Tool(Tool::Runuser),
                    &["-u", &key.account, "--", mkdir, "-p", "-m", "700", folder],
                    Given::default(),
                )?
                .ok()?;
            machine
                .run_given(
                    Program::Tool(Tool::Runuser),
                    &[
                        "-u",
                        &key.account,
                        "--",
                        keygen,
                        "-C",
                        &key.comment,
                        "-f",
                        &key.file,
                    ],
                    Given {
                        plain: vec![("SSH_ASKPASS", &askpass), ("SSH_ASKPASS_REQUIRE", "force")],
                        env: vec![(ASKPASS, if key.pin { pin } else { &none })],
                        ..Given::default()
                    },
                )?
                .ok()?;
        }
        Ok(())
    }
}

/// Why a PIN and its second typing can't make keys, if they can't.
fn refused(pin: &Secret, again: &Secret) -> Option<&'static str> {
    let typed = pin.expose();
    if typed != again.expose() {
        Some("the two PINs weren't the same")
    } else if typed.chars().count() < 4 {
        Some("a key's PIN has to be at least 4 characters")
    } else if typed.trim() != typed {
        // ssh-tpm-keygen trims what its askpass answers.
        Some("a key's PIN can't start or end with a space")
    } else {
        None
    }
}
