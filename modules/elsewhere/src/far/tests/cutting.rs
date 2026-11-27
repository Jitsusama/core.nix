//! The far line cutting its list of what's brought back once it stopped
//! for a link put at its handover: the cut written in a handover made
//! again where clearing the link took the job's folder, never through a
//! link put back once the far line cleared one or put there with none
//! cleared, never in a quest's scratch made again, and said in words of
//! its own where it can't be.

use super::crossing::refusing;
use super::handing::HALTED;
use super::sweeping::left_in;
use super::{Left, far_line, handover, kind, work};
use crate::far::{BACK, BROUGHT, HANDOVER, TREE};

/// What the far line says where its list couldn't be cut.
const UNCUT: &str = "couldn't cut the list of what it brings back, so nothing is brought back";

/// What the far line says where a link stands at its handover as it would
/// cut its list, though none was cleared there.
const STANDING: &str =
    "a link stands at its handover, so its list isn't cut and nothing is brought back";

/// A cp that, once the first file crossed, moves `moved` aside to
/// `beyond` in the far run's home, with a link to it at its name, so the
/// copy landed through it and a second would too.
fn swapping(moved: &str) -> String {
    format!(
        "PATH=${{PATH#*:}} cp \"$@\" || exit\n\
         [ -e \"$HOME/swapped\" ] || {{ mv {moved} \"$HOME/beyond\" && \
         ln -s \"$HOME/beyond\" {moved} && : > \"$HOME/swapped\"; }}"
    )
}

#[test]
fn a_link_put_at_or_above_the_handover_as_a_file_crosses_brings_no_more_and_ends_cut() {
    let mut kind = kind("");
    kind.brings_back = vec!["out".into()];
    let at = format!("../../{HANDOVER}/1");
    // The handovers' folder, the one below it, the job's handover or its
    // folder for copies, moved aside as the first file crosses. Neither
    // file is listed, and where the link the far line clears took the
    // job's handover with it, the list is cut in one made again, never in
    // what the link led to.
    for (moved, back) in [
        ("../../handover".to_owned(), "elsewhere/1/back"),
        (format!("../../{HANDOVER}"), "1/back"),
        (at.clone(), "back"),
        (format!("{at}/{BACK}"), ""),
    ] {
        let swapping = swapping(&moved);
        let programs = [("cp", swapping.as_str())];
        let job = "mkdir out && printf a > out/a && printf b > out/b";
        let (code, err, home) = far_line(Left::Whole, &[("x", "x")], job, &kind, None, &programs);
        let listed = std::fs::read_to_string(handover(&home).join(BROUGHT)).unwrap_or_default();
        let beyond = back
            .strip_suffix(BACK)
            .map(|it| home.join("beyond").join(it).join(BROUGHT));
        let beyond = beyond.map(|it| std::fs::read_to_string(it).unwrap_or_default());
        assert!(
            home.join("swapped").exists()
                && left_in(&home.join("beyond").join(back)) == ["0"]
                && beyond.as_deref().is_none_or(|it| !it.contains("cut")),
            "the far line copied the second file, or cut its list, through a link at {moved} \
             ({code:?}): {beyond:?} {err}"
        );
        assert!(
            code == Some(0)
                && err.contains(HALTED)
                && !err.contains(UNCUT)
                && !err.contains("cannot create")
                && !err.contains("No such file")
                && !err.contains("Directory nonexistent")
                && listed == "\ncut\n",
            "a link at {moved} as a file crossed didn't stop it, or end the list cut \
             ({code:?}): {listed:?} {err}"
        );
        let _ = std::fs::remove_dir_all(&home);
    }
}

