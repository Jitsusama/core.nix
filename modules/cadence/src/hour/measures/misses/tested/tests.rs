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
    ] {
        assert_eq!(test_of(said), None, "{said:?} was read as a test");
    }
}

#[test]
fn each_miss_is_judged_by_whether_its_test_runs_here() {
    let ledger = [
        miss("XY9H5C", "cat waits 2 s", None),
        miss("RCTRF8", "History paused", Some("a test somewhere")),
        miss("QCXZEB", "Super+S", Some("core src/a.rs runs")),
        miss("ZINAWZ", "Reader titles", Some("core src/a.rs gone")),
        miss("88LQC0", "backup notice", Some("world src/a.rs anything")),
        miss("16V6F4", "lost Colemak", Some("core src/a.rs unseen")),
        r#"{"at": "x", "owner": "WCJQSG", "what": "carried", "carried": true}"#.to_owned(),
    ]
    .join("\n");
    let m = judged(&ledger, |test| match (test.repo.as_str(), test.name) {
        ("world", _) => Look::Unknown,
        (_, "runs") => Look::Runs,
        (_, "unseen") => Look::Blind,
        _ => Look::Missing,
    });
    assert_eq!(
        m.behind,
        vec![
            format!("cat waits 2 s ({AT}), naming no test · XY9H5C"),
            format!(
                "History paused ({AT}), its test \"a test somewhere\" isn't REPO PATH NAME \
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
    assert_eq!(m.target, Some(0.0), "a whole ledger was left unjudged");
    assert_eq!(
        m.owner, "XY9H5C",
        "the oldest untested miss's owner doesn't lead"
    );
    assert!(
        m.says.ends_with("(1 couldn't be looked for)"),
        "the miss the hour couldn't look for went unsaid: {}",
        m.says
    );
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

fn git(repo: &Path, args: &[&str]) {
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
    let cases = [
        ("main", test("src/a.rs", "the_cat_never_waits"), Look::Runs),
        ("main", test("src", "the_cat_never_waits"), Look::Runs),
        ("main", test("src/a.rs", "elsewhere"), Look::Missing),
        ("main", test("src/a.rs", "the_cat_never"), Look::Runs),
        ("0000000", test("src/a.rs", "the_cat"), Look::Blind),
    ];
    for (rev, test, look) in cases {
        assert_eq!(
            found(&repo, rev, &test),
            look,
            "{test:?} at {rev} was found wrongly"
        );
    }
    std::fs::remove_dir_all(&repo).ok();
}
