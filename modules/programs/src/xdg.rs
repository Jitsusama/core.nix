//! Where desktop entries are, as the XDG Base Directory Specification
//! lists the data folders, and each entry's id as Quickshell gives it:
//! its file's path under `applications`, `/` made `-`, without `.desktop`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// The data folders, nearest first: `XDG_DATA_HOME` (or
/// `~/.local/share`), then each of `XDG_DATA_DIRS` (or `/usr/local/share`
/// and `/usr/share`), each once.
pub fn data_folders(
    home: Option<&str>,
    data_home: Option<&str>,
    data_dirs: Option<&str>,
) -> Vec<PathBuf> {
    let nearest = data_home
        .filter(|folder| !folder.is_empty())
        .map(PathBuf::from)
        .or_else(|| home.map(|home| Path::new(home).join(".local/share")));
    let rest = data_dirs
        .filter(|dirs| !dirs.is_empty())
        .unwrap_or("/usr/local/share:/usr/share");
    let mut folders: Vec<PathBuf> = Vec::new();
    for folder in nearest.into_iter().chain(
        rest.split(':')
            .filter(|dir| !dir.is_empty())
            .map(PathBuf::from),
    ) {
        if !folders.contains(&folder) {
            folders.push(folder);
        }
    }
    folders
}

/// The data folders this process was started with.
pub fn folders_here() -> Vec<PathBuf> {
    let var = |name| std::env::var(name).ok();
    data_folders(
        var("HOME").as_deref(),
        var("XDG_DATA_HOME").as_deref(),
        var("XDG_DATA_DIRS").as_deref(),
    )
}

/// Every desktop entry along the folders, by its id, the nearest folder's
/// winning; and every folder looked in, for inotify to watch.
pub fn entries(folders: &[PathBuf]) -> (BTreeMap<String, PathBuf>, Vec<PathBuf>) {
    let mut found = BTreeMap::new();
    let mut looked = Vec::new();
    for folder in folders {
        let applications = folder.join("applications");
        walk(&applications, &applications, &mut found, &mut looked);
    }
    (found, looked)
}

fn walk(
    base: &Path,
    folder: &Path,
    found: &mut BTreeMap<String, PathBuf>,
    looked: &mut Vec<PathBuf>,
) {
    let Ok(listed) = std::fs::read_dir(folder) else {
        return;
    };
    looked.push(folder.to_path_buf());
    let mut paths: Vec<PathBuf> = listed
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .collect();
    paths.sort();
    for path in paths {
        if path.is_dir() {
            walk(base, &path, found, looked);
        } else if let Some(id) = id_of(base, &path) {
            found.entry(id).or_insert(path);
        }
    }
}

/// An entry's id from its path under `applications`.
pub fn id_of(base: &Path, path: &Path) -> Option<String> {
    let inside = path.strip_prefix(base).ok()?.to_str()?;
    let id = inside.strip_suffix(".desktop")?.replace('/', "-");
    (!id.is_empty()).then_some(id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_data_folders_are_nearest_first_each_once() {
        assert_eq!(
            data_folders(Some("/home/joel"), None, Some("/a:/b::/a")),
            [
                PathBuf::from("/home/joel/.local/share"),
                "/a".into(),
                "/b".into()
            ]
        );
        assert_eq!(
            data_folders(None, Some("/data"), None),
            [
                PathBuf::from("/data"),
                "/usr/local/share".into(),
                "/usr/share".into()
            ]
        );
    }

    #[test]
    fn an_id_is_its_path_under_applications() {
        let base = Path::new("/share/applications");
        assert_eq!(
            id_of(
                base,
                Path::new("/share/applications/kde/org.kde.okular.desktop")
            )
            .as_deref(),
            Some("kde-org.kde.okular")
        );
        assert_eq!(
            id_of(base, Path::new("/share/applications/mimeinfo.cache")),
            None
        );
    }
}
