//! Each program's lists of what it had open, as recents.nix gives them
//! (modules/quickshell/recents.nix says each field), and the lines their
//! commands print made into `recent` and `opened` records, and a
//! program page's own lists, as Chrome's downloads, into `listed` ones.

use std::fmt::Write;
use std::time::UNIX_EPOCH;

use serde::Deserialize;
use serde_json::{Value, json};

/// At most this many of a program's paths are kept, so 1 to 9 pick one.
pub const MOST: usize = 9;

/// One source, as recents.nix lists it.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
pub struct Source {
    /// The desktop entry it's for, or none for the list programs share.
    #[serde(default)]
    pub app: Option<String>,
    /// The name of a page's own list, as `downloads` is Chrome's, which
    /// makes it a `listed` record of the lines its command printed, read
    /// by the page, rather than the program's Recent.
    #[serde(default)]
    pub name: Option<String>,
    /// What each path is: a file, a folder or a site.
    #[serde(default)]
    pub kind: Option<String>,
    /// The file read again each time it changes.
    #[serde(default)]
    pub watch: Option<String>,
    /// Read only as a watch of its program begins.
    #[serde(default)]
    pub asked: bool,
    /// The command printing its paths, newest first.
    #[serde(default)]
    pub list: Vec<String>,
    /// The folders a page's own list is of, read by this daemon rather
    /// than a command, as what was taken lately is (Taken.qml): every
    /// change in one reads it again, and one that isn't there holds
    /// nothing, as Recordings before the first recording.
    #[serde(default)]
    pub folders: Vec<String>,
    /// Whether each line ends in a tab and when, in seconds since 1970.
    #[serde(default)]
    pub dated: bool,
    /// The command printing what it has open now.
    #[serde(default)]
    pub opened: Vec<String>,
}

impl Source {
    /// The key its list is held under: `recent:` and its program, or
    /// `listed:`, its program and the list's name.
    pub fn list_key(&self) -> String {
        let app = self.app.as_deref().unwrap_or("shared");
        match &self.name {
            Some(name) => format!("listed:{app}:{name}"),
            None => format!("recent:{app}"),
        }
    }

    /// The kind of record its list is: `listed` or `recent`.
    pub fn list_kind(&self) -> &'static str {
        if self.name.is_some() {
            "listed"
        } else {
            "recent"
        }
    }

    /// The `opened` key what it has open is held under, where it says.
    pub fn opened_key(&self) -> Option<String> {
        let app = self.app.as_deref()?;
        (!self.opened.is_empty()).then(|| format!("opened:{app}"))
    }

    /// Whether it has a list to read, by a command or of its folders.
    pub fn lists(&self) -> bool {
        !self.list.is_empty() || !self.folders.is_empty()
    }

    fn site(&self) -> bool {
        self.kind.as_deref() == Some("site")
    }
}

/// A line apart from when it says it was last used, in seconds, or none
/// where the source isn't dated or the line's time isn't in digits.
pub fn dated(line: &str, is_dated: bool) -> (&str, Option<u64>) {
    if !is_dated {
        return (line, None);
    }
    // A last field that's empty is a time not known; one that isn't a
    // time is the line's own, as a site's title where its time is left
    // off, and is kept rather than lost.
    match line.rsplit_once('\t') {
        Some((rest, "")) => (rest, None),
        Some((rest, seconds)) => match seconds.parse() {
            Ok(seconds) => (rest, Some(seconds)),
            Err(_) => (line, None),
        },
        None => (line, None),
    }
}

