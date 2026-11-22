use super::{SHARE_BYTES, SHARE_FILES, WAITS, far_line, handover, kind, scratch};
use crate::far::{BACK, BROUGHT, Brought, Missed, bring_back, landed};
use std::io::ErrorKind;
#[cfg(target_os = "linux")]
use std::path::Path;
use std::time::{Duration, Instant};
use terminal_kit::Exit;

/// How long the tests of a share still carrying give it with nothing
/// crossing, in seconds.
const A_WAIT: u64 = 4;

/// How long such a test's share takes over each step, well inside the
/// wait, so only a wait that never restarts gives up on it.
const HALF_A_WAIT: Duration = Duration::from_millis(1500);

#[test]
fn a_copy_longer_than_listed_never_lands() {
    let home = scratch("longer");
    let job = handover(&home);
    std::fs::create_dir_all(job.join(BACK)).unwrap();
    std::fs::write(job.join(BACK).join("0"), "12345").unwrap();
    std::fs::write(job.join(BROUGHT), "1\tout/a\nend\n").unwrap();
    let into = home.join("near");
    assert_eq!(
        bring_back(&job, &into, &["out".into()], 1),
        Err(Missed::Lost("out/a didn't cross home".into())),
        "a copy longer than its list said was taken"
    );
    assert!(!into.join("out/a").exists(), "a longer copy landed");
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn what_couldn_t_be_found_or_sorted_to_bring_back_is_lost_not_whole() {
    let mut kind = kind("");
    // One path named twice, which is still lost once.
    kind.brings_back = vec!["out".into(), "out/".into()];
    // Each failing only at the bring-back, having said all it would, as a
    // find stopped part way does, and passing the rest to the far
    // machine's own, the stand-in's folder first on the path.
    let (_, find) = refusing("find", "*\" -type f \"*", "find \"$@\"; exit 1");
    let (_, sort) = refusing("sort", "*\" -u \"*", "sort \"$@\"; exit 1");
    let cases: [(&str, &[(&str, &str)]); 3] = [
        ("find", &[("find", &find)]),
        ("sort", &[("sort", &sort)]),
        ("find and sort", &[("find", &find), ("sort", &sort)]),
    ];
    for (step, stand_ins) in cases {
        let (code, err, home) = far_line(
            "1\tx\nend\n",
            &[("x", "x")],
            "mkdir out && printf a > out/a",
            &kind,
            None,
            stand_ins,
        );
        assert_eq!(code, Some(0), "the job didn't run past {step}: {err}");
        let landed = bring_back(&handover(&home), &home.join("near"), &kind.brings_back, 1);
        // Named once, however many steps lost it.
        assert!(
            matches!(&landed, Err(Missed::Lost(why)) if why.starts_with("out didn't come back")),
            "a {step} that failed passed what it lost as whole, or named it twice: {landed:?}"
        );
        // What a find listed before it stopped still lands, and nothing a
        // failed sort said is trusted.
        assert_eq!(
            home.join("near/out/a").exists(),
            step == "find",
            "what the {step} listed before it failed was landed wrongly"
        );
        let _ = std::fs::remove_dir_all(&home);
    }
}

#[test]
fn a_bring_back_with_nowhere_to_list_its_names_loses_every_path_it_made() {
    let mut kind = kind("");
    kind.brings_back = vec!["out".into(), "logs".into(), "gone".into()];
    // A folder where the far line lists its names, so it can't.
    let (code, err, home) = far_line(
        "1\tx\nend\n",
        &[("x", "x")],
        "mkdir \"$PWD.back\" out logs && printf a > out/a",
        &kind,
        None,
        &[],
    );
    assert!(
        code == Some(0) && err.contains("couldn't list what to bring back"),
        "the job didn't run, or the far line didn't say why it lost all ({code:?}): {err}"
    );
    let landed = bring_back(&handover(&home), &home.join("near"), &kind.brings_back, 1);
    assert!(
        matches!(&landed, Err(Missed::Lost(why)) if why.starts_with("out, logs didn't come back")),
        "names that couldn't be listed weren't each lost, or one never made was: {landed:?}"
    );
    assert!(
        !home.join("near/out/a").exists(),
        "a file landed though its names were never listed"
    );
    let _ = std::fs::remove_dir_all(&home);
}

/// A stand-in for `step` that, run with arguments `when` matches, does
/// `refusal` instead, and otherwise passes on to the real one.
fn refusing<'a>(step: &'a str, when: &str, refusal: &str) -> (&'a str, String) {
    (
        step,
        format!(
            "PATH=${{PATH#*:}}\ncase \" $* \" in {when}) {refusal} ;; esac\nexec {step} \"$@\""
        ),
    )
}

