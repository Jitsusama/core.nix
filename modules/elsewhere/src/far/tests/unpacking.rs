//! The far line making the job's tree from its archives: what it unpacks,
//! what it refuses before the job runs, and what it takes away when it is
//! stopped.

use super::crossing::refusing;
use super::{Left, far_line, far_run, handover, kind, said, scratch, work};
use crate::far::{BACK, END, Heard, LEFT, TREE, archive, relay};
use std::path::{Path, PathBuf};

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
fn an_archive_that_isn_t_plain_ustar_is_refused_before_it_is_read() {
    let made = scratch("not-plain");
    std::fs::write(made.join("a"), "inside").unwrap();
    let archive = std::process::Command::new("tar")
        .env("COPYFILE_DISABLE", "1")
        .args(["--format", "ustar", "-cf", "-", "-C"])
        .arg(&made)
        .arg("a")
        .output()
        .unwrap();
    assert!(archive.status.success(), "tar made no archive");
    let _ = std::fs::remove_dir_all(&made);
    assert_eq!(
        &archive.stdout[257..265],
        b"ustar\x0000",
        "tar wrote no ustar header"
    );
    // GNU's magic in place of ustar's, which every tar still unpacks, as
    // it does a compressed archive, and ustar's with its NUL moved, which
    // only a byte-for-byte reading refuses; the sum is made again over
    // the header with the sum's own field as spaces.
    for magic in [b"ustar  \0", b"ustar00\0"] {
        let mut bytes = archive.stdout.clone();
        bytes[257..265].copy_from_slice(magic);
        bytes[148..156].copy_from_slice(b"        ");
        let sum: u32 = bytes[..512].iter().map(|&byte| u32::from(byte)).sum();
        bytes[148..156].copy_from_slice(format!("{sum:06o}\0 ").as_bytes());
        let text = String::from_utf8(bytes).unwrap();
        let list = format!("{}\t0.tar\nend\n", text.len());
        let (code, err, home) = reading(&list, &[("0.tar", &text)], &[]);
        refused_as_not_plain(
            code,
            &err,
            &home,
            "0.tar",
            &format!("the magic {magic:?}"),
            0,
        );
    }
    // A byte short of a whole header, its magic ustar's, which tar
    // can't list, and so the job's fault, not this machine's.
    let short = String::from_utf8(archive.stdout[..511].to_vec()).unwrap();
    let list = format!("{}\t0.tar\nend\n", short.len());
    let (code, err, home) = reading(&list, &[("0.tar", &short)], &[]);
    refused_as_not_plain(code, &err, &home, "0.tar", "511 bytes", 0);
    // One archive listed twice, which no near side writes.
    let plain = String::from_utf8(archive.stdout.clone()).unwrap();
    let list = format!("{0}\t0.tar\n{0}\t0.tar\nend\n", plain.len());
    let (code, err, home) = reading(&list, &[("0.tar", &plain)], &[]);
    assert!(
        code == Some(77) && err.contains("names 0.tar twice"),
        "an archive listed twice wasn't refused ({code:?}): {err}"
    );
    assert!(
        !home.join("read").exists() && !work(&home).join("ran").exists(),
        "an archive listed twice was read, or its job ran"
    );
    let _ = std::fs::remove_dir_all(&home);
    // A plain archive, then one with GNU's magic, so each is checked,
    // not the first alone.
    let mut gnu = archive.stdout.clone();
    gnu[257..265].copy_from_slice(b"ustar  \0");
    let gnu = String::from_utf8(gnu).unwrap();
    let list = format!("{}\t0.tar\n{}\t1.tar\nend\n", plain.len(), gnu.len());
    let files = [("0.tar", plain.as_str()), ("1.tar", gnu.as_str())];
    // The first archive listed and unpacked, and the second never read.
    let (code, err, home) = reading(&list, &files, &[]);
    refused_as_not_plain(code, &err, &home, "1.tar", "a second archive", 2);
}

