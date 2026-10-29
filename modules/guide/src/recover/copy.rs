//! A machine's home folders copied off onto a drive the person plugs in,
//! the first thing to do with a damaged filesystem, before any repair
//! writes to it. Nothing on the drive is removed or formatted: the copy
//! goes into a folder named for the machine, and copying again carries on
//! where the last copy stopped. rsync reads past a file it can't read,
//! and what it couldn't is kept beside the copy, so one bad sector loses
//! one file rather than the rest. Caches are left behind, since every
//! program makes its own again.

use std::fmt::Write as _;

use crate::install::ROOT;
use crate::install::disk::described;
use crate::observe::disks::{self, Kind};
use crate::{Answer, Answers, Ask, Fault, Machine, Place, Standing, Step, Tool};

/// Where the chosen drive is mounted while it's written.
const ONTO: &str = "/run/guide/copy";

/// What the copy did, kept until the stick restarts.
pub const COPIED: &str = "/run/guide/copied";

/// The filesystems a drive the stick writes onto may have.
const WRITABLE: &[&str] = &["exfat", "vfat", "ext4", "btrfs", "xfs"];

/// What a cache folder is called, which no copy takes.
const CACHE: &str = ".cache";

/// A partition a copy can go onto, with how the person knows it.
struct Drive {
    partition: String,
    said: String,
}

/// Every partition on a plugged-in drive the stick can write onto, never
/// the stick's own, whose vault holds the machine's keys.
fn drives(machine: &mut dyn Machine) -> Result<Vec<Drive>, Fault> {
    let mut found = vec![];
    for disk in disks::observe(machine)? {
        let stick = disk
            .partitions
            .iter()
            .any(|partition| partition.label.as_deref() == Some("vault"));
        if disk.kind != Kind::Removable || stick {
            continue;
        }
        for partition in &disk.partitions {
            let Some(filesystem) = partition.filesystem.as_deref() else {
                continue;
            };
            let free = partition.mounted.is_empty() || partition.mounted == [ONTO];
            if WRITABLE.contains(&filesystem) && free {
                found.push(Drive {
                    partition: partition.path.clone(),
                    said: format!("{}, its {filesystem} partition", described(&disk)),
                });
            }
        }
    }
    Ok(found)
}

/// The folder the copy goes into, named for the machine as it names
/// itself, so two machines' copies never meet on one drive.
fn folder(machine: &mut dyn Machine) -> Result<String, Fault> {
    let named = match machine.resolve_inside(ROOT, &format!("{ROOT}/etc/hostname"))? {
        Some(at) => machine.read(&format!("{ROOT}{at}"))?,
        None => None,
    };
    let host = named
        .map(|bytes| String::from_utf8_lossy(&bytes).trim().to_string())
        .filter(|host| !host.is_empty() && !host.contains('/'));
    Ok(match host {
        Some(host) => format!("{host} home folders"),
        None => "home folders".into(),
    })
}

/// Bytes as a person reads them, in whole gigabytes but never none.
fn gigabytes(bytes: u64) -> String {
    format!("{} GB", bytes.div_ceil(1_000_000_000).max(1))
}

/// The number on the last line a measuring tool printed, whatever it
/// said of what it couldn't read.
fn measured(printed: &str) -> Option<u64> {
    printed
        .lines()
        .rev()
        .find_map(|line| line.split_whitespace().next()?.parse().ok())
}

/// The machine's home folders copied onto a drive the person chooses.
pub struct CopyHome;

