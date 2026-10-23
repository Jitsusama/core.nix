//! Writing the stick: the image core builds (the ESP, the verity hashes and
//! the store), then the vault in the rest of the stick, then a partition
//! table for the stick's own size, and reading all of it back.
//!
//! Both tables are cleared first and written last, so a write that stops
//! part way leaves a stick with no partitions rather than one whose table
//! points at half of them. Reading back drops what the system cached of
//! the stick first, so it reads what reached the stick rather than what
//! was sent: a write that only looked done once booted a laptop to `grub>`.

pub mod blocks;
pub mod device;
pub mod gpt;

use crate::progress::{Counted, Meter};
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom, Write};

use sha2::{Digest, Sha256};

use crate::Fault;
use crate::sign::Signer;
use crate::vault::store::{Fat, Store};
use blocks::{Partition, Unflushed};
use gpt::{ALIGNMENT, EFI_SYSTEM, SECTOR, Table};

/// The least a vault may be: a repair stick's needs only its manifest, and
/// FAT32 needs 65,525 clusters.
pub const LEAST_VAULT: u64 = 64 << 20;

/// The stick's image, as core builds it.
pub struct Image {
    file: File,
    table: Table,
}

impl Image {
    pub fn open(mut file: File) -> Result<Image, Fault> {
        let mut start = vec![0u8; (ALIGNMENT * SECTOR) as usize];
        file.seek(SeekFrom::Start(0))
            .and_then(|_| file.read_exact(&mut start))
            .map_err(|why| Fault::unreadable("the stick's image", why))?;
        let table = Table::read(&start)?;
        Ok(Image { file, table })
    }

    pub fn partitions(&self) -> Vec<gpt::Partition> {
        self.table.partitions()
    }
}

/// How many bytes of vault a stick of this many bytes would hold after the
/// image, or why it can't hold one at all.
pub fn vault_room(image: &Image, bytes: u64) -> Result<u64, Fault> {
    let mut table = image.table.clone();
    Ok(table
        .add_vault("vault", [0; 16], bytes / SECTOR, LEAST_VAULT / SECTOR)?
        .bytes())
}

/// What was written, for reading it back.
pub struct Written {
    pub vault: gpt::Partition,
    table: Table,
    sectors: u64,
    /// Each of the image's partitions, and the SHA-256 of what was written.
    copied: Vec<(gpt::Partition, Vec<u8>)>,
}

fn unwritten(why: io::Error) -> Fault {
    Fault::Unkept {
        path: "the stick".into(),
        why: why.to_string(),
    }
}

/// A new partition's GUID: random, marked as a version 4 UUID would be in
/// GPT's mixed-endian layout.
pub fn new_guid() -> Result<[u8; 16], Fault> {
    let mut guid = [0u8; 16];
    getrandom::fill(&mut guid).map_err(|why| Fault::unreadable("the system's randomness", why))?;
    guid[7] = (guid[7] & 0x0f) | 0x40;
    guid[8] = (guid[8] & 0x3f) | 0x80;
    Ok(guid)
}

/// Copies `bytes` from where the reader is to the writer, hashing them.
fn copy_hashed(from: &mut impl Read, to: &mut impl Write, bytes: u64) -> io::Result<Vec<u8>> {
    let mut hash = Sha256::new();
    let mut buffer = vec![0u8; 1 << 20];
    let mut left = bytes;
    while left > 0 {
        let count = buffer.len().min(left as usize);
        from.read_exact(&mut buffer[..count])?;
        hash.update(&buffer[..count]);
        to.write_all(&buffer[..count])?;
        left -= count as u64;
    }
    Ok(hash.finalize().to_vec())
}

