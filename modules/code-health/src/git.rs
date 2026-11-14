//! A repository's files as one of its refs holds them, read through git
//! from a checkout already on the machine, so what's measured is what was
//! merged, never a working tree's edits, and nothing is fetched.

use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::thread;

use crate::measure::{self, File};
use crate::merges::Merged;

/// Every file's path at `reference` under `under`, from the folder read,
/// and the text of each that's code or a folder's note.
pub fn files(
    checkout: &str,
    reference: &str,
    under: &str,
) -> Result<(Vec<String>, Vec<File>), String> {
    let prefix = if under.is_empty() {
        String::new()
    } else {
        format!("{}/", under.trim_end_matches('/'))
    };
    let mut listing = vec!["ls-tree", "-r", "-z", "--full-tree", reference];
    if !prefix.is_empty() {
        listing.extend(["--", prefix.as_str()]);
    }
    let listed = git(checkout, &listing, None)?;
    let mut paths = Vec::new();
    let mut read = Vec::new();
    for entry in listed.split(|byte| *byte == 0) {
        // "<mode> <type> <object>\t<path>"; links and submodules aren't
        // files of this repository's own.
        let entry = String::from_utf8_lossy(entry);
        let Some((about, path)) = entry.split_once('\t') else {
            continue;
        };
        if !about.starts_with("100") {
            continue;
        }
        let Some(inside) = path.strip_prefix(prefix.as_str()) else {
            continue;
        };
        paths.push(inside.to_owned());
        if measure::is_code(inside) || measure::is_note(inside) || measure::is_quarantine(inside) {
            let object = about.rsplit(' ').next().unwrap_or_default();
            read.push((inside.to_owned(), object.to_owned()));
        }
    }
    let mut asked = String::new();
    for (_, object) in &read {
        asked.push_str(object);
        asked.push('\n');
    }
    let answered = git(checkout, &["cat-file", "--batch"], Some(asked.into_bytes()))?;
    let texts = batch(&answered)?;
    if texts.len() != read.len() {
        return Err(format!(
            "git gave {} of the {} files asked for",
            texts.len(),
            read.len()
        ));
    }
    let files = read
        .into_iter()
        .zip(texts)
        .map(|((path, _), text)| File { path, text })
        .collect();
    Ok((paths, files))
}

/// Each change merged into `reference`'s own line since `since`, in Unix
/// seconds: each merge on its first-parent line, and the earliest author
/// time of the commits it brought in. A change merged by a squash or a
/// fast-forward leaves no merge, so it isn't counted. Each is titled by its
/// body's first line, where a pull request's merge keeps the request's
/// title, or by its subject when the body is empty.
pub fn merged(checkout: &str, reference: &str, since: i64) -> Result<Vec<Merged>, String> {
    let since = format!("--since=@{since}");
    let format = "--format=%H%x09%ct%x09%P%x09%s%x09%b";
    let log = git(
        checkout,
        &[
            "log",
            "-z",
            "--first-parent",
            "--merges",
            &since,
            format,
            reference,
        ],
        None,
    )?;
    let mut merged = Vec::new();
    for record in String::from_utf8_lossy(&log).split('\0') {
        let mut parts = record.splitn(5, '\t');
        let (Some(merge), Some(at), Some(parents), Some(subject), body) = (
            parts.next(),
            parts.next(),
            parts.next(),
            parts.next(),
            parts.next(),
        ) else {
            continue;
        };
        let title = body
            .and_then(|body| body.lines().map(str::trim).find(|line| !line.is_empty()))
            .unwrap_or(subject);
        let mut parents = parents.split(' ');
        let (Some(line_before), Some(brought), Ok(at)) =
            (parents.next(), parents.next(), at.parse::<i64>())
        else {
            continue;
        };
        let range = format!("{line_before}..{brought}");
        let authored = git(checkout, &["log", "--format=%at", &range], None)?;
        let first = String::from_utf8_lossy(&authored)
            .lines()
            .filter_map(|time| time.parse::<i64>().ok())
            .min();
        // What the merge brought onto the line it joined, not what that
        // line gained meanwhile.
        let files = git(
            checkout,
            &["diff", "-z", "--name-only", line_before, merge],
            None,
        )?;
        let files = String::from_utf8_lossy(&files)
            .split('\0')
            .filter(|file| !file.is_empty())
            .map(str::to_owned)
            .collect();
        if let Some(first) = first {
            merged.push(Merged {
                merge: merge.to_owned(),
                title: title.to_owned(),
                first,
                landed: at,
                files,
            });
        }
    }
    Ok(merged)
}

/// Each object's text from `git cat-file --batch`'s answer: a line of
/// "<object> <type> <size>", that many bytes, then a newline.
fn batch(answered: &[u8]) -> Result<Vec<String>, String> {
    let mut texts = Vec::new();
    let mut rest = answered;
    while !rest.is_empty() {
        let end = rest
            .iter()
            .position(|byte| *byte == b'\n')
            .ok_or("git's answer ended inside a header")?;
        let header = String::from_utf8_lossy(&rest[..end]);
        let size: usize = header
            .rsplit(' ')
            .next()
            .and_then(|size| size.parse().ok())
            .ok_or_else(|| format!("git answered oddly: {header}"))?;
        let body = rest
            .get(end + 1..end + 1 + size)
            .ok_or("git's answer ended inside a file")?;
        texts.push(String::from_utf8_lossy(body).into_owned());
        rest = rest.get(end + 2 + size..).unwrap_or_default();
    }
    Ok(texts)
}

