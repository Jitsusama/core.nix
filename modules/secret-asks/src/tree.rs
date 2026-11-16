//! What the line says of the tree an ask runs in, read by the daemon
//! itself rather than taken from the ask: its commit, and how many of its
//! files differ from it. A tree's own git settings are written by whatever
//! works in it, an agent in a fence included, and this runs outside any
//! fence, so nothing of the tree's that names a program may reach git.
//!
//! Turning settings off one by one can't do that, since a repository's own
//! config names programs under names it chooses: a clean filter that
//! `.git/info/attributes` gives every file runs as status reads a file
//! whose time moved. So the tree's settings are never read where a file
//! is. Its commit and where its index and objects lie are read first,
//! which starts nothing; then status runs over a git folder made for the
//! read, holding only that commit, with the tree's index and objects named
//! outright and no config, attributes or hooks of the tree's at all.
//!
//! Where its history is kept is read from files alone, git never run: its
//! own checkout, a worktree git vouches for, or a `.git` it wouldn't.
//!
//! Whether a tree's commit is on a forge's main is walked by the daemon
//! itself, from the commit the forge names, over each commit's own bytes
//! hashed as git names them. Anything that commits can write a tree's
//! objects, and git takes a pack's index, a multi-pack index or a
//! commit-graph at its word for which bytes a commit is and what its
//! parents are, so only a hash says a commit is the one the forge named.

use std::collections::{HashSet, VecDeque};
use std::fmt::Write as _;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

use sha1_checked::CollisionResult;
use sha2::{Digest, Sha256};

/// The tree as git says it, or that it isn't one.
#[derive(Clone, Debug, PartialEq)]
pub struct Tree {
    /// Its commit, whole, or none where it isn't a git tree.
    pub commit: Option<String>,
    /// How many of its files differ from the commit, untracked ones
    /// included, or none where git couldn't say, which the line warns of
    /// as it would of changes rather than calling the tree clean.
    pub uncommitted: Option<u64>,
    /// Where its history is kept, which says how much of it a release
    /// may write.
    pub checkout: Checkout,
    /// Whether its code is all on its origin's default branch, nothing
    /// uncommitted, asked only where a standing yes would otherwise
    /// cover its ask; none where it wasn't asked.
    pub landed: Option<bool>,
    /// Its origin's URL, where a forge the engine names serves it, which
    /// a standing yes is pinned to; none elsewhere, or where unread.
    pub origin: Option<String>,
}

/// Where a tree's history is kept, as its `.git` says, read from files
/// alone so nothing of the tree's runs: the nearest `.git` at or above it.
#[derive(Clone, Debug, PartialEq)]
pub enum Checkout {
    /// In no repository.
    None,
    /// A repository's own checkout, its `.git` the repository's folder.
    Own { dot_git: PathBuf },
    /// A worktree git vouches for: its `.git` file names a worktree's own
    /// folder among `common`'s, and that folder names this `.git` back.
    Worktree {
        dot_git: PathBuf,
        common: PathBuf,
        own: PathBuf,
    },
    /// A `.git` git wouldn't vouch for: a file naming a worktree that
    /// doesn't name it back, or a link, so its history is never opened.
    Unvouched { dot_git: PathBuf },
}

/// Where the history of the tree at `path` is kept.
pub fn checkout(path: &Path) -> Checkout {
    let Some((dot_git, meta)) = path.ancestors().find_map(|folder| {
        let dot_git = folder.join(".git");
        std::fs::symlink_metadata(&dot_git)
            .ok()
            .map(|meta| (dot_git, meta))
    }) else {
        return Checkout::None;
    };
    // A link is never vouched for: one to another worktree's `.git` file
    // would resolve to where that worktree's folder names it back.
    if meta.is_dir() {
        return Checkout::Own { dot_git };
    }
    match meta.is_file().then(|| worktree(&dot_git)).flatten() {
        Some((common, own)) => Checkout::Worktree {
            dot_git,
            common,
            own,
        },
        None => Checkout::Unvouched { dot_git },
    }
}

