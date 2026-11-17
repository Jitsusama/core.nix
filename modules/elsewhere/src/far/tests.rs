use super::{bring_back, crossing, listed, plain_quest, quoted, script, started, travels};
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
    far_run_while(list, files, run, |_| ())
}

/// Runs the far line as `far_run` does, doing `meanwhile` to the job's
/// tree a second after it starts, as a share still crossing would.
fn far_run_while(
    list: &str,
    files: &[(&str, &str)],
    run: &str,
    meanwhile: impl FnOnce(&Path),
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
    let command = ["sh", "-c", run].map(str::to_owned);
    let waits = format!("-ge {}", super::CROSSES_WITHIN);
    let line = script(
        "QEST-20261115-ABC123",
        "1",
        "s",
        Path::new(""),
        &kind(""),
        &command,
    );
    assert!(line.contains(&waits), "the far line's wait moved:\n{line}");
    let line = line.replace(&waits, &format!("-ge {WAITS}"));
    let far = std::process::Command::new("sh")
        .args(["-c", &line])
        .env("HOME", &home)
        .env_remove("XDG_CACHE_HOME")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    std::thread::sleep(std::time::Duration::from_secs(1));
    meanwhile(&job.join(super::TREE));
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
    // Gathered at 1 byte, then rewritten before it was left.
    std::fs::write(root.join("x"), "xyz").unwrap();
    let job = root.join("job");
    super::left(&root, &[("x".to_owned(), 1)], &job).unwrap();
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
    let (code, err, home) = far_run_while("3\tx\nend\n", &[("x", "x")], "cat x > ../ran", |tree| {
        // A far line that didn't wait has taken the tree already, which
        // the assertions below say better than this write could.
        let _ = std::fs::write(tree.join("x"), "xyz");
    });
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
    assert!(err.contains("didn't reach"), "the wait went unsaid: {err}");
    assert!(
        !home.join("s/ran").exists(),
        "the job ran over a file still crossing"
    );
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn what_comes_back_comes_back_under_its_own_name_past_one_that_failed() {
    // Three files brought back, the middle one unreadable, so the far
    // side's numbers skip a copy the near side never hears of.
    let (code, err, home) = far_run(
        "1\tx\nend\n",
        &[("x", "x")],
        "mkdir out && printf a > out/a && printf bb > out/b && \
         printf ccc > out/c && chmod 000 out/b",
    );
    assert_eq!(code, Some(0), "the job didn't run: {err}");
    assert!(
        err.contains("elsewhere: out/b didn't come back"),
        "a file that failed to come back went unsaid: {err}"
    );
    let into = home.join("near");
    assert_eq!(
        bring_back(&handover(&home), &into, &["out".into()], WAITS),
        Ok(2),
        "the near side didn't bring back the two that crossed"
    );
    for (name, text) in [("a", Some("a")), ("b", None), ("c", Some("ccc"))] {
        assert_eq!(
            std::fs::read_to_string(into.join("out").join(name))
                .ok()
                .as_deref(),
            text,
            "out/{name} came back wrong, so the two sides' numbers disagree"
        );
    }
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
            .is_err_and(|why| why.contains("didn't cross")),
        "a copy short of its listed size was taken: {landed:?}"
    );
    assert!(!into.join("out/a").exists(), "a short copy landed");
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn what_comes_back_never_lands_through_a_link_out_of_its_folder() {
    let (code, err, home) = far_run(
        "1\tx\nend\n",
        &[("x", "x")],
        "mkdir out && printf a > out/a && printf b > out/b",
    );
    assert_eq!(code, Some(0), "the job didn't run: {err}");
    let back = ["out".to_owned()];
    let elsewhere = home.join("elsewhere");
    std::fs::create_dir_all(&elsewhere).unwrap();
    // A folder linked away, and one file of two linked away under a
    // plain folder, so the other would land first if nothing looked.
    for (link, under) in [("out", "near"), ("out/b", "near2")] {
        let into = home.join(under);
        let at = into.join(link);
        std::fs::create_dir_all(at.parent().unwrap()).unwrap();
        let points = if link == "out" {
            elsewhere.clone()
        } else {
            elsewhere.join("b")
        };
        std::os::unix::fs::symlink(points, &at).unwrap();
        let landed = bring_back(&handover(&home), &into, &back, WAITS);
        assert!(
            landed.as_ref().is_err_and(|why| why.contains("link")),
            "a file came back through {link}: {landed:?}"
        );
        assert!(
            !elsewhere.join("a").exists() && !elsewhere.join("b").exists(),
            "a file came back through {link} to where it points"
        );
        assert!(
            link == "out" || !into.join("out/a").exists(),
            "out/a landed before the link beside it was refused"
        );
    }
    // A link within the folder is the person's own, and is written through.
    let into = home.join("near3");
    std::fs::create_dir_all(into.join("kept")).unwrap();
    std::os::unix::fs::symlink(into.join("kept"), into.join("out")).unwrap();
    assert_eq!(
        bring_back(&handover(&home), &into, &back, WAITS),
        Ok(2),
        "a link within the folder was refused"
    );
    assert_eq!(
        std::fs::read_to_string(into.join("kept/b")).ok().as_deref(),
        Some("b"),
        "what came back didn't land through the folder's own link"
    );
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
}
