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
fn a_gate_no_builder_took_says_nothing_of_its_riders() {
    assert_eq!(gated(Some(0), ""), Gated::Passed, "a gate that passed");
    assert_eq!(gated(Some(1), ""), Gated::Red, "a check that failed");
    assert_eq!(gated(None, ""), Gated::Red, "a gate a signal ended");
    let lost = "error: failed to start SSH master connection to 'nix-offload-more'\n";
    for code in [75, 69] {
        assert_eq!(
            gated(Some(code), lost),
            Gated::Unsent,
            "nix-offload's {code} says no builder took it, not that a rider broke it"
        );
    }
}

/// nix-offload ends with the first part that failed, and a VM part no
/// builder took comes first: a rider that broke another part's check
/// still reads 75 or 69.
#[test]
fn an_untaken_gate_whose_words_name_a_failed_check_is_red() {
    for words in [
        "error: Cannot build '/nix/store/x-a-check.drv'.\n       Reason: builder failed with \
         exit code 1.\n",
        "\u{2715} a check the change touched failed\n",
    ] {
        for code in [75, 69] {
            assert_eq!(
                gated(Some(code), words),
                Gated::Red,
                "a check that failed read as untaken with {code}: {words}"
            );
        }
    }
}

/// A builder's words can hold any byte, and nix-offload sets aside a
/// replay onto a newer main that failed, running the change again on its
/// own base: the gate is read by what its last run said.
#[test]
fn a_gate_is_read_by_what_its_last_run_said() {
    let root = root("words");
    fs::create_dir_all(&root).unwrap();
    let log = root.join("candidate.log");
    let failed = "error: Cannot build '/nix/store/x-a-check.drv'.\n       Reason: builder \
                  failed with exit code 1.\n";
    let mut odd = b"a builder said \xff\n".to_vec();
    odd.extend_from_slice(failed.as_bytes());
    fs::write(&log, odd).unwrap();
    assert_eq!(
        gated(Some(75), &words_of(&log)),
        Gated::Red,
        "a failed check beside a byte that isn't UTF-8 read as untaken"
    );
    let replayed = format!(
        "{failed}nix-offload: 1234567 over main failed, so the change runs again on its \
         own base\nnix-offload: no builder took it\n"
    );
    fs::write(&log, replayed).unwrap();
    assert_eq!(
        gated(Some(75), &words_of(&log)),
        Gated::Unsent,
        "a replay nix-offload set aside made an untaken run read as red"
    );
    fs::remove_dir_all(&root).unwrap();
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
fn a_rider_asked_of_again_is_told_it_waits_not_that_it_boarded() {
    let root = root("waits");
    board(&root, &rider("a1", 10)).unwrap();
    board(&root, &rider("b2", 20)).unwrap();
    let said = rides_said(&root);
    assert!(
        said.starts_with("it waits on the train, 2 riders in all"),
        "a rider asked of again wasn't told it waits on the train with the others: {said}"
    );
    assert!(
        !said.contains("put") && !said.contains("started"),
        "a rider asked of again was told it was just put on the train: {said}"
    );
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn a_train_that_gives_up_makes_every_rider_fall_saying_why() {
    let root = root("gives-up");
    for (head, at) in [("a1", 10), ("b2", 20), ("c3", 30)] {
        board(&root, &rider(head, at)).unwrap();
    }
    let mut splitting = Splitting::default();
    let log = Path::new("/state/train/c.log");
    red(&root, "c1", log, &["a1".to_owned()], &mut splitting);
    assert_eq!(
        riders(&root).len(),
        2,
        "the first rider to fail alone didn't fall by itself"
    );
    red(&root, "c2", log, &["b2".to_owned()], &mut splitting);
    assert!(
        riders(&root).is_empty(),
        "riders still rode a train that gave up"
    );
    let why = fell(&root, "c3").unwrap_or_default();
    assert!(
        why.starts_with("the train gave up") && why.contains("c.log"),
        "a rider that never rode alone wasn't told the train gave up, and where: {why}"
    );
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn a_candidate_no_builder_took_keeps_its_riders_and_search_until_it_gives_up() {
    let root = root("untaken");
    for (head, at) in [("a1", 10), ("b2", 20), ("c3", 30), ("d4", 40)] {
        board(&root, &rider(head, at)).unwrap();
    }
    let mut splitting = Splitting::default();
    assert_eq!(
        splitting.red(4),
        After::Narrows,
        "a red of four didn't narrow"
    );
    let log = Path::new("/state/train/u.log");
    let riding = ["a1".to_owned(), "b2".to_owned()];
    // A gate that exits at once comes back every few seconds, so the
    // train gives up by time, not by how many it ran.
    let mut now = 1000;
    while now < 1000 + UNTAKEN_FOR {
        assert!(
            unsent(&root, "u1", log, &riding, &mut splitting, now),
            "the train gave up {} s into candidates no builder took",
            now - 1000
        );
        assert_eq!(
            riders(&root).len(),
            4,
            "a rider fell from a run no builder took"
        );
        assert_eq!(
            splitting.take(),
            2,
            "a run no builder took lost the train's search"
        );
        now += 5;
    }
    assert!(
        !unsent(&root, "u1", log, &riding, &mut splitting, now),
        "the train never gave up on candidates no builder took"
    );
    let left: Vec<String> = riders(&root).into_iter().map(|rider| rider.head).collect();
    assert_eq!(
        left,
        ["c3", "d4"],
        "riders that didn't ride the last candidate fell, or its riders didn't"
    );
    assert_eq!(
        splitting.take(),
        usize::MAX,
        "the riders left aboard didn't start a search afresh"
    );
    let why = fell(&root, "b2").unwrap_or_default();
    assert!(
        why.starts_with("no builder or evaluator took") && why.contains("u.log"),
        "a rider wasn't told no builder took the train, and where: {why}"
    );
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn a_run_a_builder_took_counts_the_untaken_afresh() {
    let mut splitting = Splitting::default();
    assert!(splitting.untaken(0), "the train gave up at once");
    assert!(
        splitting.untaken(UNTAKEN_FOR - 1),
        "the train gave up too soon"
    );
    splitting.taken();
    assert!(
        splitting.untaken(UNTAKEN_FOR),
        "untaken time before a run a builder took still counted"
    );
    assert!(
        splitting.untaken(2 * UNTAKEN_FOR - 1),
        "untaken time before a run a builder took still counted"
    );
    assert!(
        !splitting.untaken(2 * UNTAKEN_FOR),
        "the train never gave up"
    );
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

/// Which of `riders` riders land and which fall, in order, and how many
/// candidates it took, when a candidate fails as `fails` says of the
/// riders it holds: `Splitting` choosing whom each takes, as in `ride`.
fn split_out(riders: usize, fails: &dyn Fn(&[usize]) -> bool) -> (Vec<usize>, Vec<usize>, usize) {
    let mut aboard: Vec<usize> = (0..riders).collect();
    let (mut landed, mut fell, mut runs) = (Vec::new(), Vec::new(), 0);
    let mut splitting = Splitting::default();
    while !aboard.is_empty() && runs < 50 {
        let riding: Vec<usize> = aboard.iter().copied().take(splitting.take()).collect();
        runs += 1;
        let gone = if fails(&riding) {
            match splitting.red(riding.len()) {
                After::Narrows => continue,
                After::Falls => riding,
                After::AllFall => aboard.clone(),
            }
        } else {
            splitting.landed(riding.len());
            landed.extend(riding.iter().copied());
            aboard.retain(|rider| !riding.contains(rider));
            continue;
        };
        fell.extend(gone.iter().copied());
        aboard.retain(|rider| !gone.contains(rider));
    }
    (landed, fell, runs)
}

#[test]
fn a_red_train_splits_until_the_rider_that_breaks_it_falls_alone() {
    for breaks in 0..5 {
        let (landed, fell, runs) = split_out(5, &|riding| riding.contains(&breaks));
        assert_eq!(
            fell,
            vec![breaks],
            "not only rider {breaks}, which broke the train, fell"
        );
        let others: Vec<usize> = (0..5).filter(|rider| *rider != breaks).collect();
        assert_eq!(
            landed, others,
            "the riders that broke nothing beside rider {breaks} didn't land"
        );
        assert!(
            runs <= 5,
            "the split took {runs} runs to find rider {breaks} of five"
        );
    }
}

#[test]
fn two_riders_that_break_the_train_each_fall_and_the_rest_land() {
    let (landed, fell, _) = split_out(5, &|riding| riding.contains(&1) || riding.contains(&3));
    assert_eq!(
        fell,
        vec![1, 3],
        "the two riders that broke the train didn't both fall"
    );
    assert_eq!(
        landed,
        vec![0, 2, 4],
        "the riders that broke nothing didn't land"
    );
}

#[test]
fn two_breakers_boarded_one_after_the_other_send_the_riders_behind_back_alone() {
    let (landed, fell, _) = split_out(5, &|riding| riding.contains(&1) || riding.contains(&2));
    assert_eq!(
        landed,
        vec![0],
        "a rider boarded before the two breakers didn't land"
    );
    assert_eq!(
        fell,
        vec![1, 2, 3, 4],
        "the riders from the first breaker on didn't all fall back to running alone"
    );
}

#[test]
fn a_train_red_for_every_rider_gives_up_rather_than_halving_through_them_all() {
    let (landed, fell, runs) = split_out(5, &|_| true);
    assert!(landed.is_empty(), "a rider landed on a train red for all");
    assert_eq!(
        fell,
        vec![0, 1, 2, 3, 4],
        "not every rider of a red train fell"
    );
    assert!(
        runs <= 6,
        "a train red for every rider took {runs} runs to give up, where running each \
         again alone takes five"
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
