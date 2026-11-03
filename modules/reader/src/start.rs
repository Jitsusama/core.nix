//! What the reader shows when it's started with no file, as from the
//! launcher's Programs: a page of markdown files to read, each a link that
//! g x follows into this window and Ctrl+O comes back from. Those it kept a
//! place in lately come first, newest first, then those under the home's
//! folders, newest first. It's found within a budget, so the first frame
//! never waits on a large tree, and it's written among the session's
//! runtime files, so it's never said to be shown nor kept a place in.
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

/// How many of each it lists.
const LATELY: usize = 12;
const AROUND: usize = 30;
/// How deep under the home it looks, in folders, and for how long.
const DEEPEST: usize = 3;
const LOOKING: Duration = Duration::from_millis(120);
/// Folders that hold what was built or fetched, never what he wrote.
const BUILT: [&str; 3] = ["node_modules", "target", "result"];

/// Writes the page and gives its path.
pub fn page() -> std::io::Result<PathBuf> {
    let file = at();
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let lately = state().map(|state| lately(&state)).unwrap_or_default();
    let around = home
        .as_deref()
        .map(|home| around(home, &lately, Instant::now() + LOOKING))
        .unwrap_or_default();
    let text = written(&lately, &around, home.as_deref());
    if let Some(folder) = file.parent() {
        std::fs::create_dir_all(folder)?;
    }
    let fresh = file.with_extension(format!("{}.part", std::process::id()));
    std::fs::write(&fresh, text)?;
    std::fs::rename(&fresh, &file)?;
    Ok(file)
}

/// Whether `file` is this page, which is never said shown nor kept.
pub fn is(file: &Path) -> bool {
    crate::links::same(file, &at())
}

fn at() -> PathBuf {
    std::env::var_os("XDG_RUNTIME_DIR")
        .map_or_else(std::env::temp_dir, PathBuf::from)
        .join("reader")
        .join("start.md")
}

fn state() -> Option<PathBuf> {
    std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/state")))
}

/// The files the reader kept a place in, newest first, that are still
/// there.
fn lately(state: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(state.join("reader").join("places")) else {
        return Vec::new();
    };
    let mut kept: Vec<(SystemTime, PathBuf)> = entries
        .flatten()
        .filter_map(|entry| {
            let when = entry.metadata().ok()?.modified().ok()?;
            let text = std::fs::read(entry.path()).ok()?;
            let place: serde_json::Value = serde_json::from_slice(&text).ok()?;
            let file = PathBuf::from(place.get("file")?.as_str()?);
            (file.is_file() && !is(&file)).then_some((when, file))
        })
        .collect();
    kept.sort_by(|a, b| b.0.cmp(&a.0));
    kept.into_iter().map(|(_, file)| file).take(LATELY).collect()
}

/// Markdown files under `home`, folders nearest first, leaving out hidden
/// ones and what was built, until `by`; then the newest of them.
fn around(home: &Path, leave: &[PathBuf], by: Instant) -> Vec<PathBuf> {
    let mut found: Vec<(SystemTime, PathBuf)> = Vec::new();
    let mut folders = std::collections::VecDeque::from([(home.to_path_buf(), 0)]);
    while let Some((folder, depth)) = folders.pop_front() {
        if Instant::now() > by {
            break;
        }
        let Ok(entries) = std::fs::read_dir(&folder) else {
            continue;
        };
        let mut entries: Vec<_> = entries.flatten().collect();
        entries.sort_by_key(std::fs::DirEntry::file_name);
        for entry in entries {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.starts_with('.') {
                continue;
            }
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            let path = entry.path();
            if kind.is_dir() {
                if depth < DEEPEST && !BUILT.contains(&name.as_ref()) {
                    folders.push_back((path, depth + 1));
                }
            } else if kind.is_file()
                && crate::links::markdown(&path)
                && !leave.iter().any(|one| crate::links::same(one, &path))
                && let Ok(when) = entry.metadata().and_then(|it| it.modified())
            {
                found.push((when, path));
            }
        }
    }
    found.sort_by(|a, b| b.0.cmp(&a.0));
    found.into_iter().map(|(_, file)| file).take(AROUND).collect()
}

