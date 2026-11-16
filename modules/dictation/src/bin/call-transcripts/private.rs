//! Keeps what was said on Joel's calls his alone: each call's folder
//! readable by him only, and each file in it too. What's said on a work
//! call is nobody else's, and a folder anyone may read leaves home's own
//! mode the only wall around it.

use std::fs::{DirBuilder, OpenOptions, Permissions};
use std::io::Write;
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt};
use std::path::Path;

/// A call's folder, and the folder of calls: Joel may read, write and
/// enter it, nobody else anything.
const FOLDER: u32 = 0o700;

/// A call's file: Joel may read and write it, nobody else anything.
const FILE: u32 = 0o600;

/// Writes `text` to `file` whole, as dictation's daemon.rs does, through
/// a part beside it renamed into place, readable by Joel alone.
pub fn write_whole(file: &Path, text: &str) -> std::io::Result<()> {
    let name = file.file_name().unwrap_or_default().to_string_lossy();
    let part = file.with_file_name(format!(".{name}.part"));
    let mut written = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(FILE)
        .open(&part)?;
    // A part an earlier run left keeps the mode it was made with, which
    // opening it doesn't change, so it's said again.
    written.set_permissions(Permissions::from_mode(FILE))?;
    written.write_all(text.as_bytes())?;
    drop(written);
    std::fs::rename(&part, file)
}

/// Makes `folder`, and any folder above it that isn't there yet, each
/// Joel's alone.
pub fn make_folder(folder: &Path) -> std::io::Result<()> {
    DirBuilder::new()
        .recursive(true)
        .mode(FOLDER)
        .create(folder)
}

/// Makes `calls` and every folder and file in it Joel's alone, since the
/// calls written before this daemon kept them so were readable by
/// anyone. A link is left as it is and not followed, so nothing outside
/// the calls changes. What can't be changed is said and passed by.
pub fn keep_private(calls: &Path) {
    let Ok(data) = std::fs::symlink_metadata(calls) else {
        return;
    };
    let mode = if data.is_dir() {
        FOLDER
    } else if data.is_file() {
        FILE
    } else {
        return;
    };
    if data.permissions().mode() & 0o777 != mode
        && let Err(why) = std::fs::set_permissions(calls, Permissions::from_mode(mode))
    {
        eprintln!(
            "call-transcripts: keeping {} private: {why}",
            calls.display()
        );
    }
    if data.is_dir() {
        for entry in std::fs::read_dir(calls).into_iter().flatten().flatten() {
            keep_private(&entry.path());
        }
    }
}

#[cfg(test)]
mod tests {
    use std::os::unix::fs::PermissionsExt;

    use super::*;

    fn mode(path: &Path) -> u32 {
        std::fs::symlink_metadata(path)
            .unwrap()
            .permissions()
            .mode()
            & 0o777
    }

    #[test]
    fn a_calls_folder_and_its_files_are_written_for_joel_alone() {
        let calls = tempfile::tempdir().unwrap();
        let folder = calls.path().join("2026-11-15 Planning");
        make_folder(&folder).unwrap();
        let file = folder.join("transcript.md");
        write_whole(&file, "# Transcript\n").unwrap();
        assert_eq!(
            (mode(&folder), mode(&file)),
            (0o700, 0o600),
            "a call's folder or file was written readable by others"
        );
    }

    #[test]
    fn a_part_left_open_to_all_is_closed_when_written_again() {
        let calls = tempfile::tempdir().unwrap();
        let file = calls.path().join("summary.md");
        let part = calls.path().join(".summary.md.part");
        std::fs::write(&part, "left").unwrap();
        std::fs::set_permissions(&part, std::fs::Permissions::from_mode(0o644)).unwrap();
        write_whole(&file, "summed\n").unwrap();
        assert_eq!(
            mode(&file),
            0o600,
            "a part an earlier run left kept its open mode"
        );
    }

    #[test]
    fn calls_written_before_are_made_joels_alone_and_links_are_left() {
        let calls = tempfile::tempdir().unwrap();
        let folder = calls.path().join("2026-11-14 Standup");
        std::fs::create_dir(&folder).unwrap();
        let file = folder.join("transcript.md");
        std::fs::write(&file, "# Transcript\n").unwrap();
        let elsewhere = tempfile::tempdir().unwrap();
        let outside = elsewhere.path().join("shared.md");
        std::fs::write(&outside, "not a call's").unwrap();
        std::os::unix::fs::symlink(&outside, folder.join("shared.md")).unwrap();
        for (path, open) in [
            (calls.path(), 0o755),
            (&folder, 0o755),
            (&file, 0o644),
            (&outside, 0o644),
        ] {
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(open)).unwrap();
        }
        keep_private(calls.path());
        assert_eq!(
            (mode(calls.path()), mode(&folder), mode(&file)),
            (0o700, 0o700, 0o600),
            "a call written before was left readable by others"
        );
        assert_eq!(
            mode(&outside),
            0o644,
            "a link was followed out of the calls"
        );
    }
}
