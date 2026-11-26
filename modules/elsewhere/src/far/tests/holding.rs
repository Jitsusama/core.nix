//! The far line holding the job's tree while it takes the archives: a
//! link at the tree or anything but a folder of its own there, a link put
//! above it while the line waited, and a tree gone, each judged before an
//! archive is taken, a link put there later redirecting none, one that
//! can't be cleared keeping the tree as the line clears the copy, one a
//! killed run left above it going before anything is read, and links at
//! the copies' folder or the copy, put while it's made, leaving it
//! untaken.

use super::crossing::refusing;
use super::handing::pinning;
use super::unpacking::{FIRST, archive_of, reading};
use super::{Left, far_line, handover, kind, work};
use crate::far::{HANDOVER, TREE};

#[test]
fn a_link_put_at_the_job_s_tree_or_above_it_is_answered_before_an_archive_is_taken_through_it() {
    // A mkdir that, making the copy's folder once the archive has
    // arrived, moves the job's tree, or its handover folder above it,
    // aside and puts a link to it at its name, so a rename through the
    // link would take the archive from wherever it points. At the tree
    // it's the job's doing; above it, put since the wait cleared any
    // there, it's this machine's.
    for (moved, taken, at_link, code_said, said) in [
        (
            "${at%/*}",
            "0.tar",
            Some(TREE),
            77,
            "tree in its handover is reached through a link",
        ),
        (
            "${at%/*/*}",
            "tree/0.tar",
            None,
            75,
            "a link was put above the job's tree after its wait",
        ),
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
            code == Some(code_said) && err.contains(said),
            "a link at {moved} didn't end {code_said}, saying {said:?} ({code:?}): {err}"
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
fn a_link_a_killed_run_left_above_the_job_s_handover_goes_before_anything_is_read() {
    // A uname that, as the far line begins, moves the folder each job of
    // the quest is handed over in aside, with a link to it at its name,
    // as a run killed with no stop leaves it for the next job: cleared,
    // or left where it can't be, the job is untaken, never refused.
    let swapping = format!(
        r#"[ -e "$HOME/swapped" ] || {{
  at='{FIRST}' && moved="$HOME/${{at%/*/*/*}}"
  mv "$moved" "$HOME/aside" && ln -s "$HOME/aside" "$moved" && : > "$HOME/swapped"
}}
PATH=${{PATH#*:}} exec uname "$@""#
    );
    let (rm, pinned) = refusing("rm", &format!("*\"/{HANDOVER} \"*"), "exit 1");
    for (stays, said) in [
        (false, "a link at the job's handover was cleared"),
        (true, "a link at the job's handover couldn't be cleared"),
    ] {
        let mut programs = vec![("uname", swapping.as_str())];
        if stays {
            programs.push((rm, pinned.as_str()));
        }
        let archive = archive_of("a", "inside");
        let list = format!("{}\t0.tar\nend\n", archive.len());
        let (code, err, home) = reading(&list, &[("0.tar", &archive)], &programs);
        let linked = handover(&home).parent().map(std::fs::symlink_metadata);
        // Ended by the wait's own check, never by the copy step's.
        assert!(
            home.join("swapped").exists()
                && code == Some(75)
                && err.contains(said)
                && !err.contains("so its tree stays")
                && !err.contains("put above the job's tree"),
            "a link left above the handover wasn't answered untaken, saying {said:?} \
             ({code:?}): {err}"
        );
        assert!(
            linked.is_some_and(|at| at.is_ok_and(|at| at.is_symlink()) == stays)
                && home.join("aside/1").join(TREE).join("0.tar").exists()
                && !home.join("read").exists()
                && !work(&home).join("ran").exists(),
            "the link was {} or something was read through it: {err}",
            if stays { "cleared" } else { "kept" }
        );
        let _ = std::fs::remove_dir_all(&home);
    }
}

#[test]
fn a_link_at_the_job_s_handover_that_can_t_be_cleared_keeps_its_tree_but_not_its_copy() {
    // As above, the handover folder moved aside with a link at its name,
    // but one the far line isn't allowed to clear: it ends untaken,
    // saying the tree stays, and clears the copy alone.
    let swapping = format!(
        r#"case " $* " in *"/.s/1 "*)
  PATH=${{PATH#*:}} at='{FIRST}'
  moved="$HOME/${{at%/*/*}}"
  mv "$moved" "$HOME/aside" && ln -s "$HOME/aside" "$moved" && : > "$HOME/swapped" ;;
esac
PATH=${{PATH#*:}} exec mkdir "$@""#
    );
    let (rm, pinned) = pinning();
    let archive = archive_of("a", "inside");
    let list = format!("{}\t0.tar\nend\n", archive.len());
    let (code, err, home) = reading(
        &list,
        &[("0.tar", &archive)],
        &[("mkdir", &swapping), (rm, &pinned)],
    );
    assert!(
        home.join("swapped").exists()
            && code == Some(75)
            && err.contains("put above the job's tree")
            && err.contains("so its tree stays"),
        "a link at the handover it can't clear wasn't left untaken, saying the tree stays \
         ({code:?}): {err}"
    );
    assert!(
        home.join("aside").join(TREE).join("0.tar").exists()
            && std::fs::symlink_metadata(handover(&home)).is_ok_and(|at| at.is_symlink())
            && !work(&home).join("1").exists()
            && !work(&home).join("1.run").exists(),
        "the tree past the link was cleared, or the copy or its run's file stayed: {err}"
    );
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn anything_but_a_folder_of_its_own_at_the_job_s_tree_is_refused_as_the_job_s_doing() {
    // A mkdir that, making the copy's folder, puts at the tree's name a
    // link pointing nowhere, a file, or a folder this account can't
    // enter, each the job's doing rather than a tree gone, which another
    // try would meet again. Above the tree, the list's read meets such a
    // thing first.
    let aside = r#"mv "$t" "$HOME/aside""#;
    // Root enters a folder closed to all, so the closed one is no fault
    // there and is left out.
    let id = std::process::Command::new("id").arg("-u").output();
    let as_root = id.is_ok_and(|id| String::from_utf8_lossy(&id.stdout).trim() == "0");
    for (put, swapped, kept) in [
        (
            "a link pointing nowhere at the tree",
            format!(r#"{aside} && ln -s "$HOME/nowhere" "$t""#),
            Some("0.tar"),
        ),
        (
            "a file at the tree",
            format!(r#"{aside} && : > "$t""#),
            Some("0.tar"),
        ),
        (
            "a folder closed at the tree",
            r#"chmod 0 "$t""#.to_owned(),
            None,
        ),
    ] {
        if as_root && put.contains("closed") {
            continue;
        }
        let swapping = format!(
            r#"case " $* " in *"/.s/1 "*)
  PATH=${{PATH#*:}} at='{FIRST}' && t="$HOME/${{at%/*}}"
  {swapped} && : > "$HOME/swapped" ;;
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

/// A swap a job of the quest makes as the copy is made: the step, when,
/// whether after it, the swap, what must stay, what must never be made
/// through the link, and whether the link is pinned.
type Swap = (
    &'static str,
    &'static str,
    bool,
    &'static str,
    &'static [&'static str],
    &'static [&'static str],
    bool,
);

#[test]
fn a_link_put_at_the_copies_folder_or_the_copy_while_it_is_made_leaves_the_job_untaken() {
    // Another job of the quest moving the copies' folder aside, with a
    // link to it at its name, as the archive's names are read, where the
    // far line may clear the link and where it may not; or moving the copy
    // aside, with a link at its name, once tar unpacked into it, once its
    // folders were opened, so the odd files' find meets the link and
    // refuses it, or once the odd files' find ran, before
    // the copy's check, which says it was moved; or moving the copies'
    // folder aside then, where the link can't be cleared. Each is this
    // machine's to answer, nothing is unpacked or cleared through the
    // link, and the job never runs. Between that check and the job's cd
    // only builtins run, so no stand-in reaches the cd's own check.
    let folder = r#"mv "$s" "$HOME/aside" && ln -s "$HOME/aside" "$s""#;
    let copy = r#"mv "$s/1" "$HOME/moved" && ln -s "$HOME/moved" "$s/1""#;
    let (rm, pinned) = refusing("rm", "*\"/.s \"*", "exit 1");
    let cases: [Swap; 6] = [
        (
            "tar",
            "*\" -tf \"*",
            false,
            folder,
            &["aside/1.run", "aside/1.tar", "aside/1.names"],
            &["aside/1/a"],
            false,
        ),
        (
            "tar",
            "*\" -tf \"*",
            false,
            folder,
            &["aside/1.run", "aside/1.tar", "aside/1.names"],
            &["aside/1/a"],
            true,
        ),
        ("tar", "*\" -xf \"*", true, copy, &["moved/a"], &[], false),
        // Only the copy step's own find opens with the copy: `cleared`'s
        // opens with -P.
        (
            "find",
            "\" /\"*\"/.s/1 -type d \"*",
            true,
            copy,
            &["moved/a"],
            &[],
            false,
        ),
        (
            "find",
            "*\" -links \"*",
            true,
            copy,
            &["moved/a"],
            &[],
            false,
        ),
        (
            "find",
            "*\" -links \"*",
            true,
            folder,
            &["aside/1.run", "aside/1/a", "aside/1.names"],
            &[],
            true,
        ),
    ];
    for (step, when, after, swap, kept, gone, stays) in cases {
        let swapping = format!(
            r#"PATH=${{PATH#*:}} at='{FIRST}' s="$HOME/${{at%/{HANDOVER}/*}}/.s"
swap() {{ [ -e "$HOME/swapped" ] || {{ {swap} && : > "$HOME/swapped"; }}; }}
case " $* " in {when}) [ -n "{after}" ] || swap ;; esac
{step} "$@"; r=$?
case " $* " in {when}) [ -z "{after}" ] || swap ;; esac
exit $r"#,
            after = if after { "yes" } else { "" },
        );
        let mut programs = vec![(step, swapping.as_str())];
        if stays {
            programs.push((rm, pinned.as_str()));
        }
        let archive = archive_of("a", "inside");
        let list = format!("{}\t0.tar\nend\n", archive.len());
        let (code, err, home) = far_line(
            Left::Listed(&list),
            &[("0.tar", &archive)],
            "touch ../ran",
            &kind(""),
            None,
            &programs,
        );
        let case = format!("{step} {when} {}", if stays { "pinned" } else { "" });
        assert!(
            home.join("swapped").exists()
                && code == Some(75)
                && err.contains("so its copy stays") == stays
                && (!when.contains("-type d") || err.contains("neither a file nor a folder"))
                && (step != "find"
                    || swap != copy
                    || err.contains("moved or linked from its place")),
            "a link put at {case} didn't leave the job untaken ({code:?}): {err}"
        );
        assert!(
            kept.iter().all(|kept| home.join(kept).exists())
                && gone.iter().all(|gone| !home.join(gone).exists())
                && ["aside/ran", "ran"]
                    .iter()
                    .all(|ran| !home.join(ran).exists())
                && !work(&home).join("ran").exists(),
            "with a link put at {case}, the job ran, or what lay past the link was unpacked into \
             or cleared: {err}"
        );
        let _ = std::fs::remove_dir_all(&home);
    }
}
