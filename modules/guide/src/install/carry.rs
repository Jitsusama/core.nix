//! A machine's home folders carried across its reinstall: packed, before
//! the disk is wiped, onto a drive the person plugs in, as one archive
//! that keeps every file's owner, permissions, links, times, ACLs and
//! extended attributes, sealed so only the machine's YubiKeys and its
//! stick open it; then, once the new system is installed and before it
//! ever starts, put back into its home folders.
//!
//! The archive is read back whole before anything is wiped, so a drive
//! that didn't keep it is found while the home folders are still there.
//! Its key is sealed to each YubiKey, for a person opening it anywhere,
//! and to the stick's vault, which every start of the install opens, so a
//! restart after the wipe puts it back with nothing more asked. Caches are
//! left behind, since every program makes its own again, and so is what
//! the machine says to leave (`jitsusama.guide.carry.leaveOut`), such as a
//! key store bound to the old install.

use std::fmt::Write as _;

use serde::{Deserialize, Serialize};

use crate::install::{ROOT, held, target, vault_key};
use crate::recover::copy::{self, CACHE, Drive};
use crate::vault::VaultKey;
use crate::{Answer, Answers, Ask, Fault, Machine, Place, Secret, Standing, Step, Tool};

/// Where the drive is mounted while the archive is written or read.
const ONTO: &str = "/run/guide/carry";

/// The filesystems a drive the archive goes onto may have: never FAT,
/// which holds no file of 4 GB or more.
const WRITABLE: &[&str] = &["exfat", "ext4", "btrfs", "xfs"];

/// What's kept in the folder on the drive.
const ARCHIVE: &str = "home.tar.zst.age";
const KEY_TO_YUBIKEYS: &str = "key.age";
const KEY_TO_STICK: &str = "key-for-the-stick.age";
const MANIFEST: &str = "carried.json";
const UNREAD: &str = "not carried.txt";

/// The archive's key, held in memory between packing and putting back.
const HELD_KEY: &str = "/run/guide/carry-key";

/// What packing said, kept until the stick restarts.
const PACKED: &str = "/run/guide/packed";

/// Whether the person chose to carry the home folders, as the journal
/// keeps it.
const CARRY: &str = "carry";
const LEAVE: &str = "leave";

/// What the archive holds, beside it on the drive in plain text: nothing
/// secret, only what putting it back needs to know.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Carried {
    pub format: u32,
    pub machine: String,
    /// Each account whose home folder is carried, with the numbers it
    /// owned its files by, which the new system may give another.
    pub accounts: Vec<Account>,
    /// How much the home folders came to, unpacked.
    pub bytes: u64,
    /// Whether some files couldn't be read, which UNREAD says.
    pub unread: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Account {
    pub name: String,
    pub uid: u32,
    pub gid: u32,
}

/// Every account in a passwd file whose home folder is under /home.
pub fn accounts(passwd: &str) -> Vec<Account> {
    passwd
        .lines()
        .filter_map(|line| {
            let fields: Vec<&str> = line.split(':').collect();
            let [name, _, uid, gid, _, home, _] = fields.as_slice() else {
                return None;
            };
            let under_home = home.strip_prefix("/home/")?;
            if under_home.is_empty() || under_home.contains('/') {
                return None;
            }
            Some(Account {
                name: (*name).to_string(),
                uid: uid.parse().ok()?,
                gid: gid.parse().ok()?,
            })
        })
        .collect()
}

/// The accounts of the system mounted at the root, or none when it has
/// no passwd file to read.
fn accounts_there(machine: &mut dyn Machine) -> Result<Vec<Account>, Fault> {
    let Some(at) = machine.resolve_inside(ROOT, &format!("{ROOT}/etc/passwd"))? else {
        return Ok(vec![]);
    };
    Ok(machine
        .read(&format!("{ROOT}{at}"))?
        .map(|bytes| accounts(&String::from_utf8_lossy(&bytes)))
        .unwrap_or_default())
}

/// The folder on the drive, named for the machine, so two machines'
/// archives never meet on one drive.
fn folder(machine: &str) -> String {
    format!("{ONTO}/{machine} carried home")
}

