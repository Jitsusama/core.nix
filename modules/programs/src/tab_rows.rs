//! The rows of every open tab, Chrome's and kitty's, a place of their own
//! (`tabs`), so the launcher's home finds each by its title or its site
//! as it finds a document or a person (Joel, 2026-11-11: "i need to be
//! able to freeform search for chrome tabs outside of the program"), in
//! one list, the one used last first, each said `in` its program. A
//! Chrome tab's is made from the `opened` record of what a program has
//! open (source.rs's `opened`): one row a page, two tabs on one page
//! being one row. Return goes to Chrome's window, the one the page is in
//! front of where the shell finds it by its title (`front`), and Copy the
//! Link copies its address, both Tabs' own acts (VKCF3Z's ruling
//! 98fe0553). kitty's are `kitty_tabs.rs`'s. A program with no window
//! open has no tabs to go to, so its session's are left out. Each tab's
//! drawer is served on its row, said `in` its program, and Tabs' head
//! names a program whose tabs can't be read while a window of it is open,
//! so a list missing its tabs says why (`head`).

use std::collections::BTreeMap;

use chrome_session::{host, one_line};
use serde_json::{Value, json};

use crate::kitty_tabs::{self, Kitty};

/// The place these rows are of.
pub const PLACE: &str = "tabs";

/// The words a tab's drawer and Tabs' head are said in, from the
/// vocabulary (words/tabs.json). One the vocabulary hasn't is said by its
/// id, so the tabs are served all the same, as people's names are.
pub struct Words {
    /// A tab's program, `in {program}`.
    within: String,
    /// What a tab is.
    kind: String,
    /// A program whose tabs can't be read, `{program} isn't answering`.
    silent: String,
    /// The labels of a tab's site, address and folder.
    site: String,
    address: String,
    pub(crate) folder: String,
    /// Each program's name, by its desktop entry id.
    programs: BTreeMap<String, String>,
}

impl Words {
    /// Its words from the vocabulary, each it hasn't said by its id.
    #[must_use]
    pub fn read(vocabulary: &Value) -> Words {
        let things: Vec<&Value> = vocabulary["things"]
            .as_array()
            .into_iter()
            .flatten()
            .collect();
        let of = |id: &str| things.iter().find(|thing| thing["id"] == id);
        let name = |id: &str| {
            of(id)
                .and_then(|thing| thing["name"].as_str())
                .unwrap_or(id)
                .to_owned()
        };
        let said = |field: &str| {
            of("tabs:tab")
                .and_then(|thing| thing["said"][field].as_str())
                .unwrap_or("tabs:tab")
                .to_owned()
        };
        let programs = things
            .iter()
            .filter_map(|thing| {
                let app = thing["id"].as_str()?.strip_prefix("tabs:program:")?;
                Some((app.to_owned(), thing["name"].as_str()?.to_owned()))
            })
            .collect();
        Words {
            within: said("in"),
            kind: said("kind"),
            silent: said("silent"),
            site: name("tabs:site"),
            address: name("tabs:address"),
            folder: name("tabs:folder"),
            programs,
        }
    }

    /// A saying with the program by its name, else its desktop entry id.
    fn of(&self, said: &str, app: &str) -> String {
        let program = self.programs.get(app).map_or(app, String::as_str);
        said.replace("{program}", program)
    }

    /// A tab's drawer: said in its program, titled by its row's name, by
    /// these facts and, folded under the nerd's, these.
    pub(crate) fn drawer(&self, app: &str, title: &str, facts: Value, nerd: Value) -> Value {
        let mut drawer = json!({
            "eyebrow": [
                {"said": self.of(&self.within, app), "keep": 2},
                {"said": self.kind, "keep": 1},
            ],
            "title": title,
            "body": "",
        });
        for (field, said) in [("facts", facts), ("nerd", nerd)] {
            if said.as_array().is_some_and(|said| !said.is_empty()) {
                drawer[field] = said;
            }
        }
        drawer
    }
}

/// Tabs' head, naming each of these programs whose tabs couldn't be read
/// that has a window open, in the order given; none where none has, a
/// program not running having no tabs to miss, nor before the windows
/// daemon is heard, when that isn't known (Y2YBTS, 2026-11-12, 0f04e015).
#[must_use]
pub fn head<'a>(
    silent: impl IntoIterator<Item = &'a str>,
    windows: Option<&[&Value]>,
    words: &Words,
) -> Option<Value> {
    let silent: Vec<Value> = silent
        .into_iter()
        .filter(|app| windows.is_some_and(|windows| running(app, Some(windows))))
        .map(|app| json!({"in": app, "said": words.of(&words.silent, app)}))
        .collect();
    if silent.is_empty() {
        return None;
    }
    let said: Vec<&str> = silent.iter().map(|one| text(&one["said"])).collect();
    let key = format!("head:{PLACE}");
    Some(json!({"key": key, "kind": "head", "head": {
        "key": key, "place": PLACE, "said": said.join(", "), "silent": silent,
    }}))
}

