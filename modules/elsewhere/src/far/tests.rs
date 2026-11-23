use super::{Heard, Missed, bring_back, listed, plain_quest, quoted, relay, script, travels};
use crate::settings::{Copies, Kind};
use std::path::{Path, PathBuf};

mod crossing;
mod heard;

pub(super) fn kind(setup: &str) -> Kind {
    Kind {
        description: String::new(),
        programs: vec!["tla-rs".into()],
        setup: setup.into(),
        copies: Copies::Folder,
        excludes: vec!["/target".into()],
        brings_back: vec!["out".into()],
        needs: Vec::new(),
        lasts: None,
    }
}

#[test]
fn neither_secrets_nor_hidden_files_nor_what_s_excluded_travel() {
    let excludes = ["/states".to_owned(), "cache".to_owned()];
    for path in [
        ".env",
        ".env.local",
        "deploy/.env",
        ".git/config",
        "keys/server.key",
        "cert.pem",
        "states/1.bin",
        "a/cache/b",
        "cache",
    ] {
        assert!(!travels(path, &excludes), "{path} travelled");
    }
    for path in [
        "Spec.tla",
        "a/states/1.bin",
        "src/cached.rs",
        "keys.md",
        "lib/target/x.ts",
        "web/node_modules/a.js",
    ] {
        assert!(travels(path, &excludes), "{path} was kept home");
    }
}

#[test]
fn the_far_line_waits_for_the_files_then_runs_the_setup_and_the_job_by_name() {
    let command: Vec<String> = ["/nix/store/x-tla-rs/bin/tla-rs", "-config", "it's.cfg"]
        .map(str::to_owned)
        .into();
    let line = script(
        "QEST-20261115-ABC123",
        "7-9",
        ".e",
        Path::new("a b"),
        &kind("export PATH=/far/bin:$PATH"),
        &command,
    );
    for says in [
        "/quest-workspace/QEST-20261115-ABC123/handover/elsewhere/7-9\n",
        "work=\"${XDG_CACHE_HOME:-$HOME/.cache}\"/pi/agentic-harness.pi/quest-workspace/\
         QEST-20261115-ABC123/.e/7-9\n",
        "until arrived; do",
        "cd \"$work\"/'a b' || {\n",
        "  say 'elsewhere: refusing 7-9'\n  exit 77\n}\nsay 'elsewhere: running 7-9'\n",
        "{ export PATH=/far/bin:$PATH\n  } && exec tla-rs -config 'it'\\''s.cfg'\n",
        "for path in out; do",
        "exit \"$code\"",
    ] {
        assert!(line.contains(says), "the far line lacks {says:?}:\n{line}");
    }
    assert!(
        line.find("until arrived").unwrap() < line.find("exec tla-rs").unwrap(),
        "the job ran before its files arrived"
    );
    assert!(
        script(
            "QEST-20261115-ABC123",
            "1",
            "s",
            Path::new(""),
            &kind(""),
            &command
        )
        .contains("cd \"$work\" || {\n"),
        "a job at the tree's top wasn't run there plainly"
    );
    assert!(
        script(
            "QEST-20261115-ABC123",
            "1",
            "s",
            Path::new(""),
            &kind(""),
            &command
        )
        .contains("}\nsay 'elsewhere: running 1'\n(\n  { :\n"),
        "a job at the tree's top or with no setup wasn't run there plainly"
    );
}

#[test]
fn only_what_the_kind_brings_back_comes_home_once_listed_whole() {
    let back = ["out".to_owned()];
    assert_eq!(
        listed("2\tout/n\n", &back),
        None,
        "a list not yet whole was taken"
    );
    assert_eq!(
        listed(
            "2\tout/n\n3\tout/../../.bashrc\n4\tsrc/x\n5\toutside\n6\tout/.hidden\nend\n",
            &back
        ),
        Some(vec![super::Brought {
            number: 0,
            path: "out/n".to_owned(),
            size: 2
        }]),
        "something outside what the kind brings back was taken"
    );
}

#[test]
fn a_quest_names_a_folder_and_nothing_more() {
    assert!(
        plain_quest("QEST-20261115-ABC123"),
        "a quest's id was refused"
    );
    for odd in ["QEST-20261115-ABC12/", "../QEST", "QEST-2026111-ABC123", ""] {
        assert!(!plain_quest(odd), "{odd:?} was taken for a quest");
    }
    assert_eq!(
        Heard::Started.line("1-2"),
        "elsewhere: running 1-2",
        "the start line moved"
    );
}

#[test]
fn a_word_is_quoted_only_when_a_shell_would_split_it() {
    assert_eq!(quoted("-workers"), "-workers", "a plain word was quoted");
    assert_eq!(quoted(""), "''", "an empty word was lost");
    assert_eq!(
        quoted("$HOME"),
        "'$HOME'",
        "a variable was left for the far shell"
    );
}

