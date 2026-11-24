//! The far line making the job's tree from its archives: what it unpacks,
//! what it refuses before the job runs, and what it takes away when it is
//! stopped.

use super::{Left, far_run, handover, said, scratch, work};
use crate::far::{BACK, Heard, TREE, archive, relay};
use std::path::Path;

#[test]
fn the_far_line_unpacks_every_name_as_it_is_and_puts_nothing_outside() {
    // Names a share keeps home, one like the mark names once crossed
    // under, and a path past both of ustar's name fields.
    let long = format!("{}/{}", "d".repeat(160), "f".repeat(110));
    let (code, err, home) = far_run(
        Left::Whole,
        &[
            ("lib/target/x.ts", "ok"),
            ("web/node_modules/a.js", "!"),
            ("notes.elsewhere", "1"),
            (&long, "2"),
        ],
        &format!("cat lib/target/x.ts web/node_modules/a.js notes.elsewhere {long} > ../seen"),
    );
    assert_eq!(
        code,
        Some(0),
        "a job over a file under target didn't run: {err}"
    );
    assert_eq!(
        std::fs::read_to_string(work(&home).join("seen"))
            .ok()
            .as_deref(),
        Some("ok!12"),
        "a file didn't land at its own name"
    );
    let _ = std::fs::remove_dir_all(&home);
    // A list naming what isn't one of the near side's archives, each
    // there at its listed size where it can be, is refused before any
    // is unpacked.
    for named in ["../out", "/abs/out", "a/./b", "..", "a/..", ""] {
        let list = format!("2\t{named}\nend\n");
        let (code, err, home) = far_run(Left::Listed(&list), &[("out", "no")], "true");
        refused_as_no_archive(code, &err, &home, named);
    }
    for named in ["x", "0", "0.tar.gz", "a/0.tar", ".tar"] {
        let list = format!("2\t{named}\nend\n");
        let (code, err, home) = far_run(Left::Listed(&list), &[(named, "no")], "true");
        refused_as_no_archive(code, &err, &home, named);
    }
}

/// Asserts a far run in `home` ended `code` refusing a list naming
/// `named` as no archive, said so in `err`, and left nothing behind.
fn refused_as_no_archive(code: Option<i32>, err: &str, home: &Path, named: &str) {
    assert!(
        code == Some(77) && err.contains("isn't one of its archives"),
        "a list naming {named:?} wasn't refused ({code:?}): {err}"
    );
    // So --there ends refused, not as a far side mycelium never reached.
    let heard = relay(said(home).as_slice(), "1");
    assert_eq!(heard, Heard::Refused, "a far refusal wasn't said: {err}");
    assert_eq!(
        heard.unstarted().0,
        terminal_kit::Exit::Refused,
        "a far refusal read as another end"
    );
    assert!(
        !work(home).join("out").exists()
            && !work(home).join("1").exists()
            && !work(home).join("1.run").exists()
            && !handover(home).join(TREE).exists(),
        "a refused job left a file outside its folder, its copies or its run's file, for {named:?}"
    );
    let _ = std::fs::remove_dir_all(home);
}

#[test]
fn an_archive_holding_a_link_a_pipe_or_a_shared_file_is_refused_before_the_job_runs() {
    type Made = fn(&Path);
    let odd: [(&str, Made); 3] = [
        ("a link", |made| {
            std::os::unix::fs::symlink("/etc/passwd", made.join("l")).unwrap();
        }),
        ("a pipe", |made| {
            let piped = std::process::Command::new("mkfifo")
                .arg(made.join("l"))
                .status()
                .unwrap();
            assert!(piped.success(), "mkfifo made no pipe");
        }),
        ("a file under two names", |made| {
            std::fs::write(made.join("k"), "x").unwrap();
            std::fs::hard_link(made.join("k"), made.join("l")).unwrap();
        }),
    ];
    for (what, make) in odd {
        let made = scratch("odd");
        make(&made);
        // Plain ustar, and on a Mac with no AppleDouble members, so the
        // archive holds no attributes, only headers and at most a letter:
        // all ASCII and NULs, so its text holds it.
        let archive = std::process::Command::new("tar")
            .env("COPYFILE_DISABLE", "1")
            .args(["--format", "ustar", "-cf", "-", "-C"])
            .arg(&made)
            .arg(".")
            .output()
            .unwrap();
        assert!(archive.status.success(), "tar made no archive of {what}");
        let _ = std::fs::remove_dir_all(&made);
        let text = String::from_utf8(archive.stdout).unwrap();
        let list = format!("{}\t0.tar\nend\n", text.len());
        let (code, err, home) = far_run(Left::Listed(&list), &[("0.tar", &text)], "touch ../ran");
        assert!(
            code == Some(77) && err.contains("neither a file nor a folder"),
            "a tree holding {what} wasn't refused ({code:?}): {err}"
        );
        assert!(
            !work(&home).join("ran").exists() && !work(&home).join("1").exists(),
            "a job ran over a tree holding {what}, or its copy was left"
        );
        let _ = std::fs::remove_dir_all(&home);
    }
}

