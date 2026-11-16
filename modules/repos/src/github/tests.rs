//! GitHub's part over an answer GitHub gave, cut down: three of this
//! repository's own changes, which run no checks there; one repository
//! the App can't see; and a busy one, since renamed, whose changes pass,
//! wait and fail.

use super::*;
use crate::model::{Repository, SCHEMA as RECORD};

pub(super) fn answer() -> Value {
    serde_json::from_str(include_str!("answer.json")).expect("the answer is JSON")
}

pub(super) fn one(key: &str) -> Asked {
    let (owner, name) = key.split_once('/').expect("an owner and a name");
    Asked {
        key: format!("github:{key}"),
        owner: owner.to_owned(),
        name: name.to_owned(),
    }
}

/// The repositories the answer answers, as the disk keys them: the busy
/// one by its name before GitHub renamed it.
fn asked_after() -> Vec<Asked> {
    vec![
        one("Jitsusama/core.nix"),
        one("someone/private"),
        one("someone/once-quiet"),
    ]
}

fn parsed() -> (BTreeMap<String, Vec<Change>>, Vec<String>) {
    parse(&answer(), &asked_after(), Through::App)
}

fn busy() -> Vec<Change> {
    parsed()
        .0
        .remove("github:someone/once-quiet")
        .expect("the busy one")
}

fn repository(key: &str) -> Repository {
    Repository {
        key: key.to_owned(),
        name: key.rsplit('/').next().unwrap_or(key).to_owned(),
        owner: None,
        remotes: Vec::new(),
        trunk: None,
        written_at: None,
        checkouts: Vec::new(),
        changes: Vec::new(),
        read_through: None,
    }
}

fn record_of(keys: &[&str]) -> Record {
    Record {
        schema: RECORD.to_owned(),
        read_at: 1_794_755_500,
        machine: "laptop".to_owned(),
        roots: Vec::new(),
        sources: Vec::new(),
        repositories: keys.iter().map(|key| repository(key)).collect(),
    }
}

#[test]
fn only_github_s_repositories_are_asked_after_each_once() {
    let record = record_of(&[
        "github:Jitsusama/core.nix",
        "gitlab:group/sub/thing",
        "local:~/scratch",
        "github:Jitsusama/core.nix",
        "git.example.org:someone/thing",
        "github:someone/busy",
    ]);
    assert_eq!(
        asked(&record),
        vec![one("Jitsusama/core.nix"), one("someone/busy")]
    );
}

#[test]
fn each_repository_is_asked_under_its_own_alias_quoted() {
    let text = query(&[one("Jitsusama/core.nix"), one("some\"one/busy")]);
    for asked in [
        r#"r0: repository(owner: "Jitsusama", name: "core.nix") { ...open }"#,
        r#"r1: repository(owner: "some\"one", name: "busy") { ...open }"#,
        "pullRequests(states: OPEN, first: 50,",
    ] {
        assert!(text.contains(asked), "the query lacks {asked}:\n{text}");
    }
    assert!(
        text.lines().all(|line| line.len() <= 100),
        "a line of the query runs past 100:\n{text}"
    );
}

#[test]
fn a_renamed_repository_keeps_the_key_the_disk_gives_it() {
    let (changes, _) = parsed();
    assert_eq!(
        changes.keys().collect::<Vec<_>>(),
        ["github:Jitsusama/core.nix", "github:someone/once-quiet"]
    );
}

#[test]
fn a_repository_the_app_cannot_see_is_said_to_be_unread() {
    let (changes, unread) = parsed();
    assert!(
        !changes.contains_key("github:someone/private"),
        "a repository the App can't see was given changes"
    );
    assert_eq!(
        unread,
        ["github:someone/private: not readable here: GitHub's App can't see it, or it's gone"]
    );
}

#[test]
fn a_change_running_no_checks_says_none() {
    let (changes, _) = parsed();
    let core = &changes["github:Jitsusama/core.nix"];
    assert_eq!(
        core.iter()
            .map(|change| change.github.as_ref().map(|github| github.number))
            .collect::<Vec<_>>(),
        [Some(4783), Some(4725), Some(4782)]
    );
    let first = &core[0];
    assert_eq!(
        first.title,
        "Let the Merge Net Finish Its Round When a New Build Is Installed"
    );
    assert_eq!(first.branch, "jitsusama/net-hands-over-between-rounds");
    assert_eq!(first.base, "main");
    assert_eq!(first.author.as_deref(), Some("Jitsusama"));
    assert_eq!(first.updated_at, 1_794_755_403);
    assert_eq!(
        first.github.as_ref().map(|github| github.url.as_str()),
        Some("https://github.com/Jitsusama/core.nix/pull/4783")
    );
    assert_eq!(
        first.checks,
        Checks {
            state: ChecksState::None,
            passed: 0,
            failed: Vec::new(),
            pending: Vec::new(),
        }
    );
    assert_eq!(first.review, Review::None);
}