#[test]
fn a_list_that_couldn_t_be_written_whole_ends_cut() {
    let mut kind = kind("");
    kind.brings_back = vec!["out".into()];
    // A find that fails once the far line can no longer write down what
    // it lost: its record of losses a folder, which refuses the open, and
    // where there is one, a full disk, which refuses the write. And a sort
    // after which the names, or the losses, can't be read.
    let (found, sorted) = ("*\" -type f \"*", "*\" -u \"*");
    #[cfg_attr(not(target_os = "linux"), allow(unused_mut))]
    let mut cases = vec![
        vec![refusing(
            "find",
            found,
            "rm -f \"$PWD.back.lost\" && mkdir \"$PWD.back.lost\"; exit 1",
        )],
        vec![refusing(
            "sort",
            sorted,
            "sort \"$@\"; rm -f \"$PWD.back.in\" && ln -s nowhere \"$PWD.back.in\"; exit 0",
        )],
        vec![refusing(
            "sort",
            sorted,
            "sort \"$@\"; rm -f \"$PWD.back.lost\" && ln -s nowhere \"$PWD.back.lost\"; exit 0",
        )],
    ];
    #[cfg(target_os = "linux")]
    {
        assert!(Path::new("/dev/full").exists(), "no full disk to stand in");
        cases.push(vec![
            refusing("find", found, "ln -sf /dev/full \"$PWD.back.lost\"; exit 1"),
            // A far line that wrote on past the full disk reads its losses
            // back from it next, which never ends: this ends it, so a lost
            // guard fails rather than hangs.
            refusing(
                "sort",
                sorted,
                "[ -L \"$PWD.back.lost\" ] && rm \"$PWD.back.lost\" && true > \"$PWD.back.lost\"; exec sort \"$@\"",
            ),
        ]);
    }
    let freeing = freeing_first(":");
    for stand_ins in &cases {
        let refusal = stand_ins[0].1.lines().nth(1).unwrap_or_default();
        let stand_ins: Vec<_> = stand_ins
            .iter()
            .chain([&freeing])
            .map(|(step, text)| (*step, text.as_str()))
            .collect();
        let (code, err, home) = far_line(
            "1\tx\nend\n",
            &[("x", "x")],
            "mkdir out && printf a > out/a",
            &kind,
            None,
            &stand_ins,
        );
        assert!(
            code == Some(0)
                && err.contains("the tree was freed")
                && !err.contains("before its room")
                && !err.contains("not freed with"),
            "the far line didn't end as its job did, or cut before it freed its room ({refusal}, {code:?}): {err}"
        );
        let landed = bring_back(&handover(&home), &home.join("near"), &kind.brings_back, 1);
        assert!(
            matches!(&landed, Err(Missed::Lost(why)) if why.contains("couldn't write down all")),
            "a list missing a loss it couldn't write wasn't said cut ({refusal}): {landed:?}"
        );
        assert!(
            !home.join("near/out/a").exists(),
            "a file landed from a list the far side cut ({refusal})"
        );
        let _ = std::fs::remove_dir_all(&home);
    }
}

