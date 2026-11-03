//! The file this reader shows, kept where the launcher's Recent reads it:
//! a file named for the process, holding its id, a tab and the file's whole
//! path, under the state home's reader/showing. It's written aside under a
//! name starting with a dot and moved into place, so a reader never sees
//! half of one, and taken away as the reader closes. Files left by a reader
//! that died are cleared as the next one starts.
use std::path::{Path, PathBuf};

fn folder() -> Option<PathBuf> {
    let state = std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/state"))
        })?;
    Some(state.join("reader").join("showing"))
}

/// Says this reader shows `file`, clearing what readers gone left.
pub fn keep(file: &Path) -> std::io::Result<()> {
    match folder() {
        Some(folder) => keep_in(&folder, file),
        None => Ok(()),
    }
}

fn keep_in(folder: &Path, file: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(folder)?;
    clear_gone(folder);
    let pid = std::process::id();
    let whole = std::fs::canonicalize(file)?;
    let aside = folder.join(format!(".{pid}"));
    std::fs::write(&aside, format!("{pid}\t{}\n", whole.display()))?;
    std::fs::rename(&aside, folder.join(pid.to_string()))
}

/// Takes this reader's file away, as it closes.
pub fn forget() {
    if let Some(folder) = folder() {
        forget_in(&folder);
    }
}

fn forget_in(folder: &Path) {
    let _ = std::fs::remove_file(folder.join(std::process::id().to_string()));
}

/// Removes the file of every reader no longer running.
fn clear_gone(folder: &Path) {
    let Ok(entries) = std::fs::read_dir(folder) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(pid) = name
            .to_str()
            .and_then(|name| name.trim_start_matches('.').parse::<u32>().ok())
        else {
            continue;
        };
        if !Path::new("/proc").join(pid.to_string()).exists() {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{forget_in, keep_in};

    #[test]
    fn a_reader_says_the_file_it_shows_and_takes_it_away_as_it_closes() {
        let dir = std::env::temp_dir().join(format!("reader-showing-{}", std::process::id()));
        let shown = dir.join("state/reader/showing");
        std::fs::create_dir_all(&shown).expect("a folder");
        let file = dir.join("page.md");
        std::fs::write(&file, "# A page").expect("the file");
        // A reader long gone left its file behind.
        std::fs::write(shown.join("4194303"), "4194303\t/gone.md\n").expect("a stale file");
        keep_in(&shown, &file).expect("kept");
        let pid = std::process::id();
        let said = std::fs::read_to_string(shown.join(pid.to_string())).expect("said");
        let whole = std::fs::canonicalize(&file).expect("whole");
        assert_eq!(said, format!("{pid}\t{}\n", whole.display()));
        assert!(
            !shown.join("4194303").exists(),
            "a gone reader's file is cleared"
        );
        forget_in(&shown);
        assert!(
            !shown.join(pid.to_string()).exists(),
            "closing takes it away"
        );
        std::fs::remove_dir_all(&dir).expect("cleaned");
    }
}
