//! The status cache's tests of the files it reads for itself from the
//! worktree, ignore and attributes files, where anyone may leave a link or
//! a FIFO in a file's place.

use super::Cache;
use crate::regular_file::tests::{apart, fifo};

#[test]
fn an_ignore_or_attributes_file_that_isnt_a_plain_file_reads_as_empty() {
    let folder = crate::pack_index::tests::unique("status-not-plain");
    let (waiting, device) = (folder.join("waiting"), folder.join("device"));
    std::fs::create_dir_all(&device).expect("the temp folder takes a folder");
    for name in [".gitignore", ".gitattributes"] {
        fifo(&waiting.join(name));
        std::os::unix::fs::symlink("/dev/zero", device.join(name))
            .expect("the temp folder takes a link");
    }
    // The FIFOs first, so a reader that waits on one waits there, and
    // never reads the device without end.
    let heard = apart(move || {
        let mut cache = Cache::default();
        [waiting, device].map(|dir| {
            let ignores = cache.ignore_file(&dir.join(".gitignore")).len();
            let attributes = cache.attributes_file(&dir.join(".gitattributes"), true);
            (ignores, attributes.is_empty())
        })
    });
    std::fs::remove_dir_all(&folder).expect("the temp folder gives its folder back");
    assert_eq!(
        heard.expect("a read of a FIFO or a device was still waiting after 30 s"),
        [(0, true), (0, true)],
        "an ignore or attributes file that's a FIFO, or a link to a device, wasn't read \
         as empty"
    );
}
