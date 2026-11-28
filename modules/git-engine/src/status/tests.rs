//! The status cache's tests of the files it reads for itself from the
//! worktree, ignore and attributes files, where anyone may leave a link or
//! a FIFO in a file's place.

use super::{Cache, Lies};
use crate::regular_file::tests::{apart, fifo};

#[test]
fn an_ignore_or_attributes_file_that_isnt_a_plain_file_reads_as_empty() {
    let folder = crate::pack_index::tests::unique("status-not-plain");
    let (waiting, device) = (folder.join("waiting"), folder.join("device"));
    std::fs::create_dir_all(&device).expect("the temp folder takes a folder");
    for name in [".gitignore", ".gitattributes"] {
        fifo(&waiting.join(name));
        // A device that reads as empty, so a reader that took it for a file
        // fails by the assert below rather than reading without end.
        std::os::unix::fs::symlink("/dev/null", device.join(name))
            .expect("the temp folder takes a link");
    }
    // The FIFOs first, so a reader that waits on one waits there.
    let heard = apart(move || {
        let mut cache = Cache::default();
        [waiting, device].map(|dir| {
            let ignores = cache
                .ignore_file(&dir.join(".gitignore"), Lies::Outside)
                .len();
            let attributes =
                cache.attributes_file(&dir.join(".gitattributes"), true, Lies::Outside);
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

#[test]
fn an_ignore_or_attributes_file_linked_in_the_worktree_reads_as_empty() {
    let folder = crate::pack_index::tests::unique("status-linked");
    let (tree, git) = (folder.join("tree"), folder.join("git"));
    std::fs::create_dir_all(&tree).expect("the temp folder takes a folder");
    std::fs::create_dir_all(&git).expect("the temp folder takes a folder");
    // Plain files of a pattern and an attribute, each linked to from the
    // worktree, as a stranger's clone may link to a file of the user's,
    // and from git's own files, as the user links to their dotfiles.
    let (patterns, attributes) = (folder.join("patterns"), folder.join("attributes"));
    std::fs::write(&patterns, b"*.log\n").expect("the temp folder takes a file");
    std::fs::write(&attributes, b"*.bin binary\n").expect("the temp folder takes a file");
    for dir in [&tree, &git] {
        std::os::unix::fs::symlink(&patterns, dir.join(".gitignore"))
            .expect("the temp folder takes a link");
        std::os::unix::fs::symlink(&attributes, dir.join(".gitattributes"))
            .expect("the temp folder takes a link");
    }
    let mut cache = Cache::default();
    let read = [(&tree, Lies::InTheTree), (&git, Lies::Outside)].map(|(dir, lies)| {
        let ignores = cache.ignore_file(&dir.join(".gitignore"), lies).len();
        let attributes = cache.attributes_file(&dir.join(".gitattributes"), true, lies);
        (ignores, attributes.is_empty())
    });
    std::fs::remove_dir_all(&folder).expect("the temp folder gives its folder back");
    assert_eq!(
        read,
        [(0, true), (1, false)],
        "an ignore or attributes file linked to from the worktree was followed, or one \
         linked to from git's own files wasn't"
    );
}
