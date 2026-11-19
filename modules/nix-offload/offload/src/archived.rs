//! A tree whose origin isn't on GitHub, as `there --run` sends it: HEAD's
//! files within the tree's sparse cone, as a tarball, and what the far
//! side's shell runs to unpack it.
//!
//! A lent builder can fetch a commit by name from GitHub alone, and holds
//! no credential to fetch one from anywhere else, so a large repository's
//! tree whose origin is its own host couldn't go at all, and its suites
//! ran here. Its files go instead, as its checkout holds them: every file
//! at the top and in each folder above a cone, and every cone whole. A
//! tree that isn't sparse sends all of HEAD.
//!
//! A fenced session can write its repository's config and attributes, so
//! nothing the repository says may start a program or change what goes:
//! `git archive` in it would gzip through its `tar.tar.gz.command`, and
//! run the smudge filters, `export-ignore` and `export-subst` its
//! attributes pick. So the archive is made in an empty repository of the
//! run's own that borrows the tree's objects, read with no config but its
//! own and no attributes at all, which writes nothing into the tree's.
//! Its objects alone are read, so whatever alternates they follow, the
//! commit's hash fixes what goes.

use std::env;
use std::fs;
use std::io;
use std::os::unix::fs::{DirBuilderExt, MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::there;

/// The most of HEAD's files one run packs, in bytes before packing. The
/// tarball waits in the runtime folder, held in memory (6.3 GB on 18
/// November), and a large repository's whole HEAD can be far past it,
/// where the part a run reads was about 50 MB.
pub const LARGEST: u64 = 1_000_000_000;

/// HEAD's files of a tree, packed to send.
#[derive(Debug)]
pub struct Archived {
    /// The folder the run is in, in the repository, when not its top.
    pub dir: Option<String>,
    pub archive: PathBuf,
}

/// The files at `commit` of the git tree `top` its checkout holds, packed
/// in `scratch` while they're at most `largest` bytes, or why not.
pub fn packed(
    top: &Path,
    commit: &str,
    dir: Option<String>,
    scratch: &Path,
    largest: u64,
) -> Result<Archived, String> {
    let cones = cones(top)?;
    owned(scratch)?;
    swept(scratch);
    let borrowed = Borrowed::new(top, scratch).ok_or("couldn't borrow its objects to pack them")?;
    let paths = match cones {
        Some(cones) => {
            let paths = within(&borrowed, commit, "", &cones)
                .ok_or("couldn't read HEAD's folders to find its cones")?;
            if paths.is_empty() {
                return Err("holds no file of HEAD's in its cones".to_owned());
            }
            paths
        }
        None => Vec::new(),
    };
    let size = borrowed
        .size(commit, &paths)
        .ok_or("couldn't say how large its files are")?;
    if size > largest {
        return Err(format!(
            "holds {} of HEAD's files to send, past the {} one run packs; make it sparse \
             to what the run reads (git sparse-checkout set --cone FOLDER)",
            there::sized(size),
            there::sized(largest),
        ));
    }
    let archive = there::mine(scratch, "tree.tgz");
    // Without the worktree's, which this repository has none of, git
    // reads the attributes of the tree it packs, whatever GIT_ATTR_SOURCE
    // says, and its export-ignore dropped a file.
    let packed = borrowed
        .git()
        .args(["archive", "--worktree-attributes", "--format=tar.gz", "-o"])
        .arg(&archive)
        .args([commit, "--"])
        .args(&paths)
        .stderr(Stdio::piped())
        .output();
    match packed {
        Ok(packed) if packed.status.success() => {}
        packed => {
            let _ = fs::remove_file(&archive);
            let why = packed.map_or_else(
                |error| error.to_string(),
                |packed| String::from_utf8_lossy(&packed.stderr).trim().to_owned(),
            );
            return Err(format!("couldn't be packed: {why}"));
        }
    }
    Ok(Archived { dir, archive })
}

/// Make `scratch` the person's alone. Without a runtime folder it falls
/// back to a /tmp any account can write, where one that made it first
/// could swap the borrowing repository for its own, whose config git
/// reads, or read the files packed.
fn owned(scratch: &Path) -> Result<(), String> {
    let refused = |why: &str| Err(format!("has no scratch to pack it in: {why}"));
    if let Some(parent) = scratch.parent() {
        let _ = fs::create_dir_all(parent);
    }
    match fs::DirBuilder::new().mode(0o700).create(scratch) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
        Err(error) => return refused(&error.to_string()),
    }
    let Ok(meta) = fs::symlink_metadata(scratch) else {
        return refused("it can't be read");
    };
    if meta.file_type().is_symlink() || !meta.is_dir() {
        return refused("it's a link or not a folder");
    }
    // SAFETY: geteuid can't fail.
    if meta.uid() != unsafe { libc::geteuid() } {
        return refused("another account holds it");
    }
    if meta.mode() & 0o077 != 0
        && fs::set_permissions(scratch, fs::Permissions::from_mode(0o700)).is_err()
    {
        return refused("others can reach it");
    }
    Ok(())
}

