//! zathura's `]b` and `[b`: the next or the previous document in the folder of the one a
//! zathura shows, by name, as Vim's go between buffers, coming round at either end.
//!
//! zathura has no such command, so its keys run this, given which way, the window's bus
//! name (`$DBUS`) and the document it shows (`$FILE`), and this asks that zathura to open
//! the one it chose, at the place zathura remembers for it.
//!
//! Usage: `zathura-next-document next|previous BUS DOCUMENT`
//!
//! A number before the key runs this that many times at once, each given the same
//! document, so each waits its turn on a lock and goes on from where the one before it
//! went: a file of the window's says what the last was given and what it opened, and one
//! given that same document again starts from the one opened.

use std::cmp::Ordering;
use std::ffi::OsStr;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::{env, io};

/// What this answers when it isn't given its three words.
const MISUSED: u8 = 2;

/// What this answers when there's nothing to open, or zathura couldn't be asked.
const FAILED: u8 = 1;

/// The kinds of document zathura's `MuPDF` plugin opens, by the end of their names.
const OPENS: [&str; 6] = ["cbz", "epub", "fb2", "oxps", "pdf", "xps"];

/// zathura's own page number for none, so it opens at the place it remembers.
const REMEMBERED: &str = "-2147483648";

/// Which way through the folder.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Way {
    Next,
    Previous,
}

fn main() -> ExitCode {
    let arguments: Vec<String> = env::args().skip(1).collect();
    let [way, bus, document] = arguments.as_slice() else {
        eprintln!("Usage: zathura-next-document next|previous BUS DOCUMENT");
        return ExitCode::from(MISUSED);
    };
    let way = match way.as_str() {
        "next" => Way::Next,
        "previous" => Way::Previous,
        _ => {
            eprintln!("Usage: zathura-next-document next|previous BUS DOCUMENT");
            return ExitCode::from(MISUSED);
        }
    };
    match went(way, bus, Path::new(document)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("zathura-next-document: {error}");
            ExitCode::from(FAILED)
        }
    }
}

/// Opens the document `way` of `document` in the zathura on `bus`, in turn with any other
/// run for the same window.
fn went(way: Way, bus: &str, document: &Path) -> io::Result<()> {
    let kept = kept_for(bus)?;
    let lock = File::create(kept.with_extension("lock"))?;
    lock.lock()?;
    let from = match fs::read_to_string(&kept) {
        Ok(last) => went_on_from(&last, document),
        Err(_) => document.to_path_buf(),
    };
    let folder = from.parent().unwrap_or(Path::new("."));
    let names = documents_in(folder)?;
    let name = from.file_name().unwrap_or_default();
    let Some(chosen) = beside(&names, name, way) else {
        return Err(io::Error::other(format!(
            "no other document in {}",
            folder.display()
        )));
    };
    let chosen = folder.join(chosen);
    opened(bus, &chosen)?;
    fs::write(
        &kept,
        format!("{}\n{}\n", document.display(), chosen.display()),
    )?;
    Ok(())
}

/// Where the window on `bus` keeps what its last run was given and opened.
fn kept_for(bus: &str) -> io::Result<PathBuf> {
    let runtime =
        env::var_os("XDG_RUNTIME_DIR").ok_or_else(|| io::Error::other("no XDG_RUNTIME_DIR"))?;
    let folder = Path::new(&runtime).join("zathura-next-document");
    fs::create_dir_all(&folder)?;
    Ok(folder.join(bus.replace('/', "_")))
}

/// Where a run given `document` goes on from, after the last run kept `last`: the one it
/// opened when it was given the same document, else the document itself.
fn went_on_from(last: &str, document: &Path) -> PathBuf {
    let mut lines = last.lines();
    match (lines.next(), lines.next()) {
        (Some(given), Some(opened)) if Path::new(given) == document => PathBuf::from(opened),
        _ => document.to_path_buf(),
    }
}

/// The documents zathura opens in `folder`, by name.
fn documents_in(folder: &Path) -> io::Result<Vec<std::ffi::OsString>> {
    let mut names: Vec<_> = fs::read_dir(folder)?
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_ok_and(|kind| !kind.is_dir()))
        .map(|entry| entry.file_name())
        .filter(|name| opens(name))
        .collect();
    names.sort();
    Ok(names)
}

/// Whether zathura opens a document by this name.
fn opens(name: &OsStr) -> bool {
    Path::new(name)
        .extension()
        .and_then(OsStr::to_str)
        .is_some_and(|end| OPENS.iter().any(|kind| end.eq_ignore_ascii_case(kind)))
}