/// What zstd is run as, by tar, which adds `-d` to unpack.
fn compressor() -> Result<String, Fault> {
    let zstd = Tool::Zstd.program().ok_or(Fault::Unbuilt {
        tool: Tool::Zstd.name(),
    })?;
    Ok(format!("--use-compress-program={zstd} -T0"))
}

/// What tar is told of every file's metadata, the same way in and out.
const KEPT: &[&str] = &[
    "--numeric-owner",
    "--acls",
    "--xattrs",
    "--xattrs-include=*",
];

/// What tar leaves out of the archive: every cache, then each of the
/// machine's leave-outs, a path inside a home folder, matched in every
/// account's and nowhere else.
fn left_out(leave_out: &[String]) -> Result<Vec<String>, Fault> {
    let mut args = vec![format!("--exclude={CACHE}")];
    if leave_out.is_empty() {
        return Ok(args);
    }
    args.push("--anchored".into());
    args.push("--no-wildcards-match-slash".into());
    for path in leave_out {
        let inside = path.trim_matches('/');
        if inside.is_empty() || inside.split('/').any(|part| part == "..") {
            return Err(Fault::Blocked {
                step: "pack-home",
                why: format!(
                    "\"{path}\" isn't a path inside a home folder, so it can't be left out: write the stick again with jitsusama.guide.carry.leaveOut naming paths such as .config/gh/config.yml"
                ),
            });
        }
        args.push(format!("--exclude=./*/{inside}"));
    }
    Ok(args)
}

/// Whether the person chose to carry the home folders across.
fn chosen(machine: &mut dyn Machine) -> Option<bool> {
    let journal = machine.journal();
    if journal.knows("carry-home", CARRY) {
        Some(true)
    } else if journal.knows("carry-home", LEAVE) {
        Some(false)
    } else {
        None
    }
}

/// Reinstalling wipes the home folders with the rest, so before anything
/// is opened the person chooses whether to carry them across.
pub struct ChooseCarry;

impl Step for ChooseCarry {
    fn id(&self) -> &'static str {
        "carry-home"
    }

    fn place(&self) -> Place {
        Place::Stick
    }

    fn title(&self) -> &'static str {
        "Choose whether to carry its home folders across"
    }

    fn observe(&self, machine: &mut dyn Machine) -> Result<Standing, Fault> {
        Ok(match chosen(machine) {
            Some(true) => Standing::Done {
                evidence: "its home folders are carried across, packed onto a drive first".into(),
            },
            Some(false) => Standing::Done {
                evidence: "its home folders are left, and start empty".into(),
            },
            None => Standing::Needed {
                asks: vec![Ask::Choose {
                    id: "carry",
                    question: "Installing it again wipes its home folders with the rest. Carry them across? They're packed onto a USB drive you plug in, read back, then put back once it's installed, before it ever starts.".into(),
                    choices: vec![
                        "Carry them, onto a USB drive with room for them".into(),
                        "Leave them, starting with empty home folders".into(),
                    ],
                }],
            },
        })
    }

    fn run(&self, machine: &mut dyn Machine, answers: &Answers) -> Result<(), Fault> {
        let Some(Answer::Chosen(at)) = answers.get("carry") else {
            return Err(Fault::Unanswered {
                step: self.id(),
                ask: "carry",
            });
        };
        let chosen = if *at == 0 { CARRY } else { LEAVE };
        machine.journal().record(self.id(), chosen)
    }
}

/// A step needed only to carry the home folders, such as opening the old
/// disk: done at once when the person chose to leave them.
pub struct WhenCarried(pub Box<dyn Step>);

impl Step for WhenCarried {
    fn id(&self) -> &'static str {
        self.0.id()
    }

    fn place(&self) -> Place {
        self.0.place()
    }

    fn title(&self) -> &'static str {
        self.0.title()
    }

    fn observe(&self, machine: &mut dyn Machine) -> Result<Standing, Fault> {
        if chosen(machine) == Some(false) {
            return Ok(Standing::Done {
                evidence: "nothing is carried, so it isn't needed".into(),
            });
        }
        self.0.observe(machine)
    }

    fn run(&self, machine: &mut dyn Machine, answers: &Answers) -> Result<(), Fault> {
        self.0.run(machine, answers)
    }
}

/// The drive the archive was packed onto, by its filesystem's UUID, once
/// it has been.
fn packed_onto(machine: &mut dyn Machine) -> Option<String> {
    machine
        .journal()
        .said("pack-home")
        .last()
        .map(ToString::to_string)
}

