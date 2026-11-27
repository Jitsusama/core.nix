//! Writing a file whole: cadence keeps each reading, ledger and note by
//! staging it beside its place and renaming it in, so no reader ever sees
//! half of one. And reading one another program writes whole the same
//! way, as text line by line, naming the first line that isn't text. And
//! reading a record cadence keeps, so one torn is refused, not emptied.

use serde::de::DeserializeOwned;
use std::fs;
use std::io;
use std::path::Path;
use std::time::{Duration, SystemTime};

/// How old a staged copy must be before a write takes it for one a run
/// killed between its write and its rename left behind: a write stages
/// and renames within a second, and an hour runs for at most half of one.
const LEFT_AFTER: Duration = Duration::from_secs(3600);

/// Writes `text` to `path` whole. It's staged under this process's name,
/// so an hour run by hand can't write into the one the timer's renames,
/// and removed if it fails, as no later run shares it. It's synced
/// first, so a crash leaves a file whole: a kept reading that won't parse
/// holds the hour's findings unjudged. Staged copies a killed run left in
/// the same folder are swept first, so they don't pile up there, even
/// beside a file named for its day that is never written again.
pub fn write_whole(path: &Path, text: &str) -> io::Result<()> {
    sweep_left(path);
    let staged = path.with_extension(format!("new.{}", std::process::id()));
    let written = fs::File::create(&staged).and_then(|mut file| {
        io::Write::write_all(&mut file, text.as_bytes())?;
        file.sync_all()
    });
    let done = written.and_then(|()| fs::rename(&staged, path));
    if done.is_err() {
        let _ = fs::remove_file(&staged);
    }
    done
}

/// The lines of `bytes` that are text, joined again, beside the number of
/// the first line that isn't, counted from one, so a reader can say the
/// file isn't whole rather than read its other lines as all of it. A file
/// written whole and moved into place, as each read here is, is never
/// caught half written, so a last line cut short is named like any other.
pub fn text_lines(bytes: &[u8]) -> (String, Option<usize>) {
    let mut kept = Vec::new();
    let mut first_not = None;
    for (n, line) in bytes.split(|&byte| byte == b'\n').enumerate() {
        match std::str::from_utf8(line) {
            Ok(text) => kept.push(text),
            Err(_) => first_not = first_not.or(Some(n + 1)),
        }
    }
    (kept.join("\n"), first_not)
}

/// The record kept as JSON at `path`, its default when it was never
/// written. One that can't be read or parsed into its shape is an error,
/// so a tick that tells from it and writes it back leaves it as it is
/// rather than writing an emptied record over it: read as empty, a torn
/// record would tell again everything it held, and an hour reading it
/// whole would count none.
///
/// # Errors
///
/// When the record can't be read, or isn't JSON of its shape.
pub fn read_kept<T: DeserializeOwned + Default>(path: &Path) -> io::Result<T> {
    match fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes).map_err(|err| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{} isn't JSON of its shape, so it's kept as it is: {err}",
                    path.display()
                ),
            )
        }),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(T::default()),
        Err(err) => Err(io::Error::new(
            err.kind(),
            format!(
                "{} can't be read, so it's kept as it is: {err}",
                path.display()
            ),
        )),
    }
}