/// The `recent` record of a source's list as its command printed it.
pub fn recent(source: &Source, printed: &str) -> Value {
    let lines = printed.lines().filter(|line| !line.is_empty());
    if let Some(name) = &source.name {
        // A page's own list, its lines as printed, for its page to read,
        // so the shell starts no process of its own for it.
        let lines: Vec<&str> = lines.collect();
        return json!({
            "key": source.list_key(),
            "kind": "listed",
            "app": source.app,
            "name": name,
            "lines": lines,
        });
    }
    let mut record = json!({
        "key": source.list_key(),
        "kind": "recent",
        "app": source.app,
        "of": source.kind.as_deref().unwrap_or("file"),
    });
    if source.app.is_none() {
        // The list programs share: a program, a path whole though it
        // holds a tab, and when, each program's paths found by the shell.
        let paths: Vec<Value> = lines
            .filter_map(|line| {
                let (line, used) = dated(line, source.dated);
                let (program, path) = line.split_once('\t')?;
                Some(json!({"program": program, "path": path, "used_at_s": used}))
            })
            .collect();
        record["paths"] = json!(paths);
    } else if source.site() {
        let sites: Vec<Value> = lines
            .map(|line| {
                let (line, used) = dated(line, source.dated);
                let (address, title) = line.split_once('\t').unwrap_or((line, ""));
                json!({"address": address, "title": title, "used_at_s": used})
            })
            .take(MOST)
            .collect();
        record["sites"] = json!(sites);
    } else {
        let paths: Vec<Value> = lines
            .map(|line| {
                let (path, used) = dated(line, source.dated);
                json!({"path": path, "used_at_s": used})
            })
            .take(MOST)
            .collect();
        record["paths"] = json!(paths);
    }
    record
}

/// What's in these folders, a file a line, newest first, as find's
/// `%T@\t%s\t%p` prints them: when it last changed in seconds since 1970
/// to the millisecond, its size in bytes and its whole path. Hidden
/// files, what isn't a file, and a folder that isn't there are left out.
pub fn files_in(folders: &[String]) -> String {
    let mut files: Vec<(u128, u64, String)> = Vec::new();
    for folder in folders {
        let Ok(read) = std::fs::read_dir(folder) else {
            continue;
        };
        for entry in read.flatten() {
            if entry.file_name().to_string_lossy().starts_with('.') {
                continue;
            }
            let Ok(about) = entry.metadata() else {
                continue;
            };
            if !about.is_file() {
                continue;
            }
            let at = about
                .modified()
                .ok()
                .and_then(|at| at.duration_since(UNIX_EPOCH).ok())
                .map_or(0, |at| at.as_millis());
            let path = format!("{folder}/{}", entry.file_name().to_string_lossy());
            files.push((at, about.len(), path));
        }
    }
    files.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.2.cmp(&b.2)));
    files
        .iter()
        .fold(String::new(), |mut printed, (at, size, path)| {
            let _ = writeln!(printed, "{}.{:03}\t{size}\t{path}", at / 1000, at % 1000);
            printed
        })
}

