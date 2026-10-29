//! The writer's part: writing a stick, from a machine's image and, for a
//! stick, its payload (payload.nix) sealed into the vault with
//! the stick's passphrase. The stick is a whole USB or removable disk,
//! only ever one plugged in at once, opened for writing through the
//! system's own way of letting a person do so (`stick::device`), so nothing
//! on a writer runs as root. Every stick carries on the rescue bundles the
//! writer keeps (carried).

pub mod carried;

use crate::progress::Meter;
use std::fmt::Write as _;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use crate::install::{LAYOUT_SCRIPT, MOUNT_SCRIPT, SYSTEM_CACHE, TARGET, Target};
use crate::observe::disks::{self, Disk, Kind};
use crate::rescue::{self, Keys};
use crate::sign::Signer;
use crate::stick::{self, Image};
use crate::vault::store::Store;
use crate::vault::{self, Manifest, Packer, Vault, VaultKey};
use crate::{Answer, Answers, Ask, Fault, Machine, Place, Secret, Standing, Step, words};
use carried::Carried;

fn unread(path: &Path) -> impl Fn(std::io::Error) -> Fault + '_ {
    move |why| Fault::unreadable(path.display().to_string(), why)
}

/// What a stick carries for one machine, as payload.nix built it.
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
            (
                MOUNT_SCRIPT.to_string(),
                Source::File(self.root.join("mount.sh")),
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
        let machine = &target.machine;
        let mut said = format!("Take the stick to {machine}, switched off, and plug it in.");
        if !self.carries_keys()
            && let Some(clear) = &target.clear_secure_boot_keys
        {
            // A system already there may put its old keys back the moment
            // it starts, so it's never let start once they're cleared.
            // Writing to a String can't fail.
            _ = write!(
                said,
                " This stick carries none of its Secure Boot keys, so clear its firmware's first: {clear} As you leave the firmware's setup, hold the power button until {machine} is off, so its old system never starts and puts its keys back."
            );
        }
        match &target.start_from_stick {
            // Writing to a String can't fail.
            Some(start) => _ = write!(said, " Then start it from the stick: {start}"),
            None => said.push_str(" Then start it from the stick."),
        }
        // In the order a person meets it, since a stick that started and
        // asked nothing for minutes was taken for a fault.
        said.push_str(
            " The guide carries on there. First it copies itself into memory, which takes a few minutes and asks nothing. Then it asks you to type a word to check the keyboard, and then for the stick's passphrase, the seven words you saved on your phone.",
        );
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

/// What the screen says while the stick is written, and then read back.
const WRITING: &str = "Writing the stick";
const READING_BACK: &str = "Reading it back to check it";

/// Why the stick couldn't be written.
fn kept(why: impl std::fmt::Display) -> Fault {
    Fault::Unkept {
        path: "the stick".into(),
        why: why.to_string(),
    }
}

/// Puts the bundles beside the vault's sealed files.
fn carry(bundles: &[Carried], store: &mut dyn Store) -> Result<(), Fault> {
    for bundle in bundles {
        store
            .create(&rescue::carried_as(&bundle.machine))?
            .write_all(&bundle.bytes)
            .map_err(kept)?;
    }
    Ok(())
}

/// Proves the bundles beside the vault's sealed files read back as written.
fn carried_back(bundles: &[Carried], store: &mut dyn Store) -> Result<(), Fault> {
    let mut written = bundles.to_vec();
    written.sort_by(|one, other| one.machine.cmp(&other.machine));
    if carried::kept(store) == written {
        Ok(())
    } else {
        Err(Fault::Unproven {
            step: "write-stick",
            standing: "the rescue bundles on the stick aren't what was written".into(),
        })
    }
}

/// The stick written and read back: the image, its boot program signed by
/// each signer given, and its vault, holding the payload sealed with the
/// passphrase when one is given. Says how many bytes the vault holds.
pub fn write(
    image: &Path,
    disk: &mut File,
    signers: &[Signer],
    payload: Option<(&Payload, &Secret)>,
    bundles: &[Carried],
    meter: &Meter,
) -> Result<u64, Fault> {
    let mut image = Image::open(File::open(image).map_err(unread(image))?)?;
    let imaged: u64 = image
        .partitions()
        .iter()
        .map(stick::gpt::Partition::bytes)
        .sum();
    let Some((payload, passphrase)) = payload else {
        return write_repair(&mut image, disk, signers, bundles, imaged, meter);
    };
    let needed = fits(&image, disk, payload)?;
    let key = VaultKey::generate();
    // Sealed to the passphrase alone: a YubiKey opens a vault only beside
    // the stick, which a laptop with one port both fit has no room for, so
    // every machine opens it the same way.
    let sealed = key.seal_with_passphrase(passphrase, None)?;
    let manifest = Manifest {
        format: 1,
        machine: payload.target.machine.clone(),
        opened_by: vec!["the stick's passphrase".into()],
    };
    meter.stage(WRITING, Some(imaged + needed));
    let written = stick::write(
        &mut image,
        disk,
        stick::new_guid()?,
        signers,
        meter,
        |store| {
            store
                .create(vault::KEY_PASSPHRASE)?
                .write_all(&sealed)
                .map_err(kept)?;
            carry(bundles, store)?;
            let mut packer = Packer::new(store, &key);
            for (name, from) in payload.files() {
                match from {
                    Source::File(path) => {
                        packer.add(&name, File::open(&path).map_err(unread(&path))?)?;
                    }
                    Source::Memory(bytes) => packer.add(&name, bytes)?,
                }
            }
            packer.finish(&manifest)
        },
    )?;
    // Read back as the stick will open it: with the passphrase, every file
    // extracted and checked. What was written is known now, where the
    // write's total was reckoned with room to spare.
    meter.stage(READING_BACK, Some(meter.counted()));
    stick::verify(&written, disk, meter, |store| {
        carried_back(bundles, store)?;
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

/// A repair stick written and read back: the image, its boot program
/// signed by each signer given, and a vault holding only its manifest and
/// the bundles. Says how many bytes the vault holds.
fn write_repair(
    image: &mut Image,
    disk: &mut File,
    signers: &[Signer],
    bundles: &[Carried],
    imaged: u64,
    meter: &Meter,
) -> Result<u64, Fault> {
    let manifest = repair_manifest();
    meter.stage(WRITING, Some(imaged));
    let written = stick::write(image, disk, stick::new_guid()?, signers, meter, |store| {
        let bytes = serde_json::to_vec_pretty(&manifest)
            .map_err(|why| Fault::unreadable("the manifest", why))?;
        store
            .create(vault::MANIFEST)?
            .write_all(&bytes)
            .map_err(kept)?;
        carry(bundles, store)
    })?;
    meter.stage(READING_BACK, Some(meter.counted()));
    stick::verify(&written, disk, meter, |store| {
        carried_back(bundles, store)?;
        if vault::manifest(store)? == repair_manifest() {
            Ok(())
        } else {
            Err(Fault::Unproven {
                step: "write-stick",
                standing: "the vault's manifest isn't what was written".into(),
            })
        }
    })?;
    Ok(written.vault.bytes())
}

/// Refuses a payload the stick hasn't the room for before anything is
/// written, said in what the person has. Says how much vault it needs.
fn fits(image: &Image, disk: &mut File, payload: &Payload) -> Result<u64, Fault> {
    let bytes = std::io::Seek::seek(disk, std::io::SeekFrom::End(0)).map_err(kept)?;
    let room = stick::vault_room(image, bytes)?;
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
    Ok(needed)
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

/// The rescue bundles the plugged-in stick carries, kept on the writer
/// before anything is written over them, with the writer's own machine's
/// too, which its install left where anyone can read it. A stick the
/// guide never wrote has none, and one whose vault won't be read doesn't
/// stop it being written, which is said; failing to keep what was read
/// does, since the stick's copies would be lost.
pub struct GatherBundles {
    /// Where the writer keeps them, the person's own.
    pub kept: PathBuf,
    /// Why the stick's vault couldn't be read, when it couldn't.
    pub unread: std::cell::RefCell<Option<String>>,
    /// The copy of its key the stick's vault keeps for its passphrase,
    /// sealed and so no secret, for the write to prove a kept passphrase
    /// against.
    pub phrase: std::rc::Rc<std::cell::RefCell<Option<Vec<u8>>>>,
}

impl GatherBundles {
    pub fn new(kept: PathBuf) -> GatherBundles {
        GatherBundles {
            kept,
            unread: std::cell::RefCell::default(),
            phrase: std::rc::Rc::default(),
        }
    }
}

impl Step for GatherBundles {
    fn id(&self) -> &'static str {
        "gather-bundles"
    }

    fn place(&self) -> Place {
        Place::Writer
    }

    fn title(&self) -> &'static str {
        "Keep the rescue bundles the stick carries"
    }

    fn observe(&self, machine: &mut dyn Machine) -> Result<Standing, Fault> {
        let stick = match the_stick(machine)? {
            Ok(stick) => stick,
            Err(standing) => return Ok(standing),
        };
        Ok(if machine.journal().knows(self.id(), &named(&stick)) {
            let count = carried::kept(&mut vault::store::Directory::new(&self.kept)).len();
            Standing::Done {
                evidence: match self.unread.borrow().as_deref() {
                    None => format!("the writer keeps {count} rescue bundles"),
                    Some(why) => format!(
                        "the writer keeps {count} rescue bundles, but none of the stick's, since its vault couldn't be read: {why}"
                    ),
                },
            }
        } else {
            Standing::Needed { asks: vec![] }
        })
    }

    fn run(&self, machine: &mut dyn Machine, _answers: &Answers) -> Result<(), Fault> {
        let stick = the_stick(machine)?.map_err(|standing| Fault::Blocked {
            step: self.id(),
            why: standing.to_string(),
        })?;
        let vault = stick
            .partitions
            .iter()
            .find(|partition| partition.label.as_deref() == Some("vault"));
        // Read where a desktop mounted the vault, else mounted only for
        // reading, which needs no password as opening the disk would.
        let mut found = vec![];
        let mut phrase = None;
        let mut read = |mounted: &Path| {
            let mut store = vault::store::Directory::new(mounted);
            found = carried::kept(&mut store);
            phrase = sealed_phrase(&mut store);
        };
        if let Some(vault) = vault {
            match vault.mounted.first() {
                Some(mounted) => read(Path::new(mounted)),
                // One that won't mount carries nothing to keep, which is
                // said as the step is done.
                None => {
                    if let Err(why) = machine.read_partition(&vault.path, &mut read) {
                        self.unread.replace(Some(why.to_string()));
                    }
                }
            }
        }
        self.phrase.replace(phrase);
        let own = machine
            .read(rescue::KEPT)
            .ok()
            .flatten()
            .and_then(|bytes| Carried::read(bytes, None));
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |since| since.as_secs());
        carried::keep(
            &mut vault::store::Directory::new(&self.kept),
            own,
            found,
            now,
        )?;
        machine.journal().record(self.id(), &named(&stick))
    }
}

/// The copy of its key a stick's vault keeps for its passphrase, when it
/// keeps one.
fn sealed_phrase(store: &mut dyn Store) -> Option<Vec<u8>> {
    let mut sealed = vec![];
    store
        .open(vault::KEY_PASSPHRASE)
        .ok()?
        .read_to_end(&mut sealed)
        .ok()?;
    Some(sealed)
}

/// Whether a stick whose vault has a passphrase keeps it or gets new
/// words, so the person's phone keeps one passphrase for the stick
/// whichever machine it's written for. A stick with none gets new words.
pub struct ChoosePhrase {
    /// The copy of its key the stick's vault keeps for its passphrase.
    pub sealed: Option<Vec<u8>>,
    /// What was chosen: to keep it, or not.
    pub keep: std::rc::Rc<std::cell::Cell<Option<bool>>>,
}

impl Step for ChoosePhrase {
    fn id(&self) -> &'static str {
        "choose-phrase"
    }

    fn place(&self) -> Place {
        Place::Writer
    }

    fn title(&self) -> &'static str {
        "Keep the stick's passphrase"
    }

    fn observe(&self, _machine: &mut dyn Machine) -> Result<Standing, Fault> {
        if self.sealed.is_none() {
            return Ok(Standing::Done {
                evidence: "the stick has no passphrase yet, so it gets new words".into(),
            });
        }
        Ok(match self.keep.get() {
            Some(true) => Standing::Done {
                evidence: "the stick keeps its passphrase".into(),
            },
            Some(false) => Standing::Done {
                evidence: "the stick gets new words".into(),
            },
            None => Standing::Needed {
                asks: vec![Ask::Choose {
                    id: "phrase",
                    question: format!(
                        "This stick has its passphrase already, the seven words in 1Password on your phone named \"{}\". Keep it?",
                        words::STICK_PHRASE
                    ),
                    choices: vec![
                        "Keep it, typing it from your phone".into(),
                        "Make new words, which replace it on your phone".into(),
                    ],
                }],
            },
        })
    }

    fn run(&self, _machine: &mut dyn Machine, answers: &Answers) -> Result<(), Fault> {
        let Some(Answer::Chosen(chosen)) = answers.get("phrase") else {
            return Err(Fault::Unanswered {
                step: self.id(),
                ask: "phrase",
            });
        };
        self.keep.set(Some(*chosen == 0));
        Ok(())
    }
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
    /// The keys its boot program is signed by, one for each machine it's
    /// to boot on.
    pub signers: Vec<Signer>,
    /// The rescue bundles the stick carries on, beside its vault.
    pub bundles: Vec<Carried>,
    /// Whether they were gathered from a stick first, which is then the
    /// only one this may write over.
    pub gathered: bool,
    /// The passphrase's words, made once, so those shown are those typed
    /// back and those the vault is sealed to.
    pub words: std::cell::OnceCell<Secret>,
    /// The copy of its key the stick's old vault keeps for its passphrase,
    /// and whether the person chose to keep that passphrase: when they
    /// did, what's typed is proven against it, and no new words are made.
    pub kept_phrase: Option<(Vec<u8>, Chosen)>,
}

