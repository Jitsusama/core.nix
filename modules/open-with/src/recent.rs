//! The desktop's list of files opened lately: recently-used.xbel in the data
//! folder, as the freedesktop desktop bookmark spec lays it out. It's read
//! and written through GLib's own `BookmarkFile`, as GTK's `RecentManager`
//! does, so a file is kept the way every GTK program reads it: its kind,
//! when it was added, changed and visited, and each program that opened it,
//! how often and when last. GIMP and Inkscape add to it themselves; Neovim
//! and what open-with opens add to it through this, so the launcher reads
//! one list for every program's files rather than one of each program's.

use std::error::Error;
use std::path::{Path, PathBuf};

use glib::{BookmarkFile, DateTime};

/// How long a file stays on the list once nothing has opened it, in seconds:
/// GTK's own default (gtk-recent-files-max-age), thirty days.
pub const KEPT: i64 = 30 * 24 * 60 * 60;

/// Why the list couldn't be read or written: GLib's reason, or the disk's.
pub type Failed = Box<dyn Error>;

/// Where the list is: recently-used.xbel in the data folder.
#[must_use]
pub fn place() -> PathBuf {
    glib::user_data_dir().join("recently-used.xbel")
}

/// A file one program opened, and when it last did, in microseconds.
#[derive(Debug, PartialEq, Eq)]
pub struct Opened {
    pub program: String,
    pub path: PathBuf,
    pub at: i64,
}

/// The list at `list`, or an empty one where there's none yet.
fn read(list: &Path) -> Result<BookmarkFile, glib::Error> {
    let mut bookmarks = BookmarkFile::new();
    if list.exists() {
        bookmarks.load_from_file(list)?;
    }
    Ok(bookmarks)
}

/// Says that `program` opened each of `paths` at `at` (now, when none), as
/// the command `exec` runs it, `%u` standing for the file (given as it
/// runs, since GLib quotes it as it stores it), each a
/// microsecond after the one before, so the last is the newest. A file already
/// listed gains one to the program's count; one that isn't is added with
/// its kind as GIO guesses it from its name. Anything nothing has opened in
/// `KEPT` goes, and the list is written anew and moved over the old one,
/// as GLib writes it, so a reader never sees half of it.
pub fn add(list: &Path, program: &str, exec: &str, paths: &[PathBuf], at: Option<&DateTime>) -> Result<(), Failed> {
    let mut bookmarks = read(list)?;
    let start = match at {
        Some(at) => at.clone(),
        None => DateTime::now_utc()?,
    };
    for (step, path) in (0..).zip(paths) {
        let uri = glib::filename_to_uri(path, None)?;
        let stamp = start.add(glib::TimeSpan::from_microseconds(step))?;
        if !bookmarks.has_item(&uri) {
            let (kind, _) = gio::content_type_guess(Some(path), None);
            let mime = gio::content_type_get_mime_type(&kind).unwrap_or_else(|| "application/octet-stream".into());
            bookmarks.set_mime_type(&uri, &mime);
        }
        bookmarks.set_application_info(&uri, program, exec, -1, Some(&stamp))?;
        bookmarks.set_visited_date_time(&uri, &stamp);
    }
    // When a program last opened it, rather than the bookmark's own
    // modified time, which GLib sets from the clock as anything changes.
    let now = start.to_unix();
    for uri in bookmarks.uris() {
        let last = bookmarks
            .applications(&uri)?
            .iter()
            .filter_map(|program| bookmarks.application_info(&uri, program).ok())
            .map(|(_, _, stamp)| stamp.to_unix())
            .max();
        if last.is_none_or(|last| now - last > KEPT) {
            bookmarks.remove_item(&uri)?;
        }
    }
    if let Some(folder) = list.parent() {
        std::fs::create_dir_all(folder)?;
    }
    Ok(bookmarks.to_file(list)?)
}

