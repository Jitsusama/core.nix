//! A machine's rescue bundle: its Secure Boot keys as sbctl keeps them,
//! sealed so either of its YubiKeys opens them, or the recovery passphrase
//! when neither can. It's a few KB of text and nothing but ciphertext, so
//! it can go anywhere: another machine, a stick's vault beside its sealed
//! files, anyone's disk. With
//! it, a host signs a stick the machine's firmware boots while enforcing
//! its own keys, and the stick puts those keys back on a reinstall, so the
//! firmware's setup screens are never visited. It's made on the machine
//! the keys belong to, which reads them as root and seals them in memory,
//! so they're never written down anywhere in the plain.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::rc::Rc;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

use crate::install::Target;
use crate::machine::{Given, Program};
use crate::sign::Signer;
use crate::vault::VaultKey;
use crate::{Answer, Answers, Ask, Fault, Machine, Place, Secret, Standing, Step, Tool};

/// Where sbctl keeps a machine's Secure Boot keys.
pub const SBCTL: &str = "/var/lib/sbctl";

/// What sbctl keeps there, every file of which a bundle carries: the
/// owner's GUID, and each of the platform, key exchange and db keys with
/// its certificate.
pub const KEYS: [&str; 7] = [
    "GUID",
    "keys/PK/PK.key",
    "keys/PK/PK.pem",
    "keys/KEK/KEK.key",
    "keys/KEK/KEK.pem",
    "keys/db/db.key",
    "keys/db/db.pem",
];

/// Where an installed machine keeps its own bundle, made by its install.
/// It's ciphertext, so its being on the machine gives nothing away.
pub const KEPT: &str = "/var/lib/guide/rescue-bundle.json";

/// What a machine's rescue bundle is called on a stick's vault, beside
/// its sealed files, where any host writing a stick can read it.
pub fn carried_as(machine: &str) -> String {
    format!("{machine}-rescue-bundle.json")
}

/// What opens a bundle besides its YubiKeys: a passphrase chosen for it,
/// or, for one an install made, the disk's recovery key.
pub const RECOVERY_PASSPHRASE: &str = "the recovery passphrase";
pub const RECOVERY_KEY: &str = "the disk's recovery key";

/// A bundle as it's written: who it's for and what opens it, in the
/// plain, and the keys sealed under a key of its own, which is sealed in
/// turn to the YubiKeys and, apart, to the passphrase, as the vault's is.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Written {
    format: u32,
    machine: String,
    /// The YubiKeys by the names the person knows them by, then what the
    /// passphrase is.
    opened_by: Vec<String>,
    /// The owner GUID the keys were made under, which isn't secret.
    owner: String,
    /// The SHA-256 of the db certificate, hex, so a machine can tell its
    /// own bundle still holds the keys it has now without opening it.
    db: String,
    to_keys: Option<String>,
    to_passphrase: String,
    keys: String,
    /// When it was sealed, in seconds since 1970, so of two copies of a
    /// machine's bundle the newer is kept; 0 in one from before it was said.
    #[serde(default)]
    sealed: u64,
}

/// The keys themselves, by their paths under sbctl's directory, held in
/// memory that's wiped as it's dropped.
pub struct Keys {
    files: BTreeMap<String, Zeroizing<Vec<u8>>>,
}

