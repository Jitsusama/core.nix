use disk_guard::backing::image;
use std::fs;
use std::path::{Path, PathBuf};

/// A cargo build disk's mount as one laptop's kernel listed it, with its
/// filesystem's options `options`.
fn mountinfo(options: &str) -> String {
    format!(
        "29 1 0:26 /root / rw,relatime shared:1 - btrfs /dev/mapper/root rw,ssd\n\
         667 40 7:0 / /var/cache/cargo-builds rw,noatime shared:1126 - ext4 /dev/loop0 \
         {options}\n"
    )
}

/// A `/sys/dev/block` whose loop device 7:0 names `file` as its backing.
fn block(file: &str) -> tempfile::TempDir {
    let block = tempfile::tempdir().unwrap();
    fs::create_dir_all(block.path().join("7:0/loop")).unwrap();
    fs::write(
        block.path().join("7:0/loop/backing_file"),
        format!("{file}\n"),
    )
    .unwrap();
    block
}

#[test]
fn an_image_mounted_to_discard_is_on_the_disk_its_file_is() {
    let block = block("/var/cache/cargo-builds.img");
    assert_eq!(
        image(block.path(), &mountinfo("rw,discard"), (7, 0)),
        Some(PathBuf::from("/var/cache/cargo-builds.img")),
        "an image that discards frees the disk its file is on"
    );
}

#[test]
fn an_image_that_never_discards_frees_only_itself() {
    // Its freed blocks stay in the file until a trim, which takes root.
    let block = block("/var/cache/cargo-builds.img");
    assert_eq!(
        image(block.path(), &mountinfo("rw"), (7, 0)),
        None,
        "an image that doesn't discard was counted as freeing its disk"
    );
}

#[test]
fn an_image_whose_file_was_deleted_frees_no_disk_the_guard_can_name() {
    let block = block("/var/cache/cargo-builds.img (deleted)");
    assert_eq!(
        image(block.path(), &mountinfo("rw,discard"), (7, 0)),
        None,
        "a deleted backing file was named as on a disk"
    );
}

#[test]
fn a_disk_that_is_no_loop_device_is_in_no_file() {
    let block = tempfile::tempdir().unwrap();
    fs::create_dir_all(block.path().join("259:1")).unwrap();
    assert_eq!(
        image(Path::new(block.path()), &mountinfo("rw,discard"), (259, 1)),
        None,
        "a disk with no backing file was named as an image"
    );
}
