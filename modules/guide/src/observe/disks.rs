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

pub fn observe(machine: &mut dyn Machine) -> Result<Vec<Disk>, Fault> {
    parse(
        &machine
            .run(Tool::Lsblk, &["--json", "--output-all", "--bytes"])?
            .ok()?,
    )
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
        stand_in.prints(
            Tool::Lsblk,
            &["--json", "--output-all", "--bytes"],
            surface!("lsblk.json"),
        );
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
}
