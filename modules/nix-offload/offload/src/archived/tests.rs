//! The cases of what `there --run` packs of a tree whose origin isn't on
//! GitHub, how it reads the tree's changes, and what its scratch holds.

use super::*;

/// A repository in `folder` whose HEAD holds `files`, each a path and its
/// text.
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

/// The names of the files `top`'s HEAD packs to, once it's sparse in cone
/// mode by the sparse file `sparse`.
fn sent_with(folder: &Path, top: &Path, sparse: &str) -> Result<Vec<String>, String> {
    git(top, &["config", "core.sparseCheckout", "true"]);
    git(top, &["config", "core.sparseCheckoutCone", "true"]);
    fs::create_dir_all(top.join(".git/info")).unwrap();
    fs::write(top.join(".git/info/sparse-checkout"), sparse).unwrap();
    let commit = git(top, &["rev-parse", "HEAD"]);
    let packed = packed(top, &commit, None, &folder.join("scratch"), LARGEST)?;
    let into = folder.join(format!("out-{}", fs::read_dir(folder).unwrap().count()));
    Ok(held(&packed.archive, &into)
        .into_iter()
        .map(|(name, _)| name)
        .collect())
}

/// A tree checked out to the cone `a`, with `b` beside it.
fn coned(folder: &Path) -> PathBuf {
    let top = repository(folder, &[("a/x", "1"), ("b/y", "2"), ("top", "3")]);
    git(&top, &["sparse-checkout", "set", "--cone", "a"]);
    top
}

