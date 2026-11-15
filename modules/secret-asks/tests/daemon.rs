//! The daemon run over a folder of its own, with stand-ins for 1Password's
//! op and logind's loginctl, a git tree for asks to run in, and asked
//! over its socket as the launcher asks it. Long because every case,
//! a yes, a no, a grant or the forge, is one walk through the same daemon
//! and the same stand-ins, which the file's helpers set up once.

use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixDatagram;
use std::path::{Path, PathBuf};
use std::process::Command;

use daemon_client::{Client, Daemon, until};
use serde_json::{Value, json};

/// The one item the daemon may release.
const ITEM: &str = "op://Private/Release Test/password";
/// The launcher's vocabulary, which the daemon says its rows in.
const VOCABULARY: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../quickshell/launcher/words.json"
);
/// What the stand-in op says the item holds.
const VALUE: &str = "this-is-not-a-secret";

/// A program written as a shell script, for a stand-in.
fn script(path: &Path, body: &str) {
    std::fs::write(path, format!("#!/bin/sh\n{body}\n")).expect("the stand-in is written");
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))
        .expect("the stand-in runs");
}

/// A program's path on the test's PATH, since the daemon runs git with
/// none of its own.
fn on_path(program: &str) -> PathBuf {
    std::env::var_os("PATH")
        .into_iter()
        .flat_map(|path| std::env::split_paths(&path).collect::<Vec<_>>())
        .map(|folder| folder.join(program))
        .find(|path| path.is_file())
        .unwrap_or_else(|| panic!("{program} isn't on the test's PATH"))
}

fn git(tree: &Path, args: &[&str]) {
    let ran = Command::new("git")
        .arg("-C")
        .arg(tree)
        .args(["-c", "user.name=t", "-c", "user.email=t@example.com"])
        .args(args)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .output()
        .expect("git runs");
    assert!(ran.status.success(), "git {args:?} failed: {ran:?}");
}

/// The daemon over a fresh folder: its asks, its socket, a committed
/// tree, and the stand-ins, op leaving a mark each time it runs.
struct Setting {
    daemon: Daemon,
    folder: PathBuf,
    /// A stand-in for journald's socket, which the readings' lines reach.
    journal: UnixDatagram,
}

impl Setting {
    fn new(name: &str) -> Setting {
        Setting::after(name, |_| ())
    }

