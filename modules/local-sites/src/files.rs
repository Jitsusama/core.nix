//! Which file a path names. A path's first part may name a folder Nix
//! built (a mount), which is served whole at `/<name>/`; anything else is
//! the documents sessions write. A folder answers with its `index.html`,
//! and one named without its last slash is sent to it with one, so its
//! page's relative links resolve. Nothing outside a root is reached: a
//! `..`, a hidden name (a writer's half-made `.now.XXXX`), or a link in
//! the documents leading out of them is refused, as the server before
//! this one refused it.

use std::path::{Path, PathBuf};

use crate::config::Config;

/// What a path found.
#[derive(Debug, PartialEq, Eq)]
pub enum Found {
    /// A file to send.
    File(PathBuf),
    /// A folder named without its last slash, sent here instead.
    Folder(String),
    /// Nothing is there.
    Missing,
    /// A path reaching out of its root, or one no file may have.
    Refused,
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
    let (root, rest, guarded) = match parts.split_first() {
        Some((first, rest)) if config.mounts.contains_key(*first) => {
            (config.mounts[*first].as_path(), rest, false)
        }
        _ => (config.documents.as_path(), &parts[..], true),
    };
    let named: PathBuf = rest.iter().collect();
    let mut file = root.join(named);
    if file.is_dir() {
        if !decoded.ends_with('/') {
            return Found::Folder(format!("{path}/"));
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
    Found::File(file)
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
        for folder in [documents.join("mycelium"), kit.clone()] {
            fs::create_dir_all(folder).expect("a folder");
        }
        fs::write(documents.join("mycelium/index.html"), "map").expect("a page");
        fs::write(documents.join("mycelium/.now.x1"), "half").expect("a half-made file");
        fs::write(top.join("secret"), "no").expect("a file outside");
        fs::write(kit.join("theme.css"), "roles").expect("the kit");
        let config = Config {
            listen: "127.0.0.1:8484".parse().expect("an address"),
            names: vec!["localhost".to_owned()],
            documents,
            mounts: BTreeMap::from([("kit".to_owned(), kit)]),
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
            Found::File(documents.join("mycelium/index.html"))
        );
        assert_eq!(
            find(&folders.config, "/kit/theme.css"),
            Found::File(folders.top.join("kit/theme.css"))
        );
    }

    #[test]
    fn a_folder_without_its_slash_is_sent_to_it() {
        let folders = folders("slash");
        assert_eq!(
            find(&folders.config, "/mycelium"),
            Found::Folder("/mycelium/".to_owned())
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
