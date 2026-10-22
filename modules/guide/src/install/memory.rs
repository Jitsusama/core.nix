//! The stick's system copied into memory and run from there, so the stick
//! can come out while a YubiKey takes its port, on a laptop with only one
//! that fits both.
//!
//! The stick's store is a dm-verity device, `usr`, over two of its
//! partitions: the store, and the hashes that prove it. Both are copied
//! into a memory filesystem of their own, checked against the same root
//! hash the stick booted with, and handed to the kernel as loop devices in
//! place of the stick's partitions. The kernel swaps the device's table in
//! one call, suspending and resuming it around the swap, so nothing ever
//! reads the store while it's moving, and what's read after is proved by
//! the same hash as before: a copy that differs by a bit can't be read.
//!
//! A machine without the memory to hold both, and room to install beside
//! them, keeps running from the stick, and the steps that need a free port
//! say to keep the stick in.

use crate::{Answers, Fault, Machine, Place, Standing, Step, Tool};

/// The store's verity device, as systemd names the one `usrhash=` makes.
pub const DEVICE: &str = "usr";
/// Where the copies are kept: memory of their own, sized to them.
pub const COPIES: &str = "/run/stick";
/// What the journal says when the machine kept running from the stick.
const ON_THE_STICK: &str = "too little memory";
/// What it says once the stick can come out.
const IN_MEMORY: &str = "in memory";
/// Memory left over for the install itself, beyond the copies.
const HEADROOM: u64 = 1 << 30;
/// The kernel's block device number for loop devices.
const LOOP_MAJOR: &str = "7";

/// The verity table, as `dmsetup table` prints it: where on the device it
/// starts and how long it is, then the target's arguments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Table {
    words: Vec<String>,
}

impl Table {
    /// Reads the one line of a verity table.
    pub fn parse(text: &str) -> Result<Table, Fault> {
        let words: Vec<String> = text.split_whitespace().map(String::from).collect();
        if words.len() < 13 || words[2] != "verity" {
            return Err(Fault::unreadable(
                "the stick's store device",
                format!("its table isn't one verity target: {}", text.trim()),
            ));
        }
        let table = Table { words };
        for number in [6, 7, 8, 9] {
            table.number(number)?;
        }
        Ok(table)
    }

    fn number(&self, at: usize) -> Result<u64, Fault> {
        self.words[at].parse().map_err(|_| {
            Fault::unreadable(
                "the stick's store device",
                format!("its table's {} isn't a number", self.words[at]),
            )
        })
    }

    pub fn version(&self) -> &str {
        &self.words[3]
    }

    /// The data and hash devices, by their numbers, as `major:minor`.
    pub fn data(&self) -> &str {
        &self.words[4]
    }

    pub fn hashes(&self) -> &str {
        &self.words[5]
    }

    pub fn data_block(&self) -> u64 {
        self.number(6).unwrap_or_default()
    }

    pub fn hash_block(&self) -> u64 {
        self.number(7).unwrap_or_default()
    }

    pub fn data_blocks(&self) -> u64 {
        self.number(8).unwrap_or_default()
    }

    /// Where the hashes start on their device, in hash blocks.
    pub fn hash_start(&self) -> u64 {
        self.number(9).unwrap_or_default()
    }

    pub fn algorithm(&self) -> &str {
        &self.words[10]
    }

    pub fn root_hash(&self) -> &str {
        &self.words[11]
    }

    pub fn salt(&self) -> &str {
        &self.words[12]
    }

    /// How many bytes of the data device the store is.
    pub fn data_bytes(&self) -> u64 {
        self.data_blocks() * self.data_block()
    }

    /// Whether both devices are loop devices: the copies in memory.
    pub fn in_memory(&self) -> bool {
        [self.data(), self.hashes()]
            .iter()
            .all(|device| device.split(':').next() == Some(LOOP_MAJOR))
    }

    /// The same table over other devices, every other argument as it was.
    pub fn over(&self, data: &str, hashes: &str) -> String {
        let mut words = self.words.clone();
        words[4] = data.into();
        words[5] = hashes.into();
        words.join(" ")
    }
}

/// The store's verity table, or none where the system isn't running from a
/// stick's verity store.
pub fn table(machine: &mut dyn Machine) -> Result<Option<Table>, Fault> {
    let ran = machine.run(Tool::Dmsetup, &["table", DEVICE])?;
    if ran.status != 0 {
        return Ok(None);
    }
    Table::parse(&ran.ok()?).map(Some)
}

