//! The stick's partition table. The image's entries are kept byte for byte,
//! since the stick's UKI finds its store by those partitions' GUIDs; the
//! writer only adds the vault's entry and writes both headers for the
//! stick's own size, the backup at its end where the image left it in the
//! middle (UEFI 2.10, section 5.3).

use crate::Fault;

pub const SECTOR: u64 = 512;
const SIGNATURE: &[u8; 8] = b"EFI PART";
const HEADER_BYTES: usize = 92;
const ENTRY_BYTES: usize = 128;
/// Where partitions start and end: 1 MiB, as every partitioning tool
/// aligns them.
pub const ALIGNMENT: u64 = 2048;

/// Microsoft's basic data partition, which every system reads as FAT.
const BASIC_DATA: [u8; 16] = [
    0xa2, 0xa0, 0xd0, 0xeb, 0xe5, 0xb9, 0x33, 0x44, 0x87, 0xc0, 0x68, 0xb6, 0xb7, 0x26, 0x99, 0xc7,
];

/// A GPT's entries and its disk's GUID, as an image or a stick holds them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Table {
    pub disk_guid: [u8; 16],
    /// Every entry slot, used or not, each as it was written.
    pub slots: Vec<[u8; ENTRY_BYTES]>,
}

/// A used entry's extent, in sectors, both ends included.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Partition {
    pub slot: usize,
    pub name: String,
    pub first: u64,
    pub last: u64,
}

impl Partition {
    pub fn offset(&self) -> u64 {
        self.first * SECTOR
    }

    pub fn bytes(&self) -> u64 {
        (self.last - self.first + 1) * SECTOR
    }
}

fn le64(bytes: &[u8]) -> u64 {
    u64::from_le_bytes(bytes[..8].try_into().expect("eight bytes"))
}

fn le32(bytes: &[u8]) -> u32 {
    u32::from_le_bytes(bytes[..4].try_into().expect("four bytes"))
}

impl Table {
    /// The primary table, from the first sectors of a disk or an image:
    /// its header at LBA 1 and the entries it points to.
    pub fn read(start: &[u8]) -> Result<Table, Fault> {
        let bad = |why: &str| Fault::unreadable("the image's partition table", why);
        let header = start
            .get(SECTOR as usize..SECTOR as usize + HEADER_BYTES)
            .ok_or_else(|| bad("it's shorter than a header"))?;
        if &header[..8] != SIGNATURE {
            return Err(bad("it isn't GPT"));
        }
        if le32(&header[12..]) as usize != HEADER_BYTES {
            return Err(bad("its header isn't the size UEFI gives it"));
        }
        let mut zeroed = header.to_vec();
        zeroed[16..20].fill(0);
        if crc32fast::hash(&zeroed) != le32(&header[16..]) {
            return Err(bad("its header's checksum is wrong"));
        }
        let at = le64(&header[72..]) * SECTOR;
        let count = le32(&header[80..]) as usize;
        if le32(&header[84..]) as usize != ENTRY_BYTES {
            return Err(bad("its entries aren't 128 bytes"));
        }
        let entries = start
            .get(at as usize..at as usize + count * ENTRY_BYTES)
            .ok_or_else(|| bad("its entries run past what was read"))?;
        if crc32fast::hash(entries) != le32(&header[88..]) {
            return Err(bad("its entries' checksum is wrong"));
        }
        Ok(Table {
            disk_guid: header[56..72].try_into().expect("sixteen bytes"),
            slots: entries.as_chunks::<ENTRY_BYTES>().0.to_vec(),
        })
    }

    pub fn partitions(&self) -> Vec<Partition> {
        self.slots
            .iter()
            .enumerate()
            .filter(|(_, entry)| entry[..16] != [0; 16])
            .map(|(slot, entry)| {
                let name: Vec<u16> = entry[56..128]
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .map(|pair| u16::from_le_bytes(*pair))
                    .take_while(|unit| *unit != 0)
                    .collect();
                Partition {
                    slot,
                    name: String::from_utf16_lossy(&name),
                    first: le64(&entry[32..]),
                    last: le64(&entry[40..]),
                }
            })
            .collect()
    }

