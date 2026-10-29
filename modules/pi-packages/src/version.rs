//! One package's versions: its mirror, the build of a commit into a
//! folder of its own, the swap of `current` onto it, and the removal of
//! versions nothing uses any more.

use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::SystemTime;

use crate::check;
use crate::config::{Config, Package};
use crate::{inuse, loaded};

/// The variables a build keeps from the environment it was started in:
/// who and where the account is, and how TLS is trusted. Everything else,
/// a shell's pnpm settings above all, is left out.
const KEPT: [&str; 12] = [
    "HOME",
    "USER",
    "LANG",
    "TMPDIR",
    "SSL_CERT_FILE",
    "NIX_SSL_CERT_FILE",
    "XDG_CACHE_HOME",
    "XDG_CONFIG_HOME",
    "XDG_DATA_HOME",
    "XDG_STATE_HOME",
    "XDG_RUNTIME_DIR",
    "GIT_SSH_COMMAND",
];

/// How pnpm installs a version: as pi installs a git package, production
/// dependencies only and peers left to pi, and besides that with this
/// machine's pnpm whatever one the package names, and every dependency's
/// build run, whatever its manifest allows. The lockfile is followed
/// where it agrees with these and not refused where it doesn't, as pi's
/// own install does: a package whose lockfile was written with peers
/// installed still installs.
const INSTALL: [&str; 8] = [
    "install",
    "--prod",
    "--config.auto-install-peers=false",
    "--config.strict-peer-dependencies=false",
    "--config.strict-dep-builds=false",
    "--config.manage-package-manager-versions=false",
    "--config.package-manager-strict=false",
    "--config.dangerously-allow-all-builds=true",
];

/// Where one package's mirror, versions and `current` are.
#[derive(Debug, Clone)]
pub struct Place {
    pub dir: PathBuf,
}

impl Place {
    #[must_use]
    pub fn of(config: &Config, package: &Package) -> Self {
        Self {
            dir: config.root.join(&package.key),
        }
    }

    /// The link pi is given.
    #[must_use]
    pub fn current(&self) -> PathBuf {
        self.dir.join("current")
    }

    #[must_use]
    pub fn mirror(&self) -> PathBuf {
        self.dir.join("mirror.git")
    }

    #[must_use]
    pub fn versions(&self) -> PathBuf {
        self.dir.join("versions")
    }

    /// Where versions were kept while they were called releases, a word
    /// that names giving a tree back here. Only read, so each goes once
    /// nothing uses it, and running sessions keep theirs until then.
    #[must_use]
    pub fn former_versions(&self) -> PathBuf {
        self.dir.join("releases")
    }

    /// What each running pi says it loaded (loaded.rs).
    #[must_use]
    pub fn loaded(&self) -> PathBuf {
        self.dir.join("loaded")
    }

    /// When each version `current` left was retired, as a file's time.
    #[must_use]
    pub fn retired(&self) -> PathBuf {
        self.dir.join("retired")
    }

    /// The commit `current` points at, when it points at one.
    #[must_use]
    pub fn current_commit(&self) -> Option<String> {
        let target = fs::read_link(self.current()).ok()?;
        target.file_name()?.to_str().map(str::to_owned)
    }
}

/// The environment a build runs in.
#[must_use]
pub fn build_env(config: &Config) -> Vec<(String, String)> {
    let mut env: Vec<(String, String)> = KEPT
        .iter()
        .filter(|name| !config.env.iter().any(|(key, _)| key == *name))
        .filter_map(|name| {
            std::env::var(name)
                .ok()
                .map(|value| ((*name).to_owned(), value))
        })
        .collect();
    env.extend(config.env.iter().filter(|(key, _)| key != "PATH").cloned());
    let path = std::env::join_paths(&config.path)
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or_default();
    env.push(("PATH".to_owned(), path));
    env
}