/// Writes the image and a vault to the disk, a whole stick or an image
/// file the size of one. `fill` writes the vault's files. Every byte of
/// the image's partitions and the vault is counted on the meter as it's
/// written.
pub fn write(
    image: &mut Image,
    disk: &mut File,
    vault_guid: [u8; 16],
    signer: Option<&Signer>,
    meter: &Meter,
    fill: impl FnOnce(&mut dyn Store) -> Result<(), Fault>,
) -> Result<Written, Fault> {
    let bytes = disk.seek(SeekFrom::End(0)).map_err(unwritten)?;
    if !bytes.is_multiple_of(SECTOR) {
        return Err(Fault::unreadable(
            "the stick",
            "its size isn't whole sectors",
        ));
    }
    let sectors = bytes / SECTOR;
    let mut table = image.table.clone();
    let vault = table.add_vault("vault", vault_guid, sectors, LEAST_VAULT / SECTOR)?;

    // Signed before anything is written, so a key that won't sign leaves
    // the stick as it was.
    let mut signed = vec![];
    if let Some(signer) = signer {
        for partition in image.partitions() {
            if partition.kind == EFI_SYSTEM {
                let mut esp = vec![0u8; partition.bytes() as usize];
                image
                    .file
                    .seek(SeekFrom::Start(partition.offset()))
                    .and_then(|_| image.file.read_exact(&mut esp))
                    .map_err(|why| Fault::unreadable("the stick's image", why))?;
                sign_esp(&mut esp, signer)?;
                signed.push((partition.slot, esp));
            }
        }
        if signed.is_empty() {
            return Err(Fault::unreadable(
                "the stick's image",
                "it has no EFI system partition to sign",
            ));
        }
    }

    // The head holds the protective MBR and the primary table; the tail,
    // the backup. Clearing both first means nothing reads a table until
    // everything it points at is written.
    let head = table.first_usable() * SECTOR;
    let tail = table.last_usable(sectors) * SECTOR + SECTOR;
    for (start, length) in [(0, head), (tail, bytes - tail)] {
        let mut region = Partition::new(disk, start, length).map_err(unwritten)?;
        region
            .write_all(&vec![0u8; length as usize])
            .and_then(|()| region.flush())
            .map_err(unwritten)?;
    }

    let mut copied = vec![];
    for partition in image.partitions() {
        let mut region = Counted::writes(
            Partition::new(disk, partition.offset(), partition.bytes()).map_err(unwritten)?,
            meter,
        );
        let hash = match signed.iter().find(|(slot, _)| *slot == partition.slot) {
            Some((_, esp)) => copy_hashed(&mut &esp[..], &mut region, partition.bytes()),
            None => {
                image
                    .file
                    .seek(SeekFrom::Start(partition.offset()))
                    .map_err(|why| Fault::unreadable("the stick's image", why))?;
                copy_hashed(&mut image.file, &mut region, partition.bytes())
            }
        }
        .map_err(unwritten)?;
        region.flush().map_err(unwritten)?;
        copied.push((partition, hash));
    }

    {
        // Formatted, so what the stick held there is never read; written
        // back once, after the vault is closed, rather than at every file
        // fatfs closes.
        let mut region =
            Partition::fresh(disk, vault.offset(), vault.bytes()).map_err(unwritten)?;
        let mut fat = Fat::format(Counted::writes(Unflushed(&mut region), meter))?;
        fill(&mut fat)?;
        fat.close()?;
        region.flush().map_err(unwritten)?;
    }

    for (at, bytes) in table.render(sectors) {
        let length = (bytes.len() as u64).div_ceil(SECTOR) * SECTOR;
        let mut region = Partition::new(disk, at, length).map_err(unwritten)?;
        region
            .write_all(&bytes)
            .and_then(|()| region.flush())
            .map_err(unwritten)?;
    }
    disk.sync_all().map_err(unwritten)?;
    Ok(Written {
        vault,
        table,
        sectors,
        copied,
    })
}

/// Where firmware looks for a removable disk's boot program.
const BOOT_PROGRAM: &str = "EFI/BOOT/BOOTX64.EFI";

/// Signs the boot program in an EFI system partition held in memory, the
/// one file there firmware boots on its own, so nothing else that lands on
/// a stick's partition is ever signed with a machine's key. One this
/// signer signed as it is now is left alone, and says nothing changed.
/// fatfs rewrites a file in place without the directory entries it gets
/// wrong when it makes one.
fn sign_esp(esp: &mut [u8], signer: &Signer) -> Result<bool, Fault> {
    let unsigned = |why: io::Error| Fault::unreadable("the stick's EFI system partition", why);
    let filesystem =
        fatfs::FileSystem::new(io::Cursor::new(esp), crate::vault::store::fat_options())
            .map_err(unsigned)?;
    {
        let mut file = filesystem
            .root_dir()
            .open_file(BOOT_PROGRAM)
            .map_err(|why| match why.kind() {
                io::ErrorKind::NotFound => {
                    Fault::unreadable("the stick's EFI system partition", "it has no boot program")
                }
                _ => unsigned(why),
            })?;
        let mut program = vec![];
        file.read_to_end(&mut program).map_err(unsigned)?;
        if signer.has_signed(&program) {
            return Ok(false);
        }
        let program = signer.sign(program, std::time::SystemTime::now())?;
        file.seek(SeekFrom::Start(0))
            .and_then(|_| file.write_all(&program))
            .and_then(|()| file.truncate())
            .and_then(|()| file.flush())
            .map_err(unsigned)?;
    }
    filesystem.stats().map_err(unsigned)?;
    filesystem.unmount().map_err(unsigned)?;
    Ok(true)
}