/// The SHA-256 of some bytes, in hex.
pub fn fingerprint(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// A fault in sealing or opening the bundle, said of the bundle rather
/// than the key it's sealed under.
fn of_the_bundle(fault: Fault) -> Fault {
    match fault {
        Fault::Unopened { why, .. } => Fault::Unopened {
            what: "the rescue bundle".into(),
            why,
        },
        Fault::Unkept { why, .. } => Fault::Unkept {
            path: "the rescue bundle".into(),
            why,
        },
        other => other,
    }
}

fn unreadable(why: impl std::fmt::Display) -> Fault {
    Fault::unreadable("the rescue bundle", why)
}

impl Keys {
    /// sbctl's files, refused unless every one is there.
    pub fn new(files: BTreeMap<String, Zeroizing<Vec<u8>>>) -> Result<Keys, Fault> {
        if let Some(missing) = KEYS.iter().find(|name| !files.contains_key(**name)) {
            return Err(Fault::Blocked {
                step: "seal-keys",
                why: format!("the Secure Boot keys have no {missing}, so they aren't sbctl's"),
            });
        }
        if let Some(extra) = files.keys().find(|name| !KEYS.contains(&name.as_str())) {
            return Err(Fault::Blocked {
                step: "seal-keys",
                why: format!("{extra} isn't one of sbctl's keys"),
            });
        }
        Ok(Keys { files })
    }

    /// The machine's own keys, each read as root, since sbctl lets no one
    /// else read them.
    pub fn read(machine: &mut dyn Machine) -> Result<Keys, Fault> {
        Keys::read_in(machine, SBCTL)
    }

    /// sbctl's keys from another directory, such as an install's.
    pub fn read_in(machine: &mut dyn Machine, directory: &str) -> Result<Keys, Fault> {
        let mut files = BTreeMap::new();
        for name in KEYS {
            let path = format!("{directory}/{name}");
            let ran = machine.run_given(
                Program::Tool(Tool::Cat),
                &[&path],
                Given {
                    prints_secret: true,
                    as_root: true,
                    ..Given::default()
                },
            )?;
            if ran.status != 0 {
                return Err(Fault::Blocked {
                    step: "seal-keys",
                    why: format!(
                        "there are no Secure Boot keys at {directory}: {}",
                        ran.refusal()
                    ),
                });
            }
            files.insert(name.to_string(), Zeroizing::new(ran.stdout));
        }
        Keys::new(files)
    }

    /// Each file, by its path under sbctl's directory.
    pub fn files(&self) -> impl Iterator<Item = (&str, &[u8])> {
        self.files
            .iter()
            .map(|(name, bytes)| (name.as_str(), bytes.as_slice()))
    }

    /// One of the keys' files, by its name in `KEYS`.
    pub(crate) fn file(&self, name: &str) -> &[u8] {
        self.files[name].as_slice()
    }

    /// The owner GUID the firmware knows the keys by.
    pub fn owner(&self) -> String {
        String::from_utf8_lossy(self.file("GUID"))
            .trim()
            .to_string()
    }

    /// What signs a stick's boot program for the machine: its db key.
    pub fn signer(&self) -> Result<Signer, Fault> {
        let text = |name: &str| {
            std::str::from_utf8(self.file(name))
                .map(|text| Zeroizing::new(text.to_string()))
                .map_err(|_| Fault::unreadable(name.to_string(), "not PEM"))
        };
        Signer::new(&text("keys/db/db.key")?, &text("keys/db/db.pem")?)
    }

    fn sealed(&self) -> Result<Zeroizing<Vec<u8>>, Fault> {
        let encoded: BTreeMap<&str, Zeroizing<String>> = self
            .files
            .iter()
            .map(|(name, bytes)| (name.as_str(), Zeroizing::new(STANDARD.encode(bytes))))
            .collect();
        let borrowed: BTreeMap<&str, &str> = encoded
            .iter()
            .map(|(name, text)| (*name, text.as_str()))
            .collect();
        let json = serde_json::to_vec(&borrowed).map_err(unreadable)?;
        Ok(Zeroizing::new(json))
    }

    fn unsealed(json: &[u8]) -> Result<Keys, Fault> {
        let encoded: BTreeMap<String, String> = serde_json::from_slice(json).map_err(unreadable)?;
        // Each moved into memory that's wiped, not copied.
        let encoded = encoded
            .into_iter()
            .map(|(name, text)| (name, Zeroizing::new(text)));
        let mut files = BTreeMap::new();
        for (name, text) in encoded {
            let bytes = STANDARD.decode(text.as_bytes()).map_err(unreadable)?;
            files.insert(name, Zeroizing::new(bytes));
        }
        Keys::new(files)
    }
}

impl PartialEq for Keys {
    fn eq(&self, other: &Keys) -> bool {
        self.files == other.files
    }
}

/// Which files, never what's in them.
impl std::fmt::Debug for Keys {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Keys")
            .field(&self.files.keys().collect::<Vec<_>>())
            .finish()
    }
}

