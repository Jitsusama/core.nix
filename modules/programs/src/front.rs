//! The tab in front of one of niri's windows of a program whose tabs it
//! reads, Chrome's (`front:<app>:<niri id>`), so the shell can name what
//! Joel is looking at when he says a line over a window (Super+S) and
//! match nothing itself. Chrome gives its windows no id niri knows, but
//! titles each by the tab in front of it, so the window is matched to the
//! one front tab its title begins with, followed by what Chrome adds
//! after it (" - Google Chrome"). Where two of its windows have tabs of
//! that title in front, or none has, there's no record, since which tab
//! Joel sees can't be told.

use serde_json::{Value, json};

/// The kind of record this is.
pub const KIND: &str = "front";

/// The key of the tab in front of niri's window `window` of `app`.
#[must_use]
pub fn key(app: &str, window: u64) -> String {
    format!("{KIND}:{app}:{window}")
}

/// The program and niri's window a key names, or none for a key of
/// another kind.
#[must_use]
pub fn named(key: &str) -> Option<(&str, u64)> {
    let (app, window) = key.strip_prefix("front:")?.rsplit_once(':')?;
    Some((app, window.parse().ok()?))
}

/// The record of the tab in front of niri's window `window` of `app`,
/// titled `titled`, among the tabs its `opened` record lists: its title
/// and address, where exactly one of the program's windows has a tab in
/// front whose title the window's begins with, the longest such title;
/// none otherwise.
#[must_use]
pub fn record(app: &str, window: u64, titled: &str, opened: &Value) -> Option<Value> {
    let begins = |title: &str| {
        !title.is_empty()
            && titled
                .strip_prefix(title)
                .is_some_and(|after| after.is_empty() || after.starts_with(" - "))
    };
    let fronts: Vec<&Value> = opened["sites"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|site| site["front"] == true)
        .filter(|site| site["title"].as_str().is_some_and(begins))
        .collect();
    let longest = fronts
        .iter()
        .filter_map(|site| site["title"].as_str())
        .map(str::len)
        .max()?;
    let mut matched = fronts
        .iter()
        .filter(|site| site["title"].as_str().map(str::len) == Some(longest));
    let site = matched.next()?;
    if matched.any(|other| other["window"] != site["window"]) {
        return None;
    }
    Some(json!({
        "key": key(app, window),
        "kind": KIND,
        "app": app,
        "window": window,
        "title": site["title"],
        "address": site["address"],
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opened(sites: &[(&str, bool, &str, &str)]) -> Value {
        let sites: Vec<Value> = sites
            .iter()
            .map(|(window, front, title, address)| {
                json!({"window": window, "front": front, "title": title, "address": address})
            })
            .collect();
        json!({"key": "opened:google-chrome", "sites": sites})
    }

    #[test]
    fn a_key_names_its_program_and_niris_window() {
        assert_eq!(
            named(&key("google-chrome", 82)),
            Some(("google-chrome", 82))
        );
        assert_eq!(named("opened:google-chrome"), None);
        assert_eq!(named("front:google-chrome:x"), None);
    }

    #[test]
    fn a_window_is_matched_to_the_one_front_tab_its_title_begins_with() {
        let open = opened(&[
            ("1", true, "Inbox", "https://mail.example.org/"),
            ("1", false, "Docs", "https://docs.example.org/a"),
            ("2", true, "Docs", "https://docs.example.org/b"),
        ]);
        let front =
            |titled: &str| record("google-chrome", 9, titled, &open).map(|r| r["address"].clone());
        assert_eq!(
            front("Docs - Google Chrome"),
            Some(json!("https://docs.example.org/b")),
            "the tab in front, not one behind of the same title"
        );
        assert_eq!(
            record("google-chrome", 9, "Inbox - Google Chrome", &open),
            Some(json!({
                "key": "front:google-chrome:9", "kind": "front", "app": "google-chrome",
                "window": 9, "title": "Inbox", "address": "https://mail.example.org/",
            }))
        );
        assert_eq!(front("Inboxes - Google Chrome"), None, "a title only begun");
        assert_eq!(front("Calendar"), None, "no tab of its title in front");
    }

    #[test]
    fn the_longest_title_wins_and_two_windows_alike_give_none() {
        let open = opened(&[
            ("1", true, "Inbox", "https://a.example.org/"),
            ("2", true, "Inbox - Mail", "https://b.example.org/"),
            ("3", true, "Same", "https://c.example.org/"),
            ("4", true, "Same", "https://d.example.org/"),
        ]);
        let front =
            |titled: &str| record("google-chrome", 9, titled, &open).map(|r| r["address"].clone());
        assert_eq!(
            front("Inbox - Mail - Google Chrome"),
            Some(json!("https://b.example.org/"))
        );
        assert_eq!(front("Same - Google Chrome"), None, "which can't be told");
    }
}