    /// The daemon over a fresh folder, started once `before` has done
    /// what it would find there.
    fn after(name: &str, before: impl FnOnce(&Path)) -> Setting {
        let folder =
            std::env::temp_dir().join(format!("secret-asks-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(folder.join("tree")).expect("the folder is made");
        git(&folder.join("tree"), &["init", "-q"]);
        std::fs::write(folder.join("tree/README"), "a tree").expect("a file is written");
        git(&folder.join("tree"), &["add", "README"]);
        git(&folder.join("tree"), &["commit", "-q", "-m", "a commit"]);
        on_main(&folder, &folder.join("tree"), "tree");
        let marks = folder.join("op-ran");
        script(
            &folder.join("op"),
            &format!(
                "echo \"$@\" >> '{}'\n[ \"$3\" = '{ITEM}' ] || exit 1\nprintf '%s' '{VALUE}'",
                marks.display()
            ),
        );
        std::fs::write(folder.join("locked"), "no").expect("the lock is written");
        script(
            &folder.join("loginctl"),
            &format!(
                "case \"$1\" in show-user) echo 2 ;; show-session) cat '{}' ;; esac",
                folder.join("locked").display()
            ),
        );
        before(&folder);
        let socket = folder.join("secret-asks.sock");
        let journal =
            UnixDatagram::bind(folder.join("journal.sock")).expect("the journal stand-in binds");
        // A reading comes as the act is answered, so a few seconds is long.
        journal
            .set_read_timeout(Some(std::time::Duration::from_secs(5)))
            .expect("the journal stand-in waits a while");
        let mut command = Command::new(env!("CARGO_BIN_EXE_secret-asks"));
        command
            .arg("serve")
            .arg("--socket")
            .arg(&socket)
            .arg("--asks")
            .arg(folder.join("asks"))
            .arg("--vocabulary")
            .arg(VOCABULARY)
            .arg("--item")
            .arg(ITEM)
            .arg("--op")
            .arg(folder.join("op"))
            .arg("--git")
            .arg(on_path("git"))
            .arg("--forge")
            .arg(format!("file://{}/", folder.join("forge").display()))
            .arg("--loginctl")
            .arg(folder.join("loginctl"))
            .arg("--journal")
            .arg(folder.join("journal.sock"));
        let daemon = Daemon::start(&mut command, socket, folder.clone());
        Setting {
            daemon,
            folder,
            journal,
        }
    }

    /// The next reading the journal was told, once one has come.
    fn reading(&self) -> String {
        let mut line = [0_u8; 4096];
        let length = self
            .journal
            .recv(&mut line)
            .expect("a reading reached the journal");
        String::from_utf8_lossy(&line[..length]).into_owned()
    }

    /// Puts up an ask for `item` and answers its id.
    fn ask(&self, item: &str) -> String {
        put_up(&self.folder, item)
    }

    /// A client watching Asks' rows.
    fn watching(&self) -> Client {
        let mut client = self.daemon.client();
        client.ask(&json!({"id": "w", "ask": "watch", "kind": "row", "place": "asks"}));
        client
    }

    /// The answer written for an ask, once it's there.
    fn answer(&self, id: &str) -> Value {
        let path = self.folder.join("asks").join(format!("{id}.answer.json"));
        until("no answer was written", || path.exists());
        serde_json::from_str(&std::fs::read_to_string(path).expect("the answer reads"))
            .expect("the answer is JSON")
    }

    /// How many times op ran.
    fn op_ran(&self) -> usize {
        std::fs::read_to_string(self.folder.join("op-ran")).map_or(0, |marks| marks.lines().count())
    }
}

/// Puts up an ask for `item` in the setting's folder, daemon or not, and
/// answers its id.
fn put_up(folder: &Path, item: &str) -> String {
    put_up_in(folder, &folder.join("tree"), item)
}

/// Puts up an ask for `item` to run in `tree`, and answers its id.
fn put_up_in(folder: &Path, tree: &Path, item: &str) -> String {
    let ran = Command::new(env!("CARGO_BIN_EXE_secret-asks"))
        .args(["ask", "--item", item, "--why", "To try the line.", "--asks"])
        .arg(folder.join("asks"))
        .arg("--tree")
        .arg(tree)
        .output()
        .expect("the ask is put up");
    assert!(ran.status.success(), "the ask wasn't put up: {ran:?}");
    String::from_utf8_lossy(&ran.stdout).trim().to_owned()
}

/// The row of an ask, once a watch is told of it.
fn row(client: &mut Client, id: &str) -> Value {
    client.changed(&format!("row:asks:{id}"))["row"].clone()
}

/// Says yes to an ask once its row is drawn, and waits for what it ran;
/// answers the row the yes was to.
fn said_yes(setting: &Setting, client: &mut Client, id: &str) -> Value {
    let drawn = row(client, id);
    assert_eq!(
        act(client, &drawn, 0)["releasing"],
        true,
        "the yes wasn't taken"
    );
    assert_eq!(setting.answer(id)["state"], "ran", "the yes didn't run");
    drawn
}

/// The grants Asks lists, as the daemon serves them now.
fn grants(client: &mut Client) -> Vec<Value> {
    let held = client.ask(&json!({"id": 1, "ask": "list", "kind": "row"}));
    held["records"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|record| record["row"].clone())
        .filter(|row| row["kind"] == "grant")
        .collect()
}

/// A bare repository named `name` on the setting's stand-in forge.
fn forged(folder: &Path, name: &str) -> String {
    let forge = folder.join("forge").join(format!("{name}.git"));
    git(
        folder,
        &[
            "init",
            "-q",
            "--bare",
            "--initial-branch=main",
            &forge.to_string_lossy(),
        ],
    );
    format!("file://{}", forge.display())
}

/// Has the tree's origin be a repository on the stand-in forge whose
/// main holds the tree's commit, fetched, as a tree on main is.
fn on_main(folder: &Path, tree: &Path, name: &str) {
    let origin = forged(folder, name);
    git(tree, &["remote", "add", "origin", &origin]);
    git(tree, &["push", "-q", "origin", "HEAD:refs/heads/main"]);
    git(tree, &["fetch", "-q", "origin"]);
    git(tree, &["remote", "set-head", "origin", "main"]);
}

/// A worktree of the setting's tree holding a commit the forge's main
/// doesn't, an agent's later edit Joel never saw.
fn unmerged(setting: &Setting, name: &str) -> PathBuf {
    let worktree = setting.folder.join(name);
    git(
        &setting.folder.join("tree"),
        &[
            "worktree",
            "add",
            "-q",
            "--detach",
            &worktree.to_string_lossy(),
        ],
    );
    std::fs::write(worktree.join("release.sh"), "echo edited").expect("a file is written");
    git(&worktree, &["add", "release.sh"]);
    git(&worktree, &["commit", "-q", "-m", "an unmerged commit"]);
    worktree
}

/// The fact a row says under `name`, if it says one.
fn fact(row: &Value, name: &str) -> Option<Value> {
    row["drawer"]["facts"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|fact| fact["name"] == name)
        .map(|fact| fact["said"].clone())
}

/// A repository of its own beside the setting's tree, one commit in.
fn another_repository(folder: &Path) -> PathBuf {
    let other = folder.join("other");
    std::fs::create_dir_all(&other).expect("the other tree is made");
    git(&other, &["init", "-q"]);
    std::fs::write(other.join("README"), "another tree").expect("a file is written");
    git(&other, &["add", "README"]);
    git(&other, &["commit", "-q", "-m", "a commit"]);
    on_main(folder, &other, "other");
    other
}

#[test]
fn a_yes_lets_the_same_ask_from_any_tree_of_its_repository_run_for_a_week() {
    let setting = Setting::new("granted");
    let mut client = setting.watching();
    let first = setting.ask(ITEM);
    let drawn = said_yes(&setting, &mut client, &first);
    assert_eq!(
        fact(&drawn, "Yes Lasts"),
        Some(json!("a week, from any tree of tree on main")),
        "the line didn't say what a yes lets run: {drawn}"
    );
    assert!(
        setting.reading().contains("ASK_OUTCOME=released\n"),
        "the yes wasn't read"
    );
    let worktree = setting.folder.join("worktree");
    git(
        &setting.folder.join("tree"),
        &["worktree", "add", "-q", &worktree.to_string_lossy()],
    );
    let again = put_up_in(&setting.folder, &worktree, ITEM);
    let answer = setting.answer(&again);
    assert_eq!(answer["state"], "ran", "the grant didn't run it: {answer}");
    assert_eq!(
        answer["granted"], true,
        "the answer doesn't say so: {answer}"
    );
    let reading = setting.reading();
    assert!(
        reading.contains("ASK_OUTCOME=granted\n"),
        "a release by grant isn't read as one: {reading}"
    );
    assert_eq!(setting.op_ran(), 2, "op didn't run for each");
}

#[test]
fn a_yes_lets_the_same_ask_already_waiting_run_too() {
    let setting = Setting::new("granted-waiting");
    let mut client = setting.watching();
    let first = setting.ask(ITEM);
    row(&mut client, &first);
    let waiting = setting.ask(ITEM);
    assert_eq!(
        row(&mut client, &waiting)["state"],
        "waiting",
        "the second ask didn't wait before any yes"
    );
    let drawn = client.ask(&json!({"id": 1, "ask": "list", "kind": "row"}))["records"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|record| record["row"].clone())
        .find(|row| row["thing"] == format!("ask:{first}"))
        .expect("the first ask's row is listed");
    assert_eq!(
        act(&mut client, &drawn, 0)["releasing"],
        true,
        "the yes wasn't taken"
    );
    let answer = setting.answer(&waiting);
    assert_eq!(
        answer["state"], "ran",
        "the same ask already waiting still waited: {answer}"
    );
    assert_eq!(
        answer["granted"], true,
        "the answer doesn't say so: {answer}"
    );
}

#[test]
fn a_grant_covers_no_tree_whose_code_isnt_all_on_main() {
    let setting = Setting::new("granted-unlanded");
    let mut client = setting.watching();
    let first = setting.ask(ITEM);
    said_yes(&setting, &mut client, &first);
    let tree = setting.folder.join("tree");
    let (unmerged, changed) = (
        setting.folder.join("unmerged"),
        setting.folder.join("changed"),
    );
    for worktree in [&unmerged, &changed] {
        git(
            &tree,
            &[
                "worktree",
                "add",
                "-q",
                "--detach",
                &worktree.to_string_lossy(),
            ],
        );
    }
    // An agent's later edit Joel never saw: a commit main doesn't hold,
    // and a change not yet committed.
    std::fs::write(unmerged.join("release.sh"), "echo edited").expect("a file is written");
    git(&unmerged, &["add", "release.sh"]);
    git(&unmerged, &["commit", "-q", "-m", "an unmerged commit"]);
    std::fs::write(changed.join("README"), "edited").expect("a file is written");
    for worktree in [&unmerged, &changed] {
        let id = put_up_in(&setting.folder, worktree, ITEM);
        let drawn = row(&mut client, &id);
        assert_eq!(
            drawn["state"],
            "waiting",
            "a grant covered {} though its code isn't on main",
            worktree.display()
        );
        assert_eq!(
            fact(&drawn, "Standing Yes"),
            Some(json!(
                "doesn't cover this, since its code isn't all on main"
            )),
            "the row doesn't say why the standing yes waits: {drawn}"
        );
    }
    assert_eq!(setting.op_ran(), 1, "op ran for code not on main");
}

#[test]
fn a_grant_covers_no_commit_only_the_trees_own_refs_say_is_on_main() {
    let setting = Setting::new("granted-forged");
    let mut client = setting.watching();
    let first = setting.ask(ITEM);
    said_yes(&setting, &mut client, &first);
    let worktree = unmerged(&setting, "forged");
    // What any command of the account can write: origin's refs as last
    // fetched, claiming the commit is on main though the forge never
    // took it.
    git(
        &worktree,
        &["update-ref", "refs/remotes/origin/main", "HEAD"],
    );
    let id = put_up_in(&setting.folder, &worktree, ITEM);
    assert_eq!(
        row(&mut client, &id)["state"],
        "waiting",
        "a grant covered a commit only a forged ref says is on main"
    );
    assert_eq!(setting.op_ran(), 1, "op ran for a forged main");
}

#[test]
fn a_grant_covers_no_tree_whose_origin_was_pointed_elsewhere() {
    let setting = Setting::new("granted-repointed");
    let mut client = setting.watching();
    let first = setting.ask(ITEM);
    said_yes(&setting, &mut client, &first);
    let worktree = unmerged(&setting, "repointed");
    // A repository of the agent's own on the same forge, whose main does
    // hold its commit, named as origin and fetched from.
    let fork = forged(&setting.folder, "fork");
    git(&worktree, &["push", "-q", &fork, "HEAD:refs/heads/main"]);
    git(&worktree, &["remote", "set-url", "origin", &fork]);
    git(&worktree, &["fetch", "-q", "origin"]);
    let id = put_up_in(&setting.folder, &worktree, ITEM);
    assert_eq!(
        row(&mut client, &id)["state"],
        "waiting",
        "a grant covered a tree whose origin isn't the one Joel's yes was for"
    );
    assert_eq!(setting.op_ran(), 1, "op ran for another origin's main");
}

#[test]
fn a_grant_never_covers_another_repository() {
    let setting = Setting::new("granted-elsewhere");
    let mut client = setting.watching();
    let first = setting.ask(ITEM);
    said_yes(&setting, &mut client, &first);
    let other = another_repository(&setting.folder);
    let elsewhere = put_up_in(&setting.folder, &other, ITEM);
    assert_eq!(
        row(&mut client, &elsewhere)["state"],
        "waiting",
        "a grant covered another repository"
    );
    assert_eq!(setting.op_ran(), 1, "op ran for another repository");
}

#[test]
fn a_no_grants_nothing() {
    let setting = Setting::new("no-grant");
    let mut client = setting.watching();
    let first = setting.ask(ITEM);
    let drawn = row(&mut client, &first);
    assert_eq!(
        act(&mut client, &drawn, 1)["refused"],
        true,
        "the no wasn't taken"
    );
    let again = setting.ask(ITEM);
    assert_eq!(
        row(&mut client, &again)["state"],
        "waiting",
        "a no let the same ask through"
    );
    assert!(grants(&mut client).is_empty(), "a no left a grant");
}

#[test]
fn asks_lists_a_grant_and_revoking_it_has_the_same_ask_wait_again() {
    let setting = Setting::new("revoked");
    let mut client = setting.watching();
    let first = setting.ask(ITEM);
    said_yes(&setting, &mut client, &first);
    let listed = grants(&mut client);
    assert_eq!(listed.len(), 1, "the grant isn't listed once: {listed:?}");
    let grant = &listed[0];
    assert_eq!(
        grant["name"], ITEM,
        "the grant doesn't name its item: {grant}"
    );
    assert_eq!(
        grant["note"], "secret-asks received, from tree",
        "the grant doesn't say what it lets run where: {grant}"
    );
    assert_eq!(
        grant["acts"][0]["key"], "verb:grant:forget",
        "its one act isn't Forget It: {grant}"
    );
    assert_eq!(
        grant["acts"][0]["costly"], false,
        "forgetting a standing yes is asked to be sure of: {grant}"
    );
    // The row's going may be pushed before the revoke is answered, and an
    // ask passes over pushes, so both are read here in whichever order.
    let revoke = acting(grant, 0);
    client.send(&revoke);
    let (mut answer, mut gone) = (None, None);
    while answer.is_none() || gone.is_none() {
        let said = client.heard();
        if let Some(keys) = said["gone"].as_array() {
            gone = Some(keys.clone());
        } else if said["id"] == revoke["id"] && said.get("changed").is_none() {
            answer = Some(said);
        }
    }
    let (answer, gone) = (answer.unwrap_or_default(), gone.unwrap_or_default());
    assert_eq!(answer["revoked"], true, "the revoke wasn't taken: {answer}");
    assert!(
        gone.iter().any(|key| key.as_str() == grant["key"].as_str()),
        "the grant's row didn't go: {gone:?}"
    );
    assert!(grants(&mut client).is_empty(), "the grant stayed");
    let again = setting.ask(ITEM);
    assert_eq!(
        row(&mut client, &again)["state"],
        "waiting",
        "a revoked grant still let it through"
    );
    assert_eq!(setting.op_ran(), 1, "op ran under a revoked grant");
}

#[test]
fn a_grant_runs_while_the_screen_is_locked_since_joel_said_yes_unlocked() {
    let setting = Setting::new("granted-locked");
    let mut client = setting.watching();
    let first = setting.ask(ITEM);
    said_yes(&setting, &mut client, &first);
    std::fs::write(setting.folder.join("locked"), "yes").expect("the lock is written");
    let again = setting.ask(ITEM);
    assert_eq!(
        setting.answer(&again)["state"],
        "ran",
        "a grant waited on the lock"
    );
}

/// Asks the daemon what a row's act asks, as the launcher sends it.
fn act(client: &mut Client, row: &Value, at: usize) -> Value {
    client.ask(&acting(row, at))
}

/// The ask a row's act sends, as the launcher sends it once the line has
/// been on screen long enough to read.
fn acting(row: &Value, at: usize) -> Value {
    let mut asked = row["acts"][at]["ask"].clone();
    asked["id"] = json!(7);
    asked["shown"] = json!(812);
    asked
}

/// The answer to a row's act and the row it draws again, in whichever order
/// the daemon says them: asking alone passes over the watch's pushes, and
/// one the daemon sent before its answer would be lost.
fn act_and_row(client: &mut Client, row: &Value, at: usize, id: &str) -> (Value, Value) {
    let asked = acting(row, at);
    client.send(&asked);
    let key = format!("row:asks:{id}");
    let (mut answer, mut drawn) = (None, None);
    while answer.is_none() || drawn.is_none() {
        let said = client.heard();
        if said["id"] == asked["id"] && said.get("changed").is_none() {
            answer = Some(said);
        } else if let Some(record) = said["changed"]
            .as_array()
            .and_then(|records| records.iter().find(|record| record["key"] == key))
        {
            drawn = Some(record["row"].clone());
        }
    }
    (
        answer.expect("the act was answered"),
        drawn.expect("the row was drawn again"),
    )
}

#[test]
fn it_says_hello_as_the_fixture_does() {
    let setting = Setting::new("hello");
    let mut client = setting.daemon.client();
    let mut said = client.ask(&json!({"id": 1, "ask": "hello"}));
    let fixture: Value =
        serde_json::from_str(include_str!("fixture.json")).expect("the fixture is JSON");
    let mut hello = fixture["hello"].clone();
    // When it started is the one thing that differs between runs.
    said["since"] = json!(null);
    hello["since"] = json!(null);
    assert_eq!(said, hello, "its hello isn't the fixture's");
}

#[test]
fn a_yes_releases_the_item_to_the_checker_alone() {
    let setting = Setting::new("yes");
    let mut client = setting.watching();
    let id = setting.ask(ITEM);
    let drawn = row(&mut client, &id);
    assert_eq!(drawn["state"], "waiting", "the ask didn't wait");
    let mut yes = drawn["acts"][0]["ask"].clone();
    yes["id"] = json!(7);
    // How long the launcher says the line was on screen before the yes.
    yes["shown"] = json!(812);
    assert_eq!(client.ask(&yes)["releasing"], true, "the yes wasn't taken");
    let reading = setting.reading();
    for field in [
        "ASK_OUTCOME=released\n",
        "ASK_SHOWN_MS=812\n",
        "ASK_AVOIDABLE=unknown\n",
    ] {
        assert!(
            reading.contains(field),
            "the reading lacks {field}: {reading}"
        );
    }
    assert!(
        !reading.contains(VALUE) && !reading.contains("op://"),
        "the reading says the value or the item: {reading}"
    );
    let answer = setting.answer(&id);
    assert_eq!(answer["state"], "ran", "the checker didn't run: {answer}");
    assert_eq!(answer["exit"], 0, "the checker failed: {answer}");
    let output = answer["output"].as_str().expect("an output");
    assert_eq!(
        output.trim(),
        "RELEASED: received 20 characters",
        "the checker said: {output}"
    );
    assert!(
        !answer.to_string().contains(VALUE),
        "the value reached the answer"
    );
    assert_eq!(setting.op_ran(), 1, "op didn't run once");
    // The answer is written before the ask's file goes, so it's waited for.
    let ask = setting.folder.join("asks").join(format!("{id}.ask.json"));
    until("the ask's file stayed", || !ask.exists());
}

#[test]
fn an_ask_for_any_other_item_reads_nothing_and_draws_no_row() {
    let setting = Setting::new("other");
    let id = setting.ask("op://Private/Something Real/password");
    let answer = setting.answer(&id);
    assert_eq!(
        answer["state"], "refused",
        "another item wasn't refused: {answer}"
    );
    assert_eq!(answer["why"], "not listed", "it was refused for: {answer}");
    let reading = setting.reading();
    assert!(
        reading.contains("ASK_OUTCOME=unseen\n"),
        "an ask Joel never saw isn't read as unseen: {reading}"
    );
    let mut client = setting.daemon.client();
    let held = client.ask(&json!({"id": 1, "ask": "list", "kind": "row"}));
    assert_eq!(held["records"], json!([]), "another item drew a row");
    assert_eq!(setting.op_ran(), 0, "op ran for another item");
}

#[test]
fn a_yes_for_what_changed_since_it_was_drawn_is_refused_and_drawn_again() {
    let setting = Setting::new("changed");
    let mut client = setting.watching();
    let id = setting.ask(ITEM);
    let drawn = row(&mut client, &id);
    std::fs::write(setting.folder.join("tree/new"), "a change").expect("a change is made");
    let (said, again) = act_and_row(&mut client, &drawn, 0, &id);
    assert_eq!(
        said["error"]["code"], "changed",
        "a changed tree was released: {said}"
    );
    assert_eq!(
        said["error"]["said"], "it changed since it was shown; read it again",
        "the launcher can't say why: {said}"
    );
    assert_ne!(
        again["digest"], drawn["digest"],
        "the row wasn't drawn again"
    );
    assert_eq!(setting.op_ran(), 0, "op ran for a changed ask");
    assert_eq!(
        act(&mut client, &again, 0)["releasing"],
        true,
        "the new line's yes failed"
    );
}

#[test]
fn a_yes_while_locked_reads_nothing() {
    let setting = Setting::new("locked");
    let mut client = setting.watching();
    let id = setting.ask(ITEM);
    let drawn = row(&mut client, &id);
    std::fs::write(setting.folder.join("locked"), "yes").expect("the lock is written");
    let said = act(&mut client, &drawn, 0);
    assert_eq!(
        said["error"]["message"], "the screen is locked",
        "a locked yes: {said}"
    );
    assert_eq!(
        said["error"]["said"], "not while the screen is locked",
        "the launcher can't say why: {said}"
    );
    assert_eq!(setting.op_ran(), 0, "op ran while locked");
    let reading = setting.reading();
    assert!(
        reading.contains("ASK_OUTCOME=locked\n"),
        "a locked yes isn't read: {reading}"
    );
}

#[test]
fn an_act_names_the_ask_alone_and_a_yes_keyed_by_its_row_reads_nothing() {
    let setting = Setting::new("row-keyed");
    let mut client = setting.watching();
    let id = setting.ask(ITEM);
    let drawn = row(&mut client, &id);
    let thing = format!("ask:{id}");
    for at in [0, 1] {
        assert_eq!(
            drawn["acts"][at]["ask"]["key"], thing,
            "an act names more than its ask: {drawn}"
        );
    }
    // A second form of key would widen what a forged act could match, so
    // the row's own is refused.
    let mut yes = acting(&drawn, 0);
    yes["key"] = json!(format!("row:asks:{id}"));
    yes["shown"] = json!(812);
    let said = client.ask(&yes);
    assert_eq!(said["error"]["code"], "refused", "a row-keyed yes: {said}");
    assert_eq!(setting.op_ran(), 0, "op ran for a row-keyed yes");
}

#[test]
fn the_experiments_yes_needs_no_time_shown_since_its_asks_says_none() {
    let setting = Setting::new("untimed");
    let mut client = setting.watching();
    let id = setting.ask(ITEM);
    let drawn = row(&mut client, &id);
    // The experiment's Asks (Asks.qml) sends no `shown`; the half-second
    // floor is the passwords child's, whose launcher counts it
    // (modules/passwords/tests/asks).
    let mut yes = acting(&drawn, 0);
    yes["shown"] = Value::Null;
    let said = client.ask(&yes);
    assert_eq!(said["releasing"], true, "an untimed yes: {said}");
    assert_eq!(
        setting.answer(&id)["state"],
        "ran",
        "the untimed yes didn't run"
    );
}

#[test]
fn an_ask_a_restart_cut_off_while_releasing_is_ended_not_released_again() {
    let id = std::cell::RefCell::new(String::new());
    let setting = Setting::after("restarted", |folder| {
        let asked = put_up(folder, ITEM);
        // What the daemon leaves beside an ask as it releases it.
        std::fs::write(folder.join("asks").join(format!("{asked}.releasing")), "")
            .expect("the mark is left");
        *id.borrow_mut() = asked;
    });
    let id = id.into_inner();
    let answer = setting.answer(&id);
    assert_eq!(answer["state"], "ended", "a cut-off ask: {answer}");
    assert_eq!(
        answer["why"], "ended by a restart",
        "it ended for: {answer}"
    );
    let asks = setting.folder.join("asks");
    assert!(
        !asks.join(format!("{id}.ask.json")).exists()
            && !asks.join(format!("{id}.releasing")).exists(),
        "the ask or its mark stayed"
    );
    assert_eq!(setting.op_ran(), 0, "op ran for a cut-off ask");
}

#[test]
fn a_no_refuses_and_reads_nothing() {
    let setting = Setting::new("no");
    let mut client = setting.watching();
    let id = setting.ask(ITEM);
    let drawn = row(&mut client, &id);
    let (said, after) = act_and_row(&mut client, &drawn, 1, &id);
    assert_eq!(said["refused"], true, "the no wasn't taken: {said}");
    assert_eq!(
        setting.answer(&id)["why"],
        "refused by Joel",
        "the no wasn't answered"
    );
    let reading = setting.reading();
    assert!(
        reading.contains("ASK_OUTCOME=refused\n"),
        "a no isn't read: {reading}"
    );
    assert_eq!(after["acts"], json!([]), "a refused ask still offered acts");
    // A yes from the line as it was drawn, before the no, is refused.
    let late = act(&mut client, &drawn, 0);
    assert_eq!(
        late["error"]["said"], "that ask is already answered",
        "a yes after the no: {late}"
    );
    assert_eq!(setting.op_ran(), 0, "op ran for a no");
}

#[test]
fn a_tree_s_own_settings_start_no_program_as_it_is_read() {
    let setting = Setting::new("planted");
    let tree = setting.folder.join("tree");
    let planted = setting.folder.join("planted-ran");
    // Named whole, since git runs it with no PATH.
    let mark = format!(
        "{} '{}'; {}",
        on_path("touch").display(),
        planted.display(),
        on_path("cat").display()
    );
    // Whatever works in a tree writes its settings, so an agent could name
    // a clean filter for every file, a file monitor, and an external diff.
    std::fs::write(
        tree.join(".git/info/attributes"),
        "* filter=planted diff=planted\n",
    )
    .expect("the attributes are planted");
    git(&tree, &["config", "filter.planted.clean", &mark]);
    git(&tree, &["config", "diff.planted.textconv", &mark]);
    git(&tree, &["config", "core.fsmonitor", &mark]);
    // A file whose time moved but whose size didn't is one git must read
    // through its filter to say whether it changed.
    std::fs::File::options()
        .write(true)
        .open(tree.join("README"))
        .and_then(|file| file.set_modified(std::time::UNIX_EPOCH))
        .expect("the file's time is moved");
    let read = secret_asks::tree::read(&on_path("git"), &tree);
    assert!(
        !planted.exists(),
        "reading the tree ran its planted program"
    );
    assert!(
        read.commit.is_some(),
        "the tree's commit wasn't read: {read:?}"
    );
    assert_eq!(
        read.uncommitted,
        Some(0),
        "the tree didn't read as clean: {read:?}"
    );
}
