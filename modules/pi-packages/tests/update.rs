//! An update, run against a package in a local repository, with the
//! git, node and pnpm on the PATH. The package has one dependency of its
//! own with a native-style build, so a build and its check run whole and
//! offline.

use std::collections::BTreeMap;
use std::fs::{self, File};
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, SystemTime};

use pi_packages::config::Config;
use pi_packages::run::{self, Done};
use pi_packages::version::Place;
use serde_json::json;

/// A program on the PATH, by name.
fn program(name: &str) -> PathBuf {
    std::env::var_os("PATH")
        .and_then(|path| {
            std::env::split_paths(&path)
                .map(|dir| dir.join(name))
                .find(|candidate| candidate.is_file())
        })
        .unwrap_or_else(|| panic!("{name} is not on the PATH"))
}

fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args([
            "-c",
            "user.name=someone",
            "-c",
            "user.email=someone@example.com",
        ])
        .args([
            "-c",
            "commit.gpgsign=false",
            "-c",
            "init.defaultBranch=main",
        ])
        .args(args)
        .current_dir(dir)
        .status()
        .expect("git runs");
    assert!(status.success(), "git {args:?} failed");
}

/// A scratch home, a package repository at its first commit, and the
/// settings an update is given.
struct Fixture {
    // Kept so the folder lasts as long as the fixture.
    _scratch: tempfile::TempDir,
    upstream: PathBuf,
    config: Config,
}

impl Fixture {
    fn new() -> Self {
        let scratch = tempfile::tempdir().expect("a scratch folder");
        let home = scratch.path().join("home");
        fs::create_dir_all(&home).expect("a home");
        let upstream = scratch.path().join("upstream");
        fs::create_dir_all(upstream.join("dep")).expect("the package");
        fs::write(
            upstream.join("package.json"),
            r#"{"name":"fixture","version":"1.0.0","dependencies":{"dep":"file:./dep"}}"#,
        )
        .expect("the manifest");
        fs::write(
            upstream.join("dep/package.json"),
            r#"{"name":"dep","version":"1.0.0","main":"index.js","scripts":{"install":"node make.js"}}"#,
        )
        .expect("the dependency's manifest");
        fs::write(upstream.join("dep/binding.gyp"), "{}\n").expect("a native part");
        // Its build writes the binding, unless the repository says it fails.
        fs::write(
            upstream.join("dep/make.js"),
            "const fs = require('fs');\n\
             if (fs.existsSync('fail')) process.exit(1);\n\
             fs.mkdirSync('build/Release', { recursive: true });\n\
             fs.writeFileSync('build/Release/dep.node', 'built');\n",
        )
        .expect("its build");
        fs::write(upstream.join("dep/index.js"), "module.exports = 1;\n").expect("its code");
        fs::write(upstream.join("extension.ts"), "export default () => {};\n")
            .expect("the extension");
        let env = json!({
            "HOME": home,
            "XDG_CACHE_HOME": home.join(".cache"),
            "XDG_DATA_HOME": home.join(".local/share"),
            "XDG_STATE_HOME": home.join(".local/state"),
            "XDG_CONFIG_HOME": home.join(".config"),
        });
        let node = program("node");
        let pnpm = program("pnpm");
        let gitter = program("git");
        // pnpm runs a dependency's install script through sh.
        let shell = program("sh");
        let path: Vec<PathBuf> = [&node, &pnpm, &gitter, &shell]
            .iter()
            .filter_map(|bin| bin.parent().map(Path::to_path_buf))
            .collect();
        let config = Config::from_json(&json!({
            "root": scratch.path().join("packages"),
            "packages": [{ "url": format!("file://{}", upstream.display()), "branch": "main" }],
            "git": gitter,
            "node": node,
            "pnpm": pnpm,
            "path": path,
            "env": env,
        }))
        .expect("settings");
        let fixture = Self {
            _scratch: scratch,
            upstream,
            config,
        };
        fixture.lock();
        git(&fixture.upstream, &["init", "--quiet"]);
        fixture.commit("first");
        fixture
    }

    /// Write the lockfile an install is held to.
    fn lock(&self) {
        let env = pi_packages::version::build_env(&self.config);
        let status = Command::new(&self.config.pnpm)
            .args([
                "install",
                "--lockfile-only",
                "--config.dangerously-allow-all-builds=true",
            ])
            .current_dir(&self.upstream)
            .env_clear()
            .envs(env.iter().map(|(key, value)| (key, value)))
            .stdout(Stdio::null())
            .status()
            .expect("pnpm runs");
        assert!(status.success(), "pnpm wrote no lockfile");
    }

    fn commit(&self, message: &str) {
        git(&self.upstream, &["add", "-A"]);
        git(
            &self.upstream,
            &["commit", "--quiet", "--allow-empty", "-m", message],
        );
    }

    fn place(&self) -> Place {
        Place::of(&self.config, &self.config.packages[0])
    }