/// A rescue bundle as it was read, before anything opens it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bundle {
    written: Written,
}

impl Bundle {
    /// The keys sealed for a machine: to each YubiKey's age recipient when
    /// it names them, and to the passphrase, `called` what it is to the
    /// person. `work` is scrypt's log₂ N, left to age but for tests.
    /// Refused unless the passphrase opens what was sealed and gives back
    /// the same keys.
    pub fn seal(
        target: &Target,
        keys: &Keys,
        passphrase: &Secret,
        called: &str,
        work: Option<u8>,
    ) -> Result<Vec<u8>, Fault> {
        if !target.recipients.is_empty() && target.recipients.len() != target.yubikeys.len() {
            return Err(Fault::Blocked {
                step: "seal-keys",
                why: format!(
                    "{} names {} YubiKeys and {} recipients for them",
                    target.machine,
                    target.yubikeys.len(),
                    target.recipients.len()
                ),
            });
        }
        let key = VaultKey::generate();
        let to_keys = if target.recipients.is_empty() {
            None
        } else {
            Some(STANDARD.encode(key.seal_to_keys(&target.recipients)?))
        };
        let mut opened_by = if to_keys.is_some() {
            target.yubikeys.clone()
        } else {
            vec![]
        };
        opened_by.push(called.into());
        let written = Written {
            format: 1,
            machine: target.machine.clone(),
            opened_by,
            owner: keys.owner(),
            db: fingerprint(keys.file("keys/db/db.pem")),
            to_keys,
            to_passphrase: STANDARD.encode(key.seal_with_passphrase(passphrase, work)?),
            keys: STANDARD.encode(key.encrypt(&keys.sealed()?)?),
            sealed: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |since| since.as_secs()),
        };
        let mut bytes = serde_json::to_vec_pretty(&written).map_err(unreadable)?;
        bytes.push(b'\n');
        if Bundle::read(&bytes)?.open_with_passphrase(passphrase)? != *keys {
            return Err(Fault::Unproven {
                step: "seal-keys",
                standing: "the passphrase didn't open the same keys from the bundle".into(),
            });
        }
        Ok(bytes)
    }

    pub fn read(bytes: &[u8]) -> Result<Bundle, Fault> {
        let written: Written = serde_json::from_slice(bytes).map_err(unreadable)?;
        if written.format != 1 {
            return Err(unreadable(format!(
                "it's format {}, and this guide reads format 1",
                written.format
            )));
        }
        Ok(Bundle { written })
    }

    /// The machine whose keys these are.
    pub fn machine(&self) -> &str {
        &self.written.machine
    }

    /// The YubiKeys that open it, by name, the passphrase left out.
    pub fn yubikeys(&self) -> Vec<String> {
        match (&self.written.to_keys, self.written.opened_by.split_last()) {
            (Some(_), Some((_, keys))) => keys.to_vec(),
            _ => vec![],
        }
    }

    /// What its passphrase is: the recovery passphrase, or the disk's
    /// recovery key.
    pub fn passphrase(&self) -> &str {
        self.written
            .opened_by
            .last()
            .map_or(RECOVERY_PASSPHRASE, String::as_str)
    }

    /// When it was sealed, in seconds since 1970, or 0 when it doesn't say.
    /// Nothing vouches for it: whoever seals a bundle says it.
    pub fn sealed(&self) -> u64 {
        self.written.sealed
    }

    /// Whether it holds these keys, which a machine can tell without
    /// opening it, from the db certificate's fingerprint.
    pub fn holds(&self, db_certificate: &[u8]) -> bool {
        self.written.db == fingerprint(db_certificate)
    }

    fn opened(&self, key: VaultKey) -> Result<Keys, Fault> {
        let sealed = STANDARD.decode(&self.written.keys).map_err(unreadable)?;
        let keys = Keys::unsealed(&key.decrypt(&sealed).map_err(of_the_bundle)?)?;
        if !self.holds(keys.file("keys/db/db.pem")) || keys.owner() != self.written.owner {
            return Err(unreadable("its keys aren't the ones it says it holds"));
        }
        Ok(keys)
    }

    pub fn open_with_passphrase(&self, passphrase: &Secret) -> Result<Keys, Fault> {
        let sealed = STANDARD
            .decode(&self.written.to_passphrase)
            .map_err(unreadable)?;
        // Tried as it's typed, then as the words it says, since the guide
        // makes every passphrase it seals to as words.
        let opened = match VaultKey::open_with_passphrase(&sealed, passphrase) {
            Err(_) if crate::words::typed(passphrase.expose()) != passphrase.expose() => {
                let read = Secret::new(crate::words::typed(passphrase.expose()));
                VaultKey::open_with_passphrase(&sealed, &read)
            }
            opened => opened,
        };
        self.opened(opened.map_err(of_the_bundle)?)
    }

    /// With whichever of its YubiKeys is plugged in, and its PIN, which the
    /// key asks for along with a touch.
    pub fn open_with_key(&self, pin: &Secret) -> Result<Keys, Fault> {
        let Some(to_keys) = &self.written.to_keys else {
            return Err(Fault::Unopened {
                what: "the rescue bundle".into(),
                why: "it isn't sealed to any YubiKey".into(),
            });
        };
        let sealed = STANDARD.decode(to_keys).map_err(unreadable)?;
        self.opened(VaultKey::open_with_key(&sealed, pin).map_err(of_the_bundle)?)
    }
}

