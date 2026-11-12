//! What Chrome has open, read by the daemon itself with no command: from
//! the profile's session file, as its watch begins and again as Chrome
//! writes the file while it's watched, and from the extension's host on
//! its socket first, where it's there; and its pages as rows of their
//! own place, told as it writes the file.

use std::io::{Read as _, Write as _};
use std::os::unix::net::UnixListener;
use std::thread;

use chrome_session::written::{session, tab};
use serde_json::json;

use super::Running;

const OPENED: &str = "opened:chrome";

/// Writes the profile's session file as Chrome writes a new one, holding
/// one tab a page.
fn sessioned(running: &Running, made: u64, pages: &[&str]) {
    let mut commands = Vec::new();
    for (at, address) in (10..).zip(pages) {
        commands.extend(tab(1, at, at - 10, address, 1_793_000_000_000));
    }
    std::fs::write(
        running.path(&format!("chrome/Sessions/Session_{made}")),
        session(&commands),
    )
    .unwrap();
}

#[test]
fn chromes_open_pages_are_read_from_its_session_and_again_as_it_writes_it() {
    let running = Running::new("chrome-session");
    sessioned(&running, 1, &["https://a.example.org/"]);
    let mut client = running.client();
    client.ask(&json!({"id": 1, "ask": "watch", "keys": [OPENED]}));
    assert_eq!(
        client.changed(OPENED)["sites"],
        json!([{
            "id": "", "window": "1", "front": false, "looked_at_ms": 1_793_000_000_000_u64,
            "address": "https://a.example.org/", "title": "Title 10",
        }]),
        "the session file's one tab, with no id"
    );
    sessioned(
        &running,
        2,
        &["https://a.example.org/", "https://b.example.org/"],
    );
    let told = client.changed(OPENED);
    let addresses: Vec<&str> = told["sites"]
        .as_array()
        .unwrap()
        .iter()
        .map(|site| site["address"].as_str().unwrap())
        .collect();
    assert_eq!(
        addresses,
        ["https://a.example.org/", "https://b.example.org/"],
        "the newer session file's tabs, told as Chrome wrote it"
    );
}

#[test]
fn chromes_pages_are_rows_of_their_own_told_as_it_writes_its_session() {
    let running = Running::new("chrome-tabs");
    sessioned(&running, 1, &["https://a.example.org/"]);
    let mut client = running.client();
    client.ask(&json!({"id": 1, "ask": "watch", "kind": "row", "place": "tabs"}));
    let a = client.changed("row:tabs:https://a.example.org/");
    assert_eq!(
        a["row"]["name"], "Title 10",
        "a page's row isn't named by its title: {a}"
    );
    assert_eq!(
        a["row"]["side"],
        json!([{"said": "a.example.org", "keep": 1}]),
        "a page's row isn't said by its site: {a}"
    );
    sessioned(
        &running,
        2,
        &["https://a.example.org/", "https://b.example.org/"],
    );
    let b = client.changed("row:tabs:https://b.example.org/");
    assert_eq!(
        b["row"]["name"], "Title 11",
        "a page opened since wasn't told: {b}"
    );
}

/// Writes one of the profile's bookmark files as Chrome does, whole beside
/// it and moved over, holding these pages in its other bookmarks.
fn bookmarked(running: &Running, file: &str, pages: &[(&str, &str)]) {
    let children: Vec<_> = pages
        .iter()
        .map(|(name, address)| json!({"type": "url", "name": name, "url": address}))
        .collect();
    let text = json!({"roots": {
        "bookmark_bar": {"type": "folder", "name": "Bookmarks bar", "children": []},
        "other": {"type": "folder", "name": "Other bookmarks", "children": children},
    }, "version": 1});
    let written = running.path(&format!("chrome/{file}.tmp"));
    std::fs::write(&written, text.to_string()).unwrap();
    std::fs::rename(written, running.path(&format!("chrome/{file}"))).unwrap();
}