/// Runs the far line over `files` as `list` lists them, its job touching
/// `ran` beside its copy, with `programs` and a tar that writes down each
/// time it is run in the home's `read`, ahead on its path.
fn reading(
    list: &str,
    files: &[(&str, &str)],
    programs: &[(&str, &str)],
) -> (Option<i32>, String, PathBuf) {
    let tar = "printf '%s\\n' \"$*\" >> \"$HOME/read\"\nPATH=${PATH#*:} exec tar \"$@\"";
    let programs: Vec<_> = programs.iter().copied().chain([("tar", tar)]).collect();
    far_line(
        Left::Listed(list),
        files,
        "touch ../ran",
        &kind(""),
        None,
        &programs,
    )
}

/// Asserts a far run in `home` ended `code` refusing `part` as an archive
/// that isn't plain, `what` it was, said so in `err`, its tar having been
/// run `reads` times, and left no job run and nothing of its own behind.
fn refused_as_not_plain(
    code: Option<i32>,
    err: &str,
    home: &Path,
    part: &str,
    what: &str,
    reads: usize,
) {
    assert!(
        code == Some(77) && err.contains(&format!("{part} isn't a plain one")),
        "an archive of {what} wasn't refused ({code:?}): {err}"
    );
    let read = std::fs::read_to_string(home.join("read")).unwrap_or_default();
    assert_eq!(
        read.lines().count(),
        reads,
        "tar read an archive of {what} that was refused: {read}"
    );
    assert!(
        !work(home).join("ran").exists()
            && !work(home).join("1").exists()
            && !work(home).join("1.tar").exists()
            && !work(home).join("1.names").exists()
            && !handover(home).join(TREE).exists(),
        "a job ran over an archive of {what}, or its copy, archive or names were left"
    );
    let _ = std::fs::remove_dir_all(home);
}

/// An archive the near side writes of one file, `name` holding `text`.
fn archive_of(name: &str, text: &str) -> String {
    let made = scratch(&format!("archive-of-{name}"));
    std::fs::write(made.join(name), text).unwrap();
    archive::write(&made, [name], &made, 1 << 20, u64::MAX).unwrap();
    let archive = std::fs::read_to_string(made.join("0.tar")).unwrap();
    let _ = std::fs::remove_dir_all(&made);
    archive
}

/// Where the job's first archive lies in a far run's home, as its shell
/// names it from `$HOME`.
const FIRST: &str = ".cache/pi/agentic-harness.pi/quest-workspace/QEST-20261115-ABC123/\
                     handover/elsewhere/1/tree/0.tar";

