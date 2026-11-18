use super::{Missed, bring_back, crossing, listed, plain_quest, quoted, script, started, travels};
use crate::settings::{Copies, Kind};
use std::path::{Path, PathBuf};

fn kind(setup: &str) -> Kind {
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
        ".cache/e/",
        Path::new("a b"),
        &kind("export PATH=/far/bin:$PATH"),
        &command,
    );
    for says in [
        "/quest-workspace/QEST-20261115-ABC123/handover/elsewhere/7-9\n",
        "work=\"$HOME\"/.cache/e/7-9\n",
        "until arrived; do",
        "cd \"$work\"/'a b' || exit 70\necho 'elsewhere: running 7-9'\n",
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
        .contains("cd \"$work\" || exit 70\necho 'elsewhere: running 1'\n(\n  { :\n"),
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
        started("1-2"),
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

#[test]
fn a_name_a_share_keeps_home_crosses_marked_and_a_marked_one_twice() {
    assert_eq!(
        crossing("src/main.rs"),
        "src/main.rs",
        "a plain path changed"
    );
    assert_eq!(
        crossing("lib/target/x.ts"),
        "lib/target.elsewhere/x.ts",
        "a build-named folder crossed bare"
    );
    assert_eq!(
        crossing("web/node_modules/a.js"),
        "web/node_modules.elsewhere/a.js",
        "node_modules crossed bare"
    );
    assert_eq!(
        crossing("notes.elsewhere"),
        "notes.elsewhere.elsewhere",
        "a name already marked would lose its own mark far"
    );
    assert_eq!(
        crossing("targets/x"),
        "targets/x",
        "a name only like a build's was marked"
    );
}

/// How long the far line waits for a job's files in these tests, in
/// seconds, rather than the five minutes a real crossing gets.
const WAITS: u64 = 5;

/// The job's handover folder in a far run's `home`.
fn handover(home: &Path) -> PathBuf {
    home.join(format!(
        ".cache/pi/agentic-harness.pi/quest-workspace/QEST-20261115-ABC123/{}/1",
        super::HANDOVER
    ))
}

/// Runs the far line for a job whose handover lists `list` over `files`,
/// each a crossed path in the job's tree and its text, in a home of its
/// own, and answers how it ended, what it said on stderr, and the home.
/// It waits only `WAITS` for the files, so a list they never match ends
/// quickly.
fn far_run(list: &str, files: &[(&str, &str)], run: &str) -> (Option<i32>, String, PathBuf) {
    far_line(list, files, run, &kind(""), None, &[])
}

/// Runs the far line as `far_run` does, for a job of `kind`, doing any
/// `meanwhile` to the job's tree a second after it starts, as a share
/// still crossing would, with each of `programs`, a name and its script,
/// ahead of the rest on its path.
fn far_line(
    list: &str,
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
    std::fs::create_dir_all(job.join(super::TREE)).unwrap();
    for (path, text) in files {
        let into = job.join(super::TREE).join(path);
        std::fs::create_dir_all(into.parent().unwrap()).unwrap();
        std::fs::write(into, text).unwrap();
    }
    std::fs::write(job.join(super::LEFT), list).unwrap();
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
    let line = script(
        "QEST-20261115-ABC123",
        "1",
        "s",
        Path::new(""),
        kind,
        &command,
    );
    assert!(line.contains(&waits), "the far line's wait moved:\n{line}");
    let line = line.replace(&waits, &format!("-ge {WAITS}"));
    let far = std::process::Command::new("sh")
        .args(["-c", &line])
        .env("HOME", &home)
        .env("PATH", path)
        .env_remove("XDG_CACHE_HOME")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    if let Some(meanwhile) = meanwhile {
        std::thread::sleep(std::time::Duration::from_secs(1));
        meanwhile(&job.join(super::TREE));
    }
    let output = far.wait_with_output().unwrap();
    (
        output.status.code(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
        home,
    )
}

#[test]
fn what_is_left_is_listed_at_the_size_its_copy_came_to() {
    let root = std::env::temp_dir().join(format!("elsewhere-left-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("x"), "xyz").unwrap();
    let job = root.join("job");
    super::left(&root, ["x"], &job).unwrap();
    assert_eq!(
        std::fs::read_to_string(job.join(super::LEFT)).unwrap(),
        "3\tx\nend\n",
        "a file was listed at the size it was gathered at, not the one that crossed"
    );
    let _ = std::fs::remove_dir_all(&root);
}

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
fn what_comes_back_comes_back_under_its_own_name_past_one_that_failed() {
    // The first of two folders brought back holds the one that fails,
    // so the far side's numbers skip a copy before the rest are made,
    // whatever order find walks a folder in.
    let mut kind = kind("");
    kind.brings_back = vec!["early".into(), "out".into()];
    let (code, err, home) = far_line(
        "1\tx\nend\n",
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
fn scratch(what: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("elsewhere-{what}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    root
}

#[test]
fn only_a_plain_file_is_opened_to_land() {
    let root = scratch("opened");
    std::fs::write(root.join("file"), "a").unwrap();
    std::os::unix::fs::symlink(root.join("file"), root.join("link")).unwrap();
    let made = std::process::Command::new("mkfifo")
        .arg(root.join("pipe"))
        .status()
        .unwrap();
    assert!(made.success(), "mkfifo failed");
    assert!(
        super::opened_plain(&root.join("file")).is_ok(),
        "a plain file wasn't opened"
    );
    assert!(
        super::opened_plain(&root.join("link")).is_err(),
        "a link was opened"
    );
    // Refused at the look, before any open; one put there after the look
    // is opened without waiting, which no test here can time.
    assert!(
        super::opened_plain(&root.join("pipe")).is_err(),
        "a pipe was opened"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// The names in `folder`, sorted.
fn names(folder: &Path) -> Vec<String> {
    let mut names: Vec<_> = std::fs::read_dir(folder)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

#[test]
fn a_file_that_can_t_land_leaves_its_place_as_it_was() {
    let root = scratch("unlanded");
    std::fs::write(root.join("there"), "mine").unwrap();
    std::os::unix::fs::symlink("nowhere", root.join("from")).unwrap();
    assert!(
        super::land(&root.join("from"), &root.join("there")).is_err(),
        "a link landed"
    );
    assert_eq!(
        std::fs::read_to_string(root.join("there")).unwrap(),
        "mine",
        "a landing that failed touched what was there"
    );
    assert_eq!(
        names(&root),
        ["from", "there"],
        "a landing that failed left a name of its own behind"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_file_lands_under_a_long_name_without_its_special_bits() {
    let root = scratch("long");
    std::fs::write(root.join("from"), "a").unwrap();
    // Setuid where it may be set, which Nix's build sandbox refuses, and a
    // write for anyone either way.
    let allow = |bits| {
        std::fs::set_permissions(
            root.join("from"),
            std::os::unix::fs::PermissionsExt::from_mode(bits),
        )
    };
    allow(0o4777).or_else(|_| allow(0o777)).unwrap();
    let long = "n".repeat(250);
    super::land(&root.join("from"), &root.join(&long)).expect("a long name didn't land");
    let mode = std::os::unix::fs::PermissionsExt::mode(
        &std::fs::metadata(root.join(&long)).unwrap().permissions(),
    );
    // What the umask leaves a new file asking for every bit, so the test
    // holds whatever umask runs it.
    let probe = root.join("probe");
    std::os::unix::fs::OpenOptionsExt::mode(
        std::fs::OpenOptions::new().write(true).create_new(true),
        0o777,
    )
    .open(&probe)
    .unwrap();
    let allowed =
        std::os::unix::fs::PermissionsExt::mode(&std::fs::metadata(&probe).unwrap().permissions())
            & 0o777;
    assert_eq!(
        mode & 0o7777,
        0o775 & allowed,
        "it landed with setuid or a write for anyone, or past the umask: {mode:o}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_landing_name_a_killed_run_left_is_cleared() {
    let root = scratch("stale");
    std::fs::write(root.join("from"), "new").unwrap();
    let left = |pid: &str| root.join(format!("{}-{pid}-0", super::MARK));
    // Two left by runs killed as they landed, one of them under the id
    // this process lands under next; one a live run holds; and the
    // person's own, named like neither.
    let own = std::process::id().to_string();
    let next = super::NEXT_LANDING.load(std::sync::atomic::Ordering::Relaxed);
    let mine = root.join(format!("{}-{own}-{next}", super::MARK));
    for path in [
        left("1"),
        mine.clone(),
        left("2"),
        root.join(".elsewhere-notes"),
    ] {
        std::fs::write(path, "old").unwrap();
    }
    let held = std::fs::File::open(left("2")).unwrap();
    held.try_lock().unwrap();
    super::land(&root.join("from"), &root.join("into")).expect("a stale name stopped it");
    assert_eq!(
        std::fs::read_to_string(root.join("into")).unwrap(),
        "new",
        "what landed wasn't what came back"
    );
    assert!(
        !left("1").exists() && !mine.exists(),
        "a name a killed run left stayed: {:?}",
        names(&root)
    );
    assert!(left("2").exists(), "a live run's landing was swept");
    assert!(
        root.join(".elsewhere-notes").exists(),
        "the person's own file was swept"
    );
    drop(held);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn what_a_share_can_t_carry_fails_at_once() {
    let home = scratch("uncarried");
    let job = handover(&home);
    std::fs::create_dir_all(job.join(super::BACK)).unwrap();
    let large = super::LARGEST_FILE + 1;
    std::fs::write(job.join(super::BROUGHT), format!("{large}\tout/a\nend\n")).unwrap();
    let started = std::time::Instant::now();
    let landed = bring_back(&job, &home.join("near"), &["out".into()], WAITS);
    assert!(
        landed
            .as_ref()
            .is_err_and(|why| matches!(why, Missed::Lost(why) if why.contains("past the"))),
        "a file past what a share carries wasn't refused as such: {landed:?}"
    );
    assert!(
        started.elapsed() < std::time::Duration::from_secs(WAITS),
        "it waited for a file a share can't carry"
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
        Err(Missed::Lost(why)) => why.clone(),
        other => panic!("a file that couldn't land wasn't said as lost: {other:?}"),
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
    let (code, err, home) = far_run(
        "1\tx\nend\n",
        &[("x", "x")],
        "mkdir out && printf a > out/a",
    );
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
fn what_comes_back_never_lands_through_a_link_out_of_its_folder() {
    let home = std::env::temp_dir().join(format!("elsewhere-links-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
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
fn the_far_line_takes_the_marks_off_and_puts_nothing_outside() {
    // Marks inside marks, and one name marked once beside it marked
    // twice, each landing at its own name.
    let (code, err, home) = far_run(
        "2\tlib/target.elsewhere/x.ts\n1\tnotes.elsewhere.elsewhere\n\
         1\ta/target.elsewhere/node_modules.elsewhere/y\n\
         1\ttarget.elsewhere.elsewhere/z\n1\ttarget.elsewhere/w\nend\n",
        &[
            ("lib/target.elsewhere/x.ts", "ok"),
            ("notes.elsewhere.elsewhere", "!"),
            ("a/target.elsewhere/node_modules.elsewhere/y", "1"),
            ("target.elsewhere.elsewhere/z", "2"),
            ("target.elsewhere/w", "3"),
        ],
        "cat lib/target/x.ts notes.elsewhere a/target/node_modules/y \
         target.elsewhere/z target/w > ../seen",
    );
    assert_eq!(
        code,
        Some(0),
        "a job over a file under target didn't run: {err}"
    );
    assert_eq!(
        std::fs::read_to_string(home.join("s/seen")).ok().as_deref(),
        Some("ok!123"),
        "a marked file didn't land at its own name"
    );
    let _ = std::fs::remove_dir_all(&home);
    // A tree no near side writes, naming one path twice, is refused
    // rather than nesting one inside the other.
    let (code, err, home) = far_run(
        "1\tx/a\n1\tx.elsewhere/b\nend\n",
        &[("x/a", "1"), ("x.elsewhere/b", "2")],
        "true",
    );
    assert!(
        code == Some(70) && err.contains("names") && err.contains("twice"),
        "a tree naming x twice wasn't refused ({code:?}): {err}"
    );
    let _ = std::fs::remove_dir_all(&home);
    for named in ["../out", "/abs/out", "a/./b", "..", "a/..", ""] {
        let (code, err, home) = far_run(&format!("2\t{named}\nend\n"), &[("out", "no")], "true");
        assert!(
            code == Some(70) && err.contains("outside its folder"),
            "a list naming {named:?} wasn't refused ({code:?}): {err}"
        );
        assert!(
            !home.join("s/out").exists()
                && !home.join("s/1").exists()
                && !handover(&home).join(super::TREE).exists(),
            "a refused job left a file outside its folder, or its copies, for {named:?}"
        );
        let _ = std::fs::remove_dir_all(&home);
    }
    // A name that is only a dot or two once its mark comes off.
    for named in ["..elsewhere", "...elsewhere"] {
        let (code, err, home) = far_run(&format!("1\t{named}\nend\n"), &[(named, "x")], "true");
        assert!(
            code == Some(70) && err.contains(&format!("/{named}, outside its folder")),
            "a tree naming {named} wasn't refused ({code:?}): {err}"
        );
        let _ = std::fs::remove_dir_all(&home);
    }
}
