//! Which file a path names. A path's first part may name a folder Nix
//! built (a mount), which is served whole at `/<name>/`; anything else is
//! the documents sessions write. A folder answers with its `index.html`,
//! and one named without its last slash is sent to it with one, so its
//! page's relative links resolve. Nothing outside a root is reached: a
//! `..`, a hidden name (a writer's half-made `.now.XXXX`), or a link in
//! the documents leading out of them is refused, as the server before
//! this one refused it.
//!
//! A mount is the whole of its site but for one file: its `homes.json`,
//! where the things it's home to are listed for every other site to link
//! to. Those change with the site's data, which Nix never builds, so a
//! writer of their own (Code's is the repos daemon) puts them into the
//! documents under the site's name, and a mount without its own is
//! answered with those.
//!
//! The home's own files come first of all, where the home is written into a
//! folder of its own: its page at `/`, and each of its files at `/<name>`.
//! What a path finds says who made it, Nix or the home's builder or a
//! session, since a page a session wrote is served where its script can't
//! act as one of the sites' own (http.rs). The home's own names never fall
//! through to the documents: until its builder has written one, it isn't
//! ready, so a page or sheet a session left there can't stand in for it.

use std::path::{Path, PathBuf};

use crate::config::Config;

/// The one file a mount may be answered with from the documents.
const HOMES: &str = "homes.json";

/// The files the home's builder writes into the home's folder (home.py's
/// `OWN` and `FILES`), which only that folder answers.
const HOME_OWN: [&str; 7] = [
    "index.html",
    "favicon.svg",
    "home.css",
    "strip.css",
    "llms.txt",
    "index.md",
    "rows.json",
];

/// Who made a file a path found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Made {
    /// Nix, in a mount, or the home's builder, in the home's own folder.
    Built,
    /// A session, in the documents.
    Written,
}

/// What a path found.
#[derive(Debug, PartialEq, Eq)]
pub enum Found {
    /// A file to send, and who made it.
    File(PathBuf, Made),
    /// A folder named without its last slash, sent here instead.
    Folder(String),
    /// Nothing is there.
    Missing,
    /// A path reaching out of its root, or one no file may have.
    Refused,
    /// One of the home's own files, which its builder hasn't written yet.
    Unready,
}

/// Finds the file `path`, a request's path without its query, names.
pub fn find(config: &Config, path: &str) -> Found {
    let Some(decoded) = decoded(path) else {
        return Found::Refused;
    };
    let parts: Vec<&str> = decoded.split('/').filter(|part| !part.is_empty()).collect();
    if parts
        .iter()
        .any(|part| part.starts_with('.') || part.contains('\\') || part.contains('\0'))
    {
        return Found::Refused;
    }
    if let Some(found) = home_file(config, &parts) {
        return found;
    }
    let (root, rest, guarded) = match parts.split_first() {
        Some((first, rest)) if config.mounts.contains_key(*first) => {
            let mount = config.mounts[*first].as_path();
            if *rest == [HOMES] && !mount.join(HOMES).is_file() {
                (config.documents.as_path(), &parts[..], true)
            } else {
                (mount, rest, false)
            }
        }
        _ => (config.documents.as_path(), &parts[..], true),
    };
    let named: PathBuf = rest.iter().collect();
    let mut file = root.join(named);
    if file.is_dir() {
        if !decoded.ends_with('/') {
            // Relative to where the browser asked, from `./`: a path from
            // two slashes (`//elsewhere`), or a backslash, which a browser
            // reads as one, would otherwise send it to another host, and a
            // name with a colon would read as a scheme.
            let last = path.rsplit('/').next().unwrap_or_default();
            return Found::Folder(format!("./{last}/"));
        }
        file.push("index.html");
    }
    if !file.is_file() {
        return Found::Missing;
    }
    // A mount is a store path Nix built, which links into other store
    // paths as it likes; only the documents, which sessions write, are
    // held to their own folder.
    if guarded && !within(&file, root) {
        return Found::Refused;
    }
    Found::File(file, if guarded { Made::Written } else { Made::Built })
}

/// The home's own file `parts` names, its page for none: one file in its
/// folder, never a folder, so every deeper path is a mount's or a document.
/// Answers none where the home has no folder or the name isn't its, and
/// `Unready` for one of its own names its builder hasn't written. Only
/// those names are its, whatever else the folder holds, so nothing put
/// there by another hand is served as the sites' own, and none of them is
/// a script.
fn home_file(config: &Config, parts: &[&str]) -> Option<Found> {
    let name = match parts {
        [] => "index.html",
        [name] if HOME_OWN.contains(name) => *name,
        _ => return None,
    };
    let file = config.home.as_deref()?.join(name);
    Some(if file.is_file() {
        Found::File(file, Made::Built)
    } else {
        Found::Unready
    })
}