/// What the person chose, once they have: shared by the step that asks
/// and the steps after that act on it.
pub type Chosen = std::rc::Rc<std::cell::Cell<Option<bool>>>;

/// The steps that write a stick: first, for a stick for a machine whose
/// vault has a passphrase, sealed as `sealed`, whether to keep it.
pub fn writing(write: WriteStick, sealed: Option<Vec<u8>>) -> Vec<Box<dyn Step>> {
    let Some(sealed) = sealed.filter(|_| write.payload.is_some()) else {
        return vec![Box::new(write)];
    };
    let keep = Chosen::default();
    vec![
        Box::new(ChoosePhrase {
            sealed: Some(sealed.clone()),
            keep: std::rc::Rc::clone(&keep),
        }),
        Box::new(write.keeping_phrase(sealed, keep)),
    ]
}

impl WriteStick {
    pub fn new(image: PathBuf, payload: Option<Payload>, signers: Vec<Signer>) -> WriteStick {
        WriteStick {
            image,
            payload,
            signers,
            bundles: vec![],
            gathered: false,
            words: std::cell::OnceCell::new(),
            kept_phrase: None,
        }
    }

    /// Keeping the stick's passphrase, the one `sealed` was sealed to,
    /// once `chosen` says the person chose to.
    #[must_use]
    pub fn keeping_phrase(mut self, sealed: Vec<u8>, chosen: Chosen) -> WriteStick {
        self.kept_phrase = Some((sealed, chosen));
        self
    }