/// One page open now, with every tab it's open in.
struct Page<'a> {
    address: &'a str,
    title: &'a str,
    window: &'a str,
    looked: u64,
    tabs: usize,
}

/// The rows of every page each program has open, as its `opened` record
/// lists them, by the program's desktop entry id, and of kitty's tabs,
/// with the windows daemon's records of niri's `windows`, none before it
/// is first heard: the one looked at last first, those alike in time in
/// the order given.
#[must_use]
pub fn rows<'a>(
    opened: impl IntoIterator<Item = (&'a str, &'a Value)>,
    kitty: Option<&Kitty>,
    windows: Option<&[&Value]>,
    words: &Words,
) -> Vec<Value> {
    let mut all: Vec<(u64, Value)> = opened
        .into_iter()
        .filter(|(app, _)| running(app, windows))
        .flat_map(|(app, record)| program(app, record, words))
        .collect();
    let shown = windows.unwrap_or_default();
    all.extend(
        kitty
            .map(|kitty| kitty_tabs::rows(kitty, shown, words))
            .unwrap_or_default(),
    );
    all.sort_by_key(|(looked, _)| std::cmp::Reverse(*looked));
    all.into_iter().map(|(_, row)| row).collect()
}

/// Whether a program has a window open, of those the windows daemon said,
/// one it owns; or the windows daemon isn't heard yet, when its session's
/// pages are served as it wrote them.
fn running(app: &str, windows: Option<&[&Value]>) -> bool {
    windows.is_none_or(|windows| windows.iter().any(|window| window["owner"] == app))
}

/// One program's pages as rows, each with when it was looked at.
fn program(app: &str, record: &Value, words: &Words) -> Vec<(u64, Value)> {
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
        .map(|page| {
            let front = front.get(page.window).copied().unwrap_or("");
            (page.looked, row(app, page, front, words))
        })
        .collect()
}

/// A page's row: named by its title, else its site, said by its site
/// beside it and found by it too. Its drawer says its site, and its
/// address under the nerd's facts.
fn row(app: &str, page: &Page, front: &str, words: &Words) -> Value {
    let site = host(page.address).unwrap_or(page.address);
    let title = one_line(page.title);
    let name = if title.is_empty() { site } else { &title };
    json!({
        "key": format!("row:{PLACE}:{}", page.address),
        "thing": format!("tab:{}", page.address),
        "place": PLACE,
        "kind": "tab",
        "in": app,
        "name": name,
        "glyph": "web",
        "hue": "program",
        "side": [{"said": site, "keep": 1}],
        "keywords": site,
        "held": {"address": page.address, "app": app, "front": one_line(front), "tabs": page.tabs},
        "acts": [
            {"key": "verb:tabs:go", "go": app},
            {"key": "verb:tabs:link", "copy": page.address},
        ],
        "drawer": words.drawer(
            app,
            name,
            json!([{"label": words.site, "value": site}]),
            json!([{"label": words.address, "value": page.address}]),
        ),
    })
}

fn text(value: &Value) -> &str {
    value.as_str().unwrap_or("")
}

#[cfg(test)]
pub(crate) mod tests {
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

    /// The words a tab's drawer and Tabs' head are said in, as tabs.json
    /// holds them.
    pub fn words() -> Words {
        Words::read(&json!({"things": [
            {"id": "tabs:tab", "said": {"in": "in {program}", "kind": "tab",
                "silent": "{program} isn't answering"}},
            {"id": "tabs:site", "name": "Site"},
            {"id": "tabs:address", "name": "Address"},
            {"id": "tabs:folder", "name": "Folder"},
            {"id": "tabs:program:google-chrome", "name": "Chrome"},
            {"id": "tabs:program:kitty", "name": "Kitty"},
        ]}))
    }