/// Whether the stick can come out: its system copied into memory. None
/// when it isn't known either way, as where nothing was copied.
pub fn stick_can_come_out(machine: &mut dyn Machine) -> Option<bool> {
    let journal = machine.journal();
    if journal.knows(RUN_FROM_MEMORY, IN_MEMORY) {
        Some(true)
    } else if journal.knows(RUN_FROM_MEMORY, ON_THE_STICK) {
        Some(false)
    } else {
        None
    }
}

/// What a step that needs a free port says about the stick.
pub fn port_words(machine: &mut dyn Machine) -> &'static str {
    match stick_can_come_out(machine) {
        Some(true) => {
            " If no port is free, take the stick out first: the guide asks for it back when it needs it."
        }
        Some(false) => {
            " Keep the stick in: this machine hasn't the memory to run without it, so use another port, or a hub."
        }
        None => "",
    }
}

const RUN_FROM_MEMORY: &str = "run-from-memory";

/// A block device's file, by its number, as the kernel names it.
fn device(machine: &mut dyn Machine, number: &str) -> Result<String, Fault> {
    let uevent = read_text(machine, &format!("/sys/dev/block/{number}/uevent"))?;
    uevent
        .lines()
        .find_map(|line| line.strip_prefix("DEVNAME="))
        .map(|name| format!("/dev/{name}"))
        .ok_or_else(|| Fault::unreadable(format!("block device {number}"), "it has no name"))
}

/// A block device's size in bytes, by its number.
fn size(machine: &mut dyn Machine, number: &str) -> Result<u64, Fault> {
    let sectors = read_text(machine, &format!("/sys/dev/block/{number}/size"))?;
    sectors
        .trim()
        .parse::<u64>()
        .map(|sectors| sectors * 512)
        .map_err(|why| Fault::unreadable(format!("block device {number}'s size"), why))
}

fn read_text(machine: &mut dyn Machine, path: &str) -> Result<String, Fault> {
    machine
        .read(path)?
        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
        .ok_or_else(|| Fault::unreadable(path, "it isn't there"))
}

/// The memory free for use, in bytes, as the kernel reckons it.
fn available(machine: &mut dyn Machine) -> Result<u64, Fault> {
    let meminfo = read_text(machine, "/proc/meminfo")?;
    meminfo
        .lines()
        .find_map(|line| line.strip_prefix("MemAvailable:"))
        .and_then(|kilobytes| {
            kilobytes
                .trim()
                .trim_end_matches("kB")
                .trim()
                .parse::<u64>()
                .ok()
        })
        .map(|kilobytes| kilobytes * 1024)
        .ok_or_else(|| {
            Fault::unreadable("/proc/meminfo", "it says nothing of the memory available")
        })
}

/// The loop device over a file, made read-only, or the one already there.
fn attach(machine: &mut dyn Machine, file: &str) -> Result<String, Fault> {
    let listed = machine.run(Tool::Losetup, &["--associated", file])?.ok()?;
    if let Some((device, _)) = listed.lines().next().and_then(|line| line.split_once(':')) {
        return Ok(device.to_string());
    }
    Ok(machine
        .run(Tool::Losetup, &["--find", "--show", "--read-only", file])?
        .ok()?
        .trim()
        .to_string())
}

/// A device's number, as `major:minor`, from its file's name.
fn number(machine: &mut dyn Machine, device: &str) -> Result<String, Fault> {
    let name = device.trim_start_matches("/dev/");
    Ok(read_text(machine, &format!("/sys/class/block/{name}/dev"))?
        .trim()
        .to_string())
}

pub struct RunFromMemory;