/// The repository and the worktree's own git folder a worktree's `.git`
/// file names, where git's own record agrees: the folder is one of the
/// repository's worktrees and says it is this one, so a `.git` file a
/// command wrote can't open another's history.
fn worktree(dot_git: &Path) -> Option<(PathBuf, PathBuf)> {
    let read = |path: &Path| std::fs::read_to_string(path).ok();
    let resolved = |path: &Path| std::fs::canonicalize(path).ok();
    let text = read(dot_git)?;
    let named = text.strip_prefix("gitdir:")?.trim();
    let own = resolved(&dot_git.parent()?.join(named))?;
    let common = resolved(&own.join(read(&own.join("commondir"))?.trim()))?;
    let back = resolved(&own.join(read(&own.join("gitdir"))?.trim()))?;
    let folder = |path: &Path| std::fs::symlink_metadata(path).is_ok_and(|meta| meta.is_dir());
    let belongs = own.parent() == Some(common.join("worktrees").as_path())
        && Some(back) == resolved(dot_git)
        && folder(&own)
        && folder(&common.join("objects"));
    belongs.then_some((common, own))
}

/// git's empty tree, given as where attributes come from so none apply.
const EMPTY_TREE: &str = "4b825dc642cb6eb9a060e54bf8d69288fbee4904";

/// How many reads this daemon has made a git folder for, so no two share.
static READS: AtomicU64 = AtomicU64::new(0);

/// git at `git`, with nothing of the machine's or Joel's own settings and
/// none of the tree's that runs a program on the command line's terms.
fn git_command(git: &Path) -> Command {
    let mut command = Command::new(git);
    command
        .env_clear()
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_NO_REPLACE_OBJECTS", "1")
        .env("GIT_ATTR_SOURCE", EMPTY_TREE)
        .env("GIT_TERMINAL_PROMPT", "0")
        // Status writes the tree's index only to save work, so it's asked
        // not to: this reads the tree and changes nothing in it.
        .env("GIT_OPTIONAL_LOCKS", "0")
        .env("HOME", "/var/empty")
        .args([
            "-c",
            "core.hooksPath=/dev/null",
            "-c",
            "core.fsmonitor=false",
            "-c",
            "core.untrackedCache=false",
            "-c",
            "diff.ignoreSubmodules=all",
        ])
        .stdin(Stdio::null())
        .stderr(Stdio::null());
    command
}

/// What `command` printed, when it succeeded.
fn said(command: &mut Command) -> Option<String> {
    let ran = command.output().ok()?;
    ran.status
        .success()
        .then(|| String::from_utf8_lossy(&ran.stdout).into_owned())
}

/// Where the tree at `path` keeps what status reads, asked of git with
/// the tree's own settings, which rev-parse runs no program for.
struct Places {
    top: PathBuf,
    index: PathBuf,
    objects: PathBuf,
}

fn places(git: &Path, path: &Path) -> Option<Places> {
    let told = said(git_command(git).arg("-C").arg(path).args([
        "rev-parse",
        "--path-format=absolute",
        "--show-toplevel",
        "--git-path",
        "index",
        "--git-common-dir",
    ]))?;
    let mut lines = told.lines().map(PathBuf::from);
    let (top, index, common) = (lines.next()?, lines.next()?, lines.next()?);
    Some(Places {
        top,
        index,
        objects: common.join("objects"),
    })
}

/// A git folder of the daemon's own, holding `commit` and nothing else,
/// removed when it goes.
struct Made(PathBuf);

impl Made {
    fn new(commit: &str) -> Option<Made> {
        let read = READS.fetch_add(1, Ordering::Relaxed);
        let folder =
            std::env::temp_dir().join(format!("secret-asks-read-{}-{read}", std::process::id()));
        // create_dir refuses a folder already there, so nothing planted at
        // this name beforehand is used.
        std::fs::create_dir(&folder).ok()?;
        let made = Made(folder);
        std::fs::create_dir(made.0.join("refs")).ok()?;
        std::fs::write(made.0.join("HEAD"), format!("{commit}\n")).ok()?;
        // A repository that names its objects by SHA-256 says so in its
        // config, which this folder has none of but this.
        if Format::of(commit)? == Format::Sha256 {
            std::fs::write(
                made.0.join("config"),
                "[core]\n\trepositoryformatversion = 1\n\
                 [extensions]\n\tobjectformat = sha256\n",
            )
            .ok()?;
        }
        Some(made)
    }
}

