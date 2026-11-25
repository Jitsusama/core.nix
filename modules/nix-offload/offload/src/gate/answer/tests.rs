//! answer's cases: what a gate takes, what it refuses before any fetch,
//! and that nothing planted in its state turns a removal elsewhere.

use super::*;
use std::process::Command;

/// A forge of one repo, `kestrel/osprey`, whose main has two commits,
/// and a tree cloned from it with commits of its own past main.
struct Forge {
    root: tempfile::TempDir,
    main: String,
}

fn git(folder: &Path, arguments: &[&str]) -> String {
    let output = Command::new("git")
        .args([
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@t",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(arguments)
        .current_dir(folder)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .expect("git runs");
    assert!(
        output.status.success(),
        "git {arguments:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_owned()
}

fn committed(tree: &Path, name: &str) -> String {
    fs::write(tree.join(name), name).expect("a file is written");
    git(tree, &["add", name]);
    git(tree, &["commit", "--quiet", "-m", name]);
    git(tree, &["rev-parse", "HEAD"])
}

impl Forge {
    fn new() -> Self {
        let root = tempfile::tempdir().expect("a folder");
        let bare = root.path().join("forge/kestrel/osprey.git");
        fs::create_dir_all(&bare).expect("the forge's folder");
        git(
            &bare,
            &["init", "--quiet", "--bare", "--initial-branch", "main"],
        );
        let tree = root.path().join("tree");
        git(
            root.path(),
            &["clone", "--quiet", &bare.to_string_lossy(), "tree"],
        );
        git(&tree, &["checkout", "--quiet", "-b", "main"]);
        committed(&tree, "one");
        let main = committed(&tree, "two");
        git(&tree, &["push", "--quiet", "origin", "main"]);
        Self { root, main }
    }

    fn tree(&self) -> PathBuf {
        self.root.path().join("tree")
    }

    /// A gate's folder for `commit`, with `bundle` and a request.
    fn gate(&self, commit: &str, bundle: &[u8]) -> GateFolder {
        self.gate_of("github:kestrel/osprey", commit, bundle)
    }

    /// A gate's folder for `commit` of `repo`, with `bundle` and a
    /// request.
    fn gate_of(&self, repo: &str, commit: &str, bundle: &[u8]) -> GateFolder {
        let scratch = self.root.path().join("scratch");
        let gate = scratch.join(QUEST).join(super::super::GATES).join(commit);
        fs::create_dir_all(&gate).expect("the gate's folder");
        fs::write(gate.join(BUNDLE), bundle).expect("the bundle");
        let request = Request {
            repo: Repo::parse(repo).expect("a repo"),
            commit: commit.to_owned(),
            quest: QUEST.to_owned(),
            session: None,
        };
        fs::write(gate.join(REQUEST), request.json()).expect("the request");
        GateFolder::open(&scratch, QUEST, commit).expect("the gate's folder opens")
    }

    fn bundle(&self, range: &str) -> Vec<u8> {
        let path = self.root.path().join("made.bundle");
        git(
            &self.tree(),
            &[
                "bundle",
                "create",
                "--quiet",
                &path.to_string_lossy(),
                range,
            ],
        );
        fs::read(path).expect("the bundle reads")
    }

    fn places(&self) -> Places {
        Places {
            state: self.root.path().join("state"),
            forge: self
                .root
                .path()
                .join("forge")
                .to_string_lossy()
                .into_owned(),
            repos: vec![Repo::parse("github:kestrel/osprey").expect("a repo")],
        }
    }

    /// The gate run with a run that says what it ran and passes.
    fn gated(&self, gate: &GateFolder, commit: &str) -> Result<String, Refusal> {
        let git_program = super::super::fence::on_path("git").expect("git is on the PATH");
        let fence = Fence::Bare { git: git_program };
        let request = super::super::read_request(gate).map_err(refused)?;
        let mut checked_out = String::new();
        let mut run = |tree: &Path, _: &Path| {
            checked_out = git(tree, &["rev-parse", "HEAD"]);
            Ok(0)
        };
        gated(
            &fence,
            &self.places(),
            gate,
            &request,
            (QUEST, commit),
            &mut run,
        )
        .map(|_| checked_out)
    }
}

const QUEST: &str = "QEST-20261115-ABC123";

#[test]
fn a_bundle_of_commits_past_main_is_checked_out_and_run() {
    let forge = Forge::new();
    let head = committed(&forge.tree(), "three");
    let gate = forge.gate(&head, &forge.bundle("origin/main..HEAD"));
    assert_eq!(
        forge.gated(&gate, &head),
        Ok(head),
        "the commit asked for should be the one run"
    );
}

#[test]
fn a_gate_keeps_neither_its_clone_nor_its_bundle_once_it_ends() {
    let forge = Forge::new();
    let head = committed(&forge.tree(), "three");
    let gate = forge.gate(&head, &forge.bundle("origin/main..HEAD"));
    assert_eq!(forge.gated(&gate, &head), Ok(head.clone()), "the gate runs");
    let work = forge.places().work(&head);
    assert!(
        !work.join("tree").exists(),
        "the clone and checkout should go as the run ends"
    );
    assert!(
        !work.join(BUNDLE).exists(),
        "the bundle should go as the run ends"
    );
}

#[test]
fn a_repo_this_machine_doesnt_gate_is_refused_before_any_fetch() {
    let forge = Forge::new();
    let head = committed(&forge.tree(), "three");
    let gate = forge.gate_of(
        "github:stranger/flake",
        &head,
        &forge.bundle("origin/main..HEAD"),
    );
    let refusal = forge
        .gated(&gate, &head)
        .expect_err("another owner's repo should be refused");
    assert_eq!(refusal.0, REFUSED, "{}", refusal.1);
    assert!(
        !forge.places().state.join("mirrors").exists(),
        "nothing should be fetched for a repo it doesn't gate"
    );
}

#[test]
fn a_link_in_its_own_state_is_refused_and_what_it_names_untouched() {
    let forge = Forge::new();
    let head = committed(&forge.tree(), "three");
    let gate = forge.gate(&head, &forge.bundle("origin/main..HEAD"));
    let victim = forge.root.path().join("victim");
    let theirs = victim.join(head.clone());
    fs::create_dir_all(&theirs).expect("a folder of the person's");
    fs::write(theirs.join("kept"), "the person's").expect("a file of the person's");
    let state = forge.places().state;
    fs::create_dir_all(&state).expect("the state folder");
    std::os::unix::fs::symlink(&victim, state.join("work")).expect("a link");
    let refusal = forge
        .gated(&gate, &head)
        .expect_err("work as a link should be refused");
    assert_eq!(refusal.0, UNTRUSTED, "{}", refusal.1);
    assert_eq!(
        fs::read_to_string(theirs.join("kept")).expect("the file is still there"),
        "the person's",
        "what the link names should be untouched"
    );
}

/// The result `gate` holds, as words.
fn result_of(gate: &GateFolder) -> String {
    String::from_utf8(gate.read(RESULT, 1 << 20).expect("a result is left")).expect("words")
}

/// The journal's last line, or none where no journal was kept.
fn last_journalled(places: &Places) -> Option<serde_json::Value> {
    let journal = fs::read_to_string(places.state.join(JOURNAL)).ok()?;
    Some(serde_json::from_str(journal.lines().last()?).expect("a line of json"))
}

/// A forge whose state's work folder is a link, so the state isn't
/// trusted: the gate of its head, refused for it, and the refusal.
fn untrusted_refusal() -> (Forge, String, GateFolder, (u8, String)) {
    let forge = Forge::new();
    let head = committed(&forge.tree(), "three");
    let gate = forge.gate(&head, &forge.bundle("origin/main..HEAD"));
    let state = forge.places().state;
    fs::create_dir_all(&state).expect("the state folder");
    std::os::unix::fs::symlink(forge.root.path(), state.join("work")).expect("a link");
    let refusal = forge
        .gated(&gate, &head)
        .expect_err("work as a link should be refused");
    assert_eq!(refusal.0, UNTRUSTED, "{}", refusal.1);
    assert!(
        refusal.1.contains(&*state.to_string_lossy()),
        "the refusal itself names the folder: {}",
        refusal.1
    );
    (forge, head, gate, refusal)
}

#[test]
fn a_refusal_keeps_its_words_in_the_journal_and_tells_the_asker_only_its_kind() {
    let (forge, head, gate, (_, why)) = untrusted_refusal();
    let places = forge.places();
    fs::remove_file(places.state.join("work")).expect("the link goes");
    let found = left(
        &places,
        &gate,
        (QUEST, &head),
        None,
        Err((REFUSED, why.clone())),
    );
    let result = result_of(&gate);
    assert_eq!(found.said, refusal_said(REFUSED, true));
    assert!(
        result.contains(&found.said) && !result.contains(&*places.state.to_string_lossy()),
        "the asker is sent to the journal in fixed words, with no path of this machine's: {result}"
    );
    let line = last_journalled(&places).expect("a journal is kept");
    assert_eq!(line["why"].as_str(), Some(why.as_str()));
    assert_eq!(line["said"].as_str(), Some(found.said.as_str()));
}

#[test]
fn a_refusal_with_no_state_to_trust_keeps_no_journal_and_says_so() {
    let (forge, head, gate, refusal) = untrusted_refusal();
    let places = forge.places();
    let found = left(&places, &gate, (QUEST, &head), None, Err(refusal));
    let result = result_of(&gate);
    assert_eq!(found.exit, UNTRUSTED);
    assert_eq!(found.said, refusal_said(UNTRUSTED, false));
    assert!(
        result.contains(&found.said) && !result.contains(&*places.state.to_string_lossy()),
        "the asker is told no journal line was kept, with no path of this machine's: {result}"
    );
    assert!(
        last_journalled(&places).is_none(),
        "no journal is kept in a state not trusted"
    );
}

#[test]
fn a_refusal_whose_journal_line_cant_be_written_says_it_was_printed() {
    let (forge, head, gate, (_, why)) = untrusted_refusal();
    let places = forge.places();
    fs::remove_file(places.state.join("work")).expect("the link goes");
    // A folder where the journal goes, so its line can't be added.
    fs::create_dir(places.state.join(JOURNAL)).expect("a folder in the journal's way");
    let found = left(&places, &gate, (QUEST, &head), None, Err((REFUSED, why)));
    assert_eq!(found.said, refusal_said(REFUSED, false));
    assert!(result_of(&gate).contains(&found.said));
}

#[test]
fn a_refused_gate_carries_back_no_earlier_runs_log_and_one_that_ran_does() {
    let (forge, head, gate, (_, why)) = untrusted_refusal();
    let places = forge.places();
    fs::remove_file(places.state.join("work")).expect("the link goes");
    fs::create_dir_all(places.work(&head)).expect("an earlier run's folder");
    fs::write(places.work(&head).join(LOG), "an earlier run").expect("its log");
    left(&places, &gate, (QUEST, &head), None, Err((REFUSED, why)));
    assert!(!gate.holds(LOG), "a refusal ran nothing, so carries no log");
    let ran = left(&places, &gate, (QUEST, &head), None, Ok(0));
    assert_eq!(ran.said, said_of(0));
    assert!(
        result_of(&gate).contains(&said_of(0)),
        "the result it leaves says it passed"
    );
    assert!(
        gate.holds(LOG),
        "a gate that ran carries its run's log back"
    );
}

#[test]
fn each_kind_of_refusal_says_words_of_its_own_wherever_its_detail_went() {
    for journalled in [true, false] {
        let kinds =
            [REFUSED, NOT_NOW, UNREACHED, UNTRUSTED].map(|exit| refusal_said(exit, journalled));
        assert!(
            kinds
                .iter()
                .enumerate()
                .all(|(at, said)| !kinds[..at].contains(said)),
            "{kinds:?}"
        );
    }
}

#[test]
fn a_bundle_of_another_commit_is_refused() {
    let forge = Forge::new();
    committed(&forge.tree(), "three");
    let gate = forge.gate(&forge.main, &forge.bundle("origin/main..HEAD"));
    let refusal = forge
        .gated(&gate, &forge.main)
        .expect_err("another commit should be refused");
    assert_eq!(
        refusal.0, REFUSED,
        "a wrong hash should be refused as bad data: {}",
        refusal.1
    );
}

#[test]
fn a_bundle_with_a_corrupted_object_is_refused() {
    let forge = Forge::new();
    let head = committed(&forge.tree(), "three");
    let mut bundle = forge.bundle("origin/main..HEAD");
    let last = bundle.len() - 30;
    bundle[last] ^= 0xff;
    let gate = forge.gate(&head, &bundle);
    let refusal = forge
        .gated(&gate, &head)
        .expect_err("a corrupted bundle should be refused");
    assert_eq!(
        refusal.0, REFUSED,
        "a corrupted object should be refused as bad data: {}",
        refusal.1
    );
}

#[test]
fn a_commit_not_descending_from_main_is_refused() {
    let forge = Forge::new();
    let tree = forge.tree();
    git(&tree, &["checkout", "--quiet", "--orphan", "apart"]);
    let apart = committed(&tree, "apart");
    let mut bundle = forge.bundle("HEAD");
    // A header naming main's commit as needed, over a pack that doesn't
    // descend from it, as a bundle made to pass the header's check is.
    let needed = format!("-{} two\n", forge.main);
    let line = bundle
        .iter()
        .position(|byte| *byte == b'\n')
        .expect("a version line")
        + 1;
    bundle.splice(line..line, needed.bytes());
    let gate = forge.gate(&apart, &bundle);
    let refusal = forge
        .gated(&gate, &apart)
        .expect_err("a commit apart from main should be refused");
    assert_eq!(
        refusal.0, REFUSED,
        "a commit off main should be refused as bad data: {}",
        refusal.1
    );
}

#[test]
fn a_request_for_another_commit_than_started_for_is_refused() {
    let forge = Forge::new();
    let head = committed(&forge.tree(), "three");
    let gate = forge.gate(&head, &forge.bundle("origin/main..HEAD"));
    let refusal = forge
        .gated(&gate, &forge.main)
        .expect_err("a mismatched start should be refused");
    assert_eq!(refusal.0, REFUSED, "{}", refusal.1);
}

#[test]
fn a_commit_being_gated_is_not_gated_beside_itself() {
    let forge = Forge::new();
    let head = committed(&forge.tree(), "three");
    let gate = forge.gate(&head, &forge.bundle("origin/main..HEAD"));
    let locks = forge.places().locks();
    fs::create_dir_all(&locks).expect("the locks' folder");
    let _running = locked_now(&locks.join(format!("{head}.lock")))
        .expect("a lock")
        .expect("the lock is free");
    let refusal = forge
        .gated(&gate, &head)
        .expect_err("a second gate of a commit should wait");
    assert_eq!(refusal.0, NOT_NOW, "{}", refusal.1);
}

#[test]
fn no_more_than_two_gates_run_at_once() {
    let forge = Forge::new();
    let head = committed(&forge.tree(), "three");
    let gate = forge.gate(&head, &forge.bundle("origin/main..HEAD"));
    let locks = forge.places().locks();
    fs::create_dir_all(&locks).expect("the locks' folder");
    let _running: Vec<Held> = (0..AT_ONCE)
        .map(|at| {
            locked_now(&locks.join(format!("slot-{at}.lock")))
                .expect("a lock")
                .expect("the lock is free")
        })
        .collect();
    let refusal = forge
        .gated(&gate, &head)
        .expect_err("a third gate should wait");
    assert_eq!(refusal.0, NOT_NOW, "{}", refusal.1);
}