/// Run a program in the build's environment, saying what it said when
/// it fails.
fn run(
    program: &Path,
    args: &[&str],
    cwd: &Path,
    env: &[(String, String)],
) -> Result<String, String> {
    let output = Command::new(program)
        .args(args)
        .current_dir(cwd)
        .env_clear()
        .envs(env.iter().map(|(key, value)| (key, value)))
        .output()
        .map_err(|error| format!("{}: {error}", program.display()))?;
    if output.status.success() {
        return Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned());
    }
    // pnpm says why on stdout, git on stderr; an error line is the why,
    // or else the last thing said.
    let said = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let lines = || said.lines().map(str::trim).filter(|line| !line.is_empty());
    let last = lines()
        .find(|line| line.contains("ERR_") || line.starts_with("fatal:"))
        .or_else(|| lines().next_back())
        .unwrap_or("it said nothing");
    let name = program.file_name().map_or_else(
        || program.display().to_string(),
        |name| name.to_string_lossy().into_owned(),
    );
    Err(format!(
        "{name} {} failed: {}",
        args.first().unwrap_or(&""),
        last.trim()
    ))
}

/// Bring the mirror up to date and say the commit its branch is at.
///
/// # Errors
///
/// Says why the mirror could not be made or read; a fetch that fails
/// is not one, since the mirror still knows the last commit it saw.
pub fn fetch(
    config: &Config,
    package: &Package,
    place: &Place,
    env: &[(String, String)],
) -> Result<(String, Option<String>), String> {
    fs::create_dir_all(&place.dir).map_err(|error| format!("{}: {error}", place.dir.display()))?;
    let mirror = place.mirror();
    let mut unfetched = None;
    if mirror.exists() {
        if let Err(why) = run(
            &config.git,
            &["fetch", "--quiet", "--prune", "origin"],
            &mirror,
            env,
        ) {
            unfetched = Some(why);
        }
    } else {
        let into = mirror.to_string_lossy().into_owned();
        run(
            &config.git,
            &["clone", "--quiet", "--mirror", &package.url, &into],
            &place.dir,
            env,
        )?;
    }
    let branch = format!("refs/heads/{}^{{commit}}", package.branch);
    let commit = run(
        &config.git,
        &["rev-parse", "--verify", "--quiet", &branch],
        &mirror,
        env,
    )
    .map_err(|_| format!("{} has no branch {}", package.url, package.branch))?;
    Ok((commit, unfetched))
}

/// Build `commit` into its version folder and check it whole, leaving
/// nothing behind when it fails.
///
/// # Errors
///
/// Says what the build or the check found.
pub fn build(
    config: &Config,
    package: &Package,
    place: &Place,
    commit: &str,
    env: &[(String, String)],
) -> Result<PathBuf, String> {
    let version = place.versions().join(commit);
    if version.is_dir() {
        return Ok(version);
    }
    fs::create_dir_all(place.versions())
        .map_err(|error| format!("{}: {error}", place.versions().display()))?;
    let building = place.versions().join(format!(".{commit}.building"));
    if building.exists() {
        fs::remove_dir_all(&building)
            .map_err(|error| format!("{}: {error}", building.display()))?;
    }
    let built = build_into(config, package, place, commit, &building, env);
    if let Err(why) = built {
        // Nothing half-built is kept: the next run starts over.
        let _ = fs::remove_dir_all(&building);
        return Err(why);
    }
    fs::rename(&building, &version).map_err(|error| format!("{}: {error}", version.display()))?;
    Ok(version)
}

fn build_into(
    config: &Config,
    package: &Package,
    place: &Place,
    commit: &str,
    building: &Path,
    env: &[(String, String)],
) -> Result<(), String> {
    let mirror = place.mirror().to_string_lossy().into_owned();
    let into = building.to_string_lossy().into_owned();
    run(
        &config.git,
        &[
            "clone",
            "--quiet",
            "--shared",
            "--no-checkout",
            &mirror,
            &into,
        ],
        &place.dir,
        env,
    )?;
    run(
        &config.git,
        &["checkout", "--quiet", "--detach", commit],
        building,
        env,
    )?;
    run(
        &config.git,
        &["remote", "set-url", "origin", &package.url],
        building,
        env,
    )?;
    if building.join("package.json").is_file() {
        run(&config.pnpm, &INSTALL, building, env)?;
    }
    let found = check::problems(building, &config.node, env)?;
    if found.is_empty() {
        Ok(())
    } else {
        Err(found.join("; "))
    }
}