impl Drop for Made {
    fn drop(&mut self) {
        // A folder left in the temporary folder holds no secret, only a
        // commit's name, so a failure to remove it isn't worth stopping for.
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// How many files of the tree differ from `commit`, read over a git folder
/// made for the read, so none of the tree's settings is read.
fn uncommitted(git: &Path, path: &Path, commit: &str) -> Option<u64> {
    let places = places(git, path)?;
    let made = Made::new(commit)?;
    let status = said(
        git_command(git)
            .env("GIT_DIR", &made.0)
            .env("GIT_WORK_TREE", &places.top)
            .env("GIT_INDEX_FILE", &places.index)
            .env("GIT_OBJECT_DIRECTORY", &places.objects)
            .arg("-C")
            .arg(&places.top)
            .args([
                "status",
                "--porcelain=v1",
                "--untracked-files=normal",
                "--ignore-submodules=all",
            ]),
    )?;
    Some(status.lines().count() as u64)
}

/// The URL the tree at `path` names as its origin, as its own settings
/// say it, which `config --get` reads without running anything.
pub fn origin(git: &Path, path: &Path) -> Option<String> {
    said(
        git_command(git)
            .arg("-C")
            .arg(path)
            .args(["config", "--get", "remote.origin.url"]),
    )
    .map(|said| said.trim().to_owned())
    .filter(|url| !url.is_empty())
}

/// How many commits back from a forge's main a tree's commit is looked
/// for. Past it the commit is taken as not on main, and the ask waits on
/// Joel as it would with no standing yes.
const WALKED: usize = 200_000;

/// The most a commit's bytes may be, past which it's taken as no commit
/// git made, so a forged size can't have the daemon hold gigabytes.
const LONGEST: usize = 1 << 20;

/// How a repository names its objects, told by how long a name is.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Format {
    Sha1,
    Sha256,
}

impl Format {
    /// The format `name` is a whole name in, as git writes one.
    fn of(name: &str) -> Option<Format> {
        let hex = name
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte));
        match (hex, name.len()) {
            (true, 40) => Some(Format::Sha1),
            (true, 64) => Some(Format::Sha256),
            _ => None,
        }
    }

    /// The name git gives a commit of `body`, or none for SHA-1 bytes
    /// that are half of a crafted collision, which git itself refuses.
    fn name(self, body: &[u8]) -> Option<String> {
        let header = format!("commit {}\0", body.len());
        let hash = match self {
            Format::Sha1 => {
                let mut hasher = sha1_checked::Sha1::new();
                Digest::update(&mut hasher, header.as_bytes());
                Digest::update(&mut hasher, body);
                match hasher.try_finalize() {
                    CollisionResult::Ok(hash) => hash.to_vec(),
                    CollisionResult::Mitigated(_) | CollisionResult::Collision(_) => return None,
                }
            }
            Format::Sha256 => Sha256::new()
                .chain_update(header.as_bytes())
                .chain_update(body)
                .finalize()
                .to_vec(),
        };
        Some(hash.iter().fold(String::new(), |mut hex, byte| {
            let _ = write!(hex, "{byte:02x}");
            hex
        }))
    }
}

