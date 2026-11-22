//! elsewhere run, against a stand-in realm: mycelium's placement is a
//! file, the far machine is a folder with a home of its own, and mycelium's
//! run carries the quest's handover there, runs the command in it and
//! carries the handover back, as a shared quest would cross. A test sees
//! where a job ran, what it said, what came back and how it ended, with no
//! second machine. ssh and rsync are on the path only to say they were
//! called, since a job travels by mycelium alone.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU32, Ordering};

/// The quest whose handover carries a job in every test but one.
const QUEST: &str = "QEST-20261115-ABC123";

/// The quests' scratch under a home, where the harness keeps it.
const SCRATCHES: &str = ".cache/pi/agentic-harness.pi/quest-workspace";

/// mycelium: `place` answers `placed.json`; `share` is taken; `run` on a
/// machine named `gone` never runs, and otherwise the handover crosses to
/// the far home, the command runs there through a login shell that says
/// something of its own first, and the handover crosses back. Each
/// crossing keeps home what a shared quest does: anything hidden and any
/// folder a build fills. Every ask is kept in `asked`, a line each.
const MYCELIUM: &str = r#"#!/bin/sh
echo "$@" >> "$REALM/asked"
case "$1" in
  place) cat "$REALM/placed.json"; exit 0 ;;
  share) exit 0 ;;
  run) ;;
  *) exit 64 ;;
esac
case " $* " in *" machine=gone "*) echo "no link reaches gone" >&2; exit 255 ;; esac
while [ "$1" != -- ]; do shift; done
shift
here="$HOME/$SCRATCHES" far="$REALM/far/$SCRATCHES"
cross() {
  carried=$(mktemp -d "$REALM/carried.XXXXXX")
  cp -R "$1/." "$carried/"
  find "$carried" -mindepth 1 \( -name '.*' -o -name target -o -name node_modules \) \
    -prune -exec rm -rf {} +
  mkdir -p "$2" && cp -R "$carried/." "$2/" && rm -rf "$carried"
}
cross "$here" "$far"
echo "a login shell's welcome"
(cd "$REALM/far" && HOME="$REALM/far" "$@")
code=$?
cross "$far" "$here"
exit $code
"#;

/// ssh or rsync: says it was called, which no test wants.
const NEVER: &str = "#!/bin/sh\necho \"$0 $*\" >> \"$REALM/never\"\nexit 1\n";

/// A stand-in realm of the test's own: this machine and a far one, with
/// mycelium placing work on the far one or here.
struct Realm {
    root: PathBuf,
}

