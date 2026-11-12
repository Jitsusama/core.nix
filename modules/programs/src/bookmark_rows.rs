//! The rows of Chrome's bookmarks, a place of their own (`bookmarks`), so
//! the launcher's home finds each by its name or its site as it finds an
//! open tab (Joel, 2026-11-11: "searchable chrome bookmarks would be
//! nice"). Each is read from the profile's bookmark files, `Bookmarks`
//! and, where Chrome keeps an account's bookmarks apart, as a managed
//! profile does, `AccountBookmarks`. Each file is read whole into memory
//! and parsed from that copy, never held open or written, so Chrome's own
//! file is only ever read. A page open now is left out, since its tab's
//! row is the one home shows (`tab_rows.rs`). Return opens the page in
//! Chrome, by running it with the address, and Copy the Link copies it.

use std::collections::HashSet;
use std::hash::BuildHasher;
use std::path::Path;

use chrome_session::{host, one_line};
use serde_json::{Value, json};

/// The place these rows are of.
pub const PLACE: &str = "bookmarks";

/// The profile's files Chrome keeps bookmarks in, the device's first.
pub const FILES: [&str; 2] = ["Bookmarks", "AccountBookmarks"];

/// The roots Chrome files bookmarks under, in the order it shows them.
const ROOTS: [&str; 3] = ["bookmark_bar", "other", "synced"];

/// One bookmark: its address, its name and the folder it's filed in,
/// none at a root's top.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bookmark {
    pub address: String,
    pub name: String,
    pub folder: String,
}

/// Every bookmark of a profile's files, each address once, the first
/// file's first; a file that's missing or doesn't parse gives none.
#[must_use]
pub fn read(profile: &Path) -> Vec<Bookmark> {
    let mut seen = HashSet::new();
    FILES
        .iter()
        .filter_map(|file| std::fs::read(profile.join(file)).ok())
        .flat_map(|copy| parsed(&copy))
        .filter(|bookmark| seen.insert(bookmark.address.clone()))
        .collect()
}

/// The bookmarks of one file's text, each root's in Chrome's order, each
/// a page of the web; nothing where it isn't Chrome's bookmarks.
#[must_use]
pub fn parsed(text: &[u8]) -> Vec<Bookmark> {
    let Ok(file) = serde_json::from_slice::<Value>(text) else {
        return Vec::new();
    };
    let mut bookmarks = Vec::new();
    for root in ROOTS {
        for child in children(&file["roots"][root]) {
            filed(child, "", &mut bookmarks);
        }
    }
    bookmarks
}

/// A node's bookmarks, a folder's under it, said by the folders it's in.
fn filed(node: &Value, folder: &str, bookmarks: &mut Vec<Bookmark>) {
    let name = one_line(node["name"].as_str().unwrap_or(""));
    match node["type"].as_str() {
        Some("url") => {
            let address = node["url"].as_str().unwrap_or("");
            if host(address).is_some() {
                bookmarks.push(Bookmark {
                    address: address.to_owned(),
                    name,
                    folder: folder.to_owned(),
                });
            }
        }
        Some("folder") => {
            let inner = if folder.is_empty() {
                name
            } else {
                format!("{folder} / {name}")
            };
            for child in children(node) {
                filed(child, &inner, bookmarks);
            }
        }
        _ => {}
    }
}

fn children(node: &Value) -> impl Iterator<Item = &Value> {
    node["children"].as_array().into_iter().flatten()
}

/// The rows of a program's bookmarks, by its desktop entry id, leaving
/// out each page open now, in the order Chrome files them.
#[must_use]
pub fn rows<S: BuildHasher>(
    app: &str,
    bookmarks: &[Bookmark],
    open: &HashSet<&str, S>,
) -> Vec<Value> {
    bookmarks
        .iter()
        .filter(|bookmark| !open.contains(bookmark.address.as_str()))
        .map(|bookmark| row(app, bookmark))
        .collect()
}