/// Run git in a checkout, giving it `input` on its standard input from a
/// thread of its own, so a long answer can't stall it while it's asked.
/// What git says on stderr is kept for its failure, read on a thread of
/// its own too, rather than spilt through the answer.
fn git(checkout: &str, arguments: &[&str], input: Option<Vec<u8>>) -> Result<Vec<u8>, String> {
    let failed = |error: &dyn std::fmt::Display| format!("git in {checkout}: {error}");
    let mut child = Command::new("git")
        .arg("-C")
        .arg(checkout)
        .args(arguments)
        .stdin(if input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| failed(&error))?;
    let complaint = child.stderr.take().map(|mut stderr| {
        thread::spawn(move || {
            let mut said = String::new();
            let _ = stderr.read_to_string(&mut said);
            said
        })
    });
    let writing = input
        .zip(child.stdin.take())
        .map(|(input, mut stdin)| thread::spawn(move || stdin.write_all(&input)));
    let mut out = Vec::new();
    if let Some(mut stdout) = child.stdout.take() {
        stdout
            .read_to_end(&mut out)
            .map_err(|error| failed(&error))?;
    }
    if let Some(writing) = writing {
        writing
            .join()
            .map_err(|_| failed(&"its input's writer panicked"))?
            .map_err(|error| failed(&error))?;
    }
    let status = child.wait().map_err(|error| failed(&error))?;
    let complaint = complaint
        .and_then(|reading| reading.join().ok())
        .unwrap_or_default();
    if !status.success() {
        let command = arguments.first().copied().unwrap_or_default();
        let said = complaint
            .lines()
            .rev()
            .find(|line| !line.trim().is_empty())
            .map_or_else(|| status.to_string(), |line| line.trim().to_owned());
        return Err(failed(&format!("{command} failed: {said}")));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_batch_answer_is_split_by_its_sizes() {
        let answered = b"aaa blob 3\none\nbbb blob 0\n\nccc blob 4\nt\nwo\n";
        assert_eq!(
            batch(answered).expect("a whole answer wasn't read"),
            ["one", "", "t\nwo"],
            "a file's text was cut at a newline inside it, or an empty one lost"
        );
    }

    /// When the test's repository starts, as a real date, since git reads
    /// `--since=@` only as a time once it is a date's size.
    const START: i64 = 1_700_000_000;

    /// Run git in `folder` `at` seconds after `START`, as the test's own
    /// author, reading no configuration of the machine's.
    fn commit_at(folder: &std::path::Path, at: i64, arguments: &[&str]) {
        let when = format!("@{} +0000", START + at);
        let status = Command::new("git")
            .arg("-C")
            .arg(folder)
            .args(arguments)
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_AUTHOR_NAME", "Ada")
            .env("GIT_AUTHOR_EMAIL", "ada@example.com")
            .env("GIT_COMMITTER_NAME", "Ada")
            .env("GIT_COMMITTER_EMAIL", "ada@example.com")
            .env("GIT_AUTHOR_DATE", &when)
            .env("GIT_COMMITTER_DATE", &when)
            .stdout(Stdio::null())
            .status()
            .expect("git couldn't be started");
        assert!(status.success(), "git {arguments:?} failed");
    }

    #[test]
    fn a_merge_is_timed_from_the_first_edit_it_brought_in() {
        let folder = tempfile::tempdir().expect("no folder for the repository");
        let path = folder.path();
        let empty = ["commit", "-q", "--allow-empty", "-m"];
        commit_at(path, 1_000, &["init", "-q", "-b", "main"]);
        commit_at(path, 1_000, &[empty.as_slice(), &["start"]].concat());
        commit_at(path, 2_000, &["checkout", "-q", "-b", "change"]);
        commit_at(path, 2_000, &[empty.as_slice(), &["first edit"]].concat());
        std::fs::write(path.join("notes.md"), "an edit\n").expect("the edit wasn't written");
        commit_at(path, 2_600, &["add", "notes.md"]);
        commit_at(path, 2_600, &["commit", "-q", "-m", "second edit"]);
        commit_at(path, 3_000, &["checkout", "-q", "main"]);
        std::fs::write(path.join("other.md"), "meanwhile\n").expect("main's edit wasn't written");
        commit_at(path, 3_000, &["add", "other.md"]);
        commit_at(path, 3_000, &["commit", "-q", "-m", "meanwhile"]);
        commit_at(
            path,
            5_600,
            &[
                "merge",
                "-q",
                "--no-ff",
                "-m",
                "Merge pull request #1 from change",
                "-m",
                "Show the change",
                "change",
            ],
        );
        let checkout = path.to_str().expect("the folder's path isn't text");
        let timed = merged(checkout, "main", START + 1_500).expect("the merges weren't read");
        assert_eq!(timed.len(), 1, "not one merge: {timed:?}");
        assert_eq!(
            (
                timed[0].title.as_str(),
                timed[0].first - START,
                timed[0].landed - START
            ),
            ("Show the change", 2_000, 5_600),
            "the merge wasn't timed from its first edit, or titled by its body"
        );
        assert_eq!(
            timed[0].files,
            ["notes.md"],
            "the merge's files weren't what it brought, without main's meanwhile"
        );
        let later = merged(checkout, "main", START + 6_000).expect("the merges weren't read");
        assert!(
            later.is_empty(),
            "a merge before the day was read: {later:?}"
        );
    }

    #[test]
    fn a_cut_answer_is_refused() {
        assert!(
            batch(b"aaa blob 9\nshort\n").is_err(),
            "an answer shorter than its size was taken whole"
        );
    }

    #[test]
    fn a_failed_git_says_what_git_said() {
        let refused = git("/nonexistent", &["ls-tree", "HEAD"], None)
            .expect_err("git ran in a folder that isn't there");
        assert!(
            refused.starts_with("git in /nonexistent: ls-tree failed: fatal:"),
            "git's own words weren't kept: {refused}"
        );
    }
}