/// The old system's home folders packed onto a drive the person chooses,
/// and read back, while its disk is open and mounted.
pub struct PackHome;

impl Step for PackHome {
    fn id(&self) -> &'static str {
        "pack-home"
    }

    fn place(&self) -> Place {
        Place::Stick
    }

    fn title(&self) -> &'static str {
        "Pack its home folders onto a drive"
    }

    fn observe(&self, machine: &mut dyn Machine) -> Result<Standing, Fault> {
        if chosen(machine) == Some(false) {
            return Ok(Standing::Done {
                evidence: "its home folders are left, so nothing is packed".into(),
            });
        }
        if let Some(uuid) = packed_onto(machine) {
            let said = machine.read(PACKED)?.map_or_else(
                || format!("its home folders are packed onto the drive whose filesystem is {uuid}"),
                |kept| String::from_utf8_lossy(&kept).into_owned(),
            );
            return Ok(Standing::Done { evidence: said });
        }
        let found = copy::drives(machine, ONTO, WRITABLE)?;
        if found.is_empty() {
            return Ok(Standing::Blocked {
                why: "no drive the home folders can be packed onto is plugged in".into(),
                remedy: "plug in a USB drive with an exFAT, ext4, btrfs or XFS partition and room for them: nothing on it is removed. FAT won't do, since it holds no file of 4 GB or more. If the guide said it runs from memory, the stick can come out to make a port. The guide looks again by itself".into(),
            });
        }
        let machine_name = target(machine)?.machine;
        Ok(Standing::Needed {
            asks: vec![Ask::Choose {
                id: "drive",
                question: format!(
                    "Which drive should they be packed onto? Nothing on it is removed: they go into a folder named \"{machine_name} carried home\", as one file only its YubiKeys and this stick open, leaving out caches."
                ),
                choices: found.into_iter().map(|drive| drive.said).collect(),
            }],
        })
    }

    fn run(&self, machine: &mut dyn Machine, answers: &Answers) -> Result<(), Fault> {
        let Some(Answer::Chosen(at)) = answers.get("drive") else {
            return Err(Fault::Unanswered {
                step: self.id(),
                ask: "drive",
            });
        };
        let drive = copy::drives(machine, ONTO, WRITABLE)?
            .into_iter()
            .nth(*at)
            .ok_or(Fault::Blocked {
                step: self.id(),
                why: "that drive isn't plugged in any more".into(),
            })?;
        let Some(uuid) = drive.uuid.clone() else {
            return Err(Fault::Blocked {
                step: self.id(),
                why: format!(
                    "{}'s filesystem has no UUID to find it by again: choose another drive",
                    drive.said
                ),
            });
        };
        mount(machine, &drive.partition, &[])?;
        let packed = pack(machine, self.id(), &drive);
        // Unmounted whatever happened, so the drive can come out whole.
        machine.run(Tool::Umount, &[ONTO])?.ok()?;
        let said = packed?;
        machine.write(PACKED, said.as_bytes())?;
        machine.journal().record(self.id(), &uuid)
    }
}

/// The drive's partition mounted where the archive is kept, with these
/// options besides.
fn mount(machine: &mut dyn Machine, partition: &str, options: &[&str]) -> Result<(), Fault> {
    let options = std::iter::once("X-mount.mkdir")
        .chain(options.iter().copied())
        .collect::<Vec<_>>()
        .join(",");
    machine
        .run(Tool::Mount, &["-o", &options, partition, ONTO])?
        .ok()
        .map(drop)
}