#[test]
fn a_folder_closed_to_its_owner_is_opened_so_its_files_are_checked_and_taken_away() {
    let made = scratch("closed");
    std::fs::create_dir_all(made.join("d")).unwrap();
    std::fs::write(made.join("d/f"), "inside").unwrap();
    let archive = std::process::Command::new("tar")
        .env("COPYFILE_DISABLE", "1")
        .args(["--format", "ustar", "-cf", "-", "-C"])
        .arg(&made)
        .arg("d")
        .output()
        .unwrap();
    assert!(archive.status.success(), "tar made no archive");
    let _ = std::fs::remove_dir_all(&made);
    let mut bytes = archive.stdout;
    // The folder's header comes first; its mode becomes 000 and its sum
    // is made again over the header with the sum's own field as spaces.
    assert_eq!(
        &bytes[156..157],
        b"5",
        "the archive doesn't open with the folder"
    );
    bytes[100..108].copy_from_slice(b"0000000\0");
    bytes[148..156].copy_from_slice(b"        ");
    let sum: u32 = bytes[..512].iter().map(|&byte| u32::from(byte)).sum();
    bytes[148..156].copy_from_slice(format!("{sum:06o}\0 ").as_bytes());
    let text = String::from_utf8(bytes).unwrap();
    let list = format!("{}\t0.tar\nend\n", text.len());
    let (code, err, home) = far_run(Left::Listed(&list), &[("0.tar", &text)], "cat d/f > ../ran");
    assert_eq!(
        code,
        Some(0),
        "the job didn't run over a closed folder: {err}"
    );
    assert_eq!(
        std::fs::read_to_string(work(&home).join("ran")).unwrap(),
        "inside",
        "the job didn't read the closed folder's file"
    );
    assert!(
        !work(&home).join("1").exists(),
        "a closed folder's copy was left behind"
    );
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn every_archive_is_unpacked_in_turn_into_one_tree() {
    let made = scratch("several");
    std::fs::create_dir_all(made.join("in/d")).unwrap();
    std::fs::write(made.join("in/a"), "1").unwrap();
    std::fs::write(made.join("in/d/b"), "2").unwrap();
    std::fs::write(made.join("in/d/c"), "3").unwrap();
    // Room for one file's entry and the close, so each goes in its own.
    let most = archive::taken("d/b", 1) + archive::CLOSED;
    let parts =
        archive::write(&made.join("in"), ["a", "d/b", "d/c"], &made, most, u64::MAX).unwrap();
    assert_eq!(parts.len(), 3, "the files didn't go in an archive each");
    let texts: Vec<_> = parts
        .iter()
        .map(|part| {
            let text = String::from_utf8(std::fs::read(made.join(&part.name)).unwrap()).unwrap();
            (part.name.clone(), text)
        })
        .collect();
    let _ = std::fs::remove_dir_all(&made);
    let lines: Vec<_> = texts
        .iter()
        .map(|(name, text)| format!("{}\t{name}", text.len()))
        .chain(["end".to_owned()])
        .collect();
    let list = lines.join("\n") + "\n";
    let files: Vec<_> = texts
        .iter()
        .map(|(name, text)| (name.as_str(), text.as_str()))
        .collect();
    let (code, err, home) = far_run(Left::Listed(&list), &files, "cat a d/b d/c > ../ran");
    assert_eq!(code, Some(0), "a job over three archives didn't run: {err}");
    assert_eq!(
        std::fs::read_to_string(work(&home).join("ran")).unwrap(),
        "123",
        "a file of a later archive didn't land beside the first's"
    );
    assert!(
        !work(&home).join("1").exists() && !handover(&home).join(TREE).exists(),
        "the copy or the archives were left behind"
    );
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn a_link_one_archive_makes_is_refused_before_the_next_writes_through_it() {
    let outside = scratch("through-outside");
    let made = scratch("through");
    std::fs::create_dir_all(made.join("first")).unwrap();
    std::os::unix::fs::symlink(&outside, made.join("first/l")).unwrap();
    let first = std::process::Command::new("tar")
        .env("COPYFILE_DISABLE", "1")
        .args(["--format", "ustar", "-cf", "-", "-C"])
        .arg(made.join("first"))
        .arg("l")
        .output()
        .unwrap();
    assert!(first.status.success(), "tar made no archive of a link");
    let first = String::from_utf8(first.stdout).unwrap();
    std::fs::create_dir_all(made.join("second/l")).unwrap();
    std::fs::write(made.join("second/l/f"), "through").unwrap();
    archive::write(&made.join("second"), ["l/f"], &made, 1 << 20, u64::MAX).unwrap();
    let second = String::from_utf8(std::fs::read(made.join("0.tar")).unwrap()).unwrap();
    let _ = std::fs::remove_dir_all(&made);
    let list = format!("{}\t0.tar\n{}\t1.tar\nend\n", first.len(), second.len());
    let (code, err, home) = far_run(
        Left::Listed(&list),
        &[("0.tar", &first), ("1.tar", &second)],
        "touch ../ran",
    );
    assert!(
        code == Some(77) && err.contains("neither a file nor a folder"),
        "a link the first archive made wasn't refused as the tree's fault ({code:?}): {err}"
    );
    assert!(
        !outside.join("f").exists(),
        "the second archive wrote through the first one's link"
    );
    assert!(
        !work(&home).join("ran").exists() && !work(&home).join("1").exists(),
        "a job ran over a tree holding a link, or its copy was left"
    );
    let _ = std::fs::remove_dir_all(&home);
    let _ = std::fs::remove_dir_all(&outside);
}

#[test]
fn an_archive_naming_a_member_outside_its_tree_is_refused_before_it_s_unpacked() {
    let made = scratch("climbing");
    std::fs::create_dir_all(made.join("in")).unwrap();
    std::fs::write(made.join("out"), "no").unwrap();
    let parts = archive::write(&made.join("in"), ["../out"], &made, 1 << 20, u64::MAX).unwrap();
    assert_eq!(parts.len(), 1, "one file made other than one archive");
    let text = String::from_utf8(std::fs::read(made.join("0.tar")).unwrap()).unwrap();
    let _ = std::fs::remove_dir_all(&made);
    let list = format!("{}\t0.tar\nend\n", text.len());
    let (code, err, home) = far_run(Left::Listed(&list), &[("0.tar", &text)], "touch ../ran");
    assert!(
        code == Some(77) && err.contains("names ../out, outside its tree"),
        "an archive climbing out of its tree wasn't refused as the tree's fault ({code:?}): {err}"
    );
    assert!(
        !work(&home).join("out").exists()
            && !work(&home).join("ran").exists()
            && !work(&home).join("1").exists()
            && !work(&home).join("1.names").exists(),
        "an archive climbing out wrote outside its copy, a job ran, or its copy was left"
    );
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn a_far_line_stopped_mid_job_takes_its_copy_away() {
    // Mycelium stops a run whose asker let go with the terminate signal
    // to its whole group; here the job sends a signal to the far line
    // alone, then ends, as a job the signal reached would. The far line
    // dies by the signal itself once it has cleared, so what waits on it
    // sees it stopped.
    for (signal, number) in [("TERM", 15), ("HUP", 1), ("INT", 2), ("PIPE", 13)] {
        let (code, err, home) = far_run(
            Left::Whole,
            &[("a", "1")],
            &format!("touch ../ran && kill -{signal} $PPID"),
        );
        assert!(
            work(&home).join("ran").exists(),
            "the job never ran ({code:?}): {err}"
        );
        let died_by = std::fs::read_to_string(home.join("signal")).unwrap();
        assert_eq!(
            (code, died_by.as_str()),
            (None, number.to_string().as_str()),
            "a far line stopped by {signal} ended otherwise: {err}"
        );
        assert!(
            !work(&home).join("1").exists()
                && !work(&home).join("1.run").exists()
                && !handover(&home).join(TREE).exists(),
            "a far line stopped by {signal} left its copy, its run's file or the job's archives"
        );
        assert!(
            !handover(&home).join(BACK).exists(),
            "a far line stopped by {signal} brought something back"
        );
        let _ = std::fs::remove_dir_all(&home);
    }
}
