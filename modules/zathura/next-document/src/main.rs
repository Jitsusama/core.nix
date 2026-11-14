//! zathura's `]b` and `[b`: the next or the previous document in the folder of the one a
//! zathura shows, by name, as Vim's go between buffers, coming round at either end.
//!
//! zathura has no such command, so its keys run this, given which way and the window's
//! bus name (`$DBUS`). It asks that zathura what it shows, and asks it to open the one
//! beside, at the place zathura remembers for it.
//!
//! Usage: `zathura-next-document next|previous BUS`
//!
//! A number before the key runs this that many times at once. zathura answers a request to
//! open before it opens, so each run waits its turn on a lock and holds it until zathura
//! shows what it asked for, and the next goes on from there.
//!
//! It takes its help, exits and failures from the terminal kit
//! (docs/commands.md); it answers nothing, since what it does shows in
//! zathura's window.

use std::cmp::Ordering;
use std::ffi::OsStr;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::thread;
use std::time::{Duration, Instant};
use std::{env, io};

use terminal_kit::{Exit, Failure, Help};

/// How long zathura may take to show a document, the one it has or the one asked for.
const PATIENCE: Duration = Duration::from_secs(10);

/// How often zathura is asked what it shows while it opens one.
const EVERY: Duration = Duration::from_millis(50);

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

fn help() -> Help {
    Help::new(
        "zathura-next-document",
        "opens the document beside the one a zathura shows",
    )
    .command("next BUS", "the next in its folder, by name")
    .command("previous BUS", "the one before")
    .example("zathura-next-document next :1.42", "what zathura's ]b runs")
    .exit(
        Exit::Failed,
        "nothing else to open, or zathura couldn't be asked",
    )
}

fn main() -> ExitCode {
    terminal_kit::main(&help(), env!("CARGO_PKG_VERSION"), |context| {
        let (way, bus) = match context.args.as_slice() {
            [way, bus] if way == "next" => (Way::Next, bus),
            [way, bus] if way == "previous" => (Way::Previous, bus),
            _ => {
                return Err(
                    Failure::new(Exit::Usage, "it takes next or previous, then BUS")
                        .try_this("zathura-next-document --help"),
                );
            }
        };
        went(way, bus).map_err(|error| {
            Failure::new(Exit::Failed, "the next document couldn't be opened")
                .because(error.to_string())
        })
    })
}

/// Opens the document `way` of the one the zathura on `bus` shows, in turn with any other
/// run for the same window.
fn went(way: Way, bus: &str) -> io::Result<()> {
    let lock = File::create(lock_for(bus)?)?;
    lock.lock()?;
    let from = awaited(bus, |_| true)?;
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
    awaited(bus, |shown| shown == chosen)?;
    Ok(())
}

/// The lock the runs for the window on `bus` take in turn.
fn lock_for(bus: &str) -> io::Result<PathBuf> {
    let runtime =
        env::var_os("XDG_RUNTIME_DIR").ok_or_else(|| io::Error::other("no XDG_RUNTIME_DIR"))?;
    let folder = Path::new(&runtime).join("zathura-next-document");
    fs::create_dir_all(&folder)?;
    Ok(folder.join(format!("{}.lock", bus.replace('/', "_"))))
}

/// The document the zathura on `bus` shows, once it shows one `wanted` is true of.
fn awaited(bus: &str, wanted: impl Fn(&Path) -> bool) -> io::Result<PathBuf> {
    let started = Instant::now();
    loop {
        if let Some(shown) = showing(bus)?
            && wanted(&shown)
        {
            return Ok(shown);
        }
        if started.elapsed() > PATIENCE {
            return Err(io::Error::other(format!(
                "zathura on {bus} showed no document it was to"
            )));
        }
        thread::sleep(EVERY);
    }
}

/// The document the zathura on `bus` shows, if any: none while it opens one.
fn showing(bus: &str) -> io::Result<Option<PathBuf>> {
    let answer = Command::new(env!("GDBUS"))
        .args(["call", "--session", "--dest", bus, "--object-path"])
        .args(["/org/pwmt/zathura", "--method"])
        .args([
            "org.freedesktop.DBus.Properties.Get",
            "org.pwmt.zathura",
            "filename",
        ])
        .stderr(Stdio::null())
        .output()?;
    if !answer.status.success() {
        return Ok(None);
    }
    Ok(filename_in(&String::from_utf8_lossy(&answer.stdout)).map(PathBuf::from))
}

/// The name in gdbus's answer for the `filename` property, as `(<'/d/a.pdf'>,)`, or none
/// when it's empty. `GVariant` quotes a string holding `'` in `"` instead.
fn filename_in(answer: &str) -> Option<String> {
    let after = &answer[answer.find('<')? + 1..];
    let mut characters = after.chars();
    let quote = characters
        .next()
        .filter(|first| *first == '\'' || *first == '"')?;
    let mut name = String::new();
    while let Some(character) = characters.next() {
        match character {
            '\\' => name.push(characters.next()?),
            ended if ended == quote => return Some(name).filter(|name| !name.is_empty()),
            _ => name.push(character),
        }
    }
    None
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
        .stdout(Stdio::null())
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
    fn the_name_shown_is_read_from_gdbus() {
        assert_eq!(
            filename_in("(<'/d/a.pdf'>,)\n"),
            Some("/d/a.pdf".to_owned())
        );
    }

    #[test]
    fn a_name_holding_a_quote_is_read_from_its_double_quotes() {
        assert_eq!(
            filename_in("(<\"/d/it's \\\"a\\\".pdf\">,)\n"),
            Some("/d/it's \"a\".pdf".to_owned())
        );
    }

    #[test]
    fn no_name_while_nothing_is_shown() {
        assert_eq!(filename_in("(<''>,)\n"), None);
        assert_eq!(filename_in(""), None);
    }

    #[test]
    fn quotes_and_backslashes_are_escaped_for_gdbus() {
        assert_eq!(quoted(r"/d/it's\a.pdf"), r"'/d/it\'s\\a.pdf'");
    }
}