/// The home folders packed into the folder on the mounted drive, once
/// they're known to fit, and read back. Says what it did.
fn pack(machine: &mut dyn Machine, step: &'static str, drive: &Drive) -> Result<String, Fault> {
    let target = target(machine)?;
    let folder = folder(&target.machine);
    let home = format!("{ROOT}/home");
    let meter = machine.meter();
    meter.stage("Measuring the home folders", None);
    let excluded = format!("--exclude={CACHE}");
    let summed = machine.run(Tool::Du, &["--summarize", "--bytes", &excluded, &home])?;
    let needed =
        copy::measured(&String::from_utf8_lossy(&summed.stdout)).ok_or_else(|| summed.refusal())?;
    let free = machine
        .run(Tool::Df, &["--output=avail", "--block-size=1", ONTO])?
        .ok()?;
    let free =
        copy::measured(&free).ok_or(Fault::unreadable(ONTO, "df gave no room left on it"))?;
    if needed > free {
        return Err(Fault::Blocked {
            step,
            why: format!(
                "the home folders come to {}, and the drive has {} free: plug in a bigger one",
                copy::gigabytes(needed),
                copy::gigabytes(free)
            ),
        });
    }

    // A key for this archive alone, sealed beside it before anything is
    // packed, so no archive is ever written that nothing opens.
    let key = VaultKey::generate();
    if !target.recipients.is_empty() {
        let sealed = key.seal_to_keys(&target.recipients)?;
        machine.write(&format!("{folder}/{KEY_TO_YUBIKEYS}"), &sealed)?;
    }
    let stick = vault_key(machine)?.ok_or(Fault::Blocked {
        step,
        why: "the stick's vault isn't open".into(),
    })?;
    let for_stick = stick.encrypt(key.to_secret().expose().as_bytes())?;
    machine.write(&format!("{folder}/{KEY_TO_STICK}"), &for_stick)?;
    machine.write(HELD_KEY, key.to_secret().expose().as_bytes())?;

    let compressor = compressor()?;
    let mut args = vec!["--create", "--file=-", "--sparse", &compressor];
    args.extend(KEPT);
    let left_out = left_out(&target.leave_out)?;
    args.extend(left_out.iter().map(String::as_str));
    let into = format!("--directory={home}");
    args.extend([into.as_str(), "."]);
    let archive = format!("{folder}/{ARCHIVE}");
    meter.stage_by_reads("Packing the home folders", Some(needed));
    let ran = machine.run_sealed(Tool::Tar, &args, &key, &archive)?;
    let unread = match ran.status {
        // A file changed while it was read (1) is kept as it was read.
        0 | 1 => false,
        // Some files couldn't be read, and what tar said of them is kept
        // beside it. Reading it back shows whether the rest is whole.
        2 => {
            machine.write(&format!("{folder}/{UNREAD}"), &ran.stderr)?;
            true
        }
        _ => return Err(ran.refusal()),
    };

    meter.stage_by_reads("Reading the packed home folders back", Some(needed));
    let back = machine.run_unsealed(
        Tool::Tar,
        &["--list", "--file=-", &compressor],
        &key,
        &archive,
    )?;
    if back.status != 0 {
        return Err(Fault::Unproven {
            step,
            standing: format!(
                "the archive on {} didn't read back whole, so nothing was wiped: {}",
                drive.said,
                String::from_utf8_lossy(&back.stderr).trim()
            ),
        });
    }
    let carried = Carried {
        format: 1,
        machine: target.machine.clone(),
        accounts: accounts_there(machine)?,
        bytes: needed,
        unread,
    };
    let mut manifest = serde_json::to_vec_pretty(&carried)
        .map_err(|why| Fault::unreadable("what the archive holds", why))?;
    manifest.push(b'\n');
    machine.write(&format!("{folder}/{MANIFEST}"), &manifest)?;

    let mut said = format!(
        "its home folders, {}, are packed onto {} in \"{} carried home\" and read back whole",
        copy::gigabytes(needed),
        drive.said,
        target.machine
    );
    if unread {
        // Writing to a String can't fail.
        _ = write!(
            said,
            ". Some files couldn't be read: what tar said of them is in \"{UNREAD}\" beside it"
        );
    }
    Ok(said)
}

/// The home folders put back into the new system from the drive they were
/// packed onto, before it ever starts, owned as they were.
pub struct RestoreHome;

