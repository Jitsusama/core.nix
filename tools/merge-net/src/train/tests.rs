//! What the train does, case by case, the git cases against repositories
//! of their own.

use super::*;

/// A state folder of the test's own.
fn root(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("merge-net-train-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    root
}

fn rider(head: &str, at: u64) -> Rider {
    Rider {
        head: head.to_owned(),
        branch: format!("jitsusama/{head}"),
        at,
    }
}

#[test]
fn riders_are_read_back_in_the_order_they_boarded() {
    let root = root("order");
    board(&root, &rider("b2", 20)).unwrap();
    board(&root, &rider("a1", 10)).unwrap();
    assert_eq!(
        riders(&root),
        vec![rider("a1", 10), rider("b2", 20)],
        "the riders weren't read back first boarded first"
    );
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn a_head_aboard_is_found_riding_and_no_other_is() {
    let root = root("aboard");
    board(&root, &rider("a1", 10)).unwrap();
    board(&root, &rider("b2", 20)).unwrap();
    assert_eq!(
        rider_at(&root, "b2"),
        Some(rider("b2", 20)),
        "a head aboard wasn't found riding, so ready would judge it as if it didn't"
    );
    assert_eq!(
        rider_at(&root, "c3"),
        None,
        "a head never aboard was found riding"
    );
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn a_rider_that_fell_rides_no_more_and_runs_again_alone() {
    let root = root("fell");
    board(&root, &rider("a1", 10)).unwrap();
    fall(&root, "a1", "it conflicts with main").unwrap();
    assert!(riders(&root).is_empty(), "a rider that fell still rode");
    assert_eq!(
        seat(false, fell(&root, "a1")),
        Seat::Fell("it conflicts with main".to_owned()),
        "a head that fell boarded again"
    );
    assert_eq!(seat(true, None), Seat::Rides, "a rider boarded twice");
    assert_eq!(seat(false, None), Seat::Board, "a new head didn't board");
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn a_red_train_splits_until_the_rider_that_breaks_it_falls_alone() {
    let breaks = 2;
    let mut aboard: Vec<usize> = (0..5).collect();
    let (mut landed, mut fell, mut runs) = (Vec::new(), Vec::new(), 0);
    let mut take = aboard.len();
    while !aboard.is_empty() && runs < 20 {
        let riding: Vec<usize> = aboard.iter().copied().take(take).collect();
        runs += 1;
        match split(riding.len(), !riding.contains(&breaks)) {
            Split::All => landed.extend(riding.iter().copied()),
            Split::Falls => fell.extend(riding.iter().copied()),
            Split::Front(front) => {
                take = front;
                continue;
            }
        }
        aboard.retain(|rider| !riding.contains(rider));
        take = aboard.len();
    }
    assert_eq!(
        fell,
        vec![breaks],
        "not only the rider that broke the train fell"
    );
    assert_eq!(
        landed,
        vec![0, 1, 3, 4],
        "the riders that broke nothing didn't land"
    );
    assert!(
        runs <= 5,
        "the split took {runs} runs to find one rider in five"
    );
}

#[test]
fn a_merge_is_worded_as_githubs_merge_button_words_it() {
    assert_eq!(
        subject(5066, "Jitsusama", "jitsusama/quarantine-stands-apart"),
        "Merge pull request #5066 from Jitsusama/jitsusama/quarantine-stands-apart",
        "the net's searches read a merge's pull request from these words"
    );
}

/// Git in `repo` with no user's or system's config, as the other
/// modules' tests run it, giving what it printed or why it failed.
fn git_in(repo: &Path, arguments: &[&str]) -> Result<String, String> {
    let ran = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(arguments)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_AUTHOR_NAME", "t")
        .env("GIT_AUTHOR_EMAIL", "t@t")
        .env("GIT_COMMITTER_NAME", "t")
        .env("GIT_COMMITTER_EMAIL", "t@t")
        .output()
        .map_err(|error| error.to_string())?;
    if ran.status.success() {
        Ok(String::from_utf8_lossy(&ran.stdout).trim().to_owned())
    } else {
        Err(String::from_utf8_lossy(&ran.stderr).into_owned())
    }
}

/// A repository at `repo` holding one commit on main, which it gives.
fn based(repo: &Path) -> String {
    fs::create_dir_all(repo).unwrap();
    git_in(repo, &["init", "--quiet", "-b", "main"]).unwrap();
    fs::write(repo.join("shared"), "main\n").unwrap();
    git_in(repo, &["add", "shared"]).unwrap();
    git_in(repo, &["commit", "--quiet", "-m", "base"]).unwrap();
    git_in(repo, &["rev-parse", "HEAD"]).unwrap()
}

/// Commit `words` as `file` in `repo` on `from`, giving the commit.
fn change(repo: &Path, from: &str, file: &str, words: &str) -> String {
    git_in(repo, &["checkout", "--quiet", "--detach", from]).unwrap();
    fs::write(repo.join(file), words).unwrap();
    git_in(repo, &["add", file]).unwrap();
    git_in(repo, &["commit", "--quiet", "-m", file]).unwrap();
    git_in(repo, &["rev-parse", "HEAD"]).unwrap()
}

fn merging(head: &str, number: u64) -> Merging {
    Merging {
        head: head.to_owned(),
        subject: subject(number, "o", head),
        title: format!("change {number}"),
    }
}

#[test]
fn a_passed_candidate_lands_on_main_only_as_a_fast_forward() {
    let place = root("land");
    let (origin, checkout) = (place.join("origin.git"), place.join("checkout"));
    let base = based(&checkout);
    let url = origin.display().to_string();
    git_in(&place, &["init", "--quiet", "--bare", "-b", "main", &url]).unwrap();
    let git = |arguments: &[&str]| git_in(&checkout, arguments);
    git(&["remote", "add", "origin", &url]).unwrap();
    git(&["push", "--quiet", "origin", "main"]).unwrap();
    git(&["fetch", "--quiet", "origin", "main"]).unwrap();
    let head = change(&checkout, &base, "apart", "one\n");
    let riding = [merging(&head, 1)];
    let done = cut(&git, &base, &riding).unwrap();
    let candidate = git(&["rev-parse", "HEAD"]).unwrap();
    assert_eq!(
        land(&checkout, &git, &candidate, &riding, &done.merged),
        Landing::Landed(candidate.clone()),
        "a passed candidate on a main that stood still didn't land"
    );
    assert_eq!(
        git_in(&origin, &["rev-parse", "main"]).unwrap(),
        candidate,
        "origin's main isn't the candidate that landed"
    );
    git(&["remote", "set-url", "origin", "/nowhere"]).unwrap();
    let next = change(&checkout, &candidate, "next", "two\n");
    let more = [merging(&next, 2)];
    let again = cut(&git, &candidate, &more).unwrap();
    let refused = git(&["rev-parse", "HEAD"]).unwrap();
    assert!(
        matches!(
            land(&checkout, &git, &refused, &more, &again.merged),
            Landing::Refused(_)
        ),
        "a push refused while main stood still didn't fall back"
    );
    fs::remove_dir_all(&place).unwrap();
}

#[test]
fn a_candidate_merges_each_rider_and_leaves_out_one_that_conflicts() {
    let repo = root("cut");
    let base = based(&repo);
    let git = |arguments: &[&str]| git_in(&repo, arguments);
    let apart = change(&repo, &base, "apart", "one\n");
    let clashes = change(&repo, &base, "shared", "theirs\n");
    let main = change(&repo, &base, "shared", "mains\n");
    let done = cut(&git, &main, &[merging(&apart, 1), merging(&clashes, 2)]).unwrap();
    assert_eq!(
        done,
        Cut {
            merged: vec![apart.clone()],
            conflicted: vec![clashes],
        },
        "the candidate didn't merge the rider apart and leave out the one that clashed"
    );
    assert_eq!(
        git(&["log", "-1", "--format=%P"]).unwrap(),
        format!("{main} {apart}"),
        "the candidate's merge doesn't sit on main with the rider beside it"
    );
    assert!(
        git(&["status", "--porcelain"]).unwrap().is_empty(),
        "the conflicted merge was left half done"
    );
    fs::remove_dir_all(&repo).unwrap();
}