/// The names, as a person says them: "the Nano", "the Nano or the 5Ci".
fn either(names: &[String]) -> String {
    match names {
        [] => String::new(),
        [one] => one.clone(),
        [rest @ .., last] => format!("{} or {last}", rest.join(", ")),
    }
}

/// The machine's rescue bundle made where it can be taken from: its keys
/// read as root, sealed in memory to seven words kept on the person's
/// phone and proven to open again, then written out as the person. Done
/// once a bundle there holds the keys the machine has now.
pub struct SealKeys {
    pub target: Target,
    pub bundle: PathBuf,
    /// The passphrase's words, made once, so those shown are those typed
    /// back and those the bundle is sealed to.
    pub words: std::cell::OnceCell<Secret>,
}

impl SealKeys {
    pub fn new(target: Target, bundle: PathBuf) -> SealKeys {
        SealKeys {
            target,
            bundle,
            words: std::cell::OnceCell::new(),
        }
    }

    fn words(&self) -> Result<&Secret, Fault> {
        if self.words.get().is_none() {
            let _ = self.words.set(crate::words::made()?);
        }
        Ok(self.words.get().expect("made just now"))
    }

    fn written(&self, machine: &mut dyn Machine) -> Result<Option<Bundle>, Fault> {
        match machine.read(&self.bundle.display().to_string())? {
            Some(bytes) => Ok(Bundle::read(&bytes).ok()),
            None => Ok(None),
        }
    }
}