/// A bookmark's row: named by its name, else its site, said by its site
/// beside it, and found by its site and its folder too.
fn row(app: &str, bookmark: &Bookmark) -> Value {
    let site = host(&bookmark.address).unwrap_or(&bookmark.address);
    let name = if bookmark.name.is_empty() {
        site
    } else {
        &bookmark.name
    };
    let keywords = [site, bookmark.folder.as_str()]
        .into_iter()
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    json!({
        "key": format!("row:{PLACE}:{}", bookmark.address),
        "thing": format!("bookmark:{}", bookmark.address),
        "place": PLACE,
        "kind": "bookmark",
        "name": name,
        "glyph": "web",
        "hue": "program",
        "side": [{"said": site, "keep": 1}],
        "keywords": keywords,
        "held": {"address": bookmark.address, "app": app, "folder": bookmark.folder},
        "acts": [
            {"key": "verb:recent:site:open", "open": bookmark.address, "in": app},
            {"key": "verb:recent:site:link", "copy": bookmark.address},
        ],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn url(name: &str, address: &str) -> Value {
        json!({"type": "url", "name": name, "url": address})
    }

    fn folder(name: &str, children: &[Value]) -> Value {
        json!({"type": "folder", "name": name, "children": children})
    }

    fn file(bar: &[Value], other: &[Value]) -> Vec<u8> {
        json!({"roots": {
            "bookmark_bar": folder("Bookmarks bar", bar),
            "other": folder("Other bookmarks", other),
            "synced": folder("Mobile bookmarks", &[]),
        }, "version": 1})
        .to_string()
        .into_bytes()
    }

    fn mark(address: &str, name: &str, folder: &str) -> Bookmark {
        Bookmark {
            address: address.to_owned(),
            name: name.to_owned(),
            folder: folder.to_owned(),
        }
    }

    #[test]
    fn every_bookmark_is_read_in_chromes_order_with_its_folders() {
        let text = file(
            &[url("A", "https://a.example.org/")],
            &[folder(
                "Work",
                &[
                    url("B", "https://b.example.org/x"),
                    folder("Deep", &[url("C", "http://c.example.org/")]),
                ],
            )],
        );
        assert_eq!(
            parsed(&text),
            [
                mark("https://a.example.org/", "A", ""),
                mark("https://b.example.org/x", "B", "Work"),
                mark("http://c.example.org/", "C", "Work / Deep"),
            ]
        );
    }

    #[test]
    fn what_isnt_a_page_of_the_web_or_chromes_bookmarks_gives_none() {
        let text = file(
            &[
                url("Script", "javascript:alert(1)"),
                url("Settings", "chrome://settings"),
            ],
            &[],
        );
        assert_eq!(parsed(&text), [], "neither is a page of the web");
        assert_eq!(parsed(b"not json"), [], "a file that doesn't parse");
        assert_eq!(parsed(b"{}"), [], "a file with no roots");
    }

    #[test]
    fn both_files_are_read_each_address_once() {
        let profile = std::env::temp_dir().join(format!("bookmarks-{}", std::process::id()));
        std::fs::create_dir_all(&profile).unwrap();
        std::fs::write(
            profile.join("Bookmarks"),
            file(&[url("Mine", "https://a.example.org/")], &[]),
        )
        .unwrap();
        std::fs::write(
            profile.join("AccountBookmarks"),
            file(
                &[],
                &[
                    url("Again", "https://a.example.org/"),
                    url("Account", "https://b.example.org/"),
                ],
            ),
        )
        .unwrap();
        let read = read(&profile);
        std::fs::remove_dir_all(&profile).unwrap();
        assert_eq!(
            read,
            [
                mark("https://a.example.org/", "Mine", ""),
                mark("https://b.example.org/", "Account", ""),
            ]
        );
    }

    #[test]
    fn a_bookmark_is_a_row_opening_it_in_chrome_unless_its_page_is_open() {
        let bookmarks = [
            mark("https://a.example.org/x", "", "Work"),
            mark("https://b.example.org/", "B", ""),
        ];
        let open = HashSet::from(["https://b.example.org/"]);
        let rows = rows("google-chrome", &bookmarks, &open);
        assert_eq!(rows.len(), 1, "the open page's bookmark is left out");
        let row = &rows[0];
        assert_eq!(row["key"], "row:bookmarks:https://a.example.org/x");
        assert_eq!(row["name"], "a.example.org", "named by its site");
        assert_eq!(row["keywords"], "a.example.org Work");
        assert_eq!(
            row["acts"][0],
            json!({"key": "verb:recent:site:open", "open": "https://a.example.org/x",
                   "in": "google-chrome"})
        );
    }
}
