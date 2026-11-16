//! elsewhere run, against a stand-in realm: mycelium's answer is a file,
//! the far machine is a folder, ssh runs its line in that folder and rsync
//! copies with cp, so a test sees where a job ran, what it said, what came
//! back and how it ended, with no second machine.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU32, Ordering};

/// ssh: runs the line in the far home, as a far login does, unless the
/// machine is `gone`, which ssh can't reach.
const SSH: &str = r#"#!/bin/sh
[ "$3" = gone ] && exit 255
echo "$3" >> "$REALM/reached"
cd "$REALM/far" && exec sh -c "$4"
"#;

/// rsync: copies with cp, a far path being one under the far home.
const RSYNC: &str = r#"#!/bin/sh
set -e
words=""
while [ $# -gt 0 ]; do
  case "$1" in
    --rsync-path) shift ;;
    --*) ;;
    *) words="$words $1" ;;
  esac
  shift
done
set -- $words
from=$(echo "$1" | sed "s|^[a-z]*:|$REALM/far/|")
to=$(echo "$2" | sed "s|^[a-z]*:|$REALM/far/|")
case "$1" in gone:*) exit 255 ;; esac
mkdir -p "$to"
case "$from" in
  */) cp -R "$from." "$to" ;;
  *) cp -R "$from" "$to" ;;
esac
"#;

/// A stand-in realm of the test's own: this machine, crowded or not, and
/// a far one with room.
struct Realm {
    root: PathBuf,
}

impl Realm {
    fn new(crowded: bool) -> Self {
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
        realm.program("ssh", SSH);
        realm.program("rsync", RSYNC);
        realm.program("mycelium", "#!/bin/sh\ncat \"$REALM/machines.json\"\n");
        let over = if crowded { r#"["cpu"]"# } else { "[]" };
        fs::write(
            realm.root.join("machines.json"),
            format!(
                r#"{{"Ok":{{"Machines":[
                {{"name":"{}","state":"ready","room":{{"over":{over}}}}},
                {{"name":"far","state":"ready","room":{{"over":[]}}}}]}}}}"#,
                here()
            ),
        )
        .unwrap();
        realm.settings("far");
        realm
    }

    fn program(&self, name: &str, text: &str) {
        let path = self.root.join("bin").join(name);
        fs::write(&path, text).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    }

    /// Settings sending to `machine`, opting in `sh` as the kind `check`.
    fn settings(&self, machine: &str) {
        let bin = self.root.join("bin");
        let settings = serde_json::json!({
            "machines": [machine],
            "scratch": ".cache/elsewhere",
            "mycelium": bin.join("mycelium"),
            "ssh": bin.join("ssh"),
            "rsync": bin.join("rsync"),
            "kinds": {"check": {
                "description": "a stand-in check",
                "programs": ["sh"],
                "setup": "FROM_SETUP=set; export FROM_SETUP",
                "brings_back": ["out"],
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

    fn run(&self, words: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_elsewhere"))
            .args(words)
            .env("REALM", &self.root)
            .env("HOME", self.root.join("home"))
            .env("XDG_CONFIG_HOME", self.root.join("config"))
            .current_dir(self.folder())
            .output()
            .unwrap()
    }

    /// Runs the shell line `job` as a `check`, with `options`.
    fn check(&self, options: &[&str], job: &str) -> Output {
        let mut words = vec!["run", "check"];
        words.extend_from_slice(options);
        words.extend_from_slice(&["--", "/bin/sh", "-c", job]);
        self.run(&words)
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

fn far_copy(realm: &Realm) -> PathBuf {
    realm.root.join("far/.cache/elsewhere/work/spec")
}

#[test]
fn a_crowded_machine_runs_the_job_in_a_far_copy_and_brings_back_its_output() {
    let realm = Realm::new(true);
    fs::write(realm.folder().join("Spec.tla"), "spec\n").unwrap();
    let output = realm.check(
        &[],
        "pwd; cat Spec.tla; echo \"$FROM_SETUP\"; mkdir out; echo 7 > out/n",
    );
    let (out, err) = said(&output);
    assert!(output.status.success(), "the far job failed: {err}");
    let ran_in = PathBuf::from(out.lines().next().unwrap_or_default());
    assert_eq!(
        fs::canonicalize(&ran_in).ok(),
        fs::canonicalize(far_copy(&realm)).ok(),
        "the job didn't run in the far copy: {out}"
    );
    assert!(
        out.contains("spec\nset"),
        "the job didn't see its tree or its setup: {out}"
    );
    assert!(
        err.contains("check runs on far"),
        "it didn't say where the job ran: {err}"
    );
    assert_eq!(
        fs::read_to_string(realm.folder().join("out/n"))
            .ok()
            .as_deref(),
        Some("7\n"),
        "what the kind brings back didn't come back"
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
}

#[test]
fn a_machine_with_room_runs_the_job_here() {
    let realm = Realm::new(false);
    let output = realm.check(&[], "pwd");
    let (out, err) = said(&output);
    assert!(output.status.success(), "the job here failed: {err}");
    assert_eq!(
        fs::canonicalize(out.trim()).ok(),
        fs::canonicalize(realm.folder()).ok(),
        "a machine with room sent the job away: {err}"
    );
    assert!(
        err.contains("check runs here"),
        "it didn't say the job stayed: {err}"
    );
    assert!(
        !realm.root.join("reached").exists(),
        "it reached a far machine it didn't need"
    );
    let told = realm.check(&["--there"], "pwd");
    assert!(
        said(&told).0.trim().ends_with(".cache/elsewhere/work/spec"),
        "--there didn't send the job away: {:?}",
        said(&told)
    );
}

#[test]
fn a_far_machine_ssh_cant_reach_ends_69_saying_how_to_run_it_here() {
    let realm = Realm::new(true);
    realm.settings("gone");
    let output = realm.check(&["--there"], "true");
    let (_, err) = said(&output);
    assert_eq!(
        output.status.code(),
        Some(69),
        "an unreached machine didn't end 69: {err}"
    );
    assert!(
        err.contains("--here"),
        "it didn't say how to run the job here: {err}"
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
    assert!(
        !realm.root.join("reached").exists(),
        "a refused job reached the far machine"
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
    assert_eq!(place["because"], "crowded", "where didn't say why: {out}");
    let (out, err) = said(&realm.run(&["kinds", "--json"]));
    let kinds: serde_json::Value = serde_json::from_str(&out).expect(&err);
    assert_eq!(
        kinds["kinds"][0]["programs"][0], "sh",
        "kinds didn't list the programs: {out}"
    );
}
