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
//! attributes pick, and a diff in it would run the clean filters they
//! pick. So both are made in an empty repository of the run's own that
//! borrows the tree's objects, read with no config but its own and no
//! attributes at all, which writes nothing into the tree's. Its objects
//! alone are read, so whatever alternates they follow, the commit's hash
//! fixes what goes. The sparse file is read once, never through a link,
//! and a line refused is named by its number, never quoted, so a session
//! can't have a file it can't read shown back to it.
//!
//! Each run packs in a folder of its own in the scratch, made afresh and
//! locked for as long as its process lives, so a run in another fence,
//! whose pids are counted apart, neither takes it nor sweeps it.

use std::env;
use std::fs;
use std::io::{self, Read};
use std::os::fd::{AsRawFd, FromRawFd, IntoRawFd};
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::there;

/// The most of HEAD's files one run packs, in bytes before packing. The
/// tarball waits in the runtime folder, held in memory (6.3 GB on 18
/// November), and a large repository's whole HEAD can be far past it,
/// where the part a run reads was about 50 MB.
pub const LARGEST: u64 = 1_000_000_000;

/// The largest sparse file read, far past any cone mode writes, so a
/// session can't have a run read a large file in its place.
const SPARSE_LARGEST: u64 = 16_000_000;

/// How long a run's folder is let be before its lock is asked whether its
/// run still goes, which covers the moment between making it and locking
/// it.
const NEWBORN: Duration = Duration::from_secs(60);

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
    let run = Run::new(scratch)?;
    match pack(top, commit, cones.as_deref(), &run, largest) {
        Ok(archive) => {
            run.kept();
            Ok(Archived { dir, archive })
        }
        Err(why) => {
            run.gone();
            Err(why)
        }
    }
}

