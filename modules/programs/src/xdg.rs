//! Where desktop entries are, as the XDG Base Directory Specification
//! lists the data folders, and each entry's id as Quickshell gives it:
//! its file's path under `applications`, `/` made `-`, without `.desktop`.

use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};

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

/// The most links a data folder is followed through, so a loop of them
/// ends.
const LINKS_FOLLOWED: usize = 32;

/// The links a data folder resolves through, each as the folder holding
/// it and its name, as `/run` and `current-system`: a switch or an
/// upgrade replaces one of them, which inotify on the folder itself never
/// says, since what it resolves to is a store path that never changes.
/// A link inside the store is left out, for the same reason.
pub fn links_toward(folder: &Path) -> Vec<(PathBuf, String)> {
    let store = Path::new("/nix/store");
    let mut links = Vec::new();
    let mut toward = vec![folder.to_path_buf()];
    while let Some(path) = toward.pop() {
        if links.len() >= LINKS_FOLLOWED {
            break;
        }
        for at in path.ancestors() {
            if at.starts_with(store) {
                break;
            }
            let is_link = std::fs::symlink_metadata(at).is_ok_and(|it| it.is_symlink());
            let (Some(parent), Some(name)) = (at.parent(), at.file_name()) else {
                continue;
            };
            if !is_link {
                continue;
            }
            let link = (parent.to_path_buf(), name.to_string_lossy().into_owned());
            if links.contains(&link) {
                continue;
            }
            links.push(link);
            if let Ok(target) = std::fs::read_link(at) {
                toward.push(lexical(&parent.join(target)));
            }
        }
    }
    links
}

/// A path with its `.` and `..` taken out by their words alone, so a
/// link's relative target reads as the folder it names.
fn lexical(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for part in path.components() {
        match part {
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => {}
            other => out.push(other),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_data_folder_is_followed_through_each_link_a_switch_replaces() {
        let root = std::env::temp_dir().join(format!("programs-links-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let built = root.join("system-2/sw/share");
        std::fs::create_dir_all(&built).unwrap();
        std::fs::create_dir_all(root.join("profiles")).unwrap();
        std::os::unix::fs::symlink("system-2", root.join("profiles/system")).unwrap();
        std::os::unix::fs::symlink("./profiles/system", root.join("current")).unwrap();
        // Only those under the test's own folder, since the temporary
        // folder may itself be reached through a link.
        let mut links: Vec<_> = links_toward(&root.join("current/sw/share"))
            .into_iter()
            .filter(|(folder, _)| folder.starts_with(&root))
            .collect();
        links.sort();
        assert_eq!(
            links,
            [
                (root.clone(), "current".to_owned()),
                (root.join("profiles"), "system".to_owned()),
            ]
        );
        assert!(
            links_toward(&built)
                .iter()
                .all(|(folder, _)| !folder.starts_with(&root))
        );
        std::fs::remove_dir_all(&root).unwrap();
    }

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
