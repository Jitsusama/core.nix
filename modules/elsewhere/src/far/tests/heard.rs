use super::{far_line, far_run, kind};
use crate::far::{Heard, relay};
use terminal_kit::Exit;

#[test]
fn the_far_line_waits_while_a_file_is_short_of_its_listed_size() {
    // A file the share is still writing is there, but short, until the
    // share finishes it.
    let (code, err, home) = far_line(
        "3\tx\nend\n",
        &[("x", "x")],
        "cat x > ../ran",
        &kind(""),
        Some(&|tree| {
            // A far line that didn't wait has taken the tree already,
            // which the assertions below say better than this write.
            let _ = std::fs::write(tree.join("x"), "xyz");
        }),
        &[],
    );
    assert_eq!(
        code,
        Some(0),
        "a file finished crossing wasn't taken: {err}"
    );
    assert_eq!(
        std::fs::read_to_string(home.join("s/ran")).ok().as_deref(),
        Some("xyz"),
        "the job ran before its file was whole"
    );
    let _ = std::fs::remove_dir_all(&home);
    // One the share never finishes ends the wait with nothing run.
    let (code, err, home) = far_run("3\tx\nend\n", &[("x", "x")], "touch ../ran");
    assert_eq!(
        code,
        Some(75),
        "a file short of its listed size was taken: {err}"
    );
    let heard = relay(err.as_bytes(), "1");
    assert_eq!(
        heard,
        Heard::Untaken,
        "files that never reached it weren't said: {err}"
    );
    assert_eq!(
        heard.unstarted().0,
        Exit::TryAgain,
        "files that never reached it read as for good"
    );
    assert!(
        err.contains("didn't reach") && err.contains("waiting on x"),
        "the wait, or the file it was for, went unsaid: {err}"
    );
    assert!(
        !home.join("s/ran").exists(),
        "the job ran over a file still crossing"
    );
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn a_far_machine_that_can_t_copy_the_tree_says_so_and_not_that_it_refused() {
    // A cp that fails, as a full disk or a fenced scratch would, ahead of
    // the far machine's own on its path.
    let (code, err, home) = far_line(
        "1\tx\nend\n",
        &[("x", "x")],
        "touch ../ran",
        &kind(""),
        None,
        &[("cp", "exit 1")],
    );
    let heard = relay(err.as_bytes(), "1");
    assert!(
        code == Some(75) && heard == Heard::Untaken,
        "a tree it couldn't copy read as {heard:?}, ending {code:?}: {err}"
    );
    assert!(
        !home.join("s/ran").exists() && !home.join("s/1").exists(),
        "a tree it couldn't copy was run, or left behind"
    );
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn what_a_far_side_never_said_is_mycelium_s_whatever_it_ended() {
    // mycelium's own 75 or 77, with no word of the far line's, is
    // mycelium's, and reads as a far side it never reached.
    let heard = relay(&b"a login shell's welcome\n"[..], "1-2");
    assert_eq!(
        heard,
        Heard::Nothing,
        "a far line that said nothing was heard"
    );
    assert_eq!(
        heard.unstarted().0,
        Exit::Unavailable,
        "a far side that said nothing wasn't taken for unreached"
    );
    // Another job's words are not this one's.
    assert_eq!(
        relay(
            format!("{}\n", Heard::Refused.line("1-3")).as_bytes(),
            "1-2"
        ),
        Heard::Nothing,
        "another job's refusal was heard as this one's"
    );
}