/// The `opened` record of what a source's program has open, as its
/// command printed it: a site's tabs, its id, window, whether it's in
/// front, when it was last looked at, its address and its title; or a
/// file's process and path, or, where one process has many windows,
/// niri's window (`@` and its id), the path, and what Go To is given.
pub fn opened(source: &Source, printed: &str) -> Option<Value> {
    let key = source.opened_key()?;
    let lines = printed.lines().filter(|line| !line.is_empty());
    let mut record = json!({"key": key, "kind": "opened", "app": source.app});
    if source.site() {
        let tabs: Vec<Value> = lines
            .filter_map(|line| {
                let mut fields = line.splitn(6, '\t');
                let id = fields.next()?;
                let window = fields.next()?;
                let front = fields.next()?;
                let looked = fields.next()?;
                Some(json!({
                    "id": id,
                    "window": window,
                    "front": front == "true",
                    "looked_at_ms": looked.parse::<u64>().ok(),
                    "address": fields.next().unwrap_or(""),
                    "title": fields.next().unwrap_or(""),
                }))
            })
            .collect();
        record["sites"] = json!(tabs);
    } else {
        let files: Vec<Value> = lines
            .filter_map(|line| {
                let (first, rest) = line.split_once('\t')?;
                if let Some(window) = first.strip_prefix('@') {
                    let (path, go) = rest.rsplit_once('\t').unwrap_or((rest, ""));
                    return Some(json!({
                        "pid": null,
                        "window": window.parse::<u64>().ok(),
                        "path": path,
                        "go": go,
                    }));
                }
                // A line that names no process is no file, rather than
                // one whose process is unknown.
                Some(json!({
                    "pid": first.parse::<u32>().ok()?,
                    "window": null,
                    "path": rest,
                    "go": null,
                }))
            })
            .collect();
        record["files"] = json!(files);
    }
    Some(record)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source(fields: Value) -> Source {
        serde_json::from_value(fields).unwrap()
    }

    #[test]
    fn a_dated_lines_time_is_kept_apart() {
        assert_eq!(dated("/a\t1793000000", true), ("/a", Some(1_793_000_000)));
        assert_eq!(dated("/a\t", true), ("/a", None));
        assert_eq!(dated("site\tIts Title", true), ("site\tIts Title", None));
        assert_eq!(dated("/a\tb", false), ("/a\tb", None));
    }

    #[test]
    fn a_folder_list_keeps_nine_newest_first() {
        let kitty = source(json!({"app": "kitty", "kind": "folder", "list": ["zoxide"]}));
        let printed = (1..=12)
            .map(|n| format!("/f{n}"))
            .collect::<Vec<_>>()
            .join("\n");
        let record = recent(&kitty, &printed);
        assert_eq!(record["key"], "recent:kitty");
        assert_eq!(record["of"], "folder");
        assert_eq!(record["paths"].as_array().unwrap().len(), MOST);
        assert_eq!(
            record["paths"][0],
            json!({"path": "/f1", "used_at_s": null})
        );
    }

    #[test]
    fn the_shared_list_says_each_lines_program() {
        let shared = source(json!({"list": ["recent-files"], "dated": true}));
        let record = recent(&shared, "org.pwmt.zathura\t/a\tb.pdf\t1793000000\nbare\n");
        assert_eq!(record["key"], "recent:shared");
        assert_eq!(
            record["paths"],
            json!([{
                "program": "org.pwmt.zathura",
                "path": "/a\tb.pdf",
                "used_at_s": 1_793_000_000
            }])
        );
    }

    #[test]
    fn a_pages_own_list_keeps_its_lines_under_its_name() {
        let downloads = source(json!({"app": "google-chrome", "name": "downloads"}));
        assert_eq!(downloads.list_kind(), "listed");
        let record = recent(
            &downloads,
            "/d/a.pdf\t12\t1793000000\thttps://a.example\n\n",
        );
        assert_eq!(
            record,
            json!({
                "key": "listed:google-chrome:downloads",
                "kind": "listed",
                "app": "google-chrome",
                "name": "downloads",
                "lines": ["/d/a.pdf\t12\t1793000000\thttps://a.example"],
            })
        );
    }

    #[test]
    fn a_sites_list_says_each_address_and_title() {
        let chrome = source(json!({"app": "google-chrome", "kind": "site", "dated": true}));
        let record = recent(&chrome, "https://a.example\tA\t1793000000\n");
        assert_eq!(
            record["sites"],
            json!([{"address": "https://a.example", "title": "A", "used_at_s": 1_793_000_000}])
        );
    }

    #[test]
    fn what_a_program_has_open_is_its_tabs_or_its_files() {
        let chrome = source(json!({"app": "google-chrome", "kind": "site", "opened": ["x"]}));
        let record = opened(&chrome, "7\t3\ttrue\t1793000000123\thttps://a.example\tA\n").unwrap();
        assert_eq!(record["key"], "opened:google-chrome");
        assert_eq!(record["sites"][0]["front"], true);
        assert_eq!(record["sites"][0]["looked_at_ms"], 1_793_000_000_123_u64);
        let nvim = source(json!({"app": "nvim", "kind": "file", "opened": ["x"]}));
        let record = opened(&nvim, "412\t/a.md\n@9\t/b.md\t2\n").unwrap();
        assert_eq!(
            record["files"],
            json!([
                {"pid": 412, "window": null, "path": "/a.md", "go": null},
                {"pid": null, "window": 9, "path": "/b.md", "go": "2"}
            ])
        );
        let shared = source(json!({"list": ["x"]}));
        assert!(opened(&shared, "").is_none(), "no name opens nothing");
    }
}