impl Realm {
    fn new(placed_far: bool) -> Self {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let root = std::env::temp_dir().join(format!(
            "elsewhere-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&root);
        for folder in ["far", "home/work/spec", "config/elsewhere", "bin"] {
            fs::create_dir_all(root.join(folder)).unwrap();
        }
        let realm = Realm { root };
        realm.program("mycelium", MYCELIUM);
        realm.program("ssh", NEVER);
        realm.program("rsync", NEVER);
        let machine = if placed_far { "far".to_owned() } else { here() };
        fs::write(
            realm.root.join("placed.json"),
            format!(
                r#"{{"Ok":{{"Placement":{{"placed":{{"machine":"{machine}","passed":[]}}}}}}}}"#
            ),
        )
        .unwrap();
        realm.settings("far");
        realm.git(&["init", "--quiet"]);
        realm
    }

    fn program(&self, name: &str, text: &str) {
        let path = self.root.join("bin").join(name);
        fs::write(&path, text).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    }

    /// Settings sending to `machine`, opting in `sh` as the kind `check`.
    fn settings(&self, machine: &str) {
        self.bringing(machine, &["out"]);
    }

    /// Settings as `settings` writes them, with `check` bringing back
    /// `brings_back`.
    fn bringing(&self, machine: &str, brings_back: &[&str]) {
        let settings = serde_json::json!({
            "machines": [machine],
            "scratch": ".cache/elsewhere",
            "mycelium": self.root.join("bin/mycelium"),
            "kinds": {"check": {
                "description": "a stand-in check",
                "programs": ["sh"],
                "setup": "FROM_SETUP=set; export FROM_SETUP",
                "excludes": ["build"],
                "brings_back": brings_back,
                "needs": ["reach=home"],
                "lasts": "5m",
            }},
        });
        fs::write(
            self.root.join("config/elsewhere/settings.json"),
            settings.to_string(),
        )
        .unwrap();
    }

    fn folder(&self) -> PathBuf {
        self.root.join("home/work/spec")
    }

    /// Writes `text` to `path` in the job's folder, and tracks it.
    fn tracked(&self, path: &str, text: &str) {
        self.untracked(path, text);
        self.git(&["add", "--force", path]);
    }

    fn untracked(&self, path: &str, text: &str) {
        let file = self.folder().join(path);
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(file, text).unwrap();
    }

    fn git(&self, words: &[&str]) {
        let status = Command::new("git")
            .args(words)
            .current_dir(self.folder())
            .status()
            .unwrap();
        assert!(status.success(), "git {words:?} failed");
    }

    fn run(&self, words: &[&str]) -> Output {
        self.run_in(words, Some(QUEST))
    }

    fn run_in(&self, words: &[&str], quest: Option<&str>) -> Output {
        let path = format!(
            "{}:{}",
            self.root.join("bin").display(),
            std::env::var("PATH").unwrap_or_default()
        );
        let mut command = Command::new(env!("CARGO_BIN_EXE_elsewhere"));
        command
            .args(words)
            .env("REALM", &self.root)
            .env("SCRATCHES", SCRATCHES)
            .env("HOME", self.root.join("home"))
            .env("XDG_CONFIG_HOME", self.root.join("config"))
            .env_remove("XDG_CACHE_HOME")
            .env_remove("PI_QUEST")
            .env("PATH", path)
            .current_dir(self.folder());
        if let Some(quest) = quest {
            command.env("PI_QUEST", quest);
        }
        command.output().unwrap()
    }

    /// Runs the shell line `job` as a `check`, with `options`.
    fn check(&self, options: &[&str], job: &str) -> Output {
        let mut words = vec!["run", "check"];
        words.extend_from_slice(options);
        words.extend_from_slice(&["--", "/bin/sh", "-c", job]);
        self.run(&words)
    }

    fn asked(&self) -> String {
        fs::read_to_string(self.root.join("asked")).unwrap_or_default()
    }

    /// What ssh and rsync were called for, which should be nothing.
    fn never(&self) -> String {
        fs::read_to_string(self.root.join("never")).unwrap_or_default()
    }

    /// The quest's handover here, which a job leaves as it found it.
    fn handover(&self) -> PathBuf {
        self.root
            .join("home")
            .join(SCRATCHES)
            .join(QUEST)
            .join("handover/elsewhere")
    }
}

impl Drop for Realm {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

/// This machine's name as elsewhere reads it.
fn here() -> String {
    let output = Command::new("uname").arg("-n").output().unwrap();
    let name = String::from_utf8_lossy(&output.stdout);
    name.trim().split('.').next().unwrap_or_default().to_owned()
}

fn said(output: &Output) -> (String, String) {
    (
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

/// Whether `folder` holds nothing, or isn't there.
fn emptied(folder: &Path) -> bool {
    fs::read_dir(folder).map_or(true, |mut entries| entries.next().is_none())
}

#[test]
fn a_job_placed_far_runs_there_by_mycelium_and_brings_back_its_output() {
    let realm = Realm::new(true);
    realm.tracked("Spec.tla", "spec\n");
    let output = realm.check(
        &[],
        "pwd; cat Spec.tla; echo \"$FROM_SETUP\"; mkdir out; echo 7 > out/n",
    );
    let (out, err) = said(&output);
    assert!(output.status.success(), "the far job failed: {err}");
    let far = realm.root.join("far/.cache/elsewhere");
    assert!(
        out.lines()
            .next()
            .is_some_and(|ran_in| Path::new(ran_in).starts_with(&far)),
        "the job didn't run in a copy under the far scratch: {out}"
    );
    assert!(
        out.contains("spec\nset\n"),
        "the job didn't see its tree or its setup: {out}"
    );
    assert!(
        !out.contains("welcome") && err.contains("welcome"),
        "what the far shell said of its own wasn't kept off the job's output: {out}"
    );
    assert!(
        err.contains("check runs on far"),
        "it didn't say where the job ran: {err}"
    );
    let asked = realm.asked();
    assert!(
        asked.contains("place --need reach=home --lasts 5m\n"),
        "mycelium wasn't asked to place the kind as it says it needs: {asked}"
    );
    assert!(
        asked.contains(&format!("share {QUEST}")) && asked.contains("run --need machine=far"),
        "the job didn't go by the quest's handover and mycelium's run: {asked}"
    );
    assert!(
        asked.contains(&format!("share {QUEST} --to far --wait --within 120\n")),
        "the share wasn't waited for at the far machine as long as its files take: {asked}"
    );
    assert_eq!(realm.never(), "", "ssh or rsync was called");
    assert_eq!(
        fs::read_to_string(realm.folder().join("out/n"))
            .ok()
            .as_deref(),
        Some("7\n"),
        "what the kind brings back didn't come back"
    );
    assert!(
        emptied(&far) && emptied(&realm.handover()),
        "the far copy or the handover was left behind"
    );
}

#[test]
fn a_job_clears_what_runs_here_left_but_never_a_folder_a_run_holds() {
    let realm = Realm::new(true);
    realm.tracked("Spec.tla", "spec\n");
    let handover = realm.handover();
    let left = handover.join("1-1");
    fs::create_dir_all(left.join("tree")).unwrap();
    fs::write(left.join("tree/Spec.tla"), "spec\n").unwrap();
    fs::write(left.join(".elsewhere-owner"), format!("{}\t1\n", here())).unwrap();
    let running = handover.join("2-2");
    fs::create_dir_all(&running).unwrap();
    let owner = running.join(".elsewhere-owner");
    fs::write(&owner, format!("{}\t2\n", here())).unwrap();
    let held = fs::File::open(&owner).unwrap();
    held.lock().unwrap();
    let output = realm.check(&[], "cat Spec.tla");
    let (out, err) = said(&output);
    assert!(output.status.success(), "the far job failed: {err}");
    assert_eq!(out, "spec\n", "the job didn't see its tree");
    let far_handover = realm
        .root
        .join("far")
        .join(SCRATCHES)
        .join(QUEST)
        .join("handover/elsewhere");
    assert!(
        !far_handover.join("1-1").exists(),
        "the folder a killed run left was carried again"
    );
    assert!(
        !left.exists(),
        "the folder a killed run left wasn't cleared"
    );
    assert!(running.exists(), "a folder a run still holds was cleared");
    assert!(
        owner.exists(),
        "a crossing took away the owner file a run holds"
    );
    assert!(
        err.contains(&format!(
            "cleared 1 spent job folder from {QUEST}'s handover"
        )),
        "it didn't say what it cleared: {err}"
    );
}

#[test]
fn nothing_travels_but_the_job_s_tracked_files() {
    let realm = Realm::new(true);
    realm.tracked("Spec.tla", "spec\n");
    realm.tracked(".env", "TOKEN=1\n");
    realm.tracked("cert.pem", "pem\n");
    realm.tracked("build/big", "built\n");
    realm.untracked("id.key", "key\n");
    realm.untracked("notes.txt", "mine\n");
    let output = realm.check(
        &[],
        "find \"$HOME\" -type f \\( -name .env -o -name cert.pem -o -name big \
         -o -name id.key -o -name notes.txt \\); ls",
    );
    let (out, err) = said(&output);
    assert!(output.status.success(), "the far job failed: {err}");
    assert_eq!(
        out.trim(),
        "Spec.tla",
        "something but the tracked work reached the far machine: {out}"
    );
}

#[test]
fn tracked_files_in_folders_a_build_would_fill_still_travel() {
    let realm = Realm::new(true);
    realm.tracked("lib/target/x.ts", "source\n");
    realm.tracked("lib/target/y.ts", "more\n");
    realm.tracked("web/node_modules/a.js", "vendored\n");
    realm.tracked("a b/c d.txt", "spaced\n");
    realm.untracked("bin/go", "#!/bin/sh\necho went\n");
    fs::set_permissions(
        realm.folder().join("bin/go"),
        fs::Permissions::from_mode(0o755),
    )
    .unwrap();
    realm.git(&["add", "bin/go"]);
    let output = realm.check(
        &[],
        "cat lib/target/x.ts lib/target/y.ts web/node_modules/a.js 'a b/c d.txt'; bin/go; \
         mkdir -p out/target; echo 9 > out/target/n",
    );
    let (out, err) = said(&output);
    assert!(output.status.success(), "the far job failed: {err}");
    assert!(
        err.contains("check runs on far") && !err.contains("runs here"),
        "the job didn't run far: {err}"
    );
    assert_eq!(
        out, "source\nmore\nvendored\nspaced\nwent\n",
        "tracked files under a build's folder, with spaces or runnable, didn't reach the job whole"
    );
    assert_eq!(
        fs::read_to_string(realm.folder().join("out/target/n"))
            .ok()
            .as_deref(),
        Some("9\n"),
        "what the kind brings back under a build's folder didn't come back"
    );
}

#[test]
fn a_far_job_s_failure_is_its_own_code() {
    let realm = Realm::new(true);
    let output = realm.check(&[], "echo broke >&2; exit 3");
    let (_, err) = said(&output);
    assert_eq!(
        output.status.code(),
        Some(3),
        "the far job's code wasn't passed through: {err}"
    );
    assert!(
        err.contains("broke"),
        "the far job's stderr was lost: {err}"
    );
    let ran_255 = realm.check(&[], "echo ran; exit 255");
    let (out, err) = said(&ran_255);
    assert_eq!(
        (ran_255.status.code(), out.as_str()),
        (Some(255), "ran\n"),
        "a far job ending 255 was taken for one that never ran: {err}"
    );
}

#[test]
fn a_job_placed_here_runs_here() {
    let realm = Realm::new(false);
    let output = realm.check(&[], "pwd");
    let (out, err) = said(&output);
    assert!(output.status.success(), "the job here failed: {err}");
    assert_eq!(
        fs::canonicalize(out.trim()).ok(),
        fs::canonicalize(realm.folder()).ok(),
        "a job placed here was sent away: {err}"
    );
    assert!(
        err.contains("check runs here"),
        "it didn't say the job stayed: {err}"
    );
    assert!(
        !realm.asked().contains("run "),
        "it ran something far it didn't need: {}",
        realm.asked()
    );
    let told = realm.check(&["--there"], "pwd");
    assert!(
        said(&told).0.contains("/far/.cache/elsewhere/"),
        "--there didn't send the job away: {:?}",
        said(&told)
    );
}

#[test]
fn a_job_told_there_that_can_t_go_says_why_and_is_never_made_here() {
    let realm = Realm::new(true);
    realm.settings("gone");
    let output = realm.check(&["--there"], "pwd");
    let (out, err) = said(&output);
    assert_eq!(
        output.status.code(),
        Some(69),
        "a job --there couldn't send didn't end 69: {err}"
    );
    assert!(
        out.trim().is_empty() && !err.contains("runs here"),
        "a job --there couldn't send was made here: {out} {err}"
    );
    assert!(
        err.contains("couldn't be sent to gone") && err.contains("--here"),
        "it didn't say why the job couldn't go, or point at --here: {err}"
    );
    assert!(
        emptied(&realm.handover()),
        "the handover kept a job that never left"
    );
    // What another try meets again isn't the far side being away.
    let output = realm.run_in(
        &["run", "check", "--there", "--", "/bin/sh", "-c", "pwd"],
        None,
    );
    assert_eq!(
        output.status.code(),
        Some(78),
        "--there with no quest to carry it didn't end 78: {:?}",
        said(&output)
    );
    realm.settings("far");
    realm.tracked("named\twith a tab", "");
    let output = realm.check(&["--there"], "pwd");
    assert_eq!(
        output.status.code(),
        Some(77),
        "--there with a file it can't carry didn't end 77: {:?}",
        said(&output)
    );
}

#[test]
fn a_job_placed_far_that_never_got_there_runs_here_saying_why() {
    let realm = Realm::new(true);
    realm.settings("gone");
    fs::write(
        realm.root.join("placed.json"),
        r#"{"Ok":{"Placement":{"placed":{"machine":"gone","passed":[]}}}}"#,
    )
    .unwrap();
    let output = realm.check(&[], "pwd");
    let (out, err) = said(&output);
    assert_eq!(
        output.status.code(),
        Some(0),
        "a job that never got far wasn't made here: {err}"
    );
    assert!(
        out.trim().ends_with("/home/work/spec") && err.contains("check runs here"),
        "it wasn't made here in its own folder, saying so: {out} {err}"
    );
    assert!(
        err.contains("wasn't reached"),
        "it didn't say the far side was never reached: {err}"
    );
}

#[test]
fn a_job_with_no_quest_to_carry_it_runs_here() {
    let realm = Realm::new(true);
    let output = realm.run_in(&["run", "check", "--", "/bin/sh", "-c", "pwd"], None);
    let (out, err) = said(&output);
    assert!(output.status.success(), "the job didn't run here: {err}");
    assert_eq!(
        fs::canonicalize(out.trim()).ok(),
        fs::canonicalize(realm.folder()).ok(),
        "a job with no quest was sent away: {err}"
    );
    assert!(
        err.contains("check runs here") && err.contains("quest"),
        "it didn't say the job stayed for want of a quest: {err}"
    );
    assert!(
        !realm.asked().contains("share"),
        "it shared a quest it hadn't got"
    );
}

#[test]
fn a_job_its_kind_doesnt_allow_is_refused_before_anything_runs() {
    let realm = Realm::new(true);
    let output = realm.run(&["run", "check", "--", "rm", "-rf", "x"]);
    assert_eq!(
        output.status.code(),
        Some(77),
        "a program not opted in wasn't refused"
    );
    let output = realm.run(&["run", "nope", "--", "sh"]);
    let (_, err) = said(&output);
    assert_eq!(
        output.status.code(),
        Some(66),
        "an unknown kind didn't end 66: {err}"
    );
    assert!(
        err.contains("elsewhere kinds"),
        "it didn't point at the kinds: {err}"
    );
    assert_eq!(
        realm.asked(),
        "",
        "a refused job asked mycelium for something"
    );
}

#[test]
fn a_far_job_whose_output_can_t_land_fails_and_writes_nothing_outside() {
    for (job, ends) in [
        ("mkdir out; echo 7 > out/n", 77),
        ("mkdir out; echo 7 > out/n; exit 3", 3),
    ] {
        // A realm each, since the stand-in share never removes the far
        // copy of a handover the way a real one does.
        let realm = Realm::new(true);
        let outside = realm.root.join("outside");
        fs::create_dir_all(&outside).unwrap();
        std::os::unix::fs::symlink(&outside, realm.folder().join("out")).unwrap();
        let output = realm.check(&[], job);
        let (_, err) = said(&output);
        assert_eq!(
            output.status.code(),
            Some(ends),
            "a job whose output couldn't land didn't end {ends}: {err}"
        );
        assert!(
            err.contains("out/n would land through the link"),
            "it didn't say the output was refused for a link: {err}"
        );
        if ends == 77 {
            assert!(
                err.contains("was refused as it came home") && err.contains("--here"),
                "a passing job refused its output didn't say so, or point at --here: {err}"
            );
        }
        assert!(emptied(&outside), "what came back was written outside");
        assert!(
            emptied(&realm.handover()),
            "what couldn't land was left in the handover"
        );
    }
}

#[test]
fn a_far_job_whose_output_won_t_land_fails_saying_so() {
    let realm = Realm::new(true);
    // A folder where out/n would land, which no file replaces.
    fs::create_dir_all(realm.folder().join("out/n/kept")).unwrap();
    let output = realm.check(&[], "mkdir out; echo 7 > out/n");
    let (_, err) = said(&output);
    assert_eq!(
        output.status.code(),
        Some(77),
        "a job whose output didn't land didn't end 77: {err}"
    );
    for says in ["couldn't land here", "out/n didn't land", "--here"] {
        assert!(err.contains(says), "it didn't say {says}: {err}");
    }
}

#[test]
fn a_kind_bringing_back_what_can_t_come_home_runs_here_and_says_why() {
    let realm = Realm::new(true);
    realm.bringing("far", &["out/.cache"]);
    let here = realm.check(&["--here"], "pwd");
    assert!(
        here.status.success(),
        "a job run here was refused for what it would bring back: {:?}",
        said(&here)
    );
    let far = realm.check(&[], "pwd");
    let (_, err) = said(&far);
    assert!(
        far.status.success(),
        "a job bringing back a hidden path didn't run here: {err}"
    );
    assert!(
        err.contains("runs here") && err.contains("out/.cache"),
        "it didn't say it ran here for the path: {err}"
    );
    let (out, err) = said(&realm.run(&["where", "check", "--json"]));
    let place: serde_json::Value = serde_json::from_str(&out).expect(&err);
    assert!(
        place["machine"].is_null() && place["because"] == "cant-come-home",
        "where didn't say it stays because it can't come home: {out}"
    );
    // Told to send it, it isn't made here instead.
    let sent = realm.check(&["--there"], "pwd");
    let (_, err) = said(&sent);
    assert_eq!(
        sent.status.code(),
        Some(78),
        "--there with what can't come home didn't end 78: {err}"
    );
    assert!(
        err.contains("out/.cache") && err.contains("--here") && !err.contains("runs here"),
        "--there didn't say why it can't send, or ran it here: {err}"
    );
    assert!(
        !realm.asked().contains("run "),
        "a job that can't bring back ran far: {}",
        realm.asked()
    );
}

#[test]
fn a_command_asked_wrongly_says_what_it_takes() {
    let realm = Realm::new(true);
    for (words, says) in [
        (&["where"][..], "where takes one kind"),
        (&["kinds", "extra"][..], "kinds takes no more words"),
        (&["run", "check", "--her", "--", "sh"][..], "--here"),
        (&["run", "check", "--"][..], "no command after --"),
    ] {
        let output = realm.run(words);
        let (_, err) = said(&output);
        assert_eq!(
            output.status.code(),
            Some(64),
            "{words:?} didn't end 64: {err}"
        );
        assert!(err.contains(says), "{words:?} didn't say {says}: {err}");
    }
    let output = realm.run(&["kind", "--json"]);
    assert!(
        output.status.success(),
        "kind wasn't taken for kinds: {:?}",
        said(&output)
    );
}

#[test]
fn where_and_kinds_say_it_as_json() {
    let realm = Realm::new(true);
    let (out, err) = said(&realm.run(&["where", "check", "--json"]));
    let place: serde_json::Value = serde_json::from_str(&out).expect(&err);
    assert_eq!(
        place["machine"], "far",
        "where didn't name the far machine: {out}"
    );
    assert_eq!(place["because"], "placed", "where didn't say why: {out}");
    let (out, err) = said(&realm.run(&["kinds", "--json"]));
    let kinds: serde_json::Value = serde_json::from_str(&out).expect(&err);
    assert_eq!(
        kinds["kinds"][0]["programs"][0], "sh",
        "kinds didn't list the programs: {out}"
    );
}
