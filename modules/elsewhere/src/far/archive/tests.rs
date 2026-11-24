use super::{CLOSED, Part, archived, at_most, record, taken, write};
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

/// Unpacks each of `parts` in `from` into `into` with the system's tar,
/// as the far side does.
fn unpacked(from: &Path, parts: &[Part], into: &Path) {
    std::fs::create_dir_all(into).unwrap();
    for part in parts {
        let status = std::process::Command::new("tar")
            .arg("-xf")
            .arg(from.join(&part.name))
            .arg("-C")
            .arg(into)
            .status()
            .unwrap();
        assert!(status.success(), "tar couldn't unpack {}", part.name);
    }
}

#[test]
fn what_goes_in_comes_out_of_the_system_s_tar_as_it_was() {
    let root = crate::far::tests::scratch("archive-whole");
    let long_folder = format!("{}/n", "p".repeat(120));
    let long_name = format!("d/{}", "f".repeat(120));
    let files = [
        ("run.sh", "#!/bin/sh\n"),
        ("d/empty", ""),
        ("lib/target/x.ts", "ok"),
        ("caf\u{e9}.txt", "accents"),
        (long_folder.as_str(), "split"),
        (long_name.as_str(), "pax"),
    ];
    for (path, text) in files {
        let at = root.join("tree").join(path);
        std::fs::create_dir_all(at.parent().unwrap()).unwrap();
        std::fs::write(&at, text).unwrap();
    }
    std::fs::set_permissions(
        root.join("tree/run.sh"),
        std::fs::Permissions::from_mode(0o755),
    )
    .unwrap();
    let parts = root.join("parts");
    std::fs::create_dir_all(&parts).unwrap();
    let written = write(
        &root.join("tree"),
        files.iter().map(|(path, _)| *path),
        &parts,
        1 << 20,
        u64::MAX,
    )
    .unwrap();
    assert_eq!(
        written.len(),
        1,
        "a small tree took {} archives",
        written.len()
    );
    assert_eq!(
        std::fs::metadata(parts.join("0.tar")).unwrap().len(),
        written[0].size,
        "an archive's size wasn't the size it was written at"
    );
    let into = root.join("out");
    unpacked(&parts, &written, &into);
    for (path, text) in files {
        assert_eq!(
            std::fs::read_to_string(into.join(path)).ok().as_deref(),
            Some(text),
            "{path} didn't come out as it went in"
        );
    }
    let mode = std::fs::metadata(into.join("run.sh"))
        .unwrap()
        .permissions()
        .mode();
    assert!(
        mode & 0o100 != 0,
        "a program came out unrunnable ({mode:o})"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_tree_past_one_archive_is_split_over_several_each_within_the_most() {
    let root = crate::far::tests::scratch("archive-split");
    let names = ["a", "b", "c"];
    for name in names {
        std::fs::write(root.join(name), [b'x'; 600]).unwrap();
    }
    let parts = root.join("parts");
    std::fs::create_dir_all(&parts).unwrap();
    // Room for one 600 byte file's header, data and close, not two.
    let most = 3000;
    let written = write(&root, names, &parts, most, u64::MAX).unwrap();
    assert_eq!(
        written
            .iter()
            .map(|part| part.name.as_str())
            .collect::<Vec<_>>(),
        ["0.tar", "1.tar", "2.tar"],
        "three files that fit one an archive weren't split one an archive"
    );
    assert!(
        written.iter().all(|part| part.size <= most),
        "an archive was past the most: {written:?}"
    );
    let into = root.join("out");
    unpacked(&parts, &written, &into);
    for name in names {
        assert_eq!(
            std::fs::read(into.join(name)).unwrap().len(),
            600,
            "{name} didn't come out whole across the archives"
        );
    }
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn what_a_file_takes_is_its_entry_and_the_archives_stay_within_their_bound() {
    let root = crate::far::tests::scratch("archive-taken");
    // Past both of ustar's fields, so it needs a pax record, yet each part
    // within the 255 bytes a file system takes in one name.
    let long = format!("{}/{}", "d".repeat(200), "f".repeat(200));
    let files = [("a", 600), ("b", 0), ("c", 513), (long.as_str(), 700)];
    for (path, size) in files {
        let at = root.join("tree").join(path);
        std::fs::create_dir_all(at.parent().unwrap()).unwrap();
        std::fs::write(&at, vec![b'x'; size]).unwrap();
    }
    let tree = root.join("tree");
    let parts = root.join("parts");
    std::fs::create_dir_all(&parts).unwrap();
    let takes = |path: &str, size: usize| taken(path, u64::try_from(size).unwrap());
    // Alone, each file's archive is what it takes and a close, a pax
    // header and all.
    for (path, size) in files {
        let alone = write(&tree, [path], &parts, 1 << 20, u64::MAX).unwrap();
        assert_eq!(
            alone[0].size,
            takes(path, size) + CLOSED,
            "what {path} takes isn't its entry"
        );
    }
    // Two files fit one archive at just their room, and not a byte less.
    let both = takes("a", 600) + takes("c", 513) + CLOSED;
    let fall = |most| {
        write(&tree, ["a", "c"], &parts, most, u64::MAX)
            .unwrap()
            .len()
    };
    assert_eq!(fall(both), 1);
    assert_eq!(fall(both - 1), 2);
    // However they fall, the archives come to no more than their bound.
    let all: u64 = files.iter().map(|&(path, size)| takes(path, size)).sum();
    for most in [both - 1, both, 6 * 1024, 1 << 20] {
        let written = write(&tree, files.map(|(path, _)| path), &parts, most, u64::MAX).unwrap();
        let came: u64 = written.iter().map(|part| part.size).sum();
        assert!(
            came <= archived(all, most),
            "archives of {came} bytes passed their bound of {} at most {most}",
            archived(all, most)
        );
    }
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_file_no_archive_within_the_most_holds_is_refused() {
    let root = crate::far::tests::scratch("archive-large");
    std::fs::write(root.join("big"), [0u8; 2000]).unwrap();
    let parts = root.join("parts");
    std::fs::create_dir_all(&parts).unwrap();
    let refused = write(&root, ["big"], &parts, 2048, u64::MAX);
    assert!(
        refused
            .as_ref()
            .is_err_and(|error| error.to_string().contains("big is past the 2048 bytes")),
        "a file past the most was written: {refused:?}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_file_itself_past_the_most_is_refused_as_it_is_opened_not_once_read() {
    let root = crate::far::tests::scratch("archive-larger");
    std::fs::write(root.join("big"), [0u8; 3000]).unwrap();
    let parts = root.join("parts");
    std::fs::create_dir_all(&parts).unwrap();
    let refused = write(&root, ["big"], &parts, 2048, u64::MAX);
    // The opened file is named by its own path and the size it was opened
    // at, where one found too big once read would say it grew, and an
    // entry too big would be named by its path in the tree.
    let opened = format!(
        "{} is 3000 bytes, past the 2048 bytes",
        root.join("big").display()
    );
    assert!(
        refused
            .as_ref()
            .is_err_and(|error| error.to_string().starts_with(&opened)),
        "a file past the most was read before it was refused: {refused:?}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A reader of `left` bytes that counts those it has given.
struct Counted {
    left: u64,
    given: u64,
}

impl std::io::Read for Counted {
    fn read(&mut self, into: &mut [u8]) -> std::io::Result<usize> {
        let count = into
            .len()
            .min(usize::try_from(self.left).unwrap_or(usize::MAX));
        into[..count].fill(b'x');
        self.left -= count as u64;
        self.given += count as u64;
        Ok(count)
    }
}

#[test]
fn what_holds_more_than_the_most_is_read_no_further_than_the_byte_past_it() {
    let mut past = Counted {
        left: 1 << 20,
        given: 0,
    };
    assert_eq!(
        at_most(&mut past, 16).unwrap(),
        None,
        "what held more than the most was taken"
    );
    assert_eq!(
        past.given, 17,
        "a reader past a most of 16 was read for {} bytes",
        past.given
    );
    let mut within = Counted { left: 16, given: 0 };
    assert_eq!(
        at_most(&mut within, 16).unwrap(),
        Some(vec![b'x'; 16]),
        "what held just the most wasn't taken whole"
    );
}

// Linux's own files say they are empty as they are opened and hold bytes
// as they are read, as a file growing between the two does.
#[cfg(target_os = "linux")]
#[test]
fn a_file_that_grows_past_the_most_as_it_is_read_is_refused() {
    let parts = crate::far::tests::scratch("archive-grows");
    let root = Path::new("/proc/self");
    assert_eq!(
        std::fs::metadata(root.join("status")).unwrap().len(),
        0,
        "the process's status no longer says it is empty"
    );
    let refused = write(root, ["status"], &parts, 16, u64::MAX);
    let grew = format!(
        "{} grew as it was read, past the 16 bytes",
        root.join("status").display()
    );
    assert!(
        refused
            .as_ref()
            .is_err_and(|error| error.to_string().starts_with(&grew)),
        "a file past the most once read wasn't refused as it grew: {refused:?}"
    );
    let _ = std::fs::remove_dir_all(&parts);
}

#[test]
fn a_file_that_would_take_the_archives_past_their_total_is_refused_unwritten() {
    let root = crate::far::tests::scratch("archive-total");
    let names = ["a", "b", "c"];
    for name in names {
        std::fs::write(root.join(name), [b'x'; 600]).unwrap();
    }
    let parts = root.join("parts");
    std::fs::create_dir_all(&parts).unwrap();
    // One file an archive, and room in all for two of them.
    let one = taken("a", 600) + CLOSED;
    let written = write(&root, ["a", "b"], &parts, 3000, 2 * one).unwrap();
    assert_eq!(
        written.len(),
        2,
        "two archives within the total weren't written"
    );
    // All three files in one archive, the third's room a byte short with
    // the two before it still open, and then closed archives alone before
    // the third.
    let whole = 3 * taken("a", 600) + CLOSED;
    assert_eq!(
        write(&root, names, &parts, 1 << 20, whole).unwrap().len(),
        1,
        "three files in one archive just within the total weren't written"
    );
    // What lies written when c is refused: the two entries in the archive
    // still open, or two closed archives.
    let two = 2 * taken("a", 600);
    for (most, all, before) in [
        (1 << 20, whole - 1, two),
        (3000, 2 * one, 2 * one),
        (3000, 3 * one - 1, 2 * one),
    ] {
        let _ = std::fs::remove_dir_all(&parts);
        std::fs::create_dir_all(&parts).unwrap();
        let refused = write(&root, names, &parts, most, all);
        assert!(
            refused.as_ref().is_err_and(|error| error
                .to_string()
                .contains(&format!("c would take the archives past the {all} bytes"))),
            "a third archive past {all} bytes in all wasn't refused: {refused:?}"
        );
        assert!(
            !parts.join("2.tar").exists(),
            "the archive past the total was begun before it was refused"
        );
        let lying: u64 = std::fs::read_dir(&parts)
            .unwrap()
            .map(|part| part.unwrap().metadata().unwrap().len())
            .sum();
        assert_eq!(
            lying, before,
            "c went into the archives before it was refused, in all {all}"
        );
    }
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_link_or_a_pipe_in_a_file_s_place_is_refused_not_followed_or_waited_on() {
    let root = crate::far::tests::scratch("archive-odd");
    std::fs::write(root.join("outside"), "secret").unwrap();
    std::os::unix::fs::symlink(root.join("outside"), root.join("link")).unwrap();
    let made = std::process::Command::new("mkfifo")
        .arg(root.join("pipe"))
        .status()
        .unwrap();
    assert!(made.success(), "mkfifo couldn't make a pipe");
    let parts = root.join("parts");
    std::fs::create_dir_all(&parts).unwrap();
    for name in ["link", "pipe"] {
        let refused = write(&root, [name], &parts, 1 << 20, u64::MAX);
        assert!(
            refused.as_ref().is_err_and(|error| error
                .to_string()
                .starts_with(&format!("{} can't go in", root.join(name).display()))),
            "a {name} in a file's place was taken: {refused:?}"
        );
    }
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_file_whose_folder_is_now_a_link_is_refused_not_read_through_it() {
    let root = crate::far::tests::scratch("archive-linked");
    let outside = root.join("outside");
    std::fs::create_dir_all(outside.join("b")).unwrap();
    std::fs::write(outside.join("b").join("f"), "secret").unwrap();
    std::fs::create_dir_all(root.join("a")).unwrap();
    std::os::unix::fs::symlink(&outside, root.join("d")).unwrap();
    std::os::unix::fs::symlink(outside.join("b"), root.join("a").join("b")).unwrap();
    let parts = root.join("parts");
    std::fs::create_dir_all(&parts).unwrap();
    // At the top of the name, and below a folder of the tree's own.
    for name in ["d/b/f", "a/b/f"] {
        let refused = write(&root, [name], &parts, 1 << 20, u64::MAX);
        assert!(
            refused.as_ref().is_err_and(|error| error.to_string()
                == format!(
                    "{} can't go in: a folder on its way is a link",
                    root.join(name).display()
                )),
            "{name}, through a link, was taken: {refused:?}"
        );
    }
    assert!(
        write(&root, ["outside/b/f"], &parts, 1 << 20, u64::MAX).is_ok(),
        "a file under the tree's own folders was refused"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_pax_record_s_length_counts_itself_across_a_digit_more() {
    for length in 1..200 {
        let value = "v".repeat(length);
        let record = record("path", &value);
        let (said, _) = record.split_once(' ').unwrap();
        assert_eq!(
            said.parse::<usize>().ok(),
            Some(record.len()),
            "a record of a {length} byte path miscounted itself: {record:?}"
        );
    }
}
