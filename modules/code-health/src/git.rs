//! A repository's files as one of its refs holds them, read through git
//! from a checkout already on the machine, so what's measured is what was
//! merged, never a working tree's edits, and nothing is fetched.

use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::thread;

use crate::measure::{self, File};

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
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|error| failed(&error))?;
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
    if !status.success() {
        return Err(failed(&format!("{arguments:?} {status}")));
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

    #[test]
    fn a_cut_answer_is_refused() {
        assert!(
            batch(b"aaa blob 9\nshort\n").is_err(),
            "an answer shorter than its size was taken whole"
        );
    }
}
