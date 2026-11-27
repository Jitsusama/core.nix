//! The far line handing over what a job brings back: that it never
//! writes or clears through a link a job left at its handover or above
//! it, and that a job that closes its handover's folders, or puts a
//! folder or a link of its own where the far line writes, still has only
//! what it made brought back, or nothing, said so.

use super::crossing::refusing;
use super::sweeping::left_in;
use super::{Left, far_line, handover, kind, work};
use crate::far::{BACK, BROUGHT, HANDOVER, TREE};
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

/// What the far line says where a link was put at its handover as it
/// brought back.
pub(super) const HALTED: &str =
    "a link was put at its handover as it brought back, so nothing more is brought back";

#[test]
fn a_link_a_job_leaves_at_what_it_brings_back_is_never_written_through() {
    let mut kind = kind("");
    kind.brings_back = vec!["out".into()];
    // From its copy, a job reaches the lists of what it brings back,
    // beside the copy and in the handover.
    let job = format!(
        "mkdir out && printf a > out/a && echo kept > \"$HOME/beyond\" &&
         ln -s \"$HOME/beyond\" ../1.back && ln -s \"$HOME/beyond\" ../1.back.lost &&
         ln -s \"$HOME/beyond\" ../../{HANDOVER}/1/{BROUGHT}"
    );
    let (code, err, home) = far_line(Left::Whole, &[("x", "x")], &job, &kind, None, &[]);
    assert_eq!(
        std::fs::read_to_string(home.join("beyond")).unwrap_or_default(),
        "kept\n",
        "what's brought back was written through a link the job left ({code:?}): {err}"
    );
    let list = handover(&home).join(BROUGHT);
    let listed = std::fs::read_to_string(&list).unwrap_or_default();
    assert!(
        code == Some(0)
            && !std::fs::symlink_metadata(&list).unwrap().is_symlink()
            && listed == "1\tout/a\nend\n",
        "what the job made wasn't listed whole in a list of its own ({code:?}): {listed:?} {err}"
    );
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn a_file_put_at_the_list_s_name_after_the_handover_s_clear_still_gives_way_to_its_end() {
    // As one of the job's own processes could, once the handover is made
    // new, for a kind that brings nothing back, whose list is only its
    // end, and for one that does, whose list is written in a subshell of
    // its own.
    let when = format!("*\" -rf \"*/{BROUGHT}\" \"*");
    let (step, staling) = refusing(
        "rm",
        &when,
        "[ -e \"$HOME/staled\" ] || { rm \"$@\" && for last; do :; done && \
         echo stale > \"$last\" && : > \"$HOME/staled\"; exit 0; }",
    );
    let mut nothing = kind("");
    nothing.brings_back = Vec::new();
    for (kind, run, whole) in [
        (nothing, "true", "end\n"),
        (kind(""), "mkdir out && printf a > out/a", "1\tout/a\nend\n"),
    ] {
        let (code, err, home) = far_line(
            Left::Whole,
            &[("x", "x")],
            run,
            &kind,
            None,
            &[(step, &staling)],
        );
        let listed = std::fs::read_to_string(handover(&home).join(BROUGHT)).unwrap_or_default();
        assert!(
            code == Some(0) && home.join("staled").exists() && listed == whole,
            "a file at the list's name was kept over its end ({code:?}): {listed:?} {err}"
        );
        let _ = std::fs::remove_dir_all(&home);
    }
}

#[test]
fn a_link_a_job_leaves_at_its_handover_folder_is_cleared_rather_than_written_through() {
    let mut kind = kind("");
    kind.brings_back = vec!["out".into()];
    // From its copy, a job reaches its handover folder and the folders
    // above it in the quest's scratch, and moves one aside for a link to
    // a folder holding what the far line would write or clear, closed so
    // that anything opened through the link shows.
    for at in [
        format!("{HANDOVER}/1"),
        HANDOVER.to_owned(),
        "handover".to_owned(),
    ] {
        let job = format!(
            "mkdir out && printf a > out/a &&
             mkdir -p \"$HOME/beyond/{TREE}\" \"$HOME/beyond/{BACK}\" &&
             echo kept > \"$HOME/beyond/{TREE}/kept\" && echo kept > \"$HOME/beyond/{BACK}/0\" &&
             mv ../../{at} \"$HOME/moved\" && ln -s \"$HOME/beyond\" ../../{at} &&
             chmod 500 \"$HOME/beyond\""
        );
        let (code, err, home) = far_line(Left::Whole, &[("x", "x")], &job, &kind, None, &[]);
        let beyond = home.join("beyond");
        let mode = std::fs::metadata(&beyond).unwrap().permissions().mode() & 0o777;
        let _ = std::fs::set_permissions(&beyond, std::fs::Permissions::from_mode(0o755));
        assert!(
            mode == 0o500,
            "the far line opened what a link at {at} leads to ({code:?}): {err}"
        );
        assert!(
            left_in(&beyond) == [BACK, TREE]
                && std::fs::read_to_string(beyond.join(TREE).join("kept")).is_ok()
                && std::fs::read_to_string(beyond.join(BACK).join("0")).unwrap() == "kept\n",
            "the far line wrote or cleared through a link at {at} ({code:?}): {err}"
        );
        let list = handover(&home).join(BROUGHT);
        let listed = std::fs::read_to_string(&list).unwrap_or_default();
        assert!(
            code == Some(0) && listed == "1\tout/a\nend\n",
            "what the job made wasn't listed whole in a handover of its own past a link at {at} \
             ({code:?}): {listed:?} {err}"
        );
        let _ = std::fs::remove_dir_all(&home);
    }
}

/// A job's words moving its handover folder aside for a link to a folder
/// `beyond` its home, holding what the far line would write or clear.
fn linking_its_handover() -> String {
    format!(
        "mkdir -p \"$HOME/beyond/{TREE}\" \"$HOME/beyond/{BACK}\" &&
         echo kept > \"$HOME/beyond/{TREE}/kept\" && echo kept > \"$HOME/beyond/{BACK}/0\" &&
         echo kept > \"$HOME/beyond/{BROUGHT}\" &&
         mv ../../{HANDOVER}/1 \"$HOME/moved\" && ln -s \"$HOME/beyond\" ../../{HANDOVER}/1"
    )
}

/// Whether `beyond` still holds just what `linking_its_handover` put there.
fn kept_beyond(home: &Path) -> bool {
    let beyond = home.join("beyond");
    left_in(&beyond) == [BACK, BROUGHT, TREE]
        && std::fs::read_to_string(beyond.join(TREE).join("kept")).is_ok()
        && std::fs::read_to_string(beyond.join(BACK).join("0")).unwrap() == "kept\n"
        && std::fs::read_to_string(beyond.join(BROUGHT)).unwrap() == "kept\n"
}

#[test]
fn a_link_a_job_pins_at_its_handover_folder_stops_all_it_would_bring_back() {
    let mut kind = kind("");
    kind.brings_back = vec!["out".into()];
    // A link the far line isn't allowed to clear, and a code of its own,
    // which the far line ends with all the same.
    let job = format!(
        "mkdir out && printf a > out/a && {} && exit 3",
        linking_its_handover()
    );
    let (rm, refused) = pinning();
    let programs = [(rm, refused.as_str())];
    let (code, err, home) = far_line(Left::Whole, &[("x", "x")], &job, &kind, None, &programs);
    assert!(
        kept_beyond(&home),
        "the far line wrote or cleared through a link it couldn't clear ({code:?}): {err}"
    );
    assert!(
        code == Some(3)
            && err.contains("a link it left at its handover couldn't be cleared, so nothing")
            && !work(&home).join("1").exists()
            && !work(&home).join("1.run").exists(),
        "the far line didn't say it brought nothing back, kept its copy, or ended with \
         another code than its job's ({code:?}): {err}"
    );
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn a_link_pinned_at_the_handover_as_it_brings_back_gets_no_end_written_through_it() {
    let mut kind = kind("");
    kind.brings_back = vec!["out".into()];
    // A cp that, once the job's one file crossed, moves the handover aside
    // for a link the far line isn't allowed to clear: the file's size,
    // read after, and the list's end and its cut would go through it, so
    // the file is left unlisted.
    let swapping = format!(
        "PATH=${{PATH#*:}} cp \"$@\" || exit\n\
         [ -e \"$HOME/swapped\" ] || {{ {} && : > \"$HOME/swapped\"; }}",
        linking_its_handover()
    );
    let (rm, refused) = pinning();
    let programs = [("cp", swapping.as_str()), (rm, refused.as_str())];
    let job = "mkdir out && printf a > out/a";
    let (code, err, home) = far_line(Left::Whole, &[("x", "x")], job, &kind, None, &programs);
    let listed = std::fs::read_to_string(home.join("moved").join(BROUGHT)).unwrap_or_default();
    assert!(
        home.join("swapped").exists() && kept_beyond(&home),
        "the far line ended its list, or cleared, through a link pinned as it brought \
         back ({code:?}): {err}"
    );
    assert!(
        code == Some(0)
            && err.contains("a link at the job's handover couldn't be cleared, so its tree stays")
            && err.contains(
                "so the list of what it brings back isn't ended and nothing is brought back"
            )
            && err.contains(HALTED)
            && listed.is_empty()
            && !work(&home).join("1").exists(),
        "the far line didn't say the link stays and its list is left unended, listed \
         the file the link took, or kept its copy ({code:?}): {listed:?} {err}"
    );
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn a_link_pinned_at_the_handover_between_two_files_gets_neither_copy_through_it() {
    let mut kind = kind("");
    kind.brings_back = vec!["out".into()];
    // As above, but with a second file to copy once the handover is
    // swapped: its copy, and the size read from it, would go through the
    // link, as would the end and the cut after, so neither is listed.
    let swapping = format!(
        "PATH=${{PATH#*:}} cp \"$@\" || exit\n\
         [ -e \"$HOME/swapped\" ] || {{ {} && : > \"$HOME/swapped\"; }}",
        linking_its_handover()
    );
    let (rm, refused) = pinning();
    let programs = [("cp", swapping.as_str()), (rm, refused.as_str())];
    let job = "mkdir out && printf a > out/a && printf b > out/b";
    let (code, err, home) = far_line(Left::Whole, &[("x", "x")], job, &kind, None, &programs);
    let listed = std::fs::read_to_string(home.join("moved").join(BROUGHT)).unwrap_or_default();
    assert!(
        home.join("swapped").exists()
            && kept_beyond(&home)
            && left_in(&home.join("beyond").join(BACK)) == ["0"],
        "the far line copied the second file, or ended its list, through a link pinned \
         between them ({code:?}): {err}"
    );
    assert!(
        code == Some(0) && err.contains(HALTED) && listed.is_empty(),
        "the far line didn't say nothing more is brought back, or listed a file the link \
         took ({code:?}): {listed:?} {err}"
    );
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn a_link_pinned_at_the_handover_as_its_list_is_sorted_has_nothing_brought_back_through_it() {
    let mut kind = kind("");
    kind.brings_back = vec!["out".into()];
    // A sort that, once the names to bring back are sorted, moves the
    // handover aside for a link the far line isn't allowed to clear: each
    // copy, and the list begun after, would go through it.
    let sorting = format!(
        "PATH=${{PATH#*:}} sort \"$@\" || exit\n\
         [ -e \"$HOME/swapped\" ] || case \" $* \" in *'.back '*)\n\
         {} && : > \"$HOME/swapped\" ;; esac",
        linking_its_handover()
    );
    let (rm, refused) = pinning();
    let programs = [("sort", sorting.as_str()), (rm, refused.as_str())];
    let job = "mkdir out && printf a > out/a";
    let (code, err, home) = far_line(Left::Whole, &[("x", "x")], job, &kind, None, &programs);
    assert!(
        home.join("swapped").exists() && kept_beyond(&home),
        "the far line copied, or began its list, through a link pinned as it sorted \
         ({code:?}): {err}"
    );
    assert!(
        code == Some(0) && err.contains(HALTED) && !home.join("moved").join(BROUGHT).exists(),
        "the far line didn't say nothing more is brought back, or began its list \
         ({code:?}): {err}"
    );
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn a_file_put_at_a_copy_s_name_gets_no_copy_over_it_and_is_said_for_what_it_is() {
    let mut kind = kind("");
    kind.brings_back = vec!["out".into()];
    // As below, but a file rather than a link at the next copy's name:
    // no link was put at the handover, so the far line says what was.
    let planting = "PATH=${PATH#*:} cp \"$@\" || exit\n\
         [ -e \"$HOME/planted\" ] || { printf t > \"${2%/*}/1\" && : > \"$HOME/planted\"; }";
    let programs = [("cp", planting)];
    let job = "mkdir out && printf a > out/a && printf b > out/b";
    let (code, err, home) = far_line(Left::Whole, &[("x", "x")], job, &kind, None, &programs);
    let listed = std::fs::read_to_string(handover(&home).join(BROUGHT)).unwrap_or_default();
    assert!(
        code == Some(0)
            && home.join("planted").exists()
            && err.contains("something was put at a copy's name as it brought back")
            && !err.contains(HALTED)
            && listed == "1\tout/a\n\ncut\n",
        "the far line didn't say something stood at a copy's name, or end its list cut \
         after the file it copied ({code:?}): {listed:?} {err}"
    );
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn a_link_put_at_a_copy_s_name_gets_no_copy_through_it_and_the_list_ends_cut() {
    let mut kind = kind("");
    kind.brings_back = vec!["out".into()];
    // A cp that, once the first file is copied, puts a link at the next
    // copy's name to a file of the job's, as a job of the quest may
    // meanwhile: the next copy, written there, would go through it.
    let planting = "PATH=${PATH#*:} cp \"$@\" || exit\n\
         [ -e \"$HOME/planted\" ] || \
         { ln -s \"$HOME/target\" \"${2%/*}/1\" && : > \"$HOME/planted\"; }";
    let programs = [("cp", planting)];
    let job = "mkdir out && printf a > out/a && printf b > out/b && printf t > \"$HOME/target\"";
    let (code, err, home) = far_line(Left::Whole, &[("x", "x")], job, &kind, None, &programs);
    let target = std::fs::read_to_string(home.join("target")).unwrap_or_default();
    let listed = std::fs::read_to_string(handover(&home).join(BROUGHT)).unwrap_or_default();
    assert!(
        home.join("planted").exists() && target == "t",
        "the far line copied through a link put at a copy's name ({code:?}): {target:?} {err}"
    );
    assert!(
        code == Some(0)
            && err.contains(HALTED)
            && listed.starts_with("1\tout/a\n")
            && !listed.contains("out/b")
            && listed.ends_with("\ncut\n"),
        "the far line didn't say nothing more is brought back, or end its list cut after \
         the file it copied ({code:?}): {listed:?} {err}"
    );
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn a_link_put_at_a_copy_s_own_name_as_it_crosses_is_never_read_through() {
    let mut kind = kind("");
    kind.brings_back = vec!["out".into()];
    // A cp that, once the first file is copied, puts a link at that very
    // copy's name to a file of the job's, as a job of the quest may
    // meanwhile: the copy's size, read after, would be that file's, and
    // it would be listed as crossed at it.
    let planting = "PATH=${PATH#*:} cp \"$@\" || exit\n\
         [ -e \"$HOME/planted\" ] || \
         { rm -f \"$2\" && ln -s \"$HOME/target\" \"$2\" && : > \"$HOME/planted\"; }";
    let programs = [("cp", planting)];
    let job = "mkdir out && printf a > out/a && printf b > out/b && \
         printf target > \"$HOME/target\"";
    let (code, err, home) = far_line(Left::Whole, &[("x", "x")], job, &kind, None, &programs);
    let target = std::fs::read_to_string(home.join("target")).unwrap_or_default();
    let listed = std::fs::read_to_string(handover(&home).join(BROUGHT)).unwrap_or_default();
    assert!(
        home.join("planted").exists() && target == "target",
        "the far line wrote or cleared through a link put at a copy's own name \
         ({code:?}): {target:?} {err}"
    );
    assert!(
        code == Some(0) && err.contains(HALTED) && listed == "\ncut\n",
        "the far line didn't say nothing more is brought back, or listed the file a link \
         at its copy's name took, or didn't end its list cut ({code:?}): {listed:?} {err}"
    );
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn a_link_pinned_at_the_handover_of_a_kind_bringing_nothing_back_gets_no_end_through_it() {
    let mut nothing = kind("");
    nothing.brings_back = Vec::new();
    // A mkdir that, once the handover's folder for copies is made new,
    // moves the handover aside for a link the far line isn't allowed to
    // clear: the list's end, written after, would go through it.
    let swapping = format!(
        "PATH=${{PATH#*:}} mkdir \"$@\" || exit\n\
         case \" $* \" in *\"/1/{BACK} \"*) {} ;; esac",
        linking_its_handover()
    );
    let (rm, refused) = pinning();
    let programs = [("mkdir", swapping.as_str()), (rm, refused.as_str())];
    let (code, err, home) = far_line(
        Left::Whole,
        &[("x", "x")],
        "true",
        &nothing,
        None,
        &programs,
    );
    assert!(
        home.join("moved").exists() && kept_beyond(&home),
        "the far line wrote its list's end, or cleared, through a link pinned at the \
         handover of a kind bringing nothing back ({code:?}): {err}"
    );
    assert!(
        code == Some(0)
            && err.contains("a link at the job's handover couldn't be cleared, so its tree stays")
            && !err.contains("handover wasn't cleared"),
        "the far line didn't say only that the link stays ({code:?}): {err}"
    );
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn a_handover_whose_folder_for_copies_cant_be_made_new_ends_its_list_cut() {
    let mut kind = kind("");
    kind.brings_back = vec!["out".into()];
    // A folder for copies that can't be made, as on a disk gone wrong, and
    // at the list's name, one the job wrote itself, claiming all came, or
    // a folder holding one the job closed, which the step making the
    // handover new opens and clears before it gets that far, so the cut
    // has a name of its own to be written at.
    let list = format!("../../{HANDOVER}/1/{BROUGHT}");
    for made in [
        format!("printf '1\tout/a\nend\n' > {list}"),
        format!("mkdir -p {list}/d && touch {list}/d/f && chmod 500 {list}/d"),
    ] {
        let job = format!("mkdir out && printf a > out/a && {made}");
        let (mkdir, refused) = refusing("mkdir", &format!("*\"/{BACK} \"*"), "exit 1");
        let programs = [(mkdir, refused.as_str())];
        let (code, err, home) = far_line(Left::Whole, &[("x", "x")], &job, &kind, None, &programs);
        let list = handover(&home).join(BROUGHT);
        let _ = std::fs::set_permissions(list.join("d"), std::fs::Permissions::from_mode(0o755));
        let listed = std::fs::read_to_string(&list).unwrap_or_default();
        assert!(
            code == Some(0)
                && err.contains("its folder for copies or its list couldn't be made new, so")
                && listed == "\ncut\n"
                && !handover(&home).join(TREE).exists()
                && !work(&home).join("1").exists(),
            "the far line didn't end its list cut past {made}, or kept its tree or copy \
             ({code:?}): {listed:?} {err}"
        );
        let _ = std::fs::remove_dir_all(&home);
    }
}

#[test]
fn a_folder_a_job_makes_at_its_list_s_name_still_has_what_it_made_listed() {
    let mut kind = kind("");
    kind.brings_back = vec!["out".into()];
    // A folder at the list's name, holding one the job closed, which the
    // list's own write would fail on, so the near side would wait it out.
    let list = format!("../../{HANDOVER}/1/{BROUGHT}");
    let job = format!(
        "mkdir out && printf a > out/a &&
         mkdir -p {list}/d && touch {list}/d/f && chmod 500 {list}/d"
    );
    let (code, err, home) = far_line(Left::Whole, &[("x", "x")], &job, &kind, None, &[]);
    let list = handover(&home).join(BROUGHT);
    let _ = std::fs::set_permissions(list.join("d"), std::fs::Permissions::from_mode(0o755));
    let listed = std::fs::read_to_string(&list).unwrap_or_default();
    assert!(
        code == Some(0) && listed == "1\tout/a\nend\n",
        "a folder at the list's name kept what the job made from being listed \
         ({code:?}): {listed:?} {err}"
    );
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn a_handover_folder_the_far_line_cant_open_is_never_said_to_be_a_link() {
    let mut kind = kind("");
    kind.brings_back = vec!["out".into()];
    // A chmod that fails on the job's handover folder alone, as on one the
    // far line doesn't own, though it stays open to it, leaving word that
    // it was asked.
    let (chmod, refused) = refusing(
        "chmod",
        &format!("*\"/{HANDOVER}/1 \"*"),
        "touch \"$HOME/refused\"; exit 1",
    );
    let programs = [(chmod, refused.as_str())];
    let (code, err, home) = far_line(
        Left::Whole,
        &[("x", "x")],
        "mkdir out && printf a > out/a",
        &kind,
        None,
        &programs,
    );
    let listed = std::fs::read_to_string(handover(&home).join(BROUGHT)).unwrap_or_default();
    assert!(
        home.join("refused").exists(),
        "the far line never asked to open its handover folder ({code:?}): {err}"
    );
    assert!(
        code == Some(0) && !err.contains("a link it left") && listed == "1\tout/a\nend\n",
        "a handover folder that couldn't be opened was taken for a link ({code:?}): \
         {listed:?} {err}"
    );
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn a_link_a_job_leaves_at_its_folder_for_copies_is_never_opened_through() {
    let mut kind = kind("");
    kind.brings_back = vec!["out".into()];
    // A link where the folder for copies goes, to a folder holding one
    // closed, which a find that followed the link it starts at would open.
    let at = format!("../../{HANDOVER}/1/{BACK}");
    let job = format!(
        "mkdir out && printf a > out/a && mkdir -p \"$HOME/beyond/d\" &&
         chmod 500 \"$HOME/beyond/d\" && rm -rf {at} && ln -s \"$HOME/beyond\" {at}"
    );
    let (code, err, home) = far_line(Left::Whole, &[("x", "x")], &job, &kind, None, &[]);
    let closed = home.join("beyond").join("d");
    let mode = std::fs::metadata(&closed).map(|closed| closed.permissions().mode() & 0o777);
    let _ = std::fs::set_permissions(&closed, std::fs::Permissions::from_mode(0o755));
    let listed = std::fs::read_to_string(handover(&home).join(BROUGHT)).unwrap_or_default();
    // Nothing written through it either: what it leads to holds only d.
    // Read only where it's still there, so its loss is said, not a panic.
    let beyond = if home.join("beyond").is_dir() {
        left_in(&home.join("beyond"))
    } else {
        Vec::new()
    };
    assert!(
        code == Some(0)
            && mode.as_ref().is_ok_and(|mode| *mode == 0o500)
            && beyond == ["d"]
            && listed == "1\tout/a\nend\n",
        "the far line opened or wrote what a link at its folder for copies leads to, or \
         didn't list what the job made ({code:?}): {mode:?} {beyond:?} {listed:?} {err}"
    );
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn a_hard_link_a_job_puts_at_its_handover_folder_s_name_is_never_opened() {
    let mut kind = kind("");
    kind.brings_back = vec!["out".into()];
    // A file of the job's own, open only to reading, linked hard at its
    // handover folder's name, which a chmod there would open.
    let job = format!(
        "mkdir out && printf a > out/a && echo kept > \"$HOME/own\" && chmod 400 \"$HOME/own\" &&
         mv ../../{HANDOVER}/1 \"$HOME/moved\" && ln \"$HOME/own\" ../../{HANDOVER}/1"
    );
    let (code, err, home) = far_line(Left::Whole, &[("x", "x")], &job, &kind, None, &[]);
    let mode = std::fs::metadata(home.join("own")).map(|own| own.permissions().mode() & 0o777);
    assert!(
        code == Some(0)
            && mode.as_ref().is_ok_and(|mode| *mode == 0o400)
            && err.contains("its folder for copies or its list couldn't be made new, so"),
        "the far line opened a file linked hard at its handover folder's name, or didn't say \
         nothing came back ({code:?}): {mode:?} {err}"
    );
    let _ = std::fs::remove_dir_all(&home);
}

/// A stand-in for `rm` that never clears the job's handover folder, as
/// where a link stands at it that the far line isn't allowed to clear.
pub(super) fn pinning() -> (&'static str, String) {
    refusing("rm", &format!("*\"/{HANDOVER}/1 \"*"), "exit 1")
}

#[test]
fn a_job_that_closes_its_handover_still_has_only_what_it_made_brought_back() {
    let mut kind = kind("");
    kind.brings_back = vec!["out".into()];
    // Its own list, claiming another size, left in a handover closed to
    // all, with a folder closed under the copies' folder; or in one whose
    // folder above, the one above that, or the quest's scratch itself, is
    // closed, so nothing in it can be seen.
    let at = format!("../../{HANDOVER}/1");
    for closed in [
        format!("{at}/{BACK}/d {at}"),
        format!("../../{HANDOVER}"),
        "../../handover".to_owned(),
        "../..".to_owned(),
    ] {
        let job = format!(
            "mkdir out && printf a > out/a && mkdir -p {at}/{BACK}/d && touch {at}/{BACK}/d/f &&
             printf '9\tout/a\nend\n' > {at}/{BROUGHT} && chmod 0 {closed}"
        );
        let (code, err, home) = far_line(Left::Whole, &[("x", "x")], &job, &kind, None, &[]);
        let job = handover(&home);
        for folder in job.ancestors().take(4) {
            let _ = std::fs::set_permissions(folder, std::fs::Permissions::from_mode(0o755));
        }
        let listed = std::fs::read_to_string(job.join(BROUGHT)).unwrap_or_default();
        let copied = std::fs::read_to_string(job.join(BACK).join("0"));
        assert!(
            code == Some(0)
                && listed == "1\tout/a\nend\n"
                && copied.is_ok_and(|copied| copied == "a")
                && !job.join(BACK).join("d").exists(),
            "a handover closed at {closed} kept the job's own list or lost what it made \
             ({code:?}): {listed:?} {err}"
        );
        let _ = std::fs::remove_dir_all(&home);
    }
}

#[test]
fn a_far_line_stopped_once_its_job_links_its_handover_clears_nothing_through_it() {
    // The link left as it is, in a folder the job closed, or where it
    // can't be cleared.
    let (rm, refused) = pinning();
    let cases: [(String, &[(&str, &str)]); 3] = [
        (String::new(), &[]),
        (format!("&& chmod a-w ../../{HANDOVER}"), &[]),
        ("pinned".to_owned(), &[(rm, refused.as_str())]),
    ];
    for (pinned, programs) in cases {
        let closing = if programs.is_empty() {
            pinned.as_str()
        } else {
            ""
        };
        let job = format!("{} {closing} && kill -TERM $PPID", linking_its_handover());
        let (code, err, home) =
            far_line(Left::Whole, &[("x", "x")], &job, &kind(""), None, programs);
        let closed = handover(&home).parent().unwrap().to_owned();
        let _ = std::fs::set_permissions(&closed, std::fs::Permissions::from_mode(0o755));
        assert!(
            std::fs::read_to_string(home.join("signal")).unwrap() == "15",
            "the far line wasn't stopped ({code:?}) {pinned}: {err}"
        );
        assert!(
            kept_beyond(&home),
            "a stopped far line cleared through the link at its handover {pinned}: {err}"
        );
        // Cleared wherever the far line may clear it, and left only where
        // it may not.
        let linked = std::fs::symlink_metadata(handover(&home)).is_ok_and(|at| at.is_symlink());
        assert!(
            linked != programs.is_empty(),
            "a stopped far line {} the link at its handover {pinned}: {err}",
            if linked { "kept" } else { "cleared" }
        );
        assert!(
            !work(&home).join("1").exists() && !work(&home).join("1.run").exists(),
            "a stopped far line kept its copy or its run's file {pinned}: {err}"
        );
        let _ = std::fs::remove_dir_all(&home);
    }
}

#[test]
fn a_link_a_job_puts_at_the_copies_folder_redirects_no_clearing_of_its_copy() {
    // A job that moves the copies' folder aside and puts at its name a
    // link to a folder holding a copy at its own number, with its run's
    // file and the lists and archive beside it, then ends, or is stopped,
    // or leaves a link the far line can't clear, or ends where the
    // handover's folder for copies can't be made, so nothing comes back.
    let decoy = ["1", "1.back", "1.back.lost", "1.names", "1.run", "1.tar"];
    let swapping = format!(
        "s=$(cd ../.. && pwd)/.s && mv \"$s\" \"$HOME/aside\" &&
         mkdir -p \"$HOME/decoy/1\" && : > \"$HOME/decoy/1/kept\" && cd \"$HOME/decoy\" &&
         touch {} && ln -s \"$HOME/decoy\" \"$s\"",
        decoy[1..].join(" ")
    );
    let (rm, pinned) = refusing("rm", "*\"/.s \"*", "exit 1");
    let (mkdir, unmade) = refusing("mkdir", "*\"/1/back \"*", "exit 1");
    let cases: [(&str, &[(&str, &str)]); 4] = [
        ("ended", &[]),
        ("stopped", &[]),
        ("ended past a link it can't clear", &[(rm, pinned.as_str())]),
        ("ended bringing nothing back", &[(mkdir, unmade.as_str())]),
    ];
    for (how, programs) in cases {
        let stop = if how == "stopped" {
            " && kill -TERM $PPID"
        } else {
            ""
        };
        let job = format!("{swapping}{stop}");
        let (code, err, home) =
            far_line(Left::Whole, &[("x", "x")], &job, &kind(""), None, programs);
        assert!(
            (how == "stopped")
                == std::fs::read_to_string(home.join("signal")).is_ok_and(|signal| signal == "15"),
            "the far line {how} was stopped, or wasn't ({code:?}): {err}"
        );
        // Exactly as the job left it, nothing taken and nothing written.
        assert!(
            home.join("aside").exists()
                && home.join("decoy/1/kept").exists()
                && left_in(&home.join("decoy")) == decoy
                && left_in(&home.join("decoy/1")) == ["kept"],
            "a far line {how} cleared or wrote through the link at its copies' folder \
             ({code:?}): {:?} {err}",
            left_in(&home.join("decoy"))
        );
        let linked = std::fs::symlink_metadata(work(&home)).is_ok_and(|at| at.is_symlink());
        let stays = programs.first().is_some_and(|(name, _)| *name == "rm");
        assert!(
            err.contains("so nothing is brought back") == (how == "ended bringing nothing back"),
            "a far line {how} handed back, or didn't, unlike its case ({code:?}): {err}"
        );
        assert!(
            linked == stays
                && err.contains("so its copy stays") == stays
                && err.contains("copies' folder was cleared") != stays,
            "a far line {how} kept the link it may clear, or didn't say what it did with it \
             ({code:?}): {err}"
        );
        let _ = std::fs::remove_dir_all(&home);
    }
}

#[test]
fn a_link_to_a_file_put_at_the_list_s_name_after_its_clear_is_never_written_through() {
    // As one of the job's own processes could, once the list's name is
    // cleared for its write: for a kind that brings nothing back, and for
    // one whose list is written in a step of its own, which then can't
    // write it, and cuts it only where no link stands at its name. A link
    // to a device is still written through, as at the run's file, and one
    // put there once the list is written is left to the job's processes
    // being stopped.
    // The link stands at the list's name after, so it was put there and
    // nothing was made in its place.
    let when = format!("*\" -f \"*/{BROUGHT}\" \"*");
    let (step, linking) = refusing(
        "rm",
        &when,
        "rm \"$@\"; for last; do :; done && echo kept > \"$HOME/beyond\" && \
         ln -s \"$HOME/beyond\" \"$last\" && : > \"$HOME/linked\"; exit 0",
    );
    // The helper's kind brings back out, so the one bringing nothing is
    // made so here. Only the one bringing back says it couldn't write
    // what it brings, so each row is known to have taken its own step.
    let mut nothing = kind("");
    nothing.brings_back = Vec::new();
    let unwritten = "couldn't write down all it brings back";
    for (kind, run, says) in [
        (nothing, "true", false),
        (kind(""), "mkdir out && printf a > out/a", true),
    ] {
        let (code, err, home) = far_line(
            Left::Whole,
            &[("x", "x")],
            run,
            &kind,
            None,
            &[(step, &linking)],
        );
        let beyond = std::fs::read_to_string(home.join("beyond")).unwrap_or_default();
        let list = std::fs::symlink_metadata(handover(&home).join(BROUGHT));
        assert!(
            home.join("linked").exists()
                && list.is_ok_and(|list| list.is_symlink())
                && beyond == "kept\n",
            "the list, or its cut, was written through a link put at its name, or the link \
             wasn't left there ({code:?}): {beyond:?} {err}"
        );
        assert!(
            err.contains(unwritten) == says,
            "the far line {} it couldn't write what it brings back ({code:?}): {err}",
            if says {
                "never said"
            } else {
                "bringing nothing said"
            }
        );
        let _ = std::fs::remove_dir_all(&home);
    }
}

#[test]
fn a_sort_that_fails_loses_each_path_past_a_link_put_at_the_losses_name() {
    let mut kind = kind("");
    kind.brings_back = vec!["out".into()];
    // A sort that fails having written what it sorted, as one stopped part
    // way does, and a link one of the job's processes puts meanwhile at
    // the name the losses are listed at. Both names are cleared for the
    // losses each path then makes, so none is written through the link.
    // The list's end is what shows the clear: without it, noclobber
    // refuses the names and the list ends cut. The write through the link
    // is refused by noclobber alone, which the far line keeps throughout.
    let (sort, failing) = refusing(
        "sort",
        "*\" -u \"*",
        "sort \"$@\"; echo kept > \"$HOME/beyond\" && rm -f \"$PWD.back.lost\" && \
         ln -s \"$HOME/beyond\" \"$PWD.back.lost\" && : > \"$HOME/linked\"; exit 1",
    );
    let (code, err, home) = far_line(
        Left::Whole,
        &[("x", "x")],
        "mkdir out && printf a > out/a",
        &kind,
        None,
        &[(sort, &failing)],
    );
    let beyond = std::fs::read_to_string(home.join("beyond")).unwrap_or_default();
    assert!(
        home.join("linked").exists() && beyond == "kept\n",
        "the losses were written through a link put at their name ({code:?}): {beyond:?} {err}"
    );
    let listed = std::fs::read_to_string(handover(&home).join(BROUGHT)).unwrap_or_default();
    assert!(
        code == Some(0) && listed == "-\tout\nend\n",
        "a failed sort didn't lose its path in a list that ended ({code:?}): {listed:?} {err}"
    );
    let _ = std::fs::remove_dir_all(&home);
}