#[test]
fn skipped_checks_pass_an_approved_change() {
    let approved = &busy()[0];
    assert_eq!(approved.checks.state, ChecksState::Passing);
    assert_eq!(approved.checks.passed, 4);
    assert_eq!(approved.review, Review::Approved);
    assert!(!approved.draft, "a change ready for review read as a draft");
}

#[test]
fn a_check_still_running_leaves_a_draft_pending() {
    let waiting = &busy()[1];
    assert!(waiting.draft, "a draft read as ready for review");
    assert_eq!(waiting.review, Review::Waiting);
    assert_eq!(waiting.checks.state, ChecksState::Pending);
    assert_eq!(waiting.checks.passed, 3);
    assert_eq!(waiting.checks.pending, ["Check / reminders"]);
}

#[test]
fn a_failed_run_or_status_fails_its_change_whatever_else_runs() {
    let failing = &busy()[2];
    assert_eq!(failing.review, Review::ChangesAsked);
    assert_eq!(failing.checks.state, ChecksState::Failing);
    assert_eq!(failing.checks.failed, ["build (x86_64-linux)", "ci/lint"]);
    assert_eq!(failing.checks.pending, ["Check / commits"]);
    assert_eq!(failing.checks.passed, 1);
}

#[test]
fn changes_come_updated_last_first() {
    let times: Vec<u64> = busy().iter().map(|change| change.updated_at).collect();
    assert!(
        times.windows(2).all(|pair| pair[0] >= pair[1]),
        "changes aren't updated last first: {times:?}"
    );
}

#[test]
fn a_query_refused_whole_says_why() {
    let refused = serde_json::json!({
        "errors": [{ "message": "Bad credentials" }]
    });
    let (changes, unread) = parse(&refused, &asked_after(), Through::App);
    assert_eq!(changes, BTreeMap::new());
    assert_eq!(unread, ["GitHub: Bad credentials"]);
}

#[test]
fn a_refresh_asks_a_few_dozen_at_a_time() {
    let many: Vec<Asked> = (0..=PER_QUERY)
        .map(|at| one(&format!("someone/r{at}")))
        .collect();
    let mut queries = Vec::new();
    let seen = refresh(&many, 7, |query| {
        queries.push(query.matches(": repository(").count());
        Ok(serde_json::json!({ "data": {} }))
    })
    .expect("it refreshes");
    assert_eq!(queries, [PER_QUERY, 1]);
    assert_eq!(seen.read_at, 7);
    assert_eq!(seen.unread.len(), PER_QUERY + 1);
}

#[test]
fn a_refresh_github_answers_without_data_keeps_nothing() {
    let refused = refresh(&asked_after(), 7, |_| {
        Ok(serde_json::json!({
            "errors": [{ "message": "Bad credentials" }]
        }))
    });
    assert_eq!(
        refused,
        Err("GitHub answered no data: Bad credentials".to_owned())
    );
    let unreached = refresh(&asked_after(), 7, |_| Err("can't reach GitHub".to_owned()));
    assert_eq!(unreached, Err("can't reach GitHub".to_owned()));
}

#[test]
fn a_refresh_merged_gives_each_repository_its_changes_and_github_a_source() {
    let (changes, unread) = parsed();
    let seen = Seen {
        schema: SCHEMA.to_owned(),
        read_at: 1_794_755_410,
        changes,
        unread,
        through: BTreeMap::new(),
    };
    let mut record = record_of(&["github:Jitsusama/core.nix", "local:~/scratch"]);
    merge(&mut record, &seen);
    assert_eq!(record.repositories[0].changes.len(), 3);
    assert_eq!(record.repositories[1].changes, Vec::new());
    let github = record.sources.last().expect("GitHub as a source");
    assert_eq!(github.name, "github");
    assert_eq!(github.read_at, Some(1_794_755_410));
    assert_eq!(github.unread.len(), 1);
}