    /// Chrome's rows alone, before the windows daemon is heard.
    fn rows<'a>(opened: impl IntoIterator<Item = (&'a str, &'a Value)>) -> Vec<Value> {
        super::rows(opened, None, None, &words())
    }

    fn window(id: u64, owner: &str, pid: u64, order: u64) -> Value {
        json!({"key": format!("window:{id}"), "id": id, "owner": owner, "pid": pid,
               "opened": owner, "order": order})
    }

    /// kitty's list, one kitty of one OS window holding one tab.
    fn kitty(line: &str) -> Kitty<'_> {
        Kitty {
            app: "kitty",
            lines: vec![line],
            clock: 1_793_000_000_000,
            home: "/home/joel",
        }
    }

    fn one_tab(pid: u64, title: &str) -> String {
        let os = json!([{"id": 1, "wm_class": "kitty", "tabs": [
            {"id": 3, "is_active": true, "title": title, "windows": []}]}]);
        format!("{pid}\t{os}")
    }

    #[test]
    fn chromes_and_kittys_tabs_are_one_list_the_one_used_last_first_each_said_in_its_program() {
        let record = opened(&[
            site("1", true, 1_793_000_000_500, "https://a.example/", "A"),
            site("1", false, 1_793_000_000_100, "https://b.example/", "B"),
        ]);
        let line = one_tab(40, "zsh");
        let windows = [
            window(7, "google-chrome", 30, 1),
            window(8, "kitty", 40, 300),
        ];
        let windows: Vec<&Value> = windows.iter().collect();
        let rows = super::rows(
            [("google-chrome", &record)],
            Some(&kitty(&line)),
            Some(&windows),
            &words(),
        );
        let said: Vec<(&str, &str)> = rows
            .iter()
            .map(|row| (text(&row["name"]), text(&row["in"])))
            .collect();
        assert_eq!(
            said,
            [
                ("A", "google-chrome"),
                ("zsh", "kitty"),
                ("B", "google-chrome")
            ],
            "the tabs weren't one list by when each was used, each said in its program"
        );
    }

    #[test]
    fn a_program_with_no_window_open_has_no_tabs() {
        let record = opened(&[site("1", true, 1, "https://a.example/", "A")]);
        let line = one_tab(40, "zsh");
        let windows = [window(8, "kitty", 41, 300)];
        let windows: Vec<&Value> = windows.iter().collect();
        for windows in [&windows[..], &[]] {
            let rows = super::rows(
                [("google-chrome", &record)],
                Some(&kitty(&line)),
                Some(windows),
                &words(),
            );
            assert_eq!(
                rows,
                Vec::<Value>::new(),
                "Chrome's session or a kitty gone was served with {} windows open",
                windows.len()
            );
        }
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
    fn return_goes_to_chrome_and_copy_the_link_copies_its_address_in_a_tabs_words() {
        let record = opened(&[site("1", true, 1, "https://example.org/", "Example")]);
        let rows = rows([("google-chrome", &record)]);
        assert_eq!(
            rows[0]["acts"],
            json!([
                {"key": "verb:tabs:go", "go": "google-chrome"},
                {"key": "verb:tabs:link", "copy": "https://example.org/"},
            ]),
            "a page's acts weren't a tab's Go to the Tab then Copy the Link"
        );
    }

    #[test]
    fn a_pages_drawer_says_its_a_tab_in_chrome_by_its_site_its_address_folded() {
        let record = opened(&[site("1", true, 1, "https://example.org/a", "Example")]);
        let rows = rows([("google-chrome", &record)]);
        assert_eq!(
            rows[0]["drawer"],
            json!({
                "eyebrow": [{"said": "in Chrome", "keep": 2}, {"said": "tab", "keep": 1}],
                "title": "Example",
                "body": "",
                "facts": [{"label": "Site", "value": "example.org"}],
                "nerd": [{"label": "Address", "value": "https://example.org/a"}],
            }),
            "a page's drawer isn't a tab in Chrome, by its site, its address folded"
        );
    }

    #[test]
    fn tabs_head_names_a_program_whose_tabs_cant_be_read_while_its_window_is_open() {
        let windows = [window(7, "google-chrome", 30, 1), window(8, "kitty", 40, 2)];
        let windows: Vec<&Value> = windows.iter().collect();
        assert_eq!(
            head(["google-chrome", "kitty"], Some(&windows[..]), &words()),
            Some(json!({"key": "head:tabs", "kind": "head", "head": {
                "key": "head:tabs", "place": "tabs",
                "said": "Chrome isn't answering, Kitty isn't answering",
                "silent": [
                    {"in": "google-chrome", "said": "Chrome isn't answering"},
                    {"in": "kitty", "said": "Kitty isn't answering"},
                ],
            }})),
            "Tabs' head didn't name each program whose tabs couldn't be read"
        );
        let chrome = &windows[..1];
        assert_eq!(
            head(["kitty"], Some(chrome), &words()),
            None,
            "a program with no window open was said not to answer"
        );
        assert_eq!(
            head([], Some(&windows[..]), &words()),
            None,
            "Tabs had a head while every program answered"
        );
    }
}