impl Step for RunFromMemory {
    fn id(&self) -> &'static str {
        RUN_FROM_MEMORY
    }

    fn place(&self) -> Place {
        Place::Stick
    }

    fn title(&self) -> &'static str {
        "Copy the stick's system into memory"
    }

    fn observe(&self, machine: &mut dyn Machine) -> Result<Standing, Fault> {
        if machine.journal().knows(self.id(), ON_THE_STICK) {
            return Ok(Standing::Done {
                evidence: "this machine hasn't the memory to run without the stick, so it stays in"
                    .into(),
            });
        }
        Ok(match table(machine)? {
            None => Standing::Done {
                evidence: "this system doesn't run from a stick's store".into(),
            },
            Some(table) if table.in_memory() => Standing::Done {
                evidence: "the stick's system runs from memory, so the stick can come out".into(),
            },
            Some(_) => Standing::Needed { asks: vec![] },
        })
    }

    fn run(&self, machine: &mut dyn Machine, _answers: &Answers) -> Result<(), Fault> {
        let Some(table) = table(machine)? else {
            return Ok(());
        };
        let data_bytes = table.data_bytes();
        let hash_bytes = size(machine, table.hashes())?;
        let needed = data_bytes + hash_bytes;
        if available(machine)? < needed + HEADROOM {
            return machine.journal().record(self.id(), ON_THE_STICK);
        }
        let data_from = device(machine, table.data())?;
        let hashes_from = device(machine, table.hashes())?;
        let mounts = read_text(machine, "/proc/mounts")?;
        if !mounts
            .lines()
            .any(|line| line.split_whitespace().nth(1) == Some(COPIES))
        {
            // Sized to the copies, rounded up to a mebibyte, so they can't
            // crowd out what the install needs.
            let size = format!(
                "size={}k,mode=0700,X-mount.mkdir=0700",
                needed.div_ceil(1 << 20) * 1024
            );
            machine
                .run(Tool::Mount, &["-t", "tmpfs", "-o", &size, "stick", COPIES])?
                .ok()?;
        }
        let data = format!("{COPIES}/store");
        let hashes = format!("{COPIES}/store-verity");
        let meter = machine.meter();
        meter.stage("Copying it into memory", Some(needed));
        machine.copy(&data_from, &data, data_bytes)?;
        machine.copy(&hashes_from, &hashes, hash_bytes)?;

        // Checked whole before the kernel reads from it, so a copy that
        // differs stops here, the stick's own still in use.
        meter.stage_by_reads("Checking the copy against the stick's hash", Some(needed));
        let verified = machine.run(
            Tool::Veritysetup,
            &[
                "verify",
                &data,
                &hashes,
                table.root_hash(),
                "--no-superblock",
                &format!("--format={}", table.version()),
                &format!("--data-block-size={}", table.data_block()),
                &format!("--hash-block-size={}", table.hash_block()),
                &format!("--data-blocks={}", table.data_blocks()),
                &format!("--hash-offset={}", table.hash_start() * table.hash_block()),
                &format!("--hash={}", table.algorithm()),
                &format!("--salt={}", table.salt()),
            ],
        )?;
        if verified.status != 0 {
            return Err(verified.refusal());
        }

        meter.stage("Running from the copy", None);
        let data_loop = attach(machine, &data)?;
        let hashes_loop = attach(machine, &hashes)?;
        let swapped = table.over(
            &number(machine, &data_loop)?,
            &number(machine, &hashes_loop)?,
        );
        // Without waiting on udev, which has nothing to make: the device's
        // node stays as it is. Read-only, as dm-verity takes no other table.
        machine
            .run(
                Tool::Dmsetup,
                &[
                    "reload",
                    "--readonly",
                    "--noudevsync",
                    DEVICE,
                    "--table",
                    &swapped,
                ],
            )?
            .ok()?;
        // Resuming a device with a table loaded swaps it in, suspended
        // around the swap within the one call.
        machine
            .run(Tool::Dmsetup, &["resume", "--noudevsync", DEVICE])?
            .ok()?;
        machine.journal().record(self.id(), IN_MEMORY)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TABLE: &str = "0 8916176 verity 1 8:3 8:2 4096 4096 1114522 1 sha256 165901d8b1f9d92d00dd81646c59efa4472f5b06e84f5fa76c8a179becea34e8 1f2e3d4c";

    #[test]
    fn a_verity_table_is_read_and_moved_onto_other_devices_alone() {
        let table = Table::parse(TABLE).unwrap();
        assert_eq!(table.data(), "8:3");
        assert_eq!(table.hashes(), "8:2");
        assert_eq!(table.data_bytes(), 1114522 * 4096);
        assert_eq!(table.hash_start(), 1);
        assert!(!table.in_memory());
        let moved = table.over("7:0", "7:1");
        assert_eq!(moved, TABLE.replace("8:3 8:2", "7:0 7:1"));
        assert!(Table::parse(&moved).unwrap().in_memory());
    }

    #[test]
    fn a_tables_optional_arguments_are_kept() {
        let with = format!("{TABLE} 1 ignore_zero_blocks");
        let table = Table::parse(&with).unwrap();
        assert!(table.over("7:0", "7:1").ends_with(" 1 ignore_zero_blocks"));
    }

    #[test]
    fn a_table_that_isnt_verity_is_refused() {
        assert!(Table::parse("0 2048 linear 8:3 0").is_err());
    }
}