/// What the far side's shell runs to unpack the files `archived` sends,
/// read from its standard input, in a folder of its own named in `leaves`
/// and gone when the shell ends, as `changed::checked_out`'s is, and to go
/// into the run's folder there, ending `; `; and the archive it reads.
pub fn unpacked(archived: Archived) -> (String, PathBuf) {
    unpacked_under(archived, "/tmp")
}

/// [`unpacked`], its folder made under `root`.
fn unpacked_under(archived: Archived, root: &str) -> (String, PathBuf) {
    let into = format!("{root}/nix-offload-changed-$$");
    let command = format!(
        "leaves={into}; set -e; rm -rf {into}; mkdir -p {into}/tree; \
         trap 'rm -rf {into}' EXIT; command -v tar >/dev/null || \
         tar() {{ nix shell nixpkgs#gnutar nixpkgs#gzip -c tar \"$@\"; }}; \
         tar -xzf - -C {into}/tree; cd {into}/tree/{}; ",
        there::quoted(archived.dir.as_deref().unwrap_or(".")),
    );
    (command, archived.archive)
}

/// The cones the git tree `top` is checked out to, or none when it isn't
/// sparse; refused when it's sparse by patterns other than cones.
fn cones(top: &Path) -> Result<Option<Vec<String>>, String> {
    // Unset reads false; a value git can't read is refused, never taken
    // as not sparse, which would send all of HEAD.
    let set = |key: &str| {
        there::git_with(
            top,
            &["config", "--bool", "--default", "false", key],
            None,
            &[],
        )
        .map(|value| value == "true")
        .ok_or(format!("has a {key} git can't read"))
    };
    if !set("core.sparseCheckout")? {
        return Ok(None);
    }
    // Unset, git reads its sparse file as patterns, though `git
    // sparse-checkout set` writes cones unless told otherwise.
    if !set("core.sparseCheckoutCone")? {
        return Err(
            "is sparse without cone mode, so which of its files to send can't be \
                    told; cones (git sparse-checkout set --cone) say"
                .to_owned(),
        );
    }
    // git reads a line of any other shape as a pattern, gives up on cones
    // for the whole file, and lists its lines as written, which read as
    // cones would send other files than the checkout holds.
    let file = there::git_with(
        top,
        &["rev-parse", "--git-path", "info/sparse-checkout"],
        None,
        &[],
    )
    .map(|file| top.join(file))
    .and_then(|file| fs::read_to_string(file).ok())
    .ok_or("is sparse, but its sparse file can't be read")?;
    let lines: Vec<&str> = file
        .lines()
        .map(str::trim_end)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect();
    if let Some(line) = lines.iter().find(|line| !cone_shaped(line)) {
        return Err(format!(
            "is sparse by a pattern, {line}, that git can't read as a cone, so which \
             of its files to send can't be told; git sparse-checkout set --cone says"
        ));
    }
    // Without it, git checks out every folder whole: a full cone.
    if !lines.contains(&"!/*/") {
        return Ok(None);
    }
    let listed = there::git_with(
        top,
        &["-c", "core.quotePath=false", "sparse-checkout", "list"],
        None,
        &[],
    )
    .ok_or("is sparse, but its cones can't be read")?;
    let mut cones = Vec::new();
    for cone in listed.lines() {
        // Told not to quote a name past ASCII, git still quotes one
        // holding a quote, a backslash or a control character, which
        // would match no path of HEAD's and so leave the cone out without
        // a word.
        if cone.starts_with('"') {
            return Err(format!(
                "has a cone, {cone}, whose name git quotes, so it can't be told"
            ));
        }
        let cone = cone.trim_matches('/');
        if !cone.is_empty() {
            cones.push(cone.to_owned());
        }
    }
    Ok(Some(cones))
}