/// Signs the boot program in the EFI system partition `esp`, a stick's
/// own on a disk, in place of whatever signed it before: read whole,
/// signed in memory, then only the blocks that changed written back, so a
/// stick that stops part way loses as little as can be, and read back.
/// One this signer signed already, as a reinstall from a machine's own
/// stick finds it, is never written to at all.
pub fn sign_in_place(esp: &mut File, signer: &Signer) -> Result<(), Fault> {
    let unwritten = |why: io::Error| Fault::Unkept {
        path: "the stick's EFI system partition".into(),
        why: why.to_string(),
    };
    let bytes = esp.seek(SeekFrom::End(0)).map_err(unwritten)?;
    let mut was = vec![0u8; bytes as usize];
    Partition::new(esp, 0, bytes)
        .and_then(|mut region| region.read_exact(&mut was))
        .map_err(unwritten)?;
    let mut signed = was.clone();
    if !sign_esp(&mut signed, signer)? {
        return Ok(());
    }
    let block = blocks::BLOCK as usize;
    for (at, (new, old)) in signed.chunks(block).zip(was.chunks(block)).enumerate() {
        if new != old {
            Partition::new(esp, (at * block) as u64, new.len() as u64)
                .and_then(|mut region| region.write_all(new).and_then(|()| region.flush()))
                .map_err(unwritten)?;
        }
    }
    esp.sync_all().map_err(unwritten)?;
    drop_cache(esp)?;
    let mut back = vec![0u8; bytes as usize];
    Partition::new(esp, 0, bytes)
        .and_then(|mut region| region.read_exact(&mut back))
        .map_err(unwritten)?;
    if back != signed {
        return Err(Fault::Unproven {
            step: "sign-the-stick",
            standing: "the stick's EFI system partition isn't what was written".into(),
        });
    }
    Ok(())
}

/// Forgets what the system cached of the disk, so what's read next comes
/// from the disk itself. macOS's raw disks cache nothing, and its image
/// files are read past the cache once it's told to.
fn drop_cache(disk: &File) -> Result<(), Fault> {
    use std::os::fd::AsRawFd;
    #[cfg(target_os = "linux")]
    // SAFETY: the descriptor is the open file's own; advice changes no memory.
    let refused =
        unsafe { libc::posix_fadvise(disk.as_raw_fd(), 0, 0, libc::POSIX_FADV_DONTNEED) } != 0;
    #[cfg(target_os = "macos")]
    // SAFETY: the descriptor is the open file's own; F_NOCACHE takes an int.
    let refused = unsafe { libc::fcntl(disk.as_raw_fd(), libc::F_NOCACHE, 1) } == -1;
    if refused {
        Err(Fault::unreadable(
            "the stick",
            "the system wouldn't let go of its copy",
        ))
    } else {
        Ok(())
    }
}

/// Reads the stick back: both tables as they were written, every one of
/// the image's partitions byte for byte, and the vault through `check`,
/// which should open every file in it. Every byte read is counted on the
/// meter.
pub fn verify(
    written: &Written,
    disk: &mut File,
    meter: &Meter,
    check: impl FnOnce(&mut dyn Store) -> Result<(), Fault>,
) -> Result<(), Fault> {
    drop_cache(disk)?;
    let differs = |what: &str| Fault::Unproven {
        step: "write-stick",
        standing: format!("{what} on the stick isn't what was written"),
    };
    let unread = |why: io::Error| Fault::unreadable("the stick", why);
    let mut reread = vec![0u8; (written.table.first_usable() * SECTOR) as usize];
    disk.seek(SeekFrom::Start(0))
        .and_then(|_| disk.read_exact(&mut reread))
        .map_err(unread)?;
    if Table::read(&reread)? != written.table {
        return Err(differs("the partition table"));
    }
    for (at, bytes) in written.table.render(written.sectors) {
        let mut back = vec![0u8; bytes.len()];
        disk.seek(SeekFrom::Start(at))
            .and_then(|_| disk.read_exact(&mut back))
            .map_err(unread)?;
        if back != bytes {
            return Err(differs("the partition table"));
        }
    }
    for (partition, hash) in &written.copied {
        disk.seek(SeekFrom::Start(partition.offset()))
            .map_err(unread)?;
        if &copy_hashed(
            &mut Counted::reads(&mut *disk, meter),
            &mut io::sink(),
            partition.bytes(),
        )
        .map_err(unread)?
            != hash
        {
            return Err(differs(&format!("the {} partition", partition.name)));
        }
    }
    // Counted as it comes off the stick, under the cache, so a lookup in
    // the vault's directory isn't counted once for every file.
    let mut counted = Counted::reads(disk, meter);
    let mut region = Partition::new(&mut counted, written.vault.offset(), written.vault.bytes())
        .map_err(unread)?;
    let mut fat = Fat::open(&mut region)?;
    check(&mut fat)
}
