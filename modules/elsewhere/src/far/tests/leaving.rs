//! The near side leaving a job's files in the handover: its archives, the
//! list of them, and the bytes it says they take.

use super::scratch;
use crate::far::{LEFT, TREE, archive, leave, left};

#[test]
fn what_is_left_is_its_archives_each_listed_at_its_size() {
    let root = scratch("left");
    std::fs::write(root.join("x"), "xyz").unwrap();
    let job = root.join("job");
    let bytes = left(&root, ["x"], &job).unwrap();
    let size = std::fs::metadata(job.join(TREE).join("0.tar"))
        .unwrap()
        .len();
    // A header, the data's block and the two that close it.
    assert_eq!(size, 4 * 512, "one small file's archive came to {size}");
    assert_eq!(bytes, size, "what was left was said to come to {bytes}");
    assert_eq!(
        std::fs::read_to_string(job.join(LEFT)).unwrap(),
        format!("{size}\t0.tar\nend\n"),
        "an archive wasn't listed at the size it was written at"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn what_is_left_over_several_archives_is_said_to_take_them_all() {
    let root = scratch("left-several");
    let sizes = [("a", 600), ("b", 1), ("c", 1500)];
    for (name, size) in sizes {
        std::fs::write(root.join(name), vec![b'x'; size]).unwrap();
    }
    let job = root.join("job");
    // Room for the largest file's entry and the close, so each goes in its
    // own archive, each of a size of its own.
    let most = archive::taken("c", 1500) + archive::CLOSED;
    let bytes = leave(&root, sizes.map(|(name, _)| name), &job, most, u64::MAX).unwrap();
    let parts: Vec<u64> = ["0.tar", "1.tar", "2.tar"]
        .iter()
        .map(|name| std::fs::metadata(job.join(TREE).join(name)).unwrap().len())
        .collect();
    assert_eq!(
        bytes,
        parts.iter().sum::<u64>(),
        "archives of {parts:?} bytes were said to take {bytes}"
    );
    assert_eq!(
        std::fs::read_to_string(job.join(LEFT)).unwrap(),
        format!(
            "{}\t0.tar\n{}\t1.tar\n{}\t2.tar\nend\n",
            parts[0], parts[1], parts[2]
        ),
        "the archives weren't each listed at the size it was written at"
    );
    let _ = std::fs::remove_dir_all(&root);
}