// A fenced session can write the repository's config and attributes, and
// each of these would have run a program as the person or changed what
// went: a gzip command, a smudge filter, and attributes that drop or
// rewrite a file, from HEAD, info/attributes and the config's own
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
    let run = packed.archive.parent().unwrap();
    assert!(
        !run.join("borrowed.git").exists(),
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

// git quotes a cone holding a quote when it lists it, so the cones are
// read from the sparse file itself, where cone mode writes it as named.
#[test]
fn a_cone_holding_a_quote_goes() {
    let folder = tempfile::tempdir().unwrap();
    let top = repository(folder.path(), &[("odd\"name/file", "x"), ("top", "y")]);
    git(&top, &["sparse-checkout", "set", "--cone", "odd\"name"]);
    let commit = git(&top, &["rev-parse", "HEAD"]);

    let packed = packed(&top, &commit, None, &folder.path().join("scratch"), LARGEST).unwrap();
    let names: Vec<String> = held(&packed.archive, &folder.path().join("out"))
        .into_iter()
        .map(|(name, _)| name)
        .collect();
    assert_eq!(names, ["odd\"name/file", "top"]);
}

// A glob character cone mode escapes names a folder, never a pattern, so
// a cone `b*` sends that folder alone, not one beside it it would match.
#[test]
fn a_cone_named_by_a_glob_character_sends_that_folder_alone() {
    let folder = tempfile::tempdir().unwrap();
    let top = repository(folder.path(), &[("b*/y", "1"), ("bz/w", "2"), ("top", "3")]);

    let sent = sent_with(folder.path(), &top, "/*\n!/*/\n/b\\*/\n");
    assert_eq!(sent.unwrap(), ["b*/y", "top"]);
}

// Cones that hold none of HEAD's files would send an empty tree, whose run
// would fail far from the reason.
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

// In cone mode, git checks out by patterns when its sparse file isn't
// written as cone mode writes one, so read as cones it would send what
// the checkout doesn't hold: a pattern, the lines out of order, a folder
// taken out before it's held or never held, a backslash before anything
// but a glob character, a folder held twice, or one whose parent isn't.
#[test]
fn a_sparse_file_git_cant_read_as_cones_is_refused() {
    let folder = tempfile::tempdir().unwrap();
    let top = coned(folder.path());
    for sparse in [
        "/*\n!/*/\n/a/\nb/*.y\n",
        "b/*.y\n",
        "tool/\n",
        "/*\n!/*/\n/b*/\n",
        "!/*/\n/*\n/a/\n",
        "/*\n!/*/\n!/b/*/\n/b/\n",
        "/*\n!/*/\n/a/\n!/a/*/\n!/b/*/\n",
        // A folder taken out before it's held, or never held, with a cone
        // under it, which git reads as no cone at all.
        "/*\n!/*/\n!/b/*/\n/b/c/\n",
        "/*\n!/*/\n!/b/*/\n/b/\n/b/c/\n",
        "/*\n!/*/\n/a/\n!/a/*/\n",
        "/*\n!/*/\n/a\\b/\n",
        "/*\n!/*/\n/a/\n/a/\n",
        "/*\n!/*/\n/a/b/\n",
        "/*\n!/*/\n/a/../b/\n",
    ] {
        let refused = sent_with(folder.path(), &top, sparse);
        assert!(
            refused
                .as_ref()
                .is_err_and(|why| why.contains("can't read as a cone")),
            "{sparse:?}: {refused:?}"
        );
    }
    let empty = sent_with(folder.path(), &top, "# nothing\n");
    assert!(
        empty
            .as_ref()
            .is_err_and(|why| why.contains("names nothing")),
        "{empty:?}"
    );
}

// A folder within a cone's parent is sent whole once it's held, and the
// parent's own files go beside it.
#[test]
fn a_nested_cone_sends_its_parents_files_and_itself() {
    let folder = tempfile::tempdir().unwrap();
    let top = repository(
        folder.path(),
        &[("a/f", "1"), ("a/b/x", "2"), ("a/c/y", "3"), ("top", "4")],
    );

    let sent = sent_with(folder.path(), &top, "/*\n!/*/\n/a/\n!/a/*/\n/a/b/\n");
    assert_eq!(sent.unwrap(), ["a/b/x", "a/f", "top"]);
}

// A fenced session can point its sparse file at one it can't read, or
// write any line into it, so a file reached by a link is refused, and a
// refused line is named by its number, never shown.
#[test]
fn a_refused_sparse_file_shows_nothing_of_itself() {
    let folder = tempfile::tempdir().unwrap();
    let top = coned(folder.path());
    let secret = folder.path().join("secret");
    fs::write(&secret, "/*\n!/*/\nhidden-words\n").unwrap();
    let sparse = top.join(".git/info/sparse-checkout");
    fs::remove_file(&sparse).unwrap();
    std::os::unix::fs::symlink(&secret, &sparse).unwrap();
    let commit = git(&top, &["rev-parse", "HEAD"]);

    let linked = packed(&top, &commit, None, &folder.path().join("scratch"), LARGEST);
    assert!(
        linked
            .as_ref()
            .is_err_and(|why| why.contains("can't be read") && !why.contains("hidden")),
        "{linked:?}"
    );
    fs::remove_file(&sparse).unwrap();
    let written = sent_with(folder.path(), &top, "/*\n!/*/\nhidden-words\n");
    assert!(
        written
            .as_ref()
            .is_err_and(|why| why.contains("line 3") && !why.contains("hidden")),
        "{written:?}"
    );
    let elsewhere = folder.path().join("elsewhere");
    fs::create_dir(&elsewhere).unwrap();
    fs::rename(&secret, elsewhere.join("sparse-checkout")).unwrap();
    fs::rename(top.join(".git/info"), top.join(".git/info-was")).unwrap();
    std::os::unix::fs::symlink(&elsewhere, top.join(".git/info")).unwrap();
    let folder_linked = packed(&top, &commit, None, &folder.path().join("scratch"), LARGEST);
    assert!(
        folder_linked
            .as_ref()
            .is_err_and(|why| why.contains("can't be read") && !why.contains("hidden")),
        "{folder_linked:?}"
    );
}

// A sparse file of every file at the top and no folder held back is a
// full cone to git, which lists no cone for it; all of HEAD goes.
#[test]
fn a_full_cone_sends_every_file() {
    let folder = tempfile::tempdir().unwrap();
    let top = coned(folder.path());

    let sent = sent_with(folder.path(), &top, "/*\n");
    assert_eq!(sent.unwrap(), ["a/x", "b/y", "top"]);
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

/// A run's folder in `scratch` named `name`, made `age` ago, with a lock
/// file when `locked` isn't none, held by the file it gives back when it
/// says so.
fn left(scratch: &Path, name: &str, age: Duration, lock: Option<bool>) -> Option<fs::File> {
    let dir = scratch.join(name);
    fs::create_dir_all(&dir).unwrap();
    let held = lock.and_then(|held| {
        let file = fs::File::create(dir.join("lock")).unwrap();
        // SAFETY: flock only asks of the file this test opened.
        let taken = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
        assert_eq!(taken, 0, "the test takes its lock");
        held.then_some(file)
    });
    let made = SystemTime::now() - age;
    fs::File::open(&dir).unwrap().set_modified(made).unwrap();
    held
}

// A run killed while packing leaves its folder, with a tarball of up to a
// gigabyte of memory; the next run sweeps it, and nothing of a run still
// going, a run just made, or anything else kept there.
#[test]
fn what_a_run_that_ended_left_is_swept() {
    let folder = tempfile::tempdir().unwrap();
    let top = repository(folder.path(), &[("file", "x")]);
    let commit = git(&top, &["rev-parse", "HEAD"]);
    let scratch = folder.path().join("scratch");
    fs::create_dir_all(&scratch).unwrap();
    let old = NEWBORN * 2;
    left(&scratch, "run-1-ended", old, Some(false));
    let _going = left(&scratch, "run-2-going", old, Some(true));
    left(&scratch, "run-3-unlocked", old, None);
    left(&scratch, "run-4-newborn", Duration::ZERO, None);
    // Made in the future, as a clock stepped back after a resume leaves.
    left(&scratch, "run-5-future", Duration::ZERO, Some(false));
    fs::File::open(scratch.join("run-5-future"))
        .unwrap()
        .set_modified(SystemTime::now() + NEWBORN * 2)
        .unwrap();
    fs::write(scratch.join("uplink-1"), "").unwrap();

    let packed = packed(&top, &commit, None, &scratch, LARGEST);
    assert!(packed.is_ok(), "{packed:?}");
    let own = packed.unwrap().archive.parent().unwrap().to_path_buf();
    let mut kept: Vec<String> = fs::read_dir(&scratch)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| *path != own)
        .map(|path| path.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    kept.sort();
    assert_eq!(
        kept,
        ["run-2-going", "run-4-newborn", "uplink-1"],
        "only what the ended runs left was swept"
    );
    assert!(own.join("tree.tgz").exists(), "its own tarball waits to go");
}

// A run waits for a builder for minutes after packing, and its tarball
// must still be there to send once another run has swept the scratch.
#[test]
fn a_packed_runs_tarball_outlives_another_runs_sweep() {
    let folder = tempfile::tempdir().unwrap();
    let top = repository(folder.path(), &[("file", "x")]);
    let commit = git(&top, &["rev-parse", "HEAD"]);
    let scratch = folder.path().join("scratch");
    let first = packed(&top, &commit, None, &scratch, LARGEST).unwrap();
    let made = SystemTime::now() - NEWBORN * 2;
    fs::File::open(first.archive.parent().unwrap())
        .unwrap()
        .set_modified(made)
        .unwrap();

    packed(&top, &commit, None, &scratch, LARGEST).unwrap();
    assert!(
        first.archive.exists(),
        "the waiting run's tarball was swept"
    );
}

// Others reach a scratch in a shared /tmp, so one others can read is
// closed, and one that's a link, as another account could plant, is
// refused before anything goes where it points.
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

    let planted = folder.path().join("planted");
    fs::create_dir(&planted).unwrap();
    let linked = folder.path().join("linked");
    std::os::unix::fs::symlink(&planted, &linked).unwrap();
    let refused = packed(&top, &commit, None, &linked, LARGEST);
    assert!(
        refused.as_ref().is_err_and(|why| why.contains("a link")),
        "{refused:?}"
    );
    assert!(
        fs::read_dir(&planted).unwrap().next().is_none(),
        "nothing went where it points"
    );
}

// A name past ASCII is a cone like any other, and a repository whose path
// holds a colon is still borrowed whole.
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

// A file changed is a change, one only touched isn't, and a file outside
// the cones, which the checkout doesn't hold, is none either.
#[test]
fn unchanged_reads_what_the_tree_holds() {
    let folder = tempfile::tempdir().unwrap();
    let top = coned(folder.path());
    let head = git(&top, &["rev-parse", "HEAD"]);
    let scratch = folder.path().join("scratch");
    assert!(!top.join("b").exists(), "b is outside the cone");

    assert_eq!(unchanged(&top, &head, &scratch), Ok(true));
    fs::File::options()
        .write(true)
        .open(top.join("a/x"))
        .unwrap()
        .set_modified(SystemTime::now() + Duration::from_secs(5))
        .unwrap();
    assert_eq!(unchanged(&top, &head, &scratch), Ok(true), "touched");
    fs::write(top.join("a/x"), "changed").unwrap();
    assert_eq!(unchanged(&top, &head, &scratch), Ok(false), "changed");
    assert!(
        fs::read_dir(&scratch).unwrap().next().is_none(),
        "nothing is left in the scratch"
    );
}

// A split index keeps most of its entries in a shared file beside it,
// which a copy of the index alone wouldn't find, and a tree a large
// repository's checkout splits so would then never send.
#[test]
fn unchanged_reads_a_split_index() {
    let folder = tempfile::tempdir().unwrap();
    let top = coned(folder.path());
    git(&top, &["update-index", "--split-index"]);
    let head = git(&top, &["rev-parse", "HEAD"]);
    let scratch = folder.path().join("scratch");
    assert!(
        fs::read_dir(top.join(".git")).unwrap().any(|entry| entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with("sharedindex.")),
        "the index is split"
    );

    assert_eq!(unchanged(&top, &head, &scratch), Ok(true));
    fs::write(top.join("a/x"), "changed").unwrap();
    assert_eq!(unchanged(&top, &head, &scratch), Ok(false), "changed");
}

// A sparse index holds a folder outside the cones as one entry, which the
// borrowing repository, sparse in no way of its own, has to read too.
#[test]
fn unchanged_reads_a_sparse_index() {
    let folder = tempfile::tempdir().unwrap();
    let top = coned(folder.path());
    git(
        &top,
        &["sparse-checkout", "set", "--cone", "--sparse-index", "a"],
    );
    let head = git(&top, &["rev-parse", "HEAD"]);
    let scratch = folder.path().join("scratch");
    let listed = git(&top, &["ls-files", "--sparse"]);
    assert!(
        listed.lines().any(|line| line == "b/"),
        "b is one entry: {listed}"
    );

    assert_eq!(unchanged(&top, &head, &scratch), Ok(true));
    fs::write(top.join("a/x"), "changed").unwrap();
    assert_eq!(unchanged(&top, &head, &scratch), Ok(false), "changed");
}

// A git with no sparse settings expands a sparse index to every file of
// HEAD before it diffs, reading each tree outside the cones: in a large
// repository's tree on 19 November that held 3 GB for ten minutes. So a
// tree within the folder outside the cone is taken away here, which only
// a git that keeps the index sparse never reads.
#[test]
fn unchanged_keeps_a_sparse_index_sparse() {
    let folder = tempfile::tempdir().unwrap();
    let top = repository(folder.path(), &[("a/x", "1"), ("b/c/y", "2")]);
    git(
        &top,
        &["sparse-checkout", "set", "--cone", "--sparse-index", "a"],
    );
    let head = git(&top, &["rev-parse", "HEAD"]);
    let outside = git(&top, &["rev-parse", "HEAD:b/c"]);
    let (fan, rest) = outside.split_at(2);
    fs::remove_file(top.join(".git/objects").join(fan).join(rest)).unwrap();

    assert_eq!(
        unchanged(&top, &head, &folder.path().join("scratch")),
        Ok(true)
    );
}

// An index a session planted as a pipe would hold the run forever, and
// one past a tarball's size would fill the run's folder, most often in
// memory.
#[test]
fn unchanged_copies_only_an_index_that_is_a_file() {
    let folder = tempfile::tempdir().unwrap();
    let top = repository(folder.path(), &[("file", "x")]);
    let head = git(&top, &["rev-parse", "HEAD"]);
    fs::remove_file(top.join(".git/index")).unwrap();
    let made = Command::new("mkfifo")
        .arg(top.join(".git/index"))
        .status()
        .unwrap();
    assert!(made.success(), "the test plants a pipe");

    let refused = unchanged(&top, &head, &folder.path().join("scratch"));
    assert!(
        refused
            .as_ref()
            .is_err_and(|why| why.contains("isn't a file")),
        "{refused:?}"
    );
}

// The diff git ran in the tree's own repository read a changed file
// through the clean filter its config and info/attributes named, as the
// person, outside the fence of the session that wrote them.
#[test]
fn unchanged_runs_nothing_the_repository_says() {
    let folder = tempfile::tempdir().unwrap();
    let ran = folder.path().join("ran");
    let top = repository(folder.path(), &[("file", "x")]);
    let head = git(&top, &["rev-parse", "HEAD"]);
    fs::write(
        top.join(".git/info/attributes"),
        "* filter=planted diff=planted\n",
    )
    .unwrap();
    let planted = format!("touch {}; cat", ran.display());
    git(&top, &["config", "filter.planted.clean", &planted]);
    git(&top, &["config", "filter.planted.required", "true"]);
    git(&top, &["config", "diff.planted.textconv", &planted]);
    git(&top, &["config", "diff.external", &planted]);
    git(&top, &["config", "core.fsmonitor", &planted]);
    fs::write(top.join("file"), "changed").unwrap();

    assert_eq!(
        unchanged(&top, &head, &folder.path().join("scratch")),
        Ok(false)
    );
    assert!(!ran.exists(), "no command of the repository's ran");
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
        command.contains("tar -xmzf -"),
        "it's unpacked there, its files given the time they're unpacked: {command}"
    );
    assert!(read.is_some(), "the tarball is what the far side reads");
}