    fn update(&self) -> run::Outcome {
        run::update_one(&self.config, &self.config.packages[0], SystemTime::now())
            .expect("an update")
    }

    /// Commit a change and update to it, so `current` moves on.
    fn advance(&self, message: &str) -> run::Outcome {
        fs::write(
            self.upstream.join("extension.ts"),
            format!("// {message}\n"),
        )
        .expect("a change");
        self.commit(message);
        let outcome = self.update();
        assert!(matches!(outcome.done, Done::Moved { .. }), "{outcome:?}");
        outcome
    }

    /// Remove what nothing uses, as a pi starting or ending has it done.
    fn prune(&self) -> usize {
        let mut pruned = run::prune(&self.config);
        let (_, removed) = pruned.pop().expect("the package");
        removed.expect("a prune").expect("not held")
    }

    /// The versions there are, by commit.
    fn versions(&self) -> Vec<String> {
        let mut found: Vec<String> = fs::read_dir(self.place().versions())
            .expect("versions")
            .flatten()
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect();
        found.sort();
        found
    }
}

/// A process that stands in a folder of its own and says, through a file
/// like a pi's, that it loaded `root`. It ends with the test.
fn saying(place: &Place, root: &Path) -> (Session, PathBuf) {
    let elsewhere = place.dir.join("elsewhere");
    fs::create_dir_all(&elsewhere).expect("a folder");
    let sleeping = Session(
        Command::new("sleep")
            .arg("600")
            .current_dir(&elsewhere)
            .spawn()
            .expect("a process starts"),
    );
    fs::create_dir_all(place.loaded()).expect("the loaded folder");
    let file = place.loaded().join(sleeping.0.id().to_string());
    fs::write(&file, json!({ "root": root }).to_string()).expect("what it loaded");
    (sleeping, file)
}

/// Every file under a folder, with its inode and contents.
fn files(dir: &Path) -> BTreeMap<PathBuf, (u64, Vec<u8>)> {
    let mut found = BTreeMap::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(next) = stack.pop() {
        for entry in fs::read_dir(&next).expect("a folder").flatten() {
            let path = entry.path();
            let kind = entry.file_type().expect("a kind");
            if kind.is_dir() {
                stack.push(path);
            } else if kind.is_file() {
                let inode = fs::metadata(&path).expect("metadata").ino();
                found.insert(path.clone(), (inode, fs::read(&path).expect("contents")));
            }
        }
    }
    found
}

/// A node session, stopped when the test ends, passed or failed: one
/// left running holds the build's output open, and the build never ends.
struct Session(Child);