    /// Sectors the entries take, rounded up.
    fn entry_sectors(&self) -> u64 {
        (self.slots.len() * ENTRY_BYTES).div_ceil(SECTOR as usize) as u64
    }

    /// The first sector a partition may use on a disk of this many.
    pub fn first_usable(&self) -> u64 {
        2 + self.entry_sectors()
    }

    /// The last, before the backup's entries and header.
    pub fn last_usable(&self, sectors: u64) -> u64 {
        sectors - 2 - self.entry_sectors()
    }

    /// Adds a FAT partition in the first free slot, from the first aligned
    /// sector after every other partition to the last aligned one the disk
    /// allows. Says where, or why it doesn't fit.
    pub fn add_vault(
        &mut self,
        name: &str,
        guid: [u8; 16],
        sectors: u64,
        least: u64,
    ) -> Result<Partition, Fault> {
        let after = self
            .partitions()
            .iter()
            .map(|partition| partition.last + 1)
            .max()
            .unwrap_or(self.first_usable());
        let first = after.div_ceil(ALIGNMENT) * ALIGNMENT;
        let end = (self.last_usable(sectors) + 1) / ALIGNMENT * ALIGNMENT;
        if end < first + least {
            return Err(Fault::Blocked {
                step: "write-stick",
                why: format!(
                    "the stick holds {} MB and needs at least {} MB",
                    sectors * SECTOR / 1_000_000,
                    (first + least) * SECTOR / 1_000_000 + 1
                ),
            });
        }
        let slot = self
            .slots
            .iter()
            .position(|entry| entry[..16] == [0; 16])
            .ok_or_else(|| {
                Fault::unreadable("the image's partition table", "it has no free slot")
            })?;
        let entry = &mut self.slots[slot];
        entry.fill(0);
        entry[..16].copy_from_slice(&BASIC_DATA);
        entry[16..32].copy_from_slice(&guid);
        entry[32..40].copy_from_slice(&first.to_le_bytes());
        entry[40..48].copy_from_slice(&(end - 1).to_le_bytes());
        for (unit, at) in name.encode_utf16().zip((56..128).step_by(2)) {
            entry[at..at + 2].copy_from_slice(&unit.to_le_bytes());
        }
        Ok(Partition {
            slot,
            name: name.into(),
            first,
            last: end - 1,
        })
    }