/// Whether `line` of a sparse file has a shape cone mode writes: every
/// file at the top, no folder there, a folder, or none within one.
fn cone_shaped(line: &str) -> bool {
    // Cone mode escapes a glob character in a folder's name, so a bare
    // one makes the line a pattern.
    let folder = |inner: &str| {
        let mut escaped = false;
        let globs = inner.chars().any(|char| {
            let bare = !escaped && matches!(char, '*' | '?' | '[');
            escaped = !escaped && char == '\\';
            bare
        });
        !inner.is_empty() && !inner.starts_with('/') && !globs
    };
    match line {
        "/*" | "!/*/" => true,
        _ => line
            .strip_prefix("!/")
            .and_then(|line| line.strip_suffix("/*/"))
            .or_else(|| {
                line.strip_prefix('/')
                    .and_then(|line| line.strip_suffix('/'))
            })
            .is_some_and(folder),
    }
}

/// The paths, under `at` in the tree `tree` names, a sparse checkout of
/// `cones` holds: each file there, each cone whole, and what's within each
/// folder above a cone.
fn within(borrowed: &Borrowed, tree: &str, at: &str, cones: &[String]) -> Option<Vec<String>> {
    let listed = borrowed.read(&["ls-tree", "-z", tree])?;
    let mut paths = Vec::new();
    for entry in listed.split('\0').filter(|entry| !entry.is_empty()) {
        let (about, name) = entry.split_once('\t')?;
        let mut about = about.split(' ');
        let (kind, hash) = (about.nth(1)?, about.next()?);
        let path = if at.is_empty() {
            name.to_owned()
        } else {
            format!("{at}/{name}")
        };
        if kind != "tree" || cones.contains(&path) {
            paths.push(path);
        } else if cones
            .iter()
            .any(|cone| cone.starts_with(&format!("{path}/")))
        {
            paths.extend(within(borrowed, hash, &path, cones)?);
        }
    }
    Some(paths)
}

/// Each tarball and borrowing repository in `scratch` that a run which
/// ended before it could remove them left, as one killed does. Each may
/// hold up to [`LARGEST`] bytes of the runtime folder, which is memory.
fn swept(scratch: &Path) {
    let Ok(entries) = fs::read_dir(scratch) else {
        return;
    };
    for entry in entries.flatten() {
        let left = entry.file_name().to_str().and_then(left_by);
        if left.is_some_and(|pid| !runs(pid)) {
            let path = entry.path();
            let _ = fs::remove_dir_all(&path).or_else(|_| fs::remove_file(&path));
        }
    }
}

/// The process whose run packed `name`, when it's a tarball or a
/// borrowing repository; none for what other parts of a run keep there,
/// such as the uplink's turns, whose numbers aren't processes.
fn left_by(name: &str) -> Option<u32> {
    ["tree.tgz-", "borrowed.git-"]
        .iter()
        .find_map(|prefix| name.strip_prefix(prefix)?.parse().ok())
}

/// Whether the process `pid` may still run: one of another user's, or
/// one past what a pid holds, counts.
fn runs(pid: u32) -> bool {
    let Ok(pid) = libc::pid_t::try_from(pid) else {
        return true;
    };
    // SAFETY: signal 0 only asks whether the process exists.
    let asked = unsafe { libc::kill(pid, 0) };
    asked == 0 || io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
}

/// An empty repository of this run's own, in its scratch, that borrows a
/// tree's objects and reads nothing else of it; gone when dropped.
struct Borrowed {
    dir: PathBuf,
}

impl Borrowed {
    fn new(top: &Path, scratch: &Path) -> Option<Self> {
        let asked = |arguments: &[&str]| there::git_with(top, arguments, None, &[]);
        let common = asked(&["rev-parse", "--path-format=absolute", "--git-common-dir"])?;
        let format = asked(&["rev-parse", "--show-object-format"])?;
        let dir = there::mine(scratch, "borrowed.git");
        let _ = fs::remove_dir_all(&dir);
        let made = clean()
            .args(["init", "-q", "--bare", "--template="])
            .arg(format!("--object-format={format}"))
            .arg(&dir)
            .status()
            .is_ok_and(|status| status.success());
        if !made {
            return None;
        }
        // In its alternates file, rather than GIT_ALTERNATE_OBJECT_DIRECTORIES,
        // since that splits a path holding a colon in two.
        let info = dir.join("objects").join("info");
        let objects = Path::new(&common).join("objects");
        let borrows = fs::create_dir_all(&info)
            .and_then(|()| fs::write(info.join("alternates"), format!("{}\n", objects.display())));
        let borrowed = Self { dir };
        borrows.is_ok().then_some(borrowed)
    }

