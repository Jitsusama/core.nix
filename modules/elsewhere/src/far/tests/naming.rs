//! The far line handing over what a job brings back where a job puts a
//! file or a link at a copy's own name as it crosses: that nothing is
//! copied over or through what stands there, nor read through it, and
//! that the list ends cut after the files that came back.

use super::handing::HALTED;
use super::{Left, far_line, handover, kind};
use crate::far::{BACK, BROUGHT};
use std::os::unix::fs::PermissionsExt;

#[test]
fn a_file_put_at_a_copy_s_name_gets_no_copy_over_it_and_is_said_for_what_it_is() {
    let mut kind = kind("");
    kind.brings_back = vec!["out".into()];
    // As below, but a file rather than a link at the next copy's name:
    // no link was put at the handover, so the far line says what was.
    let planting = "PATH=${PATH#*:} cp \"$@\" || exit\n\
         [ -e \"$HOME/planted\" ] || { printf t > \"${3%/*}/1\" && : > \"$HOME/planted\"; }";
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
         { ln -s \"$HOME/target\" \"${3%/*}/1\" && : > \"$HOME/planted\"; }";
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
    // it would be listed as crossed at it. A read leaves no trace on a
    // file, so the job's is closed to reading: a line reading through the
    // link fails there, says the copy didn't come back and clears the
    // link, which the link still standing shows beside the list. As root,
    // which reads it anyway, only the list shows it. The file's size and
    // mode show a write or a chmod through the link, and its being there
    // a clear.
    let planting = "PATH=${PATH#*:} cp \"$@\" || exit\n\
         [ -e \"$HOME/planted\" ] || \
         { rm -f \"$3\" && ln -s \"$HOME/target\" \"$3\" && : > \"$HOME/planted\"; }";
    let programs = [("cp", planting)];
    let job = "mkdir out && printf a > out/a && printf b > out/b && \
         printf target > \"$HOME/target\" && chmod 0 \"$HOME/target\"";
    let (code, err, home) = far_line(Left::Whole, &[("x", "x")], job, &kind, None, &programs);
    let link = std::fs::read_link(handover(&home).join(BACK).join("0")).ok();
    let target = std::fs::symlink_metadata(home.join("target"))
        .ok()
        .map(|target| {
            (
                target.is_file(),
                target.len(),
                target.permissions().mode() & 0o777,
            )
        });
    let listed = std::fs::read_to_string(handover(&home).join(BROUGHT)).unwrap_or_default();
    assert!(
        home.join("planted").exists()
            && link == Some(home.join("target"))
            && target == Some((true, 6, 0)),
        "the far line read, wrote or cleared through a link put at a copy's own name, or \
         cleared the link ({code:?}): {link:?} {target:?} {err}"
    );
    assert!(
        code == Some(0) && err.contains(HALTED) && listed == "\ncut\n",
        "the far line didn't say nothing more is brought back, or listed the file a link \
         at its copy's name took, or didn't end its list cut ({code:?}): {listed:?} {err}"
    );
    let _ = std::fs::remove_dir_all(&home);
}