    /// The protective MBR, the primary header and entries, and the backup
    /// entries and header, each with the byte offset it goes at on a disk
    /// of this many sectors.
    pub fn render(&self, sectors: u64) -> Vec<(u64, Vec<u8>)> {
        let entries: Vec<u8> = self.slots.iter().flatten().copied().collect();
        let entries_crc = crc32fast::hash(&entries);
        let last = sectors - 1;
        let backup_entries = last - self.entry_sectors();
        let header = |mine: u64, other: u64, entries_at: u64| {
            let mut header = vec![0u8; SECTOR as usize];
            header[..8].copy_from_slice(SIGNATURE);
            header[8..12].copy_from_slice(&0x0001_0000u32.to_le_bytes());
            header[12..16].copy_from_slice(&(HEADER_BYTES as u32).to_le_bytes());
            header[24..32].copy_from_slice(&mine.to_le_bytes());
            header[32..40].copy_from_slice(&other.to_le_bytes());
            header[40..48].copy_from_slice(&self.first_usable().to_le_bytes());
            header[48..56].copy_from_slice(&self.last_usable(sectors).to_le_bytes());
            header[56..72].copy_from_slice(&self.disk_guid);
            header[72..80].copy_from_slice(&entries_at.to_le_bytes());
            header[80..84].copy_from_slice(&(self.slots.len() as u32).to_le_bytes());
            header[84..88].copy_from_slice(&(ENTRY_BYTES as u32).to_le_bytes());
            header[88..92].copy_from_slice(&entries_crc.to_le_bytes());
            let crc = crc32fast::hash(&header[..HEADER_BYTES]);
            header[16..20].copy_from_slice(&crc.to_le_bytes());
            header
        };
        let mut mbr = vec![0u8; SECTOR as usize];
        let protective = &mut mbr[446..462];
        protective[1..4].copy_from_slice(&[0x00, 0x02, 0x00]);
        protective[4] = 0xee;
        protective[5..8].copy_from_slice(&[0xff, 0xff, 0xff]);
        protective[8..12].copy_from_slice(&1u32.to_le_bytes());
        let covered = u32::try_from(sectors - 1).unwrap_or(u32::MAX);
        protective[12..16].copy_from_slice(&covered.to_le_bytes());
        mbr[510..512].copy_from_slice(&[0x55, 0xaa]);
        vec![
            (0, mbr),
            (SECTOR, header(1, last, 2)),
            (2 * SECTOR, entries.clone()),
            (backup_entries * SECTOR, entries),
            (last * SECTOR, header(last, 1, backup_entries)),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A table as the writer would put it on a disk this many sectors big,
    /// laid out as the disk holds it.
    fn disk(table: &Table, sectors: u64) -> Vec<u8> {
        let mut disk = vec![0u8; (sectors * SECTOR) as usize];
        for (at, bytes) in table.render(sectors) {
            disk[at as usize..at as usize + bytes.len()].copy_from_slice(&bytes);
        }
        disk
    }

    fn image() -> Table {
        let mut slots = vec![[0u8; ENTRY_BYTES]; 128];
        let entry = &mut slots[0];
        entry[..16].copy_from_slice(&[1; 16]);
        entry[16..32].copy_from_slice(&[2; 16]);
        entry[32..40].copy_from_slice(&2048u64.to_le_bytes());
        entry[40..48].copy_from_slice(&4095u64.to_le_bytes());
        entry[48] = 0x01;
        for (unit, at) in "esp".encode_utf16().zip((56..).step_by(2)) {
            entry[at..at + 2].copy_from_slice(&unit.to_le_bytes());
        }
        Table {
            disk_guid: [9; 16],
            slots,
        }
    }

    #[test]
    fn what_it_writes_it_reads_back() {
        let table = image();
        assert_eq!(Table::read(&disk(&table, 8192)).unwrap(), table);
    }

    #[test]
    fn the_images_entries_are_kept_byte_for_byte() {
        let mut table = image();
        let before = table.slots[0];
        table.add_vault("vault", [7; 16], 1 << 20, 2048).unwrap();
        assert_eq!(table.slots[0], before);
    }

    #[test]
    fn the_vault_fills_the_disk_from_the_next_mebibyte() {
        let mut table = image();
        let sectors = 100 * 2048 + 17;
        let vault = table.add_vault("vault", [7; 16], sectors, 2048).unwrap();
        assert_eq!(vault.name, "vault");
        assert_eq!(vault.first, 4096);
        assert_eq!((vault.last + 1) % ALIGNMENT, 0);
        assert!(vault.last <= table.last_usable(sectors));
        assert!(vault.last + ALIGNMENT > table.last_usable(sectors));
    }

    #[test]
    fn a_vault_after_a_partition_ending_mid_mebibyte_starts_at_the_next() {
        // As core's image does: its store ends at sector 5296111.
        let mut table = image();
        table.slots[0][40..48].copy_from_slice(&4999u64.to_le_bytes());
        let vault = table.add_vault("vault", [7; 16], 100 * 2048, 2048).unwrap();
        assert_eq!(vault.first, 6144);
    }

    #[test]
    fn a_stick_too_small_is_refused_before_anything_is_written() {
        let mut table = image();
        assert!(matches!(
            table.add_vault("vault", [7; 16], 4096 + 2048, 2048),
            Err(Fault::Blocked { .. })
        ));
    }

    #[test]
    fn the_backup_is_at_the_disks_end() {
        let table = image();
        let sectors = 8192;
        let disk = disk(&table, sectors);
        let last = &disk[((sectors - 1) * SECTOR) as usize..];
        assert_eq!(&last[..8], SIGNATURE);
        assert_eq!(le64(&last[24..]), sectors - 1);
        assert_eq!(le64(&last[32..]), 1);
    }

    #[test]
    fn a_damaged_table_is_refused() {
        let mut disk = disk(&image(), 8192);
        disk[2 * SECTOR as usize] ^= 1;
        assert!(matches!(
            Table::read(&disk),
            Err(Fault::Unreadable { why, .. }) if why == "its entries' checksum is wrong"
        ));
    }
}
