//! The host's part: writing a stick, from a machine's image and, for a
//! reinstall stick, its payload (payload.nix) sealed into the vault with
//! the recovery passphrase. The stick is a whole USB or removable disk,
//! only ever one plugged in at once, opened for writing through the
//! system's own way of letting a person do so (stick::device), so nothing
//! on a host runs as root.

use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use crate::install::{LAYOUT_SCRIPT, SYSTEM_CACHE, TARGET, Target};
use crate::observe::disks::{self, Disk, Kind};
use crate::rescue::Keys;
use crate::sign::Signer;
use crate::stick::{self, Image};
use crate::vault::{self, Manifest, Packer, Vault, VaultKey};
use crate::{Answer, Answers, Ask, Fault, Machine, Place, Secret, Standing, Step, words};

fn unread(path: &Path) -> impl Fn(std::io::Error) -> Fault + '_ {
    move |why| Fault::unreadable(path.display().to_string(), why)
}

/// What a reinstall stick carries for one machine, as payload.nix built it.
pub struct Payload {
    root: PathBuf,
    pub target: Target,
    /// The cache's files, relative to it.
    cached: Vec<PathBuf>,
    /// The machine's Secure Boot keys, opened from its rescue bundle, when
    /// the stick carries them, so a reinstall needs no firmware visit.
    secure_boot: Option<Keys>,
}

/// Where a file the vault gets is read from: a file of the payload's, or
/// memory, for keys that are never written down in the plain.
enum Source<'a> {
    File(PathBuf),
    Memory(&'a [u8]),
}

/// Every file under a directory, links followed, relative to it, in a
/// steady order.
fn files_under(root: &Path) -> Result<Vec<PathBuf>, Fault> {
    let mut found = vec![];
    let mut pending = vec![PathBuf::new()];
    while let Some(relative) = pending.pop() {
        let here = root.join(&relative);
        for entry in std::fs::read_dir(&here).map_err(unread(&here))? {
            let entry = entry.map_err(unread(&here))?;
            let path = relative.join(entry.file_name());
            if root.join(&path).is_dir() {
                pending.push(path);
            } else {
                found.push(path);
            }
        }
    }
    found.sort();
    Ok(found)
}

impl Payload {
    pub fn open(root: &Path) -> Result<Payload, Fault> {
        let target = root.join(TARGET);
        let target: Target =
            serde_json::from_slice(&std::fs::read(&target).map_err(unread(&target))?)
                .map_err(|why| Fault::unreadable("the payload's target.json", why))?;
        Ok(Payload {
            cached: files_under(&root.join("cache"))?,
            root: root.to_path_buf(),
            target,
            secure_boot: None,
        })
    }

    /// The machine's Secure Boot keys too, opened from its rescue bundle,
    /// which has to be this machine's.
    pub fn with_secure_boot_keys(mut self, machine: &str, keys: Keys) -> Result<Payload, Fault> {
        if machine != self.target.machine {
            return Err(Fault::Blocked {
                step: "write-stick",
                why: format!(
                    "the rescue bundle is {machine}'s, and the stick is {}'s",
                    self.target.machine
                ),
            });
        }
        self.secure_boot = Some(keys);
        Ok(self)
    }

    /// Every file the vault gets, by its path there and where it's read
    /// from.
    fn files(&self) -> Vec<(String, Source<'_>)> {
        let mut files = vec![
            (TARGET.to_string(), Source::File(self.root.join(TARGET))),
            (
                LAYOUT_SCRIPT.to_string(),
                Source::File(self.root.join("layout.sh")),
            ),
        ];
        for relative in &self.cached {
            files.push((
                format!("{SYSTEM_CACHE}/{}", relative.display()),
                Source::File(self.root.join("cache").join(relative)),
            ));
        }
        if let Some(keys) = &self.secure_boot {
            for (name, bytes) in keys.files() {
                files.push((format!("sbctl/{name}"), Source::Memory(bytes)));
            }
        }
        files
    }

    /// Whether the stick carries the machine's Secure Boot keys.
    pub fn carries_keys(&self) -> bool {
        self.secure_boot.is_some()
    }

    /// What the person does with the stick once it's written, from what
    /// the machine's hardware says of its firmware: a card's words.
    pub fn next(&self) -> String {
        let target = &self.target;
        let mut said = format!("Take the stick to {}.", target.machine);
        if !self.carries_keys()
            && let Some(clear) = &target.clear_secure_boot_keys
        {
            said.push_str(&format!(
                " This stick carries none of its Secure Boot keys, so clear its firmware's first: {clear}"
            ));
        }
        match &target.start_from_stick {
            Some(start) => said.push_str(&format!(" To start it from the stick: {start}")),
            None => said.push_str(" Start it from the stick."),
        }
        let opens = if target.recipients.is_empty() {
            "the recovery passphrase"
        } else {
            "a YubiKey's PIN, or the recovery passphrase"
        };
        said.push_str(&format!(
            " The guide carries on there, and asks for {opens} first."
        ));
        said
    }