/// What a file is, by its name, as a browser is told it.
pub fn kind_of(file: &Path) -> &'static str {
    match file.extension().and_then(|extension| extension.to_str()) {
        Some("html") => "text/html; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("js" | "mjs") => "text/javascript; charset=utf-8",
        Some("json" | "map") => "application/json",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("webp") => "image/webp",
        Some("ico") => "image/x-icon",
        Some("woff2") => "font/woff2",
        Some("woff") => "font/woff",
        Some("txt") => "text/plain; charset=utf-8",
        Some("md") => "text/markdown; charset=utf-8",
        Some("wasm") => "application/wasm",
        _ => "application/octet-stream",
    }
}

fn within(file: &Path, root: &Path) -> bool {
    match (file.canonicalize(), root.canonicalize()) {
        (Ok(file), Ok(root)) => file.starts_with(root),
        _ => false,
    }
}

/// A path with its percent escapes read, or none where one is broken or
/// what it spells isn't UTF-8.
fn decoded(path: &str) -> Option<String> {
    let bytes = path.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut at = 0;
    while let Some(&byte) = bytes.get(at) {
        if byte == b'%' {
            let pair = bytes.get(at + 1..at + 3)?;
            let pair = std::str::from_utf8(pair).ok()?;
            out.push(u8::from_str_radix(pair, 16).ok()?);
            at += 3;
        } else {
            out.push(byte);
            at += 1;
        }
    }
    String::from_utf8(out).ok()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::fs;

    use super::*;

    struct Folders {
        config: Config,
        top: PathBuf,
    }

    impl Drop for Folders {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.top);
        }
    }

    fn folders(name: &str) -> Folders {
        let top =
            std::env::temp_dir().join(format!("local-sites-files-{name}-{}", std::process::id()));
        let documents = top.join("sites");
        let kit = top.join("kit");
        let code = top.join("code");
        for folder in [
            documents.join("mycelium"),
            documents.join("code"),
            kit.clone(),
            code.clone(),
        ] {
            fs::create_dir_all(folder).expect("a folder");
        }
        // A live site's homes, written beside its old record in the
        // documents, where its mount holds neither.
        fs::write(documents.join("code/homes.json"), "written").expect("its homes");
        fs::write(documents.join("code/repos.json"), "old").expect("its old record");
        fs::write(code.join("index.html"), "grove").expect("its page");
        fs::write(documents.join("mycelium/index.html"), "map").expect("a page");
        fs::write(documents.join("mycelium/.now.x1"), "half").expect("a half-made file");
        fs::write(top.join("secret"), "no").expect("a file outside");
        fs::write(kit.join("theme.css"), "roles").expect("the kit");
        // A document a session wrote where the kit's mount has no file.
        fs::create_dir_all(documents.join("kit")).expect("a folder");
        fs::write(documents.join("kit/early.js"), "steals").expect("a document");
        let config = Config {
            listen: "127.0.0.1:8484".parse().expect("an address"),
            names: vec!["localhost".to_owned()],
            documents_name: "documents.localhost".to_owned(),
            documents,
            home: None,
            mounts: BTreeMap::from([("kit".to_owned(), kit), ("code".to_owned(), code)]),
            feeds: BTreeMap::new(),
        };
        Folders { config, top }
    }

    #[test]
    fn a_folder_answers_with_its_index_and_a_mount_with_its_own_file() {
        let folders = folders("index");
        let documents = &folders.config.documents;
        assert_eq!(
            find(&folders.config, "/mycelium/"),
            Found::File(documents.join("mycelium/index.html"), Made::Written)
        );
        assert_eq!(
            find(&folders.config, "/kit/theme.css"),
            Found::File(folders.top.join("kit/theme.css"), Made::Built)
        );
    }

    #[test]
    fn no_document_is_served_at_a_mounts_path() {
        let folders = folders("kit-path");
        assert_eq!(
            find(&folders.config, "/kit/early.js"),
            Found::Missing,
            "a document was served as the kit's own"
        );
    }

    #[test]
    fn the_homes_own_files_come_first_and_are_built() {
        let mut folders = folders("home");
        let home = folders.top.join("home");
        fs::create_dir_all(&home).expect("its folder");
        fs::write(home.join("index.html"), "home").expect("its page");
        fs::write(home.join("rows.json"), "rows").expect("its rows");
        let documents = folders.config.documents.clone();
        fs::write(documents.join("index.html"), "a session's home").expect("a document");
        fs::write(documents.join("rows.json"), "a session's rows").expect("a document");
        folders.config.home = Some(home.clone());
        assert_eq!(
            find(&folders.config, "/"),
            Found::File(home.join("index.html"), Made::Built),
            "the home's page"
        );
        assert_eq!(
            find(&folders.config, "/rows.json"),
            Found::File(home.join("rows.json"), Made::Built),
            "one of its files"
        );
        assert_eq!(
            find(&folders.config, "/mycelium/"),
            Found::File(documents.join("mycelium/index.html"), Made::Written),
            "a deeper path is still a document"
        );
        fs::write(documents.join("note.html"), "a note").expect("a document");
        assert_eq!(
            find(&folders.config, "/note.html"),
            Found::File(documents.join("note.html"), Made::Written),
            "a file the home doesn't hold is still a document"
        );
        fs::write(home.join("planted.js"), "a script").expect("a stray file");
        assert_eq!(
            find(&folders.config, "/planted.js"),
            Found::Missing,
            "a file the builder never writes was served as the sites' own"
        );
    }

    #[test]
    fn the_homes_own_names_wait_for_its_builder_rather_than_fall_to_a_document() {
        let mut folders = folders("unready");
        let home = folders.top.join("home");
        fs::create_dir_all(&home).expect("its folder");
        let documents = folders.config.documents.clone();
        fs::write(documents.join("index.html"), "a session's home").expect("a document");
        fs::write(documents.join("strip.css"), "a session's strip").expect("a document");
        folders.config.home = Some(home);
        assert_eq!(
            find(&folders.config, "/"),
            Found::Unready,
            "a session's page stood as the home"
        );
        assert_eq!(
            find(&folders.config, "/strip.css"),
            Found::Unready,
            "a session's sheet stood as the home's"
        );
        folders.config.home = None;
        assert_eq!(
            find(&folders.config, "/"),
            Found::File(documents.join("index.html"), Made::Written),
            "a home written among the documents is found there"
        );
    }

    #[test]
    fn a_mount_without_its_homes_answers_with_the_documents_ones() {
        let folders = folders("homes");
        assert_eq!(
            find(&folders.config, "/code/homes.json"),
            Found::File(
                folders.config.documents.join("code/homes.json"),
                Made::Written
            )
        );
    }

    #[test]
    fn a_mounts_homes_in_the_documents_leading_out_are_refused() {
        // The mount's homes come from the documents, which sessions write,
        // so they're held to the documents as any other file there is.
        let folders = folders("homes-out");
        let homes = folders.config.documents.join("code/homes.json");
        fs::remove_file(&homes).expect("its homes gone");
        std::os::unix::fs::symlink(folders.top.join("secret"), homes).expect("a link");
        assert_eq!(
            find(&folders.config, "/code/homes.json"),
            Found::Refused,
            "a mount's homes reached outside the documents"
        );
    }

    #[test]
    fn a_mount_holding_its_own_homes_answers_with_them() {
        let folders = folders("own-homes");
        fs::write(folders.top.join("code/homes.json"), "built").expect("its own homes");
        assert_eq!(
            find(&folders.config, "/code/homes.json"),
            Found::File(folders.top.join("code/homes.json"), Made::Built)
        );
    }

    #[test]
    fn a_mount_without_any_other_file_is_missing_it() {
        let folders = folders("mount-missing");
        assert_eq!(
            find(&folders.config, "/code/repos.json"),
            Found::Missing,
            "a mount fell back to the documents for a file other than its homes"
        );
    }

    #[test]
    fn a_folder_without_its_slash_is_sent_to_it() {
        let folders = folders("slash");
        assert_eq!(
            find(&folders.config, "/mycelium"),
            Found::Folder("./mycelium/".to_owned())
        );
    }

    #[test]
    fn nothing_there_is_missing() {
        let folders = folders("missing");
        assert_eq!(find(&folders.config, "/mycelium/now.json"), Found::Missing);
    }

    #[test]
    fn a_way_out_or_a_hidden_name_is_refused() {
        let folders = folders("out");
        assert_eq!(find(&folders.config, "/../secret"), Found::Refused);
        assert_eq!(
            find(&folders.config, "/%2e%2e/secret"),
            Found::Refused,
            "escaped"
        );
        assert_eq!(find(&folders.config, "/mycelium/.now.x1"), Found::Refused);
        assert_eq!(
            find(&folders.config, "/mycelium/%zz"),
            Found::Refused,
            "a broken escape"
        );
    }

    #[test]
    fn a_link_in_the_documents_leading_out_is_refused() {
        let folders = folders("link");
        let link = folders.config.documents.join("out");
        std::os::unix::fs::symlink(folders.top.join("secret"), link).expect("a link");
        assert_eq!(find(&folders.config, "/out"), Found::Refused);
    }

    #[test]
    fn a_file_is_told_by_its_name() {
        assert_eq!(
            kind_of(Path::new("site.js")),
            "text/javascript; charset=utf-8"
        );
        assert_eq!(kind_of(Path::new("fonts/face.woff2")), "font/woff2");
        assert_eq!(kind_of(Path::new("blob")), "application/octet-stream");
    }
}