/// Whether the tree `tree` read at `path` has its code all on its
/// origin's default branch: nothing uncommitted, and its commit one that
/// `main`, the commit the forge says the branch is, comes from. The
/// commits are read from the tree's own objects over a git folder made
/// for the read, as status is, and walked here, each hashed: a forge's
/// commit the tree hasn't fetched, or bytes that aren't the commit they
/// were asked for, hold nothing it can say, and the ask waits on Joel.
pub fn landed(git: &Path, path: &Path, tree: &Tree, main: &str) -> bool {
    let (Some(commit), Some(0)) = (tree.commit.as_deref(), tree.uncommitted) else {
        return false;
    };
    let Some(format) = Format::of(commit).filter(|format| Format::of(main) == Some(*format)) else {
        return false;
    };
    let (Some(places), Some(made)) = (places(git, path), Made::new(commit)) else {
        return false;
    };
    // cat-file reads no commit-graph, but it's turned off all the same, so
    // no later change to what cat-file reads can bring one in.
    let Ok(mut reading) = git_command(git)
        .env("GIT_DIR", &made.0)
        .env("GIT_OBJECT_DIRECTORY", &places.objects)
        .args(["-c", "core.commitGraph=false", "cat-file", "--batch"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
    else {
        return false;
    };
    let found = reading
        .stdin
        .take()
        .zip(reading.stdout.take())
        .is_some_and(|(asks, says)| {
            let batch = Batch {
                asks,
                says: BufReader::new(says),
            };
            walked(format, main, commit, batch, WALKED)
        });
    // Its input is closed by now, so it ends of itself; this only makes
    // sure it has before the made folder goes.
    let _ = reading.kill();
    let _ = reading.wait();
    found
}

/// A cat-file batch: what it's asked, and what it says back.
struct Batch<A: Write, S: BufRead> {
    asks: A,
    says: S,
}

/// Whether `commit` is `main` or comes from it, within `limit` commits,
/// each asked of `batch`: only the parents of bytes that hash to the name
/// asked for are followed.
fn walked<A: Write, S: BufRead>(
    format: Format,
    main: &str,
    commit: &str,
    mut batch: Batch<A, S>,
    limit: usize,
) -> bool {
    let mut next = VecDeque::from([main.to_owned()]);
    let mut seen = HashSet::from([main.to_owned()]);
    while let Some(name) = next.pop_front() {
        if name == commit {
            return true;
        }
        let body = match asked(&mut batch.asks, &mut batch.says, &name) {
            Some(Said::Commit(body)) if format.name(&body).as_deref() == Some(name.as_str()) => {
                body
            }
            Some(Said::Commit(_) | Said::None) => continue,
            None => return false,
        };
        for parent in parents(&body).filter(|parent| Format::of(parent) == Some(format)) {
            if seen.len() >= limit {
                return false;
            }
            if seen.insert(parent.to_owned()) {
                next.push_back(parent.to_owned());
            }
        }
    }
    false
}

/// What a cat-file batch says an object is.
enum Said {
    /// A commit, of these bytes.
    Commit(Vec<u8>),
    /// Missing from the tree's objects, or no commit.
    None,
}

/// What a cat-file batch says of the object `name`, or none where it
/// can't be read on from, the walk then ending.
fn asked(asks: &mut impl Write, says: &mut impl BufRead, name: &str) -> Option<Said> {
    writeln!(asks, "{name}").ok()?;
    asks.flush().ok()?;
    let mut header = String::new();
    says.read_line(&mut header).ok()?;
    let mut words = header.split_whitespace();
    if words.next() != Some(name) {
        return None;
    }
    let (Some(kind), Some(size)) = (words.next(), words.next()) else {
        // "missing", where the tree's objects don't hold it.
        return Some(Said::None);
    };
    let size = size.parse::<usize>().ok().filter(|size| *size <= LONGEST)?;
    let mut body = vec![0; size + 1];
    says.read_exact(&mut body).ok()?;
    // The batch ends each object with a newline of its own.
    body.pop();
    Some(if kind == "commit" {
        Said::Commit(body)
    } else {
        Said::None
    })
}

/// The parents a commit of `body` names, read as git reads them: the
/// lines straight after its tree's.
fn parents(body: &[u8]) -> impl Iterator<Item = &str> {
    let mut lines = body.split(|byte| *byte == b'\n');
    let tree = lines.next().is_some_and(|line| line.starts_with(b"tree "));
    lines
        .take_while(move |line| tree && line.starts_with(b"parent "))
        .filter_map(|line| std::str::from_utf8(&line[b"parent ".len()..]).ok())
}

/// The tree at `path`, asked of the git at `git`.
pub fn read(git: &Path, path: &Path) -> Tree {
    let commit = said(
        git_command(git)
            .arg("-C")
            .arg(path)
            .args(["rev-parse", "--verify", "HEAD"]),
    )
    .map(|said| said.trim().to_owned());
    let Some(commit) = commit else {
        return Tree {
            commit: None,
            uncommitted: None,
            // A `.git` with no commit yet still says what a release may
            // write of it.
            checkout: checkout(path),
            landed: None,
            origin: None,
        };
    };
    Tree {
        uncommitted: uncommitted(git, path, &commit),
        commit: Some(commit),
        checkout: checkout(path),
        landed: None,
        origin: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A folder of the test's own, emptied first.
    fn folder(test: &str) -> PathBuf {
        let folder =
            std::env::temp_dir().join(format!("secret-asks-{test}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).unwrap();
        std::fs::canonicalize(folder).unwrap()
    }

    /// A repository at `top/repo` with a worktree named `name`, whose own
    /// folder names `back` as its tree's `.git`.
    fn repository(top: &Path, name: &str, back: &Path) -> (PathBuf, PathBuf) {
        let common = top.join("repo/.git");
        let own = common.join("worktrees").join(name);
        std::fs::create_dir_all(common.join("objects")).unwrap();
        std::fs::create_dir_all(&own).unwrap();
        std::fs::write(own.join("commondir"), "../..\n").unwrap();
        std::fs::write(own.join("gitdir"), format!("{}\n", back.display())).unwrap();
        (common, own)
    }

    /// A commit's bytes, from `parents`, in either format.
    fn commit(tree: &str, parents: &[&str]) -> Vec<u8> {
        let parents = parents.iter().fold(String::new(), |mut lines, name| {
            let _ = writeln!(lines, "parent {name}");
            lines
        });
        format!(
            "tree {tree}\n{parents}author t <t@example.com> 0 +0000\n\
             committer t <t@example.com> 0 +0000\n\na commit\n"
        )
        .into_bytes()
    }

    /// A cat-file batch that says `objects` back, by name, in the order
    /// they're asked for.
    fn batch(objects: &[(&str, &[u8])]) -> Batch<Vec<u8>, std::io::Cursor<Vec<u8>>> {
        let mut says = Vec::new();
        for (name, body) in objects {
            says.extend_from_slice(format!("{name} commit {}\n", body.len()).as_bytes());
            says.extend_from_slice(body);
            says.push(b'\n');
        }
        Batch {
            asks: Vec::new(),
            says: std::io::Cursor::new(says),
        }
    }

    /// Three commits, each the last's parent: the first, its child, and
    /// main, with their names.
    fn chain() -> [(String, Vec<u8>); 3] {
        let first = commit(EMPTY_TREE, &[]);
        let first_name = Format::Sha1.name(&first).unwrap();
        let child = commit(EMPTY_TREE, &[&first_name]);
        let child_name = Format::Sha1.name(&child).unwrap();
        let main = commit(EMPTY_TREE, &[&child_name]);
        let main_name = Format::Sha1.name(&main).unwrap();
        [(first_name, first), (child_name, child), (main_name, main)]
    }

    #[test]
    fn a_commit_is_named_as_git_names_it_in_either_format() {
        assert_eq!(
            Format::Sha1.name(&commit(EMPTY_TREE, &[])).as_deref(),
            Some("8b811f0ececd1a5d2a5f777ed3cdd589d316ca34"),
            "a SHA-1 commit isn't named as git hash-object names it"
        );
        let empty = "6ef19b41225c5369f1c104d45d8d85efa9b057b53b14b4b9b939dd74decc5321";
        assert_eq!(
            Format::Sha256.name(&commit(empty, &[])).as_deref(),
            Some("f8c36ef475e9e7697821b57aa182738fa9b6d268abdb4b097d43bbd753cdd465"),
            "a SHA-256 commit isn't named as git hash-object names it"
        );
        assert_eq!(Format::of(empty), Some(Format::Sha256), "a long name");
        assert_eq!(Format::of(EMPTY_TREE), Some(Format::Sha1), "a short name");
        assert_eq!(
            Format::of(&EMPTY_TREE.to_uppercase()),
            None,
            "git writes lower case"
        );
    }

    #[test]
    fn a_commit_is_found_back_from_main_through_commits_that_hash_right() {
        let [(first, _), (child, child_body), (main, main_body)] = chain();
        let asked = batch(&[(&main, &main_body), (&child, &child_body)]);
        assert!(
            walked(Format::Sha1, &main, &first, asked, WALKED),
            "main's grandparent wasn't found"
        );
    }

    #[test]
    fn bytes_that_arent_the_commit_asked_for_are_never_followed() {
        let [(first, _), (child, child_body), (main, _)] = chain();
        // What a forged pack would say main is: a commit naming the
        // unlanded one as its parent, under main's name.
        let forged = commit(EMPTY_TREE, &[&first]);
        let asked = batch(&[(&main, &forged), (&child, &child_body)]);
        assert!(
            !walked(Format::Sha1, &main, &first, asked, WALKED),
            "a parent of bytes that aren't main's was followed"
        );
    }

    #[test]
    fn past_its_limit_a_walk_finds_nothing() {
        let [(first, _), (child, child_body), (main, main_body)] = chain();
        let asked = batch(&[(&main, &main_body), (&child, &child_body)]);
        assert!(
            !walked(Format::Sha1, &main, &first, asked, 1),
            "a walk went past its limit"
        );
    }

    #[test]
    fn only_the_lines_straight_after_a_commits_tree_are_its_parents() {
        let parent = "8b811f0ececd1a5d2a5f777ed3cdd589d316ca34";
        let named =
            format!("tree {EMPTY_TREE}\nparent {parent}\nauthor t\n\nparent {EMPTY_TREE}\n");
        assert_eq!(
            parents(named.as_bytes()).collect::<Vec<_>>(),
            [parent],
            "a parent line elsewhere was read as one"
        );
        let late = format!("tree {EMPTY_TREE}\nauthor t\nparent {parent}\n");
        assert_eq!(
            parents(late.as_bytes()).count(),
            0,
            "a parent line after the author was read as one"
        );
    }

    #[test]
    fn a_repositorys_own_checkout_is_found_from_a_folder_within_it() {
        let top = folder("own");
        std::fs::create_dir_all(top.join(".git")).unwrap();
        std::fs::create_dir_all(top.join("deep/within")).unwrap();
        let own = Checkout::Own {
            dot_git: top.join(".git"),
        };
        assert_eq!(checkout(&top), own, "the checkout wasn't found");
        assert_eq!(
            checkout(&top.join("deep/within")),
            own,
            "a folder within it wasn't"
        );
    }

    #[test]
    fn a_worktree_is_vouched_for_only_where_its_folder_names_it_back() {
        let top = folder("worktree");
        let tree = top.join("tree");
        std::fs::create_dir_all(&tree).unwrap();
        let dot_git = tree.join(".git");
        let (common, own) = repository(&top, "tree", &dot_git);
        std::fs::write(&dot_git, format!("gitdir: {}\n", own.display())).unwrap();
        assert_eq!(
            checkout(&tree),
            Checkout::Worktree {
                dot_git: dot_git.clone(),
                common,
                own: own.clone()
            },
            "the worktree wasn't vouched for"
        );
        std::fs::write(own.join("gitdir"), "/elsewhere/.git\n").unwrap();
        assert_eq!(
            checkout(&tree),
            Checkout::Unvouched { dot_git },
            "a worktree naming another tree was vouched for"
        );
    }

    #[test]
    fn a_linked_git_folder_is_never_vouched_for() {
        let top = folder("linked");
        std::fs::create_dir_all(top.join("repo/.git")).unwrap();
        std::fs::create_dir_all(top.join("tree")).unwrap();
        let dot_git = top.join("tree/.git");
        std::os::unix::fs::symlink(top.join("repo/.git"), &dot_git).unwrap();
        assert_eq!(
            checkout(&top.join("tree")),
            Checkout::Unvouched { dot_git },
            "a link to another's history was taken as the tree's own"
        );
    }

    #[test]
    fn a_folder_in_no_repository_is_none() {
        let top = folder("none");
        assert_eq!(
            checkout(&top),
            Checkout::None,
            "a .git was found where none is"
        );
    }
}
