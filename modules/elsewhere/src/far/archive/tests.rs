use super::{Part, record, write};
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
    let written = write(&root, names, &parts, most).unwrap();
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
fn a_file_no_archive_within_the_most_holds_is_refused() {
    let root = crate::far::tests::scratch("archive-large");
    std::fs::write(root.join("big"), [0u8; 2000]).unwrap();
    let parts = root.join("parts");
    std::fs::create_dir_all(&parts).unwrap();
    let refused = write(&root, ["big"], &parts, 2048);
    assert!(
        refused
            .as_ref()
            .is_err_and(|error| error.to_string().contains("big is past the 2048 bytes")),
        "a file past the most was written: {refused:?}"
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