impl Step for SealKeys {
    fn id(&self) -> &'static str {
        "seal-keys"
    }

    fn place(&self) -> Place {
        Place::Installed
    }

    fn title(&self) -> &'static str {
        "Make a rescue bundle of its Secure Boot keys"
    }

    fn observe(&self, machine: &mut dyn Machine) -> Result<Standing, Fault> {
        if let Some(bundle) = self.written(machine)?
            && bundle.machine() == self.target.machine
        {
            let keys = Keys::read(machine)?;
            if bundle.holds(keys.file("keys/db/db.pem")) {
                return Ok(Standing::Done {
                    evidence: format!(
                        "{} holds {}'s Secure Boot keys, opened by {}",
                        self.bundle.display(),
                        self.target.machine,
                        either(&bundle.written.opened_by)
                    ),
                });
            }
        }
        let words = self.words()?.expose();
        Ok(Standing::Needed {
            asks: vec![
                Ask::Show {
                    id: "shown",
                    how: crate::words::keeping(
                        "the bundle's passphrase",
                        &format!("{} rescue bundle passphrase", self.target.machine),
                        &format!(
                            "The bundle's passphrase opens {}'s Secure Boot keys when no YubiKey will",
                            self.target.machine
                        ),
                    ),
                    shown: words.into(),
                },
                Ask::Recall {
                    id: "passphrase",
                    expected: words.into(),
                    why: "type the bundle's passphrase, reading it off your phone".into(),
                },
            ],
        })
    }

    fn run(&self, machine: &mut dyn Machine, answers: &Answers) -> Result<(), Fault> {
        let passphrase = self.words()?;
        match answers.get("passphrase") {
            Some(Answer::Typed(typed))
                if crate::words::typed(typed) == crate::words::typed(passphrase.expose()) => {}
            Some(Answer::Typed(_)) => {
                return Err(Fault::Blocked {
                    step: self.id(),
                    why: "what was typed isn't the bundle's passphrase".into(),
                });
            }
            _ => {
                return Err(Fault::Unanswered {
                    step: self.id(),
                    ask: "passphrase",
                });
            }
        }
        let keys = Keys::read(machine)?;
        let bytes = Bundle::seal(&self.target, &keys, passphrase, RECOVERY_PASSPHRASE, None)?;
        machine.write(&self.bundle.display().to_string(), &bytes)
    }
}

/// A rescue bundle opened on the host writing a stick, with a YubiKey's
/// PIN or, with none plugged in, the passphrase. The keys are held here,
/// in memory, for the stick's writing to take.
pub struct OpenRescueBundle {
    pub bundle: Bundle,
    pub opened: Rc<RefCell<Option<Keys>>>,
}

impl Step for OpenRescueBundle {
    fn id(&self) -> &'static str {
        "open-rescue-bundle"
    }

    fn place(&self) -> Place {
        Place::Host
    }

    fn title(&self) -> &'static str {
        "Open the machine's rescue bundle"
    }

    fn observe(&self, _: &mut dyn Machine) -> Result<Standing, Fault> {
        if let Some(keys) = self.opened.borrow().as_ref() {
            return Ok(Standing::Done {
                evidence: format!(
                    "{}'s Secure Boot keys are open, owned by {}",
                    self.bundle.machine(),
                    keys.owner()
                ),
            });
        }
        let keys = self.bundle.yubikeys();
        let passphrase = self.bundle.passphrase();
        if keys.is_empty() {
            return Ok(Standing::Needed {
                asks: vec![Ask::Secret {
                    id: "passphrase",
                    name: format!("{passphrase}, which opens the rescue bundle"),
                }],
            });
        }
        let named = either(&keys);
        Ok(Standing::Needed {
            asks: vec![
                Ask::Act {
                    id: "key",
                    what: format!(
                        "Plug in {named}, and touch it when it blinks. To use {passphrase} instead, leave it out."
                    ),
                },
                Ask::Secret {
                    id: "passphrase",
                    name: format!("the PIN of {named}, or with none plugged in, {passphrase}"),
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
        let plugged = if self.bundle.yubikeys().is_empty() {
            vec![]
        } else {
            crate::observe::fido2::list(machine)?
        };
        let keys = match plugged.len() {
            0 => self.bundle.open_with_passphrase(typed)?,
            1 => self.bundle.open_with_key(typed)?,
            many => {
                return Err(Fault::Blocked {
                    step: self.id(),
                    why: format!("{many} keys are plugged in; leave only one"),
                });
            }
        };
        self.opened.replace(Some(keys));
        Ok(())
    }
}
