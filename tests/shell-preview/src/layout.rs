//! The preview's own Quickshell folder and niri config, laid out from what
//! the build made, so the working copy's files show as they're edited.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::fs;
use std::io;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};

/// The working copy's files, each by what it holds, so a built file holding
/// the same is known for that file.
pub type Known = HashMap<Vec<u8>, PathBuf>;

/// Give the working copy's files at those paths under its root by what
/// each holds, passing over one that's gone, as a file deleted but still
/// tracked is.
pub fn known(root: &Path, tracked: &[PathBuf]) -> Known {
    tracked
        .iter()
        .filter_map(|name| {
            let path = root.join(name);
            fs::read(&path).ok().map(|held| (held, path))
        })
        .collect()
}

/// What laying a folder out made of its files.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct LaidOut {
    /// Files that are the working copy's, as links to it.
    pub linked: usize,
    /// Files the build made, copied.
    pub copied: usize,
}

/// Lay the built folder out in `into`, a file the working copy holds as a
/// link to it and any other copied.
///
/// Home Manager copies a module's file as it is, so a built file is the
/// working copy's file with the same bytes. A folder linked from the
/// built one, rather than in it, isn't followed, as nothing built is.
pub fn lay_out(built: &Path, known: &Known, into: &Path) -> io::Result<LaidOut> {
    let mut laid = LaidOut::default();
    let mut entries: Vec<_> = fs::read_dir(built)?.collect::<Result<_, _>>()?;
    entries.sort_by_key(fs::DirEntry::file_name);
    for entry in entries {
        let path = entry.path();
        let target = into.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            let within = lay_out(&path, known, &target)?;
            laid.linked += within.linked;
            laid.copied += within.copied;
            continue;
        }
        // A built file is often a link into the store, which is followed.
        if !fs::metadata(&path)?.is_file() {
            continue;
        }
        fs::create_dir_all(into)?;
        let held = fs::read(&path)?;
        if let Some(source) = known.get(&held) {
            symlink(source, &target)?;
            laid.linked += 1;
        } else {
            fs::write(&target, held)?;
            laid.copied += 1;
        }
    }
    Ok(laid)
}

