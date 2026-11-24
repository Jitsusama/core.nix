//! What the train does, case by case, the git cases against repositories
//! of their own.

use super::installed::newer;
use super::splitting::UNTAKEN_FOR;
use super::untaken::{Untaken, unsent};
use super::*;

/// A state folder of the test's own.
pub(super) fn root(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("merge-net-train-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    root
}

/// Two builds standing in for merge-net's, under the state folder `root`,
/// and the link the watch follows, pointing at the first.
fn builds(root: &Path) -> (PathBuf, PathBuf, PathBuf) {
    let store = root.join("store");
    fs::create_dir_all(&store).unwrap();
    let (old, new) = (store.join("old"), store.join("new"));
    fs::write(&old, "").unwrap();
    fs::write(&new, "").unwrap();
    let link = root.join("profile");
    std::os::unix::fs::symlink(&old, &link).unwrap();
    (
        fs::canonicalize(old).unwrap(),
        fs::canonicalize(new).unwrap(),
        link,
    )
}

#[test]
fn the_train_starts_as_the_build_installed_not_the_askers() {
    let root = root("starts-installed");
    let (old, new, link) = builds(&root);
    assert_eq!(
        build(&root, Ok(new.clone())),
        Some(new.clone()),
        "with no installed build named, the train didn't start as the asker's"
    );
    install(&root, &link).unwrap();
    assert_eq!(
        build(&root, Ok(new.clone())),
        Some(old.clone()),
        "the train started as the asker's build, not the one the watch follows"
    );
    fs::remove_file(&link).unwrap();
    assert_eq!(
        build(&root, Ok(new.clone())),
        Some(new),
        "an installed build that can't be read didn't leave the asker's"
    );
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn a_ride_goes_on_as_the_build_a_switch_installed() {
    let root = root("goes-on-installed");
    let (old, new, link) = builds(&root);
    assert_eq!(
        newer(&root, Ok(old.clone()), None),
        None,
        "with no installed build named, a ride went on as another"
    );
    install(&root, &link).unwrap();
    assert_eq!(
        newer(&root, Ok(old.clone()), None),
        None,
        "a ride as the installed build went on as another"
    );
    fs::remove_file(&link).unwrap();
    std::os::unix::fs::symlink(&new, &link).unwrap();
    assert_eq!(
        newer(&root, Ok(old), None),
        Some(new.clone()),
        "a ride from before the switch didn't go on as the build it installed"
    );
    let unreadable = Err(io::Error::other("no exe"));
    assert_eq!(
        newer(&root, unreadable, Some(&new)),
        None,
        "a ride handed over to the installed build, unable to read its own path, handed over again"
    );
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn a_ride_handed_over_keeps_the_right_of_way_a_fresh_one_takes_back() {
    let root = root("handed-way");
    fs::create_dir_all(folder(&root)).unwrap();
    right_of_way::give(&root, crate::state::now());
    begin(&root, true);
    assert!(
        right_of_way::holds(&root, crate::state::now()).is_some(),
        "a ride handed over gave back the right of way its riders held"
    );
    begin(&root, false);
    assert!(
        right_of_way::holds(&root, crate::state::now()).is_none(),
        "a train starting afresh kept a right of way a killed one left"
    );
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn the_watch_names_its_build_absolute_keeping_the_link() {
    let root = root("names-absolute");
    let (old, new, link) = builds(&root);
    let cwd = std::env::current_dir().unwrap();
    install(&root, &pathdiff(&link, &cwd)).unwrap();
    let named = fs::read_to_string(root.join("installed")).unwrap();
    assert!(
        Path::new(named.trim()).is_absolute(),
        "the watch named its build by a path relative to its own folder: {named}"
    );
    fs::remove_file(&link).unwrap();
    std::os::unix::fs::symlink(&new, &link).unwrap();
    assert_eq!(
        build(&root, Ok(old)),
        Some(new),
        "the named build didn't follow the link a switch moved"
    );
    fs::remove_dir_all(&root).unwrap();
}

/// `path` relative to `from`, by as many `..` as `from` is deep.
fn pathdiff(path: &Path, from: &Path) -> PathBuf {
    let mut relative = PathBuf::new();
    for _ in from.components().skip(1) {
        relative.push("..");
    }
    relative.join(path.strip_prefix("/").unwrap())
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
fn a_partial_landing_keeps_the_landed_drops_the_refused_and_seats_the_rest() {
    let root = root("partly");
    for (head, at) in [("a1", 10), ("b2", 20), ("c3", 30)] {
        board(&root, &rider(head, at)).unwrap();
    }
    let merged: Vec<String> = ["a1", "b2", "c3"].map(str::to_owned).into();
    let landing = Landing::Partly {
        landed: "m1".to_owned(),
        kept: vec!["a1".to_owned()],
        refused: "b2".to_owned(),
        why: "GitHub refused #2: it is a draft".to_owned(),
    };
    after_landing(&root, landing, &run(&merged), &mut Splitting::default());
    assert_eq!(
        riders(&root),
        vec![rider("c3", 30)],
        "the rider after the refused one didn't stay aboard alone"
    );
    assert_eq!(fell(&root, "a1"), None, "the landed rider was told it fell");
    let why = fell(&root, "b2").unwrap_or_default();
    assert!(
        why.contains("is a draft") && why.contains("the rider before it landed as m1"),
        "the refused rider wasn't told why, and that the one before it stays: {why}"
    );
    let kept: Vec<Value> = fs::read_to_string(folder(&root).join("landed.jsonl"))
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let said: Vec<(&Value, &Value, &Value)> = kept
        .iter()
        .map(|line| (&line["heads"], &line["fell"], &line["boarded"]))
        .collect();
    assert_eq!(
        said,
        [
            (&json!(["a1"]), &Value::Null, &json!([10])),
            (&Value::Null, &json!(["b2"]), &json!([20])),
        ],
        "the landed rider's boarding wasn't kept beside its head, or the refused one's wait \
         was lost"
    );
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn a_rider_that_steps_off_keeps_its_wait_once_and_one_not_aboard_none() {
    let root = root("off");
    board(&root, &rider("a1", 10)).unwrap();
    leave(&root, "a1", Left::Off);
    leave(&root, "a1", Left::Off);
    leave(&root, "never", Left::Fell);
    let kept = fs::read_to_string(folder(&root).join("landed.jsonl")).unwrap();
    let lines: Vec<Value> = kept
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(
        lines
            .iter()
            .map(|line| (&line["off"], &line["fell"], &line["boarded"]))
            .collect::<Vec<_>>(),
        [(&json!(["a1"]), &Value::Null, &json!([10]))],
        "a rider that stepped off wasn't kept once with its boarding, or one not aboard was"
    );
    assert!(riders(&root).is_empty(), "the rider stayed aboard");
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn a_landing_keeps_when_each_rider_boarded_in_its_heads_order() {
    let root = root("boarded");
    for (head, at) in [("a1", 10), ("b2", 20)] {
        board(&root, &rider(head, at)).unwrap();
    }
    // A rider whose file went keeps its slot, as none.
    let merged: Vec<String> = ["b2", "gone", "a1"].map(str::to_owned).into();
    after_landing(
        &root,
        Landing::Landed("m1".to_owned()),
        &run(&merged),
        &mut Splitting::default(),
    );
    let kept = fs::read_to_string(folder(&root).join("landed.jsonl")).unwrap();
    let line: Value = serde_json::from_str(kept.trim()).unwrap();
    assert_eq!(
        line["boarded"],
        json!([20, null, 10]),
        "the boarding times weren't kept in the heads' order: {line}"
    );
    assert!(riders(&root).is_empty(), "the landed riders stayed aboard");
    fs::remove_dir_all(&root).unwrap();
}

/// A run of `heads` begun at the epoch, its red excused by nothing.
fn run(heads: &[String]) -> Run<'_> {
    Run {
        heads,
        began: 0,
        excused: &[],
    }
}

#[test]
fn a_landing_keeps_the_checks_that_failed_at_mains_own_commit_too() {
    let root = root("excused-landing");
    board(&root, &rider("a1", 10)).unwrap();
    let merged: Vec<String> = ["a1"].map(str::to_owned).into();
    let excused: Vec<String> = ["cat-on-niri"].map(str::to_owned).into();
    let ran = Run {
        heads: &merged,
        began: 0,
        excused: &excused,
    };
    after_landing(
        &root,
        Landing::Landed("m1".to_owned()),
        &ran,
        &mut Splitting::default(),
    );
    let kept = fs::read_to_string(folder(&root).join("landed.jsonl")).unwrap_or_default();
    let line: Value = serde_json::from_str(kept.trim()).unwrap_or_default();
    assert_eq!(
        line["excused"],
        json!(["cat-on-niri"]),
        "a landing whose red main's own commit excused didn't say which checks: {kept}"
    );
    assert!(riders(&root).is_empty(), "the excused rider didn't alight");
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn a_first_rider_github_refuses_falls_alone() {
    let root = root("turned");
    for (head, at) in [("a1", 10), ("b2", 20)] {
        board(&root, &rider(head, at)).unwrap();
    }
    let merged: Vec<String> = ["a1", "b2"].map(str::to_owned).into();
    let landing = Landing::Turned {
        refused: "a1".to_owned(),
        why: "GitHub refused #1: it is a draft".to_owned(),
    };
    after_landing(&root, landing, &run(&merged), &mut Splitting::default());
    assert_eq!(
        riders(&root),
        vec![rider("b2", 20)],
        "the rider after a refused first one didn't stay aboard"
    );
    let why = fell(&root, "a1").unwrap_or_default();
    assert!(
        why.contains("is a draft") && why.contains("merged nothing"),
        "the refused first rider wasn't told why: {why}"
    );
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn a_refused_first_rider_keeps_the_search_and_a_second_in_a_row_empties_the_train() {
    let root = root("turned-twice");
    for (head, at) in [("a1", 10), ("b2", 20), ("c3", 30), ("d4", 40)] {
        board(&root, &rider(head, at)).unwrap();
    }
    let mut splitting = Splitting::default();
    assert_eq!(
        splitting.red(4),
        After::Narrows,
        "a red of four didn't narrow"
    );
    let turned = |refused: &str| Landing::Turned {
        refused: refused.to_owned(),
        why: format!("GitHub refused {refused}: 403"),
    };
    let riding: Vec<String> = ["a1", "b2"].map(str::to_owned).into();
    after_landing(&root, turned("a1"), &run(&riding), &mut splitting);
    assert_eq!(
        splitting.take(),
        1,
        "the search forgot the red lies among the three riders after the refused one"
    );
    let riding: Vec<String> = ["b2"].map(str::to_owned).into();
    after_landing(&root, turned("b2"), &run(&riding), &mut splitting);
    let why = fell(&root, "b2").unwrap_or_default();
    assert!(
        why.contains("refused the train's first rider before"),
        "a second refusal in a row wasn't read as GitHub refusing the train: {why}"
    );
    assert_eq!(
        splitting,
        Splitting::default(),
        "the train kept a search after GitHub refused it twice"
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
    let untaken = Untaken {
        candidate: "u1",
        log,
        unstarted: None,
    };
    let riding = ["a1".to_owned(), "b2".to_owned()];
    // A gate that exits at once comes back every few seconds, so the
    // train gives up by time, not by how many it ran.
    let mut now = 1000;
    while now < 1000 + UNTAKEN_FOR {
        assert!(
            unsent(&root, &untaken, &riding, &mut splitting, now),
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
        !unsent(&root, &untaken, &riding, &mut splitting, now),
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

/// A gate that never started, on a full disk say, asked no builder, and
/// its log may never have been made, so a rider that falls for it is told
/// the train's own fault rather than sent to the builders.
#[test]
fn a_rider_falling_for_a_gate_that_never_started_is_told_why() {
    let root = root("unstarted");
    board(&root, &rider("a1", 10)).unwrap();
    let mut splitting = Splitting::default();
    let untaken = Untaken {
        candidate: "u1",
        log: Path::new("/state/train/u.log"),
        unstarted: Some("its log couldn't be made"),
    };
    let riding = ["a1".to_owned()];
    assert!(
        unsent(&root, &untaken, &riding, &mut splitting, 1000),
        "the train gave up on a gate that never started at once"
    );
    assert!(
        !unsent(&root, &untaken, &riding, &mut splitting, 1000 + UNTAKEN_FOR),
        "the train never gave up on a gate that never started"
    );
    let why = fell(&root, "a1").unwrap_or_default();
    assert!(
        why.contains("gate couldn't start")
            && why.contains("its log couldn't be made")
            && !why.contains("u.log")
            && !why.starts_with("no builder"),
        "a rider wasn't told the gate never started, and why: {why}"
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
pub(super) fn git_in(repo: &Path, arguments: &[&str]) -> Result<String, String> {
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
pub(super) fn based(repo: &Path) -> String {
    fs::create_dir_all(repo).unwrap();
    git_in(repo, &["init", "--quiet", "-b", "main"]).unwrap();
    fs::write(repo.join("shared"), "main\n").unwrap();
    git_in(repo, &["add", "shared"]).unwrap();
    git_in(repo, &["commit", "--quiet", "-m", "base"]).unwrap();
    git_in(repo, &["rev-parse", "HEAD"]).unwrap()
}

/// Commit `words` as `file` in `repo` on `from`, giving the commit.
pub(super) fn change(repo: &Path, from: &str, file: &str, words: &str) -> String {
    git_in(repo, &["checkout", "--quiet", "--detach", from]).unwrap();
    fs::write(repo.join(file), words).unwrap();
    git_in(repo, &["add", file]).unwrap();
    git_in(repo, &["commit", "--quiet", "-m", file]).unwrap();
    git_in(repo, &["rev-parse", "HEAD"]).unwrap()
}

pub(super) fn merging(head: &str, number: u64) -> Merging {
    Merging {
        number,
        head: head.to_owned(),
        subject: subject(number, "o", head),
        title: format!("change {number}"),
    }
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
