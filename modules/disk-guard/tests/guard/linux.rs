use disk_guard::band::BtrfsSpace;
use disk_guard::linux::btrfs_allocation;
use std::fs;
use std::path::Path;

const GIB: u64 = 1 << 30;
const MIB: u64 = 1 << 20;

fn write(root: &Path, path: &str, value: u64) {
    let path = root.join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, format!("{value}\n")).unwrap();
}

/// A filesystem's sysfs directory as Btrfs lays it out, on one 1000 GiB
/// device: 400 GiB of data chunks, 8 GiB of metadata kept twice (dup), and
/// a system chunk.
fn sysfs(root: &Path) {
    write(root, "allocation/global_rsv_size", 512 * MIB);
    write(root, "allocation/data/total_bytes", 400 * GIB);
    write(root, "allocation/data/disk_total", 400 * GIB);
    write(root, "allocation/data/chunk_size", 10 * GIB);
    write(root, "allocation/metadata/total_bytes", 8 * GIB);
    write(root, "allocation/metadata/disk_total", 16 * GIB);
    write(root, "allocation/metadata/chunk_size", GIB);
    write(root, "allocation/metadata/bytes_used", 7 * GIB);
    write(root, "allocation/metadata/bytes_reserved", 100 * MIB);
    write(root, "allocation/metadata/bytes_pinned", 20 * MIB);
    write(root, "allocation/metadata/bytes_readonly", 4 * MIB);
    write(root, "allocation/metadata/bytes_may_use", 3 * GIB);
    write(root, "allocation/system/disk_total", 64 * MIB);
    write(root, "devices/dm-0/size", 1000 * GIB / 512);
}

#[test]
fn allocation_is_read_from_the_filesystems_sysfs() {
    let root = tempfile::tempdir().unwrap();
    sysfs(root.path());
    write(root.path(), "allocation/metadata/bytes_zone_unusable", 0);
    let space = btrfs_allocation(root.path()).unwrap();
    assert_eq!(
        space,
        BtrfsSpace {
            unallocated: 1000 * GIB - 400 * GIB - 16 * GIB - 64 * MIB,
            metadata_chunk: 2 * GIB,
            // Reservations that may yet be given back (bytes_may_use) are
            // Btrfs overcommitting, and count as free.
            metadata_free: GIB - 124 * MIB,
            global_reserve: 512 * MIB,
        }
    );
}

#[test]
fn an_older_kernel_without_zone_counts_reads_the_same() {
    let root = tempfile::tempdir().unwrap();
    sysfs(root.path());
    assert_eq!(
        btrfs_allocation(root.path()).unwrap().metadata_free,
        GIB - 124 * MIB
    );
}

#[test]
fn a_filesystem_over_two_devices_counts_both() {
    let root = tempfile::tempdir().unwrap();
    sysfs(root.path());
    write(root.path(), "devices/sdb/size", 500 * GIB / 512);
    let unallocated = 1500 * GIB - 400 * GIB - 16 * GIB - 64 * MIB;
    assert_eq!(
        btrfs_allocation(root.path()).unwrap().unallocated,
        unallocated
    );
}

#[test]
fn a_missing_count_is_an_error_rather_than_a_guess() {
    let root = tempfile::tempdir().unwrap();
    sysfs(root.path());
    fs::remove_file(root.path().join("allocation/metadata/bytes_used")).unwrap();
    assert!(btrfs_allocation(root.path()).is_err());
}
