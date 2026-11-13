//! Every file under a folder, as a test compares it before and after:
//! shared by the tests that say a folder was left as it was.

use std::collections::BTreeMap;
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

/// Every file under a folder, with its inode and contents.
pub fn files(dir: &Path) -> BTreeMap<PathBuf, (u64, Vec<u8>)> {
    let mut found = BTreeMap::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(next) = stack.pop() {
        for entry in fs::read_dir(&next).expect("a folder").flatten() {
            let path = entry.path();
            let kind = entry.file_type().expect("a kind");
            if kind.is_dir() {
                stack.push(path);
            } else if kind.is_file() {
                let inode = fs::metadata(&path).expect("metadata").ino();
                found.insert(path.clone(), (inode, fs::read(&path).expect("contents")));
            }
        }
    }
    found
}
