//! The machine's disks and their partitions, as `lsblk` gives them, and
//! which of them a system can be installed on.

use serde::Deserialize;

use crate::{Fault, Machine, Tool};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Disk {
    pub path: String,
    /// Its name under /dev/disk/by-id, which a machine's file names it by.
    pub by_id: Option<String>,
    pub model: Option<String>,
    pub serial: Option<String>,
    pub bytes: u64,
    pub kind: Kind,
    pub partitions: Vec<Partition>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Built into the machine, such as its NVMe drive, or a VM's disk.
    Internal,
    /// Plugged in, or one the kernel calls removable: a stick, a card.
    Removable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Partition {
    pub path: String,
    pub number: Option<u32>,
    /// Its GPT name, which core's disk layout gives every partition.
    pub label: Option<String>,
    pub filesystem: Option<String>,
    /// Its filesystem's UUID, which stays the same however the drive is
    /// plugged in again.
    pub uuid: Option<String>,
    /// Where it's mounted now, if anywhere.
    pub mounted: Vec<String>,
}

impl Disk {
    /// Whether a system can be installed here: built in, and nothing on it
    /// mounted, so it's neither the stick nor a disk in use.
    pub fn installable(&self) -> bool {
        self.kind == Kind::Internal && !self.in_use()
    }

    /// Whether anything on it is mounted.
    pub fn in_use(&self) -> bool {
        self.partitions
            .iter()
            .any(|partition| !partition.mounted.is_empty())
    }
}

/// How lsblk is asked for the disks: only the columns `Device` reads.
/// Asked for every column, lsblk 2.42 prints `sched` from a stack buffer
/// it never filled when a device's queue/scheduler can't be read, as for
/// an opened LUKS volume on the stick, so now and then a byte that isn't
/// UTF-8 stopped the guide mid-recovery.
pub const LSBLK: &[&str] = &[
    "--json",
    "--bytes",
    "--output",
    "NAME,PATH,TYPE,ID-LINK,MODEL,SERIAL,SIZE,TRAN,RM,PARTN,PARTLABEL,FSTYPE,UUID,MOUNTPOINTS",
];

pub fn observe(machine: &mut dyn Machine) -> Result<Vec<Disk>, Fault> {
    parse(&machine.run(Tool::Lsblk, LSBLK)?.ok()?)
}

#[derive(Deserialize)]
struct Listing {
    blockdevices: Vec<Device>,
}

#[derive(Deserialize)]
struct Device {
    name: String,
    path: String,
    #[serde(rename = "type")]
    kind: String,
    #[serde(rename = "id-link")]
    id_link: Option<String>,
    model: Option<String>,
    serial: Option<String>,
    size: u64,
    tran: Option<String>,
    rm: bool,
    partn: Option<u32>,
    partlabel: Option<String>,
    fstype: Option<String>,
    uuid: Option<String>,
    #[serde(default)]
    mountpoints: Vec<Option<String>>,
    #[serde(default)]
    children: Vec<Device>,
}

pub fn parse(json: &str) -> Result<Vec<Disk>, Fault> {
    let listing: Listing =
        serde_json::from_str(json).map_err(|why| Fault::unreadable("lsblk's disks", why))?;
    Ok(listing
        .blockdevices
        .into_iter()
        // zram is a disk to lsblk, but it's memory.
        .filter(|device| device.kind == "disk" && !device.name.starts_with("zram"))
        .map(|device| Disk {
            kind: if device.rm || device.tran.as_deref() == Some("usb") {
                Kind::Removable
            } else {
                Kind::Internal
            },
            partitions: device
                .children
                .into_iter()
                .filter(|child| child.kind == "part")
                .map(|child| Partition {
                    // A partition holding LUKS is mounted through what it
                    // opens into, so those count as its own.
                    mounted: mounted(&child),
                    path: child.path,
                    number: child.partn,
                    label: child.partlabel,
                    filesystem: child.fstype,
                    uuid: child.uuid,
                })
                .collect(),
            path: device.path,
            by_id: device.id_link,
            model: device.model,
            serial: device.serial,
            bytes: device.size,
        })
        .collect())
}

/// Everywhere a device, or anything opened from it, is mounted.
fn mounted(device: &Device) -> Vec<String> {
    device
        .mountpoints
        .iter()
        .flatten()
        .cloned()
        .chain(device.children.iter().flat_map(mounted))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stand_in::StandIn;

    #[test]
    fn the_surfaces_own_drive_is_internal_and_in_use() {
        let mut stand_in = StandIn::new();
        stand_in.prints(Tool::Lsblk, LSBLK, surface!("lsblk.json"));
        let disks = observe(&mut stand_in).unwrap();
        assert_eq!(disks.len(), 1);
        let disk = &disks[0];
        assert_eq!(disk.by_id.as_deref(), Some("nvme-eui.ace42e0005506bd1"));
        assert_eq!(disk.kind, Kind::Internal);
        assert_eq!(disk.bytes, 128_035_676_160);
        assert_eq!(disk.partitions.len(), 3);
        assert!(disk.in_use());
        assert!(!disk.installable());
    }

    #[test]
    fn the_surfaces_partitions_say_where_theyre_mounted() {
        let disks = parse(surface!("lsblk.json")).unwrap();
        let root = &disks[0].partitions[0];
        assert_eq!(root.label.as_deref(), Some("root"));
        assert_eq!(root.filesystem.as_deref(), Some("ext4"));
        assert!(
            root.mounted.iter().any(|point| point == "/"),
            "{:?}",
            root.mounted
        );
    }

    /// One block device as lsblk lists it, with only the fields read.
    fn block(
        name: &str,
        path: &str,
        kind: &str,
        tran: Option<&str>,
        rm: bool,
        mounted: Option<&str>,
        children: &[serde_json::Value],
    ) -> serde_json::Value {
        serde_json::json!({
            "name": name, "path": path, "type": kind, "size": 1u64 << 30,
            "tran": tran, "rm": rm, "mountpoints": [mounted], "children": children,
        })
    }

    /// A disk with one LUKS partition, its btrfs mounted where given.
    fn device(name: &str, tran: Option<&str>, rm: bool, mounted: Option<&str>) -> String {
        let opened = block(
            "system",
            "/dev/mapper/system",
            "crypt",
            None,
            false,
            mounted,
            &[],
        );
        let partition = block(
            &format!("{name}1"),
            &format!("/dev/{name}1"),
            "part",
            tran,
            rm,
            None,
            &[opened],
        );
        block(
            name,
            &format!("/dev/{name}"),
            "disk",
            tran,
            rm,
            None,
            &[partition],
        )
        .to_string()
    }

    fn listing(devices: &[String]) -> String {
        format!("{{\"blockdevices\":[{}]}}", devices.join(","))
    }

    #[test]
    fn a_stick_is_never_installable() {
        let disks = parse(&listing(&[
            device("sda", Some("usb"), false, None),
            device("sdb", None, true, None),
        ]))
        .unwrap();
        assert!(disks.iter().all(|disk| disk.kind == Kind::Removable));
        assert!(disks.iter().all(|disk| !disk.installable()));
    }

    #[test]
    fn an_unused_internal_disk_is_installable_and_a_vms_counts() {
        let disks = parse(&listing(&[
            device("nvme0n1", Some("nvme"), false, None),
            device("vda", None, false, None),
        ]))
        .unwrap();
        assert!(disks.iter().all(Disk::installable));
    }

    #[test]
    fn a_disk_mounted_through_luks_is_in_use() {
        let disks = parse(&listing(&[device(
            "nvme0n1",
            Some("nvme"),
            false,
            Some("/"),
        )]))
        .unwrap();
        assert_eq!(disks[0].partitions[0].mounted, ["/"]);
        assert!(!disks[0].installable());
    }

    #[test]
    fn zram_isnt_a_disk() {
        let disks = parse(&listing(&[device("zram0", None, false, Some("[SWAP]"))])).unwrap();
        assert_eq!(disks, []);
    }

    /// A device as lsblk prints it when asked `LSBLK`: only the columns
    /// asked for, each with a value, so one left out of `LSBLK` is missing
    /// here too. Every column it might print has a value, or this panics.
    fn asked(name: &str, kind: &str, children: &[serde_json::Value]) -> serde_json::Value {
        let columns = LSBLK.last().expect("LSBLK names its columns last");
        let mut device: serde_json::Map<String, serde_json::Value> = columns
            .split(',')
            .map(|column| {
                let value = match column {
                    "NAME" => serde_json::json!(name),
                    "PATH" => serde_json::json!(format!("/dev/{name}")),
                    "TYPE" => serde_json::json!(kind),
                    "ID-LINK" => serde_json::json!(format!("usb-stick-{name}")),
                    "MODEL" => serde_json::json!("Flash Drive"),
                    "SERIAL" => serde_json::json!("0123"),
                    "SIZE" => serde_json::json!(1u64 << 30),
                    "TRAN" => serde_json::json!("usb"),
                    "RM" => serde_json::json!(false),
                    "PARTN" => serde_json::json!(1),
                    "PARTLABEL" => serde_json::json!("system-boot"),
                    "FSTYPE" => serde_json::json!("vfat"),
                    "UUID" => serde_json::json!("4A2B-11C3"),
                    "MOUNTPOINTS" => serde_json::json!(["/boot"]),
                    other => panic!("asked lsblk for {other}, which nothing here gives"),
                };
                (column.to_lowercase(), value)
            })
            .collect();
        device.insert("children".to_string(), serde_json::json!(children));
        device.into()
    }

    // Each field read falls back quietly when its column isn't printed
    // (None, or Internal for the transport), so a column left out of
    // LSBLK would read as nothing there rather than fail.
    #[test]
    fn every_field_read_is_a_column_asked_for() {
        let partition = asked("sda1", "part", &[]);
        let disk = asked("sda", "disk", &[partition]);
        let disks = parse(&listing(&[disk.to_string()])).unwrap();
        assert_eq!(
            disks,
            [Disk {
                path: "/dev/sda".to_string(),
                by_id: Some("usb-stick-sda".to_string()),
                model: Some("Flash Drive".to_string()),
                serial: Some("0123".to_string()),
                bytes: 1 << 30,
                kind: Kind::Removable,
                partitions: vec![Partition {
                    path: "/dev/sda1".to_string(),
                    number: Some(1),
                    label: Some("system-boot".to_string()),
                    filesystem: Some("vfat".to_string()),
                    uuid: Some("4A2B-11C3".to_string()),
                    mounted: vec!["/boot".to_string()],
                }],
            }]
        );
    }
}
