use super::crossing::refusing;
use super::sweeping::gone;
use super::{Left, far_line, far_line_in, far_run, handover, kind, said, work};
use crate::far::{Heard, relay};
use std::path::Path;
use terminal_kit::Exit;

/// A file listed that never crosses, its near side saying it sent enough
/// that the ceiling, the wait and a second for each `SLOWEST` bytes, is
/// three times the wait: only the wait from the last that crossed can end
/// these tests in time, so one that never ended them fails rather than
/// passing on the ceiling's second.
const NEVER_CROSSING: Left<'static> = Left::Sized("3\tx\nend\n", 2 * super::WAITS * super::SLOWEST);

/// What the far line says when the wait from the last that crossed ends
/// it, never the ceiling.
const NONE_CROSSING: &str = "none crossing for";

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
    let (code, err, home) = far_run(NEVER_CROSSING, &[("x", "x")], "touch ../ran");
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
        err.contains(&format!(
            "none crossing for {}s",
            super::super::CROSSES_WITHIN
        )),
        "a wait with nothing crossing wasn't said to be one: {err}"
    );
    assert!(
        !work(&home).join("ran").exists(),
        "the job ran over a file still crossing"
    );
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn the_far_wait_is_measured_by_the_clock_not_by_its_passes() {
    // A wc that takes a second, as a pass over a large tree takes long,
    // so a pass takes three and a wait that counted its passes would make
    // as many as its limit has seconds. Each call is noted, so the passes
    // are counted rather than timed, which a busy machine only lengthens.
    let started = std::time::Instant::now();
    let (code, err, home) = far_line(
        NEVER_CROSSING,
        &[("x", "x")],
        "touch ../ran",
        &kind(""),
        None,
        &[(
            "wc",
            "PATH=${PATH#*:}\necho >> \"$HOME/wc\"\nsleep 1\nexec wc \"$@\"",
        )],
    );
    let took = started.elapsed();
    assert!(
        code == Some(75) && err.contains(NONE_CROSSING),
        "a file that never crossed wasn't waited out ({code:?}): {err}"
    );
    assert!(
        took >= std::time::Duration::from_secs(super::WAITS),
        "a {}s wait lasted {took:?}, so it gave up early",
        super::WAITS
    );
    // One call for what has grown before the passes, then each pass looks
    // at the file and at what has grown, and the last ends once it has
    // seen what has grown. A wait by the clock ends in two passes of
    // three seconds, and a slower machine's in no more; one counting its
    // passes makes five, where this fails.
    let calls = std::fs::read_to_string(home.join("wc"))
        .unwrap()
        .lines()
        .count();
    let passes = calls / 2;
    assert!(
        passes < usize::try_from(super::WAITS).unwrap(),
        "a {}s wait made {passes} passes of three seconds ({calls} calls), so it counts passes",
        super::WAITS
    );
    let _ = std::fs::remove_dir_all(&home);
}

/// How far apart the files in the next test cross: inside the wait, but
/// together past it.
const A_GAP: std::time::Duration = std::time::Duration::from_secs(3);

/// What the near side says the files crossing a gap apart come to: as
/// much as sets the ceiling, the wait and a second for each `SLOWEST`
/// bytes, at three times the wait, past the second and three gaps the
/// crossing takes, so the ceiling is chosen, not left to a list still to
/// come.
const GAP_SENT: u64 = 2 * super::WAITS * super::SLOWEST;