/// A folder of the case's own, beside no other case's, empty as it's
/// made and removed as the case ends, however it ends.
struct Kept {
    folder: PathBuf,
}

impl Kept {
    fn new(name: &str) -> Kept {
        let folder = std::env::temp_dir().join(format!("repos-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        Kept { folder }
    }

    /// Where a refresh would keep what it saw, in the folder.
    fn file(&self) -> PathBuf {
        self.folder.join("repos/github.json")
    }
}

impl Drop for Kept {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.folder);
    }
}

#[test]
fn what_a_refresh_kept_is_merged_from_its_file_and_no_file_adds_nothing() {
    let (changes, unread) = parsed();
    let seen = Seen {
        schema: SCHEMA.to_owned(),
        read_at: 1_794_755_410,
        changes,
        unread,
        through: BTreeMap::new(),
    };
    let kept = Kept::new("kept");
    let file = kept.file();
    keep(&file, &seen).expect("it keeps");
    let mut record = record_of(&["github:Jitsusama/core.nix"]);
    merge_kept(&mut record, &file);
    assert_eq!(
        record.repositories[0].changes.len(),
        3,
        "a kept refresh's changes weren't merged: {record:?}"
    );
    assert_eq!(record.sources.len(), 1, "GitHub isn't a source: {record:?}");

    let mut record = record_of(&["github:Jitsusama/core.nix"]);
    merge_kept(&mut record, &Kept::new("none").file());
    assert_eq!(
        record,
        record_of(&["github:Jitsusama/core.nix"]),
        "a machine no refresh ran on gained something"
    );
}

#[test]
fn a_file_no_refresh_wrote_is_said_unread_and_merges_nothing() {
    let kept = Kept::new("foreign");
    let file = kept.file();
    std::fs::create_dir_all(file.parent().expect("a folder")).expect("it makes");
    std::fs::write(&file, r#"{"schema": "someone.else/1"}"#).expect("it writes");
    let mut record = record_of(&["github:Jitsusama/core.nix"]);
    merge_kept(&mut record, &file);
    let github = record.sources.last().expect("GitHub as a source");
    assert_eq!(github.name, "github");
    assert_eq!(
        github.unread,
        vec![format!("{} isn't a refresh's record", file.display())],
        "a foreign file isn't said unread: {github:?}"
    );
    assert!(
        record.repositories[0].changes.is_empty(),
        "a foreign file merged changes"
    );
}

#[test]
fn what_a_refresh_saw_reads_back_as_it_was_written() {
    let (changes, unread) = parsed();
    let seen = Seen {
        schema: SCHEMA.to_owned(),
        read_at: 1_794_755_410,
        changes,
        unread,
        through: BTreeMap::from([("github:Jitsusama/core.nix".to_owned(), Through::Gh)]),
    };
    let written = serde_json::to_string(&seen).expect("it writes");
    let back: Seen = serde_json::from_str(&written).expect("it reads back");
    assert_eq!(back, seen);
    for field in [r#""review":"changes_asked""#, r#""state":"failing""#] {
        assert!(
            written.contains(field),
            "the record lacks {field}: {written}"
        );
    }
}

#[test]
fn each_change_asks_whose_review_is_asked_in_the_one_query_and_still_only_reads() {
    let text = query(&[one("Jitsusama/core.nix")]);
    assert_eq!(
        text.matches("reviewRequests(first: 10)").count(),
        1,
        "the shared query doesn't ask each change's review requests once:\n{text}"
    );
    assert_eq!(reads_only(&text), Ok(()), "the query no longer only reads");
}

#[test]
fn a_change_holds_the_people_its_review_asks_and_no_team_and_never_keeps_them() {
    let mut node = answer()["data"]["r0"]["pullRequests"]["nodes"][0].clone();
    node["reviewRequests"] = serde_json::json!({"nodes": [
        {"requestedReviewer": {"login": "jsama"}},
        {"requestedReviewer": {}},
        {"requestedReviewer": null},
        {"requestedReviewer": {"login": "osprey"}},
    ]});
    let read = change(&node).expect("the change reads");
    assert_eq!(read.review_asked_of, ["jsama", "osprey"]);
    let written = serde_json::to_string(&read).expect("it writes");
    assert!(
        !written.contains("jsama") && !written.contains("osprey"),
        "a login whose review was asked was kept: {written}"
    );
}