#[test]
fn chromes_bookmarks_are_rows_of_their_own_but_for_a_page_open_now() {
    let running = Running::new("chrome-bookmarks");
    sessioned(&running, 1, &["https://open.example.org/"]);
    bookmarked(
        &running,
        "Bookmarks",
        &[
            ("Docs", "https://a.example.org/docs"),
            ("Open", "https://open.example.org/"),
        ],
    );
    let mut client = running.client();
    let answer = client.ask(&json!({"id": 1, "ask": "watch", "kind": "row", "place": "bookmarks"}));
    let a = answer["records"]
        .as_array()
        .and_then(|records| {
            records
                .iter()
                .find(|record| record["key"] == "row:bookmarks:https://a.example.org/docs")
        })
        .unwrap_or_else(|| panic!("a watch isn't told the bookmarks' rows: {answer}"))
        .clone();
    assert_eq!(
        a["row"]["name"], "Docs",
        "a bookmark's row isn't named by it: {a}"
    );
    assert_eq!(
        a["row"]["acts"][0],
        json!({"key": "verb:recent:site:open", "open": "https://a.example.org/docs",
               "in": "chrome"}),
        "a bookmark's Return doesn't open it in Chrome: {a}"
    );
    // The session's read as the watch begins, and its open page leaves.
    let gone = client.gone();
    assert_eq!(
        gone,
        [json!("row:bookmarks:https://open.example.org/")],
        "a bookmark whose page is open isn't left out"
    );
    bookmarked(
        &running,
        "AccountBookmarks",
        &[("Account", "https://b.example.org/")],
    );
    let b = client.changed("row:bookmarks:https://b.example.org/");
    assert_eq!(
        b["row"]["name"], "Account",
        "an account's bookmark made since wasn't told: {b}"
    );
    sessioned(&running, 2, &[]);
    let open = client.changed("row:bookmarks:https://open.example.org/");
    assert_eq!(
        open["row"]["name"], "Open",
        "a bookmark whose page was open isn't told once it's shut: {open}"
    );
}

#[test]
fn chromes_pages_and_bookmarks_are_searched_by_name_site_and_folder() {
    let running = Running::new("chrome-searched");
    sessioned(
        &running,
        1,
        &["https://a.example.org/", "https://notes.example.net/"],
    );
    bookmarked(
        &running,
        "Bookmarks",
        &[("Docs", "https://docs.example.com/")],
    );
    let mut client = running.client();
    client.ask(&json!({"id": 1, "ask": "watch", "kind": "row", "place": "tabs"}));
    client.changed("row:tabs:https://notes.example.net/");
    client.ask(&json!({"id": 2, "ask": "watch", "kind": "row", "place": "bookmarks"}));
    let search = |client: &mut daemon_client::Client, id: u64, place: &str, words: &str| {
        client.ask(&json!({"id": id, "ask": "search", "place": place, "words": words}))
    };
    let said = search(&mut client, 3, "tabs", "tit");
    assert_eq!(
        said["keys"],
        json!([
            "row:tabs:https://a.example.org/",
            "row:tabs:https://notes.example.net/"
        ]),
        "pages alike in score aren't kept in the order served: {said}"
    );
    assert_eq!(
        said["marks"]["row:tabs:https://a.example.org/"],
        json!([[0, 3]]),
        "a page's name has its letters to bring forward: {said}"
    );
    let said = search(&mut client, 4, "tabs", "notes");
    assert_eq!(
        said["keys"],
        json!(["row:tabs:https://notes.example.net/"]),
        "a page isn't found by its site: {said}"
    );
    let said = search(&mut client, 5, "bookmarks", "other");
    assert_eq!(
        said["keys"],
        json!(["row:bookmarks:https://docs.example.com/"]),
        "a bookmark isn't found by its folder: {said}"
    );
    let said = search(&mut client, 6, "tabs", "docs");
    assert_eq!(
        said["keys"],
        json!([]),
        "a search of the pages finds a bookmark: {said}"
    );
}

#[test]
fn the_extensions_host_is_asked_first_on_its_socket() {
    let running = Running::new("chrome-host");
    sessioned(&running, 1, &["https://saved.example.org/"]);
    let listener = UnixListener::bind(running.path("chrome-open-tabs.sock")).unwrap();
    thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut asked = String::new();
        stream.read_to_string(&mut asked).unwrap();
        if asked == "list\n" {
            let said = "412\t7\ttrue\t1793000000123\thttps://live.example.org/\tLive\n";
            stream.write_all(said.as_bytes()).unwrap();
        }
    });
    let said = running
        .client()
        .ask(&json!({"id": 1, "ask": "get", "keys": [OPENED]}));
    assert_eq!(
        said["records"][0]["sites"],
        json!([{
            "id": "412", "window": "7", "front": true, "looked_at_ms": 1_793_000_000_123_u64,
            "address": "https://live.example.org/", "title": "Live",
        }]),
        "the host's tab, by the id it gave, rather than the file's"
    );
}
