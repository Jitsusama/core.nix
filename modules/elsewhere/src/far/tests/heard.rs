use super::{Left, far_line, far_line_in, far_run, kind, said, work};
use crate::far::{Heard, relay};
use std::path::Path;
use terminal_kit::Exit;

#[test]
fn the_far_line_waits_while_a_file_is_short_of_its_listed_size() {
    // An archive the share is still writing is there, but short, until
    // the share finishes it.
    let (code, err, home) = far_line(
        Left::Short,
        &[("x", "xyz")],
        "cat x > ../ran",
        &kind(""),
        None,
        &[],
    );
    assert_eq!(
        code,
        Some(0),
        "a file finished crossing wasn't taken: {err}"
    );
    assert_eq!(
        std::fs::read_to_string(work(&home).join("ran"))
            .ok()
            .as_deref(),
        Some("xyz"),
        "the job ran before its file was whole"
    );
    let _ = std::fs::remove_dir_all(&home);
    // One the share never finishes ends the wait with nothing run.
    let (code, err, home) = far_run(Left::Listed("3\tx\nend\n"), &[("x", "x")], "touch ../ran");
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
        !work(&home).join("ran").exists(),
        "the job ran over a file still crossing"
    );
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn the_far_wait_is_measured_by_the_clock_not_by_its_passes() {
    // A wc that takes two seconds, as a pass over a large tree takes
    // long, so a wait that counted its passes would last five times as
    // long as its limit says.
    let started = std::time::Instant::now();
    let (code, err, home) = far_line(
        Left::Listed("3\tx\nend\n"),
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
    // Its limit, a last pass begun just inside it of two wc calls, one
    // for the file and one for what has grown, and the second slept after
    // them, and two for a busy machine. Counted passes, five seconds each,
    // would take twenty-five.
    let most = std::time::Duration::from_secs(super::WAITS + 2 + 2 + 1 + 2);
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

/// Runs a far line over a tree that crosses in three steps a gap apart,
/// the last well past the wait from its start but inside it from the one
/// before, as a large tree crosses, with its list empty until all of it
/// has crossed, so only what crossed can say the wait should go on. It
/// answers whether the wait ended with all of it there, rather than
/// giving up, and why: what crossed is no archive, so it goes no further.
fn crossing_a_gap_apart(step: &dyn Fn(&Path, usize), list: &str) -> (bool, String) {
    let (code, err, home) = far_line(
        Left::Listed(""),
        &[],
        "touch ../ran",
        &kind(""),
        Some(&|tree| {
            std::fs::create_dir_all(tree).unwrap();
            for at in 0..3 {
                step(tree, at);
                std::thread::sleep(A_GAP);
            }
            let left = tree.parent().unwrap().join(super::super::LEFT);
            std::fs::write(&left, list).unwrap();
        }),
        &[],
    );
    let arrived = code == Some(77) && err.contains("isn't one of its archives");
    let _ = std::fs::remove_dir_all(&home);
    (arrived, format!("({code:?}): {err}"))
}

#[test]
fn the_far_wait_runs_from_the_last_file_that_crossed() {
    let (ran, why) = crossing_a_gap_apart(
        &|tree, at| std::fs::write(tree.join(["x", "y", "z"][at]), "1").unwrap(),
        "1\tx\n1\ty\n1\tz\nend\n",
    );
    assert!(ran, "files still crossing were given up on {why}");
}

#[test]
fn the_far_wait_runs_from_the_last_of_a_file_that_crossed() {
    // One file growing, as a large one crosses, so only its size moves.
    let (ran, why) = crossing_a_gap_apart(
        &|tree, _| {
            let mut file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(tree.join("x"))
                .unwrap();
            let mut piece = vec![0; A_PIECE];
            let mut noise = std::fs::File::open("/dev/urandom").unwrap();
            std::io::Read::read_exact(&mut noise, &mut piece).unwrap();
            std::io::Write::write_all(&mut file, &piece).unwrap();
        },
        &format!("{}\tx\nend\n", 3 * A_PIECE),
    );
    assert!(ran, "a file still crossing was given up on {why}");
}

/// What a file still crossing grows by in the test above, of noise a
/// filesystem that compresses can't squeeze small: whole blocks, as
/// the far side counts what crossed by its kilobytes.
const A_PIECE: usize = 64 << 10;

#[test]
fn the_far_wait_runs_from_the_last_empty_file_that_crossed() {
    // Files with nothing in them, so only their names say they crossed.
    let (ran, why) = crossing_a_gap_apart(
        &|tree, at| std::fs::write(tree.join(["x", "y", "z"][at]), "").unwrap(),
        "0\tx\n0\ty\n0\tz\nend\n",
    );
    assert!(ran, "empty files still crossing were given up on {why}");
}

#[test]
fn a_far_date_that_can_t_say_the_time_still_ends_the_wait() {
    // One that answers in words, which arithmetic can't read, so the wait
    // falls back to counting its passes rather than dying unsaid, or
    // ending at once.
    let started = std::time::Instant::now();
    let (code, err, home) = far_line(
        Left::Listed("3\tx\nend\n"),
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
    assert!(
        started.elapsed() >= std::time::Duration::from_secs(super::WAITS),
        "a far date with no time gave up before its passes were counted"
    );
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn a_far_date_that_loses_readings_keeps_the_wait_on_the_clock() {
    // Readings lost, as a date can fail on a busy machine, from a clock
    // some seconds on at each reading, and how many readings the wait
    // takes to end. The first two, before the wait has a time, whose
    // passes carry over to the clock with the one that found it: one that
    // forgot either takes seven, and one that chose its clock once reads
    // it once. The same two on a clock two on a reading catch one that
    // kept counting while it read the clock, which takes six. One inside
    // it, on a clock two on a reading, which costs the clock nothing: one
    // that went back to counting takes five. Five in a row inside it,
    // each a pass: one that let a lost pass go uncounted takes seven.
    assert_eq!(super::WAITS, 5, "these counts are worked out for a 5s wait");
    let cases = [
        ("0 1", 1, 6),
        ("0 1", 2, 5),
        ("1", 2, 4),
        ("1 2 3 4 5", 1, 6),
    ];
    for (lost, step, readings) in cases {
        let date = format!(
            "n=$(cat \"$HOME/clock\" 2>/dev/null || echo 0)\n\
             echo $((n + 1)) > \"$HOME/clock\"\n\
             case \" {lost} \" in *\" $n \"*) exit 1 ;; esac\n\
             echo $((1000000 + {step} * n))"
        );
        let (code, err, home) = far_line(
            Left::Listed("3\tx\nend\n"),
            &[("x", "x")],
            "touch ../ran",
            &kind(""),
            None,
            &[("date", date.as_str())],
        );
        assert!(
            code == Some(75) && err.contains("didn't reach"),
            "a file that never crossed wasn't waited out ({code:?}): {err}"
        );
        let read = std::fs::read_to_string(home.join("clock")).unwrap_or_default();
        assert_eq!(
            read.trim().parse::<u64>().ok(),
            Some(readings),
            "a wait whose date failed at readings {lost}, {step}s apart, read it wrongly often"
        );
        let _ = std::fs::remove_dir_all(&home);
    }
}

#[test]
fn a_far_clock_that_steps_back_restarts_the_wait() {
    // Ten seconds back after its first reading, and on a second a pass
    // from there, so a wait that measured from before the step would read
    // it sixteen times rather than seven. Counted by its readings, since
    // this clock moves by them rather than by the time a busy machine took.
    let date = "n=$(cat \"$HOME/clock\" 2>/dev/null || echo 0)\n\
                echo $((n + 1)) > \"$HOME/clock\"\n\
                if [ \"$n\" -eq 0 ]; then echo 1000000; else echo $((999990 + n)); fi";
    let (code, err, home) = far_line(
        Left::Listed("3\tx\nend\n"),
        &[("x", "x")],
        "touch ../ran",
        &kind(""),
        None,
        &[("date", date)],
    );
    let readings = std::fs::read_to_string(home.join("clock")).unwrap_or_default();
    assert!(
        code == Some(75) && err.contains("didn't reach"),
        "a file that never crossed wasn't waited out ({code:?}): {err}"
    );
    // The first, the one stepped back, and the wait's own from there.
    assert_eq!(
        readings.trim().parse::<u64>().ok(),
        Some(super::WAITS + 2),
        "a {}s wait over a clock that stepped back read it wrongly often",
        super::WAITS
    );
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn files_crossing_too_slowly_are_given_up_on_however_steadily_they_cross() {
    // A name a second, never the listed file, so the wait from the last
    // that crossed never ends, and only the ceiling, the wait and a
    // second for each `SLOWEST` bytes listed, ends the whole.
    let listed = 3 * super::SLOWEST;
    let (code, err, home) = far_line(
        Left::Listed(&format!("{listed}\tx\nend\n")),
        &[],
        "touch ../ran",
        &kind(""),
        Some(&|tree| {
            for at in 0..3 * super::WAITS {
                let _ = std::fs::create_dir_all(tree);
                let _ = std::fs::write(tree.join(format!("trickle-{at}")), "");
                std::thread::sleep(std::time::Duration::from_secs(1));
            }
        }),
        &[],
    );
    let most = super::WAITS + listed / super::SLOWEST;
    assert!(
        code == Some(75) && err.contains(&format!("within {most}s, still crossing too slowly")),
        "a trickle was waited on past its ceiling ({code:?}): {err}"
    );
    assert!(
        !work(&home).join("ran").exists(),
        "it ran without its files"
    );
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn a_far_machine_that_can_t_make_the_tree_says_so_and_not_that_it_refused() {
    // Each step of making the tree failing, as a full disk, a fenced
    // scratch or a program missing would, ahead of the far machine's own
    // on its path.
    for step in ["mkdir", "tar", "find"] {
        let (code, err, home) = far_line(
            Left::Whole,
            &[("target/a", "x")],
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
            !work(&home).join("ran").exists() && !work(&home).join("1").exists(),
            "a tree it couldn't make for {step} was run, or left behind"
        );
        let _ = std::fs::remove_dir_all(&home);
    }
}

#[test]
fn a_job_s_folder_that_never_crossed_is_refused_and_said_and_swept() {
    let (code, err, home) = far_line_in(
        Path::new("drafts"),
        Left::Whole,
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
        !work(&home).join("1").exists(),
        "a refused job's copy of its tree was left behind"
    );
    let _ = std::fs::remove_dir_all(&home);
    // Nor when nothing at all travels, and no tree crosses.
    let (code, err, home) = far_line_in(
        Path::new("drafts"),
        Left::Whole,
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
    let (code, err, home) = far_run(Left::Whole, &[], "touch ../ran");
    assert!(
        code == Some(0) && work(&home).join("ran").exists(),
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
    let glued = far_line(Left::Whole, &[("x", "x")], "true", &kind(""), None, &[]);
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
