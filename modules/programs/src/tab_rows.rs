//! The rows of Chrome's open tabs, a place of their own (`tabs`), so the
//! launcher's home finds each by its title or its site as it finds a
//! document or a person (Joel, 2026-11-11: "i need to be able to freeform
//! search for chrome tabs outside of the program"). Each is made from
//! the `opened` record of what a program has open (source.rs's `opened`):
//! one row a page, the one looked at last first, two tabs on one page
//! being one row. Return goes to Chrome's window, the one the page is in
//! front of where the shell finds it by its title (`front`), and Copy the
//! Link copies its address.

use std::collections::BTreeMap;

use chrome_session::{host, one_line};
use serde_json::{Value, json};

/// The place these rows are of.
pub const PLACE: &str = "tabs";

/// One page open now, with every tab it's open in.
struct Page<'a> {
    address: &'a str,
    title: &'a str,
    window: &'a str,
    looked: u64,
    tabs: usize,
}

/// The rows of every page each program has open, as its `opened` record
/// lists them, by the program's desktop entry id: the one looked at last
/// first, each program's after the one before.
#[must_use]
pub fn rows<'a>(opened: impl IntoIterator<Item = (&'a str, &'a Value)>) -> Vec<Value> {
    opened
        .into_iter()
        .flat_map(|(app, record)| program(app, record))
        .collect()
}

/// One program's pages as rows.
fn program(app: &str, record: &Value) -> Vec<Value> {
    let sites: Vec<&Value> = record["sites"].as_array().into_iter().flatten().collect();
    // What each of its windows has in front, which Chrome titles the
    // window by, so Return finds the window the page is in.
    let front: BTreeMap<&str, &str> = sites
        .iter()
        .filter(|site| site["front"] == true)
        .map(|site| (text(&site["window"]), text(&site["title"])))
        .collect();
    let mut pages: Vec<Page> = Vec::new();
    for site in &sites {
        let address = text(&site["address"]);
        if address.is_empty() {
            continue;
        }
        let looked = site["looked_at_ms"].as_u64().unwrap_or(0);
        if let Some(page) = pages.iter_mut().find(|page| page.address == address) {
            page.tabs += 1;
            if looked > page.looked {
                page.looked = looked;
                page.window = text(&site["window"]);
            }
            continue;
        }
        pages.push(Page {
            address,
            title: text(&site["title"]),
            window: text(&site["window"]),
            looked,
            tabs: 1,
        });
    }
    pages.sort_by_key(|page| std::cmp::Reverse(page.looked));
    pages
        .iter()
        .map(|page| row(app, page, front.get(page.window).copied().unwrap_or("")))
        .collect()
}

/// A page's row: named by its title, else its site, said by its site
/// beside it and found by it too.
fn row(app: &str, page: &Page, front: &str) -> Value {
    let site = host(page.address).unwrap_or(page.address);
    let title = one_line(page.title);
    let name = if title.is_empty() { site } else { &title };
    json!({
        "key": format!("row:{PLACE}:{}", page.address),
        "thing": format!("tab:{}", page.address),
        "place": PLACE,
        "kind": "tab",
        "name": name,
        "glyph": "web",
        "hue": "program",
        "side": [{"said": site, "keep": 1}],
        "keywords": site,
        "held": {"address": page.address, "app": app, "front": one_line(front), "tabs": page.tabs},
        "acts": [
            {"key": "verb:recent:site:go", "go": app},
            {"key": "verb:recent:site:link", "copy": page.address},
        ],
    })
}

fn text(value: &Value) -> &str {
    value.as_str().unwrap_or("")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn site(window: &str, front: bool, looked: u64, address: &str, title: &str) -> Value {
        json!({
            "id": "", "window": window, "front": front, "looked_at_ms": looked,
            "address": address, "title": title,
        })
    }

    fn opened(sites: &[Value]) -> Value {
        json!({"key": "opened:google-chrome", "kind": "opened", "sites": sites})
    }

    #[test]
    fn each_page_is_a_row_named_by_its_title_and_said_by_its_site_newest_first() {
        let record = opened(&[
            site(
                "1",
                true,
                100,
                "https://github.com/Jitsusama/core.nix",
                "core.nix",
            ),
            site(
                "1",
                false,
                300,
                "https://news.ycombinator.com/",
                "Hacker News",
            ),
        ]);
        let rows = rows([("google-chrome", &record)]);
        let names: Vec<&str> = rows.iter().map(|row| text(&row["name"])).collect();
        assert_eq!(
            names,
            ["Hacker News", "core.nix"],
            "the pages weren't newest first"
        );
        assert_eq!(
            rows[1]["side"],
            json!([{"said": "github.com", "keep": 1}]),
            "a page wasn't said by its site"
        );
        assert_eq!(
            rows[1]["keywords"], "github.com",
            "a page isn't found by its site"
        );
        assert_eq!(
            rows[1]["key"], "row:tabs:https://github.com/Jitsusama/core.nix",
            "a page's row isn't keyed by its address in its place"
        );
    }

    #[test]
    fn a_page_open_twice_is_one_row_counting_both_in_the_window_used_last() {
        let record = opened(&[
            site("1", true, 100, "https://a.example/", "A"),
            site("2", true, 500, "https://b.example/", "B"),
            site("2", false, 400, "https://a.example/", "A"),
        ]);
        let rows = rows([("google-chrome", &record)]);
        assert_eq!(rows.len(), 2, "a page open in two tabs wasn't one row");
        let a = rows
            .iter()
            .find(|row| row["name"] == "A")
            .expect("A has no row");
        assert_eq!(a["held"]["tabs"], 2, "the page's tabs weren't counted");
        assert_eq!(
            a["held"]["front"], "B",
            "Return wouldn't find the window used last"
        );
    }

    #[test]
    fn a_tab_with_no_address_has_no_row_and_one_with_no_title_is_named_by_its_site() {
        let record = opened(&[
            site("1", true, 100, "", "New Tab"),
            site("1", false, 200, "https://example.org/a", "  "),
        ]);
        let rows = rows([("google-chrome", &record)]);
        assert_eq!(rows.len(), 1, "a tab with no address had a row");
        assert_eq!(
            rows[0]["name"], "example.org",
            "an untitled page wasn't named by its site"
        );
    }

    #[test]
    fn return_goes_to_chrome_and_copy_the_link_copies_its_address() {
        let record = opened(&[site("1", true, 1, "https://example.org/", "Example")]);
        let rows = rows([("google-chrome", &record)]);
        assert_eq!(
            rows[0]["acts"],
            json!([
                {"key": "verb:recent:site:go", "go": "google-chrome"},
                {"key": "verb:recent:site:link", "copy": "https://example.org/"},
            ]),
            "a page's acts weren't going to Chrome then copying its link"
        );
    }
}