/// Runs a far line over a tree that crosses in three steps a gap apart,
/// the last well past the wait from its start but inside it from the one
/// before, as a large tree crosses, with its list empty until all of it
/// has crossed, so only what crossed can say the wait should go on. It
/// answers whether the wait ended with all of it there, rather than
/// giving up, and why: what crossed is no archive, so it goes no further.
fn crossing_a_gap_apart(step: &dyn Fn(&Path, usize), list: &str) -> (bool, String) {
    let (code, err, home) = far_line(
        Left::Sized("", GAP_SENT),
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
        NEVER_CROSSING,
        &[("x", "x")],
        "touch ../ran",
        &kind(""),
        None,
        &[("date", "echo not a time")],
    );
    assert!(
        code == Some(75) && err.contains(NONE_CROSSING) && err.contains("waiting on x"),
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
            NEVER_CROSSING,
            &[("x", "x")],
            "touch ../ran",
            &kind(""),
            None,
            &[("date", date.as_str())],
        );
        assert!(
            code == Some(75) && err.contains(NONE_CROSSING),
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
fn a_far_clock_that_steps_back_or_stands_still_spends_the_second_a_pass_slept() {
    // Ten seconds back after its first reading, and on a second a pass
    // from there, so a wait that measured from before the step would read
    // it sixteen times, and one that spent nothing of the pass that
    // stepped, seven, rather than six. Then one standing still for twenty
    // readings before it jumps on, so a wait that spent nothing while it
    // stood reads it twenty-three times, its jump spending the
    // `A_PASS_AT_MOST` a pass may, rather than hanging for good.
    // Counted by readings, since these clocks move by them rather than by
    // the time a busy machine took.
    for (how, clock) in [
        (
            "stepped back",
            "if [ \"$n\" -eq 0 ]; then echo 1000000; else echo $((999990 + n)); fi",
        ),
        (
            "stood still",
            "if [ \"$n\" -lt 20 ]; then echo 1000000; else echo $((1000000 + n)); fi",
        ),
    ] {
        let date = format!(
            "n=$(cat \"$HOME/clock\" 2>/dev/null || echo 0)\n\
             echo $((n + 1)) > \"$HOME/clock\"\n\
             {clock}"
        );
        let (code, err, home) = far_line(
            NEVER_CROSSING,
            &[("x", "x")],
            "touch ../ran",
            &kind(""),
            None,
            &[("date", date.as_str())],
        );
        let readings = std::fs::read_to_string(home.join("clock")).unwrap_or_default();
        assert!(
            code == Some(75) && err.contains(NONE_CROSSING),
            "a file that never crossed wasn't waited out ({code:?}): {err}"
        );
        // The first, then a pass a second, however the clock moved.
        assert_eq!(
            readings.trim().parse::<u64>().ok(),
            Some(super::WAITS + 1),
            "a {}s wait over a clock that {how} read it wrongly often",
            super::WAITS
        );
        let _ = std::fs::remove_dir_all(&home);
    }
}

#[test]
fn files_crossing_too_slowly_are_given_up_on_however_steadily_they_cross() {
    // A name a second, never the listed file, so the wait from the last
    // that crossed never ends, and only the ceiling, the wait and a
    // second for each `SLOWEST` bytes the near side sent, ends the whole.
    // The list names more, so a ceiling taken from it would end later and
    // say a ceiling of its own, not the near side's.
    let sent = 3 * super::SLOWEST;
    let (code, err, home) = far_line(
        Left::Sized(&format!("{}\tx\nend\n", super::SHARE_BYTES), sent),
        &[],
        "touch ../ran",
        &kind(""),
        Some(&|tree| {
            for at in 0..2 * super::WAITS {
                let _ = std::fs::create_dir_all(tree);
                let _ = std::fs::write(tree.join(format!("trickle-{at}")), "");
                std::thread::sleep(std::time::Duration::from_secs(1));
            }
        }),
        &[],
    );
    let most = super::WAITS + sent / super::SLOWEST;
    assert!(
        code == Some(75)
            && err.contains(&format!("{sent} bytes"))
            && err.contains(&format!("within {most}s, still crossing too slowly")),
        "a trickle was waited on past its ceiling ({code:?}): {err}"
    );
    assert!(
        !work(&home).join("ran").exists(),
        "it ran without its files"
    );
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn a_far_clock_that_steps_on_mid_crossing_spends_only_a_pass_of_it() {
    // An hour on from the fourth reading, as a clock set right or a
    // machine waking from sleep, while names cross every fifth of a
    // second and the listed file lands a few seconds in: a wait that took
    // the hour as spent gives up at once. The ceiling, the wait and a
    // second for each `SLOWEST` bytes listed, is well past the most a
    // pass may spend and the seconds the crossing takes.
    let listed = 25 * super::SLOWEST;
    let date = "n=$(cat \"$HOME/clock\" 2>/dev/null || echo 0)\n\
                echo $((n + 1)) > \"$HOME/clock\"\n\
                t=$(PATH=${PATH#*:} date +%s)\n\
                if [ \"$n\" -ge 3 ]; then echo $((t + 3600)); else echo \"$t\"; fi";
    let (code, err, home) = far_line(
        Left::Listed(&format!("{listed}\tx\nend\n")),
        &[],
        "touch ../ran",
        &kind(""),
        Some(&|tree| {
            std::fs::create_dir_all(tree).unwrap();
            for at in 0..20 {
                std::fs::write(tree.join(format!("trickle-{at}")), "").unwrap();
                std::thread::sleep(std::time::Duration::from_millis(200));
            }
            std::fs::write(tree.join("x"), "x".repeat(usize::try_from(listed).unwrap())).unwrap();
        }),
        &[("date", date)],
    );
    // What crossed is no archive, so the wait ended with all of it there.
    assert!(
        code == Some(77) && err.contains("isn't one of its archives"),
        "a crossing was given up on for a clock stepping on ({code:?}): {err}"
    );
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn a_ceiling_is_never_counted_from_more_than_a_share_carries() {
    // More than the near side could ever have sent, so a sum past it
    // stays in range of the far shell's arithmetic.
    let most = format!("bytes={}\n", crate::far::MOST_BYTES);
    assert!(
        crate::far::steps::waiting(u64::MAX, "'untaken'").contains(&most),
        "bytes past what a share carries weren't held to it"
    );
}

#[test]
fn a_far_clock_that_steps_on_while_nothing_crosses_spends_only_a_pass_of_it() {
    // An hour on after its first reading, while nothing crosses, and on a
    // second a pass from there: the step spends `A_PASS_AT_MOST`, and
    // the passes after it the rest of `WAITS`, where a wait that took
    // the hour as quiet gives up at its second reading. Counted by
    // readings, as the clock moves by them, and the near side sends
    // enough that only the quiet could end it.
    let date = "n=$(cat \"$HOME/clock\" 2>/dev/null || echo 0)\n\
                echo $((n + 1)) > \"$HOME/clock\"\n\
                if [ \"$n\" -eq 0 ]; then echo 1000000; else echo $((1003600 + n)); fi";
    let (code, err, home) = far_line(
        Left::Sized("1\tx\nend\n", 10 * super::SLOWEST),
        &[],
        "touch ../ran",
        &kind(""),
        None,
        &[("date", date)],
    );
    let readings = std::fs::read_to_string(home.join("clock")).unwrap_or_default();
    assert!(
        code == Some(75) && err.contains("none crossing"),
        "a file that never crossed wasn't waited out ({code:?}): {err}"
    );
    // The first, the step, then a pass a second for what is left.
    assert_eq!(
        readings.trim().parse::<u64>().ok(),
        Some(2 + super::WAITS - super::A_PASS_AT_MOST),
        "a clock an hour on was spent as more than a pass of {}s",
        super::A_PASS_AT_MOST
    );
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn a_far_machine_that_can_t_make_the_tree_says_so_and_not_that_it_refused() {
    // Each step of making the tree failing, as a full disk, a fenced
    // scratch or a program missing would, ahead of the far machine's own
    // on its path: each program's first call, an unpacking that fails
    // once it has made a folder and closed it, and the search for what is
    // neither a file nor a folder alone, after the calls before it. The
    // unpacking closes the folder in what follows its -C wherever that
    // lies, and says it did, so the case can't stop reaching the cleanup
    // unseen.
    let (_, unpacking) = refusing(
        "tar",
        "*\" -xf \"*",
        "tar \"$@\" || exit 9; while [ $# -gt 0 ] && [ \"$1\" != -C ]; do shift; done; \
         chmod 500 \"$2/target\" && : > \"$2/../closed\"; exit 1",
    );
    let (_, searching) = refusing("find", "*\" -type l \"*", "exit 1");
    for (step, stand_in) in [
        ("mkdir", "exit 1"),
        ("tar", "exit 1"),
        ("find", "exit 1"),
        ("tar", &unpacking),
        ("find", &searching),
    ] {
        let (code, err, home) = far_line(
            Left::Whole,
            &[("target/a", "x")],
            "touch ../ran",
            &kind(""),
            None,
            &[(step, stand_in)],
        );
        let heard = relay(said(&home).as_slice(), "1");
        assert!(
            code == Some(75) && heard == Heard::Untaken,
            "a tree it couldn't make for {step} ({stand_in:?}) read as {heard:?}, ending \
             {code:?}: {err}"
        );
        assert!(
            !work(&home).join("ran").exists()
                && !work(&home).join("1").exists()
                && !work(&home).join("1.run").exists(),
            "a tree it couldn't make for {step} ({stand_in:?}) was run, or left behind with \
             its run's file"
        );
        assert!(
            stand_in != unpacking || work(&home).join("closed").exists(),
            "the unpacking stand-in never closed a folder it made: {err}"
        );
        let _ = std::fs::remove_dir_all(&home);
    }
}

#[test]
fn a_tree_left_in_several_archives_is_unpacked_from_each() {
    // Each archive's list of names is made new where the one before's
    // lay, so a second archive is never refused as already listed.
    let (code, err, home) = far_line(
        Left::Parts,
        &[("a", "a"), ("b", "b"), ("c/d", "d")],
        "cat a b c/d > ../ran",
        &kind(""),
        None,
        &[],
    );
    let list = std::fs::read_to_string(handover(&home).join(crate::far::LEFT)).unwrap_or_default();
    assert_eq!(
        list.matches(".tar\n").count(),
        3,
        "the files weren't left in an archive each: {list:?}"
    );
    assert_eq!(code, Some(0), "a tree left in parts wasn't taken: {err}");
    assert_eq!(
        std::fs::read_to_string(work(&home).join("ran"))
            .ok()
            .as_deref(),
        Some("abd"),
        "a file of a later archive wasn't unpacked: {err}"
    );
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn a_run_s_file_another_run_holds_is_left_with_its_copy_and_tree_to_it() {
    // Another run of the job has written its run's file, its copy and its
    // names by the time this one sweeps, so this line's noclobber write
    // loses whenever the other's came. Its file names this machine and
    // process space and a process still running, the test's own, so the
    // sweep keeps it as a live run's rather than for naming none. A gone
    // run's file beside it, alike but for its process, must be taken, so
    // the sweep is known to have judged them by their processes rather
    // than passing both by, as where it can't tell its process space.
    let theirs = std::process::id();
    let ended = gone();
    let (step, winning) = refusing(
        "uname",
        "*",
        &format!(
            "runs=\"$HOME/.cache/pi/agentic-harness.pi/quest-workspace/QEST-20261115-ABC123/.s\"
             if mkdir \"$HOME/planting\" 2>/dev/null; then mkdir -p \"$runs/1\" && \
             echo {theirs} \"$(uname -n)\" \"$(readlink /proc/self/ns/pid 2>/dev/null)\" \
             > \"$runs/1.run\" && : > \"$runs/1/theirs\" && : > \"$runs/1.names\" && \
             mkdir \"$runs/2\" && \
             sed 's/^{theirs} /{ended} /' \"$runs/1.run\" > \"$runs/2.run\" && \
             : > \"$HOME/planted\"; fi"
        ),
    );
    let (code, err, home) = far_line(
        Left::Whole,
        &[("a", "x")],
        "touch ../ran",
        &kind(""),
        None,
        &[(step, &winning)],
    );
    // First, so neither a win nor the gone run's absence below is read
    // while its planting never got that far.
    assert!(
        home.join("planted").exists(),
        "the two runs weren't planted: {err}"
    );
    let heard = relay(said(&home).as_slice(), "1");
    assert!(
        code == Some(75) && heard == Heard::Untaken,
        "a run that lost its run's file read as {heard:?}, ending {code:?}: {err}"
    );
    let runs = work(&home);
    let file = std::fs::read_to_string(runs.join("1.run")).unwrap_or_default();
    assert!(
        file.starts_with(&format!("{theirs} ")),
        "the winner's run's file was cleared or written over, reading {file:?}: {err}"
    );
    assert!(
        runs.join("1/theirs").exists() && runs.join("1.names").exists(),
        "the winner's copy or names were cleared: {err}"
    );
    assert!(
        !runs.join("2").exists() && !runs.join("2.run").exists(),
        "the sweep passed a gone run by, so it never judged the winner by its process: {err}"
    );
    assert!(
        handover(&home).join(crate::far::TREE).exists(),
        "the job's tree, which the winner unpacks, was cleared: {err}"
    );
    assert!(
        !runs.join("ran").exists(),
        "the job ran though it lost: {err}"
    );
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn a_far_line_stopped_before_its_run_s_file_leaves_another_run_s_copy() {
    // Another run of the job writes its run's file, its copy and its names
    // while this one waits on its files, and this one is stopped there,
    // before it wrote a run's file of its own.
    let (step, stopping) = refusing(
        "sleep",
        "*",
        "runs=\"$HOME/.cache/pi/agentic-harness.pi/quest-workspace/QEST-20261115-ABC123/.s\"
         if mkdir \"$HOME/planting\" 2>/dev/null; then mkdir -p \"$runs/1\" && \
         echo theirs > \"$runs/1.run\" && : > \"$runs/1/theirs\" && : > \"$runs/1.names\" && \
         kill -TERM \"$PPID\"; fi",
    );
    let (code, err, home) = far_line(
        NEVER_CROSSING,
        &[("x", "x")],
        "touch ../ran",
        &kind(""),
        None,
        &[(step, &stopping)],
    );
    let died_by = std::fs::read_to_string(home.join("signal")).unwrap_or_default();
    assert_eq!(
        (code, died_by.as_str()),
        (None, "15"),
        "it wasn't stopped: {err}"
    );
    // Only the stop's own clearing takes the files still crossing, so a
    // line the signal killed outright leaves them.
    assert!(
        !handover(&home).join(crate::far::TREE).exists(),
        "the stop's clearing never ran: {err}"
    );
    let runs = work(&home);
    assert!(
        std::fs::read_to_string(runs.join("1.run")).ok().as_deref() == Some("theirs\n")
            && runs.join("1/theirs").exists()
            && runs.join("1.names").exists(),
        "a stop before its own run's file cleared another run's: {err}"
    );
    let _ = std::fs::remove_dir_all(&home);
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
        !work(&home).join("1").exists() && !work(&home).join("1.run").exists(),
        "a refused job's copy of its tree, or its run's file, was left behind"
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