    /// git in this repository alone.
    fn git(&self) -> Command {
        let mut command = clean();
        command.env("GIT_DIR", &self.dir).current_dir(&self.dir);
        command
    }

    /// git's answer to `arguments`, or none when it fails.
    fn read(&self, arguments: &[&str]) -> Option<String> {
        let output = self.git().args(arguments).output().ok()?;
        output
            .status
            .success()
            .then(|| String::from_utf8_lossy(&output.stdout).into_owned())
    }

    /// How many bytes the files `paths` name at `commit` hold, all of its
    /// files with no paths.
    fn size(&self, commit: &str, paths: &[String]) -> Option<u64> {
        let mut arguments = vec!["ls-tree", "-r", "-l", "-z", commit, "--"];
        arguments.extend(paths.iter().map(String::as_str));
        let listed = self.read(&arguments)?;
        Some(
            listed
                .split('\0')
                .filter_map(|entry| entry.split_once('\t'))
                .filter_map(|(about, _)| about.split_whitespace().nth(3)?.parse::<u64>().ok())
                .sum(),
        )
    }
}

impl Drop for Borrowed {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

/// git told of no config, attributes, hook, replaced object or other
/// repository but what it's given, whatever the caller's environment says.
fn clean() -> Command {
    let mut command = Command::new("git");
    for (name, _) in env::vars_os() {
        if name.to_string_lossy().starts_with("GIT_") {
            command.env_remove(name);
        }
    }
    command
        .args([
            "-c",
            "core.attributesFile=/dev/null",
            "-c",
            "core.hooksPath=/dev/null",
            "-c",
            "core.fsmonitor=false",
        ])
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_ATTR_NOSYSTEM", "1")
        .env("GIT_LITERAL_PATHSPECS", "1")
        .env("GIT_NO_REPLACE_OBJECTS", "1")
        .stdin(Stdio::null())
        .stderr(Stdio::null());
    command
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A repository in `folder` whose HEAD holds `files`, each a path and
    /// its text.
    fn repository(folder: &Path, files: &[(&str, &str)]) -> PathBuf {
        let top = folder.join("tree");
        for (path, text) in files {
            let path = top.join(path);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, text).unwrap();
        }
        git(&top, &["init", "-q"]);
        git(&top, &["add", "."]);
        git(&top, &["commit", "-q", "-m", "files"]);
        top
    }

    fn git(top: &Path, arguments: &[&str]) -> String {
        let output = Command::new("git")
            .args(["-c", "user.name=Test", "-c", "user.email=test@example.org"])
            .arg("-C")
            .arg(top)
            .args(arguments)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .output()
            .unwrap();
        assert!(output.status.success(), "git {arguments:?}: {output:?}");
        String::from_utf8_lossy(&output.stdout).trim().to_owned()
    }

