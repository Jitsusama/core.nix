use super::{Missed, bring_back, listed, script, travels, within_tree};
use crate::settings::{Copies, Kind};
use std::path::{Path, PathBuf};

mod crossing;
mod handing;
mod heard;
mod holding;
mod leaving;
mod sweeping;
mod unpacking;
mod words;

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
fn only_a_path_within_the_tree_is_read_from_it() {
    for path in [
        "/etc/passwd",
        "",
        "a//b",
        "a/",
        "../x",
        "a/../b",
        "./a",
        "a/.",
    ] {
        assert!(!within_tree(path), "{path:?} was read as within the tree");
    }
    for path in ["a", "src/lib.rs", "a..b/c", "web/node_modules/a.js"] {
        assert!(within_tree(path), "{path:?} was read as outside the tree");
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
        1 << 20,
        ".e",
        Path::new("a b"),
        &kind("export PATH=/far/bin:$PATH"),
        &command,
    );
    for says in [
        "/quest-workspace/QEST-20261115-ABC123/handover/elsewhere/7-9\n",
        "work=\"${XDG_CACHE_HOME:-$HOME/.cache}\"/pi/agentic-harness.pi/quest-workspace/\
         QEST-20261115-ABC123/.e/7-9\n",
        "bytes=1048576\n",
        "until arrived; do",
        "cd \"$work\"/'a b' || {\n",
        "}\nentered 'a b' || { moved; handed=\n",
        "  [ \"$why\" -ne 0 ] || { say 'elsewhere: refusing 7-9'; exit 77; }\n  \
         say 'elsewhere: can'\\''t take 7-9'\n  exit 75\n}\nentered 'a b' || {",
        "}\n  say 'elsewhere: can'\\''t take 7-9'; exit 75; }\nsay 'elsewhere: running 7-9'\n",
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
            0,
            ".s",
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
            0,
            ".s",
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

/// How long the far line waits for a job's files in these tests, in
/// seconds, rather than the five minutes a real crossing gets.
pub(super) const WAITS: u64 = 5;

/// The slowest a job's files may cross at in these tests, in bytes a
/// second, so a test can outlast its ceiling in seconds.
pub(super) const SLOWEST: u64 = 100;

/// The most of that wait one pass may spend in these tests, in seconds:
/// no less than the slowest pass they make, three seconds over a slow
/// `wc`, and inside `WAITS`, so a clock stepping on while nothing crosses
/// is seen to spend no more.
pub(super) const A_PASS_AT_MOST: u64 = 3;
const _: () = assert!(
    A_PASS_AT_MOST < WAITS,
    "a pass must spend less than the wait"
);

/// How many files, and bytes, a share carries in these tests' far lines,
/// so a test can pass them without making ten thousand files.
pub(super) const SHARE_FILES: usize = 4;
pub(super) const SHARE_BYTES: u64 = 1000;

/// What a far run in `home` said on stdout, for the near side to hear.
pub(super) fn said(home: &Path) -> Vec<u8> {
    std::fs::read(home.join("said")).unwrap()
}

/// The folder the far line's copies run in under `home`, the `.s` within
/// the quest's scratch that `far_line_in` names.
pub(super) fn work(home: &Path) -> PathBuf {
    home.join(".cache/pi/agentic-harness.pi/quest-workspace/QEST-20261115-ABC123/.s")
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
    /// As `Whole`, but each small file in an archive of its own.
    Parts,
    /// This list, with each file in the tree under its own path, as no
    /// near side leaves them, for a list that is odd or never arrives.
    Listed(&'a str),
    /// As `Listed`, with the near side saying the files come to these
    /// bytes, whatever the list says.
    Sized(&'a str, u64),
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
/// for the near side to hear, on stdout, is left in the home's `said`,
/// the signal it died by, if one ended it, in its `signal`, and its own
/// process's number in its `pid`.
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
    let (bytes, still_crossing) = leave(list, files, &home, &job);
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
    let line = quickened(
        &script(
            "QEST-20261115-ABC123",
            "1",
            bytes,
            ".s",
            folder,
            kind,
            &command,
        ),
        kind,
    );
    // The gate names dash, the strictest sh a far machine may have.
    let sh = std::env::var("ELSEWHERE_FAR_SH").unwrap_or_else(|_| "sh".to_owned());
    let far = std::process::Command::new(sh)
        .args(["-c", &line])
        .env("HOME", &home)
        .env("PATH", path)
        // As though inherited, so a line keeping it clears another's copy.
        .env("mine", "1")
        .env_remove("XDG_CACHE_HOME")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    std::fs::write(home.join("pid"), far.id().to_string()).unwrap();
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
    let signal = std::os::unix::process::ExitStatusExt::signal(&output.status);
    std::fs::write(
        home.join("signal"),
        signal.map(|signal| signal.to_string()).unwrap_or_default(),
    )
    .unwrap();
    // Its own words, then what it said for the near side to hear.
    let said = [output.stderr, output.stdout].concat();
    (
        output.status.code(),
        String::from_utf8_lossy(&said).into_owned(),
        home,
    )
}

/// Leaves `files` in the `job` folder of `home` as `left` says, and
/// answers what the near side says they come to (what it archived, or for
/// a list of its own, what `Sized` says, else the sizes listed that are
/// numbers, else, as for a list still to come, the most a share carries)
/// and each archive left short, with its whole, for `far_line_in` to
/// finish crossing.
fn leave(
    left: Left,
    files: &[(&str, &str)],
    home: &Path,
    job: &Path,
) -> (u64, Vec<(PathBuf, Vec<u8>)>) {
    let tree = job.join(super::TREE);
    let mut still_crossing = Vec::new();
    if let Left::Listed(list) | Left::Sized(list, _) = left {
        for (path, text) in files {
            let into = tree.join(path);
            std::fs::create_dir_all(into.parent().unwrap()).unwrap();
            std::fs::write(into, text).unwrap();
        }
        std::fs::write(job.join(super::LEFT), list).unwrap();
        let bytes: u64 = list
            .lines()
            .filter_map(|line| line.split('\t').next()?.parse::<u64>().ok())
            .sum();
        let bytes = match left {
            Left::Sized(_, bytes) => bytes,
            _ if bytes == 0 => SHARE_BYTES,
            _ => bytes,
        };
        return (bytes, still_crossing);
    }
    let source = home.join("source");
    for (path, text) in files {
        let into = source.join(path);
        std::fs::create_dir_all(into.parent().unwrap()).unwrap();
        std::fs::write(into, text).unwrap();
    }
    let paths = files.iter().map(|(path, _)| *path);
    // 4 blocks of 512 bytes: a header, one block of a file and the close's two,
    // so one file with data a part (two empty ones may share one), and one
    // past 512 bytes or needing a pax header for its name is refused.
    let bytes = match left {
        Left::Parts => super::leave(&source, paths, job, 4 * 512, super::MOST_BYTES),
        _ => super::left(&source, paths, job),
    }
    .unwrap();
    // No tree at all when no file crosses, as a share carrying only
    // files leaves it.
    let _ = std::fs::remove_dir(&tree);
    if matches!(left, Left::Short) {
        for part in std::fs::read_dir(&tree).unwrap() {
            let part = part.unwrap().path();
            let whole = std::fs::read(&part).unwrap();
            // Short of a block's end, so tar can't unpack what's
            // there as a whole archive whose close is still to come.
            std::fs::write(&part, &whole[..whole.len() / 2 - 1]).unwrap();
            still_crossing.push((part, whole));
        }
    }
    (bytes, still_crossing)
}

/// The far `line` with these tests' waits and totals for a real
/// crossing's: `WAITS`, `SLOWEST`, `A_PASS_AT_MOST`, `SHARE_FILES` and
/// `SHARE_BYTES`. Only a line for a `kind` that brings something back
/// bounds what it brings, so only that one must hold the bound.
fn quickened(line: &str, kind: &Kind) -> String {
    let mut line = line.to_owned();
    for (real, quick) in [
        (
            format!("-ge {}", super::CROSSES_WITHIN),
            format!("-ge {WAITS}"),
        ),
        (
            format!(
                "most=$(({} + bytes / {}))",
                super::CROSSES_WITHIN,
                super::CROSSES_AT
            ),
            format!("most=$(({WAITS} + bytes / {SLOWEST}))"),
        ),
        (
            format!("-gt {0} ] && step={0}", super::steps::A_PASS_SPENDS_AT_MOST),
            format!("-gt {A_PASS_AT_MOST} ] && step={A_PASS_AT_MOST}"),
        ),
        (
            format!(
                "most_files={} most_bytes={}",
                super::MOST_FILES,
                super::MOST_BYTES
            ),
            format!("most_files={SHARE_FILES} most_bytes={SHARE_BYTES}"),
        ),
    ] {
        let brings_nothing = real.starts_with("most_files=") && kind.brings_back.is_empty();
        assert!(
            brings_nothing || line.contains(&real),
            "the far line no longer holds {real}:\n{line}"
        );
        line = line.replace(&real, &quick);
    }
    line
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