/// The name `way` of `name` among `names`, sorted, coming round at either end; where
/// `name` isn't among them, the first that sorts after it, or before it.
fn beside<'a, T: Ord + ?Sized, N: AsRef<T>>(names: &'a [N], name: &T, way: Way) -> Option<&'a N> {
    let others: Vec<&N> = names
        .iter()
        .filter(|other| other.as_ref() != name)
        .collect();
    if others.is_empty() {
        return None;
    }
    let after = others
        .iter()
        .position(|other| other.as_ref().cmp(name) == Ordering::Greater);
    let chosen = match way {
        Way::Next => after.unwrap_or(0),
        Way::Previous => after
            .unwrap_or(others.len())
            .checked_sub(1)
            .unwrap_or(others.len() - 1),
    };
    Some(others[chosen])
}

/// Asks the zathura on `bus` to open `document`, through its `OpenDocument` method.
fn opened(bus: &str, document: &Path) -> io::Result<()> {
    let status = Command::new(env!("GDBUS"))
        .args([
            "call",
            "--session",
            "--dest",
            bus,
            "--object-path",
            "/org/pwmt/zathura",
        ])
        // The page, a negative number, would read as an option before `--`.
        .args(["--method", "org.pwmt.zathura.OpenDocument", "--"])
        .arg(quoted(&document.to_string_lossy()))
        .args(["''", REMEMBERED])
        .stdout(std::process::Stdio::null())
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "zathura on {bus} didn't open {}",
            document.display()
        )))
    }
}

/// `text` as a string in `GVariant`'s text, which gdbus reads its arguments as.
fn quoted(text: &str) -> String {
    let mut written = String::from("'");
    for character in text.chars() {
        if character == '\'' || character == '\\' {
            written.push('\\');
        }
        written.push(character);
    }
    written.push('\'');
    written
}

#[cfg(test)]
mod tests {
    use super::*;

    const NAMES: [&str; 3] = ["a.pdf", "b.pdf", "c.pdf"];

    #[test]
    fn next_is_the_one_after_by_name() {
        assert_eq!(beside(&NAMES, "a.pdf", Way::Next), Some(&"b.pdf"));
    }

    #[test]
    fn previous_is_the_one_before_by_name() {
        assert_eq!(beside(&NAMES, "b.pdf", Way::Previous), Some(&"a.pdf"));
    }

    #[test]
    fn the_last_comes_round_to_the_first() {
        assert_eq!(beside(&NAMES, "c.pdf", Way::Next), Some(&"a.pdf"));
    }

    #[test]
    fn the_first_comes_round_to_the_last() {
        assert_eq!(beside(&NAMES, "a.pdf", Way::Previous), Some(&"c.pdf"));
    }

    #[test]
    fn a_document_gone_from_the_folder_goes_on_from_where_it_sorted() {
        assert_eq!(beside(&NAMES, "b2.pdf", Way::Next), Some(&"c.pdf"));
        assert_eq!(beside(&NAMES, "b2.pdf", Way::Previous), Some(&"b.pdf"));
    }

    #[test]
    fn a_document_alone_has_none_beside_it() {
        assert_eq!(beside(&["a.pdf"], "a.pdf", Way::Next), None);
    }

    #[test]
    fn only_what_zathura_opens_is_counted() {
        assert!(opens(OsStr::new("a.PDF")));
        assert!(opens(OsStr::new("b.epub")));
        assert!(!opens(OsStr::new("c.txt")));
        assert!(!opens(OsStr::new("pdf")));
    }

    #[test]
    fn a_run_given_the_same_document_goes_on_from_the_one_opened() {
        let last = "/d/a.pdf\n/d/b.pdf\n";
        assert_eq!(
            went_on_from(last, Path::new("/d/a.pdf")),
            PathBuf::from("/d/b.pdf")
        );
    }

    #[test]
    fn a_run_given_another_document_starts_from_it() {
        let last = "/d/a.pdf\n/d/b.pdf\n";
        assert_eq!(
            went_on_from(last, Path::new("/d/b.pdf")),
            PathBuf::from("/d/b.pdf")
        );
    }

    #[test]
    fn quotes_and_backslashes_are_escaped_for_gdbus() {
        assert_eq!(quoted(r"/d/it's\a.pdf"), r"'/d/it\'s\\a.pdf'");
    }
}