/// How long the far line waits for a job's files in these tests, in
/// seconds, rather than the five minutes a real crossing gets.
pub(super) const WAITS: u64 = 5;

/// How many files, and bytes, a share carries in these tests' far lines,
/// so a test can pass them without making ten thousand files.
pub(super) const SHARE_FILES: usize = 4;
pub(super) const SHARE_BYTES: u64 = 1000;

/// What a far run in `home` said on stdout, for the near side to hear.
pub(super) fn said(home: &Path) -> Vec<u8> {
    std::fs::read(home.join("said")).unwrap()
}

/// The folder the far line's copies run in under `home`, the `s` within
/// the quest's scratch that `far_line_in` names.
pub(super) fn work(home: &Path) -> PathBuf {
    home.join(".cache/pi/agentic-harness.pi/quest-workspace/QEST-20261115-ABC123/s")
}

/// The job's handover folder in a far run's `home`.
pub(super) fn handover(home: &Path) -> PathBuf {
    home.join(format!(
        ".cache/pi/agentic-harness.pi/quest-workspace/QEST-20261115-ABC123/{}/1",
        super::HANDOVER
    ))
}

/// What the near side left for a far line in these tests.
#[derive(Clone, Copy)]
pub(super) enum Left<'a> {
    /// The files left as the near side leaves them, in archives.
    Whole,
    /// As `Whole`, but each archive at half its size until a second has
    /// passed, as a share still writing it leaves it.
    Short,
    /// This list, with each file in the tree under its own path, as no
    /// near side leaves them, for a list that is odd or never arrives.
    Listed(&'a str),
}

/// Runs the far line for a job whose handover holds `files`, each a path
/// in the job's tree and its text, left as `list` says, in a home of its
/// own, and answers how it ended, what it said on stderr, and the home.
/// It waits only `WAITS` for the files, so a list they never match ends
/// quickly.
pub(super) fn far_run(
    list: Left,
    files: &[(&str, &str)],
    run: &str,
) -> (Option<i32>, String, PathBuf) {
    far_line(list, files, run, &kind(""), None, &[])
}

/// Runs the far line as `far_line_in` does, for a job at the tree's top.
pub(super) fn far_line(
    list: Left,
    files: &[(&str, &str)],
    run: &str,
    kind: &Kind,
    meanwhile: Option<&dyn Fn(&Path)>,
    programs: &[(&str, &str)],
) -> (Option<i32>, String, PathBuf) {
    far_line_in(Path::new(""), list, files, run, kind, meanwhile, programs)
}