/// Point `current` at `commit`'s version by renaming a new link over it,
/// so a reader finds the old version or the new and never neither, and
/// start the clock on the version it left.
///
/// # Errors
///
/// Says which link could not be made or moved.
pub fn swap(place: &Place, commit: &str, now: SystemTime) -> Result<(), String> {
    let left = place.current_commit();
    let staged = place.dir.join(".current.new");
    let _ = fs::remove_file(&staged);
    symlink(Path::new("versions").join(commit), &staged)
        .map_err(|error| format!("{}: {error}", staged.display()))?;
    fs::rename(&staged, place.current())
        .map_err(|error| format!("{}: {error}", place.current().display()))?;
    if let Some(left) = left.filter(|left| left != commit) {
        retire(place, &left, now)?;
    }
    Ok(())
}

fn retire(place: &Place, commit: &str, now: SystemTime) -> Result<(), String> {
    fs::create_dir_all(place.retired())
        .map_err(|error| format!("{}: {error}", place.retired().display()))?;
    let marker = place.retired().join(commit);
    let file =
        fs::File::create(&marker).map_err(|error| format!("{}: {error}", marker.display()))?;
    file.set_modified(now)
        .map_err(|error| format!("{}: {error}", marker.display()))
}

/// Remove each version no process uses or runs, but `current` and the
/// version it left last, which a reload just made or a step back needs;
/// and anything a stopped build left. Both the versions' folder and the
/// one they were kept in before are swept, the second going once empty.
/// Says how many went.
///
/// A version goes with the pis running it, not with its age: a cache of
/// code kept past them is only disk.
///
/// # Errors
///
/// Says which version could not be read or removed.
pub fn prune(place: &Place) -> Result<usize, String> {
    // By where `current` leads rather than its commit's name, since one
    // commit can sit in both folders.
    let current = fs::canonicalize(place.current()).ok();
    let mut left = Vec::new();
    for folder in [place.versions(), place.former_versions()] {
        left.extend(versions_left(place, &folder, current.as_deref())?);
    }
    let last = left
        .iter()
        .max_by_key(|version| version.since)
        .map(|version| version.path.clone());
    let said = loaded::read(place);
    let mut removed = 0;
    for version in left {
        if Some(&version.path) == last.as_ref() || inuse::in_use(&version.path, &said) {
            continue;
        }
        let path = &version.path;
        fs::remove_dir_all(path).map_err(|error| format!("{}: {error}", path.display()))?;
        let _ = fs::remove_file(place.retired().join(&version.name));
        removed += 1;
    }
    // Not there, or still holding a version something uses: both fine.
    let _ = fs::remove_dir(place.former_versions());
    Ok(removed)
}

/// A version `current` has left, and when it left it.
struct Left {
    name: String,
    path: PathBuf,
    since: SystemTime,
}

/// The versions in `folder` other than `current`, removing what a stopped
/// build left there.
fn versions_left(
    place: &Place,
    folder: &Path,
    current: Option<&Path>,
) -> Result<Vec<Left>, String> {
    let Ok(entries) = fs::read_dir(folder) else {
        return Ok(Vec::new());
    };
    let mut left = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let path = entry.path();
        if name.starts_with('.') {
            // A build that stopped part way; this run holds the lock, so
            // no build is under way.
            fs::remove_dir_all(&path).map_err(|error| format!("{}: {error}", path.display()))?;
            continue;
        }
        if current.is_some() && fs::canonicalize(&path).ok().as_deref() == current {
            continue;
        }
        let marker = place.retired().join(&name);
        let since = if let Ok(since) = fs::metadata(&marker).and_then(|meta| meta.modified()) {
            since
        } else {
            // Left by something other than a swap: it left when it was
            // last written, as near as can be told.
            let written = fs::metadata(&path)
                .and_then(|meta| meta.modified())
                .map_err(|error| format!("{}: {error}", path.display()))?;
            retire(place, &name, written)?;
            written
        };
        left.push(Left { name, path, since });
    }
    Ok(left)
}