/// The files at `commit` of `top` within `cones`, all of them with none,
/// packed in `run`'s folder while they're at most `largest` bytes.
fn pack(
    top: &Path,
    commit: &str,
    cones: Option<&[String]>,
    run: &Run,
    largest: u64,
) -> Result<PathBuf, String> {
    let borrowed = Borrowed::new(top, run).ok_or("couldn't borrow its objects to pack them")?;
    let paths = match cones {
        Some(cones) => {
            let paths = within(&borrowed, commit, "", cones)
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
    let archive = run.dir.join("tree.tgz");
    // Without the worktree's, which this repository has none of, git
    // reads the attributes of the tree it packs, whatever GIT_ATTR_SOURCE
    // says, and its export-ignore dropped a file. git's own gzip is named
    // here, over anything a config says.
    let packed = borrowed
        .git()
        .args(["-c", "tar.tar.gz.command=git archive gzip"])
        .args(["archive", "--worktree-attributes", "--format=tar.gz", "-o"])
        .arg(&archive)
        .args([commit, "--"])
        .args(&paths)
        .stderr(Stdio::piped())
        .output();
    match packed {
        Ok(packed) if packed.status.success() => Ok(archive),
        packed => {
            let why = packed.map_or_else(
                |error| error.to_string(),
                |packed| String::from_utf8_lossy(&packed.stderr).trim().to_owned(),
            );
            Err(format!("couldn't be packed: {why}"))
        }
    }
}

/// Whether the files of the git tree `top` are its commit `head`'s, read
/// in `scratch` by a git of the run's own that borrows the tree's objects
/// and a copy of its index. In the tree's own repository, the diff would
/// read a file whose time moved through the clean filter its config and
/// info/attributes name, as the person, outside the fence of the session
/// that wrote them.
pub fn unchanged(top: &Path, head: &str, scratch: &Path) -> Result<bool, String> {
    let run = Run::new(scratch)?;
    let unchanged = diffed(top, head, &run);
    run.gone();
    unchanged
}

/// [`unchanged`], in `run`'s folder.
fn diffed(top: &Path, head: &str, run: &Run) -> Result<bool, String> {
    let borrowed =
        Borrowed::new(top, run).ok_or("couldn't borrow its objects to read its changes")?;
    let own = in_git_dir(top, None, "index")
        .map_err(|error| format!("has no index to read its changes by: {error}"))?;
    let index = run.dir.join("index");
    copied(own, &index).map_err(|error| format!("couldn't copy its index: {error}"))?;
    // A split index keeps most of its entries in a shared file beside it,
    // which git looks for beside the index it's given too.
    for shared in shared_indexes(top) {
        in_git_dir(top, None, &shared)
            .and_then(|file| copied(file, &run.dir.join(&shared)))
            .map_err(|error| format!("couldn't copy its split index's {shared}: {error}"))?;
    }
    // Without the tree's cones and sparse settings, git expands a sparse
    // index to every file of HEAD before it diffs, reading each tree
    // outside the cones: in a large repository that's minutes and
    // gigabytes. Cone mode's own file names folders alone, and nothing in
    // it runs. Whether the index is kept sparse is the tree's own say,
    // since git can't make a split index sparse.
    let mut sparse: Vec<&str> = Vec::new();
    if cones(top)?.is_some() {
        let text =
            sparse_file(top).map_err(|why| format!("is sparse, but its sparse file {why}"))?;
        let info = borrowed.dir.join("info");
        fs::create_dir_all(&info)
            .and_then(|()| fs::write(info.join("sparse-checkout"), text))
            .map_err(|error| format!("couldn't copy its sparse file: {error}"))?;
        sparse = vec![
            "-c",
            "core.sparseCheckout=true",
            "-c",
            "core.sparseCheckoutCone=true",
            "-c",
            if setting(top, "index.sparse")? {
                "index.sparse=true"
            } else {
                "index.sparse=false"
            },
        ];
    }
    let diffed = borrowed
        .git()
        .args(&sparse)
        .current_dir(top)
        .env("GIT_WORK_TREE", top)
        .env("GIT_INDEX_FILE", &index)
        .env("GIT_ATTR_SOURCE", there::EMPTY_TREE)
        .args([
            "-c",
            "diff.ignoreSubmodules=all",
            "diff",
            "--quiet",
            "--no-ext-diff",
            "--no-textconv",
            head,
            "--",
        ])
        .status();
    match diffed.map(|status| status.code()) {
        Ok(Some(0)) => Ok(true),
        Ok(Some(1)) => Ok(false),
        _ => Err("couldn't tell whether it has changes".to_owned()),
    }
}

/// The file `name` in the git tree `top`'s own git folder, or in its
/// folder `within` there, opened without following a link at its last
/// part, as a session could plant one to a file it can't read. git's own
/// answer of where the git folder is resolves every link on the way to it,
/// so a `.git` naming another repository is refused before this, by
/// `there::for_run`, which ties the git folder to the tree's own `.git`.
fn in_git_dir(top: &Path, within: Option<&str>, name: &str) -> io::Result<fs::File> {
    let dir = there::git_with(top, &["rev-parse", "--absolute-git-dir"], None, &[])
        .ok_or_else(|| io::Error::other("its git folder can't be found"))?;
    let dir = within.map_or_else(
        || PathBuf::from(&dir),
        |within| Path::new(&dir).join(within),
    );
    let dir = fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_DIRECTORY)
        .open(dir)?;
    let name = std::ffi::CString::new(name).map_err(io::Error::other)?;
    // SAFETY: openat reads a name ending in NUL, relative to a folder this
    // run holds open, and the file it opens is owned by what it gives back.
    let fd = unsafe {
        libc::openat(
            dir.as_raw_fd(),
            name.as_ptr(),
            libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC,
        )
    };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: fd was just opened, and nothing else owns it.
    Ok(unsafe { fs::File::from_raw_fd(fd) })
}

/// The shared files of the git tree `top`'s split index, by name, none for
/// an index that isn't split.
fn shared_indexes(top: &Path) -> Vec<String> {
    there::git_with(top, &["rev-parse", "--absolute-git-dir"], None, &[])
        .and_then(|dir| fs::read_dir(dir).ok())
        .into_iter()
        .flatten()
        .filter_map(|entry| entry.ok()?.file_name().into_string().ok())
        .filter(|name| name.starts_with("sharedindex."))
        .collect()
}

/// `from` copied to `to`, a file new to it, with its time, so git's
/// check of a file changed in the second its index was written holds.
/// Only a file no larger than a tarball may be is copied, since the run's
/// folder is most often in memory, and a session could plant a pipe or a
/// file that never ends.
fn copied(source: fs::File, to: &Path) -> io::Result<()> {
    let meta = source.metadata()?;
    if !meta.is_file() {
        return Err(io::Error::other("it isn't a file"));
    }
    if meta.len() > LARGEST {
        return Err(io::Error::other(format!("it's past {LARGEST} bytes")));
    }
    let modified = meta.modified()?;
    let mut source = source.take(LARGEST);
    let mut target = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(to)?;
    io::copy(&mut source, &mut target)?;
    target.set_modified(modified)
}

/// Make `scratch` the person's alone. Without a runtime folder it falls
/// back to a /tmp any account can write, where one that made it first
/// could swap the borrowing repository for its own, whose config git
/// reads, or read the files packed.
fn owned(scratch: &Path) -> Result<(), String> {
    let refused = |why: &str| {
        Err(format!(
            "has no scratch of its own in {} to read it in: {why}",
            scratch.display()
        ))
    };
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

/// Remove what a run sent once it's gone: `sent`, and the run's own folder
/// when it's a tarball packed in one, which leaves the scratch's other
/// files, a bundle among them, where they are.
pub fn gone(sent: &Path) {
    let _ = fs::remove_file(sent);
    let a_run = sent
        .parent()
        .and_then(Path::file_name)
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with("run-"));
    if a_run && let Some(run) = sent.parent() {
        let _ = fs::remove_dir_all(run);
    }
}

/// What the far side's shell runs to unpack the files `archived` sends,
/// read from its standard input, in a folder of its own named in `leaves`
/// and gone when the shell ends, as `changed::checked_out`'s is, and to go
/// into the run's folder there, ending `; `; and the archive it reads.
pub fn unpacked(archived: Archived) -> (String, PathBuf) {
    unpacked_under(archived, "/tmp")
}

/// [`unpacked`], its folder made under `root`.
///
/// Its files are given the time they're unpacked (`-m`), not the commit's
/// that `git archive` stamps them with: cargo's warm target there judges
/// a file by its time, and a file an older commit changed would read as
/// older than what the run before built from it.
fn unpacked_under(archived: Archived, root: &str) -> (String, PathBuf) {
    let into = format!("{root}/nix-offload-changed-$$");
    let command = format!(
        "leaves={into}; set -e; rm -rf {into}; mkdir -p {into}/tree; \
         trap 'rm -rf {into}' EXIT; command -v tar >/dev/null || \
         tar() {{ nix shell nixpkgs#gnutar nixpkgs#gzip -c tar \"$@\"; }}; \
         tar -xmzf - -C {into}/tree; cd {into}/tree/{}; ",
        there::quoted(archived.dir.as_deref().unwrap_or(".")),
    );
    (command, archived.archive)
}

/// The cones the git tree `top` is checked out to, or none when it isn't
/// sparse or is a full cone; refused when it's sparse by patterns.
fn cones(top: &Path) -> Result<Option<Vec<String>>, String> {
    let set = |key: &str| setting(top, key);
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
    let text = sparse_file(top).map_err(|why| format!("is sparse, but its sparse file {why}"))?;
    cones_of(&text)
}

/// Whether the git tree `top` sets its boolean `key`. Unset reads false;
/// a value git can't read is refused, never taken as false, which for
/// `core.sparseCheckout` would send all of HEAD.
fn setting(top: &Path, key: &str) -> Result<bool, String> {
    there::git_with(
        top,
        &["config", "--bool", "--default", "false", key],
        None,
        &[],
    )
    .map(|value| value == "true")
    .ok_or(format!("has a {key} git can't read"))
}

/// The sparse file of the git tree `top`, read once, and only as a plain
/// file no larger than [`SPARSE_LARGEST`], never through a link, so a
/// session can't point it at a file it can't read or one that never ends.
fn sparse_file(top: &Path) -> Result<String, String> {
    let file = in_git_dir(top, Some("info"), "sparse-checkout")
        .map_err(|error| format!("can't be read: {error}"))?;
    let meta = file
        .metadata()
        .map_err(|error| format!("can't be read: {error}"))?;
    if !meta.is_file() {
        return Err("isn't a plain file".to_owned());
    }
    if meta.len() > SPARSE_LARGEST {
        return Err("is larger than cone mode writes one".to_owned());
    }
    let mut text = String::new();
    file.take(SPARSE_LARGEST)
        .read_to_string(&mut text)
        .map_err(|_| "isn't text".to_owned())?;
    Ok(text)
}

/// The cones the sparse file `text` names, written as cone mode writes
/// it: every file at the top, no folder there, then each folder above a
/// cone held with its own folders taken out, and each cone, once; none
/// for a full cone, of the top alone. git reads any other file as
/// patterns, giving up on cones for all of it, so another is refused,
/// naming the line by its number, never what it holds.
fn cones_of(text: &str) -> Result<Option<Vec<String>>, String> {
    let lines: Vec<(usize, &str)> = text
        .lines()
        .enumerate()
        .map(|(at, line)| (at + 1, line.trim_end()))
        .filter(|(_, line)| !line.is_empty() && !line.starts_with('#'))
        .collect();
    let refused = |at: usize| {
        Err(format!(
            "is sparse by a pattern, on line {at} of its sparse file, that git can't read as \
             a cone, so which of its files to send can't be told; git sparse-checkout set \
             --cone says"
        ))
    };
    let mut rest = lines.iter();
    match (rest.next(), rest.next()) {
        (None, _) => return Err("is sparse, but its sparse file names nothing".to_owned()),
        (Some((_, "/*")), None) => return Ok(None),
        (Some((_, "/*")), Some((_, "!/*/"))) => {}
        (Some((_, "/*")), Some((at, _))) | (Some((at, _)), _) => return refused(*at),
    }
    let mut held: Vec<String> = Vec::new();
    let mut above: Vec<(usize, String)> = Vec::new();
    for (at, line) in rest {
        if let Some(inner) = line
            .strip_prefix("!/")
            .and_then(|line| line.strip_suffix("/*/"))
        {
            // A folder's own folders are taken out once it's held, once.
            let Some(name) = unescaped(inner) else {
                return refused(*at);
            };
            if !held.contains(&name) || above.iter().any(|(_, over)| *over == name) {
                return refused(*at);
            }
            above.push((*at, name));
        } else if let Some(inner) = line
            .strip_prefix('/')
            .and_then(|line| line.strip_suffix('/'))
        {
            // A folder is held once, within folders above it each held
            // with its own folders taken out.
            let Some(name) = unescaped(inner) else {
                return refused(*at);
            };
            let within = name
                .match_indices('/')
                .all(|(end, _)| above.iter().any(|(_, over)| *over == name[..end]));
            if held.contains(&name) || !within {
                return refused(*at);
            }
            held.push(name);
        } else {
            return refused(*at);
        }
    }
    // A folder above a cone holds one, or git would check out its files
    // where no cone sends them.
    for (at, over) in &above {
        if !held
            .iter()
            .any(|name| name.starts_with(&format!("{over}/")))
        {
            return refused(*at);
        }
    }
    Ok(Some(
        held.into_iter()
            .filter(|name| !above.iter().any(|(_, over)| over == name))
            .collect(),
    ))
}

/// The folder a cone line's `inner` names, its glob characters escaped
/// as cone mode escapes them; none when one is bare, a backslash escapes
/// anything else, or it isn't a folder's path.
fn unescaped(inner: &str) -> Option<String> {
    let mut name = String::new();
    let mut chars = inner.chars();
    while let Some(char) = chars.next() {
        match char {
            '\\' => match chars.next()? {
                escaped @ ('*' | '?' | '[' | '\\') => name.push(escaped),
                _ => return None,
            },
            '*' | '?' | '[' => return None,
            _ => name.push(char),
        }
    }
    let parts_named = name
        .split('/')
        .all(|part| !part.is_empty() && part != "." && part != "..");
    parts_named.then_some(name)
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

/// A folder of one run's own in the scratch, made afresh and locked by a
/// file in it, which `swept` asks of.
struct Run {
    dir: PathBuf,
    lock: fs::File,
}

impl Run {
    /// A folder of its own in `scratch`, made the person's alone, once
    /// what runs that ended left there is swept.
    fn new(scratch: &Path) -> Result<Self, String> {
        owned(scratch)?;
        swept(scratch);
        let refused =
            |error: io::Error| format!("has no folder of its own in its scratch: {error}");
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |since| since.as_nanos());
        let dir = scratch.join(format!("run-{}-{nanos}", std::process::id()));
        fs::DirBuilder::new()
            .mode(0o700)
            .create(&dir)
            .map_err(refused)?;
        let locked = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(dir.join("lock"))
            .and_then(|lock| {
                // SAFETY: flock only asks of the file this run opened.
                let taken = unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
                if taken == 0 {
                    Ok(lock)
                } else {
                    Err(io::Error::last_os_error())
                }
            });
        match locked {
            Ok(lock) => Ok(Self { dir, lock }),
            Err(error) => {
                let _ = fs::remove_dir_all(&dir);
                Err(refused(error))
            }
        }
    }

    /// Keep its folder, its tarball in it, locked until the process ends,
    /// when a later run sweeps what the send left.
    fn kept(self) {
        let _ = self.lock.into_raw_fd();
    }

    /// Remove its folder and let its lock go.
    fn gone(self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

/// Each run's folder in `scratch` whose run ended, as one killed does,
/// which may hold up to [`LARGEST`] bytes of the runtime folder, which is
/// memory; nothing else kept there, such as the uplink's turns.
fn swept(scratch: &Path) {
    let Ok(entries) = fs::read_dir(scratch) else {
        return;
    };
    for entry in entries.flatten() {
        let a_run = entry
            .file_name()
            .to_str()
            .is_some_and(|name| name.starts_with("run-"));
        if a_run && ended(&entry.path()) {
            let _ = fs::remove_dir_all(entry.path());
        }
    }
}

/// Whether the run whose folder is `dir` ended: past [`NEWBORN`], its lock
/// can be taken, or it never made one. A folder made in the future, as a
/// clock stepped back after a resume leaves, is asked its lock too, since
/// the lock is what decides and it would otherwise wait out the clock.
fn ended(dir: &Path) -> bool {
    let newborn = fs::symlink_metadata(dir)
        .and_then(|meta| meta.modified())
        .ok()
        .is_none_or(|made| made.elapsed().is_ok_and(|age| age < NEWBORN));
    if newborn {
        return false;
    }
    let lock = fs::OpenOptions::new()
        .write(true)
        .custom_flags(libc::O_NOFOLLOW)
        .open(dir.join("lock"));
    match lock {
        // SAFETY: flock only asks of the file this run opened.
        Ok(lock) => unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) == 0 },
        Err(error) => error.kind() == io::ErrorKind::NotFound,
    }
}

/// An empty repository in a run's folder that borrows a tree's objects
/// and reads nothing else of it; gone when dropped.
struct Borrowed {
    dir: PathBuf,
}

impl Borrowed {
    fn new(top: &Path, run: &Run) -> Option<Self> {
        let asked = |arguments: &[&str]| there::git_with(top, arguments, None, &[]);
        let common = asked(&["rev-parse", "--path-format=absolute", "--git-common-dir"])?;
        let format = asked(&["rev-parse", "--show-object-format"])?;
        let dir = run.dir.join("borrowed.git");
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
    // The store's git, where package.nix gives one, has every feature this
    // reads: its own gzip, GIT_ATTR_SOURCE and absolute paths.
    let mut command = Command::new(option_env!("NIX_OFFLOAD_GIT").unwrap_or("git"));
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
mod tests;