impl Step for CopyHome {
    fn id(&self) -> &'static str {
        "copy-home"
    }

    fn place(&self) -> Place {
        Place::Stick
    }

    fn title(&self) -> &'static str {
        "Copy its home folders onto a drive"
    }

    fn observe(&self, machine: &mut dyn Machine) -> Result<Standing, Fault> {
        if let Some(kept) = machine.read(COPIED)? {
            return Ok(Standing::Done {
                evidence: String::from_utf8_lossy(&kept).into_owned(),
            });
        }
        let found = drives(machine)?;
        if found.is_empty() {
            return Ok(Standing::Blocked {
                why: "no drive the home folders can be copied onto is plugged in".into(),
                remedy: "plug in a USB drive with an exFAT, FAT, ext4, btrfs or XFS partition and room for them: nothing on it is removed. If the guide said it runs from memory, the stick can come out to make a port. The guide looks again by itself".into(),
            });
        }
        let folder = folder(machine)?;
        Ok(Standing::Needed {
            asks: vec![Ask::Choose {
                id: "drive",
                question: format!(
                    "Which drive should they be copied onto? Nothing on it is removed: they go into a folder named \"{folder}\", leaving out caches."
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
        let drive = drives(machine)?
            .into_iter()
            .nth(*at)
            .ok_or(Fault::Blocked {
                step: self.id(),
                why: "that drive isn't plugged in any more".into(),
            })?;
        let folder = folder(machine)?;
        let mounted = disks::observe(machine)?
            .iter()
            .flat_map(|disk| disk.partitions.iter())
            .any(|partition| partition.path == drive.partition && partition.mounted == [ONTO]);
        if !mounted {
            machine
                .run(
                    Tool::Mount,
                    &["-o", "X-mount.mkdir", &drive.partition, ONTO],
                )?
                .ok()?;
        }
        let copied = copy(machine, self.id(), &folder);
        // Unmounted whatever happened, so the drive can come out whole.
        machine.run(Tool::Umount, &[ONTO])?.ok()?;
        let unread = copied?;
        let mut said = format!(
            "its home folders are copied onto {}, in \"{folder}\", and the drive can come out",
            drive.said
        );
        if unread {
            // Writing to a String can't fail.
            _ = write!(
                said,
                ". Some files couldn't be read: what rsync said of them is in \"{folder}, not copied.txt\" beside it"
            );
        }
        machine.write(COPIED, said.as_bytes())
    }
}

/// The home folders copied into `folder` on the mounted drive, once
/// they're known to fit. Says whether some files couldn't be read.
fn copy(machine: &mut dyn Machine, step: &'static str, folder: &str) -> Result<bool, Fault> {
    let home = format!("{ROOT}/home/");
    let meter = machine.meter();
    meter.stage("Measuring the home folders", None);
    let excluded = format!("--exclude={CACHE}");
    let summed = machine.run(Tool::Du, &["--summarize", "--bytes", &excluded, &home])?;
    let needed =
        measured(&String::from_utf8_lossy(&summed.stdout)).ok_or_else(|| summed.refusal())?;
    let free = machine
        .run(Tool::Df, &["--output=avail", "--block-size=1", ONTO])?
        .ok()?;
    let free = measured(&free).ok_or(Fault::unreadable(ONTO, "df gave no room left on it"))?;
    if needed > free {
        return Err(Fault::Blocked {
            step,
            why: format!(
                "the home folders need {}, and the drive has {} free: plug in a bigger one, or copy only what matters from the root shell",
                gigabytes(needed),
                gigabytes(free)
            ),
        });
    }
    meter.stage_by_reads("Copying the home folders", Some(needed));
    let into = format!("{ONTO}/{folder}/");
    let excluded = format!("--exclude={CACHE}/");
    // Times are kept to two seconds, as FAT and exFAT keep them, so a copy
    // run again takes only what's changed. No owner or permission is
    // asked of a drive that may hold neither.
    let ran = machine.run(
        Tool::Rsync,
        &[
            "--recursive",
            "--times",
            "--modify-window=2",
            &excluded,
            &home,
            &into,
        ],
    )?;
    match ran.status {
        0 => Ok(false),
        // Some files couldn't be read (23) or went while it ran (24).
        23 | 24 => {
            let unread = format!("{ONTO}/{folder}, not copied.txt");
            machine.write(&unread, &ran.stderr)?;
            Ok(true)
        }
        _ => Err(ran.refusal()),
    }
}
