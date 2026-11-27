//! How red first tells a change's tests from its code: by each file's
//! path, by the code that declares a new Rust test file, and by the lines
//! a change adds to a code file.

use std::collections::BTreeSet;
use std::path::Path;

/// Whether a line a change adds to the code file `path` adds a test there:
/// Rust's test attributes, `tokio`'s with its arguments, and a `cfg`
/// naming `test`, or Python's test functions, each where the line starts,
/// so a file or a comment quoting one adds none.
fn adds_inline_test(path: &str, line: &str) -> bool {
    let line = line.trim_start();
    match Path::new(path)
        .extension()
        .and_then(std::ffi::OsStr::to_str)
    {
        Some("rs") => {
            line.starts_with("#[test]")
                || line.starts_with("#[tokio::test")
                || (line.starts_with("#[cfg(") && line.contains("test"))
        }
        Some("py") => line.starts_with("def test_") || line.starts_with("async def test_"),
        _ => false,
    }
}

/// Whether a path is a test's, by the path alone.
pub fn is_test(path: &str) -> bool {
    if path.starts_with("tests/lib/") || path == "checks.nix" {
        return false;
    }
    let (folders, file) = path.rsplit_once('/').unwrap_or(("", path));
    if folders
        .split('/')
        .any(|folder| matches!(folder, "tests" | "test" | "checks"))
        || file == "checks.nix"
    {
        return true;
    }
    let stem = file.rsplit_once('.').map_or(file, |(stem, _)| stem);
    let marked = [
        "_test", "-test", ".test", "_tests", "-tests", ".tests", "_checks", "-checks",
    ];
    matches!(stem, "test" | "tests")
        || stem.starts_with("test_")
        || marked.iter().any(|end| stem.ends_with(end))
}

/// Whether a test file a change adds is one only the change's code can
/// declare: a Rust file is compiled only where a module names it, by its
/// name or a `#[path]`, and a base naming a file it lacks wouldn't build,
/// so laid over the base it's never compiled, and its check passes there.
/// One directly in a `tests` folder outside a crate's sources is one cargo
/// finds alone.
fn declared_by_code(path: &str) -> bool {
    let folders = path.rsplit_once('/').map_or("", |(folders, _)| folders);
    let file = path.rsplit('/').next().unwrap_or(path);
    let mut up = folders.rsplit('/');
    let (parent, above) = (up.next(), up.next());
    // Cargo builds each file in `tests`, and each folder there by its
    // `main.rs`.
    let found_alone = (parent == Some("tests") || (file == "main.rs" && above == Some("tests")))
        && !folders.split('/').any(|folder| folder == "src");
    Path::new(path)
        .extension()
        .is_some_and(|extension| extension == "rs")
        && !found_alone
}

/// The files that could declare the Rust module file at `path`: its
/// parent module's, beside its folder or in it, or its crate's root.
fn declarers(path: &str) -> Vec<String> {
    let Some((folder, file)) = path.rsplit_once('/') else {
        return Vec::new();
    };
    // A `mod.rs` is its folder's module, declared as a file beside that
    // folder would be.
    if file == "mod.rs" {
        return declarers(&format!("{folder}.rs"));
    }
    // A folder's `main.rs` roots a crate's binary or one of its `tests`.
    let mut found = vec![
        format!("{folder}.rs"),
        format!("{folder}/mod.rs"),
        format!("{folder}/main.rs"),
    ];
    if folder == "src" || folder.ends_with("/src") {
        found.push(format!("{folder}/lib.rs"));
    }
    found.retain(|declarer| declarer != path);
    found
}

/// A test file of the change that can't be laid over the base as the
/// change's head has it.
#[derive(Debug, PartialEq, Eq)]
pub enum Apart {
    /// A test file the change removes.
    Removed(String),
    /// A file moved, from and to, a test at either end.
    Moved(String, String),
    /// A new test file only the change's code declares, which would
    /// never run over the base.
    Declared(String),
    /// A path git quotes, which can't be asked of it again as it says it.
    Quoted(String),
}