// git archive stamps each file with its commit's time, so in a warm
// target a file an old commit changed read as older than what the run
// before built from another commit's, and cargo kept that stale build.
#[test]
fn an_unpacked_file_is_newer_than_its_commit() {
    let folder = tempfile::tempdir().unwrap();
    let top = repository(folder.path(), &[("file", "x")]);
    let amended = Command::new("git")
        .args(["-c", "user.name=Test", "-c", "user.email=test@example.org"])
        .arg("-C")
        .arg(&top)
        .args(["commit", "-q", "--amend", "--no-edit"])
        .env("GIT_COMMITTER_DATE", "2001-01-01T00:00:00Z")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .status()
        .unwrap();
    assert!(amended.success(), "the commit was dated back: {amended}");
    let commit = git(&top, &["rev-parse", "HEAD"]);
    let packed = packed(&top, &commit, None, &folder.path().join("scratch"), LARGEST).unwrap();
    let far = folder.path().join("far");
    fs::create_dir(&far).unwrap();
    let (command, archive) = unpacked_under(packed, &far.to_string_lossy());
    let ran = Command::new("sh")
        .arg("-c")
        .arg(format!(
            "{command}touch -t 200201010000 ../older; [ file -nt ../older ] && echo newer"
        ))
        .stdin(fs::File::open(archive).unwrap())
        .output()
        .unwrap();
    assert_eq!(
        String::from_utf8_lossy(&ran.stdout),
        "newer\n",
        "the file kept its commit's time: {ran:?}"
    );
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
