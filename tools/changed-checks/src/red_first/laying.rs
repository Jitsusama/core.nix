//! Laying a change's tests over its base as a commit of their own, with
//! git's plumbing alone.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use super::At;

/// A commit holding the base's tree with HEAD's `files` laid over it, and
/// an empty file at each of `empty`, made with git's plumbing alone, so
/// nothing is checked out, no ref moves and nothing is signed. It takes
/// the base's author and committer, so it differs from the base only in
/// its files and its revision.
pub fn laid_over(
    at: &At,
    files: &[String],
    empty: &[String],
    message: &str,
) -> Result<String, String> {
    let index = Index::new()?;
    let indexed = [("GIT_INDEX_FILE", index.0.as_os_str())];
    git(at.root, &["read-tree", at.base], None, &indexed)?;
    let mut listing = Vec::new();
    if !files.is_empty() {
        let mut asked = vec!["ls-tree", "-z", "HEAD", "--"];
        asked.extend(files.iter().map(String::as_str));
        listing.extend(git(at.root, &asked, None, &[])?.into_bytes());
        if !listing.is_empty() && !listing.ends_with(b"\0") {
            listing.push(0);
        }
    }
    if !empty.is_empty() {
        let blob = git(at.root, &["hash-object", "-w", "--stdin"], Some(b""), &[])?;
        for path in empty {
            listing.extend(format!("100644 blob {blob}\t{path}\0").into_bytes());
        }
    }
    let info = ["update-index", "-z", "--index-info"];
    git(at.root, &info, Some(&listing), &indexed)?;
    let tree = git(at.root, &["write-tree"], None, &indexed)?;
    let base = git(at.root, &["cat-file", "commit", at.base], None, &[])?;
    // The people are read from the header alone, which ends at the first
    // empty line, so no line of the message is taken for one.
    let header = base
        .split_once("\n\n")
        .map_or(base.as_str(), |(header, _)| header);
    let people: String = header
        .lines()
        .filter(|line| line.starts_with("author ") || line.starts_with("committer "))
        .flat_map(|line| [line, "\n"])
        .collect();
    let commit = format!(
        "tree {tree}\nparent {}\n{people}\nchanged-checks: {message}\n",
        at.base
    );
    let asked = ["hash-object", "-t", "commit", "-w", "--stdin"];
    git(at.root, &asked, Some(commit.as_bytes()), &[])
}

/// An index of the run's own, gone once dropped.
struct Index(PathBuf);

impl Index {
    fn new() -> Result<Self, String> {
        let folder = crate::cache().join("changed-checks");
        fs::create_dir_all(&folder)
            .map_err(|error| format!("making {}: {error}", folder.display()))?;
        let path = folder.join(format!("red-first-{}.index", std::process::id()));
        let _ = fs::remove_file(&path);
        Ok(Self(path))
    }
}

impl Drop for Index {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

/// Run git in the repository at `root`, given `input` and these variables,
/// and give what it printed, trimmed of its last line's end.
pub fn git(
    root: &str,
    arguments: &[&str],
    input: Option<&[u8]>,
    told: &[(&str, &std::ffi::OsStr)],
) -> Result<String, String> {
    let failed = |error: std::io::Error| format!("running git: {error}");
    let mut child = Command::new("git")
        .arg("-C")
        .arg(Path::new(root))
        .args(arguments)
        .envs(told.iter().copied())
        .stdin(if input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(failed)?;
    if let (Some(input), Some(mut stdin)) = (input, child.stdin.take()) {
        stdin.write_all(input).map_err(failed)?;
    }
    let ran = child.wait_with_output().map_err(failed)?;
    if !ran.status.success() {
        return Err(format!(
            "git {} failed: {}",
            arguments.join(" "),
            ran.status
        ));
    }
    let said = String::from_utf8(ran.stdout)
        .map_err(|_| format!("git {} printed what isn't text", arguments.join(" ")))?;
    Ok(said.trim_end_matches('\n').to_owned())
}