#[cfg(target_os = "linux")]
#[test]
fn a_list_on_a_full_disk_is_never_said_whole() {
    let mut kind = kind("");
    kind.brings_back = vec!["out".into()];
    assert!(Path::new("/dev/full").exists(), "no full disk to stand in");
    // A sort that leaves the list's own name on a full disk, so every line
    // of it fails as it's written.
    let list = handover(Path::new("")).join(BROUGHT);
    let sort = refusing(
        "sort",
        "*\" -u \"*",
        &format!(
            "sort \"$@\" && ln -sf /dev/full \"$HOME/{}\"; exit",
            list.display()
        ),
    );
    let (code, err, home) = far_line(
        "1\tx\nend\n",
        &[("x", "x")],
        "mkdir out && printf a > out/a",
        &kind,
        None,
        &[(sort.0, &sort.1)],
    );
    assert!(
        code == Some(0)
            && err.contains("couldn't write down all it brings back")
            && !err.contains("couldn't end"),
        "the far line wrote on past a line it couldn't write ({code:?}): {err}"
    );
    // Nothing at the list's name but a plain file is read, so the near side
    // waits it out rather than read a device forever.
    let landed = bring_back(&handover(&home), &home.join("near"), &kind.brings_back, 1);
    assert_eq!(
        landed,
        Err(Missed::Lost("it didn't cross home within 1s".into())),
        "a list that never got written wasn't waited out"
    );
    assert!(
        !home.join("near/out/a").exists(),
        "a file landed from a list never written"
    );
    let _ = std::fs::remove_dir_all(&home);
}

/// A stand-in for the free of the job's tree, the last thing before the
/// list's end or cut, that says if either came first or the job's copy
/// isn't freed with it, and then does `then` with the job's handover
/// folder in `$job`.
fn freeing_first(then: &str) -> (&'static str, String) {
    refusing(
        "rm",
        &format!("*\"/{} \"*", super::super::TREE),
        &format!(
            "for a; do case $a in */{tree}) job=${{a%/{tree}}} ;; esac; done\n\
             case \" $* \" in *\" {work}/1 \"*) ;; *) echo 'the copy was not freed with its tree' >&2 ;; esac\n\
             grep -qxE 'end|cut' \"$job/{BROUGHT}\" && echo 'the list ended before its room was freed' >&2\n\
             rm \"$@\" && echo 'the tree was freed' >&2; {then}; exit 0",
            tree = super::super::TREE,
            work = super::work(Path::new("$HOME")).display(),
        ),
    )
}