/// Every local file on the list beside each program that opened it, the one
/// opened last first. A link that names no local file (a web page, a file
/// on another machine) isn't one, and is left out.
pub fn list(list: &Path) -> Result<Vec<Opened>, Failed> {
    let bookmarks = read(list)?;
    let mut opened = Vec::new();
    for uri in bookmarks.uris() {
        let Ok((path, None)) = glib::filename_from_uri(&uri) else {
            continue;
        };
        for program in bookmarks.applications(&uri)? {
            let stamp = bookmarks.application_info(&uri, &program)?.2;
            let at = stamp.to_unix() * 1_000_000 + i64::from(stamp.microsecond());
            opened.push(Opened {
                program: program.to_string(),
                path: path.clone(),
                at,
            });
        }
    }
    opened.sort_by(|one, other| other.at.cmp(&one.at).then_with(|| one.path.cmp(&other.path)));
    Ok(opened)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(seconds: i64) -> DateTime {
        DateTime::from_unix_utc(seconds).unwrap()
    }

    fn folder() -> PathBuf {
        let folder = std::env::temp_dir().join(format!("recent-{}-{:?}", std::process::id(), std::thread::current().id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).unwrap();
        folder
    }

    fn seen(list: &Path) -> Vec<(String, String, i64)> {
        super::list(list)
            .unwrap()
            .into_iter()
            .map(|each| (each.program, each.path.file_name().unwrap().to_string_lossy().into_owned(), each.at))
            .collect()
    }

    #[test]
    fn each_file_a_program_opens_is_listed_newest_first() {
        let here = folder();
        let list = here.join("data/recently-used.xbel");
        add(&list, "nvim", "nvim %u", &[here.join("one.txt")], Some(&at(1_000_000))).unwrap();
        add(&list, "nvim", "nvim %u", &[here.join("two.md")], Some(&at(1_000_100))).unwrap();
        add(&list, "gimp", "gimp %u", &[here.join("one.txt")], Some(&at(1_000_050))).unwrap();
        assert_eq!(
            seen(&list),
            [
                ("nvim".to_owned(), "two.md".to_owned(), 1_000_100_000_000),
                ("gimp".to_owned(), "one.txt".to_owned(), 1_000_050_000_000),
                ("nvim".to_owned(), "one.txt".to_owned(), 1_000_000_000_000),
            ]
        );
    }

    #[test]
    fn files_said_together_keep_their_order_the_last_newest() {
        let here = folder();
        let list = here.join("recently-used.xbel");
        let files = [here.join("c.txt"), here.join("a.txt"), here.join("b.txt")];
        add(&list, "nvim", "nvim %u", &files, Some(&at(1_000_000))).unwrap();
        let names: Vec<_> = seen(&list).into_iter().map(|each| each.1).collect();
        assert_eq!(names, ["b.txt", "a.txt", "c.txt"]);
    }

    #[test]
    fn opening_a_file_again_counts_it_and_moves_it_first() {
        let here = folder();
        let list = here.join("recently-used.xbel");
        let one = here.join("one.txt");
        add(&list, "nvim", "nvim %u", std::slice::from_ref(&one), Some(&at(1_000_000))).unwrap();
        add(&list, "nvim", "nvim %u", &[here.join("two.txt")], Some(&at(1_000_010))).unwrap();
        add(&list, "nvim", "nvim %u", std::slice::from_ref(&one), Some(&at(1_000_020))).unwrap();
        let read = read(&list).unwrap();
        let uri = glib::filename_to_uri(&one, None).unwrap();
        assert_eq!(read.application_info(&uri, "nvim").unwrap().1, 2, "counted twice");
        assert_eq!(
            read.application_info(&uri, "nvim").unwrap().0,
            format!("nvim {uri}"),
            "its command as it was given, for GLib quotes it itself"
        );
        assert_eq!(read.mime_type(&uri).unwrap(), "text/plain", "its kind, from its name");
        assert_eq!(read.visited_date_time(&uri).unwrap().to_unix(), 1_000_020, "visited as it was opened");
        assert_eq!(seen(&list)[0].1, "one.txt");
    }

    #[test]
    fn what_gtk_wrote_is_kept_as_it_was() {
        let here = folder();
        let list = here.join("recently-used.xbel");
        std::fs::write(
            &list,
            r#"<?xml version="1.0" encoding="UTF-8"?>
<xbel version="1.0" xmlns:bookmark="http://www.freedesktop.org/standards/desktop-bookmarks" xmlns:mime="http://www.freedesktop.org/standards/shared-mime-info">
  <bookmark href="file:///home/someone/a%20picture.png" added="2026-10-09T21:10:28Z" modified="2026-10-09T21:39:25Z" visited="2026-10-09T21:10:28Z">
    <info><metadata owner="http://freedesktop.org">
      <mime:mime-type type="image/png"/>
      <bookmark:groups><bookmark:group>gimp</bookmark:group></bookmark:groups>
      <bookmark:applications>
        <bookmark:application name="GNU Image Manipulation Program" exec="&apos;gimp %u&apos;" modified="2026-10-09T21:39:25Z" count="3"/>
      </bookmark:applications>
    </metadata></info>
  </bookmark>
  <bookmark href="https://example.org/" added="2026-10-09T21:10:28Z" modified="2026-10-09T21:10:28Z" visited="2026-10-09T21:10:28Z">
    <info><metadata owner="http://freedesktop.org">
      <mime:mime-type type="text/html"/>
      <bookmark:applications><bookmark:application name="chrome" exec="chrome %u" modified="2026-10-09T21:10:28Z" count="1"/></bookmark:applications>
    </metadata></info>
  </bookmark>
</xbel>"#,
        )
        .unwrap();
        let october = DateTime::from_iso8601("2026-10-10T00:00:00Z", None).unwrap();
        add(&list, "nvim", "nvim %u", &[here.join("notes.md")], Some(&october)).unwrap();
        let read = read(&list).unwrap();
        let picture = "file:///home/someone/a%20picture.png";
        assert_eq!(read.application_info(picture, "GNU Image Manipulation Program").unwrap().1, 3);
        assert_eq!(read.groups(picture).unwrap(), ["gimp"]);
        assert_eq!(read.mime_type(picture).unwrap(), "image/png");
        assert_eq!(
            seen(&list).into_iter().map(|each| (each.0, each.1)).collect::<Vec<_>>(),
            [
                ("nvim".to_owned(), "notes.md".to_owned()),
                ("GNU Image Manipulation Program".to_owned(), "a picture.png".to_owned())
            ],
            "a web page names no file, so only files are listed"
        );
    }

    #[test]
    fn a_file_nothing_opened_in_thirty_days_goes() {
        let here = folder();
        let list = here.join("recently-used.xbel");
        let day = 24 * 60 * 60;
        add(&list, "nvim", "nvim %u", &[here.join("old.txt")], Some(&at(1_000_000))).unwrap();
        add(&list, "nvim", "nvim %u", &[here.join("kept.txt")], Some(&at(1_000_000 + 2 * day))).unwrap();
        add(&list, "nvim", "nvim %u", &[here.join("new.txt")], Some(&at(1_000_000 + 31 * day))).unwrap();
        let names: Vec<_> = seen(&list).into_iter().map(|each| each.1).collect();
        assert_eq!(names, ["new.txt", "kept.txt"]);
    }

    #[test]
    fn no_list_yet_is_an_empty_one() {
        assert_eq!(super::list(&folder().join("none.xbel")).unwrap(), []);
    }
}