    /// The sealed copy the kept passphrase is proven against, when the
    /// person chose to keep it.
    fn kept(&self) -> Option<&[u8]> {
        self.kept_phrase
            .as_ref()
            .filter(|(_, chosen)| chosen.get() == Some(true))
            .map(|(sealed, _)| sealed.as_slice())
    }

    /// The passphrase the person typed, as the stick's old vault was
    /// sealed to it: as the words it says, or as typed for one chosen by
    /// hand.
    fn proven(&self, sealed: &[u8], typed: &Secret) -> Result<Secret, Fault> {
        let as_words = Secret::new(words::typed(typed.expose()));
        [as_words, Secret::new(typed.expose().to_string())]
            .into_iter()
            .find(|tried| VaultKey::open_with_passphrase(sealed, tried).is_ok())
            .ok_or(Fault::Blocked {
                step: self.id(),
                why: format!(
                    "what was typed isn't the stick's passphrase, the seven words in 1Password named \"{}\"",
                    words::STICK_PHRASE
                ),
            })
    }

    /// Carrying these rescue bundles on too, gathered from the stick it
    /// writes over.
    #[must_use]
    pub fn carrying(mut self, bundles: Vec<Carried>) -> WriteStick {
        self.bundles = bundles;
        self.gathered = true;
        self
    }