#[test]
fn a_link_put_at_the_quest_s_scratch_as_a_file_crosses_brings_no_more() {
    let mut kind = kind("");
    kind.brings_back = vec!["out".into()];
    // A cp that, once the first file crossed, moves the quest's scratch
    // aside to `beyond` in the far run's home, with a link to it at its
    // name, so every folder below it is reached through the link though
    // none of them is one. Named whole, since the copy the far line runs
    // in moves with it.
    let swapping = "PATH=${PATH#*:} cp \"$@\" || exit\n\
         [ -e \"$HOME/swapped\" ] || { s=$(cd ../.. && pwd) && mv \"$s\" \"$HOME/beyond\" && \
         ln -s \"$HOME/beyond\" \"$s\" && : > \"$HOME/swapped\"; }";
    let programs = [("cp", swapping)];
    let job = "mkdir out && printf a > out/a && printf b > out/b";
    let (code, err, home) = far_line(Left::Whole, &[("x", "x")], job, &kind, None, &programs);
    let back = home.join("beyond").join(HANDOVER).join("1").join(BACK);
    let listed = std::fs::read_to_string(handover(&home).join(BROUGHT)).unwrap_or_default();
    assert!(
        home.join("swapped").exists() && left_in(&back) == ["0"],
        "the far line copied the second file through a link at the quest's scratch \
         ({code:?}): {err}"
    );
    assert!(
        code == Some(0) && err.contains(HALTED) && !listed.contains("out/"),
        "a link at the quest's scratch as a file crossed didn't stop it, or had a file \
         listed ({code:?}): {listed:?} {err}"
    );
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn a_handover_that_can_t_be_made_again_gets_no_cut_and_the_far_line_says_so() {
    let mut kind = kind("");
    kind.brings_back = vec!["out".into()];
    // The handovers' folder moved aside as above, its remake's mkdir then
    // refused, failing as the real one does; or the quest's scratch gone
    // as the first file crossed, which is never made again.
    let refused = refusing(
        "mkdir",
        "*\"/handover \"*",
        "echo \"mkdir: cannot create directory '$1': Permission denied\" >&2; exit 1",
    );
    let going = "PATH=${PATH#*:} cp \"$@\" || exit\n\
         [ -e \"$HOME/swapped\" ] || \
         { mv \"$(cd ../.. && pwd)\" \"$HOME/gone\" && : > \"$HOME/swapped\"; }";
    for gone in [false, true] {
        let swapping = swapping("../../handover");
        let programs = if gone {
            vec![("cp", going)]
        } else {
            vec![("cp", swapping.as_str()), (refused.0, refused.1.as_str())]
        };
        let job = "mkdir out && printf a > out/a && printf b > out/b";
        let (code, err, home) = far_line(Left::Whole, &[("x", "x")], job, &kind, None, &programs);
        // The copies lie in the quest's scratch, as the handover does.
        let scratch = work(&home).parent().unwrap().to_path_buf();
        assert!(
            home.join("swapped").exists() && scratch.exists() != gone,
            "the far line made a quest's scratch that went again ({code:?}): {err}"
        );
        let listed = std::fs::read_to_string(handover(&home).join(BROUGHT)).unwrap_or_default();
        assert!(
            code == Some(0)
                && err.contains(UNCUT)
                && (gone || err.contains(HALTED) && !err.contains("cannot create"))
                && listed.is_empty(),
            "the far line didn't say it couldn't cut its list, or wrote one, where its \
             handover couldn't be made again (gone {gone}, {code:?}): {listed:?} {err}"
        );
        let _ = std::fs::remove_dir_all(&home);
    }
}

#[test]
fn a_link_put_back_at_the_handover_once_cleared_gets_no_cut_through_it() {
    let mut kind = kind("");
    kind.brings_back = vec!["out".into()];
    // The handovers' folder moved aside as above, then an rm that, as the
    // far line clears the job's tree once it cleared the link, puts a link
    // back at a level it would make again, or at the list's name, to a
    // folder `back` in the far run's home: the cut would go through it.
    let swapping = swapping("../../handover");
    for relinking in [
        "ln -s \"$HOME/back\" \"${j%/*/*}\"".to_owned(),
        "mkdir \"${j%/*/*}\" && ln -s \"$HOME/back\" \"${j%/*}\"".to_owned(),
        "mkdir -p \"${j%/*}\" && ln -s \"$HOME/back\" \"$j\"".to_owned(),
        format!("mkdir -p \"$j\" && ln -s \"$HOME/back/list\" \"$j/{BROUGHT}\""),
    ] {
        let rm = format!(
            "PATH=${{PATH#*:}} rm \"$@\" || exit\n\
             [ -e \"$HOME/swapped\" ] && [ ! -e \"$HOME/relinked\" ] || exit 0\n\
             j=; for a; do case \"$a\" in */{HANDOVER}/1/{TREE}) j=${{a%/*}} ;; esac; done\n\
             [ -n \"$j\" ] || exit 0\n\
             mkdir -p \"$HOME/back\" && {relinking} && : > \"$HOME/relinked\""
        );
        let programs = [("cp", swapping.as_str()), ("rm", rm.as_str())];
        let job = "mkdir out && printf a > out/a && printf b > out/b";
        let (code, err, home) = far_line(Left::Whole, &[("x", "x")], job, &kind, None, &programs);
        assert!(
            home.join("relinked").exists() && left_in(&home.join("back")).is_empty(),
            "the far line made its handover, or cut its list, through a link put back by \
             {relinking} ({code:?}): {err}"
        );
        assert!(
            code == Some(0)
                && err.contains(HALTED)
                && err.contains("a link was put back at its handover once cleared, so its list")
                && !err.contains(UNCUT),
            "the far line didn't say a link was put back by {relinking} ({code:?}): {err}"
        );
        let _ = std::fs::remove_dir_all(&home);
    }
}

#[test]
fn a_link_that_stands_at_the_handover_though_none_was_cleared_is_said_as_standing() {
    let mut kind = kind("");
    kind.brings_back = vec!["out".into()];
    // A cp that puts a link at the first copy's own name, so the far line
    // stops with no link at its handover to clear, then an rm that, as it
    // clears the job's tree, puts a link at the job's handover or at the
    // list's name, to a folder `back` in the far run's home: the cut would
    // go through it, and no link was cleared there, so none was put back.
    let planting = "PATH=${PATH#*:} cp \"$@\" || exit\n\
         [ -e \"$HOME/planted\" ] || \
         { rm -f \"$2\" && ln -s \"$HOME/target\" \"$2\" && : > \"$HOME/planted\"; }";
    for linking in [
        "mv \"$j\" \"$HOME/aside\" && ln -s \"$HOME/back\" \"$j\"".to_owned(),
        format!("rm -f \"$j/{BROUGHT}\" && ln -s \"$HOME/back/list\" \"$j/{BROUGHT}\""),
    ] {
        let rm = format!(
            "PATH=${{PATH#*:}} rm \"$@\" || exit\n\
             [ -e \"$HOME/planted\" ] && [ ! -e \"$HOME/linked\" ] || exit 0\n\
             j=; for a; do case \"$a\" in */{HANDOVER}/1/{TREE}) j=${{a%/*}} ;; esac; done\n\
             [ -n \"$j\" ] || exit 0\n\
             mkdir -p \"$HOME/back\" && {linking} && : > \"$HOME/linked\""
        );
        let programs = [("cp", planting), ("rm", rm.as_str())];
        let job = "mkdir out && printf a > out/a && printf t > \"$HOME/target\"";
        let (code, err, home) = far_line(Left::Whole, &[("x", "x")], job, &kind, None, &programs);
        assert!(
            home.join("linked").exists() && left_in(&home.join("back")).is_empty(),
            "the far line cut its list through a link put by {linking} ({code:?}): {err}"
        );
        assert!(
            code == Some(0)
                && err.contains(HALTED)
                && err.contains(STANDING)
                && !err.contains("put back at its handover")
                && !err.contains(UNCUT),
            "the far line didn't say a link put by {linking} stands ({code:?}): {err}"
        );
        let _ = std::fs::remove_dir_all(&home);
    }
}