impl Apart {
    /// What the file is, said.
    pub fn said(&self) -> String {
        match self {
            Self::Removed(path) => format!("{path} is a test the change removes"),
            Self::Moved(from, to) => format!("{from} moved to {to}"),
            Self::Declared(path) => format!(
                "{path} is a new test file only the change's code declares, so it wouldn't run \
                 on the base"
            ),
            Self::Quoted(path) => format!("git quotes the path {path}"),
        }
    }
}

/// The files a change touched, by what they are.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Change {
    /// Test files added or edited, which are laid over the base.
    pub tests: Vec<String>,
    /// Test files that can't be laid over the base as the head has them.
    pub apart: Vec<Apart>,
    /// Every other file touched.
    pub code: Vec<String>,
}

impl Change {
    /// The change `git diff --find-renames --name-status` says.
    pub fn read(said: &str) -> Self {
        let mut change = Self::default();
        let mut declared = Vec::new();
        for line in said.lines().filter(|line| !line.is_empty()) {
            let mut fields = line.split('\t');
            let (Some(status), Some(path)) = (fields.next(), fields.next()) else {
                continue;
            };
            let to = fields.next();
            if let Some(quoted) = std::iter::once(path)
                .chain(to)
                .find(|path| path.starts_with('"'))
            {
                change.apart.push(Apart::Quoted(quoted.to_owned()));
                continue;
            }
            match (status.chars().next(), to) {
                (Some('R' | 'C'), Some(to)) if is_test(path) || is_test(to) => {
                    change
                        .apart
                        .push(Apart::Moved(path.to_owned(), to.to_owned()));
                }
                (Some('R' | 'C'), Some(to)) => {
                    change.code.extend([path.to_owned(), to.to_owned()]);
                }
                (Some('D'), _) if is_test(path) => {
                    change.apart.push(Apart::Removed(path.to_owned()));
                }
                (Some('A'), _) if is_test(path) && declared_by_code(path) => {
                    declared.push(path.to_owned());
                }
                _ if is_test(path) => change.tests.push(path.to_owned()),
                _ => change.code.push(path.to_owned()),
            }
        }
        // A new file a laid test file declares is laid with it, or the
        // laid one names a module the base lacks, and fails to build
        // there whatever its tests say; and so on down, for a laid one
        // may declare another.
        loop {
            let (laid, rest): (Vec<String>, Vec<String>) = declared.into_iter().partition(|file| {
                declarers(file)
                    .iter()
                    .any(|declarer| change.tests.contains(declarer))
            });
            declared = rest;
            if laid.is_empty() {
                break;
            }
            change.tests.extend(laid);
        }
        change
            .apart
            .extend(declared.into_iter().map(Apart::Declared));
        change
    }
}

/// Of the files `git diff --numstat` says, those it adds lines to, or
/// can't count the lines of: a test file the change only takes lines
/// from, as one dropping the cases of behaviour it takes out, adds no
/// test to fail on the base.
pub fn adding(said: &str) -> BTreeSet<String> {
    said.lines()
        .filter_map(|line| {
            let mut fields = line.splitn(3, '\t');
            let added = fields.next()?;
            let path = fields.nth(1)?;
            (added != "0").then(|| path.to_owned())
        })
        .collect()
}

/// The code files whose added lines, as `git diff -U0` gives them, add a
/// test among their code.
pub fn inline_tests(diff: &str) -> Vec<String> {
    let mut found = BTreeSet::new();
    let mut file = None;
    // A file's own `+++` line comes in its header, before its first hunk,
    // so an added line reading `++` is never taken for one.
    let mut header = false;
    for line in diff.lines() {
        if line.starts_with("diff --git ") {
            header = true;
            file = None;
        } else if line.starts_with("@@") {
            header = false;
        } else if header {
            if let Some(path) = line.strip_prefix("+++ ") {
                file = path.strip_prefix("b/");
            }
        } else if let Some(path) = file.filter(|path| {
            line.strip_prefix('+')
                .is_some_and(|added| adds_inline_test(path, added))
        }) {
            found.insert(path.to_owned());
        }
    }
    found.into_iter().collect()
}