impl Step for RestoreHome {
    fn id(&self) -> &'static str {
        "restore-home"
    }

    fn place(&self) -> Place {
        Place::Stick
    }

    fn title(&self) -> &'static str {
        "Put its home folders back"
    }

    fn observe(&self, machine: &mut dyn Machine) -> Result<Standing, Fault> {
        let Some(uuid) = packed_onto(machine) else {
            return Ok(Standing::Done {
                evidence: "no home folders were carried, so they start empty".into(),
            });
        };
        if machine.journal().knows(self.id(), &uuid) {
            return Ok(Standing::Done {
                evidence: "its home folders are back, as they were packed".into(),
            });
        }
        if machine.resolve(&by_uuid(&uuid))?.is_none() {
            return Ok(Standing::Blocked {
                why: "the drive its home folders were packed onto isn't plugged in".into(),
                remedy: "plug it in. If the guide said it runs from memory, the stick can come out to make a port. The guide looks again by itself".into(),
            });
        }
        Ok(Standing::Needed { asks: vec![] })
    }

    fn run(&self, machine: &mut dyn Machine, _answers: &Answers) -> Result<(), Fault> {
        let uuid = packed_onto(machine).ok_or(Fault::Blocked {
            step: self.id(),
            why: "no home folders were packed".into(),
        })?;
        let partition = machine.resolve(&by_uuid(&uuid))?.ok_or(Fault::Blocked {
            step: self.id(),
            why: "the drive its home folders were packed onto isn't plugged in".into(),
        })?;
        mount(machine, &partition, &["ro"])?;
        let restored = restore(machine, self.id());
        // Unmounted whatever happened, so the drive can come out whole.
        machine.run(Tool::Umount, &[ONTO])?.ok()?;
        restored?;
        machine.journal().record(self.id(), &uuid)
    }
}

fn by_uuid(uuid: &str) -> String {
    format!("/dev/disk/by-uuid/{uuid}")
}

/// The archive's key: held since it was packed, or, after a restart, from
/// its copy sealed to the stick's vault, which the install opened.
fn archive_key(
    machine: &mut dyn Machine,
    step: &'static str,
    folder: &str,
) -> Result<VaultKey, Fault> {
    if let Some(text) = held(machine, HELD_KEY)? {
        return VaultKey::from_secret(&text);
    }
    let stick = vault_key(machine)?.ok_or(Fault::Blocked {
        step,
        why: "the stick's vault isn't open".into(),
    })?;
    let sealed = machine
        .read(&format!("{folder}/{KEY_TO_STICK}"))?
        .ok_or_else(|| Fault::Blocked {
            step,
            why: format!("the drive holds no \"{KEY_TO_STICK}\" to open the archive with"),
        })?;
    let text = stick.decrypt(&sealed)?;
    VaultKey::from_secret(&Secret::new(String::from_utf8_lossy(&text).into_owned()))
}

/// The archive unpacked into the new system's home folders, each account's
/// files then given the numbers the new system gives that account.
fn restore(machine: &mut dyn Machine, step: &'static str) -> Result<(), Fault> {
    let target = target(machine)?;
    let folder = folder(&target.machine);
    let manifest = machine
        .read(&format!("{folder}/{MANIFEST}"))?
        .ok_or_else(|| Fault::Blocked {
            step,
            why: format!(
                "the drive holds no \"{} carried home/{MANIFEST}\"",
                target.machine
            ),
        })?;
    let carried: Carried = serde_json::from_slice(&manifest)
        .map_err(|why| Fault::unreadable("what the archive holds", why))?;
    let key = archive_key(machine, step, &folder)?;

    let compressor = compressor()?;
    let home = format!("--directory={ROOT}/home");
    let mut args = vec![
        "--extract",
        "--file=-",
        "--same-permissions",
        &compressor,
        &home,
    ];
    args.extend(KEPT);
    machine
        .meter()
        .stage_by_reads("Putting the home folders back", Some(carried.bytes));
    machine
        .run_unsealed(Tool::Tar, &args, &key, &format!("{folder}/{ARCHIVE}"))?
        .ok()?;

    let now = accounts_there(machine)?;
    for old in &carried.accounts {
        let Some(new) = now.iter().find(|new| new.name == old.name) else {
            continue;
        };
        let at = format!("{ROOT}/home/{}", old.name);
        let changes = [
            (old.uid != new.uid).then(|| (format!("{}", old.uid), format!("{}", new.uid))),
            (old.gid != new.gid).then(|| (format!(":{}", old.gid), format!(":{}", new.gid))),
        ];
        for (from, to) in changes.into_iter().flatten() {
            let from = format!("--from={from}");
            machine
                .run(
                    Tool::Chown,
                    &["--recursive", "--no-dereference", &from, &to, &at],
                )?
                .ok()?;
        }
    }
    Ok(())
}