/// Copy niri's config from the built folder into `into`, starting the
/// shell laid out at `shell` with that Quickshell, and give its path.
pub fn niri_config(
    built: &Path,
    into: &Path,
    quickshell: &Path,
    shell: &Path,
) -> io::Result<PathBuf> {
    fs::create_dir_all(into)?;
    for entry in fs::read_dir(built)? {
        let path = entry?.path();
        if fs::metadata(&path)?.is_file()
            && let Some(name) = path.file_name()
        {
            fs::write(into.join(name), fs::read(&path)?)?;
        }
    }
    let config = into.join("config.kdl");
    // A KDL string is quoted and escaped as a JSON one is.
    let start: Vec<String> = [quickshell, Path::new("-p"), shell]
        .iter()
        .map(|word| serde_json::Value::from(word.to_string_lossy()).to_string())
        .collect();
    let mut text = fs::read_to_string(&config)?;
    writeln!(text, "\nspawn-at-startup {}", start.join(" ")).map_err(io::Error::other)?;
    fs::write(&config, text)?;
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A built folder, a working copy and a preview's own, made afresh.
    struct Folders {
        root: PathBuf,
    }

    impl Folders {
        fn new(test: &str) -> Self {
            let root =
                std::env::temp_dir().join(format!("shell-preview-{}-{test}", std::process::id()));
            let _ = fs::remove_dir_all(&root);
            for folder in ["built/launcher", "work/modules/quickshell/launcher"] {
                fs::create_dir_all(root.join(folder)).expect("a folder");
            }
            Self { root }
        }

        fn at(&self, path: &str) -> PathBuf {
            self.root.join(path)
        }

        /// Write a file under the root, and give its path.
        fn write(&self, path: &str, text: &str) -> PathBuf {
            let path = self.at(path);
            fs::create_dir_all(path.parent().expect("a folder")).expect("its folder");
            fs::write(&path, text).expect("written");
            path
        }

        /// Give the working copy's files at those paths, by what each holds.
        fn known(&self, tracked: &[&str]) -> Known {
            let tracked: Vec<PathBuf> = tracked.iter().map(PathBuf::from).collect();
            known(&self.at("work"), &tracked)
        }
    }

    #[test]
    fn a_tracked_file_is_linked_to_the_working_copy() {
        let folders = Folders::new("linked");
        let source = folders.write("work/modules/quickshell/Launcher.qml", "Item {}\n");
        folders.write("built/Launcher.qml", "Item {}\n");
        let known = folders.known(&["modules/quickshell/Launcher.qml"]);

        let laid = lay_out(&folders.at("built"), &known, &folders.at("into")).expect("laid out");

        assert_eq!(
            laid,
            LaidOut {
                linked: 1,
                copied: 0
            }
        );
        assert_eq!(
            fs::read_link(folders.at("into/Launcher.qml")).expect("a link"),
            source
        );
    }

    #[test]
    fn a_file_the_build_made_is_copied() {
        let folders = Folders::new("copied");
        folders.write("built/Theme.qml", "QtObject { property color accent }\n");

        let laid = lay_out(&folders.at("built"), &Known::new(), &folders.at("into")).expect("laid");

        assert_eq!(
            laid,
            LaidOut {
                linked: 0,
                copied: 1
            }
        );
        let copy = folders.at("into/Theme.qml");
        assert!(!copy.is_symlink());
        assert!(fs::read_to_string(copy).expect("copied").contains("accent"));
    }

    #[test]
    fn a_file_in_a_folder_keeps_its_place() {
        let folders = Folders::new("place");
        let source = folders.write("work/modules/tuple/Tuple.qml", "Item { id: tuple }\n");
        folders.write("built/launcher/Tuple.qml", "Item { id: tuple }\n");
        let known = folders.known(&["modules/tuple/Tuple.qml"]);

        lay_out(&folders.at("built"), &known, &folders.at("into")).expect("laid out");

        let link = fs::read_link(folders.at("into/launcher/Tuple.qml")).expect("a link");
        assert_eq!(link, source);
    }

    #[test]
    fn an_edited_file_is_copied_as_built() {
        let folders = Folders::new("edited");
        folders.write("work/modules/quickshell/Launcher.qml", "Item { edited }\n");
        folders.write("built/Launcher.qml", "Item {}\n");
        let known = folders.known(&["modules/quickshell/Launcher.qml"]);

        let laid = lay_out(&folders.at("built"), &known, &folders.at("into")).expect("laid out");

        assert_eq!(
            laid,
            LaidOut {
                linked: 0,
                copied: 1
            }
        );
    }

    #[test]
    fn a_built_file_linked_into_the_store_is_read_through_its_link() {
        let folders = Folders::new("store");
        let stored = folders.write("store/Launcher.qml", "Item {}\n");
        symlink(&stored, folders.at("built/Launcher.qml")).expect("a link");
        let source = folders.write("work/modules/quickshell/Launcher.qml", "Item {}\n");
        let known = folders.known(&["modules/quickshell/Launcher.qml"]);

        lay_out(&folders.at("built"), &known, &folders.at("into")).expect("laid out");

        assert_eq!(
            fs::read_link(folders.at("into/Launcher.qml")).expect("a link"),
            source
        );
    }

    #[test]
    fn a_tracked_file_deleted_from_the_working_copy_is_passed_over() {
        let folders = Folders::new("deleted");
        assert!(folders.known(&["modules/quickshell/Gone.qml"]).is_empty());
    }

    #[test]
    fn the_config_starts_the_laid_out_shell_and_keeps_its_includes() {
        let folders = Folders::new("niri");
        folders.write("niri/config.kdl", "include \"binds.kdl\"\n");
        folders.write("niri/binds.kdl", "binds {}\n");

        let config = niri_config(
            &folders.at("niri"),
            &folders.at("into"),
            Path::new("/store/quickshell/bin/quickshell"),
            Path::new("/home/someone/a \"shell\""),
        )
        .expect("copied");

        let binds = fs::read_to_string(folders.at("into/binds.kdl")).expect("copied");
        assert_eq!(binds, "binds {}\n");
        let text = fs::read_to_string(config).expect("copied");
        assert!(text.starts_with("include \"binds.kdl\"\n"), "{text}");
        assert!(
            text.ends_with(concat!(
                r#"spawn-at-startup "/store/quickshell/bin/quickshell" "-p""#,
                r#" "/home/someone/a \"shell\"""#,
                "\n"
            )),
            "{text}"
        );
    }
}