/// Runs the far line as `far_run` does, for a job of `kind`, doing any
/// `meanwhile` to the job's tree a second after it starts, as a share
/// still crossing would, with each of `programs`, a name and its script,
/// ahead of the rest on its path, in the tree's `folder`. What it said
/// for the near side to hear, on stdout, is left in the home's `said`.
pub(super) fn far_line_in(
    folder: &Path,
    list: Left,
    files: &[(&str, &str)],
    run: &str,
    kind: &Kind,
    meanwhile: Option<&dyn Fn(&Path)>,
    programs: &[(&str, &str)],
) -> (Option<i32>, String, PathBuf) {
    static NEXT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let home = std::env::temp_dir().join(format!(
        "elsewhere-far-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&home);
    let job = handover(&home);
    std::fs::create_dir_all(&job).unwrap();
    let tree = job.join(super::TREE);
    let mut still_crossing = Vec::new();
    if let Left::Listed(list) = list {
        for (path, text) in files {
            let into = tree.join(path);
            std::fs::create_dir_all(into.parent().unwrap()).unwrap();
            std::fs::write(into, text).unwrap();
        }
        std::fs::write(job.join(super::LEFT), list).unwrap();
    } else {
        let source = home.join("source");
        for (path, text) in files {
            let into = source.join(path);
            std::fs::create_dir_all(into.parent().unwrap()).unwrap();
            std::fs::write(into, text).unwrap();
        }
        super::left(&source, files.iter().map(|(path, _)| *path), &job).unwrap();
        // No tree at all when no file crosses, as a share carrying only
        // files leaves it.
        let _ = std::fs::remove_dir(&tree);
        if matches!(list, Left::Short) {
            for part in std::fs::read_dir(&tree).unwrap() {
                let part = part.unwrap().path();
                let whole = std::fs::read(&part).unwrap();
                // Short of a block's end, so tar can't unpack what's
                // there as a whole archive whose close is still to come.
                std::fs::write(&part, &whole[..whole.len() / 2 - 1]).unwrap();
                still_crossing.push((part, whole));
            }
        }
    }
    let bin = home.join("bin");
    std::fs::create_dir_all(&bin).unwrap();
    for (name, text) in programs {
        std::fs::write(bin.join(name), format!("#!/bin/sh\n{text}\n")).unwrap();
        std::fs::set_permissions(
            bin.join(name),
            std::os::unix::fs::PermissionsExt::from_mode(0o755),
        )
        .unwrap();
    }
    let path = format!(
        "{}:{}",
        bin.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let command = ["sh", "-c", run].map(str::to_owned);
    let waits = format!("-ge {}", super::CROSSES_WITHIN);
    let line = script("QEST-20261115-ABC123", "1", "s", folder, kind, &command);
    assert!(line.contains(&waits), "the far line's wait moved:\n{line}");
    let line = line.replace(&waits, &format!("-ge {WAITS}"));
    let most = format!(
        "most_files={} most_bytes={}",
        super::MOST_FILES,
        super::MOST_BYTES
    );
    assert!(line.contains(&most), "the far line's totals moved:\n{line}");
    let line = line.replace(
        &most,
        &format!("most_files={SHARE_FILES} most_bytes={SHARE_BYTES}"),
    );
    // The gate names dash, the strictest sh a far machine may have.
    let sh = std::env::var("ELSEWHERE_FAR_SH").unwrap_or_else(|_| "sh".to_owned());
    let far = std::process::Command::new(sh)
        .args(["-c", &line])
        .env("HOME", &home)
        .env("PATH", path)
        .env_remove("XDG_CACHE_HOME")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    if !still_crossing.is_empty() {
        std::thread::sleep(std::time::Duration::from_secs(1));
        for (part, whole) in &still_crossing {
            std::fs::write(part, whole).unwrap();
        }
    }
    if let Some(meanwhile) = meanwhile {
        std::thread::sleep(std::time::Duration::from_secs(1));
        meanwhile(&tree);
    }
    let output = far.wait_with_output().unwrap();
    std::fs::write(home.join("said"), &output.stdout).unwrap();
    // Its own words, then what it said for the near side to hear.
    let said = [output.stderr, output.stdout].concat();
    (
        output.status.code(),
        String::from_utf8_lossy(&said).into_owned(),
        home,
    )
}

#[test]
fn what_is_left_is_its_archives_each_listed_at_its_size() {
    let root = scratch("left");
    std::fs::write(root.join("x"), "xyz").unwrap();
    let job = root.join("job");
    super::left(&root, ["x"], &job).unwrap();
    let size = std::fs::metadata(job.join(super::TREE).join("0.tar"))
        .unwrap()
        .len();
    // A header, the data's block and the two that close it.
    assert_eq!(size, 4 * 512, "one small file's archive came to {size}");
    assert_eq!(
        std::fs::read_to_string(job.join(super::LEFT)).unwrap(),
        format!("{size}\t0.tar\nend\n"),
        "an archive wasn't listed at the size it was written at"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn what_comes_back_comes_back_under_its_own_name_past_one_that_failed() {
    // The first of two folders brought back holds the one that fails,
    // so the far side's numbers skip a copy before the rest are made,
    // whatever order find walks a folder in.
    let mut kind = kind("");
    kind.brings_back = vec!["early".into(), "out".into()];
    let (code, err, home) = far_line(
        Left::Whole,
        &[("x", "x")],
        "mkdir early out && printf bb > early/b && \
         printf a > out/a && printf ccc > out/c && ln -s a out/l",
        &kind,
        None,
        // A cp that fails early/b, as a file that can't be read would,
        // under root as well.
        &[(
            "cp",
            "case $1 in early/b) exit 1 ;; esac\nPATH=${PATH#*:} exec cp \"$@\"",
        )],
    );
    assert!(
        err.contains("elsewhere: out/l is a link, so it isn't brought back"),
        "a link the job made was left without a word: {err}"
    );
    assert_eq!(code, Some(0), "the job didn't run: {err}");
    assert!(
        err.contains("elsewhere: early/b didn't come back"),
        "a file that failed to come back went unsaid: {err}"
    );
    let into = home.join("near");
    assert_eq!(
        bring_back(&handover(&home), &into, &kind.brings_back, WAITS),
        Err(Missed::Lost(
            "early/b didn't come back from the far side; the other 2 landed".to_owned()
        )),
        "the near side didn't say what never crossed"
    );
    for (name, text) in [
        ("early/b", None),
        ("out/a", Some("a")),
        ("out/c", Some("ccc")),
    ] {
        assert_eq!(
            std::fs::read_to_string(into.join(name)).ok().as_deref(),
            text,
            "{name} came back wrong, so the two sides' numbers disagree"
        );
    }
    let _ = std::fs::remove_dir_all(&home);
}

/// A folder of the test's own, named for `what`, emptied.
pub(super) fn scratch(what: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("elsewhere-{what}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    root
}

#[test]
fn what_a_share_can_t_carry_is_refused_and_the_rest_lands_at_once() {
    let home = scratch("uncarried");
    let job = handover(&home);
    std::fs::create_dir_all(job.join(super::BACK)).unwrap();
    std::fs::write(job.join(super::BACK).join("1"), "b").unwrap();
    let large = super::LARGEST_FILE + 1;
    std::fs::write(
        job.join(super::BROUGHT),
        format!(">{large}\tout/a\n1\tout/b\nend\n"),
    )
    .unwrap();
    let into = home.join("near");
    let started = std::time::Instant::now();
    let landed = bring_back(&job, &into, &["out".into()], WAITS);
    let why = match &landed {
        Err(Missed::Refused(why)) => why.clone(),
        other => panic!("a file past what a share carries wasn't refused: {other:?}"),
    };
    for says in ["out/a is", "past the", "the other 1 landed"] {
        assert!(why.contains(says), "it didn't say {says}: {why}");
    }
    assert!(
        started.elapsed() < std::time::Duration::from_secs(WAITS),
        "it waited for a file a share can't carry"
    );
    assert_eq!(
        std::fs::read_to_string(into.join("out/b")).ok().as_deref(),
        Some("b"),
        "one too large for a share stopped the rest landing"
    );
    // A far side saying more crossed than a share carries at all is
    // refused at once, rather than waited for.
    let many = "1\tout/x\n".repeat(super::MOST_FILES + 1);
    std::fs::write(job.join(super::BROUGHT), format!("{many}end\n")).unwrap();
    let landed = bring_back(&job, &into, &["out".into()], WAITS);
    assert!(
        matches!(&landed, Err(Missed::Refused(why)) if why.contains("files of")),
        "more files than a share carries weren't refused: {landed:?}"
    );
    // As are more bytes, in files each small enough.
    let many = format!("{}\tout/x\n", super::LARGEST_FILE).repeat(17);
    std::fs::write(job.join(super::BROUGHT), format!("{many}end\n")).unwrap();
    let landed = bring_back(&job, &into, &["out".into()], WAITS);
    assert!(
        matches!(&landed, Err(Missed::Refused(why)) if why.contains("files of")),
        "more bytes than a share carries weren't refused: {landed:?}"
    );
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn the_far_side_lists_what_a_share_can_t_hold_but_never_copies_it() {
    let large = super::LARGEST_FILE + 1;
    // The large one in a folder brought back first, so it comes before
    // the rest whatever order find walks a folder in.
    let mut kind = kind("");
    kind.brings_back = vec!["big".into(), "out".into()];
    let (code, err, home) = far_line(
        Left::Whole,
        &[("x", "x")],
        &format!(
            "mkdir big out && printf a > out/a && \
             dd if=/dev/zero of=big/big bs=1 count=0 seek={large} 2>/dev/null"
        ),
        &kind,
        None,
        &[],
    );
    assert_eq!(code, Some(0), "the job didn't run: {err}");
    let job = handover(&home);
    let list = std::fs::read_to_string(job.join(super::BROUGHT)).unwrap();
    assert!(
        list.contains(&format!(">{large}\tbig/big\n")),
        "what a share can't hold wasn't listed at its size: {list}"
    );
    assert_eq!(
        std::fs::read_dir(job.join(super::BACK)).unwrap().count(),
        1,
        "what a share can't hold was copied to cross: {list}"
    );
    let into = home.join("near");
    let landed = bring_back(&job, &into, &kind.brings_back, WAITS);
    assert!(
        matches!(&landed, Err(Missed::Refused(why))
            if why.contains(&format!("big/big is {large} bytes"))
                && why.contains("the other 1 landed")),
        "what a share can't hold wasn't refused beside the rest: {landed:?}"
    );
    assert_eq!(
        std::fs::read_to_string(into.join("out/a")).ok().as_deref(),
        Some("a"),
        "the rest didn't land past what a share can't hold"
    );
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn a_file_that_won_t_land_is_said_with_the_lost_and_the_rest_land() {
    let home = scratch("partial");
    let job = handover(&home);
    std::fs::create_dir_all(job.join(super::BACK)).unwrap();
    for (number, text) in ["a", "b"].iter().enumerate() {
        std::fs::write(job.join(super::BACK).join(number.to_string()), text).unwrap();
    }
    std::fs::write(
        job.join(super::BROUGHT),
        "1\tout/a/x\n1\tout/b\n-\tout/c\nend\n",
    )
    .unwrap();
    // A file where out/a/x needs a folder, so it can't land.
    let into = home.join("near");
    std::fs::create_dir_all(into.join("out")).unwrap();
    std::fs::write(into.join("out/a"), "mine").unwrap();
    let landed = bring_back(&job, &into, &["out".into()], WAITS);
    let why = match &landed {
        Err(Missed::Unlanded(why)) => why.clone(),
        other => panic!("a file that couldn't land wasn't said as such: {other:?}"),
    };
    for says in [
        "out/a/x didn't land",
        "out/c didn't come back",
        "the other 1 landed",
    ] {
        assert!(why.contains(says), "it didn't say {says}: {why}");
    }
    assert_eq!(
        std::fs::read_to_string(into.join("out/b")).ok().as_deref(),
        Some("b"),
        "one that failed to land stopped the rest"
    );
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn the_near_side_waits_for_each_file_at_its_listed_size() {
    let (code, err, home) = far_run(Left::Whole, &[("x", "x")], "mkdir out && printf a > out/a");
    assert_eq!(code, Some(0), "the job didn't run: {err}");
    let job = handover(&home);
    // The far list says out/a is longer than the copy that crossed.
    std::fs::write(job.join(super::BROUGHT), "5\tout/a\nend\n").unwrap();
    let into = home.join("near");
    let landed = bring_back(&job, &into, &["out".into()], 1);
    assert!(
        landed
            .as_ref()
            .is_err_and(|why| why.why().contains("didn't cross")),
        "a copy short of its listed size was taken: {landed:?}"
    );
    assert!(!into.join("out/a").exists(), "a short copy landed");
    // A link where the copy should be, to a file of the listed size, is
    // no copy at all.
    let secret = home.join("secret");
    std::fs::write(&secret, "12345").unwrap();
    std::fs::remove_file(job.join(super::BACK).join("0")).unwrap();
    std::os::unix::fs::symlink(&secret, job.join(super::BACK).join("0")).unwrap();
    let landed = bring_back(&job, &into, &["out".into()], 1);
    assert!(
        landed
            .as_ref()
            .is_err_and(|why| why.why().contains("didn't cross")),
        "a link in the handover was followed: {landed:?}"
    );
    assert!(!into.join("out/a").exists(), "a linked file landed");
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn past_a_file_refused_for_a_link_the_rest_lands() {
    let home = scratch("past-link");
    let job = handover(&home);
    std::fs::create_dir_all(job.join(super::BACK)).unwrap();
    // The one refused first, so the one after it is the one that lands.
    for (number, text) in ["a", "x"].iter().enumerate() {
        std::fs::write(job.join(super::BACK).join(number.to_string()), text).unwrap();
    }
    std::fs::write(job.join(super::BROUGHT), "1\tout/a\n1\tother/x\nend\n").unwrap();
    let elsewhere = home.join("elsewhere");
    let into = home.join("near");
    std::fs::create_dir_all(&elsewhere).unwrap();
    std::fs::create_dir_all(&into).unwrap();
    std::os::unix::fs::symlink(&elsewhere, into.join("out")).unwrap();
    let both = ["other".to_owned(), "out".to_owned()];
    let landed = bring_back(&job, &into, &both, WAITS);
    assert!(
        matches!(&landed, Err(Missed::Refused(why))
            if why.contains("out/a would land through the link") && why.contains("the other 1 landed")),
        "a file refused for a link stopped the rest, or went unsaid: {landed:?}"
    );
    assert_eq!(
        std::fs::read_to_string(into.join("other/x"))
            .ok()
            .as_deref(),
        Some("x"),
        "a file beside one refused for a link didn't land"
    );
    assert!(
        !elsewhere.join("a").exists(),
        "a file landed through the link"
    );
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn what_comes_back_never_lands_through_a_link_out_of_its_folder() {
    let home = scratch("links");
    // What the far side left, written here in an order of its own: a
    // line the kind doesn't bring back first, so each file's number is
    // its line in the list, then out/a ahead of out/b.
    let job = handover(&home);
    std::fs::create_dir_all(job.join(super::BACK)).unwrap();
    for (number, text) in ["x", "a", "b"].iter().enumerate() {
        std::fs::write(job.join(super::BACK).join(number.to_string()), text).unwrap();
    }
    std::fs::write(
        job.join(super::BROUGHT),
        "1\tother/x\n1\tout/a\n1\tout/b\nend\n",
    )
    .unwrap();
    let back = ["out".to_owned()];
    let elsewhere = home.join("elsewhere");
    std::fs::create_dir_all(&elsewhere).unwrap();
    // A folder linked away, and so neither file lands.
    let into = home.join("near");
    std::fs::create_dir_all(&into).unwrap();
    std::os::unix::fs::symlink(&elsewhere, into.join("out")).unwrap();
    let landed = bring_back(&job, &into, &back, WAITS);
    assert!(
        landed
            .as_ref()
            .is_err_and(|why| matches!(why, Missed::Refused(why)
                if why.contains("the link") && why.contains("out of its folder"))),
        "a file came back through a folder linked away: {landed:?}"
    );
    assert!(
        !elsewhere.join("a").exists() && !elsewhere.join("b").exists(),
        "a file came back to where a folder linked away points"
    );
    // A file's own name linked away is replaced, and what it points to
    // is left as it was.
    let into = home.join("near2");
    std::fs::create_dir_all(into.join("out")).unwrap();
    std::fs::write(elsewhere.join("b"), "theirs").unwrap();
    std::os::unix::fs::symlink(elsewhere.join("b"), into.join("out/b")).unwrap();
    assert_eq!(bring_back(&job, &into, &back, WAITS), Ok(2));
    assert_eq!(
        std::fs::read_to_string(elsewhere.join("b")).ok().as_deref(),
        Some("theirs"),
        "what came back was written through a link at its own name"
    );
    assert!(
        std::fs::symlink_metadata(into.join("out/b")).is_ok_and(|meta| meta.is_file()),
        "the link at out/b wasn't replaced by what came back"
    );
    // A link into a hidden folder, or to nothing, is refused as such.
    for (to, leads) in [(".git", "into a hidden folder"), ("gone", "nowhere")] {
        let into = home.join(format!("near-{to}"));
        std::fs::create_dir_all(into.join(".git")).unwrap();
        std::os::unix::fs::symlink(into.join(to), into.join("out")).unwrap();
        let landed = bring_back(&job, &into, &back, WAITS);
        assert!(
            landed
                .as_ref()
                .is_err_and(|why| matches!(why, Missed::Refused(why) if why.contains(leads))),
            "a link to {to} wasn't refused as leading {leads}: {landed:?}"
        );
        assert!(!into.join(".git/a").exists(), "a file landed in .git");
    }
    // A link within the folder is the person's own, and is written through.
    let into = home.join("near3");
    std::fs::create_dir_all(into.join("kept")).unwrap();
    std::os::unix::fs::symlink(into.join("kept"), into.join("out")).unwrap();
    assert_eq!(
        bring_back(&job, &into, &back, WAITS),
        Ok(2),
        "a link within the folder was refused"
    );
    for name in ["a", "b"] {
        assert_eq!(
            std::fs::read_to_string(into.join("kept").join(name))
                .ok()
                .as_deref(),
            Some(name),
            "kept/{name} didn't land through the folder's own link as its own line's file"
        );
    }
    // A file already there, hard linked to one outside, is replaced
    // rather than written through.
    let into = home.join("near4");
    std::fs::create_dir_all(into.join("out")).unwrap();
    std::fs::write(elsewhere.join("h"), "kept").unwrap();
    std::fs::hard_link(elsewhere.join("h"), into.join("out/a")).unwrap();
    assert_eq!(bring_back(&job, &into, &back, WAITS), Ok(2));
    assert_eq!(
        std::fs::read_to_string(elsewhere.join("h")).ok().as_deref(),
        Some("kept"),
        "what came back was written through a hard link out of the folder"
    );
    assert_eq!(
        std::fs::read_to_string(into.join("out/a")).ok().as_deref(),
        Some("a"),
        "what came back didn't land beside the hard link it replaced"
    );
    let mut names: Vec<_> = std::fs::read_dir(into.join("out"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    names.sort();
    assert_eq!(names, ["a", "b"], "landing left a file of its own behind");
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn the_far_line_unpacks_every_name_as_it_is_and_puts_nothing_outside() {
    // Names a share keeps home, one like the mark names once crossed
    // under, and a path past both of ustar's name fields.
    let long = format!("{}/{}", "d".repeat(160), "f".repeat(110));
    let (code, err, home) = far_run(
        Left::Whole,
        &[
            ("lib/target/x.ts", "ok"),
            ("web/node_modules/a.js", "!"),
            ("notes.elsewhere", "1"),
            (&long, "2"),
        ],
        &format!("cat lib/target/x.ts web/node_modules/a.js notes.elsewhere {long} > ../seen"),
    );
    assert_eq!(
        code,
        Some(0),
        "a job over a file under target didn't run: {err}"
    );
    assert_eq!(
        std::fs::read_to_string(work(&home).join("seen"))
            .ok()
            .as_deref(),
        Some("ok!12"),
        "a file didn't land at its own name"
    );
    let _ = std::fs::remove_dir_all(&home);
    // A list naming what isn't one of the near side's archives, each
    // there at its listed size where it can be, is refused before any
    // is unpacked.
    for named in ["../out", "/abs/out", "a/./b", "..", "a/..", ""] {
        let list = format!("2\t{named}\nend\n");
        let (code, err, home) = far_run(Left::Listed(&list), &[("out", "no")], "true");
        refused_as_no_archive(code, &err, &home, named);
    }
    for named in ["x", "0", "0.tar.gz", "a/0.tar", ".tar"] {
        let list = format!("2\t{named}\nend\n");
        let (code, err, home) = far_run(Left::Listed(&list), &[(named, "no")], "true");
        refused_as_no_archive(code, &err, &home, named);
    }
}

/// Asserts a far run in `home` ended `code` refusing a list naming
/// `named` as no archive, said so in `err`, and left nothing behind.
fn refused_as_no_archive(code: Option<i32>, err: &str, home: &Path, named: &str) {
    assert!(
        code == Some(77) && err.contains("isn't one of its archives"),
        "a list naming {named:?} wasn't refused ({code:?}): {err}"
    );
    // So --there ends refused, not as a far side mycelium never reached.
    let heard = relay(said(home).as_slice(), "1");
    assert_eq!(heard, Heard::Refused, "a far refusal wasn't said: {err}");
    assert_eq!(
        heard.unstarted().0,
        terminal_kit::Exit::Refused,
        "a far refusal read as another end"
    );
    assert!(
        !work(home).join("out").exists()
            && !work(home).join("1").exists()
            && !handover(home).join(super::TREE).exists(),
        "a refused job left a file outside its folder, or its copies, for {named:?}"
    );
    let _ = std::fs::remove_dir_all(home);
}

#[test]
fn an_archive_holding_a_link_a_pipe_or_a_shared_file_is_refused_before_the_job_runs() {
    type Made = fn(&Path);
    let odd: [(&str, Made); 3] = [
        ("a link", |made| {
            std::os::unix::fs::symlink("/etc/passwd", made.join("l")).unwrap();
        }),
        ("a pipe", |made| {
            let piped = std::process::Command::new("mkfifo")
                .arg(made.join("l"))
                .status()
                .unwrap();
            assert!(piped.success(), "mkfifo made no pipe");
        }),
        ("a file under two names", |made| {
            std::fs::write(made.join("k"), "x").unwrap();
            std::fs::hard_link(made.join("k"), made.join("l")).unwrap();
        }),
    ];
    for (what, make) in odd {
        let made = scratch("odd");
        make(&made);
        // Plain ustar, and on a Mac with no AppleDouble members, so the
        // archive holds no attributes, only headers and at most a letter:
        // all ASCII and NULs, so its text holds it.
        let archive = std::process::Command::new("tar")
            .env("COPYFILE_DISABLE", "1")
            .args(["--format", "ustar", "-cf", "-", "-C"])
            .arg(&made)
            .arg(".")
            .output()
            .unwrap();
        assert!(archive.status.success(), "tar made no archive of {what}");
        let _ = std::fs::remove_dir_all(&made);
        let text = String::from_utf8(archive.stdout).unwrap();
        let list = format!("{}\t0.tar\nend\n", text.len());
        let (code, err, home) = far_run(Left::Listed(&list), &[("0.tar", &text)], "touch ../ran");
        assert!(
            code == Some(77) && err.contains("neither a file nor a folder"),
            "a tree holding {what} wasn't refused ({code:?}): {err}"
        );
        assert!(
            !work(&home).join("ran").exists() && !work(&home).join("1").exists(),
            "a job ran over a tree holding {what}, or its copy was left"
        );
        let _ = std::fs::remove_dir_all(&home);
    }
}

#[test]
fn a_folder_closed_to_its_owner_is_opened_so_its_files_are_checked_and_taken_away() {
    let made = scratch("closed");
    std::fs::create_dir_all(made.join("d")).unwrap();
    std::fs::write(made.join("d/f"), "inside").unwrap();
    let archive = std::process::Command::new("tar")
        .env("COPYFILE_DISABLE", "1")
        .args(["--format", "ustar", "-cf", "-", "-C"])
        .arg(&made)
        .arg("d")
        .output()
        .unwrap();
    assert!(archive.status.success(), "tar made no archive");
    let _ = std::fs::remove_dir_all(&made);
    let mut bytes = archive.stdout;
    // The folder's header comes first; its mode becomes 000 and its sum
    // is made again over the header with the sum's own field as spaces.
    assert_eq!(
        &bytes[156..157],
        b"5",
        "the archive doesn't open with the folder"
    );
    bytes[100..108].copy_from_slice(b"0000000\0");
    bytes[148..156].copy_from_slice(b"        ");
    let sum: u32 = bytes[..512].iter().map(|&byte| u32::from(byte)).sum();
    bytes[148..156].copy_from_slice(format!("{sum:06o}\0 ").as_bytes());
    let text = String::from_utf8(bytes).unwrap();
    let list = format!("{}\t0.tar\nend\n", text.len());
    let (code, err, home) = far_run(Left::Listed(&list), &[("0.tar", &text)], "cat d/f > ../ran");
    assert_eq!(
        code,
        Some(0),
        "the job didn't run over a closed folder: {err}"
    );
    assert_eq!(
        std::fs::read_to_string(work(&home).join("ran")).unwrap(),
        "inside",
        "the job didn't read the closed folder's file"
    );
    assert!(
        !work(&home).join("1").exists(),
        "a closed folder's copy was left behind"
    );
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn a_link_one_archive_makes_is_refused_before_the_next_writes_through_it() {
    let outside = scratch("through-outside");
    let made = scratch("through");
    std::fs::create_dir_all(made.join("first")).unwrap();
    std::os::unix::fs::symlink(&outside, made.join("first/l")).unwrap();
    let first = std::process::Command::new("tar")
        .env("COPYFILE_DISABLE", "1")
        .args(["--format", "ustar", "-cf", "-", "-C"])
        .arg(made.join("first"))
        .arg("l")
        .output()
        .unwrap();
    assert!(first.status.success(), "tar made no archive of a link");
    let first = String::from_utf8(first.stdout).unwrap();
    std::fs::create_dir_all(made.join("second/l")).unwrap();
    std::fs::write(made.join("second/l/f"), "through").unwrap();
    super::archive::write(&made.join("second"), ["l/f"], &made, 1 << 20).unwrap();
    let second = String::from_utf8(std::fs::read(made.join("0.tar")).unwrap()).unwrap();
    let _ = std::fs::remove_dir_all(&made);
    let list = format!("{}\t0.tar\n{}\t1.tar\nend\n", first.len(), second.len());
    let (code, err, home) = far_run(
        Left::Listed(&list),
        &[("0.tar", &first), ("1.tar", &second)],
        "touch ../ran",
    );
    assert!(
        code == Some(77) && err.contains("neither a file nor a folder"),
        "a link the first archive made wasn't refused as the tree's fault ({code:?}): {err}"
    );
    assert!(
        !outside.join("f").exists(),
        "the second archive wrote through the first one's link"
    );
    assert!(
        !work(&home).join("ran").exists() && !work(&home).join("1").exists(),
        "a job ran over a tree holding a link, or its copy was left"
    );
    let _ = std::fs::remove_dir_all(&home);
    let _ = std::fs::remove_dir_all(&outside);
}

#[test]
fn an_archive_naming_a_member_outside_its_tree_is_refused_before_it_s_unpacked() {
    let made = scratch("climbing");
    std::fs::create_dir_all(made.join("in")).unwrap();
    std::fs::write(made.join("out"), "no").unwrap();
    let parts = super::archive::write(&made.join("in"), ["../out"], &made, 1 << 20).unwrap();
    assert_eq!(parts.len(), 1, "one file made other than one archive");
    let text = String::from_utf8(std::fs::read(made.join("0.tar")).unwrap()).unwrap();
    let _ = std::fs::remove_dir_all(&made);
    let list = format!("{}\t0.tar\nend\n", text.len());
    let (code, err, home) = far_run(Left::Listed(&list), &[("0.tar", &text)], "touch ../ran");
    assert!(
        code == Some(77) && err.contains("names ../out, outside its tree"),
        "an archive climbing out of its tree wasn't refused as the tree's fault ({code:?}): {err}"
    );
    assert!(
        !work(&home).join("out").exists()
            && !work(&home).join("ran").exists()
            && !work(&home).join("1").exists()
            && !work(&home).join("1.names").exists(),
        "an archive climbing out wrote outside its copy, a job ran, or its copy was left"
    );
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn a_far_line_stopped_mid_job_takes_its_copy_away() {
    // Mycelium stops a run whose asker let go with the terminate signal
    // to its whole group; here the job sends a signal to the far line
    // alone, then ends, as a job the signal reached would.
    for (signal, ends) in [("TERM", 143), ("HUP", 129), ("INT", 130), ("PIPE", 141)] {
        let (code, err, home) = far_run(
            Left::Whole,
            &[("a", "1")],
            &format!("touch ../ran && kill -{signal} $PPID"),
        );
        assert!(
            work(&home).join("ran").exists(),
            "the job never ran ({code:?}): {err}"
        );
        assert_eq!(
            code,
            Some(ends),
            "a far line stopped by {signal} ended otherwise: {err}"
        );
        assert!(
            !work(&home).join("1").exists() && !handover(&home).join(super::TREE).exists(),
            "a far line stopped by {signal} left its copy or the job's archives"
        );
        assert!(
            !handover(&home).join(super::BACK).exists(),
            "a far line stopped by {signal} brought something back"
        );
        let _ = std::fs::remove_dir_all(&home);
    }
}
