use super::{far_line, far_line_in, far_run, kind, said};
use crate::far::{Heard, relay};
use std::path::Path;
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
    let heard = relay(said(&home).as_slice(), "1");
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
fn the_far_wait_is_measured_by_the_clock_not_by_its_passes() {
    // A wc that takes two seconds, as a pass over a large tree takes
    // long, so a wait that counted its passes would last three times as
    // long as its limit says.
    let started = std::time::Instant::now();
    let (code, err, home) = far_line(
        "3\tx\nend\n",
        &[("x", "x")],
        "touch ../ran",
        &kind(""),
        None,
        &[("wc", "PATH=${PATH#*:}\nsleep 2\nexec wc \"$@\"")],
    );
    let took = started.elapsed();
    assert!(
        code == Some(75) && err.contains("didn't reach"),
        "a file that never crossed wasn't waited out ({code:?}): {err}"
    );
    // Its limit, a last pass of two seconds begun just inside it, the
    // second slept after that, and two for a busy machine. Counted passes,
    // three seconds each, would take fifteen and more.
    let most = std::time::Duration::from_secs(super::WAITS + 2 + 1 + 2);
    assert!(
        took >= std::time::Duration::from_secs(super::WAITS) && took < most,
        "a {}s wait lasted {took:?}, so it gave up early or counts passes",
        super::WAITS
    );
    let _ = std::fs::remove_dir_all(&home);
}

/// How far apart the files in the next test cross: inside the wait, but
/// together past it.
const A_GAP: std::time::Duration = std::time::Duration::from_secs(3);

#[test]
fn the_far_wait_runs_from_the_last_file_that_crossed() {
    // Three files a gap apart, the last well past the wait from its start
    // but inside it from the one before, as a large tree crosses.
    let (code, err, home) = far_line(
        "1\tx\n1\ty\n1\tz\nend\n",
        &[],
        "cat x y z > ../ran",
        &kind(""),
        Some(&|tree| {
            std::fs::create_dir_all(tree).unwrap();
            for name in ["x", "y", "z"] {
                std::fs::write(tree.join(name), name).unwrap();
                std::thread::sleep(A_GAP);
            }
        }),
        &[],
    );
    assert!(
        code == Some(0)
            && std::fs::read_to_string(home.join("s/ran")).ok().as_deref() == Some("xyz"),
        "files still crossing were given up on ({code:?}): {err}"
    );
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn a_far_date_that_can_t_say_the_time_still_ends_the_wait() {
    // One that answers in words, which arithmetic can't read, so the wait
    // falls back to counting its passes rather than dying unsaid.
    let (code, err, home) = far_line(
        "3\tx\nend\n",
        &[("x", "x")],
        "touch ../ran",
        &kind(""),
        None,
        &[("date", "echo not a time")],
    );
    assert!(
        code == Some(75) && err.contains("didn't reach") && err.contains("waiting on x"),
        "a far date with no time ended the line unsaid ({code:?}): {err}"
    );
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn a_far_machine_that_can_t_make_the_tree_says_so_and_not_that_it_refused() {
    // Each step of making the tree failing, as a full disk, a fenced
    // scratch or a program missing would, ahead of the far machine's own
    // on its path, over a tree with a mark to take off.
    for step in ["cp", "mkdir", "mv", "find", "awk", "sort", "cut"] {
        let (code, err, home) = far_line(
            "1\ttarget.elsewhere/a\nend\n",
            &[("target.elsewhere/a", "x")],
            "touch ../ran",
            &kind(""),
            None,
            &[(step, "exit 1")],
        );
        let heard = relay(said(&home).as_slice(), "1");
        assert!(
            code == Some(75) && heard == Heard::Untaken,
            "a tree it couldn't make for {step} read as {heard:?}, ending {code:?}: {err}"
        );
        assert!(
            !home.join("s/ran").exists() && !home.join("s/1").exists(),
            "a tree it couldn't make for {step} was run, or left behind"
        );
        let _ = std::fs::remove_dir_all(&home);
    }
}

#[test]
fn a_job_s_folder_that_never_crossed_is_refused_and_said_and_swept() {
    let (code, err, home) = far_line_in(
        Path::new("drafts"),
        "1\tx\nend\n",
        &[("x", "x")],
        "touch ../ran",
        &kind(""),
        None,
        &[],
    );
    let heard = relay(said(&home).as_slice(), "1");
    assert!(
        code == Some(77) && heard == Heard::Refused,
        "a folder that never crossed read as {heard:?}, ending {code:?}: {err}"
    );
    assert!(
        err.contains("holds nothing that travels"),
        "why it was refused went unsaid: {err}"
    );
    assert!(
        !home.join("s/1").exists(),
        "a refused job's copy of its tree was left behind"
    );
    let _ = std::fs::remove_dir_all(&home);
    // Nor when nothing at all travels, and no tree crosses.
    let (code, err, home) = far_line_in(
        Path::new("drafts"),
        "end\n",
        &[],
        "touch ../ran",
        &kind(""),
        None,
        &[],
    );
    assert!(
        code == Some(77) && err.contains("holds nothing that travels"),
        "a folder with nothing that crossed wasn't refused ({code:?}): {err}"
    );
    let _ = std::fs::remove_dir_all(&home);
    // At the tree's top, the job runs in a tree with nothing in it.
    let (code, err, home) = far_run("end\n", &[], "touch ../ran");
    assert!(
        code == Some(0) && home.join("s/ran").exists(),
        "a job with no file that travels didn't run ({code:?}): {err}"
    );
    let _ = std::fs::remove_dir_all(&home);
}

/// A stream handing out a byte a read, as a slow pipe may, so how far it
/// was read says how far its reader wanted it.
struct Trickle<'a>(&'a [u8], usize);

impl std::io::Read for Trickle<'_> {
    fn read(&mut self, into: &mut [u8]) -> std::io::Result<usize> {
        match (self.0.get(self.1), into.first_mut()) {
            (Some(byte), Some(first)) => {
                *first = *byte;
                self.1 += 1;
                Ok(1)
            }
            _ => Ok(0),
        }
    }
}

#[test]
fn the_far_line_s_word_is_heard_whatever_came_before_or_after_it() {
    // A banner with no newline of its own, and a terminal's line ends.
    let glued = far_line("1\tx\nend\n", &[("x", "x")], "true", &kind(""), None, &[]);
    let mut out = b"a welcome with no end".to_vec();
    out.extend(said(&glued.2));
    assert_eq!(
        relay(out.as_slice(), "1"),
        Heard::Started,
        "a start after a banner with no newline went unheard: {}",
        String::from_utf8_lossy(&out)
    );
    let _ = std::fs::remove_dir_all(&glued.2);
    assert_eq!(
        relay(&b"welcome\r\n\r\nelsewhere: refusing 1\r\n"[..], "1"),
        Heard::Refused,
        "a refusal a terminal ended went unheard"
    );
    // What follows a refusal is read to its end, so mycelium never
    // writes to a pipe nobody reads.
    let words = b"\nelsewhere: refusing 1\nmycelium's own last words\n";
    let mut out = Trickle(words, 0);
    assert_eq!(relay(&mut out, "1"), Heard::Refused);
    assert_eq!(
        out.1,
        words.len(),
        "what followed a refusal was left unread"
    );
    // Once it started, what follows is the job's, refusal or not.
    assert_eq!(
        relay(
            &b"\nelsewhere: running 1\n\nelsewhere: refusing 1\n"[..],
            "1"
        ),
        Heard::Started,
        "a job's own words unsaid that it started"
    );
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