    /// The files `archive` holds, each by its path and text.
    fn held(archive: &Path, into: &Path) -> Vec<(String, String)> {
        fs::create_dir_all(into).unwrap();
        let unpacked = Command::new("tar")
            .arg("-xzf")
            .arg(archive)
            .arg("-C")
            .arg(into)
            .status()
            .unwrap();
        assert!(unpacked.success(), "what's packed is a tarball");
        let mut files = Vec::new();
        let mut folders = vec![into.to_path_buf()];
        while let Some(folder) = folders.pop() {
            for entry in fs::read_dir(folder).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    folders.push(path);
                } else {
                    let name = path
                        .strip_prefix(into)
                        .unwrap()
                        .to_string_lossy()
                        .into_owned();
                    files.push((name, fs::read_to_string(&path).unwrap()));
                }
            }
        }
        files.sort();
        files
    }

    // A fenced session can write the repository's config and attributes,
    // and each of these would have run a program as the person or changed
    // what went: a gzip command, a smudge filter, and attributes that drop
    // or rewrite a file, from HEAD, info/attributes and the config's own
    // attributes file.
    #[test]
    fn nothing_the_repository_says_runs_or_changes_what_goes() {
        let folder = tempfile::tempdir().unwrap();
        let ran = folder.path().join("ran");
        let top = repository(
            folder.path(),
            &[
                (
                    ".gitattributes",
                    "dropped export-ignore\nstamped export-subst\nfiltered filter=planted\n",
                ),
                ("dropped", "kept"),
                ("stamped", "$Format:%H$"),
                ("filtered", "as committed"),
                ("noted", "kept too"),
            ],
        );
        let attributes = folder.path().join("attributes");
        fs::write(&attributes, "noted export-ignore\n").unwrap();
        fs::write(top.join(".git/info/attributes"), "* filter=planted\n").unwrap();
        let planted = format!("touch {}; cat", ran.display());
        git(&top, &["config", "filter.planted.smudge", &planted]);
        git(&top, &["config", "filter.planted.required", "true"]);
        git(&top, &["config", "tar.tar.gz.command", &planted]);
        git(
            &top,
            &[
                "config",
                "core.attributesFile",
                &attributes.to_string_lossy(),
            ],
        );
        let commit = git(&top, &["rev-parse", "HEAD"]);

        let packed = packed(&top, &commit, None, &folder.path().join("scratch"), LARGEST).unwrap();
        assert!(!ran.exists(), "no command of the repository's ran");
        let held = held(&packed.archive, &folder.path().join("unpacked"));
        let names: Vec<&str> = held.iter().map(|(name, _)| name.as_str()).collect();
        assert_eq!(
            names,
            [".gitattributes", "dropped", "filtered", "noted", "stamped"],
            "every file of HEAD's went"
        );
        assert!(
            held.contains(&("stamped".to_owned(), "$Format:%H$".to_owned())),
            "no file was rewritten: {held:?}"
        );
        assert!(
            !folder
                .path()
                .join("scratch")
                .join(format!("borrowed.git-{}", std::process::id()))
                .exists(),
            "the borrowing repository is gone"
        );
    }

    // The tarball waits in memory, so a tree past the size one run packs is
    // told to narrow, never packed.
    #[test]
    fn a_tree_past_the_largest_is_refused_before_its_packed() {
        let folder = tempfile::tempdir().unwrap();
        let top = repository(folder.path(), &[("big", "twelve bytes")]);
        let commit = git(&top, &["rev-parse", "HEAD"]);
        let scratch = folder.path().join("scratch");

        let refused = packed(&top, &commit, None, &scratch, 11).unwrap_err();
        assert!(refused.contains("make it sparse"), "{refused}");
        assert!(
            fs::read_dir(&scratch).unwrap().next().is_none(),
            "nothing is left in the scratch"
        );
        assert!(
            packed(&top, &commit, None, &scratch, 12).is_ok(),
            "twelve bytes fit"
        );
    }

    // A cone git quotes matches no path, so it would go missing unsaid.
    #[test]
    fn a_cone_git_quotes_is_refused() {
        let folder = tempfile::tempdir().unwrap();
        let top = repository(folder.path(), &[("odd\"name/file", "x"), ("top", "y")]);
        git(&top, &["sparse-checkout", "set", "--cone", "odd\"name"]);
        let commit = git(&top, &["rev-parse", "HEAD"]);

        let refused = packed(&top, &commit, None, &folder.path().join("scratch"), LARGEST);
        assert!(
            refused
                .as_ref()
                .unwrap_err()
                .contains("whose name git quotes"),
            "{refused:?}"
        );
    }

    // Cones that hold none of HEAD's files would send an empty tree, whose
    // run would fail far from the reason.
    #[test]
    fn cones_holding_no_file_are_refused() {
        let folder = tempfile::tempdir().unwrap();
        let top = repository(folder.path(), &[("held/file", "x")]);
        git(&top, &["sparse-checkout", "set", "--cone", "elsewhere"]);
        let commit = git(&top, &["rev-parse", "HEAD"]);

        let refused = packed(&top, &commit, None, &folder.path().join("scratch"), LARGEST);
        assert!(
            refused.as_ref().unwrap_err().contains("holds no file"),
            "{refused:?}"
        );
    }

    // In cone mode, git checks out by patterns when its sparse file holds
    // one that isn't a cone, and lists the file's lines, which read as
    // cones would have sent what the checkout doesn't hold.
    #[test]
    fn patterns_git_cant_read_as_cones_are_refused() {
        let folder = tempfile::tempdir().unwrap();
        let top = repository(folder.path(), &[("a/x", "1"), ("b/y", "2")]);
        git(&top, &["sparse-checkout", "set", "--cone", "a"]);
        let commit = git(&top, &["rev-parse", "HEAD"]);
        // git lists the last three as written, with no '/' or '!' to tell
        // them from cones.
        for patterns in [
            "/*\n!/*/\n/a/\nb/*.y\n",
            "b/*.y\n",
            "tool/\n",
            "/*\n!/*/\n/b*/\n",
        ] {
            fs::write(top.join(".git/info/sparse-checkout"), patterns).unwrap();

            let refused = packed(&top, &commit, None, &folder.path().join("scratch"), LARGEST);
            assert!(
                refused
                    .as_ref()
                    .is_err_and(|why| why.contains("can't read as a cone")),
                "{patterns:?}: {refused:?}"
            );
        }
    }

    // A sparse file of every file at the top and no folder held back is a
    // full cone to git, which lists no cone for it; all of HEAD goes.
    #[test]
    fn a_full_cone_sends_every_file() {
        let folder = tempfile::tempdir().unwrap();
        let top = repository(folder.path(), &[("a/x", "1"), ("top", "2")]);
        git(&top, &["sparse-checkout", "set", "--cone", "a"]);
        fs::write(top.join(".git/info/sparse-checkout"), "/*\n").unwrap();
        let commit = git(&top, &["rev-parse", "HEAD"]);

        let sent = packed(&top, &commit, None, &folder.path().join("scratch"), LARGEST).unwrap();
        let names: Vec<String> = held(&sent.archive, &folder.path().join("out"))
            .into_iter()
            .map(|(name, _)| name)
            .collect();
        assert_eq!(names, ["a/x", "top"]);
    }

    // The limit is on what a run sends, so a large file outside the cone
    // counts for nothing, and the same file inside it is refused.
    #[test]
    fn only_the_cones_files_count_toward_the_largest() {
        let folder = tempfile::tempdir().unwrap();
        let top = repository(
            folder.path(),
            &[("small/file", "x"), ("large/file", "well past ten bytes")],
        );
        let commit = git(&top, &["rev-parse", "HEAD"]);
        let scratch = folder.path().join("scratch");

        git(&top, &["sparse-checkout", "set", "--cone", "small"]);
        assert!(
            packed(&top, &commit, None, &scratch, 10).is_ok(),
            "a file outside the cone counted"
        );
        git(&top, &["sparse-checkout", "set", "--cone", "large"]);
        assert!(
            packed(&top, &commit, None, &scratch, 10).is_err_and(|why| why.contains("make it")),
            "a file inside the cone didn't count"
        );
    }

    // A run killed while packing leaves its tarball, up to a gigabyte of
    // memory, and its borrowing repository; the next run sweeps them, and
    // nothing of a run still going or anything else kept there.
    #[test]
    fn what_a_run_that_ended_left_is_swept() {
        let folder = tempfile::tempdir().unwrap();
        let top = repository(folder.path(), &[("file", "x")]);
        let commit = git(&top, &["rev-parse", "HEAD"]);
        let scratch = folder.path().join("scratch");
        fs::create_dir_all(&scratch).unwrap();
        let mut ended = Command::new("true").spawn().unwrap();
        ended.wait().unwrap();
        let mut going = Command::new("sleep").arg("60").spawn().unwrap();
        let (gone, going_pid) = (ended.id(), going.id());
        fs::write(scratch.join(format!("tree.tgz-{gone}")), "left").unwrap();
        fs::create_dir_all(scratch.join(format!("borrowed.git-{gone}/objects"))).unwrap();
        fs::write(scratch.join(format!("tree.tgz-{going_pid}")), "its").unwrap();
        fs::write(scratch.join(format!("uplink-{gone}")), "").unwrap();

        let packed = packed(&top, &commit, None, &scratch, LARGEST);
        going.kill().unwrap();
        going.wait().unwrap();
        assert!(packed.is_ok(), "{packed:?}");
        let mut left: Vec<String> = fs::read_dir(&scratch)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        left.sort();
        let mut kept = vec![
            format!("tree.tgz-{going_pid}"),
            format!("tree.tgz-{}", std::process::id()),
            format!("uplink-{gone}"),
        ];
        kept.sort();
        assert_eq!(left, kept, "only what the ended run packed was swept");
    }

    // A name past ASCII is a cone like any other, and a repository whose
    // path holds a colon is still borrowed whole.
    // Others reach a scratch in a shared /tmp, so one others can read is
    // closed, and one that's a link, as another account could plant, is
    // refused before anything goes in it.
    #[test]
    fn the_scratch_is_the_persons_alone() {
        let folder = tempfile::tempdir().unwrap();
        let top = repository(folder.path(), &[("file", "x")]);
        let commit = git(&top, &["rev-parse", "HEAD"]);
        let scratch = folder.path().join("scratch");
        fs::create_dir(&scratch).unwrap();
        fs::set_permissions(&scratch, fs::Permissions::from_mode(0o777)).unwrap();

        packed(&top, &commit, None, &scratch, LARGEST).unwrap();
        let mode = fs::metadata(&scratch).unwrap().mode() & 0o777;
        assert_eq!(mode, 0o700, "others can still reach it: {mode:o}");

        let linked = folder.path().join("linked");
        std::os::unix::fs::symlink(&scratch, &linked).unwrap();
        let refused = packed(&top, &commit, None, &linked, LARGEST);
        assert!(
            refused.as_ref().is_err_and(|why| why.contains("a link")),
            "{refused:?}"
        );
    }

    #[test]
    fn a_cone_past_ascii_goes_from_a_path_with_a_colon() {
        let folder = tempfile::tempdir().unwrap();
        let at = folder.path().join("a:b");
        let top = repository(
            &at,
            &[("café/file", "x"), ("other/file", "y"), ("top", "z")],
        );
        git(&top, &["sparse-checkout", "set", "--cone", "café"]);
        let commit = git(&top, &["rev-parse", "HEAD"]);

        let packed = packed(&top, &commit, None, &folder.path().join("scratch"), LARGEST).unwrap();
        let held = held(&packed.archive, &folder.path().join("unpacked"));
        let names: Vec<&str> = held.iter().map(|(name, _)| name.as_str()).collect();
        assert_eq!(names, ["café/file", "top"], "the cone and the top went");
    }

    // A gate taking a pass kept from a run of the files the session's own
    // cones cut would take it for the whole commit.
    #[test]
    fn a_run_of_packed_files_keeps_no_pass() {
        let folder = tempfile::tempdir().unwrap();
        let top = repository(folder.path(), &[("file", "x")]);
        let commit = git(&top, &["rev-parse", "HEAD"]);
        let packed = packed(&top, &commit, None, &folder.path().join("scratch"), LARGEST).unwrap();

        let (at, command, read) = there::ForRun::Archived(packed).sent();
        assert!(at.is_none(), "a pass would be kept at {at:?}");
        assert!(
            command.contains("tar -xzf -"),
            "it's unpacked there: {command}"
        );
        assert!(read.is_some(), "the tarball is what the far side reads");
    }

    // The far side's shell reads the tarball from its input, runs in the
    // run's folder within it, and leaves nothing once it ends.
    #[test]
    fn the_far_side_runs_in_its_folder_and_leaves_nothing() {
        let folder = tempfile::tempdir().unwrap();
        let top = repository(folder.path(), &[("zone/my tool/file", "x")]);
        let commit = git(&top, &["rev-parse", "HEAD"]);
        let packed = packed(
            &top,
            &commit,
            Some("zone/my tool".to_owned()),
            &folder.path().join("scratch"),
            LARGEST,
        )
        .unwrap();
        let far = folder.path().join("far");
        fs::create_dir(&far).unwrap();

        let (command, archive) = unpacked_under(packed, &far.to_string_lossy());
        let ran = Command::new("sh")
            .arg("-c")
            .arg(format!("{command}pwd; cat file"))
            .stdin(fs::File::open(archive).unwrap())
            .output()
            .unwrap();
        assert!(ran.status.success(), "{ran:?}");
        let said = String::from_utf8_lossy(&ran.stdout);
        assert!(
            said.ends_with("/tree/zone/my tool\nx"),
            "it ran in its folder: {said}"
        );
        assert!(
            fs::read_dir(&far).unwrap().next().is_none(),
            "its folder is gone once it ends"
        );
    }
}