/// Removes the copies staged beside `path` by runs long gone: those named
/// as `write_whole` stages any file, under any process, older than
/// `LEFT_AFTER`. A folder that can't be read is left as it is, as a write
/// into it will say why on its own.
fn sweep_left(path: &Path) {
    let folder = path
        .parent()
        .filter(|folder| !folder.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let Ok(entries) = fs::read_dir(folder) else {
        return;
    };
    let now = SystemTime::now();
    for entry in entries.flatten() {
        let name = entry.file_name();
        let named = name
            .to_str()
            .and_then(|name| name.rsplit_once(".new."))
            .is_some_and(|(_, pid)| !pid.is_empty() && pid.bytes().all(|b| b.is_ascii_digit()));
        if !named {
            continue;
        }
        let left = entry
            .metadata()
            .and_then(|meta| meta.modified())
            .ok()
            .and_then(|at| now.duration_since(at).ok())
            .is_some_and(|age| age > LEFT_AFTER);
        if left {
            let _ = fs::remove_file(entry.path());
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;

    fn scratch() -> tempfile::TempDir {
        tempfile::tempdir().expect("a scratch folder")
    }

    fn names(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(dir)
            .expect("the scratch folder reads")
            .flatten()
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn lines_that_are_text_are_kept_as_written_and_the_first_that_is_not_is_named() {
        assert_eq!(
            text_lines(b"a\r\nb\n"),
            ("a\r\nb\n".to_owned(), None),
            "lines all text didn't come back as they were"
        );
        assert_eq!(
            text_lines(b"a\n\xff\nb\n\xfe\nc"),
            ("a\nb\nc".to_owned(), Some(2)),
            "the lines that aren't text weren't left out, or the first wasn't named"
        );
        assert_eq!(
            text_lines(b"a\nb\xc3"),
            ("a".to_owned(), Some(2)),
            "a last line cut short partway through a character wasn't named"
        );
        assert_eq!(
            text_lines(b"a\nb\xff\n"),
            ("a\n".to_owned(), Some(2)),
            "a whole last line that isn't text wasn't named"
        );
    }

    #[test]
    fn a_kept_record_never_written_is_its_default_and_one_torn_or_misshapen_is_refused() {
        let scratch = scratch();
        let path = scratch.path().join("units.json");
        let never: io::Result<BTreeMap<String, i64>> = read_kept(&path);
        assert!(
            never.is_ok_and(|kept| kept.is_empty()),
            "a record never written wasn't read as empty"
        );
        fs::write(&path, "{\"a\": 1}").expect("a whole record");
        let whole: io::Result<BTreeMap<String, i64>> = read_kept(&path);
        assert_eq!(
            whole.ok(),
            Some(BTreeMap::from([("a".to_owned(), 1)])),
            "a whole record wasn't read as it was written"
        );
        for (kept, what) in [
            ("{\"a\": 1", "torn"),
            ("{\"a\": \"one\"}", "of another shape"),
        ] {
            fs::write(&path, kept).expect("a record that won't parse");
            let read: io::Result<BTreeMap<String, i64>> = read_kept(&path);
            let err = read.expect_err(&format!("a record {what} was read"));
            assert_eq!(
                err.kind(),
                io::ErrorKind::InvalidData,
                "a record {what} wasn't refused as unparsed: {err}"
            );
            assert!(
                err.to_string().contains("kept as it is"),
                "a record {what} didn't say it was kept: {err}"
            );
        }
        // One that can't be read at all, here a folder in its place, is
        // refused naming where it is, never read as empty.
        fs::remove_file(&path).expect("the torn record goes");
        fs::create_dir_all(&path).expect("a folder where the record goes");
        let unreadable: io::Result<BTreeMap<String, i64>> = read_kept(&path);
        let err = unreadable.expect_err("a record that can't be read was read");
        assert!(
            err.to_string().contains(&path.display().to_string()),
            "a record that can't be read didn't name its path: {err}"
        );
    }

    #[test]
    fn a_file_is_written_whole_and_leaves_no_staged_copy() {
        let scratch = scratch();
        let dir = scratch.path();
        let path = dir.join("hour.json");
        write_whole(&path, "{}").expect("it writes");
        write_whole(&path, "{\"a\": 1}").expect("it writes over the last");
        assert_eq!(
            fs::read_to_string(&path).ok().as_deref(),
            Some("{\"a\": 1}"),
            "the second write didn't replace the first"
        );
        assert_eq!(
            names(dir),
            ["hour.json"],
            "a staged copy was left behind after its rename"
        );
    }

    #[test]
    fn a_failed_rename_takes_its_staged_copy_with_it() {
        let scratch = scratch();
        let dir = scratch.path();
        // A folder where the file goes can't be renamed over.
        let path = dir.join("hour.json");
        fs::create_dir(&path).expect("a folder in the file's place");
        assert!(
            write_whole(&path, "{}").is_err(),
            "a rename over a folder was taken for written"
        );
        assert_eq!(
            names(dir),
            ["hour.json"],
            "the staged copy was left behind by a failed rename"
        );
    }

    #[test]
    fn copies_a_killed_run_staged_are_swept_and_a_running_ones_are_not() {
        let scratch = scratch();
        let dir = scratch.path();
        let stage = |name: &str, age: u64| {
            let file = fs::File::create(dir.join(name)).expect("a staged copy");
            let at = SystemTime::now() - Duration::from_secs(age);
            file.set_modified(at).expect("its time set");
        };
        stage("hour.new.1", 7200);
        stage("hour.new.2", 60);
        // A day's file, never written again, leaves its copy beside the hour.
        stage("2026-11-22.new.3", 7200);
        stage("hour.new.notes", 7200);
        write_whole(&dir.join("hour.json"), "{}").expect("it writes");
        assert_eq!(
            names(dir),
            ["hour.json", "hour.new.2", "hour.new.notes"],
            "only the copies killed runs left should go"
        );
    }
}