#[test]
fn an_archive_s_name_holding_a_link_a_folder_or_more_is_refused_before_it_is_read() {
    // What no share writes, a link to a plain copy of the archive and a
    // folder holding it, and one past the size its list gives, as a share
    // writing it again would leave.
    for (what, swap) in [
        ("a link", r#"ln -s "$HOME/plain.tar" "$HOME/$at""#),
        (
            "a folder",
            r#"mkdir "$HOME/$at" && cp "$HOME/plain.tar" "$HOME/$at/""#,
        ),
        (
            "twice its size",
            r#"cat "$HOME/plain.tar" "$HOME/plain.tar" > "$HOME/$at""#,
        ),
    ] {
        // A mkdir that, making the copy's folder once the archive has
        // arrived, puts that at the archive's name.
        let swapping = format!(
            r#"case " $* " in *"/.s/1 "*)
  PATH=${{PATH#*:}} at='{FIRST}'
  cp "$HOME/$at" "$HOME/plain.tar" && rm -f "$HOME/$at" && {swap} &&
    : > "$HOME/swapped" ;;
esac
PATH=${{PATH#*:}} exec mkdir "$@""#
        );
        let archive = archive_of("a", "inside");
        let list = format!("{}\t0.tar\nend\n", archive.len());
        let (code, err, home) = reading(&list, &[("0.tar", &archive)], &[("mkdir", &swapping)]);
        assert!(
            home.join("swapped").exists(),
            "the stand-in never put {what} at the archive's name ({code:?}): {err}"
        );
        refused_as_not_plain(code, &err, &home, "0.tar", what, 0);
    }
}

#[test]
fn a_link_put_at_the_job_s_tree_or_above_it_is_refused_before_an_archive_is_taken_through_it() {
    // A mkdir that, making the copy's folder once the archive has
    // arrived, moves the job's tree, or its handover folder above it,
    // aside and puts a link to it at its name, so a rename through the
    // link would take the archive from wherever it points.
    for (moved, taken, at_link) in [
        ("${at%/*}", "0.tar", Some(TREE)),
        ("${at%/*/*}", "tree/0.tar", None),
    ] {
        let swapping = format!(
            r#"case " $* " in *"/.s/1 "*)
  PATH=${{PATH#*:}} at='{FIRST}'
  moved="$HOME/{moved}"
  mv "$moved" "$HOME/aside" && ln -s "$HOME/aside" "$moved" && : > "$HOME/swapped" ;;
esac
PATH=${{PATH#*:}} exec mkdir "$@""#
        );
        let archive = archive_of("a", "inside");
        let list = format!("{}\t0.tar\nend\n", archive.len());
        let (code, err, home) = reading(&list, &[("0.tar", &archive)], &[("mkdir", &swapping)]);
        assert!(
            home.join("swapped").exists(),
            "the stand-in never put a link at {moved} ({code:?}): {err}"
        );
        assert!(
            code == Some(77) && err.contains("tree in its handover is reached through a link"),
            "a link at {moved} wasn't refused ({code:?}): {err}"
        );
        assert!(
            home.join("aside").join(taken).exists()
                && !home.join("read").exists()
                && !work(&home).join("ran").exists(),
            "an archive was taken through the link at {moved}, read, or its job ran"
        );
        // The refusal's clearing took the link as itself, never what it
        // pointed at, and took the copy too.
        let link = at_link.map_or_else(|| handover(&home), |tree| handover(&home).join(tree));
        assert!(
            std::fs::symlink_metadata(&link).is_err() && !work(&home).join("1").exists(),
            "the link at {moved} or the copy was left"
        );
        let _ = std::fs::remove_dir_all(&home);
    }
}

#[test]
fn anything_but_a_folder_of_its_own_at_the_job_s_tree_is_refused_as_the_job_s_doing() {
    // A mkdir that, making the copy's folder, puts at the tree's name, or
    // at the job's handover folder above it, a link pointing nowhere, a
    // file, or a folder this account can't enter, each the job's doing
    // rather than a tree gone, which another try would meet again.
    let tree = r#"t="$HOME/${at%/*}""#;
    let job = r#"t="$HOME/${at%/*/*}""#;
    for (put, swap, kept) in [
        ("a link pointing nowhere at the tree", tree, Some("0.tar")),
        ("a file at the tree", tree, Some("0.tar")),
        ("a folder closed at the tree", tree, None),
        (
            "a link pointing nowhere above the tree",
            job,
            Some("tree/0.tar"),
        ),
    ] {
        let swapped = match (put.starts_with("a link"), kept) {
            (true, _) => r#"mv "$t" "$HOME/aside" && ln -s "$HOME/nowhere" "$t""#,
            (false, Some(_)) => r#"mv "$t" "$HOME/aside" && : > "$t""#,
            (false, None) => r#"chmod 0 "$t""#,
        };
        let swapping = format!(
            r#"case " $* " in *"/.s/1 "*)
  PATH=${{PATH#*:}} at='{FIRST}'
  {swap} && {swapped} && : > "$HOME/swapped" ;;
esac
PATH=${{PATH#*:}} exec mkdir "$@""#
        );
        let archive = archive_of("a", "inside");
        let list = format!("{}\t0.tar\nend\n", archive.len());
        let (code, err, home) = reading(&list, &[("0.tar", &archive)], &[("mkdir", &swapping)]);
        assert!(
            home.join("swapped").exists(),
            "the stand-in never put {put} ({code:?}): {err}"
        );
        assert!(
            code == Some(77) && err.contains("or isn't a folder of its own"),
            "{put} wasn't refused as the job's ({code:?}): {err}"
        );
        assert!(
            kept.is_none_or(|kept| home.join("aside").join(kept).exists())
                && !home.join("read").exists()
                && !work(&home).join("ran").exists()
                && !work(&home).join("1").exists()
                && !work(&home).join("1.run").exists(),
            "with {put}, an archive was taken, read, its job ran, or its copy stayed"
        );
        let _ = std::fs::remove_dir_all(&home);
    }
}

#[test]
fn a_link_put_at_the_job_s_tree_after_an_archive_was_taken_redirects_no_later_one() {
    // A tar that, unpacking the first archive, moves the job's tree
    // aside and puts a link at its name to a folder holding a copy of
    // the second, so a rename by the tree's name would take that copy.
    let swapping = format!(
        r#"case " $* " in *" -xf "*)
  [ -e "$HOME/swapped" ] || {{
    at='{FIRST}' && tree="$HOME/${{at%/*}}" && mv "$tree" "$HOME/moved" &&
      mkdir "$HOME/decoy" && cp "$HOME/moved/1.tar" "$HOME/decoy/1.tar" &&
      ln -s "$HOME/decoy" "$tree" && : > "$HOME/swapped"
  }} ;;
esac
PATH=${{PATH#*:}} exec tar "$@""#
    );
    let (first, second) = (archive_of("a", "one"), archive_of("b", "two"));
    let list = format!("{}\t0.tar\n{}\t1.tar\nend\n", first.len(), second.len());
    let (code, err, home) = far_line(
        Left::Listed(&list),
        &[("0.tar", &first), ("1.tar", &second)],
        "cat a b > ../ran",
        &kind(""),
        None,
        &[("tar", &swapping)],
    );
    assert!(
        home.join("swapped").exists(),
        "the stand-in never put a link at the job's tree ({code:?}): {err}"
    );
    assert_eq!(code, Some(0), "the job didn't run: {err}");
    assert_eq!(
        std::fs::read_to_string(work(&home).join("ran")).unwrap(),
        "onetwo",
        "the job ran over other than both archives"
    );
    assert!(
        home.join("decoy").join("1.tar").exists() && !home.join("moved").join("1.tar").exists(),
        "the second archive was taken through the link, not from the tree held"
    );
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn a_job_s_tree_gone_before_its_archive_is_taken_is_left_for_another_try() {
    // A mkdir that, making the copy's folder, takes the job's tree away,
    // as nothing of the job's own does: that's this machine's fault.
    let removing = format!(
        r#"case " $* " in *"/.s/1 "*)
  at='{FIRST}' && rm -rf "$HOME/${{at%/*}}" && : > "$HOME/swapped" ;;
esac
PATH=${{PATH#*:}} exec mkdir "$@""#
    );
    let archive = archive_of("a", "inside");
    let list = format!("{}\t0.tar\nend\n", archive.len());
    let (code, err, home) = reading(&list, &[("0.tar", &archive)], &[("mkdir", &removing)]);
    assert!(
        home.join("swapped").exists(),
        "the stand-in never took the tree away ({code:?}): {err}"
    );
    assert!(
        code == Some(75)
            && err.contains("tree is gone from its handover")
            && !home.join("read").exists()
            && !work(&home).join("ran").exists()
            && !work(&home).join("1").exists()
            && !work(&home).join("1.run").exists(),
        "a tree gone wasn't left for another try ({code:?}): {err}"
    );
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn a_size_rewritten_into_the_list_after_it_arrived_is_read_as_digits_never_run() {
    // A cat that, as the copy reads the list, first gives its archive a
    // size that, read as arithmetic, runs a command or marks it plain.
    for size in [r#"a[$(touch "$HOME/ran-it")]"#, "plain=1"] {
        let rewriting = format!(
            r#"case $1 in */{LEFT})
  [ -e "$HOME/rewrote" ] || {{
    printf '%s\t0.tar\n{END}\n' '{size}' > "$1" && : > "$HOME/rewrote"; }} ;;
esac
PATH=${{PATH#*:}} exec cat "$@""#
        );
        let archive = archive_of("a", "inside");
        let list = format!("{}\t0.tar\n{END}\n", archive.len());
        let (code, err, home) = reading(&list, &[("0.tar", &archive)], &[("cat", &rewriting)]);
        assert!(
            home.join("rewrote").exists(),
            "the stand-in never rewrote the list for {size} ({code:?}): {err}"
        );
        assert!(
            code == Some(77) && err.contains("gives 0.tar a size that isn't a number"),
            "a size of {size} wasn't refused ({code:?}): {err}"
        );
        assert!(
            !home.join("ran-it").exists()
                && !home.join("read").exists()
                && !work(&home).join("ran").exists(),
            "a size of {size} ran, or its archive was read, or its job ran"
        );
        let _ = std::fs::remove_dir_all(&home);
    }
}

#[test]
fn a_folder_where_the_archive_is_taken_to_is_cleared_first() {
    // A mkdir that, making the copy's folder, makes a folder holding a
    // file where the archive is taken to, as no run of ours leaves, so a
    // rename that didn't clear it first would take the archive into it.
    let planting = r#"case " $* " in *"/.s/1 "*)
  for last; do :; done
  PATH=${PATH#*:} mkdir -p "$last.tar" && : > "$last.tar/x" && : > "$HOME/planted" ;;
esac
PATH=${PATH#*:} exec mkdir "$@""#;
    let archive = archive_of("a", "inside");
    let list = format!("{}\t0.tar\nend\n", archive.len());
    let (code, err, home) = reading(&list, &[("0.tar", &archive)], &[("mkdir", planting)]);
    assert!(
        home.join("planted").exists(),
        "the stand-in never made a folder where the archive is taken ({code:?}): {err}"
    );
    assert!(
        code == Some(0) && work(&home).join("ran").exists(),
        "a folder where the archive is taken kept the job from running ({code:?}): {err}"
    );
    assert!(
        !work(&home).join("1.tar").exists(),
        "the taken archive's name was left after the copy"
    );
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn an_archive_put_at_its_name_once_it_is_taken_is_never_read() {
    let made = scratch("swapped");
    std::fs::write(made.join("a"), "2").unwrap();
    archive::write(&made, ["a"], &made, 1 << 20, u64::MAX).unwrap();
    // A tar that, listing an archive, first puts another at the first's
    // name in the handover, as a share writes one, by a new file renamed
    // over it, so an unpacking that read the name again would unpack it.
    let swapping = format!(
        r#"case " $* " in *" -tf "*)
  cp '{}' "$HOME/{FIRST}.new" && mv -f "$HOME/{FIRST}.new" "$HOME/{FIRST}" &&
    : > "$HOME/swapped" ;;
esac
PATH=${{PATH#*:}} exec tar "$@""#,
        made.join("0.tar").display()
    );
    let (code, err, home) = far_line(
        Left::Whole,
        &[("a", "1")],
        "cat a > ../ran",
        &kind(""),
        None,
        &[("tar", &swapping)],
    );
    assert!(
        home.join("swapped").exists(),
        "the stand-in never put the second archive in place ({code:?}): {err}"
    );
    assert_eq!(code, Some(0), "the job didn't run: {err}");
    assert_eq!(
        std::fs::read_to_string(work(&home).join("ran")).unwrap(),
        "1",
        "the archive put at the name after the check was the one unpacked"
    );
    let _ = std::fs::remove_dir_all(&home);
    let _ = std::fs::remove_dir_all(&made);
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
            && !work(&home).join("1.names").exists()
            && !work(&home).join("1.tar").exists(),
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

#[test]
fn a_copy_whose_job_closed_a_folder_in_it_is_taken_away_whole() {
    // Closed to its owner, with a file under it, so nothing there can be
    // taken away unopened: once the job ends, and once it is stopped.
    let rows = [
        ("true", "ended", (Some(0), "")),
        ("kill -TERM $PPID", "stopped", (None, "15")),
    ];
    for (end, how, ending) in rows {
        let (code, err, home) = far_run(
            Left::Whole,
            &[("a", "1")],
            &format!("mkdir -p d/e && : > d/e/f && chmod 0 d/e d && : > ../ran && {end}"),
        );
        let copy = work(&home).join("1");
        let left = copy.exists() || work(&home).join("1.run").exists();
        for closed in [copy.join("d"), copy.join("d/e")] {
            let _ = std::fs::set_permissions(
                &closed,
                std::os::unix::fs::PermissionsExt::from_mode(0o755),
            );
        }
        assert!(
            work(&home).join("ran").exists(),
            "the job never closed its folder ({how}, {code:?}): {err}"
        );
        let died_by = std::fs::read_to_string(home.join("signal")).unwrap_or_default();
        assert_eq!(
            (code, died_by.as_str()),
            ending,
            "the far line wasn't {how}: {err}"
        );
        assert!(
            !left,
            "a copy whose job closed a folder in it stayed once the far line {how}: {err}"
        );
        let _ = std::fs::remove_dir_all(&home);
    }
}

#[test]
fn the_archives_unpacked_are_the_very_ones_their_list_named_when_checked() {
    // The list rewritten as the first archive's names are read, its first
    // line kept to the byte, so a read carrying on in the file meets a
    // second line, naming an archive no check saw.
    let made = scratch("unpacking-reread");
    let mut texts = Vec::new();
    for name in ["a", "smuggled"] {
        let (tree, into) = (made.join(name), made.join(format!("{name}.out")));
        std::fs::create_dir_all(&tree).unwrap();
        std::fs::create_dir_all(&into).unwrap();
        std::fs::write(tree.join(name), name).unwrap();
        archive::write(&tree, [name], &into, 1 << 20, u64::MAX).unwrap();
        texts.push(String::from_utf8(std::fs::read(into.join("0.tar")).unwrap()).unwrap());
    }
    let _ = std::fs::remove_dir_all(&made);
    // Its first listing, the first archive's, rewrites the list.
    let (tar, rewriting) = refusing(
        "tar",
        "*\" -tf \"*",
        &format!(
            "[ -e \"$HOME/rewrote\" ] || {{ at='{FIRST}' && left=\"$HOME/${{at%/*/*}}/{LEFT}\" && \
             first=$(head -n 1 \"$left\") && \
             printf '%s\\n9\\t5.tar\\nend\\n' \"$first\" > \"$left\" && \
             : > \"$HOME/rewrote\"; }}"
        ),
    );
    let list = format!("{}\t0.tar\nend\n", texts[0].len());
    let (code, err, home) = far_line(
        Left::Listed(&list),
        &[("0.tar", &texts[0]), ("5.tar", &texts[1])],
        "ls > ../seen",
        &kind(""),
        None,
        &[(tar, &rewriting)],
    );
    assert!(
        home.join("rewrote").exists(),
        "the list was never rewritten ({code:?}): {err}"
    );
    let seen = std::fs::read_to_string(work(&home).join("seen")).unwrap_or_default();
    assert!(
        code == Some(0) && seen == "a\n",
        "the far line unpacked what its checked list didn't name ({code:?}): {seen:?} {err}"
    );
    let _ = std::fs::remove_dir_all(&home);
}
