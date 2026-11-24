//! How a miss's test is read from its line and found where it runs.

use std::path::Path;

use super::*;

const AT: &str = "2026-11-14T17:24:00-05:00";

/// A ledger line for a miss of `owner`'s, with `test` as its test.
fn miss(owner: &str, what: &str, test: Option<&str>) -> String {
    let mut line = serde_json::json!({"at": AT, "owner": owner, "what": what});
    if let Some(test) = test {
        line["test"] = test.into();
    }
    line.to_string()
}

#[test]
fn a_test_is_named_by_its_repository_path_and_name() {
    assert_eq!(
        test_of("Jitsusama/core.nix modules/x/src/a.rs the_cat_never_waits mid-air"),
        Some(Test {
            repo: "core".to_owned(),
            path: "modules/x/src/a.rs",
            name: "the_cat_never_waits mid-air",
        }),
        "a test's repository, path or name was read wrongly"
    );
    for said in [
        "",
        "core",
        "core modules/x.rs",
        "core modules/x.rs  ",
        "core --output=x a",
        "core modules/x.rs a\nb",
    ] {
        assert_eq!(test_of(said), None, "{said:?} was read as a test");
    }
}

#[test]
fn each_miss_is_judged_by_whether_its_test_runs_here() {
    let ledger = [
        miss("XY9H5C", "cat waits 2 s", None),
        miss("RCTRF8", "History paused", Some("somewhere")),
        miss("QCXZEB", "Super+S", Some("core src/a.rs runs")),
        miss("ZINAWZ", "Reader titles", Some("core src/a.rs gone")),
        miss("88LQC0", "backup notice", Some("world src/a.rs anything")),
        miss("16V6F4", "lost Colemak", Some("core src/a.rs unseen")),
        r#"{"at": "x", "owner": "WCJQSG", "what": "carried", "carried": true}"#.to_owned(),
    ]
    .join("\n");
    let look = |unseen| {
        move |test: &Test<'_>| match (test.repo.as_str(), test.name) {
            ("world", _) => Look::Unknown,
            (_, "runs") => Look::Runs,
            (_, "unseen") => unseen,
            _ => Look::Missing,
        }
    };
    let m = judged(&ledger, look(Look::Blind));
    assert_eq!(
        m.behind,
        vec![
            format!("cat waits 2 s ({AT}), naming no test · XY9H5C"),
            format!(
                "History paused ({AT}), its test \"somewhere\" isn't REPO PATH NAME \
                 · RCTRF8"
            ),
            format!(
                "Reader titles ({AT}), its test gone isn't in core's src/a.rs as it runs \
                 here · ZINAWZ"
            ),
            format!(
                "backup notice ({AT}), its test is in world, which cadence has no \
                 checkout of · 88LQC0"
            ),
        ],
        "a miss was judged wrongly, or one that runs, was carried or unseen was counted"
    );
    assert_eq!(m.now, 4.0, "the count isn't its lines'");
    assert_eq!(
        m.owner, "XY9H5C",
        "the oldest untested miss's owner doesn't lead"
    );
    // An owner's ask isn't closed for a miss the hour couldn't see.
    assert_eq!(m.target, None, "an hour that couldn't look judged");
    assert!(
        m.says
            .ends_with("(1 couldn't be looked for, so none is judged)"),
        "the miss the hour couldn't look for went unsaid: {}",
        m.says
    );
    let seen = judged(&ledger, look(Look::Runs));
    assert_eq!(seen.target, Some(0.0), "a whole ledger was left unjudged");
}

#[test]
fn a_ledger_not_read_whole_judges_none() {
    let ledger = format!("{}\nnot json", miss("XY9H5C", "cat waits", None));
    let m = judged(&ledger, |_| Look::Runs);
    assert_eq!(m.target, None, "a ledger not read whole was judged");
    assert!(
        m.says.ends_with(super::super::UNREAD),
        "the ledger not read whole went unsaid: {}",
        m.says
    );
}

/// A test of core's in `path` named `name`.
fn test(path: &'static str, name: &'static str) -> Test<'static> {
    Test {
        repo: "core".to_owned(),
        path,
        name,
    }
}

/// What git says in `repo`, run as no one's config would change it.
fn git(repo: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_AUTHOR_NAME", "here")
        .env("GIT_AUTHOR_EMAIL", "here@example.org")
        .env("GIT_COMMITTER_NAME", "here")
        .env("GIT_COMMITTER_EMAIL", "here@example.org")
        .output()
        .expect("git runs");
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_owned()
}

#[test]
fn a_test_is_found_in_its_path_at_the_revision_that_runs() {
    let repo = std::env::temp_dir().join(format!("cadence-tested-{}", std::process::id()));
    std::fs::remove_dir_all(&repo).ok();
    std::fs::create_dir_all(repo.join("src")).expect("a scratch history");
    git(&repo, &["init", "-q", "-b", "main"]);
    std::fs::write(repo.join("src/a.rs"), "fn the_cat_never_waits() {}\n").expect("a test");
    std::fs::write(repo.join("b.rs"), "fn elsewhere() {}\n").expect("another");
    git(&repo, &["add", "."]);
    git(&repo, &["commit", "-q", "-m", "the test"]);
    // A folder, a pattern or part of a word names no test.
    let cases = [
        (test("src/a.rs", "the_cat_never_waits"), Look::Runs),
        (test("src/a.rs", "elsewhere"), Look::Missing),
        (test("src/a.rs", "the_cat_never"), Look::Missing),
        (test("src", "the_cat_never_waits"), Look::Folder),
        (test(".", "fn"), Look::Folder),
        (test("*", "fn"), Look::Missing),
        (test("src/b.rs", "fn"), Look::Missing),
    ];
    for (test, look) in cases {
        assert_eq!(
            found(&repo, "main", &test),
            look,
            "{test:?} was found wrongly"
        );
    }
    // A repository is read at what runs here, a switch's revision for one
    // it follows and its main for any other, or none it doesn't hold.
    let repos = super::super::Checkouts::from([
        ("core".to_owned(), repo.clone()),
        ("other".to_owned(), repo.clone()),
    ]);
    let head = git(&repo, &["rev-parse", "HEAD"]);
    for (runs, core) in [
        (Some(head.clone()), Some(head.clone())),
        (Some("0000000".to_owned()), None),
        (None, None),
    ] {
        let runs: BTreeMap<String, String> = runs
            .map(|rev| ("core".to_owned(), rev))
            .into_iter()
            .collect();
        let revs = revisions(&repos, &runs);
        assert_eq!(revs["core"], core, "core was read at the wrong revision");
        assert_eq!(
            revs["other"].as_deref(),
            Some("main"),
            "a repository cadence doesn't follow wasn't read at its main"
        );
    }
    std::fs::remove_dir_all(&repo).ok();
}