impl Drop for Session {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// A node that loaded the package's dependency through `current` and
/// stands in the version it found, as a running pi does.
fn session(place: &Place) -> Session {
    let version = fs::canonicalize(place.current()).expect("current resolves");
    Session(
        Command::new("node")
            .args([
                "-e",
                "require(process.argv[1]); setInterval(() => {}, 1000)",
            ])
            .arg(place.current().join("node_modules/dep"))
            .current_dir(version)
            .stdout(Stdio::null())
            .spawn()
            .expect("a session starts"),
    )
}

#[test]
fn a_live_session_keeps_its_version_through_an_update() {
    let fixture = Fixture::new();
    let place = fixture.place();
    let first = fixture.update();
    assert!(
        matches!(first.done, Done::Moved { from: None, .. }),
        "{first:?}"
    );
    let old = fs::canonicalize(place.current()).expect("current resolves");
    let mut live = session(&place);
    let before = files(&old);

    fs::write(
        fixture.upstream.join("extension.ts"),
        "export default () => 2;\n",
    )
    .expect("a change");
    fixture.commit("second");
    let second = fixture.update();

    let Done::Moved { from: Some(_), to } = &second.done else {
        panic!("current did not move: {second:?}");
    };
    assert_eq!(place.current_commit().as_deref(), Some(to.as_str()));
    assert_ne!(
        fs::canonicalize(place.current()).expect("current resolves"),
        old
    );
    assert!(
        live.0.try_wait().expect("the session").is_none(),
        "the session died"
    );
    assert_eq!(files(&old), before, "a file the session loaded changed");
    assert_eq!(
        second.removed, 0,
        "the version the session runs was removed"
    );

    // No longer the version before current, it stays for the session alone.
    let third = fixture.advance("third");
    assert_eq!(third.removed, 0, "{third:?}");
    assert!(old.exists(), "the version the session runs was removed");

    drop(live);
    assert_eq!(fixture.prune(), 1);
    assert!(!old.exists(), "a version nothing uses was kept");
}

#[test]
fn the_version_before_current_is_kept_and_no_other() {
    let fixture = Fixture::new();
    let place = fixture.place();
    fixture.update();
    let first = place.current_commit().expect("a commit");
    fixture.advance("second");
    let second = place.current_commit().expect("a commit");

    assert_eq!(fixture.prune(), 0, "the version before current went");
    let third = fixture.advance("third");

    assert_eq!(third.removed, 1, "{third:?}");
    let current = place.current_commit().expect("a commit");
    let mut kept = vec![second, current];
    kept.sort();
    assert_eq!(fixture.versions(), kept);
    assert!(
        !place.retired().join(first).exists(),
        "a removed version's marker was kept"
    );
}

#[test]
fn a_version_a_live_pi_says_it_loaded_is_kept_until_it_ends() {
    let fixture = Fixture::new();
    let place = fixture.place();
    fixture.update();
    let old = fs::canonicalize(place.current()).expect("current resolves");
    let (pi, file) = saying(&place, &old);
    fixture.advance("second");
    fixture.advance("third");

    assert!(old.exists(), "a version a live pi runs was removed");
    assert!(file.exists(), "a live pi's file was removed");

    drop(pi);
    assert_eq!(fixture.prune(), 1);
    assert!(!old.exists(), "a version no pi runs was kept");
    assert!(!file.exists(), "an ended pi's file was kept");
}

#[test]
fn a_file_older_than_its_process_keeps_nothing() {
    let fixture = Fixture::new();
    let place = fixture.place();
    fixture.update();
    let old = fs::canonicalize(place.current()).expect("current resolves");
    // As a pid the system gave again: the file was written before the
    // process now holding the pid started.
    let (_pi, file) = saying(&place, &old);
    let written = SystemTime::now() - Duration::from_secs(3_600);
    File::options()
        .write(true)
        .open(&file)
        .and_then(|open| open.set_modified(written))
        .expect("an old file");
    fixture.advance("second");
    fixture.advance("third");

    assert!(!old.exists(), "a reused pid kept a version");
    assert!(!file.exists(), "a file that counts for nothing was kept");
}

#[test]
fn a_version_in_the_former_folder_goes_once_nothing_uses_it() {
    let fixture = Fixture::new();
    let place = fixture.place();
    fixture.update();
    // As an install from before versions were called that left it.
    let commit = place.current_commit().expect("a commit");
    fs::create_dir_all(place.former_versions()).expect("the former folder");
    let former = place.former_versions().join(&commit);
    fs::rename(place.versions().join(&commit), &former).expect("moved back");
    let staged = place.dir.join(".current.new");
    std::os::unix::fs::symlink(Path::new("releases").join(&commit), &staged).expect("a link");
    fs::rename(&staged, place.current()).expect("current");
    let live = session(&place);

    fixture.commit("second");
    let second = fixture.update();

    assert!(matches!(second.done, Done::Moved { .. }), "{second:?}");
    assert!(
        fs::read_link(place.current())
            .expect("current")
            .starts_with("versions"),
        "current is not in versions/"
    );
    assert!(former.exists(), "a version a session runs was removed");
    fixture.advance("third");
    assert!(former.exists(), "a version a session runs was removed");

    drop(live);
    assert_eq!(fixture.prune(), 1);
    assert!(
        !place.former_versions().exists(),
        "the former folder was kept"
    );
}

#[test]
fn a_build_that_fails_leaves_current_as_it_was() {
    let fixture = Fixture::new();
    let place = fixture.place();
    fixture.update();
    let installed = place.current_commit();

    fs::write(fixture.upstream.join("dep/fail"), "").expect("a failing build");
    fixture.commit("broken");
    let broken = fixture.update();

    assert!(matches!(broken.done, Done::Failed { .. }), "{broken:?}");
    assert_eq!(place.current_commit(), installed);
    let left: Vec<_> = fs::read_dir(place.versions())
        .expect("versions")
        .flatten()
        .map(|entry| entry.file_name())
        .collect();
    assert_eq!(left.len(), 1, "a half-built version was kept: {left:?}");
}

#[test]
fn an_unbuilt_native_module_is_not_swapped_in() {
    let fixture = Fixture::new();
    let place = fixture.place();
    // The build runs but writes no binding, as a skipped build leaves it.
    fs::write(fixture.upstream.join("dep/make.js"), "\n").expect("a build that builds nothing");
    fixture.commit("unbuilt");

    let outcome = fixture.update();

    let Done::Failed { why, .. } = &outcome.done else {
        panic!("it was swapped in: {outcome:?}");
    };
    assert!(why.contains("dep's native build did not run"), "{why}");
    assert_eq!(place.current_commit(), None);
}

#[test]
fn a_branch_already_current_is_left_alone() {
    let fixture = Fixture::new();
    fixture.update();

    let again = fixture.update();

    assert!(matches!(again.done, Done::Current { .. }), "{again:?}");
}

#[test]
fn an_update_under_way_is_left_to_finish() {
    let fixture = Fixture::new();
    let place = fixture.place();
    fs::create_dir_all(&place.dir).expect("its folder");
    let held = File::create(place.dir.join(".lock")).expect("the lock");
    held.lock().expect("held");

    let outcome = fixture.update();

    assert_eq!(outcome.done, Done::Busy);
    assert_eq!(place.current_commit(), None);
}
