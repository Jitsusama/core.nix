use super::{SHARE_BYTES, SHARE_FILES, WAITS, far_line, handover, kind, scratch};
use crate::far::{BACK, BROUGHT, Missed, bring_back};
use std::time::{Duration, Instant};

#[test]
fn the_far_side_counts_only_what_crossed_toward_a_share_s_totals() {
    // One file a folder, so the far side meets them in the order the kind
    // names its folders, whatever order find walks one in.
    let mut kind = kind("");
    kind.brings_back = (1..=8).map(|folder| format!("f{folder}")).collect();
    let (code, err, home) = far_line(
        "1\tx\nend\n",
        &[("x", "x")],
        "for f in 1 2 3 4 5 6 7 8; do mkdir f$f; done && \
         printf '%300s' '' > f1/a && printf '%300s' '' > f2/b && \
         printf '%300s' '' > f3/c && printf '%300s' '' > f4/d && \
         printf '%200s' '' > f5/e && printf x > f6/f && printf x > f7/g",
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
    // Half of out/b crosses at the end of the wait, and the rest as long
    // again after it.
    let slow = job.join(BACK).join("1");
    let carrying = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(2500));
        std::fs::write(&slow, "bb").unwrap();
        std::thread::sleep(Duration::from_millis(2500));
        std::fs::write(&slow, "bbbb").unwrap();
    });
    let into = home.join("near");
    let landed = bring_back(&job, &into, &["out".into()], 3);
    carrying.join().unwrap();
    assert_eq!(landed, Ok(2), "a share still carrying was given up on");
    assert_eq!(
        std::fs::read_to_string(into.join("out/b")).ok().as_deref(),
        Some("bbbb"),
        "what crossed slowly didn't land whole"
    );
    let _ = std::fs::remove_dir_all(&home);
}