    /// How much vault it needs: each file in whole clusters, FAT32's
    /// largest being 32 KiB, with what age adds to each.
    pub fn needed(&self) -> Result<u64, Fault> {
        let cluster = 32 << 10;
        let mut needed: u64 = 16 << 20;
        for (_, from) in self.files() {
            let bytes = match from {
                Source::File(path) => std::fs::metadata(&path).map_err(unread(&path))?.len(),
                Source::Memory(bytes) => bytes.len() as u64,
            };
            needed += (bytes + bytes / 1000 + 512).div_ceil(cluster) * cluster;
        }
        Ok(needed)
    }
}

/// What a repair stick's vault says: it's no one machine's, and nothing
/// opens it, since it carries nothing secret.
fn repair_manifest() -> Manifest {
    Manifest {
        format: 1,
        machine: String::new(),
        opened_by: vec![],
    }
}

/// The stick written and read back: the image, its boot program signed
/// when a signer is given, and its vault, holding the payload sealed with
/// the passphrase when one is given. Says how many bytes the vault holds.
pub fn write(
    image: &Path,
    disk: &mut File,
    signer: Option<&Signer>,
    payload: Option<(&Payload, &Secret)>,
) -> Result<u64, Fault> {
    let kept = |why: std::io::Error| Fault::Unkept {
        path: "the stick".into(),
        why: why.to_string(),
    };
    let mut image = Image::open(File::open(image).map_err(unread(image))?)?;

    let Some((payload, passphrase)) = payload else {
        let manifest = repair_manifest();
        let written = stick::write(&mut image, disk, stick::new_guid()?, signer, |store| {
            let bytes = serde_json::to_vec_pretty(&manifest)
                .map_err(|why| Fault::unreadable("the manifest", why))?;
            store
                .create(vault::MANIFEST)?
                .write_all(&bytes)
                .map_err(kept)
        })?;
        stick::verify(&written, disk, |store| {
            if vault::manifest(store)? == repair_manifest() {
                Ok(())
            } else {
                Err(Fault::Unproven {
                    step: "write-stick",
                    standing: "the vault's manifest isn't what was written".into(),
                })
            }
        })?;
        return Ok(written.vault.bytes());
    };

    // Refused before anything is written, said in what the person has.
    let bytes = std::io::Seek::seek(disk, std::io::SeekFrom::End(0)).map_err(kept)?;
    let room = stick::vault_room(&image, bytes)?;
    let needed = payload.needed()?;
    if room < needed {
        return Err(Fault::Blocked {
            step: "write-stick",
            why: format!(
                "the stick holds {} MB of vault, and {}'s install needs {} MB",
                room / 1_000_000,
                payload.target.machine,
                needed.div_ceil(1_000_000)
            ),
        });
    }
    let key = VaultKey::generate();
    let sealed = key.seal_with_passphrase(passphrase, None)?;
    // Sealed to the YubiKeys too when the machine names their recipients,
    // so one touch opens it and the passphrase stays for when none can.
    let target = &payload.target;
    if !target.recipients.is_empty() && target.recipients.len() != target.yubikeys.len() {
        return Err(Fault::Blocked {
            step: "write-stick",
            why: format!(
                "{} names {} YubiKeys and {} recipients for them",
                target.machine,
                target.yubikeys.len(),
                target.recipients.len()
            ),
        });
    }
    let to_keys = if target.recipients.is_empty() {
        None
    } else {
        Some(key.seal_to_keys(&target.recipients)?)
    };
    let mut opened_by = if to_keys.is_some() {
        target.yubikeys.clone()
    } else {
        vec![]
    };
    opened_by.push("the recovery passphrase".into());
    let manifest = Manifest {
        format: 1,
        machine: target.machine.clone(),
        opened_by,
    };
    let written = stick::write(&mut image, disk, stick::new_guid()?, signer, |store| {
        store
            .create(vault::KEY_PASSPHRASE)?
            .write_all(&sealed)
            .map_err(kept)?;
        if let Some(to_keys) = &to_keys {
            store.create(vault::KEY)?.write_all(to_keys).map_err(kept)?;
        }
        let mut packer = Packer::new(store, &key);
        for (name, from) in payload.files() {
            match from {
                Source::File(path) => {
                    packer.add(&name, File::open(&path).map_err(unread(&path))?)?
                }
                Source::Memory(bytes) => packer.add(&name, bytes)?,
            }
        }
        packer.finish(&manifest)
    })?;
    // Read back as the stick will open it: with the passphrase, every file
    // extracted and checked.
    stick::verify(&written, disk, |store| {
        let mut sealed = vec![];
        store
            .open(vault::KEY_PASSPHRASE)?
            .read_to_end(&mut sealed)
            .map_err(kept)?;
        let mut vault = Vault::open(store, VaultKey::open_with_passphrase(&sealed, passphrase)?)?;
        let paths: Vec<String> = vault.files().iter().map(|file| file.path.clone()).collect();
        for path in &paths {
            vault.extract(path, std::io::sink())?;
        }
        Ok(())
    })?;
    Ok(written.vault.bytes())
}

/// What a person reads a stick by.
fn described(disk: &Disk) -> String {
    crate::install::disk::described(disk)
}