    /// Whether the stick is one this may write over: any, unless bundles
    /// were gathered, when only the one they were gathered from, so a
    /// stick swapped in afterwards never loses its own.
    fn may_write(&self, machine: &mut dyn Machine, stick: &Disk) -> Option<Standing> {
        (self.gathered && !machine.journal().knows("gather-bundles", &named(stick))).then(|| {
            Standing::Blocked {
                why: format!(
                    "{} isn't the stick whose rescue bundles were kept",
                    described(stick)
                ),
                remedy: "plug that stick back in, or start again with this one".into(),
            }
        })
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
        Place::Writer
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
        if let Some(standing) = self.may_write(machine, &stick) {
            return Ok(standing);
        }
        let image = std::fs::metadata(&self.image)
            .map_err(unread(&self.image))?
            .len();
        // The bundles beside the vault's files, a cluster each at least.
        let carried: u64 = self
            .bundles
            .iter()
            .map(|bundle| bundle.bytes.len() as u64 + 64 * 1024)
            .sum();
        let needed = image
            + carried
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
        let mut asks = vec![Ask::Confirm {
            id: "stick",
            what: format!(
                "{} is the stick to write. Everything on it will be replaced.",
                described(&stick)
            ),
            verb: "write it",
        }];
        if self.payload.is_some() && self.kept().is_some() {
            asks.push(Ask::Secret {
                id: "passphrase",
                name: format!(
                    "the stick's passphrase, the seven words in 1Password on your phone named \"{}\", which it keeps",
                    words::STICK_PHRASE
                ),
            });
        } else if self.payload.is_some() {
            let words = self.words()?.expose();
            asks.push(Ask::Show {
                id: "shown",
                how: words::keeping(
                    "the stick's passphrase",
                    words::STICK_PHRASE,
                    "The stick's passphrase opens its vault, on whichever machine it's started",
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
        if let Some(standing) = self.may_write(machine, &stick) {
            return Err(Fault::Blocked {
                step: self.id(),
                why: standing.to_string(),
            });
        }
        let kept;
        let passphrase = match (&self.payload, answers.get("passphrase")) {
            (None, _) => None,
            (Some(_), Some(Answer::Secret(typed))) if self.kept().is_some() => {
                kept = self.proven(self.kept().unwrap_or_default(), typed)?;
                Some(&kept)
            }
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
        write(
            &self.image,
            &mut disk,
            &self.signers,
            payload,
            &self.bundles,
            &machine.meter(),
        )?;
        machine.journal().record(self.id(), &named(&stick))
    }
}