#[cfg(target_os = "linux")]
#[test]
fn a_list_whose_end_can_t_be_written_is_said_and_ends_after_the_room_is_freed() {
    let mut kind = kind("");
    kind.brings_back = vec!["out".into()];
    assert!(Path::new("/dev/full").exists(), "no full disk to stand in");
    // Then leaves the list's name on a full disk, so only the end's write
    // fails.
    let rm = freeing_first(&format!("ln -sf /dev/full \"$job/{BROUGHT}\""));
    let (code, err, home) = far_line(
        "1\tx\nend\n",
        &[("x", "x")],
        "mkdir out && printf a > out/a",
        &kind,
        None,
        &[(rm.0, &rm.1)],
    );
    assert!(
        code == Some(0)
            && err.contains("couldn't end the list")
            && err.contains("the tree was freed")
            && !err.contains("couldn't write down all")
            && !err.contains("ended before")
            && !err.contains("not freed with"),
        "the far line didn't free its room first, or say its end failed ({code:?}): {err}"
    );
    let landed = bring_back(&handover(&home), &home.join("near"), &kind.brings_back, 1);
    assert_eq!(
        landed,
        Err(Missed::Lost("it didn't cross home within 1s".into())),
        "a list with no end wasn't waited out"
    );
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn a_list_at_a_link_is_never_read() {
    let home = scratch("linked-list");
    let job = handover(&home);
    std::fs::create_dir_all(job.join(BACK)).unwrap();
    std::fs::write(job.join(BACK).join("0"), "a").unwrap();
    std::fs::write(home.join("whole"), "1\tout/a\nend\n").unwrap();
    std::os::unix::fs::symlink(home.join("whole"), job.join(BROUGHT)).unwrap();
    assert_eq!(
        bring_back(&job, &home.join("near"), &["out".into()], 1),
        Err(Missed::Lost("it didn't cross home within 1s".into())),
        "a list at a link, which no share leaves, was read"
    );
    assert!(!home.join("near/out/a").exists(), "a linked list landed");
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn a_cut_after_half_a_line_is_cut() {
    let home = scratch("half-cut");
    let job = handover(&home);
    std::fs::create_dir_all(job.join(BACK)).unwrap();
    std::fs::write(job.join(BACK).join("0"), "a").unwrap();
    std::fs::write(job.join(BROUGHT), "1\tout/a\n1\tou\ncut\n").unwrap();
    assert_eq!(
        bring_back(&job, &home.join("near"), &["out".into()], 1),
        Err(Missed::Lost(
            "the far side couldn't write down all it would bring back".into()
        )),
        "a cut after a half-written line wasn't read as cut"
    );
    assert!(!home.join("near/out/a").exists(), "a cut list landed");
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn a_list_past_what_a_share_holds_in_one_file_is_refused() {
    let home = scratch("long-list");
    let job = handover(&home);
    std::fs::create_dir_all(&job).unwrap();
    // Sparse, so it costs the disk nothing, and twice the bound, past it
    // by more than a read's slack yet small enough that a read of all of
    // it, were the bound lost, costs a builder little.
    std::fs::File::create(job.join(BROUGHT))
        .unwrap()
        .set_len(super::super::LARGEST_FILE * 2)
        .unwrap();
    let landed = bring_back(&job, &home.join("near"), &["out".into()], 1);
    assert!(
        matches!(&landed, Err(Missed::Refused(why)) if why.contains("its list of what it brings back is past")),
        "a list no share carries whole was waited on, or read: {landed:?}"
    );
    // One at the bound is read: its first line, all NULs with no size or
    // path, passed over, and its end ending it.
    let list = std::fs::File::create(job.join(BROUGHT)).unwrap();
    list.set_len(super::super::LARGEST_FILE - 5).unwrap();
    std::io::Write::write_all(
        &mut std::fs::OpenOptions::new()
            .append(true)
            .open(job.join(BROUGHT))
            .unwrap(),
        b"\nend\n",
    )
    .unwrap();
    assert_eq!(
        bring_back(&job, &home.join("near"), &["out".into()], 1),
        Ok(0),
        "a list of just what a share holds in one file was refused"
    );
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn a_file_that_changed_as_it_crossed_is_lost_not_unlanded() {
    let home = scratch("changed");
    let job = handover(&home);
    std::fs::create_dir_all(job.join(BACK)).unwrap();
    std::fs::write(job.join(BACK).join("0"), "12345").unwrap();
    // Listed whole, and then put over at another size, as a share may.
    let files = [Brought {
        number: 0,
        path: "out/a".into(),
        size: 1,
    }];
    let into = home.join("near");
    let landed = landed(&job, &into, &["out".into()], "1\tout/a\nend\n", &files, &[]);
    assert!(
        matches!(&landed, Err(Missed::Lost(why)) if why.contains("out/a didn't come home whole")),
        "a file that changed in the share was blamed on the folder: {landed:?}"
    );
    assert!(!into.join("out/a").exists(), "a changed file landed");
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn only_what_met_no_room_or_never_crossed_is_worth_another_try() {
    for (refused, lasts, unlanded, ends) in [
        (true, true, true, Exit::Refused),
        (false, true, true, Exit::Refused),
        (false, false, true, Exit::TryAgain),
        (false, false, false, Exit::TryAgain),
    ] {
        let missed = super::super::missing(refused, lasts, unlanded, String::new());
        assert_eq!(
            missed.exit(),
            ends,
            "{missed:?} from refused {refused}, lasting {lasts}, unlanded {unlanded}"
        );
    }
    assert!(
        matches!(
            super::super::missing(false, false, true, String::new()),
            Missed::Roomless(_)
        ),
        "a miss that met no room wasn't said as one"
    );
}

#[test]
fn only_a_disk_with_no_room_is_worth_another_landing() {
    for (kind, lasts) in [
        (ErrorKind::StorageFull, false),
        (ErrorKind::QuotaExceeded, false),
        (ErrorKind::ReadOnlyFilesystem, false),
        (ErrorKind::NotADirectory, true),
        (ErrorKind::PermissionDenied, true),
    ] {
        assert_eq!(
            super::super::lasts(&kind.into()),
            lasts,
            "{kind:?} read as lasting {}",
            !lasts
        );
    }
}

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