/// The one stick plugged in, or why there isn't one.
fn the_stick(machine: &mut dyn Machine) -> Result<Result<Disk, Standing>, Fault> {
    let mut sticks: Vec<Disk> = disks::observe(machine)?
        .into_iter()
        .filter(|disk| disk.kind == Kind::Removable)
        .collect();
    Ok(match sticks.len() {
        1 => Ok(sticks.remove(0)),
        0 => Err(Standing::Blocked {
            why: "no stick is plugged in".into(),
            remedy: "plug in the stick to write, by USB".into(),
        }),
        many => Err(Standing::Blocked {
            why: format!(
                "{many} removable disks are plugged in: {}",
                sticks.iter().map(described).collect::<Vec<_>>().join("; ")
            ),
            remedy: "leave only the stick to write plugged in".into(),
        }),
    })
}

/// Who the journal says a stick was written for, by its serial or path.
fn named(disk: &Disk) -> String {
    disk.serial.clone().unwrap_or_else(|| disk.path.clone())
}

/// The stick written for a machine and read back, once its passphrase,
/// seven words made for it, is kept on the person's phone.
pub struct WriteStick {
    pub image: PathBuf,
    pub payload: Option<Payload>,
    pub signer: Option<Signer>,
    /// The passphrase's words, made once, so those shown are those typed
    /// back and those the vault is sealed to.
    pub words: std::cell::OnceCell<Secret>,
}

impl WriteStick {
    pub fn new(image: PathBuf, payload: Option<Payload>, signer: Option<Signer>) -> WriteStick {
        WriteStick {
            image,
            payload,
            signer,
            words: std::cell::OnceCell::new(),
        }
    }

    fn words(&self) -> Result<&Secret, Fault> {
        if self.words.get().is_none() {
            let _ = self.words.set(words::made()?);
        }
        Ok(self.words.get().expect("made just now"))
    }
}

impl Step for WriteStick {
    fn id(&self) -> &'static str {
        "write-stick"
    }

    fn place(&self) -> Place {
        Place::Host
    }

    fn title(&self) -> &'static str {
        "Write the stick"
    }

    fn observe(&self, machine: &mut dyn Machine) -> Result<Standing, Fault> {
        let stick = match the_stick(machine)? {
            Ok(stick) => stick,
            Err(standing) => return Ok(standing),
        };
        if machine.journal().knows(self.id(), &named(&stick)) {
            return Ok(Standing::Done {
                evidence: format!("{} is written, and read back as written", described(&stick)),
            });
        }
        let image = std::fs::metadata(&self.image)
            .map_err(unread(&self.image))?
            .len();
        let needed = image
            + self
                .payload
                .as_ref()
                .map(Payload::needed)
                .transpose()?
                .unwrap_or(stick::LEAST_VAULT);
        if stick.bytes < needed {
            return Ok(Standing::Blocked {
                why: format!(
                    "{} holds {} GB, and the stick needs {} GB",
                    described(&stick),
                    stick.bytes / 1_000_000_000,
                    needed.div_ceil(1_000_000_000)
                ),
                remedy: "plug in a bigger stick instead".into(),
            });
        }
        let mut asks = vec![Ask::Act {
            id: "stick",
            what: format!(
                "{} is the stick to write. Everything on it will be replaced.",
                described(&stick)
            ),
        }];
        if let Some(payload) = &self.payload {
            let words = self.words()?.expose();
            asks.push(Ask::Show {
                id: "shown",
                how: words::keeping(
                    "the stick's passphrase",
                    &format!("{} stick passphrase", payload.target.machine),
                    "The stick's passphrase opens its vault when no YubiKey is plugged in",
                ),
                shown: words.into(),
            });
            asks.push(Ask::Recall {
                id: "passphrase",
                expected: words.into(),
                why: "type the stick's passphrase, reading it off your phone".into(),
            });
        }
        Ok(Standing::Needed { asks })
    }

    fn run(&self, machine: &mut dyn Machine, answers: &Answers) -> Result<(), Fault> {
        let stick = the_stick(machine)?.map_err(|standing| Fault::Blocked {
            step: self.id(),
            why: standing.to_string(),
        })?;
        let passphrase = match (&self.payload, answers.get("passphrase")) {
            (None, _) => None,
            (Some(_), Some(Answer::Typed(typed))) => {
                let words = self.words()?;
                if words::typed(typed) != words::typed(words.expose()) {
                    return Err(Fault::Blocked {
                        step: self.id(),
                        why: "what was typed isn't the stick's passphrase".into(),
                    });
                }
                Some(words)
            }
            _ => {
                return Err(Fault::Unanswered {
                    step: self.id(),
                    ask: "passphrase",
                });
            }
        };
        let mounted: Vec<String> = stick
            .partitions
            .iter()
            .filter(|partition| !partition.mounted.is_empty())
            .map(|partition| partition.path.clone())
            .collect();
        let mut disk = machine.open_disk(&stick.path, &mounted)?;
        let payload = self.payload.as_ref().zip(passphrase);
        write(&self.image, &mut disk, self.signer.as_ref(), payload)?;
        machine.journal().record(self.id(), &named(&stick))
    }
}