/// The page itself, as markdown.
fn written(lately: &[PathBuf], around: &[PathBuf], home: Option<&Path>) -> String {
    let mut text = String::from("# What to Read\n\n");
    if lately.is_empty() && around.is_empty() {
        text.push_str(
            "There's no markdown file under your folders yet. Open one from its \
             folder, or start the reader with the file it's to show.\n",
        );
        return text;
    }
    text.push_str(
        "Press g x and a link's letters to read it here, Ctrl+O to come back, \
         and / to find one by its name.\n",
    );
    for (heading, files) in [("Lately", lately), ("In Your Folders", around)] {
        if files.is_empty() {
            continue;
        }
        let _ = write!(text, "\n## {heading}\n\n");
        for file in files {
            let _ = writeln!(text, "- {}", item(file, home));
        }
    }
    text
}

/// One file as a list item: its name as a link, then the folder it's in.
fn item(file: &Path, home: Option<&Path>) -> String {
    let name = file
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let folder = file.parent().map_or_else(String::new, |folder| {
        match home.and_then(|home| folder.strip_prefix(home).ok()) {
            Some(below) if below.as_os_str().is_empty() => "~".to_string(),
            Some(below) => format!("~/{}", below.display()),
            None => folder.display().to_string(),
        }
    });
    format!(
        "[{}](<{}>), in {}",
        escaped(&name),
        file.display()
            .to_string()
            .replace('<', "\\<")
            .replace('>', "\\>"),
        escaped(&folder)
    )
}

/// Words with markdown's punctuation taken as itself.
fn escaped(words: &str) -> String {
    let mut out = String::with_capacity(words.len());
    for letter in words.chars() {
        if letter.is_ascii_punctuation() && !matches!(letter, '/' | '~' | '.' | '-' | ',') {
            out.push('\\');
        }
        out.push(letter);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn folder(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("reader-start-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn it_finds_markdown_under_the_folders_but_not_hidden_or_built_or_too_deep() {
        let home = folder("around");
        for path in [
            "notes.md",
            "src/project/README.md",
            "src/project/docs/deep/too-deep.md",
            ".hidden/secret.md",
            "src/project/target/built.md",
            "src/project/main.rs",
        ] {
            let path = home.join(path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, "# A Page\n").unwrap();
        }
        let found = around(&home, &[], Instant::now() + Duration::from_secs(5));
        let mut names: Vec<_> = found
            .iter()
            .map(|file| file.strip_prefix(&home).unwrap().display().to_string())
            .collect();
        names.sort();
        assert_eq!(names, ["notes.md", "src/project/README.md"]);
        let left = around(
            &home,
            &[home.join("notes.md")],
            Instant::now() + Duration::from_secs(5),
        );
        assert_eq!(left, [home.join("src/project/README.md")]);
        std::fs::remove_dir_all(&home).unwrap();
    }

    #[test]
    fn the_files_read_lately_come_from_the_places_kept() {
        let state = folder("lately");
        let places = state.join("reader").join("places");
        std::fs::create_dir_all(&places).unwrap();
        let page = state.join("page.md");
        std::fs::write(&page, "# A Page\n").unwrap();
        let gone = state.join("gone.md");
        std::fs::write(
            places.join("a.json"),
            serde_json::json!({"file": page, "block": 0}).to_string(),
        )
        .unwrap();
        std::fs::write(
            places.join("b.json"),
            serde_json::json!({"file": gone, "block": 0}).to_string(),
        )
        .unwrap();
        assert_eq!(lately(&state), [page]);
        std::fs::remove_dir_all(&state).unwrap();
    }

    #[test]
    fn each_file_is_a_link_to_its_whole_path_under_its_folders_name() {
        let home = Path::new("/home/someone");
        let text = written(
            &[home.join("notes [old].md")],
            &[home.join("src/a b/README.md")],
            Some(home),
        );
        assert!(text.starts_with("# What to Read\n"), "{text}");
        assert!(
            text.contains("## Lately\n\n- [notes \\[old\\].md](</home/someone/notes [old].md>), in ~\n"),
            "{text}"
        );
        assert!(
            text.contains(
                "## In Your Folders\n\n- [README.md](</home/someone/src/a b/README.md>), in ~/src/a b\n"
            ),
            "{text}"
        );
    }

    #[test]
    fn with_nothing_to_read_it_says_how_to_open_a_file() {
        let text = written(&[], &[], None);
        assert!(text.contains("There's no markdown file"), "{text}");
        assert!(!text.contains("##"), "{text}");
    }
}
