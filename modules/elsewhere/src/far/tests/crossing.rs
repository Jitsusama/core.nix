use super::{SHARE_BYTES, SHARE_FILES, WAITS, far_line, handover, kind, scratch};
use crate::far::{BACK, BROUGHT, Missed, bring_back};
use std::time::{Duration, Instant};

/// How long the tests of a share still carrying give it with nothing
/// crossing, in seconds.
const A_WAIT: u64 = 4;

/// How long such a test's share takes over each step, well inside the
/// wait, so only a wait that never restarts gives up on it.
const HALF_A_WAIT: Duration = Duration::from_millis(1500);

#[test]
fn the_far_side_counts_only_what_crossed_toward_a_share_s_totals() {
    // One file a folder, so the far side meets them in their names' byte
    // order, as it sorts them, whatever order find walks one in.
    let mut kind = kind("");
    kind.brings_back = (1..=8).map(|folder| format!("f{folder}")).collect();
    let (code, err, home) = far_line(
        "1\tx\nend\n",
        &[("x", "x")],
        "for f in 1 2 3 4 5 6 7 8; do mkdir f$f; done && \
         printf '%300s' '' > f1/a && printf '%300s' '' > f2/b && \
         printf '%300s' '' > f3/c && printf '%300s' '' > f4/d && \
         printf '%101s' '' > f5/e && printf '%100s' '' > f6/f && printf x > f7/g",
        &kind,
        None,
        &[
            // A cp that fails f2/b, which mustn't count toward the totals.
            (
                "cp",
                "case $1 in f2/b) exit 1 ;; esac\nPATH=${PATH#*:} exec cp \"$@\"",
            ),
            // A find that names a file gone before it's read.
            (
                "find",
                "PATH=${PATH#*:} find \"$@\"\n\
                 [ \"$1 $2 $3\" = 'f8 -type f' ] && echo f8/ghost\nexit 0",
            ),
        ],
    );
    assert_eq!(code, Some(0), "the job didn't run: {err}");
    assert_eq!(SHARE_FILES, 4, "the counts below are for four files");
    assert_eq!(SHARE_BYTES, 1000, "the counts below are for 1000 bytes");
    let job = handover(&home);
    let list = std::fs::read_to_string(job.join(BROUGHT)).unwrap();
    let into = home.join("near");
    let started = Instant::now();
    let landed = bring_back(&job, &into, &kind.brings_back, WAITS);
    let why = match &landed {
        Err(Missed::Refused(why)) => why.clone(),
        other => panic!("what passed the totals wasn't refused: {other:?}\n{list}"),
    };
    assert!(
        started.elapsed() < Duration::from_secs(WAITS),
        "it waited for what the far side kept: {list}"
    );
    for says in [
        "f5/e is past",
        "f7/g is past",
        "f2/b, f8/ghost didn't come back",
        "the other 4 landed",
    ] {
        assert!(why.contains(says), "it didn't say {says}: {why}\n{list}");
    }
    for (name, lands) in [
        ("f1/a", true),
        ("f2/b", false),
        ("f3/c", true),
        ("f4/d", true),
        ("f5/e", false),
        ("f6/f", true),
        ("f7/g", false),
    ] {
        assert_eq!(
            into.join(name).exists(),
            lands,
            "{name} landed wrong, so the sides' totals disagree: {list}"
        );
    }
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn what_crossed_lands_when_the_rest_never_does() {
    let home = scratch("straggler");
    let job = handover(&home);
    std::fs::create_dir_all(job.join(BACK)).unwrap();
    std::fs::write(job.join(BACK).join("0"), "a").unwrap();
    std::fs::write(job.join(BROUGHT), "1\tout/a\n1\tout/b\nend\n").unwrap();
    let into = home.join("near");
    assert_eq!(
        bring_back(&job, &into, &["out".into()], 1),
        Err(Missed::Lost(
            "out/b didn't cross home; the other 1 landed".to_owned()
        )),
        "a file that never crossed wasn't said beside the rest"
    );
    assert_eq!(
        std::fs::read_to_string(into.join("out/a")).ok().as_deref(),
        Some("a"),
        "what crossed didn't land past one that never did"
    );
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn a_share_still_carrying_is_waited_for_past_the_wait() {
    let home = scratch("carrying");
    let job = handover(&home);
    std::fs::create_dir_all(job.join(BACK)).unwrap();
    std::fs::write(job.join(BACK).join("0"), "a").unwrap();
    std::fs::write(job.join(BROUGHT), "1\tout/a\n4\tout/b\nend\n").unwrap();
    // out/b crosses a byte at a time under a hidden name beside its own,
    // as mycelium writes what it receives, and is renamed over its path
    // once whole, so it ends well past the wait.
    let (back, slow) = (job.join(BACK), job.join(BACK).join(".1.crossing"));
    let carrying = std::thread::spawn(move || {
        for crossed in ["b", "bb", "bbb", "bbbb"] {
            std::thread::sleep(HALF_A_WAIT);
            std::fs::write(&slow, crossed).unwrap();
        }
        std::fs::rename(&slow, back.join("1")).unwrap();
    });
    let into = home.join("near");
    let started = Instant::now();
    let landed = bring_back(&job, &into, &["out".into()], A_WAIT);
    carrying.join().unwrap();
    assert_eq!(landed, Ok(2), "a share still carrying was given up on");
    assert!(
        started.elapsed() > Duration::from_secs(A_WAIT),
        "it ended inside the wait, so it never waited past it"
    );
    assert_eq!(
        std::fs::read_to_string(into.join("out/b")).ok().as_deref(),
        Some("bbbb"),
        "what crossed slowly didn't land whole"
    );
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn an_empty_file_listed_is_waited_for_until_it_crosses() {
    let home = scratch("empty");
    let job = handover(&home);
    std::fs::create_dir_all(job.join(BACK)).unwrap();
    std::fs::write(job.join(BROUGHT), "0\tout/e\nend\n").unwrap();
    assert_eq!(
        bring_back(&job, &home.join("near"), &["out".into()], 1),
        Err(Missed::Lost("out/e didn't cross home".to_owned())),
        "an empty file still crossing was taken for crossed"
    );
    // Empty ones crossing half a wait apart keep it going, and land.
    let list: Vec<String> = (0..4).map(|n| format!("0\tout/e{n}")).collect();
    std::fs::write(job.join(BROUGHT), format!("{}\nend\n", list.join("\n"))).unwrap();
    let back = job.join(BACK);
    let carrying = std::thread::spawn(move || {
        for n in 0..4 {
            std::thread::sleep(HALF_A_WAIT);
            std::fs::write(back.join(n.to_string()), "").unwrap();
        }
    });
    let landed = bring_back(&job, &home.join("near"), &["out".into()], A_WAIT);
    carrying.join().unwrap();
    assert_eq!(landed, Ok(4), "empty files still crossing were given up on");
    assert!(
        home.join("near/out/e3").is_file(),
        "the last empty file to cross didn't land"
    );
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn a_name_that_isn_t_utf_8_is_refused_and_the_rest_lands() {
    let home = scratch("unnamed");
    let job = handover(&home);
    std::fs::create_dir_all(job.join(BACK)).unwrap();
    std::fs::write(job.join(BACK).join("0"), "z").unwrap();
    std::fs::write(job.join(BACK).join("1"), "a").unwrap();
    std::fs::write(job.join(BROUGHT), b"1\tout/\xff\n1\tout/a\nend\n").unwrap();
    let into = home.join("near");
    let started = Instant::now();
    let landed = bring_back(&job, &into, &["out".into()], WAITS);
    assert!(
        started.elapsed() < Duration::from_secs(WAITS),
        "it waited on a list it couldn't read"
    );
    assert_eq!(
        landed,
        Err(Missed::Refused(
            "out/\u{fffd} isn't named in UTF-8, so it can't land here; the other 1 landed"
                .to_owned()
        ))
    );
    assert_eq!(
        std::fs::read_to_string(into.join("out/a")).ok().as_deref(),
        Some("a"),
        "one name that isn't UTF-8 kept the rest from landing"
    );
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn a_name_ending_in_a_carriage_return_lands_under_that_name() {
    let home = scratch("return");
    let job = handover(&home);
    std::fs::create_dir_all(job.join(BACK)).unwrap();
    std::fs::write(job.join(BACK).join("0"), "a").unwrap();
    std::fs::write(job.join(BROUGHT), "1\tout/a\r\nend\n").unwrap();
    let into = home.join("near");
    assert_eq!(bring_back(&job, &into, &["out".into()], 1), Ok(1));
    assert!(
        into.join("out/a\r").exists() && !into.join("out/a").exists(),
        "out/a\\r landed under another name"
    );
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn files_crossing_ahead_of_their_list_keep_the_wait_going() {
    let home = scratch("listless");
    let job = handover(&home);
    std::fs::create_dir_all(job.join(BACK)).unwrap();
    let (back, list) = (job.join(BACK), job.join(BROUGHT));
    // The files are there, and their list crosses after them a piece at a
    // time, half a wait apart, under a hidden name beside its own, so it
    // is renamed over its path well past the wait.
    std::fs::write(back.join("0"), "a").unwrap();
    std::fs::write(back.join("1"), "bb").unwrap();
    let crossing = job.join(".elsewhere-back.crossing");
    let carrying = std::thread::spawn(move || {
        let whole = "1\tout/a\n2\tout/b\nend\n";
        for end in [5, 10, 15, whole.len()] {
            std::thread::sleep(HALF_A_WAIT);
            std::fs::write(&crossing, &whole[..end]).unwrap();
        }
        std::thread::sleep(HALF_A_WAIT);
        std::fs::rename(&crossing, list).unwrap();
    });
    let landed = bring_back(&job, &home.join("near"), &["out".into()], A_WAIT);
    carrying.join().unwrap();
    assert_eq!(
        landed,
        Ok(2),
        "files still crossing ahead of their list were given up on"
    );
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn past_the_totals_a_handful_is_named_and_the_rest_counted() {
    let home = scratch("many");
    let job = handover(&home);
    std::fs::create_dir_all(job.join(BACK)).unwrap();
    let list: Vec<String> = (0..12).map(|n| format!(">1\tout/k{n:02}")).collect();
    // And one of another kind, named past the first kind's many.
    let list = [
        format!("{}\n1\tout/", list.join("\n")).as_bytes(),
        b"\xff\nend\n",
    ]
    .concat();
    std::fs::write(job.join(BROUGHT), list).unwrap();
    let Err(Missed::Refused(why)) = bring_back(&job, &home.join("near"), &["out".into()], 1) else {
        panic!("what passed the totals wasn't refused");
    };
    assert!(
        why.contains("out/k09 is past"),
        "the first ten weren't named: {why}"
    );
    assert!(!why.contains("out/k10"), "more than ten were named: {why}");
    assert!(
        why.contains("; 2 more like them; "),
        "the rest weren't counted: {why}"
    );
    assert!(
        why.contains("isn't named in UTF-8"),
        "another kind went unnamed behind the first: {why}"
    );
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn a_file_two_brought_back_paths_both_hold_crosses_once() {
    let mut kind = kind("");
    kind.brings_back = vec!["out".into(), "out/a".into()];
    let (code, err, home) = far_line(
        "1\tx\nend\n",
        &[("x", "x")],
        "mkdir out && printf x > out/a",
        &kind,
        None,
        &[],
    );
    assert_eq!(code, Some(0), "the job didn't run: {err}");
    assert_eq!(
        std::fs::read_to_string(handover(&home).join(BROUGHT)).unwrap(),
        "1\tout/a\nend\n",
        "a file both paths hold was listed twice"
    );
    let _ = std::fs::remove_dir_all(&home);
}
