//! What the update and keep tests share: a scratch home, a package
//! repository whose one dependency has a native-style build and whose
//! other is packed outside it, and the settings an update is given, so a
//! build and its check run whole and offline, with the git, node and pnpm
//! on the PATH.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::SystemTime;

use pi_packages::config::Config;
use pi_packages::run::{self, Done};
use pi_packages::version::Place;
use serde_json::json;

/// A program on the PATH, by name.
pub fn program(name: &str) -> PathBuf {
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
pub struct Fixture {
    // Kept so the folder lasts as long as the fixture.
    _scratch: tempfile::TempDir,
    pub upstream: PathBuf,
    pub config: Config,
}

impl Fixture {
    pub fn new() -> Self {
        let scratch = tempfile::tempdir().expect("a scratch folder");
        let home = scratch.path().join("home");
        fs::create_dir_all(&home).expect("a home");
        let upstream = scratch.path().join("upstream");
        fs::create_dir_all(upstream.join("dep")).expect("the package");
        // A packed dependency from outside the repository, the same
        // package whatever version names it, as one from a registry is.
        // pnpm gives a folder named by path a copy for each version.
        let shared = scratch.path().join("shared");
        let packed = scratch.path().join("shared-1.0.0.tgz");
        fs::create_dir_all(&shared).expect("a shared package");
        fs::write(
            shared.join("package.json"),
            r#"{"name":"shared","version":"1.0.0","main":"index.js"}"#,
        )
        .expect("its manifest");
        fs::write(shared.join("index.js"), "module.exports = 1;\n").expect("its code");
        fs::write(
            upstream.join("package.json"),
            json!({
                "name": "fixture",
                "version": "1.0.0",
                "dependencies": {
                    "dep": "file:./dep",
                    "shared": format!("file:{}", packed.display()),
                },
            })
            .to_string(),
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
            // A neighbour too, never updated here: a pi says what it loaded of
            // one package, as it does of the harness, and loads others.
            "packages": [
                { "url": format!("file://{}", upstream.display()), "branch": "main" },
                { "url": "https://example.com/neighbour", "branch": "main" },
            ],
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
        fixture.pnpm(&shared, &["pack", "--pack-destination", ".."]);
        assert!(packed.is_file(), "pnpm packed nothing");
        fixture.lock();
        git(&fixture.upstream, &["init", "--quiet"]);
        fixture.commit("first");
        fixture
    }

    /// Write the lockfile an install is held to.
    pub fn lock(&self) {
        self.pnpm(
            &self.upstream,
            &[
                "install",
                "--lockfile-only",
                "--config.dangerously-allow-all-builds=true",
            ],
        );
    }

    /// Run pnpm in `folder` as an update would, failing the test if it fails.
    fn pnpm(&self, folder: &Path, args: &[&str]) {
        let env = pi_packages::version::build_env(&self.config);
        let status = Command::new(&self.config.pnpm)
            .args(args)
            .current_dir(folder)
            .env_clear()
            .envs(env.iter().map(|(key, value)| (key, value)))
            .stdout(Stdio::null())
            .status()
            .expect("pnpm runs");
        assert!(status.success(), "pnpm {args:?} failed");
    }

    pub fn commit(&self, message: &str) {
        git(&self.upstream, &["add", "-A"]);
        git(
            &self.upstream,
            &["commit", "--quiet", "--allow-empty", "-m", message],
        );
    }

    pub fn place(&self) -> Place {
        Place::of(&self.config, &self.config.packages[0])
    }

    /// Where the neighbouring package would be.
    pub fn neighbour(&self) -> Place {
        Place::of(&self.config, &self.config.packages[1])
    }

    pub fn update(&self) -> run::Outcome {
        run::update_one(
            &self.config,
            &self.config.packages[0],
            SystemTime::now(),
            None,
        )
        .expect("an update")
    }

    /// Commit a change, so the branch moves on.
    pub fn change(&self, message: &str) {
        fs::write(
            self.upstream.join("extension.ts"),
            format!("// {message}\n"),
        )
        .expect("a change");
        self.commit(message);
    }

    /// Commit a change and update to it, so `current` moves on.
    pub fn advance(&self, message: &str) -> run::Outcome {
        self.change(message);
        let outcome = self.update();
        assert!(matches!(outcome.done, Done::Moved { .. }), "{outcome:?}");
        outcome
    }

    /// The versions there are, by commit.
    pub fn versions(&self) -> Vec<String> {
        let mut found: Vec<String> = fs::read_dir(self.place().versions())
            .expect("versions")
            .flatten()
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect();
        found.sort();
        found
    }
}
